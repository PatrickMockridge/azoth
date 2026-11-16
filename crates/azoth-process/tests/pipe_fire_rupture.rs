//! `process.pipe_fire_rupture` against
//! `validation/neqsim/captures/process_pipe_fire_rupture.tsv`.

use azoth_core::units::{
    bars, joules_per_kilogram_kelvin, kelvins, kilograms_per_cubic_meter, kilograms_per_mole,
    megapascals, meters, seconds,
};
use azoth_process::models::blowdown_pressure_profile::BlowdownProfilePoint;
use azoth_process::models::pipe_fire_rupture::{
    FluidBasis, PipeFireRuptureResult, RuptureStatus, pipe_fire_rupture,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn workbook() -> Vec<BlowdownProfilePoint> {
    let minutes = [
        0.0,
        0.083_333_333,
        0.166_666_667,
        0.25,
        0.333_333_333,
        0.416_666_667,
        0.5,
        0.583_333_333,
        0.666_666_667,
        0.75,
        0.833_333_333,
        0.916_666_667,
        1.0,
        1.083_333_333,
        1.166_666_667,
        1.25,
        1.333_333_333,
        1.416_666_667,
        1.5,
        1.583_333_333,
        1.666_666_667,
        1.75,
        1.833_333_333,
        1.916_666_667,
        2.0,
    ];
    let bara = [
        61.3, 59.7053, 58.6349, 57.3483, 55.9731, 54.5583, 53.1249, 51.6842, 50.2472, 48.8145,
        47.3977, 46.004, 44.6383, 43.3046, 42.0058, 40.7442, 39.5206, 38.3367, 37.1941, 36.0922,
        35.0302, 34.0082, 33.0255, 32.0817, 31.1758,
    ];
    minutes
        .iter()
        .zip(bara.iter())
        .map(|(minute, pressure)| BlowdownProfilePoint {
            time: seconds(minute * 60.0),
            pressure: bars(*pressure),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn run(
    material: &str,
    scenario: &str,
    max_time_s: f64,
    density: f64,
    spreadsheet_gas: bool,
    pfp: Option<f64>,
    explicit: Option<(f64, f64, f64, f64, f64, f64)>,
) -> azoth_core::Result<PipeFireRuptureResult> {
    let profile = workbook();
    pipe_fire_rupture(
        meters(0.0889),
        meters(0.0037),
        material,
        scenario,
        &profile,
        Some(meters(0.0)),
        Some(0.10),
        Some(meters(1.0)),
        Some(megapascals(0.0)),
        Some(kilograms_per_cubic_meter(density)),
        Some(joules_per_kilogram_kelvin(2283.354_699_059_34)),
        Some(kilograms_per_mole(0.0182)),
        Some(kelvins(293.15)),
        explicit.map(|value| kelvins(value.0)),
        explicit.map(|value| kelvins(value.1)),
        explicit.map(|value| {
            azoth_core::units::HeatTransfer::new::<azoth_core::units::watt_per_square_meter_kelvin>(
                value.2,
            )
        }),
        explicit.map(|value| value.3),
        explicit.map(|value| value.4),
        explicit.map(|value| value.5),
        pfp,
        Some(bars(1.0)),
        Some("step_previous"),
        Some(seconds(5.0)),
        Some(seconds(max_time_s)),
        Some(spreadsheet_gas),
    )
}

#[test]
fn the_workbook_benchmark_ruptures_at_115_seconds() {
    let result = run(
        "22Cr duplex",
        "large_jet_fire",
        200.0,
        23.748_703_715_501_2,
        true,
        None,
        None,
    )
    .expect("the capture state is valid");
    assert_eq!(result.status, RuptureStatus::Rupture);
    assert_eq!(result.fluid_basis, FluidBasis::Gas);
    close(
        result.rupture_time.expect("a rupture has a time").value,
        115.0,
        "the rupture time",
    );
    close(
        result
            .rupture_pressure
            .expect("a rupture has a pressure")
            .value
            / 1.0e5,
        32.0255,
        "the rupture pressure",
    );
    close(
        result
            .rupture_mean_wall_temperature
            .expect("a rupture has a mean wall temperature")
            .value,
        1363.995_650_233_863_4,
        "the mean wall temperature",
    );
    close(
        result
            .rupture_outer_surface_temperature
            .expect("a rupture has an outer surface temperature")
            .value,
        1365.901_405_217_304_9,
        "the outer surface temperature",
    );
    close(
        result
            .rupture_accumulated_strain
            .expect("a rupture has a strain"),
        0.982_627_579_282_624_1,
        "the accumulated strain",
    );
    close(
        result.rupture_strain_limit.expect("a rupture has a limit"),
        0.484_437_605_397_567_7,
        "the strain limit",
    );
    close(
        result
            .long_pipe_gas_two_sides
            .expect("a rupture has a release")
            .value,
        22.344_184_183_997_67,
        "the two-sided release",
    );
    close(
        result
            .long_pipe_gas_one_side
            .expect("a rupture has a release")
            .value,
        11.172_092_091_998_834,
        "the one-sided release",
    );
    close(
        result
            .short_pipe_gas_one_side
            .expect("a rupture has a release")
            .value,
        17.216_193_913_770_2,
        "the short-pipe release",
    );
    close(
        result
            .liquid_one_side
            .expect("a rupture has a release")
            .value,
        0.0,
        "the liquid release",
    );
    close(
        result
            .release_gas_temperature
            .expect("a rupture has a release temperature")
            .value,
        293.15,
        "the release temperature",
    );
    assert_eq!(result.history.len(), 24);
    let first = &result.history[0];
    close(first.time.value, 0.0, "the first time");
    close(first.pressure.value / 1.0e5, 60.3, "the first pressure");
    close(
        first.heat_flux.value,
        313_552.689_842_975_7,
        "the first heat flux",
    );
    close(
        first.von_mises_stress.value / 1.0e6,
        66.892_718_196_875_08,
        "the first stress",
    );
    close(
        first.strain_rate.value,
        2.142_168_794_355_876_4e-18 / 60.0,
        "the first rate",
    );
}

#[test]
fn the_no_rupture_state_has_no_release() {
    let result = run(
        "22Cr duplex",
        "large_jet_fire",
        10.0,
        23.748_703_715_501_2,
        true,
        None,
        None,
    )
    .expect("the capture state is valid");
    assert_eq!(result.status, RuptureStatus::NoRupture);
    assert!(result.long_pipe_gas_two_sides.is_none());
    assert!(result.rupture_time.is_none());
    assert_eq!(result.history.len(), 3);
}

#[test]
fn the_built_in_and_explicit_small_jet_agree() {
    let built_in = run(
        "22Cr duplex",
        "small_jet_fire",
        200.0,
        23.748_703_715_501_2,
        true,
        None,
        None,
    )
    .expect("the capture state is valid");
    let explicit = run(
        "22Cr duplex",
        "explicit",
        200.0,
        23.748_703_715_501_2,
        true,
        None,
        Some((1273.15, 1273.15, 100.0, 1.0, 0.85, 0.85)),
    )
    .expect("the capture state is valid");
    assert_eq!(built_in.status, RuptureStatus::NoRupture);
    assert_eq!(explicit.status, RuptureStatus::NoRupture);
    assert_eq!(built_in.history.len(), 41);
    close(
        built_in
            .history
            .last()
            .expect("a history")
            .mean_wall_temperature
            .value,
        explicit
            .history
            .last()
            .expect("a history")
            .mean_wall_temperature
            .value,
        "the final mean wall temperature",
    );
}

#[test]
fn the_stainless_and_pool_fire_states() {
    let stainless = run(
        "SS 316",
        "large_jet_fire",
        200.0,
        23.748_703_715_501_2,
        true,
        None,
        None,
    )
    .expect("the capture state is valid");
    assert_eq!(stainless.status, RuptureStatus::Rupture);
    close(
        stainless.rupture_time.expect("a rupture has a time").value,
        180.0,
        "the SS 316 rupture time",
    );
    close(
        stainless
            .rupture_pressure
            .expect("a rupture has a pressure")
            .value
            / 1.0e5,
        30.1758,
        "the SS 316 rupture pressure",
    );
    close(
        stainless
            .long_pipe_gas_two_sides
            .expect("a rupture has a release")
            .value,
        21.053_648_907_885_176,
        "the SS 316 release",
    );

    let pool = run(
        "22Cr duplex",
        "pool_fire",
        200.0,
        23.748_703_715_501_2,
        true,
        None,
        None,
    )
    .expect("the capture state is valid");
    assert_eq!(pool.status, RuptureStatus::Rupture);
    close(
        pool.rupture_time.expect("a rupture has a time").value,
        160.0,
        "the pool-fire rupture time",
    );
    close(
        pool.rupture_pressure
            .expect("a rupture has a pressure")
            .value
            / 1.0e5,
        30.1758,
        "the pool-fire rupture pressure",
    );
}

#[test]
fn the_liquid_branch_and_the_thermal_mass_pair() {
    let liquid = run(
        "22Cr duplex",
        "large_jet_fire",
        200.0,
        800.0,
        true,
        None,
        None,
    )
    .expect("the capture state is valid");
    assert_eq!(liquid.status, RuptureStatus::NoRupture);
    assert_eq!(liquid.fluid_basis, FluidBasis::Liquid);

    let off = run(
        "22Cr duplex",
        "large_jet_fire",
        60.0,
        23.748_703_715_501_2,
        false,
        None,
        None,
    )
    .expect("the capture state is valid");
    let on = run(
        "22Cr duplex",
        "large_jet_fire",
        60.0,
        23.748_703_715_501_2,
        true,
        None,
        None,
    )
    .expect("the capture state is valid");
    assert_eq!(off.status, RuptureStatus::NoRupture);
    assert_eq!(on.status, RuptureStatus::NoRupture);
    assert_eq!(off.history.len(), 13);
    assert!(
        off.history
            .last()
            .expect("a history")
            .mean_wall_temperature
            .value
            > on.history
                .last()
                .expect("a history")
                .mean_wall_temperature
                .value,
        "without the gas mass the wall runs hotter"
    );
}

#[test]
fn the_pfp_factor_prevents_the_rupture() {
    let result = run(
        "22Cr duplex",
        "large_jet_fire",
        200.0,
        23.748_703_715_501_2,
        true,
        Some(0.1),
        None,
    )
    .expect("the capture state is valid");
    assert_eq!(result.status, RuptureStatus::NoRupture);
    assert_eq!(result.history.len(), 41);
}

#[test]
fn the_class_refusals_are_the_ports() {
    assert!(
        run(
            "unobtainium",
            "large_jet_fire",
            200.0,
            23.748,
            true,
            None,
            None
        )
        .is_err(),
        "an unknown material is refused"
    );
    assert!(
        run("22Cr duplex", "jet_fire", 200.0, 23.748, true, None, None).is_err(),
        "an unknown scenario is refused"
    );
    let missing = run("22Cr duplex", "explicit", 200.0, 23.748, true, None, None);
    assert!(missing.is_err(), "an incomplete explicit group is refused");

    let profile = workbook();
    let zero_step = pipe_fire_rupture(
        meters(0.0889),
        meters(0.0037),
        "22Cr duplex",
        "large_jet_fire",
        &profile,
        None,
        Some(0.10),
        Some(meters(1.0)),
        Some(megapascals(0.0)),
        Some(kilograms_per_cubic_meter(23.748)),
        Some(joules_per_kilogram_kelvin(2283.354)),
        Some(kilograms_per_mole(0.0182)),
        Some(kelvins(293.15)),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(bars(1.0)),
        Some("step_previous"),
        Some(seconds(0.0)),
        Some(seconds(200.0)),
        Some(true),
    );
    assert!(zero_step.is_err(), "a zero step is refused");

    let huge = pipe_fire_rupture(
        meters(0.0889),
        meters(0.0037),
        "22Cr duplex",
        "large_jet_fire",
        &profile,
        None,
        Some(0.10),
        Some(meters(1.0)),
        Some(megapascals(0.0)),
        Some(kilograms_per_cubic_meter(23.748)),
        Some(joules_per_kilogram_kelvin(2283.354)),
        Some(kilograms_per_mole(0.0182)),
        Some(kelvins(293.15)),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(bars(1.0)),
        Some("step_previous"),
        Some(seconds(0.001)),
        Some(seconds(200.0)),
        Some(true),
    );
    assert!(huge.is_err(), "a step count over the cap is refused");
}
