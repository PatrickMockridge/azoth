//! `process.piping_fiv_liquid` - the AVIFF liquid-line FIV screen as a registered id.
//!
//! Spec: `specs/models/process/piping_fiv_liquid.toml`. The arithmetic is
//! `process/safety/vibration/PipingFivScreening.java` at the pin: the velocity energy term,
//! the D/t, branch and support factors, and the likelihood band.

use azoth_core::units::{Length, Velocity};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::piping_fiv_gas::FivLikelihood;

/// The class's reference liquid velocity, m/s.
const REFERENCE_LIQUID_VELOCITY: f64 = 6.0;
/// The D/t ratio above which the factor rises.
const D_OVER_T_FLOOR: f64 = 40.0;

/// Result of `process.piping_fiv_liquid`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PipingFivLiquidResult {
    /// The normalised likelihood of failure.
    pub lof: f64,
    /// The AVIFF likelihood band.
    pub likelihood: FivLikelihood,
    /// The outside-diameter-to-wall-thickness ratio.
    pub d_overt: f64,
    /// The D/t factor, floored at one.
    pub d_overt_factor: f64,
    /// The branch factor.
    pub branch_factor: f64,
    /// The support-quality factor after the clamp.
    pub support_quality_factor_applied: f64,
    /// The velocity normalised by the reference.
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

/// The liquid-line FIV likelihood and its contributing factors.
///
/// # Errors
/// `InvalidInput` for a negative velocity or branch count, a non-positive diameter or wall, a
/// fractional branch count, or a non-finite input.
pub fn piping_fiv_liquid(
    velocity: Velocity,
    outside_diameter: Length,
    wall_thickness: Length,
    branch_count: f64,
    support_quality_factor: f64,
) -> Result<PipingFivLiquidResult> {
    let spec = &model_gen::PIPING_FIV_LIQUID_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "velocity" => Some(velocity.value),
            "outside_diameter" => Some(outside_diameter.value),
            "wall_thickness" => Some(wall_thickness.value),
            "branch_count" => Some(branch_count),
            "support_quality_factor" => Some(support_quality_factor),
            _ => None,
        },
        &mut warnings,
    )?;

    for (name, value) in [
        ("velocity", velocity.value),
        ("outside_diameter", outside_diameter.value),
        ("wall_thickness", wall_thickness.value),
        ("branch_count", branch_count),
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

    let d_overt = outside_diameter.value / wall_thickness.value;
    let d_overt_factor = (d_overt / D_OVER_T_FLOOR).max(1.0);
    let branch_factor = 1.0 + 0.10 * branch_count;
    let support = support_quality_factor.clamp(1.0, 3.0);
    let energy_term = velocity.value / REFERENCE_LIQUID_VELOCITY;
    let lof = energy_term * d_overt_factor * branch_factor * support;

    Ok(PipingFivLiquidResult {
        lof,
        likelihood: FivLikelihood::band_for(lof),
        d_overt,
        d_overt_factor,
        branch_factor,
        support_quality_factor_applied: support,
        energy_term,
        warnings,
    })
}
