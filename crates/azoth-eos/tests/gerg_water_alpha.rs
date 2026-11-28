//! Spec-driven tests for `eos.gerg_water_alpha`.

use azoth_eos::gerg_water_alpha;
use azoth_eos::spec_gen;
use azoth_test_support as common;

const CALC_ID: &str = "eos.gerg_water_alpha";

fn call(case: &azoth_core::spec::TestCase) -> azoth_eos::GergWaterAlphaResult {
    gerg_water_alpha(
        common::input(case, "temperature"),
        common::input(case, "critical_temperature"),
    )
    .unwrap_or_else(|e| panic!("test `{}` should compute but failed: {e}", case.id))
}

#[test]
fn every_case_in_the_spec() {
    let spec = common::spec(spec_gen::specs(), CALC_ID);
    common::assert_skips_are_explained(spec);

    let mut executed = 0;
    for case in spec.all_tests() {
        if !case.is_active() {
            continue;
        }
        match case.kind {
            "worked_example" | "reference" => {
                let result = call(case);
                common::assert_close(
                    result.alpha,
                    common::expected(case, "alpha"),
                    case.tolerance,
                    &format!("{}::{} (alpha)", spec.id, case.id),
                );
                common::assert_consistent(&result, &format!("{}::{}", spec.id, case.id));
                common::assert_warnings_agree_with_spec(
                    spec,
                    &result.warnings,
                    |quantity| match quantity {
                        "temperature" => Some(common::input(case, "temperature")),
                        "critical_temperature" => Some(common::input(case, "critical_temperature")),
                        "alpha" => Some(result.alpha),
                        _ => None,
                    },
                    &format!("{}::{}", spec.id, case.id),
                );
            }
            "property" => match case.property {
                Some("unit_round_trip") => {}
                other => panic!("{}::{}: unhandled property {other:?}", spec.id, case.id),
            },
            other => panic!("{}::{}: unknown test kind {other:?}", spec.id, case.id),
        }
        executed += 1;
    }
    assert!(
        executed >= 3,
        "expected the worked example, the reference cases and the round trip, ran {executed}"
    );
}
