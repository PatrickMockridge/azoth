//! `process.overpressure_protection_study` against
//! `validation/neqsim/captures/process_overpressure_protection_study.tsv`.

use azoth_core::units::{
    kelvins, kilograms_per_cubic_meter, kilograms_per_mole, kilograms_per_second, pascal_seconds,
    pascals,
};
use azoth_process::models::overpressure_protection_study::{
    OverpressureProtectionStudyResult, OverpressureScenario, overpressure_protection_study,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn vapour(name: &str, rate: f64) -> OverpressureScenario {
    OverpressureScenario {
        name: name.to_string(),
        cause: "blocked_outlet".to_string(),
        phase: "vapour".to_string(),
        relief_rate: kilograms_per_second(rate),
        relief_temperature: Some(kelvins(320.0)),
        molar_mass: Some(kilograms_per_mole(0.017_445_699_999_999_998)),
        compressibility: Some(0.959_160_201_346_084_2),
        specific_heat_ratio: Some(1.279_579_897_384_378_6),
        density: None,
        viscosity: None,
        gas_mass_fraction: None,
        gas_density: None,
        liquid_density: None,
        latent_heat: None,
        liquid_heat_capacity: None,
        credible: None,
    }
}

fn run(
    mawp: f64,
    scenarios: &[OverpressureScenario],
    set_pressure: Option<f64>,
) -> OverpressureProtectionStudyResult {
    overpressure_protection_study(
        pascals(mawp),
        scenarios,
        set_pressure.map(pascals),
        None,
        None,
    )
    .expect("the captured study is valid")
}

#[test]
fn the_gas_case_governs_by_rate_and_takes_the_j_orifice() {
    let scenarios = [
        vapour("Blocked outlet small", 0.5),
        vapour("Blocked outlet gas", 2.0),
    ];
    let study = run(2.0e6, &scenarios, None);
    assert_eq!(study.governing_name.as_deref(), Some("Blocked outlet gas"));
    assert_eq!(study.governing_cause.as_deref(), Some("blocked_outlet"));
    assert_eq!(study.governing_phase.as_deref(), Some("vapour"));
    close(
        study.governing_relief_rate.expect("governing").value,
        2.0,
        "the governing rate",
    );
    close(
        study.required_area.expect("sized").value,
        5.393_485_760_524_218e-4,
        "the required area",
    );
    close(
        study.required_area_in2.expect("sized"),
        0.835_991_964_865_183_7,
        "the required area in square inches",
    );
    assert_eq!(study.recommended_orifice.as_deref(), Some("J"));
    close(
        study.selected_area_in2.expect("selected"),
        1.287,
        "the selected area",
    );
    close(
        study.selected_orifice_capacity.expect("capacity").value,
        3.078_976_961_716_487_7,
        "the selected capacity",
    );
    assert!(study.capacity_adequate);
    close(
        study.acceptance_peak_pressure.expect("accepted").value,
        2_189_867.5,
        "the peak pressure",
    );
    close(
        study.acceptance_allowable_pressure.expect("accepted").value,
        2_189_867.5,
        "the allowable pressure",
    );
    close(
        study.acceptance_accumulation_factor.expect("accepted"),
        1.1,
        "the accumulation factor",
    );
    assert_eq!(study.accepted, Some(true));
    assert!(study.messages.is_empty(), "{:?}", study.messages);
}

#[test]
fn the_liquid_case_takes_the_f_orifice() {
    let liquid = OverpressureScenario {
        name: "Liquid release".to_string(),
        cause: "blocked_outlet".to_string(),
        phase: "liquid".to_string(),
        relief_rate: kilograms_per_second(5.0),
        relief_temperature: Some(kelvins(320.0)),
        molar_mass: Some(kilograms_per_mole(0.058)),
        compressibility: Some(0.05),
        specific_heat_ratio: Some(1.1),
        density: Some(kilograms_per_cubic_meter(600.0)),
        viscosity: Some(pascal_seconds(1.0e-3)),
        gas_mass_fraction: None,
        gas_density: None,
        liquid_density: None,
        latent_heat: None,
        liquid_heat_capacity: None,
        credible: None,
    };
    let study = run(2.0e6, &[liquid], None);
    close(
        study.required_area.expect("sized").value,
        1.536_234_519_861_117_4e-4,
        "the required area",
    );
    close(
        study.required_area_in2.expect("sized"),
        0.238_116_826_812_126_81,
        "the required area in square inches",
    );
    assert_eq!(study.recommended_orifice.as_deref(), Some("F"));
    close(
        study.selected_orifice_capacity.expect("capacity").value,
        6.446_415_486_676_667,
        "the selected capacity",
    );
}

#[test]
fn the_two_phase_case_takes_the_d_orifice() {
    let two_phase = OverpressureScenario {
        name: "Two phase release".to_string(),
        cause: "blocked_outlet".to_string(),
        phase: "two_phase".to_string(),
        relief_rate: kilograms_per_second(1.0),
        relief_temperature: Some(kelvins(350.0)),
        molar_mass: Some(kilograms_per_mole(0.03)),
        compressibility: Some(0.85),
        specific_heat_ratio: Some(1.2),
        density: None,
        viscosity: None,
        gas_mass_fraction: Some(0.2),
        gas_density: Some(kilograms_per_cubic_meter(20.0)),
        liquid_density: Some(kilograms_per_cubic_meter(600.0)),
        latent_heat: Some(2.0e5),
        liquid_heat_capacity: Some(2500.0),
        credible: None,
    };
    let study = run(2.0e6, &[two_phase], None);
    close(
        study.required_area.expect("sized").value,
        1.365_597_383_837_508_4e-5,
        "the required area",
    );
    close(
        study.required_area_in2.expect("sized"),
        0.021_166_801_783_084_947,
        "the required area in square inches",
    );
    assert_eq!(study.recommended_orifice.as_deref(), Some("D"));
    close(
        study.selected_orifice_capacity.expect("capacity").value,
        5.196_817_220_063_186,
        "the selected capacity",
    );
}

#[test]
fn no_credible_scenario_is_the_class_s_message() {
    let mut fire = vapour("Not credible", 100.0);
    fire.cause = "fire".to_string();
    fire.credible = Some(false);
    let study = run(2.0e6, &[fire], None);
    assert!(study.governing_name.is_none());
    assert!(study.required_area.is_none());
    assert!(!study.capacity_adequate);
    assert!(study.accepted.is_none());
    assert_eq!(
        study.messages,
        vec!["No credible positive relief demand defined"]
    );
}

#[test]
fn a_zero_rate_leaves_the_study_unsized() {
    let zero = vapour("Zero rate", 0.0);
    let study = run(2.0e6, &[zero], None);
    assert!(study.governing_name.is_none());
    assert!(study.required_area.is_none());
    assert!(study.accepted.is_none());
    assert_eq!(
        study.messages,
        vec![
            "Zero rate: Missing or invalid pressure, relief demand, cause or relieving-phase \
             property basis",
            "Resolve all credible relief demands before selecting an orifice"
        ]
    );
}

#[test]
fn a_capacity_shortfall_names_the_largest_orifice() {
    let mut shortfall = vapour("Capacity shortfall", 1000.0);
    shortfall.molar_mass = Some(kilograms_per_mole(0.02));
    shortfall.compressibility = Some(0.9);
    shortfall.specific_heat_ratio = Some(1.3);
    let study = run(2.0e6, &[shortfall], None);
    assert_eq!(study.recommended_orifice.as_deref(), Some("T"));
    close(
        study.required_area_in2.expect("sized"),
        376.054_607_787_639_2,
        "the required area",
    );
    assert!(!study.capacity_adequate);
    assert_eq!(
        study.messages,
        vec![
            "Capacity shortfall: A single standard orifice cannot be selected for this relief \
             demand"
        ]
    );
}

#[test]
fn a_set_pressure_above_the_mawp_is_refused() {
    let study = run(8.0e5, &[vapour("Blocked outlet gas", 2.0)], Some(1.0e6));
    assert!(study.recommended_orifice.is_none());
    assert!(study.accepted.is_none());
    assert_eq!(
        study.messages,
        vec![
            "Blocked outlet gas: Missing or invalid pressure, relief demand, cause or \
             relieving-phase property basis"
                .to_string(),
            "Resolve all credible relief demands before selecting an orifice".to_string()
        ]
    );
}

#[test]
fn a_fire_case_takes_the_twenty_one_percent_basis() {
    let mut fire = vapour("Fire case", 2.0);
    fire.cause = "fire".to_string();
    fire.molar_mass = Some(kilograms_per_mole(0.02));
    fire.compressibility = Some(0.9);
    fire.specific_heat_ratio = Some(1.3);
    let study = run(2.0e6, &[fire], None);
    assert_eq!(study.recommended_orifice.as_deref(), Some("H"));
    close(
        study.acceptance_peak_pressure.expect("accepted").value,
        2_398_721.75,
        "the peak pressure",
    );
    close(
        study.acceptance_accumulation_factor.expect("accepted"),
        1.21,
        "the accumulation factor",
    );
    assert_eq!(study.accepted, Some(true));
}

#[test]
fn an_empty_study_has_nothing_to_size() {
    let study = run(2.0e6, &[], None);
    assert!(study.governing_name.is_none());
    assert!(!study.capacity_adequate);
    assert_eq!(
        study.messages,
        vec!["No credible positive relief demand defined"]
    );
}

#[test]
fn a_gas_case_without_a_molar_mass_leaves_the_study_unsized() {
    let mut gas = vapour("No molar mass", 2.0);
    gas.molar_mass = None;
    let study = run(2.0e6, &[gas], None);
    assert!(study.required_area.is_none());
    assert_eq!(
        study.messages,
        vec![
            "No molar mass: Missing or invalid pressure, relief demand, cause or relieving-phase \
             property basis",
            "Resolve all credible relief demands before selecting an orifice"
        ]
    );
}

#[test]
fn a_negative_rate_is_refused() {
    let bad = vapour("Negative", -1.0);
    let outcome = overpressure_protection_study(pascals(2.0e6), &[bad], None, None, None);
    assert!(outcome.is_err(), "a negative relief rate is not a load");
}
