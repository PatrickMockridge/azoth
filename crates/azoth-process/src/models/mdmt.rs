//! `process.mdmt` - the UCS-66 minimum design metal temperature as a registered id.
//!
//! Spec: `specs/models/process/mdmt.toml`. The arithmetic is
//! `process/safety/mdmt/MDMTCalculator.java` at the pin: the four analytic curves, the
//! stress-ratio reduction and the cold-service verdict.

use azoth_core::units::{Length, ThermodynamicTemperature, kelvins};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's lower thickness clamp, mm.
const MINIMUM_THICKNESS_MM: f64 = 6.0;
/// The class's upper thickness clamp, mm.
const MAXIMUM_THICKNESS_MM: f64 = 100.0;
/// The class's full stress-ratio reduction, K.
const FULL_REDUCTION_K: f64 = 55.0;
/// Kelvin offset.
const CELSIUS_OFFSET: f64 = 273.15;

/// Result of `process.mdmt`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MdmtResult {
    /// The minimum design metal temperature.
    #[serde(serialize_with = "scalar")]
    pub mdmt: ThermodynamicTemperature,
    /// Whether the minimum operating temperature is at or above the MDMT.
    pub acceptable: bool,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// Refuse a value the arithmetic cannot carry to a published number.
fn require_finite(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

/// The UCS-66 curve's MDMT and the cold-service verdict.
///
/// # Errors
/// `InvalidInput` for an unknown material curve, a non-positive governing thickness, a
/// stress ratio outside `(0, 1]`, or a non-finite input.
pub fn mdmt(
    material_curve: &str,
    governing_thickness: Length,
    min_operating_temperature: ThermodynamicTemperature,
    stress_ratio: Option<f64>,
) -> Result<MdmtResult> {
    let spec = &model_gen::MDMT_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "governing_thickness" => Some(governing_thickness.value),
            "stress_ratio" => stress_ratio,
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite("governing_thickness", governing_thickness.value)?;
    if let Some(value) = stress_ratio {
        require_finite("stress_ratio", value)?;
    }
    require_finite("min_operating_temperature", min_operating_temperature.value)?;

    let thickness_mm =
        (governing_thickness.value * 1000.0).clamp(MINIMUM_THICKNESS_MM, MAXIMUM_THICKNESS_MM);
    let logarithm = (thickness_mm / MINIMUM_THICKNESS_MM).ln();
    let base = match material_curve {
        "a" => -29.0 + 18.0 * logarithm,
        "b" => -48.0 + 22.0 * logarithm,
        "c" => -55.0 + 20.0 * logarithm,
        "d" => -65.0 + 18.0 * logarithm,
        _ => {
            return Err(AzothError::invalid_input(
                "material_curve",
                "the curve must be one of a, b, c or d",
            ));
        }
    };
    let ratio = stress_ratio.unwrap_or(1.0);
    let reduction = if ratio <= 0.35 {
        FULL_REDUCTION_K
    } else if ratio >= 1.0 {
        0.0
    } else {
        FULL_REDUCTION_K * (1.0 - ratio) / 0.65
    };
    let mdmt_k = base - reduction + CELSIUS_OFFSET;
    let acceptable = min_operating_temperature.value >= mdmt_k;

    Ok(MdmtResult {
        mdmt: kelvins(mdmt_k),
        acceptable,
        warnings,
    })
}
