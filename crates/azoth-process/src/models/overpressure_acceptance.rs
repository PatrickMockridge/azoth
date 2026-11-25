//! `process.overpressure_acceptance` - the accumulated-pressure acceptance limit.
//!
//! Spec: `specs/models/process/overpressure_acceptance.toml`. The arithmetic is
//! `process/safety/overpressure/OverpressureAcceptanceChecker.java` at the pin: the
//! three accumulation factors and the comparison in bara.

use azoth_core::units::{Pressure, pascals};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

const CAUSES: [&str; 13] = [
    "blocked_outlet",
    "check_valve_leakage",
    "gas_blow_by",
    "liquid_blow_by",
    "volatile_liquid_ingress",
    "inadvertent_valve_opening",
    "control_valve_failure",
    "fire",
    "thermal_expansion",
    "choke_collapse",
    "tube_rupture",
    "vacuum",
    "other",
];

/// Result of `process.overpressure_acceptance`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OverpressureAcceptanceResult {
    /// The accumulation factor.
    pub accumulation_factor: f64,
    /// `mawp accumulation_factor`.
    #[serde(serialize_with = "scalar")]
    pub allowable_pressure: Pressure,
    /// `allowable_pressure - peak_pressure`.
    #[serde(serialize_with = "scalar")]
    pub margin: Pressure,
    /// Whether the peak is at or below the allowable pressure.
    pub accepted: bool,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The accumulation factor, the allowable pressure and the acceptance verdict.
///
/// # Errors
/// `InvalidInput` for a cause the class does not name, or a non-positive or
/// non-finite pressure.
pub fn overpressure_acceptance(
    peak_pressure: Pressure,
    mawp: Pressure,
    cause: &str,
    multiple_devices: bool,
) -> Result<OverpressureAcceptanceResult> {
    let spec = &model_gen::OVERPRESSURE_ACCEPTANCE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "peak_pressure" => Some(peak_pressure.value),
            "mawp" => Some(mawp.value),
            _ => None,
        },
        &mut warnings,
    )?;
    if !CAUSES.contains(&cause) {
        return Err(AzothError::invalid_input(
            "cause",
            format!("`{cause}` is not one of the class's overpressure causes"),
        ));
    }
    for (name, value) in [("peak_pressure", peak_pressure.value), ("mawp", mawp.value)] {
        if !value.is_finite() || value <= 0.0 {
            return Err(AzothError::invalid_input(
                name,
                format!("a finite positive pressure is required, and {value} is not"),
            ));
        }
    }

    let peak_bara = peak_pressure.value / 1.0e5;
    let mawp_bara = mawp.value / 1.0e5;
    let factor = if cause == "fire" {
        1.21
    } else if multiple_devices {
        1.16
    } else {
        1.10
    };
    // The accumulation is a fraction of the gauge MAWP, `MAWP + (MAWP - atm) (factor - 1)`.
    let allowable_bara = mawp_bara + (mawp_bara - 1.01325) * (factor - 1.0);
    let margin_bara = allowable_bara - peak_bara;
    let next_up = f64::from_bits(allowable_bara.to_bits() + 1);
    Ok(OverpressureAcceptanceResult {
        accumulation_factor: factor,
        allowable_pressure: pascals(allowable_bara * 1.0e5),
        margin: pascals(margin_bara * 1.0e5),
        accepted: peak_bara <= next_up,
        warnings,
    })
}
