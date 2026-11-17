//! `process.s001_clause10_review` - the NORSOK S-001 Clause 10 review.
//!
//! Spec: `specs/models/process/s001_clause10_review.toml`. The arithmetic is NeqSim's
//! `process/safety/processsafetysystem/ProcessSafetySystemReviewEngine.java`.

use std::collections::BTreeMap;

use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::safety_review::{S001Criteria, s001_evaluate};

/// One function row a caller states.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewItem {
    /// The function id.
    pub function_id: String,
    /// The free-form function type.
    pub function_type: Option<String>,
    /// The protected equipment tag.
    pub equipment_tag: Option<String>,
}

/// One evidence row.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewEvidence {
    /// The function id.
    pub function_id: String,
    /// The evidence key.
    pub key: String,
    /// The value, absent for the class's null.
    pub value: Option<String>,
}

/// One reference row.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewReference {
    /// The function id.
    pub function_id: String,
    /// The reference text.
    pub reference: String,
}

/// One assessment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReviewAssessment {
    /// The item the assessment belongs to.
    pub function_id: String,
    /// The stable requirement identifier.
    pub requirement_id: String,
    /// The clause.
    pub clause: String,
    /// The status.
    pub status: ReviewStatus,
    /// The severity.
    pub severity: ReviewSeverity,
}

/// An assessment's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReviewStatus {
    /// Pass.
    #[serde(rename = "pass")]
    Pass,
    /// Warning.
    #[serde(rename = "warning")]
    Warning,
    /// Fail.
    #[serde(rename = "fail")]
    Fail,
    /// Info.
    #[serde(rename = "info")]
    Info,
    /// Not applicable.
    #[serde(rename = "not_applicable")]
    NotApplicable,
}

impl ReviewStatus {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warning => "warning",
            Self::Fail => "fail",
            Self::Info => "info",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// An assessment's severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReviewSeverity {
    /// Low.
    #[serde(rename = "low")]
    Low,
    /// Medium.
    #[serde(rename = "medium")]
    Medium,
    /// High.
    #[serde(rename = "high")]
    High,
    /// Critical.
    #[serde(rename = "critical")]
    Critical,
}

impl ReviewSeverity {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// One result's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Clause10Verdict {
    /// Pass.
    #[serde(rename = "pass")]
    Pass,
    /// Pass with warnings.
    #[serde(rename = "pass_with_warnings")]
    PassWithWarnings,
    /// Fail.
    #[serde(rename = "fail")]
    Fail,
}

impl Clause10Verdict {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::PassWithWarnings => "pass_with_warnings",
            Self::Fail => "fail",
        }
    }
}

/// One verdict row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReviewVerdict {
    /// The result's function id.
    pub function_id: String,
    /// The verdict.
    pub verdict: Clause10Verdict,
    /// The confidence.
    pub confidence: f64,
}

/// A coverage row's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CoverageType {
    /// PSD.
    #[serde(rename = "psd")]
    Psd,
    /// PSV.
    #[serde(rename = "psv")]
    Psv,
    /// Alarm.
    #[serde(rename = "alarm")]
    Alarm,
    /// SIF.
    #[serde(rename = "sif")]
    Sif,
    /// Secondary pressure protection.
    #[serde(rename = "secondary_pressure_protection")]
    SecondaryPressureProtection,
}

impl CoverageType {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Psd => "psd",
            Self::Psv => "psv",
            Self::Alarm => "alarm",
            Self::Sif => "sif",
            Self::SecondaryPressureProtection => "secondary_pressure_protection",
        }
    }
}

/// A coverage row's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CoverageStatus {
    /// Pass.
    #[serde(rename = "pass")]
    Pass,
    /// Warning.
    #[serde(rename = "warning")]
    Warning,
}

impl CoverageStatus {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warning => "warning",
        }
    }
}

/// One coverage row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReviewCoverage {
    /// The type token.
    pub function_type: CoverageType,
    /// The status.
    pub status: CoverageStatus,
}

/// The report's overall verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OverallVerdict {
    /// Pass.
    #[serde(rename = "pass")]
    Pass,
    /// Pass with warnings.
    #[serde(rename = "pass_with_warnings")]
    PassWithWarnings,
    /// Fail.
    #[serde(rename = "fail")]
    Fail,
}

impl OverallVerdict {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::PassWithWarnings => "pass_with_warnings",
            Self::Fail => "fail",
        }
    }
}

/// Result of `process.s001_clause10_review`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct S001Clause10ReviewResult {
    /// One row per assessment.
    pub assessments: Vec<ReviewAssessment>,
    /// One verdict per result.
    pub verdicts: Vec<ReviewVerdict>,
    /// The five coverage rows.
    pub coverage: Vec<ReviewCoverage>,
    /// The report's verdict.
    pub overall_verdict: OverallVerdict,
    /// The number of results.
    pub item_count: f64,
    /// The number of failing results.
    pub failed_item_count: f64,
    /// The number of pass-with-warnings results.
    pub warning_item_count: f64,
    /// The total number of assessments.
    pub assessment_count: f64,
    /// The number of failing assessments.
    pub fail_assessment_count: f64,
    /// The number of warning assessments.
    pub warning_assessment_count: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

struct Item {
    function_id: String,
    function_type: String,
    values: BTreeMap<String, Option<String>>,
    references: Vec<String>,
}

impl Item {
    fn contains_type(&self, tokens: &[&str]) -> bool {
        let kind = self.function_type.to_lowercase();
        tokens.iter().any(|token| kind.contains(token))
    }

    fn has(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    fn has_any(&self, keys: &[&str]) -> bool {
        keys.iter().any(|key| self.has(key))
    }

    fn get_bool(&self, keys: &[&str]) -> Option<bool> {
        for key in keys {
            let Some(value) = self.values.get(*key) else {
                continue;
            };
            let Some(text) = value else {
                continue;
            };
            let text = text.trim();
            let lower = text.to_lowercase();
            if lower == "true" || lower == "yes" || lower == "y" || text == "1" {
                return Some(true);
            }
            if lower == "false" || lower == "no" || lower == "n" || text == "0" {
                return Some(false);
            }
        }
        None
    }

    fn get_bool_default(&self, default: bool, keys: &[&str]) -> bool {
        self.get_bool(keys).unwrap_or(default)
    }

    fn get_double(&self, default: f64, keys: &[&str]) -> f64 {
        for key in keys {
            let Some(value) = self.values.get(*key) else {
                continue;
            };
            let Some(text) = value else {
                return default;
            };
            return parse_java_double(text).unwrap_or(default);
        }
        default
    }
}

/// Java's `Double.parseDouble` on a trimmed string, with the type suffix it accepts.
fn parse_java_double(raw: &str) -> Option<f64> {
    let text = raw.trim();
    let lower = text.to_ascii_lowercase();
    if lower == "nan" {
        return Some(f64::NAN);
    }
    if lower == "infinity" || lower == "+infinity" {
        return Some(f64::INFINITY);
    }
    if lower == "-infinity" {
        return Some(f64::NEG_INFINITY);
    }
    let body = text.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(text);
    body.parse::<f64>().ok()
}

fn is_psd(item: &Item) -> bool {
    item.contains_type(&["psd", "process shutdown", "shutdown"])
        || item.has_any(&["psdValveTag", "shutdownActionDefined", "psdValveFailsSafe"])
}

fn is_psv(item: &Item) -> bool {
    item.contains_type(&["psv", "relief", "safety valve"])
        || item.has_any(&[
            "psvTag",
            "psvSizingBasisDocumented",
            "requiredReliefLoadKgPerS",
        ])
}

fn is_alarm(item: &Item) -> bool {
    item.contains_type(&["alarm", "operator action"])
        || item.has_any(&["alarmTag", "alarmActionDefined", "operatorActionDefined"])
}

fn is_logic(item: &Item) -> bool {
    item.contains_type(&["sif", "logic", "sis", "psd"])
        || item.has_any(&[
            "logicSolverCertified",
            "logicSolverIndependent",
            "causeAndEffectTested",
        ])
}

fn is_secondary(item: &Item) -> bool {
    item.contains_type(&[
        "secondary_pressure",
        "secondary pressure",
        "10.4.7",
        "overpressure",
    ]) || item.has_any(&[
        "maximumEventPressureBara",
        "eventPressureBara",
        "demandFrequencyPerYear",
    ])
}

fn is_action_or_instrumented(item: &Item) -> bool {
    is_psd(item)
        || is_alarm(item)
        || is_logic(item)
        || is_secondary(item)
        || item.has_any(&["requiredResponseTimeSeconds", "actualResponseTimeSeconds"])
}

fn has_type(items: &[Item], token: &str) -> bool {
    let lower = token.to_lowercase();
    items.iter().any(|item| {
        item.function_type.to_lowercase().contains(&lower)
            || match token {
                "PSD" => is_psd(item),
                "PSV" => is_psv(item),
                "ALARM" => is_alarm(item),
                "SIF" => is_logic(item),
                "SECONDARY_PRESSURE_PROTECTION" => is_secondary(item),
                _ => false,
            }
    })
}

fn push(
    assessments: &mut Vec<ReviewAssessment>,
    function_id: &str,
    requirement_id: &str,
    clause: &str,
    status: ReviewStatus,
    severity: ReviewSeverity,
) {
    assessments.push(ReviewAssessment {
        function_id: function_id.to_string(),
        requirement_id: requirement_id.to_string(),
        clause: clause.to_string(),
        status,
        severity,
    });
}

fn required_bool(
    item: &Item,
    assessments: &mut Vec<ReviewAssessment>,
    requirement_id: &str,
    clause: &str,
    keys: &[&str],
) {
    match item.get_bool(keys) {
        Some(true) => push(
            assessments,
            &item.function_id,
            requirement_id,
            clause,
            ReviewStatus::Pass,
            ReviewSeverity::Low,
        ),
        Some(false) => push(
            assessments,
            &item.function_id,
            requirement_id,
            clause,
            ReviewStatus::Fail,
            ReviewSeverity::Critical,
        ),
        None => push(
            assessments,
            &item.function_id,
            requirement_id,
            clause,
            ReviewStatus::Warning,
            ReviewSeverity::High,
        ),
    }
}

fn not_applicable(
    item: &Item,
    assessments: &mut Vec<ReviewAssessment>,
    requirement_id: &str,
    clause: &str,
) {
    push(
        assessments,
        &item.function_id,
        requirement_id,
        clause,
        ReviewStatus::NotApplicable,
        ReviewSeverity::Low,
    );
}

fn compare_times(
    item: &Item,
    assessments: &mut Vec<ReviewAssessment>,
    requirement_id: &str,
    clause: &str,
    actual_key: &str,
    limit_key: &str,
) {
    let actual = item.get_double(f64::NAN, &[actual_key]);
    let limit = item.get_double(f64::NAN, &[limit_key]);
    if !actual.is_finite() || !limit.is_finite() {
        push(
            assessments,
            &item.function_id,
            requirement_id,
            clause,
            ReviewStatus::Warning,
            ReviewSeverity::Medium,
        );
    } else if actual <= limit {
        push(
            assessments,
            &item.function_id,
            requirement_id,
            clause,
            ReviewStatus::Pass,
            ReviewSeverity::Low,
        );
    } else {
        push(
            assessments,
            &item.function_id,
            requirement_id,
            clause,
            ReviewStatus::Fail,
            ReviewSeverity::High,
        );
    }
}

fn compare_minimum(
    item: &Item,
    assessments: &mut Vec<ReviewAssessment>,
    requirement_id: &str,
    clause: &str,
    minimum_key: &str,
    provided_key: &str,
) {
    let minimum = item.get_double(f64::NAN, &[minimum_key]);
    let provided = item.get_double(f64::NAN, &[provided_key]);
    if !minimum.is_finite() || !provided.is_finite() {
        push(
            assessments,
            &item.function_id,
            requirement_id,
            clause,
            ReviewStatus::Warning,
            ReviewSeverity::Medium,
        );
    } else if provided + 1.0e-12 >= minimum {
        push(
            assessments,
            &item.function_id,
            requirement_id,
            clause,
            ReviewStatus::Pass,
            ReviewSeverity::Low,
        );
    } else {
        push(
            assessments,
            &item.function_id,
            requirement_id,
            clause,
            ReviewStatus::Fail,
            ReviewSeverity::High,
        );
    }
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn evaluate_item(item: &Item) -> Vec<ReviewAssessment> {
    let mut assessments = Vec::new();
    // Safety lifecycle.
    required_bool(
        item,
        &mut assessments,
        "PSS-LIFECYCLE-HAZID-HAZOP-LOPA",
        "Safety lifecycle",
        &[
            "hazidHazopLopaCompleted",
            "hazardAnalysisCompleted",
            "riskAssessmentCompleted",
            "lopaCompleted",
        ],
    );
    required_bool(
        item,
        &mut assessments,
        "PSS-LIFECYCLE-SRS",
        "Safety lifecycle",
        &[
            "srsDefinesRequiredFunctions",
            "srsDocumented",
            "srsReferenceDocumented",
            "silRequirementsDocumented",
        ],
    );
    required_bool(
        item,
        &mut assessments,
        "PSS-LIFECYCLE-DESIGN-IMPLEMENTATION",
        "Safety lifecycle",
        &[
            "sisEsdFgsDesignImplemented",
            "safetySystemDesignImplemented",
            "designImplementationVerified",
            "causeAndEffectImplemented",
        ],
    );
    required_bool(
        item,
        &mut assessments,
        "PSS-LIFECYCLE-VERIFY-TEST-OPERATE",
        "Safety lifecycle",
        &[
            "verificationTestingOperationConfirmed",
            "verificationPlanDocumented",
            "proofTestProgramDocumented",
            "operationsMaintenancePlanDocumented",
        ],
    );

    // Role and interfaces.
    required_bool(
        item,
        &mut assessments,
        "PSS-10.1-ROLE",
        "10.1",
        &[
            "processSafetyRoleDefined",
            "roleDefined",
            "safetyFunctionDefined",
        ],
    );
    required_bool(
        item,
        &mut assessments,
        "PSS-10.1-INTERFACES",
        "10.1",
        &[
            "interfacesDefined",
            "systemInterfacesDefined",
            "interfaceRegisterDocumented",
        ],
    );

    // Means of protection.
    required_bool(
        item,
        &mut assessments,
        "PSS-10.3-PROTECTION-LAYERS",
        "10.3",
        &[
            "protectionLayersDocumented",
            "meansOfProtectionDocumented",
            "protectionStrategyDocumented",
        ],
    );
    required_bool(
        item,
        &mut assessments,
        "PSS-10.3-DESIGN-BASIS",
        "10.3",
        &[
            "designBasisDocumented",
            "scenarioBasisDocumented",
            "hazardBasisDocumented",
        ],
    );

    // Process-safety principles.
    required_bool(
        item,
        &mut assessments,
        "PSS-10.4.1-PRINCIPLES",
        "10.4.1",
        &[
            "processSafetyPrinciplesDocumented",
            "safeStateDefined",
            "failSafeStateDocumented",
        ],
    );
    required_bool(
        item,
        &mut assessments,
        "PSS-10.4.1-BYPASS",
        "10.4.1",
        &[
            "bypassManagementDocumented",
            "overrideManagementDocumented",
            "impairmentManagementDocumented",
        ],
    );

    // PSD valves.
    if is_psd(item) {
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.2-SHUTDOWN-ACTION",
            "10.4.2",
            &[
                "shutdownActionDefined",
                "psdActionDefined",
                "executiveActionDefined",
            ],
        );
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.2-FAILSAFE",
            "10.4.2",
            &[
                "psdValveFailsSafe",
                "valveFailsSafe",
                "finalElementFailsSafe",
            ],
        );
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.2-ISOLATION",
            "10.4.2",
            &[
                "psdValveIsolationAdequate",
                "shutdownIsolationAdequate",
                "leakTightClassDocumented",
            ],
        );
    } else {
        not_applicable(item, &mut assessments, "PSS-10.4.2-PSD", "10.4.2");
    }

    // PSV protection.
    if is_psv(item) {
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.3-SIZING-BASIS",
            "10.4.3",
            &[
                "psvSizingBasisDocumented",
                "reliefScenarioDocumented",
                "reliefLoadDocumented",
            ],
        );
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.3-PROTECTED-EQUIPMENT",
            "10.4.3",
            &[
                "protectedEquipmentDocumented",
                "protectedEquipmentTagDocumented",
            ],
        );
        let required = item.get_double(
            f64::NAN,
            &[
                "requiredReliefLoadKgPerS",
                "reliefLoadKgPerS",
                "requiredCapacityKgPerS",
            ],
        );
        let capacity = item.get_double(
            f64::NAN,
            &[
                "psvCapacityKgPerS",
                "ratedCapacityKgPerS",
                "reliefCapacityKgPerS",
            ],
        );
        if required.is_finite() && capacity.is_finite() {
            let status = if capacity + 1.0e-12 >= required {
                ReviewStatus::Pass
            } else {
                ReviewStatus::Fail
            };
            let severity = if status == ReviewStatus::Pass {
                ReviewSeverity::Low
            } else {
                ReviewSeverity::Critical
            };
            push(
                &mut assessments,
                &item.function_id,
                "PSS-10.4.3-CAPACITY",
                "10.4.3",
                status,
                severity,
            );
        } else {
            push(
                &mut assessments,
                &item.function_id,
                "PSS-10.4.3-CAPACITY",
                "10.4.3",
                ReviewStatus::Warning,
                ReviewSeverity::High,
            );
        }
    } else {
        not_applicable(item, &mut assessments, "PSS-10.4.3-PSV", "10.4.3");
    }

    // Alarm actions.
    if is_alarm(item) {
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.4-ACTION",
            "10.4.4",
            &[
                "alarmActionDefined",
                "operatorActionDefined",
                "alarmResponseDefined",
            ],
        );
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.4-SETPOINT",
            "10.4.4",
            &[
                "alarmSetpointDocumented",
                "tripSetpointDocumented",
                "actionLimitDocumented",
            ],
        );
        compare_times(
            item,
            &mut assessments,
            "PSS-10.4.4-OPERATOR-TIME",
            "10.4.4",
            "operatorResponseTimeSeconds",
            "availableOperatorResponseTimeSeconds",
        );
    } else {
        not_applicable(item, &mut assessments, "PSS-10.4.4-ALARM", "10.4.4");
    }

    // Response time.
    if is_action_or_instrumented(item) {
        compare_times(
            item,
            &mut assessments,
            "PSS-10.4.5-RESPONSE",
            "10.4.5",
            "actualResponseTimeSeconds",
            "requiredResponseTimeSeconds",
        );
    } else {
        not_applicable(item, &mut assessments, "PSS-10.4.5-RESPONSE", "10.4.5");
    }

    // Logic solver.
    if is_logic(item) {
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.6-CERTIFIED",
            "10.4.6",
            &[
                "logicSolverCertified",
                "logicSolverSuitable",
                "sisLogicSolverCertified",
            ],
        );
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.6-INDEPENDENCE",
            "10.4.6",
            &[
                "logicSolverIndependent",
                "independentFromControlSystem",
                "sisBpcsIndependenceDocumented",
            ],
        );
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.6-TESTED",
            "10.4.6",
            &[
                "causeAndEffectTested",
                "logicFunctionTested",
                "sifProofTested",
            ],
        );
    } else {
        not_applicable(item, &mut assessments, "PSS-10.4.6-LOGIC", "10.4.6");
    }

    // Secondary pressure protection.
    if is_secondary(item) {
        let event = item.get_double(
            f64::NAN,
            &[
                "maximumEventPressureBara",
                "eventPressureBara",
                "protectedSystemMaximumPressureBara",
            ],
        );
        let design = item.get_double(
            f64::NAN,
            &["designPressureBara", "protectedEquipmentDesignPressureBara"],
        );
        let test = item.get_double(
            f64::NAN,
            &[
                "testPressureBara",
                "hydrotestPressureBara",
                "pressureTestBara",
            ],
        );
        let demand = item.get_double(
            f64::NAN,
            &[
                "demandFrequencyPerYear",
                "annualDemandFrequency",
                "eventFrequencyPerYear",
            ],
        );
        let target = item.get_double(
            f64::NAN,
            &["targetFrequencyPerYear", "maximumAllowedFrequencyPerYear"],
        );
        let assessed = item.get_bool(&[
            "reliefLeakageAssessed",
            "psvLeakageAssessed",
            "reliefValveLeakageDocumented",
        ]);
        let safe = item.get_bool(&[
            "reliefLeakageToSafeLocation",
            "psvLeakageToSafeLocation",
            "leakageRoutedSafe",
        ]);
        let proof = item.get_double(
            f64::NAN,
            &[
                "proofTestIntervalMonths",
                "testIntervalMonths",
                "proofTestMonths",
            ],
        );
        let empty = !event.is_finite()
            && !design.is_finite()
            && !test.is_finite()
            && !demand.is_finite()
            && !proof.is_finite()
            && assessed.is_none()
            && safe.is_none();
        if empty {
            push(
                &mut assessments,
                &item.function_id,
                "PSS-10.4.7-SECONDARY",
                "10.4.7",
                ReviewStatus::Warning,
                ReviewSeverity::High,
            );
        } else {
            let outcome = s001_evaluate(S001Criteria {
                maximum_event_pressure: finite(event),
                design_pressure: finite(design),
                test_pressure: finite(test),
                demand_frequency_per_year: finite(demand),
                target_frequency_per_year: finite(target),
                relief_leakage_assessed: assessed,
                relief_leakage_to_safe_location: safe,
                proof_test_interval_months: finite(proof),
            });
            if outcome.acceptable {
                push(
                    &mut assessments,
                    &item.function_id,
                    "PSS-10.4.7-SECONDARY",
                    "10.4.7",
                    ReviewStatus::Pass,
                    ReviewSeverity::Low,
                );
            } else if !outcome.pressure_basis_complete || !outcome.frequency_configured {
                push(
                    &mut assessments,
                    &item.function_id,
                    "PSS-10.4.7-SECONDARY",
                    "10.4.7",
                    ReviewStatus::Warning,
                    ReviewSeverity::High,
                );
            } else {
                push(
                    &mut assessments,
                    &item.function_id,
                    "PSS-10.4.7-SECONDARY",
                    "10.4.7",
                    ReviewStatus::Fail,
                    ReviewSeverity::Critical,
                );
            }
        }
    } else {
        not_applicable(item, &mut assessments, "PSS-10.4.7-SECONDARY", "10.4.7");
    }

    // Utility dependencies.
    required_bool(
        item,
        &mut assessments,
        "PSS-10.4.8-UTILITY-LIST",
        "10.4.8",
        &[
            "requiredUtilitiesIdentified",
            "utilityDependenciesIdentified",
            "utilityMatrixDocumented",
        ],
    );
    let utility_dependent = item.get_bool(&[
        "utilityDependent",
        "dependsOnUtility",
        "requiresUtilityForAction",
    ]);
    if utility_dependent == Some(true) {
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.8-UTILITY-LOSS",
            "10.4.8",
            &[
                "failSafeOnUtilityLoss",
                "safeOnUtilityFailure",
                "utilityFailureSafeStateDocumented",
            ],
        );
    } else {
        push(
            &mut assessments,
            &item.function_id,
            "PSS-10.4.8-UTILITY-LOSS",
            "10.4.8",
            ReviewStatus::Info,
            ReviewSeverity::Low,
        );
    }

    // PSD principles.
    if is_psd(item) {
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.9-PSD-INDEPENDENCE",
            "10.4.9",
            &[
                "psdIndependentFromControl",
                "psdIndependenceDocumented",
                "independentFromControlSystem",
            ],
        );
        required_bool(
            item,
            &mut assessments,
            "PSS-10.4.9-PSD-MANUAL",
            "10.4.9",
            &[
                "manualShutdownAvailable",
                "manualInitiationDocumented",
                "manualPsdAvailable",
            ],
        );
    } else {
        not_applicable(item, &mut assessments, "PSS-10.4.9-PSD", "10.4.9");
    }

    // Survivability.
    required_bool(
        item,
        &mut assessments,
        "PSS-10.4.10-SURVIVABILITY",
        "10.4.10",
        &[
            "survivabilityRequirementDocumented",
            "fireBlastSurvivabilityDocumented",
            "survivabilityDocumented",
        ],
    );
    compare_minimum(
        item,
        &mut assessments,
        "PSS-10.4.10-ENDURANCE",
        "10.4.10",
        "requiredSurvivabilityTimeMin",
        "survivabilityTimeMin",
    );

    // Operational evidence.
    if item.has_any(&[
        "tagreaderSource",
        "instrumentDataSource",
        "bypassActive",
        "overrideActive",
        "proofTestOverdue",
        "tripDemandFailures",
        "observedResponseTimeSeconds",
    ]) {
        let bypass = item.get_bool_default(false, &["bypassActive", "sifBypassed", "psdBypassed"]);
        let override_ =
            item.get_bool_default(false, &["overrideActive", "sifOverridden", "psdOverridden"]);
        let overdue = item.get_bool_default(false, &["proofTestOverdue", "testOverdue"]);
        let failures = item.get_double(0.0, &["tripDemandFailures", "failedDemandCount"]);
        if bypass || override_ {
            push(
                &mut assessments,
                &item.function_id,
                "PSS-OPS-INSTRUMENT-DATA",
                "Operational evidence",
                ReviewStatus::Fail,
                ReviewSeverity::High,
            );
        } else if overdue || failures > 0.0 {
            push(
                &mut assessments,
                &item.function_id,
                "PSS-OPS-INSTRUMENT-DATA",
                "Operational evidence",
                ReviewStatus::Warning,
                ReviewSeverity::Medium,
            );
        } else {
            push(
                &mut assessments,
                &item.function_id,
                "PSS-OPS-INSTRUMENT-DATA",
                "Operational evidence",
                ReviewStatus::Pass,
                ReviewSeverity::Low,
            );
        }
    } else {
        push(
            &mut assessments,
            &item.function_id,
            "PSS-OPS-INSTRUMENT-DATA",
            "Operational evidence",
            ReviewStatus::Info,
            ReviewSeverity::Low,
        );
    }

    // Evidence quality.
    if item.references.is_empty() {
        push(
            &mut assessments,
            &item.function_id,
            "PSS-EVIDENCE-SOURCES",
            "Evidence quality",
            ReviewStatus::Warning,
            ReviewSeverity::Medium,
        );
    } else {
        push(
            &mut assessments,
            &item.function_id,
            "PSS-EVIDENCE-SOURCES",
            "Evidence quality",
            ReviewStatus::Pass,
            ReviewSeverity::Low,
        );
    }

    assessments
}

fn confidence(item: &Item) -> f64 {
    let mut score: f64 = 0.15;
    if !item.references.is_empty() {
        score += 0.20;
    }
    if item.has_any(&["designBasisDocumented", "scenarioBasisDocumented"]) {
        score += 0.15;
    }
    if item.has_any(&[
        "hazidHazopLopaCompleted",
        "srsDefinesRequiredFunctions",
        "verificationTestingOperationConfirmed",
    ]) {
        score += 0.10;
    }
    if item.has_any(&[
        "requiredResponseTimeSeconds",
        "actualResponseTimeSeconds",
        "operatorResponseTimeSeconds",
    ]) {
        score += 0.15;
    }
    if item.has_any(&[
        "logicSolverCertified",
        "psvCapacityKgPerS",
        "maximumEventPressureBara",
    ]) {
        score += 0.15;
    }
    if item.has_any(&["tagreaderSource", "instrumentDataSource", "bypassActive"]) {
        score += 0.10;
    }
    score.min(1.0)
}

fn verdict(assessments: &[ReviewAssessment]) -> Clause10Verdict {
    if assessments
        .iter()
        .any(|assessment| assessment.status == ReviewStatus::Fail)
    {
        Clause10Verdict::Fail
    } else if assessments
        .iter()
        .any(|assessment| assessment.status == ReviewStatus::Warning)
    {
        Clause10Verdict::PassWithWarnings
    } else {
        Clause10Verdict::Pass
    }
}

fn coverage_rows(items: &[Item]) -> Vec<ReviewCoverage> {
    [
        (CoverageType::Psd, "PSD"),
        (CoverageType::Psv, "PSV"),
        (CoverageType::Alarm, "ALARM"),
        (CoverageType::Sif, "SIF"),
        (
            CoverageType::SecondaryPressureProtection,
            "SECONDARY_PRESSURE_PROTECTION",
        ),
    ]
    .into_iter()
    .map(|(function_type, token)| ReviewCoverage {
        function_type,
        status: if has_type(items, token) {
            CoverageStatus::Pass
        } else {
            CoverageStatus::Warning
        },
    })
    .collect()
}

fn coverage_assessments(rows: &[ReviewCoverage]) -> Vec<ReviewAssessment> {
    let requirements = [
        ("PSS-COVERAGE-PSD", "10.4.2"),
        ("PSS-COVERAGE-PSV", "10.4.3"),
        ("PSS-COVERAGE-ALARM", "10.4.4"),
        ("PSS-COVERAGE-SIF", "10.4.6"),
        ("PSS-COVERAGE-SECONDARY", "10.4.7"),
    ];
    rows.iter()
        .zip(requirements)
        .map(|(row, (requirement_id, clause))| ReviewAssessment {
            function_id: "CLAUSE10-COVERAGE".to_string(),
            requirement_id: requirement_id.to_string(),
            clause: clause.to_string(),
            status: match row.status {
                CoverageStatus::Pass => ReviewStatus::Pass,
                CoverageStatus::Warning => ReviewStatus::Warning,
            },
            severity: match row.status {
                CoverageStatus::Pass => ReviewSeverity::Low,
                CoverageStatus::Warning => ReviewSeverity::Medium,
            },
        })
        .collect()
}

/// The Clause 10 review.
///
/// # Errors
/// `InvalidInput` for a duplicate function id, a duplicate evidence key, or an evidence or
/// reference row naming no item.
pub fn s001_clause10_review(
    items: Option<&[ReviewItem]>,
    evidence: Option<&[ReviewEvidence]>,
    references: Option<&[ReviewReference]>,
) -> Result<S001Clause10ReviewResult> {
    let spec = &model_gen::S001_CLAUSE10_REVIEW_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    let mut reviewed: Vec<Item> = Vec::new();
    for row in items.unwrap_or(&[]) {
        let function_id = row.function_id.clone();
        if reviewed.iter().any(|item| item.function_id == function_id) {
            return Err(AzothError::invalid_input(
                "items.function_id",
                format!("`{function_id}` appears more than once"),
            ));
        }
        reviewed.push(Item {
            function_id,
            function_type: row.function_type.clone().unwrap_or_default(),
            values: BTreeMap::new(),
            references: Vec::new(),
        });
    }

    for (index, row) in evidence.unwrap_or(&[]).iter().enumerate() {
        let Some(item) = reviewed
            .iter_mut()
            .find(|item| item.function_id == row.function_id)
        else {
            return Err(AzothError::invalid_input(
                format!("evidence[{index}].function_id"),
                format!("`{}` names no item row", row.function_id),
            ));
        };
        if item.values.contains_key(&row.key) {
            return Err(AzothError::invalid_input(
                format!("evidence[{index}].key"),
                format!(
                    "`{}` appears more than once for `{}`",
                    row.key, row.function_id
                ),
            ));
        }
        item.values.insert(row.key.clone(), row.value.clone());
    }

    for (index, row) in references.unwrap_or(&[]).iter().enumerate() {
        let Some(item) = reviewed
            .iter_mut()
            .find(|item| item.function_id == row.function_id)
        else {
            return Err(AzothError::invalid_input(
                format!("references[{index}].function_id"),
                format!("`{}` names no item row", row.function_id),
            ));
        };
        item.references.push(row.reference.clone());
    }

    let coverage = coverage_rows(&reviewed);
    let mut assessments = coverage_assessments(&coverage);
    let coverage_verdict = verdict(&assessments);
    let mut verdicts = vec![ReviewVerdict {
        function_id: "CLAUSE10-COVERAGE".to_string(),
        verdict: coverage_verdict,
        confidence: 1.0,
    }];

    for item in &reviewed {
        let item_assessments = evaluate_item(item);
        verdicts.push(ReviewVerdict {
            function_id: item.function_id.clone(),
            verdict: verdict(&item_assessments),
            confidence: confidence(item),
        });
        assessments.extend(item_assessments);
    }

    let overall_verdict = if verdicts
        .iter()
        .any(|row| row.verdict == Clause10Verdict::Fail)
    {
        OverallVerdict::Fail
    } else if verdicts
        .iter()
        .any(|row| row.verdict == Clause10Verdict::PassWithWarnings)
    {
        OverallVerdict::PassWithWarnings
    } else {
        OverallVerdict::Pass
    };

    Ok(S001Clause10ReviewResult {
        item_count: verdicts.len() as f64,
        failed_item_count: verdicts
            .iter()
            .filter(|row| row.verdict == Clause10Verdict::Fail)
            .count() as f64,
        warning_item_count: verdicts
            .iter()
            .filter(|row| row.verdict == Clause10Verdict::PassWithWarnings)
            .count() as f64,
        assessment_count: assessments.len() as f64,
        fail_assessment_count: assessments
            .iter()
            .filter(|row| row.status == ReviewStatus::Fail)
            .count() as f64,
        warning_assessment_count: assessments
            .iter()
            .filter(|row| row.status == ReviewStatus::Warning)
            .count() as f64,
        overall_verdict,
        assessments,
        verdicts,
        coverage,
        warnings,
    })
}
