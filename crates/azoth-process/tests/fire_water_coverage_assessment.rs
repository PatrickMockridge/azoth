//! `process.fire_water_coverage_assessment` against
//! `validation/neqsim/captures/process_fire_water_coverage_assessment.tsv`.

use azoth_core::units::{bars, liters_per_minute, square_meters};
use azoth_process::models::fire_water_coverage_assessment::{
    FireWaterCoverageAssessmentResult, SupplyVerdict, fire_water_coverage_assessment,
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
fn assess(
    area: f64,
    required: f64,
    spare: f64,
    worst: f64,
    covered: Option<f64>,
    existing: Option<f64>,
    margin: Option<f64>,
) -> FireWaterCoverageAssessmentResult {
    fire_water_coverage_assessment(
        square_meters(area),
        liters_per_minute(required),
        liters_per_minute(spare),
        bars(worst),
        covered.map(square_meters),
        existing.map(liters_per_minute),
        margin.map(bars),
    )
    .expect("the captured state is valid")
}

#[test]
fn no_flow_and_no_pressure_margin_routes_to_a_dedicated_supply() {
    let result = assess(
        510.0,
        5100.0,
        -1000.0,
        -0.9,
        Some(0.0),
        Some(0.0),
        Some(0.5),
    );
    close(result.coverage_gap.value, 510.0, "the coverage gap");
    assert_eq!(result.coverage_fraction, 0.0);
    close(lpm(result.flow_deficit.value), 5100.0, "the flow deficit");
    assert!(!result.flow_feasible);
    assert!(!result.pressure_feasible);
    assert_eq!(result.verdict, SupplyVerdict::DedicatedSupplyRequired);
}

#[test]
fn a_pressure_deficit_alone_is_distinguished() {
    let result = assess(
        510.0,
        5100.0,
        20000.0,
        -0.9,
        Some(0.0),
        Some(0.0),
        Some(0.5),
    );
    assert!(result.flow_feasible);
    assert!(!result.pressure_feasible);
    assert_eq!(result.verdict, SupplyVerdict::SupplyPressureDeficit);
}

#[test]
fn a_flow_deficit_alone_is_distinguished() {
    let result = assess(510.0, 5100.0, 100.0, 2.5, Some(0.0), Some(0.0), Some(0.5));
    assert!(!result.flow_feasible);
    assert!(result.pressure_feasible);
    assert_eq!(result.verdict, SupplyVerdict::SupplyFlowDeficit);
}

#[test]
fn a_fully_covered_area_is_compliant() {
    let result = assess(
        510.0,
        5100.0,
        1000.0,
        2.0,
        Some(510.0),
        Some(5200.0),
        Some(0.5),
    );
    assert_eq!(result.coverage_gap.value, 0.0);
    assert_eq!(result.coverage_fraction, 1.0);
    assert_eq!(result.flow_deficit.value, 0.0);
    assert!(result.flow_feasible);
    assert!(result.pressure_feasible);
    assert_eq!(result.verdict, SupplyVerdict::Compliant);
}

#[test]
fn a_coverage_gap_with_supply_in_hand() {
    let result = assess(
        510.0,
        5100.0,
        1000.0,
        2.0,
        Some(0.0),
        Some(5100.0),
        Some(0.5),
    );
    close(result.coverage_gap.value, 510.0, "the coverage gap");
    assert_eq!(result.coverage_fraction, 0.0);
    assert_eq!(result.flow_deficit.value, 0.0);
    assert_eq!(result.verdict, SupplyVerdict::CoverageGapSupplyAdequate);
}

#[test]
fn the_defaults_are_the_class_ones() {
    let result = assess(510.0, 5100.0, 1000.0, 2.0, None, None, None);
    close(result.coverage_gap.value, 510.0, "no coverage by default");
    close(
        lpm(result.flow_deficit.value),
        5100.0,
        "no delivery by default",
    );
    assert!(result.pressure_feasible, "a zero required margin");
}

#[test]
fn a_negative_margin_is_the_class_unguarded_state() {
    let result = assess(510.0, 5100.0, 20000.0, -0.9, None, None, Some(0.0));
    assert!(!result.pressure_feasible);
    assert_eq!(result.verdict, SupplyVerdict::SupplyPressureDeficit);
}

#[test]
fn the_range_guards_are_the_class_constructor_and_setters() {
    assert!(
        fire_water_coverage_assessment(
            square_meters(0.0),
            liters_per_minute(5100.0),
            liters_per_minute(1000.0),
            bars(2.0),
            None,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        fire_water_coverage_assessment(
            square_meters(510.0),
            liters_per_minute(0.0),
            liters_per_minute(1000.0),
            bars(2.0),
            None,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        fire_water_coverage_assessment(
            square_meters(510.0),
            liters_per_minute(5100.0),
            liters_per_minute(1000.0),
            bars(2.0),
            Some(square_meters(-1.0)),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        fire_water_coverage_assessment(
            square_meters(510.0),
            liters_per_minute(5100.0),
            liters_per_minute(1000.0),
            bars(2.0),
            None,
            Some(liters_per_minute(-1.0)),
            None,
        )
        .is_err()
    );
    assert!(
        fire_water_coverage_assessment(
            square_meters(510.0),
            liters_per_minute(5100.0),
            liters_per_minute(1000.0),
            bars(2.0),
            None,
            None,
            Some(bars(-1.0)),
        )
        .is_err()
    );
    assert!(
        fire_water_coverage_assessment(
            square_meters(510.0),
            liters_per_minute(5100.0),
            liters_per_minute(f64::NAN),
            bars(2.0),
            None,
            None,
            None,
        )
        .is_err()
    );
}
