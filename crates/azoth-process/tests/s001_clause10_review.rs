//! `process.s001_clause10_review` against
//! `validation/neqsim/captures/process_s001_clause10_review.tsv`.

use azoth_process::models::s001_clause10_review::{
    Clause10Verdict, OverallVerdict, ReviewAssessment, ReviewEvidence, ReviewItem, ReviewReference,
    ReviewSeverity, ReviewStatus, S001Clause10ReviewResult, s001_clause10_review,
};

fn item(id: &str, kind: &str) -> ReviewItem {
    ReviewItem {
        function_id: id.to_string(),
        function_type: Some(kind.to_string()),
        equipment_tag: Some("V-100".to_string()),
    }
}

fn ev(id: &str, key: &str, value: &str) -> ReviewEvidence {
    ReviewEvidence {
        function_id: id.to_string(),
        key: key.to_string(),
        value: Some(value.to_string()),
    }
}

fn base(id: &str) -> Vec<ReviewEvidence> {
    [
        ("processSafetyRoleDefined", "true"),
        ("hazidHazopLopaCompleted", "true"),
        ("srsDefinesRequiredFunctions", "true"),
        ("sisEsdFgsDesignImplemented", "true"),
        ("verificationTestingOperationConfirmed", "true"),
        ("interfacesDefined", "true"),
        ("protectionLayersDocumented", "true"),
        ("designBasisDocumented", "true"),
        ("processSafetyPrinciplesDocumented", "true"),
        ("bypassManagementDocumented", "true"),
        ("requiredUtilitiesIdentified", "true"),
        ("utilityDependent", "true"),
        ("failSafeOnUtilityLoss", "true"),
        ("survivabilityRequirementDocumented", "true"),
        ("requiredSurvivabilityTimeMin", "30.0"),
        ("survivabilityTimeMin", "60.0"),
        ("tagreaderSource", "synthetic instrument data"),
        ("overrideActive", "false"),
        ("proofTestOverdue", "false"),
        ("tripDemandFailures", "0.0"),
    ]
    .into_iter()
    .map(|(key, value)| ev(id, key, value))
    .collect()
}

fn references(id: &str) -> [ReviewReference; 2] {
    [
        ReviewReference {
            function_id: id.to_string(),
            reference: "synthetic C&E".to_string(),
        },
        ReviewReference {
            function_id: id.to_string(),
            reference: "synthetic SRS".to_string(),
        },
    ]
}

fn run(
    items: Option<&[ReviewItem]>,
    evidence: Option<&[ReviewEvidence]>,
    references: Option<&[ReviewReference]>,
) -> S001Clause10ReviewResult {
    s001_clause10_review(items, evidence, references).expect("the captured state is valid")
}

fn assessment<'a>(
    result: &'a S001Clause10ReviewResult,
    function_id: &str,
    requirement_id: &str,
) -> &'a ReviewAssessment {
    result
        .assessments
        .iter()
        .find(|row| row.function_id == function_id && row.requirement_id == requirement_id)
        .expect("the assessment is present")
}

fn example() -> S001Clause10ReviewResult {
    let items = [
        item("PSD-1001", "PSD"),
        item("PSV-1001", "PSV"),
        item("PAHH-1001", "ALARM"),
        item("SIF-1001", "SECONDARY_PRESSURE_PROTECTION"),
    ];
    let mut evidence = Vec::new();
    evidence.extend(base("PSD-1001"));
    evidence.extend([
        ev("PSD-1001", "shutdownActionDefined", "true"),
        ev("PSD-1001", "psdValveFailsSafe", "true"),
        ev("PSD-1001", "psdValveIsolationAdequate", "true"),
        ev("PSD-1001", "requiredResponseTimeSeconds", "30.0"),
        ev("PSD-1001", "actualResponseTimeSeconds", "18.0"),
        ev("PSD-1001", "logicSolverCertified", "true"),
        ev("PSD-1001", "logicSolverIndependent", "true"),
        ev("PSD-1001", "causeAndEffectTested", "true"),
        ev("PSD-1001", "psdIndependentFromControl", "true"),
        ev("PSD-1001", "manualShutdownAvailable", "true"),
    ]);
    evidence.extend(base("PSV-1001"));
    evidence.extend([
        ev("PSV-1001", "psvSizingBasisDocumented", "true"),
        ev("PSV-1001", "reliefScenarioDocumented", "true"),
        ev("PSV-1001", "protectedEquipmentDocumented", "true"),
        ev("PSV-1001", "requiredReliefLoadKgPerS", "10.0"),
        ev("PSV-1001", "psvCapacityKgPerS", "14.0"),
    ]);
    evidence.extend(base("PAHH-1001"));
    evidence.extend([
        ev("PAHH-1001", "alarmActionDefined", "true"),
        ev("PAHH-1001", "alarmSetpointDocumented", "true"),
        ev("PAHH-1001", "operatorResponseTimeSeconds", "120.0"),
        ev("PAHH-1001", "availableOperatorResponseTimeSeconds", "300.0"),
        ev("PAHH-1001", "requiredResponseTimeSeconds", "300.0"),
        ev("PAHH-1001", "actualResponseTimeSeconds", "120.0"),
    ]);
    evidence.extend(base("SIF-1001"));
    evidence.extend([
        ev("SIF-1001", "logicSolverCertified", "true"),
        ev("SIF-1001", "logicSolverIndependent", "true"),
        ev("SIF-1001", "causeAndEffectTested", "true"),
        ev("SIF-1001", "maximumEventPressureBara", "120.0"),
        ev("SIF-1001", "designPressureBara", "100.0"),
        ev("SIF-1001", "testPressureBara", "150.0"),
        ev("SIF-1001", "demandFrequencyPerYear", "1.0e-4"),
        ev("SIF-1001", "reliefLeakageAssessed", "true"),
        ev("SIF-1001", "reliefLeakageToSafeLocation", "true"),
        ev("SIF-1001", "proofTestIntervalMonths", "6.0"),
        ev("SIF-1001", "requiredResponseTimeSeconds", "10.0"),
        ev("SIF-1001", "actualResponseTimeSeconds", "5.0"),
    ]);
    let references: Vec<ReviewReference> = ["PSD-1001", "PSV-1001", "PAHH-1001", "SIF-1001"]
        .into_iter()
        .flat_map(references)
        .collect();
    run(Some(&items), Some(&evidence), Some(&references))
}

#[test]
fn the_complete_example_passes_every_verdict() {
    let result = example();
    assert_eq!(result.overall_verdict, OverallVerdict::Pass);
    assert_eq!(result.item_count, 5.0);
    assert_eq!(result.failed_item_count, 0.0);
    assert_eq!(result.warning_item_count, 0.0);
    assert_eq!(result.assessment_count, 108.0);
    assert_eq!(result.fail_assessment_count, 0.0);
    assert_eq!(result.warning_assessment_count, 0.0);
    let verdicts: Vec<(&str, Clause10Verdict, f64)> = result
        .verdicts
        .iter()
        .map(|row| (row.function_id.as_str(), row.verdict, row.confidence))
        .collect();
    assert_eq!(
        verdicts,
        vec![
            ("CLAUSE10-COVERAGE", Clause10Verdict::Pass, 1.0),
            ("PSD-1001", Clause10Verdict::Pass, 1.0),
            ("PSV-1001", Clause10Verdict::Pass, 0.85),
            ("PAHH-1001", Clause10Verdict::Pass, 0.85),
            ("SIF-1001", Clause10Verdict::Pass, 1.0),
        ]
    );
    for row in &result.coverage {
        assert_eq!(
            row.status,
            azoth_process::models::s001_clause10_review::CoverageStatus::Pass
        );
    }
    assert_eq!(
        assessment(&result, "SIF-1001", "PSS-10.4.7-SECONDARY").status,
        ReviewStatus::Pass
    );
}

#[test]
fn the_secondary_failure_fails_the_item_and_warns_the_coverage() {
    let items = [item("SIF-HP-001", "SECONDARY_PRESSURE_PROTECTION")];
    let mut evidence = base("SIF-HP-001");
    evidence.extend([
        ev("SIF-HP-001", "logicSolverCertified", "true"),
        ev("SIF-HP-001", "logicSolverIndependent", "true"),
        ev("SIF-HP-001", "causeAndEffectTested", "true"),
        ev("SIF-HP-001", "maximumEventPressureBara", "175.0"),
        ev("SIF-HP-001", "designPressureBara", "100.0"),
        ev("SIF-HP-001", "testPressureBara", "150.0"),
        ev("SIF-HP-001", "demandFrequencyPerYear", "1.0e-3"),
        ev("SIF-HP-001", "reliefLeakageAssessed", "true"),
        ev("SIF-HP-001", "reliefLeakageToSafeLocation", "true"),
        ev("SIF-HP-001", "proofTestIntervalMonths", "6.0"),
        ev("SIF-HP-001", "requiredResponseTimeSeconds", "10.0"),
        ev("SIF-HP-001", "actualResponseTimeSeconds", "5.0"),
    ]);
    let references = references("SIF-HP-001");
    let result = run(Some(&items), Some(&evidence), Some(&references));
    assert_eq!(result.overall_verdict, OverallVerdict::Fail);
    assert_eq!(result.item_count, 2.0);
    assert_eq!(result.failed_item_count, 1.0);
    assert_eq!(result.warning_item_count, 1.0);
    assert_eq!(result.assessment_count, 30.0);
    assert_eq!(result.fail_assessment_count, 1.0);
    assert_eq!(result.verdicts[1].verdict, Clause10Verdict::Fail);
    let secondary = assessment(&result, "SIF-HP-001", "PSS-10.4.7-SECONDARY");
    assert_eq!(secondary.status, ReviewStatus::Fail);
    assert_eq!(secondary.severity, ReviewSeverity::Critical);
}

#[test]
fn the_empty_input_answers_the_coverage_result_alone() {
    let result = run(None, None, None);
    assert_eq!(result.overall_verdict, OverallVerdict::PassWithWarnings);
    assert_eq!(result.item_count, 1.0);
    assert_eq!(result.failed_item_count, 0.0);
    assert_eq!(result.warning_item_count, 1.0);
    assert_eq!(result.assessment_count, 5.0);
    assert_eq!(result.warning_assessment_count, 5.0);
    assert_eq!(result.verdicts.len(), 1);
    assert_eq!(result.verdicts[0].function_id, "CLAUSE10-COVERAGE");
    assert_eq!(
        result.verdicts[0].verdict,
        Clause10Verdict::PassWithWarnings
    );
}

#[test]
fn a_gated_check_is_not_applicable_for_an_other_item() {
    let items = [item("OTHER-001", "OTHER")];
    let mut evidence = base("OTHER-001");
    evidence.push(ev("OTHER-001", "hazardBasisDocumented", "true"));
    let references = references("OTHER-001");
    let result = run(Some(&items), Some(&evidence), Some(&references));
    for (requirement, clause) in [
        ("PSS-10.4.2-PSD", "10.4.2"),
        ("PSS-10.4.3-PSV", "10.4.3"),
        ("PSS-10.4.4-ALARM", "10.4.4"),
    ] {
        let row = assessment(&result, "OTHER-001", requirement);
        assert_eq!(row.status, ReviewStatus::NotApplicable, "{requirement}");
        assert_eq!(row.clause, clause);
    }
}

#[test]
fn the_utility_loss_check_is_info_without_a_positive_dependency() {
    for value in [None, Some("false")] {
        let items = [item("UTIL-001", "OTHER")];
        let mut evidence = vec![ev("UTIL-001", "requiredUtilitiesIdentified", "true")];
        if let Some(value) = value {
            evidence.push(ev("UTIL-001", "utilityDependent", value));
        }
        let references = references("UTIL-001");
        let result = run(Some(&items), Some(&evidence), Some(&references));
        assert_eq!(
            assessment(&result, "UTIL-001", "PSS-10.4.8-UTILITY-LOSS").status,
            ReviewStatus::Info,
            "{value:?}"
        );
    }
}

#[test]
fn the_first_alias_present_decides() {
    let items = [item("PSV-ALIAS", "PSV")];
    let mut evidence = base("PSV-ALIAS");
    evidence.extend([
        ev("PSV-ALIAS", "psvSizingBasisDocumented", "true"),
        ev("PSV-ALIAS", "protectedEquipmentDocumented", "true"),
        ev("PSV-ALIAS", "requiredReliefLoadKgPerS", "not-a-number"),
        ev("PSV-ALIAS", "reliefLoadKgPerS", "10.0"),
        ev("PSV-ALIAS", "psvCapacityKgPerS", "14.0"),
    ]);
    let references = references("PSV-ALIAS");
    let result = run(Some(&items), Some(&evidence), Some(&references));
    assert_eq!(
        assessment(&result, "PSV-ALIAS", "PSS-10.4.3-CAPACITY").status,
        ReviewStatus::Warning
    );
}

#[test]
fn the_java_coercion_is_reproduced() {
    let items = [item("PSV-COERCE", "PSV")];
    let mut evidence = base("PSV-COERCE");
    evidence.extend([
        ev("PSV-COERCE", "psvSizingBasisDocumented", "true"),
        ev("PSV-COERCE", "protectedEquipmentDocumented", "true"),
        ev("PSV-COERCE", "requiredReliefLoadKgPerS", "10.0"),
        ev("PSV-COERCE", "psvCapacityKgPerS", "14.0f"),
    ]);
    let references = references("PSV-COERCE");
    let result = run(Some(&items), Some(&evidence), Some(&references));
    assert_eq!(
        assessment(&result, "PSV-COERCE", "PSS-10.4.3-CAPACITY").status,
        ReviewStatus::Pass
    );

    let mut nan = base("PSV-NAN");
    nan.extend([
        ev("PSV-NAN", "psvSizingBasisDocumented", "true"),
        ev("PSV-NAN", "protectedEquipmentDocumented", "true"),
        ev("PSV-NAN", "requiredReliefLoadKgPerS", "NaN"),
        ev("PSV-NAN", "psvCapacityKgPerS", "14.0"),
    ]);
    let items = [item("PSV-NAN", "PSV")];
    let result = run(Some(&items), Some(&nan), None);
    assert_eq!(
        assessment(&result, "PSV-NAN", "PSS-10.4.3-CAPACITY").status,
        ReviewStatus::Warning
    );
}

#[test]
fn a_duplicate_id_and_an_unknown_evidence_row_are_refused() {
    let items = [item("DUP-001", "PSD"), item("DUP-001", "PSV")];
    assert!(s001_clause10_review(Some(&items), None, None).is_err());
    let items = [item("PSD-1", "PSD")];
    let evidence = [ev("PSD-2", "shutdownActionDefined", "true")];
    assert!(s001_clause10_review(Some(&items), Some(&evidence), None).is_err());
}
