//! `process.basket_test_regression` - the basket-test fit for E and P.
//!
//! Spec: `specs/models/process/basket_test_regression.toml`. The arithmetic is NeqSim's
//! `process/safety/selfheating/BasketTestRegression.java`'s `regress`: the least-squares
//! line through `ln(deltaCrit Tc^2 / r^2)` against `1/Tc`.

use azoth_core::units::{
    Length, MolarEnergy, ThermalConductivity, ThermodynamicTemperature, joules_per_mole,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::selfheating::{R_GAS, SelfHeatingGeometry};

/// The number of distinct sizes the class recommends.
const RECOMMENDED_POINT_COUNT: usize = 3;

/// One basket-test observation.
#[derive(Debug, Clone, PartialEq)]
pub struct BasketTestPoint {
    /// The basket shape, the spec's spelling; it enters only through its critical value.
    pub geometry: String,
    /// The basket's characteristic half-dimension.
    pub characteristic_dimension: Length,
    /// The lowest oven temperature at which the sample ran away.
    pub critical_temperature: ThermodynamicTemperature,
}

/// Result of `process.basket_test_regression`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BasketTestRegressionResult {
    /// The number of observations fitted.
    pub point_count: f64,
    /// The slope of the line, in kelvin.
    pub slope: f64,
    /// The line's intercept.
    pub intercept: f64,
    /// The coefficient of determination; absent when the ordinates do not vary.
    pub r_squared: Option<f64>,
    /// `-slope R`.
    #[serde(serialize_with = "scalar")]
    pub activation_energy: MolarEnergy,
    /// The volumetric heat-release pre-factor, in W/m3; absent without a conductivity.
    pub volumetric_pre_factor: Option<f64>,
    /// The conductivity the fit used, echoed.
    #[serde(serialize_with = "optional_scalar")]
    pub effective_conductivity: Option<ThermalConductivity>,
    /// Whether a conductivity was supplied.
    pub conductivity_provided: bool,
    /// The class's warning strings, in order.
    pub messages: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The least-squares basket-test fit.
///
/// # Errors
/// `InvalidInput` for fewer than two points or for points that all share a critical
/// temperature, and for a non-positive effective conductivity.
pub fn basket_test_regression(
    points: &[BasketTestPoint],
    effective_conductivity: Option<ThermalConductivity>,
) -> Result<BasketTestRegressionResult> {
    let spec = &model_gen::BASKET_TEST_REGRESSION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "effective_conductivity" => effective_conductivity.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    if points.len() < 2 {
        return Err(AzothError::invalid_input(
            "points",
            "At least two basket-test points are required to fit an activation energy",
        ));
    }
    let n = points.len();
    let mut messages = Vec::new();
    if n < RECOMMENDED_POINT_COUNT {
        messages.push(format!(
            "Only {n} basket sizes supplied; at least {RECOMMENDED_POINT_COUNT} are recommended \
             for a defensible fit and a meaningful coefficient of determination"
        ));
    }

    let mut x = Vec::with_capacity(n);
    let mut y = Vec::with_capacity(n);
    for point in points {
        let temperature = point.critical_temperature.value;
        let dimension = point.characteristic_dimension.value;
        if dimension.is_nan() || dimension <= 0.0 {
            return Err(AzothError::invalid_input(
                "points",
                "a basket's characteristic dimension must be positive",
            ));
        }
        if temperature.is_nan() || temperature <= 0.0 {
            return Err(AzothError::invalid_input(
                "points",
                "a basket's critical temperature must be positive",
            ));
        }
        let geometry = SelfHeatingGeometry::parse(&point.geometry)?;
        x.push(1.0 / temperature);
        y.push((geometry.delta_crit() * temperature * temperature / (dimension * dimension)).ln());
    }
    let mean_x = x.iter().sum::<f64>() / n as f64;
    let mean_y = y.iter().sum::<f64>() / n as f64;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    for index in 0..n {
        sxx += (x[index] - mean_x) * (x[index] - mean_x);
        sxy += (x[index] - mean_x) * (y[index] - mean_y);
    }
    if sxx.is_nan() || sxx <= 0.0 {
        return Err(AzothError::invalid_input(
            "points",
            "All basket-test points share the same critical temperature; cannot fit a slope",
        ));
    }
    let slope = sxy / sxx;
    let intercept = mean_y - slope * mean_x;

    let mut ss_tot = 0.0;
    let mut ss_res = 0.0;
    for index in 0..n {
        let fitted = intercept + slope * x[index];
        ss_tot += (y[index] - mean_y) * (y[index] - mean_y);
        ss_res += (y[index] - fitted) * (y[index] - fitted);
    }
    let r_squared = if ss_tot > 0.0 {
        Some(1.0 - ss_res / ss_tot)
    } else {
        None
    };

    let activation_energy = -slope * R_GAS;
    if activation_energy.is_nan() || activation_energy <= 0.0 {
        messages.push(
            "Fitted activation energy is not positive; the critical temperature should fall as \
             basket size increases, so check the ordering and units of the input data"
                .to_string(),
        );
    }
    let conductivity_provided = effective_conductivity.is_some();
    let volumetric_pre_factor = match effective_conductivity {
        Some(conductivity) if activation_energy > 0.0 => {
            Some(intercept.exp() * conductivity.value * R_GAS / activation_energy)
        }
        Some(_) => None,
        None => {
            messages.push(
                "Effective thermal conductivity not supplied; the volumetric heat-release \
                 pre-factor could not be separated from the regression intercept"
                    .to_string(),
            );
            None
        }
    };

    Ok(BasketTestRegressionResult {
        point_count: n as f64,
        slope,
        intercept,
        r_squared,
        activation_energy: joules_per_mole(activation_energy),
        volumetric_pre_factor,
        effective_conductivity,
        conductivity_provided,
        messages,
        warnings,
    })
}
