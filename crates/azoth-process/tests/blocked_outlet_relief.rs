//! `process.blocked_outlet_relief` against
//! `validation/neqsim/captures/process_blocked_outlet_relief.tsv`.

use azoth_core::units::{kelvins, kilograms_per_second, pascals};
use azoth_process::models::ReliefPhase;
use azoth_process::models::blocked_outlet_relief::{
    BlockedOutletReliefResult, blocked_outlet_relief,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

/// The isentropic exponent is a property through two ideal-gas models, so it agrees to
/// `1e-7` rather than to the last bit: measured `1.9e-8` on the gas row and `1.6e-8` on the
/// two-phase one. The value itself is the check - `getGamma` would be 4% away on the gas.
fn close_exponent(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-7,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn relief(
    components: &[&str],
    mole_fractions: &[f64],
    inflow_rate: f64,
    relief_pressure: f64,
    relief_temperature: f64,
    scenario_name: Option<&str>,
) -> BlockedOutletReliefResult {
    let components: Vec<String> = components.iter().map(|name| (*name).to_string()).collect();
    blocked_outlet_relief(
        &components,
        mole_fractions,
        kilograms_per_second(inflow_rate),
        pascals(relief_pressure),
        kelvins(relief_temperature),
        scenario_name,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_gas_state_reproduces_the_capture() {
    let gas = relief(
        &["methane", "ethane"],
        &[0.9, 0.1],
        2.0,
        2.0e6,
        320.0,
        Some("Blocked outlet gas"),
    );
    assert_eq!(gas.name, "Blocked outlet gas");
    assert_eq!(gas.phase, ReliefPhase::Vapour);
    close(gas.relief_rate.value, 2.0, "the relief rate");
    close(gas.relief_temperature.value, 320.0, "the temperature");
    close(
        gas.molar_mass.value,
        0.017_445_699_999_999_998,
        "the molar mass",
    );
    close(gas.compressibility, 0.959_160_201_346_084_2, "Z");
    // `getKappa`, not `getGamma`'s 1.3319393916069173.
    close_exponent(
        gas.specific_heat_ratio,
        1.279_579_897_384_378_6,
        "the isentropic exponent",
    );
    assert_eq!(
        gas.assumptions,
        vec!["Relief rate set equal to full inflow per TR3001 SR-26460"]
    );
}

#[test]
fn the_liquid_state_carries_the_isentropic_exponent_not_gamma() {
    let liquid = relief(
        &["n-butane"],
        &[1.0],
        1.5,
        5.0e5,
        250.0,
        Some("Blocked outlet liquid"),
    );
    assert_eq!(liquid.phase, ReliefPhase::Liquid);
    close(
        liquid.molar_mass.value,
        0.058_122_999_999_999_994,
        "the molar mass",
    );
    close(liquid.compressibility, 0.021_212_138_545_870_85, "Z");
    // The capture's `gamma` is 1.2639517801547224; the class reads `kappa`.
    close_exponent(
        liquid.specific_heat_ratio,
        1_255.272_378_839_346_4,
        "the isentropic exponent",
    );
    assert!(liquid.specific_heat_ratio > 100.0);
}

#[test]
fn the_two_phase_state_reproduces_the_capture() {
    let two_phase = relief(
        &["methane", "n-butane"],
        &[0.5, 0.5],
        1.0,
        2.0e6,
        300.0,
        Some("Blocked outlet two phase"),
    );
    assert_eq!(two_phase.phase, ReliefPhase::TwoPhase);
    close(two_phase.molar_mass.value, 0.037_083, "the molar mass");
    close(two_phase.compressibility, 0.532_567_794_212_993_9, "Z");
    close_exponent(
        two_phase.specific_heat_ratio,
        1_965.173_645_351_309,
        "the isentropic exponent",
    );
}

#[test]
fn a_zero_inflow_warns_and_stays_zero() {
    let zero = relief(
        &["methane", "ethane"],
        &[0.9, 0.1],
        0.0,
        2.0e6,
        320.0,
        Some("Blocked outlet zero"),
    );
    close(zero.relief_rate.value, 0.0, "the relief rate");
    assert_eq!(
        zero.assumptions,
        vec![
            "Relief rate set equal to full inflow per TR3001 SR-26460",
            "Inflow rate not set or non-positive; relief rate is zero",
        ]
    );
}

#[test]
fn a_negative_inflow_is_refused() {
    let components = vec!["methane".to_string()];
    let outcome = blocked_outlet_relief(
        &components,
        &[1.0],
        kilograms_per_second(-1.0),
        pascals(2.0e6),
        kelvins(320.0),
        None,
    );
    assert!(outcome.is_err(), "a negative inflow is not a relief state");
}

#[test]
fn a_composition_that_does_not_sum_is_refused() {
    let components = vec!["methane".to_string(), "ethane".to_string()];
    let outcome = blocked_outlet_relief(
        &components,
        &[0.5, 0.4],
        kilograms_per_second(2.0),
        pascals(2.0e6),
        kelvins(320.0),
        None,
    );
    assert!(
        outcome.is_err(),
        "a composition that does not sum is not a fluid"
    );
}

#[test]
fn the_default_scenario_name_is_the_classes() {
    let default = relief(&["methane", "ethane"], &[0.9, 0.1], 2.0, 2.0e6, 320.0, None);
    assert_eq!(default.name, "Blocked outlet");
}
