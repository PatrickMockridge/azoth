//! `process.pump_deadhead` - the centrifugal deadhead screen as a registered id.
//!
//! Spec: `specs/models/process/pump_deadhead.toml`. The arithmetic is
//! `process/safety/pump/PumpDeadheadAnalyzer.java` at the pin: the deadhead pressure, the
//! rating verdict and the minimum-flow recirculation rise.

use azoth_core::units::{
    Length, MassDensity, Pressure, SpecificHeatCapacity, TemperatureInterval, kelvin_intervals,
    meters, pascals,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's gravitational acceleration, m/s2.
const GRAVITY: f64 = 9.806_65;
/// The class's default shut-off head multiple.
const DEFAULT_SHUTOFF_RATIO: f64 = 1.20;
/// The class's default minimum-flow efficiency.
const DEFAULT_MIN_FLOW_EFFICIENCY: f64 = 0.30;

/// Whether the pump's deadhead pressure stays within its protected rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DeadheadVerdict {
    /// No protected rating was stated.
    #[serde(rename = "no_rating")]
    NoRating,
    /// The deadhead pressure is at or below the rating.
    #[serde(rename = "within_rating")]
    WithinRating,
    /// The deadhead pressure is above the rating.
    #[serde(rename = "exceeds_rating")]
    ExceedsRating,
}

impl DeadheadVerdict {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoRating => "no_rating",
            Self::WithinRating => "within_rating",
            Self::ExceedsRating => "exceeds_rating",
        }
    }
}

/// Result of `process.pump_deadhead`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PumpDeadheadResult {
    /// `normal_discharge_pressure - suction_pressure`.
    #[serde(serialize_with = "scalar")]
    pub rated_differential_pressure: Pressure,
    /// `suction_pressure + ratio (normal_discharge_pressure - suction_pressure)`.
    #[serde(serialize_with = "scalar")]
    pub deadhead_pressure: Pressure,
    /// `rating - deadhead_pressure`; absent when no rating is stated.
    #[serde(serialize_with = "optional_scalar")]
    pub pressure_margin: Option<Pressure>,
    /// Whether the deadhead pressure is strictly above the rating.
    pub pressure_rating_exceeded: bool,
    /// `shutoff_dp/(rho g)`; absent without density or specific heat.
    #[serde(serialize_with = "optional_scalar")]
    pub shutoff_head: Option<Length>,
    /// The minimum-flow recirculation rise; absent without density or specific heat.
    #[serde(serialize_with = "optional_scalar")]
    pub recirculation_temperature_rise: Option<TemperatureInterval>,
    /// Whether the rise is strictly above the allowable one.
    pub temp_rise_exceeded: bool,
    /// The class's verdict.
    pub verdict: DeadheadVerdict,
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

/// The deadhead pressure against the protected rating and the minimum-flow rise.
///
/// # Errors
/// `InvalidInput` for a negative suction pressure, a non-positive discharge pressure, a
/// discharge pressure at or below the suction, a shut-off ratio below one, a non-positive
/// rating, density, specific heat or allowable rise, an efficiency outside `(0, 1]`, or a
/// non-finite input.
#[allow(clippy::too_many_arguments)] // The signature is the spec's declared input list.
pub fn pump_deadhead(
    suction_pressure: Pressure,
    normal_discharge_pressure: Pressure,
    shutoff_head_ratio: Option<f64>,
    protected_pressure_rating: Option<Pressure>,
    liquid_density: Option<MassDensity>,
    specific_heat: Option<SpecificHeatCapacity>,
    minimum_flow_efficiency: Option<f64>,
    max_allowable_temperature_rise: Option<TemperatureInterval>,
) -> Result<PumpDeadheadResult> {
    let spec = &model_gen::PUMP_DEADHEAD_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "suction_pressure" => Some(suction_pressure.value),
            "normal_discharge_pressure" => Some(normal_discharge_pressure.value),
            "shutoff_head_ratio" => shutoff_head_ratio,
            "protected_pressure_rating" => protected_pressure_rating.map(|value| value.value),
            "liquid_density" => liquid_density.map(|value| value.value),
            "specific_heat" => specific_heat.map(|value| value.value),
            "minimum_flow_efficiency" => minimum_flow_efficiency,
            "max_allowable_temperature_rise" => {
                max_allowable_temperature_rise.map(|value| value.value)
            }
            _ => None,
        },
        &mut warnings,
    )?;

    for (name, value) in [
        ("suction_pressure", Some(suction_pressure.value)),
        (
            "normal_discharge_pressure",
            Some(normal_discharge_pressure.value),
        ),
        ("shutoff_head_ratio", shutoff_head_ratio),
        (
            "protected_pressure_rating",
            protected_pressure_rating.map(|value| value.value),
        ),
        ("liquid_density", liquid_density.map(|value| value.value)),
        ("specific_heat", specific_heat.map(|value| value.value)),
        ("minimum_flow_efficiency", minimum_flow_efficiency),
        (
            "max_allowable_temperature_rise",
            max_allowable_temperature_rise.map(|value| value.value),
        ),
    ] {
        if let Some(value) = value {
            require_finite(name, value)?;
        }
    }
    if normal_discharge_pressure.value <= suction_pressure.value {
        return Err(AzothError::invalid_input(
            "normal_discharge_pressure",
            "the normal discharge pressure must be above the suction pressure",
        ));
    }

    let rated = normal_discharge_pressure.value - suction_pressure.value;
    let ratio = shutoff_head_ratio.unwrap_or(DEFAULT_SHUTOFF_RATIO);
    let shutoff = ratio * rated;
    let deadhead = suction_pressure.value + shutoff;

    let (pressure_margin, pressure_rating_exceeded, verdict) = match protected_pressure_rating {
        Some(rating) => {
            let margin = rating.value - deadhead;
            let exceeded = deadhead > rating.value;
            (
                Some(pascals(margin)),
                exceeded,
                if exceeded {
                    DeadheadVerdict::ExceedsRating
                } else {
                    DeadheadVerdict::WithinRating
                },
            )
        }
        None => (None, false, DeadheadVerdict::NoRating),
    };

    let (shutoff_head, temperature_rise) = match (liquid_density, specific_heat) {
        (Some(density), Some(heat)) => {
            let efficiency = minimum_flow_efficiency.unwrap_or(DEFAULT_MIN_FLOW_EFFICIENCY);
            let head = shutoff / (density.value * GRAVITY);
            let rise = shutoff * (1.0 - efficiency) / (density.value * efficiency * heat.value);
            (Some(meters(head)), Some(kelvin_intervals(rise)))
        }
        _ => (None, None),
    };
    let temp_rise_exceeded = match (temperature_rise, max_allowable_temperature_rise) {
        (Some(rise), Some(allowable)) => rise.value > allowable.value,
        _ => false,
    };

    Ok(PumpDeadheadResult {
        rated_differential_pressure: pascals(rated),
        deadhead_pressure: pascals(deadhead),
        pressure_margin,
        pressure_rating_exceeded,
        shutoff_head,
        recirculation_temperature_rise: temperature_rise,
        temp_rise_exceeded,
        verdict,
        warnings,
    })
}
