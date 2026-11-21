//! `process.equipment_cost_estimate` - one equipment item's cost stack.
//!
//! Spec: `specs/models/process/equipment_cost_estimate.toml`. The arithmetic is
//! `UnitCostEstimateBaseClass` and its per-equipment subclasses at the pin: the
//! purchased cost of the selected kind, the shared module stack, the labour
//! hours and the weight bill of materials.

use azoth_core::units::{Length, Mass, Power, Pressure, Time, Volume};
use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::costing::equipment as costing;
use crate::costing::factors;
use crate::executor::json::warnings as wire_warnings;

/// One equipment item, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct EquipmentCostSpec {
    /// The equipment kind.
    pub equipment: String,
    /// The item's name.
    pub name: Option<String>,
    /// The material of construction.
    pub material: Option<String>,
    /// A stated material factor.
    pub material_factor: Option<f64>,
    /// The current cost index.
    pub cepci: Option<f64>,
    /// Whether a mechanical design is stated.
    pub mechanical_design_present: Option<bool>,
    /// The design's maximum pressure, bar.
    pub max_design_pressure: Option<Pressure>,
    /// The equipment weight.
    pub weight_total: Option<Mass>,
    /// A design pressure read in barg.
    pub design_pressure_barg: Option<Pressure>,
    /// A design pressure read in bara.
    pub design_pressure_bara: Option<Pressure>,
    /// The contingency factor.
    pub contingency_factor: Option<f64>,
    /// The engineering factor.
    pub engineering_factor: Option<f64>,
    /// Whether to publish the weight bill of materials.
    pub include_material_take_off: Option<bool>,
    /// The vessel shell weight.
    pub shell_weight: Option<Mass>,
    /// The internals weight.
    pub internals_weight: Option<Mass>,
    /// The nozzle weight.
    pub nozzle_weight: Option<Mass>,
    /// The piping weight.
    pub piping_weight: Option<Mass>,
    /// The electrical and instrumentation weight.
    pub electro_instrument_weight: Option<Mass>,
    /// The structural steel weight.
    pub structural_weight: Option<Mass>,
    /// Annual operating hours.
    pub operating_hours: Option<Time>,
    /// Electricity price, USD/kWh.
    pub electricity_cost_per_kwh: Option<f64>,
    /// Steam price, USD/tonne.
    pub steam_cost_per_tonne: Option<f64>,
    /// Cooling-water price, USD/m3.
    pub cooling_water_cost_per_m3: Option<f64>,
    /// Fuel price, USD/GJ.
    pub fuel_cost_per_gj: Option<f64>,
    /// A vessel's inner diameter.
    pub inner_diameter: Option<Length>,
    /// The separator's tangent length.
    pub tangent_length: Option<Length>,
    /// A stated vessel volume.
    pub vessel_volume: Option<Volume>,
    /// The column internals.
    pub column_type: Option<String>,
    /// The tray type.
    pub tray_type: Option<String>,
    /// The packing material.
    pub packing_type: Option<String>,
    /// The column's or absorber's diameter.
    pub diameter: Option<Length>,
    /// The column's or absorber's height.
    pub height: Option<Length>,
    /// The column's tray count.
    pub number_of_trays: Option<f64>,
    /// The absorber's stage count.
    pub number_of_stages: Option<f64>,
    /// The packed section's height.
    pub packing_height: Option<Length>,
    /// Whether the reboiler is costed.
    pub include_reboiler: Option<bool>,
    /// Whether the column condenser is costed.
    pub include_condenser: Option<bool>,
    /// Whether the absorber's liquid distributor is costed.
    pub include_liquid_distributor: Option<bool>,
    /// Whether the absorber's mist eliminator is costed.
    pub include_mist_eliminator: Option<bool>,
    /// Whether the absorber's reflux system is costed.
    pub include_reflux_system: Option<bool>,
    /// The reboiler duty.
    pub reboiler_duty: Option<Power>,
    /// The condenser duty.
    pub condenser_duty: Option<Power>,
    /// The absorber internals.
    pub absorber_type: Option<String>,
}

/// One equipment row of the result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EquipmentCostResultRow {
    /// The stated name.
    pub name: String,
    /// The class's equipment-type string.
    pub equipment_type: String,
    /// The purchased cost, USD.
    pub purchased_equipment_cost: f64,
    /// The bare module cost, USD.
    pub bare_module_cost: f64,
    /// The total module cost, USD.
    pub total_module_cost: f64,
    /// The grass roots cost, USD.
    pub grass_roots_cost: f64,
    /// The class's total-cost answer, USD.
    pub total_cost: f64,
    /// The installation labour hours, in seconds.
    pub installation_man_hours: f64,
    /// The material factor.
    pub material_factor: f64,
    /// A published ex-works cost.
    pub equipment_cost: f64,
    /// A published installed cost.
    pub installed_cost: f64,
    /// A published cost per square metre.
    pub specific_cost_per_m2: f64,
    /// A published cost per kilowatt.
    pub specific_capex_per_kw: f64,
    /// A published annual maintenance cost.
    pub annual_maintenance_cost: f64,
    /// A published sorbent replacement cost.
    pub sorbent_replacement_cost: f64,
    /// The annual operating cost.
    pub annual_operating_cost: f64,
    /// A weight the class computes rather than carries.
    pub calculated_weight: f64,
}

/// One material-take-off row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EquipmentMtoRow {
    /// The equipment row this line belongs to.
    pub equipment_index: f64,
    /// The line's item.
    pub item: String,
    /// The line's material.
    pub material: String,
    /// The count, where the line is a count.
    pub quantity: f64,
    /// Whether the line carries a count.
    pub has_quantity: bool,
    /// The count's unit.
    pub unit: String,
    /// The line's weight.
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
pub struct EquipmentQualityFlag {
    /// The equipment row the flag belongs to.
    pub equipment_index: f64,
    /// The flag's message.
    pub message: String,
}

/// Result of `process.equipment_cost_estimate`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EquipmentCostEstimateResult {
    /// One row per equipment item.
    pub equipment: Vec<EquipmentCostResultRow>,
    /// The weighted bill of materials.
    pub material_take_off: Vec<EquipmentMtoRow>,
    /// The quality flags.
    pub quality_flags: Vec<EquipmentQualityFlag>,
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

fn require<T>(index: usize, kind: &str, field: &'static str, value: Option<T>) -> Result<T> {
    value.ok_or_else(|| {
        AzothError::invalid_input(
            format!("equipment[{index}].{field}"),
            format!("a {kind} row requires `{field}`"),
        )
    })
}

fn whole(index: usize, field: &'static str, value: f64) -> Result<f64> {
    let value = finite(format!("equipment[{index}].{field}"), value)?;
    if value.fract() != 0.0 {
        return Err(AzothError::invalid_input(
            format!("equipment[{index}].{field}"),
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

/// The equipment's purchased cost and stack for one row.
pub fn equipment_cost_estimate(
    equipment: &[EquipmentCostSpec],
) -> Result<EquipmentCostEstimateResult> {
    let mut rows = Vec::with_capacity(equipment.len());
    let mut mto = Vec::new();
    let mut flags = Vec::new();

    for (index, row) in equipment.iter().enumerate() {
        let kind = row.equipment.as_str();
        if !matches!(kind, "separator" | "column" | "absorber") {
            return Err(AzothError::invalid_input(
                format!("equipment[{index}].equipment"),
                format!("{kind:?} is not a kind this port has reached"),
            ));
        }
        let cepci = finite(
            format!("equipment[{index}].cepci"),
            row.cepci.unwrap_or(840.0),
        )?;
        if cepci <= 0.0 {
            return Err(AzothError::invalid_input(
                format!("equipment[{index}].cepci"),
                "a cost index is positive",
            ));
        }
        let material_factor = match (&row.material, row.material_factor) {
            (_, Some(value)) => finite(format!("equipment[{index}].material_factor"), value)?,
            (Some(name), None) => {
                let canonical = material_name(name).ok_or_else(|| {
                    AzothError::invalid_input(
                        format!("equipment[{index}].material"),
                        format!("{name:?} is not a material with a factor"),
                    )
                })?;
                factors::material_factor(canonical).unwrap_or(1.0)
            }
            (None, None) => 1.0,
        };
        let contingency = finite(
            format!("equipment[{index}].contingency_factor"),
            row.contingency_factor.unwrap_or(0.15),
        )?;
        let engineering = finite(
            format!("equipment[{index}].engineering_factor"),
            row.engineering_factor.unwrap_or(0.10),
        )?;
        let max_pressure = match row.max_design_pressure {
            Some(pressure) => finite(
                format!("equipment[{index}].max_design_pressure"),
                pressure.value / 1.0e5,
            )?,
            None => 0.0,
        };
        let weight = match row.weight_total {
            Some(weight) => finite(format!("equipment[{index}].weight_total"), weight.value)?,
            None => 0.0,
        };
        let design_barg = match row.design_pressure_barg {
            Some(pressure) => Some(finite(
                format!("equipment[{index}].design_pressure_barg"),
                pressure.value / 1.0e5,
            )?),
            None => None,
        };
        let _design_bara = match row.design_pressure_bara {
            Some(pressure) => Some(finite(
                format!("equipment[{index}].design_pressure_bara"),
                pressure.value / 1.0e5,
            )?),
            None => None,
        };
        let common = costing::Common {
            material_factor,
            cepci,
            design_present: row.mechanical_design_present.unwrap_or(false),
            max_design_pressure_barg: max_pressure,
            weight_total_kg: weight,
            contingency,
            engineering,
        };

        let spec = match kind {
            "separator" => {
                let volume = match row.vessel_volume {
                    Some(volume) => Some(finite(
                        format!("equipment[{index}].vessel_volume"),
                        volume.value,
                    )?),
                    None => None,
                };
                let diameter = match row.inner_diameter {
                    Some(value) => Some(finite(
                        format!("equipment[{index}].inner_diameter"),
                        value.value,
                    )?),
                    None => None,
                };
                let length = match row.tangent_length {
                    Some(value) => Some(finite(
                        format!("equipment[{index}].tangent_length"),
                        value.value,
                    )?),
                    None => None,
                };
                if volume.is_none() && (diameter.is_none() || length.is_none()) {
                    return Err(AzothError::invalid_input(
                        format!("equipment[{index}].inner_diameter"),
                        "a separator requires a vessel volume or both a diameter and a tangent length",
                    ));
                }
                costing::EquipmentSpec::Separator(costing::SeparatorSpec {
                    inner_diameter_m: diameter.unwrap_or(0.0),
                    tangent_length_m: length.unwrap_or(0.0),
                    vessel_volume_m3: volume,
                })
            }
            "column" => {
                let diameter = finite(
                    format!("equipment[{index}].diameter"),
                    require(index, kind, "diameter", row.diameter)?.value,
                )?;
                let height = finite(
                    format!("equipment[{index}].height"),
                    require(index, kind, "height", row.height)?.value,
                )?;
                let number = match row.number_of_trays {
                    Some(value) => whole(index, "number_of_trays", value)?,
                    None => 0.0,
                };
                let packing_height = match row.packing_height {
                    Some(value) => {
                        finite(format!("equipment[{index}].packing_height"), value.value)?
                    }
                    None => 0.0,
                };
                let reboiler_duty = match row.reboiler_duty {
                    Some(value) => finite(
                        format!("equipment[{index}].reboiler_duty"),
                        value.value / 1.0e3,
                    )?,
                    None => 0.0,
                };
                let condenser_duty = match row.condenser_duty {
                    Some(value) => finite(
                        format!("equipment[{index}].condenser_duty"),
                        value.value / 1.0e3,
                    )?,
                    None => 0.0,
                };
                costing::EquipmentSpec::Column(costing::ColumnSpec {
                    column_type: row
                        .column_type
                        .clone()
                        .unwrap_or_else(|| "trayed".to_string()),
                    tray_type: row.tray_type.clone().unwrap_or_else(|| "sieve".to_string()),
                    packing_type: row
                        .packing_type
                        .clone()
                        .unwrap_or_else(|| "structured".to_string()),
                    diameter_m: diameter,
                    height_m: height,
                    number_of_trays: number,
                    packing_height_m: packing_height,
                    design_pressure_barg: design_barg.unwrap_or(0.0),
                    include_reboiler: row.include_reboiler.unwrap_or(false),
                    include_condenser: row.include_condenser.unwrap_or(false),
                    reboiler_duty_kw: reboiler_duty,
                    condenser_duty_kw: condenser_duty,
                })
            }
            "absorber" => {
                let diameter = finite(
                    format!("equipment[{index}].diameter"),
                    require(index, kind, "diameter", row.diameter)?.value,
                )?;
                let height = finite(
                    format!("equipment[{index}].height"),
                    require(index, kind, "height", row.height)?.value,
                )?;
                let stages = match row.number_of_stages {
                    Some(value) => whole(index, "number_of_stages", value)?,
                    None => 0.0,
                };
                let packing_height = match row.packing_height {
                    Some(value) => {
                        finite(format!("equipment[{index}].packing_height"), value.value)?
                    }
                    None => 0.0,
                };
                let reboiler_duty = match row.reboiler_duty {
                    Some(value) => finite(
                        format!("equipment[{index}].reboiler_duty"),
                        value.value / 1.0e3,
                    )?,
                    None => 0.0,
                };
                costing::EquipmentSpec::Absorber(costing::AbsorberSpec {
                    absorber_type: row
                        .absorber_type
                        .clone()
                        .unwrap_or_else(|| "packed".to_string()),
                    tray_type: row.tray_type.clone().unwrap_or_else(|| "sieve".to_string()),
                    packing_type: row
                        .packing_type
                        .clone()
                        .unwrap_or_else(|| "structured".to_string()),
                    diameter_m: diameter,
                    height_m: height,
                    number_of_stages: stages,
                    packing_height_m: packing_height,
                    include_liquid_distributor: row.include_liquid_distributor.unwrap_or(true),
                    include_mist_eliminator: row.include_mist_eliminator.unwrap_or(true),
                    include_reboiler: row.include_reboiler.unwrap_or(false),
                    include_reflux_system: row.include_reflux_system.unwrap_or(false),
                    reboiler_duty_kw: reboiler_duty,
                })
            }
            _ => unreachable!("the kind list was checked above"),
        };

        let evaluated = costing::evaluate(&spec, &common);
        let operating = match row.operating_hours {
            Some(hours) => {
                let hours = finite(
                    format!("equipment[{index}].operating_hours"),
                    hours.value / 3600.0,
                )?;
                let opex = costing::OpexInputs {
                    hours_per_year: hours,
                    electricity_cost_per_kwh: finite(
                        format!("equipment[{index}].electricity_cost_per_kwh"),
                        row.electricity_cost_per_kwh.unwrap_or(0.08),
                    )?,
                    steam_cost_per_tonne: finite(
                        format!("equipment[{index}].steam_cost_per_tonne"),
                        row.steam_cost_per_tonne.unwrap_or(15.0),
                    )?,
                    cooling_water_cost_per_m3: finite(
                        format!("equipment[{index}].cooling_water_cost_per_m3"),
                        row.cooling_water_cost_per_m3.unwrap_or(0.05),
                    )?,
                    fuel_cost_per_gj: finite(
                        format!("equipment[{index}].fuel_cost_per_gj"),
                        row.fuel_cost_per_gj.unwrap_or(5.0),
                    )?,
                };
                costing::annual_operating_cost(&spec, &evaluated.stack, &opex)
            }
            None => 0.0,
        };

        rows.push(EquipmentCostResultRow {
            name: row.name.clone().unwrap_or_default(),
            equipment_type: evaluated.equipment_type.to_string(),
            purchased_equipment_cost: evaluated.stack.purchased_equipment_cost,
            bare_module_cost: evaluated.stack.bare_module_cost,
            total_module_cost: evaluated.stack.total_module_cost,
            grass_roots_cost: evaluated.stack.grass_roots_cost,
            total_cost: evaluated.total_cost,
            installation_man_hours: evaluated.stack.installation_man_hours * 3600.0,
            material_factor,
            equipment_cost: evaluated.extras.equipment_cost.unwrap_or(0.0),
            installed_cost: evaluated.extras.installed_cost.unwrap_or(0.0),
            specific_cost_per_m2: evaluated.extras.specific_cost_per_m2.unwrap_or(0.0),
            specific_capex_per_kw: evaluated.extras.specific_capex_per_kw.unwrap_or(0.0),
            annual_maintenance_cost: evaluated.extras.annual_maintenance_cost.unwrap_or(0.0),
            sorbent_replacement_cost: evaluated.extras.sorbent_replacement_cost.unwrap_or(0.0),
            annual_operating_cost: operating,
            calculated_weight: evaluated.extras.calculated_weight_kg.unwrap_or(0.0),
        });

        for flag in &evaluated.quality_flags {
            flags.push(EquipmentQualityFlag {
                equipment_index: index as f64,
                message: flag.clone(),
            });
        }

        if row.include_material_take_off.unwrap_or(false) {
            let material = row
                .material
                .as_deref()
                .and_then(material_name)
                .unwrap_or("Carbon Steel");
            let breakdown = costing::WeightBreakdown {
                shell_kg: mass(row.shell_weight),
                internals_kg: mass(row.internals_weight),
                nozzle_kg: mass(row.nozzle_weight),
                piping_kg: mass(row.piping_weight),
                electro_instrument_kg: mass(row.electro_instrument_weight),
                structural_kg: mass(row.structural_weight),
                material: material.to_string(),
                cost_per_weight_unit: factors::COST_PER_WEIGHT_UNIT,
            };
            for line in costing::weight_bom(&breakdown) {
                mto.push(EquipmentMtoRow {
                    equipment_index: index as f64,
                    item: line.item,
                    material: line.material.unwrap_or_default(),
                    quantity: line.quantity.unwrap_or(0.0),
                    has_quantity: line.quantity.is_some(),
                    unit: line.unit.unwrap_or_default(),
                    weight_kg: line.weight_kg.unwrap_or(0.0),
                    has_weight: line.weight_kg.is_some(),
                    cost_usd: line.cost_usd,
                    source: line.source.unwrap_or_default(),
                });
            }
        }
    }

    Ok(EquipmentCostEstimateResult {
        equipment: rows,
        material_take_off: mto,
        quality_flags: flags,
        warnings: Vec::new(),
    })
}

fn mass(value: Option<Mass>) -> f64 {
    value.map(|mass| mass.value).unwrap_or(0.0)
}
