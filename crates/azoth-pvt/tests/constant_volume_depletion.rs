//! `pvt.constant_volume_depletion` against its capture.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::kelvins;
use azoth_pvt::{ConstantVolumeDepletionResult, constant_volume_depletion};

fn binary(first: &str, first_moles: f64, second: &str, second_moles: f64) -> Vec<FluidComponent> {
    vec![
        FluidComponent::base(first, first_moles),
        FluidComponent::base(second, second_moles),
    ]
}

fn call(components: &[FluidComponent], t: f64, pressures: &[f64]) -> ConstantVolumeDepletionResult {
    constant_volume_depletion(components, None, None, kelvins(t), pressures, None).unwrap()
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-6 * expected.abs().max(1.0)
}

/// The saturation state, measured at `1e-4`: the two-phase flash's boundary differs from
/// NeqSim's `TPflash` by about `5e-6` relative.
fn loose(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-4 * expected.abs().max(1.0)
}

const PRESSURES: [f64; 8] = [
    400.0e5, 350.0e5, 300.0e5, 250.0e5, 200.0e5, 150.0e5, 100.0e5, 50.0e5,
];

#[test]
fn the_saturation_and_the_first_restored_point_match_the_capture() {
    let row = call(&binary("methane", 0.70, "nC10", 0.30), 323.15, &PRESSURES);
    assert!(loose(row.saturation_pressure, 2.895_506_343_841_552_7e7));
    assert!(loose(row.saturation_volume, 9.813_810_850_855_022e-5));
    assert!(loose(row.z_saturation, 1.057_605_950_735_409));
    assert!(close(row.initial_moles, 1.0));
    assert!(close(row.cumulative_moles_removed, 0.559_774_679_115_818_2));

    let point = &row.points[3];
    assert!(close(point.total_volume, 9.813_810_851_034_321e-5));
    assert!(close(point.relative_volume, 1.000_000_000_018_27));
    assert!(close(
        point.cumulative_mole_fraction_depleted,
        0.067_702_590_752_107_31
    ));
    // The captured residual is 1e-17: the accounting is at round-off, so the row is absent.
    assert!(point.material_balance_relative_error.is_none());
    assert!(close(point.zmix, 0.979_455_921_712_563_5));
    assert!(close(point.z_gas.unwrap(), 0.915_926_100_421_086_9));
    assert!(close(
        point.liquid_volume.unwrap(),
        8.857_643_561_828_625e-5
    ));
    assert!(loose(
        point.liquid_volume_relative_to_vsat.unwrap(),
        0.902_569_215_612_802_2
    ));
}

#[test]
fn the_last_point_matches_the_capture() {
    let row = call(&binary("methane", 0.70, "nC10", 0.30), 323.15, &PRESSURES);
    let point = &row.points[7];
    assert!(close(point.total_volume, 9.813_810_850_617_436e-5));
    assert!(close(point.relative_volume, 0.999_999_999_975_790_6));
    assert!(close(
        point.cumulative_mole_fraction_depleted,
        0.559_774_679_115_818_4
    ));
    assert!(close(point.zmix, 0.414_853_110_408_274_33));
    assert!(close(point.z_gas.unwrap(), 0.943_891_181_611_328_9));
    assert!(close(
        point.liquid_volume.unwrap(),
        6.223_393_410_158_628e-5
    ));
    assert!(close(
        point.liquid_volume_relative_to_vsat.unwrap(),
        0.634_146_459_997_893_5
    ));
}

#[test]
fn the_k_values_are_one_row_per_pressure_and_never_negative() {
    let row = call(&binary("methane", 0.70, "nC10", 0.30), 323.15, &PRESSURES);
    let k = &row.k_values;
    assert_eq!(k.len(), PRESSURES.len());
    assert!(k.iter().all(|values| values.len() == 2));
    assert!(k.iter().flatten().all(|value| *value >= 0.0));
}

#[test]
fn a_schedule_is_required() {
    let err = constant_volume_depletion(
        &binary("methane", 1.0, "nC10", 1.0),
        None,
        None,
        kelvins(300.0),
        &[],
        None,
    )
    .expect_err("a CVD needs pressures");
    assert!(err.to_string().contains("at least one pressure"));
}
