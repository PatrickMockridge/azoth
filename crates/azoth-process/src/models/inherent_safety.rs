//! `process.inherent_safety` - the Kletz four-pillar inherent-safety index.
//!
//! Spec: `specs/models/process/inherent_safety.toml`. The arithmetic is
//! `process/safety/inherent/InherentSafetyEvaluator.java` at the pin: four pillar scores and
//! their simple mean.

use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;

/// Result of `process.inherent_safety`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InherentSafetyResult {
    /// The mean of the four pillar scores.
    pub overall_index: f64,
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

/// The mean of the four Kletz pillar scores.
///
/// # Errors
/// `InvalidInput` for a score outside `[0, 10]` or a non-finite input.
pub fn inherent_safety(
    substitute_score: Option<f64>,
    minimize_score: Option<f64>,
    moderate_score: Option<f64>,
    simplify_score: Option<f64>,
) -> Result<InherentSafetyResult> {
    let spec = &model_gen::INHERENT_SAFETY_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "substitute_score" => substitute_score,
            "minimize_score" => minimize_score,
            "moderate_score" => moderate_score,
            "simplify_score" => simplify_score,
            _ => None,
        },
        &mut warnings,
    )?;

    let scores = [
        ("substitute_score", substitute_score),
        ("minimize_score", minimize_score),
        ("moderate_score", moderate_score),
        ("simplify_score", simplify_score),
    ];
    let mut sum = 0.0;
    for (name, value) in scores {
        let value = value.unwrap_or(0.0);
        require_finite(name, value)?;
        sum += value;
    }

    Ok(InherentSafetyResult {
        overall_index: sum / 4.0,
        warnings,
    })
}
