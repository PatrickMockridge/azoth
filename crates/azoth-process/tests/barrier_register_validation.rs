//! `process.barrier_register_validation` against
//! `validation/neqsim/captures/process_barrier_register_validation.tsv`.

use azoth_process::models::barrier_register_validation::{
    BarrierElement, BarrierRegisterValidationResult, BarrierRow, ElementBarrierLink,
    PerformanceStandardRow, RegisterEvidence, barrier_register_validation,
};
use azoth_process::models::safety_review::{FindingScope, RegisterRule};

fn element(id: &str, tag: Option<&str>, name: Option<&str>) -> BarrierElement {
    BarrierElement {
        id: id.to_string(),
        tag: tag.map(str::to_string),
        name: name.map(str::to_string),
        element_type: "process_equipment".to_string(),
    }
}

fn barrier_row(id: &str, standard: Option<&str>) -> BarrierRow {
    BarrierRow {
        id: id.to_string(),
        name: Some(format!("Barrier {id}")),
        description: Some("Prevents the overpressure scenario".to_string()),
        barrier_type: "prevention".to_string(),
        status: "available".to_string(),
        pfd: Some(0.001),
        effectiveness: Some(0.999),
        performance_standard_id: standard.map(str::to_string),
        equipment_tag_count: 1.0,
    }
}

fn standard(id: &str, acceptance: f64) -> PerformanceStandardRow {
    PerformanceStandardRow {
        id: id.to_string(),
        safety_function: Some("Isolate the protected equipment".to_string()),
        target_pfd: Some(0.001),
        required_availability: Some(0.995),
        proof_test_interval_hours: Some(8760.0),
        acceptance_criteria_count: acceptance,
    }
}

fn link(element_id: &str, barrier_id: &str) -> ElementBarrierLink {
    ElementBarrierLink {
        element_id: element_id.to_string(),
        barrier_id: barrier_id.to_string(),
    }
}

fn evidence(owner_kind: &str, owner_id: &str) -> RegisterEvidence {
    RegisterEvidence {
        owner_kind: owner_kind.to_string(),
        owner_id: owner_id.to_string(),
        document_id: Some(format!("SRS-{owner_id}")),
        source_reference: None,
        excerpt: Some("Evidence excerpt.".to_string()),
    }
}

fn run(
    register_id: &str,
    elements: Option<&[BarrierElement]>,
    barriers: Option<&[BarrierRow]>,
    standards: Option<&[PerformanceStandardRow]>,
    links: Option<&[ElementBarrierLink]>,
    evidence: Option<&[RegisterEvidence]>,
) -> BarrierRegisterValidationResult {
    barrier_register_validation(register_id, elements, barriers, standards, links, evidence)
        .expect("the captured state is valid")
}

fn triples(result: &BarrierRegisterValidationResult) -> Vec<(FindingScope, String, RegisterRule)> {
    result
        .findings
        .iter()
        .map(|finding| (finding.scope, finding.subject_id.clone(), finding.rule))
        .collect()
}

fn valid_register() -> BarrierRegisterValidationResult {
    let elements = [element(
        "SCE-V-101",
        Some("V-101"),
        Some("Protected equipment"),
    )];
    let barriers = [barrier_row("B-HIPPS-001", Some("PS-HIPPS-001"))];
    let standards = [standard("PS-HIPPS-001", 1.0)];
    let links = [link("SCE-V-101", "B-HIPPS-001")];
    let evidence = [
        evidence("standard", "PS-HIPPS-001"),
        evidence("barrier", "B-HIPPS-001"),
    ];
    run(
        "BR-001",
        Some(&elements),
        Some(&barriers),
        Some(&standards),
        Some(&links),
        Some(&evidence),
    )
}

#[test]
fn a_valid_register_has_no_findings() {
    let result = valid_register();
    assert_eq!(result.finding_count, 0.0);
    assert!(result.valid);
    assert!(result.findings.is_empty());
}

#[test]
fn the_register_rules_run_first() {
    let missing = run("", None, None, None, None, None);
    assert_eq!(
        triples(&missing),
        vec![
            (
                FindingScope::Register,
                String::new(),
                RegisterRule::RegisterIdMissing
            ),
            (
                FindingScope::Register,
                String::new(),
                RegisterRule::RegisterEmpty
            ),
        ]
    );
    let empty = run("BR-EMPTY", None, None, None, None, None);
    assert_eq!(
        triples(&empty),
        vec![(
            FindingScope::Register,
            "BR-EMPTY".to_string(),
            RegisterRule::RegisterEmpty
        )]
    );
}

#[test]
fn the_element_rules_precede_its_barriers() {
    let elements = [element("", None, Some("Protected equipment"))];
    let barriers = [barrier_row("B-EL-ID", Some("PS-HIPPS-001"))];
    let standards = [standard("PS-HIPPS-001", 1.0)];
    let links = [link("", "B-EL-ID")];
    let evidence = [
        evidence("standard", "PS-HIPPS-001"),
        evidence("barrier", "B-EL-ID"),
    ];
    let result = run(
        "BR-001",
        Some(&elements),
        Some(&barriers),
        Some(&standards),
        Some(&links),
        Some(&evidence),
    );
    assert_eq!(
        triples(&result),
        vec![(
            FindingScope::Element,
            String::new(),
            RegisterRule::ElementIdMissing
        )]
    );

    let no_barriers = [element("SCE-003", Some("V-101"), Some("No barriers"))];
    let result = run("BR-001", Some(&no_barriers), None, None, None, None);
    assert_eq!(
        triples(&result),
        vec![
            (
                FindingScope::Element,
                "SCE-003".to_string(),
                RegisterRule::ElementNoBarriers,
            ),
            (
                FindingScope::Element,
                "SCE-003".to_string(),
                RegisterRule::ElementNoTraceableEvidence,
            ),
        ]
    );
}

#[test]
fn an_untraceable_element_and_barrier_report_in_order() {
    let elements = [element("SCE-004", Some("V-101"), Some("No evidence"))];
    let barriers = [barrier_row("B-004", None)];
    let links = [link("SCE-004", "B-004")];
    let result = run(
        "BR-001",
        Some(&elements),
        Some(&barriers),
        None,
        Some(&links),
        None,
    );
    assert_eq!(
        triples(&result),
        vec![
            (
                FindingScope::Element,
                "SCE-004".to_string(),
                RegisterRule::ElementNoTraceableEvidence,
            ),
            (
                FindingScope::Barrier,
                "B-004".to_string(),
                RegisterRule::BarrierNoPerformanceStandard,
            ),
            (
                FindingScope::Barrier,
                "B-004".to_string(),
                RegisterRule::BarrierNoTraceableEvidence,
            ),
        ]
    );
}

#[test]
fn each_barrier_rule_is_reported() {
    let cases: Vec<(BarrierRow, RegisterRule)> = vec![
        (
            BarrierRow {
                id: String::new(),
                ..barrier_row("", Some("PS-HIPPS-001"))
            },
            RegisterRule::BarrierIdMissing,
        ),
        (
            BarrierRow {
                name: Some(String::new()),
                description: Some(String::new()),
                ..barrier_row("B-005", Some("PS-HIPPS-001"))
            },
            RegisterRule::BarrierNameOrDescriptionMissing,
        ),
        (
            BarrierRow {
                pfd: Some(1.5),
                ..barrier_row("B-006", Some("PS-HIPPS-001"))
            },
            RegisterRule::BarrierPfdOutOfRange,
        ),
        (
            BarrierRow {
                effectiveness: Some(1.5),
                ..barrier_row("B-007", Some("PS-HIPPS-001"))
            },
            RegisterRule::BarrierEffectivenessOutOfRange,
        ),
        (
            BarrierRow {
                status: "unknown".to_string(),
                ..barrier_row("B-008", Some("PS-HIPPS-001"))
            },
            RegisterRule::BarrierStatusUnknown,
        ),
        (
            BarrierRow {
                equipment_tag_count: 0.0,
                ..barrier_row("B-009", Some("PS-HIPPS-001"))
            },
            RegisterRule::BarrierNoEquipmentTag,
        ),
        (
            barrier_row("B-010", None),
            RegisterRule::BarrierNoPerformanceStandard,
        ),
    ];
    let standards = [standard("PS-HIPPS-001", 1.0)];
    for (barrier, rule) in cases {
        let element_id = format!("SCE-{}", barrier.id);
        let elements = [element(
            &element_id,
            Some("V-101"),
            Some("Protected equipment"),
        )];
        let links = [link(&element_id, &barrier.id)];
        let evidence = [
            evidence("standard", "PS-HIPPS-001"),
            evidence("barrier", &barrier.id),
        ];
        let result = run(
            "BR-001",
            Some(&elements),
            Some(&[barrier.clone()]),
            Some(&standards),
            Some(&links),
            Some(&evidence),
        );
        assert_eq!(
            triples(&result),
            vec![(FindingScope::Barrier, barrier.id.clone(), rule)],
            "{}",
            barrier.id
        );
    }
}

#[test]
fn each_standard_rule_is_reported() {
    let cases: Vec<(PerformanceStandardRow, RegisterRule)> = vec![
        (
            PerformanceStandardRow {
                id: String::new(),
                ..standard("", 1.0)
            },
            RegisterRule::StandardIdMissing,
        ),
        (
            PerformanceStandardRow {
                safety_function: Some(String::new()),
                ..standard("PS-NOFUNC", 1.0)
            },
            RegisterRule::StandardSafetyFunctionMissing,
        ),
        (
            PerformanceStandardRow {
                target_pfd: Some(0.0),
                ..standard("PS-ZEROPFD", 1.0)
            },
            RegisterRule::StandardTargetPfdOutOfRange,
        ),
        (
            PerformanceStandardRow {
                required_availability: Some(1.5),
                ..standard("PS-AVAIL", 1.0)
            },
            RegisterRule::StandardRequiredAvailabilityOutOfRange,
        ),
        (
            PerformanceStandardRow {
                proof_test_interval_hours: Some(0.0),
                ..standard("PS-PROOF", 1.0)
            },
            RegisterRule::StandardProofTestIntervalNotPositive,
        ),
        (
            standard("PS-NOCRIT", 0.0),
            RegisterRule::StandardNoAcceptanceCriteria,
        ),
    ];
    for (standard_row, rule) in cases {
        let barrier = barrier_row("B-STD", Some(&standard_row.id));
        let elements = [element(
            "SCE-STD",
            Some("V-101"),
            Some("Protected equipment"),
        )];
        let links = [link("SCE-STD", "B-STD")];
        let evidence = [
            evidence("standard", &standard_row.id),
            evidence("barrier", "B-STD"),
        ];
        let result = run(
            "BR-001",
            Some(&elements),
            Some(&[barrier]),
            Some(&[standard_row.clone()]),
            Some(&links),
            Some(&evidence),
        );
        assert_eq!(
            triples(&result),
            vec![(FindingScope::Standard, standard_row.id.clone(), rule)],
            "{}",
            standard_row.id
        );
    }
}

#[test]
fn an_untraceable_standard_reports_once() {
    let elements = [element(
        "SCE-019",
        Some("V-101"),
        Some("Protected equipment"),
    )];
    let barriers = [barrier_row("B-018", Some("PS-NOEVID"))];
    let standards = [standard("PS-NOEVID", 1.0)];
    let links = [link("SCE-019", "B-018")];
    let evidence = [evidence("barrier", "B-018")];
    let result = run(
        "BR-001",
        Some(&elements),
        Some(&barriers),
        Some(&standards),
        Some(&links),
        Some(&evidence),
    );
    assert_eq!(
        triples(&result),
        vec![(
            FindingScope::Standard,
            "PS-NOEVID".to_string(),
            RegisterRule::StandardNoTraceableEvidence,
        )]
    );
}

#[test]
fn an_unlinked_barrier_gets_only_the_linkage_finding() {
    let barriers = [barrier_row("B-UNLINKED", Some("PS-HIPPS-001"))];
    let result = run("BR-001", None, Some(&barriers), None, None, None);
    assert_eq!(
        triples(&result),
        vec![(
            FindingScope::Barrier,
            "B-UNLINKED".to_string(),
            RegisterRule::BarrierNotLinkedToElement,
        )]
    );
}

#[test]
fn a_barrier_linked_to_two_elements_is_validated_twice() {
    let shared = BarrierRow {
        status: "unknown".to_string(),
        ..barrier_row("B-TWICE", Some("PS-HIPPS-001"))
    };
    let elements = [
        element("SCE-020", Some("V-101"), Some("Protected equipment")),
        element("SCE-021", Some("V-101"), Some("Protected equipment")),
    ];
    let barriers = [shared];
    let standards = [standard("PS-HIPPS-001", 1.0)];
    let links = [link("SCE-020", "B-TWICE"), link("SCE-021", "B-TWICE")];
    let evidence = [
        evidence("standard", "PS-HIPPS-001"),
        evidence("barrier", "B-TWICE"),
    ];
    let result = run(
        "BR-001",
        Some(&elements),
        Some(&barriers),
        Some(&standards),
        Some(&links),
        Some(&evidence),
    );
    assert_eq!(
        triples(&result),
        vec![
            (
                FindingScope::Barrier,
                "B-TWICE".to_string(),
                RegisterRule::BarrierStatusUnknown,
            ),
            (
                FindingScope::Barrier,
                "B-TWICE".to_string(),
                RegisterRule::BarrierStatusUnknown,
            ),
        ]
    );
}

#[test]
fn a_shared_standard_is_validated_once_per_barrier() {
    let elements = [element(
        "SCE-022",
        Some("V-101"),
        Some("Protected equipment"),
    )];
    let barriers = [
        barrier_row("B-SHARED-1", Some("PS-SHARED")),
        barrier_row("B-SHARED-2", Some("PS-SHARED")),
    ];
    let standards = [standard("PS-SHARED", 0.0)];
    let links = [link("SCE-022", "B-SHARED-1"), link("SCE-022", "B-SHARED-2")];
    let evidence = [
        evidence("standard", "PS-SHARED"),
        evidence("barrier", "B-SHARED-1"),
        evidence("barrier", "B-SHARED-2"),
    ];
    let result = run(
        "BR-001",
        Some(&elements),
        Some(&barriers),
        Some(&standards),
        Some(&links),
        Some(&evidence),
    );
    assert_eq!(
        triples(&result),
        vec![
            (
                FindingScope::Standard,
                "PS-SHARED".to_string(),
                RegisterRule::StandardNoAcceptanceCriteria,
            ),
            (
                FindingScope::Standard,
                "PS-SHARED".to_string(),
                RegisterRule::StandardNoAcceptanceCriteria,
            ),
        ]
    );
}

#[test]
fn an_unknown_link_is_refused() {
    let elements = [element("SCE-1", Some("V-101"), Some("Protected equipment"))];
    let links = [link("SCE-1", "B-MISSING")];
    assert!(
        barrier_register_validation("BR-001", Some(&elements), None, None, Some(&links), None)
            .is_err()
    );
}
