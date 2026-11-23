//! `pvt.fetkovich_aquifer` at its boundary.
//!
//! The states are the pin's, from `validation/neqsim/captures/pvt_fetkovich_aquifer.tsv`.

use azoth_core::units::{cubic_meters, pascals};
use azoth_pvt::{FetkovichInterval, fetkovich_aquifer};

fn interval(duration: f64, start: f64, end: f64) -> FetkovichInterval {
    FetkovichInterval {
        duration,
        reservoir_pressure_start: start,
        reservoir_pressure_end: end,
    }
}

#[test]
fn the_constant_pressure_identity_and_the_linear_ramp() {
    let constant = fetkovich_aquifer(
        Some(cubic_meters(1.0e6)),
        1.0e-9,
        2.0e-7,
        pascals(3.0e7),
        &[interval(12000.0, 2.5e7, 2.5e7)],
    )
    .unwrap();
    assert_eq!(constant.states.len(), 2);
    assert!((constant.states[1].cumulative_influx / 4546.410233552938 - 1.0).abs() < 1.0e-12);
    assert!((constant.states[1].pressure / 2.5453589766447064e7 - 1.0).abs() < 1.0e-12);

    let ramp = fetkovich_aquifer(
        Some(cubic_meters(1.0e6)),
        1.0e-9,
        2.0e-7,
        pascals(3.0e7),
        &[interval(20000.0, 3.0e7, 1.8e7)],
    )
    .unwrap();
    assert!((ramp.states[1].cumulative_influx / 9054.946916666202 - 1.0).abs() < 1.0e-12);
    assert!((ramp.states[1].pressure / 2.0945053083333794e7 - 1.0).abs() < 1.0e-12);
}

#[test]
fn the_interval_extrema_are_both_retained() {
    let up = fetkovich_aquifer(
        Some(cubic_meters(1.0e6)),
        1.0e-9,
        2.0e-7,
        pascals(3.0e7),
        &[interval(10000.0, 2.5e7, 4.0e7)],
    )
    .unwrap();
    assert!(
        (up.states[1].interval_minimum_cumulative_influx / -4191.69104045766 - 1.0).abs() < 1.0e-10
    );
    assert!(
        (up.states[1].interval_maximum_cumulative_influx / 1168.8078217550703 - 1.0).abs()
            < 1.0e-10
    );

    let down = fetkovich_aquifer(
        Some(cubic_meters(1.0e6)),
        1.0e-9,
        2.0e-7,
        pascals(3.0e7),
        &[interval(10000.0, 3.5e7, 2.0e7)],
    )
    .unwrap();
    assert!(
        (down.states[1].interval_minimum_cumulative_influx / -1168.8078217550703 - 1.0).abs()
            < 1.0e-10
    );
    assert!(
        (down.states[1].interval_maximum_cumulative_influx / 4191.69104045766 - 1.0).abs()
            < 1.0e-10
    );
}

#[test]
fn infinite_storage_is_the_absent_volume_and_has_no_capacity() {
    let result = fetkovich_aquifer(
        None,
        1.0e-9,
        2.0e-7,
        pascals(3.0e7),
        &[interval(1000.0, 2.5e7, 4.0e7)],
    )
    .unwrap();
    assert!(result.capacity.is_none());
    assert!((result.states[1].cumulative_influx + 500.0).abs() < 1.0e-9);
    assert!(result.states[1].pressure == 3.0e7);
}

#[test]
fn a_multi_interval_schedule_keeps_the_signed_influx() {
    let result = fetkovich_aquifer(
        Some(cubic_meters(1.0e6)),
        1.0e-9,
        2.0e-7,
        pascals(3.0e7),
        &[
            interval(100000.0, 3.5e7, 3.5e7),
            interval(100000.0, 3.0e7, 3.0e7),
        ],
    )
    .unwrap();
    assert_eq!(result.states.len(), 3);
    assert!(result.states[1].cumulative_influx < 0.0);
    assert!(result.states[2].interval_influx > 0.0);
    assert!((result.states[2].cumulative_influx / -1.0305768228136003e-5 - 1.0).abs() < 1.0e-6);
}

#[test]
fn the_small_exposure_boundary_is_bracketed() {
    for (duration, expected) in [
        (0.00999999, 7.4997425067498564e-6),
        (0.01000001, 7.499757505749892e-6),
    ] {
        let result = fetkovich_aquifer(
            Some(cubic_meters(1000.0)),
            1.0e-11,
            1.0e-10,
            pascals(3.0e7),
            &[interval(duration, 3.0e7, 1.5e7)],
        )
        .unwrap();
        assert!((result.states[1].cumulative_influx / expected - 1.0).abs() < 1.0e-12);
    }
}

#[test]
fn a_refused_state_is_named() {
    let exhausted = fetkovich_aquifer(
        Some(cubic_meters(1.0)),
        1.0e-6,
        1.0e-5,
        pascals(3.0e7),
        &[interval(100.0, 2.5e7, 2.5e7)],
    )
    .unwrap_err();
    assert!(exhausted.to_string().contains("intervals[0]"));

    let negative = fetkovich_aquifer(
        Some(cubic_meters(-1.0)),
        1.0e-9,
        2.0e-7,
        pascals(3.0e7),
        &[],
    )
    .unwrap_err();
    assert!(negative.to_string().contains("initial_water_volume"));

    let zero_duration = fetkovich_aquifer(
        Some(cubic_meters(1.0e6)),
        1.0e-9,
        2.0e-7,
        pascals(3.0e7),
        &[interval(0.0, 3.0e7, 2.0e7)],
    )
    .unwrap_err();
    assert!(zero_duration.to_string().contains("duration"));

    let unrepresentable = fetkovich_aquifer(
        Some(cubic_meters(f64::MAX)),
        2.0,
        2.0e-7,
        pascals(3.0e7),
        &[],
    )
    .unwrap_err();
    assert!(unrepresentable.to_string().contains("representable"));
}
