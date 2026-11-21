//! `process.topsides_facility_cost` - the topsides facility stack.
//!
//! Spec: `specs/models/process/topsides_facility_cost.toml`. The arithmetic is
//! `TopsidesFacilityCostEstimator` at the pin over the process cost rows: the
//! module classification, the bulk and integration factors, the project stack
//! and the material-take-off.

use azoth_core::units::Mass;
use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::costing::topsides::{self as facility, FacilityType, ProjectContext, TopsidesFactors};
use crate::executor::json::warnings as wire_warnings;

/// One process unit, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct TopsidesEquipment {
    /// The unit's name.
    pub name: String,
    /// The process type the roll-up classified.
    pub equipment_type: String,
    /// The unit's total module cost.
    pub total_module_cost: f64,
    /// The unit's dry weight.
    pub weight: Mass,
}

/// One capital-cost line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopsidesCapitalLine {
    /// The line's name.
    pub name: String,
    /// The line's cost.
    pub cost: f64,
}

/// One summary line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopsidesSummaryLine {
    /// The line's name.
    pub name: String,
    /// The line's cost.
    pub cost: f64,
}

/// One module breakdown line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopsidesBreakdownLine {
    /// The module key.
    pub name: String,
    /// The module's cost.
    pub cost: f64,
}

/// One project cost line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopsidesProjectLine {
    /// The line's name.
    pub name: String,
    /// The line's cost.
    pub cost: f64,
}

/// One project summary line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopsidesProjectSummaryLine {
    /// The line's name.
    pub name: String,
    /// The line's cost.
    pub cost: f64,
}

/// One weight line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopsidesWeightLine {
    /// The line's name.
    pub name: String,
    /// The line's weight.
    pub weight_kg: f64,
}

/// One material-take-off row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopsidesMtoRow {
    /// The line's item.
    pub item: String,
    /// The line's category.
    pub category: String,
    /// The line's quantity.
    pub quantity: f64,
    /// The quantity's unit.
    pub unit: String,
    /// The line's weight, zero where absent.
    pub weight_kg: f64,
    /// Whether the line carries a weight.
    pub has_weight: bool,
    /// The line's cost.
    pub cost_usd: f64,
    /// The line's source.
    pub source: String,
}

/// One quality flag.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopsidesFlag {
    /// The flag's message.
    pub message: String,
}

/// Result of `process.topsides_facility_cost`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TopsidesFacilityCostResult {
    /// The ten capital-cost lines.
    pub capital_costs: Vec<TopsidesCapitalLine>,
    /// The two summary lines.
    pub capital_cost_summary: Vec<TopsidesSummaryLine>,
    /// The module breakdown.
    pub capital_cost_breakdown: Vec<TopsidesBreakdownLine>,
    /// The four project lines.
    pub project_costs: Vec<TopsidesProjectLine>,
    /// The total topsides capex.
    pub project_cost_summary: Vec<TopsidesProjectSummaryLine>,
    /// The four weight lines.
    pub weight_basis: Vec<TopsidesWeightLine>,
    /// The material-take-off rows.
    pub material_take_off: Vec<TopsidesMtoRow>,
    /// The quality flags.
    pub quality_flags: Vec<TopsidesFlag>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn finite(name: &str, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

fn number(name: &str, value: Option<f64>, default: f64) -> Result<f64> {
    finite(name, value.unwrap_or(default))
}

fn category_of(item: &str, source: Option<&str>) -> String {
    if source == Some("topsides-allowance") {
        if item.contains("Structural") {
            return "structural".to_string();
        }
        if item.contains("Piping") {
            return "piping".to_string();
        }
        if item.contains("Electrical") {
            return "electrical".to_string();
        }
        if item.contains("HVAC") {
            return "safety-systems".to_string();
        }
    }
    "equipment".to_string()
}

/// The topsides estimate over the stated process rows.
#[allow(clippy::too_many_arguments)]
pub fn topsides_facility_cost(
    equipment: &[TopsidesEquipment],
    facility_type: Option<String>,
    project_context: Option<String>,
    location_factor: Option<f64>,
    piping_bulk_factor: Option<f64>,
    electrical_instrumentation_factor: Option<f64>,
    insulation_paint_fireproofing_factor: Option<f64>,
    hvac_telecom_safety_factor: Option<f64>,
    engineering_procurement_factor: Option<f64>,
    construction_management_factor: Option<f64>,
    project_contingency_factor: Option<f64>,
    owner_cost_factor: Option<f64>,
    include_utilities_and_offsites: Option<bool>,
    estimate_class: Option<f64>,
) -> Result<TopsidesFacilityCostResult> {
    let facility_name = facility_type.unwrap_or_else(|| "fixed_platform".to_string());
    let facility = FacilityType::parse(&facility_name)
        .ok_or_else(|| AzothError::invalid_input("facility_type", "an unknown facility type"))?;
    let context_name = project_context.unwrap_or_else(|| "greenfield".to_string());
    let context = ProjectContext::parse(&context_name).ok_or_else(|| {
        AzothError::invalid_input("project_context", "an unknown project context")
    })?;
    let class = number("estimate_class", estimate_class, 4.0)?;
    if !(1.0..=5.0).contains(&class) || class.fract() != 0.0 {
        return Err(AzothError::invalid_input(
            "estimate_class",
            "an AACE class is a whole number from 1 to 5",
        ));
    }

    let mut rows = Vec::with_capacity(equipment.len());
    for (index, row) in equipment.iter().enumerate() {
        rows.push(facility::TopsidesEquipmentRow {
            name: row.name.clone(),
            equipment_type: row.equipment_type.clone(),
            total_module_cost: finite(
                &format!("equipment[{index}].total_module_cost"),
                row.total_module_cost,
            )?,
            weight: finite(&format!("equipment[{index}].weight"), row.weight.value)?,
        });
    }
    let input = facility::TopsidesInput {
        equipment: rows,
        facility_type: facility,
        project_context: context,
        location_factor: number("location_factor", location_factor, 1.0)?,
        factors: TopsidesFactors {
            piping_bulk_factor: number("piping_bulk_factor", piping_bulk_factor, 0.25)?,
            electrical_instrumentation_factor: number(
                "electrical_instrumentation_factor",
                electrical_instrumentation_factor,
                0.16,
            )?,
            insulation_paint_fireproofing_factor: number(
                "insulation_paint_fireproofing_factor",
                insulation_paint_fireproofing_factor,
                0.06,
            )?,
            hvac_telecom_safety_factor: number(
                "hvac_telecom_safety_factor",
                hvac_telecom_safety_factor,
                0.08,
            )?,
            engineering_procurement_factor: number(
                "engineering_procurement_factor",
                engineering_procurement_factor,
                0.14,
            )?,
            construction_management_factor: number(
                "construction_management_factor",
                construction_management_factor,
                0.06,
            )?,
            project_contingency_factor: number(
                "project_contingency_factor",
                project_contingency_factor,
                0.15,
            )?,
            owner_cost_factor: number("owner_cost_factor", owner_cost_factor, 0.10)?,
            include_utilities_and_offsites: include_utilities_and_offsites.unwrap_or(true),
        },
    };
    let result = facility::estimate(&input);

    let mut flags: Vec<TopsidesFlag> = result
        .quality_flags
        .iter()
        .map(|message| TopsidesFlag {
            message: message.clone(),
        })
        .collect();
    if class > 3.0 {
        flags.push(TopsidesFlag {
            message: "Estimate is not a detailed Class 1-3 vendor or firm-quantity estimate."
                .to_string(),
        });
    }

    Ok(TopsidesFacilityCostResult {
        capital_costs: result
            .capital_costs
            .iter()
            .map(|(name, cost)| TopsidesCapitalLine {
                name: name.clone(),
                cost: *cost,
            })
            .collect(),
        capital_cost_summary: result
            .capital_cost_summary
            .iter()
            .map(|(name, cost)| TopsidesSummaryLine {
                name: name.clone(),
                cost: *cost,
            })
            .collect(),
        capital_cost_breakdown: result
            .capital_cost_breakdown
            .iter()
            .map(|(name, cost)| TopsidesBreakdownLine {
                name: name.clone(),
                cost: *cost,
            })
            .collect(),
        project_costs: result
            .project_costs
            .iter()
            .map(|(name, cost)| TopsidesProjectLine {
                name: name.clone(),
                cost: *cost,
            })
            .collect(),
        project_cost_summary: result
            .project_cost_summary
            .iter()
            .map(|(name, cost)| TopsidesProjectSummaryLine {
                name: name.clone(),
                cost: *cost,
            })
            .collect(),
        weight_basis: result
            .weight_basis
            .iter()
            .map(|(name, weight)| TopsidesWeightLine {
                name: name.clone(),
                weight_kg: *weight,
            })
            .collect(),
        material_take_off: result
            .material_take_off
            .iter()
            .map(|row| TopsidesMtoRow {
                item: row.item.clone(),
                category: category_of(&row.item, row.source.as_deref()),
                quantity: row.quantity.unwrap_or(0.0),
                unit: row.unit.clone().unwrap_or_default(),
                weight_kg: row.weight_kg.unwrap_or(0.0),
                has_weight: row.weight_kg.is_some(),
                cost_usd: row.cost_usd,
                source: row.source.clone().unwrap_or_default(),
            })
            .collect(),
        quality_flags: flags,
        warnings: Vec::new(),
    })
}
