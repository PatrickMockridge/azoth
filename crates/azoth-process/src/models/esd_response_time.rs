//! `process.esd_response_time` - the ESD response-time budget.
//!
//! Spec: `specs/models/process/esd_response_time.toml`. The arithmetic is NeqSim's
//! `process/safety/esd/EsdResponseTimeSimulator.java`, through its `evaluate`.

use azoth_core::units::Time;
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;

/// One timed contribution.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EsdContribution {
    /// The contribution's name.
    pub name: String,
    /// The stage it belongs to.
    pub stage: EsdStage,
    /// The contribution's time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
}

/// The contribution's stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EsdStage {
    /// Detection.
    #[serde(rename = "detection")]
    Detection,
    /// Logic.
    #[serde(rename = "logic")]
    Logic,
    /// Final element.
    #[serde(rename = "final_element")]
    FinalElement,
}

impl EsdStage {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Detection => "detection",
            Self::Logic => "logic",
            Self::FinalElement => "final_element",
        }
    }

    /// The spec's vocabulary, parsed.
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "detection" => Ok(Self::Detection),
            "logic" => Ok(Self::Logic),
            "final_element" => Ok(Self::FinalElement),
            other => Err(AzothError::invalid_input(
                "contributions.stage",
                format!("`{other}` is not `detection`, `logic` or `final_element`"),
            )),
        }
    }
}

/// Result of `process.esd_response_time`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EsdResponseTimeResult {
    /// The echoed SIF identifier.
    pub sif_tag: String,
    /// The detection contributions' sum.
    #[serde(serialize_with = "scalar")]
    pub detection_time: Time,
    /// The logic contributions' sum.
    #[serde(serialize_with = "scalar")]
    pub logic_time: Time,
    /// The final-element contributions' sum.
    #[serde(serialize_with = "scalar")]
    pub final_element_time: Time,
    /// The three stage totals' sum.
    #[serde(serialize_with = "scalar")]
    pub total_response_time: Time,
    /// The budget, absent when none is stated.
    #[serde(serialize_with = "optional_scalar")]
    pub allowable_response_time: Option<Time>,
    /// `allowable_response_time - total_response_time`, absent with no budget.
    #[serde(serialize_with = "optional_scalar")]
    pub margin: Option<Time>,
    /// `total_response_time <= allowable_response_time`, and true with no budget.
    pub within_budget: bool,
    /// The contributions echoed in caller order.
    pub contributions: Vec<EsdContribution>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The ESD response-time budget.
///
/// # Errors
/// `InvalidInput` for an empty contribution list, a negative or non-finite time, or a
/// negative budget.
pub fn esd_response_time(
    sif_tag: Option<&str>,
    contributions: &[EsdContribution],
    allowable_response_time: Option<Time>,
) -> Result<EsdResponseTimeResult> {
    let spec = &model_gen::ESD_RESPONSE_TIME_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "allowable_response_time" => allowable_response_time.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    if contributions.is_empty() {
        return Err(AzothError::invalid_input(
            "contributions",
            "the class refuses an empty contribution list",
        ));
    }
    let mut detection = 0.0;
    let mut logic = 0.0;
    let mut final_element = 0.0;
    for row in contributions {
        let time = row.time.value;
        if !time.is_finite() || time < 0.0 {
            return Err(AzothError::invalid_input(
                "contributions.time",
                "a contribution's time must be finite and at least zero",
            ));
        }
        match row.stage {
            EsdStage::Detection => detection += time,
            EsdStage::Logic => logic += time,
            EsdStage::FinalElement => final_element += time,
        }
    }
    let total = detection + logic + final_element;
    let (allowable, margin, within_budget) = match allowable_response_time {
        Some(value) => {
            let allowable = value.value;
            (Some(allowable), Some(allowable - total), total <= allowable)
        }
        None => (None, None, true),
    };

    Ok(EsdResponseTimeResult {
        sif_tag: sif_tag.unwrap_or("ESD-SIF").to_string(),
        detection_time: azoth_core::units::seconds(detection),
        logic_time: azoth_core::units::seconds(logic),
        final_element_time: azoth_core::units::seconds(final_element),
        total_response_time: azoth_core::units::seconds(total),
        allowable_response_time: allowable.map(azoth_core::units::seconds),
        margin: margin.map(azoth_core::units::seconds),
        within_budget,
        contributions: contributions.to_vec(),
        warnings,
    })
}
