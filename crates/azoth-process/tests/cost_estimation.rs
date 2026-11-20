//! `process.cost_estimation` against
//! `validation/neqsim/captures/process_cost_calculator.tsv`.

use azoth_core::units::{Mass, Pressure, bar, kilogram};
use azoth_process::models::cost_estimation::{
    CostEstimationQuery, CostEstimationResult, cost_estimation,
};

fn query(calculation: &str) -> CostEstimationQuery {
    CostEstimationQuery {
        calculation: calculation.to_string(),
        correlation: None,
        capacity: None,
        diameter: None,
        tray_count: None,
        packing_type: None,
        piping_length: None,
        piping_schedule: None,
        valve_cv: None,
        purchased_cost: None,
        design_pressure: None,
        pressure_factor: None,
        material_factor: None,
        weight: None,
        equipment_type: None,
        material: None,
        region: None,
        currency: None,
        exchange_rate: None,
        amount: None,
        currency_direction: None,
        shell_weight: None,
        heads_weight: None,
        nozzle_count: None,
        internals_weight: None,
        current_cepci: None,
        reference_cepci: None,
        contingency_factor: None,
        engineering_factor: None,
    }
}

fn run(queries: Vec<CostEstimationQuery>) -> CostEstimationResult {
    cost_estimation(&queries).expect("the calculator answers")
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-12 * expected.abs().max(1.0)
}

#[test]
fn factor_boundaries_match_the_capture() {
    let mut at_five = query("pressure_factor");
    at_five.design_pressure = Some(Pressure::new::<bar>(5.0));
    let mut at_fifty = query("pressure_factor");
    at_fifty.design_pressure = Some(Pressure::new::<bar>(50.0));
    let mut stainless = query("material_factor");
    stainless.material = Some("ss316".to_string());
    let mut north_sea = query("location_factor");
    north_sea.region = Some("north_sea".to_string());
    let mut euros = query("currency_conversion");
    euros.amount = Some(1.0e6);
    euros.currency = Some("eur".to_string());
    let mut piping_labour = query("piping_man_hours");
    piping_labour.weight = Some(Mass::new::<kilogram>(5000.0));

    let result = run(vec![
        at_five,
        at_fifty,
        stainless,
        north_sea,
        euros,
        piping_labour,
    ]);
    let rows = &result.queries;
    assert!(close(rows[0].pressure_factor, 1.0));
    assert!(close(rows[1].pressure_factor, 1.3));
    assert!(close(rows[2].material_factor, 2.1));
    assert!(close(rows[3].location_factor, 1.35));
    assert!(close(rows[4].exchange_rate, 0.92));
    assert!(close(rows[4].converted_amount, 920_000.0));
    assert!(close(rows[4].cost_in_usd, 1_000_000.0));
    assert!(close(rows[5].installation_man_hours, 1_800_000.0));
    for row in rows {
        assert!(close(row.current_cepci, 840.0));
        assert!(close(row.reference_cepci, 607.5));
        assert!(close(row.installation_factor, 3.0));
    }
    assert!(result.bom.is_empty());
}

#[test]
fn correlations_match_the_capture() {
    let mut vessel = query("correlation");
    vessel.correlation = Some("vertical_vessel_by_volume".to_string());
    vessel.capacity = Some(100.0);
    let mut exchanger = query("correlation");
    exchanger.correlation = Some("shell_tube".to_string());
    exchanger.capacity = Some(50.0);
    let mut compressor = query("correlation");
    compressor.correlation = Some("centrifugal_compressor".to_string());
    compressor.capacity = Some(1500.0);
    let mut piping = query("correlation");
    piping.correlation = Some("piping".to_string());
    piping.diameter = Some(azoth_core::units::meters(0.1524));
    piping.piping_length = Some(azoth_core::units::meters(100.0));
    piping.piping_schedule = Some("80".to_string());
    piping.material = Some("ss316".to_string());
    let mut valve = query("correlation");
    valve.correlation = Some("control_valve".to_string());
    valve.valve_cv = Some(150.0);
    valve.material = Some("ss316".to_string());

    let result = run(vec![vessel, exchanger, compressor, piping, valve]);
    let rows = &result.queries;
    assert!(close(rows[0].purchased_equipment_cost, 92200.44216287117));
    assert!(close(rows[1].purchased_equipment_cost, 26442.288629302795));
    assert!(close(rows[2].purchased_equipment_cost, 519033.7321337709));
    assert!(close(rows[3].purchased_equipment_cost, 320066.6597236687));
    assert!(close(rows[3].material_factor, 2.1));
    assert!(close(rows[4].purchased_equipment_cost, 114220.09043285412));
}

#[test]
fn the_module_stack_and_summary_match_the_capture() {
    let mut stack = query("module_stack");
    stack.purchased_cost = Some(100_000.0);
    stack.design_pressure = Some(Pressure::new::<bar>(10.0));
    stack.material = Some("ss316".to_string());
    let mut summary = query("summary");
    summary.purchased_cost = Some(50_000.0);
    summary.design_pressure = Some(Pressure::new::<bar>(50.0));
    summary.weight = Some(Mass::new::<kilogram>(1000.0));
    summary.equipment_type = Some("vessel".to_string());
    let mut labour = query("installation_man_hours");
    labour.weight = Some(Mass::new::<kilogram>(5000.0));
    labour.equipment_type = Some("compressor".to_string());

    let result = run(vec![stack, summary, labour]);
    let rows = &result.queries;
    assert!(close(rows[0].pressure_factor, 1.05));
    assert!(close(rows[0].bare_module_cost, 486675.00000000006));
    assert!(close(rows[0].total_module_cost, 608343.7500000001));
    assert!(close(rows[0].grass_roots_cost, 912515.6250000002));
    assert!(close(rows[1].purchased_equipment_cost, 50_000.0));
    assert!(close(rows[1].bare_module_cost, 182_250.0));
    assert!(close(rows[1].total_module_cost, 227_812.5));
    assert!(close(rows[1].grass_roots_cost, 341_718.75));
    assert!(close(rows[1].installation_man_hours, 108_000.0));
    assert!(close(rows[2].installation_man_hours, 1_080_000.0));
}

#[test]
fn the_vessel_bom_matches_the_capture() {
    let mut bom = query("vessel_bom");
    bom.shell_weight = Some(Mass::new::<kilogram>(1000.0));
    bom.heads_weight = Some(Mass::new::<kilogram>(200.0));
    bom.nozzle_count = Some(4.0);
    bom.internals_weight = Some(Mass::new::<kilogram>(300.0));
    bom.material = Some("carbon_steel".to_string());

    let result = run(vec![bom]);
    let rows = &result.bom;
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].item, "Vessel Shell");
    assert!(close(rows[0].cost_usd, 535037.5394461319));
    assert!(rows[0].has_weight);
    assert!(!rows[0].has_quantity);
    assert!(close(rows[1].cost_usd, 1600.0));
    assert_eq!(rows[2].item, "Nozzles");
    assert!(rows[2].has_quantity);
    assert!(!rows[2].has_weight);
    assert!(close(rows[2].quantity, 4.0));
    assert!(close(rows[3].cost_usd, 4500.0));
}

#[test]
fn the_calculator_refuses_what_the_class_cannot_express() {
    let mut unknown = query("nonsense");
    unknown.capacity = Some(1.0);
    assert!(cost_estimation(&[unknown]).is_err());

    let mut no_capacity = query("correlation");
    no_capacity.correlation = Some("vertical_vessel_by_volume".to_string());
    assert!(cost_estimation(&[no_capacity]).is_err());

    let mut fractional = query("correlation");
    fractional.correlation = Some("sieve_trays".to_string());
    fractional.diameter = Some(azoth_core::units::meters(2.0));
    fractional.tray_count = Some(20.5);
    assert!(cost_estimation(&[fractional]).is_err());

    let mut bad_index = query("material_factor");
    bad_index.current_cepci = Some(0.0);
    assert!(cost_estimation(&[bad_index]).is_err());

    let mut unknown_material = query("material_factor");
    unknown_material.material = Some("unobtainium".to_string());
    assert!(cost_estimation(&[unknown_material]).is_err());

    let mut infinite = query("module_stack");
    infinite.purchased_cost = Some(f64::INFINITY);
    assert!(cost_estimation(&[infinite]).is_err());

    let mut negative = query("module_stack");
    negative.purchased_cost = Some(-1.0);
    assert!(cost_estimation(&[negative]).is_err());
}
