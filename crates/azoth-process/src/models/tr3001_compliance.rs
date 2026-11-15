//! `process.tr3001_compliance` - the six TR3001 overpressure checks.
//!
//! Spec: `specs/models/process/tr3001_compliance.toml`. The arithmetic is
//! `process/safety/overpressure/TR3001ComplianceChecker.java` at the pin: the
//! governing-case rule and the six findings in the class's order.

use azoth_core::units::MassRate;
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::relief_disposal_network::ReliefCause;

/// Which check a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Tr3001Check {
    /// Credible overpressure causes identified.
    #[serde(rename = "credible_scenarios")]
    CredibleScenarios,
    /// Governing relief case selected.
    #[serde(rename = "governing_case")]
    GoverningCase,
    /// Relief device capacity adequate.
    #[serde(rename = "capacity")]
    Capacity,
    /// Accumulated pressure within its limit.
    #[serde(rename = "acceptance")]
    Acceptance,
    /// The fire case uses the 21 % basis.
    #[serde(rename = "fire_basis")]
    FireBasis,
    /// Dynamic determination considered.
    #[serde(rename = "dynamic_determination")]
    DynamicDetermination,
}

impl Tr3001Check {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CredibleScenarios => "credible_scenarios",
            Self::GoverningCase => "governing_case",
            Self::Capacity => "capacity",
            Self::Acceptance => "acceptance",
            Self::FireBasis => "fire_basis",
            Self::DynamicDetermination => "dynamic_determination",
        }
    }
}

/// The check's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Tr3001Status {
    /// Passed.
    #[serde(rename = "pass")]
    Pass,
    /// Failed.
    #[serde(rename = "fail")]
    Fail,
    /// Informational.
    #[serde(rename = "info")]
    Info,
    /// Needs review.
    #[serde(rename = "needs_review")]
    NeedsReview,
}

impl Tr3001Status {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Info => "info",
            Self::NeedsReview => "needs_review",
        }
    }
}

/// One scenario, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct Tr3001Scenario {
    /// The overpressure cause.
    pub cause: String,
    /// The scenario's relief rate.
    pub relief_rate: MassRate,
    /// Whether the load was dynamically determined.
    pub dynamically_determined: bool,
    /// Whether the scenario is credible.
    pub credible: bool,
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tr3001Finding {
    /// Which check the row is.
    pub check: Tr3001Check,
    /// The check's status.
    pub status: Tr3001Status,
}

/// Result of `process.tr3001_compliance`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tr3001ComplianceResult {
    /// One row per check the class raised, in its order.
    pub findings: Vec<Tr3001Finding>,
    /// The number of passing checks.
    pub pass_count: f64,
    /// The number of needs-review checks.
    pub review_count: f64,
    /// The number of failing checks.
    pub fail_count: f64,
    /// Whether no check failed.
    pub compliant: bool,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The six checks over a study's scenario rows and sizing verdict.
///
/// # Errors
/// `InvalidInput` for a negative or non-finite rate or area, an accumulation fraction
/// outside `[0, 1]`, a cause the class does not name, or an acceptance group that is
/// half stated.
pub fn tr3001_compliance(
    scenarios: &[Tr3001Scenario],
    required_area_in2: Option<f64>,
    capacity_adequate: bool,
    acceptance_accepted: Option<bool>,
    accumulation_fraction: Option<f64>,
) -> Result<Tr3001ComplianceResult> {
    let spec = &model_gen::TR3001_COMPLIANCE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "required_area_in2" => required_area_in2,
            "accumulation_fraction" => accumulation_fraction,
            _ => None,
        },
        &mut warnings,
    )?;
    if acceptance_accepted.is_some() != accumulation_fraction.is_some() {
        return Err(AzothError::invalid_input(
            "acceptance_accepted",
            "the acceptance verdict and the accumulation fraction are stated together",
        ));
    }
    if let Some(area) = required_area_in2
        && (!area.is_finite() || area < 0.0)
    {
        return Err(AzothError::invalid_input(
            "required_area_in2",
            format!("a finite non-negative area is required, and {area} is not"),
        ));
    }
    let mut parsed = Vec::with_capacity(scenarios.len());
    for (index, scenario) in scenarios.iter().enumerate() {
        let rate = scenario.relief_rate.value;
        if !rate.is_finite() || rate < 0.0 {
            return Err(AzothError::invalid_input(
                format!("scenarios[{index}].relief_rate"),
                format!("a finite non-negative rate is required, and {rate} is not"),
            ));
        }
        let cause = ReliefCause::parse(&scenario.cause)?;
        parsed.push((
            cause,
            rate,
            scenario.dynamically_determined,
            scenario.credible,
        ));
    }

    let credible_count = parsed.iter().filter(|row| row.3).count();
    let governing = parsed.iter().fold(
        None,
        |current: Option<&(ReliefCause, f64, bool, bool)>, row| {
            if !row.3 {
                return current;
            }
            match current {
                Some(best) if best.1 >= row.1 => Some(best),
                _ => Some(row),
            }
        },
    );

    let mut findings = Vec::new();
    findings.push(Tr3001Finding {
        check: Tr3001Check::CredibleScenarios,
        status: if credible_count > 0 {
            Tr3001Status::Pass
        } else {
            Tr3001Status::Fail
        },
    });
    findings.push(Tr3001Finding {
        check: Tr3001Check::GoverningCase,
        status: if governing.is_some() {
            Tr3001Status::Pass
        } else {
            Tr3001Status::Fail
        },
    });
    if governing.is_some() {
        let status = match required_area_in2 {
            None => Tr3001Status::NeedsReview,
            Some(_) if capacity_adequate => Tr3001Status::Pass,
            Some(_) => Tr3001Status::Fail,
        };
        findings.push(Tr3001Finding {
            check: Tr3001Check::Capacity,
            status,
        });
    }
    if let Some(accepted) = acceptance_accepted {
        findings.push(Tr3001Finding {
            check: Tr3001Check::Acceptance,
            status: if accepted {
                Tr3001Status::Pass
            } else {
                Tr3001Status::Fail
            },
        });
    }
    if let Some((cause, _, dynamically_determined, _)) = governing {
        if *cause == ReliefCause::Fire {
            findings.push(Tr3001Finding {
                check: Tr3001Check::FireBasis,
                status: match accumulation_fraction {
                    Some(fraction) if fraction >= 0.20 => Tr3001Status::Pass,
                    _ => Tr3001Status::NeedsReview,
                },
            });
        }
        if *cause == ReliefCause::Fire || *cause == ReliefCause::BlockedOutlet {
            findings.push(Tr3001Finding {
                check: Tr3001Check::DynamicDetermination,
                status: if *dynamically_determined {
                    Tr3001Status::Pass
                } else {
                    Tr3001Status::NeedsReview
                },
            });
        }
    }

    let pass_count = findings
        .iter()
        .filter(|row| row.status == Tr3001Status::Pass)
        .count() as f64;
    let review_count = findings
        .iter()
        .filter(|row| row.status == Tr3001Status::NeedsReview)
        .count() as f64;
    let fail_count = findings
        .iter()
        .filter(|row| row.status == Tr3001Status::Fail)
        .count() as f64;
    Ok(Tr3001ComplianceResult {
        findings,
        pass_count,
        review_count,
        fail_count,
        compliant: fail_count == 0.0,
        warnings,
    })
}
