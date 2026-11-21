//! Composition moves between experiment steps.
//!
//! DLE removes the gas phase after every flash, CVD removes the moles that restore the cell,
//! the separator train forwards only its liquid, and swelling adds gas. Each move is a pure
//! function from one [`Fluid`] to the next; the classes mutate a shared system instead, and
//! this module is where that difference is stated rather than hidden.

use azoth_core::{AzothError, Result};

use crate::flash::Phase;
use crate::fluid::Fluid;

/// The fluid after removing one flashed phase.
///
/// # Errors
/// * [`AzothError::InvalidInput`] if a component would go negative by more than `1e-14` of its
///   amount - round-off is clipped, a real deficit is a defect - or if no material remains.
pub fn remove_phase(fluid: &Fluid, phase: &Phase) -> Result<Fluid> {
    let removed = phase.fraction * fluid.total_moles();
    let mut moles = fluid.moles().to_vec();
    for (index, amount) in moles.iter_mut().enumerate() {
        let taken = removed * phase.x[index];
        let next = *amount - taken;
        if next < -1.0e-14 * amount.abs().max(1.0) {
            return Err(AzothError::invalid_input(
                "phase",
                format!(
                    "removing the phase takes {taken} mol of component {index}, and the fluid \
                     holds {}",
                    *amount
                ),
            ));
        }
        *amount = next.max(0.0);
    }
    if moles.iter().sum::<f64>() <= 0.0 {
        return Err(AzothError::invalid_input(
            "phase",
            "removing the phase leaves no material in the cell",
        ));
    }
    Ok(fluid.with_moles(moles))
}

/// The fluid after adding a component-wise amount.
///
/// # Errors
/// * [`AzothError::InvalidInput`] if an amount is not finite or is negative.
pub fn add_amounts(fluid: &Fluid, amounts: &[f64]) -> Result<Fluid> {
    if amounts.len() != fluid.moles().len() {
        return Err(AzothError::invalid_input(
            "amounts",
            format!(
                "the fluid has {} components and the addition states {}",
                fluid.moles().len(),
                amounts.len()
            ),
        ));
    }
    let mut moles = fluid.moles().to_vec();
    for (index, amount) in amounts.iter().enumerate() {
        if !amount.is_finite() || *amount < 0.0 {
            return Err(AzothError::invalid_input(
                format!("amounts[{index}]"),
                format!("an addition is {amount} mol"),
            ));
        }
        moles[index] += amount;
    }
    Ok(fluid.with_moles(moles))
}

/// The relative material-balance error `|initial - remaining - removed|/initial`.
#[must_use]
pub fn material_balance_error(initial: f64, remaining: f64, removed: f64) -> f64 {
    (initial - remaining - removed).abs() / initial
}

/// A phase's mass fraction of the flash, `beta M_phase / M_flash`.
#[must_use]
pub fn mass_fraction_of_phase(phase: &Phase, total_moles: f64, flash_mass: f64) -> f64 {
    phase.moles(total_moles) * phase.molar_mass / flash_mass
}
