//! `process.gas_blowby` against `validation/neqsim/captures/process_gas_blowby.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{
    kelvins, kilograms_per_mole, kilograms_per_second, pascals, square_meters,
};
use azoth_process::models::gas_blowby::{BlowbyVerdict, GasBlowbyResult, gas_blowby};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn raw(
    p1: f64,
    t1: f64,
    p2: f64,
    area: f64,
    k: f64,
    m: f64,
    cd: Option<f64>,
    capacity: Option<f64>,
) -> azoth_core::Result<GasBlowbyResult> {
    gas_blowby(
        pascals(p1),
        kelvins(t1),
        pascals(p2),
        square_meters(area),
        k,
        kilograms_per_mole(m),
        cd,
        capacity.map(kilograms_per_second),
    )
}

#[allow(clippy::too_many_arguments)]
fn blowby(
    p1: f64,
    t1: f64,
    p2: f64,
    area: f64,
    k: f64,
    m: f64,
    cd: Option<f64>,
    capacity: Option<f64>,
) -> GasBlowbyResult {
    raw(p1, t1, p2, area, k, m, cd, capacity).expect("the captured state is valid")
}

#[test]
fn a_choked_blowby_is_absorbed_by_its_relief_capacity() {
    let result = blowby(
        1.0e6,
        300.0,
        5.0e5,
        1.0e-4,
        1.3,
        0.020,
        Some(0.85),
        Some(0.277_777_777_777_777_8),
    );
    assert!(result.choked);
    close(
        result.critical_pressure_ratio,
        0.545_727_733_814_065,
        "the critical ratio",
    );
    close(result.actual_pressure_ratio, 0.5, "the actual ratio");
    close(
        result.blowby_mass_rate.value,
        0.160_602_693_677_397_2,
        "the mass rate",
    );
    close(
        result.blowby_std_vol_rate.value,
        0.189_871_169_764_627_15,
        "the standard volume rate",
    );
    close(
        result.relief_margin.expect("a stated capacity").value,
        0.117_175_084_100_380_58,
        "the relief margin",
    );
    assert!(result.relief_adequate);
    assert_eq!(result.verdict, BlowbyVerdict::ReliefAdequate);
}

#[test]
fn a_subcritical_blowby_is_inadequate() {
    let result = blowby(
        1.0e6,
        300.0,
        7.0e5,
        1.0e-4,
        1.3,
        0.020,
        None,
        Some(0.027_777_777_777_777_776),
    );
    assert!(!result.choked);
    close(result.actual_pressure_ratio, 0.7, "the actual ratio");
    close(
        result.blowby_mass_rate.value,
        0.151_382_889_728_008_62,
        "the mass rate",
    );
    close(
        result.blowby_std_vol_rate.value,
        0.178_971_134_897_295_94,
        "the standard volume rate",
    );
    close(
        result.relief_margin.expect("a stated capacity").value,
        -0.123_605_111_950_230_84,
        "the relief margin",
    );
    assert!(!result.relief_adequate);
    assert_eq!(result.verdict, BlowbyVerdict::ReliefInadequate);
}

#[test]
fn the_critical_ratio_itself_is_choked() {
    let result = blowby(
        1.0e6,
        300.0,
        545_727.733_814_065,
        1.0e-4,
        1.3,
        0.020,
        None,
        Some(0.277_777_777_777_777_8),
    );
    assert!(result.choked);
    close(
        result.actual_pressure_ratio,
        0.545_727_733_814_065,
        "the boundary ratio",
    );
    close(
        result.blowby_mass_rate.value,
        0.160_602_693_677_397_2,
        "the choked rate",
    );
}

#[test]
fn a_hair_above_the_critical_ratio_is_subcritical() {
    let result = blowby(
        1.0e6,
        300.0,
        545_727.833_814_065,
        1.0e-4,
        1.3,
        0.020,
        None,
        Some(0.277_777_777_777_777_8),
    );
    assert!(!result.choked);
    close(
        result.actual_pressure_ratio,
        0.545_727_833_814_065_1,
        "the just-above ratio",
    );
    close(
        result.blowby_mass_rate.value,
        0.160_602_693_677_393_5,
        "the subcritical rate",
    );
    close(
        result.blowby_std_vol_rate.value,
        0.189_871_169_764_622_73,
        "the standard volume rate",
    );
}

#[test]
fn a_zero_margin_is_adequate() {
    let result = blowby(
        1.0e6,
        300.0,
        5.0e5,
        1.0e-4,
        1.3,
        0.020,
        None,
        Some(0.160_602_693_677_397_2),
    );
    assert_eq!(result.relief_margin.expect("a stated capacity").value, 0.0);
    assert!(result.relief_adequate);
    assert_eq!(result.verdict, BlowbyVerdict::ReliefAdequate);
}

#[test]
fn no_capacity_publishes_no_relief_data() {
    let result = blowby(1.0e6, 300.0, 5.0e5, 1.0e-4, 1.3, 0.020, None, None);
    assert!(result.relief_margin.is_none());
    assert!(!result.relief_adequate);
    assert_eq!(result.verdict, BlowbyVerdict::NoReliefData);
}

#[test]
fn the_refusals_are_the_class_validations() {
    // A downstream pressure at or above the source.
    assert!(raw(1.0e6, 300.0, 1.0e6, 1.0e-4, 1.3, 0.020, None, None).is_err());
    // A specific-heat ratio at or below one.
    assert!(raw(1.0e6, 300.0, 5.0e5, 1.0e-4, 1.0, 0.020, None, None).is_err());
    // A non-positive opening.
    assert!(raw(1.0e6, 300.0, 5.0e5, 0.0, 1.3, 0.020, None, None).is_err());
    // A discharge coefficient outside (0, 1].
    assert!(raw(1.0e6, 300.0, 5.0e5, 1.0e-4, 1.3, 0.020, Some(1.1), None).is_err());
    // A negative relief capacity.
    assert!(raw(1.0e6, 300.0, 5.0e5, 1.0e-4, 1.3, 0.020, None, Some(-1.0)).is_err());
    // A non-finite temperature.
    assert!(raw(1.0e6, f64::NAN, 5.0e5, 1.0e-4, 1.3, 0.020, None, None).is_err());
    // A non-positive molar mass.
    assert!(raw(1.0e6, 300.0, 5.0e5, 1.0e-4, 1.3, 0.0, None, None).is_err());
}
