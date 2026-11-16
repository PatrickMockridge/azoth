//! The safety ids this workspace can run, called as the typed kernels they are.
//!
//! **Not `azoth_sheets::dispatch`.** That dispatcher is generated from `specs/calcs`, and the
//! safety ids are models under `specs/models/process/`; no generated arm reaches them. This is
//! the small hand-written table that does, in the calculation book's own outcome shape.

use std::collections::BTreeMap;

use azoth_core::Warning;
use azoth_core::units::{
    bars, cubic_meters_per_second, kelvins, kilograms, kilograms_per_cubic_meter,
    kilograms_per_mole, kilograms_per_second, kilograms_per_square_meter_second, liters_per_minute,
    meters, meters_per_second, pascals, seconds, square_meters, watts_per_meter_kelvin,
    watts_per_square_meter,
};
use azoth_process::models::bow_tie::{BowTieBarrier, BowTieConsequence, BowTieThreat};
pub use azoth_sheets::dispatch::Outcome;
use azoth_sheets::dispatch_support::{
    number, optional_flag, optional_number, optional_text, refused, text, warnings,
};
use azoth_sheets::workspace::CalcOutcome;
use serde_json::{Map, Value, json};

/// Every id this workspace can run natively.
pub const NATIVE: &[&str] = &[
    "process.active_fire_protection_screening",
    "process.bleve",
    "process.bow_tie",
    "process.deluge_nozzle_layout",
    "process.fire_monitor_coverage",
    "process.fire_water_coverage_assessment",
    "process.gaussian_plume",
    "process.hazardous_area",
    "process.heavy_gas_dispersion",
    "process.jet_fire",
    "process.pfp_demand",
    "process.pool_fire",
    "process.probit",
    "process.toxic_library",
    "process.vce",
];

/// Whether a safety calculation has a native arm here.
#[must_use]
pub fn is_native(id: &str) -> bool {
    NATIVE.contains(&id)
}

/// The inputs a native id cannot run without, or `None` for an id with no arm.
///
/// The list is written beside the parsing that reads it, so a renamed input is one edit rather
/// than two tables that can disagree.
#[must_use]
pub fn required(id: &str) -> Option<&'static [&'static str]> {
    Some(match id {
        "process.active_fire_protection_screening" => &[
            "scenario",
            "blowdown_time_to_target",
            "bare_steel_time_to_critical",
            "dedicated_object_demand",
            "general_area_demand",
        ],
        "process.bleve" => &["fuel_mass", "heat_of_combustion", "radiative_fraction"],
        "process.bow_tie" => &["threats", "consequences", "barriers"],
        "process.deluge_nozzle_layout" => &[
            "protected_area",
            "required_density",
            "nozzle_k",
            "nozzle_min_pressure",
            "nozzle_max_spacing",
            "nozzle_clearance",
            "available_clearance",
        ],
        "process.fire_monitor_coverage" => &["target_area", "required_density"],
        "process.fire_water_coverage_assessment" => &[
            "protected_area",
            "required_flow",
            "spare_supply_flow",
            "worst_case_pressure_margin",
        ],
        "process.gaussian_plume" => &[
            "emission_rate",
            "effective_stack_height",
            "wind_speed",
            "stability",
            "terrain",
            "distance",
            "crosswind",
            "height",
            "threshold",
        ],
        "process.hazardous_area" => &[
            "mass_flow",
            "exit_density",
            "exit_velocity",
            "lfl_volume_fraction",
            "gas_molar_mass",
        ],
        "process.heavy_gas_dispersion" => &[
            "volumetric_rate",
            "source_density",
            "air_density",
            "wind_speed",
        ],
        "process.jet_fire" => &["mass_flow", "heat_of_combustion", "radiative_fraction"],
        "process.pfp_demand" => &[
            "incident_heat_flux",
            "steel_thickness",
            "required_survival_time",
        ],
        "process.pool_fire" => &["pool_diameter", "burn_rate"],
        "process.probit" => &["a", "b", "n"],
        "process.toxic_library" => &[],
        "process.vce" => &["cloud_fuel_mass", "heat_of_combustion", "strength_class"],
        _ => return None,
    })
}

/// Run one native safety calculation.
///
/// A missing or mistyped input is an [`Outcome::Error`] naming the field, which is the same
/// shape a kernel's own refusal takes.
#[must_use]
pub fn run(id: &str, inputs: &Value) -> Outcome {
    match arm(id, inputs) {
        Ok(outcome) | Err(outcome) => outcome,
    }
}

fn arm(id: &str, inputs: &Value) -> Result<Outcome, Outcome> {
    match id {
        "process.active_fire_protection_screening" => active_fire_protection_screening(inputs),
        "process.bow_tie" => bow_tie(inputs),
        "process.deluge_nozzle_layout" => deluge_nozzle_layout(inputs),
        "process.fire_monitor_coverage" => fire_monitor_coverage(inputs),
        "process.fire_water_coverage_assessment" => fire_water_coverage_assessment(inputs),
        "process.gaussian_plume" => gaussian_plume(inputs),
        "process.hazardous_area" => hazardous_area(inputs),
        "process.heavy_gas_dispersion" => heavy_gas_dispersion(inputs),
        "process.jet_fire" => jet_fire(inputs),
        "process.pool_fire" => pool_fire(inputs),
        "process.bleve" => bleve(inputs),
        "process.vce" => vce(inputs),
        "process.pfp_demand" => pfp_demand(inputs),
        "process.probit" => probit(inputs),
        "process.toxic_library" => toxic_library(inputs),
        other => Err(Outcome::Error {
            message: format!("`{other}` is not a safety calculation this workspace can run"),
            field: None,
        }),
    }
}

fn jet_fire(inputs: &Value) -> Result<Outcome, Outcome> {
    let mass_flow = kilograms_per_second(number(inputs, "mass_flow")?);
    let heat_of_combustion = number(inputs, "heat_of_combustion")?;
    let radiative_fraction = number(inputs, "radiative_fraction")?;
    let relative_humidity = optional_number(inputs, "relative_humidity")?;
    let distance = optional_number(inputs, "distance")?.map(meters);
    let target_flux = optional_number(inputs, "target_flux")?.map(watts_per_square_meter);
    Ok(
        match azoth_process::jet_fire(
            mass_flow,
            heat_of_combustion,
            radiative_fraction,
            relative_humidity,
            distance,
            target_flux,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                quantity(
                    &mut outputs,
                    "total_radiative_power",
                    result.total_radiative_power.value,
                    "W",
                );
                scalar_optional(&mut outputs, "transmissivity", result.transmissivity);
                quantity_optional(
                    &mut outputs,
                    "incident_heat_flux",
                    result.incident_heat_flux.map(|value| value.value),
                    "W/m**2",
                );
                quantity_optional(
                    &mut outputs,
                    "distance_to_flux",
                    result.distance_to_flux.map(|value| value.value),
                    "m",
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn pool_fire(inputs: &Value) -> Result<Outcome, Outcome> {
    let pool_diameter = meters(number(inputs, "pool_diameter")?);
    let burn_rate = kilograms_per_square_meter_second(number(inputs, "burn_rate")?);
    let air_density = optional_number(inputs, "air_density")?;
    let distance = optional_number(inputs, "distance")?.map(meters);
    let target_flux = optional_number(inputs, "target_flux")?.map(watts_per_square_meter);
    Ok(
        match azoth_process::pool_fire(pool_diameter, burn_rate, air_density, distance, target_flux)
        {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                quantity(
                    &mut outputs,
                    "total_burn_rate",
                    result.total_burn_rate.value,
                    "kg/s",
                );
                quantity(&mut outputs, "flame_height", result.flame_height.value, "m");
                quantity(
                    &mut outputs,
                    "surface_emissive_power",
                    result.surface_emissive_power.value,
                    "W/m**2",
                );
                scalar_optional(&mut outputs, "view_factor", result.view_factor);
                quantity_optional(
                    &mut outputs,
                    "incident_heat_flux",
                    result.incident_heat_flux.map(|value| value.value),
                    "W/m**2",
                );
                quantity_optional(
                    &mut outputs,
                    "distance_to_flux",
                    result.distance_to_flux.map(|value| value.value),
                    "m",
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn bleve(inputs: &Value) -> Result<Outcome, Outcome> {
    let fuel_mass = kilograms(number(inputs, "fuel_mass")?);
    let heat_of_combustion = number(inputs, "heat_of_combustion")?;
    let radiative_fraction = number(inputs, "radiative_fraction")?;
    let relative_humidity = optional_number(inputs, "relative_humidity")?;
    let distance = optional_number(inputs, "distance")?.map(meters);
    let target_flux = optional_number(inputs, "target_flux")?.map(watts_per_square_meter);
    Ok(
        match azoth_process::bleve(
            fuel_mass,
            heat_of_combustion,
            radiative_fraction,
            relative_humidity,
            distance,
            target_flux,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                quantity(
                    &mut outputs,
                    "fireball_diameter",
                    result.fireball_diameter.value,
                    "m",
                );
                quantity(
                    &mut outputs,
                    "fireball_duration",
                    result.fireball_duration.value,
                    "s",
                );
                quantity(
                    &mut outputs,
                    "surface_emissive_power",
                    result.surface_emissive_power.value,
                    "W/m**2",
                );
                scalar_optional(&mut outputs, "transmissivity", result.transmissivity);
                quantity_optional(
                    &mut outputs,
                    "incident_heat_flux",
                    result.incident_heat_flux.map(|value| value.value),
                    "W/m**2",
                );
                quantity_optional(
                    &mut outputs,
                    "distance_to_flux",
                    result.distance_to_flux.map(|value| value.value),
                    "m",
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn vce(inputs: &Value) -> Result<Outcome, Outcome> {
    let cloud_fuel_mass = kilograms(number(inputs, "cloud_fuel_mass")?);
    let heat_of_combustion = number(inputs, "heat_of_combustion")?;
    let strength_class = number(inputs, "strength_class")?;
    let ambient_pressure = optional_number(inputs, "ambient_pressure")?.map(pascals);
    let distance = optional_number(inputs, "distance")?.map(meters);
    let target_overpressure = optional_number(inputs, "target_overpressure")?.map(pascals);
    Ok(
        match azoth_process::vce(
            cloud_fuel_mass,
            heat_of_combustion,
            strength_class,
            ambient_pressure,
            distance,
            target_overpressure,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                quantity(
                    &mut outputs,
                    "combustion_energy",
                    result.combustion_energy.value,
                    "J",
                );
                scalar_optional(&mut outputs, "scaled_distance", result.scaled_distance);
                scalar_optional(
                    &mut outputs,
                    "dimensionless_overpressure",
                    result.dimensionless_overpressure,
                );
                quantity_optional(
                    &mut outputs,
                    "overpressure",
                    result.overpressure.map(|value| value.value),
                    "Pa",
                );
                quantity_optional(
                    &mut outputs,
                    "distance_to_overpressure",
                    result.distance_to_overpressure.map(|value| value.value),
                    "m",
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn pfp_demand(inputs: &Value) -> Result<Outcome, Outcome> {
    let incident_heat_flux = watts_per_square_meter(number(inputs, "incident_heat_flux")?);
    let steel_thickness = meters(number(inputs, "steel_thickness")?);
    let required_survival_time = seconds(number(inputs, "required_survival_time")?);
    let absorptivity = optional_number(inputs, "absorptivity")?;
    let initial_temperature = optional_number(inputs, "initial_temperature")?.map(kelvins);
    let critical_temperature = optional_number(inputs, "critical_temperature")?.map(kelvins);
    let fire_temperature = optional_number(inputs, "fire_temperature")?.map(kelvins);
    let pfp_conductivity = optional_number(inputs, "pfp_conductivity")?.map(watts_per_meter_kelvin);
    let fire_type = optional_text(inputs, "fire_type")?;
    Ok(
        match azoth_process::pfp_demand(
            incident_heat_flux,
            steel_thickness,
            required_survival_time,
            absorptivity,
            initial_temperature,
            critical_temperature,
            fire_temperature,
            pfp_conductivity,
            fire_type,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                scalar(
                    &mut outputs,
                    "steel_mass_per_area",
                    result.steel_mass_per_area,
                );
                quantity(
                    &mut outputs,
                    "bare_steel_time_to_critical",
                    result.bare_steel_time_to_critical.value,
                    "s",
                );
                quantity(
                    &mut outputs,
                    "required_survival_time",
                    result.required_survival_time.value,
                    "s",
                );
                outputs.insert("pfp_required".to_string(), json!(result.pfp_required));
                quantity(
                    &mut outputs,
                    "required_pfp_thickness",
                    result.required_pfp_thickness.value,
                    "m",
                );
                quantity(
                    &mut outputs,
                    "required_pfp_thickness_mm",
                    result.required_pfp_thickness_mm.value,
                    "mm",
                );
                outputs.insert("rating".to_string(), json!(result.rating));
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn bow_tie(inputs: &Value) -> Result<Outcome, Outcome> {
    let raw_threats = records(inputs, "threats")?;
    let mut threats = Vec::with_capacity(raw_threats.len());
    for (index, entry) in raw_threats.iter().enumerate() {
        let what = format!("threats[{index}]");
        let object = expect_object(entry, &what)?;
        threats.push(BowTieThreat {
            frequency: field_number(object, &what, "frequency")?,
            active: optional_field_flag(object, &what, "active")?,
        });
    }

    let raw_consequences = records(inputs, "consequences")?;
    let mut consequences = Vec::with_capacity(raw_consequences.len());
    for (index, entry) in raw_consequences.iter().enumerate() {
        let what = format!("consequences[{index}]");
        let object = expect_object(entry, &what)?;
        consequences.push(BowTieConsequence {
            severity: field_number(object, &what, "severity")?,
        });
    }

    let raw_barriers = records(inputs, "barriers")?;
    let mut barriers = Vec::with_capacity(raw_barriers.len());
    for (index, entry) in raw_barriers.iter().enumerate() {
        let what = format!("barriers[{index}]");
        let object = expect_object(entry, &what)?;
        barriers.push(BowTieBarrier {
            pfd: field_number(object, &what, "pfd")?,
            barrier_type: field_text(object, &what, "barrier_type")?.to_string(),
            functional: optional_field_flag(object, &what, "functional")?,
        });
    }

    Ok(
        match azoth_process::bow_tie(&threats, &consequences, &barriers) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                scalar(
                    &mut outputs,
                    "unmitigated_frequency_per_year",
                    result.unmitigated_frequency_per_year,
                );
                scalar(
                    &mut outputs,
                    "mitigated_frequency_per_year",
                    result.mitigated_frequency_per_year,
                );
                scalar(&mut outputs, "total_rrf", result.total_rrf);
                scalar(&mut outputs, "max_severity", result.max_severity);
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn gaussian_plume(inputs: &Value) -> Result<Outcome, Outcome> {
    let emission_rate = kilograms_per_second(number(inputs, "emission_rate")?);
    let effective_stack_height = meters(number(inputs, "effective_stack_height")?);
    let wind_speed = meters_per_second(number(inputs, "wind_speed")?);
    let stability = text(inputs, "stability")?;
    let terrain = text(inputs, "terrain")?;
    let distance = meters(number(inputs, "distance")?);
    let crosswind = meters(number(inputs, "crosswind")?);
    let height = meters(number(inputs, "height")?);
    let threshold = kilograms_per_cubic_meter(number(inputs, "threshold")?);
    Ok(
        match azoth_process::gaussian_plume(
            emission_rate,
            effective_stack_height,
            wind_speed,
            stability,
            terrain,
            distance,
            crosswind,
            height,
            threshold,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                quantity(&mut outputs, "sigma_y", result.sigma_y.value, "m");
                quantity(&mut outputs, "sigma_z", result.sigma_z.value, "m");
                quantity(
                    &mut outputs,
                    "concentration",
                    result.concentration.value,
                    "kg/m**3",
                );
                quantity(
                    &mut outputs,
                    "centerline_ground_concentration",
                    result.centerline_ground_concentration.value,
                    "kg/m**3",
                );
                quantity_optional(
                    &mut outputs,
                    "distance_to_threshold",
                    result.distance_to_threshold.map(|value| value.value),
                    "m",
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn hazardous_area(inputs: &Value) -> Result<Outcome, Outcome> {
    let mass_flow = kilograms_per_second(number(inputs, "mass_flow")?);
    let exit_density = kilograms_per_cubic_meter(number(inputs, "exit_density")?);
    let exit_velocity = meters_per_second(number(inputs, "exit_velocity")?);
    let lfl_volume_fraction = number(inputs, "lfl_volume_fraction")?;
    let gas_molar_mass = kilograms_per_mole(number(inputs, "gas_molar_mass")?);
    let ambient_density =
        optional_number(inputs, "ambient_density")?.map(kilograms_per_cubic_meter);
    let safety_factor = optional_number(inputs, "safety_factor")?;
    let release_grade = optional_text(inputs, "release_grade")?;
    Ok(
        match azoth_process::hazardous_area(
            mass_flow,
            exit_density,
            exit_velocity,
            lfl_volume_fraction,
            gas_molar_mass,
            ambient_density,
            safety_factor,
            release_grade,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                quantity(
                    &mut outputs,
                    "effective_diameter",
                    result.effective_diameter.value,
                    "m",
                );
                scalar(&mut outputs, "lfl_mass_fraction", result.lfl_mass_fraction);
                quantity(
                    &mut outputs,
                    "hazardous_distance",
                    result.hazardous_distance.value,
                    "m",
                );
                outputs.insert("release_grade".to_string(), json!(result.release_grade));
                outputs.insert(
                    "zone_classification".to_string(),
                    json!(result.zone_classification),
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn heavy_gas_dispersion(inputs: &Value) -> Result<Outcome, Outcome> {
    let volumetric_rate = cubic_meters_per_second(number(inputs, "volumetric_rate")?);
    let source_density = kilograms_per_cubic_meter(number(inputs, "source_density")?);
    let air_density = kilograms_per_cubic_meter(number(inputs, "air_density")?);
    let wind_speed = meters_per_second(number(inputs, "wind_speed")?);
    let concentration_ratio = optional_number(inputs, "concentration_ratio")?;
    Ok(
        match azoth_process::heavy_gas_dispersion(
            volumetric_rate,
            source_density,
            air_density,
            wind_speed,
            concentration_ratio,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                scalar(&mut outputs, "reduced_gravity", result.reduced_gravity);
                scalar(&mut outputs, "britter_parameter", result.britter_parameter);
                quantity_optional(
                    &mut outputs,
                    "distance_to_concentration_ratio",
                    result
                        .distance_to_concentration_ratio
                        .map(|value| value.value),
                    "m",
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn probit(inputs: &Value) -> Result<Outcome, Outcome> {
    let a = number(inputs, "a")?;
    let b = number(inputs, "b")?;
    let n = number(inputs, "n")?;
    let dose = optional_number(inputs, "dose")?;
    let probit_value = optional_number(inputs, "probit_value")?;
    Ok(match azoth_process::probit(a, b, n, dose, probit_value) {
        Ok(result) => {
            let mut outputs = BTreeMap::new();
            scalar_optional(&mut outputs, "probit", result.probit);
            scalar(&mut outputs, "probability", result.probability);
            done(outputs, &result.warnings)
        }
        Err(error) => refused(error),
    })
}

fn toxic_library(inputs: &Value) -> Result<Outcome, Outcome> {
    let chemical = optional_text(inputs, "chemical")?;
    let concentration_ppm = optional_number(inputs, "concentration_ppm")?;
    let molar_mass = optional_number(inputs, "molar_mass")?.map(kilograms_per_mole);
    let temperature = optional_number(inputs, "temperature")?.map(kelvins);
    let pressure = optional_number(inputs, "pressure")?.map(pascals);
    Ok(
        match azoth_process::toxic_library(
            chemical,
            concentration_ppm,
            molar_mass,
            temperature,
            pressure,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                if let Some(known) = result.known {
                    outputs.insert("known".to_string(), json!(known));
                }
                if let Some(name) = result.name {
                    outputs.insert("name".to_string(), json!(name));
                }
                scalar_optional(&mut outputs, "idlh_ppm", result.idlh_ppm);
                scalar_optional(&mut outputs, "erpg2_ppm", result.erpg2_ppm);
                scalar_optional(&mut outputs, "aegl2_ppm", result.aegl2_ppm);
                quantity_optional(
                    &mut outputs,
                    "mass_concentration",
                    result.mass_concentration.map(|value| value.value),
                    "kg/m**3",
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn active_fire_protection_screening(inputs: &Value) -> Result<Outcome, Outcome> {
    let scenario = text(inputs, "scenario")?;
    let blowdown_time_to_target = seconds(number(inputs, "blowdown_time_to_target")?);
    let bare_steel_time_to_critical = seconds(number(inputs, "bare_steel_time_to_critical")?);
    let dedicated_object_demand = liters_per_minute(number(inputs, "dedicated_object_demand")?);
    let general_area_demand = liters_per_minute(number(inputs, "general_area_demand")?);
    Ok(
        match azoth_process::active_fire_protection_screening(
            scenario,
            blowdown_time_to_target,
            bare_steel_time_to_critical,
            dedicated_object_demand,
            general_area_demand,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                outputs.insert(
                    "water_extinguishing".to_string(),
                    json!(result.water_extinguishing),
                );
                outputs.insert(
                    "blowdown_faster_than_failure".to_string(),
                    json!(result.blowdown_faster_than_failure),
                );
                quantity(
                    &mut outputs,
                    "blowdown_margin",
                    result.blowdown_margin.value,
                    "s",
                );
                scalar(
                    &mut outputs,
                    "selective_protection_saving_fraction",
                    result.selective_protection_saving_fraction,
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn deluge_nozzle_layout(inputs: &Value) -> Result<Outcome, Outcome> {
    let protected_area = square_meters(number(inputs, "protected_area")?);
    let required_density = number(inputs, "required_density")?;
    let nozzle_k = number(inputs, "nozzle_k")?;
    let nozzle_min_pressure = bars(number(inputs, "nozzle_min_pressure")?);
    let nozzle_max_spacing = meters(number(inputs, "nozzle_max_spacing")?);
    let nozzle_clearance = meters(number(inputs, "nozzle_clearance")?);
    let available_clearance = meters(number(inputs, "available_clearance")?);
    let operating_pressure = optional_number(inputs, "operating_pressure")?.map(bars);
    let coverage_efficiency = optional_number(inputs, "coverage_efficiency")?;
    Ok(
        match azoth_process::deluge_nozzle_layout(
            protected_area,
            required_density,
            nozzle_k,
            nozzle_min_pressure,
            nozzle_max_spacing,
            nozzle_clearance,
            available_clearance,
            operating_pressure,
            coverage_efficiency,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                quantity(
                    &mut outputs,
                    "required_flow",
                    result.required_flow.value,
                    "L/min",
                );
                quantity(
                    &mut outputs,
                    "flow_per_nozzle",
                    result.flow_per_nozzle.value,
                    "L/min",
                );
                quantity(
                    &mut outputs,
                    "coverage_per_nozzle",
                    result.coverage_per_nozzle.value,
                    "m**2",
                );
                scalar(
                    &mut outputs,
                    "nozzle_count_from_flow",
                    result.nozzle_count_from_flow,
                );
                scalar(
                    &mut outputs,
                    "nozzle_count_from_coverage",
                    result.nozzle_count_from_coverage,
                );
                scalar(&mut outputs, "nozzle_count", result.nozzle_count);
                outputs.insert(
                    "governing_criterion".to_string(),
                    json!(result.governing_criterion.as_str()),
                );
                quantity(&mut outputs, "grid_spacing", result.grid_spacing.value, "m");
                quantity(
                    &mut outputs,
                    "delivered_flow",
                    result.delivered_flow.value,
                    "L/min",
                );
                scalar(&mut outputs, "delivered_density", result.delivered_density);
                outputs.insert(
                    "pressure_adequate".to_string(),
                    json!(result.pressure_adequate),
                );
                outputs.insert("density_met".to_string(), json!(result.density_met));
                outputs.insert(
                    "clearance_adequate".to_string(),
                    json!(result.clearance_adequate),
                );
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn fire_monitor_coverage(inputs: &Value) -> Result<Outcome, Outcome> {
    let target_area = square_meters(number(inputs, "target_area")?);
    let required_density = number(inputs, "required_density")?;
    let monitor_count = optional_number(inputs, "monitor_count")?;
    let monitor_flow = optional_number(inputs, "monitor_flow")?.map(liters_per_minute);
    let effective_throw = optional_number(inputs, "effective_throw")?.map(meters);
    let sweep_angle = optional_number(inputs, "sweep_angle")?;
    let wind_speed = optional_number(inputs, "wind_speed")?.map(meters_per_second);
    let fall_height = optional_number(inputs, "fall_height")?.map(meters);
    let droplet_terminal_velocity =
        optional_number(inputs, "droplet_terminal_velocity")?.map(meters_per_second);
    let characteristic_dimension = optional_number(inputs, "characteristic_dimension")?.map(meters);
    let line_of_sight_obstructed = optional_flag(inputs, "line_of_sight_obstructed")?;
    Ok(
        match azoth_process::fire_monitor_coverage(
            target_area,
            required_density,
            monitor_count,
            monitor_flow,
            effective_throw,
            sweep_angle,
            wind_speed,
            fall_height,
            droplet_terminal_velocity,
            characteristic_dimension,
            line_of_sight_obstructed,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                quantity(&mut outputs, "total_flow", result.total_flow.value, "L/min");
                quantity(&mut outputs, "swept_area", result.swept_area.value, "m**2");
                scalar(&mut outputs, "nominal_density", result.nominal_density);
                quantity(
                    &mut outputs,
                    "drift_displacement",
                    result.drift_displacement.value,
                    "m",
                );
                scalar(
                    &mut outputs,
                    "wind_coverage_fraction",
                    result.wind_coverage_fraction,
                );
                scalar(&mut outputs, "effective_density", result.effective_density);
                outputs.insert(
                    "meets_density_still_air".to_string(),
                    json!(result.meets_density_still_air),
                );
                outputs.insert(
                    "meets_density_in_wind".to_string(),
                    json!(result.meets_density_in_wind),
                );
                quantity_optional(
                    &mut outputs,
                    "flow_needed_for_compliance",
                    result.flow_needed_for_compliance.map(|value| value.value),
                    "L/min",
                );
                outputs.insert("verdict".to_string(), json!(result.verdict));
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn fire_water_coverage_assessment(inputs: &Value) -> Result<Outcome, Outcome> {
    let protected_area = square_meters(number(inputs, "protected_area")?);
    let required_flow = liters_per_minute(number(inputs, "required_flow")?);
    let spare_supply_flow = liters_per_minute(number(inputs, "spare_supply_flow")?);
    let worst_case_pressure_margin = bars(number(inputs, "worst_case_pressure_margin")?);
    let covered_area = optional_number(inputs, "covered_area")?.map(square_meters);
    let existing_delivered_flow =
        optional_number(inputs, "existing_delivered_flow")?.map(liters_per_minute);
    let required_pressure_margin = optional_number(inputs, "required_pressure_margin")?.map(bars);
    Ok(
        match azoth_process::fire_water_coverage_assessment(
            protected_area,
            required_flow,
            spare_supply_flow,
            worst_case_pressure_margin,
            covered_area,
            existing_delivered_flow,
            required_pressure_margin,
        ) {
            Ok(result) => {
                let mut outputs = BTreeMap::new();
                quantity(
                    &mut outputs,
                    "coverage_gap",
                    result.coverage_gap.value,
                    "m**2",
                );
                scalar(&mut outputs, "coverage_fraction", result.coverage_fraction);
                quantity(
                    &mut outputs,
                    "flow_deficit",
                    result.flow_deficit.value,
                    "L/min",
                );
                outputs.insert("flow_feasible".to_string(), json!(result.flow_feasible));
                outputs.insert(
                    "pressure_feasible".to_string(),
                    json!(result.pressure_feasible),
                );
                outputs.insert("verdict".to_string(), json!(result.verdict.as_str()));
                done(outputs, &result.warnings)
            }
            Err(error) => refused(error),
        },
    )
}

fn done(outputs: BTreeMap<String, Value>, list: &[Warning]) -> Outcome {
    Outcome::Ok(CalcOutcome {
        outputs,
        warnings: warnings(list),
    })
}

fn quantity(outputs: &mut BTreeMap<String, Value>, name: &str, value: f64, unit: &str) {
    outputs.insert(
        name.to_string(),
        json!({"magnitude_si": value, "unit": unit}),
    );
}

fn quantity_optional(
    outputs: &mut BTreeMap<String, Value>,
    name: &str,
    value: Option<f64>,
    unit: &str,
) {
    if let Some(value) = value {
        quantity(outputs, name, value, unit);
    }
}

fn scalar(outputs: &mut BTreeMap<String, Value>, name: &str, value: f64) {
    outputs.insert(name.to_string(), json!(value));
}

fn scalar_optional(outputs: &mut BTreeMap<String, Value>, name: &str, value: Option<f64>) {
    if let Some(value) = value {
        scalar(outputs, name, value);
    }
}

fn records<'a>(inputs: &'a Value, name: &str) -> Result<&'a Vec<Value>, Outcome> {
    let Some(value) = inputs.get(name) else {
        return Err(Outcome::Error {
            message: format!("`{name}` is required (an array of records)"),
            field: Some(name.to_string()),
        });
    };
    value.as_array().ok_or_else(|| Outcome::Error {
        message: format!("`{name}` takes an array of records"),
        field: Some(name.to_string()),
    })
}

fn expect_object<'a>(value: &'a Value, what: &str) -> Result<&'a Map<String, Value>, Outcome> {
    value.as_object().ok_or_else(|| Outcome::Error {
        message: format!("`{what}` takes a record"),
        field: Some(what.to_string()),
    })
}

fn field_number(object: &Map<String, Value>, what: &str, name: &str) -> Result<f64, Outcome> {
    let Some(value) = object.get(name) else {
        return Err(Outcome::Error {
            message: format!("`{what}` requires `{name}`"),
            field: Some(format!("{what}.{name}")),
        });
    };
    match value.as_f64() {
        Some(number) if number.is_finite() => Ok(number),
        _ => Err(Outcome::Error {
            message: format!("`{what}.{name}` takes a finite number"),
            field: Some(format!("{what}.{name}")),
        }),
    }
}

fn field_text<'a>(
    object: &'a Map<String, Value>,
    what: &str,
    name: &str,
) -> Result<&'a str, Outcome> {
    let Some(value) = object.get(name) else {
        return Err(Outcome::Error {
            message: format!("`{what}` requires `{name}`"),
            field: Some(format!("{what}.{name}")),
        });
    };
    value.as_str().ok_or_else(|| Outcome::Error {
        message: format!("`{what}.{name}` takes a name"),
        field: Some(format!("{what}.{name}")),
    })
}

fn optional_field_flag(
    object: &Map<String, Value>,
    what: &str,
    name: &str,
) -> Result<Option<bool>, Outcome> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_bool().map(Some).ok_or_else(|| Outcome::Error {
            message: format!("`{what}.{name}` takes true or false"),
            field: Some(format!("{what}.{name}")),
        }),
    }
}
