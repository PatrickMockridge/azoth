//! `pvt.cvd_material_balance` at its boundary.
//!
//! The states are the pin's, from `validation/neqsim/captures/pvt_cvd_material_balance.tsv`.

use azoth_pvt::{CvdMaterialBalanceResult, CvdObservation, LiquidState, cvd_material_balance};

fn row(pressure: f64, gas_z: f64, liquid: f64, cumulative: f64) -> CvdObservation {
    CvdObservation {
        pressure,
        gas_z,
        liquid_volume_fraction: liquid,
        cumulative_produced: cumulative,
    }
}

fn binary() -> CvdMaterialBalanceResult {
    cvd_material_balance(
        &["A".to_string(), "B".to_string()],
        &[
            row(100.0e5, 1.0, 0.0, 0.0),
            row(50.0e5, 1.0, 0.2, 0.2),
            row(25.0e5, 1.0, 0.2, 0.4),
        ],
        &[vec![0.8, 0.2], vec![0.9, 0.1], vec![0.9, 0.1]],
        1.0e-12,
        1.0e-8,
    )
    .unwrap()
}

#[test]
fn the_analytic_binary_path_closes_every_component() {
    let result = binary();
    assert!(result.compatible);
    assert_eq!(result.stages.len(), 3);
    assert_eq!(result.stages[0].liquid_state, LiquidState::Absent);
    for stage in 1..3 {
        assert_eq!(result.stages[stage].liquid_state, LiquidState::Resolved);
        assert!((result.stages[stage].liquid_moles - 0.4).abs() < 1.0e-15);
        assert!((result.stages[stage].stage_withdrawn_moles - 0.2).abs() < 1.0e-15);
    }
    assert!((result.liquid_component_moles[1][0] - 0.25999999999999995).abs() < 1.0e-15);
    assert!((result.liquid_component_moles[1][1] - 0.13999999999999999).abs() < 1.0e-15);
    assert!((result.liquid_component_moles[2][0] - 0.26).abs() < 1.0e-15);
    assert!((result.liquid_component_moles[2][1] - 0.14).abs() < 1.0e-15);
    assert!((result.produced_component_moles[2][0] - 0.36000000000000004).abs() < 1.0e-15);
    assert!((result.remaining_component_moles[2][0] - 0.44).abs() < 1.0e-15);
    assert!(result.stages[1].component_closure.abs() < 1.0e-15);
}

#[test]
fn the_published_ns1_table_reconstructs_the_class_accounting() {
    let names: Vec<String> = [
        "CO2", "N2", "C1", "C2", "C3", "iC4", "nC4", "iC5", "nC5", "C6", "C7+",
    ]
    .iter()
    .map(|name| (*name).to_string())
    .collect();
    let psi = 6894.757293168;
    let observations = [
        row(6764.7 * psi, 1.238, 0.0, 0.0),
        row(5514.7 * psi, 1.089, 0.141, 0.09024),
        row(4314.7 * psi, 0.972, 0.197, 0.21744),
        row(3114.7 * psi, 0.913, 0.216, 0.38674),
        row(2114.7 * psi, 0.914, 0.213, 0.55686),
        row(1214.7 * psi, 0.937, 0.202, 0.72146),
        row(714.7 * psi, 0.960, 0.193, 0.81301),
    ];
    let compositions = [
        [
            0.0237, 0.0031, 0.7319, 0.0780, 0.0355, 0.0071, 0.0145, 0.0064, 0.0068, 0.0109, 0.0821,
        ],
        [
            0.0240, 0.0032, 0.7556, 0.0783, 0.0347, 0.0067, 0.0137, 0.0059, 0.0062, 0.0097, 0.0620,
        ],
        [
            0.0245, 0.0033, 0.7789, 0.0787, 0.0340, 0.0065, 0.0131, 0.0055, 0.0058, 0.0088, 0.0409,
        ],
        [
            0.0250, 0.0034, 0.7933, 0.0792, 0.0341, 0.0064, 0.0130, 0.0053, 0.0056, 0.0083, 0.0264,
        ],
        [
            0.0253, 0.0034, 0.7962, 0.0804, 0.0353, 0.0066, 0.0133, 0.0054, 0.0057, 0.0082, 0.0202,
        ],
        [
            0.0257, 0.0034, 0.7890, 0.0840, 0.0374, 0.0072, 0.0144, 0.0059, 0.0061, 0.0085, 0.0184,
        ],
        [
            0.0260, 0.0033, 0.7780, 0.0870, 0.0391, 0.0078, 0.0156, 0.0064, 0.0066, 0.0090, 0.0212,
        ],
    ];
    let rows: Vec<Vec<f64>> = compositions.iter().map(|row| row.to_vec()).collect();
    let result = cvd_material_balance(&names, &observations, &rows, 1.0e-12, 1.0e-8).unwrap();
    assert!(result.compatible);
    assert_eq!(result.stages.len(), 7);
    assert!((result.stages[1].liquid_moles - 0.11367529644728536).abs() < 1.0e-14);
    assert!((result.stages[1].vapor_moles - 0.7960847035527147).abs() < 1.0e-14);
    assert!((result.produced_component_moles[1][0] - 0.00216576).abs() < 1.0e-16);
    assert!((result.liquid_component_moles[1][0] - 0.0024282071147348488).abs() < 1.0e-14);
    assert!((result.liquid_component_moles[1][10] - 0.027147868379731703).abs() < 1.0e-14);
    assert_eq!(result.stages[6].liquid_state, LiquidState::Resolved);
}

#[test]
fn the_absent_unresolved_and_negative_states_are_distinct() {
    let absent = cvd_material_balance(
        &["A".to_string()],
        &[row(100.0, 1.0, 0.0, 0.0), row(50.0, 1.0, 0.0, 0.5)],
        &[vec![1.0], vec![1.0]],
        1.0e-12,
        1.0e-8,
    )
    .unwrap();
    assert!(absent.compatible);
    assert_eq!(absent.stages[1].liquid_state, LiquidState::Absent);

    let vanishing = cvd_material_balance(
        &["A".to_string()],
        &[
            row(100.0, 1.0, 0.0, 0.0),
            row(50.0, 1.0, 0.2, 0.6 - 1.0e-10),
        ],
        &[vec![1.0], vec![1.0]],
        1.0e-12,
        1.0e-8,
    )
    .unwrap();
    assert!(!vanishing.compatible);
    assert_eq!(vanishing.stages[1].liquid_state, LiquidState::Unresolved);

    let negative = cvd_material_balance(
        &["A".to_string(), "B".to_string()],
        &[row(100.0e5, 1.0, 0.0, 0.0), row(50.0e5, 1.0, 0.2, 0.2)],
        &[vec![0.2, 0.8], vec![0.9, 0.1]],
        1.0e-12,
        1.0e-8,
    )
    .unwrap();
    assert!(!negative.compatible);
    assert_eq!(negative.stages[1].liquid_state, LiquidState::Inconsistent);
    assert!((negative.liquid_component_moles[1][0] + 0.34).abs() < 1.0e-15);
}

#[test]
fn a_normalised_composition_is_refused_rather_than_fixed() {
    let error = cvd_material_balance(
        &["A".to_string(), "B".to_string()],
        &[row(100.0, 1.0, 0.0, 0.0), row(50.0, 1.0, 0.2, 0.2)],
        &[vec![0.5, 0.5], vec![0.8, 0.3]],
        1.0e-12,
        1.0e-8,
    )
    .unwrap_err();
    assert!(error.to_string().contains("sum to one"));

    let first = cvd_material_balance(
        &["A".to_string()],
        &[row(100.0, 1.0, 0.1, 0.0), row(50.0, 1.0, 0.2, 0.2)],
        &[vec![1.0], vec![1.0]],
        1.0e-12,
        1.0e-8,
    )
    .unwrap_err();
    assert!(first.to_string().contains("saturation reference"));

    let decreasing = cvd_material_balance(
        &["A".to_string()],
        &[row(100.0, 1.0, 0.0, 0.0), row(150.0, 1.0, 0.2, 0.2)],
        &[vec![1.0], vec![1.0]],
        1.0e-12,
        1.0e-8,
    )
    .unwrap_err();
    assert!(decreasing.to_string().contains("strictly decrease"));

    let limits = cvd_material_balance(
        &["A".to_string()],
        &[row(100.0, 1.0, 0.0, 0.0), row(50.0, 1.0, 0.2, 0.2)],
        &[vec![1.0], vec![1.0]],
        1.0e-8,
        1.0e-8,
    )
    .unwrap_err();
    assert!(limits.to_string().contains("minimum_liquid_moles"));
}
