//! `process.blocked_outlet_overpressure` against
//! `validation/neqsim/captures/process_blocked_outlet_overpressure.tsv`.

#![allow(clippy::excessive_precision)]

use azoth_core::units::{cubic_meters, kelvins, kilograms_per_second, pascals, seconds};
use azoth_process::models::blocked_outlet_overpressure::blocked_outlet_overpressure;

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 5.0e-3,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[test]
fn the_demand_and_no_demand_states_read_the_capture() {
    let reaches = blocked_outlet_overpressure(
        &[
            "methane".to_string(),
            "ethane".to_string(),
            "propane".to_string(),
        ],
        &[0.9, 0.07, 0.03],
        kelvins(298.15),
        pascals(1.0e6),
        cubic_meters(5.0),
        kelvins(298.15),
        pascals(15.0e6),
        kilograms_per_second(2.0),
        pascals(5.0e6),
        Some(seconds(1.0)),
        Some(seconds(600.0)),
    )
    .expect("the captured state is valid");
    close(
        reaches.max_pressure.value,
        5_015_206.150_224_145_5,
        "max pressure",
    );
    close(
        reaches.final_mass.value,
        200.976_493_715_673_97,
        "final mass",
    );
    close(reaches.simulated_horizon.value, 600.0, "horizon");
    close(
        reaches.time_to_relief_set.expect("the crossing").value,
        82.0,
        "time to demand",
    );
    assert!(reaches.relief_demand);

    let not_reached = blocked_outlet_overpressure(
        &[
            "methane".to_string(),
            "ethane".to_string(),
            "propane".to_string(),
        ],
        &[0.9, 0.07, 0.03],
        kelvins(298.15),
        pascals(1.0e6),
        cubic_meters(5.0),
        kelvins(298.15),
        pascals(15.0e6),
        kilograms_per_second(0.2),
        pascals(5.0e6),
        Some(seconds(1.0)),
        Some(seconds(20.0)),
    )
    .expect("the captured state is valid");
    close(
        not_reached.max_pressure.value,
        1_097_816.120_535_900_8,
        "low-flow max pressure",
    );
    assert!(not_reached.time_to_relief_set.is_none());
    assert!(!not_reached.relief_demand);
}

#[test]
fn the_refusals_are_the_class_guards() {
    assert!(
        blocked_outlet_overpressure(
            &["methane".to_string()],
            &[1.0],
            kelvins(298.15),
            pascals(1.0e6),
            cubic_meters(5.0),
            kelvins(298.15),
            pascals(15.0e6),
            kilograms_per_second(2.0),
            pascals(0.0),
            Some(seconds(1.0)),
            Some(seconds(20.0)),
        )
        .is_err()
    );
}
