//! `process.pipeline_pressure_protection` against
//! `validation/neqsim/captures/process_pipeline_pressure_protection.tsv`.

use azoth_core::units::bars;
use azoth_process::models::pipeline_pressure_protection::{
    PipelinePressureProtectionResult, pipeline_pressure_protection,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn screen(
    source: f64,
    design: f64,
    incidental: f64,
    barrier1: f64,
    barrier2: f64,
) -> PipelinePressureProtectionResult {
    pipeline_pressure_protection(
        bars(source),
        bars(design),
        incidental,
        bars(barrier1),
        bars(barrier2),
    )
    .expect("the captured pipeline state is valid")
}

#[test]
fn the_verdicts_reproduce_the_capture() {
    let fully = screen(5.0, 10.0, 1.1, 8.0, 10.0);
    close(fully.max_incidental_pressure.value, 1_100_000.0, "the MIP");
    close(fully.design_margin.value, 500_000.0, "the margin");
    assert!(fully.fully_rated && fully.protection_adequate);
    assert_eq!(fully.verdict, "FULLY_RATED");

    let two = screen(12.0, 10.0, 1.1, 8.0, 10.0);
    close(two.design_margin.value, -200_000.0, "the negative margin");
    assert!(!two.fully_rated && two.barrier1_adequate && two.barrier2_adequate);
    assert!(two.protection_adequate);
    assert_eq!(two.verdict, "TWO_BARRIERS_REQUIRED");

    let insufficient = screen(12.0, 10.0, 1.1, 11.0, 10.0);
    assert!(!insufficient.barrier1_adequate && insufficient.barrier2_adequate);
    assert!(!insufficient.protection_adequate);
    assert_eq!(insufficient.verdict, "INSUFFICIENT");

    let other = screen(12.0, 10.0, 1.2, 10.5, 11.5);
    close(
        other.max_incidental_pressure.value,
        1_200_000.0,
        "the 1.2 MIP",
    );
    assert!(!other.barrier1_adequate && other.barrier2_adequate);
    assert_eq!(other.verdict, "INSUFFICIENT");
}

#[test]
fn the_boundaries_reproduce_the_capture() {
    let at_design = screen(10.0, 10.0, 1.1, 8.0, 10.0);
    close(at_design.design_margin.value, 0.0, "the equality margin");
    assert!(at_design.fully_rated);
    assert_eq!(at_design.verdict, "FULLY_RATED");

    let at_mip = screen(12.0, 10.0, 1.1, 8.0, 11.0);
    assert!(at_mip.barrier2_adequate, "the MIP equality is adequate");
    assert_eq!(at_mip.verdict, "TWO_BARRIERS_REQUIRED");
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    assert!(
        pipeline_pressure_protection(bars(0.0), bars(10.0), 1.1, bars(8.0), bars(10.0)).is_err(),
        "a zero source pressure"
    );
    assert!(
        pipeline_pressure_protection(bars(5.0), bars(0.0), 1.1, bars(8.0), bars(10.0)).is_err(),
        "a zero design pressure"
    );
    assert!(
        pipeline_pressure_protection(bars(5.0), bars(10.0), 0.9, bars(8.0), bars(10.0)).is_err(),
        "an incidental factor below one"
    );
    assert!(
        pipeline_pressure_protection(bars(5.0), bars(10.0), 1.1, bars(-8.0), bars(10.0)).is_err(),
        "a negative barrier set point"
    );
}
