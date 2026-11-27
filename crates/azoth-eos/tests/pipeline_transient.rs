//! Spec-driven tests for `eos.pipeline_transient`.
//!
//! The spec's case pins the two implementations to each other. These tests are for the
//! two things a case cannot say: the conservation identities the stage promises, and
//! the CFL refusal.

use azoth_core::units::{
    kelvins, kilograms_per_square_meter_second, meters, pascals, seconds, watts_per_square_meter,
};
use azoth_core::AzothError;
use azoth_eos::pipeline_transient::pipeline_transient;
use azoth_eos::{Cubic, PipelineTransientResult, databank, model_gen};
use azoth_test_support as common;

const MODEL_ID: &str = "eos.pipeline_transient";

fn call_with(case: &azoth_core::spec::TestCase, time_step: f64) -> azoth_core::Result<PipelineTransientResult> {
    let names = case.list("components").expect("the case declares components");
    let (mixture, ideal_gas) =
        databank::mixture_of(names, Cubic::Pr, None).expect("the case's fluid resolves");
    let cells = case.input("cells").expect("the case declares cells");
    let n = names.len();
    let flat = case
        .matrix("initial_component_density")
        .expect("the case declares the matrix");
    let rows: Vec<Vec<f64>> = flat.chunks(n).map(<[f64]>::to_vec).collect();
    let momentum: Vec<_> = case
        .vector("initial_momentum")
        .expect("the case declares the momentum")
        .iter()
        .map(|&value| kilograms_per_square_meter_second(value))
        .collect();
    let energy: Vec<_> = case
        .vector("initial_total_energy")
        .expect("the case declares the energy")
        .iter()
        .map(|&value| pascals(value))
        .collect();
    pipeline_transient(
        &mixture,
        &ideal_gas,
        meters(case.input("length").expect("length")),
        meters(case.input("diameter").expect("diameter")),
        cells,
        case.input("friction_factor").expect("friction_factor"),
        case.input("wall_heat_flux")
            .map(watts_per_square_meter),
        case.string("left_boundary").expect("left_boundary"),
        case.string("right_boundary").expect("right_boundary"),
        case.input("left_boundary_pressure").map(pascals),
        case.input("right_boundary_pressure").map(pascals),
        case.input("left_boundary_temperature").map(kelvins),
        case.input("right_boundary_temperature").map(kelvins),
        &rows,
        &momentum,
        &energy,
        seconds(time_step),
        seconds(case.input("duration").expect("duration")),
        case.input("cfl_number"),
    )
}

fn call(case: &azoth_core::spec::TestCase) -> PipelineTransientResult {
    call_with(case, case.input("time_step").expect("time_step"))
        .unwrap_or_else(|e| panic!("case `{}` should compute but failed: {e}", case.id))
}

#[test]
fn every_case_in_the_spec() {
    let spec = model_gen::model(MODEL_ID).expect("the model should be in its own table");
    assert!(!spec.cases.is_empty(), "the model should have cases");

    for case in spec.cases {
        let result = call(case);
        let context = &format!("{}::{}", spec.id, case.id);
        let tolerance = case.tolerance;
        for name in [
            "final_velocity",
            "final_pressure",
            "final_temperature",
            "final_vapour_fraction",
            "final_density",
            "final_liquid_volume_fraction",
            "boundary_component_outflow",
            "initial_component_holdup",
            "final_component_holdup",
        ] {
            let got: &[f64] = match name {
                "final_velocity" => &result.final_velocity,
                "final_pressure" => &result.final_pressure,
                "final_temperature" => &result.final_temperature,
                "final_vapour_fraction" => &result.final_vapour_fraction,
                "final_density" => &result.final_density,
                "final_liquid_volume_fraction" => &result.final_liquid_volume_fraction,
                "boundary_component_outflow" => &result.boundary_component_outflow,
                "initial_component_holdup" => &result.initial_component_holdup,
                _ => &result.final_component_holdup,
            };
            let want = case.expected_vector(name).expect("the case states the vector");
            for (i, (&got, &want)) in got.iter().zip(want).enumerate() {
                common::assert_close(got, want, tolerance, &format!("{context} ({name}[{i}])"));
            }
        }
        for (name, got) in [
            ("steps", f64::from(result.steps)),
            ("rejections", f64::from(result.rejections)),
            ("min_time_step", result.min_time_step),
            ("initial_mass", result.initial_mass),
            ("final_mass", result.final_mass),
            ("boundary_mass_outflow", result.boundary_mass_outflow),
        ] {
            let want = case.expected_value(name).expect("the case states the scalar");
            common::assert_close(got, want, tolerance, &format!("{context} ({name})"));
        }
        common::assert_consistent(&result, context);
    }
}

#[test]
fn mass_and_component_inventories_close() {
    let spec = model_gen::model(MODEL_ID).expect("the model should be in its own table");
    let case = &spec.cases[0];
    let result = call(case);
    let context = &format!("{}::{}", spec.id, case.id);
    common::assert_close(
        result.initial_mass - result.final_mass,
        result.boundary_mass_outflow,
        1.0e-09,
        &format!("{context} (mass balance)"),
    );
    for (i, ((initial, final_), outflow)) in result
        .initial_component_holdup
        .iter()
        .zip(&result.final_component_holdup)
        .zip(&result.boundary_component_outflow)
        .enumerate()
    {
        common::assert_close(
            initial - final_,
            *outflow,
            1.0e-09,
            &format!("{context} (component {i} balance)"),
        );
    }
}

#[test]
fn a_step_above_the_initial_cfl_limit_is_refused() {
    let spec = model_gen::model(MODEL_ID).expect("the model should be in its own table");
    let case = &spec.cases[0];
    match call_with(case, 1000.0) {
        Err(AzothError::OutOfRange { field, .. }) => assert_eq!(field, "time_step"),
        other => panic!("a step above CFL should be refused, got {other:?}"),
    }
}
