//! `process.sil_verification` against
//! `validation/neqsim/captures/process_sil_verification.tsv`.

use azoth_process::models::sil_verification::{IssueCategory, IssueSeverity, sil_verification};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[test]
fn the_canonical_state_passes_with_no_issue() {
    let result = sil_verification(0.005, 2.0, "1oo1", 8760.0).expect("the captured state is valid");
    close(result.achieved_sil, 2.0, "the achieved SIL");
    assert!(result.sil_achieved, "SIL 2 claims SIL 2");
    close(result.pfd_upper, 0.0075, "the upper PFD");
    close(result.hardware_fault_tolerance, 0.0, "1oo1 has none");
    close(result.diagnostic_coverage, 0.5, "1oo1's coverage");
    close(result.issue_count, 0.0, "no issues");
    assert!(result.verification_passed, "nothing failed");
}

#[test]
fn a_pfd_above_the_claimed_maximum_is_an_error() {
    let result = sil_verification(0.05, 2.0, "1oo1", 8760.0).expect("the captured state is valid");
    close(result.achieved_sil, 1.0, "SIL 1");
    assert!(!result.sil_achieved, "the claim is not met");
    close(result.pfd_upper, 0.075_000_000_000_000_01, "the upper PFD");
    assert_eq!(result.issues.len(), 1);
    assert_eq!(result.issues[0].severity, IssueSeverity::Error);
    assert_eq!(result.issues[0].category, IssueCategory::PfdExceeded);
    assert!(!result.verification_passed, "an error fails it");
}

#[test]
fn sil4_on_a_1oo2_pair_raises_both_an_error_and_a_warning() {
    let result =
        sil_verification(5.0e-5, 4.0, "1oo2", 8760.0).expect("the captured state is valid");
    close(
        result.hardware_fault_tolerance,
        1.0,
        "1oo2's fault tolerance",
    );
    close(result.diagnostic_coverage, 0.6, "the oo2 substring");
    assert_eq!(result.issues.len(), 2);
    assert_eq!(result.issues[0].severity, IssueSeverity::Error);
    assert_eq!(result.issues[0].category, IssueCategory::Architecture);
    assert_eq!(result.issues[1].severity, IssueSeverity::Warning);
    assert_eq!(result.issues[1].category, IssueCategory::DiagnosticCoverage);
    assert!(
        !result.verification_passed,
        "the architecture error fails it"
    );
}

#[test]
fn sil3_on_a_single_channel_raises_two_warnings_and_passes() {
    let result =
        sil_verification(5.0e-4, 3.0, "1oo1", 8760.0).expect("the captured state is valid");
    assert_eq!(result.issues.len(), 2);
    assert_eq!(result.issues[0].category, IssueCategory::Architecture);
    assert_eq!(result.issues[1].category, IssueCategory::DiagnosticCoverage);
    assert!(result.verification_passed, "warnings do not fail it");
}

#[test]
fn the_architecture_table_and_the_coverage_substring() {
    let one_of_three =
        sil_verification(5.0e-5, 4.0, "1oo3", 8760.0).expect("the captured state is valid");
    close(
        one_of_three.hardware_fault_tolerance,
        2.0,
        "1oo3's fault tolerance",
    );
    assert_eq!(one_of_three.error_count, 0.0, "no architecture error");

    let four_oo_two =
        sil_verification(5.0e-4, 3.0, "4oo2", 8760.0).expect("the captured state is valid");
    close(
        four_oo_two.diagnostic_coverage,
        0.6,
        "the oo2 substring in 4oo2",
    );

    let two_oo_four =
        sil_verification(0.005, 2.0, "2oo4", 8760.0).expect("the captured state is valid");
    close(
        two_oo_four.diagnostic_coverage,
        0.5,
        "2oo4 has no oo2 or oo3",
    );
    close(
        two_oo_four.hardware_fault_tolerance,
        0.0,
        "unknown table entry",
    );
}

#[test]
fn a_long_proof_interval_is_a_warning_and_five_years_is_not() {
    let long = sil_verification(5.0e-4, 3.0, "2oo3", 87601.0).expect("the captured state is valid");
    assert!(
        long.issues
            .iter()
            .any(|issue| issue.category == IssueCategory::ProofTestInterval),
        "ten years is a warning"
    );
    let boundary =
        sil_verification(5.0e-4, 3.0, "2oo3", 43800.0).expect("the captured state is valid");
    assert!(
        !boundary
            .issues
            .iter()
            .any(|issue| issue.category == IssueCategory::ProofTestInterval),
        "exactly five years is not"
    );
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    assert!(
        sil_verification(f64::NAN, 2.0, "1oo1", 8760.0).is_err(),
        "the class answers achieved SIL 4 and passes a NaN"
    );
    assert!(
        sil_verification(-0.1, 2.0, "1oo1", 8760.0).is_err(),
        "a negative PFD"
    );
    assert!(
        sil_verification(0.005, 0.0, "1oo1", 8760.0).is_err(),
        "SIL 0"
    );
    assert!(
        sil_verification(0.005, 5.0, "1oo1", 8760.0).is_err(),
        "SIL 5"
    );
    assert!(
        sil_verification(0.005, 2.5, "1oo1", 8760.0).is_err(),
        "a fractional SIL"
    );
}
