//! `process.runaway_reaction` - the adiabatic runaway screen as a registered id.
//!
//! Spec: `specs/models/process/runaway_reaction.toml`. The arithmetic is
//! `process/safety/reaction/RunawayReactionAnalyzer.java` at the pin: the adiabatic rise,
//! the MTSR and margin, the time to maximum rate and the verdict.

use azoth_core::units::{
    AmountOfSubstance, Energy, Mass, MolarEnergy, SpecificHeatCapacity, TemperatureInterval,
    ThermodynamicTemperature, Time, joules, kelvin_intervals, kelvins, seconds,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's gas constant, J/(mol K).
const R_GAS: f64 = 8.314_462_618;
/// The class's default acceptable margin, K.
const DEFAULT_ACCEPTABLE_MARGIN_K: f64 = 50.0;
/// The class's high-severity rise, K.
const HIGH_SEVERITY_DT_K: f64 = 200.0;
/// The class's marginal-severity rise, K.
const MARGINAL_SEVERITY_DT_K: f64 = 50.0;

/// The runaway-reaction screening band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RunawayVerdict {
    /// No maximum-allowable temperature was stated.
    #[serde(rename = "no_rating")]
    NoRating,
    /// The margin is at or above the acceptable one.
    #[serde(rename = "acceptable_margin")]
    AcceptableMargin,
    /// The margin is positive but below the acceptable one.
    #[serde(rename = "marginal")]
    Marginal,
    /// The MTSR exceeds the MAT, or the adiabatic rise is severe.
    #[serde(rename = "runaway_risk")]
    RunawayRisk,
}

impl RunawayVerdict {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoRating => "no_rating",
            Self::AcceptableMargin => "acceptable_margin",
            Self::Marginal => "marginal",
            Self::RunawayRisk => "runaway_risk",
        }
    }
}

/// Result of `process.runaway_reaction`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunawayReactionResult {
    /// The adiabatic temperature rise.
    #[serde(serialize_with = "scalar")]
    pub adiabatic_temperature_rise: TemperatureInterval,
    /// The maximum temperature of the synthesis reaction.
    #[serde(serialize_with = "scalar")]
    pub mtsr: ThermodynamicTemperature,
    /// `max_allowable_temperature - mtsr`; absent without a MAT.
    #[serde(serialize_with = "optional_scalar")]
    pub temperature_margin: Option<TemperatureInterval>,
    /// Whether the MTSR is strictly above the MAT.
    pub margin_exceeded: bool,
    /// `|dH| n`.
    #[serde(serialize_with = "scalar")]
    pub total_heat_release: Energy,
    /// The time to maximum rate; absent unless the TMR pair is positive.
    #[serde(serialize_with = "optional_scalar")]
    pub time_to_max_rate: Option<Time>,
    /// Whether the gassy flag is set or the margin is exceeded.
    pub two_phase_relief_screening_required: bool,
    /// The class's verdict.
    pub verdict: RunawayVerdict,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
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

/// The adiabatic rise, MTSR, margin, time to maximum rate and runaway verdict.
///
/// # Errors
/// `InvalidInput` for a non-positive process temperature, limiting-reactant moles, reacting
/// mass or specific heat, a MAT at or below the process temperature, or a non-finite input.
#[allow(clippy::too_many_arguments)] // The signature is the spec's declared input list.
pub fn runaway_reaction(
    process_temperature: ThermodynamicTemperature,
    reaction_enthalpy: MolarEnergy,
    limiting_reactant_moles: AmountOfSubstance,
    total_mass: Mass,
    specific_heat: SpecificHeatCapacity,
    max_allowable_temperature: Option<ThermodynamicTemperature>,
    activation_energy: Option<MolarEnergy>,
    initial_heat_release_rate: Option<f64>,
    acceptable_margin: Option<TemperatureInterval>,
    gassy_system: Option<bool>,
) -> Result<RunawayReactionResult> {
    let spec = &model_gen::RUNAWAY_REACTION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "process_temperature" => Some(process_temperature.value),
            "reaction_enthalpy" => Some(reaction_enthalpy.value),
            "limiting_reactant_moles" => Some(limiting_reactant_moles.value),
            "total_mass" => Some(total_mass.value),
            "specific_heat" => Some(specific_heat.value),
            "max_allowable_temperature" => max_allowable_temperature.map(|value| value.value),
            "activation_energy" => activation_energy.map(|value| value.value),
            "initial_heat_release_rate" => initial_heat_release_rate,
            "acceptable_margin" => acceptable_margin.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    for (name, value) in [
        ("process_temperature", Some(process_temperature.value)),
        ("reaction_enthalpy", Some(reaction_enthalpy.value)),
        (
            "limiting_reactant_moles",
            Some(limiting_reactant_moles.value),
        ),
        ("total_mass", Some(total_mass.value)),
        ("specific_heat", Some(specific_heat.value)),
        (
            "max_allowable_temperature",
            max_allowable_temperature.map(|value| value.value),
        ),
        (
            "activation_energy",
            activation_energy.map(|value| value.value),
        ),
        ("initial_heat_release_rate", initial_heat_release_rate),
        (
            "acceptable_margin",
            acceptable_margin.map(|value| value.value),
        ),
    ] {
        if let Some(value) = value {
            require_finite(name, value)?;
        }
    }
    if let Some(material_limit) = max_allowable_temperature {
        if material_limit.value <= process_temperature.value {
            return Err(AzothError::invalid_input(
                "max_allowable_temperature",
                "the maximum-allowable temperature must exceed the process temperature",
            ));
        }
    }

    let heat_release = reaction_enthalpy.value.abs() * limiting_reactant_moles.value;
    let rise = heat_release / (total_mass.value * specific_heat.value);
    let mtsr = process_temperature.value + rise;

    let (temperature_margin, margin_exceeded) = match max_allowable_temperature {
        Some(material_limit) => {
            let margin = material_limit.value - mtsr;
            (Some(margin), mtsr > material_limit.value)
        }
        None => (None, false),
    };

    let time_to_max_rate = match (activation_energy, initial_heat_release_rate) {
        (Some(activation), Some(rate)) if activation.value > 0.0 && rate > 0.0 => Some(
            specific_heat.value * R_GAS * process_temperature.value.powi(2)
                / (rate * activation.value),
        ),
        _ => None,
    };

    let acceptable = acceptable_margin.map_or(DEFAULT_ACCEPTABLE_MARGIN_K, |value| value.value);
    let verdict = if let Some(margin) = temperature_margin {
        if margin_exceeded {
            RunawayVerdict::RunawayRisk
        } else if margin < acceptable {
            RunawayVerdict::Marginal
        } else {
            RunawayVerdict::AcceptableMargin
        }
    } else if rise >= HIGH_SEVERITY_DT_K {
        RunawayVerdict::RunawayRisk
    } else if rise >= MARGINAL_SEVERITY_DT_K {
        RunawayVerdict::Marginal
    } else {
        RunawayVerdict::NoRating
    };
    let two_phase = gassy_system.unwrap_or(false) || margin_exceeded;

    Ok(RunawayReactionResult {
        adiabatic_temperature_rise: kelvin_intervals(rise),
        mtsr: kelvins(mtsr),
        temperature_margin: temperature_margin.map(kelvin_intervals),
        margin_exceeded,
        total_heat_release: joules(heat_release),
        time_to_max_rate: time_to_max_rate.map(seconds),
        two_phase_relief_screening_required: two_phase,
        verdict,
        warnings,
    })
}
