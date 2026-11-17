//! `process.s001_secondary_pressure_protection` against
//! `validation/neqsim/captures/process_s001_secondary_pressure_protection.tsv`.

use azoth_core::units::pascals;
use azoth_process::models::s001_secondary_pressure_protection::{
    S001SecondaryPressureProtectionResult, s001_secondary_pressure_protection,
};

fn bara(value: f64) -> Option<azoth_core::units::Pressure> {
    Some(pascals(value * 1.0e5))
}

#[allow(clippy::too_many_arguments)]
fn screen(
    event: Option<f64>,
    design: Option<f64>,
    test: Option<f64>,
    demand: Option<f64>,
    target: Option<f64>,
    assessed: Option<bool>,
    safe: Option<bool>,
    proof: Option<f64>,
) -> S001SecondaryPressureProtectionResult {
    s001_secondary_pressure_protection(
        event.and_then(bara),
        design.and_then(bara),
        test.and_then(bara),
        demand,
        target,
        assessed,
        safe,
        proof,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_default_target_follows_the_pressure_bands() {
    let cases = [
        (90.0, 1.0e-2),
        (120.0, 1.0e-3),
        (175.0, 1.0e-4),
        (201.0, 1.0e-5),
    ];
    for (event, expected) in cases {
        let result = screen(
            Some(event),
            Some(100.0),
            Some(150.0),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            result.target_frequency_per_year,
            Some(expected),
            "event {event}"
        );
        assert!(result.pressure_basis_complete);
        assert!(!result.frequency_configured);
        assert!(!result.acceptable);
    }
}

#[test]
fn the_complete_case_is_acceptable() {
    let result = screen(
        Some(120.0),
        Some(100.0),
        Some(150.0),
        Some(1.0e-4),
        None,
        Some(true),
        Some(true),
        Some(6.0),
    );
    assert_eq!(result.target_frequency_per_year, Some(1.0e-3));
    assert!(result.pressure_basis_complete);
    assert!(result.test_pressure_valid);
    assert!(result.pressure_within_test);
    assert!(result.frequency_configured);
    assert!(result.frequency_criterion_met);
    assert!(result.leakage_assessed);
    assert!(result.leakage_routed_safe);
    assert!(result.proof_test_configured);
    assert!(result.proof_test_criterion_met);
    assert!(result.acceptable);
}

#[test]
fn an_event_above_the_test_pressure_fails() {
    let result = screen(
        Some(175.0),
        Some(100.0),
        Some(150.0),
        Some(1.0e-5),
        None,
        Some(true),
        Some(true),
        Some(6.0),
    );
    assert!(!result.pressure_within_test);
    assert!(result.frequency_criterion_met);
    assert!(!result.acceptable);
}

#[test]
fn a_missing_basis_leaves_the_target_absent() {
    let result = screen(
        None,
        None,
        None,
        Some(1.0e-3),
        None,
        Some(true),
        Some(true),
        Some(6.0),
    );
    assert!(!result.pressure_basis_complete);
    assert_eq!(result.target_frequency_per_year, None);
    assert!(!result.frequency_configured);
    assert!(result.frequency_criterion_met);
    assert!(!result.acceptable);
}

#[test]
fn the_leakage_flags_are_true_only() {
    for flags in [(None, None), (Some(false), Some(false))] {
        let result = screen(
            Some(120.0),
            Some(100.0),
            Some(150.0),
            Some(1.0e-4),
            None,
            flags.0,
            flags.1,
            Some(6.0),
        );
        assert!(!result.leakage_assessed, "{flags:?}");
        assert!(!result.leakage_routed_safe, "{flags:?}");
        assert!(!result.acceptable, "{flags:?}");
    }
}

#[test]
fn an_explicit_target_overrides_the_band() {
    let result = screen(
        Some(90.0),
        Some(100.0),
        Some(150.0),
        Some(1.0e-5),
        Some(1.0e-6),
        Some(true),
        Some(true),
        Some(6.0),
    );
    assert_eq!(result.target_frequency_per_year, Some(1.0e-6));
    assert!(result.frequency_configured);
    assert!(!result.frequency_criterion_met);
    assert!(!result.acceptable);
}

#[test]
fn a_thirteen_month_interval_fails_and_six_passes() {
    let thirteen = screen(
        Some(120.0),
        Some(100.0),
        Some(150.0),
        Some(1.0e-4),
        None,
        Some(true),
        Some(true),
        Some(13.0),
    );
    assert!(thirteen.proof_test_configured);
    assert!(!thirteen.proof_test_criterion_met);
    assert!(!thirteen.acceptable);
    let six = screen(
        Some(120.0),
        Some(100.0),
        Some(150.0),
        Some(1.0e-4),
        None,
        Some(true),
        Some(true),
        Some(6.0),
    );
    assert!(six.proof_test_criterion_met);
}

#[test]
fn the_empty_criteria_are_incomplete() {
    let result = screen(None, None, None, None, None, None, None, None);
    assert!(!result.pressure_basis_complete);
    assert!(result.test_pressure_valid);
    assert!(result.pressure_within_test);
    assert!(!result.frequency_configured);
    assert!(result.frequency_criterion_met);
    assert!(!result.proof_test_configured);
    assert!(result.proof_test_criterion_met);
    assert_eq!(result.target_frequency_per_year, None);
    assert!(!result.acceptable);
}

#[test]
fn the_nonsense_values_are_refused() {
    // A negative pressure.
    assert!(
        s001_secondary_pressure_protection(
            bara(-1.0),
            bara(100.0),
            bara(150.0),
            None,
            None,
            None,
            None,
            None
        )
        .is_err()
    );
    // A negative demand.
    assert!(
        s001_secondary_pressure_protection(
            bara(120.0),
            bara(100.0),
            bara(150.0),
            Some(-1.0e-3),
            None,
            None,
            None,
            None
        )
        .is_err()
    );
    // The class accepts these; the port refuses them.
    for interval in [-6.0, 0.0] {
        assert!(
            s001_secondary_pressure_protection(
                bara(120.0),
                bara(100.0),
                bara(150.0),
                Some(1.0e-4),
                None,
                Some(true),
                Some(true),
                Some(interval)
            )
            .is_err(),
            "interval {interval}"
        );
    }
}
