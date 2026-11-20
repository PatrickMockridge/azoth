//! `process.safety_system_performance` against
//! `validation/neqsim/captures/process_safety_system_performance.tsv`.

use azoth_process::models::safety_system_performance::{
    BarrierEquipmentTag, DeviceKind, ElementKind, EventSifRow, MeasurementDeviceRow,
    PerformanceBarrier, PerformanceBarrierStatus, PerformanceCategory, PerformanceElement,
    PerformanceRule, PerformanceSifKind, PerformanceVerdict, ProtectedEquipmentRow,
    QuantitativeSifRow, SafetyPerformanceStandard as Standard, SafetySystemDemandRow,
    SafetySystemLink, SafetySystemPerformanceResult, SifDetectorRow, StandardCriterion,
    VotingNotation, safety_system_performance,
};

fn standard(id: &str, title: &str, function: &str, target: Option<f64>) -> Standard {
    Standard {
        id: id.to_string(),
        title: Some(title.to_string()),
        safety_function: Some(function.to_string()),
        target_pfd: target,
        required_availability: Some(0.99),
        response_time_seconds: Some(60.0),
    }
}

#[allow(clippy::too_many_arguments)]
fn barrier(
    id: &str,
    name: &str,
    function: &str,
    status: PerformanceBarrierStatus,
    pfd: Option<f64>,
    effectiveness: Option<f64>,
    standard_id: Option<&str>,
    evidence: bool,
) -> PerformanceBarrier {
    PerformanceBarrier {
        id: id.to_string(),
        status,
        has_traceable_evidence: evidence,
        name: Some(name.to_string()),
        description: Some(function.to_string()),
        safety_function: Some(function.to_string()),
        pfd,
        effectiveness,
        performance_standard_id: standard_id.map(str::to_string),
    }
}

fn tag(barrier_id: &str, tag: &str) -> BarrierEquipmentTag {
    BarrierEquipmentTag {
        barrier_id: barrier_id.to_string(),
        tag: tag.to_string(),
    }
}

fn criterion(standard_id: &str) -> StandardCriterion {
    StandardCriterion {
        standard_id: standard_id.to_string(),
        criterion: "Barrier shall meet documented functional requirements.".to_string(),
    }
}

fn empty_demand(demand_id: &str, barrier_id: &str) -> SafetySystemDemandRow {
    SafetySystemDemandRow {
        demand_id: demand_id.to_string(),
        barrier_id: Some(barrier_id.to_string()),
        equipment_tag: None,
        scenario: None,
        category: PerformanceCategory::Unknown,
        demand_value: None,
        capacity_value: None,
        demand_unit: None,
        required_pfd: None,
        actual_pfd: None,
        required_availability: None,
        actual_availability: None,
        required_response_time_seconds: None,
        actual_response_time_seconds: None,
        required_effectiveness: None,
        actual_effectiveness: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    register_id: &str,
    name: &str,
    elements: &[PerformanceElement],
    links: &[SafetySystemLink],
    barriers: &[PerformanceBarrier],
    tags: &[BarrierEquipmentTag],
    standards: &[Standard],
    criteria: &[StandardCriterion],
    demands: &[SafetySystemDemandRow],
    devices: &[MeasurementDeviceRow],
    event_sifs: &[EventSifRow],
    detectors: &[SifDetectorRow],
    quantitative: &[QuantitativeSifRow],
    protected: &[ProtectedEquipmentRow],
) -> SafetySystemPerformanceResult {
    safety_system_performance(
        register_id,
        Some(name),
        Some(elements),
        Some(links),
        Some(barriers),
        Some(tags),
        Some(standards),
        Some(criteria),
        Some(demands),
        Some(devices),
        Some(event_sifs),
        Some(detectors),
        Some(quantitative),
        Some(protected),
    )
    .expect("the register assesses")
}

fn rules(result: &SafetySystemPerformanceResult) -> Vec<PerformanceRule> {
    result.findings.iter().map(|row| row.rule).collect()
}

fn deluge() -> SafetySystemPerformanceResult {
    let barriers = [barrier(
        "B-DELUGE-001",
        "Deluge system",
        "Mitigate jet fire exposure using deluge in module M01",
        PerformanceBarrierStatus::Available,
        Some(2.8e-4),
        Some(0.95),
        Some("PS-B-DELUGE-001"),
        true,
    )];
    let mut demand = empty_demand("D-DELUGE-001", "B-DELUGE-001");
    demand.category = PerformanceCategory::FirewaterDeluge;
    demand.scenario = Some("Jet fire on V-101".to_string());
    demand.demand_value = Some(10.0);
    demand.capacity_value = Some(15.0);
    demand.demand_unit = Some("L/min/m2".to_string());
    demand.required_response_time_seconds = Some(45.0);
    demand.actual_response_time_seconds = Some(30.0);
    demand.required_availability = Some(0.99);
    demand.actual_availability = Some(0.995);
    demand.required_effectiveness = Some(0.90);
    demand.actual_effectiveness = Some(0.95);
    run(
        "BR-FIRE-001",
        "Fire protection",
        &[PerformanceElement {
            id: "SCE-FW-001".to_string(),
            element_type: ElementKind::FireProtection,
        }],
        &[SafetySystemLink {
            element_id: "SCE-FW-001".to_string(),
            barrier_id: "B-DELUGE-001".to_string(),
        }],
        &barriers,
        &[tag("B-DELUGE-001", "V-101")],
        &[standard(
            "PS-B-DELUGE-001",
            "Deluge system standard",
            "Mitigate jet fire exposure using deluge in module M01",
            Some(1.0e-3),
        )],
        &[criterion("PS-B-DELUGE-001")],
        &[demand],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
}

#[test]
fn the_deluge_case_passes() {
    let result = deluge();
    assert_eq!(result.overall_verdict, PerformanceVerdict::Pass);
    assert_eq!(result.assessment_count, 1.0);
    assert_eq!(result.pass_count, 1.0);
    let assessment = &result.assessments[0];
    assert_eq!(assessment.verdict, PerformanceVerdict::Pass);
    assert_eq!(assessment.category, PerformanceCategory::FirewaterDeluge);
    assert_eq!(assessment.capacity_margin, Some(5.0));
    assert_eq!(assessment.barrier_effectiveness, Some(0.95));
    assert_eq!(assessment.actual_pfd, Some(2.8e-4));
    assert_eq!(result.findings.len(), 0);
}

#[test]
fn the_fire_and_gas_case_reads_its_devices_and_sif() {
    let barriers = [barrier(
        "B-FGS-001",
        "Fire and gas detection",
        "Detect gas or fire and activate deluge",
        PerformanceBarrierStatus::Available,
        Some(2.8e-4),
        None,
        Some("PS-B-FGS-001"),
        true,
    )];
    let mut demand = empty_demand("D-FGS-001", "B-FGS-001");
    demand.category = PerformanceCategory::FireGasDetection;
    demand.required_response_time_seconds = Some(20.0);
    demand.actual_response_time_seconds = Some(10.0);
    demand.required_availability = Some(0.99);
    demand.actual_availability = Some(0.996);
    let result = run(
        "BR-FG-001",
        "Fire and gas",
        &[PerformanceElement {
            id: "SCE-FGS-001".to_string(),
            element_type: ElementKind::InstrumentedFunction,
        }],
        &[SafetySystemLink {
            element_id: "SCE-FGS-001".to_string(),
            barrier_id: "B-FGS-001".to_string(),
        }],
        &barriers,
        &[
            tag("B-FGS-001", "V-101"),
            tag("B-FGS-001", "FD-101"),
            tag("B-FGS-001", "GD-101"),
        ],
        &[standard(
            "PS-B-FGS-001",
            "Fire and gas detection standard",
            "Detect gas or fire and activate deluge",
            Some(1.0e-3),
        )],
        &[criterion("PS-B-FGS-001")],
        &[demand],
        &[
            MeasurementDeviceRow {
                name: "FD-101".to_string(),
                kind: DeviceKind::FireDetector,
                tag: None,
                location: Some("Module M01".to_string()),
                response_time_seconds: Some(2.0),
            },
            MeasurementDeviceRow {
                name: "GD-101".to_string(),
                kind: DeviceKind::GasDetector,
                tag: None,
                location: Some("Module M01".to_string()),
                response_time_seconds: Some(8.0),
            },
        ],
        &[EventSifRow {
            name: "B-FGS-001 Deluge SIF".to_string(),
            voting_notation: VotingNotation::TwoOutOfThree,
            overridden: false,
        }],
        &[
            SifDetectorRow {
                sif_name: "B-FGS-001 Deluge SIF".to_string(),
                name: "FD-101".to_string(),
                bypassed: false,
                faulty: false,
                tripped: false,
            },
            SifDetectorRow {
                sif_name: "B-FGS-001 Deluge SIF".to_string(),
                name: "GD-101".to_string(),
                bypassed: false,
                faulty: false,
                tripped: false,
            },
            SifDetectorRow {
                sif_name: "B-FGS-001 Deluge SIF".to_string(),
                name: "GD-102".to_string(),
                bypassed: false,
                faulty: false,
                tripped: false,
            },
        ],
        &[],
        &[],
    );
    assert_eq!(result.overall_verdict, PerformanceVerdict::Pass);
    assert_eq!(
        result.assessments[0].barrier_effectiveness,
        Some(1.0 - 2.8e-4)
    );
    assert_eq!(
        result.assessments[0].max_instrument_response_time_seconds,
        Some(8.0)
    );
    assert_eq!(
        result
            .instruments
            .iter()
            .map(|row| row.tag.as_str())
            .collect::<Vec<_>>(),
        ["FD-101", "GD-101"]
    );
    assert_eq!(result.sifs.len(), 1);
    assert_eq!(result.sifs[0].kind, PerformanceSifKind::Event);
    assert_eq!(result.sifs[0].voting_notation.as_deref(), Some("2oo3"));
    assert_eq!(result.sifs[0].detector_count, Some(3.0));
    assert_eq!(result.assessments[0].related_sif_count, 1.0);
}

#[test]
fn the_pfp_capacity_failure() {
    let barriers = [barrier(
        "B-PFP-001",
        "Passive fire protection",
        "PFP coating on V-101 support steel",
        PerformanceBarrierStatus::Available,
        None,
        Some(0.8),
        Some("PS-B-PFP-001"),
        true,
    )];
    let mut demand = empty_demand("D-PFP-001", "B-PFP-001");
    demand.category = PerformanceCategory::PassiveFireProtection;
    demand.scenario = Some("Jet fire thermal radiation at support steel".to_string());
    demand.demand_value = Some(45.0);
    demand.capacity_value = Some(37.5);
    demand.demand_unit = Some("kW/m2".to_string());
    demand.required_effectiveness = Some(0.90);
    demand.actual_effectiveness = Some(0.80);
    let result = run(
        "BR-PFP-001",
        "PFP review",
        &[],
        &[],
        &barriers,
        &[tag("B-PFP-001", "V-101")],
        &[standard(
            "PS-B-PFP-001",
            "Passive fire protection standard",
            "PFP coating on V-101 support steel",
            None,
        )],
        &[criterion("PS-B-PFP-001")],
        &[demand],
        &[],
        &[],
        &[],
        &[],
        &[],
    );
    assert_eq!(result.overall_verdict, PerformanceVerdict::Fail);
    assert_eq!(
        result.assessments[0].category,
        PerformanceCategory::PassiveFireProtection
    );
    assert_eq!(result.assessments[0].capacity_margin, Some(-7.5));
    assert_eq!(
        rules(&result),
        [
            PerformanceRule::RequiredAvailabilityWithoutActual,
            PerformanceRule::RequiredResponseTimeWithoutActual,
            PerformanceRule::EffectivenessBelowRequired,
            PerformanceRule::DemandExceedsCapacity,
        ]
    );
    assert_eq!(result.fail_count, 1.0);
}

#[test]
fn the_quantitative_sif_state() {
    let barriers = [barrier(
        "B-SIF-101",
        "SIF shutdown",
        "SIF shutdown protects V-101",
        PerformanceBarrierStatus::Available,
        Some(8.0e-4),
        None,
        Some("PS-B-SIF-101"),
        true,
    )];
    let result = run(
        "BR-QSIF-001",
        "Quantitative SIF",
        &[],
        &[],
        &barriers,
        &[tag("B-SIF-101", "V-101")],
        &[standard(
            "PS-B-SIF-101",
            "SIF shutdown standard",
            "SIF shutdown protects V-101",
            Some(1.0e-3),
        )],
        &[criterion("PS-B-SIF-101")],
        &[],
        &[],
        &[],
        &[],
        &[QuantitativeSifRow {
            id: "B-SIF-101".to_string(),
            name: Some("B-SIF-101 quantitative SIL".to_string()),
            description: Some("SIF shutdown protects V-101".to_string()),
            architecture: "1oo2".to_string(),
            claimed_sil: 3.0,
            pfd_average: 8.0e-4,
            test_interval_hours: Some(8760.0),
        }],
        &[ProtectedEquipmentRow {
            sif_id: "B-SIF-101".to_string(),
            equipment: "V-101".to_string(),
        }],
    );
    assert_eq!(result.overall_verdict, PerformanceVerdict::PassWithWarnings);
    assert_eq!(result.assessments[0].category, PerformanceCategory::Unknown);
    assert_eq!(result.sifs.len(), 1);
    let sif = &result.sifs[0];
    assert_eq!(sif.kind, PerformanceSifKind::Quantitative);
    assert_eq!(sif.identifier, "B-SIF-101 quantitative SIL");
    assert_eq!(sif.claimed_sil, Some(3.0));
    assert_eq!(sif.achieved_sil, Some(3.0));
    assert_eq!(sif.sil_achieved, Some(true));
    assert_eq!(sif.pfd_average, Some(8.0e-4));
    assert_eq!(sif.pfd_upper, Some(0.0012000000000000001));
    assert_eq!(sif.risk_reduction_factor, Some(1250.0));
    assert_eq!(sif.protected_equipment_count, Some(1.0));
    assert_eq!(
        rules(&result),
        [
            PerformanceRule::RequiredAvailabilityWithoutActual,
            PerformanceRule::RequiredResponseTimeWithoutActual,
            PerformanceRule::QuantitativeSifVerificationWarning,
        ]
    );
}

#[test]
fn the_category_list_and_the_rule_coverage() {
    let barriers = [barrier(
        "B-PSD-101",
        "PSD valve",
        "Process shutdown closes shutdown valve SDV-101",
        PerformanceBarrierStatus::Available,
        Some(8.0e-4),
        None,
        Some("PS-B-PSD-101"),
        true,
    )];
    let psd = run(
        "BR-PSD-001",
        "PSD review",
        &[],
        &[],
        &barriers,
        &[tag("B-PSD-101", "V-101")],
        &[standard(
            "PS-B-PSD-101",
            "PSD valve standard",
            "Process shutdown closes shutdown valve SDV-101",
            Some(1.0e-3),
        )],
        &[criterion("PS-B-PSD-101")],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
    );
    assert_eq!(psd.assessments[0].category, PerformanceCategory::Psd);
    assert_eq!(psd.overall_verdict, PerformanceVerdict::PassWithWarnings);

    let rules_barrier = [barrier(
        "B-RULES-101",
        "Rules barrier",
        "Isolation duty",
        PerformanceBarrierStatus::OutOfService,
        None,
        None,
        None,
        false,
    )];
    let mut first = empty_demand("D-RULES-101", "B-RULES-101");
    first.scenario = Some("Loss of containment".to_string());
    first.required_pfd = Some(1.0e-3);
    first.actual_pfd = Some(1.0e-2);
    first.required_availability = Some(0.99);
    first.actual_availability = Some(0.95);
    first.required_response_time_seconds = Some(10.0);
    first.actual_response_time_seconds = Some(20.0);
    first.required_effectiveness = Some(0.9);
    first.actual_effectiveness = Some(0.5);
    first.demand_value = Some(10.0);
    first.capacity_value = Some(5.0);
    first.demand_unit = Some("kg/s".to_string());
    let mut second = empty_demand("D-RULES-102", "B-RULES-101");
    second.required_response_time_seconds = Some(5.0);
    let result = run(
        "BR-RULES-001",
        "Rule coverage",
        &[PerformanceElement {
            id: "SCE-RULES-001".to_string(),
            element_type: ElementKind::InstrumentedFunction,
        }],
        &[SafetySystemLink {
            element_id: "SCE-RULES-001".to_string(),
            barrier_id: "B-RULES-101".to_string(),
        }],
        &rules_barrier,
        &[tag("B-RULES-101", "GD-201")],
        &[],
        &[],
        &[first, second],
        &[MeasurementDeviceRow {
            name: "GD-201".to_string(),
            kind: DeviceKind::GasDetector,
            tag: None,
            location: Some("Module M02".to_string()),
            response_time_seconds: Some(30.0),
        }],
        &[EventSifRow {
            name: "B-RULES-101 SIF".to_string(),
            voting_notation: VotingNotation::TwoOutOfThree,
            overridden: true,
        }],
        &[
            SifDetectorRow {
                sif_name: "B-RULES-101 SIF".to_string(),
                name: "GD-201".to_string(),
                bypassed: true,
                faulty: false,
                tripped: false,
            },
            SifDetectorRow {
                sif_name: "B-RULES-101 SIF".to_string(),
                name: "GD-202".to_string(),
                bypassed: false,
                faulty: true,
                tripped: false,
            },
            SifDetectorRow {
                sif_name: "B-RULES-101 SIF".to_string(),
                name: "GD-203".to_string(),
                bypassed: false,
                faulty: false,
                tripped: false,
            },
        ],
        &[],
        &[],
    );
    assert_eq!(result.overall_verdict, PerformanceVerdict::Fail);
    assert_eq!(result.assessment_count, 2.0);
    assert_eq!(
        result.assessments[0].category,
        PerformanceCategory::EsdBlowdown
    );
    assert_eq!(result.assessments[0].capacity_margin, Some(-5.0));
    assert_eq!(result.findings.len(), 12);
    let first_rules: Vec<PerformanceRule> = result
        .findings
        .iter()
        .filter(|row| row.demand_id.as_deref() == Some("D-RULES-101"))
        .map(|row| row.rule)
        .collect();
    assert_eq!(
        first_rules,
        [
            PerformanceRule::BarrierNotAvailable,
            PerformanceRule::BarrierUntraceable,
            PerformanceRule::NoPerformanceStandard,
            PerformanceRule::DemandExceedsCapacity,
            PerformanceRule::SifOverridden,
            PerformanceRule::SifHealthyChannelsInsufficient,
        ]
    );
    let second_rules: Vec<PerformanceRule> = result
        .findings
        .iter()
        .filter(|row| row.demand_id.as_deref() == Some("D-RULES-102"))
        .map(|row| row.rule)
        .collect();
    assert_eq!(
        second_rules,
        [
            PerformanceRule::BarrierNotAvailable,
            PerformanceRule::BarrierUntraceable,
            PerformanceRule::NoPerformanceStandard,
            PerformanceRule::DetectorResponseTimeExceeded,
            PerformanceRule::SifOverridden,
            PerformanceRule::SifHealthyChannelsInsufficient,
        ]
    );
}

#[test]
fn the_instrumentation_warnings_and_the_element_inference() {
    let instrument = [barrier(
        "B-INST-001",
        "Gas detection",
        "Detect gas and alarm",
        PerformanceBarrierStatus::Available,
        Some(2.8e-4),
        None,
        Some("PS-B-INST-001"),
        true,
    )];
    let result = run(
        "BR-INST-001",
        "Instrumentation",
        &[],
        &[],
        &instrument,
        &[tag("B-INST-001", "V-101"), tag("B-INST-001", "GD-301")],
        &[standard(
            "PS-B-INST-001",
            "Gas detection standard",
            "Detect gas and alarm",
            Some(1.0e-3),
        )],
        &[criterion("PS-B-INST-001")],
        &[],
        &[],
        &[EventSifRow {
            name: "B-INST-001 SIF".to_string(),
            voting_notation: VotingNotation::TwoOutOfThree,
            overridden: false,
        }],
        &[
            SifDetectorRow {
                sif_name: "B-INST-001 SIF".to_string(),
                name: "GD-301".to_string(),
                bypassed: true,
                faulty: false,
                tripped: false,
            },
            SifDetectorRow {
                sif_name: "B-INST-001 SIF".to_string(),
                name: "GD-302".to_string(),
                bypassed: false,
                faulty: false,
                tripped: false,
            },
        ],
        &[],
        &[],
    );
    assert_eq!(result.overall_verdict, PerformanceVerdict::Fail);
    assert_eq!(
        rules(&result),
        [
            PerformanceRule::RequiredAvailabilityWithoutActual,
            PerformanceRule::RequiredResponseTimeWithoutActual,
            PerformanceRule::NoFireGasDetector,
            PerformanceRule::SifDetectorCountMismatch,
            PerformanceRule::SifHealthyChannelsInsufficient,
        ]
    );

    let inferred = [barrier(
        "B-INF-001",
        "Bland barrier",
        "Isolation duty",
        PerformanceBarrierStatus::Available,
        None,
        None,
        Some("PS-INF-001"),
        true,
    )];
    let result = run(
        "BR-INF-001",
        "Inferred category",
        &[PerformanceElement {
            id: "SCE-INF-001".to_string(),
            element_type: ElementKind::FireProtection,
        }],
        &[SafetySystemLink {
            element_id: "SCE-INF-001".to_string(),
            barrier_id: "B-INF-001".to_string(),
        }],
        &inferred,
        &[tag("B-INF-001", "V-301")],
        &[standard(
            "PS-INF-001",
            "Bland standard",
            "Isolation duty",
            Some(1.0e-3),
        )],
        &[criterion("PS-INF-001")],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
    );
    assert_eq!(result.overall_verdict, PerformanceVerdict::PassWithWarnings);
    assert_eq!(
        result.assessments[0].category,
        PerformanceCategory::FirewaterDeluge
    );
    assert_eq!(
        rules(&result),
        [
            PerformanceRule::TargetPfdWithoutActual,
            PerformanceRule::RequiredAvailabilityWithoutActual,
            PerformanceRule::RequiredResponseTimeWithoutActual,
            PerformanceRule::FireProtectionDemandMissing,
        ]
    );
}

#[test]
fn the_refusals_are_typed() {
    let barriers = [barrier(
        "B-1",
        "B",
        "Isolation",
        PerformanceBarrierStatus::Available,
        None,
        None,
        None,
        true,
    )];
    let orphan_link = [SafetySystemLink {
        element_id: "E".to_string(),
        barrier_id: "B-9".to_string(),
    }];
    assert!(
        safety_system_performance(
            "R",
            None,
            None,
            Some(&orphan_link),
            Some(&barriers),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None
        )
        .is_err()
    );
    let orphan_tag = [tag("B-9", "T")];
    assert!(
        safety_system_performance(
            "R",
            None,
            None,
            None,
            Some(&barriers),
            Some(&orphan_tag),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None
        )
        .is_err()
    );
    let orphan_standard = [criterion("S-9")];
    assert!(
        safety_system_performance(
            "R",
            None,
            None,
            None,
            Some(&barriers),
            None,
            None,
            Some(&orphan_standard),
            None,
            None,
            None,
            None,
            None,
            None
        )
        .is_err()
    );
    let orphan_detector = [SifDetectorRow {
        sif_name: "SIF-9".to_string(),
        name: "D".to_string(),
        bypassed: false,
        faulty: false,
        tripped: false,
    }];
    assert!(
        safety_system_performance(
            "R",
            None,
            None,
            None,
            Some(&barriers),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&orphan_detector),
            None,
            None
        )
        .is_err()
    );
    let orphan_protected = [ProtectedEquipmentRow {
        sif_id: "Q-9".to_string(),
        equipment: "E".to_string(),
    }];
    assert!(
        safety_system_performance(
            "R",
            None,
            None,
            None,
            Some(&barriers),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&orphan_protected)
        )
        .is_err()
    );
    let bad_sil = [QuantitativeSifRow {
        id: "B-1".to_string(),
        name: None,
        description: None,
        architecture: "1oo2".to_string(),
        claimed_sil: 0.0,
        pfd_average: 1.0e-3,
        test_interval_hours: None,
    }];
    assert!(
        safety_system_performance(
            "R",
            None,
            None,
            None,
            Some(&barriers),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&bad_sil),
            None
        )
        .is_err()
    );
}
