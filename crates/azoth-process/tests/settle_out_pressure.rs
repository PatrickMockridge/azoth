//! `process.settle_out_pressure` against
//! `validation/neqsim/captures/process_settle_out_pressure.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{cubic_meters, kelvins, pascals};
use azoth_process::models::settle_out_pressure::{
    SettleOutCompartment, SettleOutPressureResult, SettleOutVerdict, settle_out_pressure,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn two() -> Vec<SettleOutCompartment> {
    vec![
        SettleOutCompartment {
            name: "suction".to_string(),
            volume: cubic_meters(5.0),
            pressure: pascals(1.0e6),
            temperature: kelvins(300.0),
        },
        SettleOutCompartment {
            name: "discharge".to_string(),
            volume: cubic_meters(2.0),
            pressure: pascals(2.0e6),
            temperature: kelvins(320.0),
        },
    ]
}

fn raw(
    rows: &[SettleOutCompartment],
    temperature: Option<f64>,
    rating: Option<f64>,
) -> azoth_core::Result<SettleOutPressureResult> {
    settle_out_pressure(rows, temperature.map(kelvins), rating.map(pascals))
}

#[test]
fn two_compartments_settle_inside_the_rating() {
    let result = raw(&two(), None, Some(1.5e6)).expect("the captured state is valid");
    close(
        result.settle_out_pressure.value,
        1.285_714_285_714_285_8e6,
        "the settle-out pressure",
    );
    close(
        result.settle_out_temperature.value,
        308.571_428_571_428_56,
        "the settle-out temperature",
    );
    assert_eq!(result.settle_out_z_factor, 1.0);
    close(
        result.total_moles.value,
        3507.943_688_810_829_4,
        "the total moles",
    );
    close(result.total_volume.value, 7.0, "the total volume");
    close(
        result.max_compartment_pressure.value,
        2.0e6,
        "the maximum pressure",
    );
    close(
        result.min_compartment_pressure.value,
        1.0e6,
        "the minimum pressure",
    );
    close(
        result.margin_to_rating.expect("a rating").value,
        2.142_857_142_857_142_3e5,
        "the margin",
    );
    assert!(!result.exceeds_rating);
    assert_eq!(result.verdict, SettleOutVerdict::WithinRating);
    assert_eq!(result.compartments.len(), 2);
    close(
        result.compartments[0].moles.value,
        2004.539_250_749_045_3,
        "the suction inventory",
    );
    close(
        result.compartments[1].moles.value,
        1503.404_438_061_784,
        "the discharge inventory",
    );
}

#[test]
fn a_settle_pressure_above_the_rating_is_exceeded() {
    let result = raw(&two(), None, Some(1.0e6)).expect("the captured state is valid");
    close(
        result.margin_to_rating.expect("a rating").value,
        -2.857_142_857_142_857_7e5,
        "the margin",
    );
    assert!(result.exceeds_rating);
    assert_eq!(result.verdict, SettleOutVerdict::ExceedsRating);
}

#[test]
fn equality_with_the_rating_is_within_it() {
    let result =
        raw(&two(), None, Some(1.285_714_285_714_285_8e6)).expect("the captured state is valid");
    assert!(result.margin_to_rating.expect("a rating").value.abs() < 1.0e-6);
    assert!(!result.exceeds_rating);
    assert_eq!(result.verdict, SettleOutVerdict::WithinRating);
}

#[test]
fn a_third_compartment_and_a_stated_temperature_are_read() {
    let mut rows = two();
    rows.push(SettleOutCompartment {
        name: "header".to_string(),
        volume: cubic_meters(3.0),
        pressure: pascals(5.0e5),
        temperature: kelvins(290.0),
    });
    let three = raw(&rows, None, Some(1.5e6)).expect("the captured state is valid");
    close(
        three.settle_out_pressure.value,
        1.049_999_999_999_999_8e6,
        "the three-volume pressure",
    );
    close(
        three.settle_out_temperature.value,
        305.774_058_577_405_8,
        "the three-volume temperature",
    );
    close(
        three.total_moles.value,
        4130.042_076_974_326,
        "the total moles",
    );

    let stated = raw(&two(), Some(310.0), Some(1.5e6)).expect("the captured state is valid");
    close(
        stated.settle_out_pressure.value,
        1.291_666_666_666_666_8e6,
        "the stated-temperature pressure",
    );
    close(
        stated.settle_out_temperature.value,
        310.0,
        "the stated temperature",
    );
}

#[test]
fn no_rating_publishes_no_verdict() {
    let result = raw(&two(), None, None).expect("the captured state is valid");
    assert!(result.margin_to_rating.is_none());
    assert!(!result.exceeds_rating);
    assert_eq!(result.verdict, SettleOutVerdict::NoRating);
}

#[test]
fn the_refusals_are_the_class_validations() {
    // Fewer than two compartments.
    assert!(raw(&two()[..1], None, None).is_err());
    // A zero-volume compartment.
    let mut rows = two();
    rows[0].volume = cubic_meters(0.0);
    assert!(raw(&rows, None, None).is_err());
    // A zero-pressure compartment.
    let mut rows = two();
    rows[0].pressure = pascals(0.0);
    assert!(raw(&rows, None, None).is_err());
    // A zero-temperature compartment.
    let mut rows = two();
    rows[0].temperature = kelvins(0.0);
    assert!(raw(&rows, None, None).is_err());
    // A non-positive rating.
    assert!(raw(&two(), None, Some(0.0)).is_err());
    // A non-positive stated temperature.
    assert!(raw(&two(), Some(0.0), None).is_err());
    // A non-finite pressure.
    let mut rows = two();
    rows[0].pressure = pascals(f64::NAN);
    assert!(raw(&rows, None, None).is_err());
}
