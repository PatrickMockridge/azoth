//! `process.ideal_gas_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::models::hem_release::ReleaseStation;
use azoth_process::{IdealGasReleaseResult, ideal_gas_release};

fn pure(component: &str, t: f64, p: f64, d: f64, cd: f64, back: f64) -> IdealGasReleaseResult {
    ideal_gas_release(
        &[component.to_string()],
        &[1.0],
        kelvins(t),
        pascals(p),
        meters(d),
        cd,
        pascals(back),
    )
    .expect("the state runs")
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs()
}

#[test]
fn methane_is_choked_at_the_captured_rate() {
    let result = pure("methane", 300.0, 5_000_000.0, 0.01, 0.75, 101_325.0);
    assert_eq!(result.status, "valid");
    assert!(result.choked);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 0.520_763_925_419_132) < 2.0e-5);
    let sound = result.throat_sound_speed.expect("a sound speed").value;
    assert!(relative(sound, 430.522_529_132_446_8) < 2.0e-5);
    assert_eq!(result.stations.len(), 4);
    assert_eq!(
        result.stations[0].station,
        ReleaseStation::UpstreamStagnation
    );
    assert_eq!(result.stations[1].station, ReleaseStation::ThroatCritical);
    assert_eq!(result.stations[2].station, ReleaseStation::OrificeExit);
    assert_eq!(result.stations[3].station, ReleaseStation::AmbientExpanded);
}

#[test]
fn a_receiving_pressure_at_the_upstream_is_a_valid_zero() {
    let result = pure("methane", 300.0, 500_000.0, 0.01, 0.62, 500_000.0);
    assert_eq!(result.status, "valid");
    assert_eq!(result.mass_flow_rate.expect("a true zero").value, 0.0);
    assert!(!result.choked);
    assert!(result.throat_sound_speed.is_none());
    assert_eq!(result.stations.len(), 3);
    assert!(
        !result
            .stations
            .iter()
            .any(|row| row.station == ReleaseStation::AmbientExpanded)
    );
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "NO_FORWARD_FLOW")
    );
}

#[test]
fn an_unchoked_flow_aliases_the_ambient_to_the_throat() {
    let result = pure("methane", 300.0, 500_000.0, 0.01, 0.62, 400_000.0);
    assert!(!result.choked);
    let throat = &result.stations[1];
    let ambient = &result.stations[3];
    assert_eq!(throat.pressure.value, ambient.pressure.value);
    assert_eq!(throat.velocity.value, ambient.velocity.value);
    assert!(result.throat_sound_speed.is_some());
}

#[test]
fn a_two_phase_feed_is_unsupported() {
    let result = pure("propane", 230.0, 1_000_000.0, 0.01, 0.62, 101_325.0);
    assert_eq!(result.status, "unsupported");
    assert!(result.mass_flow_rate.is_none());
    assert!(result.stations.is_empty());
    assert_eq!(
        result.diagnostic_codes,
        vec!["UNSUPPORTED_IDEAL_GAS_REGIME"]
    );
}

#[test]
fn the_rate_scales_with_the_effective_area() {
    let base = pure("methane", 300.0, 5_000_000.0, 0.01, 0.62, 101_325.0);
    let doubled = pure("methane", 300.0, 5_000_000.0, 0.02, 0.62, 101_325.0);
    let ratio = doubled.mass_flow_rate.unwrap().value / base.mass_flow_rate.unwrap().value;
    assert!((ratio - 4.0).abs() < 1.0e-12);
}
