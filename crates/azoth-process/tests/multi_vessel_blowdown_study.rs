//! `process.multi_vessel_blowdown_study` against
//! `validation/neqsim/captures/process_multi_vessel_blowdown_study.tsv`.

#![allow(clippy::excessive_precision)]

use azoth_core::units::{
    kelvins, kilograms_per_mole, kilograms_per_second, meters, pascals, seconds,
};
use azoth_process::models::multi_vessel_blowdown_study::{
    MultiVesselSample, multi_vessel_blowdown_study,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn samples() -> Vec<MultiVesselSample> {
    let rows = [
        ("HP-sep", 0.0, 40.0),
        ("HP-sep", 1.0, 32.0),
        ("HP-sep", 2.0, 24.0),
        ("HP-sep", 3.0, 16.0),
        ("HP-sep", 4.0, 7.999999999999998),
        ("HP-sep", 5.0, 0.0),
        ("Inlet-sep", 0.0, 25.0),
        ("Inlet-sep", 1.0, 16.666666666666668),
        ("Inlet-sep", 2.0, 8.333333333333334),
        ("Inlet-sep", 3.0, 0.0),
    ];
    rows.iter()
        .map(|(source, time, flow)| MultiVesselSample {
            source: (*source).to_string(),
            time: seconds(*time),
            mass_flow: kilograms_per_second(*flow),
        })
        .collect()
}

#[test]
fn the_superposition_and_header_read_the_capture() {
    let no_header =
        multi_vessel_blowdown_study(&samples(), None, None, None, None, None, None, None)
            .expect("the captured state is valid");
    close(no_header.peak_total_mass_flow.value, 65.0, "peak total");
    close(no_header.peak_time.value, 0.0, "peak time");
    assert_eq!(no_header.profile.len(), 6);
    close(
        no_header.profile[1].total_mass_flow.value,
        48.66666666666667,
        "grid 1",
    );
    close(no_header.profile[3].total_mass_flow.value, 16.0, "grid 3");
    close(
        no_header.contributions[0].peak_contribution.value,
        40.0,
        "HP contribution",
    );
    close(
        no_header.contributions[1].peak_contribution.value,
        25.0,
        "Inlet contribution",
    );
    assert!(no_header.header_velocity.is_none());
    assert!(no_header.header_mach.is_none());
    assert!(no_header.header_mach_acceptable);

    let small = multi_vessel_blowdown_study(
        &samples(),
        Some(meters(0.2)),
        Some(pascals(1.5e5)),
        Some(kelvins(288.15)),
        Some(kilograms_per_mole(0.020)),
        Some(1.30),
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        small.header_velocity.expect("a header").value,
        1652.231406916753,
        "small header velocity",
    );
    close(
        small.header_mach.expect("a header"),
        4.186970229513444,
        "small header Mach",
    );
    assert!(!small.header_mach_acceptable);

    let large = multi_vessel_blowdown_study(
        &samples(),
        Some(meters(0.8)),
        Some(pascals(1.5e5)),
        Some(kelvins(288.15)),
        Some(kilograms_per_mole(0.020)),
        Some(1.30),
        None,
        None,
    )
    .expect("the captured state is valid");
    close(
        large.header_velocity.expect("a header").value,
        103.26446293229706,
        "large header velocity",
    );
    close(
        large.header_mach.expect("a header"),
        0.26168563934459027,
        "large header Mach",
    );
    assert!(large.header_mach_acceptable);
}

#[test]
fn the_refusals_are_the_class_guards() {
    assert!(multi_vessel_blowdown_study(&[], None, None, None, None, None, None, None).is_err());
    let mut bad = samples();
    bad[1].time = seconds(0.0);
    assert!(multi_vessel_blowdown_study(&bad, None, None, None, None, None, None, None).is_err());
    assert!(
        multi_vessel_blowdown_study(
            &samples(),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(seconds(0.0)),
        )
        .is_err()
    );
}
