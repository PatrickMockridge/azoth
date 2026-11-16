//! `process.tube_rupture_relief` against
//! `validation/neqsim/captures/process_tube_rupture_relief.tsv`.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::models::ReliefPhase;
use azoth_process::models::tube_rupture_relief::{TubeRuptureReliefResult, tube_rupture_relief};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-7,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn relief(
    diameter: f64,
    ends: Option<f64>,
    high_pressure: f64,
    low_pressure: Option<f64>,
    name: Option<&str>,
) -> TubeRuptureReliefResult {
    let components = vec!["methane".to_string(), "ethane".to_string()];
    tube_rupture_relief(
        &components,
        &[0.9, 0.1],
        meters(diameter),
        pascals(high_pressure),
        ends,
        Some(kelvins(320.0)),
        low_pressure.map(pascals),
        None,
        name,
    )
    .expect("the captured state is valid")
}

#[test]
fn two_open_ends_reproduce_the_capture() {
    let two = relief(
        0.016,
        None,
        2.0e6,
        Some(101_325.0),
        Some("Tube rupture two ends"),
    );
    assert_eq!(two.name, "Tube rupture two ends");
    assert_eq!(two.phase, ReliefPhase::Vapour);
    close(two.relief_rate.value, 1.161_532_667_390_840_2, "the rate");
    close(
        two.molar_mass.value,
        0.017_445_699_999_999_998,
        "the molar mass",
    );
    close(two.compressibility, 0.959_160_201_346_084_2, "Z");
    close(
        two.specific_heat_ratio,
        1.279_579_897_384_378_6,
        "the isentropic exponent",
    );
    assert_eq!(
        two.assumptions,
        vec![
            "2 open ends (full-bore single-tube rupture) per TR3001 SR-26616",
            "High-pressure fluid at maximum operating pressure",
            "Vaporization/flashing on the low-pressure side not modelled (screening estimate)",
            "Discharge coefficient Cd = 0.85",
        ]
    );
}

#[test]
fn one_open_end_is_half_the_two_end_rate() {
    let one = relief(
        0.016,
        Some(1.0),
        2.0e6,
        Some(101_325.0),
        Some("Tube rupture one end"),
    );
    close(
        one.relief_rate.value,
        0.580_766_333_695_420_1,
        "the one-end rate",
    );
    assert_eq!(
        one.assumptions[0],
        "1 open ends (full-bore single-tube rupture) per TR3001 SR-26616"
    );
}

#[test]
fn a_subcritical_low_side_takes_the_subcritical_branch() {
    let subcritical = relief(
        0.016,
        None,
        2.0e6,
        Some(1.5e6),
        Some("Tube rupture subcritical"),
    );
    close(
        subcritical.relief_rate.value,
        1.044_006_051_031_602,
        "the subcritical rate",
    );
}

#[test]
fn the_default_low_pressure_is_one_atmosphere() {
    let default = relief(0.016, None, 2.0e6, None, Some("Tube rupture two ends"));
    close(
        default.relief_rate.value,
        1.161_532_667_390_840_2,
        "the default-low pressure rate",
    );
}

#[test]
fn a_count_below_one_is_refused() {
    let components = vec!["methane".to_string()];
    let outcome = tube_rupture_relief(
        &components,
        &[1.0],
        meters(0.016),
        pascals(2.0e6),
        Some(0.0),
        None,
        None,
        None,
        None,
    );
    assert!(
        outcome.is_err(),
        "a count below one must be refused, not reset to two"
    );
}

#[test]
fn the_default_scenario_name_is_the_classes() {
    let default = relief(0.016, None, 2.0e6, Some(101_325.0), None);
    assert_eq!(default.name, "Tube rupture");
}
