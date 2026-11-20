//! `process.legacy_screening_release` - NeqSim's historical orifice screening release.
//!
//! Spec: `specs/models/process/legacy_screening_release.toml`. The port source is
//! `process/safety/release/LegacyScreeningReleaseModel.java`, through its `calculate`, which
//! delegates the rate to `LeakModel.calculateMassFlowRate`.
//!
//! **This is compatibility surface, not a physical model.** The class says so in its own
//! diagnostics, and the port reproduces its fallbacks: a capacity ratio outside `(1, 2]`
//! becomes `1.3`, and the stations alias the flashed upstream state because no throat is
//! solved.

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
use azoth_eos::entropy_state::{self, EntropyPhase, EntropyState, molar_mass_of};
use azoth_eos::heat_capacity::phase_capacity;
use azoth_eos::mixture::RootSide;
use azoth_eos::phase_boundary::check_composition;
use azoth_eos::results::Phase;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::hem_release::{
    HemReleaseDiagnostic, ReleasePhase, ReleaseStation, component_mass_fractions,
};

/// The class's historical gas constant, J/(kmol K).
const R: f64 = 8314.0;
/// The fallback density's gas constant, J/(mol K), as the class writes it.
const R_MOLAR: f64 = 8.314;
/// The class's historical default heat-capacity ratio.
const DEFAULT_GAMMA: f64 = 1.3;

/// One row of `stations`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LegacyScreeningReleaseStation {
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
    /// The gas phase's own density, when the flash resolved one.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_density: Option<MassDensity>,
    /// The liquid phase's density, when the flash resolved one.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_density: Option<MassDensity>,
    /// The gas phase's velocity; absent, because the screening model resolves no slip.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_velocity: Option<Velocity>,
    /// The liquid phase's velocity; absent, because the screening model resolves no slip.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_velocity: Option<Velocity>,
}

/// Result of `process.legacy_screening_release`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LegacyScreeningReleaseResult {
    /// `valid_with_warnings` on a usable state, `invalid` or `unsupported` otherwise.
    pub status: String,
    /// The screening rate; absent when the result is not usable.
    #[serde(serialize_with = "optional_scalar")]
    pub mass_flow_rate: Option<MassRate>,
    /// Whether the screening equation's critical ratio admits the receiving pressure.
    pub choked: bool,
    /// Absent: the historical equation reports no sound speed.
    #[serde(serialize_with = "optional_scalar")]
    pub throat_sound_speed: Option<Velocity>,
    /// The upstream overall mole fractions.
    pub component_mole_fractions: Vec<f64>,
    /// The upstream overall mass fractions.
    pub component_mass_fractions: Vec<f64>,
    /// The three stations, upstream first.
    pub stations: Vec<LegacyScreeningReleaseStation>,
    /// The screening and fallback records.
    pub diagnostics: Vec<HemReleaseDiagnostic>,
    /// The diagnostics' machine codes alone.
    pub diagnostic_codes: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// NeqSim's historical orifice screening release.
///
/// # Errors
/// A non-positive input, a composition that is not one entry per component, or a mixture the
/// databank cannot resolve.
pub fn legacy_screening_release(
    components: &[String],
    upstream_z: &[f64],
    upstream_t: ThermodynamicTemperature,
    upstream_p: Pressure,
    diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
) -> Result<LegacyScreeningReleaseResult> {
    let spec = &model_gen::LEGACY_SCREENING_RELEASE_SPEC;
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
            code: "SCREENING_ONLY".to_string(),
            message: "Compatibility result from the historical screening equation; not a \
                      qualified physical release model"
                .to_string(),
        },
        HemReleaseDiagnostic {
            code: "UNRESOLVED_STATIONS".to_string(),
            message: "Legacy screening does not solve throat or exit thermodynamics; the \
                      opening states alias upstream"
                .to_string(),
        },
        HemReleaseDiagnostic {
            code: "LEGACY_FALLBACK_POLICY".to_string(),
            message: "The delegated method retains historical density, molar-mass and gamma \
                      defaults when properties fail"
                .to_string(),
        },
    ];

    let t = upstream_t.value;
    let p = upstream_p.value;
    let back = back_pressure.value;
    let mut evaluations = 0_u32;
    let upstream = match entropy_state::flashed_state(
        &mixture,
        &ideal_gas,
        upstream_z,
        molar_mass,
        p,
        t,
        &mut evaluations,
    ) {
        Ok(state) => state,
        Err(error) => {
            return Ok(failure(
                "invalid",
                "LEGACY_SCREENING_ADAPTER_FAILED",
                error.0,
                warnings,
            ));
        }
    };

    let mut gamma = DEFAULT_GAMMA;
    let mut gamma_defaulted = false;
    if let Ok(reduced) = mixture.reduced_parameters(upstream_t, upstream_p) {
        let side = if matches!(upstream.phase, Phase::AllLiquid) {
            RootSide::Liquid
        } else {
            RootSide::Vapour
        };
        if let Ok(root) = mixture.phase_state(&reduced, upstream_z, side) {
            if let Ok(capacity) = phase_capacity(
                &mixture, &ideal_gas, &reduced, upstream_t, upstream_p, upstream_z, root.z,
            ) {
                let candidate = capacity.cp / capacity.cv;
                if candidate.is_finite() && candidate > 1.0 && candidate <= 2.0 {
                    gamma = candidate;
                } else {
                    gamma_defaulted = true;
                }
            } else {
                gamma_defaulted = true;
            }
        } else {
            gamma_defaulted = true;
        }
    } else {
        gamma_defaulted = true;
    }

    // **The screening path's density is the ideal-gas fallback.** The class flashes a
    // clone without initialising its physical properties, so `getDensity` is NaN and its
    // own fallback `P MW/(8.314 T)` supplies the value. The station keeps the EOS density.
    let rate_density = p * molar_mass / (R_MOLAR * t);
    let rate = if p <= back {
        0.0
    } else {
        let area = std::f64::consts::PI * diameter.value * diameter.value / 4.0;
        screening_rate(
            p,
            t,
            rate_density,
            gamma,
            1000.0 * molar_mass,
            discharge_coefficient,
            area,
            back,
        )
    };
    if !rate.is_finite() || rate < 0.0 {
        return Ok(failure(
            "invalid",
            "LEGACY_SCREENING_ADAPTER_FAILED",
            format!("Legacy method returned an invalid release rate: {rate} kg/s"),
            warnings,
        ));
    }
    let effective_area =
        discharge_coefficient * std::f64::consts::PI * diameter.value.powi(2) / 4.0;
    let velocity = if rate <= 0.0 {
        0.0
    } else {
        rate / (upstream.density * effective_area)
    };

    if gamma_defaulted {
        diagnostics.push(HemReleaseDiagnostic {
            code: "LEGACY_GAMMA_DEFAULT".to_string(),
            message: "Historical gamma default 1.3 was used".to_string(),
        });
    }
    let critical = (2.0 / (gamma + 1.0)).powf(gamma / (gamma - 1.0));
    let mut choked = back / p <= critical;
    if rate == 0.0 {
        choked = false;
        diagnostics.push(HemReleaseDiagnostic {
            code: "NO_FORWARD_FLOW_OR_SCREENING_ZERO".to_string(),
            message: "Legacy screening returned zero; inspect upstream pressure and retained \
                      fallback behavior"
                .to_string(),
        });
    }

    let stations = vec![
        station_row(&upstream, ReleaseStation::UpstreamStagnation, 0.0),
        station_row(&upstream, ReleaseStation::ThroatCritical, velocity),
        station_row(&upstream, ReleaseStation::OrificeExit, velocity),
    ];
    Ok(success(
        Some(rate),
        choked,
        upstream_z,
        mass_fractions,
        stations,
        diagnostics,
        warnings,
    ))
}

/// The historical screening rate, kg/s.
#[allow(clippy::too_many_arguments)] // one argument per class input the equation reads
fn screening_rate(
    pressure: f64,
    temperature: f64,
    density: f64,
    gamma: f64,
    molar_mass: f64,
    coefficient: f64,
    area: f64,
    back: f64,
) -> f64 {
    let critical = (2.0 / (gamma + 1.0)).powf(gamma / (gamma - 1.0));
    let ratio = back / pressure;
    if ratio <= critical {
        let factor = (2.0 / (gamma + 1.0)).powf((gamma + 1.0) / (2.0 * (gamma - 1.0)));
        coefficient * area * pressure * (gamma * molar_mass / (R * temperature)).sqrt() * factor
    } else {
        coefficient * area * (2.0 * density * (pressure - back)).max(0.0).sqrt()
    }
}

/// One published station row.
fn station_row(
    state: &EntropyState,
    station: ReleaseStation,
    velocity: f64,
) -> LegacyScreeningReleaseStation {
    let density = kilograms_per_cubic_meter(state.density);
    LegacyScreeningReleaseStation {
        station,
        pressure: pascals(state.pressure),
        temperature: kelvins(state.temperature),
        density,
        molar_enthalpy: joules_per_mole(state.molar_enthalpy),
        molar_entropy: joules_per_mole_kelvin(state.molar_entropy),
        velocity: meters_per_second(velocity),
        mass_flux: kilograms_per_square_meter_second(state.density * velocity),
        phase: ReleasePhase::from_phase(state.phase).unwrap_or(ReleasePhase::AllVapour),
        gas_mass_fraction: state.gas_mass_fraction,
        gas_density: density_of(state.gas_phase.as_ref()),
        liquid_density: density_of(state.liquid_phase.as_ref()),
        gas_velocity: None,
        liquid_velocity: None,
    }
}

/// One phase's density as a published quantity.
fn density_of(phase: Option<&EntropyPhase>) -> Option<MassDensity> {
    phase.map(|phase| kilograms_per_cubic_meter(phase.density))
}

/// The class's success result.
#[allow(clippy::too_many_arguments)] // one argument per result field
fn success(
    flow: Option<f64>,
    choked: bool,
    z: &[f64],
    mass_fractions: Vec<f64>,
    stations: Vec<LegacyScreeningReleaseStation>,
    diagnostics: Vec<HemReleaseDiagnostic>,
    warnings: Vec<Warning>,
) -> LegacyScreeningReleaseResult {
    LegacyScreeningReleaseResult {
        status: "valid_with_warnings".to_string(),
        mass_flow_rate: flow.map(kilograms_per_second),
        choked,
        throat_sound_speed: None,
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
) -> LegacyScreeningReleaseResult {
    LegacyScreeningReleaseResult {
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
