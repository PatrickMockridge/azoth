//! `process.fire_case_relief` against
//! `validation/neqsim/captures/process_fire_case_relief.tsv`.

use azoth_core::units::{kelvins, kilogram_per_hour, meters, square_meters};
use azoth_process::models::fire_case_relief::{FireCaseReliefResult, fire_case_relief};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn relief(
    drainage: bool,
    firefighting: bool,
    area: Option<f64>,
    diameter: Option<f64>,
    height: Option<f64>,
    latent: Option<f64>,
    temperature: Option<f64>,
) -> FireCaseReliefResult {
    fire_case_relief(
        area.map(square_meters),
        diameter.map(meters),
        height.map(meters),
        drainage,
        firefighting,
        latent,
        temperature.map(kelvins),
    )
    .expect("the captured fire state is valid")
}

#[test]
fn the_drainage_branches_reproduce_the_capture() {
    let drainage = relief(
        true,
        true,
        Some(100.0),
        None,
        None,
        Some(300_000.0),
        Some(350.0),
    );
    close(drainage.latent_heat, 300_000.0, "the latent heat");
    close(
        drainage.relief_rate.value,
        6.284_722_503_339_891,
        "the rate",
    );
    close(
        drainage.relief_rate_kg_per_hr.get::<kilogram_per_hour>(),
        22_625.001_012_023_607,
        "the hourly rate",
    );
    close(
        drainage.relieving_temperature.expect("stated").value,
        350.0,
        "the relieving temperature",
    );

    let no_drainage = relief(
        false,
        true,
        Some(100.0),
        None,
        None,
        Some(300_000.0),
        Some(350.0),
    );
    close(
        no_drainage.relief_rate.value,
        10.324_901_255_486_964,
        "the no-drainage rate",
    );
    close(
        no_drainage.relief_rate_kg_per_hr.get::<kilogram_per_hour>(),
        37_169.644_519_753_07,
        "the no-drainage hourly rate",
    );
}

#[test]
fn the_geometry_and_defaults_reproduce_the_capture() {
    let geometry = relief(true, true, None, Some(2.0), Some(5.0), None, None);
    close(geometry.latent_heat, 350_000.0, "the default latent heat");
    close(
        geometry.relief_rate.value,
        2.084_501_502_569_984_2,
        "the geometry rate",
    );
    assert!(
        geometry.relieving_temperature.is_none(),
        "no temperature stated"
    );

    let no_area = relief(true, true, None, None, None, None, None);
    close(no_area.relief_rate.value, 0.0, "the no-area rate");
    close(
        no_area.relief_rate_kg_per_hr.value,
        0.0,
        "the no-area hourly rate",
    );
    close(no_area.latent_heat, 350_000.0, "the default latent heat");

    let default_latent = relief(true, true, Some(100.0), None, None, None, None);
    close(
        default_latent.latent_heat,
        350_000.0,
        "the default latent heat",
    );
    close(
        default_latent.relief_rate.value,
        5.386_905_002_862_764,
        "the default-latent rate",
    );
}

#[test]
fn the_firefighting_flag_selects_the_coefficient() {
    let credited = relief(
        true,
        true,
        Some(100.0),
        None,
        None,
        Some(300_000.0),
        Some(350.0),
    );
    let uncredited = relief(
        true,
        false,
        Some(100.0),
        None,
        None,
        Some(300_000.0),
        Some(350.0),
    );
    close(
        uncredited.relief_rate.value / credited.relief_rate.value,
        34_500.0 / 21_000.0,
        "the uncredited rate is 34500/21000 of the credited one",
    );
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    assert!(
        fire_case_relief(
            Some(square_meters(-1.0)),
            None,
            None,
            true,
            true,
            None,
            None,
        )
        .is_err(),
        "a negative wetted area"
    );
    assert!(
        fire_case_relief(
            Some(square_meters(100.0)),
            None,
            None,
            true,
            true,
            Some(-1.0),
            None,
        )
        .is_err(),
        "a negative latent heat the class silently defaults"
    );
    assert!(
        fire_case_relief(
            None,
            Some(meters(-2.0)),
            Some(meters(5.0)),
            true,
            true,
            None,
            None,
        )
        .is_err(),
        "a negative diameter"
    );
    assert!(
        fire_case_relief(
            Some(square_meters(100.0)),
            None,
            None,
            true,
            true,
            None,
            Some(kelvins(0.0)),
        )
        .is_err(),
        "a zero relieving temperature"
    );
}
