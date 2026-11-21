//! `pvt.black_oil_correlations` against its capture.

use azoth_core::units::{
    kelvins, kilograms_per_cubic_meter, kilograms_per_mole, pascal_seconds, pascals,
};
use azoth_pvt::{BlackOilQuery, black_oil_correlations};

const RS_BLACK_OIL: f64 = 500.0 * 0.178_108;
const RS_P_SAT: f64 = 500.0 * 0.178_107_606_679_03;
const T_K: f64 = (200.0 - 32.0) * 5.0 / 9.0 + 273.15;
const P_BARA: f64 = 2000.0 / 14.5038;
const P_PA: f64 = P_BARA * 1.0e5;
const MU_OD: f64 = 0.005;
const RHO_G: f64 = 5.0 * 16.018_46;
const CO_PA: f64 = (1.5e-5 * 14.5038) * 1.0e-5;
const MG_KG: f64 = 0.020;

fn blank(calculation: &str) -> BlackOilQuery {
    BlackOilQuery {
        calculation: calculation.to_owned(),
        rs: None,
        gamma_g: None,
        gamma_o: None,
        api: None,
        temperature: None,
        pressure: None,
        bob: None,
        compressibility: None,
        pb: None,
        mu_od: None,
        mu_ob: None,
        rho_g: None,
        molar_mass: None,
        z: None,
    }
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-9 * expected.abs().max(1.0)
}

fn answer(query: BlackOilQuery) -> azoth_pvt::BlackOilResultRow {
    black_oil_correlations(&[query])
        .expect("the correlation answers")
        .queries
        .into_iter()
        .next()
        .expect("one row")
}

#[test]
fn the_class_test_state_matches_the_capture() {
    let mut q = blank("api_from_specific_gravity");
    q.gamma_o = Some(0.85);
    assert!(close(answer(q).factor.unwrap(), 34.970_588_235_294_116));

    let mut q = blank("specific_gravity_from_api");
    q.api = Some(35.0);
    assert!(close(answer(q).factor.unwrap(), 0.849_849_849_849_849_9));

    let mut q = blank("bubble_point_standing");
    q.rs = Some(RS_BLACK_OIL);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).pressure.unwrap(),
        1.440_138_386_822_334_3e7
    ));

    let mut q = blank("bubble_point_standing_by_specific_gravity");
    q.rs = Some(RS_BLACK_OIL);
    q.gamma_g = Some(0.8);
    q.gamma_o = Some(0.85);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).pressure.unwrap(),
        1.441_372_913_903_083_5e7
    ));

    let mut q = blank("bubble_point_vasquez_beggs");
    q.rs = Some(RS_BLACK_OIL);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).pressure.unwrap(),
        1.597_957_145_445_926_1e7
    ));

    let mut q = blank("bubble_point_glaso_by_specific_gravity");
    q.rs = Some(RS_BLACK_OIL);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(answer(q).pressure.unwrap(), 1.311_789_631_285_186e8));

    let mut q = blank("bubble_point_standing_p_sat");
    q.rs = Some(RS_P_SAT);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).pressure.unwrap(),
        1.440_141_556_394_106_9e7
    ));

    let mut q = blank("bubble_point_vasquez_beggs_p_sat");
    q.rs = Some(RS_P_SAT);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(answer(q).pressure.unwrap(), 1.356_565_709_467_18e8));

    let mut q = blank("bubble_point_glaso_p_sat");
    q.rs = Some(RS_P_SAT);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).pressure.unwrap(),
        1.630_533_072_472_219_7e7
    ));

    let mut q = blank("bubble_point_petrosky_farshad");
    q.rs = Some(RS_P_SAT);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).pressure.unwrap(),
        4.230_689_089_767_583_5e7
    ));

    let mut q = blank("bubble_point_al_marhoun");
    q.rs = Some(RS_P_SAT);
    q.gamma_g = Some(0.8);
    q.gamma_o = Some(0.85);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).pressure.unwrap(),
        1.583_023_529_193_091_6e7
    ));

    let mut q = blank("bubble_point_statistics");
    q.rs = Some(RS_P_SAT);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    let row = answer(q);
    assert!(close(row.average.unwrap(), 5.216_755_203_326_428e7));
    assert!(close(row.minimum.unwrap(), 1.440_141_556_394_106_9e7));
    assert!(close(row.maximum.unwrap(), 1.356_565_709_467_18e8));
    assert!(close(row.std_dev.unwrap(), 4.944_701_431_371_851e7));
    assert!(close(row.standing.unwrap(), 1.440_141_556_394_106_9e7));
    assert!(close(row.vasquez_beggs.unwrap(), 1.356_565_709_467_18e8));
    assert!(close(row.glaso.unwrap(), 1.630_533_072_472_219_7e7));
    assert!(close(
        row.petrosky_farshad.unwrap(),
        4.230_689_089_767_583_5e7
    ));

    let mut q = blank("solution_gor_standing");
    q.pressure = Some(pascals(P_PA));
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).solution_gor.unwrap(),
        84.561_113_076_019_15
    ));

    let mut q = blank("solution_gor_vasquez_beggs");
    q.pressure = Some(pascals(P_PA));
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).solution_gor.unwrap(),
        74.759_317_788_111_45
    ));

    let mut q = blank("oil_fvf_standing");
    q.rs = Some(RS_BLACK_OIL);
    q.gamma_g = Some(0.8);
    q.gamma_o = Some(0.85);
    q.temperature = Some(kelvins(T_K));
    assert!(close(answer(q).oil_fvf.unwrap(), 1.306_099_621_056_295_6));

    let mut q = blank("oil_fvf_vasquez_beggs");
    q.rs = Some(RS_BLACK_OIL);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(answer(q).oil_fvf.unwrap(), 1.304_969_562_5));

    let mut q = blank("oil_fvf_undersaturated");
    q.bob = Some(1.35);
    q.compressibility = Some(CO_PA);
    q.pressure = Some(pascals(P_BARA * 2.0 * 1.0e5));
    q.pb = Some(pascals(P_BARA * 1.25 * 1.0e5));
    assert!(close(answer(q).oil_fvf.unwrap(), 1.319_964_170_211_004));

    let mut q = blank("oil_compressibility_vasquez_beggs");
    q.rs = Some(RS_BLACK_OIL);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    q.pressure = Some(pascals(P_BARA * 1.5 * 1.0e5));
    assert!(close(answer(q).compressibility.unwrap(), 1.935_943_051e-9));

    let mut q = blank("dead_oil_viscosity_beggs_robinson");
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).viscosity.unwrap(),
        0.001_785_412_101_640_377_4
    ));

    let mut q = blank("dead_oil_viscosity_glaso");
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).viscosity.unwrap(),
        0.001_489_869_145_841_706
    ));

    let mut q = blank("dead_oil_viscosity_kartoatmodjo");
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(
        answer(q).viscosity.unwrap(),
        0.001_346_462_051_543_013
    ));

    let mut q = blank("saturated_oil_viscosity_beggs_robinson");
    q.mu_od = Some(pascal_seconds(MU_OD));
    q.rs = Some(RS_BLACK_OIL);
    assert!(close(
        answer(q).viscosity.unwrap(),
        0.001_059_570_000_996_391_3
    ));

    let mut q = blank("saturated_oil_viscosity_kartoatmodjo");
    q.mu_od = Some(pascal_seconds(MU_OD));
    q.rs = Some(RS_BLACK_OIL);
    assert!(close(
        answer(q).viscosity.unwrap(),
        9.564_410_867_263_277e-4
    ));

    let mut q = blank("undersaturated_oil_viscosity_vasquez_beggs");
    q.mu_ob = Some(pascal_seconds(MU_OD));
    q.pressure = Some(pascals(P_BARA * 2.0 * 1.0e5));
    q.pb = Some(pascals(P_BARA * 1.25 * 1.0e5));
    assert!(close(
        answer(q).viscosity.unwrap(),
        0.005_873_126_478_884_313
    ));

    let mut q = blank("undersaturated_oil_viscosity_bergman_sutton");
    q.mu_ob = Some(pascal_seconds(MU_OD));
    q.pressure = Some(pascals(P_BARA * 2.0 * 1.0e5));
    q.pb = Some(pascals(P_BARA * 1.25 * 1.0e5));
    assert!(close(
        answer(q).viscosity.unwrap(),
        0.005_861_876_108_114_865
    ));

    let mut q = blank("gas_fvf");
    q.pressure = Some(pascals(P_PA));
    q.temperature = Some(kelvins(T_K));
    q.z = Some(0.85);
    assert!(close(answer(q).gas_fvf.unwrap(), 0.007_925_770_132_500_001));

    let mut q = blank("gas_fvf_rb_per_mscf");
    q.pressure = Some(pascals(P_PA));
    q.temperature = Some(kelvins(T_K));
    q.z = Some(0.85);
    assert!(close(answer(q).gas_fvf.unwrap(), 1.413_013_14));

    let mut q = blank("gas_viscosity_lee_gonzalez_eakin");
    q.temperature = Some(kelvins(T_K));
    q.rho_g = Some(kilograms_per_cubic_meter(RHO_G));
    q.molar_mass = Some(kilograms_per_mole(MG_KG));
    assert!(close(
        answer(q).viscosity.unwrap(),
        1.571_932_081_666_627_6e-5
    ));
}

#[test]
fn the_clamp_and_the_api_switch_are_the_classes_own() {
    let mut q = blank("bubble_point_standing_p_sat");
    q.rs = Some(1.0 * 0.178_107_606_679_03);
    q.gamma_g = Some(0.8);
    q.api = Some(35.0);
    q.temperature = Some(kelvins(T_K));
    assert!(close(answer(q).pressure.unwrap(), 101_352.972));

    let mut low = blank("bubble_point_vasquez_beggs");
    low.rs = Some(RS_BLACK_OIL);
    low.gamma_g = Some(0.8);
    low.api = Some(30.0);
    low.temperature = Some(kelvins(T_K));
    let mut high = low.clone();
    high.api = Some(30.1);
    assert!(close(
        answer(low).pressure.unwrap(),
        1.771_646_888_261_058e7
    ));
    assert!(close(
        answer(high).pressure.unwrap(),
        1.855_966_364_521_993e7
    ));
}

#[test]
fn a_missing_field_and_a_non_positive_input_are_refused() {
    let mut q = blank("bubble_point_standing");
    q.rs = Some(RS_BLACK_OIL);
    q.gamma_g = Some(0.8);
    q.temperature = Some(kelvins(T_K));
    let err = black_oil_correlations(&[q]).expect_err("api is missing");
    assert!(err.to_string().contains("queries[0].api"));

    let mut q = blank("dead_oil_viscosity_glaso");
    q.api = Some(0.0);
    q.temperature = Some(kelvins(T_K));
    let err = black_oil_correlations(&[q]).expect_err("api must be positive");
    assert!(err.to_string().contains("queries[0].api"));

    let mut q = blank("not_a_calculation");
    q.rs = Some(1.0);
    let err = black_oil_correlations(&[q]).expect_err("unknown calculation");
    assert!(err.to_string().contains("queries[0].calculation"));
}
