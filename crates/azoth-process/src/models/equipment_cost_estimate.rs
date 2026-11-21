//! `process.equipment_cost_estimate` - one equipment item's cost stack.
//!
//! Spec: `specs/models/process/equipment_cost_estimate.toml`. The arithmetic is
//! `UnitCostEstimateBaseClass` and its per-equipment subclasses at the pin: the
//! purchased cost of the selected kind, the shared module stack, the labour
//! hours and the weight bill of materials.

use azoth_core::units::{Area, Length, Mass, MassRate, Power, Pressure, Time, Volume};
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
    /// The reactor vessel's diameter.
    pub vessel_diameter: Option<Length>,
    /// The reactor vessel's length.
    pub vessel_length: Option<Length>,
    /// The loaded catalyst mass.
    pub catalyst_mass: Option<Mass>,
    /// The reactor's catalyst-bed count.
    pub number_of_beds: Option<f64>,
    /// The reactor's total equipped weight.
    pub total_equipped_weight: Option<Mass>,
    /// The catalyst's price, USD/kg.
    pub catalyst_cost_per_kg: Option<f64>,
    /// The reactor internals' cost fraction.
    pub internals_fraction: Option<f64>,
    /// The distributor allowance per bed, USD.
    pub bed_allowance: Option<f64>,
    /// The compressor's or pump's shaft power.
    pub power: Option<Power>,
    /// The compressor type.
    pub compressor_type: Option<String>,
    /// The compressor's driver.
    pub driver_type: Option<String>,
    /// Whether the driver is costed.
    pub include_driver: Option<bool>,
    /// Whether the compressor's intercoolers are costed.
    pub include_intercoolers: Option<bool>,
    /// The pump type.
    pub pump_type: Option<String>,
    /// Whether the pump's motor is costed.
    pub include_motor: Option<bool>,
    /// Whether the pump is API 610 rated.
    pub api_rated: Option<bool>,
    /// The pump seal.
    pub seal_type: Option<String>,
    /// The expander's shaft power.
    pub shaft_power: Option<Power>,
    /// The expander type.
    pub expander_type: Option<String>,
    /// The expander's load.
    pub load_type: Option<String>,
    /// Whether the load is costed.
    pub include_load: Option<bool>,
    /// Whether a gearbox is costed.
    pub include_gearbox: Option<bool>,
    /// Whether the lube-oil system is costed.
    pub include_lube_oil_system: Option<bool>,
    /// Whether the control system is costed.
    pub include_control_system: Option<bool>,
    /// Whether the expander is cryogenic.
    pub cryogenic_service: Option<bool>,
    /// The exchanger's heat-transfer area.
    pub area: Option<Area>,
    /// The exchanger's duty.
    pub duty: Option<Power>,
    /// The exchanger type.
    pub exchanger_type: Option<String>,
    /// The TEMA designation.
    pub tema_type: Option<String>,
    /// The exchanger's shell passes.
    pub shell_passes: Option<f64>,
    /// The tank type.
    pub tank_type: Option<String>,
    /// The tank's volume.
    pub volume: Option<Volume>,
    /// Whether a floating roof is fitted.
    pub floating_roof: Option<bool>,
    /// Whether the tank foundation is costed.
    pub include_foundation: Option<bool>,
    /// Whether heating coils are costed.
    pub include_heating_coils: Option<bool>,
    /// Whether insulation is costed.
    pub include_insulation: Option<bool>,
    /// The insulation thickness.
    pub insulation_thickness: Option<Length>,
    /// The filter's element count.
    pub required_elements: Option<f64>,
    /// The filter's material grade.
    pub material_grade: Option<String>,
    /// The filter's empty shell weight.
    pub empty_vessel_weight: Option<Mass>,
    /// The filter element's price.
    pub element_cost: Option<f64>,
    /// The filter's auxiliaries fraction.
    pub auxiliaries_fraction: Option<f64>,
    /// The design's steel rate.
    pub fallback_steel_rate: Option<f64>,
    /// The design's element price.
    pub fallback_element_cost: Option<f64>,
    /// The manifold's inlet count.
    pub inlets: Option<f64>,
    /// The manifold's outlet count.
    pub outlets: Option<f64>,
    /// The manifold's location.
    pub location: Option<String>,
    /// The manifold service.
    pub manifold_type: Option<String>,
    /// The manifold header's diameter.
    pub header_diameter: Option<Length>,
    /// The manifold branch's diameter.
    pub branch_diameter: Option<Length>,
    /// The water depth.
    pub water_depth: Option<Length>,
    /// The manifold's fabricated cost per kilogram.
    pub base_cost_per_kg: Option<f64>,
    /// The manifold's branch allowance.
    pub branch_allowance: Option<f64>,
    /// The mercury bed's sorbent charge weight.
    pub sorbent_charge_weight: Option<Mass>,
    /// The sorbent price.
    pub sorbent_unit_price: Option<f64>,
    /// The mercury bed's steel rate.
    pub steel_cost_per_kg: Option<f64>,
    /// The electrolyzer technology.
    pub technology: Option<String>,
    /// Whether the balance of plant is costed.
    pub include_balance_of_plant: Option<bool>,
    /// The PSA sorbent.
    pub sorbent: Option<String>,
    /// The PSA sorbent mass per bed.
    pub sorbent_mass_per_bed: Option<Mass>,
    /// The PSA bed diameter.
    pub bed_diameter: Option<Length>,
    /// The PSA bed length.
    pub bed_length: Option<Length>,
    /// The ejector type.
    pub ejector_type: Option<String>,
    /// The ejector's suction pressure, mbar.
    pub suction_pressure: Option<f64>,
    /// The ejector's suction capacity.
    pub suction_capacity: Option<MassRate>,
    /// Whether the ejector's intercondensers are costed.
    pub include_intercondensers: Option<bool>,
    /// Whether the ejector's aftercondenser is costed.
    pub include_aftercondenser: Option<bool>,
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
        if !matches!(
            kind,
            "separator"
                | "column"
                | "absorber"
                | "reactor"
                | "compressor"
                | "pump"
                | "expander"
                | "heat_exchanger"
                | "tank"
                | "filter"
                | "manifold"
                | "mercury_removal"
                | "electrolyzer"
                | "psa"
                | "ejector"
        ) {
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
        let design_bara = match row.design_pressure_bara {
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
            "reactor" => {
                let diameter = finite(
                    format!("equipment[{index}].vessel_diameter"),
                    require(index, kind, "vessel_diameter", row.vessel_diameter)?.value,
                )?;
                let length = finite(
                    format!("equipment[{index}].vessel_length"),
                    require(index, kind, "vessel_length", row.vessel_length)?.value,
                )?;
                let catalyst = match row.catalyst_mass {
                    Some(value) => {
                        finite(format!("equipment[{index}].catalyst_mass"), value.value)?
                    }
                    None => 0.0,
                };
                let beds = match row.number_of_beds {
                    Some(value) => whole(index, "number_of_beds", value)?,
                    None => 1.0,
                };
                let equipped = match row.total_equipped_weight {
                    Some(value) => finite(
                        format!("equipment[{index}].total_equipped_weight"),
                        value.value,
                    )?,
                    None => 0.0,
                };
                let catalyst_cost = finite(
                    format!("equipment[{index}].catalyst_cost_per_kg"),
                    row.catalyst_cost_per_kg.unwrap_or(8.0),
                )?;
                let internals = finite(
                    format!("equipment[{index}].internals_fraction"),
                    row.internals_fraction.unwrap_or(0.45),
                )?;
                let allowance = finite(
                    format!("equipment[{index}].bed_allowance"),
                    row.bed_allowance.unwrap_or(25000.0),
                )?;
                costing::EquipmentSpec::Reactor(costing::ReactorSpec {
                    vessel_diameter_m: diameter,
                    vessel_length_m: length,
                    design_pressure_bara: design_bara.unwrap_or(0.0),
                    catalyst_mass_kg: catalyst,
                    number_of_beds: beds,
                    total_equipped_weight_kg: equipped,
                    catalyst_cost_per_kg: catalyst_cost,
                    internals_fraction: internals,
                    bed_allowance_usd: allowance,
                })
            }
            "compressor" => {
                let power = match row.power {
                    Some(value) => {
                        finite(format!("equipment[{index}].power"), value.value / 1.0e3)?
                    }
                    None => 0.0,
                };
                let stages = match row.number_of_stages {
                    Some(value) => whole(index, "number_of_stages", value)?,
                    None => 1.0,
                };
                costing::EquipmentSpec::Compressor(costing::CompressorSpec {
                    power_kw: power,
                    compressor_type: row
                        .compressor_type
                        .clone()
                        .unwrap_or_else(|| "centrifugal".to_string()),
                    driver_type: row
                        .driver_type
                        .clone()
                        .unwrap_or_else(|| "electric-motor".to_string()),
                    include_driver: row.include_driver.unwrap_or(true),
                    number_of_stages: stages,
                    include_intercoolers: row.include_intercoolers.unwrap_or(false),
                })
            }
            "pump" => {
                let power = match row.power {
                    Some(value) => {
                        finite(format!("equipment[{index}].power"), value.value / 1.0e3)?
                    }
                    None => 0.0,
                };
                costing::EquipmentSpec::Pump(costing::PumpSpec {
                    power_kw: power,
                    pump_type: row
                        .pump_type
                        .clone()
                        .unwrap_or_else(|| "centrifugal".to_string()),
                    include_motor: row.include_motor.unwrap_or(true),
                    api_rated: row.api_rated.unwrap_or(false),
                    seal_type: row
                        .seal_type
                        .clone()
                        .unwrap_or_else(|| "single-mechanical".to_string()),
                })
            }
            "expander" => {
                let power = match row.shaft_power {
                    Some(value) => finite(
                        format!("equipment[{index}].shaft_power"),
                        value.value / 1.0e3,
                    )?,
                    None => 0.0,
                };
                costing::EquipmentSpec::Expander(costing::ExpanderSpec {
                    shaft_power_kw: power,
                    expander_type: row
                        .expander_type
                        .clone()
                        .unwrap_or_else(|| "radial-inflow".to_string()),
                    load_type: row
                        .load_type
                        .clone()
                        .unwrap_or_else(|| "generator".to_string()),
                    include_load: row.include_load.unwrap_or(true),
                    include_gearbox: row.include_gearbox.unwrap_or(false),
                    include_lube_oil_system: row.include_lube_oil_system.unwrap_or(true),
                    include_control_system: row.include_control_system.unwrap_or(true),
                    cryogenic_service: row.cryogenic_service.unwrap_or(false),
                })
            }
            "heat_exchanger" => {
                let area = match row.area {
                    Some(value) => finite(format!("equipment[{index}].area"), value.value)?,
                    None => 0.0,
                };
                let duty = match row.duty {
                    Some(value) => finite(format!("equipment[{index}].duty"), value.value / 1.0e3)?,
                    None => 0.0,
                };
                let shell_passes = finite(
                    format!("equipment[{index}].shell_passes"),
                    row.shell_passes.unwrap_or(1.0),
                )?;
                costing::EquipmentSpec::HeatExchanger(costing::HeatExchangerSpec {
                    area_m2: area,
                    duty_kw: duty,
                    exchanger_type: row
                        .exchanger_type
                        .clone()
                        .unwrap_or_else(|| "shell-tube".to_string()),
                    tema_type: row.tema_type.clone().unwrap_or_else(|| "AES".to_string()),
                    shell_passes,
                })
            }
            "tank" => {
                let volume = match row.volume {
                    Some(value) => finite(format!("equipment[{index}].volume"), value.value)?,
                    None => 0.0,
                };
                let diameter = match row.diameter {
                    Some(value) => finite(format!("equipment[{index}].diameter"), value.value)?,
                    None => 0.0,
                };
                let height = match row.height {
                    Some(value) => finite(format!("equipment[{index}].height"), value.value)?,
                    None => 0.0,
                };
                let insulation_thickness = match row.insulation_thickness {
                    Some(value) => finite(
                        format!("equipment[{index}].insulation_thickness"),
                        value.value * 1.0e3,
                    )?,
                    None => 100.0,
                };
                costing::EquipmentSpec::Tank(costing::TankSpec {
                    tank_type: row
                        .tank_type
                        .clone()
                        .unwrap_or_else(|| "fixed-cone-roof".to_string()),
                    volume_m3: volume,
                    diameter_m: diameter,
                    height_m: height,
                    design_pressure_barg: design_barg.unwrap_or(0.0),
                    floating_roof: row.floating_roof.unwrap_or(false),
                    include_foundation: row.include_foundation.unwrap_or(true),
                    include_heating_coils: row.include_heating_coils.unwrap_or(false),
                    include_insulation: row.include_insulation.unwrap_or(false),
                    insulation_thickness_mm: insulation_thickness,
                })
            }
            "filter" => {
                let inner_diameter = match row.inner_diameter {
                    Some(value) => {
                        finite(format!("equipment[{index}].inner_diameter"), value.value)?
                    }
                    None => 0.0,
                };
                let vessel_length = match row.tangent_length {
                    Some(value) => {
                        finite(format!("equipment[{index}].tangent_length"), value.value)?
                    }
                    None => 0.0,
                };
                let elements = match row.required_elements {
                    Some(value) => whole(index, "required_elements", value)?,
                    None => 0.0,
                };
                costing::EquipmentSpec::Filter(costing::FilterSpec {
                    inner_diameter_m: inner_diameter,
                    vessel_length_m: vessel_length,
                    required_elements: elements,
                    design_pressure_bara: design_bara.unwrap_or(0.0),
                    material_grade: row.material_grade.clone().unwrap_or_default(),
                    empty_vessel_weight_kg: mass(row.empty_vessel_weight),
                    element_cost_usd: finite(
                        format!("equipment[{index}].element_cost"),
                        row.element_cost.unwrap_or(450.0),
                    )?,
                    auxiliaries_fraction: finite(
                        format!("equipment[{index}].auxiliaries_fraction"),
                        row.auxiliaries_fraction.unwrap_or(0.35),
                    )?,
                    fallback_steel_rate_usd_per_kg: finite(
                        format!("equipment[{index}].fallback_steel_rate"),
                        row.fallback_steel_rate.unwrap_or(6.0),
                    )?,
                    fallback_element_cost_usd: finite(
                        format!("equipment[{index}].fallback_element_cost"),
                        row.fallback_element_cost.unwrap_or(2500.0),
                    )?,
                })
            }
            "manifold" => {
                let inlets = match row.inlets {
                    Some(value) => whole(index, "inlets", value)?,
                    None => 0.0,
                };
                let outlets = match row.outlets {
                    Some(value) => whole(index, "outlets", value)?,
                    None => 0.0,
                };
                let length = |value: Option<Length>, field: &'static str| -> Result<f64> {
                    match value {
                        Some(value) => finite(format!("equipment[{index}].{field}"), value.value),
                        None => Ok(0.0),
                    }
                };
                costing::EquipmentSpec::Manifold(costing::ManifoldSpec {
                    inlets,
                    outlets,
                    weight_kg: weight,
                    location: row
                        .location
                        .clone()
                        .unwrap_or_else(|| "topside".to_string()),
                    material_grade: row.material_grade.clone().unwrap_or_default(),
                    manifold_type: row
                        .manifold_type
                        .clone()
                        .unwrap_or_else(|| "production".to_string()),
                    header_diameter_m: length(row.header_diameter, "header_diameter")?,
                    branch_diameter_m: length(row.branch_diameter, "branch_diameter")?,
                    water_depth_m: length(row.water_depth, "water_depth")?,
                    base_cost_per_kg: finite(
                        format!("equipment[{index}].base_cost_per_kg"),
                        row.base_cost_per_kg.unwrap_or(65.0),
                    )?,
                    branch_allowance_usd: finite(
                        format!("equipment[{index}].branch_allowance"),
                        row.branch_allowance.unwrap_or(18000.0),
                    )?,
                })
            }
            "mercury_removal" => {
                costing::EquipmentSpec::MercuryRemoval(costing::MercuryRemovalSpec {
                    vessel_shell_weight_kg: mass(row.shell_weight),
                    internals_weight_kg: mass(row.internals_weight),
                    nozzle_weight_kg: mass(row.nozzle_weight),
                    sorbent_charge_weight_kg: mass(row.sorbent_charge_weight),
                    sorbent_unit_price_usd_per_kg: finite(
                        format!("equipment[{index}].sorbent_unit_price"),
                        row.sorbent_unit_price.unwrap_or(25.0),
                    )?,
                    steel_cost_per_kg: finite(
                        format!("equipment[{index}].steel_cost_per_kg"),
                        row.steel_cost_per_kg.unwrap_or(8.0),
                    )?,
                    bed_lifetime_years: 5.0,
                })
            }
            "electrolyzer" => {
                let power = match row.power {
                    Some(value) => {
                        finite(format!("equipment[{index}].power"), value.value / 1.0e3)?
                    }
                    None => 0.0,
                };
                costing::EquipmentSpec::Electrolyzer(costing::ElectrolyzerSpec {
                    power_kw: power,
                    technology: row.technology.clone().unwrap_or_else(|| "PEM".to_string()),
                    include_balance_of_plant: row.include_balance_of_plant.unwrap_or(true),
                })
            }
            "psa" => {
                let beds = whole(
                    index,
                    "number_of_beds",
                    require(index, kind, "number_of_beds", row.number_of_beds)?,
                )?;
                let bed_diameter = match row.bed_diameter {
                    Some(value) => Some(finite(
                        format!("equipment[{index}].bed_diameter"),
                        value.value,
                    )?),
                    None => None,
                };
                let bed_length = match row.bed_length {
                    Some(value) => Some(finite(
                        format!("equipment[{index}].bed_length"),
                        value.value,
                    )?),
                    None => None,
                };
                costing::EquipmentSpec::Psa(costing::PsaSpec {
                    number_of_beds: beds,
                    sorbent: row
                        .sorbent
                        .clone()
                        .unwrap_or_else(|| "ACTIVATED_CARBON".to_string()),
                    sorbent_mass_per_bed_kg: mass(row.sorbent_mass_per_bed),
                    include_balance_of_plant: row.include_balance_of_plant.unwrap_or(true),
                    bed_diameter_m: bed_diameter,
                    bed_length_m: bed_length,
                })
            }
            "ejector" => {
                let stages = match row.number_of_stages {
                    Some(value) => whole(index, "number_of_stages", value)?,
                    None => 1.0,
                };
                let capacity = match row.suction_capacity {
                    Some(value) => finite(
                        format!("equipment[{index}].suction_capacity"),
                        value.value * 3600.0,
                    )?,
                    None => 0.0,
                };
                costing::EquipmentSpec::Ejector(costing::EjectorSpec {
                    ejector_type: row
                        .ejector_type
                        .clone()
                        .unwrap_or_else(|| "steam".to_string()),
                    number_of_stages: stages,
                    suction_pressure_mbar: finite(
                        format!("equipment[{index}].suction_pressure"),
                        row.suction_pressure.unwrap_or(100.0),
                    )?,
                    discharge_pressure_bara: design_bara.unwrap_or(1.013),
                    suction_capacity_kg_per_h: capacity,
                    include_intercondensers: row.include_intercondensers.unwrap_or(false),
                    include_aftercondenser: row.include_aftercondenser.unwrap_or(true),
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
