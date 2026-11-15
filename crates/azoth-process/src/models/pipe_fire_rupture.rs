//! `process.pipe_fire_rupture` - the pipe strain-rate rupture as a registered id.
//!
//! Spec: `specs/models/process/pipe_fire_rupture.toml`. The arithmetic is
//! `process/safety/rupture/PipeFireRuptureStudy.java` at the pin: the thick-wall von Mises
//! stress, the Sellars-Tegart strain rate, the accumulated-strain limit, the class's
//! thermal mass and the spreadsheet-style release.

use azoth_core::units::{
    Frequency, HeatFluxDensity, HeatTransfer, Length, MassDensity, MassRate, MolarMass, Pressure,
    SpecificHeatCapacity, ThermodynamicTemperature, Time, bars, kelvins,
    kilograms_per_cubic_meter, kilograms_per_second, megapascals, per_second, seconds,
    watts_per_square_meter,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;
use std::f64::consts::PI;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::blowdown_pressure_profile::{BlowdownProfilePoint, blowdown_pressure_profile};
use crate::models::pipe_fire_rupture_material::{
    PipeFireRuptureMaterialResult, pipe_fire_rupture_material,
};

/// Pascals in a bar.
const PA_PER_BAR: f64 = 1.0e5;
/// The class's universal gas constant, J/(kmol K).
const UNIVERSAL_GAS_CONSTANT_J_PER_KMOLK: f64 = 8314.0;
/// The class's gas release coefficient.
const GAS_RELEASE_COEFFICIENT: f64 = 0.00527;
/// The class's short-pipe multiplier.
const SHORT_PIPE_GAS_MULTIPLIER: f64 = 1.541;
/// The class's liquid discharge coefficient.
const LIQUID_DISCHARGE_COEFFICIENT: f64 = 0.62;
/// The class's liquid flow constant.
const LIQUID_FLOW_CONSTANT: f64 = 11.78;
/// The class's gas/liquid density switch, kg/m3.
const GAS_LIQUID_DENSITY_SWITCH_KG_PER_M3: f64 = 500.0;
/// The class's default step, s.
const DEFAULT_TIME_STEP_SECONDS: f64 = 5.0;
/// The class's default maximum time, s.
const DEFAULT_MAX_TIME_SECONDS: f64 = 3600.0;
/// The class's absolute-temperature offset.
const KELVIN_OFFSET: f64 = 273.15;
/// The class's own absolute-temperature offset, applied after the conversion.
const CLASS_OFFSET: f64 = 273.0;
/// The Stefan-Boltzmann constant the scenario carries, W/(m2 K4).
const STEFAN_BOLTZMANN_W_PER_M2K4: f64 = 5.67e-8;
/// The class's strain cap.
const STRAIN_CAP: f64 = 2.0;

/// The limiting status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RuptureStatus {
    /// The accumulated strain never exceeded the limit.
    #[serde(rename = "no_rupture")]
    NoRupture,
    /// The accumulated strain exceeded the limit.
    #[serde(rename = "rupture")]
    Rupture,
}

impl RuptureStatus {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoRupture => "no_rupture",
            Self::Rupture => "rupture",
        }
    }
}

/// The release branch the density switch selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FluidBasis {
    /// Below 500 kg/m3.
    #[serde(rename = "gas")]
    Gas,
    /// At or above 500 kg/m3.
    #[serde(rename = "liquid")]
    Liquid,
}

impl FluidBasis {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gas => "gas",
            Self::Liquid => "liquid",
        }
    }
}

/// One marching step.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PipeFireRupturePoint {
    /// The step's time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
    /// The step's gauge pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// The step's mean wall temperature.
    #[serde(serialize_with = "scalar")]
    pub mean_wall_temperature: ThermodynamicTemperature,
    /// The step's outer surface temperature.
    #[serde(serialize_with = "scalar")]
    pub outer_surface_temperature: ThermodynamicTemperature,
    /// The step's absorbed heat flux.
    #[serde(serialize_with = "scalar")]
    pub heat_flux: HeatFluxDensity,
    /// The step's thick-wall von Mises stress.
    #[serde(serialize_with = "scalar")]
    pub von_mises_stress: Pressure,
    /// The step's strain rate.
    #[serde(serialize_with = "scalar")]
    pub strain_rate: Frequency,
    /// The running accumulated strain.
    pub accumulated_strain: f64,
    /// The step's temperature-dependent rupture strain limit.
    pub rupture_strain_limit: f64,
}

/// Result of `process.pipe_fire_rupture`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PipeFireRuptureResult {
    /// The strict strain test's status.
    pub status: RuptureStatus,
    /// The branch the density switch selected.
    pub fluid_basis: FluidBasis,
    /// The first step's time whose accumulated strain exceeded the limit.
    #[serde(serialize_with = "scalar")]
    pub rupture_time: Option<Time>,
    /// The gauge pressure at the rupture step.
    #[serde(serialize_with = "scalar")]
    pub rupture_pressure: Option<Pressure>,
    /// The mean wall temperature at the rupture step.
    #[serde(serialize_with = "scalar")]
    pub rupture_mean_wall_temperature: Option<ThermodynamicTemperature>,
    /// The outer surface temperature at the rupture step.
    #[serde(serialize_with = "scalar")]
    pub rupture_outer_surface_temperature: Option<ThermodynamicTemperature>,
    /// The accumulated strain at the rupture step.
    pub rupture_accumulated_strain: Option<f64>,
    /// The temperature-dependent limit at the rupture step.
    pub rupture_strain_limit: Option<f64>,
    /// The two-sided long-pipe gas release.
    #[serde(serialize_with = "scalar")]
    pub long_pipe_gas_two_sides: Option<MassRate>,
    /// The one-sided long-pipe gas release.
    #[serde(serialize_with = "scalar")]
    pub long_pipe_gas_one_side: Option<MassRate>,
    /// The one-sided short-pipe gas release.
    #[serde(serialize_with = "scalar")]
    pub short_pipe_gas_one_side: Option<MassRate>,
    /// The one-sided liquid release.
    #[serde(serialize_with = "scalar")]
    pub liquid_one_side: Option<MassRate>,
    /// The initial temperature the gas release was evaluated at.
    #[serde(serialize_with = "scalar")]
    pub release_gas_temperature: Option<ThermodynamicTemperature>,
    /// One row per marching step, ending at the rupture step when one is found.
    pub history: Vec<PipeFireRupturePoint>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One fire exposure.
struct Scenario {
    fire_c: f64,
    gas_c: f64,
    h: f64,
    fire_emissivity: f64,
    metal_emissivity: f64,
    metal_absorptivity: f64,
    pfp: f64,
}

impl Scenario {
    /// The class's heat flux into the outer surface, kW/m2.
    fn heat_flux_kw(&self, outer_c: f64) -> f64 {
        let radiative = STEFAN_BOLTZMANN_W_PER_M2K4
            * (self.fire_emissivity * self.metal_absorptivity
                * (self.fire_c + CLASS_OFFSET).powi(4)
                - self.metal_emissivity * (outer_c + CLASS_OFFSET).powi(4));
        let convective = self.h * (self.gas_c - outer_c);
        ((radiative + convective) * self.pfp).max(0.0) / 1000.0
    }
}

/// Refuse a value the arithmetic cannot carry to a published number.
fn require_finite(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

/// Refuse a non-positive value.
fn require_positive(name: &'static str, value: f64) -> Result<()> {
    require_finite(name, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name,
            "the class's builder refuses a non-positive value",
        ))
    }
}

/// The class's three factories, or the caller's seven.
#[allow(clippy::too_many_arguments)]
fn scenario(
    fire_scenario: &str,
    fire_temperature: Option<ThermodynamicTemperature>,
    gas_temperature: Option<ThermodynamicTemperature>,
    convective: Option<HeatTransfer>,
    fire_emissivity: Option<f64>,
    metal_emissivity: Option<f64>,
    metal_absorptivity: Option<f64>,
    pfp: Option<f64>,
) -> Result<Scenario> {
    let explicit = matches!(fire_scenario, "explicit");
    let built_in = match fire_scenario {
        "large_jet_fire" => Some((1155.0, 1155.0, 100.0)),
        "small_jet_fire" => Some((1000.0, 1000.0, 100.0)),
        "pool_fire" => Some((1125.0, 1125.0, 30.0)),
        "explicit" => None,
        other => {
            return Err(AzothError::invalid_input(
                "fire_scenario",
                format!("`{other}` is not `large_jet_fire`, `small_jet_fire`, `pool_fire` or `explicit`"),
            ));
        }
    };
    if built_in.is_some()
        && (fire_temperature.is_some()
            || gas_temperature.is_some()
            || convective.is_some()
            || fire_emissivity.is_some()
            || metal_emissivity.is_some()
            || metal_absorptivity.is_some())
    {
        return Err(AzothError::invalid_input(
            "fire_scenario",
            "the three built-in scenarios carry their own temperature and emissivity group",
        ));
    }
    if explicit
        && (fire_temperature.is_none()
            || gas_temperature.is_none()
            || convective.is_none()
            || fire_emissivity.is_none()
            || metal_emissivity.is_none()
            || metal_absorptivity.is_none())
    {
        return Err(AzothError::invalid_input(
            "fire_scenario",
            "the `explicit` scenario requires all six temperature and emissivity inputs",
        ));
    }
    let (fire_c, gas_c, h) = match built_in {
        Some(values) => values,
        None => (
            fire_temperature.expect("the group was checked").value - KELVIN_OFFSET,
            gas_temperature.expect("the group was checked").value - KELVIN_OFFSET,
            convective.expect("the group was checked").value,
        ),
    };
    let fire_e = fire_emissivity.unwrap_or(1.0);
    let metal_e = metal_emissivity.unwrap_or(0.85);
    let metal_a = metal_absorptivity.unwrap_or(0.85);
    let pfp = pfp.unwrap_or(1.0);
    for value in [fire_c, gas_c, h, fire_e, metal_e, metal_a, pfp] {
        require_finite("fire_scenario", value)?;
    }
    if fire_c + KELVIN_OFFSET <= 0.0 || gas_c + KELVIN_OFFSET <= 0.0 {
        return Err(AzothError::invalid_input(
            "fire_scenario",
            "a non-positive absolute fire or gas temperature is not a temperature",
        ));
    }
    if !(0.0..=1.0).contains(&fire_e)
        || !(0.0..=1.0).contains(&metal_e)
        || !(0.0..=1.0).contains(&metal_a)
        || !(0.0..=1.0).contains(&pfp)
    {
        return Err(AzothError::invalid_input(
            "fire_scenario",
            "the class's fractions are in [0, 1]",
        ));
    }
    if h < 0.0 {
        return Err(AzothError::invalid_input(
            "convective_heat_transfer_coefficient",
            "the class refuses a negative coefficient",
        ));
    }
    Ok(Scenario {
        fire_c,
        gas_c,
        h,
        fire_emissivity: fire_e,
        metal_emissivity: metal_e,
        metal_absorptivity: metal_a,
        pfp,
    })
}

/// The class's thick-wall von Mises stress, MPa.
fn von_mises_mpa(pressure_barg: f64, outside_diameter: f64, inside_diameter: f64, weight: f64) -> f64 {
    let pressure_mpa = 0.1 * pressure_barg.max(0.0);
    let denominator = outside_diameter * outside_diameter - inside_diameter * inside_diameter;
    let mean_diameter = (outside_diameter + inside_diameter) / 2.0;
    let inner_outer_product = inside_diameter * inside_diameter * outside_diameter * outside_diameter
        / (mean_diameter * mean_diameter);
    let radial =
        pressure_mpa * (inside_diameter * inside_diameter - inner_outer_product) / denominator;
    let hoop =
        pressure_mpa * (inside_diameter * inside_diameter + inner_outer_product) / denominator;
    let longitudinal =
        pressure_mpa * inside_diameter * inside_diameter / denominator + weight;
    (radial * radial + hoop * hoop + longitudinal * longitudinal
        - radial * hoop
        - radial * longitudinal
        - hoop * longitudinal)
        .sqrt()
}

/// The material at one state.
fn material_at(
    material_name: &str,
    metal_c: f64,
    stress_mpa: f64,
) -> Result<PipeFireRuptureMaterialResult> {
    pipe_fire_rupture_material(
        material_name,
        kelvins(metal_c + KELVIN_OFFSET),
        megapascals(stress_mpa),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

/// The profile's absolute pressure at one time, bara.
fn profile_bara(
    points: &[BlowdownProfilePoint],
    time_s: f64,
    mode: &str,
    offset_bar: f64,
) -> Result<f64> {
    let result = blowdown_pressure_profile(
        points,
        seconds(time_s),
        Some(mode),
        Some(bars(offset_bar)),
    )?;
    Ok(result.pressure_absolute.value / PA_PER_BAR)
}

/// The strain-rate pipe fire-rupture march.
///
/// # Errors
/// `InvalidInput` for a non-positive geometry, fluid or temperature, an unknown material or
/// scenario, a scenario group that is not all-or-none, a malformed profile, a non-positive
/// step or maximum time, a step count over the algorithm's cap, an unknown interpolation
/// mode, or a non-finite value.
#[allow(clippy::too_many_arguments)]
pub fn pipe_fire_rupture(
    outside_diameter: Length,
    nominal_wall_thickness: Length,
    material_name: &str,
    fire_scenario: &str,
    pressure_profile: &[BlowdownProfilePoint],
    corrosion_allowance: Option<Length>,
    wall_thickness_undertolerance: Option<f64>,
    exposed_length: Option<Length>,
    weight_stress: Option<Pressure>,
    fluid_density: Option<MassDensity>,
    fluid_heat_capacity: Option<SpecificHeatCapacity>,
    gas_molecular_weight: Option<MolarMass>,
    initial_temperature: Option<ThermodynamicTemperature>,
    fire_temperature: Option<ThermodynamicTemperature>,
    gas_temperature: Option<ThermodynamicTemperature>,
    convective_heat_transfer_coefficient: Option<HeatTransfer>,
    fire_emissivity: Option<f64>,
    metal_emissivity: Option<f64>,
    metal_absorptivity: Option<f64>,
    passive_protection_heat_flux_factor: Option<f64>,
    gauge_offset: Option<Pressure>,
    interpolation_mode: Option<&str>,
    time_step: Option<Time>,
    max_time: Option<Time>,
    spreadsheet_gas_thermal_mass: Option<bool>,
) -> Result<PipeFireRuptureResult> {
    let spec = &model_gen::PIPE_FIRE_RUPTURE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "outside_diameter" => Some(outside_diameter.value),
            "nominal_wall_thickness" => Some(nominal_wall_thickness.value),
            "corrosion_allowance" => corrosion_allowance.map(|value| value.value),
            "wall_thickness_undertolerance" => wall_thickness_undertolerance,
            "exposed_length" => exposed_length.map(|value| value.value),
            "weight_stress" => weight_stress.map(|value| value.value),
            "fluid_density" => fluid_density.map(|value| value.value),
            "fluid_heat_capacity" => fluid_heat_capacity.map(|value| value.value),
            "gas_molecular_weight" => gas_molecular_weight.map(|value| value.value),
            "initial_temperature" => initial_temperature.map(|value| value.value),
            "fire_temperature" => fire_temperature.map(|value| value.value),
            "gas_temperature" => gas_temperature.map(|value| value.value),
            "convective_heat_transfer_coefficient" => {
                convective_heat_transfer_coefficient.map(|value| value.value)
            }
            "fire_emissivity" => fire_emissivity,
            "metal_emissivity" => metal_emissivity,
            "metal_absorptivity" => metal_absorptivity,
            "passive_protection_heat_flux_factor" => passive_protection_heat_flux_factor,
            "gauge_offset" => gauge_offset.map(|value| value.value),
            "time_step" => time_step.map(|value| value.value),
            "max_time" => max_time.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    require_positive("outside_diameter", outside_diameter.value)?;
    require_positive("nominal_wall_thickness", nominal_wall_thickness.value)?;
    let corrosion = corrosion_allowance.map_or(0.0, |value| value.value);
    require_finite("corrosion_allowance", corrosion)?;
    if corrosion < 0.0 {
        return Err(AzothError::invalid_input(
            "corrosion_allowance",
            "a negative corrosion allowance is not a thickness",
        ));
    }
    let undertolerance = wall_thickness_undertolerance.unwrap_or(0.0);
    require_finite("wall_thickness_undertolerance", undertolerance)?;
    if !(0.0..1.0).contains(&undertolerance) {
        return Err(AzothError::invalid_input(
            "wall_thickness_undertolerance",
            "the class's setter refuses a fraction outside [0, 1)",
        ));
    }
    let exposed = exposed_length.map_or(1.0, |value| value.value);
    require_positive("exposed_length", exposed)?;
    let weight_mpa = weight_stress.map_or(0.0, |value| value.value / 1.0e6);
    require_finite("weight_stress", weight_mpa)?;
    let density = fluid_density.map_or(1.0, |value| value.value);
    require_positive("fluid_density", density)?;
    let heat_capacity = fluid_heat_capacity.map_or(2000.0, |value| value.value);
    require_positive("fluid_heat_capacity", heat_capacity)?;
    let molecular_weight_kg_mol = gas_molecular_weight.map_or(0.018, |value| value.value);
    require_positive("gas_molecular_weight", molecular_weight_kg_mol)?;
    let molecular_weight = molecular_weight_kg_mol * 1000.0;
    let initial_k = initial_temperature.map_or(293.15, |value| value.value);
    require_positive("initial_temperature", initial_k)?;
    let initial_c = initial_k - KELVIN_OFFSET;

    let offset_bar = gauge_offset.map_or(1.0, |value| value.value / PA_PER_BAR);
    require_finite("gauge_offset", offset_bar)?;
    if offset_bar < 0.0 {
        return Err(AzothError::invalid_input(
            "gauge_offset",
            "the class's profile refuses a negative gauge offset",
        ));
    }
    let mode = interpolation_mode.unwrap_or("step_previous");
    if mode != "step_previous" && mode != "linear" {
        return Err(AzothError::invalid_input(
            "interpolation_mode",
            format!("`{mode}` is not `step_previous` or `linear`"),
        ));
    }
    if pressure_profile.is_empty() {
        return Err(AzothError::invalid_input(
            "pressure_profile",
            "the class's profile needs at least two points",
        ));
    }
    // Validate the profile once, at t = 0, before the march.
    profile_bara(pressure_profile, 0.0, mode, offset_bar)?;

    let scenario = scenario(
        fire_scenario,
        fire_temperature,
        gas_temperature,
        convective_heat_transfer_coefficient,
        fire_emissivity,
        metal_emissivity,
        metal_absorptivity,
        passive_protection_heat_flux_factor,
    )?;

    let step_seconds = time_step.map_or(DEFAULT_TIME_STEP_SECONDS, |value| value.value);
    require_positive("time_step", step_seconds)?;
    let max_seconds = max_time.map_or(DEFAULT_MAX_TIME_SECONDS, |value| value.value);
    require_positive("max_time", max_seconds)?;
    let step_count = (max_seconds / step_seconds).ceil();
    require_finite("max_time", step_count)?;
    let cap = spec
        .algorithm
        .map_or(0, |algorithm| algorithm.max_iterations) as f64;
    if step_count + 1.0 > cap {
        return Err(AzothError::invalid_input(
            "max_time",
            format!(
                "{step_count} steps exceeds the {} this procedure carries",
                cap as usize
            ),
        ));
    }
    let spreadsheet_gas = spreadsheet_gas_thermal_mass.unwrap_or(true);

    let effective_wall =
        nominal_wall_thickness.value * (1.0 - undertolerance) - corrosion;
    if effective_wall <= 0.0 {
        return Err(AzothError::invalid_input(
            "nominal_wall_thickness",
            "the effective wall thickness must be positive",
        ));
    }
    let outside_diameter_m = outside_diameter.value;
    let inside_diameter_m = outside_diameter_m - 2.0 * effective_wall;
    if inside_diameter_m <= 0.0 {
        return Err(AzothError::invalid_input(
            "outside_diameter",
            "the effective inside diameter must be positive",
        ));
    }
    let exposed_area = PI * outside_diameter_m * exposed;
    let internal_volume =
        PI / 4.0 * inside_diameter_m * inside_diameter_m * exposed;

    let initial_material = material_at(material_name, initial_c, 0.0)?;
    let metal_density = initial_material.density.value;
    let basis = if density < GAS_LIQUID_DENSITY_SWITCH_KG_PER_M3 {
        FluidBasis::Gas
    } else {
        FluidBasis::Liquid
    };

    let mut mean_c = initial_c;
    let mut outer_c = initial_c;
    let mut heat_flux_kw = scenario.heat_flux_kw(outer_c);
    let mut accumulated = 0.0;
    let mut history = Vec::new();
    let mut rupture: Option<(f64, f64, f64, f64, f64, f64)> = None;
    let steps = step_count as usize;

    for step in 0..=steps {
        let time_seconds = (step as f64 * step_seconds).min(max_seconds);
        let strain_factor = 1.0 + accumulated.max(0.0);
        let outside = outside_diameter_m * strain_factor;
        let wall = effective_wall / strain_factor;
        let inside = outside - 2.0 * wall;
        if step > 0 {
            let current_barg =
                (profile_bara(pressure_profile, time_seconds, mode, offset_bar)? - offset_bar)
                    .max(0.0);
            let previous_mean = mean_c;
            let previous_flux = heat_flux_kw;
            let previous_material = material_at(material_name, previous_mean, 0.0)?;
            let conductivity = previous_material.thermal_conductivity.value;
            let wall_heat_capacity = previous_material.heat_capacity.value;
            let metal_mass = metal_density * exposed_area * effective_wall;
            let mut thermal_mass = metal_mass * wall_heat_capacity;
            if density >= GAS_LIQUID_DENSITY_SWITCH_KG_PER_M3 {
                thermal_mass += internal_volume * density * heat_capacity;
            } else if spreadsheet_gas {
                let gas_mass = current_barg * PA_PER_BAR * molecular_weight
                    / (UNIVERSAL_GAS_CONSTANT_J_PER_KMOLK * (previous_mean + CLASS_OFFSET))
                    * internal_volume;
                thermal_mass += gas_mass * heat_capacity;
            }
            mean_c += previous_flux * 1000.0 * exposed_area * step_seconds / thermal_mass;
            outer_c = mean_c + 0.5 * previous_flux * 1000.0 * wall / conductivity;
            heat_flux_kw = scenario.heat_flux_kw(outer_c);
        }

        let pressure_barg =
            (profile_bara(pressure_profile, time_seconds, mode, offset_bar)? - offset_bar)
                .max(0.0);
        let stress_mpa = von_mises_mpa(pressure_barg, outside, inside, weight_mpa);
        let properties = material_at(material_name, mean_c, stress_mpa)?;
        let rate_per_minute = properties.strain_rate.value * 60.0;
        if step > 0 {
            accumulated = (accumulated + step_seconds * rate_per_minute / 60.0).min(STRAIN_CAP);
        }
        let limit = properties.rupture_strain_limit;
        history.push(PipeFireRupturePoint {
            time: seconds(time_seconds),
            pressure: bars(pressure_barg),
            mean_wall_temperature: kelvins(mean_c + KELVIN_OFFSET),
            outer_surface_temperature: kelvins(outer_c + KELVIN_OFFSET),
            heat_flux: watts_per_square_meter(heat_flux_kw * 1000.0),
            von_mises_stress: megapascals(stress_mpa),
            strain_rate: per_second(rate_per_minute / 60.0),
            accumulated_strain: accumulated,
            rupture_strain_limit: limit,
        });
        if accumulated > limit {
            rupture = Some((
                time_seconds,
                pressure_barg,
                mean_c,
                outer_c,
                accumulated,
                limit,
            ));
            break;
        }
    }

    let (status, rupture_time, rupture_pressure, rupture_mean, rupture_outer, rupture_strain,
        rupture_limit) = match rupture {
        Some((time, pressure, mean, outer, strain, limit)) => (
            RuptureStatus::Rupture,
            Some(seconds(time)),
            Some(bars(pressure)),
            Some(kelvins(mean + KELVIN_OFFSET)),
            Some(kelvins(outer + KELVIN_OFFSET)),
            Some(strain),
            Some(limit),
        ),
        None => (
            RuptureStatus::NoRupture,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
    };

    let (two_sides, one_side, short, liquid) = match rupture {
        Some((_, pressure, _, _, _, _)) => {
            let area = PI / 4.0 * inside_diameter_m * inside_diameter_m;
            if density < GAS_LIQUID_DENSITY_SWITCH_KG_PER_M3 {
                let two = GAS_RELEASE_COEFFICIENT
                    * area
                    * pressure
                    * PA_PER_BAR
                    * (molecular_weight / (initial_c + CLASS_OFFSET).max(1.0)).sqrt();
                let one = two / 2.0;
                (
                    Some(kilograms_per_second(two)),
                    Some(kilograms_per_second(one)),
                    Some(kilograms_per_second(one * SHORT_PIPE_GAS_MULTIPLIER)),
                    Some(kilograms_per_second(0.0)),
                )
            } else {
                let specific_gravity = density / 1000.0;
                let liquid_rate = (area * 1.0e6 * LIQUID_DISCHARGE_COEFFICIENT
                    / LIQUID_FLOW_CONSTANT
                    / (specific_gravity / (pressure.max(1.0) * 100.0)).sqrt())
                    / 60.0
                    * specific_gravity;
                (
                    Some(kilograms_per_second(0.0)),
                    Some(kilograms_per_second(0.0)),
                    Some(kilograms_per_second(0.0)),
                    Some(kilograms_per_second(liquid_rate)),
                )
            }
        }
        None => (None, None, None, None),
    };

    Ok(PipeFireRuptureResult {
        status,
        fluid_basis: basis,
        rupture_time,
        rupture_pressure,
        rupture_mean_wall_temperature: rupture_mean,
        rupture_outer_surface_temperature: rupture_outer,
        rupture_accumulated_strain: rupture_strain,
        rupture_strain_limit: rupture_limit,
        long_pipe_gas_two_sides: two_sides,
        long_pipe_gas_one_side: one_side,
        short_pipe_gas_one_side: short,
        liquid_one_side: liquid,
        release_gas_temperature: rupture.map(|_| kelvins(initial_c + KELVIN_OFFSET)),
        history,
        warnings,
    })
}
