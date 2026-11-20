//! The common flashed TP state the safety studies read.
//!
//! One flash, the bulk and per-phase densities, the phase amounts and the capacity ratio
//! `Cp/Cv`, weighted over the phases exactly as `SystemThermo` sums them. The inventory,
//! leak-detection, envelope-MDMT and fire-rupture ids all read this state; none of them
//! restates the flash.

use azoth_core::units::{kelvins, pascals};
use azoth_core::{AzothError, Result};
use azoth_eos::entropy_state::molar_mass_of;
use azoth_eos::heat_capacity::phase_capacity;
use azoth_eos::mixture::{Mixture, ReducedParameters, RootSide};
use azoth_eos::molar_enthalpy_entropy::IdealGasModel;
use azoth_eos::pr_molar_volume::MOLAR_GAS_CONSTANT;
use azoth_eos::pt_flash::pt_flash;
use azoth_eos::results::Phase;
use azoth_eos::tp_multiflash::tp_multiflash;

/// One flashed phase.
#[derive(Debug, Clone)]
pub(crate) struct PhaseInfo {
    /// The phase's mole fraction of the feed.
    pub(crate) mole_fraction: f64,
    /// The phase's composition.
    pub(crate) composition: Vec<f64>,
    /// The phase's compressibility `Z`.
    pub(crate) z_factor: f64,
    /// The phase's molar mass, kg/mol.
    pub(crate) molar_mass: f64,
    /// The phase's corrected molar volume, m³/mol.
    pub(crate) molar_volume: f64,
    /// The phase's density, kg/m³.
    pub(crate) density: f64,
    /// The phase's isobaric molar heat capacity, J/(mol K).
    pub(crate) cp: f64,
    /// The phase's isochoric molar heat capacity, J/(mol K).
    pub(crate) cv: f64,
}

/// A flashed state and the phases it carries.
#[derive(Debug, Clone)]
pub(crate) struct FlashState {
    /// The state's temperature, K.
    pub(crate) temperature: f64,
    /// The state's pressure, Pa.
    pub(crate) pressure: f64,
    /// The feed's bulk density, kg/m³.
    pub(crate) bulk_density: f64,
    /// `sum(nu_i Cp_i) / sum(nu_i Cv_i)`.
    pub(crate) gamma: f64,
    /// The gas phase's index, when the flash found one.
    pub(crate) gas_phase: Option<usize>,
    /// The oil phase's index, when the flash found one.
    pub(crate) oil_phase: Option<usize>,
    /// The aqueous phase's index, when the flash found one.
    pub(crate) aqueous_phase: Option<usize>,
    /// Every phase, in the flash's order.
    pub(crate) phases: Vec<PhaseInfo>,
}

impl FlashState {
    /// The gas phase's density, or the bulk density when no gas phase exists.
    pub(crate) fn gas_density(&self) -> f64 {
        self.gas_phase
            .map(|index| self.phases[index].density)
            .unwrap_or(self.bulk_density)
    }

    /// The liquid density: oil, then aqueous, then the fallback.
    pub(crate) fn liquid_density(&self, fallback: f64) -> f64 {
        self.oil_phase
            .or(self.aqueous_phase)
            .map(|index| self.phases[index].density)
            .unwrap_or(fallback)
    }

    /// Whether the flash found a gas phase.
    pub(crate) fn has_gas(&self) -> bool {
        self.gas_phase.is_some()
    }

    /// The gas phase's mole fraction of the feed.
    pub(crate) fn gas_mole_fraction(&self) -> f64 {
        self.gas_phase
            .map(|index| self.phases[index].mole_fraction)
            .unwrap_or(0.0)
    }
}

/// Flash a feed at a temperature and pressure.
///
/// `multiphase` runs the three-phase flash and labels its liquids by water content; the
/// ordinary flash is one vapour and one liquid.
pub(crate) fn flash(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    temperature: f64,
    pressure: f64,
    z: &[f64],
    multiphase: bool,
) -> Result<FlashState> {
    let t = kelvins(temperature);
    let p = pascals(pressure);
    let reduced = mixture.reduced_parameters(t, p)?;
    let ordinary = pt_flash(mixture, t, p, z)?;
    if !multiphase || ordinary.phase == Phase::Trivial {
        return from_ordinary(mixture, ideal_gas, &reduced, t, p, z, &ordinary);
    }

    let split = tp_multiflash(mixture, t, p, z)?;
    if split.phase_count < 2 {
        return from_ordinary(mixture, ideal_gas, &reduced, t, p, z, &ordinary);
    }
    let mut phases = Vec::with_capacity(split.x.len());
    for (index, composition) in split.x.iter().enumerate() {
        let mole_fraction = split.phase_fractions[index];
        let root = split.z_factor[index];
        if !(mole_fraction > 0.0) || !root.is_finite() || root <= 0.0 {
            continue;
        }
        phases.push(phase_info(
            mixture,
            ideal_gas,
            &reduced,
            t,
            p,
            composition,
            root,
            mole_fraction,
        )?);
    }
    if phases.is_empty() {
        return Err(AzothError::invalid_input(
            "flash",
            "the multiphase flash kept no phase",
        ));
    }
    let water = water_index(mixture);
    let gas_like = ordinary.phase != Phase::AllLiquid;
    let gas_phase = if gas_like {
        Some(
            (0..phases.len())
                .max_by(|left, right| phases[*left].z_factor.total_cmp(&phases[*right].z_factor))
                .expect("the phase list is not empty"),
        )
    } else {
        None
    };
    let mut oil_phase = None;
    let mut aqueous_phase = None;
    for (index, phase) in phases.iter().enumerate() {
        if Some(index) == gas_phase {
            continue;
        }
        if is_aqueous(&phase.composition, water) {
            if aqueous_phase.is_none() {
                aqueous_phase = Some(index);
            }
        } else if oil_phase.is_none() {
            oil_phase = Some(index);
        }
    }
    Ok(assemble(t, p, phases, gas_phase, oil_phase, aqueous_phase))
}

fn from_ordinary(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    reduced: &ReducedParameters,
    t: azoth_core::units::ThermodynamicTemperature,
    p: azoth_core::units::Pressure,
    z: &[f64],
    flash: &azoth_eos::results::PtFlashResult,
) -> Result<FlashState> {
    let water = water_index(mixture);
    match flash.phase {
        Phase::TwoPhase => {
            let beta = flash.vapour_fraction.ok_or_else(|| {
                AzothError::invalid_input(
                    "flash",
                    "the flash split the feed and reported no vapour fraction",
                )
            })?;
            let vapour = phase_info(
                mixture,
                ideal_gas,
                reduced,
                t,
                p,
                &flash.y,
                flash.z_vapour,
                beta,
            )?;
            let liquid = phase_info(
                mixture,
                ideal_gas,
                reduced,
                t,
                p,
                &flash.x,
                flash.z_liquid,
                1.0 - beta,
            )?;
            let aqueous = is_aqueous(&liquid.composition, water);
            Ok(assemble(
                t,
                p,
                vec![vapour, liquid],
                Some(0),
                if aqueous { None } else { Some(1) },
                if aqueous { Some(1) } else { None },
            ))
        }
        Phase::AllVapour => {
            let root = mixture.phase_state(reduced, z, RootSide::Vapour)?.z;
            let phase = phase_info(mixture, ideal_gas, reduced, t, p, z, root, 1.0)?;
            Ok(assemble(t, p, vec![phase], Some(0), None, None))
        }
        Phase::AllLiquid => {
            let root = mixture.phase_state(reduced, z, RootSide::Liquid)?.z;
            let phase = phase_info(mixture, ideal_gas, reduced, t, p, z, root, 1.0)?;
            let aqueous = is_aqueous(z, water);
            Ok(assemble(
                t,
                p,
                vec![phase],
                None,
                if aqueous { None } else { Some(0) },
                if aqueous { Some(0) } else { None },
            ))
        }
        Phase::Trivial => Err(AzothError::invalid_input(
            "flash",
            "the feed flash converged to a trivial solution, which names no phase",
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn phase_info(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    reduced: &ReducedParameters,
    t: azoth_core::units::ThermodynamicTemperature,
    p: azoth_core::units::Pressure,
    composition: &[f64],
    root: f64,
    mole_fraction: f64,
) -> Result<PhaseInfo> {
    let molar_mass = molar_mass_of(mixture, composition)?;
    let shift = mixture.volume_shift(composition);
    let molar_volume = root * MOLAR_GAS_CONSTANT * t.value / p.value - shift;
    if !(molar_volume > 0.0 && molar_volume.is_finite()) {
        return Err(AzothError::invalid_input(
            "flash",
            format!(
                "the corrected molar volume at {:.6} Pa is not positive",
                p.value
            ),
        ));
    }
    let capacity = phase_capacity(mixture, ideal_gas, reduced, t, p, composition, root)?;
    Ok(PhaseInfo {
        mole_fraction,
        composition: composition.to_vec(),
        z_factor: root,
        molar_mass,
        molar_volume,
        density: molar_mass / molar_volume,
        cp: capacity.cp,
        cv: capacity.cv,
    })
}

fn assemble(
    t: azoth_core::units::ThermodynamicTemperature,
    p: azoth_core::units::Pressure,
    phases: Vec<PhaseInfo>,
    gas_phase: Option<usize>,
    oil_phase: Option<usize>,
    aqueous_phase: Option<usize>,
) -> FlashState {
    let mass: f64 = phases
        .iter()
        .map(|phase| phase.mole_fraction * phase.molar_mass)
        .sum();
    let volume: f64 = phases
        .iter()
        .map(|phase| phase.mole_fraction * phase.molar_volume)
        .sum();
    let cp: f64 = phases
        .iter()
        .map(|phase| phase.mole_fraction * phase.cp)
        .sum();
    let cv: f64 = phases
        .iter()
        .map(|phase| phase.mole_fraction * phase.cv)
        .sum();
    FlashState {
        temperature: t.value,
        pressure: p.value,
        bulk_density: mass / volume,
        gamma: cp / cv,
        gas_phase,
        oil_phase,
        aqueous_phase,
        phases,
    }
}

fn water_index(mixture: &Mixture) -> Option<usize> {
    mixture.index_of("water")
}

fn is_aqueous(composition: &[f64], water: Option<usize>) -> bool {
    water
        .and_then(|index| composition.get(index).copied())
        .is_some_and(|fraction| fraction > 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use azoth_eos::Cubic;
    use azoth_eos::databank::mixture_of;

    #[test]
    fn a_two_phase_state_carries_both_densities_and_a_ratio() {
        let (mixture, ideal_gas) =
            mixture_of(&["methane", "n-butane"], Cubic::Pr, None).expect("the pair resolves");
        let state = flash(&mixture, &ideal_gas, 330.0, 2.5e6, &[0.6, 0.4], false)
            .expect("the state flashes");
        assert!(state.has_gas());
        assert!(state.gas_density() > 0.0);
        assert!(state.liquid_density(800.0) > state.gas_density());
        assert!(state.gamma > 1.0 && state.gamma <= 2.0);
        assert!(state.bulk_density > 0.0);
    }
}
