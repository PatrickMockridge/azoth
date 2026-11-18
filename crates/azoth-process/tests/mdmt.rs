//! `process.mdmt` against `validation/neqsim/captures/process_mdmt.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{kelvins, meters};
use azoth_process::models::mdmt::{MdmtResult, mdmt};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn curve(material_curve: &str, thickness: f64, ratio: Option<f64>, minimum: f64) -> MdmtResult {
    mdmt(material_curve, meters(thickness), kelvins(minimum), ratio)
        .expect("the captured state is valid")
}

#[test]
fn the_four_curves_answer_their_captured_mdmt() {
    close(
        curve("a", 0.025, None, 233.15).mdmt.value,
        269.838_094_401_522_6,
        "curve A",
    );
    close(
        curve("b", 0.025, None, 233.15).mdmt.value,
        256.546_559_824_083_2,
        "curve B",
    );
    close(
        curve("c", 0.025, None, 233.15).mdmt.value,
        246.692_327_112_802_9,
        "curve C",
    );
    close(
        curve("d", 0.025, None, 233.15).mdmt.value,
        233.838_094_401_522_6,
        "curve D",
    );
}

#[test]
fn a_full_stress_ratio_reduction_is_55_k() {
    let result = curve("b", 0.025, Some(0.35), 193.15);
    close(result.mdmt.value, 201.546_559_824_083_2, "the reduced MDMT");
    assert!(!result.acceptable);
}

#[test]
fn the_thickness_is_clamped_to_six_and_one_hundred_mm() {
    close(
        curve("b", 0.006, None, 223.15).mdmt.value,
        225.149_999_999_999_98,
        "the lower clamp",
    );
    close(
        curve("b", 0.1, None, 293.15).mdmt.value,
        287.045_035_768_720_77,
        "the upper clamp",
    );
    close(
        curve("b", 0.15, None, 293.15).mdmt.value,
        287.045_035_768_720_77,
        "above the upper clamp",
    );
}

#[test]
fn the_cold_service_verdict_is_the_comparison() {
    assert!(curve("d", 0.025, None, 238.15).acceptable);
    assert!(!curve("d", 0.025, None, 228.15).acceptable);
}

#[test]
fn the_refusals_are_the_class_validations() {
    // An unknown curve.
    assert!(mdmt("e", meters(0.025), kelvins(233.15), None).is_err());
    // A non-positive thickness.
    assert!(mdmt("b", meters(0.0), kelvins(233.15), None).is_err());
    // A stress ratio outside (0, 1].
    assert!(mdmt("b", meters(0.025), kelvins(233.15), Some(0.0)).is_err());
    assert!(mdmt("b", meters(0.025), kelvins(233.15), Some(1.1)).is_err());
    // A non-finite operating temperature.
    assert!(mdmt("b", meters(0.025), kelvins(f64::NAN), None).is_err());
}
