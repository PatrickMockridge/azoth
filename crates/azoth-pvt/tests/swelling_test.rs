//! `pvt.swelling_test` against its capture.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::kelvins;
use azoth_pvt::{SwellingTestResult, swelling_test};

fn rows(names: &[(&str, f64)]) -> Vec<FluidComponent> {
    names
        .iter()
        .map(|(name, moles)| FluidComponent::base(*name, *moles))
        .collect()
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-6 * expected.abs().max(1.0)
}

#[test]
fn the_upstream_pinned_case_matches_the_capture() {
    let oil = rows(&[("methane", 0.2), ("nC10", 0.8)]);
    let gas = rows(&[("methane", 1.0)]);
    let row: SwellingTestResult = swelling_test(
        &oil,
        Some(&gas),
        None,
        Some("zero"),
        Some(kelvins(353.15)),
        &[0.0, 0.10, 0.20],
        Some(false),
    )
    .unwrap();
    assert!(close(row.original_oil_moles, 1.0));
    assert!(close(row.points[0].pressure, 5_128_370.972_140_508));
    assert!(close(row.points[0].relative_oil_volume, 1.0));
    assert!(close(row.points[1].pressure, 7_320_109.921_488_98));
    assert!(close(row.points[1].relative_oil_volume, 1.030_102_097_142_687_7));
    assert!(close(row.points[2].pressure, 9_314_150.739_597_373));
    assert!(close(row.points[2].relative_oil_volume, 1.060_231_741_573_248_5));
}

#[test]
fn the_mixed_gas_and_zero_injection_match_their_capture_blocks() {
    let oil = rows(&[("methane", 0.5), ("nC10", 0.5)]);
    let gas = rows(&[("methane", 0.7), ("ethane", 0.3)]);
    let row = swelling_test(
        &oil,
        Some(&gas),
        None,
        Some("zero"),
        Some(kelvins(323.15)),
        &[0.0, 0.05, 0.15, 0.25],
        Some(false),
    )
    .unwrap();
    assert!(close(row.points[0].pressure, 1.441_124_803_534_498_3e7));
    assert!(close(row.points[1].pressure, 1.497_556_075_131_841_2e7));
    assert!(close(row.points[3].relative_oil_volume, 1.111_512_321_716_036_3));

    let zero = swelling_test(
        &oil,
        None,
        None,
        Some("zero"),
        Some(kelvins(323.15)),
        &[0.0, 0.0],
        Some(false),
    )
    .unwrap();
    assert!(close(zero.points[0].pressure, 1.441_124_803_534_498_3e7));
    assert!(close(zero.points[1].relative_oil_volume, 1.0));
}

#[test]
fn a_schedule_that_decreases_or_injects_without_a_gas_is_refused() {
    let oil = rows(&[("methane", 0.5), ("nC10", 0.5)]);
    let err = swelling_test(
        &oil,
        None,
        None,
        None,
        Some(kelvins(323.15)),
        &[0.1, 0.05],
        None,
    )
    .expect_err("a cumulative schedule cannot decrease");
    assert!(err.to_string().contains("cannot decrease"));

    let err = swelling_test(
        &oil,
        None,
        None,
        None,
        Some(kelvins(323.15)),
        &[0.0, 0.1],
        None,
    )
    .expect_err("an injection needs a gas");
    assert!(err.to_string().contains("no injection gas"));
}
