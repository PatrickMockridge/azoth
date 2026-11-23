//! The tuner, against `pvt_tune_to_saturation.tsv`'s two blocks.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{kelvins, pascals};
use azoth_pvt::tune_to_saturation;

/// The class's own base saturation on the capture's fluid.
const CLASS_BASE_SATURATION_PA: f64 = 46_441_392.517_089_844;
/// This port's base saturation: 1265 Pa below the class's, because the lumps' cards are
/// re-correlated from each lump's molar mass and density rather than mass-averaged over the
/// cuts. The early return is held on this port's own base.
const PORT_BASE_SATURATION_PA: f64 = 46_440_126.991_271_97;

fn plus() -> FluidComponent {
    FluidComponent {
        fluid: 0.0,
        name: "C11_PC".to_owned(),
        pseudo_component: true,
        moles: 0.1,
        molar_mass: Some(0.2562),
        density: Some(839.8),
        normal_boiling_point: Some(590.595_456_656_000_1),
        critical_temperature: Some(718.284_068_477_579_6),
        critical_pressure: Some(1_538_175.288_664_231_8),
        acentric_factor: Some(0.888_879_164_023_810_3),
        critical_volume: Some(0.000_649_550_881_261_717_9),
        racket_z: Some(0.260_437_583_460_600_14),
        racket_z_cpa: Some(0.260_437_583_460_600_14),
        parachor: Some(658.808),
        critical_viscosity: Some(2.629_369_743_222_56e-05),
        triple_point_temperature: Some(302.469_391_002_341_9),
        heat_of_fusion: Some(46_266.210_417_662_005),
        ideal_gas_enthalpy_of_formation: Some(-422_284.12),
        cp_a: Some(12.061_362_782_895_609),
        cp_b: Some(1.461_396_729_616_855_7),
        cp_c: Some(-0.000_579_542_540_458_157_2),
        cp_d: Some(0.0),
        attractive_m: Some(1.740_037_118_563_999_8),
    }
}

fn rows() -> [FluidComponent; 3] {
    [
        FluidComponent::base("methane", 0.8),
        FluidComponent::base("nC10", 0.1),
        plus(),
    ]
}

#[test]
fn the_early_return_takes_no_step() {
    let result = tune_to_saturation(
        &rows(),
        "C11_PC",
        kelvins(350.0),
        pascals(PORT_BASE_SATURATION_PA + 300.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("the tuner runs");
    assert_eq!(result.iterations, 0.0);
    assert!(result.converged);
    assert_eq!(result.tuned_molar_mass, 0.2562);
    // The no-step answer is the port's own base state, one construction of the class's fluid.
    let relative = (result.final_saturation_pressure - PORT_BASE_SATURATION_PA).abs()
        / PORT_BASE_SATURATION_PA;
    assert!(relative < 1e-9, "{relative}: {result:?}");
}

#[test]
fn the_loop_takes_the_capture_path() {
    let result = tune_to_saturation(
        &rows(),
        "C11_PC",
        kelvins(350.0),
        pascals(CLASS_BASE_SATURATION_PA + 2000.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("the tuner runs");
    assert!(result.converged, "{result:?}");
    assert!(
        result.iterations > 0.0 && result.iterations < 50.0,
        "{result:?}"
    );
    // The class's own run answers 0.2562238822937013 kg/mol after 17 steps.
    assert!(
        (result.tuned_molar_mass - 0.256_223_882_293_701_3).abs() < 1e-4,
        "{result:?}"
    );
}
