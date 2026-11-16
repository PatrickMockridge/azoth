//! The safety ids this workspace can run, called as the typed kernels they are.
//!
//! **Not `azoth_sheets::dispatch`.** That dispatcher is generated from `specs/calcs`, and the
//! safety ids are models under `specs/models/process/`; no generated arm reaches them. This is
//! the small hand-written table that does, in the calculation book's own outcome shape.

use std::collections::BTreeMap;

use azoth_core::Warning;
use azoth_core::units::{
    kelvins, kilograms, kilograms_per_second, kilograms_per_square_meter_second, meters, pascals,
    seconds, watts_per_meter_kelvin, watts_per_square_meter,
};
use azoth_process::models::bow_tie::{BowTieBarrier, BowTieConsequence, BowTieThreat};
pub use azoth_sheets::dispatch::Outcome;
use azoth_sheets::dispatch_support::{number, optional_number, optional_text, refused, warnings};
use azoth_sheets::workspace::CalcOutcome;
use serde_json::{Map, Value, json};

/// Every id this workspace can run natively.
pub const NATIVE: &[&str] = &[
    "process.bleve",
    "process.bow_tie",
    "process.jet_fire",
    "process.pfp_demand",
    "process.pool_fire",
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
        "process.bleve" => &["fuel_mass", "heat_of_combustion", "radiative_fraction"],
        "process.bow_tie" => &["threats", "consequences", "barriers"],
        "process.jet_fire" => &["mass_flow", "heat_of_combustion", "radiative_fraction"],
        "process.pfp_demand" => &[
            "incident_heat_flux",
            "steel_thickness",
            "required_survival_time",
        ],
        "process.pool_fire" => &["pool_diameter", "burn_rate"],
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
        "process.bow_tie" => bow_tie(inputs),
        "process.jet_fire" => jet_fire(inputs),
        "process.pool_fire" => pool_fire(inputs),
        "process.bleve" => bleve(inputs),
        "process.vce" => vce(inputs),
        "process.pfp_demand" => pfp_demand(inputs),
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
