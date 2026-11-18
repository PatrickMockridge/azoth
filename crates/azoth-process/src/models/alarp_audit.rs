//! `process.alarp_audit` - the ALARP measure audit.
//!
//! Spec: `specs/models/process/alarp_audit.toml`. The arithmetic is
//! `process/safety/alarp/ALARPAuditReport.java` at the pin: each measure's implied cost of
//! averting a fatality against `VSL x disproportion_factor`.

use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;

/// The class's default value of a statistical life, NOK.
const DEFAULT_VALUE_OF_STATISTICAL_LIFE: f64 = 30.0e6;
/// The class's default disproportion factor.
const DEFAULT_DISPROPORTION_FACTOR: f64 = 3.0;

/// The ALARP verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AlarpVerdict {
    /// The measure is at or below the threshold.
    #[serde(rename = "implement")]
    Implement,
    /// The measure is above the threshold.
    #[serde(rename = "reject")]
    Reject,
}

impl AlarpVerdict {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implement => "implement",
            Self::Reject => "reject",
        }
    }
}

/// One candidate measure.
#[derive(Debug, Clone, PartialEq)]
pub struct AlarpMeasure {
    /// The measure's description.
    pub description: String,
    /// The averted fatality frequency per year.
    pub risk_reduction_per_year: f64,
    /// The annualised implementation cost, NOK/year.
    pub annualised_cost: f64,
}

/// One evaluated measure.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlarpEvaluationResult {
    /// The measure's description.
    pub description: String,
    /// The implied cost of averting a fatality, NOK; absent where the class answers infinity.
    pub icaf: Option<f64>,
    /// The ALARP verdict.
    pub verdict: AlarpVerdict,
}

/// Result of `process.alarp_audit`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlarpAuditResult {
    /// One row per measure, in caller order.
    pub evaluations: Vec<AlarpEvaluationResult>,
    /// `VSL * disproportion_factor`, NOK.
    pub threshold: f64,
    /// The number of evaluated measures.
    pub measure_count: f64,
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

/// Each measure's ICAF and its ALARP verdict.
///
/// # Errors
/// `InvalidInput` for a negative risk reduction or cost, a non-finite VSL, factor, measure or
/// threshold.
pub fn alarp_audit(
    measures: &[AlarpMeasure],
    value_of_statistical_life: Option<f64>,
    disproportion_factor: Option<f64>,
) -> Result<AlarpAuditResult> {
    let vsl = value_of_statistical_life.unwrap_or(DEFAULT_VALUE_OF_STATISTICAL_LIFE);
    let factor = disproportion_factor.unwrap_or(DEFAULT_DISPROPORTION_FACTOR);
    require_finite("value_of_statistical_life", vsl)?;
    require_finite("disproportion_factor", factor)?;
    let threshold = vsl * factor;
    if !threshold.is_finite() {
        return Err(AzothError::invalid_input(
            "threshold",
            "the value of a statistical life times the disproportion factor is not finite",
        ));
    }

    let mut evaluations = Vec::with_capacity(measures.len());
    for measure in measures {
        if measure.risk_reduction_per_year < 0.0 || measure.annualised_cost < 0.0 {
            return Err(AzothError::invalid_input(
                "risk_reduction_per_year",
                "the risk reduction and the annualised cost must be non-negative",
            ));
        }
        require_finite("risk_reduction_per_year", measure.risk_reduction_per_year)?;
        require_finite("annualised_cost", measure.annualised_cost)?;
        let icaf = if measure.risk_reduction_per_year == 0.0 {
            f64::INFINITY
        } else {
            measure.annualised_cost / measure.risk_reduction_per_year
        };
        evaluations.push(AlarpEvaluationResult {
            description: measure.description.clone(),
            icaf: if icaf.is_finite() { Some(icaf) } else { None },
            verdict: if icaf <= threshold {
                AlarpVerdict::Implement
            } else {
                AlarpVerdict::Reject
            },
        });
    }

    Ok(AlarpAuditResult {
        evaluations,
        threshold,
        measure_count: measures.len() as f64,
        warnings: Vec::new(),
    })
}
