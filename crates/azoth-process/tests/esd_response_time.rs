//! `process.esd_response_time` against
//! `validation/neqsim/captures/process_esd_response_time.tsv`.

use azoth_core::units::seconds;
use azoth_process::models::esd_response_time::{EsdContribution, EsdStage, esd_response_time};

fn row(name: &str, stage: EsdStage, time_s: f64) -> EsdContribution {
    EsdContribution {
        name: name.to_string(),
        stage,
        time: seconds(time_s),
    }
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= 1.0e-12,
        "{what}: {actual} against {expected}"
    );
}

#[test]
fn the_class_within_budget_state() {
    let rows = [
        row("PT-1001 high-pressure", EsdStage::Detection, 1.0),
        row("Logic solver scan + 2oo3 vote", EsdStage::Logic, 0.5),
        row("ESDV-2001", EsdStage::FinalElement, 8.5),
    ];
    let result = esd_response_time(Some("ESD-1234"), &rows, Some(seconds(15.0)))
        .expect("the state evaluates");
    assert_eq!(result.sif_tag, "ESD-1234");
    close(result.detection_time.value, 1.0, "detection");
    close(result.logic_time.value, 0.5, "logic");
    close(result.final_element_time.value, 8.5, "final element");
    close(result.total_response_time.value, 10.0, "total");
    close(
        result
            .allowable_response_time
            .expect("the budget is stated")
            .value,
        15.0,
        "allowable",
    );
    close(
        result.margin.expect("the margin is stated").value,
        5.0,
        "margin",
    );
    assert!(result.within_budget);
    assert_eq!(result.contributions.len(), 3);
    assert_eq!(result.contributions[2].stage, EsdStage::FinalElement);
    close(result.contributions[2].time.value, 8.5, "echoed valve time");
}

#[test]
fn the_class_exceeds_budget_state() {
    let rows = [
        row("LT-3001 high-level", EsdStage::Detection, 2.0),
        row("Logic solver", EsdStage::Logic, 0.5),
        row("ESDV-3002 (large bore)", EsdStage::FinalElement, 21.0),
    ];
    let result = esd_response_time(Some("ESD-9999"), &rows, Some(seconds(15.0)))
        .expect("the state evaluates");
    close(result.total_response_time.value, 23.5, "total");
    close(
        result.margin.expect("the margin is stated").value,
        -8.5,
        "margin",
    );
    assert!(!result.within_budget);
}

#[test]
fn no_budget_is_within_and_has_no_margin() {
    let rows = [row("TT-1", EsdStage::Detection, 1.0)];
    let result = esd_response_time(Some("ESD-NOBUDGET"), &rows, None).expect("the state evaluates");
    close(result.total_response_time.value, 1.0, "total");
    assert!(result.allowable_response_time.is_none());
    assert!(result.margin.is_none());
    assert!(result.within_budget);
    assert_eq!(result.sif_tag, "ESD-NOBUDGET");
}

#[test]
fn an_equal_budget_passes_and_a_zero_contribution_is_kept() {
    let rows = [
        row("DT-1", EsdStage::Detection, 0.0),
        row("logic", EsdStage::Logic, 2.0),
    ];
    let result = esd_response_time(Some("ESD-ZERO"), &rows, Some(seconds(2.0)))
        .expect("the state evaluates");
    close(result.total_response_time.value, 2.0, "total");
    close(
        result.margin.expect("the margin is stated").value,
        0.0,
        "margin",
    );
    assert!(result.within_budget);
    close(result.contributions[0].time.value, 0.0, "zero contribution");
}

#[test]
fn the_default_tag_the_empty_list_and_a_negative_time() {
    let rows = [row("TT-1", EsdStage::Detection, 1.0)];
    let result = esd_response_time(None, &rows, None).expect("the state evaluates");
    assert_eq!(result.sif_tag, "ESD-SIF");
    assert!(esd_response_time(None, &[], None).is_err());
    let negative = [row("DT-1", EsdStage::Detection, -1.0)];
    assert!(esd_response_time(None, &negative, None).is_err());
}
