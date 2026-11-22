//! `process.safety_envelope` - the operational safety envelope.
//!
//! Spec: `specs/models/process/safety_envelope.toml`. The arithmetic is NeqSim's
//! `process/safety/envelope/SafetyEnvelopeCalculator.java` and the queries are
//! `SafetyEnvelope.java`'s.

use azoth_core::units::{Pressure, ThermodynamicTemperature, kelvins, pascals};
use azoth_core::{AzothError, Result, Warning, WarningCode, apply_checks};
use azoth_eos::hydrate::{HydrateModel, hydrate_mixture_of};
use azoth_eos::hydrate_formation_temperature::hydrate_formation_temperature;
use azoth_eos::mixture::Mixture;
use azoth_eos::molar_enthalpy_entropy::IdealGasModel;
use azoth_eos::{Cubic, databank};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::fluid_state;
use crate::models::wax_appearance::wat_temperature;

/// A reference pressure ratio's atmospheric pressure, bar.
const ATMOSPHERIC_BAR: f64 = 1.01325;
/// The CO2 triple point, bar and K.
const CO2_TRIPLE_BAR: f64 = 5.18;
/// The CO2 triple-point temperature, K.
const CO2_TRIPLE_K: f64 = 216.55;

/// The envelope's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EnvelopeKind {
    /// Hydrate formation.
    #[serde(rename = "hydrate")]
    Hydrate,
    /// Wax appearance.
    #[serde(rename = "wax")]
    Wax,
    /// CO2 freezing.
    #[serde(rename = "co2_freezing")]
    Co2Freezing,
    /// Minimum design metal temperature.
    #[serde(rename = "mdmt")]
    Mdmt,
    /// The phase envelope.
    #[serde(rename = "phase_envelope")]
    PhaseEnvelope,
}

impl EnvelopeKind {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hydrate => "hydrate",
            Self::Wax => "wax",
            Self::Co2Freezing => "co2_freezing",
            Self::Mdmt => "mdmt",
            Self::PhaseEnvelope => "phase_envelope",
        }
    }

    /// The class's envelope name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Hydrate => "Hydrate Formation",
            Self::Wax => "Wax Appearance",
            Self::Co2Freezing => "CO2 Freezing",
            Self::Mdmt => "MDMT",
            Self::PhaseEnvelope => "Phase Envelope",
        }
    }
}

/// One supplied phase-curve point.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeTracePoint {
    /// The point's pressure.
    pub pressure: Pressure,
    /// The point's temperature.
    pub temperature: ThermodynamicTemperature,
}

/// One curve point.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnvelopePoint {
    /// The arm the point belongs to.
    pub envelope: EnvelopeKind,
    /// The point's index within its arm.
    pub index: usize,
    /// The point's pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// The limit temperature; absent when the arm's flash failed.
    #[serde(serialize_with = "optional_scalar")]
    pub temperature: Option<ThermodynamicTemperature>,
    /// The point's margin.
    pub margin: f64,
    /// The limit less the margin.
    #[serde(serialize_with = "optional_scalar")]
    pub safe_temperature: Option<ThermodynamicTemperature>,
}

/// One arm's summary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnvelopeSummary {
    /// The arm.
    pub envelope: EnvelopeKind,
    /// The class's envelope name.
    pub name: String,
    /// The points the arm carries.
    pub point_count: usize,
    /// The feed's water mole fraction.
    pub reference_water_content: f64,
    /// The class's reference wax content.
    pub reference_wax_content: f64,
}

/// One arm's query at the operating point.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnvelopeQuery {
    /// The arm.
    pub envelope: EnvelopeKind,
    /// The interpolated limit at the operating pressure.
    #[serde(serialize_with = "optional_scalar")]
    pub temperature: Option<ThermodynamicTemperature>,
    /// The limit less the interpolated margin.
    #[serde(serialize_with = "optional_scalar")]
    pub safe_temperature: Option<ThermodynamicTemperature>,
    /// `operating_temperature - limit`.
    #[serde(serialize_with = "optional_scalar")]
    pub margin: Option<ThermodynamicTemperature>,
    /// The class's strict safe test.
    pub safe: bool,
}

/// Result of `process.safety_envelope`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SafetyEnvelopeResult {
    /// One summary row per arm.
    pub envelopes: Vec<EnvelopeSummary>,
    /// Every arm's points.
    pub points: Vec<EnvelopePoint>,
    /// The queries, when an operating point is stated.
    pub queries: Vec<EnvelopeQuery>,
    /// Whether every query is safe.
    pub all_safe: Option<bool>,
    /// The arm with the smallest finite margin.
    pub most_limiting_envelope: Option<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One arm's arrays, in the class's bar-based form.
struct Arm {
    envelope: EnvelopeKind,
    pressure_bar: Vec<f64>,
    temperature: Vec<Option<f64>>,
    margin: Vec<f64>,
}

/// The operational safety envelope.
///
/// # Errors
/// `InvalidInput` for a non-positive pressure, temperature or design temperature, fewer than
/// two points, `max_pressure` below `min_pressure`, one operating coordinate without the
/// other, or a trace point that is not physical.
#[allow(clippy::too_many_arguments)]
pub fn safety_envelope(
    components: &[String],
    z: &[f64],
    min_pressure: Pressure,
    max_pressure: Pressure,
    num_points: usize,
    eos: Option<&str>,
    hydrate_margin: Option<f64>,
    wax_margin: Option<f64>,
    co2_margin: Option<f64>,
    mdmt_margin: Option<f64>,
    mdmt_design_temperature: Option<ThermodynamicTemperature>,
    operating_pressure: Option<Pressure>,
    operating_temperature: Option<ThermodynamicTemperature>,
    phase_trace: Option<&[EnvelopeTracePoint]>,
) -> Result<SafetyEnvelopeResult> {
    if z.len() != components.len() || z.iter().any(|value| !value.is_finite()) {
        return Err(AzothError::invalid_input(
            "z",
            "the composition needs one finite mole fraction per component",
        ));
    }
    let spec = &model_gen::SAFETY_ENVELOPE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "min_pressure" => Some(min_pressure.value),
            "max_pressure" => Some(max_pressure.value),
            "num_points" => Some(num_points as f64),
            "mdmt_design_temperature" => mdmt_design_temperature.map(|value| value.value),
            "operating_pressure" => operating_pressure.map(|value| value.value),
            "operating_temperature" => operating_temperature.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;
    if num_points < 2 {
        return Err(AzothError::invalid_input(
            "num_points",
            "the class divides by num_points - 1, so at least two points are needed",
        ));
    }
    if max_pressure.value < min_pressure.value {
        return Err(AzothError::invalid_input(
            "max_pressure",
            "the arm's last pressure is below its first",
        ));
    }
    if operating_pressure.is_some() != operating_temperature.is_some() {
        return Err(AzothError::invalid_input(
            "operating_pressure",
            "both operating coordinates are stated, or neither",
        ));
    }
    let design_temperature = mdmt_design_temperature.map_or(300.0, |value| value.value);
    if !(design_temperature > 0.0 && design_temperature.is_finite()) {
        return Err(AzothError::invalid_input(
            "mdmt_design_temperature",
            "an absolute temperature is positive",
        ));
    }
    let hydrate_margin = hydrate_margin.unwrap_or(5.0);
    let wax_margin = wax_margin.unwrap_or(5.0);
    let co2_margin = co2_margin.unwrap_or(3.0);
    let mdmt_margin = mdmt_margin.unwrap_or(10.0);

    let cubic = match eos.unwrap_or("srk") {
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
    let water_content = mixture
        .index_of("water")
        .and_then(|index| z.get(index).copied())
        .unwrap_or(0.0);

    let pressure_bar: Vec<f64> = (0..num_points)
        .map(|index| {
            let fraction = index as f64 / (num_points - 1) as f64;
            (min_pressure.value + fraction * (max_pressure.value - min_pressure.value)) / 1.0e5
        })
        .collect();

    let mut arms: Vec<Arm> = Vec::new();
    match hydrate_mixture_of(&names, cubic, None, HydrateModel::Pvtsim) {
        Ok((hydrate_mixture, _)) => arms.push(hydrate_arm(
            &hydrate_mixture,
            z,
            &pressure_bar,
            hydrate_margin,
            &mut warnings,
        )),
        Err(_) => {
            for _ in 0..num_points {
                warnings.push(Warning::for_field(
                    WarningCode::SolverNotConverged,
                    "hydrate",
                    "the hydrate flash failed at this pressure; the point is absent",
                ));
            }
            arms.push(Arm {
                envelope: EnvelopeKind::Hydrate,
                pressure_bar: pressure_bar.clone(),
                temperature: vec![None; num_points],
                margin: vec![hydrate_margin; num_points],
            });
        }
    }
    arms.push(wax_arm(
        &mixture,
        z,
        &pressure_bar,
        wax_margin,
        &mut warnings,
    ));
    arms.push(co2_arm(&pressure_bar, co2_margin));
    arms.push(mdmt_arm(
        &mixture,
        &ideal_gas,
        z,
        &pressure_bar,
        design_temperature,
        mdmt_margin,
        &mut warnings,
    ));
    if let Some(trace) = phase_trace {
        let mut arm = Arm {
            envelope: EnvelopeKind::PhaseEnvelope,
            pressure_bar: Vec::with_capacity(trace.len()),
            temperature: Vec::with_capacity(trace.len()),
            margin: Vec::with_capacity(trace.len()),
        };
        for point in trace {
            if point.pressure.value.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
                || point.temperature.value.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
                || !point.pressure.value.is_finite()
                || !point.temperature.value.is_finite()
            {
                return Err(AzothError::invalid_input(
                    "phase_trace",
                    "a trace point needs a positive finite pressure and temperature",
                ));
            }
            arm.pressure_bar.push(point.pressure.value / 1.0e5);
            arm.temperature.push(Some(point.temperature.value));
            arm.margin.push(0.0);
        }
        arms.push(arm);
    }

    let mut points = Vec::new();
    let mut summaries = Vec::new();
    for arm in &arms {
        summaries.push(EnvelopeSummary {
            envelope: arm.envelope,
            name: arm.envelope.name().to_string(),
            point_count: arm.pressure_bar.len(),
            reference_water_content: if arm.envelope == EnvelopeKind::Hydrate {
                water_content
            } else {
                0.0
            },
            reference_wax_content: 0.0,
        });
        for index in 0..arm.pressure_bar.len() {
            let temperature = arm.temperature[index];
            points.push(EnvelopePoint {
                envelope: arm.envelope,
                index,
                pressure: pascals(arm.pressure_bar[index] * 1.0e5),
                temperature: temperature.map(kelvins),
                margin: arm.margin[index],
                safe_temperature: temperature.map(|value| kelvins(value - arm.margin[index])),
            });
        }
    }

    let (queries, all_safe, most_limiting_envelope) =
        match (operating_pressure, operating_temperature) {
            (Some(pressure), Some(temperature)) => {
                let pressure_bar = pressure.value / 1.0e5;
                let mut queries = Vec::with_capacity(arms.len());
                for arm in &arms {
                    queries.push(query_arm(arm, pressure_bar, temperature.value));
                }
                let all_safe = queries.iter().all(|query| query.safe);
                let mut most_limiting: Option<(String, f64)> = None;
                for query in &queries {
                    if let Some(margin) = query.margin {
                        let value = margin.value;
                        if value.is_finite() {
                            match &most_limiting {
                                Some((_, smallest)) if value >= *smallest => {}
                                _ => {
                                    most_limiting = Some((query.envelope.name().to_string(), value))
                                }
                            }
                        }
                    }
                }
                (queries, Some(all_safe), most_limiting.map(|(name, _)| name))
            }
            _ => (Vec::new(), None, None),
        };

    Ok(SafetyEnvelopeResult {
        envelopes: summaries,
        points,
        queries,
        all_safe,
        most_limiting_envelope,
        warnings,
    })
}

fn hydrate_arm(
    mixture: &Mixture,
    z: &[f64],
    pressure_bar: &[f64],
    margin: f64,
    warnings: &mut Vec<Warning>,
) -> Arm {
    let mut temperatures = Vec::with_capacity(pressure_bar.len());
    let mut margins = Vec::with_capacity(pressure_bar.len());
    for pressure in pressure_bar {
        match hydrate_formation_temperature(mixture, pascals(pressure * 1.0e5), z) {
            Ok(result) => temperatures.push(Some(result.temperature.value)),
            Err(_) => {
                temperatures.push(None);
                warnings.push(Warning::for_field(
                    WarningCode::SolverNotConverged,
                    "hydrate",
                    "the hydrate flash failed at this pressure; the point is absent",
                ));
            }
        }
        margins.push(margin);
    }
    Arm {
        envelope: EnvelopeKind::Hydrate,
        pressure_bar: pressure_bar.to_vec(),
        temperature: temperatures,
        margin: margins,
    }
}

fn wax_arm(
    mixture: &Mixture,
    z: &[f64],
    pressure_bar: &[f64],
    margin: f64,
    warnings: &mut Vec<Warning>,
) -> Arm {
    let mut temperatures = Vec::with_capacity(pressure_bar.len());
    let mut margins = Vec::with_capacity(pressure_bar.len());
    for pressure in pressure_bar {
        match wat_temperature(mixture, 320.0, pressure * 1.0e5, z) {
            Ok(temperature) => temperatures.push(Some(temperature)),
            Err(_) => {
                temperatures.push(None);
                warnings.push(Warning::for_field(
                    WarningCode::SolverNotConverged,
                    "wax",
                    "the wax appearance search failed at this pressure; the point is absent",
                ));
            }
        }
        margins.push(margin);
    }
    Arm {
        envelope: EnvelopeKind::Wax,
        pressure_bar: pressure_bar.to_vec(),
        temperature: temperatures,
        margin: margins,
    }
}

fn co2_arm(pressure_bar: &[f64], margin: f64) -> Arm {
    let temperatures: Vec<Option<f64>> = pressure_bar
        .iter()
        .map(|pressure| {
            let temperature = if *pressure < CO2_TRIPLE_BAR {
                CO2_TRIPLE_K * (pressure / CO2_TRIPLE_BAR).powf(0.1)
            } else {
                CO2_TRIPLE_K + 0.004 * (pressure - CO2_TRIPLE_BAR)
            };
            Some(temperature)
        })
        .collect();
    Arm {
        envelope: EnvelopeKind::Co2Freezing,
        pressure_bar: pressure_bar.to_vec(),
        temperature: temperatures,
        margin: vec![margin; pressure_bar.len()],
    }
}

fn mdmt_arm(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    z: &[f64],
    pressure_bar: &[f64],
    design_temperature: f64,
    margin: f64,
    warnings: &mut Vec<Warning>,
) -> Arm {
    let mut temperatures = Vec::with_capacity(pressure_bar.len());
    for pressure in pressure_bar {
        let temperature = match fluid_state::flash(
            mixture,
            ideal_gas,
            design_temperature,
            pressure * 1.0e5,
            z,
            false,
        ) {
            Ok(state) => {
                let mut gamma = state.gamma;
                if !gamma.is_finite() || gamma <= 1.0 || gamma > 2.0 {
                    gamma = 1.3;
                }
                if *pressure <= ATMOSPHERIC_BAR {
                    design_temperature
                } else {
                    design_temperature * (ATMOSPHERIC_BAR / pressure).powf((gamma - 1.0) / gamma)
                }
            }
            Err(_) => {
                warnings.push(Warning::for_field(
                    WarningCode::SolverNotConverged,
                    "mdmt",
                    "the MDMT flash failed at this pressure; the class's designT - 50 K is used",
                ));
                design_temperature - 50.0
            }
        };
        temperatures.push(Some(temperature));
    }
    Arm {
        envelope: EnvelopeKind::Mdmt,
        pressure_bar: pressure_bar.to_vec(),
        temperature: temperatures,
        margin: vec![margin; pressure_bar.len()],
    }
}

fn query_arm(arm: &Arm, pressure_bar: f64, operating_temperature: f64) -> EnvelopeQuery {
    let temperature = temperature_at(arm, pressure_bar);
    let safe_temperature = temperature.map(|value| value - margin_at(arm, pressure_bar));
    let safe = if arm.envelope == EnvelopeKind::PhaseEnvelope {
        true
    } else {
        match safe_temperature {
            Some(value) => operating_temperature > value,
            None => true,
        }
    };
    EnvelopeQuery {
        envelope: arm.envelope,
        temperature: temperature.map(kelvins),
        safe_temperature: safe_temperature.map(kelvins),
        margin: temperature.map(|value| kelvins(operating_temperature - value)),
        safe,
    }
}

fn temperature_at(arm: &Arm, pressure_bar: f64) -> Option<f64> {
    for index in 0..arm.pressure_bar.len().saturating_sub(1) {
        let left = arm.pressure_bar[index];
        let right = arm.pressure_bar[index + 1];
        if !((left <= pressure_bar && right >= pressure_bar)
            || (left >= pressure_bar && right <= pressure_bar))
        {
            continue;
        }
        let (Some(left_temperature), Some(right_temperature)) =
            (arm.temperature[index], arm.temperature[index + 1])
        else {
            return None;
        };
        let delta = right - left;
        if delta.abs() < 1.0e-10 {
            return Some(left_temperature);
        }
        let fraction = (pressure_bar - left) / delta;
        return Some(left_temperature + fraction * (right_temperature - left_temperature));
    }
    None
}

fn margin_at(arm: &Arm, pressure_bar: f64) -> f64 {
    for index in 0..arm.pressure_bar.len().saturating_sub(1) {
        let left = arm.pressure_bar[index];
        let right = arm.pressure_bar[index + 1];
        if !((left <= pressure_bar && right >= pressure_bar)
            || (left >= pressure_bar && right <= pressure_bar))
        {
            continue;
        }
        let delta = right - left;
        if delta.abs() < 1.0e-10 {
            return arm.margin[index];
        }
        let fraction = (pressure_bar - left) / delta;
        return arm.margin[index] + fraction * (arm.margin[index + 1] - arm.margin[index]);
    }
    f64::NAN
}
