//! `pvt.constant_mass_expansion` against its capture.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::kelvins;
use azoth_pvt::{ConstantMassExpansionResult, constant_mass_expansion};

fn binary(first: &str, first_moles: f64, second: &str, second_moles: f64) -> Vec<FluidComponent> {
    vec![
        FluidComponent::base(first, first_moles),
        FluidComponent::base(second, second_moles),
    ]
}

fn call(components: &[FluidComponent], t: f64, pressures: &[f64]) -> ConstantMassExpansionResult {
    constant_mass_expansion(components, kelvins(t), pressures, None, None, None).unwrap()
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-6 * expected.abs().max(1.0)
}

/// The saturation-dependent fields, measured at `1e-4`.
///
/// The two-phase flash's boundary differs from NeqSim's `TPflash` by about `5e-6` relative
/// at these states, and `Y` and the liquid relative volume divide by `V/Vsat - 1`, which is
/// small enough that the boundary difference is magnified to `2.5e-5`. The measured residual
/// is the tolerance.
fn loose(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-4 * expected.abs().max(1.0)
}

const PRESSURES: [f64; 7] = [300.0e5, 250.0e5, 200.0e5, 150.0e5, 100.0e5, 50.0e5, 10.0e5];

#[test]
fn the_saturation_and_single_phase_points_match_the_capture() {
    let row = call(&binary("methane", 0.70, "nC10", 0.30), 323.15, &PRESSURES);
    assert!(
        loose(row.saturation_pressure, 2.895_506_343_841_552_7e7),
        "{}",
        row.saturation_pressure
    );
    assert!(loose(row.saturation_volume, 9.813_810_850_855_022e-5));
    assert!(loose(row.z_saturation, 1.057_605_950_735_409));
    assert!(loose(
        row.saturation_isothermal_compressibility,
        3.695_056_608_091_261_6e-9
    ));

    let point = &row.points[0];
    assert!(close(point.pressure, 3.0e7));
    assert!(
        close(point.total_volume, 9.771_147_835_546_112e-5),
        "total={}",
        point.total_volume
    );
    assert!(close(point.relative_volume, 0.995_652_757_531_474_8));
    assert!(close(point.density, 551.783_689_157_402_3));
    assert!(close(point.gas_volume, 9.771_147_835_546_112e-5));
    assert!(close(point.gas_standard_volume, 0.023_644_828_562_694_307));
    assert!(close(point.bg, 0.004_132_467_194_523_274));
    assert!(close(point.z_gas, 1.091_009_470_731_482));
    assert!(close(point.oil_density.unwrap(), 551.783_689_157_402_3));
    assert!(close(
        point.oil_viscosity.unwrap(),
        1.501_758_258_544_223e-4
    ));
    assert!(close(point.viscosity.unwrap(), 1.501_758_258_544_223e-4));
    assert!(close(
        point.isothermal_compressibility.unwrap(),
        3.570_486_569_966_042_5e-9
    ));
    assert!(point.liquid_relative_volume.is_none());
    assert!(point.y_factor.is_none());
}

#[test]
fn the_two_phase_points_match_the_capture() {
    let row = call(&binary("methane", 0.70, "nC10", 0.30), 323.15, &PRESSURES);

    let point = &row.points[1];
    assert!(close(point.pressure, 2.5e7));
    assert!(close(point.total_volume, 1.048_025_541_889_867e-4));
    assert!(close(point.relative_volume, 1.067_908_845_826_755));
    assert!(close(point.density, 176.169_209_066_002_48));
    assert!(close(point.gas_volume, 1.622_611_856_681_117_8e-5));
    assert!(close(
        point.gas_standard_volume,
        0.003_897_553_363_742_691_6
    ));
    assert!(close(point.bg, 0.004_163_154_946_833_049));
    assert!(close(point.z_gas, 0.915_926_100_421_117_1));
    assert!(close(point.oil_density.unwrap(), 576.418_063_870_379_1));
    assert!(close(
        point.oil_viscosity.unwrap(),
        1.786_516_505_684_866_4e-4
    ));
    assert!(loose(
        point.liquid_relative_volume.unwrap(),
        0.902_569_215_652_432_9
    ));
    assert!(
        loose(point.y_factor.unwrap(), 2.329_630_780_947_422),
        "y={}",
        point.y_factor.unwrap()
    );
    assert!(point.viscosity.is_none());
    assert!(point.isothermal_compressibility.is_none());

    let point = &row.points[6];
    assert!(close(point.pressure, 1.0e6));
    assert!(close(point.total_volume, 0.001_883_116_162_941_92));
    assert!(close(point.relative_volume, 19.188_429_363_073_11));
    assert!(close(point.density, 6.099_569_246_840_118));
    assert!(close(point.gas_volume, 0.001_823_987_290_897_257_7));
    assert!(close(point.gas_standard_volume, 0.016_254_727_350_381_48));
    assert!(close(point.bg, 0.112_212_727_508_742_3));
    assert!(close(point.z_gas, 0.987_506_515_969_426_6));
    assert!(close(point.oil_density.unwrap(), 723.674_606_569_442_5));
    assert!(close(
        point.oil_viscosity.unwrap(),
        6.064_926_444_454_098e-4
    ));
    assert!(loose(
        point.liquid_relative_volume.unwrap(),
        0.602_506_742_215_341_1
    ));
    assert!(loose(point.y_factor.unwrap(), 1.536_969_623_950_655_2));
}

#[test]
fn the_other_fluids_match_their_capture_blocks() {
    let row = call(
        &binary("methane", 0.95, "n-butane", 0.05),
        275.0,
        &[100.0e5, 50.0e5, 20.0e5, 10.0e5, 5.0e5],
    );
    assert!(
        loose(row.saturation_pressure, 7_495_916.938_781_738),
        "sat={}",
        row.saturation_pressure
    );
    assert!(loose(row.saturation_volume, 2.383_332_456_830_583_4e-4));
    assert!(loose(
        row.saturation_isothermal_compressibility,
        1.640_035_852_095_335_8e-7
    ));

    let unsaturate = call(
        &binary("methane", 0.70, "nC10", 0.30),
        323.15,
        &[400.0e5, 350.0e5, 300.0e5, 250.0e5],
    );
    assert!(loose(
        unsaturate.saturation_pressure,
        2.895_506_343_841_552_7e7
    ));
    // 400, 350 and 300 bar are above the 289.6-bar saturation pressure; 250 is not.
    assert!(unsaturate.points[0].y_factor.is_none());
    assert!(unsaturate.points[1].y_factor.is_none());
    assert!(unsaturate.points[2].y_factor.is_none());
    assert!(unsaturate.points[3].y_factor.is_some());
}

#[test]
fn a_pressure_schedule_is_required() {
    let err = constant_mass_expansion(
        &binary("methane", 1.0, "nC10", 1.0),
        kelvins(300.0),
        &[],
        None,
        None,
        None,
    )
    .expect_err("a CCE needs pressures");
    assert!(err.to_string().contains("at least one pressure"));
}
