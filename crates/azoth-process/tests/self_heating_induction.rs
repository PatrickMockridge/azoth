//! `process.self_heating_induction` against
//! `validation/neqsim/captures/process_self_heating_induction.tsv`.

use azoth_core::units::{
    joules_per_mole, kelvin_intervals, kelvins, meters, seconds, watts_per_meter_kelvin,
};
use azoth_process::models::self_heating_induction::{
    SelfHeatingInductionResult, self_heating_induction,
};

const STEADY_NOTE: &str = "A steady temperature profile was reached, so the body is subcritical at this boundary temperature";

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn induction(boundary: f64) -> SelfHeatingInductionResult {
    induction_on("slab", boundary, None)
}

fn induction_on(geometry: &str, boundary: f64, initial: Option<f64>) -> SelfHeatingInductionResult {
    self_heating_induction(
        geometry,
        meters(0.05),
        watts_per_meter_kelvin(0.09),
        1.8e5,
        joules_per_mole(110_000.0),
        5.0e13,
        kelvins(boundary),
        initial.map(kelvins),
        Some(kelvin_intervals(100.0)),
        Some(seconds(432_000.0)),
        Some(21.0),
    )
    .expect("the captured state is valid")
}

#[test]
fn the_igniting_slab_reproduces_the_capture() {
    let result = induction(548.345_132_619_142_5);
    assert!(result.ignited);
    close(
        result.induction_time.expect("it ignited").value,
        3_631.288_064_208_441,
        "the induction time",
    );
    close(
        result.peak_temperature.value,
        648.492_201_613_980_7,
        "the peak",
    );
    close(
        result.peak_temperature_rise.value,
        100.147_068_994_838_15,
        "the peak rise",
    );
    assert!(!result.steady_state_reached);
    close(
        result.simulated_time.value,
        3_631.288_064_208_441,
        "the simulated time",
    );
    assert!(result.messages.is_empty());
    assert_eq!(result.history.len(), 365);
    close(result.history[0].time.value, 0.0, "the first sample's time");
    close(
        result.history[1].time.value,
        10.000_000_000_000_002,
        "the second sample's time",
    );
    close(
        result.history[1].centre_temperature.value,
        548.437_625_526_695_8,
        "the second sample's centre",
    );
    let last = result.history.last().expect("the final sample");
    close(last.time.value, 3_631.288_064_208_441, "the last time");
    close(
        last.max_temperature.value,
        648.492_201_613_980_7,
        "the last peak",
    );
}

#[test]
fn the_subcritical_slab_reaches_a_steady_profile() {
    let result = induction(518.345_132_619_142_5);
    assert!(!result.ignited);
    assert_eq!(result.induction_time, None);
    close(
        result.peak_temperature.value,
        526.204_349_048_002_9,
        "the peak",
    );
    close(
        result.peak_temperature_rise.value,
        7.859_216_428_860_35,
        "the peak rise",
    );
    assert!(result.steady_state_reached);
    close(
        result.simulated_time.value,
        27_165.000_000_000_004,
        "the simulated time",
    );
    assert_eq!(result.messages, vec![STEADY_NOTE.to_string()]);
    assert_eq!(result.history.len(), 340);
}

#[test]
fn a_hotter_boundary_shortens_the_induction_time() {
    let result = induction(588.345_132_619_142_5);
    close(
        result.induction_time.expect("it ignited").value,
        585.336_802_494_161_9,
        "the induction time",
    );
    close(
        result.peak_temperature.value,
        688.742_595_958_479,
        "the peak",
    );
    close(
        result.peak_temperature_rise.value,
        100.397_463_339_336_47,
        "the peak rise",
    );
    assert_eq!(result.history.len(), 277);
}

#[test]
fn the_sphere_reproduces_the_capture() {
    let result = induction_on("sphere", 580.819_674_283_266_1, None);
    close(
        result.induction_time.expect("it ignited").value,
        944.585_538_175_886,
        "the induction time",
    );
    close(
        result.peak_temperature.value,
        681.256_757_712_353_3,
        "the peak",
    );
    close(
        result.peak_temperature_rise.value,
        100.437_083_429_087_24,
        "the peak rise",
    );
    assert_eq!(result.history.len(), 285);
}

#[test]
fn a_pre_heated_start_ignites_sooner() {
    let result = induction_on("slab", 548.345_132_619_142_5, Some(588.345_132_619_142_5));
    close(
        result.initial_temperature.value,
        588.345_132_619_142_5,
        "the initial temperature",
    );
    close(
        result.induction_time.expect("it ignited").value,
        564.893_170_605_319_6,
        "the induction time",
    );
    close(
        result.peak_temperature.value,
        648.741_176_573_860_2,
        "the peak",
    );
    close(
        result.peak_temperature_rise.value,
        100.396_043_954_717_7,
        "the peak rise",
    );
    assert_eq!(result.history.len(), 233);
}

#[test]
fn the_near_critical_run_reaches_a_steady_profile() {
    let result = induction(528.845_132_619_142_5);
    assert!(!result.ignited);
    close(
        result.peak_temperature.value,
        550.722_624_210_399_8,
        "the peak",
    );
    close(
        result.peak_temperature_rise.value,
        21.877_491_591_257_24,
        "the peak rise",
    );
    assert!(result.steady_state_reached);
    close(
        result.simulated_time.value,
        83_837.500_000_000_01,
        "the simulated time",
    );
    assert_eq!(result.messages, vec![STEADY_NOTE.to_string()]);
    assert_eq!(result.history.len(), 263);
}

#[test]
fn a_non_one_dimensional_geometry_is_refused() {
    assert!(
        self_heating_induction(
            "cube",
            meters(0.05),
            watts_per_meter_kelvin(0.09),
            1.8e5,
            joules_per_mole(110_000.0),
            5.0e13,
            kelvins(500.0),
            None,
            None,
            None,
            Some(21.0),
        )
        .is_err()
    );
}

#[test]
fn a_node_count_below_five_is_refused() {
    assert!(
        self_heating_induction(
            "slab",
            meters(0.05),
            watts_per_meter_kelvin(0.09),
            1.8e5,
            joules_per_mole(110_000.0),
            5.0e13,
            kelvins(500.0),
            None,
            None,
            None,
            Some(3.0),
        )
        .is_err()
    );
}
