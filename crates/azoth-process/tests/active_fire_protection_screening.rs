//! `process.active_fire_protection_screening` against
//! `validation/neqsim/captures/process_active_fire_protection_screening.tsv`.

use azoth_core::units::{liters_per_minute, seconds};
use azoth_process::models::active_fire_protection_screening::{
    ActiveFireProtectionScreeningResult, active_fire_protection_screening,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn screen(
    scenario: &str,
    blowdown: f64,
    bare: f64,
    dedicated: f64,
    general: f64,
) -> ActiveFireProtectionScreeningResult {
    active_fire_protection_screening(
        scenario,
        seconds(blowdown),
        seconds(bare),
        liters_per_minute(dedicated),
        liters_per_minute(general),
    )
    .expect("the captured state is valid")
}

#[test]
fn a_gas_jet_fire_is_not_extinguishable_but_blowdown_wins() {
    let result = screen("gas_jet_fire", 600.0, 900.0, 170.0, 5100.0);
    assert!(!result.water_extinguishing);
    assert!(result.blowdown_faster_than_failure);
    close(result.blowdown_margin.value, 300.0, "the blowdown margin");
    close(
        result.selective_protection_saving_fraction,
        1.0 - 170.0 / 5100.0,
        "the selective saving",
    );
}

#[test]
fn a_liquid_pool_fire_is_extinguishable() {
    let result = screen("liquid_pool_fire", 600.0, 900.0, 170.0, 5100.0);
    assert!(result.water_extinguishing);
}

#[test]
fn a_two_phase_jet_fire_is_not_extinguishable() {
    let result = screen("two_phase_jet_fire", 600.0, 900.0, 170.0, 5100.0);
    assert!(!result.water_extinguishing);
}

#[test]
fn an_ordinary_combustible_fire_is_extinguishable() {
    let result = screen("ordinary_combustible_fire", 600.0, 900.0, 170.0, 5100.0);
    assert!(result.water_extinguishing);
}

#[test]
fn a_slower_blowdown_leaves_a_negative_margin() {
    let result = screen("gas_jet_fire", 1200.0, 400.0, 170.0, 5100.0);
    assert!(!result.blowdown_faster_than_failure);
    close(result.blowdown_margin.value, -800.0, "the blowdown margin");
}

#[test]
fn an_unknown_scenario_is_refused() {
    let result = active_fire_protection_screening(
        "pool_fire",
        seconds(600.0),
        seconds(900.0),
        liters_per_minute(170.0),
        liters_per_minute(5100.0),
    );
    assert!(result.is_err());
}

#[test]
fn the_range_guards_are_the_class_constructor_and_setters() {
    assert!(
        active_fire_protection_screening(
            "gas_jet_fire",
            seconds(0.0),
            seconds(900.0),
            liters_per_minute(170.0),
            liters_per_minute(5100.0),
        )
        .is_err()
    );
    assert!(
        active_fire_protection_screening(
            "gas_jet_fire",
            seconds(600.0),
            seconds(0.0),
            liters_per_minute(170.0),
            liters_per_minute(5100.0),
        )
        .is_err()
    );
    assert!(
        active_fire_protection_screening(
            "gas_jet_fire",
            seconds(600.0),
            seconds(900.0),
            liters_per_minute(-1.0),
            liters_per_minute(5100.0),
        )
        .is_err()
    );
    assert!(
        active_fire_protection_screening(
            "gas_jet_fire",
            seconds(600.0),
            seconds(900.0),
            liters_per_minute(170.0),
            liters_per_minute(0.0),
        )
        .is_err()
    );
    assert!(
        active_fire_protection_screening(
            "gas_jet_fire",
            seconds(f64::NAN),
            seconds(900.0),
            liters_per_minute(170.0),
            liters_per_minute(5100.0),
        )
        .is_err()
    );
}
