//! `pvt.differential_liberation` against its capture.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::kelvins;
use azoth_pvt::{DifferentialLiberationResult, differential_liberation};

fn binary(first: &str, first_moles: f64, second: &str, second_moles: f64) -> Vec<FluidComponent> {
    vec![
        FluidComponent::base(first, first_moles),
        FluidComponent::base(second, second_moles),
    ]
}

fn call(components: &[FluidComponent], t: f64, pressures: &[f64]) -> DifferentialLiberationResult {
    differential_liberation(components, kelvins(t), pressures, None, None, None).unwrap()
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-6 * expected.abs().max(1.0)
}

fn loose(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-4 * expected.abs().max(1.0)
}

const PRESSURES: [f64; 10] = [
    250.0e5, 200.0e5, 150.0e5, 100.0e5, 70.0e5, 50.0e5, 30.0e5, 20.0e5, 10.0e5, 1.0e5,
];

#[test]
fn the_first_points_match_the_capture() {
    let row = call(&binary("methane", 0.70, "nC10", 0.30), 323.15, &PRESSURES);
    assert!(loose(
        row.saturation_pressure.unwrap(),
        2.895_506_343_841_552_7e7
    ));
    // The class's saturation volume is phase 0 at its midpoint; ours is the material phase,
    // and at this state they are the same phase.
    assert!(loose(
        row.saturation_volume.unwrap(),
        9.813_810_850_855_022e-5
    ));
    assert!(close(row.voil_std.unwrap(), 5.609_020_215_003_901e-5));

    let point = &row.points[0];
    assert!(close(point.total_volume, 1.048_025_541_889_866_8e-4));
    assert!(loose(
        point.relative_volume.unwrap(),
        1.067_908_845_826_754_7
    ));
    assert!(close(point.liquid_volume, 8.857_643_562_217_553e-5));
    assert!(loose(
        point.liquid_volume_relative_to_vsat.unwrap(),
        0.902_569_215_652_432_9
    ));
    assert!(close(point.oil_density, 576.418_063_870_379_1));
    assert!(close(point.gas_volume.unwrap(), 1.622_611_856_681_117_8e-5));
    assert!(close(
        point.gas_standard_volume.unwrap(),
        0.003_897_553_363_742_691_6
    ));
    assert!(close(point.z_gas.unwrap(), 0.915_926_100_421_117_1));
    assert!(close(
        point.relative_gas_gravity.unwrap(),
        0.598_698_705_521_173
    ));
    assert!(close(point.bo.unwrap(), 1.579_178_398_844_724_6));
    assert!(close(point.bg.unwrap(), 0.004_163_154_946_833_049));
    assert!(close(point.rs.unwrap(), 226.424_429_346_130_9));
}

#[test]
fn the_last_point_matches_the_capture() {
    let row = call(&binary("methane", 0.70, "nC10", 0.30), 323.15, &PRESSURES);
    let point = &row.points[9];
    assert!(close(point.total_volume, 3.804_759_519_342_824e-4));
    assert!(loose(
        point.relative_volume.unwrap(),
        3.876_944_009_993_158_2
    ));
    assert!(close(point.liquid_volume, 5.795_382_544_867_234e-5));
    assert!(loose(
        point.liquid_volume_relative_to_vsat.unwrap(),
        0.590_533_344_583_701_1
    ));
    assert!(close(point.oil_density, 728.921_026_805_574_6));
    assert!(close(point.gas_volume.unwrap(), 3.225_221_264_856_099_6e-4));
    assert!(close(
        point.gas_standard_volume.unwrap(),
        2.842_442_004_938_507_5e-4
    ));
    assert!(close(point.z_gas.unwrap(), 0.998_540_659_076_143_3));
    assert!(close(
        point.relative_gas_gravity.unwrap(),
        0.591_575_189_874_806_7
    ));
    assert!(close(point.bo.unwrap(), 1.033_225_469_461_639_8));
    assert!(close(point.bg.unwrap(), 1.134_665_635_834_449_8));
    assert!(close(point.rs.unwrap(), 0.0));
}

#[test]
fn a_schedule_that_starts_above_saturation_matches_its_capture_block() {
    let row = call(
        &binary("methane", 0.60, "nC10", 0.40),
        323.15,
        &[250.0e5, 200.0e5, 150.0e5, 100.0e5, 50.0e5, 20.0e5, 5.0e5],
    );
    assert!(loose(
        row.saturation_pressure.unwrap(),
        2.228_773_365_020_752e7
    ));
    assert!(loose(
        row.saturation_volume.unwrap(),
        1.120_187_760_371_956_4e-4
    ));
    assert!(close(row.voil_std.unwrap(), 7.538_217_673_675_99e-5));

    // The first point is single phase: no gas, and no liquid relative volume.
    let point = &row.points[0];
    assert!(close(point.total_volume, 1.109_354_901_129_878_3e-4));
    assert!(loose(
        point.relative_volume.unwrap(),
        0.990_329_425_454_103_2
    ));
    assert!(close(point.oil_density, 599.806_247_146_239_6));
    assert!(point.gas_volume.is_none());
    assert!(point.gas_standard_volume.is_none());
    assert!(point.bg.is_none());
    assert!(point.liquid_volume_relative_to_vsat.is_none());
    assert!(close(point.bo.unwrap(), 1.471_640_842_906_178));
    assert!(close(point.rs.unwrap(), 187.943_969_215_828_64));

    let point = &row.points[6];
    assert!(close(point.liquid_volume, 7.826_949_904_123_92e-5));
    assert!(close(point.oil_density, 726.594_134_793_407_3));
    assert!(close(
        point.liquid_volume_relative_to_vsat.unwrap(),
        0.698_717_677_608_349_8
    ));
    assert!(close(point.gas_volume.unwrap(), 1.463_628_852_503_51e-4));
    assert!(close(point.z_gas.unwrap(), 0.993_591_624_985_005_1));
    assert!(close(point.bg.unwrap(), 0.225_808_386_003_323_7));
}

#[test]
fn a_schedule_is_required() {
    let err = differential_liberation(
        &binary("methane", 1.0, "nC10", 1.0),
        kelvins(300.0),
        &[],
        None,
        None,
        None,
    )
    .expect_err("a DLE needs pressures");
    assert!(err.to_string().contains("at least one pressure"));
}
