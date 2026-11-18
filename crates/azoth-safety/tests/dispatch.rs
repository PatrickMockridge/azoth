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
    assert!((number(outputs, "swept_area") - 2_513.274_122_871_834_6).abs() < 1e-9);
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
fn the_fire_case_relief_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.fire_case_relief",
        &json!({
            "wetted_area": 100.0,
            "has_drainage": false,
            "has_fire_fighting": true,
            "latent_heat": 300_000.0,
            "relieving_temperature": 350.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "latent_heat") - 300_000.0).abs() < 1e-9);
    assert!((number(outputs, "relief_rate") - 10.324_901_255_486_964).abs() < 1e-9);
    // kg/h is the display unit; the magnitude is SI, as every quantity here is.
    assert!((number(outputs, "relief_rate_kg_per_hr") - 10.324_901_255_486_964).abs() < 1e-9);
    assert!((number(outputs, "relieving_temperature") - 350.0).abs() < 1e-12);
}

#[test]
fn the_overpressure_acceptance_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.overpressure_acceptance",
        &json!({
            "peak_pressure": 1_100_000.0,
            "mawp": 1_000_000.0,
            "cause": "blocked_outlet",
            "multiple_devices": false,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "accumulation_factor") - 1.1).abs() < 1e-12);
    assert!((number(outputs, "allowable_pressure") - 1_100_000.0).abs() < 1e-6);
    assert!((number(outputs, "margin") - 0.0).abs() < 1e-6);
    assert_eq!(outputs.outputs["accepted"], json!(true));
}

#[test]
fn the_pipeline_protection_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.pipeline_pressure_protection",
        &json!({
            "max_source_pressure": 12.0,
            "design_pressure": 10.0,
            "incidental_factor": 1.1,
            "barrier1_setpoint": 8.0,
            "barrier2_setpoint": 10.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "max_incidental_pressure") - 1_100_000.0).abs() < 1e-6);
    assert!((number(outputs, "design_margin") + 200_000.0).abs() < 1e-6);
    assert_eq!(outputs.outputs["fully_rated"], json!(false));
    assert_eq!(outputs.outputs["protection_adequate"], json!(true));
    assert_eq!(outputs.outputs["verdict"], json!("TWO_BARRIERS_REQUIRED"));
}

#[test]
fn the_psv_sizing_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.psv_sizing",
        &json!({
            "service": "gas",
            "mass_flow": 1.0,
            "set_pressure": 1_000_000.0,
            "overpressure_fraction": 0.1,
            "back_pressure": 800_000.0,
            "temperature": 300.0,
            "molar_mass": 0.02,
            "compressibility": 0.9,
            "specific_heat_ratio": 1.3,
            "is_balanced_bellows": true,
            "has_rupture_disk": false,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "relieving_pressure") - 1_100_000.0).abs() < 1e-6);
    assert!((number(outputs, "required_area") - 7.267_430_890_193_706e-4).abs() < 1e-15);
    assert!((number(outputs, "required_area_in2") - 1.126_454_040_888_106_3).abs() < 1e-12);
    assert_eq!(outputs.outputs["recommended_orifice"], json!("J"));
    assert!((number(outputs, "back_pressure_correction") - 0.643_499_521_697_216_9).abs() < 1e-12);
}

#[test]
fn the_psv_sizing_liquid_branch_names_the_missing_flow() {
    match dispatch::run(
        "process.psv_sizing",
        &json!({
            "service": "liquid",
            "set_pressure": 1_000_000.0,
            "overpressure_fraction": 0.1,
            "back_pressure": 101_325.0,
        }),
    ) {
        Outcome::Error { field, .. } => assert_eq!(field.as_deref(), Some("volume_flow")),
        Outcome::Ok(_) => panic!("a liquid service with no flow ran"),
    }
}

#[test]
fn the_psv_fire_sizing_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.psv_fire_sizing",
        &json!({
            "initial_mass": 5000.0,
            "initial_pressure": 2_000_000.0,
            "set_pressure": 1_000_000.0,
            "initial_temperature": 300.0,
            "fire_heat_input": 100_000.0,
            "vessel_volume": 10.0,
            "molar_mass": 0.02,
            "specific_heat_ratio": 1.3,
            "compressibility": 0.9,
            "heat_capacity": 2000.0,
            "blowdown_time": 900.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "overpressure_fraction") - 0.21).abs() < 1e-12);
    assert!((number(outputs, "required_area") - 0.002_364_274_017_247_606_5).abs() < 1e-15);
    assert!((number(outputs, "required_area_in2") - 3.664_632_055_997_902_4).abs() < 1e-12);
    assert_eq!(outputs.outputs["recommended_orifice"], json!("N"));
}

#[test]
fn the_nog070_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.nog070_sil_determination",
        &json!({"sif_type": "hipps_pipeline", "pfd_average": 0.005}),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "catalogue_minimum_sil") - 3.0).abs() < 1e-12);
    assert!((number(outputs, "achieved_sil") - 2.0).abs() < 1e-12);
    assert_eq!(outputs.outputs["compliant"], json!(false));
    assert!((number(outputs, "required_pfd_reduction_factor") - 10.0).abs() < 1e-12);
}

#[test]
fn the_s001_secondary_pressure_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.s001_secondary_pressure_protection",
        &json!({
            "maximum_event_pressure": 1.2e7,
            "design_pressure": 1.0e7,
            "test_pressure": 1.5e7,
            "demand_frequency_per_year": 1.0e-4,
            "relief_leakage_assessed": true,
            "relief_leakage_to_safe_location": true,
            "proof_test_interval_months": 6.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "target_frequency_per_year") - 1.0e-3).abs() < 1e-12);
    assert_eq!(outputs.outputs["pressure_basis_complete"], json!(true));
    assert_eq!(outputs.outputs["acceptable"], json!(true));
}

#[test]
fn the_gas_blowby_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.gas_blowby",
        &json!({
            "upstream_pressure": 1.0e6,
            "upstream_temperature": 300.0,
            "downstream_pressure": 5.0e5,
            "restriction_area": 1.0e-4,
            "specific_heat_ratio": 1.3,
            "molar_mass": 0.020,
            "downstream_relief_capacity": 0.2777777777777778,
        }),
    );
    let outputs = ok(&outcome);
    assert_eq!(outputs.outputs["choked"], json!(true));
    assert!((number(outputs, "blowby_mass_rate") - 0.1606026936773972).abs() < 1e-12);
    assert_eq!(outputs.outputs["verdict"], json!("relief_adequate"));
}

#[test]
fn the_pump_deadhead_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.pump_deadhead",
        &json!({
            "suction_pressure": 2.0e5,
            "normal_discharge_pressure": 1.0e6,
            "protected_pressure_rating": 1.5e6,
            "liquid_density": 998.0,
            "specific_heat": 4180.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "deadhead_pressure") - 1.16e6).abs() < 1e-6);
    assert_eq!(outputs.outputs["verdict"], json!("within_rating"));
}

#[test]
fn the_runaway_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.runaway_reaction",
        &json!({
            "process_temperature": 350.0,
            "reaction_enthalpy": -100000.0,
            "limiting_reactant_moles": 10.0,
            "total_mass": 100.0,
            "specific_heat": 2000.0,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "adiabatic_temperature_rise") - 5.0).abs() < 1e-12);
    assert_eq!(outputs.outputs["verdict"], json!("no_rating"));
}

#[test]
fn the_mdmt_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.mdmt",
        &json!({
            "material_curve": "b",
            "governing_thickness": 0.025,
            "min_operating_temperature": 233.15,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "mdmt") - 256.5465598240832).abs() < 1e-9);
    assert_eq!(outputs.outputs["acceptable"], json!(false));
}

#[test]
fn the_sif_case_runs_through_the_dispatch() {
    let outcome = dispatch::run(
        "process.safety_instrumented_function",
        &json!({
            "pfd_average": 0.005,
            "claimed_sil": 2.0,
            "unmitigated_frequency_per_year": 0.1,
            "target_frequency_per_year": 1.0e-5,
            "test_interval_hours": 8760.0,
            "lambda_du_per_hour": 1.0e-6,
            "spurious_trip_rate_per_year": 0.0,
            "mttr_hours": 0.0,
            "base_availability": 0.99,
        }),
    );
    let outputs = ok(&outcome);
    assert!((number(outputs, "risk_reduction_factor") - 200.0).abs() < 1e-9);
    assert!((number(outputs, "mitigated_frequency_per_year") - 5.0e-4).abs() < 1e-15);
    assert!((number(outputs, "required_pfd") - 1.0e-4).abs() < 1e-15);
    assert!((number(outputs, "required_sil") - 3.0).abs() < 1e-12);
    assert_eq!(outputs.outputs["pfd_within_claimed_sil"], json!(true));
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
fn every_native_id_declares_the_inputs_its_form_requires() {
    for id in dispatch::NATIVE {
        let required = dispatch::required(id).unwrap_or_else(|| panic!("`{id}` has no table"));
        let entry = azoth_safety::catalogue::calculation_of(id)
            .unwrap_or_else(|| panic!("`{id}` is dispatched but not in the catalogue"));
        assert!(entry.native, "{id}");
        match entry.form {
            Some(form) => {
                let mut declared: Vec<&str> = form
                    .inputs
                    .iter()
                    .filter(|input| input.required)
                    .map(|input| input.name)
                    .collect();
                let mut listed: Vec<&str> = required.to_vec();
                declared.sort_unstable();
                listed.sort_unstable();
                assert_eq!(listed, declared, "{id}");
            }
            None => {
                assert_eq!(*id, "process.bow_tie");
                assert!(!required.is_empty());
            }
        }
    }
    assert!(dispatch::required("process.gas_dispersion").is_none());
}

#[test]
fn every_form_has_an_arm_and_every_listed_entry_a_refusal() {
    for entry in azoth_safety::catalogue::calculations() {
        if entry.form.is_some() {
            assert!(
                dispatch::NATIVE.contains(&entry.id),
                "`{}` has a form but no arm",
                entry.id
            );
        }
        if !entry.native {
            assert!(
                entry.refusal.is_some(),
                "`{}` is listed without a refusal",
                entry.id
            );
        }
    }
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
