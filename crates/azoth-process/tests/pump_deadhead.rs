//! `process.pump_deadhead` against `validation/neqsim/captures/process_pump_deadhead.tsv`.

use azoth_core::units::{
    joules_per_kilogram_kelvin, kelvin_intervals, kilograms_per_cubic_meter, pascals,
};
use azoth_process::models::pump_deadhead::{DeadheadVerdict, PumpDeadheadResult, pump_deadhead};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn raw(
    ps: f64,
    pn: f64,
    ratio: Option<f64>,
    rating: Option<f64>,
    density: Option<f64>,
    heat: Option<f64>,
    efficiency: Option<f64>,
    allowable: Option<f64>,
) -> azoth_core::Result<PumpDeadheadResult> {
    pump_deadhead(
        pascals(ps),
        pascals(pn),
        ratio,
        rating.map(pascals),
        density.map(kilograms_per_cubic_meter),
        heat.map(joules_per_kilogram_kelvin),
        efficiency,
        allowable.map(kelvin_intervals),
    )
}

#[allow(clippy::too_many_arguments)]
fn pump(
    ps: f64,
    pn: f64,
    ratio: Option<f64>,
    rating: Option<f64>,
    density: Option<f64>,
    heat: Option<f64>,
    efficiency: Option<f64>,
    allowable: Option<f64>,
) -> PumpDeadheadResult {
    raw(ps, pn, ratio, rating, density, heat, efficiency, allowable)
        .expect("the captured state is valid")
}

#[test]
fn the_deadhead_pressure_stays_within_the_rating() {
    let result = pump(
        2.0e5,
        1.0e6,
        None,
        Some(1.5e6),
        Some(998.0),
        Some(4180.0),
        None,
        None,
    );
    close(
        result.rated_differential_pressure.value,
        8.0e5,
        "the rated differential",
    );
    close(
        result.deadhead_pressure.value,
        1.16e6,
        "the deadhead pressure",
    );
    close(
        result.pressure_margin.expect("a rating").value,
        3.4e5,
        "the margin",
    );
    assert!(!result.pressure_rating_exceeded);
    close(
        result.shutoff_head.expect("a density").value,
        98.088_934_314_510_14,
        "the head",
    );
    close(
        result
            .recirculation_temperature_rise
            .expect("a density")
            .value,
        0.536_959_085_635_385_6,
        "the recirculation rise",
    );
    assert!(!result.temp_rise_exceeded);
    assert_eq!(result.verdict, DeadheadVerdict::WithinRating);
}

#[test]
fn a_deadhead_above_the_rating_is_exceeded() {
    let result = pump(
        2.0e5,
        1.0e6,
        None,
        Some(1.1e6),
        Some(998.0),
        Some(4180.0),
        None,
        None,
    );
    close(
        result.pressure_margin.expect("a rating").value,
        -60_000.0,
        "the margin",
    );
    assert!(result.pressure_rating_exceeded);
    assert_eq!(result.verdict, DeadheadVerdict::ExceedsRating);
}

#[test]
fn equality_with_the_rating_is_within_it() {
    let result = pump(
        2.0e5,
        1.0e6,
        None,
        Some(1.16e6),
        Some(998.0),
        Some(4180.0),
        None,
        None,
    );
    assert_eq!(result.pressure_margin.expect("a rating").value, 0.0);
    assert!(!result.pressure_rating_exceeded);
    assert_eq!(result.verdict, DeadheadVerdict::WithinRating);
}

#[test]
fn no_rating_publishes_no_rating_verdict() {
    let result = pump(
        2.0e5,
        1.0e6,
        None,
        None,
        Some(998.0),
        Some(4180.0),
        None,
        None,
    );
    assert!(result.pressure_margin.is_none());
    assert!(!result.pressure_rating_exceeded);
    assert_eq!(result.verdict, DeadheadVerdict::NoRating);
}

#[test]
fn the_temperature_rise_test_is_strict() {
    let result = pump(
        2.0e5,
        1.0e6,
        None,
        Some(1.5e6),
        Some(998.0),
        Some(4180.0),
        None,
        Some(0.5),
    );
    assert!(result.temp_rise_exceeded);
    assert_eq!(result.verdict, DeadheadVerdict::WithinRating);
}

#[test]
fn an_absent_density_leaves_the_head_and_rise_absent() {
    let result = pump(2.0e5, 1.0e6, None, Some(1.5e6), None, None, None, None);
    assert!(result.shutoff_head.is_none());
    assert!(result.recirculation_temperature_rise.is_none());
    assert!(!result.temp_rise_exceeded);
    assert_eq!(result.verdict, DeadheadVerdict::WithinRating);
}

#[test]
fn a_half_efficiency_halves_the_rise_against_the_default() {
    let result = pump(
        2.0e5,
        1.0e6,
        None,
        Some(1.5e6),
        Some(998.0),
        Some(4180.0),
        Some(0.5),
        None,
    );
    close(
        result
            .recirculation_temperature_rise
            .expect("a density")
            .value,
        0.230_125_322_415_165_26,
        "the rise at half efficiency",
    );
}

#[test]
fn the_refusals_are_the_class_validations() {
    // A discharge at the suction pressure.
    assert!(raw(2.0e5, 2.0e5, None, None, None, None, None, None).is_err());
    // A negative suction pressure.
    assert!(raw(-1.0, 1.0e6, None, None, None, None, None, None).is_err());
    // A shut-off ratio below one.
    assert!(raw(2.0e5, 1.0e6, Some(0.9), None, None, None, None, None).is_err());
    // A non-positive rating.
    assert!(raw(2.0e5, 1.0e6, None, Some(0.0), None, None, None, None).is_err());
    // A non-positive density.
    assert!(raw(2.0e5, 1.0e6, None, None, Some(0.0), None, None, None).is_err());
    // A non-positive specific heat.
    assert!(raw(2.0e5, 1.0e6, None, None, None, Some(0.0), None, None).is_err());
    // An efficiency outside (0, 1].
    assert!(raw(2.0e5, 1.0e6, None, None, None, None, Some(1.1), None).is_err());
    // A non-positive allowable rise.
    assert!(raw(2.0e5, 1.0e6, None, None, None, None, None, Some(0.0)).is_err());
    // A non-finite discharge pressure.
    assert!(raw(2.0e5, f64::NAN, None, None, None, None, None, None).is_err());
}
