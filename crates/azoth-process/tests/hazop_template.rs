//! `process.hazop_template` against
//! `validation/neqsim/captures/process_hazop_template.tsv`.

use azoth_process::models::hazop_template::{
    HazopGridParameter, HazopTemplateResult, hazop_template,
};

const GUIDE_WORDS: [&str; 7] = [
    "no",
    "more",
    "less",
    "reverse",
    "as_well_as",
    "part_of",
    "other_than",
];

fn grid(parameters: &[&str], equipment: Option<&str>) -> HazopTemplateResult {
    let rows: Vec<HazopGridParameter> = parameters
        .iter()
        .map(|parameter| HazopGridParameter {
            parameter: (*parameter).to_string(),
        })
        .collect();
    hazop_template(&rows, equipment).expect("the captured state is valid")
}

fn pairs(result: &HazopTemplateResult) -> Vec<(&'static str, &'static str)> {
    result
        .deviations
        .iter()
        .map(|row| (row.guide_word.as_str(), row.parameter.as_str()))
        .collect()
}

#[test]
fn the_two_parameter_grid_is_fourteen_rows() {
    let result = grid(&["flow", "pressure"], None);
    assert_eq!(result.deviation_count, 14.0);
    assert_eq!(result.deviations.len(), 14);

    let expected: Vec<(&str, &str)> = ["flow", "pressure"]
        .into_iter()
        .flat_map(|parameter| GUIDE_WORDS.map(|guide| (guide, parameter)))
        .collect();
    assert_eq!(pairs(&result), expected);

    for row in &result.deviations {
        assert_eq!(row.cause, "TBD");
        assert_eq!(row.consequence, "TBD");
        assert_eq!(row.safeguard, "TBD");
        assert_eq!(row.recommendation, None);
    }
}

#[test]
fn the_seed_table_selects_by_equipment_type() {
    let separator = grid(&[], Some("Separator"));
    assert_eq!(
        pairs(&separator),
        vec![
            ("no", "flow"),
            ("more", "level"),
            ("less", "level"),
            ("more", "pressure"),
            ("less", "pressure"),
        ]
    );

    let compressor = grid(&[], Some("Compressor"));
    assert_eq!(
        pairs(&compressor),
        vec![
            ("no", "flow"),
            ("reverse", "flow"),
            ("more", "pressure"),
            ("more", "temperature"),
        ]
    );

    let pump = grid(&[], Some("Pump"));
    assert_eq!(
        pairs(&pump),
        vec![("no", "flow"), ("reverse", "flow"), ("more", "pressure")]
    );

    let valve = grid(&[], Some("ThrottlingValve"));
    assert_eq!(
        pairs(&valve),
        vec![("no", "flow"), ("more", "flow"), ("less", "flow")]
    );

    let cooler = grid(&[], Some("Cooler"));
    assert_eq!(
        pairs(&cooler),
        vec![
            ("more", "temperature"),
            ("less", "temperature"),
            ("no", "flow"),
        ]
    );

    let pipe = grid(&[], Some("PipeBeggsAndBrills"));
    assert_eq!(
        pairs(&pipe),
        vec![
            ("no", "flow"),
            ("less", "flow"),
            ("less", "pressure"),
        ]
    );

    let default = grid(&[], Some("Stream"));
    assert_eq!(
        pairs(&default),
        vec![
            ("no", "flow"),
            ("more", "flow"),
            ("more", "pressure"),
            ("less", "pressure"),
        ]
    );
}

#[test]
fn the_first_matching_branch_wins() {
    // `CompressorCooler` matches the compressor branch before the heat-exchanger words.
    assert_eq!(grid(&[], Some("CompressorCooler")).deviations.len(), 4);
    // A scrubber is a separator, not a pipe.
    assert_eq!(grid(&[], Some("ScrubberPipe")).deviations.len(), 5);
    assert_eq!(grid(&[], Some("HEATEXCHANGER")).deviations.len(), 3);
}

#[test]
fn an_absent_type_runs_no_seed_arm_and_a_blank_one_runs_the_default() {
    assert_eq!(grid(&["flow"], None).deviations.len(), 7);
    let blank = grid(&["flow"], Some(""));
    assert_eq!(blank.deviations.len(), 11);
    assert_eq!(
        &pairs(&blank)[..4],
        &[
            ("no", "flow"),
            ("more", "flow"),
            ("more", "pressure"),
            ("less", "pressure"),
        ]
    );
}

#[test]
fn the_seed_rows_precede_the_grid_rows() {
    let result = grid(&["flow"], Some("Separator"));
    assert_eq!(result.deviation_count, 12.0);
    assert_eq!(
        pairs(&result),
        vec![
            ("no", "flow"),
            ("more", "level"),
            ("less", "level"),
            ("more", "pressure"),
            ("less", "pressure"),
            ("no", "flow"),
            ("more", "flow"),
            ("less", "flow"),
            ("reverse", "flow"),
            ("as_well_as", "flow"),
            ("part_of", "flow"),
            ("other_than", "flow"),
        ]
    );
}

#[test]
fn an_empty_grid_is_no_rows() {
    let result = hazop_template(&[], None).expect("an empty grid is a state");
    assert_eq!(result.deviation_count, 0.0);
    assert!(result.deviations.is_empty());
}

#[test]
fn an_unknown_parameter_is_refused() {
    let rows = vec![HazopGridParameter {
        parameter: "viscosity".to_string(),
    }];
    assert!(hazop_template(&rows, None).is_err());
}
