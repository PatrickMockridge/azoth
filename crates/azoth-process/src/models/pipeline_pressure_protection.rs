//! `process.pipeline_pressure_protection` - the two-barrier pipeline screen.
//!
//! Spec: `specs/models/process/pipeline_pressure_protection.toml`. The arithmetic is
//! `process/safety/overpressure/PipelinePressureProtectionCalculator.java` at the pin:
//! the incidental pressure, the design margin and the three verdicts, in bar.

use azoth_core::units::{Pressure, bars};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// Result of `process.pipeline_pressure_protection`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PipelinePressureProtectionResult {
    /// `incidental_factor design_pressure`.
    #[serde(serialize_with = "scalar")]
    pub max_incidental_pressure: Pressure,
    /// `design_pressure - max_source_pressure`.
    #[serde(serialize_with = "scalar")]
    pub design_margin: Pressure,
    /// Whether the source is at or below the design pressure.
    pub fully_rated: bool,
    /// Whether barrier 1 is at or below the design pressure.
    pub barrier1_adequate: bool,
    /// Whether barrier 2 is at or below the maximum incidental pressure.
    pub barrier2_adequate: bool,
    /// Whether the segment is fully rated or both barriers are adequate.
    pub protection_adequate: bool,
    /// `FULLY_RATED`, `TWO_BARRIERS_REQUIRED` or `INSUFFICIENT`.
    pub verdict: String,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The incidental pressure, the design margin and the protection verdict.
///
/// # Errors
/// `InvalidInput` for a non-positive or non-finite pressure, or an incidental
/// factor below one.
pub fn pipeline_pressure_protection(
    max_source_pressure: Pressure,
    design_pressure: Pressure,
    incidental_factor: f64,
    barrier1_setpoint: Pressure,
    barrier2_setpoint: Pressure,
) -> Result<PipelinePressureProtectionResult> {
    let spec = &model_gen::PIPELINE_PRESSURE_PROTECTION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "max_source_pressure" => Some(max_source_pressure.value),
            "design_pressure" => Some(design_pressure.value),
            "incidental_factor" => Some(incidental_factor),
            "barrier1_setpoint" => Some(barrier1_setpoint.value),
            "barrier2_setpoint" => Some(barrier2_setpoint.value),
            _ => None,
        },
        &mut warnings,
    )?;
    for (name, value) in [
        ("max_source_pressure", max_source_pressure.value),
        ("design_pressure", design_pressure.value),
        ("barrier1_setpoint", barrier1_setpoint.value),
        ("barrier2_setpoint", barrier2_setpoint.value),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(AzothError::invalid_input(
                name,
                format!("a finite positive pressure is required, and {value} is not"),
            ));
        }
    }
    if !incidental_factor.is_finite() || incidental_factor < 1.0 {
        return Err(AzothError::invalid_input(
            "incidental_factor",
            format!("a factor of at least one is required, and {incidental_factor} is not"),
        ));
    }

    let source_bar = max_source_pressure.value / 1.0e5;
    let design_bar = design_pressure.value / 1.0e5;
    let barrier1_bar = barrier1_setpoint.value / 1.0e5;
    let barrier2_bar = barrier2_setpoint.value / 1.0e5;
    let max_incidental_bar = incidental_factor * design_bar;
    let design_margin_bar = design_bar - source_bar;
    let fully_rated = source_bar <= design_bar;
    let barrier1_adequate = barrier1_bar <= design_bar;
    let barrier2_adequate = barrier2_bar <= max_incidental_bar;
    let protection_adequate = fully_rated || (barrier1_adequate && barrier2_adequate);
    let verdict = if fully_rated {
        "FULLY_RATED"
    } else if protection_adequate {
        "TWO_BARRIERS_REQUIRED"
    } else {
        "INSUFFICIENT"
    };
    Ok(PipelinePressureProtectionResult {
        max_incidental_pressure: bars(max_incidental_bar),
        design_margin: bars(design_margin_bar),
        fully_rated,
        barrier1_adequate,
        barrier2_adequate,
        protection_adequate,
        verdict: verdict.to_string(),
        warnings,
    })
}
