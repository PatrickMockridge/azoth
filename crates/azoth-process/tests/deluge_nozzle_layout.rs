//! `process.deluge_nozzle_layout` against
//! `validation/neqsim/captures/process_deluge_nozzle_layout.tsv`.

use azoth_core::units::{bars, meters, square_meters};
use azoth_process::models::deluge_nozzle_layout::{
    DelugeNozzleLayoutResult, GoverningCriterion, deluge_nozzle_layout,
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
fn layout(
    area: f64,
    density: f64,
    k: f64,
    min_pressure: f64,
    spacing: f64,
    clearance: f64,
    available: f64,
    pressure: Option<f64>,
    efficiency: Option<f64>,
) -> DelugeNozzleLayoutResult {
    deluge_nozzle_layout(
        square_meters(area),
        density,
        k,
        bars(min_pressure),
        meters(spacing),
        meters(clearance),
        meters(available),
        pressure.map(bars),
        efficiency,
    )
    .expect("the captured state is valid")
}

#[test]
fn high_velocity_nozzles_are_coverage_governed() {
    let result = layout(510.0, 10.0, 42.9, 3.5, 3.0, 1.0, 1.2, Some(5.0), Some(1.0));
    close(lpm(result.required_flow.value), 5100.0, "the required flow");
    close(
        lpm(result.flow_per_nozzle.value),
        95.927_316_234_740_99,
        "the flow per nozzle",
    );
    close(result.coverage_per_nozzle.value, 9.0, "the coverage square");
    assert_eq!(result.nozzle_count_from_flow, 54.0);
    assert_eq!(result.nozzle_count_from_coverage, 57.0);
    assert_eq!(result.nozzle_count, 57.0);
    assert_eq!(result.governing_criterion, GoverningCriterion::Coverage);
    close(
        result.grid_spacing.value,
        2.991_215_208_080_594,
        "the grid spacing",
    );
    close(
        lpm(result.delivered_flow.value),
        5467.857_025_380_236,
        "the delivered flow",
    );
    close(
        result.delivered_density,
        10.721_288_285_059_288,
        "the delivered density",
    );
    assert!(result.pressure_adequate);
    assert!(result.density_met);
    assert!(result.clearance_adequate);
}

#[test]
fn high_capacity_nozzles_are_coverage_governed() {
    let result = layout(510.0, 10.0, 79.0, 3.5, 3.7, 1.0, 1.2, Some(5.0), Some(1.0));
    assert_eq!(result.nozzle_count, 38.0);
    assert_eq!(result.governing_criterion, GoverningCriterion::Coverage);
    close(
        result.delivered_density,
        13.162_109_938_145_822,
        "the delivered density",
    );
}

#[test]
fn medium_velocity_nozzles_are_flow_governed() {
    let result = layout(510.0, 10.0, 28.0, 1.4, 3.0, 0.6, 1.2, Some(5.0), Some(1.0));
    assert_eq!(result.nozzle_count_from_flow, 82.0);
    assert_eq!(result.nozzle_count_from_coverage, 57.0);
    assert_eq!(result.governing_criterion, GoverningCriterion::Flow);
    close(
        result.delivered_density,
        10.066_690_345_763_758,
        "the delivered density",
    );
}

#[test]
fn a_pressure_below_the_nozzle_minimum_is_flagged() {
    let result = layout(100.0, 10.0, 42.9, 3.5, 3.0, 1.0, 1.2, Some(2.0), None);
    assert!(!result.pressure_adequate);
    assert!(result.density_met);
    assert_eq!(result.nozzle_count, 17.0);
    assert_eq!(result.governing_criterion, GoverningCriterion::Flow);
}

#[test]
fn a_reduced_coverage_efficiency_adds_nozzles() {
    let result = layout(510.0, 10.0, 42.9, 3.5, 3.0, 1.0, 1.2, Some(5.0), Some(0.8));
    close(result.coverage_per_nozzle.value, 7.2, "the coverage square");
    assert_eq!(result.nozzle_count_from_flow, 54.0);
    assert_eq!(result.nozzle_count_from_coverage, 71.0);
    assert_eq!(result.nozzle_count, 71.0);
}

#[test]
fn the_clearance_check_uses_the_nozzle_requirement() {
    let result = layout(100.0, 10.0, 42.9, 3.5, 3.0, 1.0, 0.5, Some(5.0), None);
    assert!(!result.clearance_adequate, "0.5 m against the 1.0 m needed");
    let passing = layout(100.0, 10.0, 42.9, 3.5, 3.0, 1.0, 1.2, Some(5.0), None);
    assert!(passing.clearance_adequate);
}

#[test]
fn the_defaults_are_the_class_ones() {
    let result = layout(510.0, 10.0, 42.9, 3.5, 3.0, 1.0, 1.2, None, None);
    close(
        lpm(result.flow_per_nozzle.value),
        95.927_316_234_740_99,
        "the 5 bar default",
    );
    close(result.coverage_per_nozzle.value, 9.0, "the unit efficiency");
}

#[test]
fn the_range_guards_are_the_class_constructor_and_setters() {
    assert!(
        deluge_nozzle_layout(
            square_meters(0.0),
            10.0,
            42.9,
            bars(3.5),
            meters(3.0),
            meters(1.0),
            meters(1.2),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(510.0),
            0.0,
            42.9,
            bars(3.5),
            meters(3.0),
            meters(1.0),
            meters(1.2),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(510.0),
            10.0,
            0.0,
            bars(3.5),
            meters(3.0),
            meters(1.0),
            meters(1.2),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(510.0),
            10.0,
            42.9,
            bars(0.0),
            meters(3.0),
            meters(1.0),
            meters(1.2),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(510.0),
            10.0,
            42.9,
            bars(3.5),
            meters(0.0),
            meters(1.0),
            meters(1.2),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(510.0),
            10.0,
            42.9,
            bars(3.5),
            meters(3.0),
            meters(-1.0),
            meters(1.2),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(510.0),
            10.0,
            42.9,
            bars(3.5),
            meters(3.0),
            meters(1.0),
            meters(-1.0),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(510.0),
            10.0,
            42.9,
            bars(3.5),
            meters(3.0),
            meters(1.0),
            meters(1.2),
            Some(bars(0.0)),
            None,
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(510.0),
            10.0,
            42.9,
            bars(3.5),
            meters(3.0),
            meters(1.0),
            meters(1.2),
            None,
            Some(0.0),
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(510.0),
            10.0,
            42.9,
            bars(3.5),
            meters(3.0),
            meters(1.0),
            meters(1.2),
            None,
            Some(1.1),
        )
        .is_err()
    );
    assert!(
        deluge_nozzle_layout(
            square_meters(f64::NAN),
            10.0,
            42.9,
            bars(3.5),
            meters(3.0),
            meters(1.0),
            meters(1.2),
            None,
            None,
        )
        .is_err()
    );
}
