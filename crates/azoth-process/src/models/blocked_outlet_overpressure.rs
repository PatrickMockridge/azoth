//! `process.blocked_outlet_overpressure` - the blocked-outlet pressure build-up.
//!
//! Spec: `specs/models/process/blocked_outlet_overpressure.toml`. The arithmetic is
//! `process/safety/depressurization/BlockedOutletOverpressureAnalyzer.java` at the pin: a
//! vessel filling run whose target is the relief set pressure, then the first crossing.

use azoth_core::units::{
    Area, HeatTransfer, Length, Mass, MassRate, Pressure, SpecificHeatCapacity,
    ThermodynamicTemperature, Time, Volume, seconds,
};
use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::models::vessel_filling::{VesselFillingResult, vessel_filling};

/// Result of `process.blocked_outlet_overpressure`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlockedOutletOverpressureResult {
    /// The relief set pressure.
    #[serde(serialize_with = "scalar")]
    pub relief_set_pressure: Pressure,
    /// The highest recorded pressure.
    #[serde(serialize_with = "scalar")]
    pub max_pressure: Pressure,
    /// The last recorded pressure.
    #[serde(serialize_with = "scalar")]
    pub final_pressure: Pressure,
    /// The last recorded inventory mass.
    #[serde(serialize_with = "scalar")]
    pub final_mass: Mass,
    /// The continued feed mass flow.
    #[serde(serialize_with = "scalar")]
    pub inlet_mass_flow: MassRate,
    /// The configured horizon.
    #[serde(serialize_with = "scalar")]
    pub simulated_horizon: Time,
    /// The first sample at or above the set pressure.
    #[serde(serialize_with = "optional_scalar")]
    pub time_to_relief_set: Option<Time>,
    /// Whether the set pressure was reached.
    pub relief_demand: bool,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The blocked-outlet pressure build-up to the relief set pressure.
///
/// # Errors
/// `InvalidInput` for a non-positive dimension or relief set pressure, or a feed state the
/// filling flash cannot resolve.
#[allow(clippy::too_many_arguments)] // one parameter per quantity the class reads
pub fn blocked_outlet_overpressure(
    components: &[String],
    initial_z: &[f64],
    initial_t: ThermodynamicTemperature,
    initial_p: Pressure,
    vessel_volume: Volume,
    inlet_temperature: ThermodynamicTemperature,
    supply_pressure: Pressure,
    inlet_mass_flow: MassRate,
    relief_set_pressure: Pressure,
    time_step: Option<Time>,
    max_time: Option<Time>,
) -> Result<BlockedOutletOverpressureResult> {
    let step = time_step.map_or(1.0, |value| value.value);
    let span = max_time.map_or(3600.0, |value| value.value);
    if relief_set_pressure.value <= 0.0 || !relief_set_pressure.value.is_finite() {
        return Err(AzothError::invalid_input(
            "relief_set_pressure",
            "the class refuses a non-positive set pressure",
        ));
    }
    if step <= 0.0 || span <= 0.0 || !step.is_finite() || !span.is_finite() {
        return Err(AzothError::invalid_input(
            "time_step",
            "the class refuses a non-positive step or horizon",
        ));
    }

    let filling: VesselFillingResult = vessel_filling(
        components,
        initial_z,
        initial_t,
        initial_p,
        vessel_volume,
        inlet_temperature,
        supply_pressure,
        inlet_mass_flow,
        Some(relief_set_pressure),
        None::<Mass>,
        None::<Area>,
        None::<SpecificHeatCapacity>,
        None::<HeatTransfer>,
        None::<Length>,
        None::<Length>,
        None::<ThermodynamicTemperature>,
        None::<ThermodynamicTemperature>,
        Some(seconds(step)),
        Some(seconds(span)),
    )?;

    let mut time_to_relief_set = None;
    for point in &filling.points {
        if point.pressure.value >= relief_set_pressure.value {
            time_to_relief_set = Some(point.time.value);
            break;
        }
    }

    Ok(BlockedOutletOverpressureResult {
        relief_set_pressure,
        max_pressure: filling.max_pressure,
        final_pressure: filling.final_pressure,
        final_mass: filling.final_mass,
        inlet_mass_flow,
        simulated_horizon: seconds(span),
        time_to_relief_set: time_to_relief_set.map(seconds),
        relief_demand: time_to_relief_set.is_some(),
        warnings: filling.warnings,
    })
}
