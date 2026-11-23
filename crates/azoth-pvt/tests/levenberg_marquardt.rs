//! The Marquardt loop, its Ridders derivative and its floor.

use azoth_core::AzothError;
use azoth_pvt::{ConvergenceReason, levenberg_marquardt};

fn one_parameter(parameters: &[f64]) -> Vec<f64> {
    vec![parameters[0], 2.0 * parameters[0], 3.0 * parameters[0]]
}

#[test]
fn one_parameter_recovers_its_value_and_stops_on_the_floor() {
    let observed = [2.0, 4.0, 6.0];
    let sigma = [1.0, 1.0, 1.0];
    let result = levenberg_marquardt(
        &[(0.0, 10.0)],
        &[1.0],
        50,
        &observed,
        &sigma,
        |parameters| Ok(one_parameter(parameters)),
    )
    .expect("the linear model solves");
    assert!((result.parameters[0] - 2.0).abs() < 1e-6, "{result:?}");
    assert_eq!(result.reason, ConvergenceReason::ChiSquareTolerance);
    assert_eq!(result.iterations, 5);
}

#[test]
fn two_parameters_recover_their_values() {
    let x = [0.0, 1.0, 2.0, 3.0];
    let observed = [1.0, 3.0, 5.0, 7.0];
    let sigma = [1.0; 4];
    let result = levenberg_marquardt(
        &[(-10.0, 10.0), (-10.0, 10.0)],
        &[0.0, 0.0],
        200,
        &observed,
        &sigma,
        |parameters| {
            Ok(x.iter()
                .map(|x| parameters[0] + parameters[1] * x)
                .collect())
        },
    )
    .expect("the two-parameter model solves");
    // The class stops on chi-square at the five-iteration floor, not on parameter
    // exactness: this linear fit lands 9e-5 from the intercept there.
    assert!((result.parameters[0] - 1.0).abs() < 1e-4, "{result:?}");
    assert!((result.parameters[1] - 2.0).abs() < 1e-4, "{result:?}");
    assert!(result.iterations >= 5);
}

#[test]
fn a_bound_pinned_parameter_is_singular() {
    let observed = [2.0];
    let sigma = [1.0];
    let result = levenberg_marquardt(&[(1.0, 1.0)], &[1.0], 50, &observed, &sigma, |parameters| {
        Ok(vec![parameters[0]])
    })
    .expect("the pinned model still answers at its state");
    assert_eq!(result.reason, ConvergenceReason::SingularMatrix);
    assert_eq!(result.iterations, 1);
}

#[test]
fn an_exactly_fit_model_still_takes_the_floor() {
    let observed = [1.0, 2.0, 3.0];
    let sigma = [1.0, 1.0, 1.0];
    let result = levenberg_marquardt(
        &[(0.0, 10.0)],
        &[1.0],
        50,
        &observed,
        &sigma,
        |parameters| Ok(one_parameter(parameters)),
    )
    .expect("the exact model solves");
    assert_eq!(result.iterations, 5);
    assert!(result.chi_square < 1e-12);
}

#[test]
fn an_evaluation_refusal_propagates() {
    let error = levenberg_marquardt(&[(0.0, 1.0)], &[0.5], 10, &[1.0], &[1.0], |_| {
        Err(AzothError::invalid_input("state", "the flash refused"))
    })
    .expect_err("the refusal is the answer");
    assert!(matches!(error, AzothError::InvalidInput { .. }));
}
