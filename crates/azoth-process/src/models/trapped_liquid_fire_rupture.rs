//! `process.trapped_liquid_fire_rupture` - the trapped-liquid fire rupture study.
//!
//! Spec: `specs/models/process/trapped_liquid_fire_rupture.toml`. The arithmetic is
//! NeqSim's `process/safety/rupture/TrappedLiquidFireRuptureStudy.java`, with the wall,
//! fire and material helper classes.

use azoth_core::units::{
    Length, Mass, MassDensity, Pressure, ThermodynamicTemperature, Time, Volume, kelvins, pascals,
    seconds,
};
use azoth_core::{AzothError, Result, Warning, WarningCode, apply_checks};
use azoth_eos::Cubic;
use azoth_eos::databank;
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::fire_exposure::FireExposure;
use crate::models::fluid_state;
use crate::models::material_strength::{
    allowable_rupture_stress_pa, api5l_strengths, flange_class_rating_pa, normalize_grade,
    temperature_reduced_flange_rating_pa, von_mises_pa,
};
use crate::models::transient_wall::TransientWall;

/// The fire exposure's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireKind {
    /// API 521 pool fire.
    Api521PoolFire,
    /// A fixed heat flux.
    FixedHeatFlux,
    /// A radiative fire.
    RadiativeFire,
}

impl FireKind {
    /// The spec's vocabulary, parsed.
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "api_521_pool_fire" => Ok(Self::Api521PoolFire),
            "fixed_heat_flux" => Ok(Self::FixedHeatFlux),
            "radiative_fire" => Ok(Self::RadiativeFire),
            other => Err(AzothError::invalid_input(
                "fire_type",
                format!(
                    "`{other}` is not `api_521_pool_fire`, `fixed_heat_flux` or `radiative_fire`"
                ),
            )),
        }
    }
}

/// The class's limiting failure mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FailureMode {
    /// No failure or demand.
    #[serde(rename = "none")]
    None,
    /// The relief set pressure.
    #[serde(rename = "relief_set_pressure")]
    ReliefSetPressure,
    /// A vapor pocket.
    #[serde(rename = "vapor_pocket")]
    VaporPocket,
    /// Pipe rupture.
    #[serde(rename = "pipe_rupture")]
    PipeRupture,
    /// Flange failure.
    #[serde(rename = "flange_failure")]
    FlangeFailure,
}

impl FailureMode {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ReliefSetPressure => "relief_set_pressure",
            Self::VaporPocket => "vapor_pocket",
            Self::PipeRupture => "pipe_rupture",
            Self::FlangeFailure => "flange_failure",
        }
    }
}

/// One recorded step.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RupturePoint {
    /// The step's time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
    /// The step's pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// The liquid temperature.
    #[serde(serialize_with = "scalar")]
    pub liquid_temperature: ThermodynamicTemperature,
    /// The inner wall surface temperature.
    #[serde(serialize_with = "scalar")]
    pub inner_wall_temperature: ThermodynamicTemperature,
    /// The outer wall surface temperature.
    #[serde(serialize_with = "scalar")]
    pub outer_wall_temperature: ThermodynamicTemperature,
    /// The thin-wall von Mises stress.
    #[serde(serialize_with = "scalar")]
    pub von_mises_stress: Pressure,
    /// The temperature-retained allowable stress.
    #[serde(serialize_with = "scalar")]
    pub allowable_stress: Pressure,
    /// The temperature-reduced flange rating.
    #[serde(serialize_with = "scalar")]
    pub flange_rating: Pressure,
}

/// Result of `process.trapped_liquid_fire_rupture`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrappedLiquidFireRuptureResult {
    /// The segment identifier.
    pub segment_id: String,
    /// The recorded time series.
    pub trajectory: Vec<RupturePoint>,
    /// The class's limiting mode.
    pub limiting_failure_mode: FailureMode,
    /// Whether the limiting mode is a pipe or flange failure.
    pub rupture_predicted: bool,
    /// The first relief-set time.
    #[serde(serialize_with = "optional_scalar")]
    pub time_to_relief_set: Option<Time>,
    /// The first vapor-pocket time.
    #[serde(serialize_with = "optional_scalar")]
    pub time_to_vapor_pocket: Option<Time>,
    /// The first pipe-rupture time.
    #[serde(serialize_with = "optional_scalar")]
    pub time_to_pipe_rupture: Option<Time>,
    /// The first flange-failure time.
    #[serde(serialize_with = "optional_scalar")]
    pub time_to_flange_failure: Option<Time>,
    /// The earlier of the pipe and flange times.
    #[serde(serialize_with = "optional_scalar")]
    pub minimum_failure_time: Option<Time>,
    /// The last pressure.
    #[serde(serialize_with = "scalar")]
    pub final_pressure: Pressure,
    /// The last liquid temperature.
    #[serde(serialize_with = "scalar")]
    pub final_liquid_temperature: ThermodynamicTemperature,
    /// The last outer wall temperature.
    #[serde(serialize_with = "scalar")]
    pub final_outer_wall_temperature: ThermodynamicTemperature,
    /// The class's applied standards.
    pub standards_applied: Vec<String>,
    /// The class's recommendations.
    pub recommendations: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The trapped-liquid fire rupture study.
///
/// # Errors
/// `InvalidInput` for a missing fire parameter, a non-positive geometry, time, wall or
/// liquid property, fewer than three wall nodes, an unknown grade or fire type, or a
/// factor outside its range.
#[allow(clippy::too_many_arguments)]
pub fn trapped_liquid_fire_rupture(
    components: &[String],
    z: &[f64],
    inventory_pressure: Pressure,
    inventory_temperature: ThermodynamicTemperature,
    inventory_liquid_mass: Mass,
    inventory_total_mass: Mass,
    inventory_total_volume: Volume,
    pipe_internal_diameter: Length,
    wall_thickness: Length,
    exposed_length: Length,
    eos: Option<&str>,
    multiphase_check: Option<bool>,
    segment_id: Option<&str>,
    fire_type: Option<&str>,
    environmental_factor: Option<f64>,
    fire_heat_flux: Option<f64>,
    fire_emissivity: Option<f64>,
    fire_view_factor: Option<f64>,
    flame_temperature: Option<ThermodynamicTemperature>,
    ambient_temperature: Option<ThermodynamicTemperature>,
    passive_protection_factor: Option<f64>,
    fire_exposed_area: Option<f64>,
    api5l_grade: Option<&str>,
    ambient_tensile_strength: Option<Pressure>,
    flange_class: Option<i64>,
    relief_set_pressure: Option<Pressure>,
    relief_limits_pressure: Option<bool>,
    max_time: Option<Time>,
    time_step: Option<Time>,
    wall_density: Option<MassDensity>,
    wall_heat_capacity: Option<f64>,
    wall_conductivity: Option<f64>,
    liquid_heat_capacity: Option<f64>,
    liquid_expansion: Option<f64>,
    liquid_bulk_modulus: Option<Pressure>,
    inner_htc: Option<f64>,
    outer_htc: Option<f64>,
    tensile_strength_factor: Option<f64>,
    wall_nodes: Option<usize>,
    vapor_pocket_detection: Option<bool>,
) -> Result<TrappedLiquidFireRuptureResult> {
    let spec = &model_gen::TRAPPED_LIQUID_FIRE_RUPTURE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "inventory_pressure" => Some(inventory_pressure.value),
            "inventory_temperature" => Some(inventory_temperature.value),
            "inventory_total_mass" => Some(inventory_total_mass.value),
            "inventory_total_volume" => Some(inventory_total_volume.value),
            "pipe_internal_diameter" => Some(pipe_internal_diameter.value),
            "wall_thickness" => Some(wall_thickness.value),
            "exposed_length" => Some(exposed_length.value),
            "max_time" => max_time.map(|value| value.value),
            "time_step" => time_step.map(|value| value.value),
            "tensile_strength_factor" => tensile_strength_factor,
            "passive_protection_factor" => passive_protection_factor,
            _ => None,
        },
        &mut warnings,
    )?;

    let diameter = pipe_internal_diameter.value;
    let wall = wall_thickness.value;
    let length = exposed_length.value;
    let outer_diameter = diameter + 2.0 * wall;
    let area = std::f64::consts::PI * outer_diameter * length;
    let exposed_area = fire_exposed_area.unwrap_or(area);
    let ambient = ambient_temperature.map_or(298.15, |value| value.value);
    let kind = FireKind::parse(fire_type.unwrap_or("api_521_pool_fire"))?;
    let mut fire = match kind {
        FireKind::Api521PoolFire => {
            FireExposure::api521_pool_fire(exposed_area, environmental_factor.unwrap_or(1.0))?
        }
        FireKind::FixedHeatFlux => {
            let flux = fire_heat_flux.ok_or_else(|| {
                AzothError::invalid_input("fire_heat_flux", "a fixed flux is required")
            })?;
            FireExposure::fixed_heat_flux(exposed_area, flux)?
        }
        FireKind::RadiativeFire => {
            let emissivity = fire_emissivity.ok_or_else(|| {
                AzothError::invalid_input(
                    "fire_emissivity",
                    "a radiative fire states its emissivity",
                )
            })?;
            let view = fire_view_factor.ok_or_else(|| {
                AzothError::invalid_input(
                    "fire_view_factor",
                    "a radiative fire states its view factor",
                )
            })?;
            let flame = flame_temperature.map_or(1200.0, |value| value.value);
            FireExposure::radiative_fire(exposed_area, emissivity, view, flame, ambient)?
        }
    };
    if let Some(factor) = passive_protection_factor {
        fire = fire.with_passive_protection(factor)?;
    }

    let tensile = match ambient_tensile_strength {
        Some(value) => value.value,
        None => {
            let grade = api5l_grade.unwrap_or("B");
            let normalized = normalize_grade(grade)?;
            api5l_strengths(&normalized)?.1
        }
    };
    let factor = tensile_strength_factor.unwrap_or(0.75);
    if !(factor > 0.0 && factor.is_finite()) {
        return Err(AzothError::invalid_input(
            "tensile_strength_factor",
            "the class requires a positive factor",
        ));
    }
    let flange_rating_ambient = flange_class_rating_pa(flange_class.unwrap_or(0));

    let liquid_mass = inventory_liquid_mass.value.max(inventory_total_mass.value);
    if inventory_liquid_mass.value <= 0.0 {
        warnings.push(Warning::for_field(
            WarningCode::FallbackToInput,
            "inventory_liquid_mass",
            "No liquid inventory was reported; total mass was used for thermal capacity.",
        ));
    }

    let step = time_step.map_or(2.0, |value| value.value);
    let horizon = max_time.map_or(1800.0, |value| value.value);
    if !(step > 0.0 && step.is_finite() && horizon > 0.0 && horizon.is_finite()) {
        return Err(AzothError::invalid_input(
            "time_step",
            "the time control is positive and finite",
        ));
    }
    let nodes = wall_nodes.unwrap_or(15);
    if nodes < 3 {
        return Err(AzothError::invalid_input(
            "wall_nodes",
            "the class requires at least three nodes",
        ));
    }
    let wall_density = wall_density.map_or(7850.0, |value| value.value);
    let wall_cp = wall_heat_capacity.unwrap_or(500.0);
    let wall_k = wall_conductivity.unwrap_or(45.0);
    let liquid_cp = liquid_heat_capacity.unwrap_or(2200.0);
    let liquid_expansion = liquid_expansion.unwrap_or(7.0e-4);
    let bulk_modulus = liquid_bulk_modulus.map_or(1.2e9, |value| value.value);
    let inner_htc = inner_htc.unwrap_or(500.0);
    let outer_htc = outer_htc.unwrap_or(35.0);
    let initial_pressure = inventory_pressure.value;
    let initial_temperature = inventory_temperature.value;
    let mut transient = TransientWall::new(
        wall,
        wall_k,
        wall_density,
        wall_cp,
        initial_temperature,
        nodes,
    )?;

    let cubic = match eos.unwrap_or("pr") {
        "pr" => Cubic::Pr,
        "srk" => Cubic::Srk,
        other => {
            return Err(AzothError::invalid_input(
                "eos",
                format!("`{other}` is not `pr` or `srk`"),
            ));
        }
    };
    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, cubic, None)?;

    let steps = (horizon / step).ceil() as usize;
    let mut trajectory = Vec::with_capacity(steps + 1);
    let mut pressure = initial_pressure;
    let mut liquid_temperature = initial_temperature;
    let mut time_to_relief_set = None;
    let mut time_to_vapor_pocket = None;
    let mut time_to_pipe_rupture = None;
    let mut time_to_flange_failure = None;
    let vacuum = vapor_pocket_detection.unwrap_or(true);
    let relief_limit = relief_limits_pressure.unwrap_or(false);
    let set_pressure = relief_set_pressure.map(|value| value.value);

    for index in 0..=steps {
        let time = (index as f64 * step).min(horizon);
        let outer = transient.outer_wall_temperature();
        let inner = transient.inner_wall_temperature();
        let allowable = allowable_rupture_stress_pa(tensile, outer, factor);
        let stress = von_mises_pa(pressure, diameter / 2.0, wall);
        let flange_rating = temperature_reduced_flange_rating_pa(flange_rating_ambient, outer);
        trajectory.push(RupturePoint {
            time: seconds(time),
            pressure: pascals(pressure),
            liquid_temperature: kelvins(liquid_temperature),
            inner_wall_temperature: kelvins(inner),
            outer_wall_temperature: kelvins(outer),
            von_mises_stress: pascals(stress),
            allowable_stress: pascals(allowable),
            flange_rating: pascals(flange_rating),
        });
        if let Some(set) = set_pressure {
            if pressure >= set && time_to_relief_set.is_none() {
                time_to_relief_set = Some(seconds(time));
            }
        }
        if vacuum && time_to_vapor_pocket.is_none() {
            if let Ok(state) = fluid_state::flash(
                &mixture,
                &ideal_gas,
                liquid_temperature,
                pressure,
                z,
                multiphase_check.unwrap_or(false),
            ) {
                if state.gas_mole_fraction() > 1.0e-10 {
                    time_to_vapor_pocket = Some(seconds(time));
                }
            }
        }
        if stress >= allowable && time_to_pipe_rupture.is_none() {
            time_to_pipe_rupture = Some(seconds(time));
        }
        if pressure >= flange_rating && time_to_flange_failure.is_none() {
            time_to_flange_failure = Some(seconds(time));
        }
        if stress >= allowable || pressure >= flange_rating {
            break;
        }
        if index == steps {
            break;
        }

        let previous_mean = transient.mean_wall_temperature();
        let flux = fire.incident_heat_flux(outer)?;
        transient.advance_time_step(
            step,
            liquid_temperature,
            inner_htc,
            fire.ambient_temperature,
            outer_htc,
            flux,
        )?;
        let absorbed = transient.heat_absorbed(area, previous_mean);
        let incident = flux * area * step;
        let heat_to_liquid = (incident - absorbed.max(0.0)).max(0.0);
        liquid_temperature += heat_to_liquid / (liquid_mass * liquid_cp);
        pressure = initial_pressure
            + bulk_modulus * liquid_expansion * (liquid_temperature - initial_temperature).max(0.0);
        if let Some(set) = set_pressure {
            if pressure >= set && relief_limit {
                pressure = set;
            }
        }
    }

    let limiting_failure_mode = determine_mode(
        time_to_pipe_rupture,
        time_to_flange_failure,
        time_to_relief_set,
        time_to_vapor_pocket,
    );
    let minimum_failure_time = match (time_to_pipe_rupture, time_to_flange_failure) {
        (Some(left), Some(right)) => Some(if left.value <= right.value {
            left
        } else {
            right
        }),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    };
    let last = trajectory
        .last()
        .expect("the trajectory has at least one point");
    let final_pressure = last.pressure;
    let final_liquid_temperature = last.liquid_temperature;
    let final_outer_wall_temperature = last.outer_wall_temperature;
    Ok(TrappedLiquidFireRuptureResult {
        segment_id: segment_id.unwrap_or("trapped-segment").to_string(),
        trajectory,
        limiting_failure_mode,
        rupture_predicted: matches!(
            limiting_failure_mode,
            FailureMode::PipeRupture | FailureMode::FlangeFailure
        ),
        time_to_relief_set,
        time_to_vapor_pocket,
        time_to_pipe_rupture,
        time_to_flange_failure,
        minimum_failure_time,
        final_pressure,
        final_liquid_temperature,
        final_outer_wall_temperature,
        standards_applied: vec![
            "API 521 / ISO 23251 fire exposure and relief screening basis".to_string(),
            "ASME B31.3/B31.4 pressure piping stress screening".to_string(),
            "ASME B16.5 flange class pressure-temperature screening".to_string(),
            "CCPS/TNO consequence handoff basis for post-rupture source terms".to_string(),
        ],
        recommendations: vec![
            "Verify trapped inventory and isolation boundaries from current P&IDs, line lists, and valve positions.".to_string(),
            "Use certified material, flange, bolt, and gasket pressure-temperature data for final design decisions.".to_string(),
            "Evaluate PSV, thermal relief, drain/vent, or procedural safeguards for blocked-in liquid segments exposed to fire.".to_string(),
            "Use the source-term handoff for consequence screening if pipe or flange failure is predicted.".to_string(),
        ],
        warnings,
    })
}

fn determine_mode(
    pipe: Option<Time>,
    flange: Option<Time>,
    relief: Option<Time>,
    vapor: Option<Time>,
) -> FailureMode {
    let mut best = f64::INFINITY;
    let mut mode = FailureMode::None;
    if let Some(value) = pipe {
        if value.value < best {
            best = value.value;
            mode = FailureMode::PipeRupture;
        }
    }
    if let Some(value) = flange {
        if value.value < best {
            mode = FailureMode::FlangeFailure;
        }
    }
    if mode == FailureMode::None && relief.is_some() {
        mode = FailureMode::ReliefSetPressure;
    }
    if mode == FailureMode::None && vapor.is_some() {
        mode = FailureMode::VaporPocket;
    }
    mode
}
