//! `process.vacuum_collapse` against
//! `validation/neqsim/captures/process_vacuum_collapse.tsv`.

use azoth_core::units::{cubic_meters, kelvins, kilograms_per_mole, pascals};
use azoth_process::models::vacuum_collapse::{
    VacuumCollapseResult, VacuumVerdict, vacuum_collapse,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-6,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn raw(
    components: &[String],
    fractions: &[f64],
    volume: f64,
    initial_pressure: f64,
    initial_temperature: f64,
    cold_end: f64,
    atmospheric: Option<f64>,
    rating: Option<f64>,
    setpoint: Option<f64>,
    makeup_mass: Option<f64>,
    steps: Option<f64>,
) -> azoth_core::Result<VacuumCollapseResult> {
    vacuum_collapse(
        components,
        fractions,
        cubic_meters(volume),
        pascals(initial_pressure),
        kelvins(initial_temperature),
        kelvins(cold_end),
        atmospheric.map(pascals),
        rating.map(pascals),
        setpoint.map(pascals),
        makeup_mass.map(kilograms_per_mole),
        steps,
    )
}

fn mixed() -> (Vec<String>, Vec<f64>) {
    (
        vec!["methane".to_string(), "propane".to_string()],
        vec![0.2, 0.8],
    )
}

#[test]
fn a_non_condensing_cool_develops_no_vacuum() {
    let components = vec!["methane".to_string()];
    let result = raw(
        &components,
        &[1.0],
        10.0,
        5.0e5,
        300.0,
        250.0,
        None,
        Some(1.0e5),
        None,
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        result.final_pressure.value,
        414_496.255_250_451_2,
        "the final pressure",
    );
    close(
        result.minimum_pressure.value,
        414_496.255_250_451_2,
        "the minimum pressure",
    );
    close(
        result.minimum_pressure_temperature.value,
        250.0,
        "the minimum temperature",
    );
    assert_eq!(result.vacuum_depth.value, 0.0);
    close(
        result.margin_to_rating.value,
        314_496.255_250_451_2,
        "the margin",
    );
    assert!(!result.vacuum_present);
    assert!(result.within_rating);
    assert_eq!(result.makeup_gas_moles.value, 0.0);
    assert_eq!(result.makeup_gas_mass.value, 0.0);
    assert_eq!(result.verdict, VacuumVerdict::NoVacuum);
}

#[test]
fn a_condensing_sweep_pulls_a_vacuum_within_the_rating() {
    let (components, fractions) = mixed();
    let result = raw(
        &components,
        &fractions,
        10.0,
        5.0e5,
        300.0,
        180.0,
        None,
        Some(2.0e4),
        None,
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        result.final_pressure.value,
        65_232.373_822_237_59,
        "the final pressure",
    );
    close(
        result.minimum_pressure.value,
        65_232.373_822_237_59,
        "the minimum pressure",
    );
    close(
        result.vacuum_depth.value,
        36_092.626_177_762_41,
        "the depth",
    );
    close(
        result.margin_to_rating.value,
        45_232.373_822_237_59,
        "the margin",
    );
    assert!(result.vacuum_present);
    assert!(result.within_rating);
    close(
        result.makeup_gas_moles.value,
        241.163_619_453_124_1,
        "the make-up moles",
    );
    close(
        result.makeup_gas_mass.value,
        6.755_812_937_188_147,
        "the make-up mass",
    );
    assert_eq!(result.verdict, VacuumVerdict::VacuumWithinRating);
}

#[test]
fn the_rating_and_the_makeup_setpoint_are_read() {
    let (components, fractions) = mixed();
    let exceeds = raw(
        &components,
        &fractions,
        10.0,
        5.0e5,
        300.0,
        180.0,
        None,
        Some(8.0e4),
        None,
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        exceeds.margin_to_rating.value,
        -14_767.626_177_762_416,
        "the negative margin",
    );
    assert!(!exceeds.within_rating);
    assert_eq!(exceeds.verdict, VacuumVerdict::VacuumExceedsRating);

    let no_rating = raw(
        &components,
        &fractions,
        10.0,
        5.0e5,
        300.0,
        180.0,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        no_rating.margin_to_rating.value,
        65_232.373_822_237_59,
        "the full-vacuum margin",
    );
    assert!(no_rating.within_rating);

    let makeup = raw(
        &components,
        &fractions,
        10.0,
        5.0e5,
        300.0,
        180.0,
        None,
        Some(2.0e4),
        Some(1.0e5),
        Some(0.028),
        None,
    )
    .expect("the captured state is valid");
    close(
        makeup.makeup_gas_moles.value,
        232.310_237_762_315_84,
        "the setpoint moles",
    );
    close(
        makeup.makeup_gas_mass.value,
        6.504_686_657_344_844,
        "the setpoint mass",
    );
}

#[test]
fn the_refusals_are_the_class_validations() {
    let (components, fractions) = mixed();
    // A cold end at the initial temperature.
    assert!(
        raw(
            &components,
            &fractions,
            10.0,
            5.0e5,
            300.0,
            300.0,
            None,
            None,
            None,
            None,
            None
        )
        .is_err()
    );
    // A non-positive volume.
    assert!(
        raw(
            &components,
            &fractions,
            0.0,
            5.0e5,
            300.0,
            180.0,
            None,
            None,
            None,
            None,
            None
        )
        .is_err()
    );
    // A non-integer step count.
    assert!(
        raw(
            &components,
            &fractions,
            10.0,
            5.0e5,
            300.0,
            180.0,
            None,
            None,
            None,
            None,
            Some(1.5)
        )
        .is_err()
    );
    // A negative rating.
    assert!(
        raw(
            &components,
            &fractions,
            10.0,
            5.0e5,
            300.0,
            180.0,
            None,
            Some(-1.0),
            None,
            None,
            None
        )
        .is_err()
    );
    // A non-finite initial pressure.
    assert!(
        raw(
            &components,
            &fractions,
            10.0,
            f64::NAN,
            300.0,
            180.0,
            None,
            None,
            None,
            None,
            None
        )
        .is_err()
    );
}
