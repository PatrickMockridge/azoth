use azoth_safety::command::Command;
use azoth_safety::study::{Barrier, Bowtie, Consequence, SafetyStudy, Threat};
use azoth_safety::workspace::{SafetyWorkspace, Target};
use serde_json::Value;

const DEMO: &str = include_str!("../../../specs/safety/demo.toml");
const BLANK: &str = include_str!("../../../specs/safety/blank.toml");

fn envelope(workspace: &SafetyWorkspace) -> Value {
    serde_json::from_str(&workspace.to_json().expect("the envelope writes")).expect("json")
}

fn scenario<'a>(envelope: &'a Value, id: &str) -> &'a Value {
    envelope["study"]["scenarios"]
        .as_array()
        .expect("scenarios")
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no scenario `{id}`"))
}

fn bowtie<'a>(envelope: &'a Value, id: &str) -> &'a Value {
    envelope["study"]["bowties"]
        .as_array()
        .expect("bowties")
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("no bow-tie `{id}`"))
}

fn small_bowtie() -> Bowtie {
    Bowtie {
        id: "bt_1".to_string(),
        name: "One".to_string(),
        top_event: "Loss of containment".to_string(),
        threats: vec![Threat {
            id: "th_1".to_string(),
            label: "Corrosion".to_string(),
            frequency: 0.1,
            active: true,
        }],
        barriers: vec![Barrier {
            id: "pb_1".to_string(),
            label: "Inspection".to_string(),
            pfd: 0.01,
            barrier_type: "prevention".to_string(),
            functional: true,
        }],
        consequences: vec![Consequence {
            id: "co_1".to_string(),
            label: "Fire".to_string(),
            severity: 4.0,
            scenario: None,
        }],
    }
}

#[test]
fn the_demo_calculates_every_stale_item() {
    let mut workspace = SafetyWorkspace::open(DEMO).expect("the demo opens");
    workspace.calculate(None).expect("it calculates");
    let value = envelope(&workspace);
    assert_eq!(value["ok"], true);
    assert_eq!(value["dirty"], false);

    let jet = scenario(&value, "jet_fire_1");
    assert_eq!(jet["stale"], false);
    let flux = jet["result"]["outputs"]["incident_heat_flux"]["magnitude_si"]
        .as_f64()
        .expect("the jet flux");
    assert!((flux - 2_119.417_169_158_887).abs() < 1e-6);
    let profile = jet["profile"].as_array().expect("the profile");
    assert_eq!(profile.len(), 6);
    let row = profile
        .iter()
        .find(|row| row["target"]["magnitude_si"] == 12_500.0)
        .expect("the 12.5 kW/m2 row");
    assert_eq!(row["reached"], true);
    let distance = row["distance"]["magnitude_si"]
        .as_f64()
        .expect("a distance");
    assert!((distance - 8.780_123_032_005_108).abs() < 1e-9);

    let pool = scenario(&value, "pool_fire_1");
    let pool_flux = pool["result"]["outputs"]["incident_heat_flux"]["magnitude_si"]
        .as_f64()
        .expect("the pool flux");
    assert!((pool_flux - 6_784.823_922_879_632).abs() < 1e-6);

    let diagram = bowtie(&value, "bt_separator_release");
    assert_eq!(diagram["stale"], false);
    assert_eq!(diagram["counts"]["threats"], 2);
    assert_eq!(diagram["counts"]["prevention_barriers"], 1);
    assert_eq!(diagram["counts"]["mitigation_barriers"], 1);
    assert_eq!(diagram["counts"]["consequences"], 2);
    let rrf = diagram["derived"]["total_rrf"].as_f64().expect("the RRF");
    assert!((rrf - 100.0).abs() < 1e-9);
    assert_eq!(
        diagram["graph"]["nodes"].as_array().expect("nodes").len(),
        7
    );
    assert_eq!(
        diagram["graph"]["edges"].as_array().expect("edges").len(),
        6
    );
}

#[test]
fn an_input_edit_stales_only_its_scenario() {
    let mut workspace = SafetyWorkspace::open(DEMO).expect("the demo opens");
    workspace.calculate(None).expect("it calculates");
    workspace
        .apply(&Command::SetScenarioInput {
            scenario: "jet_fire_1".to_string(),
            input: "distance".to_string(),
            value: serde_json::json!(80.0),
        })
        .expect("the edit applies");
    let value = envelope(&workspace);
    assert_eq!(value["dirty"], true);
    assert_eq!(scenario(&value, "jet_fire_1")["stale"], true);
    assert_eq!(scenario(&value, "pool_fire_1")["stale"], false);
    assert_eq!(bowtie(&value, "bt_separator_release")["stale"], false);

    workspace
        .calculate(Some(&Target::Scenario("jet_fire_1".to_string())))
        .expect("it calculates");
    let value = envelope(&workspace);
    assert_eq!(value["dirty"], false);
}

#[test]
fn a_layout_edit_does_not_stale_a_result() {
    let mut workspace = SafetyWorkspace::open(DEMO).expect("the demo opens");
    workspace.calculate(None).expect("it calculates");
    workspace
        .apply(&Command::SetBowtiePosition {
            bowtie: "bt_separator_release".to_string(),
            node: "top_event".to_string(),
            x: 500.0,
            y: 130.0,
        })
        .expect("the position applies");
    let value = envelope(&workspace);
    assert_eq!(value["dirty"], false);
    let position = &bowtie(&value, "bt_separator_release")["graph"]["nodes"][4]["position"];
    assert_eq!(position["x"], 500.0);
    assert_eq!(position["y"], 130.0);
    assert!(
        value["study"]["document"]
            .as_str()
            .expect("document")
            .contains("500")
    );
}

#[test]
fn set_bowtie_is_validated() {
    let mut workspace = SafetyWorkspace::open(BLANK).expect("the blank opens");
    let mut bad = small_bowtie();
    bad.barriers[0].pfd = 1.5;
    assert!(
        workspace
            .apply(&Command::SetBowtie { bowtie: bad })
            .is_err()
    );

    let mut bad = small_bowtie();
    bad.threats[0].frequency = -0.1;
    assert!(
        workspace
            .apply(&Command::SetBowtie { bowtie: bad })
            .is_err()
    );

    let mut bad = small_bowtie();
    bad.consequences[0].severity = 4.5;
    assert!(
        workspace
            .apply(&Command::SetBowtie { bowtie: bad })
            .is_err()
    );

    let mut bad = small_bowtie();
    bad.barriers[0].barrier_type = "sometimes".to_string();
    assert!(
        workspace
            .apply(&Command::SetBowtie { bowtie: bad })
            .is_err()
    );

    let mut bad = small_bowtie();
    let copy = bad.threats[0].clone();
    bad.threats.push(copy);
    assert!(
        workspace
            .apply(&Command::SetBowtie { bowtie: bad })
            .is_err()
    );

    workspace
        .apply(&Command::SetBowtie {
            bowtie: small_bowtie(),
        })
        .expect("the sound bow-tie applies");
    workspace.calculate(None).expect("it calculates");
    let value = envelope(&workspace);
    assert_eq!(value["ok"], true);
    let rrf = bowtie(&value, "bt_1")["derived"]["total_rrf"]
        .as_f64()
        .expect("the RRF");
    assert!((rrf - 100.0).abs() < 1e-9);
}

#[test]
fn removing_a_bowtie_takes_its_layout_and_its_result() {
    let mut workspace = SafetyWorkspace::open(DEMO).expect("the demo opens");
    workspace.calculate(None).expect("it calculates");
    workspace
        .apply(&Command::RemoveBowtie {
            id: "bt_separator_release".to_string(),
        })
        .expect("it removes");
    let value = envelope(&workspace);
    assert_eq!(
        value["study"]["bowties"].as_array().expect("bowties").len(),
        0
    );
    assert!(
        value["study"]["layout"]["bowties"]
            .get("bt_separator_release")
            .is_none()
    );
}

#[test]
fn an_unsupported_calculation_is_a_diagnostic_not_an_open_failure() {
    let study = r#"
id = "safety_study_1"
name = "Hand-written"

[[scenarios]]
id = "dispersion_1"
name = "Dispersion screen"
calculation = "process.gas_dispersion"
"#;
    let mut workspace = SafetyWorkspace::open(study).expect("a study naming a model still opens");
    let value = envelope(&workspace);
    assert_eq!(value["ok"], false);
    let diagnostics = scenario(&value, "dispersion_1")["diagnostics"]
        .as_array()
        .expect("diagnostics");
    assert_eq!(diagnostics[0]["code"], "unsupported_calculation");
    assert_eq!(diagnostics[0]["severity"], "error");
    workspace
        .calculate(None)
        .expect("a skipped item is not a failure");
    assert_eq!(
        scenario(&envelope(&workspace), "dispersion_1")["stale"],
        true
    );
}

#[test]
fn a_dangling_consequence_scenario_warns_and_the_bowtie_still_runs() {
    let study = r#"
id = "safety_study_1"
name = "Dangling"

[[bowties]]
id = "bt_1"
name = "One"
top_event = "Loss of containment"

[[bowties.threats]]
id = "th_1"
label = "Corrosion"
frequency = 0.1

[[bowties.barriers]]
id = "pb_1"
label = "Inspection"
pfd = 0.01
barrier_type = "prevention"

[[bowties.consequences]]
id = "co_1"
label = "Fire"
severity = 4.0
scenario = "not_there"
"#;
    let mut workspace = SafetyWorkspace::open(study).expect("the study opens");
    let value = envelope(&workspace);
    assert_eq!(value["ok"], true, "a warning does not clear ok");
    let diagnostics = bowtie(&value, "bt_1")["diagnostics"]
        .as_array()
        .expect("diagnostics");
    assert_eq!(diagnostics[0]["code"], "unknown_scenario");
    assert_eq!(diagnostics[0]["severity"], "warning");
    workspace.calculate(None).expect("it calculates");
    let value = envelope(&workspace);
    let rrf = bowtie(&value, "bt_1")["derived"]["total_rrf"]
        .as_f64()
        .expect("the RRF");
    assert!((rrf - 100.0).abs() < 1e-9);
}

#[test]
fn add_scenario_refuses_unknown_and_non_scenario_ids() {
    let mut workspace = SafetyWorkspace::open(BLANK).expect("the blank opens");
    for calculation in [
        "process.not_a_model",
        "process.gas_dispersion",
        "process.bow_tie",
    ] {
        assert!(
            workspace
                .apply(&Command::AddScenario {
                    id: "one".to_string(),
                    calculation: calculation.to_string(),
                    name: None,
                })
                .is_err(),
            "{calculation} was added"
        );
    }
    workspace
        .apply(&Command::AddScenario {
            id: "jet_1".to_string(),
            calculation: "process.jet_fire".to_string(),
            name: None,
        })
        .expect("a native scenario is added");
    for (input, value) in [
        ("mass_flow", serde_json::json!(2.0)),
        ("heat_of_combustion", serde_json::json!(50_000_000.0)),
        ("radiative_fraction", serde_json::json!(0.2)),
        ("distance", serde_json::json!(20.0)),
    ] {
        workspace
            .apply(&Command::SetScenarioInput {
                scenario: "jet_1".to_string(),
                input: input.to_string(),
                value,
            })
            .expect("the input sets");
    }
    workspace.calculate(None).expect("it calculates");
    let value = envelope(&workspace);
    assert_eq!(scenario(&value, "jet_1")["ready"], true);
    assert_eq!(scenario(&value, "jet_1")["stale"], false);
}

#[test]
fn a_hand_written_integer_input_is_a_quantity() {
    let study = r#"
id = "safety_study_1"
name = "Integers"

[[scenarios]]
id = "jet_1"
name = "Jet"
calculation = "process.jet_fire"

[scenarios.inputs]
mass_flow = 2
heat_of_combustion = 50000000
radiative_fraction = 1
distance = 20
"#;
    let mut workspace = SafetyWorkspace::open(study).expect("the study opens");
    workspace.calculate(None).expect("it calculates");
    let value = envelope(&workspace);
    assert_eq!(scenario(&value, "jet_1")["ready"], true);
    assert_eq!(scenario(&value, "jet_1")["stale"], false);
}

#[test]
fn calculate_is_refused_for_a_target_the_study_does_not_carry() {
    let mut workspace = SafetyWorkspace::open(DEMO).expect("the demo opens");
    assert!(
        workspace
            .calculate(Some(&Target::Scenario("nope".to_string())))
            .is_err()
    );
    assert!(
        workspace
            .calculate(Some(&Target::Bowtie("nope".to_string())))
            .is_err()
    );
}

#[test]
fn the_study_survives_a_round_trip_after_an_edit() {
    let mut workspace = SafetyWorkspace::open(DEMO).expect("the demo opens");
    workspace
        .apply(&Command::SetStudyName {
            name: "Edited".to_string(),
        })
        .expect("the name sets");
    let document = workspace.document().expect("the document writes");
    let again = SafetyWorkspace::open(&document).expect("the written study opens");
    assert_eq!(
        SafetyStudy::parse(&document).expect("parses").name,
        "Edited"
    );
    assert_eq!(again.document().expect("writes"), document);
}
