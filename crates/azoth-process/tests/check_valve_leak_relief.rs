//! `process.check_valve_leak_relief` against
//! `validation/neqsim/captures/process_check_valve_leak_relief.tsv`.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::models::ReliefPhase;
use azoth_process::models::check_valve_leak_relief::{
    CheckValveLeakReliefResult, check_valve_leak_relief,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-7,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn relief(
    diameter: f64,
    fraction: Option<f64>,
    upstream_pressure: f64,
    downstream_pressure: Option<f64>,
    name: Option<&str>,
) -> CheckValveLeakReliefResult {
    let components = vec!["methane".to_string(), "ethane".to_string()];
    check_valve_leak_relief(
        &components,
        &[0.9, 0.1],
        meters(diameter),
        pascals(upstream_pressure),
        fraction,
        Some(kelvins(320.0)),
        downstream_pressure.map(pascals),
        None,
        name,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_default_fraction_reproduces_the_capture() {
    let default = relief(
        0.1524,
        None,
        2.0e6,
        Some(101_325.0),
        Some("Check valve default"),
    );
    assert_eq!(default.name, "Check valve default");
    assert_eq!(default.phase, ReliefPhase::Vapour);
    close(
        default.relief_rate.value,
        0.526_903_886_034_755_5,
        "the rate",
    );
    close(
        default.molar_mass.value,
        0.017_445_699_999_999_998,
        "the molar mass",
    );
    close(default.compressibility, 0.959_160_201_346_084_2, "Z");
    close(
        default.specific_heat_ratio,
        1.279_579_897_384_378_6,
        "the isentropic exponent",
    );
    assert_eq!(
        default.assumptions,
        vec![
            "Leak area = 1.00% of nominal flow area per TR3001 SR-26466",
            "Upstream pressure taken at high-pressure source MAAP",
            "Discharge coefficient Cd = 0.85",
        ]
    );
}

#[test]
fn a_small_fraction_is_clamped_to_the_minimum() {
    let clamped = relief(
        0.1524,
        Some(0.0005),
        2.0e6,
        Some(101_325.0),
        Some("Check valve clamped"),
    );
    close(
        clamped.relief_rate.value,
        0.052_690_388_603_475_545,
        "the clamped rate",
    );
    assert_eq!(
        clamped.assumptions,
        vec![
            "Leak area = 0.10% of nominal flow area per TR3001 SR-26466",
            "Upstream pressure taken at high-pressure source MAAP",
            "Discharge coefficient Cd = 0.85",
            "Leak area fraction below TR3001 minimum 0.1%; clamped to 0.001",
        ]
    );
}

#[test]
fn a_subcritical_downstream_takes_the_subcritical_branch() {
    let subcritical = relief(
        0.1524,
        None,
        2.0e6,
        Some(1.5e6),
        Some("Check valve subcritical"),
    );
    close(
        subcritical.relief_rate.value,
        0.473_590_507_418_120_03,
        "the subcritical rate",
    );
}

#[test]
fn a_zero_diameter_answers_zero_with_the_warning() {
    let zero = relief(0.0, None, 2.0e6, Some(101_325.0), Some("Check valve zero"));
    close(zero.relief_rate.value, 0.0, "the zero rate");
    assert_eq!(
        zero.assumptions,
        vec![
            "Leak area = 1.00% of nominal flow area per TR3001 SR-26466",
            "Upstream pressure taken at high-pressure source MAAP",
            "Discharge coefficient Cd = 0.85",
            "Nominal check-valve diameter not set; relief rate is zero",
        ]
    );
}

#[test]
fn a_negative_fraction_is_refused() {
    let components = vec!["methane".to_string()];
    let outcome = check_valve_leak_relief(
        &components,
        &[1.0],
        meters(0.1524),
        pascals(2.0e6),
        Some(-0.001),
        None,
        None,
        None,
        None,
    );
    assert!(outcome.is_err(), "a negative leak fraction is not an area");
}

#[test]
fn the_default_scenario_name_is_the_classes() {
    let default = relief(0.1524, None, 2.0e6, Some(101_325.0), None);
    assert_eq!(default.name, "Check valve leakage");
}
