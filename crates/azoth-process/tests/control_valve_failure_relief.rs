//! `process.control_valve_failure_relief` against
//! `validation/neqsim/captures/process_control_valve_failure_relief.tsv`.

use azoth_core::units::{kelvins, pascals, square_meters};
use azoth_process::models::ReliefPhase;
use azoth_process::models::control_valve_failure_relief::{
    ControlValveFailureReliefResult, control_valve_failure_relief,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn close_rate(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-7,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)] // one argument per captured input
fn relief(
    upstream_pressure: f64,
    upstream_temperature: Option<f64>,
    downstream_pressure: Option<f64>,
    effective_area: Option<f64>,
    cv: Option<f64>,
    margin: Option<f64>,
    coefficient: Option<f64>,
    name: Option<&str>,
) -> ControlValveFailureReliefResult {
    let components = vec!["methane".to_string(), "ethane".to_string()];
    control_valve_failure_relief(
        &components,
        &[0.9, 0.1],
        pascals(upstream_pressure),
        upstream_temperature.map(kelvins),
        downstream_pressure.map(pascals),
        effective_area.map(square_meters),
        cv,
        margin,
        coefficient,
        name,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_explicit_area_reproduces_the_capture() {
    let area = relief(
        2.0e6,
        Some(320.0),
        Some(101_325.0),
        Some(0.001),
        None,
        None,
        None,
        Some("CV fail area"),
    );
    assert_eq!(area.name, "CV fail area");
    assert_eq!(area.phase, ReliefPhase::Vapour);
    close_rate(area.relief_rate.value, 2.888_494_774_655_727_7, "the rate");
    close(area.relief_temperature.value, 320.0, "the temperature");
    close(
        area.molar_mass.value,
        0.017_445_699_999_999_998,
        "the molar mass",
    );
    close(area.compressibility, 0.959_160_201_346_084_2, "Z");
    assert_eq!(
        area.assumptions,
        vec![
            "Fail-open control valve, upstream at maximum operating pressure per TR3001 SR-26501",
            "Discharge coefficient Cd = 0.85",
        ]
    );
}

#[test]
fn the_default_downstream_pressure_is_one_atmosphere() {
    let default = relief(
        2.0e6,
        Some(320.0),
        None,
        Some(0.001),
        None,
        None,
        None,
        Some("CV fail default downstream"),
    );
    close_rate(
        default.relief_rate.value,
        2.888_494_774_655_727_7,
        "the default-downstream rate",
    );
}

#[test]
fn a_cv_derives_the_area_and_says_so() {
    let by_cv = relief(
        2.0e6,
        Some(320.0),
        Some(101_325.0),
        None,
        Some(150.0),
        None,
        None,
        Some("CV fail cv"),
    );
    close_rate(
        by_cv.relief_rate.value,
        13.368_301_928_438_62,
        "the Cv rate",
    );
    assert_eq!(
        by_cv.assumptions,
        vec![
            "Fail-open control valve, upstream at maximum operating pressure per TR3001 SR-26501",
            "Discharge coefficient Cd = 0.85",
            "Effective area derived from Cv with screening relation Cv = 24.6 Cd A_in2",
        ]
    );
}

#[test]
fn the_erosion_margin_multiplies_cv_and_warns() {
    let margin = relief(
        2.0e6,
        Some(320.0),
        Some(101_325.0),
        None,
        Some(150.0),
        Some(1.25),
        None,
        Some("CV fail margin"),
    );
    close_rate(
        margin.relief_rate.value,
        16.710_377_410_548_272,
        "the margined rate",
    );
    assert_eq!(
        margin.assumptions,
        vec![
            "Fail-open control valve, upstream at maximum operating pressure per TR3001 SR-26501",
            "Discharge coefficient Cd = 0.85",
            "Cv increased by erosion/vibration margin 1.25 per TR3001 SR-26501",
            "Effective area derived from Cv with screening relation Cv = 24.6 Cd A_in2",
        ]
    );
}

#[test]
fn a_subcritical_downstream_takes_the_subcritical_branch() {
    let subcritical = relief(
        2.0e6,
        Some(320.0),
        Some(1.5e6),
        Some(0.001),
        None,
        None,
        None,
        Some("CV fail subcritical"),
    );
    close_rate(
        subcritical.relief_rate.value,
        2.596_230_056_867_641,
        "the subcritical rate",
    );
}

#[test]
fn no_area_and_no_cv_answers_zero_with_the_warning() {
    let none = relief(
        2.0e6,
        Some(320.0),
        Some(101_325.0),
        None,
        None,
        None,
        None,
        Some("CV fail no area"),
    );
    close(none.relief_rate.value, 0.0, "the zero rate");
    assert_eq!(
        none.assumptions,
        vec![
            "Fail-open control valve, upstream at maximum operating pressure per TR3001 SR-26501",
            "Discharge coefficient Cd = 0.85",
            "Neither effective area nor Cv set; relief rate is zero",
        ]
    );
}

#[test]
fn a_coefficient_outside_the_unit_interval_is_refused() {
    for bad in [0.0, 1.5] {
        let outcome = control_valve_failure_relief(
            &["methane".to_string()],
            &[1.0],
            pascals(2.0e6),
            None,
            None,
            Some(square_meters(0.001)),
            None,
            None,
            Some(bad),
            None,
        );
        assert!(
            outcome.is_err(),
            "Cd = {bad} is not a discharge coefficient"
        );
    }
}

#[test]
fn a_margin_below_one_is_refused() {
    let outcome = control_valve_failure_relief(
        &["methane".to_string()],
        &[1.0],
        pascals(2.0e6),
        None,
        None,
        None,
        Some(150.0),
        Some(0.5),
        None,
        None,
    );
    assert!(
        outcome.is_err(),
        "a margin below one would shrink the capacity"
    );
}

#[test]
fn the_default_scenario_name_is_the_classes() {
    let default = relief(
        2.0e6,
        Some(320.0),
        Some(101_325.0),
        Some(0.001),
        None,
        None,
        None,
        None,
    );
    assert_eq!(default.name, "Control valve failure");
}
