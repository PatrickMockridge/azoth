//! `process.finite_rate_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::{FiniteRateReleaseResult, finite_rate_release};

fn heptane(fraction: f64, relaxation: f64, residence: f64) -> FiniteRateReleaseResult {
    finite_rate_release(
        &["methane".to_string(), "n-heptane".to_string()],
        &[fraction, 1.0 - fraction],
        kelvins(300.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        0.02,
        relaxation,
        residence,
        "public-screening:v1",
    )
    .expect("the state runs")
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs()
}

#[test]
fn half_progress_is_at_the_captured_rate() {
    let result = heptane(0.1125, 0.5, 0.25);
    assert_eq!(result.status, "valid");
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 1.810_506_659_228_489_5) < 2.0e-5);
    let throat = &result.stations[1];
    assert!(relative(throat.gas_velocity.unwrap().value, 71.054_434_054_923_97) < 2.0e-5);
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "FINITE_RATE_CONSERVATION")
    );
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "DRIFT_FLUX_CLOSURE")
    );
}

#[test]
fn a_faster_relaxation_moves_toward_equilibrium() {
    let slow = heptane(0.1125, 0.5, 0.25).mass_flow_rate.unwrap().value;
    let fast = heptane(0.1125, 0.1, 0.25).mass_flow_rate.unwrap().value;
    assert!(fast < slow);
    assert!(relative(fast, 1.729_427_209_015_69) < 2.0e-5);
}

#[test]
fn a_near_equilibrium_progress_matches_the_drift_answer() {
    let result = heptane(0.1125, 0.01, 0.5);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 1.717_522_271_728_852_7) < 2.0e-5);
}

#[test]
fn an_all_gas_upstream_is_unsupported() {
    let result = finite_rate_release(
        &["methane".to_string()],
        &[1.0],
        kelvins(300.0),
        pascals(2_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        0.02,
        0.5,
        0.25,
        "public-screening:v1",
    )
    .expect("the state runs");
    assert_eq!(result.status, "unsupported");
    assert_eq!(
        result.diagnostic_codes,
        vec!["FINITE_RATE_REGIME_UNSUPPORTED"]
    );
}

#[test]
fn an_empty_provenance_is_refused() {
    let refused = finite_rate_release(
        &["methane".to_string(), "n-heptane".to_string()],
        &[0.1125, 0.8875],
        kelvins(300.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        0.02,
        0.5,
        0.25,
        " ",
    );
    assert!(refused.is_err());
}
