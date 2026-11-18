//! `process.piping_fiv_gas` against `validation/neqsim/captures/process_piping_fiv_gas.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{kilograms_per_cubic_meter, meters, meters_per_second};
use azoth_process::models::piping_fiv_gas::{FivLikelihood, PipingFivGasResult, piping_fiv_gas};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn gas(
    density: f64,
    velocity: f64,
    outside_diameter: f64,
    wall_thickness: f64,
    branch_count: f64,
    pulsation_factor: f64,
    support_quality_factor: f64,
) -> PipingFivGasResult {
    piping_fiv_gas(
        kilograms_per_cubic_meter(density),
        meters_per_second(velocity),
        meters(outside_diameter),
        meters(wall_thickness),
        branch_count,
        pulsation_factor,
        support_quality_factor,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_band_boundaries_are_exclusive_upward() {
    let low = gas(1.0, 10.0, 0.2, 0.01, 0.0, 1.0, 1.0);
    close(low.lof, 0.002, "the low score");
    close(low.rho_v2.value, 100.0, "the momentum flux");
    close(low.energy_term, 0.002, "the energy term");
    assert_eq!(low.likelihood, FivLikelihood::Low);

    let medium = gas(150.0, 10.0, 0.2, 0.01, 0.0, 1.0, 1.0);
    close(medium.lof, 0.3, "the 0.3 boundary");
    assert_eq!(medium.likelihood, FivLikelihood::Medium);

    let high = gas(250.0, 10.0, 0.2, 0.01, 0.0, 1.0, 1.0);
    close(high.lof, 0.5, "the 0.5 boundary");
    assert_eq!(high.likelihood, FivLikelihood::High);

    let very_high = gas(500.0, 10.0, 0.2, 0.01, 0.0, 1.0, 1.0);
    close(very_high.lof, 1.0, "the 1.0 boundary");
    assert_eq!(very_high.likelihood, FivLikelihood::VeryHigh);
}

#[test]
fn the_dt_factor_is_floored_at_one() {
    let result = gas(1.0, 10.0, 0.6, 0.01, 0.0, 1.0, 1.0);
    close(result.d_overt, 60.0, "the D/t");
    close(result.d_overt_factor, 1.5, "the D/t factor");
    close(result.lof, 0.003, "the score");
    assert_eq!(result.likelihood, FivLikelihood::Low);
}

#[test]
fn the_branch_factor_is_one_plus_fifteen_percent() {
    let result = gas(1.0, 10.0, 0.2, 0.01, 2.0, 1.0, 1.0);
    close(result.branch_factor, 1.3, "the branch factor");
    close(result.lof, 0.0026000000000000003, "the score");
}

#[test]
fn the_two_clamps_are_published() {
    let clamped = gas(1.0, 10.0, 0.2, 0.01, 0.0, 5.0, 5.0);
    close(clamped.pulsation_factor_applied, 4.0, "the pulsation clamp");
    close(
        clamped.support_quality_factor_applied,
        3.0,
        "the support clamp",
    );
    close(clamped.lof, 0.024, "the score");
}

#[test]
fn the_refusals_are_the_class_validations() {
    // A negative density or velocity.
    assert!(
        piping_fiv_gas(
            kilograms_per_cubic_meter(-1.0),
            meters_per_second(10.0),
            meters(0.2),
            meters(0.01),
            0.0,
            1.0,
            1.0,
        )
        .is_err()
    );
    assert!(
        piping_fiv_gas(
            kilograms_per_cubic_meter(1.0),
            meters_per_second(-1.0),
            meters(0.2),
            meters(0.01),
            0.0,
            1.0,
            1.0,
        )
        .is_err()
    );
    // A non-positive diameter or wall.
    assert!(
        piping_fiv_gas(
            kilograms_per_cubic_meter(1.0),
            meters_per_second(10.0),
            meters(0.0),
            meters(0.01),
            0.0,
            1.0,
            1.0,
        )
        .is_err()
    );
    assert!(
        piping_fiv_gas(
            kilograms_per_cubic_meter(1.0),
            meters_per_second(10.0),
            meters(0.2),
            meters(0.0),
            0.0,
            1.0,
            1.0,
        )
        .is_err()
    );
    // A negative branch count and a fractional one.
    assert!(
        piping_fiv_gas(
            kilograms_per_cubic_meter(1.0),
            meters_per_second(10.0),
            meters(0.2),
            meters(0.01),
            -1.0,
            1.0,
            1.0,
        )
        .is_err()
    );
    assert!(
        piping_fiv_gas(
            kilograms_per_cubic_meter(1.0),
            meters_per_second(10.0),
            meters(0.2),
            meters(0.01),
            1.5,
            1.0,
            1.0,
        )
        .is_err()
    );
}
