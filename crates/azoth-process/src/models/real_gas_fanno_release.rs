//! `process.real_gas_fanno_release` - NeqSim's real-gas Fanno pipe release.
//!
//! Spec: `specs/models/process/real_gas_fanno_release.toml`. The port source is
//! `process/safety/release/RealGasFannoPipeReleaseModel.java`, through its `calculate` and
//! its `trace`.
//!
//! A one-sided adiabatic constant-area pipe of a single equilibrium gas, marched in ten
//! axial steps with a predictor-corrector pressure gradient and a PH flash at every energy
//! state. The flux is found by two nested bisections and closure-checked.

use azoth_core::units::{
    Length, MassDensity, MassFlux, MassRate, MolarEnergy, MolarHeatCapacity, Pressure,
    ThermodynamicTemperature, Velocity, joules_per_mole, joules_per_mole_kelvin, kelvins,
    kilograms_per_cubic_meter, kilograms_per_second, kilograms_per_square_meter_second,
    meters_per_second, pascals,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use azoth_eos::cubic::Cubic;
use azoth_eos::databank;
use azoth_eos::entropy_state::molar_mass_of;
use azoth_eos::heat_capacity::phase_capacity;
use azoth_eos::mixture::{Mixture, RootSide};
use azoth_eos::molar_enthalpy_entropy::{IdealGasModel, molar_enthalpy_entropy};
use azoth_eos::ph_flash::ph_flash;
use azoth_eos::phase_boundary::check_composition;
use azoth_eos::pr_molar_volume::MOLAR_GAS_CONSTANT;
use azoth_eos::ps_flash::ps_flash;
use azoth_eos::results::Phase;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::hem_release::{
    HemReleaseDiagnostic, ReleasePhase, ReleaseStation, component_mass_fractions,
};

/// The axial step count.
const AXIAL_STEPS: usize = 10;
/// The root-iteration count.
const ROOT_ITERATIONS: usize = 16;
/// The property-iteration count.
const PROPERTY_ITERATIONS: usize = 20;
/// The class's sonic limit for the march.
const SONIC_LIMIT: f64 = 0.9995;
/// The class's property tolerance.
const PROPERTY_TOLERANCE: f64 = 2.0e-8;
/// The class's flux closure.
const FLUX_CLOSURE: f64 = 2.0e-7;
/// A hydrate former's name list.
const HYDRATE_FORMERS: [&str; 8] = [
    "methane", "ethane", "propane", "i-butane", "n-butane", "CO2", "H2S", "nitrogen",
];

/// One row of `stations`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RealGasFannoReleaseStation {
    /// Which station this row is.
    pub station: ReleaseStation,
    /// Absolute pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// Temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// Mass over the EOS total volume.
    #[serde(serialize_with = "scalar")]
    pub density: MassDensity,
    /// Molar enthalpy.
    #[serde(serialize_with = "scalar")]
    pub molar_enthalpy: MolarEnergy,
    /// Molar entropy.
    #[serde(serialize_with = "scalar")]
    pub molar_entropy: MolarHeatCapacity,
    /// The station's axial velocity.
    #[serde(serialize_with = "scalar")]
    pub velocity: Velocity,
    /// The station's mass flux, before the discharge coefficient.
    #[serde(serialize_with = "scalar")]
    pub mass_flux: MassFlux,
    /// What the feed flash found.
    pub phase: ReleasePhase,
    /// The gas phase's share of the mass.
    pub gas_mass_fraction: f64,
    /// The gas phase's own density.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_density: Option<MassDensity>,
    /// Absent: this model admits one gas phase.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_density: Option<MassDensity>,
    /// Absent: this model resolves no slip.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_velocity: Option<Velocity>,
    /// Absent: this model resolves no slip.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_velocity: Option<Velocity>,
}

/// Result of `process.real_gas_fanno_release`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RealGasFannoReleaseResult {
    /// `valid`, `invalid` or `unsupported`.
    pub status: String,
    /// The release rate; absent when the result is not usable.
    #[serde(serialize_with = "optional_scalar")]
    pub mass_flow_rate: Option<MassRate>,
    /// Whether the exit is sonic.
    pub choked: bool,
    /// The exit sound speed.
    #[serde(serialize_with = "optional_scalar")]
    pub throat_sound_speed: Option<Velocity>,
    /// The upstream overall mole fractions.
    pub component_mole_fractions: Vec<f64>,
    /// The upstream overall mass fractions.
    pub component_mass_fractions: Vec<f64>,
    /// `f L / D`.
    pub friction_length: Option<f64>,
    /// The pipe-inlet Mach number.
    pub inlet_mach: Option<f64>,
    /// The pipe-exit Mach number.
    pub exit_mach: Option<f64>,
    /// The resolved stations, upstream first.
    pub stations: Vec<RealGasFannoReleaseStation>,
    /// The model's assumption and check records.
    pub diagnostics: Vec<HemReleaseDiagnostic>,
    /// The diagnostics' machine codes alone.
    pub diagnostic_codes: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One stationed gas state, SI magnitudes.
#[derive(Debug, Clone)]
struct GasState {
    pressure: f64,
    temperature: f64,
    density: f64,
    molar_enthalpy: f64,
    molar_entropy: f64,
    velocity: f64,
    mach: f64,
}

/// One trace.
#[derive(Debug, Clone)]
struct Trace {
    completed: bool,
    inlet: Option<GasState>,
    exit: Option<GasState>,
    failure: String,
}

/// NeqSim's real-gas Fanno pipe release.
///
/// # Errors
/// A non-positive input, a one-sided flow path, a composition that is not one entry per
/// component, or a mixture the databank cannot resolve.
#[allow(clippy::too_many_arguments)] // one argument per spec input
pub fn real_gas_fanno_release(
    components: &[String],
    upstream_z: &[f64],
    upstream_t: ThermodynamicTemperature,
    upstream_p: Pressure,
    diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
    flow_path_length: f64,
    darcy_friction_factor: f64,
) -> Result<RealGasFannoReleaseResult> {
    let spec = &model_gen::REAL_GAS_FANNO_RELEASE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "upstream_t" => Some(upstream_t.value),
            "upstream_p" => Some(upstream_p.value),
            "diameter" => Some(diameter.value),
            "discharge_coefficient" => Some(discharge_coefficient),
            "back_pressure" => Some(back_pressure.value),
            "flow_path_length" => Some(flow_path_length),
            "darcy_friction_factor" => Some(darcy_friction_factor),
            _ => None,
        },
        &mut warnings,
    )?;
    let has_length = flow_path_length > 0.0;
    let has_friction = darcy_friction_factor > 0.0;
    if has_length != has_friction {
        return Err(AzothError::invalid_input(
            "flow_path_length",
            "the pipe length and Darcy friction factor must both be zero or both be positive"
                .to_string(),
        ));
    }
    if !has_length {
        return Ok(failure(
            "unsupported",
            "UNSUPPORTED_REAL_GAS_FANNO_REGIME",
            "Real-gas Fanno model requires positive pipe length and friction".to_string(),
            warnings,
        ));
    }

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, Cubic::Pr, None)?;
    if upstream_z.len() != mixture.len() {
        return Err(AzothError::invalid_input(
            "upstream_z",
            format!(
                "a mixture of {} components needs that many mole fractions, and {upstream_z:?} \
                 has {}",
                mixture.len(),
                upstream_z.len()
            ),
        ));
    }
    check_composition(upstream_z, mixture.len(), "upstream_z")?;
    let molar_mass = molar_mass_of(&mixture, upstream_z)?;
    let mass_fractions = component_mass_fractions(&mixture, upstream_z)?;
    let mixture_names: Vec<String> = mixture
        .names()
        .map_or_else(|| components.to_vec(), <[String]>::to_vec);
    if let Err((code, message)) = solid_gate(&mixture, &mixture_names, upstream_t.value, upstream_z)
    {
        return Ok(failure("invalid", code, message, warnings));
    }

    let mut diagnostics = vec![
        HemReleaseDiagnostic {
            code: "REAL_GAS_FANNO_ASSUMPTIONS".to_string(),
            message: "Steady adiabatic constant-area single-equilibrium-gas flow with EOS \
                      properties"
                .to_string(),
        },
        HemReleaseDiagnostic {
            code: "QUASI_STEADY_ONE_SIDED_PIPE".to_string(),
            message: "One pipe segment from stagnation boundary to release plane; transient \
                      waves and the opposite rupture side are excluded"
                .to_string(),
        },
        HemReleaseDiagnostic {
            code: "SPECIFIED_DARCY_FRICTION".to_string(),
            message: "Specified Darcy friction; no roughness or Reynolds correlation inferred"
                .to_string(),
        },
        HemReleaseDiagnostic {
            code: "SOLID_RISK_ASSESSED".to_string(),
            message: "UPSTREAM_STAGNATION: Mixture-specific solid and hydrate applicability \
                      checks passed"
                .to_string(),
        },
    ];

    let t0 = upstream_t.value;
    let p0 = upstream_p.value;
    match azoth_eos::pt_flash::pt_flash(&mixture, upstream_t, upstream_p, upstream_z) {
        Ok(flash) if matches!(flash.phase, Phase::AllVapour) => {}
        Ok(_) => {
            return Ok(failure(
                "unsupported",
                "UNSUPPORTED_REAL_GAS_FANNO_REGIME",
                "The real-gas Fanno model requires one equilibrium gas phase".to_string(),
                warnings,
            ));
        }
        Err(error) => {
            return Ok(failure(
                "invalid",
                "REAL_GAS_FANNO_CALCULATION_FAILED",
                format!("{error}"),
                warnings,
            ));
        }
    }
    let upstream = match gas_state(&mixture, &ideal_gas, upstream_z, molar_mass, p0, t0, 0.0) {
        Ok(state) => state,
        Err(message) => {
            return Ok(failure(
                "unsupported",
                "UNSUPPORTED_REAL_GAS_FANNO_REGIME",
                message,
                warnings,
            ));
        }
    };
    if p0 <= back_pressure.value {
        diagnostics.push(HemReleaseDiagnostic {
            code: "NO_FORWARD_FLOW".to_string(),
            message: "Upstream pressure is at or below receiving pressure".to_string(),
        });
        let mut throat = upstream.clone();
        throat.velocity = 0.0;
        let ambient = throat.clone();
        return Ok(success(
            Some(0.0),
            false,
            None,
            None,
            None,
            upstream_z,
            mass_fractions,
            vec![
                row(&upstream, ReleaseStation::UpstreamStagnation),
                row(&throat, ReleaseStation::ThroatCritical),
                row(&ambient, ReleaseStation::OrificeExit),
            ],
            diagnostics,
            warnings,
        ));
    }

    let h0 = upstream.molar_enthalpy;
    let s0 = upstream.molar_entropy;
    let sound0 = match sound_speed_at(&mixture, &ideal_gas, upstream_z, p0, t0) {
        Ok(sound) => sound,
        Err(message) => {
            return Ok(failure(
                "invalid",
                "REAL_GAS_FANNO_CALCULATION_FAILED",
                message,
                warnings,
            ));
        }
    };
    let upper_flux = upstream.density * sound0 * 1.5;
    let mut low_flux = upper_flux * 1.0e-4;
    let mut high_flux = upper_flux;
    let step_length = flow_path_length / AXIAL_STEPS as f64;
    let trace = |flux: f64| {
        trace_pipe(
            &mixture,
            &ideal_gas,
            upstream_z,
            molar_mass,
            h0,
            s0,
            flux,
            step_length,
            darcy_friction_factor,
            diameter.value,
            p0,
        )
    };
    let low_trace = trace(low_flux);
    if !low_trace.completed {
        return Ok(failure(
            "invalid",
            "REAL_GAS_FANNO_CALCULATION_FAILED",
            format!(
                "Cannot establish the low-flux pipe solution: {}",
                low_trace.failure
            ),
            warnings,
        ));
    }
    let mut critical = low_trace;
    for _ in 0..ROOT_ITERATIONS {
        let trial_flux = 0.5 * (low_flux + high_flux);
        let trial = trace(trial_flux);
        match &trial.exit {
            Some(exit) if trial.completed && exit.mach < SONIC_LIMIT => {
                low_flux = trial_flux;
                critical = trial;
            }
            _ => high_flux = trial_flux,
        }
    }
    let critical_exit = match &critical.exit {
        Some(exit) => exit.clone(),
        None => {
            return Ok(failure(
                "invalid",
                "REAL_GAS_FANNO_CALCULATION_FAILED",
                "The critical trace has no exit state".to_string(),
                warnings,
            ));
        }
    };
    let choked = back_pressure.value <= critical_exit.pressure;
    let (accepted_flux, accepted_exit) = if choked {
        (low_flux, critical_exit)
    } else {
        let mut lower = upper_flux * 1.0e-4;
        let mut upper = low_flux;
        let mut accepted = critical_exit.clone();
        let mut accepted_flux = low_flux;
        for _ in 0..ROOT_ITERATIONS {
            let trial_flux = 0.5 * (lower + upper);
            let trial = trace(trial_flux);
            match &trial.exit {
                Some(exit) if trial.completed && exit.pressure >= back_pressure.value => {
                    lower = trial_flux;
                    accepted = exit.clone();
                    accepted_flux = trial_flux;
                }
                _ => upper = trial_flux,
            }
        }
        (accepted_flux, accepted)
    };
    let energy_error = (h0 / molar_mass
        - accepted_exit.molar_enthalpy / molar_mass
        - 0.5 * accepted_exit.velocity * accepted_exit.velocity)
        .abs();
    if energy_error > 0.2_f64.max((h0 / molar_mass).abs() * 1.0e-6) {
        return Ok(failure(
            "invalid",
            "REAL_GAS_FANNO_CALCULATION_FAILED",
            format!("Stagnation-energy closure failed: {energy_error} J/kg"),
            warnings,
        ));
    }
    let flux_error = (accepted_flux - accepted_exit.density * accepted_exit.velocity).abs()
        / 1.0_f64.max(accepted_flux.abs());
    if flux_error > FLUX_CLOSURE {
        return Ok(failure(
            "invalid",
            "REAL_GAS_FANNO_CALCULATION_FAILED",
            format!("Mass-flux closure failed: {flux_error}"),
            warnings,
        ));
    }
    let mut ambient = accepted_exit.clone();
    if choked {
        let flash = match ps_flash(
            &mixture,
            &ideal_gas,
            back_pressure,
            joules_per_mole_kelvin(s0),
            upstream_z,
        ) {
            Ok(flash) => flash,
            Err(error) => {
                return Ok(failure(
                    "invalid",
                    "REAL_GAS_FANNO_CALCULATION_FAILED",
                    format!("{error}"),
                    warnings,
                ));
            }
        };
        let temperature = flash.temperature.value;
        let enthalpy = match molar_enthalpy_entropy(
            &mixture,
            &ideal_gas,
            kelvins(temperature),
            back_pressure,
            upstream_z,
            vapour_root(&mixture, upstream_z, temperature, back_pressure.value).unwrap_or(1.0),
        ) {
            Ok(state) => state.h.value,
            Err(error) => {
                return Ok(failure(
                    "invalid",
                    "REAL_GAS_FANNO_CALCULATION_FAILED",
                    format!("{error}"),
                    warnings,
                ));
            }
        };
        let velocity = (2.0 * (h0 - enthalpy) / molar_mass).max(0.0).sqrt();
        ambient = GasState {
            pressure: back_pressure.value,
            temperature,
            density: back_pressure.value * molar_mass / (MOLAR_GAS_CONSTANT * temperature),
            molar_enthalpy: enthalpy,
            molar_entropy: s0,
            velocity,
            mach: 0.0,
        };
    }
    let sound = match sound_speed_at(
        &mixture,
        &ideal_gas,
        upstream_z,
        accepted_exit.pressure,
        accepted_exit.temperature,
    ) {
        Ok(sound) => sound,
        Err(message) => {
            return Ok(failure(
                "invalid",
                "REAL_GAS_FANNO_CALCULATION_FAILED",
                message,
                warnings,
            ));
        }
    };
    let area = std::f64::consts::PI * diameter.value * diameter.value / 4.0;
    let rate = discharge_coefficient * area * accepted_flux;
    diagnostics.push(HemReleaseDiagnostic {
        code: "REAL_GAS_FANNO_SOLUTION".to_string(),
        message: "EOS march solved; the exit Mach is reported on the result".to_string(),
    });
    let inlet_mach = critical.inlet.as_ref().map_or(0.0, |inlet| inlet.mach);
    Ok(success(
        Some(rate),
        choked,
        Some(sound),
        Some(darcy_friction_factor * flow_path_length / diameter.value),
        Some((inlet_mach, accepted_exit.mach)),
        upstream_z,
        mass_fractions,
        vec![
            row(&upstream, ReleaseStation::UpstreamStagnation),
            row(&accepted_exit, ReleaseStation::ThroatCritical),
            row(&accepted_exit, ReleaseStation::OrificeExit),
            row(&ambient, ReleaseStation::AmbientExpanded),
        ],
        diagnostics,
        warnings,
    ))
}

/// The vapour root at a state.
fn vapour_root(
    mixture: &Mixture,
    z: &[f64],
    temperature: f64,
    pressure: f64,
) -> std::result::Result<f64, String> {
    let reduced = mixture
        .reduced_parameters(kelvins(temperature), pascals(pressure))
        .map_err(|error| format!("{error}"))?;
    Ok(mixture
        .phase_state(&reduced, z, RootSide::Vapour)
        .map_err(|error| format!("{error}"))?
        .z)
}

/// One state at a stated temperature and pressure.
#[allow(clippy::too_many_arguments)] // one argument per state input
fn gas_state(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    z: &[f64],
    molar_mass: f64,
    pressure: f64,
    temperature: f64,
    mass_flux: f64,
) -> std::result::Result<GasState, String> {
    let root = vapour_root(mixture, z, temperature, pressure)?;
    let density = molar_mass / (root * MOLAR_GAS_CONSTANT * temperature / pressure);
    let caloric = molar_enthalpy_entropy(
        mixture,
        ideal_gas,
        kelvins(temperature),
        pascals(pressure),
        z,
        root,
    )
    .map_err(|error| format!("{error}"))?;
    let sound = sound_speed_at(mixture, ideal_gas, z, pressure, temperature)?;
    let velocity = mass_flux / density;
    Ok(GasState {
        pressure,
        temperature,
        density,
        molar_enthalpy: caloric.h.value,
        molar_entropy: caloric.s.value,
        velocity,
        mach: velocity / sound,
    })
}

/// One energy state by the PH flash and the density fixed point.
#[allow(clippy::too_many_arguments)] // one argument per state input
fn energy_state(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    z: &[f64],
    molar_mass: f64,
    h0: f64,
    mass_flux: f64,
    pressure: f64,
) -> std::result::Result<GasState, String> {
    if !pressure.is_finite() || pressure <= 0.0 {
        return Err("the energy-state pressure is not positive".to_string());
    }
    let mut density = molar_mass / (MOLAR_GAS_CONSTANT * 300.0 / pressure);
    let mut temperature = 300.0;
    for _ in 0..PROPERTY_ITERATIONS {
        let velocity = mass_flux / density;
        let static_enthalpy = h0 - 0.5 * velocity * velocity * molar_mass;
        let flash = ph_flash(
            mixture,
            ideal_gas,
            pascals(pressure),
            joules_per_mole(static_enthalpy),
            z,
        )
        .map_err(|error| format!("{error}"))?;
        if flash.phase != Phase::AllVapour {
            return Err("The real-gas Fanno model requires one equilibrium gas phase".to_string());
        }
        temperature = flash.temperature.value;
        let root = vapour_root(mixture, z, temperature, pressure)?;
        let updated = molar_mass / (root * MOLAR_GAS_CONSTANT * temperature / pressure);
        let converged = (updated - density).abs() <= PROPERTY_TOLERANCE * 1.0_f64.max(updated);
        density = updated;
        if converged {
            break;
        }
    }
    gas_state(
        mixture,
        ideal_gas,
        z,
        molar_mass,
        pressure,
        temperature,
        mass_flux,
    )
}

/// The inlet state at a mass flux by the entropy root.
#[allow(clippy::too_many_arguments)] // one argument per state input
fn inlet_state(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    z: &[f64],
    molar_mass: f64,
    h0: f64,
    s0: f64,
    mass_flux: f64,
    pressure0: f64,
    upstream: &GasState,
) -> std::result::Result<GasState, String> {
    let sound0 = sound_speed_at(mixture, ideal_gas, z, pressure0, upstream.temperature)?;
    if mass_flux <= upstream.density * sound0 * 1.0e-6 {
        let velocity = mass_flux / upstream.density;
        let mut state = upstream.clone();
        state.velocity = velocity;
        state.mach = velocity / sound0;
        return Ok(state);
    }
    let high = energy_state(mixture, ideal_gas, z, molar_mass, h0, mass_flux, pressure0)?;
    let high_error = high.molar_entropy - s0;
    let mut low_pressure = pressure0;
    let mut low = high.clone();
    let mut low_error = high_error;
    for index in 1..=36 {
        if low_error * high_error <= 0.0 {
            break;
        }
        low_pressure = pressure0 * (-(200.0_f64).ln() * index as f64 / 36.0).exp();
        low = energy_state(
            mixture,
            ideal_gas,
            z,
            molar_mass,
            h0,
            mass_flux,
            low_pressure,
        )?;
        low_error = low.molar_entropy - s0;
    }
    if low_error * high_error > 0.0 {
        return Err("Cannot bracket the subsonic isentropic pipe inlet".to_string());
    }
    let mut low_state = low;
    let mut high_state = high;
    let mut low_p = low_pressure;
    let mut high_p = pressure0;
    let mut low_e = low_error;
    let mut high_e = high_error;
    for _ in 0..ROOT_ITERATIONS {
        let pressure = 0.5 * (low_p + high_p);
        let middle = energy_state(mixture, ideal_gas, z, molar_mass, h0, mass_flux, pressure)?;
        let error = middle.molar_entropy - s0;
        if error * low_e <= 0.0 {
            high_p = pressure;
            high_state = middle;
            high_e = error;
        } else {
            low_p = pressure;
            low_state = middle;
            low_e = error;
        }
    }
    let result = if low_e.abs() < high_e.abs() {
        low_state
    } else {
        high_state
    };
    if result.mach >= 1.0 {
        return Err("Pipe inlet is not subsonic".to_string());
    }
    Ok(result)
}

/// The steady momentum pressure gradient at one state.
#[allow(clippy::too_many_arguments)] // one argument per gradient input
fn pressure_gradient(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    z: &[f64],
    molar_mass: f64,
    h0: f64,
    mass_flux: f64,
    friction: f64,
    diameter: f64,
    pressure0: f64,
    state: &GasState,
) -> std::result::Result<f64, String> {
    let pressure = state.pressure;
    let step = 20.0_f64.max(pressure * 2.0e-5);
    let lower = energy_state(
        mixture,
        ideal_gas,
        z,
        molar_mass,
        h0,
        mass_flux,
        pressure - step,
    )?;
    let upper = energy_state(
        mixture,
        ideal_gas,
        z,
        molar_mass,
        h0,
        mass_flux,
        (pressure + step).min(pressure0 * (1.0 - 1.0e-12)),
    )?;
    let velocity_derivative = (upper.velocity - lower.velocity) / (upper.pressure - lower.pressure);
    let denominator = 1.0 + mass_flux * velocity_derivative;
    if !denominator.is_finite() || denominator <= 2.0e-4 {
        return Err("Steady momentum equation reached its sonic singularity".to_string());
    }
    Ok(-friction * mass_flux * mass_flux / (2.0 * diameter * state.density * denominator))
}

/// The ten-step one-sided march at one mass flux.
#[allow(clippy::too_many_arguments)] // one argument per trace input
fn trace_pipe(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    z: &[f64],
    molar_mass: f64,
    h0: f64,
    s0: f64,
    mass_flux: f64,
    step_length: f64,
    friction: f64,
    diameter: f64,
    pressure0: f64,
) -> Trace {
    let upstream = match gas_state(mixture, ideal_gas, z, molar_mass, pressure0, 300.0, 0.0) {
        Ok(state) => state,
        Err(failure) => {
            return Trace {
                completed: false,
                inlet: None,
                exit: None,
                failure,
            };
        }
    };
    let inlet = match inlet_state(
        mixture, ideal_gas, z, molar_mass, h0, s0, mass_flux, pressure0, &upstream,
    ) {
        Ok(state) => state,
        Err(failure) => {
            return Trace {
                completed: false,
                inlet: None,
                exit: None,
                failure,
            };
        }
    };
    let mut state = inlet.clone();
    for _ in 0..AXIAL_STEPS {
        if state.mach >= SONIC_LIMIT {
            return Trace {
                completed: false,
                inlet: Some(inlet),
                exit: Some(state),
                failure: "sonic limit reached before the pipe exit".to_string(),
            };
        }
        let gradient = match pressure_gradient(
            mixture, ideal_gas, z, molar_mass, h0, mass_flux, friction, diameter, pressure0, &state,
        ) {
            Ok(value) => value,
            Err(failure) => {
                return Trace {
                    completed: false,
                    inlet: Some(inlet),
                    exit: Some(state),
                    failure,
                };
            }
        };
        let predicted_pressure = state.pressure + gradient * step_length;
        if !(predicted_pressure > 0.0) || predicted_pressure >= state.pressure {
            return Trace {
                completed: false,
                inlet: Some(inlet),
                exit: Some(state),
                failure: "pressure step did not decrease pressure".to_string(),
            };
        }
        let predicted = match energy_state(
            mixture,
            ideal_gas,
            z,
            molar_mass,
            h0,
            mass_flux,
            predicted_pressure,
        ) {
            Ok(value) => value,
            Err(failure) => {
                return Trace {
                    completed: false,
                    inlet: Some(inlet),
                    exit: Some(state),
                    failure,
                };
            }
        };
        if predicted.mach >= 1.0 {
            return Trace {
                completed: false,
                inlet: Some(inlet),
                exit: Some(predicted),
                failure: "predictor crossed the sonic limit".to_string(),
            };
        }
        let corrected_gradient = match pressure_gradient(
            mixture, ideal_gas, z, molar_mass, h0, mass_flux, friction, diameter, pressure0,
            &predicted,
        ) {
            Ok(value) => 0.5 * (gradient + value),
            Err(failure) => {
                return Trace {
                    completed: false,
                    inlet: Some(inlet),
                    exit: Some(predicted),
                    failure,
                };
            }
        };
        let corrected_pressure = state.pressure + corrected_gradient * step_length;
        if !(corrected_pressure > 0.0) || corrected_pressure >= state.pressure {
            return Trace {
                completed: false,
                inlet: Some(inlet),
                exit: Some(state),
                failure: "corrected pressure step did not decrease pressure".to_string(),
            };
        }
        state = match energy_state(
            mixture,
            ideal_gas,
            z,
            molar_mass,
            h0,
            mass_flux,
            corrected_pressure,
        ) {
            Ok(value) => value,
            Err(failure) => {
                return Trace {
                    completed: false,
                    inlet: Some(inlet),
                    exit: Some(state),
                    failure,
                };
            }
        };
    }
    let completed = state.mach < 1.0;
    Trace {
        completed,
        inlet: Some(inlet),
        exit: Some(state),
        failure: if completed {
            String::new()
        } else {
            "exit is sonic".to_string()
        },
    }
}

/// One phase's isentropic speed at a gas state.
fn sound_speed_at(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    z: &[f64],
    pressure: f64,
    temperature: f64,
) -> std::result::Result<f64, String> {
    let reduced = mixture
        .reduced_parameters(kelvins(temperature), pascals(pressure))
        .map_err(|error| format!("{error}"))?;
    let root = vapour_root(mixture, z, temperature, pressure)?;
    let capacity = phase_capacity(
        mixture,
        ideal_gas,
        &reduced,
        kelvins(temperature),
        pascals(pressure),
        z,
        root,
    )
    .map_err(|error| format!("{error}"))?;
    let molar_mass = molar_mass_of(mixture, z).map_err(|error| format!("{error}"))?;
    let volume = root * MOLAR_GAS_CONSTANT * temperature / pressure;
    let gamma = capacity.cp / capacity.cv;
    Ok(volume * (-gamma * capacity.d_p_d_v / molar_mass).sqrt())
}

/// The conservative solid and hydrate gate.
fn solid_gate(
    mixture: &Mixture,
    names: &[String],
    temperature: f64,
    z: &[f64],
) -> std::result::Result<(), (&'static str, String)> {
    for (index, (component, fraction)) in mixture.components().iter().zip(z).enumerate() {
        if *fraction <= 1.0e-12 {
            continue;
        }
        let triple = component.triple_point_temperature;
        if triple.is_finite() && triple > 0.0 && triple < 1000.0 && temperature <= triple {
            return Err((
                "SOLID_RISK_ASSESSMENT_FAILED",
                format!(
                    "component {index} is at or below its triple point, and this port does not \
                     reproduce the per-component solid flash"
                ),
            ));
        }
    }
    let water = names
        .iter()
        .position(|name| name == "water")
        .map(|index| z[index])
        .unwrap_or(0.0);
    if water > 1.0e-12 {
        for former in HYDRATE_FORMERS {
            if let Some(index) = names.iter().position(|name| name == former)
                && z[index] > 1.0e-12
            {
                return Err((
                    "SOLID_RISK_ASSESSMENT_FAILED",
                    "a water-bearing hydrate-forming state is not reproduced by this port"
                        .to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// One published station row.
fn row(state: &GasState, station: ReleaseStation) -> RealGasFannoReleaseStation {
    let density = kilograms_per_cubic_meter(state.density);
    RealGasFannoReleaseStation {
        station,
        pressure: pascals(state.pressure),
        temperature: kelvins(state.temperature),
        density,
        molar_enthalpy: joules_per_mole(state.molar_enthalpy),
        molar_entropy: joules_per_mole_kelvin(state.molar_entropy),
        velocity: meters_per_second(state.velocity),
        mass_flux: kilograms_per_square_meter_second(state.density * state.velocity),
        phase: ReleasePhase::AllVapour,
        gas_mass_fraction: 1.0,
        gas_density: Some(density),
        liquid_density: None,
        gas_velocity: None,
        liquid_velocity: None,
    }
}

/// The class's success result.
#[allow(clippy::too_many_arguments)] // one argument per result field
fn success(
    flow: Option<f64>,
    choked: bool,
    sound_speed: Option<f64>,
    friction_length: Option<f64>,
    machs: Option<(f64, f64)>,
    z: &[f64],
    mass_fractions: Vec<f64>,
    stations: Vec<RealGasFannoReleaseStation>,
    diagnostics: Vec<HemReleaseDiagnostic>,
    warnings: Vec<Warning>,
) -> RealGasFannoReleaseResult {
    RealGasFannoReleaseResult {
        status: "valid".to_string(),
        mass_flow_rate: flow.map(kilograms_per_second),
        choked,
        throat_sound_speed: sound_speed.map(meters_per_second),
        component_mole_fractions: z.to_vec(),
        component_mass_fractions: mass_fractions,
        friction_length,
        inlet_mach: machs.map(|(inlet, _)| inlet),
        exit_mach: machs.map(|(_, exit)| exit),
        stations,
        diagnostic_codes: diagnostics.iter().map(|row| row.code.clone()).collect(),
        diagnostics,
        warnings,
    }
}

/// The class's failure result: no rate, no stations, one diagnostic.
fn failure(
    status: &str,
    code: &str,
    message: String,
    warnings: Vec<Warning>,
) -> RealGasFannoReleaseResult {
    RealGasFannoReleaseResult {
        status: status.to_string(),
        mass_flow_rate: None,
        choked: false,
        throat_sound_speed: None,
        component_mole_fractions: Vec::new(),
        component_mass_fractions: Vec::new(),
        friction_length: None,
        inlet_mach: None,
        exit_mach: None,
        stations: Vec::new(),
        diagnostics: vec![HemReleaseDiagnostic {
            code: code.to_string(),
            message,
        }],
        diagnostic_codes: vec![code.to_string()],
        warnings,
    }
}
