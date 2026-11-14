//! `process.bow_tie` against `validation/neqsim/captures/process_bow_tie.tsv`.

use azoth_process::models::bow_tie::{
    BowTieBarrier, BowTieConsequence, BowTieThreat, bow_tie,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn threat(frequency: f64, active: bool) -> BowTieThreat {
    BowTieThreat {
        frequency,
        active: Some(active),
    }
}

fn severity(severity: f64) -> BowTieConsequence {
    BowTieConsequence { severity }
}

fn barrier(pfd: f64, barrier_type: &str, functional: bool) -> BowTieBarrier {
    BowTieBarrier {
        pfd,
        barrier_type: barrier_type.to_string(),
        functional: Some(functional),
    }
}

#[test]
fn the_class_test_state_reproduces_the_capture() {
    let result = bow_tie(
        &[threat(0.1, true), threat(0.05, true)],
        &[severity(5.0)],
        &[barrier(0.01, "prevention", true)],
    )
    .expect("the captured state is valid");
    close(
        result.unmitigated_frequency_per_year,
        0.150_000_000_000_000_02,
        "the active threat sum",
    );
    close(
        result.mitigated_frequency_per_year,
        0.001_500_000_000_000_000_2,
        "the prevention product",
    );
    close(result.total_rrf, 100.0, "the RRF");
    close(result.max_severity, 5.0, "the severity");
}

#[test]
fn a_mitigation_barrier_does_not_move_the_frequency() {
    let result = bow_tie(
        &[threat(0.1, true)],
        &[severity(4.0)],
        &[
            barrier(0.01, "prevention", true),
            barrier(0.5, "mitigation", true),
        ],
    )
    .expect("the captured state is valid");
    close(result.unmitigated_frequency_per_year, 0.1, "the sum");
    close(result.mitigated_frequency_per_year, 0.001, "the prevention only");
    close(result.total_rrf, 100.0, "the RRF");
}

#[test]
fn both_inactive_threats_and_functional_gates_are_read() {
    let result = bow_tie(
        &[threat(0.1, true), threat(0.2, false)],
        &[severity(3.0), severity(5.0)],
        &[
            barrier(0.1, "both", true),
            barrier(0.01, "prevention", false),
        ],
    )
    .expect("the captured state is valid");
    close(
        result.unmitigated_frequency_per_year,
        0.1,
        "the active threat only",
    );
    close(
        result.mitigated_frequency_per_year,
        0.010_000_000_000_000_002,
        "the both barrier only",
    );
    close(result.total_rrf, 9.999_999_999_999_998, "the RRF");
    close(result.max_severity, 5.0, "the largest severity");
}

#[test]
fn the_link_between_a_threat_and_a_barrier_is_not_read() {
    let result = bow_tie(
        &[threat(0.1, true)],
        &[severity(2.0)],
        &[
            barrier(0.1, "prevention", true),
            barrier(0.2, "prevention", true),
        ],
    )
    .expect("the captured state is valid");
    close(
        result.mitigated_frequency_per_year,
        0.002_000_000_000_000_000_5,
        "both barriers multiply",
    );
    close(result.total_rrf, 49.999_999_999_999_99, "the RRF");
}

#[test]
fn a_nonfunctional_barrier_leaves_the_frequency_alone() {
    let result = bow_tie(
        &[threat(0.2, true)],
        &[severity(4.0)],
        &[barrier(0.01, "prevention", false)],
    )
    .expect("the captured state is valid");
    close(result.unmitigated_frequency_per_year, 0.2, "the sum");
    close(result.mitigated_frequency_per_year, 0.2, "the same frequency");
    close(result.total_rrf, 1.0, "no reduction");
}

#[test]
fn the_empty_states_are_answered() {
    let empty = bow_tie(&[], &[], &[]).expect("an empty bow-tie is valid");
    close(empty.unmitigated_frequency_per_year, 0.0, "no threats");
    close(empty.mitigated_frequency_per_year, 0.0, "no mitigation");
    close(empty.total_rrf, 0.0, "the class's zero at zero mitigated");
    close(empty.max_severity, 0.0, "no consequences");

    let inactive = bow_tie(&[threat(0.1, false)], &[severity(5.0)], &[])
        .expect("an inactive threat is valid");
    close(inactive.unmitigated_frequency_per_year, 0.0, "the inactive sum");
    close(inactive.max_severity, 5.0, "the severity still reads");
}

#[test]
fn the_values_the_class_accepts_are_refused() {
    assert!(
        bow_tie(&[threat(-1.0, true)], &[], &[]).is_err(),
        "a negative frequency"
    );
    assert!(
        bow_tie(&[threat(f64::NAN, true)], &[], &[]).is_err(),
        "a NaN frequency"
    );
    assert!(
        bow_tie(&[], &[severity(-1.0)], &[]).is_err(),
        "a negative severity"
    );
    assert!(
        bow_tie(&[], &[severity(1.5)], &[]).is_err(),
        "a fractional severity"
    );
    assert!(
        bow_tie(&[], &[], &[barrier(1.5, "prevention", true)]).is_err(),
        "a PFD above one"
    );
    assert!(
        bow_tie(&[], &[], &[barrier(-0.5, "prevention", true)]).is_err(),
        "a negative PFD"
    );
    assert!(
        bow_tie(&[], &[], &[barrier(f64::NAN, "prevention", true)]).is_err(),
        "a NaN PFD"
    );
    assert!(
        bow_tie(&[], &[], &[barrier(0.1, "detection", true)]).is_err(),
        "a barrier type the class does not define"
    );
}
