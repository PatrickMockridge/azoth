//! `process.depressurization` against
//! `validation/neqsim/captures/process_depressurization.tsv`.

#![allow(clippy::excessive_precision)]

use azoth_core::units::{
    cubic_meters, kelvins, meters, pascals, seconds, square_meters, watts_per_square_meter_kelvin,
};
use azoth_process::models::depressurization::depressurization;

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 2.0e-2,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn close_loose(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 5.0e-2,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn gas() -> Vec<String> {
    ["methane", "ethane", "propane", "n-butane"]
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

fn two_phase() -> Vec<String> {
    [
        "nitrogen",
        "CO2",
        "methane",
        "ethane",
        "propane",
        "n-butane",
        "n-pentane",
        "n-heptane",
        "n-octane",
    ]
    .iter()
    .map(|name| (*name).to_string())
    .collect()
}

#[test]
fn the_cold_and_fire_states_read_the_capture() {
    let cold = depressurization(
        &gas(),
        &[0.85, 0.08, 0.04, 0.03],
        kelvins(313.15),
        pascals(9.0e6),
        cubic_meters(30.0),
        meters(0.03),
        0.85,
        pascals(2.0e5),
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
        Some(seconds(1.0)),
        Some(seconds(60.0)),
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        cold.min_fluid_temperature.value,
        286.339_213_126_009_67,
        "cold minimum temperature",
    );
    close(cold.max_pressure.value, 9.0e6, "cold maximum pressure");
    assert_eq!(cold.recorded_points, 61.0);
    assert!(!cold.vapour_withdrawal_used);
    assert!(cold.pressure_monotonic_non_increasing);

    let wall = depressurization(
        &gas(),
        &[0.85, 0.08, 0.04, 0.03],
        kelvins(313.15),
        pascals(9.0e6),
        cubic_meters(30.0),
        meters(0.03),
        0.85,
        pascals(2.0e5),
        Some(5.0e5),
        Some(true),
        None,
        None,
        None,
        None,
        None,
        Some(azoth_core::units::kilograms(20000.0)),
        Some(square_meters(50.0)),
        Some(azoth_core::units::joules_per_kilogram_kelvin(470.0)),
        Some(watts_per_square_meter_kelvin(60.0)),
        Some(seconds(1.0)),
        Some(seconds(60.0)),
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        wall.max_wall_temperature.value,
        316.059_292_981_987_37,
        "wall maximum temperature",
    );
    assert!(wall.fire_heat_routed_to_wall);

    let preset = depressurization(
        &gas(),
        &[0.85, 0.08, 0.04, 0.03],
        kelvins(313.15),
        pascals(9.0e6),
        cubic_meters(30.0),
        meters(0.03),
        0.85,
        pascals(2.0e5),
        None,
        None,
        Some(square_meters(25.0)),
        Some("jet_fire_peak"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(seconds(1.0)),
        Some(seconds(10.0)),
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        preset.max_pressure.value,
        9_101_749.364_148_969,
        "preset peak",
    );
    close(
        preset.max_fluid_temperature.value,
        322.860_988_040_285_7,
        "preset temperature",
    );
    assert!(!preset.pressure_monotonic_non_increasing);
}

#[test]
fn the_withdrawal_modes_read_the_capture() {
    let z = [0.005, 0.015, 0.45, 0.06, 0.04, 0.03, 0.03, 0.12, 0.25];
    for (mode, vapour, expected) in [
        ("vapour", true, 331.026_079_803_372_7),
        ("bulk", false, 332.196_661_415_450_6),
        ("auto", true, 331.026_079_803_372_7),
    ] {
        let result = depressurization(
            &two_phase(),
            &z,
            kelvins(333.15),
            pascals(6.0e6),
            cubic_meters(40.0),
            meters(0.04),
            0.85,
            pascals(6.0e5),
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
            Some(seconds(2.0)),
            Some(seconds(20.0)),
            None,
            Some(mode),
        )
        .expect("the captured state is valid");
        close(
            result.min_fluid_temperature.value,
            expected,
            &format!("{mode} minimum temperature"),
        );
        assert_eq!(result.vapour_withdrawal_used, vapour, "{mode}");
        assert_eq!(result.recorded_points, 11.0);
    }
}

#[test]
fn the_pure_feeds_read_the_capture() {
    let methane = depressurization(
        &["methane".to_string()],
        &[1.0],
        kelvins(313.15),
        pascals(9.0e6),
        cubic_meters(1.0),
        meters(0.01),
        0.62,
        pascals(101_325.0),
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
        Some(seconds(1.0)),
        Some(seconds(20.0)),
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        methane.min_fluid_temperature.value,
        285.554_391_087_445_1,
        "methane minimum temperature",
    );

    let propane = depressurization(
        &["propane".to_string()],
        &[1.0],
        kelvins(300.0),
        pascals(9.9e5),
        cubic_meters(1.0),
        meters(0.01),
        0.62,
        pascals(101_325.0),
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
        Some(seconds(1.0)),
        Some(seconds(20.0)),
        None,
        None,
    )
    .expect("the captured state is valid");
    assert!(propane.vapour_withdrawal_used);
    close(
        propane.min_fluid_temperature.value,
        298.342_935_903_766_54,
        "propane minimum temperature",
    );
}

#[test]
fn the_api_run_is_evidence_for_the_criteria() {
    let result = depressurization(
        &gas(),
        &[0.85, 0.08, 0.04, 0.03],
        kelvins(313.15),
        pascals(9.0e6),
        cubic_meters(30.0),
        meters(0.03),
        0.85,
        pascals(2.0e5),
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
        Some(seconds(1.0)),
        Some(seconds(900.0)),
        None,
        None,
    )
    .expect("the captured state is valid");
    close_loose(
        result.time_to_half_pressure.expect("the crossing").value,
        118.592_955_029_248_91,
        "half-pressure time",
    );
    close_loose(
        result.time_to_7_barg.expect("the crossing").value,
        482.607_764_034_718_85,
        "7 barg time",
    );
    close_loose(
        result.min_fluid_temperature.value,
        175.129_279_141_646_1,
        "minimum temperature",
    );
    assert!(result.half_pressure_criterion_met);
    assert!(result.seven_barg_criterion_met);
    assert!(result.vapour_withdrawal_used);
}

#[test]
fn the_refusals_are_the_class_guards() {
    let base = |volume: f64, coefficient: f64, preset: Option<&str>, explicit: bool| {
        depressurization(
            &gas(),
            &[0.85, 0.08, 0.04, 0.03],
            kelvins(313.15),
            pascals(9.0e6),
            cubic_meters(volume),
            meters(0.03),
            coefficient,
            pascals(2.0e5),
            None,
            None,
            if explicit {
                Some(square_meters(25.0))
            } else {
                None
            },
            preset,
            if explicit { Some(0.9) } else { None },
            None,
            None,
            None,
            None,
            None,
            None,
            Some(seconds(1.0)),
            Some(seconds(5.0)),
            None,
            None,
        )
    };
    assert!(base(0.0, 0.85, None, false).is_err());
    assert!(base(30.0, 1.5, None, false).is_err());
    assert!(base(30.0, 0.85, Some("jet_fire_peak"), true).is_err());
    assert!(base(30.0, 0.85, Some("unknown_preset"), false).is_err());
}
