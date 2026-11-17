//! `process.hazop_consequence_population` - the consequence catalogue's fill.
//!
//! Spec: `specs/models/process/hazop_consequence_population.toml`. The arithmetic is
//! NeqSim's `process/safety/hazid/HazopConsequenceAutoPopulator.java`: the deviation to
//! calculation catalogue, the equipment categorisation, the neutral flow fallback and the
//! placeholder-only fill.

use azoth_core::{Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::hazid::{
    HazopGuideWord, HazopParameter, is_placeholder, mapping_for, recommendation_for,
};

/// One deviation row a caller states.
#[derive(Debug, Clone, PartialEq)]
pub struct HazopDeviationInput {
    /// The guide word, the spec's spelling.
    pub guide_word: String,
    /// The process parameter, the spec's spelling.
    pub parameter: String,
    /// The hypothesised cause, copied unchanged.
    pub cause: String,
    /// The consequence cell, filled only when it is a placeholder.
    pub consequence: String,
    /// The safeguard cell, filled only when it is a placeholder.
    pub safeguard: String,
    /// The recommendation cell, filled only when it is a placeholder.
    pub recommendation: Option<String>,
}

/// One populated deviation row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HazopPopulationRow {
    /// The guide word.
    pub guide_word: HazopGuideWord,
    /// The process parameter.
    pub parameter: HazopParameter,
    /// The hypothesised cause, copied unchanged.
    pub cause: String,
    /// The consequence, mapped when the source cell was a placeholder.
    pub consequence: String,
    /// The safeguard, mapped when the source cell was a placeholder.
    pub safeguard: String,
    /// The recommendation, cited when the source cell was a placeholder.
    pub recommendation: Option<String>,
}

/// Result of `process.hazop_consequence_population`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HazopConsequencePopulationResult {
    /// The rows, one per input row and in its order.
    pub populated: Vec<HazopPopulationRow>,
    /// The number of rows a catalogue mapping was found for.
    pub mapped_count: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The catalogue-filled deviation rows.
///
/// # Errors
/// `InvalidInput` for a guide word or parameter that is not one of the spec's words.
pub fn hazop_consequence_population(
    deviations: &[HazopDeviationInput],
    equipment_type: Option<&str>,
) -> Result<HazopConsequencePopulationResult> {
    let spec = &model_gen::HAZOP_CONSEQUENCE_POPULATION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    let mut populated = Vec::with_capacity(deviations.len());
    let mut mapped_count = 0.0;
    for row in deviations {
        let guide_word = HazopGuideWord::parse(&row.guide_word)?;
        let parameter = HazopParameter::parse(&row.parameter)?;
        let mapping = mapping_for(guide_word, parameter, equipment_type);
        let (consequence, safeguard, recommendation) = match &mapping {
            Some(chosen) => {
                mapped_count += 1.0;
                let consequence = if is_placeholder(Some(&row.consequence)) {
                    chosen.consequence.clone()
                } else {
                    row.consequence.clone()
                };
                let safeguard = if is_placeholder(Some(&row.safeguard)) {
                    chosen.safeguard.to_string()
                } else {
                    row.safeguard.clone()
                };
                let recommendation = if is_placeholder(row.recommendation.as_deref()) {
                    Some(recommendation_for(chosen))
                } else {
                    row.recommendation.clone()
                };
                (consequence, safeguard, recommendation)
            }
            None => (
                row.consequence.clone(),
                row.safeguard.clone(),
                row.recommendation.clone(),
            ),
        };
        populated.push(HazopPopulationRow {
            guide_word,
            parameter,
            cause: row.cause.clone(),
            consequence,
            safeguard,
            recommendation,
        });
    }

    Ok(HazopConsequencePopulationResult {
        populated,
        mapped_count,
        warnings,
    })
}
