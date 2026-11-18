//! `process.piping_fiv_gas` - the AVIFF gas-line FIV screen as a registered id.
//!
//! Spec: `specs/models/process/piping_fiv_gas.toml`. The arithmetic is
//! `process/safety/vibration/PipingFivScreening.java` at the pin: the momentum-flux energy
//! term, the D/t, branch, pulsation and support factors, and the likelihood band.

use azoth_core::units::{Length, MassDensity, Pressure, Velocity, pascals};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's reference gas momentum flux, Pa.
const REFERENCE_GAS_RHO_V2: f64 = 50_000.0;
/// The D/t ratio above which the factor rises.
const D_OVER_T_FLOOR: f64 = 40.0;

/// The AVIFF likelihood-of-failure band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FivLikelihood {
    /// Below 0.3: acceptable.
    #[serde(rename = "low")]
    Low,
    /// Below 0.5: basic mitigation.
    #[serde(rename = "medium")]
    Medium,
    /// Below 1.0: a detailed dynamic assessment.
    #[serde(rename = "high")]
    High,
    /// At or above 1.0: redesign.
    #[serde(rename = "very_high")]
    VeryHigh,
}

impl FivLikelihood {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::VeryHigh => "very_high",
        }
    }

    /// The band a likelihood score falls in, with the class's exclusive upper bounds.
    #[must_use]
    pub fn band_for(lof: f64) -> Self {
        if lof < 0.3 {
            Self::Low
        } else if lof < 0.5 {
            Self::Medium
        } else if lof < 1.0 {
            Self::High
        } else {
            Self::VeryHigh
        }
    }
}

/// Result of `process.piping_fiv_gas`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PipingFivGasResult {
    /// The normalised likelihood of failure.
    pub lof: f64,
    /// The AVIFF likelihood band.
    pub likelihood: FivLikelihood,
    /// The gas momentum flux.
    #[serde(serialize_with = "scalar")]
    pub rho_v2: Pressure,
    /// The outside-diameter-to-wall-thickness ratio.
    pub d_overt: f64,
    /// The D/t factor, floored at one.
    pub d_overt_factor: f64,
    /// The branch factor.
    pub branch_factor: f64,
    /// The pulsation factor after the clamp.
    pub pulsation_factor_applied: f64,
    /// The support-quality factor after the clamp.
    pub support_quality_factor_applied: f64,
    /// The momentum flux normalised by the reference.
    pub energy_term: f64,
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

/// The gas-line FIV likelihood and its contributing factors.
///
/// # Errors
/// `InvalidInput` for a negative density, velocity or branch count, a non-positive diameter or
/// wall, a fractional branch count, or a non-finite input.
pub fn piping_fiv_gas(
    density: MassDensity,
    velocity: Velocity,
    outside_diameter: Length,
    wall_thickness: Length,
    branch_count: f64,
    pulsation_factor: f64,
    support_quality_factor: f64,
) -> Result<PipingFivGasResult> {
    let spec = &model_gen::PIPING_FIV_GAS_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "density" => Some(density.value),
            "velocity" => Some(velocity.value),
            "outside_diameter" => Some(outside_diameter.value),
            "wall_thickness" => Some(wall_thickness.value),
            "branch_count" => Some(branch_count),
            "pulsation_factor" => Some(pulsation_factor),
            "support_quality_factor" => Some(support_quality_factor),
            _ => None,
        },
        &mut warnings,
    )?;

    for (name, value) in [
        ("density", density.value),
        ("velocity", velocity.value),
        ("outside_diameter", outside_diameter.value),
        ("wall_thickness", wall_thickness.value),
        ("branch_count", branch_count),
        ("pulsation_factor", pulsation_factor),
        ("support_quality_factor", support_quality_factor),
    ] {
        require_finite(name, value)?;
    }
    if branch_count.fract() != 0.0 {
        return Err(AzothError::invalid_input(
            "branch_count",
            "the small-bore connection count is a whole number",
        ));
    }

    let rho_v2 = density.value * velocity.value * velocity.value;
    let d_overt = outside_diameter.value / wall_thickness.value;
    let d_overt_factor = (d_overt / D_OVER_T_FLOOR).max(1.0);
    let branch_factor = 1.0 + 0.15 * branch_count;
    let pulsation = pulsation_factor.clamp(1.0, 4.0);
    let support = support_quality_factor.clamp(1.0, 3.0);
    let energy_term = rho_v2 / REFERENCE_GAS_RHO_V2;
    let lof = energy_term * d_overt_factor * branch_factor * pulsation * support;

    Ok(PipingFivGasResult {
        lof,
        likelihood: FivLikelihood::band_for(lof),
        rho_v2: pascals(rho_v2),
        d_overt,
        d_overt_factor,
        branch_factor,
        pulsation_factor_applied: pulsation,
        support_quality_factor_applied: support,
        energy_term,
        warnings,
    })
}
