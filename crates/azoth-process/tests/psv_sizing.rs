//! `process.psv_sizing` against `validation/neqsim/captures/process_psv_sizing.tsv`.

use azoth_core::units::{
    cubic_meters_per_second, joules_per_kilogram_kelvin, kelvins, kilograms_per_cubic_meter,
    kilograms_per_mole, kilograms_per_second, pascal_seconds, pascals, square_meters,
};
use azoth_process::models::psv_sizing::{PsvSizingResult, psv_sizing};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn gas(
    mass_flow: f64,
    set_pressure: f64,
    overpressure: f64,
    back_pressure: f64,
    temperature: f64,
    molar_mass: f64,
    compressibility: f64,
    specific_heat_ratio: f64,
    balanced: bool,
    rupture_disk: bool,
) -> PsvSizingResult {
    psv_sizing(
        "gas",
        pascals(set_pressure),
        overpressure,
        pascals(back_pressure),
        Some(balanced),
        Some(rupture_disk),
        Some(kilograms_per_second(mass_flow)),
        Some(kelvins(temperature)),
        Some(kilograms_per_mole(molar_mass)),
        Some(compressibility),
        Some(specific_heat_ratio),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("the captured gas state is valid")
}

fn liquid(
    volume_flow: f64,
    density: f64,
    set_pressure: f64,
    overpressure: f64,
    back_pressure: f64,
    viscosity: f64,
    balanced: bool,
) -> PsvSizingResult {
    psv_sizing(
        "liquid",
        pascals(set_pressure),
        overpressure,
        pascals(back_pressure),
        Some(balanced),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(cubic_meters_per_second(volume_flow)),
        Some(kilograms_per_cubic_meter(density)),
        Some(pascal_seconds(viscosity)),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("the captured liquid state is valid")
}

#[allow(clippy::too_many_arguments)]
fn two_phase(
    mass_flow: f64,
    set_pressure: f64,
    overpressure: f64,
    back_pressure: f64,
    temperature: f64,
    gas_fraction: f64,
    gas_density: f64,
    liquid_density: f64,
    latent_heat: f64,
    liquid_heat_capacity: f64,
) -> PsvSizingResult {
    psv_sizing(
        "two_phase",
        pascals(set_pressure),
        overpressure,
        pascals(back_pressure),
        None,
        None,
        Some(kilograms_per_second(mass_flow)),
        Some(kelvins(temperature)),
        None,
        None,
        None,
        None,
        Some(kilograms_per_cubic_meter(liquid_density)),
        None,
        Some(gas_fraction),
        Some(kilograms_per_cubic_meter(gas_density)),
        Some(latent_heat),
        Some(joules_per_kilogram_kelvin(liquid_heat_capacity)),
        None,
        None,
    )
    .expect("the captured two-phase state is valid")
}

#[test]
fn the_gas_branches_reproduce_the_capture() {
    let choked = gas(
        1.0, 1.0e6, 0.10, 1.01325e5, 300.0, 0.020, 0.9, 1.3, false, false,
    );
    close(
        choked.relieving_pressure.value,
        1_089_867.5,
        "the relieving pressure",
    );
    close(
        choked.back_pressure_fraction,
        0.092_970_016_997_479_05,
        "the ratio",
    );
    close(choked.discharge_coefficient, 0.975, "Kd");
    close(
        choked.required_area.value,
        4.720_066_551_198_152_4e-4,
        "the area",
    );
    close(
        choked.required_area_in2.unwrap(),
        0.731_611_778_659_271,
        "the area in in2",
    );
    assert_eq!(choked.recommended_orifice.as_deref(), Some("H"));
    close(
        choked.selected_area_in2.unwrap(),
        0.785,
        "the selected area",
    );
    close(choked.back_pressure_correction.unwrap(), 1.0, "Kb");
    close(choked.combination_correction.unwrap(), 1.0, "Kc");

    let balanced = gas(1.0, 1.0e6, 0.10, 8.0e5, 300.0, 0.020, 0.9, 1.3, true, false);
    close(
        balanced.back_pressure_correction.unwrap(),
        0.638_947_488_430_712_5,
        "the balanced Kb",
    );
    close(
        balanced.required_area_in2.unwrap(),
        1.145_026_456_643_795_3,
        "the balanced area",
    );
    assert_eq!(balanced.recommended_orifice.as_deref(), Some("J"));

    let high = gas(
        1.0, 1.0e6, 0.10, 6.6e5, 300.0, 0.020, 0.9, 1.3, false, false,
    );
    close(
        high.back_pressure_correction.unwrap(),
        0.788_843_597_960_302_6,
        "the conventional Kb",
    );
    close(
        high.required_area_in2.unwrap(),
        0.927_448_458_162_030_3,
        "the high-back-pressure area",
    );

    let severe = gas(
        1.0, 1.0e6, 0.10, 9.5e5, 300.0, 0.020, 0.9, 1.3, false, false,
    );
    close(
        severe.back_pressure_correction.unwrap(),
        0.256_668_815_245_89,
        "the severe Kb",
    );
    assert_eq!(severe.recommended_orifice.as_deref(), Some("L"));

    let rupture = gas(
        1.0, 1.0e6, 0.10, 1.01325e5, 300.0, 0.020, 0.9, 1.3, false, true,
    );
    close(
        rupture.combination_correction.unwrap(),
        0.9,
        "Kc with a rupture disk",
    );
    close(
        rupture.required_area_in2.unwrap(),
        0.812_901_976_288_078_9,
        "the rupture-disk area",
    );

    let exhausted = gas(
        100.0, 1.0e6, 0.10, 1.01325e5, 300.0, 0.020, 0.9, 1.3, false, false,
    );
    close(
        exhausted.required_area_in2.unwrap(),
        73.161_177_865_927_1,
        "the requirement",
    );
    assert_eq!(exhausted.recommended_orifice.as_deref(), Some("T"));
    close(
        exhausted.selected_area_in2.unwrap(),
        26.0,
        "the exhausted ladder answer",
    );
}

#[test]
fn the_capacity_helper_is_not_the_sizing_inverse() {
    let sized = gas(
        1.0, 1.0e6, 0.10, 1.01325e5, 300.0, 0.020, 0.9, 1.3, false, false,
    );
    let area = sized.selected_area_in2.unwrap() * 6.4516e-4;
    let rated = psv_sizing(
        "gas",
        pascals(1.0e6),
        0.10,
        pascals(1.01325e5),
        Some(false),
        Some(false),
        Some(kilograms_per_second(1.0)),
        Some(kelvins(300.0)),
        Some(kilograms_per_mole(0.020)),
        Some(0.9),
        Some(1.3),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(square_meters(area)),
        Some(0.975),
    )
    .expect("the captured capacity state is valid");
    let capacity = rated
        .mass_flow_capacity
        .expect("stated with the area")
        .value;
    close(capacity, 1.071_835_324_399_452_7, "the capacity helper");
    assert!(
        capacity >= 1.0,
        "the selected orifice admits the 1.0 kg/s demand: {capacity} kg/s"
    );
}

#[test]
fn the_liquid_branches_reproduce_the_capture() {
    let low = liquid(0.01, 800.0, 1.0e6, 0.10, 1.01325e5, 0.001, false);
    close(
        low.required_area.value,
        3.094_084_141_581_176_3e-4,
        "the low-viscosity area",
    );
    close(
        low.required_area_in2.unwrap(),
        0.479_584_001_113_084_6,
        "the area in in2",
    );
    assert_eq!(low.recommended_orifice.as_deref(), Some("G"));
    close(
        low.viscosity_correction.unwrap(),
        1.0,
        "Kv at high Reynolds",
    );
    close(
        low.mass_flow_rate.unwrap().value,
        8.0,
        "the liquid mass flow",
    );
    close(low.back_pressure_correction.unwrap(), 1.0, "Kw");

    let high = liquid(0.01, 800.0, 1.0e6, 0.10, 1.01325e5, 1.0, false);
    close(
        high.viscosity_correction.unwrap(),
        0.869_546_086_470_695_5,
        "the viscous Kv",
    );
    close(
        high.required_area_in2.unwrap(),
        0.551_533_735_330_366_5,
        "the viscous area",
    );

    let low_overpressure = liquid(0.01, 800.0, 1.0e6, 0.05, 1.01325e5, 0.001, false);
    close(
        low_overpressure.relieving_pressure.value,
        1_044_933.75,
        "the 5 % relieving pressure",
    );
    close(
        low_overpressure.required_area_in2.unwrap(),
        0.613_587_343_318_740_9,
        "the Kp area",
    );

    let balanced = liquid(0.01, 800.0, 1.0e6, 0.10, 7.7e5, 0.001, true);
    close(
        balanced.back_pressure_correction.unwrap(),
        0.896_746_049_405_088_3,
        "the balanced Kw",
    );
    close(
        balanced.required_area_in2.unwrap(),
        0.940_173_117_734_205_3,
        "the balanced area",
    );

    let conventional = liquid(0.01, 800.0, 1.0e6, 0.10, 7.7e5, 0.001, false);
    close(
        conventional.back_pressure_correction.unwrap(),
        1.0,
        "the conventional Kw",
    );
    close(
        conventional.required_area_in2.unwrap(),
        0.843_096_529_085_013_6,
        "the conventional area",
    );
}

#[test]
fn the_two_phase_branches_reproduce_the_capture() {
    let high = two_phase(
        1.0, 1.0e6, 0.10, 1.0e5, 350.0, 0.2, 20.0, 600.0, 2.0e5, 2500.0,
    );
    close(
        high.required_area.value,
        3.026_985_822_305_454_4e-5,
        "the omega-above-one area",
    );
    close(
        high.back_pressure_fraction,
        0.091_754_272_881_795_26,
        "the two-phase ratio",
    );
    close(high.discharge_coefficient, 0.85, "the two-phase Kd");
    assert!(high.required_area_in2.is_none(), "no orifice is selected");
    assert!(high.recommended_orifice.is_none(), "no letter is published");

    let low = two_phase(
        1.0, 1.0e6, 0.10, 1.0e5, 300.0, 0.01, 50.0, 700.0, 1.0e6, 2000.0,
    );
    close(
        low.required_area.value,
        3.376_960_533_581_787e-5,
        "the omega-at-or-below-one area",
    );
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    let base = |service: &str| {
        psv_sizing(
            service,
            pascals(1.0e6),
            0.10,
            pascals(1.01325e5),
            Some(false),
            Some(false),
            Some(kilograms_per_second(1.0)),
            Some(kelvins(300.0)),
            Some(kilograms_per_mole(0.020)),
            Some(0.9),
            Some(1.3),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    };
    assert!(base("gas").is_ok(), "the base gas state is valid");
    assert!(
        psv_sizing(
            "gas",
            pascals(1.0e6),
            0.10,
            pascals(1.01325e5),
            Some(false),
            Some(false),
            Some(kilograms_per_second(0.0)),
            Some(kelvins(300.0)),
            Some(kilograms_per_mole(0.020)),
            Some(0.9),
            Some(1.3),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .is_err(),
        "a zero mass flow"
    );
    assert!(
        psv_sizing(
            "gas",
            pascals(1.0e6),
            0.10,
            pascals(1.01325e5),
            Some(false),
            Some(false),
            Some(kilograms_per_second(1.0)),
            Some(kelvins(300.0)),
            Some(kilograms_per_mole(0.020)),
            Some(0.9),
            Some(1.0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .is_err(),
        "an isentropic exponent of one"
    );
    assert!(
        psv_sizing(
            "liquid",
            pascals(1.0e6),
            0.10,
            pascals(1.01325e5),
            Some(false),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(cubic_meters_per_second(0.0)),
            Some(kilograms_per_cubic_meter(800.0)),
            Some(pascal_seconds(0.001)),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .is_err(),
        "a zero liquid volume flow"
    );
    assert!(
        psv_sizing(
            "two_phase",
            pascals(1.0e6),
            0.10,
            pascals(1.0e5),
            None,
            None,
            Some(kilograms_per_second(1.0)),
            Some(kelvins(300.0)),
            None,
            None,
            None,
            None,
            Some(kilograms_per_cubic_meter(0.0)),
            None,
            Some(0.01),
            Some(kilograms_per_cubic_meter(50.0)),
            Some(1.0e6),
            Some(joules_per_kilogram_kelvin(2000.0)),
            None,
            None,
        )
        .is_err(),
        "a zero liquid density"
    );
    assert!(
        psv_sizing(
            "gas",
            pascals(1.0e6),
            0.10,
            pascals(1.2e6),
            Some(false),
            Some(false),
            Some(kilograms_per_second(1.0)),
            Some(kelvins(300.0)),
            Some(kilograms_per_mole(0.020)),
            Some(0.9),
            Some(1.3),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .is_err(),
        "a back pressure above the relieving pressure"
    );
    assert!(base("liquid").is_err(), "a gas-shaped liquid call");
    assert!(base("two_phase").is_err(), "a gas-shaped two-phase call");
    assert!(
        psv_sizing(
            "gas",
            pascals(1.0e6),
            0.10,
            pascals(1.01325e5),
            Some(false),
            Some(false),
            Some(kilograms_per_second(1.0)),
            Some(kelvins(300.0)),
            Some(kilograms_per_mole(0.020)),
            Some(0.9),
            Some(1.3),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(square_meters(0.001)),
            None,
        )
        .is_err(),
        "a capacity area without its coefficient"
    );
}
