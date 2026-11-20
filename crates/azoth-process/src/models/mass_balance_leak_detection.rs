//! `process.mass_balance_leak_detection` - the mass-balance leak sensitivity.
//!
//! Spec: `specs/models/process/mass_balance_leak_detection.toml`. The arithmetic is
//! NeqSim's `process/safety/leakdetection/MassBalanceLeakDetector.java`, through
//! `calculateSensitivity`.

use azoth_core::units::{
    MassDensity, MassRate, Pressure, ThermodynamicTemperature, Time, Volume,
    kilograms_per_cubic_meter, kilograms_per_second, seconds,
};
use azoth_core::{AzothError, Result, Warning, WarningCode, apply_checks};
use azoth_eos::Cubic;
use azoth_eos::databank;
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::fluid_state;

/// Result of `process.mass_balance_leak_detection`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MassBalanceLeakDetectionResult {
    /// The echoed inlet stream name.
    pub inlet_stream_name: Option<String>,
    /// The echoed outlet stream name.
    pub outlet_stream_name: Option<String>,
    /// The measured inlet flow.
    #[serde(serialize_with = "scalar")]
    pub inlet_flow: MassRate,
    /// The measured outlet flow.
    #[serde(serialize_with = "scalar")]
    pub outlet_flow: MassRate,
    /// `inlet_flow - outlet_flow`.
    #[serde(serialize_with = "scalar")]
    pub measured_imbalance: MassRate,
    /// The meters' combined uncertainty.
    #[serde(serialize_with = "scalar")]
    pub flow_uncertainty: MassRate,
    /// The linepack's uncertainty as an equivalent leak rate.
    #[serde(serialize_with = "scalar")]
    pub linepack_uncertainty: MassRate,
    /// `sqrt(flow_uncertainty^2 + linepack_uncertainty^2)`.
    #[serde(serialize_with = "scalar")]
    pub combined_uncertainty: MassRate,
    /// `|measured_imbalance| + combined_uncertainty`.
    #[serde(serialize_with = "scalar")]
    pub minimum_detectable_leak_rate: MassRate,
    /// The rate over the larger absolute flow; absent when that flow is zero.
    pub minimum_detectable_leak_fraction: Option<f64>,
    /// The bulk density the linepack term used; absent when no linepack is stated.
    #[serde(serialize_with = "optional_scalar")]
    pub density: Option<MassDensity>,
    /// The clamped averaging window.
    #[serde(serialize_with = "scalar")]
    pub detection_window: Time,
    /// The clamped confidence multiplier.
    pub confidence_multiplier: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The mass-balance leak sensitivity.
///
/// # Errors
/// `InvalidInput` for a non-finite or negative flow, a non-positive pressure, temperature or
/// supplied density, or an unknown equation of state.
#[allow(clippy::too_many_arguments)]
pub fn mass_balance_leak_detection(
    components: &[String],
    z: &[f64],
    inlet_pressure: Pressure,
    inlet_temperature: ThermodynamicTemperature,
    inlet_flow: MassRate,
    outlet_flow: MassRate,
    eos: Option<&str>,
    inlet_density: Option<MassDensity>,
    linepack_volume: Option<Volume>,
    flow_measurement_uncertainty_fraction: Option<f64>,
    pressure_uncertainty: Option<Pressure>,
    temperature_uncertainty: Option<f64>,
    detection_window: Option<Time>,
    confidence_multiplier: Option<f64>,
    inlet_stream_name: Option<&str>,
    outlet_stream_name: Option<&str>,
) -> Result<MassBalanceLeakDetectionResult> {
    let spec = &model_gen::MASS_BALANCE_LEAK_DETECTION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "inlet_pressure" => Some(inlet_pressure.value),
            "inlet_temperature" => Some(inlet_temperature.value),
            "inlet_density" => inlet_density.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    let pressure = inlet_pressure.value;
    let temperature = inlet_temperature.value;
    let inlet = inlet_flow.value;
    let outlet = outlet_flow.value;
    for (name, value) in [
        ("inlet_pressure", pressure),
        ("inlet_temperature", temperature),
        ("inlet_flow", inlet),
        ("outlet_flow", outlet),
    ] {
        if !value.is_finite() {
            return Err(AzothError::invalid_input(
                name,
                "a non-finite value is not a number this calculation publishes",
            ));
        }
    }
    if pressure <= 0.0 || temperature <= 0.0 {
        return Err(AzothError::invalid_input(
            "inlet_pressure",
            "the inlet state must be a positive absolute pressure and temperature",
        ));
    }

    let volume = linepack_volume.map_or(0.0, |value| value.value.max(0.0));
    let fraction = flow_measurement_uncertainty_fraction
        .unwrap_or(0.01)
        .max(0.0);
    let delta_pressure = pressure_uncertainty.map_or(0.0, |value| value.value.max(0.0));
    let delta_temperature = temperature_uncertainty.unwrap_or(0.0).max(0.0);
    let window = detection_window.map_or(60.0, |value| value.value.max(1.0));
    let confidence = confidence_multiplier.unwrap_or(3.0).max(0.0);

    let density = match inlet_density {
        Some(value) => {
            if value.value <= 0.0 {
                return Err(AzothError::invalid_input(
                    "inlet_density",
                    "a density is positive",
                ));
            }
            Some(value.value)
        }
        None if volume > 0.0 => {
            let cubic = match eos.unwrap_or("pr") {
                "pr" => Cubic::Pr,
                "srk" => Cubic::Srk,
                other => {
                    return Err(AzothError::invalid_input(
                        "eos",
                        format!("`{other}` is not `pr` or `srk`"),
                    ));
                }
            };
            let names: Vec<&str> = components.iter().map(String::as_str).collect();
            let (mixture, ideal_gas) = databank::mixture_of(&names, cubic, None)?;
            match fluid_state::flash(&mixture, &ideal_gas, temperature, pressure, z, false) {
                Ok(state) => Some(state.bulk_density),
                Err(_) => {
                    warnings.push(Warning::for_field(
                        WarningCode::SolverNotConverged,
                        "inlet_density",
                        "the inlet density flash failed; the linepack uncertainty is zero",
                    ));
                    None
                }
            }
        }
        None => None,
    };

    let measured_imbalance = inlet - outlet;
    let flow_uncertainty =
        confidence * ((inlet * fraction).powi(2) + (outlet * fraction).powi(2)).sqrt();
    let linepack_uncertainty = match density {
        Some(rho) if volume > 0.0 => {
            let relative = ((delta_pressure / pressure).powi(2)
                + (delta_temperature / temperature).powi(2))
            .sqrt();
            confidence * rho * volume * relative / window
        }
        _ => 0.0,
    };
    let combined_uncertainty =
        (flow_uncertainty * flow_uncertainty + linepack_uncertainty * linepack_uncertainty).sqrt();
    let minimum_detectable_leak_rate = measured_imbalance.abs() + combined_uncertainty;
    let reference = inlet.abs().max(outlet.abs());
    let minimum_detectable_leak_fraction = if reference > 0.0 {
        Some(minimum_detectable_leak_rate / reference)
    } else {
        None
    };

    Ok(MassBalanceLeakDetectionResult {
        inlet_stream_name: inlet_stream_name.map(str::to_string),
        outlet_stream_name: outlet_stream_name.map(str::to_string),
        inlet_flow: kilograms_per_second(inlet),
        outlet_flow: kilograms_per_second(outlet),
        measured_imbalance: kilograms_per_second(measured_imbalance),
        flow_uncertainty: kilograms_per_second(flow_uncertainty),
        linepack_uncertainty: kilograms_per_second(linepack_uncertainty),
        combined_uncertainty: kilograms_per_second(combined_uncertainty),
        minimum_detectable_leak_rate: kilograms_per_second(minimum_detectable_leak_rate),
        minimum_detectable_leak_fraction,
        density: density.map(kilograms_per_cubic_meter),
        detection_window: seconds(window),
        confidence_multiplier: confidence,
        warnings,
    })
}
