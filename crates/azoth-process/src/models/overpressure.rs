//! The overpressure family's thermo-bound helpers.
//!
//! `ReliefFluidState` and `NozzleFlow` are package-private in NeqSim's
//! `process/safety/overpressure/`; this module is their single home, so the four relief loads
//! and the study do not each carry a copy. It is not an id: nothing here is registered, and the
//! spec of the id that reads it says which class the arithmetic belongs to.
//!
//! **`ReliefFluidState` reads `getKappa`, not `getGamma`.** NeqSim computes
//! `-Cp/Cv * V/P * getdPdVtn()`; the plain `Cp/Cv` is a different property, and the DTO's field
//! name (`specificHeatRatio`) does not say which one it carries. The capture
//! `process_blocked_outlet_relief.tsv` prints both beside each state: 1.27958 against 1.33194
//! for the gas row, and thousands on a liquid.
//!
//! **The density the class computes is read by no formula** and is not computed here: none of
//! the four loads publishes it, and the study's density is a scenario-row input.

use azoth_core::Result;
use azoth_core::units::{Pressure, ThermodynamicTemperature};
use azoth_eos::entropy_state::molar_mass_of;
use azoth_eos::heat_capacity::phase_capacity;
use azoth_eos::mixture::{Mixture, ReducedParameters, RootSide};
use azoth_eos::molar_enthalpy_entropy::IdealGasModel;
use azoth_eos::pr_molar_volume::MOLAR_GAS_CONSTANT;
use azoth_eos::pt_flash::pt_flash;
use azoth_eos::results::Phase;

use crate::models::relief_disposal_network::ReliefPhase;

/// `NozzleFlow`'s gas constant, J/(mol K).
pub(crate) const NOZZLE_R: f64 = 8.314_462_618;
/// The class's default specific-heat ratio when the flash answers a non-physical one.
pub(crate) const FALLBACK_KAPPA: f64 = 1.3;
/// The class's default molar mass, kg/mol.
pub(crate) const FALLBACK_MOLAR_MASS: f64 = 0.020;
/// The class's default compressibility.
pub(crate) const FALLBACK_COMPRESSIBILITY: f64 = 1.0;
/// The class's default downstream pressure, Pa: 1.01325 bara.
pub(crate) const DEFAULT_DOWNSTREAM_PRESSURE: f64 = 101_325.0;
/// The class's default relieving temperature, K.
pub(crate) const DEFAULT_TEMPERATURE: f64 = 288.15;

/// The relieving-fluid snapshot, and the class's warning strings in their order.
pub(crate) struct ReliefFluidState {
    pub specific_heat_ratio: f64,
    pub molar_mass: f64,
    pub compressibility: f64,
    pub phase: ReliefPhase,
    pub assumptions: Vec<String>,
}

impl ReliefFluidState {
    /// The class's outer fallback: default vapour properties and its one warning.
    fn failed() -> Self {
        Self {
            specific_heat_ratio: FALLBACK_KAPPA,
            molar_mass: FALLBACK_MOLAR_MASS,
            compressibility: FALLBACK_COMPRESSIBILITY,
            phase: ReliefPhase::Vapour,
            assumptions: vec![
                "Relief-fluid flash failed; using default vapour properties".to_string(),
            ],
        }
    }
}

/// One phase's capacity and molar mass at a root.
struct PhaseProperties {
    molar_mass: f64,
    cp: f64,
    cv: f64,
    d_p_d_v: f64,
    root: f64,
}

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
    let capacity = phase_capacity(mixture, ideal_gas, reduced, t, p, composition, root)?;
    Ok(PhaseProperties {
        molar_mass,
        cp: capacity.cp,
        cv: capacity.cv,
        d_p_d_v: capacity.d_p_d_v,
        root,
    })
}

/// `-Cp/Cv * V/P * dPdV` for one phase.
fn single_phase_kappa(phase: &PhaseProperties, t: f64, p: f64) -> f64 {
    let molar_volume = phase.root * MOLAR_GAS_CONSTANT * t / p;
    -(phase.cp / phase.cv) * (molar_volume / p) * phase.d_p_d_v
}

/// The same expression over the two phases, with NeqSim's own weighting.
///
/// `getCp`/`getCv` are the phase totals, `getVolume()` is the beta-weighted molar volume, and
/// `getdPdVtn()` sums each phase's derivative with respect to *that phase's* total volume, which
/// is `dP/dv_i` over `beta_i`.
fn two_phase_kappa(
    vapour: &PhaseProperties,
    liquid: &PhaseProperties,
    beta: f64,
    t: f64,
    p: f64,
) -> f64 {
    let cp = beta * vapour.cp + (1.0 - beta) * liquid.cp;
    let cv = beta * vapour.cv + (1.0 - beta) * liquid.cv;
    let molar_volume =
        (beta * vapour.root + (1.0 - beta) * liquid.root) * MOLAR_GAS_CONSTANT * t / p;
    let d_p_d_v = vapour.d_p_d_v / beta + liquid.d_p_d_v / (1.0 - beta);
    -(cp / cv) * (molar_volume / p) * d_p_d_v
}

/// `ReliefFluidState.evaluate`: flash at the relieving conditions and read kappa, M, Z and phase.
///
/// The class's non-physical-value fallbacks are reproduced, and their warning strings are
/// returned in `assumptions` exactly as `ReliefScenario.getAssumptions()` carries them.
pub(crate) fn relief_fluid_state(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    t: ThermodynamicTemperature,
    p: Pressure,
    z: &[f64],
) -> ReliefFluidState {
    let mut assumptions = Vec::new();
    let reduced = match mixture.reduced_parameters(t, p) {
        Ok(reduced) => reduced,
        Err(_) => return ReliefFluidState::failed(),
    };
    let flash = match pt_flash(mixture, t, p, z) {
        Ok(flash) => flash,
        Err(_) => return ReliefFluidState::failed(),
    };

    let (phase, kappa, molar_mass, compressibility) = match flash.phase {
        Phase::TwoPhase => {
            let Some(beta) = flash.vapour_fraction else {
                return ReliefFluidState::failed();
            };
            if !(0.0..=1.0).contains(&beta) || beta <= 0.0 || beta >= 1.0 {
                return ReliefFluidState::failed();
            }
            let vapour = match phase_properties(
                mixture,
                ideal_gas,
                &reduced,
                t,
                p,
                &flash.y,
                flash.z_vapour,
            ) {
                Ok(phase) => phase,
                Err(_) => return ReliefFluidState::failed(),
            };
            let liquid = match phase_properties(
                mixture,
                ideal_gas,
                &reduced,
                t,
                p,
                &flash.x,
                flash.z_liquid,
            ) {
                Ok(phase) => phase,
                Err(_) => return ReliefFluidState::failed(),
            };
            let kappa = two_phase_kappa(&vapour, &liquid, beta, t.value, p.value);
            let molar_mass = beta * vapour.molar_mass + (1.0 - beta) * liquid.molar_mass;
            let compressibility = beta * vapour.root + (1.0 - beta) * liquid.root;
            (ReliefPhase::TwoPhase, kappa, molar_mass, compressibility)
        }
        Phase::AllVapour | Phase::AllLiquid => {
            let side = if flash.phase == Phase::AllVapour {
                RootSide::Vapour
            } else {
                RootSide::Liquid
            };
            let root = match mixture.phase_state(&reduced, z, side) {
                Ok(state) => state.z,
                Err(_) => return ReliefFluidState::failed(),
            };
            let state = match phase_properties(mixture, ideal_gas, &reduced, t, p, z, root) {
                Ok(state) => state,
                Err(_) => return ReliefFluidState::failed(),
            };
            let phase = if flash.phase == Phase::AllVapour {
                ReliefPhase::Vapour
            } else {
                ReliefPhase::Liquid
            };
            let kappa = single_phase_kappa(&state, t.value, p.value);
            (phase, kappa, state.molar_mass, state.root)
        }
        Phase::Trivial => return ReliefFluidState::failed(),
    };

    let (specific_heat_ratio, kappa_warning) = checked_kappa(kappa);
    if let Some(warning) = kappa_warning {
        assumptions.push(warning.to_string());
    }
    let (molar_mass, molar_mass_warning) = checked_molar_mass(molar_mass);
    if let Some(warning) = molar_mass_warning {
        assumptions.push(warning.to_string());
    }
    let compressibility = if compressibility.is_finite() && compressibility > 0.0 {
        compressibility
    } else {
        FALLBACK_COMPRESSIBILITY
    };
    ReliefFluidState {
        specific_heat_ratio,
        molar_mass,
        compressibility,
        phase,
        assumptions,
    }
}

/// The class's `getKappa` check: finite and above one, else 1.3 with its warning.
fn checked_kappa(kappa: f64) -> (f64, Option<&'static str>) {
    if kappa.is_finite() && kappa > 1.0 {
        (kappa, None)
    } else {
        (
            FALLBACK_KAPPA,
            Some(
                "Fluid Cp/Cv non-physical at relief conditions; using default specific-heat ratio 1.3",
            ),
        )
    }
}

/// The class's molar-mass check: finite and positive, else 0.020 kg/mol with its warning.
fn checked_molar_mass(molar_mass: f64) -> (f64, Option<&'static str>) {
    if molar_mass.is_finite() && molar_mass > 0.0 {
        (molar_mass, None)
    } else {
        (
            FALLBACK_MOLAR_MASS,
            Some("Fluid molar mass non-physical; using default 0.020 kg/mol"),
        )
    }
}

/// `NozzleFlow.criticalPressureRatio`: the choked ratio `(2/(k+1))^(k/(k-1))`.
#[must_use]
pub(crate) fn critical_pressure_ratio(k: f64) -> f64 {
    (2.0 / (k + 1.0)).powf(k / (k - 1.0))
}

/// `NozzleFlow.gasMassRateKgPerS`: the choked or subcritical isentropic gas flow, kg/s.
#[allow(clippy::too_many_arguments)] // one argument per quantity the class method reads
pub(crate) fn gas_mass_rate(
    discharge_coefficient: f64,
    area_m2: f64,
    upstream_pressure_pa: f64,
    downstream_pressure_pa: f64,
    upstream_temperature_k: f64,
    molar_mass_kg_per_mol: f64,
    specific_heat_ratio: f64,
) -> f64 {
    let k = specific_heat_ratio;
    let ratio = downstream_pressure_pa / upstream_pressure_pa;
    if ratio <= critical_pressure_ratio(k) {
        return discharge_coefficient
            * area_m2
            * upstream_pressure_pa
            * (k * molar_mass_kg_per_mol / (NOZZLE_R * upstream_temperature_k)
                * (2.0 / (k + 1.0)).powf((k + 1.0) / (k - 1.0)))
            .sqrt();
    }
    let term = ratio.powf(2.0 / k) - ratio.powf((k + 1.0) / k);
    let term = if term < 0.0 { 0.0 } else { term };
    discharge_coefficient
        * area_m2
        * upstream_pressure_pa
        * (2.0 * molar_mass_kg_per_mol / (NOZZLE_R * upstream_temperature_k)
            * (k / (k - 1.0))
            * term)
            .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_critical_ratio_is_the_classes() {
        // (2/(1.3+1))^(1.3/0.3) = 0.545727733814065
        assert!((critical_pressure_ratio(1.3) - 0.545_727_733_814_065).abs() < 1e-12);
    }

    #[test]
    fn a_downstream_at_the_upstream_pressure_answers_zero() {
        let rate = gas_mass_rate(0.85, 1.0e-4, 1.0e6, 1.0e6, 300.0, 0.02, 1.3);
        assert!(rate.abs() < 1e-12, "{rate}");
    }

    #[test]
    fn a_non_physical_kappa_takes_the_classes_default_and_warning() {
        for bad in [f64::NAN, f64::INFINITY, 0.9, 1.0] {
            let (value, warning) = checked_kappa(bad);
            assert_eq!(value, FALLBACK_KAPPA);
            assert_eq!(
                warning,
                Some(
                    "Fluid Cp/Cv non-physical at relief conditions; using default specific-heat ratio 1.3"
                )
            );
        }
        assert_eq!(
            checked_kappa(1.279_579_897_384_378_6),
            (1.279_579_897_384_378_6, None)
        );
    }

    #[test]
    fn a_non_physical_molar_mass_takes_the_classes_default_and_warning() {
        for bad in [f64::NAN, f64::NEG_INFINITY, 0.0, -0.02] {
            let (value, warning) = checked_molar_mass(bad);
            assert_eq!(value, FALLBACK_MOLAR_MASS);
            assert_eq!(
                warning,
                Some("Fluid molar mass non-physical; using default 0.020 kg/mol")
            );
        }
        assert_eq!(checked_molar_mass(0.017_445_7), (0.017_445_7, None));
    }
}
