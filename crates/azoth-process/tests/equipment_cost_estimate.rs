//! `process.equipment_cost_estimate` against
//! `validation/neqsim/captures/process_cost_equipment.tsv`.

use azoth_core::units::{Mass, Power, Pressure, Volume, bar, kilogram, kilowatt, meters};
use azoth_process::models::equipment_cost_estimate::{
    EquipmentCostEstimateResult, EquipmentCostSpec, equipment_cost_estimate,
};

fn row(equipment: &str) -> EquipmentCostSpec {
    EquipmentCostSpec {
        equipment: equipment.to_string(),
        name: None,
        material: None,
        material_factor: None,
        cepci: None,
        mechanical_design_present: None,
        max_design_pressure: None,
        weight_total: None,
        design_pressure_barg: None,
        design_pressure_bara: None,
        contingency_factor: None,
        engineering_factor: None,
        include_material_take_off: None,
        shell_weight: None,
        internals_weight: None,
        nozzle_weight: None,
        piping_weight: None,
        electro_instrument_weight: None,
        structural_weight: None,
        operating_hours: None,
        electricity_cost_per_kwh: None,
        steam_cost_per_tonne: None,
        cooling_water_cost_per_m3: None,
        fuel_cost_per_gj: None,
        inner_diameter: None,
        tangent_length: None,
        vessel_volume: None,
        column_type: None,
        tray_type: None,
        packing_type: None,
        diameter: None,
        height: None,
        number_of_trays: None,
        number_of_stages: None,
        packing_height: None,
        include_reboiler: None,
        include_condenser: None,
        include_liquid_distributor: None,
        include_mist_eliminator: None,
        include_reflux_system: None,
        reboiler_duty: None,
        condenser_duty: None,
        absorber_type: None,
    }
}

fn run(rows: Vec<EquipmentCostSpec>) -> EquipmentCostEstimateResult {
    equipment_cost_estimate(&rows).expect("the estimate answers")
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-12 * expected.abs().max(1.0)
}

#[test]
fn a_trayed_column_matches_the_capture() {
    let mut column = row("column");
    column.name = Some("Depropaniser".to_string());
    column.column_type = Some("trayed".to_string());
    column.tray_type = Some("sieve".to_string());
    column.diameter = Some(meters(2.0));
    column.height = Some(meters(25.0));
    column.number_of_trays = Some(30.0);
    column.design_pressure_barg = Some(Pressure::new::<bar>(15.0));
    column.include_reboiler = Some(true);
    column.include_condenser = Some(true);
    column.reboiler_duty = Some(Power::new::<kilowatt>(2000.0));
    column.condenser_duty = Some(Power::new::<kilowatt>(1500.0));

    let result = run(vec![column]);
    let row = &result.equipment[0];
    assert_eq!(row.name, "Depropaniser");
    assert_eq!(row.equipment_type, "column");
    assert!(close(row.purchased_equipment_cost, 186272.12543695548));
    assert_eq!(row.bare_module_cost, 0.0);
    assert_eq!(row.total_cost, 0.0);
    assert_eq!(row.installation_man_hours, 0.0);
    assert!(close(row.material_factor, 1.0));
    assert!(result.material_take_off.is_empty());
    assert!(result.quality_flags.is_empty());
}

#[test]
fn the_stainless_column_carries_the_material_factor_twice() {
    let mut carbon = row("column");
    carbon.diameter = Some(meters(2.0));
    carbon.height = Some(meters(25.0));
    carbon.number_of_trays = Some(30.0);
    carbon.design_pressure_barg = Some(Pressure::new::<bar>(15.0));
    carbon.include_reboiler = Some(true);
    carbon.include_condenser = Some(true);
    carbon.reboiler_duty = Some(Power::new::<kilowatt>(2000.0));
    carbon.condenser_duty = Some(Power::new::<kilowatt>(1500.0));
    let mut stainless = carbon.clone();
    stainless.material = Some("ss316".to_string());

    let result = run(vec![carbon, stainless]);
    assert!(close(
        result.equipment[1].purchased_equipment_cost,
        391171.4634176065
    ));
    assert!(close(result.equipment[1].material_factor, 2.1));
    assert!(close(
        result.equipment[1].purchased_equipment_cost / result.equipment[0].purchased_equipment_cost,
        2.1
    ));
}

#[test]
fn the_packed_column_and_zero_column_match_the_capture() {
    let mut packed = row("column");
    packed.column_type = Some("packed".to_string());
    packed.packing_type = Some("structured".to_string());
    packed.diameter = Some(meters(1.5));
    packed.height = Some(meters(15.0));
    packed.packing_height = Some(meters(10.0));
    packed.design_pressure_barg = Some(Pressure::new::<bar>(15.0));
    packed.include_reboiler = Some(true);
    packed.include_condenser = Some(true);
    packed.reboiler_duty = Some(Power::new::<kilowatt>(1000.0));
    packed.condenser_duty = Some(Power::new::<kilowatt>(800.0));

    let mut zero = row("column");
    zero.diameter = Some(meters(0.0));
    zero.height = Some(meters(0.0));
    zero.include_reboiler = Some(false);
    zero.include_condenser = Some(false);

    let result = run(vec![packed, zero]);
    assert!(close(
        result.equipment[0].purchased_equipment_cost,
        149981.67037473136
    ));
    assert_eq!(result.equipment[1].purchased_equipment_cost, 0.0);
    assert_eq!(result.quality_flags.len(), 1);
    assert_eq!(result.quality_flags[0].equipment_index, 1.0);
}

#[test]
fn the_absorber_states_match_the_capture() {
    let mut packed = row("absorber");
    packed.absorber_type = Some("packed".to_string());
    packed.packing_type = Some("structured".to_string());
    packed.diameter = Some(meters(2.0));
    packed.height = Some(meters(15.0));
    packed.packing_height = Some(meters(10.0));
    packed.design_pressure_barg = Some(Pressure::new::<bar>(60.0));

    let mut trayed = row("absorber");
    trayed.absorber_type = Some("trayed".to_string());
    trayed.tray_type = Some("valve".to_string());
    trayed.diameter = Some(meters(2.0));
    trayed.height = Some(meters(15.0));
    trayed.number_of_stages = Some(12.0);
    trayed.design_pressure_barg = Some(Pressure::new::<bar>(60.0));
    trayed.include_liquid_distributor = Some(false);
    trayed.include_mist_eliminator = Some(false);

    let result = run(vec![packed, trayed]);
    assert!(close(
        result.equipment[0].purchased_equipment_cost,
        155322.29397076587
    ));
    assert!(close(
        result.equipment[1].purchased_equipment_cost,
        62090.37414978155
    ));
}

#[test]
fn the_separator_carries_its_stack_and_bill_of_materials() {
    let mut separator = row("separator");
    separator.name = Some("InletSeparator".to_string());
    separator.inner_diameter = Some(meters(0.8361865022733448));
    separator.tangent_length = Some(meters(1.6723730045466896));
    separator.mechanical_design_present = Some(true);
    separator.max_design_pressure = Some(Pressure::new::<bar>(110.00000000000001));
    separator.weight_total = Some(Mass::new::<kilogram>(3315.9396795551834));
    separator.include_material_take_off = Some(true);
    separator.shell_weight = Some(Mass::new::<kilogram>(2098.6959997184704));
    separator.piping_weight = Some(Mass::new::<kilogram>(839.4783998873882));
    separator.electro_instrument_weight = Some(Mass::new::<kilogram>(167.89567997747764));
    separator.structural_weight = Some(Mass::new::<kilogram>(209.86959997184704));

    let result = run(vec![separator]);
    let row = &result.equipment[0];
    assert!(close(row.purchased_equipment_cost, 4185.028871129008));
    assert!(close(row.bare_module_cost, 17231.85637687369));
    assert!(close(row.total_module_cost, 21539.820471092113));
    assert!(close(row.grass_roots_cost, 32309.73070663817));
    assert!(close(row.total_cost, 21539.820471092113));
    assert!(close(
        row.installation_man_hours,
        99.47819038665551 * 3600.0
    ));
    assert_eq!(result.material_take_off.len(), 4);
    assert!(close(
        result.material_take_off[0].cost_usd,
        16789.567997747763
    ));
    assert_eq!(result.material_take_off[0].item, "Vessel Shell");
    assert!(close(
        result.material_take_off[3].cost_usd,
        1343.1654398198211
    ));
    assert!(result.quality_flags.is_empty());
}

#[test]
fn the_estimate_refuses_what_it_cannot_express() {
    let unknown = row("reactor");
    assert!(equipment_cost_estimate(&[unknown]).is_err());

    let mut no_diameter = row("column");
    no_diameter.height = Some(meters(10.0));
    assert!(equipment_cost_estimate(&[no_diameter]).is_err());

    let no_capacity = row("separator");
    assert!(equipment_cost_estimate(&[no_capacity]).is_err());

    let mut unknown_material = row("column");
    unknown_material.diameter = Some(meters(1.0));
    unknown_material.height = Some(meters(10.0));
    unknown_material.material = Some("unobtainium".to_string());
    assert!(equipment_cost_estimate(&[unknown_material]).is_err());

    let mut bad_cepci = row("column");
    bad_cepci.diameter = Some(meters(1.0));
    bad_cepci.height = Some(meters(10.0));
    bad_cepci.cepci = Some(0.0);
    assert!(equipment_cost_estimate(&[bad_cepci]).is_err());

    let mut fractional = row("column");
    fractional.diameter = Some(meters(1.0));
    fractional.height = Some(meters(10.0));
    fractional.number_of_trays = Some(20.5);
    assert!(equipment_cost_estimate(&[fractional]).is_err());

    let mut infinite = row("separator");
    infinite.vessel_volume = Some(Volume::new::<azoth_core::units::cubic_meter>(f64::INFINITY));
    assert!(equipment_cost_estimate(&[infinite]).is_err());
}
