use azoth_safety::study::SafetyStudy;

const DEMO: &str = include_str!("../../../specs/safety/demo.toml");
const BLANK: &str = include_str!("../../../specs/safety/blank.toml");

fn round_trip(document: &str) {
    let study = SafetyStudy::parse(document).expect("the study reads");
    let written = study.to_document().expect("the study writes");
    let again = SafetyStudy::parse(&written).expect("the written study reads");
    assert_eq!(again, study);
    assert_eq!(again.to_document().expect("the study writes"), written);
}

#[test]
fn the_shipped_studies_read_and_round_trip() {
    for document in [BLANK, DEMO] {
        round_trip(document);
    }
}

#[test]
fn a_blank_document_is_a_blank_study() {
    let study = SafetyStudy::parse("").expect("an empty document is a blank study");
    assert_eq!(study, SafetyStudy::default());
    assert!(study.scenarios.is_empty());
    assert!(study.bowties.is_empty());
}

#[test]
fn the_demo_is_six_scenarios_and_one_bowtie() {
    let study = SafetyStudy::parse(DEMO).expect("the demo reads");
    assert_eq!(study.scenarios.len(), 6);
    assert_eq!(study.bowties.len(), 1);
    assert_eq!(
        study
            .scenario("jet_fire_1")
            .expect("the jet fire")
            .calculation,
        "process.jet_fire"
    );
    let bowtie = study.bowtie("bt_separator_release").expect("the bow-tie");
    assert_eq!(bowtie.threats.len(), 2);
    assert_eq!(bowtie.barriers.len(), 2);
    assert_eq!(bowtie.consequences.len(), 2);
    assert_eq!(
        bowtie.consequences[0].scenario.as_deref(),
        Some("jet_fire_1")
    );
    let layout = study.layout.as_ref().expect("the demo is laid out");
    assert_eq!(
        layout.bowties["bt_separator_release"].top_event,
        Some([480.0, 120.0])
    );
}

#[test]
fn two_scenarios_with_one_id_are_refused() {
    let twice = r#"
id = "safety_study_1"
name = "Twice"

[[scenarios]]
id = "same"
name = "One"
calculation = "process.jet_fire"

[[scenarios]]
id = "same"
name = "Two"
calculation = "process.jet_fire"
"#;
    assert!(SafetyStudy::parse(twice).is_err());
}

#[test]
fn two_threats_with_one_id_are_refused() {
    let twice = r#"
id = "safety_study_1"
name = "Twice"

[[bowties]]
id = "bt_1"
name = "One"
top_event = "Loss of containment"

[[bowties.threats]]
id = "same"
label = "One"
frequency = 0.1

[[bowties.threats]]
id = "same"
label = "Two"
frequency = 0.1
"#;
    assert!(SafetyStudy::parse(twice).is_err());
}

#[test]
fn an_empty_name_is_refused() {
    let unnamed = r#"
id = "safety_study_1"
name = " "
"#;
    assert!(SafetyStudy::parse(unnamed).is_err());
}

#[test]
fn the_written_document_keeps_the_active_defaults() {
    let minimal = r#"
id = "safety_study_1"
name = "Defaults"

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
"#;
    let study = SafetyStudy::parse(minimal).expect("the study reads");
    let written = study.to_document().expect("the study writes");
    assert!(written.contains("active = true"), "{written}");
    assert!(written.contains("functional = true"), "{written}");
}
