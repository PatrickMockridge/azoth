//! `process.ranz_marshall_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::{RanzMarshallReleaseResult, ranz_marshall_release};

fn correlation(
    morphology: &str,
    velocity: f64,
    diffusivities: &[f64],
) -> RanzMarshallReleaseResult {
    ranz_marshall_release(
        &["methane".to_string(), "n-heptane".to_string()],
        &[0.1125, 0.8875],
        kelvins(300.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        0.02,
        0.25,
        morphology,
        0.001,
        800.0,
        0.001,
        velocity,
        diffusivities,
        "public-correlation:v1",
    )
    .expect("the state runs")
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs()
}

#[test]
fn bubbles_in_range_are_at_the_captured_rate() {
    let result = correlation("gas_bubbles", 0.01, &[1.0e-8, 2.0e-8]);
    assert_eq!(result.status, "valid");
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 1.851_642_930_106_992) < 2.0e-5);
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "RANZ_MARSHALL_COMPONENT_TRANSFER")
    );
}

#[test]
fn the_stagnant_limit_is_resolved() {
    let result = correlation("gas_bubbles", 0.0, &[1.0e-8, 2.0e-8]);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 1.872_257_013_164_362) < 2.0e-5);
}

#[test]
fn the_applicability_precedes_the_morphology() {
    let droplets = correlation("liquid_droplets", 0.01, &[1.0e-8, 2.0e-8]);
    assert_eq!(droplets.status, "unsupported");
    assert_eq!(
        droplets.diagnostic_codes,
        vec!["LIQUID_DROPLET_HYDRODYNAMICS_UNSUPPORTED"]
    );
    let out_of_range = correlation("liquid_droplets", 0.25, &[1.0e-8, 2.0e-8]);
    assert_eq!(out_of_range.status, "unsupported");
    assert_eq!(
        out_of_range.diagnostic_codes,
        vec!["RANZ_MARSHALL_RANGE_UNSUPPORTED"]
    );
}

#[test]
fn a_schmidt_bound_is_unsupported() {
    let result = correlation("gas_bubbles", 0.01, &[5.0e-9, 2.0e-8]);
    assert_eq!(result.status, "unsupported");
    assert_eq!(
        result.diagnostic_codes,
        vec!["RANZ_MARSHALL_RANGE_UNSUPPORTED"]
    );
}

#[test]
fn an_incomplete_diffusivity_basis_is_invalid() {
    let result = correlation("gas_bubbles", 0.01, &[1.0e-8]);
    assert_eq!(result.status, "invalid");
    assert_eq!(
        result.diagnostic_codes,
        vec!["RANZ_MARSHALL_COMPONENT_BASIS_INVALID"]
    );
}
