//! `process.tr3001_compliance` against
//! `validation/neqsim/captures/process_tr3001_compliance.tsv`.

use azoth_core::units::kilograms_per_second;
use azoth_process::models::tr3001_compliance::{
    Tr3001Check, Tr3001ComplianceResult, Tr3001Scenario, Tr3001Status, tr3001_compliance,
};

fn scenario(cause: &str, rate: f64, dynamic: bool, credible: bool) -> Tr3001Scenario {
    Tr3001Scenario {
        cause: cause.to_string(),
        relief_rate: kilograms_per_second(rate),
        dynamically_determined: dynamic,
        credible,
    }
}

fn check(
    rows: &[Tr3001Scenario],
    capacity: bool,
    area: Option<f64>,
    accepted: Option<bool>,
    fraction: Option<f64>,
) -> Tr3001ComplianceResult {
    tr3001_compliance(rows, area, capacity, accepted, fraction)
        .expect("the captured study state is valid")
}

fn expect(result: &Tr3001ComplianceResult, rows: &[(Tr3001Check, Tr3001Status)]) {
    let actual: Vec<(Tr3001Check, Tr3001Status)> = result
        .findings
        .iter()
        .map(|row| (row.check, row.status))
        .collect();
    assert_eq!(actual, rows);
}

#[test]
fn the_compliant_study_reproduces_the_capture() {
    let result = check(
        &[
            scenario("fire", 1.0, true, true),
            scenario("blocked_outlet", 0.5, true, true),
        ],
        true,
        Some(0.5),
        Some(true),
        Some(0.21),
    );
    expect(
        &result,
        &[
            (Tr3001Check::CredibleScenarios, Tr3001Status::Pass),
            (Tr3001Check::GoverningCase, Tr3001Status::Pass),
            (Tr3001Check::Capacity, Tr3001Status::Pass),
            (Tr3001Check::Acceptance, Tr3001Status::Pass),
            (Tr3001Check::FireBasis, Tr3001Status::Pass),
            (Tr3001Check::DynamicDetermination, Tr3001Status::Pass),
        ],
    );
    assert_eq!(result.pass_count, 6.0);
    assert_eq!(result.fail_count, 0.0);
    assert!(result.compliant);
}

#[test]
fn the_no_credible_study_skips_the_capacity_check() {
    let result = check(
        &[
            scenario("fire", 1.0, true, false),
            scenario("blocked_outlet", 0.5, true, false),
        ],
        true,
        Some(0.5),
        Some(true),
        Some(0.21),
    );
    expect(
        &result,
        &[
            (Tr3001Check::CredibleScenarios, Tr3001Status::Fail),
            (Tr3001Check::GoverningCase, Tr3001Status::Fail),
            (Tr3001Check::Acceptance, Tr3001Status::Pass),
        ],
    );
    assert_eq!(result.fail_count, 2.0);
    assert!(!result.compliant);
}

#[test]
fn the_unsized_and_review_states_reproduce_the_capture() {
    let unsized_state = check(
        &[scenario("fire", 1.0, false, true)],
        false,
        None,
        Some(true),
        Some(0.21),
    );
    expect(
        &unsized_state,
        &[
            (Tr3001Check::CredibleScenarios, Tr3001Status::Pass),
            (Tr3001Check::GoverningCase, Tr3001Status::Pass),
            (Tr3001Check::Capacity, Tr3001Status::NeedsReview),
            (Tr3001Check::Acceptance, Tr3001Status::Pass),
            (Tr3001Check::FireBasis, Tr3001Status::Pass),
            (Tr3001Check::DynamicDetermination, Tr3001Status::NeedsReview),
        ],
    );
    assert_eq!(unsized_state.review_count, 2.0);
    assert!(unsized_state.compliant, "needs-review does not fail");

    let review = check(
        &[scenario("fire", 1.0, true, true)],
        true,
        Some(0.5),
        Some(true),
        Some(0.10),
    );
    expect(
        &review,
        &[
            (Tr3001Check::CredibleScenarios, Tr3001Status::Pass),
            (Tr3001Check::GoverningCase, Tr3001Status::Pass),
            (Tr3001Check::Capacity, Tr3001Status::Pass),
            (Tr3001Check::Acceptance, Tr3001Status::Pass),
            (Tr3001Check::FireBasis, Tr3001Status::NeedsReview),
            (Tr3001Check::DynamicDetermination, Tr3001Status::Pass),
        ],
    );
    assert!(review.compliant);
}

#[test]
fn the_failing_states_reproduce_the_capture() {
    let capacity = check(
        &[scenario("fire", 1.0, true, true)],
        false,
        Some(30.0),
        Some(true),
        Some(0.21),
    );
    assert_eq!(
        capacity.findings[2].status,
        Tr3001Status::Fail,
        "the capacity check"
    );
    assert!(!capacity.compliant);

    let acceptance = check(
        &[scenario("fire", 1.0, true, true)],
        true,
        Some(0.5),
        Some(false),
        Some(0.21),
    );
    assert_eq!(
        acceptance.findings[3].status,
        Tr3001Status::Fail,
        "the acceptance check"
    );
    assert!(!acceptance.compliant);
}

#[test]
fn the_skipped_checks_reproduce_the_capture() {
    let other = check(
        &[scenario("control_valve_failure", 1.0, false, true)],
        true,
        Some(0.5),
        Some(true),
        Some(0.10),
    );
    assert_eq!(
        other.findings.len(),
        4,
        "fire basis and dynamic are skipped"
    );
    assert!(other.compliant);

    let null_acceptance = check(
        &[scenario("blocked_outlet", 1.0, false, true)],
        true,
        Some(0.5),
        None,
        None,
    );
    expect(
        &null_acceptance,
        &[
            (Tr3001Check::CredibleScenarios, Tr3001Status::Pass),
            (Tr3001Check::GoverningCase, Tr3001Status::Pass),
            (Tr3001Check::Capacity, Tr3001Status::Pass),
            (Tr3001Check::DynamicDetermination, Tr3001Status::NeedsReview),
        ],
    );
    assert!(null_acceptance.compliant);
}

#[test]
fn the_empty_scenario_list_is_a_defined_state() {
    let empty = check(&[], false, None, None, None);
    assert_eq!(empty.findings.len(), 2, "credible and governing only");
    assert_eq!(empty.fail_count, 2.0);
    assert!(!empty.compliant);
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    assert!(
        tr3001_compliance(
            &[scenario("fire", -1.0, true, true)],
            None,
            false,
            None,
            None
        )
        .is_err(),
        "a negative rate"
    );
    assert!(
        tr3001_compliance(
            &[scenario("fire", 1.0, true, true)],
            Some(-1.0),
            false,
            None,
            None
        )
        .is_err(),
        "a negative required area"
    );
    assert!(
        tr3001_compliance(
            &[scenario("fire", 1.0, true, true)],
            Some(0.5),
            true,
            Some(true),
            None
        )
        .is_err(),
        "a half-stated acceptance"
    );
    assert!(
        tr3001_compliance(
            &[scenario("not_a_cause", 1.0, true, true)],
            None,
            false,
            None,
            None
        )
        .is_err(),
        "a cause the class does not name"
    );
}
