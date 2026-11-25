//! `process.leak_blowdown` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals, seconds};
use azoth_process::{LeakBlowdownResult, leak_blowdown};

#[allow(clippy::too_many_arguments)] // one argument per input the helper states
fn run(
    components: &[&str],
    z: &[f64],
    t: f64,
    p: f64,
    duration: f64,
    volume: f64,
    diameter: f64,
    coefficient: f64,
    back: f64,
    step: f64,
) -> LeakBlowdownResult {
    let components: Vec<String> = components.iter().map(|name| (*name).to_string()).collect();
    leak_blowdown(
        &components,
        z,
        kelvins(t),
        pascals(p),
        seconds(duration),
        Some(azoth_core::units::cubic_meters(volume)),
        Some(meters(diameter)),
        Some(coefficient),
        Some(pascals(back)),
        Some(seconds(step)),
        Some("Test"),
    )
    .expect("the state runs")
}

#[test]
fn the_first_point_is_the_class_s_scalar_oracle() {
    let result = run(
        &["methane"],
        &[1.0],
        300.0,
        5_000_000.0,
        10.0,
        1.0,
        0.01,
        0.62,
        101_325.0,
        1.0,
    );
    assert_eq!(result.status, "valid");
    assert_eq!(result.recorded_points, 11);
    let first = &result.points[0];
    let error =
        (first.mass_flow_rate.value - 0.430_510_155_343_228_2).abs() / 0.430_510_155_343_228_2;
    assert!(error <= 1.0e-6, "the first rate differs by {error:.3e}");
    assert_eq!(first.time.value, 0.0);
    assert_eq!(first.temperature.value, 300.0);
    assert_eq!(first.pressure.value, 5_000_000.0);
    assert_eq!(
        result.peak_mass_flow_rate.expect("a peak").value,
        first.mass_flow_rate.value
    );
}

#[test]
fn the_substeps_land_on_the_captured_second_state() {
    let result = run(
        &["methane"],
        &[1.0],
        300.0,
        5_000_000.0,
        2.0,
        1.0,
        0.01,
        0.62,
        101_325.0,
        1.0,
    );
    // The captured one-second state: the VU flash at `U - removed h`, 1% substeps.
    let second = &result.points[1];
    assert!(
        (second.pressure.value - 4_918_856.095_330_249_5).abs() / 4_918_856.095_330_249_5 <= 1.0e-6
    );
    assert!(
        (second.temperature.value - 298.759_028_771_403_9).abs() / 298.759_028_771_403_9 <= 1.0e-6
    );
    assert!(
        (second.mass_flow_rate.value - 0.424_401_558_884_396_35).abs() / 0.424_401_558_884_396_35
            <= 1.0e-6
    );
    assert!(second.pressure.value < result.points[0].pressure.value);
}

#[test]
fn a_long_horizon_lands_on_the_captured_end_state() {
    let result = run(
        &["methane"],
        &[1.0],
        300.0,
        5_000_000.0,
        60.0,
        1.0,
        0.01,
        0.62,
        101_325.0,
        1.0,
    );
    assert_eq!(result.recorded_points, 61);
    let total = result.total_mass_released.expect("a total").value;
    assert!((total - 17.708_274_360_596_057).abs() / 17.708_274_360_596_057 <= 1.0e-6);
    let last = &result.points[60];
    assert!((last.pressure.value - 1_997_215.737_462_847).abs() / 1_997_215.737_462_847 <= 1.0e-6);
    assert!(
        (last.temperature.value - 236.326_329_588_963_4).abs() / 236.326_329_588_963_4 <= 1.0e-6
    );
    assert!(last.mass_flow_rate.value > 0.0);
}

#[test]
fn a_hundred_second_sample_stops_where_the_capture_does() {
    let result = run(
        &["methane"],
        &[1.0],
        300.0,
        5_000_000.0,
        100.0,
        1.0,
        0.01,
        0.62,
        101_325.0,
        100.0,
    );
    assert_eq!(result.number_of_points, 2);
    assert_eq!(result.recorded_points, 2);
    let total = result.total_mass_released.expect("a total").value;
    assert!((total - 23.850_934_475_836_42).abs() / 23.850_934_475_836_42 <= 1.0e-6);
    assert!(
        (result.points[1].temperature.value - 203.727_033_879_957_9).abs() / 203.727_033_879_957_9
            <= 1.0e-6
    );
}

#[test]
fn a_liquid_initial_state_is_refused() {
    let result = run(
        &["propane"],
        &[1.0],
        300.0,
        1_100_000.0,
        10.0,
        0.01,
        0.01,
        0.62,
        101_325.0,
        1.0,
    );
    assert_eq!(result.status, "invalid");
    assert_eq!(result.recorded_points, 0);
    assert_eq!(result.diagnostic_codes, vec!["PHASE_BOUNDARY".to_string()]);
}

#[test]
fn a_two_phase_initial_state_is_refused() {
    let result = run(
        &["methane", "ethane"],
        &[0.5, 0.5],
        200.0,
        1_000_000.0,
        1.0,
        1.0,
        0.01,
        0.62,
        100_000.0,
        1.0,
    );
    assert_eq!(result.status, "invalid");
    assert_eq!(result.diagnostic_codes, vec!["PHASE_BOUNDARY".to_string()]);
}

#[test]
fn no_forward_flow_records_a_zero_rate_and_momentum() {
    let result = run(
        &["methane"],
        &[1.0],
        300.0,
        500_000.0,
        3.0,
        1.0,
        0.01,
        0.62,
        600_000.0,
        1.0,
    );
    assert_eq!(result.total_mass_released.expect("a total").value, 0.0);
    assert_eq!(result.points[0].jet_momentum.value, 0.0);
    assert_eq!(result.points[0].jet_velocity.value, 0.0);
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "NO_FORWARD_FLOW")
    );
}

#[test]
fn a_bad_composition_or_step_is_refused() {
    let components = vec!["methane".to_string()];
    let error = leak_blowdown(
        &components,
        &[0.5, 0.5],
        kelvins(300.0),
        pascals(5_000_000.0),
        seconds(1.0),
        Some(azoth_core::units::cubic_meters(1.0)),
        Some(meters(0.01)),
        Some(0.62),
        Some(pascals(101_325.0)),
        Some(seconds(1.0)),
        None,
    )
    .expect_err("two fractions for one component");
    assert_eq!(error.field(), Some("initial_z"));

    let error = leak_blowdown(
        &components,
        &[1.0],
        kelvins(300.0),
        pascals(5_000_000.0),
        seconds(1.0),
        Some(azoth_core::units::cubic_meters(1.0)),
        Some(meters(0.01)),
        Some(0.62),
        Some(pascals(101_325.0)),
        Some(seconds(0.0)),
        None,
    )
    .expect_err("a zero step");
    assert_eq!(error.field(), Some("time_step"));
}
