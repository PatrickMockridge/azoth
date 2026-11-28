//! `eos.cpa_water_caloric_alpha` - the caloric-fitted SRK-CPA water alpha function.
//!
//! ```text
//! u     = 1 - sqrt(T/Tc)
//! alpha = (1 + c1*u + c2*u**2 + c3*u**3 + c4*u**4 + c5*u**5)**2
//! ```
//!
//! The coefficients are the constructor array of `AttractiveTermCPAWaterCaloric`; the
//! polynomial is `AttractiveTermMatCop5PRUMR`'s. Spec:
//! `specs/calcs/eos/cpa_water_caloric_alpha.toml`.

use azoth_core::{Result, apply_checks};

use crate::results::CpaWaterCaloricAlphaResult;
use crate::spec_gen;

/// The class's five calibrated coefficients.
const CPA_CALORIC: [f64; 5] = [
    0.6670190973128074,
    -0.25656681951681287,
    1.3687767086793399,
    -3.1891110889699097,
    5.218580308820144,
];

/// The caloric-fitted SRK-CPA water alpha function.
///
/// # Errors
/// * [`azoth_core::AzothError::OutOfRange`] if either temperature is not positive.
///
/// # Example
/// ```
/// use azoth_eos::cpa_water_caloric_alpha;
///
/// let r = cpa_water_caloric_alpha(298.15, 647.3)?;
/// assert!((r.alpha - 1.4813963230257607).abs() < 1e-15);
/// # Ok::<(), azoth_core::AzothError>(())
/// ```
pub fn cpa_water_caloric_alpha(
    temperature: f64,
    critical_temperature: f64,
) -> Result<CpaWaterCaloricAlphaResult> {
    let spec = &spec_gen::CPA_WATER_CALORIC_ALPHA_SPEC;
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

    let u = 1.0 - (temperature / critical_temperature).sqrt();
    let c = CPA_CALORIC;
    let s = 1.0
        + c[0] * u
        + c[1] * u * u
        + c[2] * u * u * u
        + c[3] * u * u * u * u
        + c[4] * u * u * u * u * u;
    let alpha = s * s;

    apply_checks(
        spec.derived_checks(),
        |name| (name == "alpha").then_some(alpha),
        &mut warnings,
    )?;

    Ok(CpaWaterCaloricAlphaResult { alpha, warnings })
}
