//! `process.heat_exchanger`'s `runTransient`, held to
//! `validation/neqsim/captures/process_heat_exchanger_transient.tsv`.
//!
//! The wall trajectory is flow-independent while no holdup is stated, so the upstream test's
//! ten steps are reproduced at the capture's cased flow. The CSTR states and the outlets
//! inherit the flash, so they carry a tolerance; the wall does not.

use azoth_core::AzothError;
use azoth_core::units::{kelvins, pascals};
use azoth_process::Stream;
use azoth_process::kernels::heat_exchanger::{TransientSettings, heat_exchanger_transient};

/// 100 000 kg/hr of methane, the capture's cased flow.
const N: f64 = 1731.4578182246325;

fn close(actual: f64, expected: f64, tolerance: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= tolerance,
        "{what}: {actual} against {expected}, relative error {error:.3e} > {tolerance:.3e}"
    );
}

fn methane(temperature: f64, pressure: f64, flow: f64) -> Stream {
    Stream::from_pt(
        vec!["methane".to_string()],
        vec![1.0],
        flow,
        pascals(pressure),
        kelvins(temperature),
    )
    .expect("methane flashes")
}

fn settings() -> TransientSettings {
    TransientSettings {
        time_step_s: 1.0,
        wall_mass_kg: 200.0,
        wall_cp_j_per_kg_k: 500.0,
        wall_temperature_k: None,
        shell_holdup_volume_m3: 0.0,
        tube_holdup_volume_m3: 0.0,
        shell_side_htc_w_per_m2_k: 500.0,
        tube_side_htc_w_per_m2_k: 800.0,
        heat_transfer_area_m2: 20.0,
        shell_fluid_temperature_k: None,
        tube_fluid_temperature_k: None,
        tube_is_hot: false,
    }
}

#[test]
fn the_first_step_seeds_the_wall_to_the_mean_of_the_inlets() {
    let hot = methane(353.15, 1.0e6, N);
    let cold = methane(293.15, 1.0e6, N);
    let outcome = heat_exchanger_transient(&hot, &cold, &settings()).expect("the step runs");
    let state = outcome
        .transient
        .expect("the dynamic branch carries a state");
    close(
        state.wall_temperature.value,
        322.142_861_459_910_8,
        1e-9,
        "wall",
    );
    close(
        state
            .shell_fluid_temperature
            .expect("the solved state")
            .value,
        349.133_379_559_042_1,
        1e-6,
        "shell fluid",
    );
    close(
        state
            .tube_fluid_temperature
            .expect("the solved state")
            .value,
        298.979_171_772_396_6,
        1e-6,
        "tube fluid",
    );
    close(
        outcome.hot_out.t.value,
        349.133_379_559_042_1,
        1e-6,
        "hot outlet",
    );
    close(
        outcome.cold_out.t.value,
        298.979_171_772_396_6,
        1e-6,
        "cold outlet",
    );
    close(outcome.duty.value, 269_905.180_991_312_95, 1e-6, "duty");
    close(
        state.thermal_inventory_energy.value,
        3.221_428_614_599_108_3e7,
        1e-9,
        "inventory",
    );
    close(
        state.balance_accumulation.value,
        -100_713.854_008_913_04,
        1e-9,
        "accumulation",
    );
    close(
        state.balance_boundary_energy.value,
        -100_713.854_008_911_18,
        1e-9,
        "boundary",
    );
    assert!(
        state.balance_residual.value.abs() < 1e-6,
        "residual {}",
        state.balance_residual.value
    );
}

#[test]
fn ten_steps_of_the_wall_ode_follow_the_capture() {
    let hot = methane(353.15, 1.0e6, N);
    let cold = methane(293.15, 1.0e6, N);
    // With no fluid holdup the wall reaches its stationary state on the first interval, so
    // every captured step carries the same wall.
    let expected = [322.142_861_459_910_8; 10];
    let settings = settings();
    for (step, want) in expected.iter().enumerate() {
        let outcome = heat_exchanger_transient(&hot, &cold, &settings).expect("the step runs");
        let state = outcome
            .transient
            .expect("the dynamic branch carries a state");
        close(
            state.wall_temperature.value,
            *want,
            1e-9,
            &format!("step {}", step + 1),
        );
    }
}

#[test]
fn the_holdup_cstrs_carry_the_fluid_temperatures() {
    let hot = methane(423.15, 3.0e6, N);
    let cold = methane(303.15, 3.0e6, N);
    let mut settings = settings();
    settings.wall_mass_kg = 400.0;
    settings.heat_transfer_area_m2 = 2.0;
    settings.shell_holdup_volume_m3 = 40.0;
    settings.tube_holdup_volume_m3 = 30.0;

    let first = heat_exchanger_transient(&hot, &cold, &settings).expect("the step runs");
    let state = first.transient.expect("the dynamic branch carries a state");
    close(
        state.wall_temperature.value,
        362.972_610_059_641_7,
        1e-9,
        "wall 1",
    );
    close(
        state
            .shell_fluid_temperature
            .expect("the solved state")
            .value,
        423.111_583_017_522_9,
        1e-6,
        "shell fluid 1",
    );
    close(
        state
            .tube_fluid_temperature
            .expect("the solved state")
            .value,
        303.212_009_416_177,
        1e-6,
        "tube fluid 1",
    );

    // The probe's five rows replay one interval under the same UUID; the capture's rows are
    // identical, so a second call with the same start state is the same state.
    let second = heat_exchanger_transient(&hot, &cold, &settings).expect("the step runs");
    let state = second
        .transient
        .expect("the dynamic branch carries a state");
    close(
        state.wall_temperature.value,
        362.972_610_059_641_7,
        1e-9,
        "wall 2",
    );
    close(
        state
            .shell_fluid_temperature
            .expect("the solved state")
            .value,
        423.111_583_017_522_9,
        1e-6,
        "shell fluid 2",
    );
    close(
        state
            .tube_fluid_temperature
            .expect("the solved state")
            .value,
        303.212_009_416_177,
        1e-6,
        "tube fluid 2",
    );
}

#[test]
fn the_interval_scale_moves_the_wall_and_the_outlets() {
    let hot = methane(353.15, 1.0e6, N);
    let cold = methane(293.15, 1.0e6, N);
    for (dt, wall, shell_out) in [
        (0.5, 322.597_568_647_903_8, 349.192_396_367_343_4),
        (1.0, 322.142_861_459_910_8, 349.133_379_559_042_1),
    ] {
        let mut settings = settings();
        settings.time_step_s = dt;
        settings.wall_temperature_k = Some(323.15);
        let outcome = heat_exchanger_transient(&hot, &cold, &settings).expect("the step runs");
        let state = outcome
            .transient
            .expect("the dynamic branch carries a state");
        close(state.wall_temperature.value, wall, 1e-9, "wall");
        close(outcome.hot_out.t.value, shell_out, 1e-6, "hot outlet");
    }
}

#[test]
fn a_hot_tube_maps_the_duty_to_the_hot_side() {
    let cold = methane(293.15, 1.0e6, N);
    let hot = methane(353.15, 1.0e6, N);
    let mut settings = settings();
    settings.tube_is_hot = true;
    let outcome = heat_exchanger_transient(&hot, &cold, &settings).expect("the step runs");
    let state = outcome
        .transient
        .expect("the dynamic branch carries a state");
    close(
        state.wall_temperature.value,
        324.206_422_677_742_07,
        1e-9,
        "wall",
    );
    // The class's `getDuty()` is the shell-to-wall transfer; on a cold shell it is negative.
    close(
        outcome.duty.value,
        -268_331.746_688_753_6,
        1e-6,
        "shell-to-wall duty",
    );
}

#[test]
fn a_zero_flow_side_is_refused() {
    let hot = methane(353.15, 1.0e6, 0.0);
    let cold = methane(293.15, 1.0e6, N);
    let error = heat_exchanger_transient(&hot, &cold, &settings())
        .err()
        .expect("zero flow");
    assert!(
        matches!(&error, AzothError::InvalidInput { field, .. } if field.as_str() == "hot_in_n"),
        "{error:?}"
    );
}

#[test]
fn a_two_phase_side_is_refused() {
    let two_phase = Stream::from_pt(
        vec!["methane".to_string(), "n-butane".to_string()],
        vec![0.5, 0.5],
        N,
        pascals(500_000.0),
        kelvins(250.0),
    )
    .expect("the mixture flashes");
    let cold = methane(293.15, 500_000.0, N);
    let error = heat_exchanger_transient(&two_phase, &cold, &settings())
        .err()
        .expect("two phases");
    assert!(
        matches!(&error, AzothError::InvalidInput { field, .. } if field.as_str() == "stream"),
        "{error:?}"
    );
}

#[test]
fn a_non_positive_wall_cp_is_refused() {
    let hot = methane(353.15, 1.0e6, N);
    let cold = methane(293.15, 1.0e6, N);
    let mut settings = settings();
    settings.wall_cp_j_per_kg_k = 0.0;
    let error = heat_exchanger_transient(&hot, &cold, &settings)
        .err()
        .expect("singular wall");
    assert!(
        matches!(&error, AzothError::InvalidInput { field, .. } if field.as_str() == "wall_cp"),
        "{error:?}"
    );
}

#[test]
fn a_non_positive_step_is_refused() {
    let hot = methane(353.15, 1.0e6, N);
    let cold = methane(293.15, 1.0e6, N);
    for dt in [0.0, -1.0] {
        let mut settings = settings();
        settings.time_step_s = dt;
        let error = heat_exchanger_transient(&hot, &cold, &settings)
            .err()
            .expect("a non-positive interval");
        assert!(
            matches!(&error, AzothError::InvalidInput { field, .. } if field.as_str() == "time_step"),
            "{error:?}"
        );
    }
}

#[test]
fn a_non_finite_carried_temperature_is_refused() {
    let hot = methane(353.15, 1.0e6, N);
    let cold = methane(293.15, 1.0e6, N);
    let mut settings = settings();
    settings.wall_temperature_k = Some(f64::INFINITY);
    let error = heat_exchanger_transient(&hot, &cold, &settings)
        .err()
        .expect("no such state");
    assert!(
        matches!(&error, AzothError::InvalidInput { field, .. } if field.as_str() == "wall_temperature"),
        "{error:?}"
    );
}

#[test]
fn a_nan_wall_temperature_is_the_first_step_seed() {
    let hot = methane(353.15, 1.0e6, N);
    let cold = methane(293.15, 1.0e6, N);
    let mut settings = settings();
    settings.wall_temperature_k = Some(f64::NAN);
    let outcome = heat_exchanger_transient(&hot, &cold, &settings).expect("the step runs");
    let state = outcome
        .transient
        .expect("the dynamic branch carries a state");
    close(
        state.wall_temperature.value,
        322.142_861_459_910_8,
        1e-9,
        "wall seed",
    );
}
