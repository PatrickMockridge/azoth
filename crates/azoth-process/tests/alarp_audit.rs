//! `process.alarp_audit` against `validation/neqsim/captures/process_alarp_audit.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_process::models::alarp_audit::{
    AlarpAuditResult, AlarpMeasure, AlarpVerdict, alarp_audit,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn measure(risk_reduction: f64, cost: f64) -> AlarpMeasure {
    AlarpMeasure {
        description: "vent".to_string(),
        risk_reduction_per_year: risk_reduction,
        annualised_cost: cost,
    }
}

fn evaluate(measures: &[AlarpMeasure], vsl: Option<f64>, factor: Option<f64>) -> AlarpAuditResult {
    alarp_audit(measures, vsl, factor).expect("the captured state is valid")
}

#[test]
fn the_default_threshold_and_the_verdicts() {
    let implement = evaluate(&[measure(1.0e-3, 1.0e4)], None, None);
    close(implement.threshold, 9.0e7, "the default threshold");
    close(implement.measure_count, 1.0, "the measure count");
    close(
        implement.evaluations[0].icaf.expect("a finite ICAF"),
        1.0e7,
        "the implemented ICAF",
    );
    assert_eq!(implement.evaluations[0].verdict, AlarpVerdict::Implement);

    let reject = evaluate(&[measure(1.0e-3, 1.0e6)], None, None);
    close(
        reject.evaluations[0].icaf.expect("a finite ICAF"),
        1.0e9,
        "the rejected ICAF",
    );
    assert_eq!(reject.evaluations[0].verdict, AlarpVerdict::Reject);

    let equal = evaluate(&[measure(1.0e-3, 9.0e4)], None, None);
    assert_eq!(equal.evaluations[0].verdict, AlarpVerdict::Implement);
}

#[test]
fn a_zero_risk_measure_has_an_absent_icaf_and_is_rejected() {
    let result = evaluate(&[measure(0.0, 0.0)], None, None);
    assert!(result.evaluations[0].icaf.is_none());
    assert_eq!(result.evaluations[0].verdict, AlarpVerdict::Reject);

    let free = evaluate(&[measure(1.0e-3, 0.0)], None, None);
    close(
        free.evaluations[0].icaf.expect("a finite ICAF"),
        0.0,
        "the free ICAF",
    );
    assert_eq!(free.evaluations[0].verdict, AlarpVerdict::Implement);
}

#[test]
fn a_custom_value_and_factor_move_the_threshold() {
    let result = evaluate(&[measure(1.0e-3, 2.0e4)], Some(1.0e7), Some(2.0));
    close(result.threshold, 2.0e7, "the custom threshold");
    assert_eq!(result.evaluations[0].verdict, AlarpVerdict::Implement);
}

#[test]
fn an_empty_measure_list_is_an_empty_audit() {
    let result = evaluate(&[], None, None);
    close(result.measure_count, 0.0, "the empty count");
    assert!(result.evaluations.is_empty());
}

#[test]
fn the_refusals_are_the_class_validations() {
    assert!(alarp_audit(&[measure(-1.0, 1.0)], None, None).is_err());
    assert!(alarp_audit(&[measure(1.0, -1.0)], None, None).is_err());
    assert!(alarp_audit(&[measure(f64::NAN, 1.0)], None, None).is_err());
    assert!(alarp_audit(&[], Some(f64::NAN), None).is_err());
    assert!(alarp_audit(&[], None, Some(f64::INFINITY)).is_err());
}
