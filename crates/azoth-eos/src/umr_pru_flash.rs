//! `eos.umr_pru_flash` - the UMR-PRU isothermal flash.
//!
//! Spec: `specs/models/eos/umr_pru_flash.toml`. The fluid is
//! [`crate::databank::umr_pru_mixture_of`]'s: Peng-Robinson with the standard PR alpha,
//! the UMR rule on the `_umr` UNIFAC tables and NeqSim's quadratic co-volume
//! (`bmixType = 1`). The loop is `eos.pt_flash`'s successive substitution; the spec
//! declares no fallback, so the second-order scheme - which needs the composition
//! derivatives the UMR rule does not provide - is never entered.

use azoth_core::Result;
use azoth_core::units::{Pressure, ThermodynamicTemperature};

use crate::databank::umr_pru_mixture_of;
use crate::pt_flash::pt_flash_impl;
use crate::results::UmrPruFlashResult;

/// The UMR-PRU flash at a temperature and pressure.
///
/// # Errors
/// As [`crate::pt_flash::pt_flash`]. The loop has no fallback, so a cap leaves the last
/// successive-substitution iterate in the result.
pub fn umr_pru_flash(
    names: &[&str],
    t: ThermodynamicTemperature,
    p: Pressure,
    z: &[f64],
    tolerance: Option<f64>,
) -> Result<UmrPruFlashResult> {
    let (mixture, _) = umr_pru_mixture_of(names)?;
    let (result, _converged) = pt_flash_impl(
        &mixture,
        t,
        p,
        z,
        &crate::model_gen::UMR_PRU_FLASH_SPEC,
        tolerance,
        Some(false),
    )?;
    Ok(UmrPruFlashResult {
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
        iterations: result.iterations,
        residual: result.residual,
        warnings: result.warnings,
    })
}
