//! `process.acoustic_induced_vibration_screening` against
//! `validation/neqsim/captures/process_acoustic_induced_vibration_screening.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{kelvins, kilograms_per_mole, kilograms_per_second, meters, pascals};
use azoth_process::models::acoustic_induced_vibration_screening::{
    AcousticInducedVibrationResult, acoustic_induced_vibration_screening, band_for,
};
use azoth_process::models::piping_fiv_gas::FivLikelihood;

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn aiv(
    mass_flow: f64,
    upstream: f64,
    downstream: f64,
    temperature: f64,
    molecular_weight: f64,
) -> AcousticInducedVibrationResult {
    acoustic_induced_vibration_screening(
        kilograms_per_second(mass_flow),
        pascals(upstream),
        pascals(downstream),
        kelvins(temperature),
        kilograms_per_mole(molecular_weight),
        meters(0.2),
        meters(0.01),
    )
    .expect("the captured state is valid")
}

#[test]
fn the_captured_states_answer_their_sound_power_levels() {
    let low = aiv(1.0, 1.0e6, 0.9e6, 300.0, 0.020);
    close(low.pwl, 114.14305035848987, "the low PWL");
    close(low.allowable_pwl, 171.1, "the low allowable");
    close(low.margin, -56.95694964151012, "the low margin");
    close(low.pressure_drop_ratio, 0.1, "the low ratio");
    assert_eq!(low.likelihood, FivLikelihood::Low);

    let medium = aiv(10.0, 5.0e6, 0.5e6, 350.0, 0.020);
    close(medium.pwl, 169.29914217387292, "the medium PWL");
    close(medium.margin, -1.8008578261270713, "the medium margin");
    assert_eq!(medium.likelihood, FivLikelihood::Medium);

    let high = aiv(40.0, 5.0e6, 0.05e6, 400.0, 0.020);
    close(high.pwl, 183.5263820298605, "the high PWL");
    close(high.margin, 12.42638202986052, "the high margin");
    assert_eq!(high.likelihood, FivLikelihood::VeryHigh);
}

#[test]
fn a_zero_downstream_pressure_is_a_full_let_down() {
    let result = aiv(1.0, 1.0e6, 0.0, 300.0, 0.020);
    close(result.pressure_drop_ratio, 1.0, "the ratio");
    close(result.pwl, 150.14305035848986, "the PWL");
    close(result.margin, -20.956949641510136, "the margin");
    assert_eq!(result.likelihood, FivLikelihood::Low);
}

#[test]
fn the_band_boundaries_are_exclusive_upward() {
    assert_eq!(band_for(-10.0), FivLikelihood::Medium);
    assert_eq!(band_for(0.0), FivLikelihood::High);
    assert_eq!(band_for(10.0), FivLikelihood::VeryHigh);
    assert_eq!(band_for(-10.000001), FivLikelihood::Low);
    assert_eq!(band_for(-0.000001), FivLikelihood::Medium);
    assert_eq!(band_for(9.999999), FivLikelihood::High);
}

#[test]
fn the_refusals_are_the_class_validations() {
    // A non-positive flow.
    assert!(
        acoustic_induced_vibration_screening(
            kilograms_per_second(0.0),
            pascals(1.0e6),
            pascals(0.9e6),
            kelvins(300.0),
            kilograms_per_mole(0.020),
            meters(0.2),
            meters(0.01),
        )
        .is_err()
    );
    // A non-positive upstream pressure.
    assert!(
        acoustic_induced_vibration_screening(
            kilograms_per_second(1.0),
            pascals(0.0),
            pascals(0.0),
            kelvins(300.0),
            kilograms_per_mole(0.020),
            meters(0.2),
            meters(0.01),
        )
        .is_err()
    );
    // A downstream pressure at or above the upstream one.
    for downstream in [1.0e6, 1.1e6, -1.0] {
        assert!(
            acoustic_induced_vibration_screening(
                kilograms_per_second(1.0),
                pascals(1.0e6),
                pascals(downstream),
                kelvins(300.0),
                kilograms_per_mole(0.020),
                meters(0.2),
                meters(0.01),
            )
            .is_err()
        );
    }
    // A non-positive temperature, molar mass, diameter or wall.
    for (temperature, molar, diameter, wall) in [
        (0.0, 0.020, 0.2, 0.01),
        (300.0, 0.0, 0.2, 0.01),
        (300.0, 0.020, 0.0, 0.01),
        (300.0, 0.020, 0.2, 0.0),
    ] {
        assert!(
            acoustic_induced_vibration_screening(
                kilograms_per_second(1.0),
                pascals(1.0e6),
                pascals(0.9e6),
                kelvins(temperature),
                kilograms_per_mole(molar),
                meters(diameter),
                meters(wall),
            )
            .is_err()
        );
    }
}
