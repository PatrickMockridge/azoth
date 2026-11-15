//! `process.lopa` against `validation/neqsim/captures/process_lopa.tsv`.

use azoth_core::units::bars;
use azoth_process::models::lopa::{LopaProtectionLayer, LopaResult, Sts0131PressureCategory, lopa};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn layer(name: &str, pfd: f64, before: f64, after: f64) -> LopaProtectionLayer {
    LopaProtectionLayer {
        name: name.to_string(),
        pfd,
        frequency_before_per_year: before,
        frequency_after_per_year: after,
    }
}

fn call(
    initiating: f64,
    target: f64,
    mitigated: f64,
    layers: &[LopaProtectionLayer],
) -> LopaResult {
    try_call(initiating, target, mitigated, layers).expect("the captured state is valid")
}

fn try_call(
    initiating: f64,
    target: f64,
    mitigated: f64,
    layers: &[LopaProtectionLayer],
) -> azoth_core::Result<LopaResult> {
    lopa(initiating, target, mitigated, layers, None, None, None)
}

#[test]
fn the_target_met_state_reproduces_the_capture() {
    let result = call(
        0.1,
        1.1e-4,
        1.000_000_000_000_000_2e-4,
        &[
            layer("L0", 0.1, 0.1, 0.010_000_000_000_000_002),
            layer(
                "L1",
                0.01,
                0.010_000_000_000_000_002,
                1.000_000_000_000_000_2e-4,
            ),
        ],
    );
    assert!(result.target_met, "the target is met");
    close(
        result.gap_to_target,
        -9.999_999_999_999_986e-6,
        "the signed gap",
    );
    close(result.total_rrf, 999.999_999_999_999_9, "the total RRF");
    close(
        result.required_additional_rrf.unwrap(),
        0.0,
        "no additional RRF",
    );
    close(result.required_additional_sil, 0.0, "no additional SIL");
    assert_eq!(result.layer_contributions.len(), 2);
    close(
        result.layer_contributions[0].contribution_per_year,
        0.09,
        "the first contribution",
    );
    close(
        result.layer_contributions[1].contribution_per_year,
        0.009_900_000_000_000_003,
        "the second contribution",
    );
    close(
        result.layer_contributions[1].rrf.unwrap(),
        100.0,
        "L1's RRF",
    );
}

#[test]
fn the_gap_state_reproduces_the_capture() {
    let result = call(0.5, 1.0e-5, 0.05, &[layer("L0", 0.1, 0.5, 0.05)]);
    assert!(!result.target_met, "the target is not met");
    close(result.gap_to_target, 0.049_99, "the gap");
    close(result.total_rrf, 10.0, "the total RRF");
    close(
        result.required_additional_rrf.unwrap(),
        5000.0,
        "the required RRF",
    );
    close(result.required_additional_sil, 3.0, "the required SIL");
}

#[test]
fn the_boundary_is_inclusive() {
    let result = call(0.5, 0.05, 0.05, &[layer("L0", 0.1, 0.5, 0.05)]);
    assert!(result.target_met, "mitigated == target is met");
    close(result.gap_to_target, 0.0, "the gap is zero");
    close(result.required_additional_rrf.unwrap(), 0.0, "no RRF");
}

#[test]
fn a_zero_mitigated_frequency_has_a_zero_rrf() {
    let result = call(0.1, 1.0e-4, 0.0, &[layer("L0", 0.0, 0.1, 0.0)]);
    assert!(result.target_met, "zero is below the target");
    close(result.total_rrf, 0.0, "the class's zero");
    assert!(
        result.layer_contributions[0].rrf.is_none(),
        "a zero PFD's RRF is absent"
    );
}

#[test]
fn a_zero_target_that_is_not_met_has_no_required_rrf() {
    let result = call(
        0.1,
        0.0,
        0.010_000_000_000_000_002,
        &[layer("L0", 0.1, 0.1, 0.010_000_000_000_000_002)],
    );
    assert!(!result.target_met, "a positive frequency misses zero");
    assert!(
        result.required_additional_rrf.is_none(),
        "the class's +Infinity is absent"
    );
    close(result.required_additional_sil, 4.0, "the SIL of zero");
}

#[test]
fn the_sts0131_bands_and_their_boundaries() {
    let band = |event: f64, design: f64, test: f64| {
        lopa(
            0.1,
            1.0e-4,
            1.0e-4,
            &[],
            Some(bars(event)),
            Some(bars(design)),
            Some(bars(test)),
        )
        .expect("the captured state is valid")
    };
    let below = band(5.0, 10.0, 15.0);
    assert_eq!(
        below.sts0131_pressure_category,
        Some(Sts0131PressureCategory::BelowOrAtDesignPressure)
    );
    close(
        below.sts0131_target_frequency_per_year.unwrap(),
        1.0e-2,
        "the below-design target",
    );
    let above_design = band(12.0, 10.0, 15.0);
    assert_eq!(
        above_design.sts0131_pressure_category,
        Some(Sts0131PressureCategory::AboveDesignToTestPressure)
    );
    close(
        above_design.sts0131_target_frequency_per_year.unwrap(),
        1.0e-3,
        "the above-design target",
    );
    let above_test = band(18.0, 10.0, 15.0);
    assert_eq!(
        above_test.sts0131_pressure_category,
        Some(Sts0131PressureCategory::AboveTestToTwoTimesDesignPressure)
    );
    close(
        above_test.sts0131_target_frequency_per_year.unwrap(),
        1.0e-4,
        "the above-test target",
    );
    let above_two = band(21.0, 10.0, 15.0);
    assert_eq!(
        above_two.sts0131_pressure_category,
        Some(Sts0131PressureCategory::AboveTwoTimesDesignPressure)
    );
    close(
        above_two.sts0131_target_frequency_per_year.unwrap(),
        1.0e-5,
        "the above-two-times target",
    );
    // The exact boundaries are inclusive on the lower side: events equal to the test
    // pressure and to twice the design pressure fall in the lower band.
    assert_eq!(
        band(15.0, 10.0, 15.0).sts0131_pressure_category,
        Some(Sts0131PressureCategory::AboveDesignToTestPressure)
    );
    assert_eq!(
        band(20.0, 10.0, 15.0).sts0131_pressure_category,
        Some(Sts0131PressureCategory::AboveTestToTwoTimesDesignPressure)
    );
    assert!(
        band(5.0, 10.0, 15.0).sts0131_pressure_category.is_some(),
        "the triple is present"
    );
}

#[test]
fn the_pressure_triple_is_all_or_nothing_and_the_class_guards_hold() {
    assert!(
        lopa(0.1, 1.0e-4, 1.0e-4, &[], Some(bars(5.0)), None, None).is_err(),
        "a partial triple"
    );
    assert!(
        lopa(
            0.1,
            1.0e-4,
            1.0e-4,
            &[],
            Some(bars(0.0)),
            Some(bars(10.0)),
            Some(bars(15.0))
        )
        .is_err(),
        "a zero event pressure"
    );
    assert!(
        lopa(
            0.1,
            1.0e-4,
            1.0e-4,
            &[],
            Some(bars(5.0)),
            Some(bars(10.0)),
            Some(bars(9.0))
        )
        .is_err(),
        "a test pressure below design"
    );
}

#[test]
fn the_values_the_class_accepts_are_refused() {
    assert!(
        try_call(-0.1, 1.0e-4, 1.0e-4, &[]).is_err(),
        "a negative initiating frequency"
    );
    assert!(
        try_call(0.1, -1.0e-4, 1.0e-4, &[]).is_err(),
        "a negative target"
    );
    assert!(
        try_call(0.1, 1.0e-4, -1.0e-4, &[]).is_err(),
        "a negative mitigated frequency"
    );
    assert!(
        try_call(0.1, 1.0e-4, 1.0e-4, &[layer("L0", 1.5, 0.1, 0.01)]).is_err(),
        "a PFD above one"
    );
    assert!(
        try_call(0.1, 1.0e-4, 1.0e-4, &[layer("L0", 0.1, -0.1, 0.01)]).is_err(),
        "a negative layer frequency"
    );
}
