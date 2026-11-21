//! `pvt.gas_correlations` against its capture.

use azoth_core::units::{kelvins, kilograms_per_mole, pascals};
use azoth_pvt::{GasQuery, gas_correlations};

fn blank(calculation: &str) -> GasQuery {
    GasQuery {
        calculation: calculation.to_owned(),
        gamma_g: None,
        y_h2s: None,
        y_co2: None,
        y_n2: None,
        temperature: None,
        pressure: None,
        reference_pressure: None,
        tpc: None,
        ppc: None,
        tpr: None,
        ppr: None,
        molar_mass: None,
    }
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-9 * expected.abs().max(1.0)
}

fn answer(query: GasQuery) -> azoth_pvt::GasResultRow {
    gas_correlations(&[query])
        .expect("the correlation answers")
        .queries
        .into_iter()
        .next()
        .expect("one row")
}

#[test]
fn the_capture_states_match() {
    let mut q = blank("pseudo_critical_temperature_standing");
    q.gamma_g = Some(0.75);
    assert!(close(answer(q).temperature.unwrap(), 224.84375));

    let mut q = blank("pseudo_critical_pressure_standing");
    q.gamma_g = Some(0.75);
    assert!(close(answer(q).pressure.unwrap(), 4_599_872.102_483_487));

    let mut q = blank("pseudo_critical_temperature_sutton");
    q.gamma_g = Some(0.75);
    assert!(close(answer(q).temperature.unwrap(), 216.5));

    let mut q = blank("pseudo_critical_pressure_sutton");
    q.gamma_g = Some(0.75);
    assert!(close(answer(q).pressure.unwrap(), 4_526_572.346_557_454));

    let mut q = blank("pseudo_critical_temperature_piper");
    q.gamma_g = Some(0.75);
    q.y_h2s = Some(0.05);
    q.y_co2 = Some(0.03);
    q.y_n2 = Some(0.02);
    assert!(close(
        answer(q).temperature.unwrap(),
        216.115_140_142_022_26
    ));

    let mut q = blank("pseudo_critical_pressure_piper");
    q.gamma_g = Some(0.75);
    q.y_h2s = Some(0.05);
    q.y_co2 = Some(0.03);
    q.y_n2 = Some(0.02);
    assert!(close(answer(q).pressure.unwrap(), 5_120_813.200_459_205));

    let mut q = blank("wichert_aziz");
    q.tpc = Some(kelvins(222.22));
    q.ppc = Some(pascals(4_551_000.0));
    q.y_h2s = Some(0.10);
    q.y_co2 = Some(0.05);
    let row = answer(q);
    assert!(close(row.temperature.unwrap(), 210.700_309_626_484_85));
    assert!(close(row.pressure.unwrap(), 4_295_041.534_221_613));

    let mut q = blank("pseudo_reduced_temperature");
    q.temperature = Some(kelvins(366.48));
    q.tpc = Some(kelvins(222.22));
    assert!(close(answer(q).factor.unwrap(), 1.649_176_491_764_917_7));

    let mut q = blank("pseudo_reduced_pressure");
    q.pressure = Some(pascals(13_790_000.0));
    q.ppc = Some(pascals(4_550_000.0));
    assert!(close(answer(q).factor.unwrap(), 3.030_769_230_769_231));

    let mut q = blank("z_factor_hall_yarborough");
    q.tpr = Some(1.5);
    q.ppr = Some(2.0);
    assert!(close(answer(q).factor.unwrap(), 0.820_833_779_785_029_5));

    let mut q = blank("z_factor_hall_yarborough");
    q.tpr = Some(2.0);
    q.ppr = Some(10.0);
    assert!(close(answer(q).factor.unwrap(), 1.143_899_379_647_683));

    let mut q = blank("z_factor_hall_yarborough");
    q.tpr = Some(1.5);
    q.ppr = Some(0.0);
    assert_eq!(answer(q).factor.unwrap(), 1.0);

    let mut q = blank("z_factor_dranchuk_abou_kassem");
    q.tpr = Some(1.5);
    q.ppr = Some(2.0);
    assert!(close(answer(q).factor.unwrap(), 0.821_465_125_614_778_6));

    let mut q = blank("z_factor_dranchuk_abou_kassem");
    q.tpr = Some(2.0);
    q.ppr = Some(10.0);
    assert!(close(answer(q).factor.unwrap(), 1.144_448_554_810_509));

    let mut q = blank("z_factor_papay");
    q.tpr = Some(1.5);
    q.ppr = Some(2.0);
    assert!(close(answer(q).factor.unwrap(), 0.828_025_299_621_606_5));

    let mut q = blank("z_factor_papay");
    q.tpr = Some(1.5);
    q.ppr = Some(10.0);
    assert!(close(answer(q).factor.unwrap(), 1.450_184_577_372_872_3));

    let mut q = blank("z_factor_sutton");
    q.pressure = Some(pascals(13_790_000.0));
    q.temperature = Some(kelvins(366.48));
    q.gamma_g = Some(0.65);
    assert!(close(answer(q).factor.unwrap(), 0.897_739_052_953_383_2));

    let mut q = blank("z_factor_sour_gas");
    q.pressure = Some(pascals(13_790_000.0));
    q.temperature = Some(kelvins(366.48));
    q.gamma_g = Some(0.75);
    q.y_h2s = Some(0.10);
    q.y_co2 = Some(0.05);
    assert!(close(answer(q).factor.unwrap(), 0.890_363_336_925_573_8));

    let mut q = blank("gas_density");
    q.pressure = Some(pascals(13_790_000.0));
    q.temperature = Some(kelvins(366.48));
    q.gamma_g = Some(0.65);
    q.molar_mass = Some(kilograms_per_mole(0.01604));
    assert!(close(answer(q).density.unwrap(), 80.859_986_162_413_69));

    let mut q = blank("gas_fvf_from_z");
    q.pressure = Some(pascals(13_790_000.0));
    q.temperature = Some(kelvins(366.48));
    q.gamma_g = Some(0.65);
    assert!(close(answer(q).gas_fvf.unwrap(), 0.008_373_189_425_724_832));

    let mut q = blank("pseudo_pressure_from_correlation");
    q.pressure = Some(pascals(20_684_000.0));
    q.reference_pressure = Some(pascals(101_325.0));
    q.temperature = Some(kelvins(366.48));
    q.gamma_g = Some(0.65);
    q.molar_mass = Some(kilograms_per_mole(0.01604));
    assert!(close(
        answer(q).pseudo_pressure.unwrap(),
        2_865_418.435_858_538_4
    ));

    let mut q = blank("delta_pseudo_pressure");
    q.pressure = Some(pascals(20_684_000.0));
    q.reference_pressure = Some(pascals(6_895_000.0));
    q.temperature = Some(kelvins(366.48));
    q.gamma_g = Some(0.65);
    q.molar_mass = Some(kilograms_per_mole(0.01604));
    assert!(close(
        answer(q).pseudo_pressure.unwrap(),
        2_520_842.079_975_609_7
    ));

    let mut q = blank("h2s_concentration_from_temperature");
    q.temperature = Some(kelvins(366.48));
    assert!(close(
        answer(q).h2s_fraction.unwrap(),
        0.881_525_795_653_298_4
    ));

    let mut q = blank("h2s_concentration_from_co2");
    q.temperature = Some(kelvins(366.48));
    q.y_co2 = Some(0.05);
    assert!(close(
        answer(q).h2s_fraction.unwrap(),
        2.046_204_444_894_731_6
    ));
}

#[test]
fn a_gas_gravity_outside_the_correlation_range_is_refused() {
    let mut q = blank("pseudo_critical_temperature_standing");
    q.gamma_g = Some(0.54);
    let err = gas_correlations(&[q]).expect_err("Standing refuses 0.54");
    assert!(err.to_string().contains("queries[0].gamma_g"));

    let mut q = blank("pseudo_critical_temperature_sutton");
    q.gamma_g = Some(1.69);
    let err = gas_correlations(&[q]).expect_err("Sutton refuses 1.69");
    assert!(err.to_string().contains("queries[0].gamma_g"));
}

#[test]
fn the_pseudo_pressure_boundaries_are_refused() {
    let mut q = blank("pseudo_pressure_from_correlation");
    q.pressure = Some(pascals(20_684_000.0));
    q.reference_pressure = Some(pascals(0.0));
    q.temperature = Some(kelvins(366.48));
    q.gamma_g = Some(0.65);
    q.molar_mass = Some(kilograms_per_mole(0.01604));
    let err = gas_correlations(&[q]).expect_err("a zero reference pressure is refused");
    assert!(err.to_string().contains("queries[0].reference_pressure"));

    let mut q = blank("pseudo_pressure_from_correlation");
    q.pressure = Some(pascals(20_684_000.0));
    q.reference_pressure = Some(pascals(101_325.0));
    q.temperature = Some(kelvins(100.0));
    q.gamma_g = Some(0.65);
    q.molar_mass = Some(kilograms_per_mole(0.01604));
    let err = gas_correlations(&[q]).expect_err("Tpr <= 1 is refused");
    assert!(err.to_string().contains("tpr"));
}
