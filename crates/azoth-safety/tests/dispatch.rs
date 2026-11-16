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
fn the_gaussian_plume_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.gaussian_plume",
        &json!({
            "emission_rate": 1.0,
            "effective_stack_height": 5.0,
            "wind_speed": 3.0,
            "stability": "B",
            "terrain": "urban",
            "distance": 500.0,
            "crosswind": 0.0,
            "height": 0.0,
            "threshold": 1.0e-4,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "sigma_y") - 146.059_348_668_044_3).abs() < 1e-9);
    assert!((number(outputs, "sigma_z") - 97.979_589_711_327_14).abs() < 1e-9);
    assert!((number(outputs, "concentration") - 7.404_545_545_613_47e-6).abs() < 1e-15);
    assert!((number(outputs, "distance_to_threshold") - 121.271_019_705_978_8).abs() < 1e-9);
}

#[test]
fn the_hazardous_area_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.hazardous_area",
        &json!({
            "mass_flow": 0.1,
            "exit_density": 6.0,
            "exit_velocity": 340.0,
            "lfl_volume_fraction": 0.044,
            "gas_molar_mass": 0.01604,
            "safety_factor": 0.25,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "effective_diameter") - 0.007_900_234_374_579_8).abs() < 1e-15);
    assert!((number(outputs, "lfl_mass_fraction") - 0.024_854_779_008_840_32).abs() < 1e-12);
    assert!((number(outputs, "hazardous_distance") - 15.194_674_516_657_654).abs() < 1e-9);
    assert_eq!(outputs.outputs["release_grade"], json!("SECONDARY"));
    assert_eq!(outputs.outputs["zone_classification"], json!("Zone 2"));
}

#[test]
fn the_heavy_gas_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.heavy_gas_dispersion",
        &json!({
            "volumetric_rate": 1.470_461_327_559_685_8,
            "source_density": 0.680_058_687_200_945_9,
            "air_density": 1.225_012_265_990_694_6,
            "wind_speed": 5.0,
            "concentration_ratio": 0.022,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "reduced_gravity") + 4.364_033_533_659_363).abs() < 1e-9);
    assert!((number(outputs, "britter_parameter") - 0.389_471_738_142_793_9).abs() < 1e-12);
    assert!(
        (number(outputs, "distance_to_concentration_ratio") - 17.602_061_561_927_734).abs() < 1e-9
    );
}

#[test]
fn the_probit_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.probit",
        &json!({"a": 5.0, "b": 0.0, "n": 1.0, "dose": 1.0, "probit_value": 5.0}),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "probit") - 5.0).abs() < 1e-15);
    assert!((number(outputs, "probability") - 0.500_000_000_5).abs() < 1e-12);
}

#[test]
fn the_toxic_library_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.toxic_library",
        &json!({"chemical": "hydrogen sulfide"}),
    );
    let outputs = ok(&outcome);
    assert_eq!(outputs.outputs["known"], json!(true));
    assert_eq!(outputs.outputs["name"], json!("Hydrogen sulfide"));
    assert!((number(outputs, "idlh_ppm") - 100.0).abs() < 1e-12);
    assert!((number(outputs, "erpg2_ppm") - 30.0).abs() < 1e-12);
    assert!((number(outputs, "aegl2_ppm") - 27.0).abs() < 1e-12);
}

#[test]
fn the_active_fire_protection_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.active_fire_protection_screening",
        &json!({
            "scenario": "liquid_pool_fire",
            "blowdown_time_to_target": 600.0,
            "bare_steel_time_to_critical": 900.0,
            "dedicated_object_demand": 170.0,
            "general_area_demand": 5100.0,
        }),
    );
    let outputs = ok(&outcome);
    assert_eq!(outputs.outputs["water_extinguishing"], json!(true));
    assert_eq!(outputs.outputs["blowdown_faster_than_failure"], json!(true));
    assert!((number(outputs, "blowdown_margin") - 300.0).abs() < 1e-12);
    assert!(
        (number(outputs, "selective_protection_saving_fraction") - 0.966_666_666_666_666_7).abs()
            < 1e-12
    );
}

#[test]
fn the_deluge_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.deluge_nozzle_layout",
        &json!({
            "protected_area": 510.0,
            "required_density": 10.0,
            "nozzle_k": 42.9,
            "nozzle_min_pressure": 3.5,
            "nozzle_max_spacing": 3.0,
            "nozzle_clearance": 1.0,
            "available_clearance": 1.2,
            "operating_pressure": 5.0,
            "coverage_efficiency": 1.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "required_flow") - 0.085).abs() < 1e-15);
    assert!((number(outputs, "nozzle_count") - 57.0).abs() < 1e-12);
    assert!((number(outputs, "grid_spacing") - 2.991_215_208_080_594).abs() < 1e-12);
    assert_eq!(outputs.outputs["governing_criterion"], json!("coverage"));
}

#[test]
fn the_fire_monitor_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.fire_monitor_coverage",
        &json!({
            "target_area": 3.0,
            "required_density": 2000.0,
            "monitor_count": 1.0,
            "monitor_flow": 2400.0,
            "effective_throw": 40.0,
            "sweep_angle": 180.0,
            "wind_speed": 0.0,
            "fall_height": 12.0,
            "droplet_terminal_velocity": 8.0,
            "line_of_sight_obstructed": false,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "total_flow") - 0.04).abs() < 1e-15);
    assert!((number(outputs, "swept_area") - 2513.274_122_871_834_6).abs() < 1e-9);
    assert!((number(outputs, "nominal_density") - 800.0).abs() < 1e-12);
    assert!((number(outputs, "flow_needed_for_compliance") - 0.1).abs() < 1e-15);
    assert_eq!(
        outputs.outputs["verdict"],
        json!("NOT SUITABLE - required density not achieved even in still air")
    );
}

#[test]
fn the_fire_water_coverage_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.fire_water_coverage_assessment",
        &json!({
            "protected_area": 510.0,
            "required_flow": 5100.0,
            "spare_supply_flow": 20000.0,
            "worst_case_pressure_margin": -0.9,
            "covered_area": 0.0,
            "existing_delivered_flow": 0.0,
            "required_pressure_margin": 0.5,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "coverage_gap") - 510.0).abs() < 1e-12);
    assert!((number(outputs, "flow_deficit") - 0.085).abs() < 1e-15);
    assert_eq!(outputs.outputs["flow_feasible"], json!(true));
    assert_eq!(outputs.outputs["pressure_feasible"], json!(false));
    assert_eq!(outputs.outputs["verdict"], json!("supply_pressure_deficit"));
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
        dispatch::required(id).unwrap_or_else(|| panic!("`{id}` has no table"));
        assert!(dispatch::is_native(id), "{id}");
    }
    // An id whose every declared input is optional has an empty list; S2's form-equality
    // invariant is what holds this to the generated form.
    assert_eq!(dispatch::required("process.toxic_library"), Some(&[][..]));
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
