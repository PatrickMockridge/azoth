//! The column's declared solver strategies.

use azoth_process::kernels::distillation_column::SolverType;

/// Every strategy the model's enum declares round-trips its own name, and a name that is not
/// one of the ten is not a strategy.
#[test]
fn every_strategy_round_trips_its_declared_name() {
    for name in [
        "direct_substitution",
        "damped_substitution",
        "inside_out",
        "matrix_inside_out",
        "wegstein",
        "sum_rates",
        "newton",
        "naphtali_sandholm",
        "mesh_residual",
        "auto",
    ] {
        let solver = SolverType::from_name(name).expect("a declared strategy");
        assert_eq!(solver.name(), name);
    }
    assert_eq!(SolverType::from_name("bogus"), None);
}
