//! The shared cost arithmetic: factor tables, correlations, module stack and the
//! roll-ups, held to their own exact identities before any id is registered.

use azoth_process::costing::correlations::{
    centrifugal_compressor, centrifugal_pump, horizontal_vessel_by_volume, installation_man_hours,
    piping_cost, vertical_vessel_by_volume, vessel_bom,
};
use azoth_process::costing::equipment::{
    Common, EquipmentSpec, Kind, OpexInputs, SeparatorSpec, ValveSpec, WellSpec, evaluate,
    weight_bom,
};
use azoth_process::costing::factors::{
    CEPCI_2019, CEPCI_2025, cepci_for_year, cepci_ratio, exchange_rate, location_factor,
    material_factor, pressure_factor,
};
use azoth_process::costing::module_stack::{
    bare_module, calculator_summary, equipment_stack, grass_roots, total_module,
};
use azoth_process::costing::process::{
    EquipmentRow, ProcessCostInput, classify_equipment, estimate,
};
use azoth_process::costing::topsides::{
    FacilityType, ProjectContext, TopsidesEquipmentRow, TopsidesFactors, TopsidesInput,
    estimate as estimate_topsides,
};

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-9 * expected.abs().max(1.0)
}

fn common(material: &str, design_present: bool) -> Common {
    Common {
        material_factor: material_factor(material).unwrap(),
        cepci: CEPCI_2025,
        design_present,
        max_design_pressure_barg: 10.0,
        weight_total_kg: 5000.0,
        contingency: 0.15,
        engineering: 0.10,
    }
}

#[test]
fn factor_tables_are_the_classes() {
    assert_eq!(pressure_factor(5.0), 1.0);
    assert_eq!(pressure_factor(5.0001), 1.05);
    assert_eq!(pressure_factor(20.0), 1.15);
    assert_eq!(pressure_factor(50.0), 1.30);
    assert_eq!(pressure_factor(200.0), 1.90);
    assert_eq!(pressure_factor(201.0), 2.5);

    assert_eq!(material_factor("Carbon Steel"), Some(1.0));
    assert_eq!(material_factor("SS316"), Some(2.1));
    assert_eq!(material_factor("titanium"), Some(4.5));
    assert_eq!(material_factor("unobtainium"), None);

    assert_eq!(location_factor("US Gulf Coast"), 1.0);
    assert_eq!(location_factor("North Sea / Norway"), 1.35);
    assert_eq!(location_factor("China"), 0.75);
    assert_eq!(location_factor("Middle East"), 1.10);

    assert_eq!(exchange_rate("USD"), Some(1.0));
    assert_eq!(exchange_rate("EUR"), Some(0.92));
    assert_eq!(exchange_rate("NOK"), Some(11.0));
    assert_eq!(exchange_rate("XXX"), None);

    assert_eq!(cepci_for_year(2019), Some(607.5));
    assert_eq!(cepci_for_year(2025), Some(840.0));
    assert_eq!(cepci_for_year(2018), None);
    assert_eq!(cepci_ratio(CEPCI_2019), 1.0);
}

#[test]
fn vessel_correlations_hold_their_clamps() {
    let ratio = 1.0;
    let low = vertical_vessel_by_volume(0.1, ratio);
    let at_min = vertical_vessel_by_volume(0.3, ratio);
    assert!(close(low, at_min));
    let high = vertical_vessel_by_volume(600.0, ratio);
    let at_max = vertical_vessel_by_volume(520.0, ratio);
    assert!(close(high, at_max));

    let horizontal = horizontal_vessel_by_volume(100.0, 1.0);
    let expected =
        10.0_f64.powf(3.5565 + 0.3776 * 100.0_f64.log10() + 0.0905 * 100.0_f64.log10().powi(2));
    assert!(close(horizontal, expected));
}

#[test]
fn parallel_units_split_a_large_duty() {
    assert!(close(
        centrifugal_compressor(6000.0, 1.0),
        2.0 * centrifugal_compressor(3000.0, 1.0)
    ));
    assert!(close(
        centrifugal_pump(600.0, 1.0),
        2.0 * centrifugal_pump(300.0, 1.0)
    ));
    assert_eq!(centrifugal_pump(0.0, 1.0), 0.0);
}

#[test]
fn piping_cost_carries_the_material_factor_inside() {
    let base = piping_cost(0.2032, 100.0, 40, 1.0, 1.0);
    let stainless = piping_cost(0.2032, 100.0, 40, 2.1, 1.0);
    assert!(close(stainless, base * 2.1));
    let schedule_80 = piping_cost(0.2032, 100.0, 80, 1.0, 1.0);
    assert!(close(schedule_80, base * 1.5));
    let schedule_160 = piping_cost(0.2032, 100.0, 160, 1.0, 1.0);
    assert!(close(schedule_160, base * 2.5));
}

#[test]
fn the_module_stack_is_the_classes() {
    let bmc = bare_module(1000.0, 1.05, 2.1);
    assert!(close(bmc, 1000.0 * (1.89 + 1.35 * 1.05 * 2.1)));
    let tmc = total_module(bmc, 0.15, 0.10);
    assert!(close(tmc, bmc * 1.25));
    assert!(close(grass_roots(tmc), tmc * 1.5));

    let stack = equipment_stack(
        1000.0,
        &azoth_process::costing::module_stack::StackInput {
            design_present: true,
            max_design_pressure_barg: 10.0,
            weight_kg: 5000.0,
            equipment_type: "vessel",
            material_factor: 2.1,
            contingency: 0.15,
            engineering: 0.10,
        },
    );
    assert!(close(stack.bare_module_cost, bmc));
    assert!(close(stack.total_module_cost, tmc));
    assert!(close(stack.grass_roots_cost, tmc * 1.5));
    assert!(close(stack.installation_man_hours, 150.0));

    let no_design = equipment_stack(
        1000.0,
        &azoth_process::costing::module_stack::StackInput {
            design_present: false,
            max_design_pressure_barg: 10.0,
            weight_kg: 5000.0,
            equipment_type: "vessel",
            material_factor: 2.1,
            contingency: 0.15,
            engineering: 0.10,
        },
    );
    assert_eq!(no_design.purchased_equipment_cost, 1000.0);
    assert_eq!(no_design.bare_module_cost, 0.0);
    assert_eq!(no_design.installation_man_hours, 0.0);

    let summary = calculator_summary(
        1000.0,
        &azoth_process::costing::module_stack::SummaryInput {
            design_pressure_barg: 10.0,
            weight_kg: 5000.0,
            equipment_type: "vessel",
            material_factor: 2.1,
            location_factor: 1.35,
            contingency: 0.15,
            engineering: 0.10,
        },
    );
    let pec = 1000.0 * 2.1 * 1.35;
    assert!(close(summary.purchased_equipment_cost, pec));
    assert!(close(
        summary.bare_module_cost,
        pec * (1.89 + 1.35 * 1.05 * 1.0)
    ));
}

#[test]
fn man_hours_and_the_vessel_bom_are_the_classes() {
    assert_eq!(installation_man_hours(5000.0, "vessel"), 150.0);
    assert_eq!(installation_man_hours(5000.0, "exchanger"), 200.0);
    assert_eq!(installation_man_hours(5000.0, "pump"), 250.0);
    assert_eq!(installation_man_hours(5000.0, "compressor"), 300.0);
    assert_eq!(installation_man_hours(5000.0, "column"), 175.0);

    let rows = vessel_bom(1000.0, 200.0, 4.0, 300.0, "Carbon Steel", 2.1, 1.0);
    assert_eq!(rows.len(), 4);
    assert!(close(rows[1].cost_usd, 200.0 * 8.0 * 2.1));
    assert!(close(rows[2].cost_usd, 500.0 * 4.0 * 2.1));
    assert!(close(rows[3].cost_usd, 300.0 * 15.0 * 2.1));
}

#[test]
fn the_column_material_factor_is_applied_twice() {
    let column = |material: &str| {
        let spec = EquipmentSpec::Column(azoth_process::costing::equipment::ColumnSpec {
            column_type: "trayed".to_string(),
            tray_type: "sieve".to_string(),
            packing_type: "metal".to_string(),
            diameter_m: 2.0,
            height_m: 20.0,
            number_of_trays: 30.0,
            packing_height_m: 0.0,
            design_pressure_barg: 15.0,
            include_reboiler: true,
            include_condenser: true,
            reboiler_duty_kw: 2000.0,
            condenser_duty_kw: 1500.0,
        });
        evaluate(&spec, &common(material, true))
    };
    let carbon = column("Carbon Steel");
    let stainless = column("SS316");
    assert!(close(
        stainless.stack.purchased_equipment_cost,
        carbon.stack.purchased_equipment_cost * 2.1
    ));
    assert!(close(
        stainless.stack.bare_module_cost / stainless.stack.purchased_equipment_cost,
        1.89 + 1.35 * 1.05 * 2.1
    ));
}

#[test]
fn a_control_valve_carries_the_material_factor_three_times() {
    let valve = |material: &str| {
        let spec = EquipmentSpec::Valve(ValveSpec {
            valve_type: "control".to_string(),
            nominal_size_in: 4.0,
            valve_cv: 150.0,
            pressure_class: 600.0,
            include_actuator: false,
            actuator_type: "pneumatic".to_string(),
        });
        evaluate(&spec, &common(material, true))
    };
    let carbon = valve("Carbon Steel");
    let stainless = valve("SS316");
    assert!(close(
        stainless.stack.purchased_equipment_cost,
        carbon.stack.purchased_equipment_cost * 2.1 * 2.1
    ));
}

#[test]
fn a_zero_capacity_answers_zero_and_a_well_is_all_in() {
    let separator = EquipmentSpec::Separator(SeparatorSpec {
        inner_diameter_m: 0.0,
        tangent_length_m: 0.0,
        vessel_volume_m3: None,
    });
    let result = evaluate(&separator, &common("Carbon Steel", true));
    assert_eq!(result.stack.purchased_equipment_cost, 0.0);
    assert_eq!(result.total_cost, 0.0);

    let well = EquipmentSpec::Well(WellSpec {
        well_capex_usd: 80.0e6,
    });
    let result = evaluate(&well, &common("Carbon Steel", false));
    assert_eq!(result.stack.purchased_equipment_cost, 80.0e6);
    assert_eq!(result.stack.bare_module_cost, 80.0e6);
    assert_eq!(result.stack.total_module_cost, 80.0e6);
    assert_eq!(result.stack.grass_roots_cost, 80.0e6);
    assert_eq!(result.total_cost, 80.0e6);
    assert_eq!(result.stack.installation_man_hours, 0.0);
    assert_eq!(result.kind, Kind::Well);
}

#[test]
fn the_weight_bom_is_the_base_class() {
    let rows = weight_bom(&azoth_process::costing::equipment::WeightBreakdown {
        shell_kg: 1000.0,
        internals_kg: 100.0,
        nozzle_kg: 50.0,
        piping_kg: 200.0,
        electro_instrument_kg: 10.0,
        structural_kg: 400.0,
        material: "Carbon Steel".to_string(),
        cost_per_weight_unit: 8.0,
    });
    assert_eq!(rows.len(), 6);
    assert!(close(rows[0].cost_usd, 8000.0));
    assert!(close(rows[1].cost_usd, 100.0 * 8.0 * 1.5));
    assert!(close(rows[5].cost_usd, 400.0 * 8.0 * 0.8));
}

#[test]
fn the_process_roll_up_is_the_classes() {
    let rows = vec![
        EquipmentRow {
            name: "InletSeparator".to_string(),
            equipment_class: "Separator".to_string(),
            purchased_equipment_cost: 100_000.0,
            bare_module_cost: 300_000.0,
            total_module_cost: 375_000.0,
            grass_roots_cost: 562_500.0,
            total_cost: 375_000.0,
            installation_man_hours: 100.0,
            weight: 5000.0,
            power_kw: None,
            duty_kw: None,
        },
        EquipmentRow {
            name: "MainCompressor".to_string(),
            equipment_class: "Compressor".to_string(),
            purchased_equipment_cost: 200_000.0,
            bare_module_cost: 600_000.0,
            total_module_cost: 750_000.0,
            grass_roots_cost: 1_125_000.0,
            total_cost: 750_000.0,
            installation_man_hours: 200.0,
            weight: 10_000.0,
            power_kw: Some(1000.0),
            duty_kw: None,
        },
    ];
    let input = ProcessCostInput {
        equipment: rows,
        location_factor: 1.35,
        complexity_factor: 1.2,
        opex: Some(OpexInputs {
            hours_per_year: 8000.0,
            electricity_cost_per_kwh: 0.10,
            steam_cost_per_tonne: 15.0,
            cooling_water_cost_per_m3: 0.05,
            fuel_cost_per_gj: 5.0,
        }),
        annual_revenue: Some(5_000_000.0),
        ..ProcessCostInput::default()
    };
    let result = estimate(&input);
    assert!(close(
        result.total_purchased_equipment_cost,
        300_000.0 * 1.35
    ));
    assert!(close(result.total_module_cost, 1_125_000.0 * 1.35));
    assert!(close(
        result.total_grass_roots_cost,
        (1_125_000.0 * 1.35 * 1.15) * 1.2
    ));
    assert!(close(result.total_installation_man_hours, 300.0));
    assert_eq!(result.cost_by_equipment_type.len(), 2);
    assert!(close(
        result.cost_by_discipline[0].1,
        result.total_purchased_equipment_cost
    ));
    let project = &result.project;
    assert!(close(
        project.total_project_cost,
        project.direct_installed_cost
            + project.indirect_cost
            + project.project_contingency
            + project.owner_cost
    ));
    assert_eq!(classify_equipment("ThreePhaseSeparator"), "Vessels");
    assert_eq!(classify_equipment("HeatExchanger"), "Heat Exchangers");
    let operating = result.operating_costs.as_ref().unwrap();
    assert!(close(operating.electricity, 1000.0 * 8000.0 * 0.10));
    assert!(close(
        operating.maintenance,
        result.total_module_cost * 0.04
    ));
    assert!(result.payback_period_years.is_some());
    assert!(result.npv.is_some());
}

#[test]
fn the_topsides_stack_is_the_classes() {
    let input = TopsidesInput {
        equipment: vec![
            TopsidesEquipmentRow {
                name: "InletSeparator".to_string(),
                equipment_type: "Vessels".to_string(),
                total_module_cost: 100_000.0,
                weight: 5000.0,
            },
            TopsidesEquipmentRow {
                name: "Well1".to_string(),
                equipment_type: "Wells".to_string(),
                total_module_cost: 80_000_000.0,
                weight: 0.0,
            },
        ],
        facility_type: FacilityType::FixedPlatform,
        project_context: ProjectContext::Greenfield,
        location_factor: 1.35,
        factors: TopsidesFactors::default(),
    };
    let result = estimate_topsides(&input);
    assert!(close(result.capital_costs[0].1, 135_000.0));
    assert!(close(result.capital_cost_summary[1].1, 80_000_000.0 * 1.35));
    let breakdown = &result.capital_cost_breakdown;
    assert!(
        breakdown
            .iter()
            .any(|(key, _)| key == "module.separationAndTreatmentModules")
    );
    assert_eq!(result.material_take_off.len(), 1 + 4);
}
