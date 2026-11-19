//! `process.vessel_filling` against `validation/neqsim/captures/process_vessel_filling.tsv`.

#![allow(clippy::excessive_precision)]

use azoth_core::units::{
    cubic_meters, kelvins, kilograms, kilograms_per_second, pascals, seconds, square_meters,
    watts_per_square_meter_kelvin,
};
use azoth_process::models::vessel_filling::{VesselFillingResult, vessel_filling};

fn close(actual: f64, expected: f64, what: &str, tolerance: f64) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= tolerance,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn pure(
    supply: f64,
    target: Option<f64>,
    wall_mass: f64,
    wall_area: f64,
    wall_cp: f64,
    htc: f64,
    woodfield: Option<(f64, f64)>,
    liner: Option<(f64, f64)>,
    step: f64,
    span: f64,
) -> VesselFillingResult {
    vessel_filling(
        &["methane".to_string()],
        &[1.0],
        kelvins(288.15),
        pascals(2.0e6),
        cubic_meters(0.05),
        kelvins(288.15),
        pascals(supply),
        kilograms_per_second(0.02),
        target.map(pascals),
        Some(kilograms(wall_mass)),
        Some(square_meters(wall_area)),
        Some(azoth_core::units::joules_per_kilogram_kelvin(wall_cp)),
        Some(watts_per_square_meter_kelvin(htc)),
        woodfield.map(|(nozzle, _)| azoth_core::units::meters(nozzle)),
        woodfield.map(|(_, height)| azoth_core::units::meters(height)),
        liner.map(|(min, _)| kelvins(min)),
        liner.map(|(_, max)| kelvins(max)),
        Some(seconds(step)),
        Some(seconds(span)),
    )
    .expect("the captured state is valid")
}

#[test]
fn the_mixture_state_reads_the_capture() {
    let result = vessel_filling(
        &[
            "methane".to_string(),
            "ethane".to_string(),
            "propane".to_string(),
        ],
        &[0.9, 0.07, 0.03],
        kelvins(288.15),
        pascals(2.0e6),
        cubic_meters(0.05),
        kelvins(288.15),
        pascals(30.0e6),
        kilograms_per_second(0.01),
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
    )
    .expect("the captured state is valid");
    close(
        result.final_pressure.value,
        2_354_539.644_363_285_5,
        "final pressure",
        5.0e-3,
    );
    close(
        result.min_fluid_temperature.value,
        277.820_641_456_461_26,
        "minimum temperature",
        5.0e-3,
    );
    close(
        result.final_mass.value,
        0.990_450_047_010_954_7,
        "final mass",
        5.0e-3,
    );
    assert_eq!(result.recorded_points, 21.0);
    assert_eq!(result.vu_flash_fallback_count, 0.0);
}

#[test]
fn the_short_pure_states_read_the_capture() {
    let no_wall = pure(35.0e6, None, 0.0, 0.0, 470.0, 50.0, None, None, 0.5, 5.0);
    close(
        no_wall.final_pressure.value,
        2_220_644.202_686_956_6,
        "no wall pressure",
        5.0e-3,
    );
    close(
        no_wall.min_fluid_temperature.value,
        283.628_725_973_726_6,
        "no wall min",
        5.0e-3,
    );
    assert_eq!(no_wall.recorded_points, 11.0);

    let wall = pure(35.0e6, None, 50.0, 1.0, 470.0, 50.0, None, None, 0.5, 5.0);
    close(
        wall.final_pressure.value,
        2_224_022.285_306_290_3,
        "wall pressure",
        5.0e-3,
    );
    close(
        wall.min_fluid_temperature.value,
        284.008_807_242_697_5,
        "wall min",
        5.0e-3,
    );
    assert!(wall.liner_limits_met);

    let woodfield = pure(
        35.0e6,
        None,
        50.0,
        1.0,
        470.0,
        50.0,
        Some((0.01, 0.8)),
        None,
        0.5,
        5.0,
    );
    close(
        woodfield.final_pressure.value,
        2_228_517.033_013_947_3,
        "woodfield pressure",
        5.0e-3,
    );
    close(
        woodfield.min_fluid_temperature.value,
        284.514_553_308_989_83,
        "woodfield min",
        5.0e-3,
    );

    let liner = pure(
        35.0e6,
        None,
        50.0,
        1.0,
        470.0,
        50.0,
        None,
        Some((233.15, 287.0)),
        0.5,
        5.0,
    );
    assert!(liner.liner_over_temperature);
    assert!(!liner.liner_under_temperature);
    assert!(!liner.liner_limits_met);
}

#[test]
fn the_long_run_is_evidence_for_the_summary() {
    let no_wall = pure(35.0e6, None, 0.0, 0.0, 470.0, 50.0, None, None, 0.5, 600.0);
    close(
        no_wall.final_pressure.value,
        54_866_348.113_965_84,
        "long no-wall pressure",
        1.0e-3,
    );
    close(
        no_wall.max_fluid_temperature.value,
        359.353_424_121_533_2,
        "long no-wall max",
        1.0e-3,
    );
    close(
        no_wall.final_mass.value,
        12.700_382_722_410_922,
        "long no-wall mass",
        5.0e-3,
    );
    assert_eq!(no_wall.recorded_points, 1201.0);
}

#[test]
fn the_refusals_are_the_class_guards() {
    assert!(
        vessel_filling(
            &["methane".to_string()],
            &[1.0],
            kelvins(288.15),
            pascals(2.0e6),
            cubic_meters(0.0),
            kelvins(288.15),
            pascals(35.0e6),
            kilograms_per_second(0.02),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(seconds(0.5)),
            Some(seconds(5.0)),
        )
        .is_err()
    );
    assert!(
        vessel_filling(
            &["methane".to_string()],
            &[1.0],
            kelvins(288.15),
            pascals(2.0e6),
            cubic_meters(0.05),
            kelvins(288.15),
            pascals(35.0e6),
            kilograms_per_second(0.02),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(kelvins(300.0)),
            Some(kelvins(250.0)),
            Some(seconds(0.5)),
            Some(seconds(5.0)),
        )
        .is_err()
    );
    assert!(
        vessel_filling(
            &["methane".to_string()],
            &[1.0],
            kelvins(288.15),
            pascals(2.0e6),
            cubic_meters(0.05),
            kelvins(288.15),
            pascals(35.0e6),
            kilograms_per_second(0.02),
            None,
            None,
            None,
            None,
            None,
            Some(azoth_core::units::meters(0.01)),
            None,
            None,
            None,
            Some(seconds(0.5)),
            Some(seconds(5.0)),
        )
        .is_err()
    );
}
