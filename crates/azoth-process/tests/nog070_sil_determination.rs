//! `process.nog070_sil_determination` against
//! `validation/neqsim/captures/process_nog070_sil_determination.tsv`.

use azoth_process::models::nog070_sil_determination::nog070_sil_determination;

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[test]
fn a_compliant_hipps_reproduces_the_capture() {
    let result = nog070_sil_determination("hipps_pipeline", 1.0e-4, None)
        .expect("the captured state is valid");
    close(result.catalogue_minimum_sil, 3.0, "the catalogue");
    close(result.minimum_sil, 3.0, "the minimum");
    close(result.achieved_sil, 3.0, "the achieved SIL");
    assert!(result.compliant, "3 meets 3");
    assert!(
        result.required_pfd_reduction_factor.is_none(),
        "a compliant SIF needs no reduction"
    );
}

#[test]
fn a_non_compliant_hipps_needs_a_factor_of_ten() {
    let result = nog070_sil_determination("hipps_pipeline", 5.0e-3, None)
        .expect("the captured state is valid");
    close(result.achieved_sil, 2.0, "the achieved SIL");
    assert!(!result.compliant, "2 misses 3");
    close(
        result.required_pfd_reduction_factor.unwrap(),
        10.0,
        "10^(3-2)",
    );
}

#[test]
fn the_custom_type_takes_the_callers_minimum() {
    let explicit =
        nog070_sil_determination("custom", 5.0e-3, Some(3.0)).expect("the captured state is valid");
    close(explicit.catalogue_minimum_sil, 0.0, "the catalogue");
    close(explicit.minimum_sil, 3.0, "the caller's minimum");
    assert!(!explicit.compliant, "2 misses 3");
    close(
        explicit.required_pfd_reduction_factor.unwrap(),
        10.0,
        "the factor",
    );

    let catalogue =
        nog070_sil_determination("custom", 5.0e-3, None).expect("the captured state is valid");
    close(catalogue.minimum_sil, 0.0, "the catalogue's zero");
    assert!(catalogue.compliant, "everything meets zero");
}

#[test]
fn the_sil_zero_band_is_distinct_from_the_required_sil_map() {
    let result = nog070_sil_determination("psd_high_vibration", 0.2, None)
        .expect("the captured state is valid");
    close(result.achieved_sil, 0.0, "pfdToSil reads 0 at 0.2");
    assert!(
        !result.compliant,
        "0 misses the high-vibration minimum of 1"
    );
    close(
        result.required_pfd_reduction_factor.unwrap(),
        10.0,
        "the factor",
    );
}

#[test]
fn the_pfd_to_sil_boundaries_reproduce_the_capture() {
    let boundaries = [
        (0.1, 0.0),
        (0.0999, 1.0),
        (0.01, 1.0),
        (0.009, 2.0),
        (0.001, 2.0),
        (0.0009, 3.0),
        (1.0e-4, 3.0),
        (9.9e-5, 4.0),
    ];
    for (pfd, expected) in boundaries {
        let result = nog070_sil_determination("custom", pfd, Some(0.0))
            .expect("the captured state is valid");
        close(result.achieved_sil, expected, "the boundary");
    }
}

#[test]
fn the_catalogue_is_the_classes_eighteen_rows() {
    let catalogue = [
        ("psd_process_segment", 2.0),
        ("esd_topside_isolation", 2.0),
        ("esd_subsea_isolation", 3.0),
        ("hipps_pipeline", 3.0),
        ("blowdown_hydrocarbon_segment", 2.0),
        ("fg_gas_detection_esd", 2.0),
        ("fg_fire_detection_esd", 2.0),
        ("fg_hvac_shutdown", 2.0),
        ("fg_deluge_release", 2.0),
        ("psd_compressor_anti_surge", 2.0),
        ("psd_high_vibration", 1.0),
        ("psd_high_level", 2.0),
        ("psd_low_low_level", 2.0),
        ("psd_burner_management", 2.0),
        ("esd_riser", 3.0),
        ("esd_wellhead", 2.0),
        ("esd_loading_release", 2.0),
        ("custom", 0.0),
    ];
    for (sif_type, expected) in catalogue {
        let result =
            nog070_sil_determination(sif_type, 1.0e-3, None).expect("the captured state is valid");
        close(result.catalogue_minimum_sil, expected, sif_type);
    }
}

#[test]
fn the_class_guards_are_reproduced() {
    assert!(
        nog070_sil_determination("hipps_pipeline", 0.0, None).is_err(),
        "a zero PFD"
    );
    assert!(
        nog070_sil_determination("hipps_pipeline", 1.0, None).is_err(),
        "a PFD of one"
    );
    assert!(
        nog070_sil_determination("hipps_pipeline", 1.5, None).is_err(),
        "a PFD above one"
    );
    assert!(
        nog070_sil_determination("hipps_pipeline", f64::NAN, None).is_err(),
        "a NaN PFD"
    );
    assert!(
        nog070_sil_determination("custom", 5.0e-3, Some(-1.0)).is_err(),
        "a negative minimum"
    );
    assert!(
        nog070_sil_determination("custom", 5.0e-3, Some(5.0)).is_err(),
        "a minimum above four"
    );
    assert!(
        nog070_sil_determination("not_a_sif", 5.0e-3, None).is_err(),
        "a type the catalogue does not name"
    );
}
