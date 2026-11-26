//! `pvt.separator_path` at its boundary.
//!
//! The states are the pin's, from `validation/neqsim/captures/pvt_separator_path.tsv`.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{kelvins, pascals};
use azoth_pvt::{SeparatorPathResult, separator_path};

fn sample() -> Vec<FluidComponent> {
    vec![
        FluidComponent::base("methane", 70.0),
        FluidComponent::base("ethane", 10.0),
        FluidComponent::base("propane", 5.0),
        FluidComponent::base("nC10", 15.0),
    ]
}

fn relative(actual: f64, expected: f64) -> f64 {
    (actual / expected - 1.0).abs()
}

#[test]
fn the_one_stage_stock_tank_is_the_capture() {
    let result: SeparatorPathResult = separator_path(
        &sample(),
        &[pascals(101_325.0)],
        &[kelvins(288.15)],
        Some("pr"),
        Some("classic"),
        None,
        None,
    )
    .unwrap();
    assert_eq!(result.final_pressure, 101_325.0);
    assert_eq!(result.final_temperature, 288.15);
    assert!(relative(result.volumes[0], 1.998425076249661) < 1.0e-6);
    assert!(relative(result.volumes[1], 0.002925770637213811) < 1.0e-6);
    assert_eq!(result.volumes[2], 0.0);
    assert!(relative(result.gas_oil_ratio.unwrap(), 683.0422900657537) < 1.0e-6);
    assert!(
        (result.component_moles[0][0]
            + result.component_moles[1][0]
            + result.component_moles[2][0]
            - 70.0)
            .abs()
            < 1.0e-6
    );
}

#[test]
fn the_two_stage_path_retains_the_offgas_condensate() {
    let result = separator_path(
        &sample(),
        &[pascals(2_000_000.0), pascals(101_325.0)],
        &[kelvins(400.0), kelvins(288.15)],
        Some("pr"),
        Some("classic"),
        None,
        None,
    )
    .unwrap();
    assert!(relative(result.volumes[0], 1.9949580169411532) < 1.0e-6);
    assert!(relative(result.volumes[1], 0.0029378272171125736) < 1.0e-6);
    assert!(relative(result.gas_oil_ratio.unwrap(), 679.059001605236) < 1.0e-6);
}

#[test]
fn the_three_phase_path_accounts_for_water() {
    let mut rows = sample();
    rows.push(FluidComponent::base("water", 10.0));
    let result = separator_path(
        &rows,
        &[pascals(1_000_000.0), pascals(101_325.0)],
        &[kelvins(310.0), kelvins(288.15)],
        Some("pr"),
        Some("classic"),
        None,
        Some(true),
    )
    .unwrap();
    assert!(result.volumes.iter().all(|volume| *volume > 0.0));
    assert!(
        (result.component_moles[0][4]
            + result.component_moles[1][4]
            + result.component_moles[2][4]
            - 10.0)
            .abs()
            < 1.0e-6
    );
}

#[test]
fn a_gas_only_path_publishes_the_ratio_absent() {
    let result = separator_path(
        &[FluidComponent::base("methane", 100.0)],
        &[pascals(101_325.0)],
        &[kelvins(288.15)],
        Some("pr"),
        Some("classic"),
        None,
        None,
    )
    .unwrap();
    assert_eq!(result.volumes[1], 0.0);
    assert!(result.gas_oil_ratio.is_none());
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn a_declaration_that_cannot_be_a_path_is_refused() {
    let increasing = separator_path(
        &sample(),
        &[pascals(1_000_000.0), pascals(2_000_000.0)],
        &[kelvins(300.0), kelvins(288.15)],
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(increasing.to_string().contains("strictly decreasing"));

    let unequal = separator_path(
        &sample(),
        &[pascals(1_000_000.0)],
        &[kelvins(300.0), kelvins(288.15)],
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(unequal.to_string().contains("equal nonempty"));

    let nonfinite = separator_path(
        &sample(),
        &[pascals(1_000_000.0)],
        &[kelvins(f64::NAN)],
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(nonfinite.to_string().contains("strictly decreasing"));
}
