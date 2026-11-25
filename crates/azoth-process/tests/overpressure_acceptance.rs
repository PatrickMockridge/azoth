//! `process.overpressure_acceptance` against
//! `validation/neqsim/captures/process_overpressure_acceptance.tsv`.

use azoth_core::units::pascals;
use azoth_process::models::overpressure_acceptance::{
    OverpressureAcceptanceResult, overpressure_acceptance,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn accept(peak: f64, mawp: f64, cause: &str, multiple: bool) -> OverpressureAcceptanceResult {
    overpressure_acceptance(pascals(peak), pascals(mawp), cause, multiple)
        .expect("the captured acceptance state is valid")
}

#[test]
fn the_fire_state_reproduces_the_capture() {
    let fire = accept(1_050_000.0, 1_000_000.0, "fire", false);
    close(fire.accumulation_factor, 1.21, "the fire factor");
    close(fire.allowable_pressure.value, 1_188_721.75, "the allowable");
    close(fire.margin.value, 138_721.750_000_000_03, "the margin");
    assert!(fire.accepted, "10.5 bara is below 12.1");

    let ignored = accept(1_200_000.0, 1_000_000.0, "fire", true);
    close(
        ignored.accumulation_factor,
        1.21,
        "a fire ignores the device count",
    );
}

#[test]
fn the_non_fire_branches_reproduce_the_capture() {
    let single = accept(1_100_000.0, 1_000_000.0, "blocked_outlet", false);
    close(single.accumulation_factor, 1.10, "the single factor");
    close(
        single.allowable_pressure.value,
        1_089_867.5,
        "the allowable",
    );
    close(single.margin.value, -10_132.5, "the margin");
    assert!(
        !single.accepted,
        "the gauge basis puts 11.0 bara above the 10.9 bara allowable"
    );

    let multiple = accept(1_150_000.0, 1_000_000.0, "blocked_outlet", true);
    close(multiple.accumulation_factor, 1.16, "the multiple factor");
    close(
        multiple.allowable_pressure.value,
        1_143_788.0,
        "the multiple allowable",
    );
}

#[test]
fn the_exceeded_states_are_rejected() {
    let fire = accept(1_220_000.0, 1_000_000.0, "fire", false);
    close(
        fire.margin.value,
        -31_278.249_999_999_91,
        "the negative margin",
    );
    assert!(!fire.accepted, "12.2 bara is above 12.1");

    let single = accept(1_110_000.0, 1_000_000.0, "blocked_outlet", false);
    assert!(!single.accepted, "11.1 bara is above 11.0");
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    assert!(
        overpressure_acceptance(
            pascals(1_000_000.0),
            pascals(1_000_000.0),
            "not_a_cause",
            false
        )
        .is_err(),
        "a cause the class does not name"
    );
    assert!(
        overpressure_acceptance(pascals(0.0), pascals(1_000_000.0), "fire", false).is_err(),
        "a zero peak pressure"
    );
    assert!(
        overpressure_acceptance(pascals(1_000_000.0), pascals(-1.0), "fire", false).is_err(),
        "a negative MAWP"
    );
}
