//! `pvt.van_everdingen_hurst_aquifer` at its boundary.
//!
//! The states are the pin's, from `validation/neqsim/captures/pvt_van_everdingen_hurst.tsv`;
//! the refusals are the class's own domain checks.

use azoth_core::units::{meters, pascal_seconds, seconds, square_meters};
use azoth_pvt::van_everdingen_hurst_aquifer;

fn close(actual: f64, expected: f64) -> bool {
    (actual / expected - 1.0).abs() < 1.0e-12
}

#[test]
fn the_infinite_acting_pressure_and_derivative_are_the_captures() {
    let row = van_everdingen_hurst_aquifer(
        "dimensionless_pressure",
        Some(0.005),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(close(
        row.dimensionless_pressure.unwrap(),
        0.0773837496680494
    ));

    let derivative = van_everdingen_hurst_aquifer(
        "dimensionless_pressure_derivative",
        Some(0.005),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(close(
        derivative.dimensionless_pressure_derivative.unwrap(),
        7.507012527254403
    ));
}

#[test]
fn the_late_time_and_bounded_branches_are_the_captures() {
    let late = van_everdingen_hurst_aquifer(
        "dimensionless_pressure",
        Some(1.0e5),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(close(
        late.dimensionless_pressure.unwrap(),
        6.161035384162979
    ));

    let bounded = van_everdingen_hurst_aquifer(
        "dimensionless_pressure",
        Some(100.0),
        None,
        Some(5.0),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(close(
        bounded.dimensionless_pressure.unwrap(),
        9.192771245767434
    ));
}

#[test]
fn the_dimensionless_time_and_constant_are_si() {
    let time = van_everdingen_hurst_aquifer(
        "dimensionless_time",
        None,
        None,
        None,
        Some(square_meters(1.0e-13)),
        Some(seconds(8.64e7)),
        Some(0.2),
        Some(pascal_seconds(5.0e-4)),
        Some(1.0e-9),
        Some(meters(3000.0)),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(close(time.dimensionless_time.unwrap(), 9.599999999999998));

    let constant = van_everdingen_hurst_aquifer(
        "aquifer_constant",
        None,
        None,
        None,
        None,
        None,
        Some(0.2),
        None,
        Some(1.0e-9),
        Some(meters(3000.0)),
        Some(meters(30.0)),
        Some(180.0),
        None,
        None,
    )
    .unwrap();
    assert!(close(
        constant.aquifer_constant.unwrap(),
        0.16964600329384885
    ));
}

#[test]
fn the_carter_tracy_history_is_the_capture() {
    let row = van_everdingen_hurst_aquifer(
        "carter_tracy",
        None,
        Some(&[0.0, 10.0, 20.0, 40.0, 80.0, 160.0]),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(0.16964600329384885),
        Some(&[0.0, 5.0e5, 1.2e6, 2.2e6, 3.5e6, 5.0e6]),
    )
    .unwrap();
    let influx = row.cumulative_influx.unwrap();
    let expected = [
        0.0,
        513800.1920958346,
        1622029.7705783644,
        5083532.268602057,
        14575989.592201814,
        38319577.38347466,
    ];
    for (actual, expected) in influx.iter().zip(expected) {
        if expected == 0.0 {
            assert_eq!(*actual, 0.0);
        } else {
            assert!(close(*actual, expected));
        }
    }
}

#[test]
fn the_influence_table_is_the_capture() {
    let row = van_everdingen_hurst_aquifer(
        "influence_table",
        None,
        Some(&[1.0, 10.0, 100.0, 1000.0]),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let pressures = row.dimensionless_pressures.unwrap();
    let expected = [
        0.8021451666033006,
        1.6508947048253173,
        2.722894443143703,
        3.860590595578624,
    ];
    for (actual, expected) in pressures.iter().zip(expected) {
        assert!(close(*actual, expected));
    }
}

#[test]
fn a_refused_state_is_named() {
    let negative = van_everdingen_hurst_aquifer(
        "dimensionless_pressure",
        Some(-1.0),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(negative.to_string().contains("dimensionless_time"));

    let nan_radius = van_everdingen_hurst_aquifer(
        "dimensionless_pressure",
        Some(1.0),
        None,
        Some(f64::NAN),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(nan_radius.to_string().contains("NaN"));

    let non_increasing = van_everdingen_hurst_aquifer(
        "carter_tracy",
        None,
        Some(&[0.0, 0.0]),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(1.0),
        Some(&[0.0, 1.0]),
    )
    .unwrap_err();
    assert!(non_increasing.to_string().contains("strictly increasing"));

    let missing = van_everdingen_hurst_aquifer(
        "dimensionless_time",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(missing.to_string().contains("permeability"));

    let unknown = van_everdingen_hurst_aquifer(
        "not_a_calculation",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(unknown.to_string().contains("not_a_calculation"));
}
