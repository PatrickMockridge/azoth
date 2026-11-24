//! `pvt.generalized_material_balance` at its boundary.
//!
//! The states are the pin's, from `validation/neqsim/captures/pvt_generalized_material_balance.tsv`.

use azoth_core::units::pascals;
use azoth_pvt::{
    GeneralizedMaterialBalanceResult, MaterialBalanceObservation, generalized_material_balance,
};

fn record(pressure: f64, rs: f64, bo: f64, bg: f64, rv: f64) -> MaterialBalanceObservation {
    MaterialBalanceObservation {
        pressure,
        rs,
        bo,
        bg,
        bw: 1.02,
        rv,
        net_oil_produced: 0.0,
        net_gas_produced: 0.0,
        net_water_produced: 0.0,
        water_influx: 0.0,
        expansion_fraction: 0.0,
    }
}

fn history(oil: f64, gas: f64, support: bool) -> Vec<MaterialBalanceObservation> {
    let initial = record(3.0e7, 100.0, 1.6, 0.004, 0.0002);
    let records = [
        (260.0, 80.0, 1.5, 0.005, 0.00016),
        (210.0, 60.0, 1.4, 0.008, 0.00012),
        (160.0, 40.0, 1.3, 0.012, 0.00008),
        (110.0, 20.0, 1.2, 0.020, 0.00004),
    ];
    let initial_volume = oil * initial.bo + gas * initial.bg;
    let initial_oil = oil + initial.rv * gas;
    let initial_gas = gas + initial.rs * oil;
    let mut out = Vec::new();
    for (index, (pressure, rs, bo, bg, rv)) in records.iter().enumerate() {
        let mut current = record(*pressure * 1.0e5, *rs, *bo, *bg, *rv);
        let fraction = if support {
            0.001 * (index as f64 + 1.0)
        } else {
            0.0
        };
        let influx = if support {
            0.002 * initial_volume * (index as f64 + 1.0)
        } else {
            0.0
        };
        let net_water = if support {
            -0.001 * initial_volume * (index as f64 + 1.0)
        } else {
            0.0
        };
        let remaining = initial_volume * (1.0 - fraction) - influx + net_water * current.bw;
        let oil_volume = remaining
            * if oil == 0.0 {
                0.01 * (index as f64 + 1.0)
            } else {
                0.58 - 0.08 * index as f64
            };
        let gas_volume = remaining - oil_volume;
        let free_oil = oil_volume / current.bo;
        let free_gas = gas_volume / current.bg;
        current.net_oil_produced = initial_oil - (free_oil + current.rv * free_gas);
        current.net_gas_produced = initial_gas - (free_gas + current.rs * free_oil);
        current.net_water_produced = net_water;
        current.water_influx = influx;
        current.expansion_fraction = fraction;
        out.push(current);
    }
    out
}

fn fit_oil_and_gas(
    observations: &[MaterialBalanceObservation],
) -> Result<GeneralizedMaterialBalanceResult, azoth_core::AzothError> {
    generalized_material_balance(
        "oil_and_gas",
        pascals(3.0e7),
        100.0,
        1.6,
        0.004,
        1.02,
        0.0002,
        observations,
        None,
    )
}

#[test]
fn the_oil_and_gas_fit_is_the_capture() {
    let result = fit_oil_and_gas(&history(1.0e7, 2.0e9, false)).unwrap();
    assert!((result.free_oil / 9999999.999999976 - 1.0).abs() < 1.0e-9);
    assert!((result.free_gas / 2.0000000000000012e9 - 1.0).abs() < 1.0e-9);
    assert!((result.ooip / 1.0399999999999976e7 - 1.0).abs() < 1.0e-9);
    assert!((result.ogip / 2.999999999999999e9 - 1.0).abs() < 1.0e-9);
    assert!((result.gas_cap_ratio.unwrap() / 0.5000000000000014 - 1.0).abs() < 1.0e-9);
    assert!((result.condition_number / 10.34514772279802 - 1.0).abs() < 1.0e-9);
    assert!((result.rms_residual - 4.307366907596588e-9).abs() < 1.0e-6);
}

#[test]
fn the_known_gas_cap_folds_the_ratio_into_one_predictor() {
    let result = generalized_material_balance(
        "known_gas_cap",
        pascals(3.0e7),
        100.0,
        1.6,
        0.004,
        1.02,
        0.0002,
        &history(1.0e7, 2.0e9, true),
        Some(0.5),
    )
    .unwrap();
    assert!((result.free_oil / 1.0e7 - 1.0).abs() < 1.0e-9);
    assert!((result.free_gas / 2.0e9 - 1.0).abs() < 1.0e-9);
    assert!((result.gas_cap_ratio.unwrap() / 0.5 - 1.0).abs() < 1.0e-9);
    assert!((result.condition_number - 1.0).abs() < 1.0e-12);
}

#[test]
fn the_initially_gas_fit_leaves_the_gas_cap_ratio_absent_with_a_warning() {
    let result = generalized_material_balance(
        "initially_gas",
        pascals(3.0e7),
        100.0,
        1.6,
        0.004,
        1.02,
        0.0002,
        &history(0.0, 2.0e9, false),
        None,
    )
    .unwrap();
    assert_eq!(result.free_oil, 0.0);
    assert!((result.free_gas / 2.0e9 - 1.0).abs() < 1.0e-9);
    assert!(result.gas_cap_ratio.is_none());
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn a_refused_state_is_named() {
    let short = fit_oil_and_gas(&history(1.0e7, 2.0e9, false)[..1]).unwrap_err();
    assert!(short.to_string().contains("at least two"));

    let near_critical = generalized_material_balance(
        "oil_and_gas",
        pascals(3.0e7),
        100.0,
        1.6,
        0.004,
        1.02,
        0.01,
        &history(1.0e7, 2.0e9, false),
        None,
    )
    .unwrap_err();
    assert!(near_critical.to_string().contains("Rs*Rv"));

    let mut first = record(2.0e7, 0.0, 1.1, 0.02, 0.0);
    first.net_oil_produced = 1.0e5;
    first.net_gas_produced = 1.0e6;
    let mut second = record(1.0e7, 0.0, 1.2, 0.03, 0.0);
    second.net_oil_produced = 2.0e5;
    second.net_gas_produced = 2.0e6;
    let collinear = generalized_material_balance(
        "oil_and_gas",
        pascals(3.0e7),
        0.0,
        1.0,
        0.01,
        1.02,
        0.0,
        &[first, second],
        None,
    )
    .unwrap_err();
    assert!(collinear.to_string().contains("condition"));
}
