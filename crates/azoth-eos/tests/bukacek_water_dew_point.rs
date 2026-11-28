//! Spec-driven tests for `eos.bukacek_water_dew_point`.

use azoth_eos::bukacek_water_dew_point;
use azoth_eos::spec_gen;
use azoth_test_support as common;

const CALC_ID: &str = "eos.bukacek_water_dew_point";

fn call(case: &azoth_core::spec::TestCase) -> azoth_eos::BukacekWaterDewPointResult {
    bukacek_water_dew_point(
        common::input(case, "water_mole_fraction"),
        common::input(case, "pressure"),
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
                    result.temperature,
                    common::expected(case, "temperature"),
                    case.tolerance,
                    &format!("{}::{} (temperature)", spec.id, case.id),
                );
                common::assert_consistent(&result, &format!("{}::{}", spec.id, case.id));
                common::assert_warnings_agree_with_spec(
                    spec,
                    &result.warnings,
                    |quantity| match quantity {
                        "water_mole_fraction" => Some(common::input(case, "water_mole_fraction")),
                        "pressure" => Some(common::input(case, "pressure")),
                        "temperature" => Some(result.temperature),
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
        "expected the worked example, the reference case and the round trip, ran {executed}"
    );
}
