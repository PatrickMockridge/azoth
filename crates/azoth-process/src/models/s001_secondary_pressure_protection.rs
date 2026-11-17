//! `process.s001_secondary_pressure_protection` - the NORSOK S-001 clause 10.4.7 screen.
//!
//! Spec: `specs/models/process/s001_secondary_pressure_protection.toml`. The arithmetic is
//! NeqSim's `process/safety/processsafetysystem/S001SecondaryPressureProtectionCriteria.java`.

use azoth_core::units::Pressure;
use azoth_core::{Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::safety_review::{S001Criteria, s001_evaluate};

/// Result of `process.s001_secondary_pressure_protection`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct S001SecondaryPressureProtectionResult {
    /// The resolved target frequency, absent where the band cannot resolve.
    pub target_frequency_per_year: Option<f64>,
    /// Whether all three pressures are finite and positive.
    pub pressure_basis_complete: bool,
    /// Whether the test pressure is at or above the design pressure.
    pub test_pressure_valid: bool,
    /// Whether the event pressure is at or below the test pressure.
    pub pressure_within_test: bool,
    /// Whether the demand and target are configured.
    pub frequency_configured: bool,
    /// Whether the demand is at or below the target.
    pub frequency_criterion_met: bool,
    /// Whether relief leakage has been assessed.
    pub leakage_assessed: bool,
    /// Whether the leakage is routed to a safe location.
    pub leakage_routed_safe: bool,
    /// Whether the proof-test interval is configured.
    pub proof_test_configured: bool,
    /// Whether the interval is at most twelve months.
    pub proof_test_criterion_met: bool,
    /// Whether every criterion is met.
    pub acceptable: bool,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

const BARA_IN_PASCALS: f64 = 1.0e5;

fn bara(pressure: Option<Pressure>) -> Option<f64> {
    pressure.map(|value| value.value / BARA_IN_PASCALS)
}

/// The S-001 secondary-pressure screen.
///
/// # Errors
/// `InvalidInput` for a negative pressure or frequency, or a non-positive proof-test interval.
pub fn s001_secondary_pressure_protection(
    maximum_event_pressure: Option<Pressure>,
    design_pressure: Option<Pressure>,
    test_pressure: Option<Pressure>,
    demand_frequency_per_year: Option<f64>,
    target_frequency_per_year: Option<f64>,
    relief_leakage_assessed: Option<bool>,
    relief_leakage_to_safe_location: Option<bool>,
    proof_test_interval_months: Option<f64>,
) -> Result<S001SecondaryPressureProtectionResult> {
    let spec = &model_gen::S001_SECONDARY_PRESSURE_PROTECTION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "maximum_event_pressure" => maximum_event_pressure.map(|value| value.value),
            "design_pressure" => design_pressure.map(|value| value.value),
            "test_pressure" => test_pressure.map(|value| value.value),
            "demand_frequency_per_year" => demand_frequency_per_year,
            "target_frequency_per_year" => target_frequency_per_year,
            "proof_test_interval_months" => proof_test_interval_months,
            _ => None,
        },
        &mut warnings,
    )?;

    let outcome = s001_evaluate(S001Criteria {
        maximum_event_pressure: bara(maximum_event_pressure),
        design_pressure: bara(design_pressure),
        test_pressure: bara(test_pressure),
        demand_frequency_per_year,
        target_frequency_per_year,
        relief_leakage_assessed,
        relief_leakage_to_safe_location,
        proof_test_interval_months,
    });

    Ok(S001SecondaryPressureProtectionResult {
        target_frequency_per_year: outcome.target_frequency_per_year,
        pressure_basis_complete: outcome.pressure_basis_complete,
        test_pressure_valid: outcome.test_pressure_valid,
        pressure_within_test: outcome.pressure_within_test,
        frequency_configured: outcome.frequency_configured,
        frequency_criterion_met: outcome.frequency_criterion_met,
        leakage_assessed: outcome.leakage_assessed,
        leakage_routed_safe: outcome.leakage_routed_safe,
        proof_test_configured: outcome.proof_test_configured,
        proof_test_criterion_met: outcome.proof_test_criterion_met,
        acceptable: outcome.acceptable,
        warnings,
    })
}
