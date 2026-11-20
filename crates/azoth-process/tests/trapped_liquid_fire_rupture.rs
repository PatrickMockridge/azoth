//! `process.trapped_liquid_fire_rupture` against
//! `validation/neqsim/captures/process_trapped_liquid_fire_rupture.tsv`.

use azoth_core::units::{cubic_meters, kelvins, kilograms, meters, pascals, seconds};
use azoth_process::models::trapped_liquid_fire_rupture::{
    FailureMode, TrappedLiquidFireRuptureResult, trapped_liquid_fire_rupture,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn run(
    pressure_bar: f64,
    liquid_mass: f64,
    total_mass: f64,
    flange: i64,
    flux: f64,
    max_time: f64,
    relief: bool,
    passive: Option<f64>,
    vapor: bool,
) -> TrappedLiquidFireRuptureResult {
    let components = vec![
        "methane".to_string(),
        "ethane".to_string(),
        "n-butane".to_string(),
    ];
    trapped_liquid_fire_rupture(
        &components,
        &[0.5, 0.3, 0.2],
        pascals(pressure_bar * 1.0e5),
        kelvins(300.0),
        kilograms(liquid_mass),
        kilograms(total_mass),
        cubic_meters(10.0),
        meters(0.1),
        meters(0.01),
        meters(10.0),
        None,
        None,
        None,
        Some("fixed_heat_flux"),
        None,
        Some(flux),
        None,
        None,
        None,
        None,
        passive,
        None,
        None,
        None,
        Some(flange),
        relief.then(|| pascals(6.0e6)),
        Some(relief),
        Some(seconds(max_time)),
        Some(seconds(2.0)),
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
        Some(vapor),
    )
    .expect("the state evaluates")
}

#[test]
fn the_no_failure_state() {
    let result = run(
        50.0,
        2246.059130169441,
        2556.756576013232,
        0,
        10000.0,
        20.0,
        false,
        None,
        false,
    );
    assert_eq!(result.limiting_failure_mode, FailureMode::None);
    assert!(!result.rupture_predicted);
    close(result.trajectory.len() as f64, 11.0, "points");
    close(
        result.final_pressure.value,
        5012760.306260937,
        "final pressure",
    );
    close(
        result.final_liquid_temperature.value,
        300.01519084078683,
        "final liquid",
    );
    close(
        result.final_outer_wall_temperature.value,
        305.30679875134905,
        "final outer wall",
    );
}

#[test]
fn the_flange_failure_state() {
    let result = run(
        10.0,
        4000.0,
        4061.735103001965,
        150,
        500000.0,
        600.0,
        false,
        None,
        true,
    );
    assert_eq!(result.limiting_failure_mode, FailureMode::FlangeFailure);
    assert!(result.rupture_predicted);
    close(
        result
            .time_to_flange_failure
            .expect("the flange fails")
            .value,
        22.0,
        "flange time",
    );
    close(
        result.minimum_failure_time.expect("a failure time").value,
        22.0,
        "minimum",
    );
    close(
        result.final_pressure.value,
        1465832.210042704,
        "final pressure",
    );
    close(
        result.final_outer_wall_temperature.value,
        586.9348854255791,
        "final outer wall",
    );
}

#[test]
fn the_pipe_rupture_state() {
    let result = run(
        50.0,
        2246.059130169441,
        2556.756576013232,
        0,
        500000.0,
        600.0,
        false,
        None,
        true,
    );
    assert_eq!(result.limiting_failure_mode, FailureMode::PipeRupture);
    close(
        result
            .time_to_pipe_rupture
            .expect("the pipe ruptures")
            .value,
        62.0,
        "pipe time",
    );
    close(
        result.minimum_failure_time.expect("a failure time").value,
        62.0,
        "minimum",
    );
    close(
        result.final_pressure.value,
        10333528.154974353,
        "final pressure",
    );
    close(
        result.final_liquid_temperature.value,
        306.3494382797314,
        "final liquid",
    );
    close(
        result.final_outer_wall_temperature.value,
        893.3867869221738,
        "final outer wall",
    );
}

#[test]
fn the_relief_and_pfp_states() {
    let relief = run(
        50.0,
        2246.059130169441,
        2556.756576013232,
        0,
        500000.0,
        600.0,
        true,
        None,
        true,
    );
    close(
        relief.time_to_relief_set.expect("the relief sets").value,
        26.0,
        "relief time",
    );
    close(
        relief
            .time_to_pipe_rupture
            .expect("the pipe ruptures")
            .value,
        74.0,
        "pipe time",
    );
    close(relief.final_pressure.value, 6.0e6, "relief pressure");
    let pfp = run(
        50.0,
        2246.059130169441,
        2556.756576013232,
        0,
        500000.0,
        600.0,
        false,
        Some(0.3),
        true,
    );
    close(
        pfp.time_to_pipe_rupture.expect("the pipe ruptures").value,
        520.0,
        "pfp pipe time",
    );
    close(pfp.final_pressure.value, 41687831.037177796, "pfp pressure");
    close(
        pfp.final_outer_wall_temperature.value,
        642.2964759288351,
        "pfp outer wall",
    );
}

#[test]
fn a_vapor_pocket_is_recorded_and_the_refusals_are_typed() {
    let result = run(
        50.0,
        2246.059130169441,
        2556.756576013232,
        0,
        1000.0,
        20.0,
        false,
        None,
        true,
    );
    close(
        result
            .time_to_vapor_pocket
            .expect("the vapor pocket appears")
            .value,
        0.0,
        "vapor time",
    );
    let components = vec!["methane".to_string()];
    assert!(
        trapped_liquid_fire_rupture(
            &components,
            &[1.0],
            pascals(5.0e6),
            kelvins(300.0),
            kilograms(1.0),
            kilograms(1.0),
            cubic_meters(1.0),
            meters(0.1),
            meters(0.01),
            meters(10.0),
            None,
            None,
            None,
            Some("fixed_heat_flux"),
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
            Some(seconds(10.0)),
            Some(seconds(2.0)),
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
        .is_err()
    );
}
