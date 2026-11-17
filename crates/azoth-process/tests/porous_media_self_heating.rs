//! `process.porous_media_self_heating` against
//! `validation/neqsim/captures/process_porous_media_self_heating.tsv`.

use azoth_core::units::{joules_per_mole, kelvins, meters, watts_per_meter_kelvin};
use azoth_process::models::SelfHeatingVerdict;
use azoth_process::models::porous_media_self_heating::{
    PorousMediaSelfHeatingResult, porous_media_self_heating,
};

const FK_NOTE: &str = "Frank-Kamenetskii theory assumes a uniform boundary temperature, \
conduction-controlled heat loss (large Biot number) and negligible reactant consumption; use \
SemenovSelfHeatingAnalyzer when surface cooling controls";

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn porous(
    geometry: Option<&str>,
    dimension: f64,
    conductivity: f64,
    energy: f64,
    pre_factor: f64,
    boundary: f64,
    marginal_ratio: Option<f64>,
    pipe_insulation: Option<bool>,
) -> PorousMediaSelfHeatingResult {
    porous_media_self_heating(
        geometry,
        meters(dimension),
        watts_per_meter_kelvin(conductivity),
        joules_per_mole(energy),
        pre_factor,
        kelvins(boundary),
        marginal_ratio,
        pipe_insulation,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_slab_reproduces_the_capture() {
    let result = porous(
        Some("slab"),
        0.05,
        0.09,
        110_000.0,
        5.0e13,
        430.0,
        None,
        None,
    );
    close(result.delta, 0.004_317_180_673_554_722, "delta");
    close(result.delta_crit, 0.878, "delta_crit");
    close(result.delta_ratio, 0.004_917_062_270_563_465, "the ratio");
    close(
        result
            .critical_temperature
            .expect("the bracket holds")
            .value,
        528.345_132_619_142_5,
        "the critical temperature",
    );
    close(
        result.critical_dimension.value,
        0.713_045_346_929_867_9,
        "the critical dimension",
    );
    close(
        result.temperature_margin.expect("a margin").value,
        98.345_132_619_142_53,
        "the margin",
    );
    close(
        result.dimension_margin.value,
        0.663_045_346_929_867_8,
        "the dimension margin",
    );
    close(
        result.fk_temperature_scale.value,
        13.975_855_800_620_002,
        "the temperature scale",
    );
    assert_eq!(result.verdict, SelfHeatingVerdict::Subcritical);
    assert_eq!(result.messages, vec![FK_NOTE.to_string()]);
}

#[test]
fn the_five_shapes_carry_their_own_critical_values() {
    let cylinder = porous(
        Some("infinite_cylinder"),
        0.05,
        0.09,
        110_000.0,
        5.0e13,
        430.0,
        None,
        None,
    );
    assert_eq!(
        cylinder.geometry,
        azoth_process::models::SelfHeatingGeometry::InfiniteCylinder
    );
    close(cylinder.delta_crit, 2.0, "cylinder delta_crit");
    close(
        cylinder.critical_temperature.expect("a bracket").value,
        547.955_299_168_825_1,
        "cylinder critical temperature",
    );
    close(
        cylinder.critical_dimension.value,
        1.076_179_933_870_050_2,
        "cylinder critical dimension",
    );

    let sphere = porous(
        Some("sphere"),
        0.05,
        0.09,
        110_000.0,
        5.0e13,
        430.0,
        None,
        None,
    );
    close(sphere.delta_crit, 3.322, "sphere delta_crit");
    close(
        sphere.critical_temperature.expect("a bracket").value,
        560.819_674_283_266_1,
        "sphere critical temperature",
    );

    let cube = porous(
        Some("cube"),
        0.05,
        0.09,
        110_000.0,
        5.0e13,
        430.0,
        None,
        None,
    );
    close(cube.delta_crit, 2.52, "cube delta_crit");
    close(
        cube.critical_dimension.value,
        1.208_008_979_718_831_8,
        "cube critical dimension",
    );

    let equicylinder = porous(
        Some("equicylinder"),
        0.05,
        0.09,
        110_000.0,
        5.0e13,
        430.0,
        None,
        None,
    );
    close(equicylinder.delta_crit, 2.76, "equicylinder delta_crit");
    close(
        equicylinder.critical_temperature.expect("a bracket").value,
        556.048_297_137_022,
        "equicylinder critical temperature",
    );
}

#[test]
fn the_critical_dimension_state_screens_marginal() {
    let result = porous(
        Some("slab"),
        0.713_045_346_929_867_9,
        0.09,
        110_000.0,
        5.0e13,
        430.0,
        None,
        None,
    );
    close(result.delta, 0.877_999_999_999_995_8, "delta");
    close(result.delta_ratio, 0.999_999_999_999_995_2, "the ratio");
    close(
        result.critical_temperature.expect("a bracket").value,
        429.999_992_996_454_24,
        "the critical temperature",
    );
    close(
        result.temperature_margin.expect("a margin").value,
        -7.003_545_761_108_398_4e-6,
        "the margin",
    );
    close(result.dimension_margin.value, 0.0, "the dimension margin");
    assert_eq!(result.verdict, SelfHeatingVerdict::Marginal);
}

#[test]
fn the_thin_marginal_and_thick_layers_move_the_verdict() {
    let thin = porous(
        Some("slab"),
        0.178_261_336_732_466_97,
        0.09,
        110_000.0,
        5.0e13,
        430.0,
        None,
        None,
    );
    assert_eq!(thin.verdict, SelfHeatingVerdict::Subcritical);
    close(thin.delta_ratio, 0.062_499_999_999_999_674, "thin ratio");
    close(
        thin.critical_temperature.expect("a bracket").value,
        476.046_834_141_016,
        "thin critical temperature",
    );

    let near = porous(
        Some("slab"),
        0.677_393_079_583_374_5,
        0.09,
        110_000.0,
        5.0e13,
        430.0,
        None,
        None,
    );
    assert_eq!(near.verdict, SelfHeatingVerdict::Marginal);
    close(near.delta_ratio, 0.902_499_999_999_993_1, "near ratio");
    close(
        near.temperature_margin.expect("a margin").value,
        1.539_066_880_941_391,
        "near margin",
    );

    let thick = porous(
        Some("slab"),
        1.426_090_693_859_735_7,
        0.09,
        110_000.0,
        5.0e13,
        430.0,
        None,
        None,
    );
    assert_eq!(thick.verdict, SelfHeatingVerdict::SelfIgnition);
    close(thick.delta_ratio, 3.999_999_999_999_967, "thick ratio");
    close(
        thick.dimension_margin.value,
        -0.713_045_346_929_867_9,
        "thick margin",
    );
}

#[test]
fn the_pipe_insulation_flag_forces_the_slab_and_records_its_note() {
    let result = porous(
        Some("cube"),
        0.05,
        0.09,
        110_000.0,
        5.0e13,
        453.15,
        None,
        Some(true),
    );
    assert_eq!(
        result.geometry,
        azoth_process::models::SelfHeatingGeometry::Slab
    );
    close(result.delta, 0.018_718_886_767_640_146, "delta");
    close(
        result.critical_temperature.expect("a bracket").value,
        528.345_132_619_142_5,
        "the critical temperature",
    );
    close(
        result.critical_dimension.value,
        0.342_434_295_511_159_06,
        "the critical dimension",
    );
    close(
        result.fk_temperature_scale.value,
        15.521_206_199_294_152,
        "the temperature scale",
    );
    assert_eq!(result.messages.len(), 2);
    assert!(result.messages[0].contains("conservative"));
    assert_eq!(result.messages[1], FK_NOTE);
}

#[test]
fn a_failed_bracket_answers_absent() {
    let result = porous(Some("slab"), 0.05, 0.09, 1000.0, 5.0e13, 430.0, None, None);
    close(result.delta, 683_003_410.343_874_6, "delta");
    close(result.delta_ratio, 777_908_212.236_759_2, "the ratio");
    assert_eq!(result.critical_temperature, None);
    assert_eq!(result.temperature_margin, None);
    close(
        result.critical_dimension.value,
        1.792_692_601_731_319_5e-6,
        "the critical dimension",
    );
    close(
        result.fk_temperature_scale.value,
        1_537.344_138_068_2,
        "the temperature scale",
    );
    assert_eq!(result.verdict, SelfHeatingVerdict::SelfIgnition);
    assert_eq!(result.messages.len(), 2);
    assert!(result.messages[0].contains("could not be bracketed"));
    assert_eq!(result.messages[1], FK_NOTE);
}

#[test]
fn an_unknown_geometry_is_refused() {
    assert!(
        porous_media_self_heating(
            Some("trapezoid"),
            meters(0.05),
            watts_per_meter_kelvin(0.09),
            joules_per_mole(110_000.0),
            5.0e13,
            kelvins(430.0),
            None,
            None,
        )
        .is_err()
    );
}
