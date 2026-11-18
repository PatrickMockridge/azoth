//! `process.depressurization_acceptance` against
//! `validation/neqsim/captures/process_depressurization_acceptance.tsv`.

#![allow(clippy::excessive_precision)]

use azoth_core::units::{kilograms, kilograms_per_second, pascals, seconds};
use azoth_process::models::depressurization_acceptance::{
    DepressurizationAcceptancePoint, DepressurizationAcceptanceResult, depressurization_acceptance,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn trajectory() -> Vec<DepressurizationAcceptancePoint> {
    vec![
        DepressurizationAcceptancePoint {
            time: seconds(0.0),
            pressure: pascals(5.0e6),
            mass: kilograms(100.0),
            mass_flow: kilograms_per_second(0.0),
        },
        DepressurizationAcceptancePoint {
            time: seconds(100.0),
            pressure: pascals(1.0e6),
            mass: kilograms(40.0),
            mass_flow: kilograms_per_second(5.0),
        },
        DepressurizationAcceptancePoint {
            time: seconds(200.0),
            pressure: pascals(5.0e5),
            mass: kilograms(20.0),
            mass_flow: kilograms_per_second(2.0),
        },
    ]
}

#[allow(clippy::too_many_arguments)]
fn accept(
    escape: Option<f64>,
    rupture: Option<f64>,
    pressure: Option<f64>,
    mass: Option<f64>,
    rate: Option<f64>,
) -> DepressurizationAcceptanceResult {
    depressurization_acceptance(
        &trajectory(),
        escape.map(seconds),
        rupture.map(seconds),
        pressure.map(pascals),
        mass.map(kilograms),
        rate.map(kilograms_per_second),
    )
    .expect("the captured state is valid")
}

#[test]
fn the_passing_and_failing_criteria_read_the_capture() {
    let passed = accept(Some(120.0), Some(100.0), Some(1.5e6), Some(50.0), Some(6.0));
    close(passed.limiting_time.value, 100.0, "limiting time");
    close(
        passed.pressure_at_limiting_time.value,
        1.0e6,
        "pressure at the limit",
    );
    close(
        passed.remaining_mass_at_limiting_time.value,
        40.0,
        "mass at the limit",
    );
    close(passed.peak_discharge_rate.value, 5.0, "peak rate");
    assert!(passed.acceptable);

    assert!(!accept(Some(120.0), Some(100.0), Some(0.9e6), Some(50.0), Some(6.0)).acceptable);
    assert!(!accept(Some(120.0), Some(100.0), Some(1.5e6), Some(30.0), Some(6.0)).acceptable);
    assert!(!accept(Some(120.0), Some(100.0), Some(1.5e6), Some(50.0), Some(4.0)).acceptable);

    let none = accept(None, None, None, None, None);
    close(none.limiting_time.value, 200.0, "no-criteria limiting time");
    assert!(none.acceptable);
    assert!(!none.pressure_configured);
    assert!(!none.mass_configured);
    assert!(!none.fire_rate_configured);
}

#[test]
fn the_limiting_time_and_interpolation_read_the_capture() {
    let escape = accept(Some(50.0), Some(150.0), Some(1.5e6), Some(80.0), Some(6.0));
    close(escape.limiting_time.value, 50.0, "escape limiting time");
    close(
        escape.pressure_at_limiting_time.value,
        3.0e6,
        "interpolated pressure",
    );
    close(
        escape.remaining_mass_at_limiting_time.value,
        70.0,
        "interpolated mass",
    );
    close(
        escape.peak_discharge_rate.value,
        0.0,
        "peak before the sample",
    );

    let equality = accept(None, None, Some(1.0e6), Some(40.0), Some(5.0));
    close(
        equality.limiting_time.value,
        200.0,
        "equality limiting time",
    );
    close(
        equality.pressure_at_limiting_time.value,
        5.0e5,
        "endpoint pressure",
    );
    assert!(equality.acceptable);

    let beyond = accept(Some(500.0), None, Some(1.5e6), Some(50.0), Some(6.0));
    close(
        beyond.limiting_time.value,
        200.0,
        "beyond-last limiting time",
    );
    close(
        beyond.remaining_mass_at_limiting_time.value,
        20.0,
        "endpoint mass",
    );
}

#[test]
fn the_empty_trajectory_is_refused() {
    assert!(depressurization_acceptance(&[], None, None, None, None, None).is_err());
}
