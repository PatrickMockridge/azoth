//! `process.risk_matrix` against `validation/neqsim/captures/process_risk_matrix.tsv`.

use azoth_core::units::kilograms_per_hour;
use azoth_process::models::risk_matrix::{
    RiskConsequenceCategory, RiskEquipment, RiskLevel, RiskProbabilityCategory, risk_matrix,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn equipment(
    name: &str,
    failures_per_year: f64,
    mttr_hours: f64,
    production_loss_percent: f64,
    loss_kg_hr: f64,
) -> RiskEquipment {
    RiskEquipment {
        name: name.to_string(),
        failures_per_year,
        mttr_hours,
        production_loss_percent,
        production_loss_kg_per_hr: kilograms_per_hour(loss_kg_hr),
    }
}

#[test]
fn the_five_bands_reproduce_the_capture() {
    let result = risk_matrix(
        &[
            equipment("EQ0", 0.05, 48.0, 3.0, 1000.0),
            equipment("EQ1", 0.3, 48.0, 15.0, 1000.0),
            equipment("EQ2", 0.7, 48.0, 35.0, 1000.0),
            equipment("EQ3", 1.5, 48.0, 65.0, 1000.0),
            equipment("EQ4", 3.0, 48.0, 90.0, 1000.0),
        ],
        Some(0.5),
        Some(5000.0),
    )
    .expect("the captured state is valid");
    assert_eq!(result.assessments.len(), 5);
    let first = &result.assessments[0];
    close(first.mtbf_hours, 175_200.0, "the MTBF");
    close(first.expected_downtime_hours_per_year, 2.4, "the downtime");
    assert_eq!(
        first.probability_category,
        RiskProbabilityCategory::VeryLow
    );
    assert_eq!(
        first.consequence_category,
        RiskConsequenceCategory::Negligible
    );
    close(first.risk_score, 1.0, "the score");
    assert_eq!(first.risk_level, RiskLevel::Low);
    close(first.cost_per_failure, 264_000.0, "the cost per failure");
    close(first.annual_risk_cost, 13_200.0, "the annual cost");

    let third = &result.assessments[2];
    close(third.mtbf_hours, 12_514.285_714_285_716, "EQ2's MTBF");
    assert_eq!(third.probability_category, RiskProbabilityCategory::Medium);
    assert_eq!(third.consequence_category, RiskConsequenceCategory::Moderate);
    close(third.risk_score, 9.0, "EQ2's score");
    assert_eq!(third.risk_level, RiskLevel::Medium);

    let fifth = &result.assessments[4];
    assert_eq!(fifth.probability_category, RiskProbabilityCategory::VeryHigh);
    assert_eq!(
        fifth.consequence_category,
        RiskConsequenceCategory::Catastrophic
    );
    close(fifth.risk_score, 25.0, "EQ4's score");
    assert_eq!(fifth.risk_level, RiskLevel::Critical);
    close(
        result.total_annual_risk_cost,
        1_465_200.0,
        "the total annual cost",
    );
}

#[test]
fn the_boundaries_reproduce_the_capture() {
    let result = risk_matrix(
        &[
            equipment("EQ0", 0.0, 48.0, 0.0, 1000.0),
            equipment("EQ1", 0.1, 48.0, 5.0, 1000.0),
            equipment("EQ2", 0.5, 48.0, 20.0, 1000.0),
            equipment("EQ3", 1.0, 48.0, 50.0, 1000.0),
            equipment("EQ4", 2.0, 48.0, 80.0, 1000.0),
            equipment("EQ5", 2.5, 48.0, 100.0, 1000.0),
        ],
        Some(0.5),
        Some(5000.0),
    )
    .expect("the captured state is valid");
    close(
        result.assessments[0].mtbf_hours,
        1_000_000.0,
        "the zero-failure sentinel",
    );
    close(
        result.assessments[0].expected_downtime_hours_per_year,
        0.0,
        "no downtime",
    );
    assert_eq!(
        result.assessments[1].probability_category,
        RiskProbabilityCategory::Low
    );
    assert_eq!(
        result.assessments[1].consequence_category,
        RiskConsequenceCategory::Minor
    );
    assert_eq!(
        result.assessments[3].probability_category,
        RiskProbabilityCategory::High
    );
    assert_eq!(
        result.assessments[3].consequence_category,
        RiskConsequenceCategory::Major
    );
    close(result.assessments[3].risk_score, 16.0, "EQ3's score");
    assert_eq!(result.assessments[3].risk_level, RiskLevel::Critical);
    close(
        result.assessments[4].expected_downtime_hours_per_year,
        96.0,
        "EQ4's downtime",
    );
    close(
        result.total_annual_risk_cost,
        1_610_400.0,
        "the total annual cost",
    );
}

#[test]
fn a_cost_state_reproduces_the_capture() {
    let result = risk_matrix(
        &[equipment("EQ0", 0.5, 48.0, 50.0, 1000.0)],
        Some(0.5),
        Some(5000.0),
    )
    .expect("the captured state is valid");
    let row = &result.assessments[0];
    close(row.expected_downtime_hours_per_year, 24.0, "the downtime");
    close(row.cost_per_failure, 264_000.0, "the cost per failure");
    close(row.annual_risk_cost, 132_000.0, "the annual cost");
    close(
        result.total_annual_risk_cost,
        132_000.0,
        "the total annual cost",
    );
}

#[test]
fn zero_failures_carry_the_sentinel_and_a_real_cost_per_failure() {
    let result = risk_matrix(
        &[equipment("EQ0", 0.0, 24.0, 10.0, 500.0)],
        Some(0.5),
        Some(5000.0),
    )
    .expect("the captured state is valid");
    let row = &result.assessments[0];
    close(row.mtbf_hours, 1_000_000.0, "the sentinel");
    close(row.cost_per_failure, 126_000.0, "the repair cost");
    close(row.annual_risk_cost, 0.0, "no failures, no annual cost");
    assert_eq!(row.risk_level, RiskLevel::Low);
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    assert!(
        risk_matrix(
            &[equipment("EQ", -0.5, 48.0, 15.0, 1000.0)],
            None,
            None
        )
        .is_err(),
        "a negative frequency"
    );
    assert!(
        risk_matrix(
            &[equipment("EQ", f64::NAN, 48.0, 15.0, 1000.0)],
            None,
            None
        )
        .is_err(),
        "a NaN frequency"
    );
    assert!(
        risk_matrix(&[equipment("EQ", 0.5, -48.0, 15.0, 1000.0)], None, None).is_err(),
        "a negative repair time"
    );
    assert!(
        risk_matrix(&[equipment("EQ", 0.5, 48.0, 101.0, 1000.0)], None, None).is_err(),
        "a loss above 100 percent"
    );
    assert!(
        risk_matrix(&[equipment("EQ", 0.5, 48.0, -1.0, 1000.0)], None, None).is_err(),
        "a negative loss"
    );
    assert!(
        risk_matrix(&[equipment("EQ", 0.5, 48.0, 15.0, -1000.0)], None, None).is_err(),
        "a negative mass loss"
    );
    assert!(
        risk_matrix(&[equipment("EQ", 0.5, 48.0, 15.0, 1000.0)], Some(-1.0), None).is_err(),
        "a negative price"
    );
    assert!(
        risk_matrix(&[equipment("EQ", 0.5, 48.0, 15.0, 1000.0)], None, Some(-1.0)).is_err(),
        "a negative downtime cost"
    );
}
