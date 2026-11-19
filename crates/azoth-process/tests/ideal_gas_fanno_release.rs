//! `process.ideal_gas_fanno_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::{IdealGasFannoReleaseResult, ideal_gas_fanno_release};

fn pipe(
    pressure: f64,
    back: f64,
    diameter: f64,
    length: f64,
    friction: f64,
) -> IdealGasFannoReleaseResult {
    ideal_gas_fanno_release(
        &["nitrogen".to_string()],
        &[1.0],
        kelvins(300.0),
        pascals(pressure),
        meters(diameter),
        1.0,
        pascals(back),
        length,
        friction,
    )
    .expect("the state runs")
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs()
}

#[test]
fn the_long_pipe_is_choked_at_the_captured_rate() {
    let result = pipe(5_000_000.0, 101_325.0, 0.1, 100.0, 0.02);
    assert_eq!(result.status, "valid");
    assert!(result.choked);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 26.690_230_724_521_093) < 1.0e-5);
    assert!(result.friction_length.expect("a friction length") > 0.0);
    assert!(result.exit_mach.expect("an exit Mach") > 0.0);
    assert_eq!(result.stations.len(), 4);
}

#[test]
fn the_short_pipe_passes_more() {
    let long = pipe(5_000_000.0, 101_325.0, 0.1, 100.0, 0.02)
        .mass_flow_rate
        .unwrap()
        .value;
    let short = pipe(5_000_000.0, 101_325.0, 0.1, 10.0, 0.01)
        .mass_flow_rate
        .unwrap()
        .value;
    assert!(short > long);
    assert!(relative(short, 68.905_927_815_126_73) < 1.0e-5);
}

#[test]
fn the_high_back_pressure_is_unchoked() {
    let result = pipe(5_000_000.0, 4_500_000.0, 0.1, 100.0, 0.02);
    assert!(!result.choked);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 12.467_507_577_150_702) < 1.0e-5);
}

#[test]
fn the_nasa_case_matches() {
    let result = ideal_gas_fanno_release(
        &["nitrogen".to_string()],
        &[1.0],
        kelvins(299.816_666_666_7),
        pascals(344_737.864_658_4),
        meters(0.1524),
        1.0,
        pascals(101_325.0),
        81.4578,
        0.002,
    )
    .expect("the state runs");
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 10.783_603_252_875_281) < 1.0e-5);
}

#[test]
fn a_zero_path_is_unsupported() {
    let result = ideal_gas_fanno_release(
        &["nitrogen".to_string()],
        &[1.0],
        kelvins(300.0),
        pascals(5_000_000.0),
        meters(0.1),
        1.0,
        pascals(101_325.0),
        0.0,
        0.0,
    )
    .expect("the state runs");
    assert_eq!(result.status, "unsupported");
    assert_eq!(result.diagnostic_codes, vec!["UNSUPPORTED_FANNO_REGIME"]);
}

#[test]
fn a_receiving_pressure_above_the_upstream_is_a_zero() {
    let result = pipe(5_000_000.0, 5_100_000.0, 0.1, 100.0, 0.02);
    assert_eq!(result.mass_flow_rate.expect("a true zero").value, 0.0);
    assert!(!result.choked);
    assert_eq!(result.stations.len(), 3);
}
