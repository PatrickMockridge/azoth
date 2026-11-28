//! `eos.bukacek_water_dew_point` - the Bukacek water dew point.
//!
//! ```text
//! T0 = 273.15 K
//! x_k  = bukacek(T_{k-1}, P)
//! T_k  = T_{k-1} - (x_k - x)*1e5
//! stop when |(x_k - x)/x| <= 1e-8 or after 1,000 iterations, returning T_k
//! ```
//!
//! The class's own fixed step, with no bracket and no convergence flag. Spec:
//! `specs/calcs/eos/bukacek_water_dew_point.toml`, which records why a capped state is
//! not asserted.

use azoth_core::{Result, apply_checks};

use crate::bukacek_water_in_gas::water_in_gas_mole_fraction;
use crate::results::BukacekWaterDewPointResult;
use crate::spec_gen;

/// `BukacekWaterInGas.waterDewPointTemperature`'s start temperature, K.
const START_TEMPERATURE: f64 = 273.15;

/// The class's fixed step, K per unit mole fraction.
const STEP: f64 = 1e5;

/// The class's relative stopping test.
const RELATIVE_TOLERANCE: f64 = 1e-8;

/// The class's iteration cap.
const MAX_ITERATIONS: u32 = 1000;

/// The Bukacek water dew-point temperature.
///
/// # Errors
/// * [`azoth_core::AzothError::OutOfRange`] if `water_mole_fraction` or `pressure` is not
///   positive.
///
/// # Example
/// ```
/// use azoth_eos::bukacek_water_dew_point;
///
/// let r = bukacek_water_dew_point(20.0e-6, 7_000_000.0)?;
/// assert!((r.temperature - 248.33510900783565).abs() < 1e-9);
/// # Ok::<(), azoth_core::AzothError>(())
/// ```
pub fn bukacek_water_dew_point(
    water_mole_fraction: f64,
    pressure: f64,
) -> Result<BukacekWaterDewPointResult> {
    let spec = &spec_gen::BUKACEK_WATER_DEW_POINT_SPEC;
    let mut warnings = Vec::new();

    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "water_mole_fraction" => Some(water_mole_fraction),
            "pressure" => Some(pressure),
            _ => None,
        },
        &mut warnings,
    )?;

    let mut temperature = START_TEMPERATURE;
    for _ in 0..MAX_ITERATIONS {
        let x = water_in_gas_mole_fraction(temperature, pressure);
        temperature -= (x - water_mole_fraction) * STEP;
        if ((x - water_mole_fraction) / water_mole_fraction).abs() <= RELATIVE_TOLERANCE {
            break;
        }
    }

    apply_checks(
        spec.derived_checks(),
        |name| (name == "temperature").then_some(temperature),
        &mut warnings,
    )?;

    Ok(BukacekWaterDewPointResult {
        temperature,
        warnings,
    })
}
