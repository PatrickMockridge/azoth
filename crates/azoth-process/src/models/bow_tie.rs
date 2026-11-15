//! `process.bow_tie` - the bow-tie risk sum as a registered id.
//!
//! Spec: `specs/models/process/bow_tie.toml`. The arithmetic is
//! `process/safety/risk/bowtie/BowTieModel.java` at the pin: the active threat sum, the
//! prevention-barrier product and the maximum severity.

use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;

/// One threat, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct BowTieThreat {
    /// The threat's frequency in 1/year.
    pub frequency: f64,
    /// Whether the threat contributes; absent is the class's `true`.
    pub active: Option<bool>,
}

/// One consequence's severity.
#[derive(Debug, Clone, PartialEq)]
pub struct BowTieConsequence {
    /// The consequence severity.
    pub severity: f64,
}

/// One barrier, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct BowTieBarrier {
    /// The barrier's probability of failure on demand.
    pub pfd: f64,
    /// `prevention`, `mitigation` or `both`.
    pub barrier_type: String,
    /// Whether the barrier is functional; absent is the class's `true`.
    pub functional: Option<bool>,
}

/// Result of `process.bow_tie`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BowTieResult {
    /// The sum of the active threats' frequencies.
    pub unmitigated_frequency_per_year: f64,
    /// The unmitigated frequency times the functional prevention barriers' PFDs.
    pub mitigated_frequency_per_year: f64,
    /// `unmitigated / mitigated`; zero when the mitigated frequency is zero.
    pub total_rrf: f64,
    /// The largest consequence severity.
    pub max_severity: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The active threat sum, the prevention-barrier product and the maximum severity.
///
/// # Errors
/// `InvalidInput` for a negative or non-finite frequency, a negative or non-whole
/// severity, a PFD outside `[0, 1]`, or a barrier type the class does not define.
pub fn bow_tie(
    threats: &[BowTieThreat],
    consequences: &[BowTieConsequence],
    barriers: &[BowTieBarrier],
) -> Result<BowTieResult> {
    let spec = &model_gen::BOW_TIE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    for (index, threat) in threats.iter().enumerate() {
        if !threat.frequency.is_finite() || threat.frequency < 0.0 {
            return Err(AzothError::invalid_input(
                format!("threats[{index}].frequency"),
                format!(
                    "a threat frequency is finite and at least zero, and {} is not",
                    threat.frequency
                ),
            ));
        }
    }
    for (index, consequence) in consequences.iter().enumerate() {
        let severity = consequence.severity;
        if !severity.is_finite() || severity < 0.0 || severity.fract() != 0.0 {
            return Err(AzothError::invalid_input(
                format!("consequences[{index}].severity"),
                format!("a severity is a whole number at least zero, and {severity} is not"),
            ));
        }
    }
    for (index, barrier) in barriers.iter().enumerate() {
        if !barrier.pfd.is_finite() || !(0.0..=1.0).contains(&barrier.pfd) {
            return Err(AzothError::invalid_input(
                format!("barriers[{index}].pfd"),
                format!("a barrier PFD is in [0, 1], and {} is not", barrier.pfd),
            ));
        }
        if !matches!(
            barrier.barrier_type.as_str(),
            "prevention" | "mitigation" | "both"
        ) {
            return Err(AzothError::invalid_input(
                format!("barriers[{index}].barrier_type"),
                format!(
                    "a barrier type is prevention, mitigation or both, and {:?} is not",
                    barrier.barrier_type
                ),
            ));
        }
    }

    let mut unmitigated = 0.0;
    for threat in threats {
        if threat.active.unwrap_or(true) {
            unmitigated += threat.frequency;
        }
    }
    let mut mitigated = unmitigated;
    for barrier in barriers {
        let prevention = matches!(barrier.barrier_type.as_str(), "prevention" | "both");
        if prevention && barrier.functional.unwrap_or(true) {
            mitigated *= barrier.pfd;
        }
    }
    let mut max_severity = 0.0;
    for consequence in consequences {
        if consequence.severity > max_severity {
            max_severity = consequence.severity;
        }
    }
    let total_rrf = if mitigated > 0.0 {
        unmitigated / mitigated
    } else {
        0.0
    };

    Ok(BowTieResult {
        unmitigated_frequency_per_year: unmitigated,
        mitigated_frequency_per_year: mitigated,
        total_rrf,
        max_severity,
        warnings,
    })
}
