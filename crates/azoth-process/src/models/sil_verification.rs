//! `process.sil_verification` - the SIL verification arithmetic.
//!
//! Spec: `specs/models/process/sil_verification.toml`. The arithmetic is
//! `process/safety/risk/sis/SILVerificationResult.java` at the pin: the achieved SIL, the
//! upper PFD, the architecture table, the coverage rule and the issue checks.

use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::safety_instrumented_function::required_sil;

/// One issue's severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IssueSeverity {
    /// A condition that does not fail the verification.
    #[serde(rename = "warning")]
    Warning,
    /// A condition that fails the verification.
    #[serde(rename = "error")]
    Error,
    /// Information.
    #[serde(rename = "info")]
    Info,
}

impl IssueSeverity {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Info => "info",
        }
    }
}

/// One issue's category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum IssueCategory {
    /// The PFD exceeds the claimed SIL's maximum.
    #[serde(rename = "pfd_exceeded")]
    PfdExceeded,
    /// The hardware fault tolerance is below what the claimed SIL asks for.
    #[serde(rename = "architecture")]
    Architecture,
    /// The diagnostic coverage may be insufficient.
    #[serde(rename = "diagnostic_coverage")]
    DiagnosticCoverage,
    /// The proof-test interval is long.
    #[serde(rename = "proof_test_interval")]
    ProofTestInterval,
    /// A common-cause condition.
    #[serde(rename = "common_cause")]
    CommonCause,
    /// A systematic-capability condition.
    #[serde(rename = "systematic")]
    Systematic,
}

impl IssueCategory {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PfdExceeded => "pfd_exceeded",
            Self::Architecture => "architecture",
            Self::DiagnosticCoverage => "diagnostic_coverage",
            Self::ProofTestInterval => "proof_test_interval",
            Self::CommonCause => "common_cause",
            Self::Systematic => "systematic",
        }
    }
}

/// One issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SilIssue {
    /// The issue's severity.
    pub severity: IssueSeverity,
    /// The issue's category.
    pub category: IssueCategory,
}

/// Result of `process.sil_verification`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SilVerificationResult {
    /// The claimed SIL, echoed.
    pub claimed_sil: f64,
    /// The SIL the PFD maps to.
    pub achieved_sil: f64,
    /// Whether the achieved SIL is at least the claimed one.
    pub sil_achieved: bool,
    /// The PFD, echoed.
    pub pfd_average: f64,
    /// `1.5 PFD`.
    pub pfd_upper: f64,
    /// The architecture's fault tolerance.
    pub hardware_fault_tolerance: f64,
    /// The claimed SIL, the class's systematic-capability claim.
    pub systematic_capability: f64,
    /// The diagnostic coverage the architecture's spelling implies.
    pub diagnostic_coverage: f64,
    /// The number of issues raised.
    pub issue_count: f64,
    /// The number of errors among them.
    pub error_count: f64,
    /// The number of warnings among them.
    pub warning_count: f64,
    /// Whether no error was raised.
    pub verification_passed: bool,
    /// One row per issue, in the class's check order.
    pub issues: Vec<SilIssue>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn hardware_fault_tolerance(architecture: &str) -> f64 {
    match architecture {
        "1oo1" => 0.0,
        "1oo2" => 1.0,
        "2oo2" => 0.0,
        "2oo3" => 1.0,
        "1oo3" => 2.0,
        _ => 0.0,
    }
}

fn diagnostic_coverage(architecture: &str) -> f64 {
    if architecture.contains("oo2") || architecture.contains("oo3") {
        0.6
    } else {
        0.5
    }
}

fn max_pfd_for_sil(sil: usize) -> f64 {
    match sil {
        1 => 0.1,
        2 => 0.01,
        3 => 0.001,
        4 => 0.0001,
        _ => 1.0,
    }
}

/// The achieved SIL, the upper PFD, the architecture table and the issue set.
///
/// # Errors
/// `InvalidInput` for a negative or non-finite PFD, a claimed SIL outside 1..4 or not
/// whole, or a negative or non-finite proof-test interval.
pub fn sil_verification(
    pfd_average: f64,
    claimed_sil: f64,
    architecture: &str,
    test_interval_hours: f64,
) -> Result<SilVerificationResult> {
    let spec = &model_gen::SIL_VERIFICATION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "pfd_average" => Some(pfd_average),
            "claimed_sil" => Some(claimed_sil),
            "test_interval_hours" => Some(test_interval_hours),
            _ => None,
        },
        &mut warnings,
    )?;
    for (name, value) in [
        ("pfd_average", pfd_average),
        ("claimed_sil", claimed_sil),
        ("test_interval_hours", test_interval_hours),
    ] {
        if !value.is_finite() {
            return Err(AzothError::invalid_input(
                name,
                "a non-finite value is not a number this calculation publishes",
            ));
        }
    }
    if claimed_sil.fract() != 0.0 || !(1.0..=4.0).contains(&claimed_sil) {
        return Err(AzothError::invalid_input(
            "claimed_sil",
            format!("a claimed SIL is a whole number in [1, 4], and {claimed_sil} is not"),
        ));
    }
    let claimed = claimed_sil as usize;

    let achieved = required_sil(pfd_average);
    let hardware_fault_tolerance = hardware_fault_tolerance(architecture);
    let diagnostic_coverage = diagnostic_coverage(architecture);
    let proof_test_interval_years = test_interval_hours / 8760.0;
    let mut issues: Vec<SilIssue> = Vec::new();

    if pfd_average > max_pfd_for_sil(claimed) {
        issues.push(SilIssue {
            severity: IssueSeverity::Error,
            category: IssueCategory::PfdExceeded,
        });
    }
    if claimed == 4 && hardware_fault_tolerance < 2.0 {
        issues.push(SilIssue {
            severity: IssueSeverity::Error,
            category: IssueCategory::Architecture,
        });
    } else if claimed == 3 && hardware_fault_tolerance < 1.0 {
        issues.push(SilIssue {
            severity: IssueSeverity::Warning,
            category: IssueCategory::Architecture,
        });
    }
    if proof_test_interval_years > 5.0 {
        issues.push(SilIssue {
            severity: IssueSeverity::Warning,
            category: IssueCategory::ProofTestInterval,
        });
    }
    if claimed >= 3 && diagnostic_coverage < 0.9 {
        issues.push(SilIssue {
            severity: IssueSeverity::Warning,
            category: IssueCategory::DiagnosticCoverage,
        });
    }

    let error_count = issues
        .iter()
        .filter(|issue| issue.severity == IssueSeverity::Error)
        .count() as f64;
    let warning_count = issues
        .iter()
        .filter(|issue| issue.severity == IssueSeverity::Warning)
        .count() as f64;

    Ok(SilVerificationResult {
        claimed_sil,
        achieved_sil: achieved as f64,
        sil_achieved: achieved >= claimed,
        pfd_average,
        pfd_upper: pfd_average * 1.5,
        hardware_fault_tolerance,
        systematic_capability: claimed_sil,
        diagnostic_coverage,
        issue_count: issues.len() as f64,
        error_count,
        warning_count,
        verification_passed: error_count == 0.0,
        issues,
        warnings,
    })
}
