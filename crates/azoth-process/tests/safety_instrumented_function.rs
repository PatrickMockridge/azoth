//! `process.safety_instrumented_function` against
//! `validation/neqsim/captures/process_safety_instrumented_function.tsv`.

use azoth_process::models::safety_instrumented_function::safety_instrumented_function;

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn call(
    pfd: f64,
    sil: f64,
    unmitigated: f64,
    target: f64,
    interval: f64,
    lambda: Option<f64>,
    spurious: Option<f64>,
    mttr: Option<f64>,
    base: Option<f64>,
) -> azoth_process::models::safety_instrumented_function::SafetyInstrumentedFunctionResult {
    safety_instrumented_function(
        pfd,
        sil,
        unmitigated,
        target,
        interval,
        lambda,
        spurious,
        mttr,
        base,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_class_state_reproduces_the_capture() {
    let result = call(
        0.005,
        2.0,
        0.1,
        1.0e-5,
        8760.0,
        Some(1.0e-6),
        Some(0.0),
        Some(0.0),
        Some(0.99),
    );
    close(result.risk_reduction_factor.unwrap(), 200.0, "the RRF");
    close(
        result.mitigated_frequency_per_year,
        5.0e-4,
        "the mitigated frequency",
    );
    close(result.required_pfd, 1.0e-4, "the required PFD");
    close(result.required_sil, 3.0, "the required SIL");
    close(result.max_pfd_for_claimed_sil, 0.01, "the SIL 2 maximum");
    close(result.proof_test_interval_years, 1.0, "one year");
    assert!(result.pfd_within_claimed_sil, "the PFD is in band");
    close(result.pfd_1oo1.unwrap(), 0.00438, "the 1oo1 PFD");
    close(
        result.pfd_1oo2.unwrap(),
        2.557_920_000_000_000_4e-5,
        "the 1oo2 PFD",
    );
    close(
        result.pfd_2oo3.unwrap(),
        7.673_760_000_000_001e-5,
        "the 2oo3 PFD",
    );
    close(
        result.availability_with_spurious_trips.unwrap(),
        0.99,
        "no spurious trips",
    );
    assert!(result.warnings.is_empty(), "the PFD is in band");
}

#[test]
fn a_zero_unmitigated_frequency_answers_one() {
    let result = call(0.005, 2.0, 0.0, 1.0e-5, 8760.0, None, None, None, None);
    close(result.required_pfd, 1.0, "the class's one");
    close(result.required_sil, 1.0, "the SIL of one");
    close(
        result.mitigated_frequency_per_year,
        0.0,
        "no frequency to mitigate",
    );
    assert!(result.pfd_1oo1.is_none(), "no failure rate, no PFD");
    assert!(
        result.availability_with_spurious_trips.is_none(),
        "no base availability, no availability"
    );
}

#[test]
fn a_zero_pfd_has_no_risk_reduction_factor() {
    let result = call(
        0.0,
        2.0,
        0.1,
        1.0e-5,
        8760.0,
        Some(1.0e-6),
        Some(0.0),
        Some(0.0),
        Some(0.99),
    );
    assert!(
        result.risk_reduction_factor.is_none(),
        "the class's +Infinity is absent"
    );
    assert!(!result.pfd_within_claimed_sil, "zero is below the band");
    assert_eq!(result.warnings.len(), 1, "the out-of-band warning");
    close(
        result.mitigated_frequency_per_year,
        0.0,
        "a perfect barrier",
    );
}

#[test]
fn spurious_trips_derate_the_availability() {
    let result = call(
        0.005,
        2.0,
        0.1,
        1.0e-5,
        8760.0,
        Some(1.0e-6),
        Some(2.0),
        Some(48.0),
        Some(0.99),
    );
    close(
        result.availability_with_spurious_trips.unwrap(),
        0.979_150_684_931_506_9,
        "0.99 (1 - 96/8760)",
    );
}

#[test]
fn an_out_of_band_pfd_is_published_with_a_warning() {
    let result = call(
        0.5,
        1.0,
        0.1,
        1.0e-5,
        8760.0,
        Some(1.0e-6),
        Some(0.0),
        Some(0.0),
        Some(0.99),
    );
    close(result.risk_reduction_factor.unwrap(), 2.0, "the RRF");
    close(
        result.mitigated_frequency_per_year,
        0.05,
        "the mitigated frequency",
    );
    close(result.max_pfd_for_claimed_sil, 0.1, "the SIL 1 maximum");
    assert!(!result.pfd_within_claimed_sil, "outside the band");
    assert_eq!(result.warnings.len(), 1, "a warning, not a refusal");
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    assert!(
        safety_instrumented_function(-0.1, 2.0, 0.1, 1.0e-5, 8760.0, None, None, None, None)
            .is_err(),
        "a negative PFD"
    );
    assert!(
        safety_instrumented_function(f64::NAN, 2.0, 0.1, 1.0e-5, 8760.0, None, None, None, None)
            .is_err(),
        "a NaN PFD"
    );
    assert!(
        safety_instrumented_function(0.005, 0.0, 0.1, 1.0e-5, 8760.0, None, None, None, None)
            .is_err(),
        "SIL 0"
    );
    assert!(
        safety_instrumented_function(0.005, 5.0, 0.1, 1.0e-5, 8760.0, None, None, None, None)
            .is_err(),
        "SIL 5"
    );
    assert!(
        safety_instrumented_function(0.005, 2.5, 0.1, 1.0e-5, 8760.0, None, None, None, None)
            .is_err(),
        "a fractional SIL"
    );
}
