//! `process.api537_flare_flame_model` against
//! `validation/neqsim/captures/process_api537_flare_flame.tsv`.

use azoth_core::units::{kilograms_per_second, meters, meters_per_second, watts_per_square_meter};
use azoth_process::models::api537_flare_flame_model::{
    Api537FlareFlameModelResult, FlareTargetFlux, api537_flare_flame_model,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn targets(fluxes: &[f64]) -> Vec<FlareTargetFlux> {
    fluxes
        .iter()
        .map(|flux| FlareTargetFlux {
            flux: watts_per_square_meter(*flux),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn flare(
    mass: f64,
    heat: f64,
    radiative: f64,
    exit: f64,
    ground: f64,
    sound: f64,
    target: &[f64],
    stack: Option<f64>,
    wind: Option<f64>,
    tau: Option<f64>,
    eta: Option<f64>,
) -> Api537FlareFlameModelResult {
    api537_flare_flame_model(
        kilograms_per_second(mass),
        heat,
        radiative,
        meters_per_second(exit),
        meters(ground),
        meters(sound),
        &targets(target),
        stack.map(meters),
        wind.map(meters_per_second),
        tau,
        eta,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_flame_length_and_sterile_zones_reproduce_the_capture() {
    let result = flare(
        50.0,
        50.0e6,
        0.2,
        200.0,
        60.0,
        100.0,
        &[1580.0, 4730.0, 6300.0, 9460.0],
        Some(40.0),
        Some(0.0),
        Some(1.0),
        Some(3.0e-6),
    );
    close(result.heat_release.value, 2.5e9, "the heat release");
    close(result.radiative_power.value, 5.0e8, "the radiative power");
    close(
        result.flame_length.value,
        101.258_834_478_888_55,
        "the flame length",
    );
    assert_eq!(result.flame_tilt, 0.0);
    close(
        result.flame_center_height.value,
        90.629_417_239_444_28,
        "the centre height",
    );
    close(
        result.flame_tip_height.value,
        141.258_834_478_888_56,
        "the tip height",
    );
    close(
        result.heat_flux_at_ground_distance.value,
        3368.018_925_366_640_4,
        "the ground flux",
    );
    assert_eq!(result.sterile_radii.len(), 4);
    close(
        result.sterile_radii[0].radius.value,
        130.265_317_298_768_14,
        "the 1.58 kW/m2 sterile radius",
    );
    close(
        result.sterile_radii[1].radius.value,
        14.082_031_417_585_814,
        "the 4.73 kW/m2 sterile radius",
    );
    assert_eq!(result.sterile_radii[2].radius.value, 0.0);
    assert_eq!(result.sterile_radii[3].radius.value, 0.0);
    close(
        result.sound_power_level,
        158.750_612_633_917,
        "the sound power level",
    );
    close(
        result.sound_pressure_level,
        107.750_612_633_917,
        "the sound pressure level",
    );
    close(
        result.equivalent_flame_temperature.value,
        1228.967_293_421_844_5,
        "the equivalent flame temperature",
    );
}

#[test]
fn no_wind_leaves_the_tip_vertical() {
    let result = flare(
        50.0,
        50.0e6,
        0.2,
        200.0,
        60.0,
        100.0,
        &[1580.0, 4730.0],
        Some(40.0),
        Some(0.0),
        None,
        None,
    );
    assert_eq!(result.flame_tilt, 0.0);
    assert_eq!(result.flame_tip_horizontal.value, 0.0);
    close(
        result.heat_flux_at_ground_distance.value,
        3368.018_925_366_640_4,
        "the ground flux",
    );
}

#[test]
fn a_wind_tilts_the_flame_and_moves_the_sterile_zones() {
    let result = flare(
        50.0,
        50.0e6,
        0.2,
        200.0,
        60.0,
        100.0,
        &[1580.0, 4730.0],
        Some(40.0),
        Some(100.0),
        None,
        None,
    );
    close(result.flame_tilt, 0.463_647_609_000_806_1, "the tilt");
    close(
        result.flame_center_horizontal.value,
        22.642_163_721_719_427,
        "the centre offset",
    );
    close(
        result.flame_tip_horizontal.value,
        45.284_327_443_438_855,
        "the tip offset",
    );
    close(
        result.heat_flux_at_ground_distance.value,
        4589.759_326_845_264,
        "the ground flux",
    );
    close(
        result.sterile_radii[1].radius.value,
        56.384_990_670_975_71,
        "the 4.73 kW/m2 sterile radius",
    );
}

#[test]
fn the_documentation_example() {
    let result = flare(
        50.0,
        50.0e6,
        0.2,
        200.0,
        60.0,
        100.0,
        &[1580.0, 4730.0],
        Some(40.0),
        Some(10.0),
        None,
        None,
    );
    close(result.flame_tilt, 0.049_958_395_721_942_765, "the tilt");
    close(
        result.heat_flux_at_ground_distance.value,
        3458.314_186_391_075_5,
        "the ground flux",
    );
    close(
        result.sterile_radii[1].radius.value,
        17.011_041_506_893_72,
        "the 4.73 kW/m2 sterile radius",
    );
    close(
        result.sound_pressure_level,
        107.750_612_633_917,
        "the sound pressure level",
    );
}

#[test]
fn a_zero_mass_flow_publishes_zeroes() {
    let result = flare(
        0.0,
        50.0e6,
        0.2,
        200.0,
        60.0,
        100.0,
        &[1580.0, 4730.0],
        Some(40.0),
        None,
        None,
        None,
    );
    assert_eq!(result.heat_release.value, 0.0);
    assert_eq!(result.flame_length.value, 0.0);
    assert_eq!(result.flame_center_height.value, 40.0);
    assert_eq!(result.heat_flux_at_ground_distance.value, 0.0);
    assert_eq!(result.sterile_radii[0].radius.value, 0.0);
    assert_eq!(result.sound_power_level, 0.0);
    assert_eq!(result.sound_pressure_level, -51.0);
    assert_eq!(result.equivalent_flame_temperature.value, 0.0);
}

#[test]
fn a_reduced_transmissivity_halves_the_flux() {
    let result = flare(
        50.0,
        50.0e6,
        0.2,
        200.0,
        60.0,
        100.0,
        &[1580.0, 4730.0],
        Some(40.0),
        None,
        Some(0.5),
        None,
    );
    close(
        result.heat_flux_at_ground_distance.value,
        1684.009_462_683_320_2,
        "the ground flux",
    );
    close(
        result.sterile_radii[0].radius.value,
        66.164_044_698_716_32,
        "the 1.58 kW/m2 sterile radius",
    );
    assert_eq!(result.sterile_radii[1].radius.value, 0.0);
}

#[test]
fn the_range_guards_are_the_class_constructor_and_the_port_refusals() {
    let call = |mass: f64,
                heat: f64,
                radiative: f64,
                exit: f64,
                ground: f64,
                stack: f64,
                wind: f64,
                tau: f64,
                eta: f64| {
        api537_flare_flame_model(
            kilograms_per_second(mass),
            heat,
            radiative,
            meters_per_second(exit),
            meters(ground),
            meters(100.0),
            &targets(&[1580.0]),
            Some(meters(stack)),
            Some(meters_per_second(wind)),
            Some(tau),
            Some(eta),
        )
    };
    assert!(
        call(50.0, 50.0e6, 0.2, 200.0, 60.0, 40.0, 0.0, 1.0, 3.0e-6).is_ok(),
        "the base state is valid"
    );
    assert!(call(-1.0, 50.0e6, 0.2, 200.0, 60.0, 40.0, 0.0, 1.0, 3.0e-6).is_err());
    assert!(call(50.0, 0.0, 0.2, 200.0, 60.0, 40.0, 0.0, 1.0, 3.0e-6).is_err());
    assert!(call(50.0, 50.0e6, 1.5, 200.0, 60.0, 40.0, 0.0, 1.0, 3.0e-6).is_err());
    assert!(call(50.0, 50.0e6, 0.2, 0.0, 60.0, 40.0, 0.0, 1.0, 3.0e-6).is_err());
    assert!(call(50.0, 50.0e6, 0.2, 200.0, -1.0, 40.0, 0.0, 1.0, 3.0e-6).is_err());
    assert!(call(50.0, 50.0e6, 0.2, 200.0, 60.0, -1.0, 0.0, 1.0, 3.0e-6).is_err());
    assert!(call(50.0, 50.0e6, 0.2, 200.0, 60.0, 40.0, -1.0, 1.0, 3.0e-6).is_err());
    assert!(call(50.0, 50.0e6, 0.2, 200.0, 60.0, 40.0, 0.0, 2.0, 3.0e-6).is_err());
    assert!(call(50.0, 50.0e6, 0.2, 200.0, 60.0, 40.0, 0.0, 1.0, -1.0).is_err());
    let zero_target = api537_flare_flame_model(
        kilograms_per_second(50.0),
        50.0e6,
        0.2,
        meters_per_second(200.0),
        meters(60.0),
        meters(100.0),
        &targets(&[0.0]),
        None,
        None,
        None,
        None,
    );
    assert!(zero_target.is_err());
}
