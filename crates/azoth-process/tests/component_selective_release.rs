//! `process.component_selective_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::{
    ComponentSelectiveReleaseResult, component_selective_release, finite_rate_release,
};

fn heptane(times: &[f64]) -> ComponentSelectiveReleaseResult {
    component_selective_release(
        &["methane".to_string(), "n-heptane".to_string()],
        &[0.1125, 0.8875],
        kelvins(300.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        0.02,
        times,
        0.25,
        "public-component-screening:v1",
    )
    .expect("the state runs")
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs()
}

#[test]
fn methane_faster_than_heptane_is_at_the_captured_rate() {
    let result = heptane(&[0.1, 1.0]);
    assert_eq!(result.status, "valid");
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 1.732_951_879_438_95) < 2.0e-5);
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "COMPONENT_SELECTIVE_PHASE_TRANSFER")
    );
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "COMPONENT_TRANSFER_CONSERVATION")
    );
}

#[test]
fn uniform_times_recover_the_uniform_model() {
    let component = heptane(&[0.5, 0.5]);
    let uniform = finite_rate_release(
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
        "uniform-limit:v1",
    )
    .expect("the uniform model runs");
    let component_rate = component.mass_flow_rate.expect("a component rate").value;
    let uniform_rate = uniform.mass_flow_rate.expect("a uniform rate").value;
    assert!(relative(component_rate, uniform_rate) < 1.0e-12);
}

#[test]
fn the_times_order_moves_the_rate() {
    let methane_fast = heptane(&[0.1, 1.0]).mass_flow_rate.unwrap().value;
    let heptane_fast = heptane(&[1.0, 0.1]).mass_flow_rate.unwrap().value;
    assert!(heptane_fast > methane_fast);
    assert!(relative(heptane_fast, 1.835_128_256_471_335_8) < 2.0e-5);
}

#[test]
fn an_incomplete_basis_is_unsupported() {
    let result = heptane(&[0.1]);
    assert_eq!(result.status, "unsupported");
    assert_eq!(
        result.diagnostic_codes,
        vec!["COMPONENT_TRANSFER_REGIME_UNSUPPORTED"]
    );
    assert!(result.stations.is_empty());
}

#[test]
fn a_non_positive_time_is_refused() {
    let refused = component_selective_release(
        &["methane".to_string(), "n-heptane".to_string()],
        &[0.1125, 0.8875],
        kelvins(300.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        0.02,
        &[0.0, 1.0],
        0.25,
        "source",
    );
    assert!(refused.is_err());
}
