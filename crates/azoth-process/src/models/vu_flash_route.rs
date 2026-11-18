//! The VU flash route the blowdown classes take.
//!
//! NeqSim's `ThermodynamicOperations.VUflash` dispatches: a pure feed that is subcritical and
//! whose internal energy lies between the saturated values takes `VUflashSingleComp`, which
//! reads the system's pressure and never reads the volume it is given; every other feed takes
//! `OptimizedVUflash`, the mixture iteration. The port tries the single-component route when
//! the mixture has one component and falls through to the mixture route when it refuses, which
//! is the class's own range check: an out-of-range pure state is exactly what the mixture
//! route answers.

use azoth_core::Result;
use azoth_core::units::{cubic_meters_per_mole, joules_per_mole, pascals};
use azoth_eos::mixture::Mixture;
use azoth_eos::molar_enthalpy_entropy::IdealGasModel;
use azoth_eos::results::Phase;
use azoth_eos::vu_flash::vu_flash;
use azoth_eos::vu_flash_single_comp::vu_flash_single_comp;

/// What a VU flash resolved: the pressure, temperature and, where it split, the phases.
///
/// The filling reads only the pressure and temperature; the phase split is read by
/// `process.depressurization`, which withdraws a phase.
#[allow(dead_code)]
pub(crate) struct VuOutcome {
    /// The pressure, Pa.
    pub pressure: f64,
    /// The temperature, K.
    pub temperature: f64,
    /// The phase the flash found.
    pub phase: Phase,
    /// The vapour fraction where the state is two-phase.
    pub vapour_fraction: Option<f64>,
    /// The liquid-phase composition.
    pub x: Vec<f64>,
    /// The vapour-phase composition.
    pub y: Vec<f64>,
}

/// The class's VU dispatch, over molar volume and molar internal energy.
///
/// # Errors
/// The mixture route's refusal; the pure route's refusal is not fatal and falls through.
pub(crate) fn vu_at(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    previous_pressure: f64,
    molar_volume: f64,
    molar_energy: f64,
    z: &[f64],
) -> Result<VuOutcome> {
    if mixture.len() == 1 {
        if let Ok(single) = vu_flash_single_comp(
            mixture,
            ideal_gas,
            pascals(previous_pressure),
            cubic_meters_per_mole(molar_volume),
            joules_per_mole(molar_energy),
        ) {
            return Ok(VuOutcome {
                // `VUflashSingleComp` does not solve for a new pressure: the pressure it is
                // given is the saturation pressure its temperature is found at.
                pressure: previous_pressure,
                temperature: single.t.value,
                phase: single.phase,
                vapour_fraction: Some(single.vapour_fraction),
                x: vec![1.0],
                y: vec![1.0],
            });
        }
    }
    let flash = vu_flash(
        mixture,
        ideal_gas,
        cubic_meters_per_mole(molar_volume),
        joules_per_mole(molar_energy),
        z,
    )?;
    Ok(VuOutcome {
        pressure: flash.pressure.value,
        temperature: flash.temperature.value,
        phase: flash.phase,
        vapour_fraction: flash.vapour_fraction,
        x: flash.x,
        y: flash.y,
    })
}
