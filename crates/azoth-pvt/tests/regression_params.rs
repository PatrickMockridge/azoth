//! The regression's row transforms, held to `pvt_regression.tsv`'s base and observed states.
//!
//! The capture's CCE block is the first PVT laboratory case on a TBP fluid: the class's own
//! rows, printed by `CharacterizationFluidRebuildProbe.row`, are the id's boundary.

use azoth_characterization::KijSource;
use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::kelvins;
use azoth_eos::Cubic;
use azoth_pvt::{
    RegressionParameter, RegressionParameterConfig, build_fluid, constant_mass_expansion_of,
    differential_liberation_of,
};

fn base(name: &str, moles: f64) -> FluidComponent {
    FluidComponent::base(name, moles)
}

fn c15() -> FluidComponent {
    FluidComponent {
        fluid: 0.0,
        name: "C15_PC".to_owned(),
        pseudo_component: true,
        moles: 0.1,
        molar_mass: Some(0.150),
        density: Some(820.0),
        normal_boiling_point: Some(459.785),
        critical_temperature: Some(617.630_088_327_571),
        critical_pressure: Some(2_317_913.323_563_696_4),
        acentric_factor: Some(0.620_715_808_239_756_8),
        critical_volume: Some(0.000_503_168_517_096_898_7),
        racket_z: Some(0.261_875_090_062_474_84),
        racket_z_cpa: Some(0.261_875_090_062_474_84),
        parachor: Some(410.3),
        critical_viscosity: Some(2.711_835_035_833_783_8e-05),
        triple_point_temperature: Some(243.9455),
        heat_of_fusion: Some(21_846.780_317_779_318),
        ideal_gas_enthalpy_of_formation: Some(-266_956.0),
        cp_a: Some(-2.833_126_210_103_083_4),
        cp_b: Some(0.828_756_107_986_086_2),
        cp_c: Some(-0.000_336_685_958_202_490_37),
        cp_d: Some(0.0),
        attractive_m: Some(1.389_195_974),
    }
}

fn assert_close(actual: f64, expected: f64, relative: f64, label: &str) {
    let difference = (actual - expected).abs() / expected.abs().max(1e-300);
    assert!(
        difference <= relative,
        "{label}: got {actual}, expected {expected}, relative {difference}"
    );
}

#[test]
fn the_cce_base_state_is_the_capture() {
    let rows = [base("methane", 0.6), base("nC10", 0.3), c15()];
    let parameters = [RegressionParameterConfig {
        parameter: RegressionParameter::TcMultiplierC7Plus,
        initial: 1.0,
        lower: 0.95,
        upper: 1.05,
    }];
    let built = build_fluid(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &parameters,
        &[1.0],
    )
    .expect("the base fluid builds");
    let cce = constant_mass_expansion_of(&built.fluid, kelvins(373.15), &[2.5e7, 2.0e7, 1.5e7])
        .expect("the base CCE runs");
    let expected = [
        0.996_053_650_580_541_4,
        1.095_285_410_451_278_6,
        1.293_585_208_286_591_3,
    ];
    for (index, point) in cce.points.iter().enumerate() {
        assert_close(
            point.relative_volume,
            expected[index],
            // Measured: the TBP-fluid CCE is 1.4e-6 from the class; the existing
            // CCE cases hold 1e-4, so this is the same comparison ten times tighter.
            1e-5,
            &format!("base relative_volume[{index}]"),
        );
    }
}

#[test]
fn the_tc_multiplier_reproduces_the_capture_observation() {
    let rows = [base("methane", 0.6), base("nC10", 0.3), c15()];
    let parameters = [RegressionParameterConfig {
        parameter: RegressionParameter::TcMultiplierC7Plus,
        initial: 1.0,
        lower: 0.95,
        upper: 1.05,
    }];
    let built = build_fluid(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &parameters,
        &[1.03],
    )
    .expect("the tuned fluid builds");
    let cce = constant_mass_expansion_of(&built.fluid, kelvins(373.15), &[2.5e7, 2.0e7, 1.5e7])
        .expect("the tuned CCE runs");
    let expected = [
        0.998_031_797_063_637_8,
        1.102_821_596_061_994_3,
        1.300_910_800_403_610_5,
    ];
    for (index, point) in cce.points.iter().enumerate() {
        assert_close(
            point.relative_volume,
            expected[index],
            1e-5,
            &format!("tuned relative_volume[{index}]"),
        );
    }
}

#[test]
fn the_three_multipliers_move_only_the_pseudo_rows() {
    let rows = [base("methane", 0.6), base("nC10", 0.3), c15()];
    let parameters = [
        RegressionParameterConfig {
            parameter: RegressionParameter::TcMultiplierC7Plus,
            initial: 1.0,
            lower: 0.95,
            upper: 1.05,
        },
        RegressionParameterConfig {
            parameter: RegressionParameter::PcMultiplierC7Plus,
            initial: 1.0,
            lower: 0.95,
            upper: 1.05,
        },
        RegressionParameterConfig {
            parameter: RegressionParameter::OmegaMultiplierC7Plus,
            initial: 1.0,
            lower: 0.9,
            upper: 1.1,
        },
    ];
    let built = build_fluid(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &parameters,
        &[1.02, 0.98, 1.05],
    )
    .expect("the multiplied fluid builds");
    let components = built.fluid.mixture.components();
    assert_close(
        components[0].tc.value,
        190.56,
        1e-12,
        "methane Tc is untouched",
    );
    assert_close(
        components[2].tc.value,
        617.630_088_327_571 * 1.02,
        1e-12,
        "C15 Tc",
    );
    assert_close(
        components[2].pc.value,
        2_317_913.323_563_696_4 * 0.98,
        1e-12,
        "C15 Pc",
    );
    assert_close(
        components[2].omega,
        0.620_715_808_239_756_8 * 1.05,
        1e-12,
        "C15 acentric",
    );
}

#[test]
fn the_volume_shift_scales_the_peneloux_constant() {
    let rows = [base("methane", 0.6), base("nC10", 0.3), c15()];
    let parameter = RegressionParameterConfig {
        parameter: RegressionParameter::VolumeShiftC7Plus,
        initial: 1.0,
        lower: 0.8,
        upper: 1.2,
    };
    let base_fluid = build_fluid(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &[parameter],
        &[1.0],
    )
    .expect("the base fluid builds");
    let scaled = build_fluid(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &[parameter],
        &[1.2],
    )
    .expect("the scaled fluid builds");
    let base_shift = base_fluid.fluid.mixture.components()[2].volume_shift_srk;
    let scaled_shift = scaled.fluid.mixture.components()[2].volume_shift_srk;
    assert_close(
        scaled_shift / base_shift,
        1.2,
        1e-12,
        "the shift scales by 1.2",
    );
}

#[test]
fn the_bips_set_the_named_pairs() {
    let rows = [
        base("nitrogen", 0.01),
        base("methane", 0.59),
        base("ethane", 0.1),
        base("nC10", 0.2),
        c15(),
    ];
    let parameters = [
        RegressionParameterConfig {
            parameter: RegressionParameter::BipMethaneC7Plus,
            initial: 0.0,
            lower: 0.0,
            upper: 0.1,
        },
        RegressionParameterConfig {
            parameter: RegressionParameter::BipC2C6C7Plus,
            initial: 0.0,
            lower: 0.0,
            upper: 0.05,
        },
        RegressionParameterConfig {
            parameter: RegressionParameter::BipN2Hc,
            initial: 0.0,
            lower: 0.02,
            upper: 0.12,
        },
    ];
    let built = build_fluid(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &parameters,
        &[0.05, 0.02, 0.07],
    )
    .expect("the BIP fluid builds");
    let mixture = &built.fluid.mixture;
    assert_close(mixture.kij(1, 4), 0.05, 1e-12, "methane-C15");
    assert_close(mixture.kij(2, 4), 0.02, 1e-12, "ethane-C15");
    assert_close(mixture.kij(0, 1), 0.07, 1e-12, "nitrogen-methane");
    assert_close(mixture.kij(0, 4), 0.07, 1e-12, "nitrogen-C15");
}

#[test]
fn the_csp_factors_are_carried_beside_the_fluid() {
    let rows = [base("methane", 1.0)];
    let parameter = RegressionParameterConfig {
        parameter: RegressionParameter::ViscosityCsp3,
        initial: 1.0,
        lower: 0.5,
        upper: 2.0,
    };
    let built = build_fluid(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &[parameter],
        &[1.35],
    )
    .expect("the CSP fluid builds");
    assert_eq!(built.csp, [1.0, 1.0, 1.35, 1.0]);
}

#[test]
fn the_co2_bip_reproduces_the_capture_observation() {
    let rows = [base("methane", 0.45), base("CO2", 0.05), base("nC10", 0.50)];
    let parameters = [RegressionParameterConfig {
        parameter: RegressionParameter::BipCo2Hc,
        initial: 0.12,
        lower: 0.08,
        upper: 0.18,
    }];
    let built = build_fluid(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &parameters,
        &[0.15],
    )
    .expect("the BIP fluid builds");
    let dle = differential_liberation_of(&built.fluid, kelvins(323.15), &[2.5e7, 1.5e7, 8.0e6])
        .expect("the DLE runs");
    let rs = [
        124.113_237_435_896_83,
        117.874_064_538_772_71,
        54.857_243_478_268_62,
    ];
    let bo = [
        1.297_774_285_932_201,
        1.319_622_112_619_866_4,
        1.166_844_714_158_733_5,
    ];
    let rho = [
        657.125_144_090_342_9,
        642.589_478_034_383_2,
        684.894_612_278_302_4,
    ];
    for (index, point) in dle.points.iter().enumerate() {
        assert_close(
            point.rs.expect("the DLE answers Rs"),
            rs[index],
            1e-6,
            &format!("Rs[{index}]"),
        );
        assert_close(
            point.bo.expect("the DLE answers Bo"),
            bo[index],
            1e-6,
            &format!("Bo[{index}]"),
        );
        assert_close(
            point.oil_density,
            rho[index],
            1e-6,
            &format!("oil density[{index}]"),
        );
    }
}
