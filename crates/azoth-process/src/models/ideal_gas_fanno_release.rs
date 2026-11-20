//! `process.ideal_gas_fanno_release` - NeqSim's ideal-gas Fanno pipe release.
//!
//! Spec: `specs/models/process/ideal_gas_fanno_release.toml`. The port source is
//! `process/safety/release/IdealGasFannoPipeReleaseModel.java`, through its `calculate`.
//!
//! A quasi-steady adiabatic constant-area pipe of a calorically perfect gas: the Fanno
//! function and two 120-iteration bisections locate the exit Mach, and the class's `1/M`
//! stagnation-pressure ratio is kept.

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
use azoth_eos::mixture::RootSide;
use azoth_eos::molar_enthalpy_entropy::molar_enthalpy_entropy;
use azoth_eos::phase_boundary::check_composition;
use azoth_eos::pr_molar_volume::MOLAR_GAS_CONSTANT;
use azoth_eos::pt_flash::pt_flash;
use azoth_eos::results::Phase;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::hem_release::{
    HemReleaseDiagnostic, ReleasePhase, ReleaseStation, component_mass_fractions,
};

/// The smallest solved Mach.
const MIN_MACH: f64 = 1.0e-10;
/// The bisection cap.
const BISECTION_ITERATIONS: usize = 120;
/// The mass-flux closure.
const FLUX_CLOSURE: f64 = 1.0e-9;
/// A hydrate former's name list.
const HYDRATE_FORMERS: [&str; 8] = [
    "methane", "ethane", "propane", "i-butane", "n-butane", "CO2", "H2S", "nitrogen",
];

/// One row of `stations`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IdealGasFannoReleaseStation {
    /// Which station this row is.
    pub station: ReleaseStation,
    /// Absolute pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// Temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// Ideal-gas density.
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

/// Result of `process.ideal_gas_fanno_release`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IdealGasFannoReleaseResult {
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
    pub stations: Vec<IdealGasFannoReleaseStation>,
    /// The model's assumption and check records.
    pub diagnostics: Vec<HemReleaseDiagnostic>,
    /// The diagnostics' machine codes alone.
    pub diagnostic_codes: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One isentropic station, in SI magnitudes.
#[derive(Debug, Clone)]
struct Station {
    station: ReleaseStation,
    pressure: f64,
    temperature: f64,
    density: f64,
    molar_enthalpy: f64,
    molar_entropy: f64,
    velocity: f64,
}

/// One solved Fanno state.
struct FlowState {
    pressure: f64,
    stagnation_pressure: f64,
    station: Station,
}

/// NeqSim's ideal-gas Fanno pipe release.
///
/// # Errors
/// A non-positive input, a flow path that is not both positive or both zero, a composition
/// that is not one entry per component, or a mixture the databank cannot resolve.
#[allow(clippy::too_many_arguments)] // one argument per spec input
pub fn ideal_gas_fanno_release(
    components: &[String],
    upstream_z: &[f64],
    upstream_t: ThermodynamicTemperature,
    upstream_p: Pressure,
    diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
    flow_path_length: f64,
    darcy_friction_factor: f64,
) -> Result<IdealGasFannoReleaseResult> {
    let spec = &model_gen::IDEAL_GAS_FANNO_RELEASE_SPEC;
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
            "UNSUPPORTED_FANNO_REGIME",
            "Fanno model requires positive pipe length and Darcy friction".to_string(),
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

    let mut diagnostics = vec![
        HemReleaseDiagnostic {
            code: "FANNO_MODEL_ASSUMPTIONS".to_string(),
            message: "Steady adiabatic constant-area calorically perfect gas with specified \
                      constant Darcy friction"
                .to_string(),
        },
        HemReleaseDiagnostic {
            code: "QUASI_STEADY_ONE_SIDED_PIPE".to_string(),
            message: "One pipe segment from stagnation boundary to release plane; transient \
                      waves and the opposite rupture side are excluded"
                .to_string(),
        },
    ];
    let mixture_names: Vec<String> = mixture
        .names()
        .map_or_else(|| components.to_vec(), <[String]>::to_vec);
    if let Err((code, message)) = solid_gate(&mixture, &mixture_names, upstream_t.value, upstream_z)
    {
        return Ok(failure("invalid", code, message, warnings));
    }
    diagnostics.push(HemReleaseDiagnostic {
        code: "SOLID_RISK_ASSESSED".to_string(),
        message: "UPSTREAM_STAGNATION: Mixture-specific solid and hydrate applicability checks \
                  passed"
            .to_string(),
    });

    let t0 = upstream_t.value;
    let p0 = upstream_p.value;
    let flash = match pt_flash(&mixture, upstream_t, upstream_p, upstream_z) {
        Ok(flash) => flash,
        Err(error) => {
            return Ok(failure(
                "invalid",
                "FANNO_CALCULATION_FAILED",
                format!("{error}"),
                warnings,
            ));
        }
    };
    if !matches!(flash.phase, Phase::AllVapour) {
        return Ok(failure(
            "unsupported",
            "UNSUPPORTED_FANNO_REGIME",
            "The Fanno pipe model requires one gas phase".to_string(),
            warnings,
        ));
    }
    let reduced = match mixture.reduced_parameters(upstream_t, upstream_p) {
        Ok(reduced) => reduced,
        Err(error) => {
            return Ok(failure(
                "invalid",
                "FANNO_CALCULATION_FAILED",
                format!("{error}"),
                warnings,
            ));
        }
    };
    let root = match mixture.phase_state(&reduced, upstream_z, RootSide::Vapour) {
        Ok(state) => state.z,
        Err(error) => {
            return Ok(failure(
                "invalid",
                "FANNO_CALCULATION_FAILED",
                format!("{error}"),
                warnings,
            ));
        }
    };
    let capacity = match phase_capacity(
        &mixture, &ideal_gas, &reduced, upstream_t, upstream_p, upstream_z, root,
    ) {
        Ok(capacity) => capacity,
        Err(error) => {
            return Ok(failure(
                "invalid",
                "FANNO_CALCULATION_FAILED",
                format!("{error}"),
                warnings,
            ));
        }
    };
    let gamma = capacity.cp / capacity.cv;
    if !gamma.is_finite() || gamma <= 1.0 || gamma > 2.0 {
        return Ok(failure(
            "invalid",
            "FANNO_CALCULATION_FAILED",
            "Heat-capacity ratio must be in (1, 2]".to_string(),
            warnings,
        ));
    }
    let caloric = match molar_enthalpy_entropy(
        &mixture, &ideal_gas, upstream_t, upstream_p, upstream_z, root,
    ) {
        Ok(state) => state,
        Err(error) => {
            return Ok(failure(
                "invalid",
                "FANNO_CALCULATION_FAILED",
                format!("{error}"),
                warnings,
            ));
        }
    };
    let specific_gas_constant = MOLAR_GAS_CONSTANT / molar_mass;
    let density0 = p0 / (specific_gas_constant * t0);
    let stagnation = Station {
        station: ReleaseStation::UpstreamStagnation,
        pressure: p0,
        temperature: t0,
        density: density0,
        molar_enthalpy: caloric.h.value,
        molar_entropy: caloric.s.value,
        velocity: 0.0,
    };
    if p0 <= back_pressure.value {
        diagnostics.push(HemReleaseDiagnostic {
            code: "NO_FORWARD_FLOW".to_string(),
            message: "Upstream pressure is at or below receiving pressure".to_string(),
        });
        let mut throat = stagnation.clone();
        throat.station = ReleaseStation::ThroatCritical;
        let mut exit = stagnation.clone();
        exit.station = ReleaseStation::OrificeExit;
        return Ok(success(
            Some(0.0),
            false,
            None,
            None,
            None,
            upstream_z,
            mass_fractions,
            vec![row(&stagnation), row(&throat), row(&exit)],
            diagnostics,
            warnings,
        ));
    }

    let friction_length = darcy_friction_factor * flow_path_length / diameter.value;
    let inlet_mach_at_choke = match inlet_mach(1.0, friction_length, gamma) {
        Ok(mach) => mach,
        Err(message) => {
            return Ok(failure(
                "invalid",
                "FANNO_CALCULATION_FAILED",
                message,
                warnings,
            ));
        }
    };
    let critical = match state(
        p0,
        t0,
        molar_mass,
        gamma,
        caloric.h.value,
        caloric.s.value,
        inlet_mach_at_choke,
        1.0,
    ) {
        Ok(state) => state,
        Err(message) => {
            return Ok(failure(
                "invalid",
                "FANNO_CALCULATION_FAILED",
                message,
                warnings,
            ));
        }
    };
    let choked = back_pressure.value <= critical.pressure;
    let exit_mach = if choked {
        1.0
    } else {
        match unchoked_exit_mach(
            back_pressure.value,
            p0,
            t0,
            molar_mass,
            gamma,
            caloric.h.value,
            caloric.s.value,
            friction_length,
        ) {
            Ok(mach) => mach,
            Err(message) => {
                return Ok(failure(
                    "invalid",
                    "FANNO_CALCULATION_FAILED",
                    message,
                    warnings,
                ));
            }
        }
    };
    let inlet = match inlet_mach(exit_mach, friction_length, gamma) {
        Ok(mach) => mach,
        Err(message) => {
            return Ok(failure(
                "invalid",
                "FANNO_CALCULATION_FAILED",
                message,
                warnings,
            ));
        }
    };
    let exit = match state(
        p0,
        t0,
        molar_mass,
        gamma,
        caloric.h.value,
        caloric.s.value,
        inlet,
        exit_mach,
    ) {
        Ok(state) => state,
        Err(message) => {
            return Ok(failure(
                "invalid",
                "FANNO_CALCULATION_FAILED",
                message,
                warnings,
            ));
        }
    };
    let inlet_flux = mass_flux(p0, t0, gamma, specific_gas_constant, inlet);
    let relative_closure = (exit.station.density * exit.station.velocity - inlet_flux).abs()
        / 1.0_f64.max(inlet_flux.abs());
    if relative_closure > FLUX_CLOSURE {
        return Ok(failure(
            "invalid",
            "FANNO_CALCULATION_FAILED",
            format!("Fanno mass-flux closure failed: {relative_closure}"),
            warnings,
        ));
    }
    let mut exit_station = exit.station.clone();
    exit_station.station = ReleaseStation::ThroatCritical;
    let mut orifice = exit.station;
    orifice.station = ReleaseStation::OrificeExit;
    let ambient = if choked {
        match expanded(
            exit.stagnation_pressure,
            t0,
            molar_mass,
            gamma,
            caloric.h.value,
            caloric.s.value,
            back_pressure.value,
        ) {
            Ok(station) => station,
            Err(message) => {
                return Ok(failure(
                    "invalid",
                    "FANNO_CALCULATION_FAILED",
                    message,
                    warnings,
                ));
            }
        }
    } else {
        exit_station.clone()
    };
    let mut ambient = ambient;
    ambient.station = ReleaseStation::AmbientExpanded;
    let area = std::f64::consts::PI * diameter.value * diameter.value / 4.0;
    let rate = discharge_coefficient * area * orifice.density * orifice.velocity;
    let sound_speed = (gamma * specific_gas_constant * orifice.temperature).sqrt();
    diagnostics.push(HemReleaseDiagnostic {
        code: "FANNO_SOLUTION".to_string(),
        message: "Darcy friction length solved; the exit Mach is reported on the result"
            .to_string(),
    });
    Ok(success(
        Some(rate),
        choked,
        Some(sound_speed),
        Some(friction_length),
        Some((inlet, exit_mach)),
        upstream_z,
        mass_fractions,
        vec![
            row(&stagnation),
            row(&exit_station),
            row(&orifice),
            row(&ambient),
        ],
        diagnostics,
        warnings,
    ))
}

/// The Fanno function.
fn fanno_function(mach: f64, gamma: f64) -> f64 {
    let mach2 = mach * mach;
    (1.0 - mach2) / (gamma * mach2)
        + (gamma + 1.0) / (2.0 * gamma)
            * ((gamma + 1.0) * mach2 / (2.0 + (gamma - 1.0) * mach2)).ln()
}

/// The class's `1/M` stagnation-pressure ratio.
fn stagnation_pressure_ratio(mach: f64, gamma: f64) -> f64 {
    1.0 / mach
        * ((2.0 + (gamma - 1.0) * mach * mach) / (gamma + 1.0))
            .powf((gamma + 1.0) / (2.0 * (gamma - 1.0)))
}

/// One Fanno state at an inlet and exit Mach.
#[allow(clippy::too_many_arguments)] // one argument per state input
fn state(
    pressure0: f64,
    temperature0: f64,
    molar_mass: f64,
    gamma: f64,
    enthalpy0: f64,
    entropy0: f64,
    inlet_mach: f64,
    exit_mach: f64,
) -> std::result::Result<FlowState, String> {
    let pressure0_exit = pressure0 * stagnation_pressure_ratio(exit_mach, gamma)
        / stagnation_pressure_ratio(inlet_mach, gamma);
    let stagnation_factor = 1.0 + 0.5 * (gamma - 1.0) * exit_mach * exit_mach;
    let pressure = pressure0_exit / stagnation_factor.powf(gamma / (gamma - 1.0));
    let temperature = temperature0 / stagnation_factor;
    let specific_gas_constant = MOLAR_GAS_CONSTANT / molar_mass;
    let sound_speed = (gamma * specific_gas_constant * temperature).sqrt();
    let velocity = exit_mach * sound_speed;
    let density = pressure / (specific_gas_constant * temperature);
    let heat_capacity = gamma * specific_gas_constant / (gamma - 1.0);
    let entropy = entropy0 + heat_capacity * (temperature / temperature0).ln()
        - specific_gas_constant * (pressure / pressure0).ln();
    if !pressure.is_finite() || pressure <= 0.0 || !temperature.is_finite() {
        return Err("the Fanno station state is not positive".to_string());
    }
    Ok(FlowState {
        pressure,
        stagnation_pressure: pressure0_exit,
        station: Station {
            station: ReleaseStation::OrificeExit,
            pressure,
            temperature,
            density,
            molar_enthalpy: enthalpy0 - 0.5 * velocity * velocity * molar_mass,
            molar_entropy: entropy * molar_mass,
            velocity,
        },
    })
}

/// The inlet Mach for a given exit Mach and friction length.
fn inlet_mach(
    exit_mach: f64,
    friction_length: f64,
    gamma: f64,
) -> std::result::Result<f64, String> {
    let target = fanno_function(exit_mach, gamma) + friction_length;
    if !target.is_finite() || target >= fanno_function(MIN_MACH, gamma) {
        return Err("Fanno friction length exceeds the bounded Mach solver range".to_string());
    }
    let mut low = MIN_MACH;
    let mut high = exit_mach;
    for _ in 0..BISECTION_ITERATIONS {
        let middle = 0.5 * (low + high);
        if fanno_function(middle, gamma) > target {
            low = middle;
        } else {
            high = middle;
        }
    }
    Ok(0.5 * (low + high))
}

/// The exit Mach for an unchoked exit at the receiving pressure.
#[allow(clippy::too_many_arguments)] // one argument per solver input
fn unchoked_exit_mach(
    back_pressure: f64,
    pressure0: f64,
    temperature0: f64,
    molar_mass: f64,
    gamma: f64,
    enthalpy0: f64,
    entropy0: f64,
    friction_length: f64,
) -> std::result::Result<f64, String> {
    let mut low = MIN_MACH;
    let mut high = 1.0;
    for _ in 0..BISECTION_ITERATIONS {
        let middle = 0.5 * (low + high);
        let inlet = inlet_mach(middle, friction_length, gamma)?;
        let candidate = state(
            pressure0,
            temperature0,
            molar_mass,
            gamma,
            enthalpy0,
            entropy0,
            inlet,
            middle,
        )?;
        if candidate.pressure > back_pressure {
            low = middle;
        } else {
            high = middle;
        }
    }
    Ok(0.5 * (low + high))
}

/// The ideal-gas mass flux at one Mach.
fn mass_flux(pressure0: f64, temperature0: f64, gamma: f64, gas_constant: f64, mach: f64) -> f64 {
    let factor = 1.0 + 0.5 * (gamma - 1.0) * mach * mach;
    let temperature = temperature0 / factor;
    let pressure = pressure0 / factor.powf(gamma / (gamma - 1.0));
    pressure / (gas_constant * temperature) * mach * (gamma * gas_constant * temperature).sqrt()
}

/// The isentropic receiving-pressure state.
#[allow(clippy::too_many_arguments)] // one argument per state input
fn expanded(
    pressure0: f64,
    temperature0: f64,
    molar_mass: f64,
    gamma: f64,
    enthalpy0: f64,
    entropy0: f64,
    pressure: f64,
) -> std::result::Result<Station, String> {
    let pressure_ratio = pressure / pressure0;
    let temperature = temperature0 * pressure_ratio.powf((gamma - 1.0) / gamma);
    let specific_gas_constant = MOLAR_GAS_CONSTANT / molar_mass;
    let heat_capacity = gamma * specific_gas_constant / (gamma - 1.0);
    let velocity = (2.0 * heat_capacity * (temperature0 - temperature))
        .max(0.0)
        .sqrt();
    let density = pressure / (specific_gas_constant * temperature);
    Ok(Station {
        station: ReleaseStation::AmbientExpanded,
        pressure,
        temperature,
        density,
        molar_enthalpy: enthalpy0 - 0.5 * velocity * velocity * molar_mass,
        molar_entropy: entropy0,
        velocity,
    })
}

/// The conservative solid and hydrate gate: a state that would need the class's
/// `TPSolidflash` or hydrate flash is refused rather than declared clear.
fn solid_gate(
    mixture: &azoth_eos::mixture::Mixture,
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
fn row(station: &Station) -> IdealGasFannoReleaseStation {
    let density = kilograms_per_cubic_meter(station.density);
    IdealGasFannoReleaseStation {
        station: station.station,
        pressure: pascals(station.pressure),
        temperature: kelvins(station.temperature),
        density,
        molar_enthalpy: joules_per_mole(station.molar_enthalpy),
        molar_entropy: joules_per_mole_kelvin(station.molar_entropy),
        velocity: meters_per_second(station.velocity),
        mass_flux: kilograms_per_square_meter_second(station.density * station.velocity),
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
    stations: Vec<IdealGasFannoReleaseStation>,
    diagnostics: Vec<HemReleaseDiagnostic>,
    warnings: Vec<Warning>,
) -> IdealGasFannoReleaseResult {
    IdealGasFannoReleaseResult {
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
) -> IdealGasFannoReleaseResult {
    IdealGasFannoReleaseResult {
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
