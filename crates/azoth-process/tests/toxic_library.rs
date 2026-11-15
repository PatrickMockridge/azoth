//! `process.toxic_library` against
//! `validation/neqsim/captures/process_toxic_library.tsv`.

use azoth_core::units::{kelvins, kilograms_per_mole, pascals};
use azoth_process::models::toxic_library::{ToxicLibraryResult, toxic_library};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn row(chemical: &str) -> ToxicLibraryResult {
    toxic_library(Some(chemical), None, None, None, None).expect("a threshold lookup is a state")
}

fn convert_raw(
    ppm: f64,
    molar: f64,
    temperature: f64,
    pressure: f64,
) -> azoth_core::Result<ToxicLibraryResult> {
    toxic_library(
        None,
        Some(ppm),
        Some(kilograms_per_mole(molar)),
        Some(kelvins(temperature)),
        Some(pascals(pressure)),
    )
}

fn convert(ppm: f64, molar: f64, temperature: f64, pressure: f64) -> ToxicLibraryResult {
    convert_raw(ppm, molar, temperature, pressure).expect("a complete conversion group is a state")
}

#[test]
fn the_ten_rows_are_the_class_table() {
    let rows = [
        ("H2S", "Hydrogen sulfide", 100.0, 30.0, 27.0),
        ("CO", "Carbon monoxide", 1200.0, 350.0, 83.0),
        ("Cl2", "Chlorine", 10.0, 3.0, 2.0),
        ("NH3", "Ammonia", 300.0, 150.0, 160.0),
        ("SO2", "Sulfur dioxide", 100.0, 3.0, 0.75),
        ("HCN", "Hydrogen cyanide", 50.0, 10.0, 7.1),
        ("HF", "Hydrogen fluoride", 30.0, 20.0, 24.0),
        ("HCl", "Hydrogen chloride", 50.0, 20.0, 22.0),
        ("methanol", "Methanol", 6000.0, 1000.0, 530.0),
        ("benzene", "Benzene", 500.0, 150.0, 800.0),
    ];
    for (chemical, name, idlh, erpg2, aegl2) in rows {
        let result = row(chemical);
        assert_eq!(result.known, Some(true), "{chemical}");
        assert_eq!(result.name.as_deref(), Some(name));
        close(result.idlh_ppm.expect("known"), idlh, "IDLH");
        close(result.erpg2_ppm.expect("known"), erpg2, "ERPG-2");
        close(result.aegl2_ppm.expect("known"), aegl2, "AEGL-2");
    }
}

#[test]
fn the_aliases_and_the_lowercase_names_resolve() {
    assert_eq!(
        row("hydrogen sulfide").name.as_deref(),
        Some("Hydrogen sulfide")
    );
    assert_eq!(
        row("carbon monoxide").name.as_deref(),
        Some("Carbon monoxide")
    );
    assert_eq!(row("chlorine").name.as_deref(), Some("Chlorine"));
    assert_eq!(row("ammonia").name.as_deref(), Some("Ammonia"));
    assert_eq!(
        row("sulfur dioxide").name.as_deref(),
        Some("Sulfur dioxide")
    );
    // The two lowercase names are case-insensitive because their keys are lowercase.
    assert_eq!(row("METHANOL").name.as_deref(), Some("Methanol"));
    assert_eq!(row("Benzene").name.as_deref(), Some("Benzene"));
}

#[test]
fn the_measured_case_sensitivity_is_reproduced() {
    // The javadoc says case-insensitive; the code is exact-case for the formula keys.
    for chemical in [
        "h2s", "H2s", "co", "cl2", "nh3", "so2", "hcn", "Hcn", "hf", "hcl",
    ] {
        assert_eq!(row(chemical).known, Some(false), "{chemical} must miss");
        assert!(row(chemical).name.is_none(), "{chemical} has no name");
    }
    assert_eq!(row("H2S").known, Some(true));
    assert_eq!(row("HCN").known, Some(true));
}

#[test]
fn an_unknown_or_blank_name_is_a_false_not_a_refusal() {
    for chemical in ["argon", "", "  "] {
        let result = row(chemical);
        assert_eq!(result.known, Some(false), "{chemical:?}");
        assert!(result.name.is_none());
        assert!(result.idlh_ppm.is_none());
    }
}

#[test]
fn the_conversion_is_the_class_r8_314() {
    let first = convert(100.0, 0.0340819, 288.15, 101_325.0);
    assert!(first.known.is_none(), "no chemical was named");
    close(
        first.mass_concentration.expect("a complete group").value,
        1.441_490_438_974_068e-4,
        "101325 Pa",
    );
    let second = convert(100.0, 0.0340819, 300.0, 100_000.0);
    close(
        second.mass_concentration.expect("a complete group").value,
        1.366_446_155_079_785_2e-4,
        "1 bar",
    );
}

#[test]
fn the_guards_are_refusals() {
    assert!(toxic_library(None, None, None, None, None).is_err());
    assert!(
        toxic_library(
            None,
            Some(100.0),
            Some(kilograms_per_mole(0.0340819)),
            Some(kelvins(288.15)),
            None,
        )
        .is_err(),
        "a conversion group without a pressure is refused"
    );
    assert!(convert_raw(-100.0, 0.0340819, 288.15, 101_325.0).is_err());
    assert!(convert_raw(100.0, 0.0340819, 0.0, 101_325.0).is_err());
    assert!(convert_raw(100.0, 0.0340819, 288.15, 0.0).is_err());
    assert!(convert_raw(100.0, 0.0, 288.15, 101_325.0).is_err());
    assert!(convert_raw(f64::NAN, 0.0340819, 288.15, 101_325.0).is_err());
}
