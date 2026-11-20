//! `process.open_drain_review` against
//! `validation/neqsim/captures/process_open_drain_review.tsv`.

use azoth_process::models::open_drain_review::{
    OpenDrainEvidence, OpenDrainItem, OpenDrainReference, OpenDrainReviewResult, OpenDrainSeverity,
    OpenDrainStandard, OpenDrainStatus, OpenDrainVerdict, open_drain_review,
};

const ORDER: [&str; 19] = [
    "OD-S001-9.4.1-STANDARDS",
    "OD-S001-9.4.1-COLLECTION",
    "OD-S001-9.4.1-CAPACITY",
    "OD-S001-9.4.1-BACKFLOW",
    "OD-S001-9.4.1-BUNDING",
    "OD-S001-9.4.1-FLOATER",
    "OD-S001-9.4.2-INERTING",
    "OD-S001-9.4.2-CLOSED-OPEN",
    "OD-S001-9.4.2-HAZ-NONHAZ",
    "OD-S001-9.4.2-COMMON-SUMP",
    "OD-S001-9.4.2-SEAL-BACKPRESSURE",
    "OD-S001-9.4.2-VENT",
    "OD-S001-9.4.2-FIRE-SEALS",
    "OD-S001-9.4.2-HELIDECK",
    "OD-S001-9.4.2-DRILLING-PROD",
    "OD-S001-9.4.2-TOTE",
    "OD-S001-9.4.3-FIRE-DIVISION",
    "OD-S001-9.4.1-UTILITY",
    "OD-OPS-TAGREADER",
];

fn item(area_id: &str, area_type: &str) -> OpenDrainItem {
    OpenDrainItem {
        area_id: area_id.to_string(),
        area_type: Some(area_type.to_string()),
        drain_system_type: None,
    }
}

fn evidence(area_id: &str, rows: &[(&str, &str)]) -> Vec<OpenDrainEvidence> {
    rows.iter()
        .map(|(key, value)| OpenDrainEvidence {
            area_id: area_id.to_string(),
            key: (*key).to_string(),
            value: Some((*value).to_string()),
        })
        .collect()
}

fn run(
    items: &[OpenDrainItem],
    evidence: &[OpenDrainEvidence],
    standards: &[OpenDrainStandard],
    references: &[OpenDrainReference],
) -> OpenDrainReviewResult {
    open_drain_review(
        Some("open-drain review"),
        None,
        Some(items),
        Some(evidence),
        Some(standards),
        Some(references),
    )
    .expect("the review evaluates")
}

fn statuses(result: &OpenDrainReviewResult) -> Vec<OpenDrainStatus> {
    result.assessments.iter().map(|row| row.status).collect()
}

fn assert_counts(result: &OpenDrainReviewResult, fails: f64, warnings: f64) {
    assert_eq!(result.assessment_count, 19.0);
    assert_eq!(result.fail_assessment_count, fails);
    assert_eq!(result.warning_assessment_count, warnings);
    assert_eq!(result.results[0].verdict, result.overall_verdict);
    let ids: Vec<&str> = result
        .assessments
        .iter()
        .map(|row| row.requirement_id.as_str())
        .collect();
    assert_eq!(ids, ORDER);
}

#[test]
fn the_complete_example() {
    let standards = vec![
        OpenDrainStandard {
            area_id: "AREA-1".to_string(),
            standard: "NORSOK P-002".to_string(),
        },
        OpenDrainStandard {
            area_id: "AREA-1".to_string(),
            standard: "ISO 13702".to_string(),
        },
    ];
    let references = vec![OpenDrainReference {
        area_id: "AREA-1".to_string(),
        reference: "STID-AREA-1".to_string(),
    }];
    let result = run(
        &[item("AREA-1", "process")],
        &evidence(
            "AREA-1",
            &[
                ("sourceHasFlammableOrHazardousLiquid", "true"),
                ("hasOpenDrainMeasures", "true"),
                ("fireWaterCapacityKgPerS", "10.0"),
                ("liquidLeakRateKgPerS", "2.0"),
                ("drainageCapacityKgPerS", "15.0"),
                ("backflowPrevented", "true"),
                ("closedOpenDrainInteractionPrevented", "true"),
                ("hazardousNonHazardousPhysicallySeparated", "true"),
                ("hazardousNonHazardousCombined", "false"),
                ("sealDesignedForMaxBackpressure", "true"),
                ("ventTerminatedSafe", "true"),
                ("openDrainDependsOnUtility", "false"),
            ],
        ),
        &standards,
        &references,
    );
    assert_eq!(result.overall_verdict, OpenDrainVerdict::Pass);
    assert_eq!(result.results[0].confidence, 0.85);
    assert_counts(&result, 0.0, 0.0);
    let mut expected = vec![OpenDrainStatus::Pass; 19];
    expected[4] = OpenDrainStatus::NotApplicable;
    expected[5] = OpenDrainStatus::NotApplicable;
    expected[6] = OpenDrainStatus::NotApplicable;
    expected[9] = OpenDrainStatus::NotApplicable;
    expected[12] = OpenDrainStatus::NotApplicable;
    expected[13] = OpenDrainStatus::NotApplicable;
    expected[14] = OpenDrainStatus::NotApplicable;
    expected[15] = OpenDrainStatus::NotApplicable;
    expected[16] = OpenDrainStatus::NotApplicable;
    expected[18] = OpenDrainStatus::Info;
    assert_eq!(statuses(&result), expected);
}

#[test]
fn the_capacity_failure() {
    let result = run(
        &[item("AREA-2", "process")],
        &evidence(
            "AREA-2",
            &[
                ("sourceHasFlammableOrHazardousLiquid", "true"),
                ("hasOpenDrainMeasures", "true"),
                ("fireWaterCapacityKgPerS", "10.0"),
                ("liquidLeakRateKgPerS", "2.0"),
                ("drainageCapacityKgPerS", "5.0"),
            ],
        ),
        &[],
        &[],
    );
    assert_eq!(result.overall_verdict, OpenDrainVerdict::Fail);
    assert_eq!(result.results[0].confidence, 0.55);
    assert_counts(&result, 1.0, 6.0);
    let rows = statuses(&result);
    assert_eq!(rows[0], OpenDrainStatus::Warning);
    assert_eq!(rows[2], OpenDrainStatus::Fail);
    assert_eq!(rows[3], OpenDrainStatus::Warning);
    assert_eq!(result.assessments[2].severity, OpenDrainSeverity::Critical);
    assert_eq!(result.assessments[0].severity, OpenDrainSeverity::Medium);
}

#[test]
fn the_hazardous_separation_failure() {
    let result = run(
        &[item("AREA-3", "process")],
        &evidence(
            "AREA-3",
            &[
                ("sourceHasFlammableOrHazardousLiquid", "true"),
                ("hazardousNonHazardousCombined", "true"),
                ("hazardousNonHazardousPhysicallySeparated", "false"),
            ],
        ),
        &[],
        &[],
    );
    assert_eq!(result.overall_verdict, OpenDrainVerdict::Fail);
    assert_eq!(result.results[0].confidence, 0.2);
    assert_counts(&result, 1.0, 7.0);
    assert_eq!(statuses(&result)[8], OpenDrainStatus::Fail);
    assert_eq!(result.assessments[8].severity, OpenDrainSeverity::Critical);
}

#[test]
fn the_missing_evidence_state() {
    let result = run(
        &[item("AREA-4", "process")],
        &evidence("AREA-4", &[("sourceHasFlammableOrHazardousLiquid", "true")]),
        &[],
        &[],
    );
    assert_eq!(result.overall_verdict, OpenDrainVerdict::PassWithWarnings);
    assert_eq!(result.results[0].confidence, 0.2);
    assert_counts(&result, 0.0, 8.0);
    assert_eq!(
        result.results[0].verdict,
        OpenDrainVerdict::PassWithWarnings
    );
}

#[test]
fn the_utility_dependence_and_operational_states() {
    let utility = run(
        &[item("AREA-5", "process")],
        &evidence(
            "AREA-5",
            &[
                ("sourceHasFlammableOrHazardousLiquid", "true"),
                ("openDrainDependsOnUtility", "true"),
            ],
        ),
        &[],
        &[],
    );
    assert_eq!(utility.overall_verdict, OpenDrainVerdict::Fail);
    assert_eq!(utility.results[0].confidence, 0.2);
    assert_eq!(statuses(&utility)[17], OpenDrainStatus::Fail);
    assert_eq!(statuses(&utility)[18], OpenDrainStatus::Info);

    let operational = run(
        &[item("AREA-6", "process")],
        &evidence(
            "AREA-6",
            &[
                ("sourceHasFlammableOrHazardousLiquid", "true"),
                ("observedBackflowEvents", "1"),
                ("sumpHighLevelEvents", "2"),
            ],
        ),
        &[],
        &[],
    );
    assert_eq!(operational.overall_verdict, OpenDrainVerdict::Fail);
    assert_eq!(operational.results[0].confidence, 0.35);
    assert_eq!(statuses(&operational)[18], OpenDrainStatus::Fail);
    assert_eq!(
        operational.assessments[18].severity,
        OpenDrainSeverity::High
    );
}

#[test]
fn the_empty_review_and_the_coercion_state() {
    let empty = open_drain_review(None, None, None, None, None, None).expect("an empty review");
    assert_eq!(empty.overall_verdict, OpenDrainVerdict::Pass);
    assert_eq!(empty.item_count, 0.0);
    assert_eq!(empty.assessment_count, 0.0);
    assert!(empty.results.is_empty());

    let coercion = run(
        &[item("AREA-7", "process")],
        &evidence(
            "AREA-7",
            &[
                ("sourceHasFlammableOrHazardousLiquid", "yes"),
                ("hasOpenDrainMeasures", "1"),
                ("designStandards", "NORSOK P-002 and ISO 13702"),
                ("fireWaterCapacityKgPerS", "10.5"),
                ("drainageCapacityKgPerS", "20"),
            ],
        ),
        &[],
        &[],
    );
    assert_eq!(coercion.overall_verdict, OpenDrainVerdict::PassWithWarnings);
    assert_eq!(coercion.results[0].confidence, 0.65);
    assert_counts(&coercion, 0.0, 5.0);
    assert_eq!(statuses(&coercion)[0], OpenDrainStatus::Pass);
    assert_eq!(statuses(&coercion)[2], OpenDrainStatus::Pass);
}

#[test]
fn the_refusals_are_typed() {
    let items = [item("A", "process"), item("A", "process")];
    assert!(open_drain_review(None, None, Some(&items), None, None, None).is_err());

    let rows = evidence("A", &[("k", "1"), ("k", "2")]);
    assert!(
        open_drain_review(
            None,
            None,
            Some(&[item("A", "process")]),
            Some(&rows),
            None,
            None
        )
        .is_err()
    );

    let orphan = evidence("B", &[("k", "1")]);
    assert!(
        open_drain_review(
            None,
            None,
            Some(&[item("A", "process")]),
            Some(&orphan),
            None,
            None
        )
        .is_err()
    );

    let standard = [OpenDrainStandard {
        area_id: "B".to_string(),
        standard: "S".to_string(),
    }];
    assert!(
        open_drain_review(
            None,
            None,
            Some(&[item("A", "process")]),
            None,
            Some(&standard),
            None
        )
        .is_err()
    );

    let reference = [OpenDrainReference {
        area_id: "B".to_string(),
        reference: "R".to_string(),
    }];
    assert!(
        open_drain_review(
            None,
            None,
            Some(&[item("A", "process")]),
            None,
            None,
            Some(&reference)
        )
        .is_err()
    );
}
