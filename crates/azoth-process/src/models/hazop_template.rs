//! `process.hazop_template` - the deviation grid a HAZOP node starts from.
//!
//! Spec: `specs/models/process/hazop_template.toml`. The arithmetic is NeqSim's
//! `process/safety/hazid/HAZOPTemplate.java`: `generateGrid` and the seed table
//! `fromProcessSystem` applies.

use azoth_core::{Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::hazid::{HazopGuideWord, HazopParameter, seed_rows};

/// The class's un-populated cell marker.
const TBD: &str = "TBD";

/// One parameter the grid is generated for.
#[derive(Debug, Clone, PartialEq)]
pub struct HazopGridParameter {
    /// The parameter, the spec's spelling.
    pub parameter: String,
}

/// One deviation row of a HAZOP worksheet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HazopTemplateDeviation {
    /// The guide word.
    pub guide_word: HazopGuideWord,
    /// The process parameter.
    pub parameter: HazopParameter,
    /// The hypothesised cause.
    pub cause: String,
    /// The consequence if no protection acts.
    pub consequence: String,
    /// The existing safeguard.
    pub safeguard: String,
    /// The new recommendation; absent for a generated row.
    pub recommendation: Option<String>,
}

/// Result of `process.hazop_template`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HazopTemplateResult {
    /// The rows, grid rows first.
    pub deviations: Vec<HazopTemplateDeviation>,
    /// The number of rows.
    pub deviation_count: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The generated and seeded deviation rows.
///
/// # Errors
/// `InvalidInput` for a grid parameter that is not one of the six spec words.
pub fn hazop_template(
    grid_parameters: &[HazopGridParameter],
    equipment_type: Option<&str>,
) -> Result<HazopTemplateResult> {
    let spec = &model_gen::HAZOP_TEMPLATE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    let mut deviations = Vec::new();
    // The class seeds a process-derived node before the caller appends a grid, so the seed
    // rows come first.
    if let Some(kind) = equipment_type {
        for &(guide_word, parameter) in seed_rows(kind) {
            deviations.push(HazopTemplateDeviation {
                guide_word,
                parameter,
                cause: TBD.to_string(),
                consequence: TBD.to_string(),
                safeguard: TBD.to_string(),
                recommendation: None,
            });
        }
    }
    for row in grid_parameters {
        let parameter = HazopParameter::parse(&row.parameter)?;
        for guide_word in HazopGuideWord::ALL {
            deviations.push(HazopTemplateDeviation {
                guide_word,
                parameter,
                cause: TBD.to_string(),
                consequence: TBD.to_string(),
                safeguard: TBD.to_string(),
                recommendation: None,
            });
        }
    }

    Ok(HazopTemplateResult {
        deviation_count: deviations.len() as f64,
        deviations,
        warnings,
    })
}
