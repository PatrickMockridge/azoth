//! `process.fire_monitor_coverage` against
//! `validation/neqsim/captures/process_fire_monitor_coverage.tsv`.

use azoth_core::units::{liters_per_minute, meters, meters_per_second, square_meters};
use azoth_process::models::fire_monitor_coverage::{
    FireMonitorCoverageResult, fire_monitor_coverage,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn lpm(value: f64) -> f64 {
    value * 60_000.0
}

#[allow(clippy::too_many_arguments)]
fn monitor(
    area: f64,
    density: f64,
    count: Option<f64>,
    flow: Option<f64>,
    throw_m: Option<f64>,
    sweep: Option<f64>,
    wind: Option<f64>,
    fall: Option<f64>,
    terminal: Option<f64>,
    dimension: Option<f64>,
    obstructed: Option<bool>,
) -> FireMonitorCoverageResult {
    fire_monitor_coverage(
        square_meters(area),
        density,
        count,
        flow.map(liters_per_minute),
        throw_m.map(meters),
        sweep,
        wind.map(meters_per_second),
        fall.map(meters),
        terminal.map(meters_per_second),
        dimension.map(meters),
        obstructed,
    )
    .expect("the captured state is valid")
}

#[test]
fn a_large_deck_monitor_is_not_suitable() {
    let result = monitor(
        510.0,
        10.0,
        Some(2.0),
        Some(2400.0),
        Some(40.0),
        Some(90.0),
        Some(0.0),
        Some(10.0),
        Some(8.0),
        None,
        Some(false),
    );
    close(lpm(result.total_flow.value), 4800.0, "the total flow");
    close(
        result.swept_area.value,
        2_513.274_122_871_834_6,
        "the swept area",
    );
    close(
        result.nominal_density,
        9.411_764_705_882_353,
        "the nominal density",
    );
    assert_eq!(result.drift_displacement.value, 0.0);
    assert_eq!(result.wind_coverage_fraction, 1.0);
    assert!(!result.meets_density_still_air);
    assert!(!result.meets_density_in_wind);
    close(
        result
            .flow_needed_for_compliance
            .expect("a finite fraction")
            .value,
        5100.0 / 60_000.0,
        "the compliance flow",
    );
    assert_eq!(
        result.verdict,
        "NOT SUITABLE - required density not achieved even in still air"
    );
}

#[test]
fn still_air_coverage_is_full() {
    let result = monitor(
        3.0,
        2000.0,
        Some(1.0),
        Some(2400.0),
        Some(40.0),
        Some(180.0),
        Some(0.0),
        Some(12.0),
        Some(8.0),
        None,
        Some(false),
    );
    assert_eq!(result.wind_coverage_fraction, 1.0);
    close(result.nominal_density, 800.0, "the nominal density");
    close(
        lpm(result
            .flow_needed_for_compliance
            .expect("a finite fraction")
            .value),
        6000.0,
        "the compliance flow",
    );
}

#[test]
fn wind_drift_removes_the_coverage() {
    let result = monitor(
        3.0,
        2000.0,
        Some(1.0),
        Some(2400.0),
        Some(40.0),
        Some(180.0),
        Some(15.0),
        Some(12.0),
        Some(8.0),
        None,
        Some(false),
    );
    close(result.drift_displacement.value, 22.5, "the drift");
    assert_eq!(result.wind_coverage_fraction, 0.0);
    assert_eq!(result.effective_density, 0.0);
    assert!(result.flow_needed_for_compliance.is_none());
}

#[test]
fn an_obstructed_line_of_sight_disqualifies_a_monitor() {
    let result = monitor(
        100.0,
        10.0,
        Some(4.0),
        Some(4000.0),
        Some(40.0),
        Some(360.0),
        Some(0.0),
        Some(10.0),
        Some(8.0),
        None,
        Some(true),
    );
    assert!(result.meets_density_still_air);
    assert!(result.meets_density_in_wind);
    assert_eq!(
        result.verdict,
        "NOT SUITABLE - line of sight to part of the target is obstructed; a monitor cannot \
         wet surfaces it cannot see"
    );
}

#[test]
fn a_wind_limited_density_is_marginal() {
    let result = monitor(
        100.0,
        10.0,
        Some(1.0),
        Some(2400.0),
        Some(40.0),
        Some(90.0),
        Some(5.0),
        Some(10.0),
        Some(8.0),
        None,
        Some(false),
    );
    assert!(result.meets_density_still_air);
    assert!(!result.meets_density_in_wind);
    close(result.drift_displacement.value, 6.25, "the drift");
    close(result.wind_coverage_fraction, 0.375, "the fraction");
    close(result.effective_density, 9.0, "the effective density");
    assert_eq!(
        result.verdict,
        "MARGINAL - density achieved in still air only; wind drift takes it below the \
         requirement"
    );
}

#[test]
fn a_small_drift_leaves_the_density_feasible() {
    let result = monitor(
        100.0,
        10.0,
        Some(2.0),
        Some(2400.0),
        Some(40.0),
        Some(90.0),
        Some(1.0),
        Some(10.0),
        Some(8.0),
        None,
        Some(false),
    );
    assert!(result.meets_density_still_air);
    assert!(result.meets_density_in_wind);
    close(result.effective_density, 42.0, "the effective density");
    assert_eq!(
        result.verdict,
        "FEASIBLE ON DENSITY - carry forward to a trajectory and shadowing study"
    );
}

#[test]
fn the_flow_needed_scales_with_the_coverage_fraction() {
    let result = monitor(
        510.0,
        10.0,
        Some(1.0),
        Some(2400.0),
        Some(40.0),
        Some(90.0),
        Some(5.0),
        Some(10.0),
        Some(8.0),
        None,
        Some(false),
    );
    close(
        result.wind_coverage_fraction,
        0.723_245_348_268_720_2,
        "the fraction",
    );
    close(
        result
            .flow_needed_for_compliance
            .expect("a finite fraction")
            .value,
        7_051.548_982_939_475 / 60_000.0,
        "the compliance flow",
    );
}

#[test]
fn drift_can_remove_every_bit_of_the_coverage() {
    let result = monitor(
        510.0,
        10.0,
        Some(1.0),
        Some(2400.0),
        Some(40.0),
        Some(90.0),
        Some(100.0),
        Some(10.0),
        Some(8.0),
        None,
        Some(false),
    );
    close(result.drift_displacement.value, 125.0, "the drift");
    assert_eq!(result.wind_coverage_fraction, 0.0);
    assert!(result.flow_needed_for_compliance.is_none());
}

#[test]
fn a_stated_dimension_replaces_the_area_root() {
    let result = monitor(
        510.0,
        10.0,
        Some(1.0),
        Some(2400.0),
        Some(40.0),
        Some(90.0),
        Some(5.0),
        Some(10.0),
        Some(8.0),
        Some(50.0),
        Some(false),
    );
    close(result.wind_coverage_fraction, 0.875, "the fraction");
    close(
        lpm(result
            .flow_needed_for_compliance
            .expect("a finite fraction")
            .value),
        5_828.571_428_571_428,
        "the compliance flow",
    );
}

#[test]
fn the_defaults_are_the_class_ones() {
    let result = monitor(
        100.0, 10.0, None, None, None, None, None, None, None, None, None,
    );
    close(lpm(result.total_flow.value), 2400.0, "the default flow");
    close(
        result.swept_area.value,
        1_256.637_061_435_917_3,
        "the swept area",
    );
    close(result.drift_displacement.value, 0.0, "no wind");
    assert!(result.meets_density_still_air);
}

#[test]
fn a_fractional_monitor_count_is_refused() {
    let result = fire_monitor_coverage(
        square_meters(100.0),
        10.0,
        Some(1.5),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert!(result.is_err(), "the class's setMonitors takes an int");
}

#[test]
fn the_range_guards_are_the_class_constructor_and_setters() {
    let base = || {
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
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
    };
    assert!(base().is_ok(), "the default state is valid");
    assert!(
        fire_monitor_coverage(
            square_meters(0.0),
            10.0,
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
    assert!(
        fire_monitor_coverage(
            square_meters(100.0),
            0.0,
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
    assert!(
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
            Some(0.0),
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
    assert!(
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
            None,
            Some(liters_per_minute(0.0)),
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
    assert!(
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
            None,
            None,
            Some(meters(0.0)),
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
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
            None,
            None,
            None,
            Some(0.0),
            None,
            None,
            None,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
            None,
            None,
            None,
            Some(361.0),
            None,
            None,
            None,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
            None,
            None,
            None,
            None,
            Some(meters_per_second(-1.0)),
            None,
            None,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
            None,
            None,
            None,
            None,
            None,
            Some(meters(0.0)),
            None,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(meters_per_second(0.0)),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        fire_monitor_coverage(
            square_meters(100.0),
            10.0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(meters(0.0)),
            None,
        )
        .is_err()
    );
}
