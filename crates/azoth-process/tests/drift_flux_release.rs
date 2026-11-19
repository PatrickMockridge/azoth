//! `process.drift_flux_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::{DriftFluxReleaseResult, drift_flux_release};

fn heptane(fraction: f64, sigma: f64) -> DriftFluxReleaseResult {
    drift_flux_release(
        &["methane".to_string(), "n-heptane".to_string()],
        &[fraction, 1.0 - fraction],
        kelvins(300.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        sigma,
    )
    .expect("the state runs")
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs()
}

#[test]
fn the_window_is_choked_at_the_captured_rate() {
    let result = heptane(0.1125, 0.02);
    assert_eq!(result.status, "valid");
    assert!(result.choked);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 1.717_522_271_728_852_7) < 2.0e-5);
    let throat = &result.stations[1];
    assert!(relative(throat.gas_velocity.unwrap().value, 71.869_939_795_632_62) < 2.0e-5);
    assert!(relative(throat.liquid_velocity.unwrap().value, 58.569_366_531_081_94) < 2.0e-5);
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "PREDICTIVE_DRIFT_FLUX")
    );
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "DRIFT_FLUX_CLOSURE")
    );
}

#[test]
fn the_upper_window_state_is_used() {
    let result = heptane(0.125, 0.02);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 1.614_618_773_654_917_6) < 2.0e-5);
}

#[test]
fn the_surface_tension_moves_the_rate() {
    let low = heptane(0.1125, 0.02).mass_flow_rate.unwrap().value;
    let high = heptane(0.1125, 0.05).mass_flow_rate.unwrap().value;
    assert!(high > low);
    assert!(relative(high, 1.717_634_551_156_514_5) < 2.0e-5);
}

#[test]
fn the_gas_area_bound_is_unsupported() {
    let result = heptane(0.8, 0.02);
    assert_eq!(result.status, "unsupported");
    assert_eq!(
        result.diagnostic_codes,
        vec!["DRIFT_FLUX_REGIME_UNSUPPORTED"]
    );
}

#[test]
fn a_single_gas_opening_is_unsupported() {
    let result = drift_flux_release(
        &["methane".to_string()],
        &[1.0],
        kelvins(300.0),
        pascals(2_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        0.02,
    )
    .expect("the state runs");
    assert_eq!(result.status, "unsupported");
    assert_eq!(
        result.diagnostic_codes,
        vec!["DRIFT_FLUX_REGIME_UNSUPPORTED"]
    );
}
