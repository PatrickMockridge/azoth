//! `pvt.water_correlations` against its capture.

use azoth_core::units::{kelvins, pascals};
use azoth_pvt::{WaterQuery, water_correlations};

fn blank(calculation: &str) -> WaterQuery {
    WaterQuery {
        calculation: calculation.to_owned(),
        temperature: None,
        pressure: None,
        salinity_ppm: None,
        salinity_wt_fraction: None,
        salinity_molality: None,
    }
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-9 * expected.abs().max(1.0)
}

fn answer(query: WaterQuery) -> azoth_pvt::WaterResultRow {
    water_correlations(&[query])
        .expect("the correlation answers")
        .queries
        .into_iter()
        .next()
        .expect("one row")
}

#[test]
fn the_capture_states_match() {
    let mut q = blank("water_fvf_mccain");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    assert!(close(answer(q).fvf.unwrap(), 1.033_936_072_371_395_7));

    let mut q = blank("water_fvf_osif");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    q.salinity_ppm = Some(30_000.0);
    assert!(close(answer(q).fvf.unwrap(), 0.999_141_530_891_126_2));

    let mut q = blank("dead_water_viscosity_mccain");
    q.temperature = Some(kelvins(366.48));
    q.salinity_ppm = Some(30_000.0);
    assert!(close(
        answer(q).viscosity.unwrap(),
        3.354_601_229_917_465e-4
    ));

    let mut q = blank("water_viscosity_mccain");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    q.salinity_ppm = Some(30_000.0);
    assert!(close(
        answer(q).viscosity.unwrap(),
        3.851_883_276_754_872_3e-4
    ));

    let mut q = blank("water_compressibility_mccain");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    q.salinity_ppm = Some(30_000.0);
    assert!(close(
        answer(q).compressibility.unwrap(),
        4.055_508_388_263_189_4e-10
    ));

    let mut q = blank("brine_density_batzle_wang");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    q.salinity_wt_fraction = Some(0.03);
    assert!(close(answer(q).density.unwrap(), 994.544_024_114_124_8));

    let mut q = blank("water_density");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    assert!(close(answer(q).density.unwrap(), 973.907_012_848_368_8));

    let mut q = blank("solution_gas_water_ratio_culberson");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    q.salinity_ppm = Some(30_000.0);
    assert!(close(answer(q).rsw.unwrap(), 2.671_882_062_636_093));

    let mut q = blank("water_gas_surface_tension");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    assert!(close(
        answer(q).surface_tension.unwrap(),
        0.040_492_916_450_803_8
    ));

    let mut q = blank("rsw_mccain");
    q.temperature = Some(kelvins(373.15));
    q.pressure = Some(pascals(10_000_000.0));
    q.salinity_molality = Some(0.0);
    assert!(close(answer(q).rsw.unwrap(), 1.579_118_880_362_129_2));

    let mut q = blank("rsw_mccain");
    q.temperature = Some(kelvins(373.15));
    q.pressure = Some(pascals(10_000_000.0));
    q.salinity_molality = Some(0.620_626_797_601_188_8);
    assert!(close(answer(q).rsw.unwrap(), 1.575_715_730_017_222_4));
}

#[test]
fn a_negative_salinity_and_a_weight_fraction_outside_the_range_are_refused() {
    let mut q = blank("water_fvf_osif");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    q.salinity_ppm = Some(-1.0);
    let err = water_correlations(&[q]).expect_err("a negative salinity is refused");
    assert!(err.to_string().contains("queries[0].salinity_ppm"));

    let mut q = blank("brine_density_batzle_wang");
    q.temperature = Some(kelvins(366.48));
    q.pressure = Some(pascals(20_684_000.0));
    q.salinity_wt_fraction = Some(0.31);
    let err = water_correlations(&[q]).expect_err("0.31 is outside [0, 0.3]");
    assert!(err.to_string().contains("queries[0].salinity_wt_fraction"));
}
