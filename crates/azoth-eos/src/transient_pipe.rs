//! `eos.transient_pipe` - NeqSim's `TransientPipe`, the isothermal drift-flux march.
//!
//! The class divides the line into cells, carries each cell's gas and liquid mass, mixture
//! momentum and mixture energy, and advances them with the class's face split, an adaptive
//! CFL step, a drift-flux velocity split and an acoustic pressure update. Every
//! `thermodynamicUpdateInterval` steps each cell is flashed at its own pressure and the
//! inlet temperature, and the phase properties are written back.
//!
//! The Rust twin of `python/src/azoth/eos/reference/transient_pipe.py`, written to mirror it
//! line for line.

#[allow(unused_imports)]
use azoth_core::units::{
    Length, Mass, MassRate, Pressure, ThermodynamicTemperature, Time, kilograms,
    kilograms_per_cubic_meter, kilograms_per_mole, meters, pascals, seconds,
};
use azoth_core::{AzothError, Result, apply_checks};

use crate::Cubic;
use crate::databank;
use crate::hydrate_inhibitor_wt::{PhaseLabel, label};
use crate::molar_enthalpy_entropy::{IdealGasModel, molar_enthalpy_entropy};
use crate::parachor_mixture_surface_tension::parachor_mixture_surface_tension;
use crate::phase_transport::{PhaseKind, viscosity_only};
use crate::pipe_beggs_and_brills::{corrected_molar_volume, molar_mass};
use crate::pr_molar_volume::MOLAR_GAS_CONSTANT;
use crate::results::TransientPipeResult;
use crate::sound_speed::phase_speed_of;
use crate::tp_multiflash::tp_multiflash;
use crate::{mixture::Mixture, model_gen};

/// `TransientPipe.GRAVITY`, and the same constant in `DriftFluxModel` and `FlowRegimeDetector`.
const GRAVITY: f64 = 9.81;
/// `TransientPipe.maxSimulationTime`.
const DEFAULT_MAX_TIME: f64 = 3600.0;
/// `TransientPipe.dt`, used only for the history allocation.
const DEFAULT_DT: f64 = 0.1;
/// `TransientPipe.cflNumber`.
const DEFAULT_CFL: f64 = 0.5;
/// `TransientPipe.minTimeStep`.
const MIN_TIME_STEP: f64 = 1.0e-4;
/// `TransientPipe.maxTimeStep`.
const MAX_TIME_STEP: f64 = 10.0;
/// `TransientPipe.thermodynamicUpdateInterval`.
const DEFAULT_THERMO_INTERVAL: usize = 10;
/// `TransientPipe.historyInterval`; the class has no setter.
const HISTORY_INTERVAL: usize = 10;

const GAS_FALLBACK: (f64, f64, f64, f64) = (1.0, 1.0e-5, 0.0, 340.0);
const LIQUID_FALLBACK: (f64, f64, f64, f64) = (800.0, 1.0e-3, 0.0, 1500.0);
const DEFAULT_SIGMA: f64 = 0.020;
const DEFAULT_WATER_SIGMA: f64 = 0.072;

/// `FlowRegimeDetector.DEGENERATE_GAS_VISCOSITY_FRACTION`.
const DEGENERATE_GAS_VISCOSITY_FRACTION: f64 = 0.01;
/// `FlowRegimeDetector.SHELTERING_COEFFICIENT`.
const SHELTERING_COEFFICIENT: f64 = 0.01;
/// `FlowRegimeDetector.ANNULAR_BRIDGING_HOLDUP`.
const ANNULAR_BRIDGING_HOLDUP: f64 = 0.24;
/// `FlowRegimeDetector.BUBBLE_PACKING_VOID_FRACTION`.
const BUBBLE_PACKING_VOID_FRACTION: f64 = 0.52;
/// `FlowRegimeDetector.DEFAULT_GAS_OIL_IFT`.
const DEFAULT_GAS_OIL_IFT: f64 = 0.020;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Regime {
    StratifiedSmooth,
    StratifiedWavy,
    Slug,
    Annular,
    DispersedBubble,
    Bubble,
    Churn,
    Mist,
    SinglePhaseGas,
    SinglePhaseLiquid,
}

#[derive(Debug, Clone, Copy)]
struct Drift {
    c0: f64,
    drift_velocity: f64,
    gas_velocity: f64,
    liquid_velocity: f64,
    void_fraction: f64,
    liquid_holdup: f64,
}

impl Default for Drift {
    fn default() -> Self {
        Self {
            c0: 1.0,
            drift_velocity: 0.0,
            gas_velocity: 0.0,
            liquid_velocity: 0.0,
            void_fraction: 0.0,
            liquid_holdup: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct Phase {
    kind: PhaseLabel,
    fraction: f64,
    composition: Vec<f64>,
    z_factor: f64,
    density: f64,
    molar_mass: f64,
    viscosity: f64,
    enthalpy: f64,
    sound_speed: f64,
    volume_fraction: f64,
}

#[derive(Debug, Clone)]
struct Fluid {
    gas: Option<Phase>,
    liquid: Option<Phase>,
    total_density: f64,
    surface_tension: f64,
    gas_beta: f64,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Section {
    position: f64,
    length: f64,
    diameter: f64,
    area: f64,
    inclination: f64,
    elevation: f64,
    roughness: f64,
    pressure: f64,
    temperature: f64,
    gas_holdup: f64,
    liquid_holdup: f64,
    gas_velocity: f64,
    liquid_velocity: f64,
    gas_density: f64,
    liquid_density: f64,
    gas_viscosity: f64,
    liquid_viscosity: f64,
    gas_sound_speed: f64,
    liquid_sound_speed: f64,
    surface_tension: f64,
    gas_enthalpy: f64,
    liquid_enthalpy: f64,
    flow_regime: Regime,
    mixture_velocity: f64,
    mixture_density: f64,
    superficial_gas_velocity: f64,
    superficial_liquid_velocity: f64,
    gravity_pressure_gradient: f64,
    friction_pressure_gradient: f64,
}

impl Section {
    fn update_derived(&mut self) {
        if self.gas_holdup + self.liquid_holdup > 1.0 {
            let total = self.gas_holdup + self.liquid_holdup;
            self.gas_holdup /= total;
            self.liquid_holdup /= total;
        }
        let gas = self.gas_holdup * self.gas_velocity;
        let liquid = self.liquid_holdup * self.liquid_velocity;
        self.update_derived_with(gas, liquid);
    }

    fn update_derived_with(&mut self, gas_superficial: f64, liquid_superficial: f64) {
        self.mixture_density =
            self.gas_holdup * self.gas_density + self.liquid_holdup * self.liquid_density;
        self.mixture_velocity = gas_superficial + liquid_superficial;
        self.superficial_gas_velocity = gas_superficial;
        self.superficial_liquid_velocity = liquid_superficial;
    }

    fn conservative(&self) -> [f64; 4] {
        [
            self.gas_density * self.gas_holdup,
            self.liquid_density * self.liquid_holdup,
            self.mixture_density * self.mixture_velocity,
            self.mixture_density
                * (self.gas_holdup * self.gas_enthalpy + self.liquid_holdup * self.liquid_enthalpy),
        ]
    }

    fn wallis_sound_speed(&self) -> f64 {
        let rho_m = self.mixture_density;
        let term_g = if self.gas_holdup > 1e-6 {
            self.gas_holdup / (self.gas_density * self.gas_sound_speed * self.gas_sound_speed)
        } else {
            0.0
        };
        let term_l = if self.liquid_holdup > 1e-6 {
            self.liquid_holdup
                / (self.liquid_density * self.liquid_sound_speed * self.liquid_sound_speed)
        } else {
            0.0
        };
        let compressibility = term_g + term_l;
        if compressibility <= 0.0 || rho_m <= 0.0 {
            return jmax(self.gas_sound_speed, self.liquid_sound_speed);
        }
        (1.0 / (rho_m * compressibility)).sqrt()
    }
}

fn jmax(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn jmin(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

fn phase_kind(kind: PhaseLabel) -> PhaseKind {
    match kind {
        PhaseLabel::Gas => PhaseKind::Gas,
        PhaseLabel::Oil => PhaseKind::Oil,
        PhaseLabel::Aqueous => PhaseKind::Aqueous,
    }
}

/// The three-phase refusal's reason, matched by [`is_three_phase`] so a per-cell flash can
/// tell it from a failed flash.
const THREE_PHASE_REASON: &str = "the flash leaves both an oil and an aqueous phase, and the class's three-phase liquid averaging has no counterpart in this model";

fn flash_phases(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    names: &[String],
    t: f64,
    p: f64,
    z: &[f64],
    srk_shift: bool,
) -> Result<Vec<Phase>> {
    let flash = tp_multiflash(mixture, azoth_core::units::kelvins(t), pascals(p), z)?;
    let reduced = mixture.reduced_parameters(azoth_core::units::kelvins(t), pascals(p))?;
    let components = mixture.components();
    let mut shifted = Vec::new();
    let mut unshifted = Vec::new();
    for (index, fraction) in flash.phase_fractions.iter().enumerate() {
        if *fraction <= 0.0 {
            continue;
        }
        let composition = &flash.x[index];
        let root = flash.z_factor[index];
        let ideal = root * MOLAR_GAS_CONSTANT * t / p;
        let volume = corrected_molar_volume(root, t, p, composition, components, srk_shift);
        if !volume.is_finite() || volume <= 0.0 {
            return Err(AzothError::invalid_input(
                "components",
                "a phase's molar volume is not positive",
            ));
        }
        shifted.push(*fraction * volume);
        unshifted.push(*fraction * ideal);
    }
    let total_shifted: f64 = shifted.iter().sum();
    let total_unshifted: f64 = unshifted.iter().sum();
    if total_shifted <= 0.0 || total_unshifted <= 0.0 {
        return Err(AzothError::invalid_input(
            "components",
            "the flash left no phase volume",
        ));
    }

    let mut phases = Vec::new();
    let mut position = 0usize;
    for (index, fraction) in flash.phase_fractions.iter().enumerate() {
        if *fraction <= 0.0 {
            continue;
        }
        let composition = flash.x[index].clone();
        let root = flash.z_factor[index];
        let molar_mass = molar_mass(&composition, components, names)?;
        let volume = corrected_molar_volume(root, t, p, &composition, components, srk_shift);
        let kind = label(mixture, &reduced, &composition, root)?;
        let mu = viscosity_only(
            mixture,
            phase_kind(kind),
            azoth_core::units::kelvins(t),
            pascals(p),
            &composition,
        )?;
        let enthalpy = molar_enthalpy_entropy(
            mixture,
            ideal_gas,
            azoth_core::units::kelvins(t),
            pascals(p),
            &composition,
            root,
        )?
        .h
        .value
            / molar_mass
            * 1000.0;
        let speed = phase_speed_of(
            mixture,
            ideal_gas,
            &reduced,
            azoth_core::units::kelvins(t),
            pascals(p),
            &composition,
            root,
        )?;
        phases.push(Phase {
            kind,
            fraction: *fraction,
            composition,
            z_factor: root,
            density: molar_mass / volume,
            molar_mass,
            viscosity: mu,
            enthalpy,
            sound_speed: speed,
            volume_fraction: unshifted[position] / total_unshifted,
        });
        position += 1;
    }
    Ok(phases)
}

fn fluid_state(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    names: &[String],
    t: f64,
    p: f64,
    z: &[f64],
    srk_shift: bool,
) -> Result<Fluid> {
    let phases = flash_phases(mixture, ideal_gas, names, t, p, z, srk_shift)?;
    let gas = phases
        .iter()
        .find(|phase| phase.kind == PhaseLabel::Gas)
        .cloned();
    let liquids: Vec<&Phase> = phases
        .iter()
        .filter(|phase| phase.kind != PhaseLabel::Gas)
        .collect();
    if liquids.len() > 1 {
        return Err(AzothError::invalid_input("components", THREE_PHASE_REASON));
    }
    let liquid = liquids.first().map(|phase| (*phase).clone());
    let parachors: Vec<f64> = mixture.components().iter().map(|c| c.parachor).collect();
    let mut sigma = DEFAULT_SIGMA;
    if let (Some(gas), Some(liquid)) = (&gas, &liquid) {
        let calculated = parachor_mixture_surface_tension(
            &parachors,
            kilograms_per_cubic_meter(gas.density),
            kilograms_per_mole(gas.molar_mass),
            &gas.composition,
            kilograms_per_cubic_meter(liquid.density),
            kilograms_per_mole(liquid.molar_mass),
            &liquid.composition,
        )
        .map(|result| result.sigma.value)
        .unwrap_or(0.0);
        sigma = if calculated <= 0.0 {
            if liquid.kind == PhaseLabel::Aqueous {
                DEFAULT_WATER_SIGMA
            } else {
                DEFAULT_GAS_OIL_IFT
            }
        } else {
            calculated
        };
    } else if let Some(liquid) = &liquid {
        sigma = if liquid.kind == PhaseLabel::Aqueous {
            DEFAULT_WATER_SIGMA
        } else {
            DEFAULT_SIGMA
        };
    }
    let total_mass: f64 = phases
        .iter()
        .map(|phase| phase.fraction * phase.molar_mass)
        .sum();
    let mut total_volume = 0.0;
    for phase in &phases {
        let volume = corrected_molar_volume(
            phase.z_factor,
            t,
            p,
            &phase.composition,
            mixture.components(),
            srk_shift,
        );
        total_volume += phase.fraction * volume;
    }
    let gas_beta = gas.as_ref().map(|phase| phase.fraction).unwrap_or(0.0);
    Ok(Fluid {
        gas,
        liquid,
        total_density: total_mass / total_volume,
        surface_tension: sigma,
        gas_beta,
    })
}

fn gradient_update(section: &mut Section, params: &Drift) {
    section.gravity_pressure_gradient = -(params.liquid_holdup * section.liquid_density
        + params.void_fraction * section.gas_density)
        * GRAVITY
        * section.inclination.sin();
    section.friction_pressure_gradient = friction_gradient(section, params, section.roughness);
}

fn friction_factor(re: f64, rough: f64, diameter: f64) -> f64 {
    if re < 10.0 {
        return 6.4;
    }
    if re < 2300.0 {
        return 64.0 / re;
    }
    let rel = rough / diameter;
    let term = (rel / 3.7).powf(1.11) + 6.9 / re;
    jmax((-1.8 * term.log10()).powi(-2), 0.001)
}

fn single_gas_friction(section: &Section, rough: f64) -> f64 {
    let d = section.diameter;
    let rho = section.gas_density;
    let mu = section.gas_viscosity;
    let u = section.superficial_gas_velocity;
    if rho <= 0.0 || mu <= 0.0 || u <= 0.0 || d <= 0.0 {
        return 0.0;
    }
    let f = friction_factor(rho * u * d / mu, rough, d);
    -f * rho * u * u / (2.0 * d)
}

fn single_liquid_friction(section: &Section, rough: f64) -> f64 {
    let d = section.diameter;
    let rho = section.liquid_density;
    let mu = section.liquid_viscosity;
    let u = section.superficial_liquid_velocity;
    if rho <= 0.0 || mu <= 0.0 || u <= 0.0 || d <= 0.0 {
        return 0.0;
    }
    let f = friction_factor(rho * u * d / mu, rough, d);
    -f * rho * u * u / (2.0 * d)
}

fn homogeneous_friction(section: &Section, params: &Drift, rough: f64) -> f64 {
    let d = section.diameter;
    let u = section.superficial_gas_velocity + section.superficial_liquid_velocity;
    let alpha_l = params.liquid_holdup;
    let alpha_g = params.void_fraction;
    let rho = alpha_l * section.liquid_density + alpha_g * section.gas_density;
    let mu = alpha_l * section.liquid_viscosity + alpha_g * section.gas_viscosity;
    let f = friction_factor(rho * u * d / mu, rough, d);
    -f * rho * u * u.abs() / (2.0 * d)
}

fn stratified_friction(section: &Section, params: &Drift, rough: f64) -> f64 {
    let d = section.diameter;
    let h = params.liquid_holdup * d;
    let beta = 2.0 * (1.0 - 2.0 * h / d).clamp(-1.0, 1.0).acos();
    let a_l = d * d / 8.0 * (beta - beta.sin());
    let a_g = std::f64::consts::PI * d * d / 4.0 - a_l;
    let s_l = d * beta / 2.0;
    let s_g = d * (std::f64::consts::PI - beta / 2.0);
    if a_l < 1e-10 || a_g < 1e-10 {
        return homogeneous_friction(section, params, rough);
    }
    let u_l = params.liquid_velocity;
    let u_g = params.gas_velocity;
    let d_hl = 4.0 * a_l / s_l;
    let d_hg = 4.0 * a_g / s_g;
    let f_l = friction_factor(
        section.liquid_density * u_l.abs() * d_hl / section.liquid_viscosity,
        rough,
        d_hl,
    );
    let f_g = friction_factor(
        section.gas_density * u_g.abs() * d_hg / section.gas_viscosity,
        rough,
        d_hg,
    );
    let dp_l = f_l * section.liquid_density * u_l * u_l.abs() / (2.0 * d_hl);
    let dp_g = f_g * section.gas_density * u_g * u_g.abs() / (2.0 * d_hg);
    -(a_l * dp_l + a_g * dp_g) / (std::f64::consts::PI * d * d / 4.0)
}

fn annular_friction(section: &Section, params: &Drift, rough: f64) -> f64 {
    let d = section.diameter;
    let alpha_l = params.liquid_holdup;
    let delta = d / 2.0 * (1.0 - (1.0 - alpha_l).max(0.0).sqrt());
    let d_core = d - 2.0 * delta;
    if d_core < 0.1 * d {
        return homogeneous_friction(section, params, rough);
    }
    let u_g = params.gas_velocity;
    let f_g = friction_factor(
        section.gas_density * u_g.abs() * d_core / section.gas_viscosity,
        rough + 0.5 * delta,
        d_core,
    );
    -f_g * section.gas_density * u_g * u_g.abs() / (2.0 * d_core)
}

fn friction_gradient(section: &Section, params: &Drift, rough: f64) -> f64 {
    match section.flow_regime {
        Regime::SinglePhaseGas => single_gas_friction(section, rough),
        Regime::SinglePhaseLiquid => single_liquid_friction(section, rough),
        Regime::StratifiedSmooth | Regime::StratifiedWavy => {
            stratified_friction(section, params, rough)
        }
        Regime::Annular => annular_friction(section, params, rough),
        _ => homogeneous_friction(section, params, rough),
    }
}

fn pressure_gradient(section: &mut Section, params: &Drift) -> f64 {
    gradient_update(section, params);
    section.gravity_pressure_gradient + section.friction_pressure_gradient
}

fn holdup_from_level(h: f64, d: f64) -> f64 {
    if h <= 0.0 {
        return 0.0;
    }
    if h >= d {
        return 1.0;
    }
    let beta = 2.0 * (1.0 - 2.0 * h / d).clamp(-1.0, 1.0).acos();
    (beta - beta.sin()) / (2.0 * std::f64::consts::PI)
}

#[allow(clippy::too_many_arguments)]
fn stratified_residual(
    h: f64,
    u_sl: f64,
    u_sg: f64,
    d: f64,
    rho_l: f64,
    rho_g: f64,
    mu_l: f64,
    mu_g: f64,
    theta: f64,
) -> f64 {
    let beta = 2.0 * (1.0 - 2.0 * h / d).clamp(-1.0, 1.0).acos();
    let a_l = d * d / 8.0 * (beta - beta.sin());
    let a_g = std::f64::consts::PI * d * d / 4.0 - a_l;
    let s_l = d * beta / 2.0;
    let s_g = d * (std::f64::consts::PI - beta / 2.0);
    let s_i = d * (beta / 2.0).sin();
    if a_l < 1e-10 || a_g < 1e-10 {
        return f64::NAN;
    }
    let u_l = u_sl * std::f64::consts::PI * d * d / 4.0 / a_l;
    let u_g = u_sg * std::f64::consts::PI * d * d / 4.0 / a_g;
    let d_hl = 4.0 * a_l / s_l;
    let d_hg = 4.0 * a_g / (s_g + s_i);
    let re_l = rho_l * u_l.abs() * d_hl / mu_l;
    let gas_viscosity = if mu_g > 0.0 {
        mu_g
    } else {
        mu_l * DEGENERATE_GAS_VISCOSITY_FRACTION
    };
    let re_g = rho_g * u_g.abs() * d_hg / gas_viscosity;
    let f_l = if re_l > 2000.0 {
        0.046 * re_l.powf(-0.2)
    } else {
        16.0 / jmax(re_l, 1.0)
    };
    let f_g = if re_g > 2000.0 {
        0.046 * re_g.powf(-0.2)
    } else {
        16.0 / jmax(re_g, 1.0)
    };
    let tau_wl = f_l * rho_l * u_l * u_l.abs() / 2.0;
    let tau_wg = f_g * rho_g * u_g * u_g.abs() / 2.0;
    let tau_i = f_g * rho_g * (u_g - u_l) * (u_g - u_l).abs() / 2.0;
    let gravity = (rho_l - rho_g) * GRAVITY * theta.sin();
    (-tau_wl * s_l + tau_i * s_i) / a_l - (-tau_wg * s_g - tau_i * s_i) / a_g - gravity
}

#[allow(clippy::too_many_arguments)]
fn estimate_stratified_level(
    u_sl: f64,
    u_sg: f64,
    d: f64,
    rho_l: f64,
    rho_g: f64,
    mu_l: f64,
    mu_g: f64,
    theta: f64,
) -> f64 {
    let mut low = 0.01 * d;
    let mut high = 0.99 * d;
    let mut residual_low = stratified_residual(low, u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta);
    let residual_high = stratified_residual(high, u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta);
    if !residual_low.is_finite() || !residual_high.is_finite() {
        return 0.5 * d;
    }
    if residual_low * residual_high > 0.0 {
        return if residual_low.abs() <= residual_high.abs() {
            low
        } else {
            high
        };
    }
    for _ in 0..60 {
        let mid = 0.5 * (low + high);
        let residual_mid = stratified_residual(mid, u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta);
        if !residual_mid.is_finite() {
            return mid;
        }
        if residual_low * residual_mid <= 0.0 {
            high = mid;
        } else {
            low = mid;
            residual_low = residual_mid;
        }
        if high - low < 1e-9 * d {
            break;
        }
    }
    0.5 * (low + high)
}

/// `DriftFluxModel.estimateStratifiedLevel`: its own level rule, not the detector's.
///
/// The class carries two stratified-level solvers. `FlowRegimeDetector` bisects a momentum
/// residual; `DriftFluxModel` adjusts a level over twenty steps. The drift-flux closure
/// reads the second, so the two are kept apart here.
fn drift_stratified_level(u_sl: f64, u_sg: f64, d: f64, theta: f64, rho_l: f64, rho_g: f64) -> f64 {
    let mut h = 0.5 * d;
    for _ in 0..20 {
        let alpha_l = holdup_from_level(h, d);
        let alpha_g = 1.0 - alpha_l;
        if alpha_l < 0.01 || alpha_g < 0.01 {
            break;
        }
        let area = std::f64::consts::PI * d * d / 4.0;
        let u_l = u_sl * area / (alpha_l * area);
        let u_g = u_sg * area / (alpha_g * area);
        let tau_ratio = rho_g * u_g * u_g / (rho_l * u_l * u_l + 1e-10);
        let gravity = (rho_l - rho_g) * GRAVITY * theta.sin() * d / (rho_l * u_l * u_l + 1e-10);
        let adjustment = 0.05 * d * (tau_ratio - 1.0 - gravity);
        h = jmax(0.01 * d, jmin(0.99 * d, h - adjustment));
    }
    h
}

#[allow(clippy::too_many_arguments)]
fn bubble_parameters(
    _u_m: f64,
    _d: f64,
    theta: f64,
    rho_l: f64,
    rho_g: f64,
    sigma: f64,
) -> (f64, f64) {
    let c0 = 1.2;
    let delta_rho = jmax(rho_l - rho_g, 0.01);
    let bubble = 1.53 * (GRAVITY * sigma * delta_rho / (rho_l * rho_l)).powf(0.25);
    if theta.abs() > 0.01 {
        (c0, bubble * theta.sin().abs())
    } else {
        (c0, 0.1 * bubble)
    }
}

#[allow(clippy::too_many_arguments)]
fn slug_parameters(u_m: f64, d: f64, theta: f64, rho_l: f64, rho_g: f64) -> (f64, f64) {
    let fr = u_m / (GRAVITY * d).sqrt();
    let c0 = if fr > 3.5 {
        1.2
    } else {
        1.05 + 0.15 * theta.sin()
    };
    let delta_rho = rho_l - rho_g;
    let u_horiz = 0.54 * (GRAVITY * d * delta_rho / rho_l).sqrt();
    let u_vert = 0.35 * (GRAVITY * d * delta_rho / rho_l).sqrt();
    let abs_theta = theta.abs();
    if abs_theta < std::f64::consts::PI / 6.0 {
        (c0, u_horiz * theta.cos() + u_vert * theta.sin())
    } else if abs_theta > std::f64::consts::PI / 3.0 {
        (c0, u_vert * theta.sin())
    } else {
        let w = (abs_theta - std::f64::consts::PI / 6.0) / (std::f64::consts::PI / 6.0);
        (
            c0,
            (1.0 - w) * u_horiz * theta.cos() + w * u_vert * theta.sin(),
        )
    }
}

fn annular_parameters(d: f64, theta: f64, rho_l: f64, rho_g: f64) -> (f64, f64) {
    let delta_rho = rho_l - rho_g;
    (
        1.0,
        0.2 * (GRAVITY * d * delta_rho.abs() / rho_l).sqrt() * theta.sin(),
    )
}

fn drift_flux(section: &Section) -> Drift {
    let u_sl = section.superficial_liquid_velocity;
    let u_sg = section.superficial_gas_velocity;
    let u_m = u_sl + u_sg;
    let d = section.diameter;
    let theta = section.inclination;
    let rho_l = section.liquid_density;
    let rho_g = section.gas_density;
    let sigma = section.surface_tension;
    let mut params = Drift::default();
    match section.flow_regime {
        Regime::Bubble | Regime::DispersedBubble => {
            (params.c0, params.drift_velocity) =
                bubble_parameters(u_m, d, theta, rho_l, rho_g, sigma);
        }
        Regime::Slug => {
            (params.c0, params.drift_velocity) = slug_parameters(u_m, d, theta, rho_l, rho_g);
        }
        Regime::Annular | Regime::Churn => {
            (params.c0, params.drift_velocity) = annular_parameters(d, theta, rho_l, rho_g);
        }
        Regime::StratifiedSmooth | Regime::StratifiedWavy => {
            let h_l = drift_stratified_level(u_sl, u_sg, d, theta, rho_l, rho_g);
            let alpha_l = holdup_from_level(h_l, d);
            params.liquid_holdup = alpha_l;
            params.void_fraction = 1.0 - alpha_l;
            if u_sl + u_sg > 1e-10 && params.void_fraction > 1e-10 {
                params.gas_velocity = u_sg / params.void_fraction;
                params.liquid_velocity = u_sl / jmax(params.liquid_holdup, 1e-10);
                params.c0 = params.gas_velocity / (u_sl + u_sg);
                params.drift_velocity = 0.0;
            } else {
                params.c0 = 1.0;
                params.drift_velocity = 0.0;
            }
        }
        _ => {}
    }
    if u_m > 1e-10 {
        let denominator = params.c0 * u_m + params.drift_velocity;
        if denominator > 1e-10 {
            params.void_fraction = u_sg / denominator;
        } else {
            params.void_fraction = if u_sg > 0.0 { 1.0 } else { 0.0 };
        }
        params.void_fraction = jmax(0.0, jmin(1.0, params.void_fraction));
        params.liquid_holdup = 1.0 - params.void_fraction;
        if params.void_fraction > 1e-10 {
            params.gas_velocity = u_sg / params.void_fraction;
        }
        if params.liquid_holdup > 1e-10 {
            params.liquid_velocity = u_sl / params.liquid_holdup;
        }
    }
    params
}

fn flow_oriented(section: &Section) -> Section {
    let gas_flow = section.superficial_gas_velocity;
    let liquid_flow = section.superficial_liquid_velocity;
    if !(gas_flow <= 0.0 && liquid_flow <= 0.0 && (gas_flow < 0.0 || liquid_flow < 0.0)) {
        return section.clone();
    }
    let mut oriented = section.clone();
    oriented.inclination = -section.inclination;
    oriented.gas_velocity = -section.gas_velocity;
    oriented.liquid_velocity = -section.liquid_velocity;
    oriented.update_derived_with(-gas_flow, -liquid_flow);
    oriented
}

fn liquid_area_fraction(h: f64, d: f64) -> f64 {
    if h <= 0.0 || d <= 0.0 {
        return 0.0;
    }
    if h >= d {
        return 1.0;
    }
    let beta = 2.0 * (1.0 - 2.0 * h / d).clamp(-1.0, 1.0).acos();
    (beta - beta.sin()) / (2.0 * std::f64::consts::PI)
}

fn dispersed_bubble(u_sl: f64, u_sg: f64, d: f64, rho_l: f64, _rho_g: f64, sigma: f64) -> bool {
    let u_m = u_sl + u_sg;
    let sigma_eff = if sigma < 1e-6 {
        DEFAULT_GAS_OIL_IFT
    } else {
        sigma
    };
    let we = rho_l * u_m * u_m * d / sigma_eff;
    // `Math.sqrt(U_SG)` on a negative gas superficial velocity is NaN, and the whole
    // criterion is then false; the class does not clamp it to zero.
    if u_sg < 0.0 {
        return false;
    }
    let u_crit = 0.725 + 4.15 * u_sg.sqrt();
    let ratio = if u_m != 0.0 { u_sg / u_m } else { f64::NAN };
    u_m > u_crit && we > 20.0 && ratio < 0.52
}

fn annular_ratio(u_sg: f64, rho_l: f64, rho_g: f64, sigma: f64) -> f64 {
    let delta_rho = rho_l - rho_g;
    if delta_rho < 1e-6 {
        return 0.0;
    }
    let sigma_eff = if sigma < 1e-6 {
        DEFAULT_GAS_OIL_IFT
    } else {
        sigma
    };
    let u_crit = 3.1 * (sigma_eff * GRAVITY * delta_rho / (rho_g * rho_g)).powf(0.25);
    u_sg / u_crit
}

fn kh_margin(u_sg: f64, h: f64, d: f64, rho_l: f64, rho_g: f64) -> f64 {
    if h < 0.01 * d || h > 0.99 * d {
        return 0.0;
    }
    let beta = 2.0 * (1.0 - 2.0 * h / d).clamp(-1.0, 1.0).acos();
    let a_g = std::f64::consts::PI * d * d / 4.0 - d * d / 8.0 * (beta - beta.sin());
    let s_i = d * (beta / 2.0).sin();
    if a_g < 1e-10 {
        return 0.0;
    }
    let u_g = u_sg * std::f64::consts::PI * d * d / 4.0 / a_g;
    let delta_rho = rho_l - rho_g;
    let c2 = 1.0 - h / d;
    let u_crit = c2 * (delta_rho * GRAVITY * a_g / (rho_g * s_i)).sqrt();
    if u_crit <= 0.0 {
        return 0.0;
    }
    u_g / u_crit
}

#[allow(clippy::too_many_arguments)]
fn wavy(
    u_sl: f64,
    u_sg: f64,
    h: f64,
    d: f64,
    theta: f64,
    rho_l: f64,
    rho_g: f64,
    mu_l: f64,
) -> bool {
    if h < 0.01 * d || h > 0.99 * d {
        return false;
    }
    let beta = 2.0 * (1.0 - 2.0 * h / d).clamp(-1.0, 1.0).acos();
    let a = std::f64::consts::PI * d * d / 4.0;
    let a_l = d * d / 8.0 * (beta - beta.sin());
    let a_g = a - a_l;
    if a_l < 1e-12 || a_g < 1e-12 {
        return false;
    }
    let u_g = u_sg * a / a_g;
    let u_l = u_sl * a / a_l;
    if u_l <= 0.0 {
        return false;
    }
    let delta_rho = rho_l - rho_g;
    let cos_theta = theta.cos();
    if delta_rho <= 0.0 || cos_theta <= 0.0 {
        return false;
    }
    let nu_l = mu_l / rho_l;
    let u_crit = (4.0 * nu_l * delta_rho * GRAVITY * cos_theta
        / (SHELTERING_COEFFICIENT * rho_g * u_l))
        .sqrt();
    u_g > u_crit
}

fn flow_regime(section: &Section) -> Regime {
    let section = flow_oriented(section);
    let u_sl = section.superficial_liquid_velocity;
    let u_sg = section.superficial_gas_velocity;
    let alpha_l = section.liquid_holdup;
    let alpha_g = section.gas_holdup;
    if alpha_l <= 0.0 && alpha_g > 0.0 {
        return Regime::SinglePhaseGas;
    }
    if alpha_g <= 0.0 && alpha_l > 0.0 {
        return Regime::SinglePhaseLiquid;
    }
    if alpha_l <= 0.0 && alpha_g <= 0.0 {
        return if u_sg != 0.0 {
            Regime::SinglePhaseGas
        } else {
            Regime::SinglePhaseLiquid
        };
    }
    let d = section.diameter;
    let theta = section.inclination;
    let rho_l = section.liquid_density;
    let rho_g = section.gas_density;
    let mu_l = section.liquid_viscosity;
    let mu_g = section.gas_viscosity;
    let sigma = section.surface_tension;

    if theta.abs() > 10.0_f64.to_radians() {
        if dispersed_bubble(u_sl, u_sg, d, rho_l, rho_g, sigma) {
            return Regime::DispersedBubble;
        }
        if annular_ratio(u_sg, rho_l, rho_g, sigma) > 1.0 {
            return Regime::Annular;
        }
        if theta > 0.0 {
            // `calcBubbleRiseVelocity` answers zero when the phases have no density contrast.
            let u_bubble = if rho_l - rho_g < 1e-6 {
                0.0
            } else {
                1.53 * (GRAVITY * sigma * (rho_l - rho_g) / (rho_l * rho_l)).powf(0.25)
            };
            let transport = u_sg + u_sl + u_bubble;
            if u_sg >= 0.0 && transport > 0.0 && 1.0 - alpha_l < BUBBLE_PACKING_VOID_FRACTION {
                let void = u_sg / transport;
                if void < 0.25 {
                    return Regime::Bubble;
                }
            }
            return Regime::Slug;
        }
        let h_l = estimate_stratified_level(u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta);
        if kh_margin(u_sg, h_l, d, rho_l, rho_g) > 1.0 {
            return Regime::Slug;
        }
        if wavy(u_sl, u_sg, h_l, d, theta, rho_l, rho_g, mu_l) {
            return Regime::StratifiedWavy;
        }
        return Regime::StratifiedSmooth;
    }

    if dispersed_bubble(u_sl, u_sg, d, rho_l, rho_g, sigma) {
        return Regime::DispersedBubble;
    }
    let h_l = estimate_stratified_level(u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta);
    if kh_margin(u_sg, h_l, d, rho_l, rho_g) > 1.0 {
        return if liquid_area_fraction(h_l, d) >= ANNULAR_BRIDGING_HOLDUP {
            Regime::Slug
        } else {
            Regime::Annular
        };
    }
    if wavy(u_sl, u_sg, h_l, d, theta, rho_l, rho_g, mu_l) {
        return Regime::StratifiedWavy;
    }
    Regime::StratifiedSmooth
}

fn split_mach_plus(mach: f64) -> f64 {
    if mach.abs() <= 1.0 {
        0.25 * (mach + 1.0) * (mach + 1.0)
    } else {
        0.5 * (mach + mach.abs())
    }
}

fn split_mach_minus(mach: f64) -> f64 {
    if mach.abs() <= 1.0 {
        -0.25 * (mach - 1.0) * (mach - 1.0)
    } else {
        0.5 * (mach - mach.abs())
    }
}

fn split_pressure_plus(mach: f64, pressure: f64) -> f64 {
    if mach.abs() <= 1.0 {
        pressure * 0.25 * (mach + 1.0) * (mach + 1.0) * (2.0 - mach)
    } else {
        pressure * 0.5 * (1.0 + mach.signum())
    }
}

fn split_pressure_minus(mach: f64, pressure: f64) -> f64 {
    if mach.abs() <= 1.0 {
        pressure * 0.25 * (mach - 1.0) * (mach - 1.0) * (2.0 + mach)
    } else {
        pressure * 0.5 * (1.0 - mach.signum())
    }
}

fn ausm_flux(left: &Section, right: &Section) -> [f64; 4] {
    let rho_l_l = left.liquid_density * left.liquid_holdup;
    let rho_g_l = left.gas_density * left.gas_holdup;
    let rho_l_r = right.liquid_density * right.liquid_holdup;
    let rho_g_r = right.gas_density * right.gas_holdup;
    let c_face = 0.5 * (left.wallis_sound_speed() + right.wallis_sound_speed());
    let p_plus = split_pressure_plus(left.mixture_velocity / c_face, left.pressure);
    let p_minus = split_pressure_minus(right.mixture_velocity / c_face, right.pressure);
    let p_face = p_plus + p_minus;

    let m_g =
        split_mach_plus(left.gas_velocity / c_face) + split_mach_minus(right.gas_velocity / c_face);
    let m_l = split_mach_plus(left.liquid_velocity / c_face)
        + split_mach_minus(right.liquid_velocity / c_face);
    let mdot_g = m_g * c_face * if m_g > 0.0 { rho_g_l } else { rho_g_r };
    let mdot_l = m_l * c_face * if m_l > 0.0 { rho_l_l } else { rho_l_r };
    let mom_g = mdot_g
        * if m_g > 0.0 {
            left.gas_velocity
        } else {
            right.gas_velocity
        };
    let mom_l = mdot_l
        * if m_l > 0.0 {
            left.liquid_velocity
        } else {
            right.liquid_velocity
        };
    let (h_g, u_g) = if m_g > 0.0 {
        (left.gas_enthalpy, left.gas_velocity)
    } else {
        (right.gas_enthalpy, right.gas_velocity)
    };
    let (h_l, u_l) = if m_l > 0.0 {
        (left.liquid_enthalpy, left.liquid_velocity)
    } else {
        (right.liquid_enthalpy, right.liquid_velocity)
    };
    let energy_g = mdot_g * (h_g + 0.5 * u_g * u_g);
    let energy_l = mdot_l * (h_l + 0.5 * u_l * u_l);
    let u_mix = if m_g + m_l > 0.0 {
        left.mixture_velocity
    } else {
        right.mixture_velocity
    };
    [
        mdot_g,
        mdot_l,
        mom_g + mom_l + p_face,
        energy_g + energy_l + p_face * u_mix,
    ]
}

fn calculate_time_step(sections: &[Section], cfl: f64, dx: f64) -> f64 {
    let mut max_wave: f64 = 0.0;
    let mut nan_count = 0usize;
    for section in sections {
        let c = section.wallis_sound_speed();
        let v_g = section.gas_velocity;
        let v_l = section.liquid_velocity;
        let p = section.pressure;
        if v_g.is_nan() || v_l.is_nan() || p.is_nan() || c.is_nan() {
            nan_count += 1;
            continue;
        }
        let v_g = jmax(-500.0, jmin(500.0, v_g));
        let v_l = jmax(-100.0, jmin(100.0, v_l));
        let c = jmax(10.0, jmin(1000.0, c));
        let local = jmax(
            jmax((v_g + c).abs(), (v_g - c).abs()),
            jmax((v_l + c).abs(), (v_l - c).abs()),
        );
        if local.is_finite() {
            max_wave = jmax(max_wave, local);
        }
    }
    if nan_count > sections.len() / 4 {
        return MIN_TIME_STEP;
    }
    if max_wave <= 0.0 {
        max_wave = 100.0;
    }
    jmax(MIN_TIME_STEP, jmin(MAX_TIME_STEP, cfl * dx / max_wave))
}

struct Pipe<'a> {
    mixture: &'a Mixture,
    ideal_gas: &'a IdealGasModel,
    names: &'a [String],
    fluid: Fluid,
    sections: Vec<Section>,
    inlet_mass_flow: f64,
    outlet_mass_flow: f64,
    inlet_pressure: f64,
    outlet_pressure: f64,
    inlet_temperature: f64,
    cfl: f64,
    max_time: f64,
    thermo_interval: usize,
    srk_shift: bool,
    inlet_boundary: &'a str,
    outlet_boundary: &'a str,
    dx: f64,
    simulation_time: f64,
    total_steps: u32,
    current_step: usize,
    mass_inventory: f64,
    history: Vec<Vec<f64>>,
    history_index: usize,
    overall_z: Vec<f64>,
}

impl<'a> Pipe<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        mixture: &'a Mixture,
        ideal_gas: &'a IdealGasModel,
        names: &'a [String],
        fluid: Fluid,
        sections: Vec<Section>,
        inlet_mass_flow: f64,
        inlet_pressure: f64,
        outlet_pressure: f64,
        inlet_temperature: f64,
        cfl: f64,
        max_time: f64,
        thermo_interval: usize,
        srk_shift: bool,
        inlet_boundary: &'a str,
        outlet_boundary: &'a str,
    ) -> Self {
        let dx = sections[0].length;
        let history_rows = (max_time / (DEFAULT_DT * HISTORY_INTERVAL as f64)) as usize + 1;
        let history = vec![vec![0.0; sections.len()]; history_rows];
        Self {
            mixture,
            ideal_gas,
            names,
            fluid,
            sections,
            inlet_mass_flow,
            outlet_mass_flow: inlet_mass_flow,
            inlet_pressure,
            outlet_pressure,
            inlet_temperature,
            cfl,
            max_time,
            thermo_interval,
            srk_shift,
            inlet_boundary,
            outlet_boundary,
            dx,
            simulation_time: 0.0,
            total_steps: 0,
            current_step: 0,
            mass_inventory: 0.0,
            history,
            history_index: 0,
            overall_z: Vec::new(),
        }
    }

    fn fluxes(&self, old: &[Section]) -> Vec<[f64; 4]> {
        let n = old.len();
        let mut fluxes = vec![[0.0; 4]; n + 1];
        if self.inlet_boundary == "constant_flow" {
            let fluid = &self.fluid;
            let rho_mix = if fluid.total_density > 1e-5 {
                fluid.total_density
            } else {
                old[0].mixture_density
            };
            let area = old[0].area;
            let u_inlet = self.inlet_mass_flow / (rho_mix * area);
            let p_inlet = old[0].pressure;
            let (rho_g, h_g, alpha_g) = fluid
                .gas
                .as_ref()
                .map(|gas| (gas.density, gas.enthalpy, gas.volume_fraction))
                .unwrap_or((0.0, 0.0, 0.0));
            let (rho_l, h_l, alpha_l) = fluid
                .liquid
                .as_ref()
                .map(|liquid| (liquid.density, liquid.enthalpy, liquid.volume_fraction))
                .unwrap_or((0.0, 0.0, 0.0));
            fluxes[0][0] = rho_g * alpha_g * u_inlet;
            fluxes[0][1] = rho_l * alpha_l * u_inlet;
            fluxes[0][2] = (rho_g * alpha_g + rho_l * alpha_l) * u_inlet * u_inlet + p_inlet;
            let h_total = rho_g * alpha_g * (h_g + 0.5 * u_inlet * u_inlet)
                + rho_l * alpha_l * (h_l + 0.5 * u_inlet * u_inlet);
            fluxes[0][3] = h_total * u_inlet + p_inlet * u_inlet;
        } else if self.inlet_boundary == "closed" {
            fluxes[0][2] = old[0].pressure;
        } else {
            fluxes[0] = ausm_flux(&old[0], &old[0]);
        }
        for index in 1..n {
            fluxes[index] = ausm_flux(&old[index - 1], &old[index]);
        }
        if self.outlet_boundary == "constant_pressure" {
            fluxes[n] = ausm_flux(&old[n - 1], &old[n - 1]);
        } else if self.outlet_boundary == "closed" {
            fluxes[n][2] = old[n - 1].pressure;
        } else {
            let section = &old[n - 1];
            let u_outlet = self.outlet_mass_flow / (section.mixture_density * section.area);
            let p_outlet = section.pressure;
            let rho_g = section.gas_density;
            let rho_l = section.liquid_density;
            let alpha_g = section.gas_holdup;
            let alpha_l = section.liquid_holdup;
            fluxes[n][0] = rho_g * alpha_g * u_outlet;
            fluxes[n][1] = rho_l * alpha_l * u_outlet;
            fluxes[n][2] = (rho_g * alpha_g + rho_l * alpha_l) * u_outlet * u_outlet + p_outlet;
            let h_total = rho_g * alpha_g * (section.gas_enthalpy + 0.5 * u_outlet * u_outlet)
                + rho_l * alpha_l * (section.liquid_enthalpy + 0.5 * u_outlet * u_outlet);
            fluxes[n][3] = h_total * u_outlet + p_outlet * u_outlet;
        }
        fluxes
    }

    fn apply_boundary(&mut self) {
        let sections = &mut self.sections;
        sections[0].temperature = self.inlet_temperature;
        if self.inlet_boundary == "constant_pressure" {
            sections[0].pressure = self.inlet_pressure;
        } else if self.inlet_boundary == "constant_flow" {
            let inlet = &mut sections[0];
            let u_m = self.inlet_mass_flow / (inlet.mixture_density * inlet.area);
            let alpha_g = inlet.gas_holdup;
            let alpha_l = inlet.liquid_holdup;
            if alpha_l < 0.001 || alpha_g < 0.001 {
                inlet.gas_velocity = u_m;
                inlet.liquid_velocity = u_m;
            } else {
                inlet.gas_velocity = u_m;
                inlet.liquid_velocity = u_m;
                inlet.update_derived();
                let params = drift_flux(inlet);
                let v_g = params.c0 * u_m + params.drift_velocity;
                let v_l = (inlet.mixture_density * u_m - inlet.gas_density * alpha_g * v_g)
                    / (inlet.liquid_density * alpha_l);
                inlet.gas_velocity = v_g;
                inlet.liquid_velocity = v_l;
            }
        } else {
            sections[0].gas_velocity = 0.0;
            sections[0].liquid_velocity = 0.0;
        }
        let last = sections.len() - 1;
        if self.outlet_boundary == "constant_pressure" {
            sections[last].pressure = self.outlet_pressure;
        } else if self.outlet_boundary == "constant_flow" {
            let u_outlet =
                self.outlet_mass_flow / (sections[last].mixture_density * sections[last].area);
            sections[last].gas_velocity = u_outlet;
            sections[last].liquid_velocity = u_outlet;
        } else {
            sections[last].gas_velocity = 0.0;
            sections[last].liquid_velocity = 0.0;
        }
    }

    fn time_step(&self) -> f64 {
        calculate_time_step(&self.sections, self.cfl, self.dx)
    }

    fn update_boundary_pressures(&mut self) {
        let sections = &mut self.sections;
        if self.inlet_boundary == "constant_flow" && sections.len() > 1 {
            let gravity =
                sections[0].mixture_density * GRAVITY * sections[0].inclination.sin() * self.dx;
            sections[0].pressure = sections[1].pressure + gravity;
        }
        let last = sections.len() - 1;
        if self.outlet_boundary == "constant_flow" && sections.len() > 1 {
            let gravity = sections[last].mixture_density
                * GRAVITY
                * sections[last].inclination.sin()
                * self.dx;
            sections[last].pressure = sections[last - 1].pressure - gravity;
        }
    }

    fn update_thermodynamics(&mut self) -> Result<()> {
        for section in &mut self.sections {
            let state = fluid_state(
                self.mixture,
                self.ideal_gas,
                self.names,
                section.temperature,
                section.pressure,
                &self.overall_z,
                self.srk_shift,
            );
            // The class's per-cell try/catch keeps the previous properties when a flash
            // fails. A flash that *succeeds* into three phases is not a failure: the
            // class's three-phase averaging is the branch this model refuses, so it is
            // reported rather than swallowed as a stale cell.
            let state = match state {
                Ok(state) => state,
                Err(error) if is_three_phase(&error) => return Err(error),
                Err(_) => continue,
            };
            if let Some(gas) = &state.gas {
                section.gas_density = gas.density;
                section.gas_viscosity = gas.viscosity;
                section.gas_enthalpy = gas.enthalpy;
                section.gas_sound_speed = gas.sound_speed;
            }
            if let Some(liquid) = &state.liquid {
                section.liquid_density = liquid.density;
                section.liquid_viscosity = liquid.viscosity;
                section.liquid_enthalpy = liquid.enthalpy;
                section.liquid_sound_speed = liquid.sound_speed;
            }
            section.surface_tension = if state.surface_tension > 1e-6 {
                state.surface_tension
            } else if state
                .liquid
                .as_ref()
                .is_some_and(|liquid| liquid.kind == PhaseLabel::Aqueous)
            {
                DEFAULT_WATER_SIGMA
            } else {
                DEFAULT_SIGMA
            };
        }
        Ok(())
    }

    fn store_history(&mut self) {
        if self.history_index < self.history.len() {
            for (index, section) in self.sections.iter().enumerate() {
                self.history[self.history_index][index] = section.pressure;
            }
            self.history_index += 1;
        }
    }

    fn check_convergence(&mut self) {
        self.mass_inventory = self
            .sections
            .iter()
            .map(|section| section.mixture_density * section.area * self.dx)
            .sum();
    }

    fn run(&mut self, z: &[f64]) -> Result<()> {
        self.overall_z = z.to_vec();
        self.store_history();
        while self.simulation_time < self.max_time {
            let dt = self.time_step();
            self.advance(dt);
            self.simulation_time += dt;
            self.total_steps += 1;
            self.current_step += 1;
            for section in &mut self.sections {
                section.flow_regime = flow_regime(section);
            }
            if self.current_step % self.thermo_interval == 0 {
                self.update_thermodynamics()?;
            }
            if self.total_steps as usize % HISTORY_INTERVAL == 0 {
                self.store_history();
            }
            self.check_convergence();
        }
        Ok(())
    }

    fn advance(&mut self, dt: f64) {
        self.apply_boundary();
        let old: Vec<Section> = self.sections.clone();
        let fluxes = self.fluxes(&old);
        let count = self.sections.len();
        for index in 0..count {
            let conserved_old = old[index].conservative();
            let mut conserved = [0.0; 4];
            for k in 0..4 {
                conserved[k] =
                    conserved_old[k] - dt / self.dx * (fluxes[index + 1][k] - fluxes[index][k]);
                if conserved[k].is_nan() || conserved[k].is_infinite() {
                    conserved[k] = conserved_old[k];
                }
            }
            add_sources(&mut conserved, &self.sections[index], dt);
            update_primitive(
                &mut self.sections[index],
                &old[index],
                &conserved,
                index,
                count,
            );
        }
        self.apply_boundary();
        self.update_boundary_pressures();
    }
}

fn add_sources(conserved: &mut [f64; 4], section: &Section, dt: f64) {
    let mut rho_m = section.mixture_density;
    if rho_m.is_nan() || rho_m <= 0.0 {
        rho_m = 100.0;
    }
    let gravity = -rho_m * GRAVITY * section.inclination.sin();
    if !gravity.is_nan() {
        conserved[2] += gravity * dt;
    }
    let params = drift_flux(section);
    let mut friction_section = section.clone();
    let total = pressure_gradient(&mut friction_section, &params);
    let mut friction = total - friction_section.gravity_pressure_gradient;
    if friction.is_nan() {
        friction = 0.0;
    } else {
        friction = jmax(-1000.0, jmin(1000.0, friction));
    }
    conserved[2] += friction * dt;
}

fn update_primitive(
    section: &mut Section,
    old: &Section,
    conserved: &[f64; 4],
    index: usize,
    count: usize,
) {
    let mass_g = jmax(conserved[0], 1e-10);
    let mass_l = jmax(conserved[1], 1e-10);
    let momentum = conserved[2];
    let mut rho_g = section.gas_density;
    let mut rho_l = section.liquid_density;
    if rho_g.is_nan() || rho_g <= 0.0 {
        rho_g = old.gas_density;
    }
    if rho_l.is_nan() || rho_l <= 0.0 {
        rho_l = old.liquid_density;
    }
    let mut alpha_g = mass_g / rho_g;
    let mut alpha_l = mass_l / rho_l;
    let mut total = alpha_g + alpha_l;
    if total > 0.0 && !total.is_nan() {
        alpha_g /= total;
        alpha_l /= total;
    } else {
        alpha_g = old.gas_holdup;
        alpha_l = old.liquid_holdup;
    }
    alpha_g = jmax(0.0001, jmin(0.9999, alpha_g));
    alpha_l = jmax(0.0001, jmin(0.9999, alpha_l));
    total = alpha_g + alpha_l;
    alpha_g /= total;
    alpha_l /= total;
    section.gas_holdup = alpha_g;
    section.liquid_holdup = alpha_l;
    let rho_m = mass_g + mass_l;
    let mut u_m = momentum / jmax(rho_m, 1e-10);
    if u_m.is_nan() || u_m.abs() > 200.0 {
        u_m = old.mixture_velocity;
    }
    u_m = jmax(-200.0, jmin(200.0, u_m));
    if alpha_l < 0.001 {
        section.gas_velocity = u_m;
        section.liquid_velocity = 0.0;
    } else if alpha_g < 0.001 {
        section.gas_velocity = 0.0;
        section.liquid_velocity = u_m;
    } else {
        section.gas_velocity = u_m;
        section.liquid_velocity = u_m;
        section.update_derived();
        let params = drift_flux(section);
        let mut c0 = params.c0;
        let mut drift = params.drift_velocity;
        if c0.is_nan() || !(0.9..=1.5).contains(&c0) {
            c0 = 1.0;
        }
        if drift.is_nan() || drift.abs() > 10.0 {
            drift = 0.0;
        }
        let numerator = drift + c0 * rho_m * u_m / rho_l;
        let denominator = 1.0 - c0 * alpha_g + c0 * rho_g * alpha_g / rho_l;
        let mut v_g = if denominator.abs() > 1e-10 {
            numerator / denominator
        } else {
            u_m
        };
        let mut v_l = if alpha_l > 1e-10 {
            (rho_m * u_m - rho_g * alpha_g * v_g) / (rho_l * alpha_l)
        } else {
            u_m
        };
        if v_g.is_nan() || v_g.abs() > 300.0 {
            v_g = u_m;
        }
        if v_l.is_nan() || v_l.abs() > 300.0 {
            v_l = u_m;
        }
        section.gas_velocity = v_g;
        section.liquid_velocity = v_l;
    }
    section.temperature = old.temperature;
    if index != 0 && index != count - 1 {
        let rho_old = old.mixture_density;
        let rho_new = mass_g + mass_l;
        let mut c_mix = section.wallis_sound_speed();
        if c_mix.is_nan() || !(10.0..=1000.0).contains(&c_mix) {
            c_mix = 200.0;
        }
        let mut d_rho = rho_new - rho_old;
        let cap = 0.05 * rho_old;
        d_rho = jmax(-cap, jmin(cap, d_rho));
        let mut d_p = c_mix * c_mix * d_rho;
        let mut old_p = old.pressure;
        if old_p.is_nan() || old_p <= 0.0 {
            old_p = 1e6;
        }
        let cap = 0.05 * old_p;
        d_p = jmax(-cap, jmin(cap, d_p));
        section.pressure = jmax(1e5, jmin(5e7, old_p + d_p));
    } else {
        section.pressure = old.pressure;
    }
    section.update_derived();
}

/// Whether a flash error is the model's three-phase refusal rather than a failed flash.
fn is_three_phase(error: &AzothError) -> bool {
    matches!(
        error,
        AzothError::InvalidInput { field, reason }
            if field == "components" && reason == THREE_PHASE_REASON
    )
}

fn refuse_low_points(elevation: &[f64], inclinations: &[f64]) -> Result<()> {
    for index in 1..elevation.len().saturating_sub(1) {
        if elevation[index] < elevation[index - 1] && elevation[index] <= elevation[index + 1] {
            return Err(AzothError::invalid_input(
                "elevation_profile",
                "a local minimum is an accumulation zone, and the class's liquid-accumulation \
                 tracker has no counterpart in this model",
            ));
        }
    }
    for index in 1..inclinations.len() {
        if inclinations[index - 1] < 5.0_f64.to_radians()
            && inclinations[index] > 30.0_f64.to_radians()
        {
            return Err(AzothError::invalid_input(
                "inclination_profile",
                "a riser base is an accumulation zone, and the class's liquid-accumulation \
                 tracker has no counterpart in this model",
            ));
        }
    }
    Ok(())
}

/// NeqSim `TransientPipe.run`: the isothermal drift-flux march.
///
/// # Errors
/// * [`AzothError::OutOfRange`] if an input is outside the spec's hard range.
/// * [`AzothError::InvalidInput`] for a three-phase flash, a low-point or riser-base
///   elevation profile, a boundary outside the three, or a profile of the wrong length.
#[allow(clippy::too_many_arguments)]
pub fn transient_pipe(
    components: &[String],
    inlet_z: &[f64],
    inlet_mass_flow: MassRate,
    inlet_p: Pressure,
    inlet_t: ThermodynamicTemperature,
    length: Length,
    diameter: Length,
    number_of_sections: usize,
    roughness: Length,
    inlet_boundary: &str,
    outlet_boundary: &str,
    elevation_profile: Option<&[f64]>,
    inclination_profile: Option<&[f64]>,
    outlet_pressure: Option<Pressure>,
    max_simulation_time: Option<Time>,
    cfl: Option<f64>,
    thermodynamic_update_interval: Option<usize>,
    eos: Option<&str>,
) -> Result<TransientPipeResult> {
    let spec = &model_gen::TRANSIENT_PIPE_SPEC;
    let mut warnings = Vec::new();
    if number_of_sections == 0 {
        return Err(AzothError::invalid_input(
            "number_of_sections",
            "a line needs at least one cell",
        ));
    }
    let supported = ["constant_flow", "constant_pressure", "closed"];
    if !supported.contains(&inlet_boundary) {
        return Err(AzothError::invalid_input(
            "inlet_boundary",
            format!("`{inlet_boundary}` is not one of {supported:?}"),
        ));
    }
    if !supported.contains(&outlet_boundary) {
        return Err(AzothError::invalid_input(
            "outlet_boundary",
            format!("`{outlet_boundary}` is not one of {supported:?}"),
        ));
    }
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "inlet_mass_flow" => Some(inlet_mass_flow.value),
            "inlet_p" => Some(inlet_p.value),
            "inlet_t" => Some(inlet_t.value),
            "length" => Some(length.value),
            "diameter" => Some(diameter.value),
            "number_of_sections" => Some(number_of_sections as f64),
            "roughness" => Some(roughness.value),
            "outlet_pressure" => outlet_pressure.map(|value| value.value),
            "max_simulation_time" => max_simulation_time.map(|value| value.value),
            "cfl" => cfl,
            "thermodynamic_update_interval" => {
                thermodynamic_update_interval.map(|value| value as f64)
            }
            _ => None,
        },
        &mut warnings,
    )?;

    let cubic = match eos.unwrap_or("srk") {
        "srk" => Cubic::Srk,
        "pr" => Cubic::Pr,
        other => {
            return Err(AzothError::invalid_input(
                "eos",
                format!("`{other}` is not one of `pr` or `srk`"),
            ));
        }
    };
    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, cubic, None)?;
    let srk_shift = matches!(cubic, Cubic::Srk);

    if let Some(profile) = elevation_profile {
        if profile.len() < number_of_sections {
            return Err(AzothError::invalid_input(
                "elevation_profile",
                "the profile needs one elevation per section",
            ));
        }
    } else if let Some(profile) = inclination_profile {
        if profile.len() < number_of_sections {
            return Err(AzothError::invalid_input(
                "inclination_profile",
                "the profile needs one inclination per section",
            ));
        }
    }

    let dx = length.value / number_of_sections as f64;
    let area = std::f64::consts::PI * diameter.value * diameter.value / 4.0;
    let mut sections: Vec<Section> = (0..number_of_sections)
        .map(|index| Section {
            position: (index as f64 + 0.5) * dx,
            length: dx,
            diameter: diameter.value,
            area,
            inclination: 0.0,
            elevation: 0.0,
            roughness: roughness.value,
            pressure: 0.0,
            temperature: 0.0,
            gas_holdup: 0.0,
            liquid_holdup: 0.0,
            gas_velocity: 0.0,
            liquid_velocity: 0.0,
            gas_density: 0.0,
            liquid_density: 0.0,
            gas_viscosity: 0.0,
            liquid_viscosity: 0.0,
            gas_sound_speed: 0.0,
            liquid_sound_speed: 0.0,
            surface_tension: 0.0,
            gas_enthalpy: 0.0,
            liquid_enthalpy: 0.0,
            flow_regime: Regime::StratifiedSmooth,
            mixture_velocity: 0.0,
            mixture_density: 0.0,
            superficial_gas_velocity: 0.0,
            superficial_liquid_velocity: 0.0,
            gravity_pressure_gradient: 0.0,
            friction_pressure_gradient: 0.0,
        })
        .collect();
    let mut inclinations = vec![0.0; number_of_sections];
    let mut elevations = vec![0.0; number_of_sections];
    if let Some(profile) = elevation_profile {
        for index in 0..number_of_sections {
            elevations[index] = profile[index];
            if index > 0 {
                let dz = profile[index] - profile[index - 1];
                inclinations[index] = dz.atan2(dx);
            }
        }
    } else if let Some(profile) = inclination_profile {
        let mut elevation = 0.0;
        for index in 0..number_of_sections {
            inclinations[index] = profile[index];
            elevations[index] = elevation;
            elevation += dx * profile[index].sin();
        }
    }
    refuse_low_points(&elevations, &inclinations)?;

    let fluid = fluid_state(
        &mixture,
        &ideal_gas,
        components,
        inlet_t.value,
        inlet_p.value,
        inlet_z,
        srk_shift,
    )?;
    let (rho_g, mu_g, h_g, c_g) = fluid
        .gas
        .as_ref()
        .map(|gas| (gas.density, gas.viscosity, gas.enthalpy, gas.sound_speed))
        .unwrap_or(GAS_FALLBACK);
    let (rho_l, mu_l, h_l, c_l) = fluid
        .liquid
        .as_ref()
        .map(|liquid| {
            (
                liquid.density,
                liquid.viscosity,
                liquid.enthalpy,
                liquid.sound_speed,
            )
        })
        .unwrap_or(LIQUID_FALLBACK);
    let sigma = fluid.surface_tension;
    let gas_beta = fluid.gas_beta;
    let u_sg = inlet_mass_flow.value * gas_beta / (rho_g * area);
    let u_sl = inlet_mass_flow.value * (1.0 - gas_beta) / (rho_l * area);

    let total_elevation = elevations.last().copied().unwrap_or(0.0) - elevations[0];
    let alpha_l_init = u_sl / (u_sl + u_sg + 1e-10);
    let alpha_g_init = 1.0 - alpha_l_init;
    let mut rho_m = alpha_l_init * rho_l + alpha_g_init * rho_g;
    if rho_m < 1.0 {
        rho_m = rho_g;
    }
    let hydrostatic = rho_m * GRAVITY * total_elevation;
    let mut mu_m = alpha_l_init * mu_l + alpha_g_init * mu_g;
    if mu_m <= 0.0 {
        mu_m = if mu_g > 0.0 { mu_g } else { 1e-5 };
    }
    let u_m = u_sl + u_sg;
    let re = rho_m * u_m * diameter.value / mu_m;
    let f = if re > 2300.0 {
        0.316 * re.powf(-0.25)
    } else {
        64.0 / jmax(re, 1.0)
    };
    let friction = f * rho_m * u_m * u_m * length.value / (2.0 * diameter.value);
    let outlet_p = outlet_pressure
        .map(|value| value.value)
        .unwrap_or(inlet_p.value - hydrostatic - friction);
    let outlet_p = jmax(outlet_p, 1e5);

    for (index, section) in sections.iter_mut().enumerate() {
        let fraction = if number_of_sections > 1 {
            index as f64 / (number_of_sections - 1) as f64
        } else {
            0.0
        };
        section.pressure = inlet_p.value * (1.0 - fraction) + outlet_p * fraction;
        section.temperature = inlet_t.value;
        section.inclination = inclinations[index];
        section.elevation = elevations[index];
        section.gas_density = rho_g * section.pressure / inlet_p.value;
        section.liquid_density = rho_l;
        section.gas_viscosity = mu_g;
        section.liquid_viscosity = mu_l;
        section.surface_tension = sigma;
        section.gas_enthalpy = h_g;
        section.liquid_enthalpy = h_l;
        section.gas_sound_speed = c_g;
        section.liquid_sound_speed = c_l;
        section.liquid_holdup = alpha_l_init;
        section.gas_holdup = alpha_g_init;
        section.liquid_velocity = u_sl / jmax(alpha_l_init, 0.01);
        section.gas_velocity = u_sg / jmax(1.0 - alpha_l_init, 0.01);
        section.update_derived();
        section.flow_regime = flow_regime(section);
    }

    let max_time = max_simulation_time
        .map(|value| value.value)
        .unwrap_or(DEFAULT_MAX_TIME);
    let cfl_value = cfl.unwrap_or(DEFAULT_CFL);
    let thermo_interval = thermodynamic_update_interval.unwrap_or(DEFAULT_THERMO_INTERVAL);
    let mut pipe = Pipe::new(
        &mixture,
        &ideal_gas,
        components,
        fluid,
        sections,
        inlet_mass_flow.value,
        inlet_p.value,
        outlet_p,
        inlet_t.value,
        cfl_value,
        max_time,
        thermo_interval,
        srk_shift,
        inlet_boundary,
        outlet_boundary,
    );
    pipe.run(inlet_z)?;

    let pressure_profile = pipe
        .sections
        .iter()
        .map(|section| section.pressure)
        .collect();
    let temperature_profile = pipe
        .sections
        .iter()
        .map(|section| section.temperature)
        .collect();
    let liquid_holdup_profile = pipe
        .sections
        .iter()
        .map(|section| section.liquid_holdup)
        .collect();
    let gas_velocity_profile = pipe
        .sections
        .iter()
        .map(|section| section.gas_velocity)
        .collect();
    let liquid_velocity_profile = pipe
        .sections
        .iter()
        .map(|section| section.liquid_velocity)
        .collect();
    Ok(TransientPipeResult {
        pressure_profile,
        temperature_profile,
        liquid_holdup_profile,
        gas_velocity_profile,
        liquid_velocity_profile,
        pressure_history: pipe.history.clone(),
        simulation_time: seconds(pipe.simulation_time),
        total_time_steps: pipe.total_steps,
        mass_inventory: kilograms(pipe.mass_inventory),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(gas_speed: f64, liquid_speed: f64, pressure: f64) -> Section {
        Section {
            position: 0.0,
            length: 1.0,
            diameter: 0.1,
            area: std::f64::consts::PI * 0.01 / 4.0,
            inclination: 0.0,
            elevation: 0.0,
            roughness: 1e-4,
            pressure,
            temperature: 300.0,
            gas_holdup: 1.0,
            liquid_holdup: 0.0,
            gas_velocity: gas_speed,
            liquid_velocity: liquid_speed,
            gas_density: 46.8,
            liquid_density: 500.0,
            gas_viscosity: 1.2e-5,
            liquid_viscosity: 1e-4,
            gas_sound_speed: 340.0,
            liquid_sound_speed: 600.0,
            surface_tension: 0.02,
            gas_enthalpy: 0.0,
            liquid_enthalpy: 0.0,
            flow_regime: Regime::SinglePhaseGas,
            mixture_velocity: gas_speed,
            mixture_density: 46.8,
            superficial_gas_velocity: gas_speed,
            superficial_liquid_velocity: 0.0,
            gravity_pressure_gradient: 0.0,
            friction_pressure_gradient: 0.0,
        }
    }

    #[test]
    fn the_cfl_step_is_the_class_rule() {
        let step = calculate_time_step(&[section(0.0, 0.0, 5e6)], 0.5, 20.0);
        assert!((step - 10.0 / 340.0).abs() < 1e-15, "{step}");
    }

    #[test]
    fn a_section_with_a_nan_state_falls_to_the_minimum_step() {
        let step = calculate_time_step(&[section(0.0, 0.0, f64::NAN)], 0.5, 20.0);
        assert_eq!(step, MIN_TIME_STEP);
    }

    #[test]
    fn the_step_is_clamped_at_both_ends() {
        let slow = calculate_time_step(&[section(0.0, 0.0, 5e6)], 0.5, 100_000.0);
        assert_eq!(slow, MAX_TIME_STEP);
        let fast = calculate_time_step(&[section(500.0, 0.0, 5e6)], 0.5, 1e-9);
        assert_eq!(fast, MIN_TIME_STEP);
    }
}
