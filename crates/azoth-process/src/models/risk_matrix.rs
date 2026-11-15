//! `process.risk_matrix` - the risk matrix's static arithmetic.
//!
//! Spec: `specs/models/process/risk_matrix.toml`. The arithmetic is
//! `process/safety/risk/RiskMatrix.java` at the pin: the probability and consequence
//! bands, the score and level, the MTBF sentinel and the two cost terms.

use azoth_core::units::MassRate;
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The failure-frequency band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RiskProbabilityCategory {
    /// Below 0.1 failures a year.
    #[serde(rename = "very_low")]
    VeryLow,
    /// 0.1 to 0.5.
    #[serde(rename = "low")]
    Low,
    /// 0.5 to 1.
    #[serde(rename = "medium")]
    Medium,
    /// 1 to 2.
    #[serde(rename = "high")]
    High,
    /// 2 and above.
    #[serde(rename = "very_high")]
    VeryHigh,
}

impl RiskProbabilityCategory {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VeryLow => "very_low",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::VeryHigh => "very_high",
        }
    }
}

/// The production-loss band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RiskConsequenceCategory {
    /// Below 5 %.
    #[serde(rename = "negligible")]
    Negligible,
    /// 5 to 20 %.
    #[serde(rename = "minor")]
    Minor,
    /// 20 to 50 %.
    #[serde(rename = "moderate")]
    Moderate,
    /// 50 to 80 %.
    #[serde(rename = "major")]
    Major,
    /// 80 % and above.
    #[serde(rename = "catastrophic")]
    Catastrophic,
}

impl RiskConsequenceCategory {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Negligible => "negligible",
            Self::Minor => "minor",
            Self::Moderate => "moderate",
            Self::Major => "major",
            Self::Catastrophic => "catastrophic",
        }
    }
}

/// The score's band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RiskLevel {
    /// A score of 1 to 4.
    #[serde(rename = "low")]
    Low,
    /// 5 to 9.
    #[serde(rename = "medium")]
    Medium,
    /// 10 to 15.
    #[serde(rename = "high")]
    High,
    /// 16 to 25.
    #[serde(rename = "critical")]
    Critical,
}

impl RiskLevel {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// One equipment row, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct RiskEquipment {
    /// The equipment's name.
    pub name: String,
    /// The failure frequency in 1/year.
    pub failures_per_year: f64,
    /// The mean time to repair, in hours.
    pub mttr_hours: f64,
    /// The production loss, in percent.
    pub production_loss_percent: f64,
    /// The absolute production loss.
    pub production_loss_kg_per_hr: MassRate,
}

/// One assessment row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RiskAssessmentRow {
    /// The equipment's name.
    pub name: String,
    /// The failure frequency.
    pub failures_per_year: f64,
    /// `8760 / failures_per_year`, or 1e6 at zero.
    pub mtbf_hours: f64,
    /// The mean time to repair.
    pub mttr_hours: f64,
    /// `failures_per_year mttr`.
    pub expected_downtime_hours_per_year: f64,
    /// The frequency band.
    pub probability_category: RiskProbabilityCategory,
    /// The band's level.
    pub probability_level: f64,
    /// The production loss, in percent.
    pub production_loss_percent: f64,
    /// The absolute production loss.
    #[serde(serialize_with = "scalar")]
    pub production_loss_kg_hr: MassRate,
    /// The loss band.
    pub consequence_category: RiskConsequenceCategory,
    /// The band's level.
    pub consequence_level: f64,
    /// The two levels' product.
    pub risk_score: f64,
    /// The score's band.
    pub risk_level: RiskLevel,
    /// The cost of one failure.
    pub cost_per_failure: f64,
    /// The cost of one failure times the frequency.
    pub annual_risk_cost: f64,
}

/// Result of `process.risk_matrix`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RiskMatrixResult {
    /// One row per input row, in caller order.
    pub assessments: Vec<RiskAssessmentRow>,
    /// The sum of the annual costs.
    pub total_annual_risk_cost: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn probability_category(failures_per_year: f64) -> (RiskProbabilityCategory, f64) {
    if failures_per_year < 0.1 {
        (RiskProbabilityCategory::VeryLow, 1.0)
    } else if failures_per_year < 0.5 {
        (RiskProbabilityCategory::Low, 2.0)
    } else if failures_per_year < 1.0 {
        (RiskProbabilityCategory::Medium, 3.0)
    } else if failures_per_year < 2.0 {
        (RiskProbabilityCategory::High, 4.0)
    } else {
        (RiskProbabilityCategory::VeryHigh, 5.0)
    }
}

fn consequence_category(loss_percent: f64) -> (RiskConsequenceCategory, f64) {
    if loss_percent < 5.0 {
        (RiskConsequenceCategory::Negligible, 1.0)
    } else if loss_percent < 20.0 {
        (RiskConsequenceCategory::Minor, 2.0)
    } else if loss_percent < 50.0 {
        (RiskConsequenceCategory::Moderate, 3.0)
    } else if loss_percent < 80.0 {
        (RiskConsequenceCategory::Major, 4.0)
    } else {
        (RiskConsequenceCategory::Catastrophic, 5.0)
    }
}

fn risk_level(score: f64) -> RiskLevel {
    if score <= 4.0 {
        RiskLevel::Low
    } else if score <= 9.0 {
        RiskLevel::Medium
    } else if score <= 15.0 {
        RiskLevel::High
    } else {
        RiskLevel::Critical
    }
}

/// The bands, the score and level, the MTBF, the downtime and the two cost terms.
///
/// # Errors
/// `InvalidInput` for a negative or non-finite frequency, repair time or mass loss, a
/// loss percentage outside `[0, 100]`, or a negative price or downtime cost.
pub fn risk_matrix(
    equipment: &[RiskEquipment],
    product_price_per_kg: Option<f64>,
    downtime_cost_per_hour: Option<f64>,
) -> Result<RiskMatrixResult> {
    let spec = &model_gen::RISK_MATRIX_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "product_price_per_kg" => product_price_per_kg,
            "downtime_cost_per_hour" => downtime_cost_per_hour,
            _ => None,
        },
        &mut warnings,
    )?;
    let price = product_price_per_kg.unwrap_or(0.5);
    let downtime_cost = downtime_cost_per_hour.unwrap_or(5000.0);
    for (name, value) in [
        ("product_price_per_kg", price),
        ("downtime_cost_per_hour", downtime_cost),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(AzothError::invalid_input(
                name,
                format!("a non-negative finite value is required, and {value} is not"),
            ));
        }
    }

    let mut assessments = Vec::with_capacity(equipment.len());
    let mut total_annual_risk_cost = 0.0;
    for (index, row) in equipment.iter().enumerate() {
        if !row.failures_per_year.is_finite() || row.failures_per_year < 0.0 {
            return Err(AzothError::invalid_input(
                format!("equipment[{index}].failures_per_year"),
                format!(
                    "a failure frequency is finite and at least zero, and {} is not",
                    row.failures_per_year
                ),
            ));
        }
        if !row.mttr_hours.is_finite() || row.mttr_hours < 0.0 {
            return Err(AzothError::invalid_input(
                format!("equipment[{index}].mttr_hours"),
                format!(
                    "a repair time is finite and at least zero, and {} is not",
                    row.mttr_hours
                ),
            ));
        }
        if !row.production_loss_percent.is_finite()
            || !(0.0..=100.0).contains(&row.production_loss_percent)
        {
            return Err(AzothError::invalid_input(
                format!("equipment[{index}].production_loss_percent"),
                format!(
                    "a production loss is in [0, 100] percent, and {} is not",
                    row.production_loss_percent
                ),
            ));
        }
        if !row.production_loss_kg_per_hr.value.is_finite()
            || row.production_loss_kg_per_hr.value < 0.0
        {
            return Err(AzothError::invalid_input(
                format!("equipment[{index}].production_loss_kg_per_hr"),
                "a mass loss is finite and at least zero",
            ));
        }

        let (probability_category, probability_level) =
            probability_category(row.failures_per_year);
        let (consequence_category, consequence_level) =
            consequence_category(row.production_loss_percent);
        let risk_score = probability_level * consequence_level;
        let mtbf_hours = if row.failures_per_year > 0.0 {
            8760.0 / row.failures_per_year
        } else {
            1.0e6
        };
        let expected_downtime_hours_per_year = row.failures_per_year * row.mttr_hours;
        let loss_kg_hr = row.production_loss_kg_per_hr.value * 3600.0;
        let cost_per_failure = loss_kg_hr * row.mttr_hours * price
            + row.mttr_hours * downtime_cost;
        let annual_risk_cost = cost_per_failure * row.failures_per_year;
        total_annual_risk_cost += annual_risk_cost;

        assessments.push(RiskAssessmentRow {
            name: row.name.clone(),
            failures_per_year: row.failures_per_year,
            mtbf_hours,
            mttr_hours: row.mttr_hours,
            expected_downtime_hours_per_year,
            probability_category,
            probability_level,
            production_loss_percent: row.production_loss_percent,
            production_loss_kg_hr: row.production_loss_kg_per_hr,
            consequence_category,
            consequence_level,
            risk_score,
            risk_level: risk_level(risk_score),
            cost_per_failure,
            annual_risk_cost,
        });
    }

    Ok(RiskMatrixResult {
        assessments,
        total_annual_risk_cost,
        warnings,
    })
}
