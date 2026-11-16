//! `process.pipe_fire_rupture_material` against
//! `validation/neqsim/captures/process_pipe_fire_rupture_material.tsv`.

use azoth_core::units::{
    joules_per_kilogram_kelvin, kelvins, kilograms_per_cubic_meter, megapascals, per_second,
    watts_per_meter_kelvin,
};
use azoth_process::models::pipe_fire_rupture_material::{
    PipeFireRuptureMaterialPoint, PipeFireRuptureMaterialResult, pipe_fire_rupture_material,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn evaluate(
    name: &str,
    temperature_k: f64,
    stress_mpa: f64,
) -> azoth_core::Result<PipeFireRuptureMaterialResult> {
    pipe_fire_rupture_material(
        name,
        kelvins(temperature_k),
        megapascals(stress_mpa),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

fn custom_point(
    temperature_k: f64,
    cp: f64,
    k: f64,
    uts_mpa: f64,
    effect: f64,
    limit: f64,
) -> PipeFireRuptureMaterialPoint {
    PipeFireRuptureMaterialPoint {
        temperature: kelvins(temperature_k),
        heat_capacity: joules_per_kilogram_kelvin(cp),
        thermal_conductivity: watts_per_meter_kelvin(k),
        ultimate_tensile_strength: megapascals(uts_mpa),
        strain_effect_factor: effect,
        rupture_strain_limit: limit,
    }
}

#[test]
fn the_six_tables_at_600_degrees() {
    let duplex = evaluate("22Cr duplex", 873.15, 100.0).expect("the capture state is valid");
    assert_eq!(duplex.resolved_material_name, "22Cr duplex");
    close(duplex.density.value, 7850.0, "22Cr density");
    close(duplex.heat_capacity.value, 670.0, "22Cr cp");
    close(duplex.thermal_conductivity.value, 28.0, "22Cr k");
    close(
        duplex.ultimate_tensile_strength.value / 1.0e6,
        371.0,
        "22Cr UTS",
    );
    close(duplex.strain_effect_factor, 1.0, "22Cr effect");
    close(duplex.rupture_strain_limit, 0.22, "22Cr limit");
    close(
        duplex.temperature_correction_factor,
        1.260_162_565_147_361_6,
        "22Cr correction",
    );
    close(
        duplex.strain_rate.value,
        1.210_785_393_254_57e-11,
        "22Cr strain rate",
    );

    let stainless = evaluate("SS 316", 873.15, 100.0).expect("the capture state is valid");
    close(
        stainless.heat_capacity.value,
        549.959_235_668_789_8,
        "SS 316 cp",
    );
    close(stainless.thermal_conductivity.value, 22.7, "SS 316 k");
    close(
        stainless.ultimate_tensile_strength.value / 1.0e6,
        363.0,
        "SS 316 UTS",
    );
    close(
        stainless.temperature_correction_factor,
        1.079_717_107_935_261_4,
        "SS 316 correction",
    );
    close(
        stainless.strain_rate.value,
        1.394_931_509_282_041e-10,
        "SS 316 strain rate",
    );

    let six_mo = evaluate("6Mo", 873.15, 100.0).expect("the capture state is valid");
    close(six_mo.heat_capacity.value, 549.959_235_668_789_8, "6Mo cp");
    close(
        six_mo.ultimate_tensile_strength.value / 1.0e6,
        480.0,
        "6Mo UTS",
    );
    close(
        six_mo.strain_rate.value,
        1.682_558_825_117_445_7e-11,
        "6Mo strain rate",
    );
}

#[test]
fn the_temperature_clamps_and_the_800_degree_state() {
    let hot = evaluate("22Cr duplex", 1073.15, 100.0).expect("the capture state is valid");
    close(hot.heat_capacity.value, 750.0, "800 C cp");
    close(
        hot.ultimate_tensile_strength.value / 1.0e6,
        157.0,
        "800 C UTS",
    );
    close(hot.strain_effect_factor, 0.7, "800 C effect");
    close(hot.rupture_strain_limit, 0.23, "800 C limit");
    close(
        hot.strain_rate.value,
        8.718_460_200_416_476e-6,
        "800 C strain rate",
    );

    let cold = evaluate("22Cr duplex", 173.15, 100.0).expect("a low clamp is a state");
    close(cold.heat_capacity.value, 480.0, "low clamp cp");
    close(cold.thermal_conductivity.value, 15.0, "low clamp k");
    close(
        cold.ultimate_tensile_strength.value / 1.0e6,
        650.0,
        "low clamp UTS",
    );
    close(
        cold.strain_rate.value,
        1.756_844_949_575_397e-19,
        "low clamp strain rate",
    );

    let high = evaluate("22Cr duplex", 1673.15, 100.0).expect("a high clamp is a state");
    close(high.heat_capacity.value, 900.0, "high clamp cp");
    close(
        high.ultimate_tensile_strength.value / 1.0e6,
        0.0,
        "high clamp UTS",
    );
    close(high.strain_effect_factor, 2.3, "high clamp effect");
    close(
        high.temperature_correction_factor,
        1.0,
        "high clamp correction",
    );
    close(
        high.strain_rate.value,
        94.573_835_099_756_04,
        "high clamp strain rate",
    );
}

#[test]
fn a_custom_curve_is_the_callers_material() {
    let curve = [
        custom_point(273.15, 500.0, 16.0, 600.0, 1.0, 0.2),
        custom_point(373.15, 520.0, 17.0, 550.0, 1.1, 0.2),
        custom_point(473.15, 540.0, 18.0, 500.0, 1.2, 0.2),
    ];
    let result = pipe_fire_rupture_material(
        "Probe custom",
        kelvins(323.15),
        megapascals(100.0),
        Some(&curve),
        Some(kilograms_per_cubic_meter(8000.0)),
        Some(1.0e10),
        Some(0.02),
        Some(2.0),
        Some(kelvins(40000.0)),
        Some(per_second(8.333_333_333_333_333e-4)),
        Some(kelvins(1173.15)),
    )
    .expect("the capture state is valid");
    assert_eq!(result.resolved_material_name, "Probe custom");
    close(result.density.value, 8000.0, "custom density");
    close(result.heat_capacity.value, 510.0, "custom cp");
    close(result.thermal_conductivity.value, 16.5, "custom k");
    close(
        result.ultimate_tensile_strength.value / 1.0e6,
        575.0,
        "custom UTS",
    );
    close(result.strain_effect_factor, 1.05, "custom effect");
    close(
        result.temperature_correction_factor,
        0.384_654_957_400_517_25,
        "custom correction",
    );
    close(
        result.strain_rate.value,
        4.724_508_577_974_46e-12,
        "custom strain rate",
    );
}

#[test]
fn the_class_selector_aliases() {
    assert_eq!(
        evaluate("SUPERDUPLEX", 873.15, 100.0)
            .expect("the alias is a state")
            .resolved_material_name,
        "Superduplex"
    );
    assert_eq!(
        evaluate("25Cr", 873.15, 100.0)
            .expect("the alias is a state")
            .resolved_material_name,
        "Superduplex"
    );
    assert_eq!(
        evaluate("API 5L-X52", 873.15, 100.0)
            .expect("the alias is a state")
            .resolved_material_name,
        "CS 360 API 5L-X52"
    );
    assert_eq!(
        evaluate("6 Mo", 873.15, 100.0)
            .expect("the alias is a state")
            .resolved_material_name,
        "6Mo"
    );
    assert_eq!(
        evaluate("A106", 873.15, 100.0)
            .expect("the alias is a state")
            .resolved_material_name,
        "CS 235 ASME A106-B"
    );
    assert_eq!(
        evaluate("316", 873.15, 100.0)
            .expect("the alias is a state")
            .resolved_material_name,
        "SS 316"
    );
}

#[test]
fn the_class_refusals_are_the_ports() {
    assert!(
        evaluate("", 873.15, 100.0).is_err(),
        "an empty name is refused"
    );
    assert!(
        evaluate("unobtainium", 873.15, 100.0).is_err(),
        "an unknown name is refused"
    );

    let curve = [
        custom_point(273.15, 500.0, 16.0, 600.0, 1.0, 0.2),
        custom_point(373.15, 520.0, 17.0, 550.0, 1.1, 0.2),
    ];
    let partial = pipe_fire_rupture_material(
        "Partial",
        kelvins(323.15),
        megapascals(100.0),
        Some(&curve),
        Some(kilograms_per_cubic_meter(8000.0)),
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert!(partial.is_err(), "a curve without its constants is refused");

    let constants_without_curve = pipe_fire_rupture_material(
        "22Cr duplex",
        kelvins(323.15),
        megapascals(100.0),
        None,
        Some(kilograms_per_cubic_meter(8000.0)),
        Some(1.0e10),
        Some(0.02),
        Some(2.0),
        Some(kelvins(40000.0)),
        Some(per_second(8.333_333_333_333_333e-4)),
        Some(kelvins(1173.15)),
    );
    assert!(
        constants_without_curve.is_err(),
        "constants without a curve are refused"
    );

    let short = pipe_fire_rupture_material(
        "Short",
        kelvins(323.15),
        megapascals(100.0),
        Some(&curve[..1]),
        Some(kilograms_per_cubic_meter(8000.0)),
        Some(1.0e10),
        Some(0.02),
        Some(2.0),
        Some(kelvins(40000.0)),
        Some(per_second(8.333_333_333_333_333e-4)),
        Some(kelvins(1173.15)),
    );
    assert!(short.is_err(), "a one-point curve is refused");

    let zero_alpha = pipe_fire_rupture_material(
        "Zero alpha",
        kelvins(323.15),
        megapascals(100.0),
        Some(&curve),
        Some(kilograms_per_cubic_meter(8000.0)),
        Some(1.0e10),
        Some(0.0),
        Some(2.0),
        Some(kelvins(40000.0)),
        Some(per_second(8.333_333_333_333_333e-4)),
        Some(kelvins(1173.15)),
    );
    assert!(zero_alpha.is_err(), "a zero alpha is refused");

    let zero_temperature = pipe_fire_rupture_material(
        "22Cr duplex",
        kelvins(0.0),
        megapascals(100.0),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert!(
        zero_temperature.is_err(),
        "a zero absolute temperature is refused"
    );
}
