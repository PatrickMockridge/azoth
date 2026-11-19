//! `process.prescribed_slip_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::{PrescribedSlipReleaseResult, hem_release, prescribed_slip_release};

fn slip(ratio: f64) -> PrescribedSlipReleaseResult {
    prescribed_slip_release(
        &["methane".to_string(), "propane".to_string()],
        &[0.5, 0.5],
        kelvins(270.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        ratio,
    )
    .expect("the state runs")
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs()
}

#[test]
fn the_partition_is_choked_at_the_captured_rate() {
    let result = slip(2.0);
    assert_eq!(result.status, "valid");
    assert!(result.choked);
    assert!(result.throat_sound_speed.is_none());
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 0.596_051_120_535_030_2) < 2.0e-5);
    let throat = &result.stations[1];
    assert!(relative(throat.gas_velocity.unwrap().value, 254.779_585_957_156_48) < 2.0e-5);
    assert!(
        relative(
            throat.liquid_velocity.unwrap().value,
            127.389_792_978_578_24
        ) < 2.0e-5
    );
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "PRESCRIBED_PHASE_SLIP")
    );
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "SLIP_CLOSURE")
    );
}

#[test]
fn a_unit_ratio_recovers_the_homogeneous_rate() {
    let base = hem_release(
        &["methane".to_string(), "propane".to_string()],
        &[0.5, 0.5],
        kelvins(270.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
    )
    .expect("the base runs");
    let unity = slip(1.0);
    let base_rate = base.mass_flow_rate.expect("a base rate").value;
    let unity_rate = unity.mass_flow_rate.expect("a unity rate").value;
    assert!(relative(unity_rate, base_rate) < 1.0e-12);
}

#[test]
fn a_single_phase_opening_is_unsupported() {
    let result = prescribed_slip_release(
        &["methane".to_string()],
        &[1.0],
        kelvins(300.0),
        pascals(2_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        2.0,
    )
    .expect("the state runs");
    assert_eq!(result.status, "unsupported");
    assert_eq!(result.diagnostic_codes, vec!["SLIP_REGIME_UNSUPPORTED"]);
    assert!(result.stations.is_empty());
}

#[test]
fn the_slip_ratio_below_one_is_refused() {
    let refused = prescribed_slip_release(
        &["methane".to_string(), "propane".to_string()],
        &[0.5, 0.5],
        kelvins(270.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        0.99,
    );
    assert!(refused.is_err());
}

#[test]
fn the_rate_rises_with_the_slip_ratio() {
    let one = slip(1.0).mass_flow_rate.unwrap().value;
    let two = slip(2.0).mass_flow_rate.unwrap().value;
    let four = slip(4.0).mass_flow_rate.unwrap().value;
    assert!(one < two && two < four);
    assert!(relative(four, 0.607_564_856_649_622_8) < 2.0e-5);
}
