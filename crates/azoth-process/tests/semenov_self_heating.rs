//! `process.semenov_self_heating` against
//! `validation/neqsim/captures/process_semenov_self_heating.tsv`.

use azoth_core::units::{
    cubic_meters, joules_per_mole, kelvins, square_meters, watts_per_square_meter_kelvin,
};
use azoth_process::models::SelfHeatingVerdict;
use azoth_process::models::semenov_self_heating::{SemenovSelfHeatingResult, semenov_self_heating};

const SEMENOV_NOTE: &str = "Semenov theory assumes a spatially uniform body temperature (small \
Biot number); use PorousMediaSelfHeatingAnalyzer when internal conduction controls";
const BRACKET_NOTE: &str = "Critical ambient temperature could not be bracketed between 150.0 K \
and 2000.0 K; check the kinetic parameters";

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn semenov(
    volume: f64,
    surface_area: f64,
    heat_transfer_coefficient: f64,
    activation_energy: f64,
    pre_factor: f64,
    ambient_temperature: f64,
    marginal_ratio: Option<f64>,
) -> SemenovSelfHeatingResult {
    semenov_self_heating(
        cubic_meters(volume),
        square_meters(surface_area),
        watts_per_square_meter_kelvin(heat_transfer_coefficient),
        joules_per_mole(activation_energy),
        pre_factor,
        kelvins(ambient_temperature),
        marginal_ratio,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_class_test_state_reproduces_the_capture() {
    let result = semenov(0.01, 0.3, 10.0, 110_000.0, 5.0e13, 400.0, None);
    close(result.psi, 5.957_193_865_908_410_4e-5, "psi");
    close(result.psi_crit, 0.367_879_441_171_442_33, "psi_crit");
    close(result.psi_ratio, 1.619_333_183_430_652_2e-4, "psi_ratio");
    let critical = result.critical_temperature.expect("the bracket holds");
    close(
        critical.value,
        558.741_828_054_189_7,
        "the critical temperature",
    );
    close(
        result
            .temperature_margin
            .expect("the margin follows the critical temperature")
            .value,
        158.741_828_054_189_68,
        "the temperature margin",
    );
    close(
        result.critical_temperature_rise.value,
        12.093_763_808,
        "the critical rise",
    );
    assert_eq!(result.verdict, SelfHeatingVerdict::Subcritical);
    assert_eq!(result.messages, vec![SEMENOV_NOTE.to_string()]);
}

#[test]
fn the_critical_state_screens_as_self_ignition() {
    let result = semenov(
        0.01,
        0.3,
        10.0,
        110_000.0,
        5.0e13,
        558.741_828_054_189_7,
        None,
    );
    close(result.psi, 0.367_879_587_092_770_85, "psi");
    close(result.psi_ratio, 1.000_000_396_655_295_7, "the ratio");
    close(
        result.temperature_margin.expect("zero margin").value,
        0.0,
        "the margin",
    );
    close(
        result.critical_temperature_rise.value,
        23.597_384_475_704_725,
        "the critical rise",
    );
    assert_eq!(result.verdict, SelfHeatingVerdict::SelfIgnition);
}

#[test]
fn the_subcritical_state_has_a_positive_margin() {
    let result = semenov(
        0.01,
        0.3,
        10.0,
        110_000.0,
        5.0e13,
        518.741_828_054_189_7,
        None,
    );
    close(result.psi, 0.068_752_022_648_981_64, "psi");
    close(result.psi_ratio, 0.186_887_373_836_531_5, "the ratio");
    close(
        result.temperature_margin.expect("a margin").value,
        40.0,
        "the margin",
    );
    close(
        result.critical_temperature_rise.value,
        20.339_676_264_715_97,
        "the critical rise",
    );
    assert_eq!(result.verdict, SelfHeatingVerdict::Subcritical);
}

#[test]
fn the_ignition_state_has_a_negative_margin() {
    let result = semenov(
        0.01,
        0.3,
        10.0,
        110_000.0,
        5.0e13,
        568.741_828_054_189_7,
        None,
    );
    close(result.psi, 0.538_400_217_531_939_7, "psi");
    close(result.psi_ratio, 1.463_523_527_755_468_6, "the ratio");
    close(
        result.temperature_margin.expect("a margin").value,
        -10.0,
        "the margin",
    );
    close(
        result.critical_temperature_rise.value,
        24.449_604_540_351_917,
        "the critical rise",
    );
    assert_eq!(result.verdict, SelfHeatingVerdict::SelfIgnition);
}

#[test]
fn the_bulky_body_raises_psi_and_lowers_the_critical_temperature() {
    let result = semenov(0.01, 0.2, 10.0, 110_000.0, 5.0e13, 450.0, None);
    close(result.psi, 0.002_785_192_578_648_422_3, "psi");
    close(result.psi_ratio, 0.007_570_938_375_298_996, "the ratio");
    close(
        result
            .critical_temperature
            .expect("the bracket holds")
            .value,
        548.491_645_604_372,
        "the critical temperature",
    );
    close(
        result.temperature_margin.expect("a margin").value,
        98.491_645_604_372_02,
        "the margin",
    );
    close(
        result.critical_temperature_rise.value,
        15.306_169_819_5,
        "the critical rise",
    );
    assert_eq!(result.verdict, SelfHeatingVerdict::Subcritical);
}

#[test]
fn the_marginal_band_is_inclusive_at_its_lower_edge() {
    let result = semenov(
        0.01,
        0.3,
        10.0,
        110_000.0,
        5.0e13,
        553.741_828_054_189_7,
        Some(0.8),
    );
    close(result.psi, 0.302_454_986_663_432_8, "psi");
    close(result.psi_ratio, 0.822_157_894_174_032_2, "the ratio");
    close(
        result.temperature_margin.expect("a margin").value,
        5.0,
        "the margin",
    );
    close(
        result.critical_temperature_rise.value,
        23.176_943_395_166_134,
        "the critical rise",
    );
    assert_eq!(result.verdict, SelfHeatingVerdict::Marginal);
}

#[test]
fn a_failed_bracket_answers_absent_and_carries_both_messages() {
    let result = semenov(0.01, 0.3, 10.0, 1000.0, 5.0e13, 400.0, None);
    close(result.psi, 92_749_276.726_051_76, "psi");
    close(result.psi_ratio, 252_118_673.527_145_95, "the ratio");
    assert_eq!(result.critical_temperature, None);
    assert_eq!(result.temperature_margin, None);
    close(
        result.critical_temperature_rise.value,
        1_330.314_018_88,
        "the critical rise",
    );
    assert_eq!(result.verdict, SelfHeatingVerdict::SelfIgnition);
    assert_eq!(
        result.messages,
        vec![BRACKET_NOTE.to_string(), SEMENOV_NOTE.to_string()]
    );
}

#[test]
fn a_non_positive_input_is_refused() {
    assert!(
        semenov_self_heating(
            cubic_meters(0.0),
            square_meters(0.3),
            watts_per_square_meter_kelvin(10.0),
            joules_per_mole(110_000.0),
            5.0e13,
            kelvins(400.0),
            None,
        )
        .is_err()
    );
}

#[test]
fn a_marginal_ratio_outside_the_open_interval_is_refused() {
    assert!(
        semenov_self_heating(
            cubic_meters(0.01),
            square_meters(0.3),
            watts_per_square_meter_kelvin(10.0),
            joules_per_mole(110_000.0),
            5.0e13,
            kelvins(400.0),
            Some(1.0),
        )
        .is_err()
    );
}
