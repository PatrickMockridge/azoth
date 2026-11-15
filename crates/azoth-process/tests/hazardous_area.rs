//! `process.hazardous_area` against
//! `validation/neqsim/captures/process_hazardous_area.tsv`.

use azoth_core::units::{
    kilograms_per_cubic_meter, kilograms_per_mole, kilograms_per_second, meters_per_second,
};
use azoth_process::models::hazardous_area::{HazardousAreaResult, hazardous_area};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)] // one parameter per declared input
fn area(
    flow: f64,
    density: f64,
    velocity: f64,
    lfl: f64,
    molar_mass: f64,
    ambient: Option<f64>,
    factor: Option<f64>,
    grade: Option<&str>,
) -> azoth_core::Result<HazardousAreaResult> {
    hazardous_area(
        kilograms_per_second(flow),
        kilograms_per_cubic_meter(density),
        meters_per_second(velocity),
        lfl,
        kilograms_per_mole(molar_mass),
        ambient.map(kilograms_per_cubic_meter),
        factor,
        grade,
    )
}

#[test]
fn the_documentation_state_reproduces() {
    let result = area(0.1, 6.0, 340.0, 0.044, 0.01604, None, None, None)
        .expect("the capture state is valid");
    close(result.effective_diameter.value, 0.007_900_234_374_579_8, "d_e");
    close(result.lfl_mass_fraction, 0.024_854_779_008_840_32, "Y_LFL");
    close(result.hazardous_distance.value, 7.597_337_258_328_827, "the distance");
    assert_eq!(result.release_grade, "SECONDARY");
    assert_eq!(result.zone_classification, "Zone 2");
}

#[test]
fn the_safety_factor_and_ambient_density_overrides() {
    let tighter = area(0.1, 6.0, 340.0, 0.044, 0.01604, None, Some(0.25), None)
        .expect("the capture state is valid");
    close(tighter.hazardous_distance.value, 15.194_674_516_657_654, "k = 0.25");

    let ambient = area(0.1, 6.0, 340.0, 0.044, 0.01604, Some(1.0), None, None)
        .expect("the capture state is valid");
    close(ambient.hazardous_distance.value, 8.408_711_461_072_347, "rho_a = 1.0");
}

#[test]
fn a_tenfold_flow_scales_the_diameter_and_the_distance() {
    let result = area(1.0, 6.0, 340.0, 0.044, 0.01604, None, None, None)
        .expect("the capture state is valid");
    close(result.effective_diameter.value, 0.024_982_734_672_828_01, "d_e");
    close(result.hazardous_distance.value, 24.024_889_888_778_134, "the distance");
}

#[test]
fn the_grade_maps_to_the_class_zone() {
    let primary = area(0.1, 6.0, 340.0, 0.044, 0.01604, None, None, Some("primary"))
        .expect("a primary grade is valid");
    assert_eq!(primary.release_grade, "PRIMARY");
    assert_eq!(primary.zone_classification, "Zone 1");
    close(primary.hazardous_distance.value, 7.597_337_258_328_827, "the unchanged distance");

    let continuous = area(0.1, 6.0, 340.0, 0.044, 0.01604, None, None, Some("continuous"))
        .expect("a continuous grade is valid");
    assert_eq!(continuous.release_grade, "CONTINUOUS");
    assert_eq!(continuous.zone_classification, "Zone 0");
}

#[test]
fn the_guards_are_refusals() {
    for (flow, density, velocity, lfl, molar_mass) in [
        (0.0, 6.0, 340.0, 0.044, 0.01604),
        (0.1, 0.0, 340.0, 0.044, 0.01604),
        (0.1, 6.0, 0.0, 0.044, 0.01604),
        (0.1, 6.0, 340.0, 0.0, 0.01604),
        (0.1, 6.0, 340.0, 1.0, 0.01604),
        (0.1, 6.0, 340.0, 0.044, 0.0),
        (f64::NAN, 6.0, 340.0, 0.044, 0.01604),
    ] {
        assert!(
            area(flow, density, velocity, lfl, molar_mass, None, None, None).is_err(),
            "({flow}, {density}, {velocity}, {lfl}, {molar_mass}) is not a valid state"
        );
    }
    assert!(area(0.1, 6.0, 340.0, 0.044, 0.01604, Some(0.0), None, None).is_err());
    assert!(area(0.1, 6.0, 340.0, 0.044, 0.01604, None, Some(0.0), None).is_err());
    assert!(area(0.1, 6.0, 340.0, 0.044, 0.01604, None, Some(1.1), None).is_err());
    assert!(area(0.1, 6.0, 340.0, 0.044, 0.01604, None, None, Some("major")).is_err());
}
