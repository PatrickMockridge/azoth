//! `process.real_gas_fanno_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::{RealGasFannoReleaseResult, real_gas_fanno_release};

fn pipe(
    components: &[&str],
    z: &[f64],
    t: f64,
    p: f64,
    back: f64,
    length: f64,
    friction: f64,
) -> RealGasFannoReleaseResult {
    real_gas_fanno_release(
        &components
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>(),
        z,
        kelvins(t),
        pascals(p),
        meters(0.08),
        0.9,
        pascals(back),
        length,
        friction,
    )
    .expect("the state runs")
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs()
}

#[test]
fn the_dilute_case_matches_the_capture() {
    let result = pipe(
        &["nitrogen"],
        &[1.0],
        300.0,
        200_000.0,
        101_325.0,
        12.0,
        0.015,
    );
    assert_eq!(result.status, "valid");
    assert!(!result.choked);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 1.277_278_966_897_692) < 1.0e-3);
    assert!(result.exit_mach.expect("an exit Mach") > 0.0);
    assert!(result.friction_length.expect("a friction length") > 0.0);
    assert_eq!(result.stations.len(), 4);
}

#[test]
fn a_two_phase_state_is_unsupported() {
    let result = pipe(
        &["methane", "n-hexane"],
        &[0.55, 0.45],
        285.0,
        6_000_000.0,
        101_325.0,
        12.0,
        0.015,
    );
    assert_eq!(result.status, "unsupported");
    assert_eq!(
        result.diagnostic_codes,
        vec!["UNSUPPORTED_REAL_GAS_FANNO_REGIME"]
    );
}

#[test]
fn a_zero_path_is_unsupported() {
    let result = pipe(&["nitrogen"], &[1.0], 300.0, 200_000.0, 101_325.0, 0.0, 0.0);
    assert_eq!(result.status, "unsupported");
    assert!(result.stations.is_empty());
}
