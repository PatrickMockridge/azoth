//! `eos.pt_flash_check` - the isothermal flash with a convergence flag.
//!
//! Spec: `specs/models/eos/pt_flash_check.toml`. The arithmetic is `eos.pt_flash`'s;
//! this module exposes the caller's tolerance and turns the iteration cap into a
//! result when `strict = false`.

use azoth_core::Result;
use azoth_core::units::{Pressure, ThermodynamicTemperature};

use crate::mixture::Mixture;
use crate::pt_flash::pt_flash_impl;
use crate::results::PtFlashCheckResult;

/// The isothermal flash with a settable tolerance and a non-throwing cap.
///
/// # Errors
/// As [`crate::pt_flash::pt_flash`], except that a cap is a result when
/// `strict = Some(false)`.
pub fn pt_flash_check(
    mixture: &Mixture,
    t: ThermodynamicTemperature,
    p: Pressure,
    z: &[f64],
    tolerance: Option<f64>,
    strict: Option<bool>,
) -> Result<PtFlashCheckResult> {
    let (result, converged) = pt_flash_impl(
        mixture,
        t,
        p,
        z,
        &crate::model_gen::PT_FLASH_SPEC,
        tolerance,
        strict,
    )?;
    Ok(PtFlashCheckResult {
        vapour_fraction: result.vapour_fraction,
        x: result.x,
        y: result.y,
        k: result.k,
        ln_phi_liquid: result.ln_phi_liquid,
        ln_phi_vapour: result.ln_phi_vapour,
        z_liquid: result.z_liquid,
        z_vapour: result.z_vapour,
        min_t_over_tc: result.min_t_over_tc,
        phase: result.phase,
        converged,
        iterations: result.iterations,
        residual: result.residual,
        warnings: result.warnings,
    })
}
