//! `process.mass_balance_leak_detection` against
//! `validation/neqsim/captures/process_mass_balance_leak_detection.tsv`.

use azoth_core::units::{
    cubic_meters, kelvins, kilograms_per_cubic_meter, kilograms_per_second, pascals, seconds,
};
use azoth_process::models::mass_balance_leak_detection::{
    MassBalanceLeakDetectionResult, mass_balance_leak_detection,
};

fn components() -> Vec<String> {
    vec!["methane".to_string(), "ethane".to_string()]
}

fn close(actual: f64, expected: f64, what: &str) {
    close_with(actual, expected, what, 1.0e-9);
}

fn close_with(actual: f64, expected: f64, what: &str, tolerance: f64) {
    let error = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        error <= tolerance,
        "{what}: {actual} against {expected}, relative error {error:.3e} over {tolerance:.3e}"
    );
}

fn run_no_linepack() -> MassBalanceLeakDetectionResult {
    mass_balance_leak_detection(
        &components(),
        &[0.9, 0.1],
        pascals(5.0e6),
        kelvins(300.0),
        kilograms_per_second(10.0),
        kilograms_per_second(9.8),
        None,
        None,
        Some(cubic_meters(0.0)),
        Some(0.01),
        None,
        None,
        Some(seconds(60.0)),
        Some(3.0),
        Some("inlet"),
        Some("outlet"),
    )
    .expect("the state evaluates")
}

#[test]
fn no_linepack_is_the_meter_pair() {
    let result = run_no_linepack();
    assert_eq!(result.inlet_stream_name.as_deref(), Some("inlet"));
    assert_eq!(result.outlet_stream_name.as_deref(), Some("outlet"));
    close(result.measured_imbalance.value, 0.2, "imbalance");
    close(result.flow_uncertainty.value, 0.4200428549564914, "flow");
    close(result.linepack_uncertainty.value, 0.0, "linepack");
    close(
        result.combined_uncertainty.value,
        0.4200428549564914,
        "combined",
    );
    close(
        result.minimum_detectable_leak_rate.value,
        0.620042854956496,
        "MDR",
    );
    close(
        result
            .minimum_detectable_leak_fraction
            .expect("the reference flow is positive"),
        0.06200428549564957,
        "fraction",
    );
}

#[test]
fn a_linepack_state_reads_the_inlet_density() {
    let result = mass_balance_leak_detection(
        &components(),
        &[0.9, 0.1],
        pascals(5.0e6),
        kelvins(300.0),
        kilograms_per_second(10.0),
        kilograms_per_second(9.8),
        None,
        None,
        Some(cubic_meters(100.0)),
        Some(0.01),
        Some(pascals(10000.0)),
        Some(0.5),
        Some(seconds(60.0)),
        Some(3.0),
        None,
        None,
    )
    .expect("the state evaluates");
    // The port's bulk density is `6.2e-8` relative below the class's on this state.
    close_with(
        result.density.expect("the density is read").value,
        39.30385975046646,
        "density",
        1.0e-7,
    );
    close_with(
        result.linepack_uncertainty.value,
        0.5116216283472782,
        "linepack",
        1.0e-7,
    );
    close_with(
        result.combined_uncertainty.value,
        0.6619612455368671,
        "combined",
        1.0e-7,
    );
    close_with(
        result.minimum_detectable_leak_rate.value,
        0.8619612455368717,
        "MDR",
        1.0e-7,
    );
    close_with(
        result
            .minimum_detectable_leak_fraction
            .expect("the reference flow is positive"),
        0.08619612455368712,
        "fraction",
        1.0e-7,
    );
}

#[test]
fn a_supplied_density_supersedes_the_flash() {
    let result = mass_balance_leak_detection(
        &components(),
        &[0.9, 0.1],
        pascals(5.0e6),
        kelvins(300.0),
        kilograms_per_second(10.0),
        kilograms_per_second(9.8),
        None,
        Some(kilograms_per_cubic_meter(39.30386219986391)),
        Some(cubic_meters(100.0)),
        Some(0.01),
        Some(pascals(10000.0)),
        Some(0.5),
        Some(seconds(60.0)),
        Some(3.0),
        None,
        None,
    )
    .expect("the state evaluates");
    close(
        result.linepack_uncertainty.value,
        0.5116216283472782,
        "linepack",
    );
    close(
        result.density.expect("the density is stated").value,
        39.30386219986391,
        "density",
    );
}

#[test]
fn a_zero_reference_has_no_fraction() {
    let result = mass_balance_leak_detection(
        &components(),
        &[0.9, 0.1],
        pascals(5.0e6),
        kelvins(300.0),
        kilograms_per_second(0.0),
        kilograms_per_second(0.0),
        None,
        None,
        Some(cubic_meters(0.0)),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("the state evaluates");
    assert_eq!(result.minimum_detectable_leak_rate.value, 0.0);
    assert!(result.minimum_detectable_leak_fraction.is_none());
    assert!(result.density.is_none());
}

#[test]
fn the_setters_clamp() {
    let result = mass_balance_leak_detection(
        &components(),
        &[0.9, 0.1],
        pascals(5.0e6),
        kelvins(300.0),
        kilograms_per_second(10.0),
        kilograms_per_second(9.8),
        None,
        None,
        Some(cubic_meters(-5.0)),
        Some(-0.1),
        Some(pascals(-100000.0)),
        Some(-2.0),
        Some(seconds(0.0)),
        Some(-1.0),
        None,
        None,
    )
    .expect("the state evaluates");
    close(result.flow_uncertainty.value, 0.0, "flow");
    close(result.linepack_uncertainty.value, 0.0, "linepack");
    close(result.minimum_detectable_leak_rate.value, 0.2, "MDR");
    close(result.detection_window.value, 1.0, "window");
    close(result.confidence_multiplier, 0.0, "confidence");
}
