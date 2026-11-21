//! `process.cost_estimation` - the cost-estimation calculator as a registered id.
//!
//! Spec: `specs/models/process/cost_estimation.toml`. The arithmetic is
//! `process/costestimation/CostEstimationCalculator.java` at the pin: the factor
//! tables, the Turton correlation dispatch, the module stack, the labour rates
//! and the vessel bill of materials.

use azoth_core::units::{Length, Mass, Pressure};
use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::costing::correlations;
use crate::costing::factors;
use crate::costing::module_stack;
use crate::costing::records::MtoItem;
use crate::executor::json::warnings as wire_warnings;

/// One calculator call, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct CostEstimationQuery {
    /// The calculator call.
    pub calculation: String,
    /// The correlation, where the call is a correlation.
    pub correlation: Option<String>,
    /// The correlation's capacity.
    pub capacity: Option<f64>,
    /// The tray or piping diameter.
    pub diameter: Option<Length>,
    /// The tray count.
    pub tray_count: Option<f64>,
    /// The packing material.
    pub packing_type: Option<String>,
    /// The piping length.
    pub piping_length: Option<Length>,
    /// The piping schedule.
    pub piping_schedule: Option<String>,
    /// The control valve's flow coefficient.
    pub valve_cv: Option<f64>,
    /// The purchased equipment cost.
    pub purchased_cost: Option<f64>,
    /// The pressure the step table reads.
    pub design_pressure: Option<Pressure>,
    /// A stated pressure factor.
    pub pressure_factor: Option<f64>,
    /// A stated material factor.
    pub material_factor: Option<f64>,
    /// The equipment weight.
    pub weight: Option<Mass>,
    /// The labour vocabulary's equipment type.
    pub equipment_type: Option<String>,
    /// The material of construction.
    pub material: Option<String>,
    /// The location region.
    pub region: Option<String>,
    /// The currency.
    pub currency: Option<String>,
    /// A stated exchange rate.
    pub exchange_rate: Option<f64>,
    /// The amount the conversion converts.
    pub amount: Option<f64>,
    /// The conversion direction.
    pub currency_direction: Option<String>,
    /// The vessel shell weight.
    pub shell_weight: Option<Mass>,
    /// The heads' weight.
    pub heads_weight: Option<Mass>,
    /// The nozzle count.
    pub nozzle_count: Option<f64>,
    /// The internals weight.
    pub internals_weight: Option<Mass>,
    /// The current cost index.
    pub current_cepci: Option<f64>,
    /// The reference cost index.
    pub reference_cepci: Option<f64>,
    /// The contingency factor.
    pub contingency_factor: Option<f64>,
    /// The engineering factor.
    pub engineering_factor: Option<f64>,
}

/// One calculator answer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CostEstimationResultRow {
    /// The call this row answered.
    pub calculation: String,
    /// The correlation, or the empty string.
    pub correlation: String,
    /// The correlation's USD answer, or the summary route's located purchased cost.
    pub purchased_equipment_cost: f64,
    /// The step table's factor or the stated override.
    pub pressure_factor: f64,
    /// The material factor the row used.
    pub material_factor: f64,
    /// The location factor the row used.
    pub location_factor: f64,
    /// The exchange rate the row used.
    pub exchange_rate: f64,
    /// The bare module cost.
    pub bare_module_cost: f64,
    /// The total module cost.
    pub total_module_cost: f64,
    /// The grass roots cost.
    pub grass_roots_cost: f64,
    /// The installation labour hours.
    pub installation_man_hours: f64,
    /// The currency conversion's answer.
    pub converted_amount: f64,
    /// The converted amount in USD.
    pub cost_in_usd: f64,
    /// The current cost index.
    pub current_cepci: f64,
    /// The reference cost index.
    pub reference_cepci: f64,
    /// Current over reference.
    pub cepci_ratio: f64,
    /// The contingency factor.
    pub contingency_factor: f64,
    /// The engineering factor.
    pub engineering_factor: f64,
    /// The class's stored Lang factor.
    pub installation_factor: f64,
}

/// One bill-of-materials line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CostBomRow {
    /// The line's item.
    pub item: String,
    /// The material, or the empty string.
    pub material: String,
    /// The count, where the line is a count.
    pub quantity: f64,
    /// Whether the line carries a count.
    pub has_quantity: bool,
    /// The count's unit, or the empty string.
    pub unit: String,
    /// The line's weight.
    pub weight_kg: f64,
    /// Whether the line carries a weight.
    pub has_weight: bool,
    /// The line's cost.
    pub cost_usd: f64,
    /// The line's source, or the empty string.
    pub source: String,
}

/// Result of `process.cost_estimation`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CostEstimationResult {
    /// One row per query.
    pub queries: Vec<CostEstimationResultRow>,
    /// The vessel bill of materials.
    pub bom: Vec<CostBomRow>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn finite(name: String, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

fn require<T>(index: usize, field: &'static str, value: Option<T>, why: &str) -> Result<T> {
    value.ok_or_else(|| {
        AzothError::invalid_input(
            format!("queries[{index}].{field}"),
            format!("a {why} row requires `{field}`"),
        )
    })
}

fn whole(index: usize, field: &'static str, value: f64) -> Result<f64> {
    let value = finite(format!("queries[{index}].{field}"), value)?;
    if value.fract() != 0.0 {
        return Err(AzothError::invalid_input(
            format!("queries[{index}].{field}"),
            "a count is a whole number",
        ));
    }
    Ok(value)
}

fn material_name(value: &str) -> Option<&'static str> {
    Some(match value {
        "carbon_steel" => "Carbon Steel",
        "ss304" => "SS304",
        "ss316" => "SS316",
        "ss316l" => "SS316L",
        "monel" => "Monel",
        "hastelloy_c" => "Hastelloy C",
        "inconel" => "Inconel",
        "titanium" => "Titanium",
        "nickel" => "Nickel",
        _ => return None,
    })
}

fn region_name(value: &str) -> Option<&'static str> {
    Some(match value {
        "us_gulf_coast" => "US Gulf Coast",
        "us_midwest" => "US Midwest",
        "us_west_coast" => "US West Coast",
        "western_europe" => "Western Europe",
        "north_sea" => "North Sea / Norway",
        "middle_east" => "Middle East",
        "southeast_asia" => "Southeast Asia",
        "china" => "China",
        "australia" => "Australia",
        "brazil" => "Brazil",
        "west_africa" => "West Africa",
        _ => return None,
    })
}

fn bom_rows(rows: &[MtoItem]) -> Vec<CostBomRow> {
    rows.iter()
        .map(|row| CostBomRow {
            item: row.item.clone(),
            material: row.material.clone().unwrap_or_default(),
            quantity: row.quantity.unwrap_or(0.0),
            has_quantity: row.quantity.is_some(),
            unit: row.unit.clone().unwrap_or_default(),
            weight_kg: row.weight_kg.unwrap_or(0.0),
            has_weight: row.weight_kg.is_some(),
            cost_usd: row.cost_usd,
            source: row.source.clone().unwrap_or_default(),
        })
        .collect()
}

const CALCULATIONS: [&str; 10] = [
    "correlation",
    "module_stack",
    "summary",
    "installation_man_hours",
    "piping_man_hours",
    "currency_conversion",
    "pressure_factor",
    "material_factor",
    "location_factor",
    "vessel_bom",
];

const CORRELATIONS: [&str; 16] = [
    "vertical_vessel_by_volume",
    "horizontal_vessel_by_volume",
    "vertical_vessel_by_weight",
    "horizontal_vessel_by_weight",
    "shell_tube",
    "plate",
    "air_cooler",
    "sieve_trays",
    "valve_trays",
    "bubble_cap_trays",
    "packing",
    "centrifugal_pump",
    "centrifugal_compressor",
    "reciprocating_compressor",
    "piping",
    "control_valve",
];

/// The calculator over the stated queries.
pub fn cost_estimation(queries: &[CostEstimationQuery]) -> Result<CostEstimationResult> {
    let mut rows = Vec::with_capacity(queries.len());
    let mut bom: Vec<CostBomRow> = Vec::new();

    for (index, query) in queries.iter().enumerate() {
        if !CALCULATIONS.contains(&query.calculation.as_str()) {
            return Err(AzothError::invalid_input(
                format!("queries[{index}].calculation"),
                format!(
                    "{:?} is not one of the calculator's calls",
                    query.calculation
                ),
            ));
        }
        let current_cepci = finite(
            format!("queries[{index}].current_cepci"),
            query.current_cepci.unwrap_or(840.0),
        )?;
        let reference_cepci = finite(
            format!("queries[{index}].reference_cepci"),
            query.reference_cepci.unwrap_or(607.5),
        )?;
        if current_cepci <= 0.0 || reference_cepci <= 0.0 {
            return Err(AzothError::invalid_input(
                format!("queries[{index}].current_cepci"),
                "a cost index is positive",
            ));
        }
        let ratio = current_cepci / reference_cepci;
        let material_factor = match (&query.material, query.material_factor) {
            (_, Some(value)) => finite(format!("queries[{index}].material_factor"), value)?,
            (Some(name), None) => {
                let canonical = material_name(name).ok_or_else(|| {
                    AzothError::invalid_input(
                        format!("queries[{index}].material"),
                        format!("{name:?} is not a material with a factor"),
                    )
                })?;
                factors::material_factor(canonical).unwrap_or(1.0)
            }
            (None, None) => 1.0,
        };
        let location_factor = match &query.region {
            Some(region) => {
                let name = region_name(region).ok_or_else(|| {
                    AzothError::invalid_input(
                        format!("queries[{index}].region"),
                        format!("{region:?} is not a location region"),
                    )
                })?;
                factors::location_factor(name)
            }
            None => 1.0,
        };
        let exchange_rate = match query.exchange_rate {
            Some(rate) => finite(format!("queries[{index}].exchange_rate"), rate)?,
            None => match &query.currency {
                Some(currency) => factors::exchange_rate(currency).ok_or_else(|| {
                    AzothError::invalid_input(
                        format!("queries[{index}].currency"),
                        format!("{currency:?} is not a currency with a default rate"),
                    )
                })?,
                None => 1.0,
            },
        };
        if exchange_rate <= 0.0 {
            return Err(AzothError::invalid_input(
                format!("queries[{index}].exchange_rate"),
                "an exchange rate is positive",
            ));
        }
        let contingency = finite(
            format!("queries[{index}].contingency_factor"),
            query
                .contingency_factor
                .unwrap_or(factors::CONTINGENCY_DEFAULT),
        )?;
        let engineering = finite(
            format!("queries[{index}].engineering_factor"),
            query
                .engineering_factor
                .unwrap_or(factors::ENGINEERING_DEFAULT),
        )?;
        let design_pressure = query
            .design_pressure
            .map(|pressure| pressure.value / 1.0e5)
            .map(|value| finite(format!("queries[{index}].design_pressure"), value))
            .transpose()?;
        let stated_pressure_factor = query
            .pressure_factor
            .map(|value| finite(format!("queries[{index}].pressure_factor"), value))
            .transpose()?;
        let pressure_factor = stated_pressure_factor
            .unwrap_or_else(|| factors::pressure_factor(design_pressure.unwrap_or(0.0)));

        let mut row = CostEstimationResultRow {
            calculation: query.calculation.clone(),
            correlation: query.correlation.clone().unwrap_or_default(),
            purchased_equipment_cost: 0.0,
            pressure_factor,
            material_factor,
            location_factor,
            exchange_rate,
            bare_module_cost: 0.0,
            total_module_cost: 0.0,
            grass_roots_cost: 0.0,
            installation_man_hours: 0.0,
            converted_amount: 0.0,
            cost_in_usd: 0.0,
            current_cepci,
            reference_cepci,
            cepci_ratio: ratio,
            contingency_factor: contingency,
            engineering_factor: engineering,
            installation_factor: factors::INSTALLATION_FACTOR_STORED,
        };

        match query.calculation.as_str() {
            "correlation" => {
                row.purchased_equipment_cost =
                    correlation_answer(index, query, material_factor, ratio)?;
            }
            "module_stack" => {
                let purchased = require(
                    index,
                    "purchased_cost",
                    query.purchased_cost,
                    "module_stack",
                )?;
                let purchased = finite(format!("queries[{index}].purchased_cost"), purchased)?;
                if purchased < 0.0 {
                    return Err(AzothError::invalid_input(
                        format!("queries[{index}].purchased_cost"),
                        "a purchased cost is at least zero",
                    ));
                }
                row.purchased_equipment_cost = purchased;
                row.bare_module_cost =
                    module_stack::bare_module(purchased, pressure_factor, material_factor);
                row.total_module_cost =
                    module_stack::total_module(row.bare_module_cost, contingency, engineering);
                row.grass_roots_cost = module_stack::grass_roots(row.total_module_cost);
            }
            "summary" => {
                let purchased = finite(
                    format!("queries[{index}].purchased_cost"),
                    require(index, "purchased_cost", query.purchased_cost, "summary")?,
                )?;
                let weight = require(index, "weight", query.weight, "summary")?;
                let weight = finite(format!("queries[{index}].weight"), weight.value)?;
                let equipment_type = query
                    .equipment_type
                    .clone()
                    .unwrap_or_else(|| "vessel".to_string());
                let stack = module_stack::calculator_summary(
                    purchased,
                    &module_stack::SummaryInput {
                        design_pressure_barg: design_pressure.unwrap_or(0.0),
                        weight_kg: weight,
                        equipment_type: &equipment_type,
                        material_factor,
                        location_factor,
                        contingency,
                        engineering,
                    },
                );
                row.purchased_equipment_cost = stack.purchased_equipment_cost;
                row.bare_module_cost = stack.bare_module_cost;
                row.total_module_cost = stack.total_module_cost;
                row.grass_roots_cost = stack.grass_roots_cost;
                row.installation_man_hours = stack.installation_man_hours * 3600.0;
            }
            "installation_man_hours" => {
                let weight = finite(
                    format!("queries[{index}].weight"),
                    require(index, "weight", query.weight, "installation_man_hours")?.value,
                )?;
                let equipment_type = query
                    .equipment_type
                    .clone()
                    .unwrap_or_else(|| "vessel".to_string());
                row.installation_man_hours =
                    correlations::installation_man_hours(weight, &equipment_type) * 3600.0;
            }
            "piping_man_hours" => {
                let weight = finite(
                    format!("queries[{index}].weight"),
                    require(index, "weight", query.weight, "piping_man_hours")?.value,
                )?;
                row.installation_man_hours = correlations::piping_man_hours(weight) * 3600.0;
            }
            "currency_conversion" => {
                let amount = finite(
                    format!("queries[{index}].amount"),
                    require(index, "amount", query.amount, "currency_conversion")?,
                )?;
                let direction = query.currency_direction.as_deref().unwrap_or("from_usd");
                match direction {
                    "from_usd" => {
                        row.converted_amount = amount * exchange_rate;
                        row.cost_in_usd = amount;
                    }
                    "to_usd" => {
                        row.converted_amount = amount / exchange_rate;
                        row.cost_in_usd = row.converted_amount;
                    }
                    other => {
                        return Err(AzothError::invalid_input(
                            format!("queries[{index}].currency_direction"),
                            format!("{other:?} is not `from_usd` or `to_usd`"),
                        ));
                    }
                }
            }
            "pressure_factor" | "material_factor" | "location_factor" => {}
            "vessel_bom" => {
                let nozzle_count = whole(
                    index,
                    "nozzle_count",
                    require(index, "nozzle_count", query.nozzle_count, "vessel_bom")?,
                )?;
                let shell = require(index, "shell_weight", query.shell_weight, "vessel_bom")?;
                let heads = require(index, "heads_weight", query.heads_weight, "vessel_bom")?;
                let internals = require(
                    index,
                    "internals_weight",
                    query.internals_weight,
                    "vessel_bom",
                )?;
                let shell = finite(format!("queries[{index}].shell_weight"), shell.value)?;
                let heads = finite(format!("queries[{index}].heads_weight"), heads.value)?;
                let internals = finite(
                    format!("queries[{index}].internals_weight"),
                    internals.value,
                )?;
                let canonical = query
                    .material
                    .as_deref()
                    .and_then(material_name)
                    .unwrap_or("Carbon Steel");
                let rows = correlations::vessel_bom(
                    shell,
                    heads,
                    nozzle_count,
                    internals,
                    canonical,
                    material_factor,
                    ratio,
                );
                bom.extend(bom_rows(&rows));
            }
            _ => {
                return Err(AzothError::invalid_input(
                    format!("queries[{index}].calculation"),
                    "the calculation is not one this kernel dispatches",
                ));
            }
        }

        rows.push(row);
    }

    Ok(CostEstimationResult {
        queries: rows,
        bom,
        warnings: Vec::new(),
    })
}

fn correlation_answer(
    index: usize,
    query: &CostEstimationQuery,
    material_factor: f64,
    ratio: f64,
) -> Result<f64> {
    let correlation = require(
        index,
        "correlation",
        query.correlation.clone(),
        "correlation",
    )?;
    if !CORRELATIONS.contains(&correlation.as_str()) {
        return Err(AzothError::invalid_input(
            format!("queries[{index}].correlation"),
            format!("{correlation:?} is not one of the calculator's correlations"),
        ));
    }
    let capacity = || -> Result<f64> {
        let value = require(index, "capacity", query.capacity, "correlation")?;
        finite(format!("queries[{index}].capacity"), value)
    };
    match correlation.as_str() {
        "vertical_vessel_by_volume" => {
            Ok(correlations::vertical_vessel_by_volume(capacity()?, ratio))
        }
        "horizontal_vessel_by_volume" => Ok(correlations::horizontal_vessel_by_volume(
            capacity()?,
            ratio,
        )),
        "vertical_vessel_by_weight" => {
            Ok(correlations::vertical_vessel_by_weight(capacity()?, ratio))
        }
        "horizontal_vessel_by_weight" => Ok(correlations::horizontal_vessel_by_weight(
            capacity()?,
            ratio,
        )),
        "shell_tube" => Ok(correlations::shell_tube_heat_exchanger(capacity()?, ratio)),
        "plate" => Ok(correlations::plate_heat_exchanger(capacity()?, ratio)),
        "air_cooler" => Ok(correlations::air_cooler(capacity()?, ratio)),
        "sieve_trays" | "valve_trays" | "bubble_cap_trays" => {
            let diameter = finite(
                format!("queries[{index}].diameter"),
                require(index, "diameter", query.diameter, "correlation")?.value,
            )?;
            let count = whole(
                index,
                "tray_count",
                require(index, "tray_count", query.tray_count, "correlation")?,
            )?;
            Ok(match correlation.as_str() {
                "valve_trays" => correlations::valve_trays(diameter, count, ratio),
                "bubble_cap_trays" => correlations::bubble_cap_trays(diameter, count, ratio),
                _ => correlations::sieve_trays(diameter, count, ratio),
            })
        }
        "packing" => {
            let packing_type = query.packing_type.as_deref().unwrap_or("plastic");
            Ok(correlations::packing(capacity()?, packing_type, ratio))
        }
        "centrifugal_pump" => Ok(correlations::centrifugal_pump(capacity()?, ratio)),
        "centrifugal_compressor" => Ok(correlations::centrifugal_compressor(capacity()?, ratio)),
        "reciprocating_compressor" => {
            Ok(correlations::reciprocating_compressor(capacity()?, ratio))
        }
        "piping" => {
            let diameter = finite(
                format!("queries[{index}].diameter"),
                require(index, "diameter", query.diameter, "correlation")?.value,
            )?;
            let length = finite(
                format!("queries[{index}].piping_length"),
                require(index, "piping_length", query.piping_length, "correlation")?.value,
            )?;
            let schedule = query.piping_schedule.as_deref().unwrap_or("40");
            let schedule = match schedule {
                "40" => 40,
                "80" => 80,
                "160" => 160,
                other => {
                    return Err(AzothError::invalid_input(
                        format!("queries[{index}].piping_schedule"),
                        format!("{other:?} is not one of the piping schedules"),
                    ));
                }
            };
            Ok(correlations::piping_cost(
                diameter,
                length,
                schedule,
                material_factor,
                ratio,
            ))
        }
        "control_valve" => {
            let cv = finite(
                format!("queries[{index}].valve_cv"),
                require(index, "valve_cv", query.valve_cv, "correlation")?,
            )?;
            Ok(correlations::control_valve(cv, material_factor, ratio))
        }
        _ => Err(AzothError::invalid_input(
            format!("queries[{index}].correlation"),
            "the correlation is not one this kernel dispatches",
        )),
    }
}
