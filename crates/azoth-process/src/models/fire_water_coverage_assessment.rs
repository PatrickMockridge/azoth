//! `process.fire_water_coverage_assessment` - the coverage gate as a registered id.
//!
//! Spec: `specs/models/process/fire_water_coverage_assessment.toml`. The arithmetic is
//! `process/safety/firewater/FireWaterCoverageAssessment.java` at the pin: the coverage, flow
//! and pressure questions in the class's own order.

use azoth_core::units::{Area, Pressure, VolumeRate, cubic_meters_per_second, square_meters};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// Which of the three questions failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SupplyVerdict {
    /// Nothing is missing.
    #[serde(rename = "compliant")]
    Compliant,
    /// The area is short of coverage and the supply has margin.
    #[serde(rename = "coverage_gap_supply_adequate")]
    CoverageGapSupplyAdequate,
    /// The spare supply cannot meet the flow deficit.
    #[serde(rename = "supply_flow_deficit")]
    SupplyFlowDeficit,
    /// The serving section's worst-case pressure margin is inadequate.
    #[serde(rename = "supply_pressure_deficit")]
    SupplyPressureDeficit,
    /// Neither a flow nor a pressure margin exists.
    #[serde(rename = "dedicated_supply_required")]
    DedicatedSupplyRequired,
}

impl SupplyVerdict {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compliant => "compliant",
            Self::CoverageGapSupplyAdequate => "coverage_gap_supply_adequate",
            Self::SupplyFlowDeficit => "supply_flow_deficit",
            Self::SupplyPressureDeficit => "supply_pressure_deficit",
            Self::DedicatedSupplyRequired => "dedicated_supply_required",
        }
    }
}

/// Result of `process.fire_water_coverage_assessment`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FireWaterCoverageAssessmentResult {
    /// The unprotected area.
    #[serde(serialize_with = "scalar")]
    pub coverage_gap: Area,
    /// The protected fraction of the area.
    pub coverage_fraction: f64,
    /// The flow the existing supply falls short by.
    #[serde(serialize_with = "scalar")]
    pub flow_deficit: VolumeRate,
    /// Whether the spare supply meets the deficit.
    pub flow_feasible: bool,
    /// Whether the worst-case margin meets the required one.
    pub pressure_feasible: bool,
    /// The class's classification.
    pub verdict: SupplyVerdict,
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

/// The coverage, flow and pressure questions, and the class's verdict.
///
/// # Errors
/// `InvalidInput` for a non-positive protected area or required flow, a negative covered
/// area, delivered flow or required margin, or a non-finite input.
pub fn fire_water_coverage_assessment(
    protected_area: Area,
    required_flow: VolumeRate,
    spare_supply_flow: VolumeRate,
    worst_case_pressure_margin: Pressure,
    covered_area: Option<Area>,
    existing_delivered_flow: Option<VolumeRate>,
    required_pressure_margin: Option<Pressure>,
) -> Result<FireWaterCoverageAssessmentResult> {
    let spec = &model_gen::FIRE_WATER_COVERAGE_ASSESSMENT_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "protected_area" => Some(protected_area.value),
            "required_flow" => Some(required_flow.value),
            "covered_area" => covered_area.map(|value| value.value),
            "existing_delivered_flow" => existing_delivered_flow.map(|value| value.value),
            "required_pressure_margin" => required_pressure_margin.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite("protected_area", protected_area.value)?;
    require_finite("required_flow", required_flow.value)?;
    require_finite("spare_supply_flow", spare_supply_flow.value)?;
    require_finite(
        "worst_case_pressure_margin",
        worst_case_pressure_margin.value,
    )?;
    let covered = covered_area.map_or(0.0, |value| value.value);
    require_finite("covered_area", covered)?;
    let existing = existing_delivered_flow.map_or(0.0, |value| value.value);
    require_finite("existing_delivered_flow", existing)?;
    let required_margin = required_pressure_margin.map_or(0.0, |value| value.value);
    require_finite("required_pressure_margin", required_margin)?;

    // The class's operation order, to the letter.
    let gap = (protected_area.value - covered).max(0.0);
    let fraction = (covered / protected_area.value).min(1.0);
    let deficit = (required_flow.value - existing).max(0.0);
    let flow_ok = spare_supply_flow.value >= deficit;
    let pressure_ok = worst_case_pressure_margin.value >= required_margin;
    let any_gap = gap > 0.0 || deficit > 0.0;
    let verdict = if !any_gap {
        SupplyVerdict::Compliant
    } else if !flow_ok && !pressure_ok {
        SupplyVerdict::DedicatedSupplyRequired
    } else if !pressure_ok {
        SupplyVerdict::SupplyPressureDeficit
    } else if !flow_ok {
        SupplyVerdict::SupplyFlowDeficit
    } else {
        SupplyVerdict::CoverageGapSupplyAdequate
    };

    Ok(FireWaterCoverageAssessmentResult {
        coverage_gap: square_meters(gap),
        coverage_fraction: fraction,
        flow_deficit: cubic_meters_per_second(deficit),
        flow_feasible: flow_ok,
        pressure_feasible: pressure_ok,
        verdict,
        warnings,
    })
}
