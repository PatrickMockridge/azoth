//! Tests for `eos.umr_pru_flash`.
//!
//! The spec's case pins the two implementations to each other. These tests are for the
//! two things a case cannot say: the split's own identities, and the co-volume rule
//! `SystemUMRPRUEos` installs with `setBmixType(1)`.

use azoth_core::WarningCode;
use azoth_core::units::{kelvins, pascals};
use azoth_eos::umr_pru_flash::umr_pru_flash;
use azoth_eos::{UmrPruFlashResult, databank, model_gen};
use azoth_test_support as common;

const MODEL_ID: &str = "eos.umr_pru_flash";

fn names(case: &azoth_core::spec::TestCase) -> Vec<String> {
    case.list("components")
        .expect("the case declares components")
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

fn call(case: &azoth_core::spec::TestCase) -> UmrPruFlashResult {
    umr_pru_flash(
        &names(case).iter().map(String::as_str).collect::<Vec<_>>(),
        kelvins(common::input(case, "T")),
        pascals(common::input(case, "P")),
        case.vector("z").expect("z"),
        None,
    )
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
        if let Some(want) = case.expected_value("vapour_fraction") {
            let got = result.vapour_fraction.expect("the case expects a split");
            common::assert_close(got, want, tolerance, &format!("{context} (beta)"));
        }
        for (name, got) in [
            ("x", &result.x),
            ("y", &result.y),
            ("k", &result.k),
            ("ln_phi_liquid", &result.ln_phi_liquid),
            ("ln_phi_vapour", &result.ln_phi_vapour),
        ] {
            let want = case
                .expected_vector(name)
                .expect("the case states the vector");
            for (i, (&got, &want)) in got.iter().zip(want).enumerate() {
                common::assert_close(got, want, tolerance, &format!("{context} ({name}[{i}])"));
            }
        }
        for name in ["z_liquid", "z_vapour", "min_t_over_tc"] {
            if let Some(want) = case.expected_value(name) {
                let got = match name {
                    "z_liquid" => result.z_liquid,
                    "z_vapour" => result.z_vapour,
                    _ => result.min_t_over_tc,
                };
                common::assert_close(got, want, tolerance, &format!("{context} ({name})"));
            }
        }
        assert_eq!(
            result.phase.as_str(),
            case.expected_string("phase")
                .expect("the case states the phase")
        );
        common::assert_consistent(&result, context);
    }
}

fn reference_split() -> UmrPruFlashResult {
    umr_pru_flash(
        &["methane", "n-butane"],
        kelvins(330.0),
        pascals(25.0e5),
        &[0.6, 0.4],
        None,
    )
    .expect("the reference state computes")
}

#[test]
fn the_split_satisfies_the_flash_equations() {
    let result = reference_split();
    let beta = result.vapour_fraction.expect("two-phase");
    assert!(0.0 < beta && beta < 1.0, "beta = {beta}");

    for (i, (&x, &y)) in result.x.iter().zip(&result.y).enumerate() {
        common::assert_close(
            x * (1.0 - beta) + y * beta,
            [0.6, 0.4][i],
            1.0e-12,
            "material balance",
        );
        common::assert_close(
            (x.ln() + result.ln_phi_liquid[i]).exp(),
            (y.ln() + result.ln_phi_vapour[i]).exp(),
            1.0e-9,
            "isofugacity",
        );
    }
    common::assert_close(result.x.iter().sum::<f64>(), 1.0, 1.0e-12, "sum x");
    common::assert_close(result.y.iter().sum::<f64>(), 1.0, 1.0e-12, "sum y");
}

/// `SystemUMRPRUEos` sets `setBmixType(1)`: `b_ij = ((sqrt(b_i) + sqrt(b_j))/2)^2`.
///
/// The linear sum would give a different mixture co-volume; this pins the one the class
/// actually uses.
#[test]
fn the_co_volume_is_the_quadratic_rule() {
    let (mixture, _) = databank::umr_pru_mixture_of(&["methane", "n-butane"])
        .expect("the pair resolves for UMR-PRU");
    let reduced = mixture
        .reduced_parameters(kelvins(330.0), pascals(25.0e5))
        .expect("the state reduces");
    let x = [0.6, 0.4];
    let (_, b_mix) = mixture.mixture_parameters(&reduced, &x);
    let bij = |i: usize, j: usize| ((reduced.b[i].sqrt() + reduced.b[j].sqrt()) * 0.5).powi(2);
    let expected =
        x[0] * x[0] * bij(0, 0) + 2.0 * x[0] * x[1] * bij(0, 1) + x[1] * x[1] * bij(1, 1);
    common::assert_close(b_mix, expected, 1.0e-14, "quadratic co-volume");
}

#[test]
fn a_cap_is_a_result() {
    let result = umr_pru_flash(
        &["methane", "n-butane"],
        kelvins(330.0),
        pascals(25.0e5),
        &[0.6, 0.4],
        Some(1.0e-300),
    )
    .expect("an unreachable tolerance is a cap, not an error");
    assert!(!result.converged);
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.code == WarningCode::SolverNotConverged),
        "a cap carries the warning"
    );
}
