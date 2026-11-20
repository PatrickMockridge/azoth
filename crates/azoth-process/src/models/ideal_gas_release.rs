//! `process.ideal_gas_release` - NeqSim's ideal-gas isentropic orifice release.
//!
//! Spec: `specs/models/process/ideal_gas_release.toml`. The port source is
//! `process/safety/release/IdealGasReleaseModel.java`, through its `calculate`.
//!
//! The class is the calorically perfect companion to `process.hem_release`: one gas phase,
//! a constant heat-capacity ratio, and an isentropic expansion from the stagnation state to
//! the critical ratio or the receiving pressure. The upstream density is ideal-gas while the
//! stagnation enthalpy and entropy are the EOS flash's.

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

/// One row of `stations`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IdealGasReleaseStation {
    /// Which station this row is.
    pub station: ReleaseStation,
    /// Absolute pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// Temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// Ideal-gas density, `P M/(R T)`.
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
    /// The liquid phase's density; absent, because this model admits one gas phase.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_density: Option<MassDensity>,
    /// The gas phase's velocity; absent, because this model resolves no slip.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_velocity: Option<Velocity>,
    /// The liquid phase's velocity; absent, because this model resolves no slip.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_velocity: Option<Velocity>,
}

/// Result of `process.ideal_gas_release`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IdealGasReleaseResult {
    /// `valid`, `valid_with_warnings`, `invalid` or `unsupported`.
    pub status: String,
    /// The release rate; absent when the result is not usable.
    #[serde(serialize_with = "optional_scalar")]
    pub mass_flow_rate: Option<MassRate>,
    /// Whether the accepted flow is choked.
    pub choked: bool,
    /// The throat's ideal-gas sound speed.
    #[serde(serialize_with = "optional_scalar")]
    pub throat_sound_speed: Option<Velocity>,
    /// The upstream overall mole fractions.
    pub component_mole_fractions: Vec<f64>,
    /// The upstream overall mass fractions.
    pub component_mass_fractions: Vec<f64>,
    /// The resolved stations, upstream first.
    pub stations: Vec<IdealGasReleaseStation>,
    /// The model's assumption and warning records.
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

/// NeqSim's ideal-gas isentropic orifice release.
///
/// # Errors
/// A non-positive input, a composition that is not one entry per component, or a mixture the
/// databank cannot resolve.
pub fn ideal_gas_release(
    components: &[String],
    upstream_z: &[f64],
    upstream_t: ThermodynamicTemperature,
    upstream_p: Pressure,
    diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
) -> Result<IdealGasReleaseResult> {
    let spec = &model_gen::IDEAL_GAS_RELEASE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "upstream_t" => Some(upstream_t.value),
            "upstream_p" => Some(upstream_p.value),
            "diameter" => Some(diameter.value),
            "discharge_coefficient" => Some(discharge_coefficient),
            "back_pressure" => Some(back_pressure.value),
            _ => None,
        },
        &mut warnings,
    )?;

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
            code: "IDEAL_GAS_ASSUMPTIONS".to_string(),
            message: "Calorically perfect gas, constant gamma, isentropic acceleration; no \
                      real-gas departure or phase change"
                .to_string(),
        },
        HemReleaseDiagnostic {
            code: "NO_PROPERTY_FALLBACK".to_string(),
            message: "Mixture molar mass and gamma are required properties; no default is \
                      substituted"
                .to_string(),
        },
        HemReleaseDiagnostic {
            code: "EXIT_ALIASES_THROAT".to_string(),
            message: "Zero-length opening: exit equals accepted throat".to_string(),
        },
    ];

    let t0 = upstream_t.value;
    let p0 = upstream_p.value;
    let flash = match pt_flash(&mixture, upstream_t, upstream_p, upstream_z) {
        Ok(flash) => flash,
        Err(error) => {
            return Ok(failure(
                "invalid",
                "IDEAL_GAS_CALCULATION_FAILED",
                format!("{error}"),
                warnings,
            ));
        }
    };
    if !matches!(flash.phase, Phase::AllVapour) {
        return Ok(failure(
            "unsupported",
            "UNSUPPORTED_IDEAL_GAS_REGIME",
            "The ideal-gas adapter requires one gas phase".to_string(),
            warnings,
        ));
    }

    let reduced = match mixture.reduced_parameters(upstream_t, upstream_p) {
        Ok(reduced) => reduced,
        Err(error) => {
            return Ok(failure(
                "invalid",
                "IDEAL_GAS_CALCULATION_FAILED",
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
                "IDEAL_GAS_CALCULATION_FAILED",
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
                "IDEAL_GAS_CALCULATION_FAILED",
                format!("{error}"),
                warnings,
            ));
        }
    };
    let gamma = capacity.cp / capacity.cv;
    if !gamma.is_finite() || gamma <= 1.0 || gamma > 2.0 {
        return Ok(failure(
            "invalid",
            "IDEAL_GAS_CALCULATION_FAILED",
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
                "IDEAL_GAS_CALCULATION_FAILED",
                format!("{error}"),
                warnings,
            ));
        }
    };
    let density0 = p0 * molar_mass / (MOLAR_GAS_CONSTANT * t0);

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
        let stations = vec![row(&stagnation), row(&throat), row(&exit)];
        return Ok(success(
            "valid",
            Some(0.0),
            false,
            None,
            upstream_z,
            mass_fractions,
            stations,
            diagnostics,
            warnings,
        ));
    }

    let critical_ratio = (2.0 / (gamma + 1.0)).powf(gamma / (gamma - 1.0));
    let back_ratio = back_pressure.value / p0;
    let choked = back_ratio <= critical_ratio;
    let throat_pressure = if choked {
        p0 * critical_ratio
    } else {
        back_pressure.value
    };
    let mut throat = expanded(&stagnation, molar_mass, gamma, throat_pressure);
    throat.station = ReleaseStation::ThroatCritical;
    let mut exit = throat.clone();
    exit.station = ReleaseStation::OrificeExit;
    let mut ambient = if choked {
        expanded(&stagnation, molar_mass, gamma, back_pressure.value)
    } else {
        exit.clone()
    };
    ambient.station = ReleaseStation::AmbientExpanded;
    let stations = vec![row(&stagnation), row(&throat), row(&exit), row(&ambient)];
    let area = std::f64::consts::PI * diameter.value * diameter.value / 4.0;
    let mass_flow = discharge_coefficient * area * throat.density * throat.velocity;
    let sound_speed = (gamma * MOLAR_GAS_CONSTANT * throat.temperature / molar_mass).sqrt();
    Ok(success(
        "valid",
        Some(mass_flow),
        choked,
        Some(sound_speed),
        upstream_z,
        mass_fractions,
        stations,
        diagnostics,
        warnings,
    ))
}

/// One isentropic expansion from the stagnation state.
fn expanded(stagnation: &Station, molar_mass: f64, gamma: f64, pressure: f64) -> Station {
    let ratio = pressure / stagnation.pressure;
    let temperature = stagnation.temperature * ratio.powf((gamma - 1.0) / gamma);
    let specific_gas_constant = MOLAR_GAS_CONSTANT / molar_mass;
    let heat_capacity = gamma * specific_gas_constant / (gamma - 1.0);
    let velocity = (2.0 * heat_capacity * (stagnation.temperature - temperature))
        .max(0.0)
        .sqrt();
    let density = pressure * molar_mass / (MOLAR_GAS_CONSTANT * temperature);
    let enthalpy = stagnation.molar_enthalpy - 0.5 * velocity * velocity * molar_mass;
    Station {
        station: stagnation.station,
        pressure,
        temperature,
        density,
        molar_enthalpy: enthalpy,
        molar_entropy: stagnation.molar_entropy,
        velocity,
    }
}

/// One published station row.
fn row(station: &Station) -> IdealGasReleaseStation {
    let density = kilograms_per_cubic_meter(station.density);
    IdealGasReleaseStation {
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
    status: &str,
    flow: Option<f64>,
    choked: bool,
    sound_speed: Option<f64>,
    z: &[f64],
    mass_fractions: Vec<f64>,
    stations: Vec<IdealGasReleaseStation>,
    diagnostics: Vec<HemReleaseDiagnostic>,
    warnings: Vec<Warning>,
) -> IdealGasReleaseResult {
    IdealGasReleaseResult {
        status: status.to_string(),
        mass_flow_rate: flow.map(kilograms_per_second),
        choked,
        throat_sound_speed: sound_speed.map(meters_per_second),
        component_mole_fractions: z.to_vec(),
        component_mass_fractions: mass_fractions,
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
) -> IdealGasReleaseResult {
    IdealGasReleaseResult {
        status: status.to_string(),
        mass_flow_rate: None,
        choked: false,
        throat_sound_speed: None,
        component_mole_fractions: Vec::new(),
        component_mass_fractions: Vec::new(),
        stations: Vec::new(),
        diagnostics: vec![HemReleaseDiagnostic {
            code: code.to_string(),
            message,
        }],
        diagnostic_codes: vec![code.to_string()],
        warnings,
    }
}
