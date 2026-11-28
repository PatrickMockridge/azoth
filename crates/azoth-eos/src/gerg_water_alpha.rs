//! `eos.gerg_water_alpha` - the GERG water attraction-temperature function.
//!
//! ```text
//! u     = 1 - sqrt(T/Tc)
//! alpha = (1 + c1*u + c2*u**2 + c3*u**4)**2
//! ```
//!
//! The coefficients are `AttractiveTermGERG.parametersGERG` at 273.16 K and above and
//! `parametersSolidGERG` below it. Spec: `specs/calcs/eos/gerg_water_alpha.toml`, which
//! records why the constructor's copy of the component's parameters is never read and why
//! the temperature derivatives are not carried.

use azoth_core::{Result, apply_checks};

use crate::results::GergWaterAlphaResult;
use crate::spec_gen;

/// The liquid-set coefficients, `AttractiveTermGERG.parametersGERG`.
const GERG_LIQUID: [f64; 3] = [0.905436, -0.213781, 0.26005];

/// The solid-set coefficients, `AttractiveTermGERG.parametersSolidGERG`.
const GERG_SOLID: [f64; 3] = [0.106025, 2.683845, -4.75638];

/// The class's `temperature < 273.16` test, in `selectWaterParameters`.
const SWITCH_TEMPERATURE: f64 = 273.16;

/// The GERG water alpha function.
///
/// `temperature` and `critical_temperature` are absolute temperatures; the critical
/// temperature is the component's `getComponent().getTC()`.
///
/// # Errors
/// * [`azoth_core::AzothError::OutOfRange`] if either temperature is not positive.
///
/// # Example
/// ```
/// use azoth_eos::gerg_water_alpha;
///
/// let r = gerg_water_alpha(300.0, 647.3)?;
/// assert!((r.alpha - 1.6127672229530905).abs() < 1e-15);
/// # Ok::<(), azoth_core::AzothError>(())
/// ```
pub fn gerg_water_alpha(
    temperature: f64,
    critical_temperature: f64,
) -> Result<GergWaterAlphaResult> {
    let spec = &spec_gen::GERG_WATER_ALPHA_SPEC;
    let mut warnings = Vec::new();

    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "temperature" => Some(temperature),
            "critical_temperature" => Some(critical_temperature),
            _ => None,
        },
        &mut warnings,
    )?;

    let c = if temperature >= SWITCH_TEMPERATURE {
        GERG_LIQUID
    } else {
        GERG_SOLID
    };
    let u = 1.0 - (temperature / critical_temperature).sqrt();
    let s = 1.0 + c[0] * u + c[1] * u * u + c[2] * u * u * u * u;
    let alpha = s * s;

    apply_checks(
        spec.derived_checks(),
        |name| (name == "alpha").then_some(alpha),
        &mut warnings,
    )?;

    Ok(GergWaterAlphaResult { alpha, warnings })
}
