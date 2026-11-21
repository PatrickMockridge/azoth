//! `process.process_cost_estimate` - the process-level cost roll-up.
//!
//! Spec: `specs/models/process/process_cost_estimate.toml`. The arithmetic is
//! `ProcessCostEstimate` at the pin, over the per-unit summary rows its live
//! walk produces: the totals, the type and discipline splits, the project stack,
//! the operating cost and the simple economics.

use azoth_core::units::{Mass, Power, Time};
use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::costing::equipment::OpexInputs;
use crate::costing::process::{self as rollup, EquipmentRow, ProcessCostInput};
use crate::executor::json::warnings as wire_warnings;

/// One unit's summary, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessCostEquipment {
    /// The unit's name.
    pub name: String,
    /// The Java class simple name.
    pub equipment_class: String,
    /// The purchased cost.
    pub purchased_equipment_cost: f64,
    /// The bare module cost.
    pub bare_module_cost: f64,
    /// The total module cost.
    pub total_module_cost: f64,
    /// The grass roots cost.
    pub grass_roots_cost: f64,
    /// The unit's total cost.
    pub total_cost: f64,
    /// The installation labour.
    pub installation_man_hours: Time,
    /// The unit's weight.
    pub weight: Mass,
    /// The unit's rotating power.
    pub power: Option<Power>,
    /// The unit's thermal duty.
    pub duty: Option<Power>,
}

/// One summary row of the result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProcessCostEquipmentResult {
    /// The unit's name.
    pub name: String,
    /// The Java class simple name.
    pub equipment_class: String,
    /// The type category.
    pub equipment_type: String,
    /// The purchased cost.
    pub purchased_equipment_cost: f64,
    /// The bare module cost.
    pub bare_module_cost: f64,
    /// The total module cost.
    pub total_module_cost: f64,
    /// The grass roots cost.
    pub grass_roots_cost: f64,
    /// The unit's total cost.
    pub total_cost: f64,
    /// The installation labour, in seconds.
    pub installation_man_hours: f64,
    /// The unit's weight.
    pub weight: f64,
    /// The located purchased cost.
    pub located_purchased_equipment_cost: f64,
    /// The location factor.
    pub location_factor: f64,
}

/// One equipment-type cost line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypeCostLine {
    /// The type category.
    pub name: String,
    /// Its located purchased cost.
    pub cost: f64,
}

/// One discipline cost line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DisciplineCostLine {
    /// The discipline.
    pub name: String,
    /// Its cost.
    pub cost: f64,
}

/// One project cost line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProjectCostLine {
    /// The line's name.
    pub name: String,
    /// Its cost.
    pub cost: f64,
}

/// One operating-cost line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OperatingCostLine {
    /// The line's name.
    pub name: String,
    /// Its annual cost.
    pub cost: f64,
}

/// One material-take-off row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProcessCostMtoRow {
    /// The unit's name.
    pub name: String,
    /// The type category.
    pub equipment_type: String,
    /// The unit's weight.
    pub weight_kg: f64,
    /// The unit's located purchased cost.
    pub cost_usd: f64,
}

/// One quality flag.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProcessCostFlag {
    /// The flag's message.
    pub message: String,
}

/// Result of `process.process_cost_estimate`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProcessCostEstimateResult {
    /// The located purchased equipment cost.
    pub total_purchased_equipment_cost: f64,
    /// The located bare module cost.
    pub total_bare_module_cost: f64,
    /// The located total module cost.
    pub total_module_cost: f64,
    /// The grass roots cost.
    pub total_grass_roots_cost: f64,
    /// The summed installation labour, in seconds.
    pub total_installation_man_hours: f64,
    /// The per-unit summaries.
    pub equipment: Vec<ProcessCostEquipmentResult>,
    /// The located purchased cost by type.
    pub cost_by_equipment_type: Vec<TypeCostLine>,
    /// The five discipline lines.
    pub cost_by_discipline: Vec<DisciplineCostLine>,
    /// The project cost stack.
    pub project_costs: Vec<ProjectCostLine>,
    /// The total project cost.
    pub total_project_cost: f64,
    /// The nine operating-cost lines.
    pub operating_costs: Vec<OperatingCostLine>,
    /// Whether the operating-cost block is present.
    pub has_operating_cost: bool,
    /// The annual operating cost.
    pub total_annual_operating_cost: f64,
    /// Whether the economics are present.
    pub has_economics: bool,
    /// The payback period, or zero where absent.
    pub payback_period_years: f64,
    /// The return on investment.
    pub roi_percent: f64,
    /// The net present value.
    pub npv: f64,
    /// The summed unit total costs.
    pub summed_unit_total_cost: f64,
    /// Whether a system weight was stated.
    pub has_weight_based_capex: bool,
    /// The weight-based capex.
    pub weight_based_capex: f64,
    /// The material-take-off rows.
    pub material_take_off: Vec<ProcessCostMtoRow>,
    /// The quality flags.
    pub quality_flags: Vec<ProcessCostFlag>,
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

/// The roll-up over the stated unit rows.
#[allow(clippy::too_many_arguments)]
pub fn process_cost_estimate(
    equipment: &[ProcessCostEquipment],
    location_factor: Option<f64>,
    complexity_factor: Option<f64>,
    project_contingency: Option<f64>,
    working_capital: Option<f64>,
    system_total_weight: Option<Mass>,
    capex_per_weight: Option<f64>,
    operating_hours: Option<Time>,
    electricity_cost_per_kwh: Option<f64>,
    steam_cost_per_tonne: Option<f64>,
    cooling_water_cost_per_m3: Option<f64>,
    labor_cost_per_hour: Option<f64>,
    annual_revenue: Option<f64>,
    discount_rate: Option<f64>,
    project_life_years: Option<f64>,
) -> Result<ProcessCostEstimateResult> {
    let mut rows = Vec::with_capacity(equipment.len());
    for (index, row) in equipment.iter().enumerate() {
        let man_hours = finite(
            &format!("equipment[{index}].installation_man_hours"),
            row.installation_man_hours.value / 3600.0,
        )?;
        let weight = finite(&format!("equipment[{index}].weight"), row.weight.value)?;
        let power = match row.power {
            Some(power) => Some(finite(
                &format!("equipment[{index}].power"),
                power.value / 1.0e3,
            )?),
            None => None,
        };
        let duty = match row.duty {
            Some(duty) => Some(finite(
                &format!("equipment[{index}].duty"),
                duty.value / 1.0e3,
            )?),
            None => None,
        };
        rows.push(EquipmentRow {
            name: row.name.clone(),
            equipment_class: row.equipment_class.clone(),
            purchased_equipment_cost: finite(
                &format!("equipment[{index}].purchased_equipment_cost"),
                row.purchased_equipment_cost,
            )?,
            bare_module_cost: finite(
                &format!("equipment[{index}].bare_module_cost"),
                row.bare_module_cost,
            )?,
            total_module_cost: finite(
                &format!("equipment[{index}].total_module_cost"),
                row.total_module_cost,
            )?,
            grass_roots_cost: finite(
                &format!("equipment[{index}].grass_roots_cost"),
                row.grass_roots_cost,
            )?,
            total_cost: finite(&format!("equipment[{index}].total_cost"), row.total_cost)?,
            installation_man_hours: man_hours,
            weight,
            power_kw: power,
            duty_kw: duty,
        });
    }

    let operating = match operating_hours {
        Some(hours) => {
            let hours = finite("operating_hours", hours.value / 3600.0)?;
            Some(OpexInputs {
                hours_per_year: hours,
                electricity_cost_per_kwh: number(
                    "electricity_cost_per_kwh",
                    electricity_cost_per_kwh,
                    0.08,
                )?,
                steam_cost_per_tonne: number("steam_cost_per_tonne", steam_cost_per_tonne, 15.0)?,
                cooling_water_cost_per_m3: number(
                    "cooling_water_cost_per_m3",
                    cooling_water_cost_per_m3,
                    0.05,
                )?,
                fuel_cost_per_gj: 5.0,
            })
        }
        None => None,
    };
    let revenue = match annual_revenue {
        Some(value) => Some(finite("annual_revenue", value)?),
        None => None,
    };
    let life = match project_life_years {
        Some(value) => {
            let value = finite("project_life_years", value)?;
            if value < 0.0 || value.fract() != 0.0 {
                return Err(AzothError::invalid_input(
                    "project_life_years",
                    "a project life is a whole number of years",
                ));
            }
            value as u32
        }
        None => 10,
    };
    let system_weight = match system_total_weight {
        Some(value) => Some(finite("system_total_weight", value.value)?),
        None => None,
    };

    let input = ProcessCostInput {
        equipment: rows,
        location_factor: number("location_factor", location_factor, 1.0)?,
        complexity_factor: number("complexity_factor", complexity_factor, 1.0)?,
        project_contingency: number("project_contingency", project_contingency, 0.10)?,
        working_capital: number("working_capital", working_capital, 0.15)?,
        system_total_weight: system_weight,
        capex_per_weight: number("capex_per_weight", capex_per_weight, 1000.0)?,
        opex: operating,
        labor_cost_per_hour: number("labor_cost_per_hour", labor_cost_per_hour, 50.0)?,
        annual_revenue: revenue,
        discount_rate: number("discount_rate", discount_rate, 0.10)?,
        project_life_years: life,
    };
    let result = rollup::estimate(&input);

    let equipment_rows = result
        .equipment
        .iter()
        .map(|row| ProcessCostEquipmentResult {
            name: row.name.clone(),
            equipment_class: row.equipment_class.clone(),
            equipment_type: row.equipment_type.clone(),
            purchased_equipment_cost: row.purchased_equipment_cost,
            bare_module_cost: row.bare_module_cost,
            total_module_cost: row.total_module_cost,
            grass_roots_cost: row.grass_roots_cost,
            total_cost: row.total_cost,
            installation_man_hours: row.installation_man_hours * 3600.0,
            weight: row.weight,
            located_purchased_equipment_cost: row.located_purchased_equipment_cost,
            location_factor: row.location_factor,
        })
        .collect();
    let mto = result
        .equipment
        .iter()
        .map(|row| ProcessCostMtoRow {
            name: row.name.clone(),
            equipment_type: row.equipment_type.clone(),
            weight_kg: row.weight,
            cost_usd: row.located_purchased_equipment_cost,
        })
        .collect();

    let mut project_costs: Vec<ProjectCostLine> = vec![
        ProjectCostLine {
            name: "directInstalledCost".to_string(),
            cost: result.project.direct_installed_cost,
        },
        ProjectCostLine {
            name: "indirectCost".to_string(),
            cost: result.project.indirect_cost,
        },
        ProjectCostLine {
            name: "projectContingency".to_string(),
            cost: result.project.project_contingency,
        },
        ProjectCostLine {
            name: "ownerCost".to_string(),
            cost: result.project.owner_cost,
        },
    ];
    project_costs.push(ProjectCostLine {
        name: "totalProjectCost".to_string(),
        cost: result.project.total_project_cost,
    });

    let operating_costs = result
        .operating_costs
        .as_ref()
        .map(|operating| {
            vec![
                ("Electricity", operating.electricity),
                ("Steam", operating.steam),
                ("Cooling Water", operating.cooling_water),
                ("Maintenance", operating.maintenance),
                ("Operating Labor", operating.operating_labor),
                ("Supervision", operating.supervision),
                ("Operating Supplies", operating.supplies),
                ("Laboratory", operating.laboratory),
                ("Overhead", operating.overhead),
            ]
            .into_iter()
            .map(|(name, cost)| OperatingCostLine {
                name: name.to_string(),
                cost,
            })
            .collect()
        })
        .unwrap_or_default();

    Ok(ProcessCostEstimateResult {
        total_purchased_equipment_cost: result.total_purchased_equipment_cost,
        total_bare_module_cost: result.total_bare_module_cost,
        total_module_cost: result.total_module_cost,
        total_grass_roots_cost: result.total_grass_roots_cost,
        total_installation_man_hours: result.total_installation_man_hours * 3600.0,
        equipment: equipment_rows,
        cost_by_equipment_type: result
            .cost_by_equipment_type
            .iter()
            .map(|(name, cost)| TypeCostLine {
                name: name.clone(),
                cost: *cost,
            })
            .collect(),
        cost_by_discipline: result
            .cost_by_discipline
            .iter()
            .map(|(name, cost)| DisciplineCostLine {
                name: name.clone(),
                cost: *cost,
            })
            .collect(),
        project_costs,
        total_project_cost: result.project.total_project_cost,
        operating_costs,
        has_operating_cost: result.operating_costs.is_some(),
        total_annual_operating_cost: result.annual_operating_cost.unwrap_or(0.0),
        has_economics: result.payback_period_years.is_some()
            || result.roi_percent.is_some()
            || result.npv.is_some(),
        payback_period_years: result.payback_period_years.unwrap_or(0.0),
        roi_percent: result.roi_percent.unwrap_or(0.0),
        npv: result.npv.unwrap_or(0.0),
        summed_unit_total_cost: result.summed_unit_total_cost,
        has_weight_based_capex: result.weight_based_capex.is_some(),
        weight_based_capex: result.weight_based_capex.unwrap_or(0.0),
        material_take_off: mto,
        quality_flags: result
            .quality_flags
            .iter()
            .map(|message| ProcessCostFlag {
                message: message.clone(),
            })
            .collect(),
        warnings: Vec::new(),
    })
}
