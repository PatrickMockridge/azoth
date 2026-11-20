//! `process.legacy_screening_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::models::hem_release::ReleaseStation;
use azoth_process::{LegacyScreeningReleaseResult, legacy_screening_release};

fn pure(
    component: &str,
    t: f64,
    p: f64,
    d: f64,
    cd: f64,
    back: f64,
) -> LegacyScreeningReleaseResult {
    legacy_screening_release(
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
    let result = pure("methane", 300.0, 5_000_000.0, 0.01, 0.62, 101_325.0);
    assert_eq!(result.status, "valid_with_warnings");
    assert!(result.choked);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 0.430_510_155_343_228_2) < 2.0e-5);
    assert_eq!(result.stations.len(), 3);
    assert_eq!(
        result.stations[0].station,
        ReleaseStation::UpstreamStagnation
    );
    assert_eq!(result.stations[1].station, ReleaseStation::ThroatCritical);
    assert_eq!(result.stations[2].station, ReleaseStation::OrificeExit);
    assert!(result.throat_sound_speed.is_none());
    assert_eq!(result.stations[0].velocity.value, 0.0);
    assert!(result.stations[1].velocity.value > 0.0);
}

#[test]
fn the_opening_aliases_the_upstream_state() {
    let result = pure("methane", 300.0, 5_000_000.0, 0.01, 0.62, 101_325.0);
    let upstream = &result.stations[0];
    let throat = &result.stations[1];
    assert_eq!(upstream.pressure.value, throat.pressure.value);
    assert_eq!(upstream.temperature.value, throat.temperature.value);
    assert_eq!(upstream.density.value, throat.density.value);
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "SCREENING_ONLY")
    );
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "UNRESOLVED_STATIONS")
    );
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "LEGACY_FALLBACK_POLICY")
    );
}

#[test]
fn the_subcritical_branch_is_unchoked() {
    let result = pure("methane", 300.0, 500_000.0, 0.01, 0.62, 400_000.0);
    assert!(!result.choked);
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 0.039_053_388_805_444_796) < 2.0e-5);
}

#[test]
fn a_receiving_pressure_at_the_upstream_is_a_valid_zero() {
    let result = pure("methane", 300.0, 500_000.0, 0.01, 0.62, 600_000.0);
    assert_eq!(result.status, "valid_with_warnings");
    assert_eq!(result.mass_flow_rate.expect("a true zero").value, 0.0);
    assert!(!result.choked);
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "NO_FORWARD_FLOW_OR_SCREENING_ZERO")
    );
}

#[test]
fn a_liquid_feed_keeps_its_phase() {
    let result = pure("propane", 300.0, 1_100_000.0, 0.01, 0.62, 101_325.0);
    assert_eq!(result.status, "valid_with_warnings");
    assert!(result.stations[0].gas_mass_fraction < 1.0e-12);
    assert!(result.stations[0].liquid_density.is_some());
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 0.162_016_290_242_578_55) < 2.0e-5);
}
