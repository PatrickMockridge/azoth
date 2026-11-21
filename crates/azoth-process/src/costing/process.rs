//! The process-level roll-up over per-unit cost rows.
//!
//! `ProcessCostEstimate` walks a live `ProcessSystem`; the port takes the rows
//! the walk produces. Everything below the row — the sums, the location and
//! complexity factors, the discipline split, the project stack, OPEX and the
//! simple economics — is the class's arithmetic unchanged.

use crate::costing::equipment::OpexInputs;
use crate::costing::records::MtoItem;

/// One unit's cost row, the summary the live walk publishes.
#[derive(Clone, Debug)]
pub struct EquipmentRow {
    /// The unit name.
    pub name: String,
    /// The Java class simple name, which selects the type category.
    pub equipment_class: String,
    /// Purchased equipment cost, unlocated USD.
    pub purchased_equipment_cost: f64,
    /// Bare module cost, unlocated USD.
    pub bare_module_cost: f64,
    /// Total module cost, unlocated USD.
    pub total_module_cost: f64,
    /// Grass roots cost, unlocated USD.
    pub grass_roots_cost: f64,
    /// The unit's total cost.
    pub total_cost: f64,
    /// Installation labour, man-hours.
    pub installation_man_hours: f64,
    /// The unit's weight, kg.
    pub weight: f64,
    /// Rotating power, kW, where the unit has it.
    pub power_kw: Option<f64>,
    /// Thermal duty, kW, where the unit has it.
    pub duty_kw: Option<f64>,
}

/// The roll-up inputs.
#[derive(Clone, Debug)]
pub struct ProcessCostInput {
    /// The per-unit rows.
    pub equipment: Vec<EquipmentRow>,
    /// The location factor.
    pub location_factor: f64,
    /// The process complexity factor.
    pub complexity_factor: f64,
    /// The project contingency factor.
    pub project_contingency: f64,
    /// The working-capital (owner) factor.
    pub working_capital: f64,
    /// The system total weight, kg, where the caller states it.
    pub system_total_weight: Option<f64>,
    /// The weight-based CAPEX coefficient, kNOK per tonne.
    pub capex_per_weight: f64,
    /// The OPEX inputs; `None` leaves the OPEX block absent.
    pub opex: Option<OpexInputs>,
    /// Operating labour cost, USD/hour.
    pub labor_cost_per_hour: f64,
    /// Annual revenue, USD, where the caller states it.
    pub annual_revenue: Option<f64>,
    /// The discount rate.
    pub discount_rate: f64,
    /// The project life, years.
    pub project_life_years: u32,
}

impl Default for ProcessCostInput {
    fn default() -> Self {
        Self {
            equipment: Vec::new(),
            location_factor: 1.0,
            complexity_factor: 1.0,
            project_contingency: 0.10,
            working_capital: 0.15,
            system_total_weight: None,
            capex_per_weight: 1000.0,
            opex: None,
            labor_cost_per_hour: 50.0,
            annual_revenue: None,
            discount_rate: 0.10,
            project_life_years: 10,
        }
    }
}

/// One output row, the per-unit summary with its located purchased cost.
#[derive(Clone, Debug)]
pub struct EquipmentSummary {
    /// The unit name.
    pub name: String,
    /// The class simple name.
    pub equipment_class: String,
    /// The type category.
    pub equipment_type: String,
    /// Located purchased equipment cost.
    pub located_purchased_equipment_cost: f64,
    /// Unlocated cost stack.
    pub purchased_equipment_cost: f64,
    /// Unlocated bare module cost.
    pub bare_module_cost: f64,
    /// Unlocated total module cost.
    pub total_module_cost: f64,
    /// Unlocated grass roots cost.
    pub grass_roots_cost: f64,
    /// The unit's total cost.
    pub total_cost: f64,
    /// Installation labour, man-hours.
    pub installation_man_hours: f64,
    /// The unit's weight, kg.
    pub weight: f64,
    /// The location factor applied.
    pub location_factor: f64,
}

/// The OPEX breakdown, the class's nine lines.
#[derive(Clone, Debug, Default)]
pub struct OperatingCost {
    /// Electricity.
    pub electricity: f64,
    /// Steam.
    pub steam: f64,
    /// Cooling water.
    pub cooling_water: f64,
    /// Maintenance.
    pub maintenance: f64,
    /// Operating labour.
    pub operating_labor: f64,
    /// Supervision.
    pub supervision: f64,
    /// Operating supplies.
    pub supplies: f64,
    /// Laboratory.
    pub laboratory: f64,
    /// Overhead.
    pub overhead: f64,
}

impl OperatingCost {
    /// The sum of the nine lines.
    #[must_use]
    pub fn total(&self) -> f64 {
        self.electricity
            + self.steam
            + self.cooling_water
            + self.maintenance
            + self.operating_labor
            + self.supervision
            + self.supplies
            + self.laboratory
            + self.overhead
    }
}

/// The project cost stack.
#[derive(Clone, Debug, Default)]
pub struct ProjectCost {
    /// The direct installed cost (total module).
    pub direct_installed_cost: f64,
    /// The indirect cost.
    pub indirect_cost: f64,
    /// The project contingency.
    pub project_contingency: f64,
    /// The owner cost.
    pub owner_cost: f64,
    /// The total project cost.
    pub total_project_cost: f64,
}

/// The roll-up result.
#[derive(Clone, Debug, Default)]
pub struct ProcessCostResult {
    /// Total purchased equipment cost, located.
    pub total_purchased_equipment_cost: f64,
    /// Total bare module cost, located.
    pub total_bare_module_cost: f64,
    /// Total module cost, located.
    pub total_module_cost: f64,
    /// Grass roots cost.
    pub total_grass_roots_cost: f64,
    /// Total installation labour.
    pub total_installation_man_hours: f64,
    /// The per-unit summaries.
    pub equipment: Vec<EquipmentSummary>,
    /// Cost by equipment type, located PEC.
    pub cost_by_equipment_type: Vec<(String, f64)>,
    /// Cost by discipline.
    pub cost_by_discipline: Vec<(String, f64)>,
    /// The project stack.
    pub project: ProjectCost,
    /// The OPEX breakdown, where OPEX was requested.
    pub operating_costs: Option<OperatingCost>,
    /// The annual operating cost.
    pub annual_operating_cost: Option<f64>,
    /// Payback, years, where profitable.
    pub payback_period_years: Option<f64>,
    /// Return on investment, percent.
    pub roi_percent: Option<f64>,
    /// Net present value, USD.
    pub npv: Option<f64>,
    /// The sum of the rows' total costs.
    pub summed_unit_total_cost: f64,
    /// The weight-based CAPEX, kNOK.
    pub weight_based_capex: Option<f64>,
    /// The material-take-off rows.
    pub material_take_off: Vec<MtoItem>,
    /// Quality flags.
    pub quality_flags: Vec<String>,
}

/// Classify a unit by its Java class simple name, the class's ordered `contains` chain.
#[must_use]
pub fn classify_equipment(equipment_class: &str) -> &'static str {
    if equipment_class.contains("Separator") || equipment_class.contains("Scrubber") {
        "Vessels"
    } else if equipment_class.contains("Compressor") {
        "Compressors"
    } else if equipment_class.contains("Pump") {
        "Pumps"
    } else if equipment_class.contains("Expander") {
        "Expanders"
    } else if equipment_class.contains("HeatExchanger")
        || equipment_class.contains("Cooler")
        || equipment_class.contains("Heater")
    {
        "Heat Exchangers"
    } else if equipment_class.contains("Valve") {
        "Valves"
    } else if equipment_class.contains("Tank") {
        "Tanks"
    } else if equipment_class.contains("Column")
        || equipment_class.contains("Distillation")
        || equipment_class.contains("Absorber")
    {
        "Columns"
    } else if equipment_class.contains("Mixer") || equipment_class.contains("Splitter") {
        "Piping"
    } else if equipment_class.contains("Pipeline") || equipment_class.contains("Pipe") {
        "Pipelines"
    } else if equipment_class.contains("Well") {
        "Wells"
    } else if equipment_class.contains("Reservoir") {
        "Reservoir"
    } else if equipment_class.contains("Stream") {
        "Streams"
    } else {
        "Other"
    }
}

/// Roll one equipment summary into the process totals. `IndexMap`-free: the
/// type order is first appearance, as the class's `LinkedHashMap` keeps it.
#[must_use]
pub fn estimate(input: &ProcessCostInput) -> ProcessCostResult {
    let mut result = ProcessCostResult::default();
    if input.equipment.is_empty() {
        result.quality_flags.push(
            "No equipment cost summaries were available. State at least one unit.".to_string(),
        );
    }

    let mut total_pec = 0.0;
    let mut total_bmc = 0.0;
    let mut total_tmc = 0.0;
    let mut total_man_hours = 0.0;
    let mut by_type: Vec<(String, f64)> = Vec::new();

    for row in &input.equipment {
        let equipment_type = classify_equipment(&row.equipment_class).to_string();
        total_pec += row.purchased_equipment_cost;
        total_bmc += row.bare_module_cost;
        total_tmc += row.total_module_cost;
        total_man_hours += row.installation_man_hours;
        let located = row.purchased_equipment_cost * input.location_factor;
        match by_type.iter_mut().find(|(name, _)| *name == equipment_type) {
            Some((_, value)) => *value += located,
            None => by_type.push((equipment_type.clone(), located)),
        }
        result.equipment.push(EquipmentSummary {
            name: row.name.clone(),
            equipment_class: row.equipment_class.clone(),
            equipment_type,
            located_purchased_equipment_cost: located,
            purchased_equipment_cost: row.purchased_equipment_cost,
            bare_module_cost: row.bare_module_cost,
            total_module_cost: row.total_module_cost,
            grass_roots_cost: row.grass_roots_cost,
            total_cost: row.total_cost,
            installation_man_hours: row.installation_man_hours,
            weight: row.weight,
            location_factor: input.location_factor,
        });
        result.summed_unit_total_cost += row.total_cost;
        result.material_take_off.push(MtoItem {
            item: row.name.clone(),
            material: None,
            quantity: Some(row.weight),
            unit: Some("kg".to_string()),
            weight_kg: Some(row.weight),
            cost_usd: located,
            source: Some("process-cost-estimate".to_string()),
        });
    }

    result.total_purchased_equipment_cost = total_pec * input.location_factor;
    result.total_bare_module_cost = total_bmc * input.location_factor;
    result.total_module_cost = total_tmc * input.location_factor;
    result.total_installation_man_hours = total_man_hours;
    result.cost_by_equipment_type = by_type;

    let tmc = result.total_module_cost;
    result.total_grass_roots_cost = (tmc + 0.10 * tmc + 0.05 * tmc) * input.complexity_factor;

    result.cost_by_discipline = vec![
        (
            "Process Equipment".to_string(),
            result.total_purchased_equipment_cost,
        ),
        (
            "Piping & Valves".to_string(),
            result.total_bare_module_cost * 0.25 / 0.30,
        ),
        (
            "Electrical & Instrumentation".to_string(),
            result.total_bare_module_cost * 0.15 / 0.30,
        ),
        (
            "Civil & Structural".to_string(),
            result.total_bare_module_cost * 0.10 / 0.30,
        ),
        (
            "Other (HVAC, Paint, Insulation)".to_string(),
            result.total_bare_module_cost * 0.20 / 0.30,
        ),
    ];

    let direct = tmc;
    let indirect = (result.total_grass_roots_cost - tmc).max(0.0);
    let contingency = (direct + indirect) * input.project_contingency;
    let owner = (direct + indirect) * input.working_capital;
    result.project = ProjectCost {
        direct_installed_cost: direct,
        indirect_cost: indirect,
        project_contingency: contingency,
        owner_cost: owner,
        total_project_cost: direct + indirect + contingency + owner,
    };

    if let Some(opex) = input.opex {
        let mut operating = OperatingCost::default();
        let mut major = 0usize;
        for row in &input.equipment {
            major += 1;
            let class = &row.equipment_class;
            if (class.contains("Compressor") || class.contains("Pump"))
                && let Some(power) = row.power_kw
            {
                operating.electricity +=
                    power * opex.hours_per_year * opex.electricity_cost_per_kwh;
            }
            if (class.contains("Heater") || class.contains("Reboiler"))
                && let Some(duty) = row.duty_kw
            {
                let steam_flow = duty.abs() * 3.6 / 2200.0;
                operating.steam +=
                    steam_flow * opex.hours_per_year * opex.steam_cost_per_tonne / 1000.0;
            }
            if (class.contains("Cooler") || class.contains("Condenser"))
                && let Some(duty) = row.duty_kw
            {
                let flow = duty.abs() * 3.6 / (4.18 * 10.0) / 1000.0;
                operating.cooling_water +=
                    flow * opex.hours_per_year * opex.cooling_water_cost_per_m3;
            }
        }
        operating.maintenance = result.total_module_cost * 0.04;
        let operators = (major.max(1) / 5).max(2);
        let shifts = 3;
        operating.operating_labor =
            (operators * shifts) as f64 * opex.hours_per_year / 3.0 * input.labor_cost_per_hour;
        operating.supervision = operating.operating_labor * 0.18;
        operating.supplies = operating.maintenance * 0.12;
        operating.laboratory = operating.operating_labor * 0.15;
        operating.overhead =
            (operating.operating_labor + operating.supervision + operating.maintenance) * 0.55;
        let annual = operating.total();
        result.annual_operating_cost = Some(annual);
        result.operating_costs = Some(operating);

        if let Some(revenue) = input.annual_revenue {
            let profit = revenue - annual;
            result.payback_period_years = if profit > 0.0 {
                Some(result.total_grass_roots_cost / profit)
            } else {
                None
            };
            result.roi_percent = Some(if result.total_grass_roots_cost <= 0.0 {
                0.0
            } else {
                profit / result.total_grass_roots_cost * 100.0
            });
            let mut npv = -result.total_grass_roots_cost;
            for year in 1..=input.project_life_years {
                npv += profit / (1.0 + input.discount_rate).powi(year as i32);
            }
            result.npv = Some(npv);
        }
    }

    result.weight_based_capex = input
        .system_total_weight
        .map(|weight| weight * input.capex_per_weight);

    if result.total_purchased_equipment_cost <= 0.0 {
        result
            .quality_flags
            .push("No positive purchased equipment cost was available.".to_string());
    }
    result
}
