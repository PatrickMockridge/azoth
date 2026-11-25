//! `process.dynamic_psv_sizing` against
//! `validation/neqsim/captures/process_dynamic_psv_sizing.tsv`.

#![allow(clippy::excessive_precision)]

use azoth_core::units::{cubic_meters, kelvins, pascals, watts};
use azoth_process::models::dynamic_psv_sizing::{DynamicPsvSizingResult, dynamic_psv_sizing};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-6,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn sizing(initial_p: f64, volume: f64, fire: f64, set: f64) -> DynamicPsvSizingResult {
    dynamic_psv_sizing(
        &[
            "methane".to_string(),
            "ethane".to_string(),
            "propane".to_string(),
        ],
        &[0.85, 0.1, 0.05],
        kelvins(350.0),
        pascals(initial_p),
        cubic_meters(volume),
        watts(fire),
        pascals(set),
        0.21,
        pascals(1.0e5),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_steady_and_dynamic_areas_are_the_captured_states() {
    let fifty = sizing(5.0e6, 5.0, 5.0e5, 5.0e6);
    close(
        fifty.steady_required_area.value,
        1.3532429190157695e-4,
        "steady 50 bara",
    );
    close(
        fifty.dynamic_required_area.value,
        5.44015269712034e-5,
        "dynamic 50 bara",
    );
    close(
        fifty.oversizing_ratio.expect("a positive dynamic area"),
        2.487509072552481,
        "oversizing 50 bara",
    );
    close(
        fifty.allowable_pressure.value,
        6_028_721.75,
        "allowable 50 bara",
    );
    close(
        fifty.peak_dynamic_pressure.value,
        6_028_721.746_112_543,
        "peak 50 bara",
    );
    close(
        fifty.steady_relief_rate.value,
        1.428_571_428_571_428_6,
        "rate 50 bara",
    );

    let low = sizing(1.1e6, 1.0, 1.5e5, 1.1e6);
    close(
        low.steady_required_area.value,
        1.9739820536756255e-4,
        "steady 11 bara",
    );
    assert!(
        low.dynamic_required_area.value.is_nan(),
        "the peak cannot be held at 11 bara, so the class answers no dynamic area"
    );
    assert!(low.oversizing_ratio.is_none() || low.oversizing_ratio.unwrap().is_nan());
    close(
        low.allowable_pressure.value,
        1_309_721.75,
        "allowable 11 bara",
    );
    close(
        low.peak_dynamic_pressure.value,
        1_116_540.736_216_803_5,
        "peak 11 bara",
    );
    close(
        low.steady_relief_rate.value,
        0.428_571_428_571_428_55,
        "rate 11 bara",
    );
}

#[test]
fn the_refusals_are_the_class_guards() {
    assert!(
        dynamic_psv_sizing(
            &["methane".to_string()],
            &[1.0],
            kelvins(350.0),
            pascals(5.0e6),
            cubic_meters(-1.0),
            watts(5.0e5),
            pascals(5.0e6),
            0.21,
            pascals(1.0e5),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        dynamic_psv_sizing(
            &["methane".to_string()],
            &[1.0],
            kelvins(350.0),
            pascals(5.0e6),
            cubic_meters(5.0),
            watts(5.0e5),
            pascals(5.0e6),
            0.21,
            pascals(1.0e5),
            Some(1.5),
            None,
            None,
            None,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        dynamic_psv_sizing(
            &["methane".to_string(), "ethane".to_string()],
            &[1.0],
            kelvins(350.0),
            pascals(5.0e6),
            cubic_meters(5.0),
            watts(5.0e5),
            pascals(5.0e6),
            0.21,
            pascals(1.0e5),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .is_err()
    );
}
