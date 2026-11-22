//! `pvt.property_sweep` against its capture.

use azoth_characterization::component_row::FluidComponent;
use azoth_pvt::{PropertySweepResult, property_sweep};

fn binary() -> Vec<FluidComponent> {
    vec![
        FluidComponent::base("methane", 0.70),
        FluidComponent::base("nC10", 0.30),
    ]
}

fn methane() -> Vec<FluidComponent> {
    vec![FluidComponent::base("methane", 1.0)]
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-6 * expected.abs().max(1.0)
}

fn call(
    components: &[FluidComponent],
    mode: &str,
    temperatures: &[f64],
    pressures: &[f64],
) -> PropertySweepResult {
    property_sweep(components, mode, temperatures, pressures, None, None, None).unwrap()
}

#[test]
fn the_density_sweep_matches_the_capture() {
    let row = call(
        &binary(),
        "density",
        &[323.15, 313.15, 303.15],
        &[5.0e5, 5.0e5, 5.0e5],
    );
    assert!(close(
        row.points[0].gas_density.unwrap(),
        3.050_545_344_825_013
    ));
    assert!(close(
        row.points[0].oil_density.unwrap(),
        726.594_134_793_407_3
    ));
    assert!(row.points[0].aqueous_density.is_none());
    assert!(row.points[0].gas_viscosity.is_none());
    assert!(row.points[1].oil_density.unwrap() > row.points[0].oil_density.unwrap());
    assert!(row.points[2].gas_phase_available);
    assert!(row.points[2].oil_phase_available);

    let gas = call(
        &methane(),
        "density",
        &[300.0, 290.0, 280.0],
        &[5.0e5, 5.0e5, 5.0e5],
    );
    assert!(gas.points[0].gas_density.unwrap() > 1.0);
    assert!(gas.points[0].oil_density.is_none());
    assert!(gas.points[0].gas_phase_available);
    assert!(!gas.points[0].oil_phase_available);
}

#[test]
fn the_viscosity_sweep_matches_the_capture() {
    let row = call(
        &binary(),
        "viscosity",
        &[323.15, 313.15, 303.15],
        &[5.0e5, 5.0e5, 5.0e5],
    );
    assert!(close(
        row.points[0].gas_viscosity.unwrap(),
        1.203_786_083_340_327_5e-5
    ));
    assert!(close(
        row.points[0].oil_viscosity.unwrap(),
        6.239_054_659_190_214e-4
    ));
    assert!(row.points[0].gas_density.is_none());

    let gas = call(
        &methane(),
        "viscosity",
        &[300.0, 290.0, 280.0],
        &[5.0e5, 5.0e5, 5.0e5],
    );
    assert!(close(
        gas.points[0].gas_viscosity.unwrap(),
        1.132_123_749_468_799_2e-5
    ));
    assert!(gas.points[0].oil_viscosity.is_none());
    assert!(gas.points[0].gas_phase_available);
    assert!(!gas.points[0].oil_phase_available);
}

#[test]
fn the_wax_modes_are_owed_and_the_grid_is_checked() {
    let err = property_sweep(
        &binary(),
        "wax_fraction",
        &[300.0],
        &[5.0e5],
        None,
        None,
        None,
    )
    .expect_err("the wax modes are owed with the TBP wax model");
    assert!(err.to_string().contains("owed"));

    let err = property_sweep(
        &binary(),
        "density",
        &[300.0, 290.0],
        &[5.0e5],
        None,
        None,
        None,
    )
    .expect_err("the schedule's arrays are parallel");
    assert!(err.to_string().contains("2 temperatures and 1 pressures"));
}
