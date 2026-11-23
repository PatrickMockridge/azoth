//! A phase's viscosity, by the correlation NeqSim's phase type selects.
//!
//! A gas and a liquid phase both default to `PFCTViscosityMethodHeavyOil` at the pin
//! (`GasPhysicalProperties` and `LiquidPhysicalProperties` construct the same class), so
//! [`azoth_eos::viscosity`] is the right call for both. An aqueous phase uses the water form,
//! and a negative answer is floored at `1e-5 Pa s` exactly where `PhysicalProperties` floors
//! it.

use azoth_core::units::{kelvins, pascals};
use azoth_core::{AzothError, Result};
use azoth_eos::Mixture;

use crate::standard::VISCOSITY_FLOOR_PA_S;

/// Which phase's viscosity is wanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    /// A gas phase.
    Gas,
    /// A liquid hydrocarbon phase.
    Oil,
    /// An aqueous phase.
    Aqueous,
}

/// One phase's dynamic viscosity, in Pa·s.
///
/// # Errors
/// * [`AzothError::InvalidInput`] if the correlation answers a NaN, which is not a viscosity.
/// * Propagates the correlation's refusals.
pub fn phase_viscosity(
    mixture: &Mixture,
    t: f64,
    p: f64,
    x: &[f64],
    kind: TransportKind,
) -> Result<f64> {
    let value = match kind {
        TransportKind::Gas | TransportKind::Oil => {
            azoth_eos::viscosity(mixture, kelvins(t), pascals(p), x)?
                .mu
                .value
        }
        TransportKind::Aqueous => {
            azoth_eos::aqueous_viscosity(mixture, kelvins(t), pascals(p), x)?
                .viscosity
                .value
        }
    };
    floor(value)
}

/// The same with the PVT regression's four PFCT correction factors.
///
/// **Every phase takes the PFCT form once a factor is set**, aqueous included: the class's
/// `applyCspViscosityCorrectionFactor` forces `PFCT` on each stored phase slot before it
/// writes the factor, so the water correlation is not the one that answers there.
///
/// # Errors
/// The same as [`phase_viscosity`], plus a non-finite correction factor.
pub fn phase_viscosity_with_csp(
    mixture: &Mixture,
    t: f64,
    p: f64,
    x: &[f64],
    correction: [f64; 4],
) -> Result<f64> {
    let value = azoth_eos::viscosity_with_csp(mixture, kelvins(t), pascals(p), x, correction)?
        .mu
        .value;
    floor(value)
}

fn floor(value: f64) -> Result<f64> {
    if value.is_nan() {
        return Err(AzothError::invalid_input(
            "viscosity",
            "the correlation answered a NaN at this state",
        ));
    }
    Ok(if value < 0.0 {
        VISCOSITY_FLOOR_PA_S
    } else {
        value
    })
}
