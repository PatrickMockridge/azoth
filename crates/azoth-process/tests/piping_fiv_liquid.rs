//! `process.piping_fiv_liquid` against `validation/neqsim/captures/process_piping_fiv_liquid.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{meters, meters_per_second};
use azoth_process::models::piping_fiv_gas::FivLikelihood;
use azoth_process::models::piping_fiv_liquid::{PipingFivLiquidResult, piping_fiv_liquid};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn liquid(
    velocity: f64,
    outside_diameter: f64,
    wall_thickness: f64,
    branch_count: f64,
    support_quality_factor: f64,
) -> PipingFivLiquidResult {
    piping_fiv_liquid(
        meters_per_second(velocity),
        meters(outside_diameter),
        meters(wall_thickness),
        branch_count,
        support_quality_factor,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_band_boundaries_are_exclusive_upward() {
    let low = liquid(1.0, 0.2, 0.01, 0.0, 1.0);
    close(low.lof, 0.16666666666666666, "the low score");
    close(low.energy_term, 0.16666666666666666, "the energy term");
    assert_eq!(low.likelihood, FivLikelihood::Low);

    let medium = liquid(1.8, 0.2, 0.01, 0.0, 1.0);
    close(medium.lof, 0.3, "the 0.3 boundary");
    assert_eq!(medium.likelihood, FivLikelihood::Medium);

    let high = liquid(3.0, 0.2, 0.01, 0.0, 1.0);
    close(high.lof, 0.5, "the 0.5 boundary");
    assert_eq!(high.likelihood, FivLikelihood::High);

    let very_high = liquid(6.0, 0.2, 0.01, 0.0, 1.0);
    close(very_high.lof, 1.0, "the 1.0 boundary");
    assert_eq!(very_high.likelihood, FivLikelihood::VeryHigh);
}

#[test]
fn the_branch_factor_is_one_plus_ten_percent() {
    let result = liquid(1.0, 0.2, 0.01, 2.0, 1.0);
    close(result.branch_factor, 1.2, "the branch factor");
    close(result.lof, 0.19999999999999998, "the score");
}

#[test]
fn the_support_clamp_is_published() {
    let result = liquid(1.0, 0.2, 0.01, 0.0, 5.0);
    close(
        result.support_quality_factor_applied,
        3.0,
        "the support clamp",
    );
    close(result.lof, 0.5, "the score");
}

#[test]
fn the_refusals_are_the_class_validations() {
    // A negative velocity.
    assert!(
        piping_fiv_liquid(meters_per_second(-1.0), meters(0.2), meters(0.01), 0.0, 1.0).is_err()
    );
    // A non-positive diameter or wall.
    assert!(
        piping_fiv_liquid(meters_per_second(1.0), meters(0.0), meters(0.01), 0.0, 1.0).is_err()
    );
    assert!(piping_fiv_liquid(meters_per_second(1.0), meters(0.2), meters(0.0), 0.0, 1.0).is_err());
    // A negative branch count and a fractional one.
    assert!(
        piping_fiv_liquid(meters_per_second(1.0), meters(0.2), meters(0.01), -1.0, 1.0).is_err()
    );
    assert!(
        piping_fiv_liquid(meters_per_second(1.0), meters(0.2), meters(0.01), 1.5, 1.0).is_err()
    );
}
