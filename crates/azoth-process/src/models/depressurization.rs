//! `process.depressurization` - the equilibrium vessel blowdown.
//!
//! Spec: `specs/models/process/depressurization.toml`. The arithmetic is
//! `process/safety/depressurization/DepressurizationSimulator.java` at the pin: the mole
//! alignment, the discharge selection, the mass and energy balance, the lumped wall, the
//! fire boundary, the VU flash and the API 521 summary.

use azoth_core::units::{
    Area, HeatTransfer, Length, Mass, MassRate, Power, Pressure, SpecificHeatCapacity,
    ThermodynamicTemperature, Time, Volume, kelvins, kilograms, kilograms_per_second, pascals,
    seconds, watts,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use azoth_eos::cubic::Cubic;
use azoth_eos::databank;
use azoth_eos::entropy_state::molar_mass_of;
use azoth_eos::heat_capacity::phase_capacity;
use azoth_eos::mixture::{Mixture, ReducedParameters, RootSide};
use azoth_eos::molar_enthalpy_entropy::{IdealGasModel, molar_enthalpy_entropy};
use azoth_eos::pr_molar_volume::MOLAR_GAS_CONSTANT;
use azoth_eos::pt_flash::pt_flash;
use azoth_eos::results::Phase;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::vu_flash_route::vu_at;

/// The class's gas constant, J/(mol K).
const GAS_CONSTANT: f64 = 8.314;
/// Stefan-Boltzmann constant, W/(m2 K4).
const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;
/// The density ceiling above which a phase is not the vapour leaving the valve, kg/m3.
const VAPOUR_PHASE_MAX_DENSITY: f64 = 300.0;
/// The largest share of the vapour phase one step removes.
const MAX_VAPOUR_FRACTION_PER_STEP: f64 = 0.25;
/// The class's fallback capacity ratio.
const FALLBACK_GAMMA: f64 = 1.3;
/// The API 521 criterion horizon, s.
const CRITERION_HORIZON_S: f64 = 900.0;
/// The 7 barg absolute pressure, Pa.
const SEVEN_BARG_PA: f64 = 8.0e5;

/// Which inventory leaves through the valve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WithdrawalMode {
    /// Vapour when multiphase, bulk otherwise.
    Auto,
    /// The bulk composition.
    Bulk,
    /// The lightest phase.
    Vapour,
}

impl WithdrawalMode {
    fn parse(name: Option<&str>) -> Result<Self> {
        match name.unwrap_or("auto") {
            "auto" => Ok(Self::Auto),
            "bulk" => Ok(Self::Bulk),
            "vapour" => Ok(Self::Vapour),
            other => Err(AzothError::invalid_input(
                "withdrawal_mode",
                format!("`{other}` is not `auto`, `bulk` or `vapour`"),
            )),
        }
    }
}

/// One recorded step of a blowdown.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DepressurizationPoint {
    /// The step's time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
    /// The vessel pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// The fluid temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// The vessel inventory.
    #[serde(serialize_with = "scalar")]
    pub mass: Mass,
    /// The wall metal temperature.
    #[serde(serialize_with = "scalar")]
    pub wall_temperature: ThermodynamicTemperature,
    /// The instantaneous discharge rate.
    #[serde(serialize_with = "scalar")]
    pub mass_flow: MassRate,
}

/// Result of `process.depressurization`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DepressurizationResult {
    /// One row per step.
    pub points: Vec<DepressurizationPoint>,
    /// The pre-alignment flash's pressure.
    #[serde(serialize_with = "scalar")]
    pub initial_pressure: Pressure,
    /// The constant fire duty.
    #[serde(serialize_with = "scalar")]
    pub fire_heat_input: Power,
    /// Whether the fire duty is routed through the wall.
    pub fire_heat_routed_to_wall: bool,
    /// The steps that used the ideal-gas fallback.
    pub vu_flash_fallback_count: f64,
    /// Whether any step discharged the vapour phase.
    pub vapour_withdrawal_used: bool,
    /// The interpolated time to half the initial pressure.
    #[serde(serialize_with = "optional_scalar")]
    pub time_to_half_pressure: Option<Time>,
    /// The interpolated time to 8 bara absolute.
    #[serde(serialize_with = "optional_scalar")]
    pub time_to_7_barg: Option<Time>,
    /// The coldest recorded fluid temperature.
    #[serde(serialize_with = "scalar")]
    pub min_fluid_temperature: ThermodynamicTemperature,
    /// The coldest recorded wall temperature.
    #[serde(serialize_with = "scalar")]
    pub min_wall_temperature: ThermodynamicTemperature,
    /// The highest recorded pressure.
    #[serde(serialize_with = "scalar")]
    pub max_pressure: Pressure,
    /// The hottest recorded fluid temperature.
    #[serde(serialize_with = "scalar")]
    pub max_fluid_temperature: ThermodynamicTemperature,
    /// The hottest recorded wall temperature.
    #[serde(serialize_with = "scalar")]
    pub max_wall_temperature: ThermodynamicTemperature,
    /// Whether the half-pressure crossing is at or within 900 s.
    pub half_pressure_criterion_met: bool,
    /// Whether the 8 bara crossing is at or within 900 s.
    pub seven_barg_criterion_met: bool,
    /// Whether the pressure never rose beyond tolerance.
    pub pressure_monotonic_non_increasing: bool,
    /// Whether the inventory never grew beyond tolerance.
    pub mass_monotonic_non_increasing: bool,
    /// The recorded rows.
    pub recorded_points: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One phase's properties at its own composition and root.
struct PhaseProperties {
    composition: Vec<f64>,
    molar_mass: f64,
    density: f64,
    cp: f64,
    cv: f64,
    molar_enthalpy: f64,
}

#[allow(clippy::too_many_arguments)] // one parameter per property the flash exposes
fn phase_properties(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    reduced: &ReducedParameters,
    temperature: ThermodynamicTemperature,
    pressure: Pressure,
    composition: &[f64],
    root: f64,
) -> Result<PhaseProperties> {
    let molar_mass = molar_mass_of(mixture, composition)?;
    let shift = mixture.volume_shift(composition);
    let molar_volume = root * MOLAR_GAS_CONSTANT * temperature.value / pressure.value - shift;
    if !molar_volume.is_finite() || molar_volume <= 0.0 {
        return Err(AzothError::invalid_input(
            "initial_p",
            format!("the corrected molar volume is {molar_volume} m3/mol"),
        ));
    }
    let capacity = phase_capacity(
        mixture,
        ideal_gas,
        reduced,
        temperature,
        pressure,
        composition,
        root,
    )?;
    let enthalpy =
        molar_enthalpy_entropy(mixture, ideal_gas, temperature, pressure, composition, root)?;
    Ok(PhaseProperties {
        composition: composition.to_vec(),
        molar_mass,
        density: molar_mass / molar_volume,
        cp: capacity.cp,
        cv: capacity.cv,
        molar_enthalpy: enthalpy.h.value,
    })
}

/// The flashed state's phases and its bulk properties.
struct FlashInfo {
    vapour_fraction: Option<f64>,
    phases: Vec<PhaseProperties>,
    bulk_molar_mass: f64,
    bulk_density: f64,
    bulk_cp: f64,
    bulk_cv: f64,
    bulk_molar_enthalpy: f64,
}

fn flash_info(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    temperature: ThermodynamicTemperature,
    pressure: Pressure,
    z: &[f64],
) -> Result<FlashInfo> {
    let reduced = mixture.reduced_parameters(temperature, pressure)?;
    let flash = pt_flash(mixture, temperature, pressure, z)?;
    match flash.phase {
        Phase::TwoPhase => {
            let beta = flash.vapour_fraction.ok_or_else(|| {
                AzothError::invalid_input(
                    "initial_p",
                    "the flash split the feed and reported no vapour fraction",
                )
            })?;
            let vapour = phase_properties(
                mixture,
                ideal_gas,
                &reduced,
                temperature,
                pressure,
                &flash.y,
                flash.z_vapour,
            )?;
            let liquid = phase_properties(
                mixture,
                ideal_gas,
                &reduced,
                temperature,
                pressure,
                &flash.x,
                flash.z_liquid,
            )?;
            let mass = beta * vapour.molar_mass + (1.0 - beta) * liquid.molar_mass;
            let volume = beta * (vapour.molar_mass / vapour.density)
                + (1.0 - beta) * (liquid.molar_mass / liquid.density);
            let bulk_cp = beta * vapour.cp + (1.0 - beta) * liquid.cp;
            let bulk_cv = beta * vapour.cv + (1.0 - beta) * liquid.cv;
            let bulk_molar_enthalpy =
                beta * vapour.molar_enthalpy + (1.0 - beta) * liquid.molar_enthalpy;
            Ok(FlashInfo {
                vapour_fraction: Some(beta),
                phases: vec![liquid, vapour],
                bulk_molar_mass: mass,
                bulk_density: mass / volume,
                bulk_cp,
                bulk_cv,
                bulk_molar_enthalpy,
            })
        }
        Phase::AllVapour | Phase::AllLiquid => {
            let side = if matches!(flash.phase, Phase::AllVapour) {
                RootSide::Vapour
            } else {
                RootSide::Liquid
            };
            let root = mixture.phase_state(&reduced, z, side)?.z;
            let phase =
                phase_properties(mixture, ideal_gas, &reduced, temperature, pressure, z, root)?;
            Ok(FlashInfo {
                vapour_fraction: None,
                bulk_molar_mass: phase.molar_mass,
                bulk_density: phase.density,
                bulk_cp: phase.cp,
                bulk_cv: phase.cv,
                bulk_molar_enthalpy: phase.molar_enthalpy,
                phases: vec![phase],
            })
        }
        Phase::Trivial => Err(AzothError::invalid_input(
            "initial_p",
            "the flash found a trivial solution at the initial state",
        )),
    }
}

/// The state a VU flash resolved, built from the flash's own phase split rather than a
/// re-flash: the class carries the VU state forward and reads the discharge from it.
#[allow(clippy::too_many_arguments)] // one parameter per field the flash returned
fn info_from_flash(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    temperature: ThermodynamicTemperature,
    pressure: Pressure,
    phase: Phase,
    vapour_fraction: Option<f64>,
    x: &[f64],
    y: &[f64],
    z: &[f64],
) -> Result<FlashInfo> {
    let reduced = mixture.reduced_parameters(temperature, pressure)?;
    if let (Phase::TwoPhase, Some(beta)) = (phase, vapour_fraction) {
        let vapour_root = mixture.phase_state(&reduced, y, RootSide::Vapour)?.z;
        let liquid_root = mixture.phase_state(&reduced, x, RootSide::Liquid)?.z;
        let vapour = phase_properties(
            mixture,
            ideal_gas,
            &reduced,
            temperature,
            pressure,
            y,
            vapour_root,
        )?;
        let liquid = phase_properties(
            mixture,
            ideal_gas,
            &reduced,
            temperature,
            pressure,
            x,
            liquid_root,
        )?;
        let mass = beta * vapour.molar_mass + (1.0 - beta) * liquid.molar_mass;
        let volume = beta * (vapour.molar_mass / vapour.density)
            + (1.0 - beta) * (liquid.molar_mass / liquid.density);
        let bulk_cp = beta * vapour.cp + (1.0 - beta) * liquid.cp;
        let bulk_cv = beta * vapour.cv + (1.0 - beta) * liquid.cv;
        let bulk_molar_enthalpy =
            beta * vapour.molar_enthalpy + (1.0 - beta) * liquid.molar_enthalpy;
        Ok(FlashInfo {
            vapour_fraction: Some(beta),
            phases: vec![liquid, vapour],
            bulk_molar_mass: mass,
            bulk_density: mass / volume,
            bulk_cp,
            bulk_cv,
            bulk_molar_enthalpy,
        })
    } else {
        let side = if matches!(phase, Phase::AllVapour) {
            RootSide::Vapour
        } else {
            RootSide::Liquid
        };
        let root = mixture.phase_state(&reduced, z, side)?.z;
        let single =
            phase_properties(mixture, ideal_gas, &reduced, temperature, pressure, z, root)?;
        Ok(FlashInfo {
            vapour_fraction: None,
            bulk_molar_mass: single.molar_mass,
            bulk_density: single.density,
            bulk_cp: single.cp,
            bulk_cv: single.cv,
            bulk_molar_enthalpy: single.molar_enthalpy,
            phases: vec![single],
        })
    }
}

/// The class's orifice rate: choked or subsonic, NaN and negative floored at zero.
#[allow(clippy::too_many_arguments)] // one parameter per quantity the relation reads
fn orifice_rate(
    area: f64,
    coefficient: f64,
    pressure: f64,
    temperature: f64,
    molar_mass: f64,
    gamma: f64,
    compressibility: f64,
    back_pressure: f64,
) -> f64 {
    let critical_ratio = (2.0 / (gamma + 1.0)).powf(gamma / (gamma - 1.0));
    let pressure_ratio = back_pressure / pressure;
    let mut rate = if pressure_ratio <= critical_ratio {
        coefficient
            * area
            * pressure
            * (gamma * molar_mass / (compressibility * GAS_CONSTANT * temperature)).sqrt()
            * (2.0 / (gamma + 1.0)).powf((gamma + 1.0) / (2.0 * (gamma - 1.0)))
    } else {
        let mut term = (2.0 * gamma / (gamma - 1.0))
            * (pressure_ratio.powf(2.0 / gamma) - pressure_ratio.powf((gamma + 1.0) / gamma));
        if term < 0.0 {
            term = 0.0;
        }
        coefficient
            * area
            * pressure
            * (molar_mass / (compressibility * GAS_CONSTANT * temperature)).sqrt()
            * term.sqrt()
    };
    if rate < 0.0 || rate.is_nan() {
        rate = 0.0;
    }
    rate
}

/// The fire preset's `(emissivity, flame temperature, convection coefficient)`.
fn preset_parameters(name: &str) -> Result<(f64, f64, f64)> {
    match name {
        "pool_fire_peak" => Ok((0.9, 1100.0, 30.0)),
        "pool_fire_background" => Ok((0.9, 950.0, 30.0)),
        "jet_fire_peak" => Ok((1.0, 1300.0, 100.0)),
        "jet_fire_background" => Ok((0.9, 1100.0, 50.0)),
        other => Err(AzothError::invalid_input(
            "fire_preset",
            format!(
                "`{other}` is not `pool_fire_peak`, `pool_fire_background`, `jet_fire_peak` or `jet_fire_background`"
            ),
        )),
    }
}

/// The equilibrium vessel blowdown.
///
/// # Errors
/// `InvalidInput` for a non-positive dimension, a coefficient outside `(0, 1]`, a fire
/// ambiguity, an unknown mode or preset, a non-finite input, or a feed state the flash
/// cannot resolve.
#[allow(clippy::too_many_arguments)] // one parameter per quantity the class reads
pub fn depressurization(
    components: &[String],
    initial_z: &[f64],
    initial_t: ThermodynamicTemperature,
    initial_p: Pressure,
    vessel_volume: Volume,
    orifice_diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
    fire_heat_input: Option<f64>,
    fire_heat_input_to_wall: Option<bool>,
    fire_exposed_area: Option<Area>,
    fire_preset: Option<&str>,
    fire_emissivity: Option<f64>,
    fire_flame_temperature: Option<ThermodynamicTemperature>,
    fire_convective_coefficient: Option<HeatTransfer>,
    wall_mass: Option<Mass>,
    wall_area: Option<Area>,
    wall_specific_heat: Option<SpecificHeatCapacity>,
    wall_heat_transfer_coefficient: Option<HeatTransfer>,
    time_step: Option<Time>,
    max_time: Option<Time>,
    stop_pressure: Option<Pressure>,
    withdrawal_mode: Option<&str>,
) -> Result<DepressurizationResult> {
    let spec = &model_gen::DEPRESSURIZATION_SPEC;
    let mut warnings = Vec::new();
    let duty = fire_heat_input.unwrap_or(0.0);
    let route_to_wall = fire_heat_input_to_wall.unwrap_or(false);
    let exposed_area = fire_exposed_area.map_or(0.0, |value| value.value);
    let wall_metal = wall_mass.map_or(0.0, |value| value.value);
    let wall_face = wall_area.map_or(0.0, |value| value.value);
    let wall_capacity = wall_specific_heat.map_or(470.0, |value| value.value);
    let wall_film = wall_heat_transfer_coefficient.map_or(50.0, |value| value.value);
    let step_seconds = time_step.map_or(1.0, |value| value.value);
    let span = max_time.map_or(900.0, |value| value.value);
    let stop = stop_pressure.map_or(1.5e5, |value| value.value);
    let mode = WithdrawalMode::parse(withdrawal_mode)?;

    let explicit_fire = fire_emissivity.is_some()
        || fire_flame_temperature.is_some()
        || fire_convective_coefficient.is_some();
    let (emissivity, flame_temperature, convection) = match fire_preset {
        Some(name) => {
            if explicit_fire {
                return Err(AzothError::invalid_input(
                    "fire_preset",
                    "the preset and the explicit flame parameters are two statements of one boundary",
                ));
            }
            preset_parameters(name)?
        }
        None => (
            fire_emissivity.unwrap_or(0.9),
            fire_flame_temperature.map_or(1100.0, |value| value.value),
            fire_convective_coefficient.map_or(30.0, |value| value.value),
        ),
    };

    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "initial_t" => Some(initial_t.value),
            "initial_p" => Some(initial_p.value),
            "vessel_volume" => Some(vessel_volume.value),
            "orifice_diameter" => Some(orifice_diameter.value),
            "discharge_coefficient" => Some(discharge_coefficient),
            "back_pressure" => Some(back_pressure.value),
            "time_step" => Some(step_seconds),
            "max_time" => Some(span),
            "stop_pressure" => Some(stop),
            _ => None,
        },
        &mut warnings,
    )?;

    for (name, value) in [
        ("initial_t", initial_t.value),
        ("initial_p", initial_p.value),
        ("vessel_volume", vessel_volume.value),
        ("orifice_diameter", orifice_diameter.value),
        ("discharge_coefficient", discharge_coefficient),
        ("back_pressure", back_pressure.value),
        ("fire_heat_input", duty),
        ("fire_exposed_area", exposed_area),
        ("fire_emissivity", emissivity),
        ("fire_flame_temperature", flame_temperature),
        ("fire_convective_coefficient", convection),
        ("wall_mass", wall_metal),
        ("wall_area", wall_face),
        ("wall_specific_heat", wall_capacity),
        ("wall_heat_transfer_coefficient", wall_film),
        ("time_step", step_seconds),
        ("max_time", span),
        ("stop_pressure", stop),
    ] {
        if !value.is_finite() {
            return Err(AzothError::invalid_input(
                name,
                "a non-finite value is not a number this calculation publishes",
            ));
        }
    }
    if vessel_volume.value <= 0.0
        || orifice_diameter.value <= 0.0
        || initial_t.value <= 0.0
        || initial_p.value <= 0.0
    {
        return Err(AzothError::invalid_input(
            "vessel_volume",
            "the class requires the volume, orifice and initial state to be positive",
        ));
    }
    if discharge_coefficient <= 0.0 || discharge_coefficient > 1.0 {
        return Err(AzothError::invalid_input(
            "discharge_coefficient",
            "the port refuses a coefficient outside (0, 1]",
        ));
    }
    if back_pressure.value < 0.0 {
        return Err(AzothError::invalid_input(
            "back_pressure",
            "a negative receiving pressure is not a pressure",
        ));
    }
    if step_seconds <= 0.0 || span <= 0.0 || stop <= 0.0 {
        return Err(AzothError::invalid_input(
            "time_step",
            "the class refuses a non-positive step, horizon or stop pressure",
        ));
    }

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, Cubic::Pr, None)?;
    if initial_z.len() != mixture.len() {
        return Err(AzothError::invalid_input(
            "initial_z",
            format!(
                "a mixture of {} components needs that many mole fractions, and {initial_z:?} has {}",
                mixture.len(),
                initial_z.len()
            ),
        ));
    }
    azoth_eos::phase_boundary::check_composition(initial_z, mixture.len(), "initial_z")?;
    if initial_z.iter().any(|value| !value.is_finite()) {
        return Err(AzothError::invalid_input(
            "initial_z",
            "a mole fraction must be finite",
        ));
    }

    let p0 = initial_p.value;
    let mut state = flash_info(&mixture, &ideal_gas, initial_t, initial_p, initial_z)?;
    let mut composition = initial_z.to_vec();
    let mut pressure = initial_p.value;
    let mut temperature = initial_t.value;
    let mut mass = state.bulk_density * vessel_volume.value;
    let mut moles = mass / state.bulk_molar_mass;
    let mut energy = state.bulk_molar_enthalpy * moles - pressure * vessel_volume.value;
    let mut wall_temperature = temperature;
    let mut fallbacks = 0.0;
    let mut vapour_used = false;
    let mut time = 0.0;
    let area = std::f64::consts::PI * 0.25 * orifice_diameter.value * orifice_diameter.value;
    let mut points = vec![DepressurizationPoint {
        time: seconds(time),
        pressure: pascals(pressure),
        temperature: kelvins(temperature),
        mass: kilograms(mass),
        wall_temperature: kelvins(wall_temperature),
        mass_flow: kilograms_per_second(0.0),
    }];
    let mut min_fluid_temperature = temperature;
    let mut min_wall_temperature = wall_temperature;
    let mut max_pressure = pressure;
    let mut max_fluid_temperature = temperature;
    let mut max_wall_temperature = wall_temperature;

    while time < span && pressure > stop && mass > 1.0e-6 {
        let selected = match mode {
            WithdrawalMode::Bulk => None,
            _ => state
                .phases
                .iter()
                .enumerate()
                .min_by(|(_, first), (_, second)| {
                    first
                        .density
                        .partial_cmp(&second.density)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .filter(|(_, phase)| phase.density <= VAPOUR_PHASE_MAX_DENSITY)
                .filter(|_| state.vapour_fraction.is_some())
                .map(|(index, _)| index),
        };
        let (molar_mass, cp, cv, density, enthalpy, phase_composition) = match selected {
            Some(index) => {
                let phase = &state.phases[index];
                (
                    phase.molar_mass,
                    phase.cp,
                    phase.cv,
                    phase.density,
                    phase.molar_enthalpy,
                    phase.composition.clone(),
                )
            }
            None => (
                state.bulk_molar_mass,
                state.bulk_cp,
                state.bulk_cv,
                state.bulk_density,
                state.bulk_molar_enthalpy,
                Vec::new(),
            ),
        };
        let gamma = if cv > 0.0 { cp / cv } else { FALLBACK_GAMMA };
        let mut compressibility = pressure * molar_mass / (density * GAS_CONSTANT * temperature);
        if compressibility <= 0.0 || !compressibility.is_finite() {
            compressibility = 1.0;
        }
        let rate = orifice_rate(
            area,
            discharge_coefficient,
            pressure,
            temperature,
            molar_mass,
            gamma,
            compressibility,
            back_pressure.value,
        );
        let mut dm = (rate * step_seconds).min(mass);
        let specific_enthalpy = enthalpy / molar_mass;

        let fire_through_wall = route_to_wall && wall_metal > 0.0 && wall_face > 0.0;
        let effective_fire = if exposed_area > 0.0 {
            let surface = if fire_through_wall {
                wall_temperature
            } else {
                temperature
            };
            let radiative =
                emissivity * STEFAN_BOLTZMANN * (flame_temperature.powi(4) - surface.powi(4));
            let convective = convection * (flame_temperature - surface);
            (radiative + convective).max(0.0) * exposed_area
        } else {
            duty
        };
        let direct_fire = if fire_through_wall {
            0.0
        } else {
            effective_fire
        };
        let q_wall = if wall_metal > 0.0 {
            wall_film * wall_face * (wall_temperature - temperature)
        } else {
            0.0
        };
        if wall_metal > 0.0 {
            let external = if fire_through_wall {
                effective_fire
            } else {
                0.0
            };
            wall_temperature += (external - q_wall) * step_seconds / (wall_metal * wall_capacity);
        }
        let u_before = energy;
        let mut new_moles = moles;
        let mut new_composition = composition.clone();
        let new_mass;
        if let Some(index) = selected {
            vapour_used = true;
            let beta = state
                .vapour_fraction
                .expect("the vapour selection requires a split");
            let phase_moles = if index == 1 {
                beta * moles
            } else {
                (1.0 - beta) * moles
            };
            let requested = dm / molar_mass;
            let removed = requested.min(phase_moles * MAX_VAPOUR_FRACTION_PER_STEP);
            dm = removed * molar_mass;
            new_moles = (moles - removed).max(0.0);
            if new_moles > 0.0 {
                new_composition = composition
                    .iter()
                    .zip(phase_composition.iter())
                    .map(|(bulk, phase_fraction)| {
                        (moles * bulk - removed * phase_fraction) / new_moles
                    })
                    .collect();
                let total: f64 = new_composition.iter().sum();
                if total > 0.0 {
                    for value in &mut new_composition {
                        *value /= total;
                    }
                }
            }
            energy = u_before
                + (-specific_enthalpy * dm)
                + direct_fire * step_seconds
                + q_wall * step_seconds;
            new_mass = new_moles * molar_mass_of(&mixture, &new_composition)?;
        } else {
            new_mass = mass - dm;
            new_moles *= new_mass / mass;
            energy = u_before
                + (-specific_enthalpy * dm)
                + direct_fire * step_seconds
                + q_wall * step_seconds;
        }
        if new_moles <= 0.0 || new_mass <= 0.0 {
            break;
        }
        let molar_volume = vessel_volume.value / new_moles;
        let molar_energy = energy / new_moles;
        match vu_at(
            &mixture,
            &ideal_gas,
            pressure,
            molar_volume,
            molar_energy,
            &new_composition,
        ) {
            Ok(outcome) => {
                pressure = outcome.pressure;
                temperature = outcome.temperature;
                state = info_from_flash(
                    &mixture,
                    &ideal_gas,
                    kelvins(outcome.temperature),
                    pascals(outcome.pressure),
                    outcome.phase,
                    outcome.vapour_fraction,
                    &outcome.x,
                    &outcome.y,
                    &new_composition,
                )?;
            }
            Err(_) => {
                fallbacks += 1.0;
                pressure = new_mass * GAS_CONSTANT * temperature
                    / (molar_mass * vessel_volume.value)
                    * compressibility;
                pressure = pressure.max(back_pressure.value);
            }
        }
        composition = new_composition;
        moles = new_moles;
        mass = if selected.is_some() {
            new_moles * molar_mass_of(&mixture, &composition)?
        } else {
            new_mass
        };
        time += step_seconds;
        min_fluid_temperature = min_fluid_temperature.min(temperature);
        min_wall_temperature = min_wall_temperature.min(wall_temperature);
        max_pressure = max_pressure.max(pressure);
        max_fluid_temperature = max_fluid_temperature.max(temperature);
        max_wall_temperature = max_wall_temperature.max(wall_temperature);
        points.push(DepressurizationPoint {
            time: seconds(time),
            pressure: pascals(pressure),
            temperature: kelvins(temperature),
            mass: kilograms(mass),
            wall_temperature: kelvins(wall_temperature),
            mass_flow: kilograms_per_second(rate),
        });
    }

    let mut pressure_monotonic = true;
    let mut mass_monotonic = true;
    for rows in points.windows(2) {
        let previous_pressure = rows[0].pressure.value;
        let tolerance = (1.0e-3_f64).max(previous_pressure.abs() * 1.0e-4);
        if rows[1].pressure.value > previous_pressure + tolerance {
            pressure_monotonic = false;
        }
        let previous_mass = rows[0].mass.value;
        let mass_tolerance = 1.0e-8_f64.max(previous_mass.abs() * 1.0e-9);
        if rows[1].mass.value > previous_mass + mass_tolerance {
            mass_monotonic = false;
        }
    }
    let half = 0.5 * p0;
    let mut time_to_half = None;
    let mut time_to_seven = None;
    for index in 1..points.len() {
        let previous = &points[index - 1];
        let current = &points[index];
        if time_to_half.is_none()
            && previous.pressure.value > half
            && current.pressure.value <= half
        {
            time_to_half = Some(crossing_time(previous, current, half));
        }
        if time_to_seven.is_none()
            && previous.pressure.value > SEVEN_BARG_PA
            && current.pressure.value <= SEVEN_BARG_PA
        {
            time_to_seven = Some(crossing_time(previous, current, SEVEN_BARG_PA));
        }
    }
    let recorded = points.len() as f64;
    Ok(DepressurizationResult {
        points,
        initial_pressure: pascals(p0),
        fire_heat_input: watts(duty),
        fire_heat_routed_to_wall: route_to_wall && wall_metal > 0.0 && wall_face > 0.0,
        vu_flash_fallback_count: fallbacks,
        vapour_withdrawal_used: vapour_used,
        time_to_half_pressure: time_to_half.map(seconds),
        time_to_7_barg: time_to_seven.map(seconds),
        min_fluid_temperature: kelvins(min_fluid_temperature),
        min_wall_temperature: kelvins(min_wall_temperature),
        max_pressure: pascals(max_pressure),
        max_fluid_temperature: kelvins(max_fluid_temperature),
        max_wall_temperature: kelvins(max_wall_temperature),
        half_pressure_criterion_met: time_to_half.is_some_and(|value| value <= CRITERION_HORIZON_S),
        seven_barg_criterion_met: time_to_seven.is_some_and(|value| value <= CRITERION_HORIZON_S),
        pressure_monotonic_non_increasing: pressure_monotonic,
        mass_monotonic_non_increasing: mass_monotonic,
        recorded_points: recorded,
        warnings,
    })
}

/// The linearly interpolated time a pressure crossing occurs at.
fn crossing_time(
    previous: &DepressurizationPoint,
    current: &DepressurizationPoint,
    target: f64,
) -> f64 {
    let drop = previous.pressure.value - current.pressure.value;
    if drop.abs() < 1.0e-12 {
        return current.time.value;
    }
    // numerics-ok: the class clamps this interpolated crossing fraction
    let fraction = ((previous.pressure.value - target) / drop).clamp(0.0, 1.0);
    previous.time.value + fraction * (current.time.value - previous.time.value)
}
