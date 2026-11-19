//! `process.vessel_filling` - the gas-charging transient.
//!
//! Spec: `specs/models/process/vessel_filling.toml`. The arithmetic is
//! `process/safety/depressurization/VesselFillingSimulator.java` at the pin: the inlet
//! stagnation enthalpy, the mole alignment, the VU-flash march, the lumped wall, the
//! Woodfield film coefficient and the liner temperature limits.

use azoth_core::units::{
    Area, HeatTransfer, Length, Mass, MassRate, Pressure, SpecificHeatCapacity,
    ThermodynamicTemperature, Time, Volume, kelvins, kilograms, pascals, seconds,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use azoth_eos::cubic::Cubic;
use azoth_eos::databank;
use azoth_eos::entropy_state::{flashed_state, molar_mass_of};
use azoth_eos::heat_capacity::phase_capacity;
use azoth_eos::mixture::{Mixture, ReducedParameters, RootSide};
use azoth_eos::molar_enthalpy_entropy::IdealGasModel;
use azoth_eos::pt_flash::pt_flash;
use azoth_eos::results::Phase;
use azoth_eos::thermal_conductivity::thermal_conductivity;
use azoth_eos::viscosity::viscosity;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::vu_flash_route::vu_at;

/// The class's gas constant, J/(mol K).
const GAS_CONSTANT: f64 = 8.314;
/// Standard gravity for the Woodfield Rayleigh number, m/s2.
const GRAVITY: f64 = 9.80665;

/// One recorded step of a filling run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VesselFillingPoint {
    /// The step's time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
    /// The vessel pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// The gas temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// The vessel inventory.
    #[serde(serialize_with = "scalar")]
    pub mass: Mass,
    /// The wall metal temperature.
    #[serde(serialize_with = "scalar")]
    pub wall_temperature: ThermodynamicTemperature,
}

/// Result of `process.vessel_filling`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VesselFillingResult {
    /// One row per step.
    pub points: Vec<VesselFillingPoint>,
    /// The supply stagnation temperature.
    #[serde(serialize_with = "scalar")]
    pub inlet_temperature: ThermodynamicTemperature,
    /// The charging mass flow.
    #[serde(serialize_with = "scalar")]
    pub inlet_mass_flow: MassRate,
    /// The steps that used the ideal-gas fallback.
    pub vu_flash_fallback_count: f64,
    /// The hottest recorded gas temperature.
    #[serde(serialize_with = "scalar")]
    pub max_fluid_temperature: ThermodynamicTemperature,
    /// The coldest recorded gas temperature.
    #[serde(serialize_with = "scalar")]
    pub min_fluid_temperature: ThermodynamicTemperature,
    /// The highest recorded pressure.
    #[serde(serialize_with = "scalar")]
    pub max_pressure: Pressure,
    /// The last recorded pressure.
    #[serde(serialize_with = "scalar")]
    pub final_pressure: Pressure,
    /// The last recorded inventory mass.
    #[serde(serialize_with = "scalar")]
    pub final_mass: Mass,
    /// Whether any gas temperature exceeded the upper liner limit.
    pub liner_over_temperature: bool,
    /// Whether any gas temperature fell below the lower liner limit.
    pub liner_under_temperature: bool,
    /// Whether every gas temperature stayed within the limits.
    pub liner_limits_met: bool,
    /// The recorded rows.
    pub recorded_points: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The bulk molar heat capacity after one TP flash.
fn bulk_cp(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    temperature: ThermodynamicTemperature,
    pressure: Pressure,
    z: &[f64],
) -> Result<f64> {
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
            let vapour = phase_cp(
                mixture,
                ideal_gas,
                &reduced,
                temperature,
                pressure,
                &flash.y,
                flash.z_vapour,
            )?;
            let liquid = phase_cp(
                mixture,
                ideal_gas,
                &reduced,
                temperature,
                pressure,
                &flash.x,
                flash.z_liquid,
            )?;
            Ok(beta * vapour + (1.0 - beta) * liquid)
        }
        Phase::AllVapour | Phase::AllLiquid => {
            let side = if matches!(flash.phase, Phase::AllVapour) {
                RootSide::Vapour
            } else {
                RootSide::Liquid
            };
            let root = mixture.phase_state(&reduced, z, side)?.z;
            phase_cp(mixture, ideal_gas, &reduced, temperature, pressure, z, root)
        }
        Phase::Trivial => Err(AzothError::invalid_input(
            "initial_p",
            "the flash found a trivial solution at the initial state",
        )),
    }
}

#[allow(clippy::too_many_arguments)] // one parameter per property the phase exposes
fn phase_cp(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    reduced: &ReducedParameters,
    temperature: ThermodynamicTemperature,
    pressure: Pressure,
    composition: &[f64],
    root: f64,
) -> Result<f64> {
    Ok(phase_capacity(
        mixture,
        ideal_gas,
        reduced,
        temperature,
        pressure,
        composition,
        root,
    )?
    .cp)
}

/// The Woodfield mixed-convection coefficient, or `None` when a property is unavailable.
#[allow(clippy::too_many_arguments)] // one parameter per property the correlation reads
fn woodfield_coefficient(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    z: &[f64],
    molar_mass: f64,
    temperature: f64,
    pressure: f64,
    density: f64,
    inlet_mass_flow: f64,
    inlet_temperature: f64,
    nozzle: f64,
    height: f64,
) -> Option<f64> {
    let mu = viscosity(mixture, kelvins(temperature), pascals(pressure), z)
        .ok()?
        .mu
        .value;
    let k = thermal_conductivity(
        mixture,
        ideal_gas,
        kelvins(temperature),
        pascals(pressure),
        z,
    )
    .ok()?
    .k
    .value;
    let cp_molar = bulk_cp(
        mixture,
        ideal_gas,
        kelvins(temperature),
        pascals(pressure),
        z,
    )
    .ok()?;
    if mu <= 0.0 || k <= 0.0 || cp_molar <= 0.0 || molar_mass <= 0.0 || density <= 0.0 {
        return None;
    }
    let cp_mass = cp_molar / molar_mass;
    let alpha = k / (density * cp_mass);
    let nu = mu / density;
    let beta = 1.0 / temperature;
    let nozzle_area = std::f64::consts::PI * 0.25 * nozzle * nozzle;
    let inlet_velocity = inlet_mass_flow / (density * nozzle_area);
    let reynolds = density * inlet_velocity.abs() * nozzle / mu;
    let rayleigh =
        GRAVITY * beta * (inlet_temperature - temperature).abs() * height.powi(3) / (nu * alpha);
    let nusselt = 0.56 * reynolds.powf(0.67) + 0.104 * rayleigh.powf(0.352);
    let coefficient = nusselt * k / height;
    (coefficient > 0.0 && coefficient.is_finite()).then_some(coefficient)
}

/// The gas-charging transient of a vessel.
///
/// # Errors
/// `InvalidInput` for a non-positive dimension, a liner limit order, one Woodfield dimension
/// without the other, a non-finite input, or a feed state the flash cannot resolve.
#[allow(clippy::too_many_arguments)] // one parameter per quantity the class reads
pub fn vessel_filling(
    components: &[String],
    initial_z: &[f64],
    initial_t: ThermodynamicTemperature,
    initial_p: Pressure,
    vessel_volume: Volume,
    inlet_temperature: ThermodynamicTemperature,
    supply_pressure: Pressure,
    inlet_mass_flow: MassRate,
    target_pressure: Option<Pressure>,
    wall_mass: Option<Mass>,
    wall_area: Option<Area>,
    wall_specific_heat: Option<SpecificHeatCapacity>,
    wall_heat_transfer_coefficient: Option<HeatTransfer>,
    woodfield_nozzle_diameter: Option<Length>,
    vessel_height: Option<Length>,
    liner_min_temperature: Option<ThermodynamicTemperature>,
    liner_max_temperature: Option<ThermodynamicTemperature>,
    time_step: Option<Time>,
    max_time: Option<Time>,
) -> Result<VesselFillingResult> {
    let spec = &model_gen::VESSEL_FILLING_SPEC;
    let mut warnings = Vec::new();
    let target = target_pressure.map_or(f64::INFINITY, |value| value.value);
    let wall_metal = wall_mass.map_or(0.0, |value| value.value);
    let wall_face = wall_area.map_or(0.0, |value| value.value);
    let wall_capacity = wall_specific_heat.map_or(470.0, |value| value.value);
    let wall_film = wall_heat_transfer_coefficient.map_or(50.0, |value| value.value);
    let nozzle = woodfield_nozzle_diameter.map(|value| value.value);
    let height = vessel_height.map(|value| value.value);
    let liner_min = liner_min_temperature.map_or(f64::NEG_INFINITY, |value| value.value);
    let liner_max = liner_max_temperature.map_or(f64::INFINITY, |value| value.value);
    let step_seconds = time_step.map_or(1.0, |value| value.value);
    let span = max_time.map_or(600.0, |value| value.value);

    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "initial_t" => Some(initial_t.value),
            "initial_p" => Some(initial_p.value),
            "vessel_volume" => Some(vessel_volume.value),
            "inlet_temperature" => Some(inlet_temperature.value),
            "supply_pressure" => Some(supply_pressure.value),
            "inlet_mass_flow" => Some(inlet_mass_flow.value),
            "target_pressure" => Some(target_pressure.map_or(1.0, |value| value.value)),
            "woodfield_nozzle_diameter" => Some(nozzle.unwrap_or(1.0)),
            "vessel_height" => Some(height.unwrap_or(1.0)),
            "time_step" => Some(step_seconds),
            "max_time" => Some(span),
            _ => None,
        },
        &mut warnings,
    )?;

    for (name, value) in [
        ("initial_t", initial_t.value),
        ("initial_p", initial_p.value),
        ("vessel_volume", vessel_volume.value),
        ("inlet_temperature", inlet_temperature.value),
        ("supply_pressure", supply_pressure.value),
        ("inlet_mass_flow", inlet_mass_flow.value),
        ("wall_mass", wall_metal),
        ("wall_area", wall_face),
        ("wall_specific_heat", wall_capacity),
        ("wall_heat_transfer_coefficient", wall_film),
        ("woodfield_nozzle_diameter", nozzle.unwrap_or(1.0)),
        ("vessel_height", height.unwrap_or(1.0)),
        ("time_step", step_seconds),
        ("max_time", span),
    ] {
        if !value.is_finite() {
            return Err(AzothError::invalid_input(
                name,
                "a non-finite value is not a number this calculation publishes",
            ));
        }
    }
    if vessel_volume.value <= 0.0
        || inlet_temperature.value <= 0.0
        || supply_pressure.value <= 0.0
        || inlet_mass_flow.value <= 0.0
    {
        return Err(AzothError::invalid_input(
            "vessel_volume",
            "the class requires the volume and inlet conditions to be positive",
        ));
    }
    if target_pressure.is_some_and(|value| value.value <= 0.0 || !value.value.is_finite()) {
        return Err(AzothError::invalid_input(
            "target_pressure",
            "the class refuses a non-positive target pressure",
        ));
    }
    if nozzle.is_some() != height.is_some() {
        return Err(AzothError::invalid_input(
            "woodfield_nozzle_diameter",
            "the Woodfield correlation needs both the inlet nozzle diameter and the vessel height",
        ));
    }
    if nozzle.is_some_and(|value| value <= 0.0) || height.is_some_and(|value| value <= 0.0) {
        return Err(AzothError::invalid_input(
            "woodfield_nozzle_diameter",
            "the class refuses a non-positive nozzle diameter or vessel height",
        ));
    }
    if liner_max_temperature.is_some() && liner_max <= liner_min {
        return Err(AzothError::invalid_input(
            "liner_max_temperature",
            "the class requires the upper liner limit to exceed the lower one",
        ));
    }
    if step_seconds <= 0.0 || span <= 0.0 {
        return Err(AzothError::invalid_input(
            "time_step",
            "the class refuses a non-positive step or horizon",
        ));
    }
    if initial_t.value <= 0.0 || initial_p.value <= 0.0 {
        return Err(AzothError::invalid_input(
            "initial_t",
            "the initial temperature and pressure must be positive",
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
    let molar_mass = molar_mass_of(&mixture, initial_z)?;
    let mut evaluations = 0;
    let initial = flashed_state(
        &mixture,
        &ideal_gas,
        initial_z,
        molar_mass,
        initial_p.value,
        initial_t.value,
        &mut evaluations,
    )
    .map_err(|error| AzothError::invalid_input("initial_p", error.to_string()))?;
    let inlet = flashed_state(
        &mixture,
        &ideal_gas,
        initial_z,
        molar_mass,
        supply_pressure.value,
        inlet_temperature.value,
        &mut evaluations,
    )
    .map_err(|error| AzothError::invalid_input("supply_pressure", error.to_string()))?;
    let inlet_enthalpy = inlet.molar_enthalpy / molar_mass;

    let mut pressure = initial_p.value;
    let mut temperature = initial_t.value;
    let mut mass = initial.density * vessel_volume.value;
    let moles = mass / molar_mass;
    let mut energy = initial.molar_enthalpy * moles - pressure * vessel_volume.value;
    let mut wall_temperature = temperature;
    let mut fallbacks = 0.0;
    let mut time = 0.0;
    let mut points = vec![VesselFillingPoint {
        time: seconds(time),
        pressure: pascals(pressure),
        temperature: kelvins(temperature),
        mass: kilograms(mass),
        wall_temperature: kelvins(wall_temperature),
    }];
    let mut max_fluid_temperature = temperature;
    let mut min_fluid_temperature = temperature;
    let mut max_pressure = pressure;

    while time < span && pressure < target && mass > 0.0 {
        let dm = inlet_mass_flow.value * step_seconds;
        let new_mass = mass + dm;
        let density = mass / vessel_volume.value;
        let coefficient = match (nozzle, height) {
            (Some(nozzle), Some(height)) => woodfield_coefficient(
                &mixture,
                &ideal_gas,
                initial_z,
                molar_mass,
                temperature,
                pressure,
                density,
                inlet_mass_flow.value,
                inlet_temperature.value,
                nozzle,
                height,
            )
            .unwrap_or(wall_film),
            _ => wall_film,
        };
        let q_wall = if wall_metal > 0.0 {
            coefficient * wall_face * (wall_temperature - temperature)
        } else {
            0.0
        };
        let d_u = inlet_enthalpy * dm + q_wall * step_seconds;
        if wall_metal > 0.0 {
            wall_temperature += (-q_wall) * step_seconds / (wall_metal * wall_capacity);
        }
        let new_energy = energy + d_u;
        let new_moles = new_mass / molar_mass;
        let molar_volume_step = vessel_volume.value / new_moles;
        let molar_energy = new_energy / new_moles;
        match vu_at(
            &mixture,
            &ideal_gas,
            pressure,
            molar_volume_step,
            molar_energy,
            initial_z,
        ) {
            Ok(outcome) => {
                temperature = outcome.temperature;
                pressure = outcome.pressure;
            }
            Err(_) => {
                fallbacks += 1.0;
                pressure =
                    new_mass * GAS_CONSTANT * temperature / (molar_mass * vessel_volume.value);
            }
        }
        mass = new_mass;
        energy = new_energy;
        time += step_seconds;
        max_fluid_temperature = max_fluid_temperature.max(temperature);
        min_fluid_temperature = min_fluid_temperature.min(temperature);
        max_pressure = max_pressure.max(pressure);
        points.push(VesselFillingPoint {
            time: seconds(time),
            pressure: pascals(pressure),
            temperature: kelvins(temperature),
            mass: kilograms(mass),
            wall_temperature: kelvins(wall_temperature),
        });
    }

    let mut liner_over = false;
    let mut liner_under = false;
    for point in &points {
        if point.temperature.value > liner_max {
            liner_over = true;
        }
        if point.temperature.value < liner_min {
            liner_under = true;
        }
    }
    let recorded = points.len() as f64;
    let final_point = points.last().expect("the initial row is always recorded");
    let final_pressure = final_point.pressure;
    let final_mass = final_point.mass;
    Ok(VesselFillingResult {
        points,
        inlet_temperature,
        inlet_mass_flow,
        vu_flash_fallback_count: fallbacks,
        max_fluid_temperature: kelvins(max_fluid_temperature),
        min_fluid_temperature: kelvins(min_fluid_temperature),
        max_pressure: pascals(max_pressure),
        final_pressure,
        final_mass,
        liner_over_temperature: liner_over,
        liner_under_temperature: liner_under,
        liner_limits_met: !liner_over && !liner_under,
        recorded_points: recorded,
        warnings,
    })
}
