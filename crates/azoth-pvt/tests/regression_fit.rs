//! The whole fit, against the capture's two tuned states.

use azoth_characterization::KijSource;
use azoth_characterization::component_row::FluidComponent;
use azoth_eos::Cubic;
use azoth_pvt::{
    CceExperiment, CceRow, DleExperiment, ExperimentWeights, RegressionData, RegressionParameter,
    RegressionParameterConfig, RegressionParameterRow, fit, regression,
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

#[test]
fn the_cce_fit_recovers_the_capture_parameter() {
    let rows = [base("methane", 0.6), base("nC10", 0.3), c15()];
    let parameters = [RegressionParameterConfig {
        parameter: RegressionParameter::TcMultiplierC7Plus,
        initial: 1.0,
        lower: 0.95,
        upper: 1.05,
    }];
    let data = RegressionData {
        cce: vec![CceExperiment {
            temperature: 373.15,
            pressures: vec![2.5e7, 2.0e7, 1.5e7],
            relative_volumes: vec![
                0.998_031_797_063_637_8,
                1.102_821_596_061_994_3,
                1.300_910_800_403_610_5,
            ],
        }],
        cvd: Vec::new(),
        dle: Vec::new(),
        viscosity: Vec::new(),
        weights: ExperimentWeights::default(),
    };
    let result = fit(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &parameters,
        &data,
        6,
    )
    .expect("the CCE fit runs");
    // The class's own fit answered 1.029994778729241; the TBP CCE is 1.4e-6 from it.
    assert!(
        (result.values[0] - 1.029_994_778_729_241).abs() < 1e-4,
        "{result:?}"
    );
    assert!(result.chi_square.is_finite() && result.chi_square >= 0.0);
    assert_eq!(result.standard_errors.len(), 1);
    assert!(result.standard_errors[0] > 0.0);
}

#[test]
fn the_dle_fit_recovers_the_capture_parameter() {
    let rows = [base("methane", 0.45), base("CO2", 0.05), base("nC10", 0.50)];
    let parameters = [RegressionParameterConfig {
        parameter: RegressionParameter::BipCo2Hc,
        initial: 0.12,
        lower: 0.08,
        upper: 0.18,
    }];
    let data = RegressionData {
        cce: Vec::new(),
        cvd: Vec::new(),
        dle: vec![DleExperiment {
            temperature: 323.15,
            pressures: vec![2.5e7, 1.5e7, 8.0e6],
            rs: vec![
                124.113_237_435_896_83,
                117.874_064_538_772_71,
                54.857_243_478_268_62,
            ],
            bo: vec![
                1.297_774_285_932_201,
                1.319_622_112_619_866_4,
                1.166_844_714_158_733_5,
            ],
            oil_density: vec![
                657.125_144_090_342_9,
                642.589_478_034_383_2,
                684.894_612_278_302_4,
            ],
        }],
        viscosity: Vec::new(),
        weights: ExperimentWeights::default(),
    };
    let result = fit(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &parameters,
        &data,
        6,
    )
    .expect("the DLE fit runs");
    // The class's own fit answered 0.14999999104346481.
    assert!(
        (result.values[0] - 0.149_999_991_043_464_81).abs() < 1e-6,
        "{result:?}"
    );
}

#[test]
fn the_model_surface_matches_the_fit() {
    let rows = [base("methane", 0.6), base("nC10", 0.3), c15()];
    let parameters = [RegressionParameterRow {
        parameter: "tc_multiplier_c7plus".to_owned(),
        initial: 1.0,
        lower: 0.95,
        upper: 1.05,
    }];
    let observations = [
        CceRow {
            experiment: 0.0,
            temperature: 373.15,
            pressure: 2.5e7,
            relative_volume: 0.998_031_797_063_637_8,
        },
        CceRow {
            experiment: 0.0,
            temperature: 373.15,
            pressure: 2.0e7,
            relative_volume: 1.102_821_596_061_994_3,
        },
        CceRow {
            experiment: 0.0,
            temperature: 373.15,
            pressure: 1.5e7,
            relative_volume: 1.300_910_800_403_610_5,
        },
    ];
    let result = regression(
        &rows,
        &parameters,
        Some(&observations),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(6),
    )
    .expect("the model surface runs");
    assert!((result.fitted_parameters[0].value - 1.029_994_778_729_241).abs() < 1e-4);
    assert_eq!(
        result.fitted_parameters[0].parameter,
        "tc_multiplier_c7plus"
    );
    assert_eq!(result.objectives.len(), 1);
    assert_eq!(result.objectives[0].experiment, "cce");
    assert!((result.total_objective - result.objectives[0].value).abs() < 1e-18);
    assert_eq!(result.degrees_of_freedom, 2.0);
    assert!(!result.convergence_reason.is_empty());
}

#[test]
fn a_non_contiguous_experiment_is_refused() {
    let rows = [base("methane", 0.6), base("nC10", 0.3), c15()];
    let parameters = [RegressionParameterRow {
        parameter: "tc_multiplier_c7plus".to_owned(),
        initial: 1.0,
        lower: 0.95,
        upper: 1.05,
    }];
    let observations = [
        CceRow {
            experiment: 0.0,
            temperature: 373.15,
            pressure: 2.5e7,
            relative_volume: 1.0,
        },
        CceRow {
            experiment: 1.0,
            temperature: 373.15,
            pressure: 2.0e7,
            relative_volume: 1.0,
        },
        CceRow {
            experiment: 0.0,
            temperature: 373.15,
            pressure: 1.5e7,
            relative_volume: 1.0,
        },
    ];
    let error = regression(
        &rows,
        &parameters,
        Some(&observations),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(6),
    )
    .expect_err("a reopening experiment is refused");
    assert!(matches!(error, azoth_core::AzothError::InvalidInput { .. }));
}
