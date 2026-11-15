//! `process.heavy_gas_dispersion` against
//! `validation/neqsim/captures/process_heavy_gas_dispersion.tsv`.

use azoth_core::units::{
    VolumeRate, cubic_meter_per_second, kilograms_per_cubic_meter, meters_per_second,
};
use azoth_process::models::heavy_gas_dispersion::{
    HeavyGasDispersionResult, heavy_gas_dispersion,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn dense(
    rate: f64,
    source: f64,
    air: f64,
    wind: f64,
    ratio: Option<f64>,
) -> azoth_core::Result<HeavyGasDispersionResult> {
    heavy_gas_dispersion(
        VolumeRate::new::<cubic_meter_per_second>(rate),
        kilograms_per_cubic_meter(source),
        kilograms_per_cubic_meter(air),
        meters_per_second(wind),
        ratio,
    )
}

#[test]
fn the_methane_rows_are_the_analyzers_dense_arm() {
    // `process_gas_dispersion.tsv`'s pr_methane_forced_heavy state: 1 kg/s at the
    // analyzer's source density over 5 m/s.
    let lfl = dense(
        1.470_461_327_559_685_8,
        0.680_058_687_200_945_9,
        1.225_012_265_990_694_6,
        5.0,
        Some(0.044),
    )
    .expect("the capture state is valid");
    close(lfl.reduced_gravity, -4.364_033_533_659_363, "the reduced gravity");
    close(lfl.britter_parameter, 0.389_471_738_142_793_9, "the Britter parameter");
    close(
        lfl.distance_to_concentration_ratio
            .expect("a ratio is stated")
            .value,
        9.765_355_077_286_93,
        "the LFL distance",
    );

    let half = dense(
        1.470_461_327_559_685_8,
        0.680_058_687_200_945_9,
        1.225_012_265_990_694_6,
        5.0,
        Some(0.022),
    )
    .expect("the capture state is valid");
    close(
        half.distance_to_concentration_ratio
            .expect("a ratio is stated")
            .value,
        17.602_061_561_927_734,
        "the half-LFL distance",
    );
}

#[test]
fn the_alpha_floor_carries_the_small_parameter() {
    let result = dense(1.0e-6, 1.0, 1.225, 10.0, Some(0.5)).expect("the capture state is valid");
    close(result.reduced_gravity, -1.801_836_734_693_878_2, "g0");
    // Published unfloored, below the fit's 0.01 floor.
    close(result.britter_parameter, 0.007_985_206_924_768_607, "alpha");
    close(
        result
            .distance_to_concentration_ratio
            .expect("a ratio is stated")
            .value,
        0.001_802_500_925_221_660_4,
        "the floored distance",
    );
}

#[test]
fn equal_density_publishes_zero_and_the_floor_carries_the_distance() {
    let result = dense(1.0, 1.225, 1.225, 5.0, Some(0.1)).expect("equal density is valid");
    assert_eq!(result.reduced_gravity, 0.0);
    assert_eq!(result.britter_parameter, 0.0);
    close(
        result
            .distance_to_concentration_ratio
            .expect("a ratio is stated")
            .value,
        10.011_865_297_009_068,
        "the distance at the floor",
    );
}

#[test]
fn a_buoyant_source_is_accepted() {
    let result = dense(1.0, 0.5, 1.225, 5.0, Some(0.1)).expect("a light source is valid");
    close(result.reduced_gravity, -5.805_918_367_346_94, "the negative g0");
    close(
        result.britter_parameter,
        0.404_183_300_320_682_84,
        "alpha over |g0|",
    );
    close(
        result
            .distance_to_concentration_ratio
            .expect("a ratio is stated")
            .value,
        3.970_730_450_687_776,
        "the distance",
    );
}

#[test]
fn the_distance_is_absent_without_a_ratio() {
    let result = dense(1.0, 1.0, 1.225, 5.0, None)
        .expect("the parameters without a ratio are a state");
    close(result.reduced_gravity, -1.801_836_734_693_878_2, "g0");
    close(result.britter_parameter, 0.253_114_001_909_182_2, "alpha");
    assert!(result.distance_to_concentration_ratio.is_none());
}

#[test]
fn the_guards_are_refusals() {
    for (rate, source, air, wind) in [
        (0.0, 1.0, 1.225, 5.0),
        (1.0, 0.0, 1.225, 5.0),
        (1.0, 1.0, 0.0, 5.0),
        (1.0, 1.0, 1.225, 0.0),
        (f64::NAN, 1.0, 1.225, 5.0),
        (1.0, f64::INFINITY, 1.225, 5.0),
    ] {
        assert!(
            dense(rate, source, air, wind, Some(0.2)).is_err(),
            "({rate}, {source}, {air}, {wind}) is not a positive state"
        );
    }
    assert!(dense(1.0, 1.0, 1.225, 5.0, Some(0.0)).is_err());
    assert!(dense(1.0, 1.0, 1.225, 5.0, Some(1.0)).is_err());
    assert!(dense(1.0, 1.0, 1.225, 5.0, Some(f64::NAN)).is_err());
}
