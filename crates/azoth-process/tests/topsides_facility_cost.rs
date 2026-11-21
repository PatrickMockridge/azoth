//! `process.topsides_facility_cost` against
//! `validation/neqsim/captures/process_cost_topsides.tsv`.

use azoth_core::units::{Mass, kilogram};
use azoth_process::models::topsides_facility_cost::{
    TopsidesEquipment, TopsidesFacilityCostResult, topsides_facility_cost,
};

fn unit(name: &str, kind: &str, tmc: f64, weight: f64) -> TopsidesEquipment {
    TopsidesEquipment {
        name: name.to_string(),
        equipment_type: kind.to_string(),
        total_module_cost: tmc,
        weight: Mass::new::<kilogram>(weight),
    }
}

fn run(rows: Vec<TopsidesEquipment>) -> TopsidesFacilityCostResult {
    topsides_facility_cost(
        &rows,
        Some("fixed_platform".to_string()),
        Some("greenfield".to_string()),
        Some(1.35),
        Some(0.25),
        Some(0.16),
        Some(0.06),
        Some(0.08),
        Some(0.14),
        Some(0.06),
        Some(0.15),
        Some(0.10),
        Some(true),
        Some(3.0),
    )
    .expect("the topsides estimate answers")
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-12 * expected.abs().max(1.0)
}

#[test]
fn the_fixed_platform_matches_the_capture() {
    let result = run(vec![
        unit("Feed", "Streams", 0.0, 0.0),
        unit(
            "Inlet Separator",
            "Vessels",
            14905.679819876523,
            1312.8218157927067,
        ),
        unit(
            "Export Compressor",
            "Compressors",
            1997444.3385886613,
            695625.7181328537,
        ),
        unit("Aftercooler", "Heat Exchangers", 9387.9, 228.0),
        unit(
            "Export Control Valve",
            "Valves",
            272641.47134845454,
            274.7412026459561,
        ),
    ]);
    assert_eq!(result.capital_costs.len(), 10);
    assert!(close(result.capital_costs[0].cost, 3097412.17617194));
    assert!(close(result.capital_costs[1].cost, 8473911.565988204));
    assert!(close(result.capital_costs[7].cost, 557534.1917109492));
    assert_eq!(result.capital_cost_summary.len(), 2);
    assert!(close(
        result.capital_cost_summary[0].cost,
        1.899475884566912e7
    ));
    assert!(
        result
            .capital_cost_breakdown
            .iter()
            .any(|row| row.name == "module.separationAndTreatmentModules")
    );
    assert!(close(
        result
            .capital_cost_breakdown
            .iter()
            .find(|row| row.name == "module.compressionAndRotatingEquipment")
            .unwrap()
            .cost,
        2696549.857094693
    ));
    assert_eq!(result.project_costs.len(), 4);
    assert!(close(
        result.project_cost_summary[0].cost,
        2.754240032622022e7
    ));
    assert_eq!(result.weight_basis.len(), 4);
    assert!(close(result.weight_basis[3].weight_kg, 1569242.882590408));
    assert_eq!(result.material_take_off.len(), 9);
    assert!(result.material_take_off[0].has_weight);
    assert_eq!(result.material_take_off[7].category, "electrical");
    assert!(!result.material_take_off[7].has_weight);
    assert!(close(
        result.material_take_off[8].cost_usd,
        247792.97409375518
    ));
    assert_eq!(result.quality_flags.len(), 3);
    assert_eq!(
        result.quality_flags[0].message,
        "Facility type factor set: Fixed platform"
    );
}

#[test]
fn a_class_four_basis_adds_the_not_detailed_flag() {
    let result = topsides_facility_cost(
        &[unit("Inlet Separator", "Vessels", 100_000.0, 5000.0)],
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
    assert_eq!(result.quality_flags.len(), 4);
    assert_eq!(
        result.quality_flags[3].message,
        "Estimate is not a detailed Class 1-3 vendor or firm-quantity estimate."
    );
}

#[test]
fn a_well_is_excluded_from_the_topsides_scope() {
    let result = topsides_facility_cost(
        &[
            unit("Inlet Separator", "Vessels", 100_000.0, 5000.0),
            unit("Well1", "Wells", 80_000_000.0, 0.0),
        ],
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
    assert!(close(result.capital_costs[0].cost, 100_000.0));
    assert!(close(result.capital_cost_summary[1].cost, 80_000_000.0));
    assert_eq!(result.material_take_off.len(), 1 + 4);
    assert!(
        result
            .quality_flags
            .iter()
            .any(|flag| flag.message.contains("Excluded non-topsides item"))
    );
}

#[test]
fn the_estimate_refuses_an_unknown_facility_or_class() {
    assert!(
        topsides_facility_cost(
            &[],
            Some("submarine".to_string()),
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
        .is_err()
    );
    assert!(
        topsides_facility_cost(
            &[],
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
            Some(6.0),
        )
        .is_err()
    );
}
