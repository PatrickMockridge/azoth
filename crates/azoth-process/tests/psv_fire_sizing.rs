//! `process.psv_fire_sizing` against
//! `validation/neqsim/captures/process_psv_fire_sizing.tsv`.

use azoth_core::units::{
    cubic_meters, joules_per_kilogram_kelvin, kelvins, kilograms, kilograms_per_mole, pascals,
    seconds, square_meters, watts,
};
use azoth_process::models::psv_fire_sizing::{PsvFireSizingResult, psv_fire_sizing};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn fire(
    initial_mass: f64,
    initial_pressure: f64,
    set_pressure: f64,
    initial_temperature: f64,
    fire_heat_input: f64,
    vessel_volume: f64,
    molar_mass: f64,
    specific_heat_ratio: f64,
    compressibility: f64,
    heat_capacity: f64,
    blowdown_time: f64,
    blowdown_percent: Option<f64>,
    orifice_area: Option<f64>,
    cv_discharge_coefficient: Option<f64>,
) -> PsvFireSizingResult {
    psv_fire_sizing(
        kilograms(initial_mass),
        pascals(initial_pressure),
        pascals(set_pressure),
        kelvins(initial_temperature),
        watts(fire_heat_input),
        cubic_meters(vessel_volume),
        kilograms_per_mole(molar_mass),
        specific_heat_ratio,
        compressibility,
        joules_per_kilogram_kelvin(heat_capacity),
        seconds(blowdown_time),
        blowdown_percent,
        orifice_area.map(square_meters),
        cv_discharge_coefficient,
    )
    .expect("the captured fire state is valid")
}

fn peak_state() -> PsvFireSizingResult {
    fire(
        1000.0, 2.0e6, 1.0e6, 300.0, 5.0e6, 10.0, 0.020, 1.3, 0.9, 2000.0, 900.0, None, None, None,
    )
}

#[test]
fn the_peak_and_average_flows_reproduce_the_capture() {
    let peak = peak_state();
    close(peak.overpressure_fraction, 0.21, "the overpressure");
    close(
        peak.required_area.value,
        0.026_500_694_094_615_227,
        "the peak area",
    );
    close(
        peak.required_area_in2,
        41.076_157_998_969_6,
        "the peak area in in2",
    );
    assert_eq!(peak.recommended_orifice, "T");
    close(peak.selected_area_in2, 26.0, "the exhausted ladder");
    close(
        peak.back_pressure_fraction,
        0.085_238_618_709_550_82,
        "the ratio",
    );
    close(peak.discharge_coefficient, 0.975, "Kd");
    close(peak.back_pressure_correction, 1.0, "Kb");
    close(peak.combination_correction, 1.0, "Kc");
    assert!(peak.blowdown_pressure.is_none(), "no percentage stated");
    assert!(peak.flow_coefficient.is_none(), "no area stated");

    let average = fire(
        5000.0, 2.0e6, 1.0e6, 300.0, 1.0e5, 10.0, 0.020, 1.3, 0.9, 2000.0, 900.0, None, None, None,
    );
    close(
        average.required_area.value,
        0.002_406_594_782_058_630_3,
        "the average area",
    );
    close(
        average.required_area_in2,
        3.730_229_372_649_622_3,
        "the average area in in2",
    );
    assert_eq!(average.recommended_orifice, "N");
    close(average.selected_area_in2, 4.34, "the N area");
}

#[test]
fn the_two_helpers_reproduce_the_capture() {
    let helpers = fire(
        1000.0,
        2.0e6,
        1.0e6,
        300.0,
        5.0e6,
        10.0,
        0.020,
        1.3,
        0.9,
        2000.0,
        900.0,
        Some(10.0),
        Some(0.001),
        Some(0.975),
    );
    close(
        helpers.blowdown_pressure.expect("stated").value,
        900_000.0,
        "the blowdown pressure",
    );
    close(
        helpers.flow_coefficient.expect("stated"),
        37.176_824_353_648_705,
        "the Cv",
    );
}

#[test]
fn the_dead_inputs_change_nothing() {
    let first = peak_state();
    let second = fire(
        1000.0, 5.0e6, 1.0e6, 300.0, 5.0e6, 30.0, 0.020, 1.3, 0.9, 2000.0, 900.0, None, None, None,
    );
    assert_eq!(
        first.required_area.value, second.required_area.value,
        "initial_pressure and vessel_volume are read by no formula"
    );
}

#[test]
fn the_class_values_the_port_refuses_are_refused() {
    assert!(
        psv_fire_sizing(
            kilograms(1000.0),
            pascals(2.0e6),
            pascals(1.0e6),
            kelvins(300.0),
            watts(5.0e6),
            cubic_meters(10.0),
            kilograms_per_mole(0.020),
            1.3,
            0.9,
            joules_per_kilogram_kelvin(2000.0),
            seconds(0.0),
            None,
            None,
            None,
        )
        .is_err(),
        "a zero blowdown time"
    );
    assert!(
        psv_fire_sizing(
            kilograms(1000.0),
            pascals(2.0e6),
            pascals(1.0e6),
            kelvins(300.0),
            watts(5.0e6),
            cubic_meters(10.0),
            kilograms_per_mole(0.020),
            1.3,
            0.9,
            joules_per_kilogram_kelvin(2000.0),
            seconds(900.0),
            Some(101.0),
            None,
            None,
        )
        .is_err(),
        "a blowdown percentage above 100"
    );
    assert!(
        psv_fire_sizing(
            kilograms(1000.0),
            pascals(2.0e6),
            pascals(1.0e6),
            kelvins(300.0),
            watts(5.0e6),
            cubic_meters(10.0),
            kilograms_per_mole(0.020),
            1.0,
            0.9,
            joules_per_kilogram_kelvin(2000.0),
            seconds(900.0),
            None,
            None,
            None,
        )
        .is_err(),
        "an isentropic exponent of one"
    );
    assert!(
        psv_fire_sizing(
            kilograms(1000.0),
            pascals(2.0e6),
            pascals(1.0e6),
            kelvins(300.0),
            watts(-5.0e6),
            cubic_meters(10.0),
            kilograms_per_mole(0.020),
            1.3,
            0.9,
            joules_per_kilogram_kelvin(2000.0),
            seconds(900.0),
            None,
            None,
            None,
        )
        .is_err(),
        "a negative fire heat input"
    );
    assert!(
        psv_fire_sizing(
            kilograms(1000.0),
            pascals(2.0e6),
            pascals(1.0e6),
            kelvins(300.0),
            watts(5.0e6),
            cubic_meters(10.0),
            kilograms_per_mole(0.020),
            1.3,
            0.9,
            joules_per_kilogram_kelvin(2000.0),
            seconds(900.0),
            None,
            Some(square_meters(0.001)),
            None,
        )
        .is_err(),
        "an area without its coefficient"
    );
}
