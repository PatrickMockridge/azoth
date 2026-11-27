//! `eos.state_at_tp` - one state in one call.
//!
//! Spec: `specs/models/eos/state_at_tp.toml`. The split is `eos.pt_flash_check`'s; the
//! volumetric and caloric properties are `molar_enthalpy_entropy`'s and
//! `phase_capacity`'s, evaluated at the flash's own roots and composition-weighted.
//! The phase class is a screening label from the mixture's Kay pseudo-critical point;
//! it is not a stability analysis.

use azoth_core::Result;
use azoth_core::units::{Pressure, ThermodynamicTemperature};

use crate::entropy_state::molar_mass_of;
use crate::heat_capacity::phase_capacity;
use crate::mixture::{Mixture, ReducedParameters};
use crate::molar_enthalpy_entropy::{IdealGasModel, molar_enthalpy_entropy};
use crate::pr_molar_volume::MOLAR_GAS_CONSTANT;
use crate::pt_flash_check::pt_flash_check;
use crate::results::{Phase, PhaseClass, StateAtTpResult};

/// One phase's molar properties at its own composition and root.
struct PhaseProperties {
    molar_mass: f64,
    molar_volume: f64,
    enthalpy: f64,
    cp: f64,
    cv: f64,
}

#[allow(clippy::too_many_arguments)] // one parameter per property the state exposes
fn phase_properties(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    reduced: &ReducedParameters,
    t: ThermodynamicTemperature,
    p: Pressure,
    composition: &[f64],
    root: f64,
) -> Result<PhaseProperties> {
    let molar_mass = molar_mass_of(mixture, composition)?;
    let shift = mixture.volume_shift(composition);
    let molar_volume = root * MOLAR_GAS_CONSTANT * t.value / p.value - shift;
    if !molar_volume.is_finite() || molar_volume <= 0.0 {
        return Err(azoth_core::AzothError::invalid_input(
            "T",
            format!("the corrected molar volume is {molar_volume} m3/mol at this state"),
        ));
    }
    let enthalpy = molar_enthalpy_entropy(mixture, ideal_gas, t, p, composition, root)?;
    let capacity = phase_capacity(mixture, ideal_gas, reduced, t, p, composition, root)?;
    Ok(PhaseProperties {
        molar_mass,
        molar_volume,
        enthalpy: enthalpy.h.value,
        cp: capacity.cp,
        cv: capacity.cv,
    })
}

/// The Kay pseudo-critical coordinates the screening class reads.
fn pseudo_critical(mixture: &Mixture, z: &[f64]) -> (f64, f64) {
    let mut tc = 0.0;
    let mut pc = 0.0;
    for (component, fraction) in mixture.components().iter().zip(z) {
        tc += fraction * component.tc.value;
        pc += fraction * component.pc.value;
    }
    (tc, pc)
}

fn classify(phase: Phase, temperature: f64, pressure: f64, tc_pc: f64, pc_pc: f64) -> PhaseClass {
    if phase == Phase::TwoPhase {
        return PhaseClass::TwoPhase;
    }
    if pressure >= pc_pc && temperature >= tc_pc {
        return PhaseClass::DenseSupercritical;
    }
    if pressure >= pc_pc {
        return PhaseClass::DenseSinglePhase;
    }
    match phase {
        Phase::AllLiquid => PhaseClass::SinglePhaseLiquid,
        Phase::AllVapour => PhaseClass::SinglePhaseGas,
        _ => PhaseClass::Undetermined,
    }
}

/// One thermodynamic state: the split and the bulk volumetric and caloric properties.
///
/// # Errors
/// Propagates the flash, the ideal-gas enthalpy and the heat-capacity failures. A
/// single-phase state whose composition carries no molar mass is refused by the
/// density it would divide by.
pub fn state_at_tp(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    t: ThermodynamicTemperature,
    p: Pressure,
    z: &[f64],
    tolerance: Option<f64>,
    strict: Option<bool>,
) -> Result<StateAtTpResult> {
    let flash = pt_flash_check(mixture, t, p, z, tolerance, strict)?;
    let reduced = mixture.reduced_parameters(t, p)?;

    let (beta, phases): (Option<f64>, Vec<(f64, Vec<f64>, f64)>) = match flash.phase {
        Phase::TwoPhase => {
            let beta = flash.vapour_fraction.ok_or_else(|| {
                azoth_core::AzothError::invalid_input(
                    "z",
                    "the flash split the feed and reported no vapour fraction",
                )
            })?;
            (
                Some(beta),
                vec![
                    (1.0 - beta, flash.x.clone(), flash.z_liquid),
                    (beta, flash.y.clone(), flash.z_vapour),
                ],
            )
        }
        Phase::AllLiquid => (None, vec![(1.0, z.to_vec(), flash.z_liquid)]),
        _ => (None, vec![(1.0, z.to_vec(), flash.z_vapour)]),
    };

    let mut molar_mass = 0.0;
    let mut molar_volume = 0.0;
    let mut enthalpy = 0.0;
    let mut cp = 0.0;
    let mut cv = 0.0;
    for (fraction, composition, root) in &phases {
        let properties = phase_properties(mixture, ideal_gas, &reduced, t, p, composition, *root)?;
        molar_mass += fraction * properties.molar_mass;
        molar_volume += fraction * properties.molar_volume;
        enthalpy += fraction * properties.enthalpy;
        cp += fraction * properties.cp;
        cv += fraction * properties.cv;
    }
    if !molar_volume.is_finite() || molar_volume <= 0.0 {
        return Err(azoth_core::AzothError::invalid_input(
            "T",
            format!("the bulk molar volume is {molar_volume} m3/mol at this state"),
        ));
    }

    let (tc_pc, pc_pc) = pseudo_critical(mixture, z);
    let phase_class = classify(flash.phase, t.value, p.value, tc_pc, pc_pc);
    Ok(StateAtTpResult {
        phase: flash.phase,
        phase_class,
        converged: flash.converged,
        vapour_fraction: beta,
        x: flash.x,
        y: flash.y,
        molar_mass,
        molar_volume,
        density: molar_mass / molar_volume,
        internal_energy: enthalpy - p.value * molar_volume,
        enthalpy,
        cp,
        cv,
        compressibility: p.value * molar_volume / (MOLAR_GAS_CONSTANT * t.value),
        system_sound_speed: None,
        interface_tension: None,
        warnings: flash.warnings,
    })
}
