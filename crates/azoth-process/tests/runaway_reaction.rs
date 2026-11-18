//! `process.runaway_reaction` against
//! `validation/neqsim/captures/process_runaway_reaction.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{
    joules_per_kilogram_kelvin, joules_per_mole, kelvin_intervals, kelvins, kilograms, moles,
};
use azoth_process::models::runaway_reaction::{
    RunawayReactionResult, RunawayVerdict, runaway_reaction,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn raw(
    t0: f64,
    enthalpy: f64,
    reactant: f64,
    mass: f64,
    heat: f64,
    material_limit: Option<f64>,
    activation: Option<f64>,
    rate: Option<f64>,
    margin: Option<f64>,
    gassy: Option<bool>,
) -> azoth_core::Result<RunawayReactionResult> {
    runaway_reaction(
        kelvins(t0),
        joules_per_mole(enthalpy),
        moles(reactant),
        kilograms(mass),
        joules_per_kilogram_kelvin(heat),
        material_limit.map(kelvins),
        activation.map(joules_per_mole),
        rate,
        margin.map(kelvin_intervals),
        gassy,
    )
}

#[allow(clippy::too_many_arguments)]
fn screen(
    t0: f64,
    enthalpy: f64,
    reactant: f64,
    mass: f64,
    heat: f64,
    material_limit: Option<f64>,
    activation: Option<f64>,
    rate: Option<f64>,
    margin: Option<f64>,
    gassy: Option<bool>,
) -> RunawayReactionResult {
    raw(
        t0,
        enthalpy,
        reactant,
        mass,
        heat,
        material_limit,
        activation,
        rate,
        margin,
        gassy,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_no_mat_bands_are_inclusive() {
    let low = screen(
        350.0, -100_000.0, 10.0, 100.0, 2000.0, None, None, None, None, None,
    );
    close(low.adiabatic_temperature_rise.value, 5.0, "the low rise");
    close(low.mtsr.value, 355.0, "the low MTSR");
    assert!(low.temperature_margin.is_none());
    assert!(!low.margin_exceeded);
    close(low.total_heat_release.value, 1.0e6, "the low heat");
    assert_eq!(low.verdict, RunawayVerdict::NoRating);

    let marginal = screen(
        350.0,
        -1_000_000.0,
        10.0,
        100.0,
        2000.0,
        None,
        None,
        None,
        None,
        None,
    );
    close(
        marginal.adiabatic_temperature_rise.value,
        50.0,
        "the marginal rise",
    );
    assert_eq!(marginal.verdict, RunawayVerdict::Marginal);

    let risk = screen(
        350.0,
        -4_000_000.0,
        10.0,
        100.0,
        2000.0,
        None,
        None,
        None,
        None,
        None,
    );
    close(
        risk.adiabatic_temperature_rise.value,
        200.0,
        "the risk rise",
    );
    assert_eq!(risk.verdict, RunawayVerdict::RunawayRisk);
}

#[test]
fn a_mat_turns_the_margin_into_the_verdict() {
    let acceptable = screen(
        350.0,
        -500_000.0,
        10.0,
        100.0,
        2000.0,
        Some(500.0),
        None,
        None,
        None,
        None,
    );
    close(acceptable.mtsr.value, 375.0, "the acceptable MTSR");
    close(
        acceptable.temperature_margin.expect("a MAT").value,
        125.0,
        "the acceptable margin",
    );
    assert!(!acceptable.margin_exceeded);
    assert_eq!(acceptable.verdict, RunawayVerdict::AcceptableMargin);

    let marginal = screen(
        350.0,
        -800_000.0,
        10.0,
        100.0,
        2000.0,
        Some(400.0),
        None,
        None,
        None,
        None,
    );
    close(
        marginal.temperature_margin.expect("a MAT").value,
        10.0,
        "the marginal margin",
    );
    assert_eq!(marginal.verdict, RunawayVerdict::Marginal);
}

#[test]
fn an_exceeded_margin_and_a_gassy_system_force_the_two_phase_screen() {
    let result = screen(
        350.0,
        -800_000.0,
        10.0,
        100.0,
        2000.0,
        Some(370.0),
        None,
        None,
        None,
        Some(true),
    );
    close(result.mtsr.value, 390.0, "the MTSR");
    close(
        result.temperature_margin.expect("a MAT").value,
        -20.0,
        "the margin",
    );
    assert!(result.margin_exceeded);
    assert!(result.two_phase_relief_screening_required);
    assert_eq!(result.verdict, RunawayVerdict::RunawayRisk);
}

#[test]
fn the_tmr_needs_both_positive_inputs() {
    let result = screen(
        350.0,
        -100_000.0,
        10.0,
        100.0,
        2000.0,
        None,
        Some(80_000.0),
        Some(1.0),
        None,
        None,
    );
    close(
        result.time_to_max_rate.expect("the TMR pair").value,
        25_463.041_767_624_996,
        "the TMR",
    );
    assert_eq!(result.verdict, RunawayVerdict::NoRating);

    let absent = screen(
        350.0,
        -100_000.0,
        10.0,
        100.0,
        2000.0,
        None,
        None,
        Some(1.0),
        None,
        None,
    );
    assert!(absent.time_to_max_rate.is_none());
}

#[test]
fn the_refusals_are_the_class_validations() {
    // A MAT at the process temperature.
    assert!(
        raw(
            350.0,
            -100_000.0,
            10.0,
            100.0,
            2000.0,
            Some(350.0),
            None,
            None,
            None,
            None
        )
        .is_err()
    );
    // A non-positive process temperature.
    assert!(
        raw(
            0.0, -100_000.0, 10.0, 100.0, 2000.0, None, None, None, None, None
        )
        .is_err()
    );
    // Non-positive limiting-reactant moles.
    assert!(
        raw(
            350.0, -100_000.0, 0.0, 100.0, 2000.0, None, None, None, None, None
        )
        .is_err()
    );
    // A non-positive reacting mass.
    assert!(
        raw(
            350.0, -100_000.0, 10.0, 0.0, 2000.0, None, None, None, None, None
        )
        .is_err()
    );
    // A non-positive specific heat.
    assert!(
        raw(
            350.0, -100_000.0, 10.0, 100.0, 0.0, None, None, None, None, None
        )
        .is_err()
    );
    // A non-finite temperature.
    assert!(
        raw(
            f64::NAN,
            -100_000.0,
            10.0,
            100.0,
            2000.0,
            None,
            None,
            None,
            None,
            None
        )
        .is_err()
    );
}
