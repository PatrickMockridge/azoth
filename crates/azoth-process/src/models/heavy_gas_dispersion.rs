//! `process.heavy_gas_dispersion` - the Britter-McQuaid dense-gas distance.
//!
//! Spec: `specs/models/process/heavy_gas_dispersion.toml`. The arithmetic is
//! `process/safety/dispersion/HeavyGasDispersion.java` at the pin: the signed reduced
//! gravity, the unfloored characteristic parameter, and the fit with its `0.01` floor.

use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;

/// Result of `process.heavy_gas_dispersion`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HeavyGasDispersionResult {
    /// `g0 = 9.81 (rho_s - rho_a) / rho_a`, signed; the value is in m/s2.
    pub reduced_gravity: f64,
    /// `alpha = (|g0|^2 q / U^5)^0.2`, unfloored.
    pub britter_parameter: f64,
    /// The downwind distance to the stated ratio; absent without a ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_to_concentration_ratio: Option<f64>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The class's signed reduced gravity.
fn reduced_gravity(source_density: f64, air_density: f64) -> f64 {
    9.81 * (source_density - air_density) / air_density
}

/// The class's characteristic parameter, over `|g0|` and unfloored.
fn britter_parameter(reduced: f64, volumetric_rate: f64, wind_speed: f64) -> f64 {
    (reduced.abs() * reduced.abs() * volumetric_rate / wind_speed.powi(5)).powf(0.2)
}

/// The class's fitted distance, with `alpha` floored at 0.01.
fn distance(volumetric_rate: f64, wind_speed: f64, alpha: f64, ratio: f64) -> f64 {
    let scale = (volumetric_rate / wind_speed).sqrt();
    let b = -0.25 * alpha.max(0.01).log10();
    let log_d = -0.85 * ratio.log10() + b;
    scale * 10.0f64.powf(log_d)
}

fn require_positive(name: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(AzothError::invalid_input(
            name,
            format!("a finite positive value is required, and {value} is not"),
        ));
    }
    Ok(())
}

/// The dense-gas distance to a concentration ratio.
///
/// # Errors
/// `InvalidInput` for a non-positive or non-finite rate, density or wind speed, or a ratio
/// outside `(0, 1)`.
pub fn heavy_gas_dispersion(
    volumetric_rate: f64,
    source_density: f64,
    air_density: f64,
    wind_speed: f64,
    concentration_ratio: Option<f64>,
) -> Result<HeavyGasDispersionResult> {
    let spec = &model_gen::HEAVY_GAS_DISPERSION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "volumetric_rate" => Some(volumetric_rate),
            "source_density" => Some(source_density),
            "air_density" => Some(air_density),
            "wind_speed" => Some(wind_speed),
            "concentration_ratio" => concentration_ratio,
            _ => None,
        },
        &mut warnings,
    )?;
    require_positive("volumetric_rate", volumetric_rate)?;
    require_positive("source_density", source_density)?;
    require_positive("air_density", air_density)?;
    require_positive("wind_speed", wind_speed)?;
    if let Some(ratio) = concentration_ratio {
        if !ratio.is_finite() || ratio <= 0.0 || ratio >= 1.0 {
            return Err(AzothError::invalid_input(
                "concentration_ratio",
                format!("a ratio in (0, 1) is required, and {ratio} is not"),
            ));
        }
    }

    let reduced = reduced_gravity(source_density, air_density);
    let alpha = britter_parameter(reduced, volumetric_rate, wind_speed);
    let downwind = concentration_ratio.map(|ratio| distance(volumetric_rate, wind_speed, alpha, ratio));
    Ok(HeavyGasDispersionResult {
        reduced_gravity: reduced,
        britter_parameter: alpha,
        distance_to_concentration_ratio: downwind,
        warnings,
    })
}
