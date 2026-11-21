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
