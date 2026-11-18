//! `process.acoustic_induced_vibration_screening` - the Carucci-Mueller AIV screen.
//!
//! Spec: `specs/models/process/acoustic_induced_vibration_screening.toml`. The arithmetic
//! is `process/safety/vibration/AcousticInducedVibrationScreening.java` at the pin: the
//! sound power level, the design-line allowable and the margin's likelihood band.

use azoth_core::units::{Length, MassRate, MolarMass, Pressure, ThermodynamicTemperature};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::piping_fiv_gas::FivLikelihood;

/// The class's kg/s to lb/s factor.
const KG_PER_S_TO_LB_PER_S: f64 = 2.204_622_621_8;
/// The class's kelvin to degrees Rankine factor.
const KELVIN_TO_RANKINE: f64 = 1.8;
/// The Carucci-Mueller additive constant, dB.
const PWL_CONSTANT: f64 = 126.1;
/// The design-line intercept, dB.
const DESIGN_LINE_INTERCEPT: f64 = 173.6;
/// The design-line slope, dB per unit D/t.
const DESIGN_LINE_SLOPE: f64 = 0.125;
/// The grams in a kilogram, for the correlation's g/mol form.
const GRAMS_PER_KILOGRAM: f64 = 1000.0;

/// Result of `process.acoustic_induced_vibration_screening`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AcousticInducedVibrationResult {
    /// The sound power level, dB.
    pub pwl: f64,
    /// The design-line allowable sound power level, dB.
    pub allowable_pwl: f64,
    /// `pwl - allowable_pwl`, dB.
    pub margin: f64,
    /// The AVIFF likelihood band.
    pub likelihood: FivLikelihood,
    /// The pressure-drop ratio.
    pub pressure_drop_ratio: f64,
    /// The outside-diameter-to-wall-thickness ratio.
    pub d_overt: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The AIV band from the margin, with the class's exclusive upper bounds.
#[must_use]
pub fn band_for(margin: f64) -> FivLikelihood {
    if margin < -10.0 {
        FivLikelihood::Low
    } else if margin < 0.0 {
        FivLikelihood::Medium
    } else if margin < 10.0 {
        FivLikelihood::High
    } else {
        FivLikelihood::VeryHigh
    }
}

/// Refuse a value the arithmetic cannot carry to a published number.
fn require_finite(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

/// The sound power level downstream of a pressure-reducing device and its band.
///
/// # Errors
/// `InvalidInput` for a non-positive flow, upstream pressure, temperature, molar mass or
/// diameter, a downstream pressure outside `[0, upstream)`, or a non-finite input.
#[allow(clippy::too_many_arguments)] // The signature is the spec's declared input list.
pub fn acoustic_induced_vibration_screening(
    mass_flow: MassRate,
    upstream_pressure: Pressure,
    downstream_pressure: Pressure,
    temperature: ThermodynamicTemperature,
    molecular_weight: MolarMass,
    outside_diameter: Length,
    wall_thickness: Length,
) -> Result<AcousticInducedVibrationResult> {
    let spec = &model_gen::ACOUSTIC_INDUCED_VIBRATION_SCREENING_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "mass_flow" => Some(mass_flow.value),
            "upstream_pressure" => Some(upstream_pressure.value),
            "downstream_pressure" => Some(downstream_pressure.value),
            "temperature" => Some(temperature.value),
            "molecular_weight" => Some(molecular_weight.value),
            "outside_diameter" => Some(outside_diameter.value),
            "wall_thickness" => Some(wall_thickness.value),
            _ => None,
        },
        &mut warnings,
    )?;

    for (name, value) in [
        ("mass_flow", mass_flow.value),
        ("upstream_pressure", upstream_pressure.value),
        ("downstream_pressure", downstream_pressure.value),
        ("temperature", temperature.value),
        ("molecular_weight", molecular_weight.value),
        ("outside_diameter", outside_diameter.value),
        ("wall_thickness", wall_thickness.value),
    ] {
        require_finite(name, value)?;
    }
    if downstream_pressure.value >= upstream_pressure.value {
        return Err(AzothError::invalid_input(
            "downstream_pressure",
            "the downstream pressure must be strictly below the upstream pressure",
        ));
    }

    let pressure_drop_ratio =
        (upstream_pressure.value - downstream_pressure.value) / upstream_pressure.value;
    let flow_lb_per_s = mass_flow.value * KG_PER_S_TO_LB_PER_S;
    let temperature_rankine = temperature.value * KELVIN_TO_RANKINE;
    let d_overt = outside_diameter.value / wall_thickness.value;
    let molecular_weight_gmol = molecular_weight.value * GRAMS_PER_KILOGRAM;

    let argument = pressure_drop_ratio.powf(3.6)
        * flow_lb_per_s
        * flow_lb_per_s
        * (temperature_rankine / molecular_weight_gmol).powf(1.2);
    let pwl = 10.0 * argument.log10() + PWL_CONSTANT;
    let allowable_pwl = DESIGN_LINE_INTERCEPT - DESIGN_LINE_SLOPE * d_overt;
    let margin = pwl - allowable_pwl;

    Ok(AcousticInducedVibrationResult {
        pwl,
        allowable_pwl,
        margin,
        likelihood: band_for(margin),
        pressure_drop_ratio,
        d_overt,
        warnings,
    })
}
