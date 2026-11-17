//! `process.basket_test_regression` against
//! `validation/neqsim/captures/process_basket_test_regression.tsv`.

use azoth_core::units::{kelvins, meters, watts_per_meter_kelvin};
use azoth_process::models::basket_test_regression::{
    BasketTestPoint, BasketTestRegressionResult, basket_test_regression,
};

const MISSING_CONDUCTIVITY: &str = "Effective thermal conductivity not supplied; the volumetric \
heat-release pre-factor could not be separated from the regression intercept";
const TWO_POINTS: &str = "Only 2 basket sizes supplied; at least 3 are recommended for a \
defensible fit and a meaningful coefficient of determination";

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn point(geometry: &str, dimension: f64, temperature: f64) -> BasketTestPoint {
    BasketTestPoint {
        geometry: geometry.to_string(),
        characteristic_dimension: meters(dimension),
        critical_temperature: kelvins(temperature),
    }
}

fn fit(points: &[BasketTestPoint], conductivity: Option<f64>) -> BasketTestRegressionResult {
    basket_test_regression(points, conductivity.map(watts_per_meter_kelvin))
        .expect("the captured state is valid")
}

#[test]
fn the_four_point_cube_set_recovers_both_constants() {
    let points = [
        point("cube", 0.0125, 634.626_468_271_017_1),
        point("cube", 0.025, 591.294_053_941_965_1),
        point("cube", 0.05, 553.737_232_834_100_7),
        point("cube", 0.1, 520.846_261_829_137_8),
    ];
    let result = fit(&points, Some(0.09));
    close(result.point_count, 4.0, "the point count");
    close(result.slope, -13_229.959_020_078_551, "the slope");
    close(result.intercept, 43.441_229_356_038_03, "the intercept");
    close(
        result.r_squared.expect("the ordinates vary"),
        0.999_999_999_999_943_2,
        "r squared",
    );
    close(result.activation_energy.value, 109_999.999_710_115_04, "E");
    close(
        result
            .volumetric_pre_factor
            .expect("a conductivity was supplied"),
        5.000_002_332_125_494_5e13,
        "P",
    );
    assert!(result.conductivity_provided);
    assert!(result.messages.is_empty());
}

#[test]
fn the_mixed_geometries_fit_the_same_line() {
    let points = [
        point("cube", 0.025, 591.294_053_941_965_1),
        point("sphere", 0.05, 560.819_674_283_266_1),
        point("infinite_cylinder", 0.1, 515.754_719_823_598_9),
    ];
    let result = fit(&points, Some(0.09));
    close(result.slope, -13_229.960_782_943_328, "the slope");
    close(result.intercept, 43.441_231_981_608_31, "the intercept");
    close(result.activation_energy.value, 110_000.014_367_388_31, "E");
    close(
        result
            .volumetric_pre_factor
            .expect("a conductivity was supplied"),
        5.000_014_793_758_595e13,
        "P",
    );
}

#[test]
fn a_missing_conductivity_leaves_the_pre_factor_absent() {
    let points = [
        point("cube", 0.025, 591.294_053_941_965_1),
        point("cube", 0.05, 553.737_232_834_100_7),
        point("cube", 0.1, 520.846_261_829_137_8),
    ];
    let result = fit(&points, None);
    close(result.activation_energy.value, 109_999.996_196_856_24, "E");
    assert_eq!(result.volumetric_pre_factor, None);
    assert_eq!(result.effective_conductivity, None);
    assert!(!result.conductivity_provided);
    assert_eq!(result.messages, vec![MISSING_CONDUCTIVITY.to_string()]);
}

#[test]
fn two_points_fit_exactly_and_carry_the_recommendation() {
    let points = [
        point("cube", 0.025, 591.294_053_941_965_1),
        point("cube", 0.1, 520.846_261_829_137_8),
    ];
    let result = fit(&points, Some(0.09));
    close(result.point_count, 2.0, "the point count");
    close(result.slope, -13_229.958_589_203_492, "the slope");
    close(result.intercept, 43.441_228_890_499_836, "the intercept");
    close(
        result.r_squared.expect("two points fit exactly"),
        1.0,
        "r squared",
    );
    close(result.activation_energy.value, 109_999.996_127_620_46, "E");
    close(
        result
            .volumetric_pre_factor
            .expect("a conductivity was supplied"),
        5.000_000_167_274_655e13,
        "P",
    );
    assert_eq!(result.messages, vec![TWO_POINTS.to_string()]);
}

#[test]
fn a_single_point_is_refused() {
    let points = [point("cube", 0.05, 423.15)];
    assert!(basket_test_regression(&points, None).is_err());
}

#[test]
fn equal_critical_temperatures_are_refused() {
    let points = [point("cube", 0.05, 423.15), point("cube", 0.1, 423.15)];
    assert!(basket_test_regression(&points, None).is_err());
}
