//! `process.bubble_transfer_zone_release` against the states its cases state.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::{BubbleTransferZoneReleaseResult, bubble_transfer_zone_release};

fn zone(fraction: f64, vertical: f64, diffusivities: &[f64]) -> BubbleTransferZoneReleaseResult {
    bubble_transfer_zone_release(
        &["methane".to_string(), "n-heptane".to_string()],
        &[fraction, 1.0 - fraction],
        kelvins(300.0),
        pascals(3_000_000.0),
        meters(0.01),
        0.62,
        pascals(100_000.0),
        0.02,
        0.1,
        vertical,
        0.001,
        diffusivities,
        "synthetic-transfer-zone:v1",
    )
    .expect("the state runs")
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs()
}

#[test]
fn the_short_zone_is_at_the_captured_rate() {
    let result = zone(0.1125, 0.5, &[1.0e-8, 2.0e-8]);
    assert_eq!(result.status, "valid");
    let rate = result.mass_flow_rate.expect("a rate").value;
    assert!(relative(rate, 1.717_522_271_728_852_7) < 5.0e-5);
    assert!(
        result
            .diagnostic_codes
            .iter()
            .any(|code| code == "PREDICTIVE_BUBBLE_TRANSFER_ZONE")
    );
}

#[test]
fn the_long_zone_saturates() {
    let short = zone(0.1125, 0.5, &[1.0e-8, 2.0e-8])
        .mass_flow_rate
        .unwrap()
        .value;
    let long = zone(0.1125, 2.0, &[1.0e-8, 2.0e-8])
        .mass_flow_rate
        .unwrap()
        .value;
    assert!(relative(short, long) < 1.0e-6);
}

#[test]
fn the_upper_window_state_is_used() {
    let rate = zone(0.125, 0.5, &[1.0e-8, 2.0e-8])
        .mass_flow_rate
        .unwrap()
        .value;
    assert!(relative(rate, 1.614_618_773_654_917_1) < 5.0e-5);
}

#[test]
fn the_delegate_range_refusal_is_prefixed() {
    let result = zone(0.1125, 0.5, &[1.0e-10, 2.0e-10]);
    assert_eq!(result.status, "unsupported");
    assert_eq!(
        result.diagnostic_codes,
        vec!["TRANSFER_ZONE_RANZ_MARSHALL_RANGE_UNSUPPORTED"]
    );
    assert!(result.stations.is_empty());
}
