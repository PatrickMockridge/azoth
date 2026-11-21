//! The two flashes the PVT experiments use, and the phases they answer with.
//!
//! `SaturationPressure`, `SaturationTemperature` and the separator train force NeqSim's
//! multiphase check on; the other experiments run a plain `TPflash` that can only see a
//! vapour and a liquid. [`FlashMode`] states which one an experiment uses, because the
//! classes' "two phases" predicate is the flash's own phase count and the two flashes do not
//! count the same way.
//!
//! A phase is labelled with NeqSim's `PhaseEos.init` rule,
//! [`azoth_eos::hydrate_inhibitor_wt::label`], and the phases are ordered Gas, Oil, Aqueous.
//! That ordering is what makes the classes' `phase(0) = gas` and `phase(1) = oil` reproducible
//! without pretending the multiphase solve promised an order.

use azoth_core::units::{Pressure, ThermodynamicTemperature, kelvins, pascals};
use azoth_core::{AzothError, Result, Warning};
use azoth_eos::hydrate_inhibitor_wt::{PhaseLabel, label};
use azoth_eos::{Mixture, ReducedParameters, RootSide};

use crate::volume;

/// Which flash an experiment asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashMode {
    /// NeqSim's `TPflash` without the multiphase check: a vapour and a liquid.
    TwoPhase,
    /// `TPmultiflash`, selected by the classes that set `setMultiPhaseCheck(true)`.
    Multiphase,
}

/// One phase of a flash.
#[derive(Debug, Clone, PartialEq)]
pub struct Phase {
    /// Its NeqSim phase type.
    pub label: PhaseLabel,
    /// Its mole fraction of the feed.
    pub fraction: f64,
    /// Its mole fractions.
    pub x: Vec<f64>,
    /// The cubic root it sits on.
    pub z_factor: f64,
    /// `ln phi_i` in the phase.
    pub ln_phi: Vec<f64>,
    /// Its molar mass, in kg/mol.
    pub molar_mass: f64,
    /// Its molar volume, translated when the fluid asks for it, in m3/mol.
    pub molar_volume: f64,
    /// Its molar volume before the translation, in m3/mol.
    pub molar_volume_uncorrected: f64,
}

impl Phase {
    /// Its mass density, in kg/m3.
    #[must_use]
    pub fn density(&self) -> f64 {
        self.molar_mass / self.molar_volume
    }

    /// Its extensive amount at the flash's total, in mol.
    #[must_use]
    pub fn moles(&self, total: f64) -> f64 {
        self.fraction * total
    }

    /// Its extensive volume at the flash's total, in m3.
    #[must_use]
    pub fn volume(&self, total: f64) -> f64 {
        self.moles(total) * self.molar_volume
    }

    /// Its extensive untranslated volume at the flash's total, in m3.
    #[must_use]
    pub fn volume_uncorrected(&self, total: f64) -> f64 {
        self.moles(total) * self.molar_volume_uncorrected
    }
}

/// A converged flash.
#[derive(Debug, Clone, PartialEq)]
pub struct Flash {
    /// The phases, ordered Gas, Oil, Aqueous, each with a non-negligible fraction.
    pub phases: Vec<Phase>,
    /// The flash's own phase count, before any fraction was filtered - the classes'
    /// `getNumberOfPhases()`.
    pub phase_count: u32,
    /// The feed's total amount, in mol.
    pub total_moles: f64,
    /// The temperature, in K.
    pub t: f64,
    /// The pressure, in Pa.
    pub p: f64,
    /// Caveats from the flash.
    pub warnings: Vec<Warning>,
}

impl Flash {
    /// The gas phase, if there is one.
    #[must_use]
    pub fn gas(&self) -> Option<&Phase> {
        self.phase_with(PhaseLabel::Gas)
    }

    /// The oil phase, if there is one.
    #[must_use]
    pub fn oil(&self) -> Option<&Phase> {
        self.phase_with(PhaseLabel::Oil)
    }

    /// The aqueous phase, if there is one.
    #[must_use]
    pub fn aqueous(&self) -> Option<&Phase> {
        self.phase_with(PhaseLabel::Aqueous)
    }

    fn phase_with(&self, wanted: PhaseLabel) -> Option<&Phase> {
        self.phases.iter().find(|phase| phase.label == wanted)
    }

    /// The extensive volume with the Peneloux translation applied where the fluid asks, m3.
    #[must_use]
    pub fn volume(&self) -> f64 {
        self.phases
            .iter()
            .map(|phase| phase.volume(self.total_moles))
            .sum()
    }

    /// The extensive untranslated volume, m3.
    #[must_use]
    pub fn volume_uncorrected(&self) -> f64 {
        self.phases
            .iter()
            .map(|phase| phase.volume_uncorrected(self.total_moles))
            .sum()
    }

    /// The flash's mass, in kg.
    #[must_use]
    pub fn mass(&self) -> f64 {
        self.phases
            .iter()
            .map(|phase| phase.moles(self.total_moles) * phase.molar_mass)
            .sum()
    }

    /// `sum_k beta_k Z_k`, the mixture compressibility.
    #[must_use]
    pub fn z_mix(&self) -> f64 {
        self.phases
            .iter()
            .map(|phase| phase.fraction * phase.z_factor)
            .sum()
    }

    /// `P V_corr/(n R T)` of the whole flash.
    #[must_use]
    pub fn zvolcorr(&self) -> f64 {
        self.p * self.volume() / (self.total_moles * crate::standard::NEQSIM_GAS_CONSTANT * self.t)
    }
}

/// Flash a feed and wrap its phases.
///
/// # Errors
/// * Propagates the flash's refusals, and a phase whose composition the labelling rule cannot
///   classify.
pub(crate) fn run(
    mixture: &Mixture,
    corrected: bool,
    total_moles: f64,
    z: &[f64],
    t: ThermodynamicTemperature,
    p: Pressure,
    mode: FlashMode,
) -> Result<Flash> {
    let reduced = mixture.reduced_parameters(t, p)?;
    let (mut phases, phase_count, warnings) = match mode {
        FlashMode::TwoPhase => two_phase(mixture, corrected, t.value, p.value, z, &reduced)?,
        FlashMode::Multiphase => multiflash(mixture, corrected, t.value, p.value, z, &reduced)?,
    };
    phases.sort_by_key(|phase| order(phase.label));
    Ok(Flash {
        phases,
        phase_count,
        total_moles,
        t: t.value,
        p: p.value,
        warnings,
    })
}

fn order(label: PhaseLabel) -> u8 {
    match label {
        PhaseLabel::Gas => 0,
        PhaseLabel::Oil => 1,
        PhaseLabel::Aqueous => 2,
    }
}

fn two_phase(
    mixture: &Mixture,
    corrected: bool,
    t: f64,
    p: f64,
    z: &[f64],
    reduced: &ReducedParameters,
) -> Result<(Vec<Phase>, u32, Vec<Warning>)> {
    let result = azoth_eos::pt_flash(mixture, kelvins(t), pascals(p), z)?;
    let mut phases = Vec::with_capacity(2);
    let count = match result.phase {
        azoth_eos::Phase::TwoPhase => {
            let beta = result.vapour_fraction.ok_or_else(|| {
                AzothError::invalid_input(
                    "flash",
                    "a two-phase result carries no vapour fraction, so the split is unknown",
                )
            })?;
            phases.push(build(
                mixture,
                corrected,
                t,
                p,
                reduced,
                &result.x,
                result.z_liquid,
                1.0 - beta,
                &result.ln_phi_liquid,
            )?);
            phases.push(build(
                mixture,
                corrected,
                t,
                p,
                reduced,
                &result.y,
                result.z_vapour,
                beta,
                &result.ln_phi_vapour,
            )?);
            2
        }
        azoth_eos::Phase::AllLiquid => {
            // **The single phase is the feed, not the negative flash's hypothetical
            // liquid.** NeqSim clamps `beta` to zero and re-initialises at `z`, so the root
            // and the fugacity coefficients are recomputed there; the negative flash's
            // `z_liquid` belongs to the composition it invented.
            let state = mixture.phase_state(reduced, z, RootSide::Liquid)?;
            phases.push(build(
                mixture,
                corrected,
                t,
                p,
                reduced,
                z,
                state.z,
                1.0,
                &state.ln_phi,
            )?);
            1
        }
        azoth_eos::Phase::AllVapour => {
            let state = mixture.phase_state(reduced, z, RootSide::Vapour)?;
            phases.push(build(
                mixture,
                corrected,
                t,
                p,
                reduced,
                z,
                state.z,
                1.0,
                &state.ln_phi,
            )?);
            1
        }
        azoth_eos::Phase::Trivial => {
            let (root, ln_phi) = stable_root(mixture, reduced, z)?;
            phases.push(build(
                mixture, corrected, t, p, reduced, z, root, 1.0, &ln_phi,
            )?);
            1
        }
    };
    Ok((phases, count, result.warnings))
}

/// The stable single-phase root when the flash converged to `x = y = z`.
///
/// `pt_flash` does not decide which phase a trivial solution is; the two cubic roots' Gibbs
/// energies do, and the lower is the state the feed is in.
fn stable_root(
    mixture: &Mixture,
    reduced: &ReducedParameters,
    z: &[f64],
) -> Result<(f64, Vec<f64>)> {
    let liquid = mixture.phase_state(reduced, z, RootSide::Liquid);
    let vapour = mixture.phase_state(reduced, z, RootSide::Vapour);
    let gibbs = |ln_phi: &[f64]| {
        z.iter()
            .zip(ln_phi)
            .map(|(fraction, ln)| fraction * ln)
            .sum::<f64>()
    };
    match (liquid, vapour) {
        (Ok(liquid), Ok(vapour)) => {
            if gibbs(&liquid.ln_phi) <= gibbs(&vapour.ln_phi) {
                Ok((liquid.z, liquid.ln_phi))
            } else {
                Ok((vapour.z, vapour.ln_phi))
            }
        }
        (Ok(state), Err(_)) | (Err(_), Ok(state)) => Ok((state.z, state.ln_phi)),
        (Err(first), Err(second)) => Err(AzothError::invalid_input(
            "flash",
            format!("neither cubic root is admissible: {first}; {second}"),
        )),
    }
}

fn multiflash(
    mixture: &Mixture,
    corrected: bool,
    t: f64,
    p: f64,
    z: &[f64],
    reduced: &ReducedParameters,
) -> Result<(Vec<Phase>, u32, Vec<Warning>)> {
    let result = azoth_eos::tp_multiflash(mixture, kelvins(t), pascals(p), z)?;
    let mut phases = Vec::with_capacity(result.x.len());
    for index in 0..result.x.len() {
        let fraction = result.phase_fractions[index];
        let z_factor = result.z_factor[index];
        if !(fraction.is_finite() && fraction > 1.1e-12 && z_factor.is_finite()) {
            continue;
        }
        let ln_phi = result.ln_phi.get(index).cloned().unwrap_or_default();
        phases.push(build(
            mixture,
            corrected,
            t,
            p,
            reduced,
            &result.x[index],
            z_factor,
            fraction,
            &ln_phi,
        )?);
    }
    Ok((phases, result.phase_count, result.warnings))
}

#[allow(clippy::too_many_arguments)]
fn build(
    mixture: &Mixture,
    corrected: bool,
    t: f64,
    p: f64,
    reduced: &ReducedParameters,
    x: &[f64],
    z_factor: f64,
    fraction: f64,
    ln_phi: &[f64],
) -> Result<Phase> {
    Ok(Phase {
        label: label(mixture, reduced, x, z_factor)?,
        fraction,
        x: x.to_vec(),
        z_factor,
        ln_phi: ln_phi.to_vec(),
        molar_mass: volume::phase_molar_mass(mixture, x)?,
        molar_volume: volume::corrected_molar_volume(mixture, x, z_factor, t, p, corrected),
        molar_volume_uncorrected: volume::molar_volume(z_factor, t, p),
    })
}
