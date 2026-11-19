//! `process.non_equilibrium_blowdown` against
//! `validation/neqsim/captures/process_non_equilibrium_blowdown.tsv`.

#![allow(clippy::excessive_precision)]

use azoth_core::units::{
    cubic_meters, kelvins, kilograms, meters, pascals, seconds, square_meters,
    watts_per_square_meter_kelvin,
};
use azoth_process::models::non_equilibrium_blowdown::non_equilibrium_blowdown;

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 5.0e-3,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn two_phase() -> Vec<String> {
    ["methane", "propane", "n-butane"]
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

#[test]
fn the_fire_exposed_state_reads_the_capture() {
    let result = non_equilibrium_blowdown(
        &two_phase(),
        &[0.6, 0.2, 0.2],
        kelvins(280.0),
        pascals(5.0e6),
        cubic_meters(10.0),
        meters(0.012),
        0.85,
        pascals(1.0e5),
        None,
        None,
        None,
        Some(square_meters(20.0)),
        Some(kilograms(4000.0)),
        Some(azoth_core::units::joules_per_kilogram_kelvin(470.0)),
        Some(watts_per_square_meter_kelvin(50.0)),
        Some(watts_per_square_meter_kelvin(80.0)),
        Some(square_meters(4.0)),
        Some(350_000.0),
        None,
        None,
        None,
        None,
        Some(seconds(1.0)),
        Some(seconds(20.0)),
        Some(pascals(3.0e5)),
    )
    .expect("the captured state is valid");
    close(
        result.final_gas_temperature.value,
        280.096_028_503_062_3,
        "final gas temperature",
    );
    close(
        result.final_liquid_temperature.value,
        280.000_123_741_830_8,
        "final liquid temperature",
    );
    assert_eq!(result.recorded_points, 21.0);
    assert_eq!(result.flash_fallback_count, 0.0);
    assert!(result.max_temperature_bifurcation.value > 0.0);
}

#[test]
fn a_single_phase_feed_is_refused() {
    assert!(
        non_equilibrium_blowdown(
            &["methane".to_string()],
            &[1.0],
            kelvins(300.0),
            pascals(5.0e6),
            cubic_meters(10.0),
            meters(0.012),
            0.85,
            pascals(1.0e5),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(seconds(1.0)),
            Some(seconds(20.0)),
            Some(pascals(3.0e5)),
        )
        .is_err()
    );
}
