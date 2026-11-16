//! `process.blowdown_pressure_profile` - the profile lookup as a registered id.
//!
//! Spec: `specs/models/process/blowdown_pressure_profile.toml`. The arithmetic is
//! `process/safety/rupture/BlowdownPressureProfile.java` at the pin: the piecewise step or
//! linear lookup, its absolute `1e-10` second window, both endpoint clamps and the gauge
//! offset.

use azoth_core::units::{Pressure, Time, bars};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's absolute near-equality on seconds.
const NEARLY_EQUAL_SECONDS: f64 = 1.0e-10;
/// The class's default absolute-to-gauge offset, bar.
const DEFAULT_GAUGE_OFFSET_BAR: f64 = 1.0;
/// Pascals in a bar.
const PA_PER_BAR: f64 = 100_000.0;

/// One point of the profile.
#[derive(Debug, Clone, PartialEq)]
pub struct BlowdownProfilePoint {
    /// The point's time.
    pub time: Time,
    /// The point's absolute pressure.
    pub pressure: Pressure,
}

/// Result of `process.blowdown_pressure_profile`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlowdownPressureProfileResult {
    /// The profile's absolute pressure at the query time.
    #[serde(serialize_with = "scalar")]
    pub pressure_absolute: Pressure,
    /// The absolute pressure minus the gauge offset.
    #[serde(serialize_with = "scalar")]
    pub pressure_gauge: Pressure,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// Refuse a value the arithmetic cannot carry to a published number.
fn require_finite(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

/// The piecewise pressure lookup.
///
/// # Errors
/// `InvalidInput` for fewer than two points, a negative or non-finite time, a non-positive
/// or non-finite pressure, times that are not strictly increasing, a negative query time, a
/// negative gauge offset, an unknown interpolation mode, or a non-finite value.
pub fn blowdown_pressure_profile(
    points: &[BlowdownProfilePoint],
    time: Time,
    interpolation_mode: Option<&str>,
    gauge_offset: Option<Pressure>,
) -> Result<BlowdownPressureProfileResult> {
    let spec = &model_gen::BLOWDOWN_PRESSURE_PROFILE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "time" => Some(time.value),
            "gauge_offset" => gauge_offset.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite("time", time.value)?;
    let offset = gauge_offset.map_or(DEFAULT_GAUGE_OFFSET_BAR, |value| value.value / PA_PER_BAR);
    require_finite("gauge_offset", offset)?;
    let mode = interpolation_mode.unwrap_or("step_previous");
    let linear = match mode {
        "step_previous" => false,
        "linear" => true,
        other => {
            return Err(AzothError::invalid_input(
                "interpolation_mode",
                format!("`{other}` is not `step_previous` or `linear`"),
            ));
        }
    };

    if points.len() < 2 {
        return Err(AzothError::invalid_input(
            "points",
            "the class refuses a profile with fewer than two time/pressure points",
        ));
    }
    let mut previous = f64::NEG_INFINITY;
    for point in points {
        require_finite("points.time", point.time.value)?;
        require_finite("points.pressure", point.pressure.value)?;
        if point.time.value < 0.0 {
            return Err(AzothError::invalid_input(
                "points.time",
                "the class refuses a negative profile time",
            ));
        }
        if point.pressure.value <= 0.0 {
            return Err(AzothError::invalid_input(
                "points.pressure",
                "the class refuses a non-positive absolute pressure",
            ));
        }
        if point.time.value <= previous {
            return Err(AzothError::invalid_input(
                "points.time",
                "the class requires strictly increasing profile times",
            ));
        }
        previous = point.time.value;
    }

    let pressure_bar = |point: &BlowdownProfilePoint| point.pressure.value / PA_PER_BAR;
    let last = points.len() - 1;
    let query = time.value;
    let absolute = if query <= points[0].time.value {
        pressure_bar(&points[0])
    } else if query >= points[last].time.value {
        pressure_bar(&points[last])
    } else {
        let mut answer = pressure_bar(&points[last]);
        for index in 1..=last {
            let upper_time = points[index].time.value;
            if (query - upper_time).abs() < NEARLY_EQUAL_SECONDS {
                answer = pressure_bar(&points[index]);
                break;
            }
            if query < upper_time {
                answer = if linear {
                    let lower_time = points[index - 1].time.value;
                    let lower = pressure_bar(&points[index - 1]);
                    let upper = pressure_bar(&points[index]);
                    lower + (upper - lower) * (query - lower_time) / (upper_time - lower_time)
                } else {
                    pressure_bar(&points[index - 1])
                };
                break;
            }
        }
        answer
    };

    Ok(BlowdownPressureProfileResult {
        pressure_absolute: bars(absolute),
        pressure_gauge: bars(absolute - offset),
        warnings,
    })
}

