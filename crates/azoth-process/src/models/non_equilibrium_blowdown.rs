//! `process.non_equilibrium_blowdown` - the two-zone fire-exposed blowdown.
//!
//! Spec: `specs/models/process/non_equilibrium_blowdown.toml`. The arithmetic is
//! `process/safety/depressurization/NonEquilibriumBlowdownModel.java` at the pin: the vapour
//! and liquid zones, the vapour-controlled pressure, the wetted-wall boiling, the
//! interfacial coupling and the boil-off transfer.

use azoth_core::units::{
    Area, HeatTransfer, Length, Mass, Pressure, SpecificHeatCapacity, TemperatureInterval,
    ThermodynamicTemperature, Time, Volume, joules_per_mole, kelvin_intervals, kelvins, kilograms,
    pascals, seconds,
};
use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use azoth_eos::cubic::Cubic;
use azoth_eos::databank;
use azoth_eos::entropy_state::molar_mass_of;
use azoth_eos::heat_capacity::phase_capacity;
use azoth_eos::mixture::{Mixture, ReducedParameters, RootSide};
use azoth_eos::molar_enthalpy_entropy::{IdealGasModel, molar_enthalpy_entropy};
use azoth_eos::ph_flash::ph_flash;
use azoth_eos::pr_molar_volume::MOLAR_GAS_CONSTANT;
use azoth_eos::pt_flash::pt_flash;
use azoth_eos::results::Phase;
use azoth_eos::thermal_conductivity::thermal_conductivity;
use azoth_eos::tv_flash::tv_flash;
use azoth_eos::viscosity::viscosity;

use crate::executor::json::{scalar, warnings as wire_warnings};

/// The class's gas constant, J/(mol K).
const GAS_CONSTANT: f64 = 8.314;
/// Stefan-Boltzmann constant, W/(m2 K4).
const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;
/// Standard gravity, m/s2.
const GRAVITY: f64 = 9.80665;
/// The class's zone temperature floor, K.
const MIN_ZONE_TEMPERATURE: f64 = 100.0;
/// The Rohsenow Prandtl exponent.
const ROHSENOW_PRANDTL_EXPONENT: f64 = 1.7;

/// One recorded step of a non-equilibrium blowdown.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NonEquilibriumBlowdownPoint {
    /// The step's time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
    /// The vessel pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// The vapour-zone temperature.
    #[serde(serialize_with = "scalar")]
    pub gas_temperature: ThermodynamicTemperature,
    /// The liquid-zone temperature.
    #[serde(serialize_with = "scalar")]
    pub liquid_temperature: ThermodynamicTemperature,
    /// The wall metal temperature.
    #[serde(serialize_with = "scalar")]
    pub wall_temperature: ThermodynamicTemperature,
    /// The vapour-zone mass.
    #[serde(serialize_with = "scalar")]
    pub gas_mass: Mass,
    /// The liquid-zone mass.
    #[serde(serialize_with = "scalar")]
    pub liquid_mass: Mass,
}

/// Result of `process.non_equilibrium_blowdown`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NonEquilibriumBlowdownResult {
    /// One row per step.
    pub points: Vec<NonEquilibriumBlowdownPoint>,
    /// The initial pressure.
    #[serde(serialize_with = "scalar")]
    pub initial_pressure: Pressure,
    /// The steps that hit a guarded flash failure.
    pub flash_fallback_count: f64,
    /// The final vapour-zone temperature.
    #[serde(serialize_with = "scalar")]
    pub final_gas_temperature: ThermodynamicTemperature,
    /// The final liquid-zone temperature.
    #[serde(serialize_with = "scalar")]
    pub final_liquid_temperature: ThermodynamicTemperature,
    /// The largest vapour-minus-liquid temperature difference.
    #[serde(serialize_with = "scalar")]
    pub max_temperature_bifurcation: TemperatureInterval,
    /// The recorded rows.
    pub recorded_points: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One phase's properties.
struct PhaseProps {
    molar_mass: f64,
    density: f64,
    cp: f64,
    cv: f64,
}

#[allow(clippy::too_many_arguments)]
fn phase_props(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    reduced: &ReducedParameters,
    temperature: ThermodynamicTemperature,
    pressure: Pressure,
    composition: &[f64],
    root: f64,
) -> Result<PhaseProps> {
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
    Ok(PhaseProps {
        molar_mass,
        density: molar_mass / molar_volume,
        cp: capacity.cp,
        cv: capacity.cv,
    })
}

/// A zone's bulk properties.
struct ZoneProps {
    molar_mass: f64,
    density: f64,
    cp_mass: f64,
    gamma: f64,
}

fn zone_props(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    temperature: ThermodynamicTemperature,
    pressure: Pressure,
    z: &[f64],
) -> Result<ZoneProps> {
    let reduced = mixture.reduced_parameters(temperature, pressure)?;
    let flash = pt_flash(mixture, temperature, pressure, z)?;
    match flash.phase {
        Phase::TwoPhase => {
            let beta = flash.vapour_fraction.ok_or_else(|| {
                AzothError::invalid_input("initial_p", "the zone flash split and reported no beta")
            })?;
            let vapour = phase_props(
                mixture,
                ideal_gas,
                &reduced,
                temperature,
                pressure,
                &flash.y,
                flash.z_vapour,
            )?;
            let liquid = phase_props(
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
            let cp = beta * vapour.cp + (1.0 - beta) * liquid.cp;
            let cv = beta * vapour.cv + (1.0 - beta) * liquid.cv;
            Ok(ZoneProps {
                molar_mass: mass,
                density: mass / volume,
                cp_mass: cp / mass,
                gamma: cp / cv,
            })
        }
        Phase::AllVapour | Phase::AllLiquid => {
            let side = if matches!(flash.phase, Phase::AllVapour) {
                RootSide::Vapour
            } else {
                RootSide::Liquid
            };
            let root = mixture.phase_state(&reduced, z, side)?.z;
            let phase = phase_props(mixture, ideal_gas, &reduced, temperature, pressure, z, root)?;
            Ok(ZoneProps {
                molar_mass: phase.molar_mass,
                density: phase.density,
                cp_mass: phase.cp / phase.molar_mass,
                gamma: phase.cp / phase.cv,
            })
        }
        Phase::Trivial => Err(AzothError::invalid_input(
            "initial_p",
            "the zone flash found a trivial solution",
        )),
    }
}

/// The Rohsenow nucleate-boiling heat flux.
#[allow(clippy::too_many_arguments)]
fn rohsenow(
    liquid_viscosity: f64,
    latent_heat: f64,
    liquid_density: f64,
    vapour_density: f64,
    surface_tension: f64,
    liquid_cp: f64,
    superheat: f64,
    prandtl: f64,
    surface_constant: f64,
) -> Option<f64> {
    if liquid_viscosity <= 0.0
        || latent_heat <= 0.0
        || liquid_density <= 0.0
        || surface_tension <= 0.0
        || liquid_cp <= 0.0
        || surface_constant <= 0.0
        || prandtl <= 0.0
        || vapour_density < 0.0
        || vapour_density >= liquid_density
    {
        return None;
    }
    if superheat <= 0.0 {
        return Some(0.0);
    }
    let buoyancy = (GRAVITY * (liquid_density - vapour_density) / surface_tension).sqrt();
    let bracket = liquid_cp * superheat
        / (surface_constant * latent_heat * prandtl.powf(ROHSENOW_PRANDTL_EXPONENT));
    let flux = liquid_viscosity * latent_heat * buoyancy * bracket.powi(3);
    flux.is_finite().then_some(flux)
}

/// Record a state and track the largest vapour-minus-liquid difference.
#[allow(clippy::too_many_arguments)]
fn push_point(
    time: f64,
    pressure: f64,
    gas_temperature: f64,
    liquid_temperature: f64,
    wall_temperature: f64,
    gas_mass: f64,
    liquid_mass: f64,
    points: &mut Vec<NonEquilibriumBlowdownPoint>,
    max_bifurcation: &mut f64,
) {
    *max_bifurcation = max_bifurcation.max(gas_temperature - liquid_temperature);
    points.push(NonEquilibriumBlowdownPoint {
        time: seconds(time),
        pressure: pascals(pressure),
        gas_temperature: kelvins(gas_temperature),
        liquid_temperature: kelvins(liquid_temperature),
        wall_temperature: kelvins(wall_temperature),
        gas_mass: kilograms(gas_mass),
        liquid_mass: kilograms(liquid_mass),
    });
}

/// The two-zone fire-exposed blowdown.
///
/// # Errors
/// `InvalidInput` for a non-positive dimension, a feed that does not split into vapour and
/// liquid, a non-finite input, or a state a zone flash cannot resolve.
#[allow(clippy::too_many_arguments)]
pub fn non_equilibrium_blowdown(
    components: &[String],
    initial_z: &[f64],
    initial_t: ThermodynamicTemperature,
    initial_p: Pressure,
    vessel_volume: Volume,
    orifice_diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
    fire_emissivity: Option<f64>,
    fire_flame_temperature: Option<ThermodynamicTemperature>,
    fire_convective_coefficient: Option<HeatTransfer>,
    fire_exposed_area: Option<Area>,
    wall_mass: Option<Mass>,
    wall_specific_heat: Option<SpecificHeatCapacity>,
    inside_gas_coefficient: Option<HeatTransfer>,
    interfacial_coefficient: Option<HeatTransfer>,
    interfacial_area: Option<Area>,
    latent_heat: Option<f64>,
    surface_tension: Option<f64>,
    surface_fluid_constant: Option<f64>,
    critical_heat_flux: Option<f64>,
    effective_boiling_coefficient: Option<HeatTransfer>,
    time_step: Option<Time>,
    max_time: Option<Time>,
    stop_pressure: Option<Pressure>,
) -> Result<NonEquilibriumBlowdownResult> {
    let emissivity = fire_emissivity.unwrap_or(0.9);
    let flame = fire_flame_temperature.map_or(1100.0, |value| value.value);
    let convection = fire_convective_coefficient.map_or(30.0, |value| value.value);
    let exposed = fire_exposed_area.map_or(0.0, |value| value.value);
    let metal = wall_mass.map_or(0.0, |value| value.value);
    let wall_cp = wall_specific_heat.map_or(470.0, |value| value.value);
    let inside = inside_gas_coefficient.map_or(30.0, |value| value.value);
    let interface_coefficient = interfacial_coefficient.map_or(200.0, |value| value.value);
    let interface_area = interfacial_area.map_or(0.0, |value| value.value);
    let latent = latent_heat.unwrap_or(350_000.0);
    let sigma = surface_tension.unwrap_or(0.015);
    let csf = surface_fluid_constant.unwrap_or(0.011);
    let chf = critical_heat_flux.unwrap_or(1.0e6);
    let boiling = effective_boiling_coefficient.map_or(5000.0, |value| value.value);
    let step = time_step.map_or(1.0, |value| value.value);
    let span = max_time.map_or(900.0, |value| value.value);
    let stop = stop_pressure.map_or(1.5e5, |value| value.value);

    for (name, value) in [
        ("vessel_volume", vessel_volume.value),
        ("orifice_diameter", orifice_diameter.value),
        ("discharge_coefficient", discharge_coefficient),
        ("back_pressure", back_pressure.value),
        ("fire_emissivity", emissivity),
        ("fire_flame_temperature", flame),
        ("fire_convective_coefficient", convection),
        ("fire_exposed_area", exposed),
        ("wall_mass", metal),
        ("wall_specific_heat", wall_cp),
        ("inside_gas_coefficient", inside),
        ("interfacial_coefficient", interface_coefficient),
        ("interfacial_area", interface_area),
        ("latent_heat", latent),
        ("surface_tension", sigma),
        ("surface_fluid_constant", csf),
        ("critical_heat_flux", chf),
        ("effective_boiling_coefficient", boiling),
        ("time_step", step),
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
    if discharge_coefficient <= 0.0 || step <= 0.0 || span <= 0.0 || stop <= 0.0 {
        return Err(AzothError::invalid_input(
            "discharge_coefficient",
            "the coefficient, step, horizon and stop pressure must be positive",
        ));
    }

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, Cubic::Pr, None)?;
    if initial_z.len() != mixture.len() {
        return Err(AzothError::invalid_input(
            "initial_z",
            "the composition must have one entry per component",
        ));
    }
    azoth_eos::phase_boundary::check_composition(initial_z, mixture.len(), "initial_z")?;
    let flash = pt_flash(&mixture, initial_t, initial_p, initial_z)?;
    let beta = match flash.phase {
        Phase::TwoPhase => flash.vapour_fraction.ok_or_else(|| {
            AzothError::invalid_input(
                "initial_p",
                "the feed split and reported no vapour fraction",
            )
        })?,
        _ => {
            return Err(AzothError::invalid_input(
                "initial_p",
                "the class requires an initial coexisting vapour and liquid phase",
            ));
        }
    };
    let mut gas_composition = flash.y.clone();
    let mut liquid_composition = flash.x.clone();
    // **The class charges by the EOS volume**: `getVolume("m3")` sums each phase's
    // `Z R T / P`, not the physical-properties density, and the inventory is scaled so
    // that volume fills the vessel.
    let eos_molar_volume = beta * flash.z_vapour * MOLAR_GAS_CONSTANT * initial_t.value
        / initial_p.value
        + (1.0 - beta) * flash.z_liquid * MOLAR_GAS_CONSTANT * initial_t.value / initial_p.value;
    if !eos_molar_volume.is_finite() || eos_molar_volume <= 0.0 {
        return Err(AzothError::invalid_input(
            "initial_p",
            format!("the EOS molar volume of the feed is {eos_molar_volume} m3/mol"),
        ));
    }
    let total_moles = vessel_volume.value / eos_molar_volume;
    let mut gas_moles = beta * total_moles;
    let mut liquid_moles = (1.0 - beta) * total_moles;
    let mut gas_temperature = initial_t.value;
    let mut liquid_temperature = initial_t.value;
    let mut wall_temperature = initial_t.value;
    let mut pressure = initial_p.value;
    let eff_interface = if interface_area > 0.0 {
        interface_area
    } else {
        vessel_volume.value.powf(2.0 / 3.0)
    };
    let eff_external = if exposed > 0.0 {
        exposed
    } else {
        4.0 * vessel_volume.value.powf(2.0 / 3.0)
    };
    let eff_wall_mass = if metal > 0.0 { metal } else { 1.0 };
    let area = std::f64::consts::PI * 0.25 * orifice_diameter.value * orifice_diameter.value;
    let mut fallbacks = 0.0;
    let mut time = 0.0;
    let mut points = Vec::new();
    let mut max_bifurcation = 0.0_f64;
    push_point(
        time,
        pressure,
        gas_temperature,
        liquid_temperature,
        wall_temperature,
        gas_moles * molar_mass_of(&mixture, &gas_composition)?,
        liquid_moles * molar_mass_of(&mixture, &liquid_composition)?,
        &mut points,
        &mut max_bifurcation,
    );

    while time < span && pressure > stop && gas_moles > 0.0 {
        let gas = match zone_props(
            &mixture,
            &ideal_gas,
            kelvins(gas_temperature),
            pascals(pressure),
            &gas_composition,
        ) {
            Ok(props) => props,
            Err(_) => {
                fallbacks += 1.0;
                break;
            }
        };
        let liquid = match zone_props(
            &mixture,
            &ideal_gas,
            kelvins(liquid_temperature),
            pascals(pressure),
            &liquid_composition,
        ) {
            Ok(props) => props,
            Err(_) => {
                fallbacks += 1.0;
                break;
            }
        };
        // The class reads the compressibility from the physical-properties density at the
        // pressure the step started from, and uses it in both the rate and the flow work.
        let z_gas = if gas.density > 0.0 && gas.molar_mass > 0.0 {
            let value = pressure * gas.molar_mass / (gas.density * GAS_CONSTANT * gas_temperature);
            if value.is_finite() && value > 0.0 {
                value
            } else {
                1.0
            }
        } else {
            1.0
        };
        let gas_mass = gas_moles * gas.molar_mass;
        let liquid_mass = liquid_moles * liquid.molar_mass;
        let liquid_volume = if liquid.density > 0.0 {
            liquid_mass / liquid.density
        } else {
            0.0
        };
        let gas_volume = (vessel_volume.value - liquid_volume).max(1.0e-6);
        let molar_volume = gas_volume / gas_moles;
        let tv = match tv_flash(
            &mixture,
            &ideal_gas,
            kelvins(gas_temperature),
            azoth_core::units::cubic_meters_per_mole(molar_volume),
            &gas_composition,
        ) {
            Ok(result) => result,
            Err(_) => {
                fallbacks += 1.0;
                break;
            }
        };
        pressure = tv.pressure.value.max(back_pressure.value);

        let critical_ratio = (2.0 / (gas.gamma + 1.0)).powf(gas.gamma / (gas.gamma - 1.0));
        let pressure_ratio = back_pressure.value / pressure;
        let mut rate = if pressure_ratio <= critical_ratio {
            discharge_coefficient
                * area
                * pressure
                * (gas.gamma * gas.molar_mass / (z_gas * GAS_CONSTANT * gas_temperature)).sqrt()
                * (2.0 / (gas.gamma + 1.0)).powf((gas.gamma + 1.0) / (2.0 * (gas.gamma - 1.0)))
        } else {
            let mut term = (2.0 * gas.gamma / (gas.gamma - 1.0))
                * (pressure_ratio.powf(2.0 / gas.gamma)
                    - pressure_ratio.powf((gas.gamma + 1.0) / gas.gamma));
            if term < 0.0 {
                term = 0.0;
            }
            discharge_coefficient
                * area
                * pressure
                * (gas.molar_mass / (z_gas * GAS_CONSTANT * gas_temperature)).sqrt()
                * term.sqrt()
        };
        if rate < 0.0 || rate.is_nan() {
            rate = 0.0;
        }
        let dm_out = (rate * step).min(gas_mass);

        let radiative = emissivity * STEFAN_BOLTZMANN * (flame.powi(4) - wall_temperature.powi(4));
        let convective = convection * (flame - wall_temperature);
        let fire_flux = (radiative + convective).max(0.0);
        let fire_heat = fire_flux * eff_external;

        let wetted = (liquid_volume / vessel_volume.value).clamp(0.0, 1.0);
        let wetted_area = eff_external * wetted;
        let unwetted_area = eff_external * (1.0 - wetted);
        let wall_to_gas = inside * unwetted_area * (wall_temperature - gas_temperature);
        let superheat = wall_temperature - liquid_temperature;
        let boiling_flux = if superheat > 0.0 {
            let mu = viscosity(
                &mixture,
                kelvins(liquid_temperature),
                pascals(pressure),
                &liquid_composition,
            )
            .map_or(0.0, |value| value.mu.value);
            let k = thermal_conductivity(
                &mixture,
                &ideal_gas,
                kelvins(liquid_temperature),
                pascals(pressure),
                &liquid_composition,
            )
            .map_or(0.0, |value| value.k.value);
            let prandtl = if k > 0.0 && mu > 0.0 {
                mu * liquid.cp_mass / k
            } else {
                5.0
            };
            rohsenow(
                mu,
                latent,
                liquid.density,
                gas.density,
                sigma,
                liquid.cp_mass,
                superheat,
                prandtl,
                csf,
            )
            .unwrap_or(boiling * superheat)
        } else {
            0.0
        }
        .min(chf);
        let wall_to_liquid = boiling_flux * wetted_area;
        let interface_heat =
            interface_coefficient * eff_interface * (gas_temperature - liquid_temperature);
        let dm_vap = ((wall_to_liquid.max(0.0) * step / latent) as f64).min(0.5 * liquid_mass);

        wall_temperature +=
            (fire_heat - wall_to_gas - wall_to_liquid) * step / (eff_wall_mass * wall_cp);
        wall_temperature = wall_temperature.clamp(MIN_ZONE_TEMPERATURE, flame);
        if gas_mass > 1.0e-9 && gas.cp_mass > 0.0 {
            // **A fixed-volume open system**: `m cv dT = Q - (h - u) dm_out`, with
            // `h - u = Z R T / M`; without the flow work the vapour does not cool on
            // depressurisation at all.
            let cv_gas_mass = gas.cp_mass / gas.gamma.max(1.0);
            let flow_work = z_gas * GAS_CONSTANT * gas_temperature / gas.molar_mass * dm_out;
            let d_u = (wall_to_gas - interface_heat) * step
                + dm_vap * cv_gas_mass * (liquid_temperature - gas_temperature)
                - flow_work;
            gas_temperature += d_u / (gas_mass * cv_gas_mass);
        }
        gas_temperature = gas_temperature.clamp(MIN_ZONE_TEMPERATURE, wall_temperature);
        if liquid_mass > 1.0e-9 && liquid.cp_mass > 0.0 {
            let d_u = (interface_heat + wall_to_liquid) * step - dm_vap * latent;
            liquid_temperature += d_u / (liquid_mass * liquid.cp_mass);
        }
        liquid_temperature =
            liquid_temperature.clamp(MIN_ZONE_TEMPERATURE, wall_temperature.max(gas_temperature));

        if gas_mass > 0.0 {
            gas_moles *= (gas_mass - dm_out) / gas_mass;
        }
        if dm_vap > 0.0 && liquid_moles > 0.0 {
            let moved = dm_vap / liquid.molar_mass;
            let moved = moved.min(liquid_moles);
            let gas_total = gas_moles + moved;
            if gas_total > 0.0 {
                for (index, value) in gas_composition.iter_mut().enumerate() {
                    *value = (gas_moles * *value + moved * liquid_composition[index]) / gas_total;
                }
            }
            gas_moles = gas_total;
            liquid_moles -= moved;
        }
        // **Flash boil-off**: liquid left superheated by the pressure drop flashes
        // isenthalpically at the new pressure, and the vapour it forms moves to the vapour
        // zone, cooling the liquid. Without this the liquid zone only boiled from wall heat
        // and stayed at its initial pressure-equilibrium state.
        let liquid_mass_now = liquid_moles * molar_mass_of(&mixture, &liquid_composition)?;
        if liquid_moles > 0.0 && liquid_mass_now > 1.0e-6 {
            let liquid_pressure = pascals(pressure);
            // **The liquid system is a single oil phase, so its enthalpy is the liquid
            // root's, not the equilibrium split's**: the class's `TPflash` on the phase
            // system leaves one phase, and the PH flash is what creates the vapour.
            let liquid_root_enthalpy = (|| -> std::result::Result<f64, AzothError> {
                let reduced =
                    mixture.reduced_parameters(kelvins(liquid_temperature), liquid_pressure)?;
                let root = mixture
                    .phase_state(&reduced, &liquid_composition, RootSide::Liquid)?
                    .z;
                Ok(molar_enthalpy_entropy(
                    &mixture,
                    &ideal_gas,
                    kelvins(liquid_temperature),
                    liquid_pressure,
                    &liquid_composition,
                    root,
                )?
                .h
                .value)
            })();
            let flashed = liquid_root_enthalpy.and_then(|enthalpy| {
                ph_flash(
                    &mixture,
                    &ideal_gas,
                    liquid_pressure,
                    joules_per_mole(enthalpy),
                    &liquid_composition,
                )
            });
            match flashed {
                Ok(solution) => {
                    liquid_temperature = solution.temperature.value;
                    if solution.phase == Phase::TwoPhase {
                        let split = solution.vapour_fraction.unwrap_or(0.0);
                        let moved_moles = split * liquid_moles;
                        let vapour_mass = moved_moles * molar_mass_of(&mixture, &solution.y)?;
                        let gas_mass_before =
                            gas_moles * molar_mass_of(&mixture, &gas_composition)?;
                        let gas_total = gas_moles + moved_moles;
                        if gas_total > 0.0 {
                            for (index, value) in gas_composition.iter_mut().enumerate() {
                                *value = (gas_moles * *value + moved_moles * solution.y[index])
                                    / gas_total;
                            }
                        }
                        gas_moles = gas_total;
                        liquid_moles *= 1.0 - split;
                        liquid_composition = solution.x.clone();
                        if vapour_mass > 0.0 {
                            let mixed = gas_mass_before + vapour_mass;
                            gas_temperature = (gas_mass_before * gas_temperature
                                + vapour_mass * liquid_temperature)
                                / mixed;
                        }
                    }
                }
                Err(_) => {
                    fallbacks += 1.0;
                }
            }
        }
        time += step;
        push_point(
            time,
            pressure,
            gas_temperature,
            liquid_temperature,
            wall_temperature,
            gas_moles * molar_mass_of(&mixture, &gas_composition)?,
            liquid_moles * molar_mass_of(&mixture, &liquid_composition)?,
            &mut points,
            &mut max_bifurcation,
        );
    }

    let recorded = points.len() as f64;
    Ok(NonEquilibriumBlowdownResult {
        points,
        initial_pressure: initial_p,
        flash_fallback_count: fallbacks,
        final_gas_temperature: kelvins(gas_temperature),
        final_liquid_temperature: kelvins(liquid_temperature),
        max_temperature_bifurcation: kelvin_intervals(max_bifurcation),
        recorded_points: recorded,
        warnings: Vec::new(),
    })
}
