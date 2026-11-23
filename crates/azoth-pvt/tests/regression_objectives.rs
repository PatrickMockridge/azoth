//! The regression's two ledgers and its sample scales.

use azoth_characterization::KijSource;
use azoth_characterization::component_row::FluidComponent;
use azoth_eos::Cubic;
use azoth_pvt::{
    CceExperiment, DleExperiment, ExperimentWeights, RegressionData, RegressionParameter,
    RegressionParameterConfig, ViscosityPhase, ViscosityPoint, build_fluid, chi_square, objectives,
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

fn tc_parameter() -> RegressionParameterConfig {
    RegressionParameterConfig {
        parameter: RegressionParameter::TcMultiplierC7Plus,
        initial: 1.0,
        lower: 0.95,
        upper: 1.05,
    }
}

fn cce_rows() -> [FluidComponent; 3] {
    [base("methane", 0.6), base("nC10", 0.3), c15()]
}

fn cce_data() -> CceExperiment {
    CceExperiment {
        temperature: 373.15,
        pressures: vec![2.5e7, 2.0e7, 1.5e7],
        relative_volumes: vec![
            0.998_031_797_063_637_8,
            1.102_821_596_061_994_3,
            1.300_910_800_403_610_5,
        ],
    }
}

#[test]
fn the_sample_scales_are_the_class_weights() {
    let weights = ExperimentWeights::default();
    let data = RegressionData {
        cce: Vec::new(),
        cvd: Vec::new(),
        dle: vec![DleExperiment {
            temperature: 323.15,
            pressures: vec![2.5e7],
            rs: vec![100.0],
            bo: vec![1.3],
            oil_density: vec![0.05],
        }],
        viscosity: vec![ViscosityPoint {
            temperature: 320.0,
            pressure: 5.0e5,
            viscosity: 3.5e-4,
            phase: ViscosityPhase::Oil,
        }],
        weights,
    };
    let samples = data.samples().expect("the observations are well formed");
    let sigmas: Vec<f64> = samples.iter().map(|sample| sample.sigma).collect();
    assert_eq!(sigmas.len(), 4);
    // DLE: max(|obs|*0.01, 1e-3) / weight.
    assert!((sigmas[0] - 1.0).abs() < 1e-15, "Rs sigma");
    assert!((sigmas[1] - 0.013).abs() < 1e-15, "Bo sigma");
    assert!(
        (sigmas[2] - 0.001).abs() < 1e-15,
        "density sigma at the floor"
    );
    // Viscosity: max(|obs|*0.02, 1e-10) / 0.5.
    assert!((sigmas[3] - 1.4e-5).abs() < 1e-18, "viscosity sigma");
}

#[test]
fn the_cce_ledgers_are_near_zero_at_the_capture_fit() {
    let parameters = [tc_parameter()];
    let built = build_fluid(
        &cce_rows(),
        Cubic::Srk,
        KijSource::Database,
        true,
        &parameters,
        &[1.03],
    )
    .expect("the tuned fluid builds");
    let data = RegressionData {
        cce: vec![cce_data()],
        cvd: Vec::new(),
        dle: Vec::new(),
        viscosity: Vec::new(),
        weights: ExperimentWeights::default(),
    };
    let samples = data.samples().expect("the observations are well formed");
    assert_eq!(samples.len(), 3);
    for sample in &samples {
        assert!((sample.sigma - 0.01).abs() < 1e-15, "CCE sigma");
    }
    let chi = chi_square(&built.fluid, built.csp, &samples, &data).expect("the chi-square runs");
    assert!(chi < 1e-6, "chi-square at the capture fit: {chi}");
    let ledger = objectives(&built.fluid, built.csp, &data).expect("the ledger runs");
    assert!(ledger.cce.expect("a CCE objective") < 1e-10, "{ledger:?}");
}

#[test]
fn the_dle_ledgers_are_near_zero_at_the_capture_fit() {
    let rows = [base("methane", 0.45), base("CO2", 0.05), base("nC10", 0.50)];
    let parameter = RegressionParameterConfig {
        parameter: RegressionParameter::BipCo2Hc,
        initial: 0.12,
        lower: 0.08,
        upper: 0.18,
    };
    let built = build_fluid(
        &rows,
        Cubic::Srk,
        KijSource::Database,
        true,
        &[parameter],
        &[0.15],
    )
    .expect("the BIP fluid builds");
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
    let samples = data.samples().expect("the observations are well formed");
    assert_eq!(samples.len(), 9);
    let chi = chi_square(&built.fluid, built.csp, &samples, &data).expect("the chi-square runs");
    assert!(chi < 1e-6, "chi-square at the capture fit: {chi}");
    let ledger = objectives(&built.fluid, built.csp, &data).expect("the ledger runs");
    assert!(ledger.dle.expect("a DLE objective") < 1e-10, "{ledger:?}");
}

#[test]
fn a_zero_weight_is_refused() {
    let data = RegressionData {
        cce: Vec::new(),
        cvd: Vec::new(),
        dle: Vec::new(),
        viscosity: vec![ViscosityPoint {
            temperature: 320.0,
            pressure: 5.0e5,
            viscosity: 3.5e-4,
            phase: ViscosityPhase::Oil,
        }],
        weights: ExperimentWeights {
            cce: 1.0,
            cvd: 1.0,
            dle: 1.0,
            viscosity: 0.0,
        },
    };
    assert!(data.samples().is_err());
}
