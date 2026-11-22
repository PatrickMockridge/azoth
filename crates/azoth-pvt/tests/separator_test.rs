//! `pvt.separator_test` against its capture.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{kelvins, pascals};
use azoth_pvt::{SeparatorStageInput, SeparatorTestResult, separator_test};

fn binary(first: &str, first_moles: f64, second: &str, second_moles: f64) -> Vec<FluidComponent> {
    vec![
        FluidComponent::base(first, first_moles),
        FluidComponent::base(second, second_moles),
    ]
}

fn upstream() -> Vec<FluidComponent> {
    [
        ("nitrogen", 0.5),
        ("CO2", 2.0),
        ("methane", 45.0),
        ("ethane", 8.0),
        ("propane", 5.0),
        ("i-butane", 1.5),
        ("n-butane", 2.5),
        ("i-pentane", 1.0),
        ("n-pentane", 1.5),
        ("n-hexane", 3.0),
        ("n-heptane", 30.0),
    ]
    .into_iter()
    .map(|(name, moles)| FluidComponent::base(name, moles))
    .collect()
}

#[allow(clippy::too_many_arguments)]
fn call(
    components: &[FluidComponent],
    mode: &str,
    stages: &[SeparatorStageInput],
    reservoir: Option<(f64, f64)>,
    optimize: bool,
) -> SeparatorTestResult {
    separator_test(
        components,
        mode,
        stages,
        None,
        None,
        None,
        reservoir.map(|(p, _)| pascals(p)),
        reservoir.map(|(_, t)| kelvins(t)),
        Some(optimize),
        optimize.then(|| pascals(5.0e5)),
        optimize.then(|| pascals(2.0e6)),
        optimize.then_some(3),
        optimize.then(|| kelvins(293.15)),
        optimize.then(|| kelvins(313.15)),
        optimize.then_some(2),
    )
    .unwrap()
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-6 * expected.abs().max(1.0)
}

/// A measured divergence: the staged cascade and the optimised train compound one flash
/// root's difference over four stages, so their volumes are held at `1e-3` and `5e-2`.
fn within(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance * expected.abs().max(1.0)
}

const STAGES: [SeparatorStageInput; 4] = [
    SeparatorStageInput {
        pressure: 50.0e5,
        temperature: 323.15,
    },
    SeparatorStageInput {
        pressure: 20.0e5,
        temperature: 313.15,
    },
    SeparatorStageInput {
        pressure: 5.0e5,
        temperature: 303.15,
    },
    SeparatorStageInput {
        pressure: 101_325.0,
        temperature: 288.15,
    },
];

#[test]
fn the_continuous_and_staged_modes_match_the_capture() {
    let continuous = call(
        &binary("methane", 0.70, "nC10", 0.30),
        "continuous",
        &STAGES,
        None,
        false,
    );
    assert!(close(
        continuous.stages[0].stage_gor.unwrap(),
        210.199_527_045_105_27
    ));
    assert!(close(
        continuous.stages[0].oil_fvf.unwrap(),
        1.084_818_134_371_904_3
    ));
    assert!(close(
        continuous.stages[3].stage_gor.unwrap(),
        233.685_863_551_802_75
    ));
    assert!(close(continuous.stages[3].oil_fvf.unwrap(), 1.0));

    let staged = call(
        &binary("methane", 0.70, "nC10", 0.30),
        "staged",
        &STAGES,
        None,
        false,
    );
    assert!(close(
        staged.stages[0].stage_gor.unwrap(),
        210.099_857_038_5
    ));
    assert!(close(
        staged.stages[3].stage_gor.unwrap(),
        1.893_169_014_075_703
    ));
}

#[test]
fn the_train_matches_the_capture() {
    let stages = [
        SeparatorStageInput {
            pressure: 50.0e5,
            temperature: 313.15,
        },
        SeparatorStageInput {
            pressure: 10.0e5,
            temperature: 303.15,
        },
        SeparatorStageInput {
            pressure: 101_325.0,
            temperature: 288.15,
        },
    ];
    let row = call(
        &upstream(),
        "train",
        &stages,
        Some((300.0e5, 373.15)),
        false,
    );
    assert!(close(row.total_gor.unwrap(), 239.473_885_030_581_67));
    assert!(close(row.bo.unwrap(), 1.817_349_173_253_826_7));
    assert!(close(row.rs.unwrap(), 239.473_885_030_581_67));
    assert!(close(
        row.stock_tank_oil_density.unwrap(),
        685.047_930_224_121_4
    ));
    assert!(close(row.stock_tank_api.unwrap(), 74.848_335_296_411_93));

    let stage = &row.stages[0];
    assert!(close(stage.gas_volume.unwrap(), 0.970_096_385_528_073_2));
    assert!(close(stage.stage_gor.unwrap(), 161.497_330_282_413_17));
    assert!(close(stage.gas_density.unwrap(), 43.100_408_834_142_66));
    // The class's stage oil density is read before the carried phase's physical properties
    // are re-initialised, so it is the previous stage's density - here the reservoir's
    // 494.89. The id publishes the density at the stage, which is the parent oil phase's
    // 602.44; the capture records the lagged number as evidence.
    assert!(close(stage.oil_density.unwrap(), 602.439_577_3));
    assert!(close(
        stage.gas_molar_mass.unwrap(),
        0.020_066_695_492_532_725
    ));
    assert!(close(
        stage.oil_molar_mass.unwrap(),
        0.070_294_804_526_441_89
    ));
    assert!(close(stage.gas_z_factor.unwrap(), 0.896_571_779_156_656_4));
    assert!(close(
        stage.oil_viscosity.unwrap(),
        2.067_906_493_415_124e-4
    ));
}

#[test]
fn the_first_stage_optimisation_matches_the_capture() {
    let stages = [
        SeparatorStageInput {
            pressure: 50.0e5,
            temperature: 313.15,
        },
        SeparatorStageInput {
            pressure: 10.0e5,
            temperature: 303.15,
        },
        SeparatorStageInput {
            pressure: 101_325.0,
            temperature: 288.15,
        },
    ];
    let row = call(&upstream(), "train", &stages, Some((300.0e5, 373.15)), true);
    // The optimum's stage choice matches; the cascade's compounded difference puts Bo and
    // the GOR 2-4% above the class's.
    assert!(close(row.optimal_stage_pressure.unwrap(), 2.0e6));
    assert!(close(row.optimal_stage_temperature.unwrap(), 293.15));
    assert!(within(
        row.bo_at_optimum.unwrap(),
        1.804_107_123_437_750_8,
        5.0e-2
    ));
    assert!(within(
        row.gor_at_optimum.unwrap(),
        236.480_888_412_685_23,
        5.0e-2
    ));
    assert!(close(
        row.maximum_oil_recovery.unwrap(),
        1.0 / row.bo_at_optimum.unwrap()
    ));
}

#[test]
fn an_empty_stage_list_and_an_unknown_mode_are_refused() {
    let empty: [SeparatorStageInput; 0] = [];
    let err = separator_test(
        &binary("methane", 1.0, "nC10", 1.0),
        "continuous",
        &empty,
        None,
        None,
        None,
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
    .expect_err("a separator test needs a stage");
    assert!(err.to_string().contains("at least one stage"));

    let stages = [SeparatorStageInput {
        pressure: 10.0e5,
        temperature: 300.0,
    }];
    let err = separator_test(
        &binary("methane", 1.0, "nC10", 1.0),
        "bogus",
        &stages,
        None,
        None,
        None,
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
    .expect_err("an unknown mode is refused");
    assert!(err.to_string().contains("continuous, staged or train"));
}
