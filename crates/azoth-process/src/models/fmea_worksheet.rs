//! `process.fmea_worksheet` - the FMEA risk-priority ranking.
//!
//! Spec: `specs/models/process/fmea_worksheet.toml`. The arithmetic is NeqSim's
//! `process/safety/hazid/FMEAWorksheet.java`: `RPN = S O D`, the stable descending ranking
//! and the threshold filter.

use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;

/// The lowest score the class accepts.
const SCORE_MIN: f64 = 1.0;

/// The highest score the class accepts.
const SCORE_MAX: f64 = 10.0;

/// One failure-mode entry.
#[derive(Debug, Clone, PartialEq)]
pub struct FmeaEntry {
    /// The component or item analysed.
    pub component: String,
    /// The failure mode.
    pub failure_mode: String,
    /// The effect on the system.
    pub effect: String,
    /// The root cause.
    pub cause: String,
    /// The severity score, a whole number in `[1, 10]`.
    pub severity: f64,
    /// The occurrence score, a whole number in `[1, 10]`.
    pub occurrence: f64,
    /// The detection score, a whole number in `[1, 10]`.
    pub detection: f64,
    /// The recommended mitigation; absent when the class carries a null.
    pub mitigation: Option<String>,
}

/// One ranked row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FmeaRankedEntry {
    /// The component or item analysed.
    pub component: String,
    /// The failure mode.
    pub failure_mode: String,
    /// The severity score.
    pub severity: f64,
    /// The occurrence score.
    pub occurrence: f64,
    /// The detection score.
    pub detection: f64,
    /// `severity occurrence detection`.
    pub rpn: f64,
    /// The recommended mitigation; absent when the class carries a null.
    pub mitigation: Option<String>,
}

/// Result of `process.fmea_worksheet`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FmeaWorksheetResult {
    /// The entries by descending RPN, caller order for a tie.
    pub ranked_entries: Vec<FmeaRankedEntry>,
    /// The entries at or above the threshold, caller order.
    pub entries_above_threshold: Vec<FmeaRankedEntry>,
    /// The largest RPN, absent on an empty worksheet.
    pub highest_rpn: Option<f64>,
    /// The number of entries.
    pub entry_count: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn whole(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0
}

fn ranked(entry: &FmeaEntry) -> FmeaRankedEntry {
    FmeaRankedEntry {
        component: entry.component.clone(),
        failure_mode: entry.failure_mode.clone(),
        severity: entry.severity,
        occurrence: entry.occurrence,
        detection: entry.detection,
        rpn: entry.severity * entry.occurrence * entry.detection,
        mitigation: entry.mitigation.clone(),
    }
}

/// The RPN ranking and the entries at or above the threshold.
///
/// # Errors
/// `InvalidInput` for a severity, occurrence or detection outside `[1, 10]` or not a whole
/// number, or for a threshold that is not a whole number.
pub fn fmea_worksheet(entries: &[FmeaEntry], rpn_threshold: f64) -> Result<FmeaWorksheetResult> {
    let spec = &model_gen::FMEA_WORKSHEET_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    for entry in entries {
        for score in [entry.severity, entry.occurrence, entry.detection] {
            if !whole(score) || !(SCORE_MIN..=SCORE_MAX).contains(&score) {
                return Err(AzothError::invalid_input(
                    "entries",
                    "S, O, D must each be in [1, 10]",
                ));
            }
        }
    }
    if !whole(rpn_threshold) {
        return Err(AzothError::invalid_input(
            "rpn_threshold",
            "the threshold must be a whole number",
        ));
    }

    let mut ranked_entries: Vec<FmeaRankedEntry> = entries.iter().map(ranked).collect();
    // `Collections.sort` is stable; `sort_by` is too, so a tie keeps caller order.
    ranked_entries.sort_by(|a, b| b.rpn.total_cmp(&a.rpn));

    let entries_above_threshold: Vec<FmeaRankedEntry> = entries
        .iter()
        .map(ranked)
        .filter(|row| row.rpn >= rpn_threshold)
        .collect();

    Ok(FmeaWorksheetResult {
        highest_rpn: ranked_entries.first().map(|row| row.rpn),
        entry_count: entries.len() as f64,
        entries_above_threshold,
        ranked_entries,
        warnings,
    })
}
