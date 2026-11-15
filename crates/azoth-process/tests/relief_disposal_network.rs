//! `process.relief_disposal_network` against
//! `validation/neqsim/captures/process_relief_disposal_network.tsv`.

use azoth_core::units::{kilogram_per_hour, kilograms_per_second};
use azoth_process::models::relief_disposal_network::{
    ReliefCause, ReliefContribution, ReliefDisposalNetworkResult, ReliefPhase,
    relief_disposal_network,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn row(name: &str, cause: &str, phase: &str, rate: f64, simultaneous: bool) -> ReliefContribution {
    ReliefContribution {
        item_name: name.to_string(),
        cause: cause.to_string(),
        phase: phase.to_string(),
        mass_flow: kilograms_per_second(rate),
        simultaneous,
    }
}

fn network(rows: &[ReliefContribution]) -> ReliefDisposalNetworkResult {
    relief_disposal_network(rows).expect("the captured network state is valid")
}

#[test]
fn the_simultaneous_total_and_peak_reproduce_the_capture() {
    let all = network(&[
        row("A", "fire", "vapour", 1.0, true),
        row("B", "blocked_outlet", "liquid", 0.5, true),
        row("C", "control_valve_failure", "vapour", 2.0, true),
    ]);
    close(all.total_simultaneous.value, 3.5, "the total");
    close(
        all.total_simultaneous_kg_hr.get::<kilogram_per_hour>(),
        12_600.0,
        "the hourly total",
    );
    close(all.peak_single.value, 2.0, "the peak");
    assert_eq!(all.governing_contributor.as_deref(), Some("C"));
    assert_eq!(all.contributions.len(), 3);
    assert_eq!(all.contributions[0].cause, ReliefCause::Fire);
    assert_eq!(all.contributions[1].phase, ReliefPhase::Liquid);
    close(
        all.contributions[2]
            .mass_flow_kg_hr
            .get::<kilogram_per_hour>(),
        7_200.0,
        "the row's hourly rate",
    );

    let mixed = network(&[
        row("A", "fire", "vapour", 1.0, true),
        row("B", "blocked_outlet", "liquid", 2.0, false),
        row("C", "tube_rupture", "two_phase", 0.5, true),
    ]);
    close(mixed.total_simultaneous.value, 1.5, "the mixed total");
    close(mixed.peak_single.value, 2.0, "the mixed peak");
    assert_eq!(mixed.governing_contributor.as_deref(), Some("B"));
    assert_eq!(mixed.contributions[2].phase, ReliefPhase::TwoPhase);
}

#[test]
fn the_tie_and_the_zero_states_reproduce_the_capture() {
    let tie = network(&[
        row("A", "fire", "vapour", 1.0, true),
        row("B", "fire", "vapour", 1.0, true),
    ]);
    close(tie.peak_single.value, 1.0, "the tied peak");
    assert_eq!(
        tie.governing_contributor.as_deref(),
        Some("A"),
        "the first row wins the tie"
    );

    let zero = network(&[
        row("A", "other", "vapour", 0.0, true),
        row("B", "other", "vapour", 0.0, false),
    ]);
    close(zero.total_simultaneous.value, 0.0, "the zero total");
    close(zero.peak_single.value, 0.0, "the zero peak");
    assert!(zero.governing_contributor.is_none(), "no positive rate");
}

#[test]
fn the_empty_network_is_a_defined_state() {
    let empty = network(&[]);
    close(empty.total_simultaneous.value, 0.0, "the empty total");
    close(empty.peak_single.value, 0.0, "the empty peak");
    assert!(empty.governing_contributor.is_none(), "no governing row");
    assert!(empty.contributions.is_empty());
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    assert!(
        relief_disposal_network(&[row("A", "other", "vapour", -1.0, true)]).is_err(),
        "a negative rate"
    );
    assert!(
        relief_disposal_network(&[row("A", "not_a_cause", "vapour", 1.0, true)]).is_err(),
        "a cause the class does not name"
    );
    assert!(
        relief_disposal_network(&[row("A", "other", "plasma", 1.0, true)]).is_err(),
        "a phase the class does not name"
    );
}
