//! `process.flow_induced_pulsation_screening` against
//! `validation/neqsim/captures/process_flow_induced_pulsation_screening.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{kilograms_per_cubic_meter, meters, meters_per_second};
use azoth_process::models::flow_induced_pulsation_screening::{
    FlowInducedPulsationResult, flow_induced_pulsation_screening,
};
use azoth_process::models::piping_fiv_gas::FivLikelihood;

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn pulsation(
    acoustic_length: f64,
    branch_diameter: f64,
    run_diameter: f64,
    velocity: f64,
    density: f64,
    speed_of_sound: f64,
    edge_radius: Option<f64>,
    termination: &str,
    strouhal: Option<f64>,
    mode_count: Option<f64>,
) -> FlowInducedPulsationResult {
    flow_induced_pulsation_screening(
        meters(acoustic_length),
        meters(branch_diameter),
        meters(run_diameter),
        meters_per_second(velocity),
        kilograms_per_cubic_meter(density),
        meters_per_second(speed_of_sound),
        edge_radius.map(meters),
        Some(termination),
        strouhal,
        mode_count,
    )
    .expect("the captured state is valid")
}

fn base(velocity: f64, density: f64) -> FlowInducedPulsationResult {
    pulsation(
        3.0, 0.15, 0.3, velocity, density, 400.0, None, "closed", None, None,
    )
}

#[test]
fn an_unlocked_branch_is_low_and_prints_every_mode() {
    let result = base(20.0, 10.0);
    assert_eq!(result.likelihood, FivLikelihood::Low);
    assert!(!result.any_mode_locked_in);
    close(result.shedding_frequency.value, 62.813150873601366, "fs");
    close(
        result.lowest_resonance_velocity.value,
        10.613488694560113,
        "the lowest resonance velocity",
    );
    close(
        result.effective_width.value,
        0.11780972450961724,
        "the effective width",
    );
    close(result.rho_v2.value, 4000.0, "the momentum flux");
    close(result.energy_term, 0.2, "the energy term");
    close(result.severity, 0.0, "the severity");
    assert_eq!(result.modes.len(), 5);
    close(
        result.modes[0].frequency.value,
        33.333333333333336,
        "mode 0",
    );
    close(result.modes[1].frequency.value, 100.0, "mode 1");
    close(
        result.modes[2].frequency.value,
        166.66666666666666,
        "mode 2",
    );
    close(
        result.modes[3].frequency.value,
        233.33333333333334,
        "mode 3",
    );
    close(result.modes[4].frequency.value, 300.0, "mode 4");
    assert!(result.modes.iter().all(|mode| !mode.locked_in));
    close(
        result.modes[0].envelope_low.value,
        26.66666666666667,
        "mode 0's lower envelope",
    );
    close(
        result.modes[0].envelope_high.value,
        40.0,
        "mode 0's upper envelope",
    );
}

#[test]
fn a_closed_branch_locks_on_the_second_mode() {
    let result = base(31.84, 10.0);
    assert_eq!(result.likelihood, FivLikelihood::Medium);
    assert!(result.any_mode_locked_in);
    close(result.mode_weight, 0.3333333333333333, "the mode weight");
    close(result.severity, 0.16896426666666667, "the severity");
    assert!(!result.modes[0].locked_in);
    assert!(result.modes[1].locked_in);
}

#[test]
fn an_open_branch_locks_on_the_fundamental() {
    let result = pulsation(3.0, 0.15, 0.3, 20.0, 10.0, 400.0, None, "open", None, None);
    assert_eq!(result.likelihood, FivLikelihood::Medium);
    close(result.severity, 0.2, "the severity");
    close(result.mode_weight, 1.0, "the weight");
    close(
        result.lowest_resonance_velocity.value,
        21.226977389120226,
        "the lowest resonance velocity",
    );
    assert!(result.modes[0].locked_in);
}

#[test]
fn the_severity_bands_are_exact_at_their_boundaries() {
    let high = pulsation(3.0, 0.15, 0.3, 20.0, 15.0, 400.0, None, "open", None, None);
    close(high.severity, 0.3, "the 0.3 boundary");
    assert_eq!(high.likelihood, FivLikelihood::High);

    let very_high = pulsation(3.0, 0.15, 0.3, 20.0, 50.0, 400.0, None, "open", None, None);
    close(very_high.severity, 1.0, "the 1.0 boundary");
    assert_eq!(very_high.likelihood, FivLikelihood::VeryHigh);

    let negligible = base(9.03, 10.0);
    assert!(negligible.any_mode_locked_in);
    close(
        negligible.severity,
        0.04077044999999999,
        "the negligible severity",
    );
    assert_eq!(negligible.likelihood, FivLikelihood::Low);
}

#[test]
fn the_mode_count_and_edge_radius_are_the_callers() {
    let one = pulsation(
        3.0,
        0.15,
        0.3,
        31.84,
        10.0,
        400.0,
        None,
        "closed",
        None,
        Some(1.0),
    );
    assert_eq!(one.modes.len(), 1);
    assert!(!one.any_mode_locked_in);

    let three = pulsation(
        3.0,
        0.15,
        0.3,
        31.84,
        10.0,
        400.0,
        None,
        "closed",
        None,
        Some(3.0),
    );
    assert_eq!(three.modes.len(), 3);
    assert!(three.modes[1].locked_in);

    let edge = pulsation(
        3.0,
        0.15,
        0.3,
        20.0,
        10.0,
        400.0,
        Some(0.05),
        "closed",
        None,
        None,
    );
    close(edge.effective_width.value, 0.16780972450961723, "the width");
    close(edge.shedding_frequency.value, 44.09756360440186, "fs");
    close(edge.severity, 0.0, "the severity");
}

#[test]
fn the_refusals_are_the_class_validations() {
    let good = (3.0, 0.15, 0.3, 20.0, 10.0, 400.0);
    // Each non-positive geometric or acoustic input.
    for (index, bad) in [
        (0, (0.0, 0.15, 0.3, 20.0, 10.0, 400.0)),
        (1, (3.0, 0.0, 0.3, 20.0, 10.0, 400.0)),
        (2, (3.0, 0.15, 0.0, 20.0, 10.0, 400.0)),
        (5, (3.0, 0.15, 0.3, 20.0, 10.0, 0.0)),
    ] {
        let _ = index;
        assert!(
            flow_induced_pulsation_screening(
                meters(bad.0),
                meters(bad.1),
                meters(bad.2),
                meters_per_second(bad.3),
                kilograms_per_cubic_meter(bad.4),
                meters_per_second(bad.5),
                None,
                Some("closed"),
                None,
                None,
            )
            .is_err()
        );
    }
    // A negative edge radius, velocity or density.
    assert!(
        flow_induced_pulsation_screening(
            meters(good.0),
            meters(good.1),
            meters(good.2),
            meters_per_second(good.3),
            kilograms_per_cubic_meter(good.4),
            meters_per_second(good.5),
            Some(meters(-1.0)),
            Some("closed"),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        flow_induced_pulsation_screening(
            meters(good.0),
            meters(good.1),
            meters(good.2),
            meters_per_second(-1.0),
            kilograms_per_cubic_meter(good.4),
            meters_per_second(good.5),
            None,
            Some("closed"),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        flow_induced_pulsation_screening(
            meters(good.0),
            meters(good.1),
            meters(good.2),
            meters_per_second(good.3),
            kilograms_per_cubic_meter(-1.0),
            meters_per_second(good.5),
            None,
            Some("closed"),
            None,
            None,
        )
        .is_err()
    );
    // A zero Strouhal number and a mode count below one or fractional.
    for (strouhal, modes) in [(Some(0.0), None), (None, Some(0.0)), (None, Some(1.5))] {
        assert!(
            flow_induced_pulsation_screening(
                meters(good.0),
                meters(good.1),
                meters(good.2),
                meters_per_second(good.3),
                kilograms_per_cubic_meter(good.4),
                meters_per_second(good.5),
                None,
                Some("closed"),
                strouhal,
                modes,
            )
            .is_err()
        );
    }
    // An unknown termination.
    assert!(
        flow_induced_pulsation_screening(
            meters(good.0),
            meters(good.1),
            meters(good.2),
            meters_per_second(good.3),
            kilograms_per_cubic_meter(good.4),
            meters_per_second(good.5),
            None,
            Some("neither"),
            None,
            None,
        )
        .is_err()
    );
}
