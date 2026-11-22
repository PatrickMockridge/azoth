//! `pvt.mmp` against its capture.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{kelvins, pascals};
use azoth_pvt::{MmpResult, mmp};

fn rows(names: &[(&str, f64)]) -> Vec<FluidComponent> {
    names
        .iter()
        .map(|(name, moles)| FluidComponent::base(*name, *moles))
        .collect()
}

fn oil() -> Vec<FluidComponent> {
    rows(&[("methane", 0.70), ("nC10", 0.30)])
}

fn gas() -> Vec<FluidComponent> {
    rows(&[("methane", 1.0)])
}

#[allow(clippy::too_many_arguments)]
fn call(
    method: &str,
    min_bar: f64,
    max_bar: f64,
    points: usize,
    nodes: usize,
    pv: f64,
    threshold: f64,
) -> MmpResult {
    mmp(
        &oil(),
        &gas(),
        None,
        None,
        kelvins(323.15),
        Some(method),
        Some(threshold),
        Some(points),
        Some(pascals(min_bar * 1.0e5)),
        Some(pascals(max_bar * 1.0e5)),
        Some(nodes),
        Some(pv),
    )
    .unwrap()
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-6 * expected.abs().max(1.0)
}

/// The key tie line's 0.5-bar bisection walks a tie-line length the two flashes measure a
/// little differently, so its crossing is held at the measured 3e-2.
fn within(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance * expected.abs().max(1.0)
}

#[test]
fn the_slim_tube_and_rising_bubble_match_the_capture() {
    let slim = call("slim_tube", 100.0, 300.0, 3, 6, 0.4, 0.90);
    assert!(slim.mmp.is_none());
    assert_eq!(slim.mechanism, "immiscible");
    assert_eq!(slim.points.len(), 3);
    assert!(close(slim.points[0].recovery, 0.091_230_782_449_553_4));
    assert!(close(slim.points[1].recovery, 0.135_847_394_311_698_18));
    assert!(close(slim.points[2].recovery, 0.2));

    let rising = call("rising_bubble", 100.0, 300.0, 3, 6, 0.4, 0.90);
    assert_eq!(rising.mmp, slim.mmp);
    assert_eq!(rising.mechanism, slim.mechanism);
    for (left, right) in rising.points.iter().zip(&slim.points) {
        assert!(close(left.recovery, right.recovery));
    }
}

#[test]
fn the_key_tie_line_matches_the_capture() {
    let row = call("key_tie_line", 100.0, 500.0, 3, 6, 0.4, 0.90);
    assert!(within(row.mmp.unwrap(), 4.001_953_125e7, 3.0e-2));
    assert_eq!(row.mechanism, "combined");
    assert!(row.points.is_empty());
}

#[test]
fn the_wide_slim_tube_finds_an_mmp() {
    let row = call("slim_tube", 100.0, 1000.0, 5, 6, 0.4, 0.50);
    assert!(close(row.mmp.unwrap(), 4.093_75e7));
    assert_eq!(row.mechanism, "combined");
    assert_eq!(row.points.len(), 5);
    assert!(close(row.points[1].recovery, 0.2));
    assert!(close(row.points[2].recovery, 1.0));
}

#[test]
fn a_gas_without_a_shared_component_is_refused() {
    let err = mmp(
        &oil(),
        &rows(&[("CO2", 1.0)]),
        None,
        None,
        kelvins(323.15),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect_err("the oil carries no CO2");
    assert!(err.to_string().contains("not a component of the oil"));
}
