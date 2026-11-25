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
        vessel_diameter: None,
        vessel_length: None,
        catalyst_mass: None,
        number_of_beds: None,
        total_equipped_weight: None,
        catalyst_cost_per_kg: None,
        internals_fraction: None,
        bed_allowance: None,
        power: None,
        compressor_type: None,
        driver_type: None,
        include_driver: None,
        include_intercoolers: None,
        pump_type: None,
        include_motor: None,
        api_rated: None,
        seal_type: None,
        shaft_power: None,
        expander_type: None,
        load_type: None,
        include_load: None,
        include_gearbox: None,
        include_lube_oil_system: None,
        include_control_system: None,
        cryogenic_service: None,
        area: None,
        duty: None,
        exchanger_type: None,
        tema_type: None,
        shell_passes: None,
        tank_type: None,
        volume: None,
        floating_roof: None,
        include_foundation: None,
        include_heating_coils: None,
        include_insulation: None,
        insulation_thickness: None,
        required_elements: None,
        material_grade: None,
        empty_vessel_weight: None,
        element_cost: None,
        auxiliaries_fraction: None,
        fallback_steel_rate: None,
        fallback_element_cost: None,
        inlets: None,
        outlets: None,
        location: None,
        manifold_type: None,
        header_diameter: None,
        branch_diameter: None,
        water_depth: None,
        base_cost_per_kg: None,
        branch_allowance: None,
        sorbent_charge_weight: None,
        sorbent_unit_price: None,
        steel_cost_per_kg: None,
        technology: None,
        include_balance_of_plant: None,
        sorbent: None,
        sorbent_mass_per_bed: None,
        bed_diameter: None,
        bed_length: None,
        ejector_type: None,
        suction_pressure: None,
        suction_capacity: None,
        include_intercondensers: None,
        include_aftercondenser: None,
        nominal_diameter_in: None,
        length: None,
        schedule: None,
        installation_type: None,
        include_fittings: None,
        fittings_per_hundred_m: None,
        number_of_flanges: None,
        valve_type: None,
        nominal_size_in: None,
        valve_cv: None,
        pressure_class: None,
        include_actuator: None,
        actuator_type: None,
        mixer_type: None,
        pipe_diameter_in: None,
        number_of_elements: None,
        flanged_connections: None,
        splitter_type: None,
        number_of_outlets: None,
        inlet_diameter_in: None,
        outlet_diameter_in: None,
        include_control_valves: None,
        include_flow_meters: None,
        well_capex: None,
        core_weight: None,
        total_weight: None,
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
    assert!(close(row.purchased_equipment_cost, 4823.880926503424));
    assert!(close(row.bare_module_cost, 19862.329714877847));
    assert!(close(row.total_module_cost, 24827.91214359731));
    assert!(close(row.grass_roots_cost, 37241.86821539597));
    assert!(close(row.total_cost, 24827.91214359731));
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

#[test]
fn the_machines_and_reactor_match_the_capture() {
    let mut reactor = row("reactor");
    reactor.vessel_diameter = Some(meters(0.5));
    reactor.vessel_length = Some(meters(2.0));
    reactor.design_pressure_bara = Some(Pressure::new::<bar>(110.00000000000001));
    reactor.catalyst_mass = Some(Mass::new::<kilogram>(157.0102926847301));
    reactor.number_of_beds = Some(1.0);
    reactor.total_equipped_weight = Some(Mass::new::<kilogram>(839.1623792871716));
    reactor.mechanical_design_present = Some(true);
    reactor.max_design_pressure = Some(Pressure::new::<bar>(110.00000000000001));
    reactor.weight_total = Some(Mass::new::<kilogram>(839.1623792871716));

    let mut compressor = row("compressor");
    compressor.power = Some(Power::new::<kilowatt>(1314.898215238133));
    compressor.mechanical_design_present = Some(true);
    compressor.max_design_pressure = Some(Pressure::new::<bar>(110.00000000000001));
    compressor.weight_total = Some(Mass::new::<kilogram>(702848.7190347943));

    let mut pump = row("pump");
    pump.power = Some(Power::new::<kilowatt>(75.0));
    pump.mechanical_design_present = Some(true);
    pump.max_design_pressure = Some(Pressure::new::<bar>(110.00000000000001));
    pump.weight_total = Some(Mass::new::<kilogram>(753.7347107302816));

    let mut expander = row("expander");
    expander.shaft_power = Some(Power::new::<kilowatt>(2000.0));
    expander.expander_type = Some("radial-inflow".to_string());
    expander.load_type = Some("generator".to_string());

    let result = run(vec![reactor, compressor, pump, expander]);
    assert!(close(
        result.equipment[0].purchased_equipment_cost,
        32507.612368330385
    ));
    assert!(close(
        result.equipment[0].bare_module_cost,
        133850.09392660036
    ));
    assert!(close(
        result.equipment[0].installation_man_hours,
        29.370683275051004 * 3600.0
    ));
    assert!(close(
        result.equipment[1].purchased_equipment_cost,
        566585.9790593369
    ));
    assert!(close(
        result.equipment[1].installation_man_hours,
        42170.92314208765 * 3600.0
    ));
    assert!(close(
        result.equipment[2].purchased_equipment_cost,
        22458.556775479567
    ));
    assert!(close(
        result.equipment[3].purchased_equipment_cost,
        1464123.3831943076
    ));
    assert_eq!(result.equipment[3].bare_module_cost, 0.0);
    assert_eq!(result.equipment[3].total_cost, 0.0);
}

#[test]
fn the_exchanger_tank_and_filter_match_the_capture() {
    let mut exchanger = row("heat_exchanger");
    exchanger.exchanger_type = Some("shell-tube".to_string());
    exchanger.mechanical_design_present = Some(true);
    exchanger.max_design_pressure = Some(Pressure::new::<bar>(110.00000000000001));
    exchanger.weight_total = Some(Mass::new::<kilogram>(532.6884503426853));

    let mut tank = row("tank");
    tank.tank_type = Some("fixed-cone-roof".to_string());
    tank.volume = Some(Volume::new::<azoth_core::units::cubic_meter>(5000.0));
    tank.diameter = Some(meters(20.0));
    tank.height = Some(meters(16.0));
    tank.include_foundation = Some(true);

    let mut filter = row("filter");
    filter.inner_diameter = Some(meters(0.5));
    filter.tangent_length = Some(meters(1.5));
    filter.required_elements = Some(1.0);
    filter.design_pressure_bara = Some(Pressure::new::<bar>(55.00000000000001));
    filter.material_grade = Some("SA-516-70".to_string());
    filter.empty_vessel_weight = Some(Mass::new::<kilogram>(323.91707924523325));
    filter.mechanical_design_present = Some(true);
    filter.max_design_pressure = Some(Pressure::new::<bar>(55.00000000000001));
    filter.weight_total = Some(Mass::new::<kilogram>(431.0922030188032));

    let result = run(vec![exchanger, tank, filter]);
    assert!(close(
        result.equipment[0].purchased_equipment_cost,
        4261.5076027414825
    ));
    assert!(close(
        result.equipment[0].total_module_cost,
        21933.446942860064
    ));
    assert!(close(
        result.equipment[1].purchased_equipment_cost,
        179613.0916761046
    ));
    assert_eq!(result.equipment[1].total_cost, 0.0);
    assert!(close(
        result.equipment[2].purchased_equipment_cost,
        7651.934577484859
    ));
    assert!(close(
        result.equipment[2].installation_man_hours,
        15.088227105658111 * 3600.0
    ));
}

#[test]
fn the_manifold_mercury_electrolyzer_psa_and_ejector_match_the_capture() {
    let mut manifold = row("manifold");
    manifold.inlets = Some(2.0);
    manifold.outlets = Some(3.0);
    manifold.weight_total = Some(Mass::new::<kilogram>(2653.866258910617));
    manifold.location = Some("topside".to_string());
    manifold.material_grade = Some("A106-B".to_string());
    manifold.manifold_type = Some("production".to_string());
    manifold.header_diameter = Some(meters(0.3048));
    manifold.branch_diameter = Some(meters(0.0));
    manifold.water_depth = Some(meters(0.0));
    manifold.mechanical_design_present = Some(true);
    manifold.max_design_pressure = Some(Pressure::new::<bar>(110.00000000000001));

    let mut mercury = row("mercury_removal");
    mercury.shell_weight = Some(Mass::new::<kilogram>(19496.039027883457));
    mercury.internals_weight = Some(Mass::new::<kilogram>(383.2632545290422));
    mercury.nozzle_weight = Some(Mass::new::<kilogram>(974.8019513941729));
    mercury.sorbent_charge_weight = Some(Mass::new::<kilogram>(4665.265090580843));
    mercury.mechanical_design_present = Some(true);
    mercury.max_design_pressure = Some(Pressure::new::<bar>(110.00000000000001));
    mercury.weight_total = Some(Mass::new::<kilogram>(32161.80686997908));

    let mut electrolyzer = row("electrolyzer");
    electrolyzer.power = Some(Power::new::<kilowatt>(3271.185));
    electrolyzer.technology = Some("PEM".to_string());
    electrolyzer.include_balance_of_plant = Some(true);
    electrolyzer.mechanical_design_present = Some(true);
    electrolyzer.max_design_pressure = Some(Pressure::new::<bar>(33.0));
    electrolyzer.weight_total = Some(Mass::new::<kilogram>(27805.0725));

    let mut psa = row("psa");
    psa.number_of_beds = Some(4.0);
    psa.sorbent = Some("activated_carbon".to_string());
    psa.sorbent_mass_per_bed = Some(Mass::new::<kilogram>(20000.0));
    psa.include_balance_of_plant = Some(true);

    let mut ejector = row("ejector");
    ejector.ejector_type = Some("steam".to_string());
    ejector.number_of_stages = Some(2.0);
    ejector.suction_pressure = Some(50.0);
    ejector.design_pressure_bara = Some(Pressure::new::<bar>(1.013));
    ejector.suction_capacity = Some(azoth_core::units::MassRate::new::<
        azoth_core::units::kilogram_per_hour,
    >(500.0));

    let result = run(vec![manifold, mercury, electrolyzer, psa, ejector]);
    assert!(close(
        result.equipment[0].purchased_equipment_cost,
        298501.3068291901
    ));
    assert!(close(
        result.equipment[0].total_module_cost,
        1536348.9135864878
    ));
    assert!(close(
        result.equipment[1].purchased_equipment_cost,
        283464.4611349745
    ));
    assert!(close(
        result.equipment[1].sorbent_replacement_cost,
        145789.53408065136
    ));
    assert!(close(
        result.equipment[2].purchased_equipment_cost,
        3594170.3093879404
    ));
    assert!(close(result.equipment[2].specific_capex_per_kw, 1250.0));
    assert!(close(
        result.equipment[3].purchased_equipment_cost,
        1638000.0
    ));
    assert!(close(
        result.equipment[4].purchased_equipment_cost,
        150282.87211884474
    ));
    assert_eq!(result.equipment[4].total_cost, 0.0);
}

#[test]
fn the_pipework_states_match_the_capture() {
    let mut pipe = row("pipe");
    pipe.nominal_diameter_in = Some(8.0);
    pipe.length = Some(meters(500.0));
    pipe.schedule = Some("40".to_string());
    pipe.installation_type = Some("above-ground".to_string());
    pipe.include_fittings = Some(true);
    pipe.fittings_per_hundred_m = Some(15.0);
    pipe.number_of_flanges = Some(4.0);
    pipe.include_insulation = Some(false);

    let mut valve = row("valve");
    valve.valve_type = Some("control".to_string());
    valve.nominal_size_in = Some(4.0);
    valve.valve_cv = Some(150.0);
    valve.pressure_class = Some(600.0);
    valve.include_actuator = Some(true);
    valve.actuator_type = Some("pneumatic".to_string());

    let mut mixer = row("mixer");
    mixer.mixer_type = Some("static".to_string());
    mixer.pipe_diameter_in = Some(8.0);
    mixer.number_of_elements = Some(12.0);
    mixer.pressure_class = Some(300.0);
    mixer.flanged_connections = Some(true);

    let mut splitter = row("splitter");
    splitter.splitter_type = Some("manifold".to_string());
    splitter.number_of_outlets = Some(4.0);
    splitter.inlet_diameter_in = Some(10.0);
    splitter.outlet_diameter_in = Some(6.0);
    splitter.pressure_class = Some(600.0);
    splitter.include_control_valves = Some(false);
    splitter.include_flow_meters = Some(false);

    let mut well = row("well");
    well.well_capex = Some(80.0e6);

    let result = run(vec![pipe, valve, mixer, splitter, well]);
    assert!(close(
        result.equipment[0].purchased_equipment_cost,
        795792.2151429062
    ));
    assert!(close(
        result.equipment[1].purchased_equipment_cost,
        82335.92259507756
    ));
    assert!(close(
        result.equipment[2].purchased_equipment_cost,
        13180.74384039505
    ));
    assert!(close(
        result.equipment[3].purchased_equipment_cost,
        4004.770953582229
    ));
    assert_eq!(result.equipment[4].purchased_equipment_cost, 80.0e6);
    assert_eq!(result.equipment[4].bare_module_cost, 80.0e6);
    assert_eq!(result.equipment[4].total_cost, 80.0e6);
    assert_eq!(result.equipment[4].installation_man_hours, 0.0);
}

#[test]
fn the_bahx_matches_the_capture() {
    let mut bahx = row("bahx");
    bahx.core_weight = Some(Mass::new::<kilogram>(7076.159999999999));
    bahx.total_weight = Some(Mass::new::<kilogram>(7277.613908961697));
    bahx.area = Some(azoth_core::units::Area::new::<
        azoth_core::units::square_meter,
    >(2100.673511375631));
    bahx.mechanical_design_present = Some(true);
    bahx.max_design_pressure = Some(Pressure::new::<bar>(55.0));
    bahx.weight_total = Some(Mass::new::<kilogram>(7277.613908961697));

    let result = run(vec![bahx]);
    let row = &result.equipment[0];
    assert!(close(row.purchased_equipment_cost, 596625.9198089183));
    assert!(close(row.bare_module_cost, 2335790.476051915));
    assert!(close(row.total_module_cost, 2919738.095064894));
    assert!(close(
        row.installation_man_hours,
        254.7164868136594 * 3600.0
    ));
    assert!(close(row.equipment_cost, 596625.9198089183));
    assert!(close(row.installed_cost, 2297009.7912643356));
    assert!(close(row.specific_cost_per_m2, 284.01649117678284));
    assert!(close(row.annual_maintenance_cost, 11932.518396178366));
}
