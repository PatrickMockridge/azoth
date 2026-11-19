//! `process.bubble_transfer_zone_release` - NeqSim's predictive bubble transfer zone.
//!
//! Spec: `specs/models/process/bubble_transfer_zone_release.toml`. The port source is
//! `process/safety/release/PredictiveBubbleTransferZoneReleaseModel.java`, through its
//! `calculate` and `predictHydrodynamics`.
//!
//! The drift-flux base supplies the phase velocities; a fixed point then predicts the bubble
//! diameter, the relative velocity and the residence time, and the Ranz-Marshall adapter
//! solves each iterate.

use azoth_core::units::{
    Length, MassDensity, MassFlux, MassRate, MolarEnergy, MolarHeatCapacity, Pressure,
    ThermodynamicTemperature, Velocity, kilograms_per_second,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::drift_flux_release::{DriftFluxReleaseStation, drift_flux_release};
use crate::models::hem_release::{HemReleaseDiagnostic, ReleasePhase, ReleaseStation};
use crate::models::ranz_marshall_release::{RanzMarshallReleaseStation, ranz_marshall_release};

/// The class's fixed-point iteration cap.
const MAX_FIXED_POINT_ITERATIONS: usize = 24;
/// The class's fixed-point relative tolerance.
const FIXED_POINT_RELATIVE_TOLERANCE: f64 = 1.0e-8;
/// The class's gravity.
const GRAVITY: f64 = 9.80665;
/// The Eotvos bound.
const MAX_EOTVOS_NUMBER: f64 = 4.0;
/// The Weber bound.
const MAX_WEBER_NUMBER: f64 = 3.0;
/// The hydraulic-diameter fraction.
const MAX_BUBBLE_TO_HYDRAULIC_DIAMETER: f64 = 0.25;

/// One row of `stations`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BubbleTransferZoneReleaseStation {
    /// Which station this row is.
    pub station: ReleaseStation,
    /// Absolute pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// Temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// The relaxed harmonic mixture density at the opening.
    #[serde(serialize_with = "scalar")]
    pub density: MassDensity,
    /// Molar enthalpy.
    #[serde(serialize_with = "scalar")]
    pub molar_enthalpy: MolarEnergy,
    /// Molar entropy.
    #[serde(serialize_with = "scalar")]
    pub molar_entropy: MolarHeatCapacity,
    /// The station's bulk velocity.
    #[serde(serialize_with = "scalar")]
    pub velocity: Velocity,
    /// The station's mass flux, before the discharge coefficient.
    #[serde(serialize_with = "scalar")]
    pub mass_flux: MassFlux,
    /// What the base flash found.
    pub phase: ReleasePhase,
    /// The relaxed gas phase's share of the mass.
    pub gas_mass_fraction: f64,
    /// The gas phase's own density.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_density: Option<MassDensity>,
    /// The liquid phase's own density.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_density: Option<MassDensity>,
    /// The solved gas velocity.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_velocity: Option<Velocity>,
    /// The solved liquid velocity.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_velocity: Option<Velocity>,
}

/// Result of `process.bubble_transfer_zone_release`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BubbleTransferZoneReleaseResult {
    /// `valid`, `invalid` or `unsupported`.
    pub status: String,
    /// The delegate's release rate; absent when the result is not usable.
    #[serde(serialize_with = "optional_scalar")]
    pub mass_flow_rate: Option<MassRate>,
    /// The base scan's choked flag.
    pub choked: bool,
    /// Absent: the derived model drops the base's throat sound speed.
    #[serde(serialize_with = "optional_scalar")]
    pub throat_sound_speed: Option<Velocity>,
    /// The upstream overall mole fractions.
    pub component_mole_fractions: Vec<f64>,
    /// The upstream overall mass fractions.
    pub component_mass_fractions: Vec<f64>,
    /// The delegate's stations.
    pub stations: Vec<BubbleTransferZoneReleaseStation>,
    /// The model's assumptions and the delegate's retained records.
    pub diagnostics: Vec<HemReleaseDiagnostic>,
    /// The diagnostics' machine codes alone.
    pub diagnostic_codes: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One predicted hydrodynamic state.
#[derive(Debug, Clone, Copy)]
struct Prediction {
    diameter: f64,
    gas_velocity: f64,
    liquid_velocity: f64,
    relative_velocity: f64,
    residence: f64,
    liquid_density: f64,
}

/// The class's prediction refusal.
enum TransferRefusal {
    Unsupported(String),
    Invalid(String),
}

/// NeqSim's predictive bubble transfer zone.
///
/// # Errors
/// A non-positive or empty input, a composition that is not one entry per component, or a
/// mixture the databank cannot resolve.
#[allow(clippy::too_many_arguments)] // one argument per spec input
pub fn bubble_transfer_zone_release(
    components: &[String],
    upstream_z: &[f64],
    upstream_t: ThermodynamicTemperature,
    upstream_p: Pressure,
    diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
    surface_tension: f64,
    transfer_zone_hydraulic_diameter: f64,
    transfer_zone_vertical_length: f64,
    continuous_phase_dynamic_viscosity: f64,
    component_diffusivities: &[f64],
    parameter_provenance: &str,
) -> Result<BubbleTransferZoneReleaseResult> {
    let spec = &model_gen::BUBBLE_TRANSFER_ZONE_RELEASE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "upstream_t" => Some(upstream_t.value),
            "upstream_p" => Some(upstream_p.value),
            "diameter" => Some(diameter.value),
            "discharge_coefficient" => Some(discharge_coefficient),
            "back_pressure" => Some(back_pressure.value),
            "surface_tension" => Some(surface_tension),
            "transfer_zone_hydraulic_diameter" => Some(transfer_zone_hydraulic_diameter),
            "transfer_zone_vertical_length" => Some(transfer_zone_vertical_length),
            "continuous_phase_dynamic_viscosity" => Some(continuous_phase_dynamic_viscosity),
            _ => None,
        },
        &mut warnings,
    )?;
    if !surface_tension.is_finite() || surface_tension <= 0.0 {
        return Err(AzothError::invalid_input(
            "surface_tension",
            "the surface tension must be finite and positive".to_string(),
        ));
    }
    if !transfer_zone_hydraulic_diameter.is_finite() || transfer_zone_hydraulic_diameter <= 0.0 {
        return Err(AzothError::invalid_input(
            "transfer_zone_hydraulic_diameter",
            "the hydraulic diameter must be finite and positive".to_string(),
        ));
    }
    if !transfer_zone_vertical_length.is_finite() || transfer_zone_vertical_length <= 0.0 {
        return Err(AzothError::invalid_input(
            "transfer_zone_vertical_length",
            "the vertical length must be finite and positive".to_string(),
        ));
    }
    if !continuous_phase_dynamic_viscosity.is_finite() || continuous_phase_dynamic_viscosity <= 0.0
    {
        return Err(AzothError::invalid_input(
            "continuous_phase_dynamic_viscosity",
            "the continuous-phase viscosity must be finite and positive".to_string(),
        ));
    }
    if parameter_provenance.trim().is_empty() {
        return Err(AzothError::invalid_input(
            "parameter_provenance",
            "the parameter provenance must not be empty".to_string(),
        ));
    }
    if component_diffusivities.len() != components.len() {
        return Ok(failure(
            "invalid",
            "TRANSFER_ZONE_COMPONENT_BASIS_INVALID",
            "Diffusivity components must exactly match the release-fluid components".to_string(),
            warnings,
        ));
    }
    if component_diffusivities
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(AzothError::invalid_input(
            "component_diffusivities",
            "every component diffusivity must be finite and positive".to_string(),
        ));
    }

    let base = drift_flux_release(
        components,
        upstream_z,
        upstream_t,
        upstream_p,
        diameter,
        discharge_coefficient,
        back_pressure,
        surface_tension,
    )?;
    if base.status != "valid" {
        let code = base
            .diagnostic_codes
            .first()
            .map_or("UNKNOWN", String::as_str);
        let message = base
            .diagnostics
            .first()
            .map_or(String::new(), |row| row.message.clone());
        let status = base.status.clone();
        return Ok(failure(
            &status,
            &format!("TRANSFER_ZONE_BASE_{code}"),
            message,
            base.warnings,
        ));
    }
    if base.mass_flow_rate.is_some_and(|rate| rate.value == 0.0) {
        return Ok(success(
            Some(0.0),
            false,
            &base.component_mole_fractions,
            base.component_mass_fractions.clone(),
            base.stations.iter().map(station_from_drift).collect(),
            vec![HemReleaseDiagnostic {
                code: "TRANSFER_ZONE_NOT_APPLIED".to_string(),
                message: "No forward release flow".to_string(),
            }],
            base.warnings,
        ));
    }

    let mut gas_density = base.stations[1]
        .gas_density
        .map_or(0.0, |value| value.value);
    let mut liquid_density = base.stations[1]
        .liquid_density
        .map_or(0.0, |value| value.value);
    let mut gas_velocity = base.stations[1]
        .gas_velocity
        .map_or(0.0, |value| value.value);
    let mut liquid_velocity = base.stations[1]
        .liquid_velocity
        .map_or(0.0, |value| value.value);
    let mut previous: Option<Prediction> = None;
    let mut previous_rate = f64::NAN;
    for _ in 1..=MAX_FIXED_POINT_ITERATIONS {
        let prediction = match predict(
            gas_density,
            liquid_density,
            gas_velocity,
            liquid_velocity,
            surface_tension,
            transfer_zone_hydraulic_diameter,
            transfer_zone_vertical_length,
            diameter.value,
        ) {
            Ok(prediction) => prediction,
            Err(refusal) => return Ok(refused(refusal, base.warnings)),
        };
        let delegate = ranz_marshall_release(
            components,
            upstream_z,
            upstream_t,
            upstream_p,
            diameter,
            discharge_coefficient,
            back_pressure,
            surface_tension,
            prediction.residence,
            "gas_bubbles",
            prediction.diameter,
            prediction.liquid_density,
            continuous_phase_dynamic_viscosity,
            prediction.relative_velocity,
            component_diffusivities,
            parameter_provenance,
        );
        let delegate = match delegate {
            Ok(delegate) => delegate,
            Err(error) => {
                return Ok(failure(
                    "invalid",
                    "TRANSFER_ZONE_CLOSURE_FAILED",
                    format!("{error}"),
                    base.warnings,
                ));
            }
        };
        if delegate.status != "valid" {
            let code = delegate
                .diagnostic_codes
                .first()
                .map_or("UNKNOWN", String::as_str);
            let message = delegate
                .diagnostics
                .first()
                .map_or(String::new(), |row| row.message.clone());
            let status = delegate.status.clone();
            return Ok(failure(
                &status,
                &format!("TRANSFER_ZONE_{code}"),
                message,
                delegate.warnings,
            ));
        }
        let rate = delegate.mass_flow_rate.map_or(0.0, |rate| rate.value);
        if let Some(previous) = previous {
            if converged(&previous, &prediction)
                && relative_difference(previous_rate, rate) <= FIXED_POINT_RELATIVE_TOLERANCE
            {
                let mut diagnostics = vec![HemReleaseDiagnostic {
                    code: "MODEL_ASSUMPTIONS".to_string(),
                    message: "Predictive vertical spherical-bubble transfer zone with bounded \
                              Eotvos/Weber/geometry diameter, resolved drift-flux phase \
                              velocities, gas residence time and Ranz-Marshall external-film \
                              transfer; no population balance, entrainment, annular flow, \
                              interfacial heat transfer or solids"
                        .to_string(),
                }];
                diagnostics.push(HemReleaseDiagnostic {
                    code: "PREDICTIVE_BUBBLE_TRANSFER_ZONE".to_string(),
                    message: "Diameter, relative velocity and residence time from the bounded \
                              transfer-zone prediction"
                        .to_string(),
                });
                for row in &delegate.diagnostics {
                    if row.code != "MODEL_ASSUMPTIONS" {
                        diagnostics.push(row.clone());
                    }
                }
                return Ok(success(
                    Some(rate),
                    delegate.choked,
                    &delegate.component_mole_fractions,
                    delegate.component_mass_fractions.clone(),
                    delegate.stations.iter().map(station).collect(),
                    diagnostics,
                    delegate.warnings.clone(),
                ));
            }
        }
        previous = Some(prediction);
        previous_rate = rate;
        if let Some(next) = delegate.stations.get(1) {
            gas_density = next.gas_density.map_or(0.0, |value| value.value);
            liquid_density = next.liquid_density.map_or(0.0, |value| value.value);
            gas_velocity = next.gas_velocity.map_or(0.0, |value| value.value);
            liquid_velocity = next.liquid_velocity.map_or(0.0, |value| value.value);
        } else {
            break;
        }
    }
    Ok(failure(
        "invalid",
        "TRANSFER_ZONE_NOT_CONVERGED",
        "Bubble diameter, phase velocity, residence time and release rate did not converge \
         within 24 fixed-point iterations"
            .to_string(),
        base.warnings,
    ))
}

/// The predicted hydrodynamics of one station.
#[allow(clippy::too_many_arguments)] // one argument per prediction input
fn predict(
    gas_density: f64,
    liquid_density: f64,
    gas_velocity: f64,
    liquid_velocity: f64,
    surface_tension: f64,
    hydraulic_diameter: f64,
    vertical_length: f64,
    opening_diameter: f64,
) -> std::result::Result<Prediction, TransferRefusal> {
    if gas_density <= 0.0 || liquid_density <= 0.0 || gas_velocity <= 0.0 || liquid_velocity <= 0.0
    {
        return Err(TransferRefusal::Unsupported(
            "Resolved one-gas/one-liquid phase velocities are required".to_string(),
        ));
    }
    if liquid_density <= gas_density {
        return Err(TransferRefusal::Unsupported(
            "Transfer-zone closure requires liquid density greater than gas density".to_string(),
        ));
    }
    let relative_velocity = (gas_velocity - liquid_velocity).abs();
    let eotvos_diameter =
        (MAX_EOTVOS_NUMBER * surface_tension / (GRAVITY * (liquid_density - gas_density))).sqrt();
    let weber_diameter = if relative_velocity == 0.0 {
        f64::INFINITY
    } else {
        MAX_WEBER_NUMBER * surface_tension
            / (liquid_density * relative_velocity * relative_velocity)
    };
    let hydraulic_limit = MAX_BUBBLE_TO_HYDRAULIC_DIAMETER * hydraulic_diameter;
    let diameter = eotvos_diameter
        .min(weber_diameter)
        .min(hydraulic_limit)
        .min(opening_diameter);
    if !diameter.is_finite() || diameter <= 0.0 {
        return Err(TransferRefusal::Invalid(
            "predicted bubble diameter must be positive".to_string(),
        ));
    }
    let residence = vertical_length / gas_velocity;
    if !residence.is_finite() || residence <= 0.0 {
        return Err(TransferRefusal::Invalid(
            "predicted bubble residence time must be positive".to_string(),
        ));
    }
    Ok(Prediction {
        diameter,
        gas_velocity,
        liquid_velocity,
        relative_velocity,
        residence,
        liquid_density,
    })
}

/// Whether two predictions agree at the class tolerance.
fn converged(previous: &Prediction, current: &Prediction) -> bool {
    relative_difference(previous.diameter, current.diameter) <= FIXED_POINT_RELATIVE_TOLERANCE
        && relative_difference(previous.gas_velocity, current.gas_velocity)
            <= FIXED_POINT_RELATIVE_TOLERANCE
        && relative_difference(previous.liquid_velocity, current.liquid_velocity)
            <= FIXED_POINT_RELATIVE_TOLERANCE
        && relative_difference(previous.relative_velocity, current.relative_velocity)
            <= FIXED_POINT_RELATIVE_TOLERANCE
        && relative_difference(previous.residence, current.residence)
            <= FIXED_POINT_RELATIVE_TOLERANCE
}

/// The relative difference, infinite for a non-finite operand.
fn relative_difference(first: f64, second: f64) -> f64 {
    if !first.is_finite() || !second.is_finite() {
        return f64::INFINITY;
    }
    (first - second).abs() / 1.0e-15_f64.max(first.abs().max(second.abs()))
}

/// One Ranz-Marshall station as this model's row.
fn station(row: &RanzMarshallReleaseStation) -> BubbleTransferZoneReleaseStation {
    BubbleTransferZoneReleaseStation {
        station: row.station,
        pressure: row.pressure,
        temperature: row.temperature,
        density: row.density,
        molar_enthalpy: row.molar_enthalpy,
        molar_entropy: row.molar_entropy,
        velocity: row.velocity,
        mass_flux: row.mass_flux,
        phase: row.phase,
        gas_mass_fraction: row.gas_mass_fraction,
        gas_density: row.gas_density,
        liquid_density: row.liquid_density,
        gas_velocity: row.gas_velocity,
        liquid_velocity: row.liquid_velocity,
    }
}

/// One drift-flux station as this model's row.
fn station_from_drift(row: &DriftFluxReleaseStation) -> BubbleTransferZoneReleaseStation {
    BubbleTransferZoneReleaseStation {
        station: row.station,
        pressure: row.pressure,
        temperature: row.temperature,
        density: row.density,
        molar_enthalpy: row.molar_enthalpy,
        molar_entropy: row.molar_entropy,
        velocity: row.velocity,
        mass_flux: row.mass_flux,
        phase: row.phase,
        gas_mass_fraction: row.gas_mass_fraction,
        gas_density: row.gas_density,
        liquid_density: row.liquid_density,
        gas_velocity: row.gas_velocity,
        liquid_velocity: row.liquid_velocity,
    }
}

/// The class's success result.
#[allow(clippy::too_many_arguments)] // one argument per result field
fn success(
    flow: Option<f64>,
    choked: bool,
    z: &[f64],
    mass_fractions: Vec<f64>,
    stations: Vec<BubbleTransferZoneReleaseStation>,
    diagnostics: Vec<HemReleaseDiagnostic>,
    warnings: Vec<Warning>,
) -> BubbleTransferZoneReleaseResult {
    BubbleTransferZoneReleaseResult {
        status: "valid".to_string(),
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

/// The class's result for a prediction refusal.
fn refused(refusal: TransferRefusal, warnings: Vec<Warning>) -> BubbleTransferZoneReleaseResult {
    match refusal {
        TransferRefusal::Unsupported(message) => failure(
            "unsupported",
            "TRANSFER_ZONE_REGIME_UNSUPPORTED",
            message,
            warnings,
        ),
        TransferRefusal::Invalid(message) => {
            failure("invalid", "TRANSFER_ZONE_CLOSURE_FAILED", message, warnings)
        }
    }
}

/// The class's failure result: no rate, no stations, one diagnostic.
fn failure(
    status: &str,
    code: &str,
    message: String,
    warnings: Vec<Warning>,
) -> BubbleTransferZoneReleaseResult {
    BubbleTransferZoneReleaseResult {
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
