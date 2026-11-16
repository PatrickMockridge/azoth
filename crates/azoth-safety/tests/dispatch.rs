use azoth_safety::dispatch::{self, Outcome};
use serde_json::{Value, json};

fn ok(outcome: &Outcome) -> &azoth_sheets::workspace::CalcOutcome {
    match outcome {
        Outcome::Ok(value) => value,
        Outcome::Error { message, .. } => panic!("the kernel refused: {message}"),
    }
}

fn number(outputs: &azoth_sheets::workspace::CalcOutcome, name: &str) -> f64 {
    let value = outputs
        .outputs
        .get(name)
        .unwrap_or_else(|| panic!("no output `{name}`"));
    value
        .get("magnitude_si")
        .unwrap_or(value)
        .as_f64()
        .unwrap_or_else(|| panic!("`{name}` is not a number"))
}

#[test]
fn the_jet_fire_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.jet_fire",
        &json!({
            "mass_flow": 2.0,
            "heat_of_combustion": 50_000_000.0,
            "radiative_fraction": 0.2,
            "distance": 20.0,
            "target_flux": 12_500.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "total_radiative_power") - 20_000_000.0).abs() < 1e-6);
    assert!((number(outputs, "incident_heat_flux") - 2_119.417_169_158_887).abs() < 1e-6);
    assert!((number(outputs, "distance_to_flux") - 8.780_123_032_005_108).abs() < 1e-9);
}

#[test]
fn the_pool_fire_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.pool_fire",
        &json!({"pool_diameter": 10.0, "burn_rate": 0.06, "distance": 10.0}),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "total_burn_rate") - 4.712_388_980_384_69).abs() < 1e-9);
    assert!((number(outputs, "flame_height") - 16.678_690_349_069_907).abs() < 1e-9);
    assert!((number(outputs, "incident_heat_flux") - 6_784.823_922_879_632).abs() < 1e-6);
}

#[test]
fn the_bleve_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.bleve",
        &json!({
            "fuel_mass": 1000.0,
            "heat_of_combustion": 46_000_000.0,
            "radiative_fraction": 0.3,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "fireball_diameter") - 58.0).abs() < 1e-9);
    assert!((number(outputs, "fireball_duration") - 4.5).abs() < 1e-9);
    assert!((number(outputs, "surface_emissive_power") - 290_175.480_865_128_3).abs() < 1e-6);
}

#[test]
fn the_vce_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.vce",
        &json!({
            "cloud_fuel_mass": 1000.0,
            "heat_of_combustion": 46_000_000.0,
            "strength_class": 4.0,
            "distance": 20.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "overpressure") - 10_132.5).abs() < 1e-9);
    assert!((number(outputs, "scaled_distance") - 0.260_225_342_756_867_5).abs() < 1e-12);
}

#[test]
fn the_pfp_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.pfp_demand",
        &json!({
            "incident_heat_flux": 100_000.0,
            "steel_thickness": 0.012,
            "required_survival_time": 3600.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "steel_mass_per_area") - 94.2).abs() < 1e-12);
    assert!((number(outputs, "bare_steel_time_to_critical") - 197.453_666_666_666_66).abs() < 1e-9);
    assert_eq!(outputs.outputs["rating"], json!("h60"));
    assert_eq!(outputs.outputs["pfp_required"], json!(true));
}

#[test]
fn the_bow_tie_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.bow_tie",
        &json!({
            "threats": [
                {"frequency": 0.1, "active": true},
                {"frequency": 0.05, "active": true},
            ],
            "consequences": [{"severity": 5.0}],
            "barriers": [
                {"pfd": 0.01, "barrier_type": "prevention", "functional": true},
                {"pfd": 0.5, "barrier_type": "mitigation", "functional": true},
            ],
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "unmitigated_frequency_per_year") - 0.15).abs() < 1e-12);
    assert!((number(outputs, "mitigated_frequency_per_year") - 0.0015).abs() < 1e-15);
    assert!((number(outputs, "total_rrf") - 100.0).abs() < 1e-9);
    assert!((number(outputs, "max_severity") - 5.0).abs() < 1e-12);
}

#[test]
fn every_native_id_declares_its_required_inputs() {
    for id in dispatch::NATIVE {
        let required = dispatch::required(id).unwrap_or_else(|| panic!("`{id}` has no table"));
        assert!(!required.is_empty(), "{id}");
        assert!(dispatch::is_native(id), "{id}");
    }
    assert!(dispatch::required("process.gas_dispersion").is_none());
}

#[test]
fn a_missing_input_names_its_field() {
    let outcome = dispatch::run("process.jet_fire", &json!({}));
    match outcome {
        Outcome::Error { field, .. } => assert_eq!(field.as_deref(), Some("mass_flow")),
        Outcome::Ok(_) => panic!("an empty input map ran"),
    }
}

#[test]
fn an_unknown_id_is_refused_rather_than_panicking() {
    match dispatch::run("process.not_a_calculation", &Value::Null) {
        Outcome::Error { message, .. } => assert!(message.contains("not_a_calculation")),
        Outcome::Ok(_) => panic!("an unknown id ran"),
    }
}
