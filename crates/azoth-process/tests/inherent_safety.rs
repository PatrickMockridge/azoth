//! `process.inherent_safety` against `validation/neqsim/captures/process_inherent_safety.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_process::models::inherent_safety::inherent_safety;

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[test]
fn the_index_is_the_mean_of_the_four_scores() {
    let mixed = inherent_safety(Some(8.0), Some(6.0), Some(4.0), Some(6.0))
        .expect("the captured state is valid");
    close(mixed.overall_index, 6.0, "the mixed mean");

    let zero = inherent_safety(None, None, None, None).expect("the zero evaluator is valid");
    close(zero.overall_index, 0.0, "the zero mean");

    let bounds = inherent_safety(Some(0.0), Some(10.0), Some(0.0), Some(10.0))
        .expect("the boundary state is valid");
    close(bounds.overall_index, 5.0, "the boundary mean");

    let one = inherent_safety(None, None, None, Some(10.0)).expect("one pillar is valid");
    close(one.overall_index, 2.5, "one pillar");
}

#[test]
fn the_refusals_are_the_class_validations() {
    assert!(inherent_safety(Some(11.0), None, None, None).is_err());
    assert!(inherent_safety(None, Some(-1.0), None, None).is_err());
    assert!(inherent_safety(None, None, Some(f64::NAN), None).is_err());
}
