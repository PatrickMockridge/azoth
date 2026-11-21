//! The laboratory experiments' fluid and flash machinery.

use azoth_characterization::KijSource;
use azoth_characterization::component_row::FluidComponent;
use azoth_eos::Cubic;
use azoth_pvt::flash::FlashMode;
use azoth_pvt::fluid::Fluid;
use azoth_pvt::volume::volume_shift;

fn oil() -> Vec<FluidComponent> {
    vec![
        FluidComponent::base("methane", 0.2),
        FluidComponent::base("nC10", 0.8),
    ]
}

#[test]
fn the_union_merges_a_gas_component_into_the_oils_row() {
    let gas = vec![FluidComponent::base("methane", 1.0)];
    let (fluid, template) =
        Fluid::from_rows_pair(&oil(), &gas, Cubic::Srk, KijSource::Database, true).unwrap();
    assert_eq!(fluid.moles(), &[0.2, 0.8]);
    assert_eq!(template.amounts, vec![1.0, 0.0]);
    let swelled = fluid.with_added(&template, 0.1);
    assert!((swelled.moles()[0] - 0.3).abs() < 1e-15);
    assert!((swelled.moles()[1] - 0.8).abs() < 1e-15);
}

#[test]
fn the_volume_shift_is_the_srk_one_for_an_srk_fluid() {
    let fluid = Fluid::from_rows(&oil(), Cubic::Srk, KijSource::Database, true).unwrap();
    let x = fluid.z();
    let shift = volume_shift(&fluid.mixture, &x);
    let expected: f64 = x
        .iter()
        .zip(fluid.mixture.components())
        .map(|(fraction, component)| fraction * component.volume_shift_srk)
        .sum();
    assert!((shift - expected).abs() < 1e-15);
    let pr: f64 = x
        .iter()
        .zip(fluid.mixture.components())
        .map(|(fraction, component)| fraction * component.volume_shift)
        .sum();
    assert!((shift - pr).abs() > 1e-12);
}

#[test]
fn the_flash_orders_its_phases_gas_first() {
    let fluid = Fluid::from_rows(&oil(), Cubic::Srk, KijSource::Database, true).unwrap();
    let flash = fluid.flash(323.15, 5.0e6, FlashMode::TwoPhase).unwrap();
    assert!(flash.gas().is_some());
    assert!(flash.oil().is_some());
    assert_eq!(
        flash.phases[0].label,
        azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Gas
    );
    assert!((flash.phases[0].fraction + flash.phases[1].fraction - 1.0).abs() < 1e-12);
}

#[test]
fn a_two_fluid_row_list_is_refused() {
    let mut second = FluidComponent::base("nC10", 1.0);
    second.fluid = 1.0;
    let err = Fluid::from_rows(
        &[FluidComponent::base("methane", 1.0), second],
        Cubic::Srk,
        KijSource::Database,
        true,
    )
    .expect_err("two fluids are not one id's input");
    assert!(err.to_string().contains("more than one fluid"));
}
