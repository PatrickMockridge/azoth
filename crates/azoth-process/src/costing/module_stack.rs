//! The purchased-to-grass-roots module stack.

use crate::costing::correlations::installation_man_hours;
use crate::costing::factors::{
    BARE_MODULE_B1, BARE_MODULE_B2, GRASS_ROOTS_FACTOR, pressure_factor,
};

/// The five capital-cost numbers one equipment estimate publishes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModuleStack {
    /// Purchased equipment cost, USD.
    pub purchased_equipment_cost: f64,
    /// Bare module cost, USD.
    pub bare_module_cost: f64,
    /// Total module cost, USD.
    pub total_module_cost: f64,
    /// Grass roots cost, USD.
    pub grass_roots_cost: f64,
    /// Installation labour, man-hours.
    pub installation_man_hours: f64,
}

/// The bare module cost, `PEC * (B1 + B2 * Fp * Fm)`.
#[must_use]
pub fn bare_module(purchased_cost: f64, pressure_factor: f64, material_factor: f64) -> f64 {
    purchased_cost * (BARE_MODULE_B1 + BARE_MODULE_B2 * pressure_factor * material_factor)
}

/// The total module cost, `BMC * (1 + contingency + engineering)`.
#[must_use]
pub fn total_module(bare_module_cost: f64, contingency: f64, engineering: f64) -> f64 {
    bare_module_cost * (1.0 + contingency + engineering)
}

/// The grass roots cost, 1.5 times the total module cost.
#[must_use]
pub fn grass_roots(total_module_cost: f64) -> f64 {
    total_module_cost * GRASS_ROOTS_FACTOR
}

/// The unit estimator's stack basis.
#[derive(Clone, Copy, Debug)]
pub struct StackInput<'a> {
    /// Whether a mechanical design is present; without one the class returns
    /// the purchased cost and nothing else.
    pub design_present: bool,
    /// The design's maximum pressure, barg.
    pub max_design_pressure_barg: f64,
    /// The equipment weight, kg.
    pub weight_kg: f64,
    /// The class's equipment-type string.
    pub equipment_type: &'a str,
    /// The material factor.
    pub material_factor: f64,
    /// The contingency factor.
    pub contingency: f64,
    /// The engineering factor.
    pub engineering: f64,
}

/// The unit estimator's stack: nothing beyond the purchased cost without a design.
#[must_use]
pub fn equipment_stack(purchased_equipment_cost: f64, input: &StackInput<'_>) -> ModuleStack {
    if !input.design_present {
        return ModuleStack {
            purchased_equipment_cost,
            ..ModuleStack::default()
        };
    }
    let bmc = bare_module(
        purchased_equipment_cost,
        pressure_factor(input.max_design_pressure_barg),
        input.material_factor,
    );
    let tmc = total_module(bmc, input.contingency, input.engineering);
    ModuleStack {
        purchased_equipment_cost,
        bare_module_cost: bmc,
        total_module_cost: tmc,
        grass_roots_cost: grass_roots(tmc),
        installation_man_hours: installation_man_hours(input.weight_kg, input.equipment_type),
    }
}

/// The calculator's summary-route basis.
#[derive(Clone, Copy, Debug)]
pub struct SummaryInput<'a> {
    /// The design pressure, barg.
    pub design_pressure_barg: f64,
    /// The equipment weight, kg.
    pub weight_kg: f64,
    /// The class's equipment-type string.
    pub equipment_type: &'a str,
    /// The material factor.
    pub material_factor: f64,
    /// The location factor.
    pub location_factor: f64,
    /// The contingency factor.
    pub contingency: f64,
    /// The engineering factor.
    pub engineering: f64,
}

/// The calculator's summary route: the purchased cost is located and material-
/// factored, and the bare module uses `Fm = 1.0` rather than the same factor.
#[must_use]
pub fn calculator_summary(purchased_cost: f64, input: &SummaryInput<'_>) -> ModuleStack {
    let pec = purchased_cost * input.material_factor * input.location_factor;
    let bmc = bare_module(pec, pressure_factor(input.design_pressure_barg), 1.0);
    let tmc = total_module(bmc, input.contingency, input.engineering);
    ModuleStack {
        purchased_equipment_cost: pec,
        bare_module_cost: bmc,
        total_module_cost: tmc,
        grass_roots_cost: grass_roots(tmc),
        installation_man_hours: installation_man_hours(input.weight_kg, input.equipment_type),
    }
}
