//! Spec-driven tests for `eos.transient_pipe`.

use azoth_core::units::{kelvins, kilograms_per_second, meters, pascals, seconds};
use azoth_eos::model_gen;
use azoth_eos::results::TransientPipeResult;
use azoth_eos::transient_pipe::transient_pipe;
use azoth_test_support as common;

const MODEL_ID: &str = "eos.transient_pipe";
const MAX_TIME_STEP: f64 = 10.0;

fn call(case: &azoth_core::spec::TestCase) -> TransientPipeResult {
    let components: Vec<String> = common::list_input(case, "components")
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    transient_pipe(
        &components,
        case.vector("inlet_z")
            .expect("the case states its composition"),
        kilograms_per_second(common::input(case, "inlet_mass_flow")),
        pascals(common::input(case, "inlet_p")),
        kelvins(common::input(case, "inlet_t")),
        meters(common::input(case, "length")),
        meters(common::input(case, "diameter")),
        common::input(case, "number_of_sections") as usize,
        meters(common::input(case, "roughness")),
        common::input_str(case, "inlet_boundary"),
        common::input_str(case, "outlet_boundary"),
        case.vector("elevation_profile"),
        case.vector("inclination_profile"),
        case.input("outlet_pressure").map(pascals),
        case.input("max_simulation_time").map(seconds),
        case.input("cfl"),
        case.input("thermodynamic_update_interval")
            .map(|value| value as usize),
        case.string("eos"),
    )
    .unwrap_or_else(|e| panic!("case `{}` should compute but failed: {e}", case.id))
}

fn assert_vector(case: &azoth_core::spec::TestCase, context: &str, name: &str, got: &[f64]) {
    if let Some(expected) = case.expected_vector(name) {
        assert_eq!(got.len(), expected.len(), "{context}: {name} length");
        for (index, (value, want)) in got.iter().zip(expected).enumerate() {
            common::assert_close(
                *value,
                *want,
                case.tolerance,
                &format!("{context}: {name}[{index}]"),
            );
        }
    }
}

#[test]
fn every_case_in_the_spec() {
    let spec = model_gen::model(MODEL_ID).expect("the model should be in its table");
    assert!(!spec.cases.is_empty(), "the model should have cases");

    for case in spec.cases {
        let result = call(case);
        let context = &format!("{}::{}", spec.id, case.id);
        assert_vector(case, context, "pressure_profile", &result.pressure_profile);
        assert_vector(
            case,
            context,
            "temperature_profile",
            &result.temperature_profile,
        );
        assert_vector(
            case,
            context,
            "liquid_holdup_profile",
            &result.liquid_holdup_profile,
        );
        assert_vector(
            case,
            context,
            "gas_velocity_profile",
            &result.gas_velocity_profile,
        );
        assert_vector(
            case,
            context,
            "liquid_velocity_profile",
            &result.liquid_velocity_profile,
        );
        if let Some(expected) = case.matrix("pressure_history") {
            let columns = result
                .pressure_history
                .first()
                .map_or(0, std::vec::Vec::len);
            assert_eq!(
                expected.len(),
                result.pressure_history.len() * columns,
                "{context}: pressure_history shape"
            );
            for (index, value) in expected.iter().enumerate() {
                let row = index / columns;
                let column = index % columns;
                common::assert_close(
                    result.pressure_history[row][column],
                    *value,
                    case.tolerance,
                    &format!("{context}: pressure_history[{row}][{column}]"),
                );
            }
        }
        if let Some(want) = case.expected_value("mass_inventory") {
            common::assert_close(
                result.mass_inventory.value,
                want,
                case.tolerance,
                &format!("{context}: mass_inventory"),
            );
        }
        let max_time = case.input("max_simulation_time").unwrap_or(3600.0);
        assert!(
            result.simulation_time.value >= max_time,
            "{context}: the march ended before its stated time"
        );
        assert!(
            result.simulation_time.value < max_time + MAX_TIME_STEP,
            "{context}: the last step overshot by more than the class's maximum"
        );
        assert!(result.total_time_steps > 0, "{context}: no step ran");
        common::assert_consistent(&result, context);
    }
}

#[test]
fn the_three_phase_flash_is_refused() {
    let error = transient_pipe(
        &[
            "methane".to_owned(),
            "n-heptane".to_owned(),
            "water".to_owned(),
        ],
        &[0.5, 0.3, 0.2],
        kilograms_per_second(5.0),
        pascals(5.0e6),
        kelvins(300.0),
        meters(100.0),
        meters(0.1),
        5,
        meters(1.0e-4),
        "constant_flow",
        "constant_pressure",
        None,
        None,
        Some(pascals(4.0e6)),
        Some(seconds(1.0)),
        Some(0.5),
        Some(10),
        Some("srk"),
    )
    .expect_err("a three-phase flash has no port");
    assert!(
        format!("{error}").contains("oil and an aqueous"),
        "the refusal should name the three-phase averaging: {error}"
    );
}

#[test]
fn a_low_point_is_refused() {
    let error = transient_pipe(
        &["methane".to_owned(), "n-butane".to_owned()],
        &[0.9, 0.1],
        kilograms_per_second(5.0),
        pascals(5.0e6),
        kelvins(300.0),
        meters(100.0),
        meters(0.1),
        5,
        meters(1.0e-4),
        "constant_flow",
        "constant_pressure",
        Some(&[0.0, -1.0, 0.0, 0.0, 0.0]),
        None,
        Some(pascals(4.0e6)),
        Some(seconds(1.0)),
        Some(0.5),
        Some(10),
        Some("srk"),
    )
    .expect_err("a local minimum is an accumulation zone");
    assert!(
        format!("{error}").contains("accumulation"),
        "the refusal should name the accumulation zone: {error}"
    );
}

#[test]
fn a_boundary_outside_the_three_is_refused() {
    let error = transient_pipe(
        &["methane".to_owned(), "n-butane".to_owned()],
        &[0.9, 0.1],
        kilograms_per_second(5.0),
        pascals(5.0e6),
        kelvins(300.0),
        meters(100.0),
        meters(0.1),
        5,
        meters(1.0e-4),
        "transient_flow",
        "constant_pressure",
        None,
        None,
        Some(pascals(4.0e6)),
        Some(seconds(1.0)),
        Some(0.5),
        Some(10),
        Some("srk"),
    )
    .expect_err("the transient boundary arm is outside this model");
    assert!(
        format!("{error}").contains("transient_flow"),
        "the refusal should name the boundary: {error}"
    );
}
