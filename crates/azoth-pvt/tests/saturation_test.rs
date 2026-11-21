//! `pvt.saturation_test` against its capture.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{kelvins, pascals};
use azoth_pvt::{SaturationTestResult, saturation_test};

fn binary(first: &str, first_moles: f64, second: &str, second_moles: f64) -> Vec<FluidComponent> {
    vec![
        FluidComponent::base(first, first_moles),
        FluidComponent::base(second, second_moles),
    ]
}

fn pressure(components: &[FluidComponent], t: f64) -> SaturationTestResult {
    saturation_test(
        components,
        "pressure",
        None,
        None,
        Some(kelvins(t)),
        None,
        None,
        None,
    )
    .unwrap()
}

fn temperature(components: &[FluidComponent], p: f64) -> SaturationTestResult {
    saturation_test(
        components,
        "temperature",
        None,
        None,
        None,
        Some(pascals(p)),
        None,
        None,
    )
    .unwrap()
}

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance * expected.abs().max(1.0)
}

#[test]
fn the_upper_saturation_pressures_match_the_capture() {
    let fluid = binary("methane", 0.70, "nC10", 0.30);
    let row = pressure(&fluid, 323.15);
    assert!(
        close(
            row.saturation_pressure.unwrap(),
            2.895_519_599_914_550_8e7,
            1.0e-6
        ),
        "methane/nC10 at 323.15 K: {:?}",
        row.saturation_pressure
    );

    let fluid = binary("methane", 0.95, "n-butane", 0.05);
    let row = pressure(&fluid, 275.0);
    assert!(
        close(
            row.saturation_pressure.unwrap(),
            7_495_956.039_428_711,
            1.0e-6
        ),
        "methane/n-butane at 275 K: {:?}",
        row.saturation_pressure
    );
}

#[test]
fn the_upper_saturation_temperature_matches_the_capture() {
    let fluid = binary("methane", 0.70, "nC10", 0.30);
    let row = temperature(&fluid, 5.21e6);
    assert!(
        close(
            row.saturation_temperature.unwrap(),
            557.124_538_421_630_9,
            1.0e-6
        ),
        "methane/nC10 at 52.1 bar: {:?}",
        row.saturation_temperature
    );
}

#[test]
fn an_unbracketed_search_answers_the_grid_top_with_a_warning() {
    let fluid = vec![FluidComponent::base("methane", 1.0)];
    let row = pressure(&fluid, 300.0);
    assert_eq!(row.saturation_pressure, Some(1.0e8));
    assert_eq!(row.warnings.len(), 1);

    let row = temperature(&fluid, 1.0e7);
    assert_eq!(row.saturation_temperature, Some(1200.0));
    assert_eq!(row.warnings.len(), 1);
}

#[test]
fn the_bounds_belong_to_the_temperature_search_only() {
    let fluid = vec![FluidComponent::base("methane", 1.0)];
    let err = saturation_test(
        &fluid,
        "pressure",
        None,
        None,
        Some(kelvins(300.0)),
        None,
        Some(kelvins(200.0)),
        Some(kelvins(400.0)),
    )
    .expect_err("a pressure search has no temperature bounds");
    assert!(err.to_string().contains("pressure search"));

    let err = saturation_test(
        &fluid,
        "temperature",
        None,
        None,
        None,
        Some(pascals(5.0e6)),
        Some(kelvins(200.0)),
        None,
    )
    .expect_err("one bound is not a bound pair");
    assert!(err.to_string().contains("together"));

    let err = saturation_test(
        &fluid,
        "temperature",
        None,
        None,
        None,
        Some(pascals(5.0e6)),
        Some(kelvins(20.0)),
        Some(kelvins(400.0)),
    )
    .expect_err("the lower bound is below the class grid");
    assert!(err.to_string().contains("30"));
}
