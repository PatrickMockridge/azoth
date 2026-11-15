//! `process.relief_disposal_network` - the simultaneous relief-load summation.
//!
//! Spec: `specs/models/process/relief_disposal_network.toml`. The arithmetic is
//! `process/safety/overpressure/ReliefDisposalNetwork.java` at the pin: the
//! simultaneous total, the peak single contribution and the row echoes.

use azoth_core::units::{MassRate, kilograms_per_hour, kilograms_per_second};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The overpressure cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReliefCause {
    /// Blocked outlet.
    #[serde(rename = "blocked_outlet")]
    BlockedOutlet,
    /// Check-valve leakage.
    #[serde(rename = "check_valve_leakage")]
    CheckValveLeakage,
    /// Gas blow-by.
    #[serde(rename = "gas_blow_by")]
    GasBlowBy,
    /// Liquid blow-by.
    #[serde(rename = "liquid_blow_by")]
    LiquidBlowBy,
    /// Volatile liquid ingress.
    #[serde(rename = "volatile_liquid_ingress")]
    VolatileLiquidIngress,
    /// Inadvertent valve opening.
    #[serde(rename = "inadvertent_valve_opening")]
    InadvertentValveOpening,
    /// Control-valve failure.
    #[serde(rename = "control_valve_failure")]
    ControlValveFailure,
    /// Fire.
    #[serde(rename = "fire")]
    Fire,
    /// Thermal expansion.
    #[serde(rename = "thermal_expansion")]
    ThermalExpansion,
    /// Choke collapse.
    #[serde(rename = "choke_collapse")]
    ChokeCollapse,
    /// Tube rupture.
    #[serde(rename = "tube_rupture")]
    TubeRupture,
    /// Vacuum.
    #[serde(rename = "vacuum")]
    Vacuum,
    /// Other.
    #[serde(rename = "other")]
    Other,
}

impl ReliefCause {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BlockedOutlet => "blocked_outlet",
            Self::CheckValveLeakage => "check_valve_leakage",
            Self::GasBlowBy => "gas_blow_by",
            Self::LiquidBlowBy => "liquid_blow_by",
            Self::VolatileLiquidIngress => "volatile_liquid_ingress",
            Self::InadvertentValveOpening => "inadvertent_valve_opening",
            Self::ControlValveFailure => "control_valve_failure",
            Self::Fire => "fire",
            Self::ThermalExpansion => "thermal_expansion",
            Self::ChokeCollapse => "choke_collapse",
            Self::TubeRupture => "tube_rupture",
            Self::Vacuum => "vacuum",
            Self::Other => "other",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "blocked_outlet" => Self::BlockedOutlet,
            "check_valve_leakage" => Self::CheckValveLeakage,
            "gas_blow_by" => Self::GasBlowBy,
            "liquid_blow_by" => Self::LiquidBlowBy,
            "volatile_liquid_ingress" => Self::VolatileLiquidIngress,
            "inadvertent_valve_opening" => Self::InadvertentValveOpening,
            "control_valve_failure" => Self::ControlValveFailure,
            "fire" => Self::Fire,
            "thermal_expansion" => Self::ThermalExpansion,
            "choke_collapse" => Self::ChokeCollapse,
            "tube_rupture" => Self::TubeRupture,
            "vacuum" => Self::Vacuum,
            "other" => Self::Other,
            _ => {
                return Err(AzothError::invalid_input(
                    "cause",
                    format!("`{value}` is not one of the class's overpressure causes"),
                ));
            }
        })
    }
}

/// The relieving phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReliefPhase {
    /// Vapour.
    #[serde(rename = "vapour")]
    Vapour,
    /// Liquid.
    #[serde(rename = "liquid")]
    Liquid,
    /// Two-phase.
    #[serde(rename = "two_phase")]
    TwoPhase,
}

impl ReliefPhase {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vapour => "vapour",
            Self::Liquid => "liquid",
            Self::TwoPhase => "two_phase",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "vapour" => Self::Vapour,
            "liquid" => Self::Liquid,
            "two_phase" => Self::TwoPhase,
            _ => {
                return Err(AzothError::invalid_input(
                    "phase",
                    format!("`{value}` is not `vapour`, `liquid` or `two_phase`"),
                ));
            }
        })
    }
}

/// One contribution, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReliefContribution {
    /// The protected item's name.
    pub item_name: String,
    /// The overpressure cause.
    pub cause: String,
    /// The relieving phase.
    pub phase: String,
    /// The contribution's relief rate.
    pub mass_flow: MassRate,
    /// Whether the load is concurrent.
    pub simultaneous: bool,
}

/// One echoed contribution.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReliefDisposalRow {
    /// The protected item's name.
    pub item_name: String,
    /// The overpressure cause.
    pub cause: ReliefCause,
    /// The relieving phase.
    pub phase: ReliefPhase,
    /// The rate.
    #[serde(serialize_with = "scalar")]
    pub mass_flow: MassRate,
    /// The rate times 3600.
    #[serde(serialize_with = "scalar")]
    pub mass_flow_kg_hr: MassRate,
    /// Whether the load is concurrent.
    pub simultaneous: bool,
}

/// Result of `process.relief_disposal_network`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReliefDisposalNetworkResult {
    /// The sum of the simultaneous rates.
    #[serde(serialize_with = "scalar")]
    pub total_simultaneous: MassRate,
    /// The simultaneous total times 3600.
    #[serde(serialize_with = "scalar")]
    pub total_simultaneous_kg_hr: MassRate,
    /// The largest single rate.
    #[serde(serialize_with = "scalar")]
    pub peak_single: MassRate,
    /// The item name of the first peak, absent when no rate is positive.
    pub governing_contributor: Option<String>,
    /// One row per input row, in caller order.
    pub contributions: Vec<ReliefDisposalRow>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The simultaneous total, the peak single load and the echoed rows.
///
/// # Errors
/// `InvalidInput` for a negative or non-finite rate, or a cause or phase the class
/// does not name.
pub fn relief_disposal_network(
    contributions: &[ReliefContribution],
) -> Result<ReliefDisposalNetworkResult> {
    let spec = &model_gen::RELIEF_DISPOSAL_NETWORK_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    let mut total = 0.0;
    let mut peak = 0.0;
    let mut governing: Option<String> = None;
    let mut rows = Vec::with_capacity(contributions.len());
    for (index, contribution) in contributions.iter().enumerate() {
        let rate = contribution.mass_flow.value;
        if !rate.is_finite() || rate < 0.0 {
            return Err(AzothError::invalid_input(
                format!("contributions[{index}].mass_flow"),
                format!("a finite non-negative rate is required, and {rate} is not"),
            ));
        }
        let cause = ReliefCause::parse(&contribution.cause)?;
        let phase = ReliefPhase::parse(&contribution.phase)?;
        if contribution.simultaneous {
            total += rate;
        }
        if rate > peak {
            peak = rate;
            governing = Some(contribution.item_name.clone());
        }
        rows.push(ReliefDisposalRow {
            item_name: contribution.item_name.clone(),
            cause,
            phase,
            mass_flow: kilograms_per_second(rate),
            mass_flow_kg_hr: kilograms_per_hour(rate * 3600.0),
            simultaneous: contribution.simultaneous,
        });
    }
    Ok(ReliefDisposalNetworkResult {
        total_simultaneous: kilograms_per_second(total),
        total_simultaneous_kg_hr: kilograms_per_hour(total * 3600.0),
        peak_single: kilograms_per_second(peak),
        governing_contributor: governing,
        contributions: rows,
        warnings,
    })
}
