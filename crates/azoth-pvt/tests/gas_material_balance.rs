//! `pvt.gas_material_balance` against its capture.

use azoth_core::units::kelvins;
use azoth_pvt::{GasMaterialBalanceResult, gas_material_balance};

#[allow(clippy::too_many_arguments)]
fn call(
    calculation: &str,
    pressure: Option<&[f64]>,
    z: Option<&[f64]>,
    gp: Option<&[f64]>,
    we: Option<&[f64]>,
    pressure_value: Option<f64>,
    z_value: Option<f64>,
    temperature: Option<f64>,
    gas_gravity: Option<f64>,
) -> azoth_core::Result<GasMaterialBalanceResult> {
    gas_material_balance(
        calculation,
        pressure,
        z,
        gp,
        we,
        pressure_value,
        z_value,
        temperature.map(kelvins),
        gas_gravity,
    )
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-9 * expected.abs().max(1.0)
}

const PRESSURE: [f64; 5] = [30.0e6, 25.0e6, 20.0e6, 15.0e6, 10.0e6];
const Z: [f64; 5] = [0.9, 0.88, 0.85, 0.82, 0.80];
const GP: [f64; 5] = [0.0, 1.0e6, 2.0e6, 3.0e6, 4.0e6];
const WE: [f64; 5] = [0.0, 0.0, 0.0, 0.0, 0.0];

#[test]
fn the_bg_and_the_pz_fit_match_the_capture() {
    let row = call(
        "bg",
        None,
        None,
        None,
        None,
        Some(30.0e6),
        Some(0.9),
        Some(350.0),
        None,
    )
    .unwrap();
    assert!(close(row.gas_fvf.unwrap(), 0.003_685_055_938_484_985_5));

    let row = call(
        "fit_volumetric",
        Some(&PRESSURE),
        Some(&Z),
        Some(&GP),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(close(row.ogip.unwrap(), 6_482_720.260_271_815));
    assert!(close(row.pi_over_zi.unwrap(), 33_569_518.716_577_55));
    assert!(close(row.slope.unwrap(), -5.178_307_464_892_834e-5));
    assert!(close(row.r_squared.unwrap(), 0.998_697_084_170_453));

    let row = call(
        "fit_volumetric_from_gravity",
        Some(&PRESSURE),
        Some(&Z),
        Some(&GP),
        None,
        None,
        None,
        Some(350.0),
        Some(0.7),
    )
    .unwrap();
    assert!(close(row.ogip.unwrap(), 6_345_454.146_515_704));
    assert!(close(row.pi_over_zi.unwrap(), 32_915_051.207_889_61));
    assert!(close(row.slope.unwrap(), -5.187_186_046_559_536e-5));
    assert!(close(row.r_squared.unwrap(), 0.991_743_375_005_492_7));
}

#[test]
fn the_cole_plot_and_havlena_odeh_match_the_capture() {
    let row = call(
        "cole_plot",
        Some(&PRESSURE),
        Some(&Z),
        Some(&GP),
        None,
        None,
        None,
        Some(350.0),
        None,
    )
    .unwrap();
    let gp = row.cole_gp.unwrap();
    let ratio = row.cole_f_over_eg.unwrap();
    assert_eq!(gp.len(), 4);
    assert!(close(gp[3], 4.0e6));
    assert!(close(ratio[0], 6_769_230.769_230_775));
    assert!(close(ratio[1], 6_800_000.000_000_002));
    assert!(close(ratio[2], 6_648_648.648_648_65));
    assert!(close(ratio[3], 6_400_000.000_000_001));

    let row = call(
        "fit_havlena_odeh",
        Some(&PRESSURE),
        Some(&Z),
        Some(&GP),
        Some(&WE),
        None,
        None,
        Some(350.0),
        None,
    )
    .unwrap();
    assert!(close(row.ogip.unwrap(), 6_467_980.237_368_953));
    assert!(close(row.pi_over_zi.unwrap(), 0.0));
    assert!(close(row.slope.unwrap(), 6_467_980.237_368_953));
    assert!(close(row.r_squared.unwrap(), 0.998_901_143_103_677_2));
}

#[test]
fn a_flat_pz_line_leaves_the_ogip_absent() {
    let pressure = [30.0e6, 25.0e6];
    let z = [0.9, 0.75];
    let gp = [0.0, 1.0e6];
    let row = call(
        "fit_volumetric",
        Some(&pressure),
        Some(&z),
        Some(&gp),
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(row.ogip.is_none());
    assert_eq!(row.warnings.len(), 1);
}

#[test]
fn a_short_history_and_a_singular_fit_are_refused() {
    let pressure = [30.0e6];
    let z = [0.9];
    let gp = [0.0];
    let err = call(
        "fit_volumetric",
        Some(&pressure),
        Some(&z),
        Some(&gp),
        None,
        None,
        None,
        None,
        None,
    )
    .expect_err("one point is not a fit");
    assert!(err.to_string().contains("at least two"));

    let pressure = [30.0e6, 25.0e6];
    let z = [0.9, 0.88];
    let gp = [1.0e6, 1.0e6];
    let err = call(
        "fit_volumetric",
        Some(&pressure),
        Some(&z),
        Some(&gp),
        None,
        None,
        None,
        None,
        None,
    )
    .expect_err("a constant cumulative gas is singular");
    assert!(err.to_string().contains("singular"));
}
