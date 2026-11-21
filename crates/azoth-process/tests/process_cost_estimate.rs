//! `process.process_cost_estimate` against
//! `validation/neqsim/captures/process_cost_process.tsv`.

use azoth_core::units::{Mass, Power, Time, hour, kilogram, kilowatt};
use azoth_process::models::process_cost_estimate::{
    ProcessCostEquipment, ProcessCostEstimateResult, process_cost_estimate,
};

#[allow(clippy::too_many_arguments)]
fn unit(
    name: &str,
    class: &str,
    pec: f64,
    bmc: f64,
    tmc: f64,
    grc: f64,
    man_hours: f64,
    weight: f64,
) -> ProcessCostEquipment {
    ProcessCostEquipment {
        name: name.to_string(),
        equipment_class: class.to_string(),
        purchased_equipment_cost: pec,
        bare_module_cost: bmc,
        total_module_cost: tmc,
        grass_roots_cost: grc,
        total_cost: tmc,
        installation_man_hours: Time::new::<hour>(man_hours),
        weight: Mass::new::<kilogram>(weight),
        power: None,
        duty: None,
    }
}

fn run(rows: Vec<ProcessCostEquipment>) -> ProcessCostEstimateResult {
    process_cost_estimate(
        &rows,
        Some(1.0),
        Some(1.0),
        Some(0.10),
        Some(0.15),
        Some(Mass::new::<kilogram>(601995.8012041049)),
        Some(1000.0),
        Some(Time::new::<hour>(8000.0)),
        Some(0.08),
        Some(15.0),
        Some(0.05),
        Some(50.0),
        Some(5_000_000.0),
        Some(0.10),
        Some(10.0),
    )
    .expect("the roll-up answers")
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-12 * expected.abs().max(1.0)
}

#[test]
fn the_class_process_matches_the_capture() {
    let mut compressor = unit(
        "MainCompressor",
        "Compressor",
        236944.74435086618,
        975619.9848646914,
        1219524.9810808643,
        1829287.4716212964,
        36086.44083680164,
        601440.6806133607,
    );
    compressor.power = Some(Power::new::<kilowatt>(243.39509113104845));
    let mut cooler = unit(
        "AfterCooler",
        "Cooler",
        1056.0,
        4348.08,
        5435.1,
        8152.650000000001,
        5.28,
        132.0,
    );
    cooler.duty = Some(Power::new::<kilowatt>(-339531.601308147));

    let result = run(vec![
        unit("Feed", "Stream", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        unit(
            "InletSeparator",
            "Separator",
            2710.0997797401924,
            11158.835843080242,
            13948.544803850302,
            20922.817205775453,
            10.761660837894372,
            358.7220279298124,
        ),
        compressor,
        cooler,
        unit(
            "JTValve",
            "ThrottlingValve",
            52972.234870373686,
            218113.17707876363,
            272641.47134845454,
            408962.20702268183,
            2.2539496985037624,
            64.3985628143932,
        ),
    ]);
    assert!(close(
        result.total_purchased_equipment_cost,
        293683.07900098007
    ));
    assert!(close(result.total_bare_module_cost, 1209240.0777865353));
    assert!(close(result.total_module_cost, 1511550.0972331692));
    assert!(close(result.total_grass_roots_cost, 1738282.6118181446));
    assert!(close(
        result.total_installation_man_hours,
        36104.73644733804 * 3600.0
    ));
    assert_eq!(result.equipment.len(), 5);
    assert_eq!(result.equipment[1].equipment_type, "Vessels");
    assert!(close(
        result.equipment[2].located_purchased_equipment_cost,
        236944.74435086618
    ));
    assert_eq!(result.cost_by_equipment_type.len(), 5);
    assert_eq!(result.cost_by_discipline.len(), 5);
    assert_eq!(result.project_costs.len(), 5);
    assert!(close(result.total_project_cost, 2172853.2647726806));
    assert!(close(result.operating_costs[0].cost, 155772.858323871));
    assert!(close(result.operating_costs[3].cost, 60462.00388932677));
    assert!(close(
        result.total_annual_operating_cost,
        1851641.1872564566
    ));
    assert!(close(result.payback_period_years, 0.5521234126117187));
    assert!(close(result.roi_percent, 181.118926884423));
    assert!(close(result.npv, 1.7607019385921277e7));
    assert!(close(result.summed_unit_total_cost, 1511550.0972331692));
    assert!(close(result.weight_based_capex, 6.019958012041049e8));
    assert_eq!(result.material_take_off.len(), 5);
    assert!(result.has_operating_cost);
    assert!(result.has_economics);
    assert!(result.has_weight_based_capex);
    assert!(result.quality_flags.is_empty());
}

#[test]
fn an_empty_roll_up_carries_its_flag_and_zero_totals() {
    let result = process_cost_estimate(
        &[],
        None,
        None,
        None,
        None,
        None,
        None,
        Some(Time::new::<hour>(8000.0)),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("the roll-up answers");
    assert_eq!(result.total_purchased_equipment_cost, 0.0);
    assert_eq!(result.equipment.len(), 0);
    assert_eq!(result.quality_flags.len(), 2);
    assert!(result.has_operating_cost);
    assert!(!result.has_economics);
    assert!(!result.has_weight_based_capex);
    assert!(result.payback_period_years == 0.0);
    assert!(close(
        result.total_annual_operating_cost,
        800_000.0 + 144_000.0 + 120_000.0 + 519_200.0
    ));
}

#[test]
fn the_location_and_complexity_factors_scale_the_totals() {
    let row = unit(
        "InletSeparator",
        "Vessels",
        100_000.0,
        300_000.0,
        375_000.0,
        562_500.0,
        100.0,
        5000.0,
    );
    let base = process_cost_estimate(
        std::slice::from_ref(&row),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    let located = process_cost_estimate(
        std::slice::from_ref(&row),
        Some(1.35),
        Some(1.2),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(close(
        located.total_purchased_equipment_cost,
        base.total_purchased_equipment_cost * 1.35
    ));
    assert!(close(
        located.total_grass_roots_cost,
        base.total_grass_roots_cost * 1.35 * 1.2
    ));
}
