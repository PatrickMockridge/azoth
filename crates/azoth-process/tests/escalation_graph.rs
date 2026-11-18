//! `process.escalation_graph` against `validation/neqsim/captures/process_escalation_graph.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_process::models::escalation_graph::{
    EscalationExposure, EscalationGraphResult, EscalationItem, escalation_graph,
};

fn item(item_id: &str, failure_threshold: f64) -> EscalationItem {
    EscalationItem {
        item_id: item_id.to_string(),
        failure_threshold,
    }
}

fn exposure(source: &str, target: &str, load: f64) -> EscalationExposure {
    EscalationExposure {
        source: source.to_string(),
        target: target.to_string(),
        load,
    }
}

fn names(result: &EscalationGraphResult, worst: bool) -> Vec<String> {
    let rows = if worst {
        &result.worst_case_items
    } else {
        &result.failed_items
    };
    rows.iter().map(|row| row.item_id.clone()).collect()
}

fn evaluate(
    items: &[EscalationItem],
    exposures: &[EscalationExposure],
    initiator: &str,
) -> EscalationGraphResult {
    escalation_graph(items, exposures, initiator).expect("the captured state is valid")
}

#[test]
fn the_chain_propagates_to_every_item() {
    let result = evaluate(
        &[item("A", 0.0), item("B", 50.0), item("C", 50.0)],
        &[exposure("A", "B", 100.0), exposure("B", "C", 100.0)],
        "A",
    );
    assert_eq!(result.failure_count, 3.0);
    assert!(result.any_escalation);
    assert_eq!(names(&result, false), ["A", "B", "C"]);
    assert_eq!(names(&result, true), ["A", "B", "C"]);
}

#[test]
fn a_load_below_the_threshold_does_not_propagate() {
    let result = evaluate(
        &[item("A", 0.0), item("B", 50.0)],
        &[exposure("A", "B", 10.0)],
        "A",
    );
    assert_eq!(result.failure_count, 1.0);
    assert!(!result.any_escalation);
    assert_eq!(names(&result, false), ["A"]);
}

#[test]
fn the_threshold_comparison_is_inclusive() {
    let result = evaluate(
        &[item("A", 0.0), item("B", 50.0)],
        &[exposure("A", "B", 50.0)],
        "A",
    );
    assert_eq!(result.failure_count, 2.0);
}

#[test]
fn two_paths_accumulate_on_the_target() {
    let result = evaluate(
        &[item("A", 0.0), item("B", 50.0), item("C", 50.0)],
        &[
            exposure("A", "B", 100.0),
            exposure("A", "C", 30.0),
            exposure("B", "C", 30.0),
        ],
        "A",
    );
    assert_eq!(names(&result, false), ["A", "B", "C"]);
}

#[test]
fn the_worst_case_is_the_largest_propagation() {
    let result = evaluate(
        &[
            item("A", 0.0),
            item("B", 50.0),
            item("C", 50.0),
            item("D", 50.0),
        ],
        &[
            exposure("A", "B", 100.0),
            exposure("A", "C", 100.0),
            exposure("A", "D", 100.0),
        ],
        "B",
    );
    assert_eq!(result.failure_count, 1.0);
    assert_eq!(result.worst_case_count, 4.0);
    assert_eq!(names(&result, true), ["A", "B", "C", "D"]);
}

#[test]
fn a_cycle_stops_at_its_fixed_point() {
    let result = evaluate(
        &[item("A", 50.0), item("B", 50.0)],
        &[exposure("A", "B", 100.0), exposure("B", "A", 100.0)],
        "A",
    );
    assert_eq!(names(&result, false), ["A", "B"]);
}

#[test]
fn a_duplicate_item_keeps_its_first_position_and_its_last_threshold() {
    let result = evaluate(
        &[item("A", 0.0), item("B", 50.0), item("B", 75.0)],
        &[exposure("A", "B", 80.0)],
        "A",
    );
    assert_eq!(names(&result, false), ["A", "B"]);

    // The later threshold is the one read: 80 does not reach 100.
    let raised = evaluate(
        &[item("A", 0.0), item("B", 50.0), item("B", 100.0)],
        &[exposure("A", "B", 80.0)],
        "A",
    );
    assert_eq!(names(&raised, false), ["A"]);
}

#[test]
fn the_refusals_are_the_graph_shape() {
    // An unknown initiator and an exposure endpoint that names no item.
    assert!(escalation_graph(&[item("A", 0.0)], &[], "X").is_err());
    assert!(escalation_graph(&[item("A", 0.0)], &[exposure("A", "X", 1.0)], "A").is_err());
    // A non-finite threshold or load.
    assert!(escalation_graph(&[item("A", f64::NAN)], &[], "A").is_err());
    assert!(
        escalation_graph(
            &[item("A", 0.0), item("B", 1.0)],
            &[exposure("A", "B", f64::INFINITY)],
            "A"
        )
        .is_err()
    );
}
