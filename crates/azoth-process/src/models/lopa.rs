//! `process.lopa` - the layer-of-protection analysis result arithmetic.
//!
//! Spec: `specs/models/process/lopa.toml`. The arithmetic is
//! `process/safety/risk/sis/LOPAResult.java` at the pin: the target status and gap, the
//! RRFs, the required additional SIL, the layer rows and the STS0131 bands.

use azoth_core::units::Pressure;
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::safety_instrumented_function::required_sil;

/// The STS0131 pressure band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Sts0131PressureCategory {
    /// The event pressure is at or below the design pressure.
    #[serde(rename = "below_or_at_design_pressure")]
    BelowOrAtDesignPressure,
    /// The event pressure is above the design pressure and at or below the test pressure.
    #[serde(rename = "above_design_to_test_pressure")]
    AboveDesignToTestPressure,
    /// The event pressure is above the test pressure and at or below twice the design.
    #[serde(rename = "above_test_to_two_times_design_pressure")]
    AboveTestToTwoTimesDesignPressure,
    /// The event pressure is above twice the design pressure.
    #[serde(rename = "above_two_times_design_pressure")]
    AboveTwoTimesDesignPressure,
}

impl Sts0131PressureCategory {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BelowOrAtDesignPressure => "below_or_at_design_pressure",
            Self::AboveDesignToTestPressure => "above_design_to_test_pressure",
            Self::AboveTestToTwoTimesDesignPressure => "above_test_to_two_times_design_pressure",
            Self::AboveTwoTimesDesignPressure => "above_two_times_design_pressure",
        }
    }
}

/// One protection layer, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct LopaProtectionLayer {
    /// The layer's name.
    pub name: String,
    /// The layer's probability of failure on demand.
    pub pfd: f64,
    /// The frequency the caller records before the layer.
    pub frequency_before_per_year: f64,
    /// The frequency the caller records after the layer.
    pub frequency_after_per_year: f64,
}

/// One layer's reported contribution.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LopaLayerContribution {
    /// The layer's name.
    pub name: String,
    /// The layer's PFD.
    pub pfd: f64,
    /// The recorded frequency before the layer.
    pub frequency_before_per_year: f64,
    /// The recorded frequency after the layer.
    pub frequency_after_per_year: f64,
    /// `before - after`.
    pub contribution_per_year: f64,
    /// `1 / PFD`; absent at a zero PFD.
    pub rrf: Option<f64>,
}

/// Result of `process.lopa`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LopaResult {
    /// Whether the mitigated frequency is at or below the target.
    pub target_met: bool,
    /// `mitigated - target`.
    pub gap_to_target: f64,
    /// `initiating / mitigated`; zero when the mitigated frequency is zero.
    pub total_rrf: f64,
    /// `mitigated / target` when not met and the target is positive; absent otherwise.
    pub required_additional_rrf: Option<f64>,
    /// The SIL the required RRF asks for, or zero when the target is met.
    pub required_additional_sil: f64,
    /// One row per input layer.
    pub layer_contributions: Vec<LopaLayerContribution>,
    /// The STS0131 pressure band, absent without the pressure triple.
    pub sts0131_pressure_category: Option<Sts0131PressureCategory>,
    /// The band's target frequency, absent with the category.
    pub sts0131_target_frequency_per_year: Option<f64>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn require_finite(name: &str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name.to_string(),
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

fn sts0131_category(event: f64, design: f64, test: f64) -> (Sts0131PressureCategory, f64) {
    if event > 2.0 * design {
        (Sts0131PressureCategory::AboveTwoTimesDesignPressure, 1.0e-5)
    } else if event > test {
        (
            Sts0131PressureCategory::AboveTestToTwoTimesDesignPressure,
            1.0e-4,
        )
    } else if event > design {
        (Sts0131PressureCategory::AboveDesignToTestPressure, 1.0e-3)
    } else {
        (Sts0131PressureCategory::BelowOrAtDesignPressure, 1.0e-2)
    }
}

/// The target status and gap, the RRFs, the required SIL, the layer rows and the STS0131
/// band.
///
/// # Errors
/// `InvalidInput` for a negative or non-finite frequency, a PFD outside `[0, 1]`, a
/// negative layer frequency, a partial pressure triple, a non-positive pressure, or a
/// test pressure below the design pressure.
#[allow(clippy::too_many_arguments)]
pub fn lopa(
    initiating_event_frequency_per_year: f64,
    target_frequency_per_year: f64,
    mitigated_frequency_per_year: f64,
    layers: &[LopaProtectionLayer],
    event_pressure: Option<Pressure>,
    design_pressure: Option<Pressure>,
    test_pressure: Option<Pressure>,
) -> Result<LopaResult> {
    let spec = &model_gen::LOPA_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "initiating_event_frequency_per_year" => Some(initiating_event_frequency_per_year),
            "target_frequency_per_year" => Some(target_frequency_per_year),
            "mitigated_frequency_per_year" => Some(mitigated_frequency_per_year),
            "event_pressure" => event_pressure.map(|value| value.value),
            "design_pressure" => design_pressure.map(|value| value.value),
            "test_pressure" => test_pressure.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite(
        "initiating_event_frequency_per_year",
        initiating_event_frequency_per_year,
    )?;
    require_finite("target_frequency_per_year", target_frequency_per_year)?;
    require_finite("mitigated_frequency_per_year", mitigated_frequency_per_year)?;
    for (index, layer) in layers.iter().enumerate() {
        if !layer.pfd.is_finite() || !(0.0..=1.0).contains(&layer.pfd) {
            return Err(AzothError::invalid_input(
                format!("layers[{index}].pfd"),
                format!("a layer PFD is in [0, 1], and {} is not", layer.pfd),
            ));
        }
        for (field, value) in [
            ("frequency_before_per_year", layer.frequency_before_per_year),
            ("frequency_after_per_year", layer.frequency_after_per_year),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(AzothError::invalid_input(
                    format!("layers[{index}].{field}"),
                    format!("a layer frequency is finite and at least zero, and {value} is not"),
                ));
            }
        }
    }

    let sts0131 = match (event_pressure, design_pressure, test_pressure) {
        (None, None, None) => None,
        (Some(event), Some(design), Some(test)) => {
            for (name, value) in [
                ("event_pressure", event.value),
                ("design_pressure", design.value),
                ("test_pressure", test.value),
            ] {
                if !value.is_finite() || value <= 0.0 {
                    return Err(AzothError::invalid_input(
                        name,
                        format!("an STS0131 pressure is positive, and {value} is not"),
                    ));
                }
            }
            if test.value < design.value {
                return Err(AzothError::invalid_input(
                    "test_pressure",
                    "the test pressure is at least the design pressure",
                ));
            }
            Some(sts0131_category(event.value, design.value, test.value))
        }
        _ => {
            return Err(AzothError::invalid_input(
                "event_pressure",
                "the STS0131 triple is all three pressures or none",
            ));
        }
    };

    let target_met = mitigated_frequency_per_year <= target_frequency_per_year;
    let gap_to_target = mitigated_frequency_per_year - target_frequency_per_year;
    let total_rrf = if mitigated_frequency_per_year > 0.0 {
        initiating_event_frequency_per_year / mitigated_frequency_per_year
    } else {
        0.0
    };
    let required_additional_rrf = if target_met {
        Some(0.0)
    } else if target_frequency_per_year > 0.0 {
        Some(mitigated_frequency_per_year / target_frequency_per_year)
    } else {
        None
    };
    let required_additional_sil = if target_met {
        0.0
    } else {
        let rrf = required_additional_rrf.unwrap_or(f64::INFINITY);
        required_sil(1.0 / rrf) as f64
    };

    let layer_contributions: Vec<LopaLayerContribution> = layers
        .iter()
        .map(|layer| LopaLayerContribution {
            name: layer.name.clone(),
            pfd: layer.pfd,
            frequency_before_per_year: layer.frequency_before_per_year,
            frequency_after_per_year: layer.frequency_after_per_year,
            contribution_per_year: layer.frequency_before_per_year - layer.frequency_after_per_year,
            rrf: if layer.pfd > 0.0 {
                Some(1.0 / layer.pfd)
            } else {
                None
            },
        })
        .collect();

    let (sts0131_pressure_category, sts0131_target_frequency_per_year) = match sts0131 {
        Some((category, target)) => (Some(category), Some(target)),
        None => (None, None),
    };

    Ok(LopaResult {
        target_met,
        gap_to_target,
        total_rrf,
        required_additional_rrf,
        required_additional_sil,
        layer_contributions,
        sts0131_pressure_category,
        sts0131_target_frequency_per_year,
        warnings,
    })
}
