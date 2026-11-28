//! `eos.bukacek_water_in_gas` - the Bukacek water content of a gas.
//!
//! ```text
//! tau   = (667.14 - T)/667.14
//! e     = (a1*tau + a2*tau**1.5 + a3*tau**3 + a4*tau**3.5 + a5*tau**4 + a6*tau**7.5)/(1 - tau)
//! psw   = 220.64*exp(e)                                        bar
//! mg    = 761900.42*psw/P_bar + 16.016*10**(-1716.26/T + 6.69449)
//! x     = (mg/16800)/(101325/(8.3144621*288.15))
//! ```
//!
//! Spec: `specs/calcs/eos/bukacek_water_in_gas.toml`, which records the bar convention,
//! the fixed gas molar mass and the standard cubic metre.

use azoth_core::{Result, apply_checks};

use crate::results::BukacekWaterInGasResult;
use crate::spec_gen;

/// `BukacekWaterInGas.getWaterInGas`'s `tau` polynomial coefficients.
const A: [f64; 6] = [-7.85823, 1.83991, -11.7811, 22.67, -15.9393, 1.77516];

/// Water's critical temperature as the class writes it, `393.99 + 273.15` K.
const TC_WATER: f64 = 667.14;

/// Water's critical pressure as the class writes it, bar.
const PC_WATER_BAR: f64 = 220.64;

/// Pa per bar, the conversion the class omits because it assumes bar.
const PA_PER_BAR: f64 = 100_000.0;

/// The water mole fraction at a temperature and pressure, without the spec checks.
///
/// `pressure` is in Pa; the class's correlation is in bar.
pub(crate) fn water_in_gas_mole_fraction(temperature: f64, pressure: f64) -> f64 {
    let tau = (TC_WATER - temperature) / TC_WATER;
    let exponent = (A[0] * tau
        + A[1] * tau.powf(1.5)
        + A[2] * tau.powi(3)
        + A[3] * tau.powf(3.5)
        + A[4] * tau.powi(4)
        + A[5] * tau.powf(7.5))
        / (1.0 - tau);
    let psw_bar = PC_WATER_BAR * exponent.exp();
    let mg_per_sm3 = 761_900.42 * psw_bar / (pressure / PA_PER_BAR)
        + 16.016 * 10f64.powf(-1716.26 / temperature + 6.69449);
    let molgas_sm3 = 101_325.0 / (8.3144621 * 288.15);
    (mg_per_sm3 / 16_800.0) / molgas_sm3
}

/// The Bukacek water content of a gas, as a mole fraction.
///
/// # Errors
/// * [`azoth_core::AzothError::OutOfRange`] if `temperature` or `pressure` is not positive.
///
/// # Example
/// ```
/// use azoth_eos::bukacek_water_in_gas;
///
/// let r = bukacek_water_in_gas(288.15, 100_000.0)?;
/// assert!((r.water_mole_fraction - 0.01040391418367174).abs() < 1e-15);
/// # Ok::<(), azoth_core::AzothError>(())
/// ```
pub fn bukacek_water_in_gas(temperature: f64, pressure: f64) -> Result<BukacekWaterInGasResult> {
    let spec = &spec_gen::BUKACEK_WATER_IN_GAS_SPEC;
    let mut warnings = Vec::new();

    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "temperature" => Some(temperature),
            "pressure" => Some(pressure),
            _ => None,
        },
        &mut warnings,
    )?;

    let water_mole_fraction = water_in_gas_mole_fraction(temperature, pressure);

    apply_checks(
        spec.derived_checks(),
        |name| (name == "water_mole_fraction").then_some(water_mole_fraction),
        &mut warnings,
    )?;

    Ok(BukacekWaterInGasResult {
        water_mole_fraction,
        warnings,
    })
}
