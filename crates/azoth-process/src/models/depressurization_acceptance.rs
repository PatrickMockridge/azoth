//! `process.depressurization_acceptance` - the STS-0131 acceptance of a stored blowdown.
//!
//! Spec: `specs/models/process/depressurization_acceptance.toml`. The arithmetic is
//! `process/safety/depressurization/STS0131AcceptanceCriteria.java` at the pin: the limiting
//! time, the interpolated state, the peak rate and the three configured criteria.

use azoth_core::units::{
    Mass, MassRate, Pressure, Time, kilograms, kilograms_per_second, pascals, seconds,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// One sample of a blowdown trajectory.
#[derive(Debug, Clone, PartialEq)]
pub struct DepressurizationAcceptancePoint {
    /// The sample's time.
    pub time: Time,
    /// The sample's pressure.
    pub pressure: Pressure,
    /// The sample's inventory.
    pub mass: Mass,
    /// The sample's discharge rate.
    pub mass_flow: MassRate,
}

/// Result of `process.depressurization_acceptance`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DepressurizationAcceptanceResult {
    /// The earliest configured limiting time bounded by the trajectory.
    #[serde(serialize_with = "scalar")]
    pub limiting_time: Time,
    /// The interpolated pressure at the limiting time.
    #[serde(serialize_with = "scalar")]
    pub pressure_at_limiting_time: Pressure,
    /// The interpolated inventory at the limiting time.
    #[serde(serialize_with = "scalar")]
    pub remaining_mass_at_limiting_time: Mass,
    /// The maximum discharge rate at or before the limiting time.
    #[serde(serialize_with = "scalar")]
    pub peak_discharge_rate: MassRate,
    /// Whether a pressure limit was given.
    pub pressure_configured: bool,
    /// Whether the pressure is at or below its limit.
    pub pressure_criterion_met: bool,
    /// Whether a mass limit was given.
    pub mass_configured: bool,
    /// Whether the mass is at or below its limit.
    pub mass_criterion_met: bool,
    /// Whether a fire-rate limit was given.
    pub fire_rate_configured: bool,
    /// Whether the peak rate is at or below its limit.
    pub fire_rate_criterion_met: bool,
    /// Whether all three criteria pass.
    pub acceptable: bool,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// A positive configured time, or `None` for the class's absent sentinel.
fn positive_time(value: Option<Time>) -> Option<f64> {
    value
        .map(|time| time.value)
        .filter(|time| time.is_finite() && *time > 0.0)
}

/// The linear interpolation of a series at a time, endpoint values outside the range.
fn interpolate(times: &[f64], values: &[f64], target: f64) -> f64 {
    if target <= times[0] {
        return values[0];
    }
    for index in 1..times.len() {
        let upper = times[index];
        if target <= upper {
            let lower = times[index - 1];
            let fraction = if upper > lower {
                (target - lower) / (upper - lower)
            } else {
                0.0
            };
            return values[index - 1] + fraction * (values[index] - values[index - 1]);
        }
    }
    values[values.len() - 1]
}

/// The STS-0131 acceptance of a stored blowdown trajectory.
///
/// # Errors
/// `InvalidInput` for an empty trajectory or a non-finite sample.
#[allow(clippy::too_many_arguments)] // one parameter per criterion the class reads
pub fn depressurization_acceptance(
    points: &[DepressurizationAcceptancePoint],
    time_to_escape: Option<Time>,
    estimated_time_to_rupture: Option<Time>,
    maximum_pressure_at_rupture: Option<Pressure>,
    maximum_remaining_mass: Option<Mass>,
    maximum_escalated_fire_rate: Option<MassRate>,
) -> Result<DepressurizationAcceptanceResult> {
    let spec = &model_gen::DEPRESSURIZATION_ACCEPTANCE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    if points.is_empty() {
        return Err(AzothError::invalid_input(
            "points",
            "the class requires at least one time sample",
        ));
    }
    for point in points {
        for (name, value) in [
            ("time", point.time.value),
            ("pressure", point.pressure.value),
            ("mass", point.mass.value),
            ("mass_flow", point.mass_flow.value),
        ] {
            if !value.is_finite() {
                return Err(AzothError::invalid_input(
                    name,
                    "a non-finite sample is not a state this calculation compares",
                ));
            }
        }
    }

    let times: Vec<f64> = points.iter().map(|point| point.time.value).collect();
    let pressures: Vec<f64> = points.iter().map(|point| point.pressure.value).collect();
    let masses: Vec<f64> = points.iter().map(|point| point.mass.value).collect();

    let mut limiting = times[times.len() - 1];
    for candidate in [
        positive_time(time_to_escape),
        positive_time(estimated_time_to_rupture),
    ]
    .into_iter()
    .flatten()
    {
        limiting = limiting.min(candidate);
    }
    let limiting = limiting.max(0.0);
    let pressure_value = interpolate(&times, &pressures, limiting);
    let mass_value = interpolate(&times, &masses, limiting);
    let peak = points
        .iter()
        .filter(|point| point.time.value <= limiting)
        .map(|point| point.mass_flow.value)
        .fold(0.0_f64, f64::max);

    let pressure_limit = maximum_pressure_at_rupture.map(|value| value.value);
    let mass_limit = maximum_remaining_mass.map(|value| value.value);
    let fire_limit = maximum_escalated_fire_rate.map(|value| value.value);
    let pressure_configured = pressure_limit.is_some();
    let mass_configured = mass_limit.is_some();
    let fire_rate_configured = fire_limit.is_some();
    let pressure_met = pressure_limit.is_none_or(|limit| pressure_value <= limit);
    let mass_met = mass_limit.is_none_or(|limit| mass_value <= limit);
    let fire_met = fire_limit.is_none_or(|limit| peak <= limit);

    Ok(DepressurizationAcceptanceResult {
        limiting_time: seconds(limiting),
        pressure_at_limiting_time: pascals(pressure_value),
        remaining_mass_at_limiting_time: kilograms(mass_value),
        peak_discharge_rate: kilograms_per_second(peak),
        pressure_configured,
        pressure_criterion_met: pressure_met,
        mass_configured,
        mass_criterion_met: mass_met,
        fire_rate_configured,
        fire_rate_criterion_met: fire_met,
        acceptable: pressure_met && mass_met && fire_met,
        warnings,
    })
}
