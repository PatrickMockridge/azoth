//! `process.norsok_p002_compliance` - the NORSOK P-002 point-check set.
//!
//! Spec: `specs/models/process/norsok_p002_compliance.toml`. The arithmetic is
//! `process/safety/compliance/NorsokP002ComplianceChecker.java` at the pin: five numeric
//! comparisons against defaults and two caller-supplied recorded checks.

use azoth_core::units::{Pressure, Velocity};
use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;

/// The class's flare-line Mach limit.
const FLARE_MACH_LIMIT: f64 = 0.7;
/// The class's blowdown momentum-flux limit, Pa.
const BLOWDOWN_MOMENTUM_FLUX_LIMIT: f64 = 200_000.0;
/// The class's erosional momentum-flux limit, Pa.
const EROSIONAL_MOMENTUM_FLUX_LIMIT: f64 = 100_000.0;
/// The class's vent-gas velocity limit, m/s.
const VENT_VELOCITY_LIMIT: f64 = 60.0;
/// The class's liquid carry-over limit, volume fraction.
const CARRY_OVER_LIMIT: f64 = 1.0e-3;

/// The NORSOK P-002 criterion a finding belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum P002Criterion {
    /// Flare-line gas Mach number at or below 0.7.
    #[serde(rename = "flare_line_mach_07")]
    FlareLineMach07,
    /// Blowdown momentum flux at or below 200000 Pa.
    #[serde(rename = "blowdown_rho_v2")]
    BlowdownRhoV2,
    /// Two-phase erosional momentum flux at or below 100000 Pa.
    #[serde(rename = "erosional_velocity")]
    ErosionalVelocity,
    /// Atmospheric vent gas velocity at or below 60 m/s.
    #[serde(rename = "vent_gas_velocity")]
    VentGasVelocity,
    /// Liquid carry-over fraction at or below 0.001.
    #[serde(rename = "liquid_carry_over")]
    LiquidCarryOver,
    /// A depressurisation valve sized externally.
    #[serde(rename = "depressurisation_valve_size")]
    DepressurisationValveSize,
    /// A drain slope or capacity checked externally.
    #[serde(rename = "drain_slope_capacity")]
    DrainSlopeCapacity,
}

impl P002Criterion {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FlareLineMach07 => "flare_line_mach_07",
            Self::BlowdownRhoV2 => "blowdown_rho_v2",
            Self::ErosionalVelocity => "erosional_velocity",
            Self::VentGasVelocity => "vent_gas_velocity",
            Self::LiquidCarryOver => "liquid_carry_over",
            Self::DepressurisationValveSize => "depressurisation_valve_size",
            Self::DrainSlopeCapacity => "drain_slope_capacity",
        }
    }

    /// Parse the spec's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "flare_line_mach_07" => Some(Self::FlareLineMach07),
            "blowdown_rho_v2" => Some(Self::BlowdownRhoV2),
            "erosional_velocity" => Some(Self::ErosionalVelocity),
            "vent_gas_velocity" => Some(Self::VentGasVelocity),
            "liquid_carry_over" => Some(Self::LiquidCarryOver),
            "depressurisation_valve_size" => Some(Self::DepressurisationValveSize),
            "drain_slope_capacity" => Some(Self::DrainSlopeCapacity),
            _ => None,
        }
    }
}

/// One caller-stated point check.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct P002CheckRow {
    /// The segment, equipment or item.
    pub name: String,
    /// The criterion, the spec's spelling.
    pub criterion: String,
    /// The flare-line Mach number.
    pub mach_number: Option<f64>,
    /// The Mach limit.
    pub mach_limit: Option<f64>,
    /// The momentum flux.
    pub momentum_flux: Option<Pressure>,
    /// The momentum-flux limit.
    pub momentum_flux_limit: Option<Pressure>,
    /// The vent gas velocity.
    pub vent_velocity: Option<Velocity>,
    /// The vent-velocity limit.
    pub vent_velocity_limit: Option<Velocity>,
    /// The liquid carry-over fraction.
    pub carry_over_fraction: Option<f64>,
    /// The carry-over limit.
    pub carry_over_limit: Option<f64>,
    /// The caller's pass flag for a recorded criterion.
    pub recorded_ok: Option<bool>,
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct P002FindingResult {
    /// The segment, equipment or item.
    pub name: String,
    /// The criterion checked.
    pub criterion: P002Criterion,
    /// Whether the check passed.
    pub compliant: bool,
}

/// Result of `process.norsok_p002_compliance`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NorsokP002ComplianceResult {
    /// One finding per check, in caller order.
    pub findings: Vec<P002FindingResult>,
    /// Whether every finding is compliant.
    pub compliant: bool,
    /// The number of non-compliant findings.
    pub non_compliant_count: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// Refuse a value the arithmetic cannot carry to a published comparison.
fn require_finite(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation compares",
        ))
    }
}

/// Refuse a field the criterion does not read.
fn forbid(condition: bool, criterion: P002Criterion, field: &'static str) -> Result<()> {
    if condition {
        Err(AzothError::invalid_input(
            field,
            format!("`{}` does not read `{field}`", criterion.as_str()),
        ))
    } else {
        Ok(())
    }
}

/// The P-002 findings over the caller's point checks.
///
/// # Errors
/// `InvalidInput` for an unknown criterion, a row missing the field its criterion reads, a
/// row carrying another criterion's field, or a non-finite number.
pub fn norsok_p002_compliance(checks: &[P002CheckRow]) -> Result<NorsokP002ComplianceResult> {
    let mut findings = Vec::with_capacity(checks.len());
    for row in checks {
        let criterion = P002Criterion::parse(&row.criterion).ok_or_else(|| {
            AzothError::invalid_input(
                "criterion",
                "the criterion must be one of the seven NORSOK P-002 checks",
            )
        })?;
        let compliant = match criterion {
            P002Criterion::FlareLineMach07 => {
                let value = row.mach_number.ok_or_else(|| {
                    AzothError::invalid_input("mach_number", "the flare-line Mach check reads it")
                })?;
                forbid(row.momentum_flux.is_some(), criterion, "momentum_flux")?;
                forbid(
                    row.momentum_flux_limit.is_some(),
                    criterion,
                    "momentum_flux_limit",
                )?;
                forbid(row.vent_velocity.is_some(), criterion, "vent_velocity")?;
                forbid(
                    row.vent_velocity_limit.is_some(),
                    criterion,
                    "vent_velocity_limit",
                )?;
                forbid(
                    row.carry_over_fraction.is_some(),
                    criterion,
                    "carry_over_fraction",
                )?;
                forbid(
                    row.carry_over_limit.is_some(),
                    criterion,
                    "carry_over_limit",
                )?;
                forbid(row.recorded_ok.is_some(), criterion, "recorded_ok")?;
                let limit = row.mach_limit.unwrap_or(FLARE_MACH_LIMIT);
                require_finite("mach_number", value)?;
                require_finite("mach_limit", limit)?;
                value <= limit
            }
            P002Criterion::BlowdownRhoV2 | P002Criterion::ErosionalVelocity => {
                let value = row.momentum_flux.ok_or_else(|| {
                    AzothError::invalid_input("momentum_flux", "the momentum-flux check reads it")
                })?;
                forbid(row.mach_number.is_some(), criterion, "mach_number")?;
                forbid(row.mach_limit.is_some(), criterion, "mach_limit")?;
                forbid(row.vent_velocity.is_some(), criterion, "vent_velocity")?;
                forbid(
                    row.vent_velocity_limit.is_some(),
                    criterion,
                    "vent_velocity_limit",
                )?;
                forbid(
                    row.carry_over_fraction.is_some(),
                    criterion,
                    "carry_over_fraction",
                )?;
                forbid(
                    row.carry_over_limit.is_some(),
                    criterion,
                    "carry_over_limit",
                )?;
                forbid(row.recorded_ok.is_some(), criterion, "recorded_ok")?;
                let default_limit = if criterion == P002Criterion::BlowdownRhoV2 {
                    BLOWDOWN_MOMENTUM_FLUX_LIMIT
                } else {
                    EROSIONAL_MOMENTUM_FLUX_LIMIT
                };
                let limit = row
                    .momentum_flux_limit
                    .map_or(default_limit, |value| value.value);
                require_finite("momentum_flux", value.value)?;
                require_finite("momentum_flux_limit", limit)?;
                value.value <= limit
            }
            P002Criterion::VentGasVelocity => {
                let value = row.vent_velocity.ok_or_else(|| {
                    AzothError::invalid_input("vent_velocity", "the vent-velocity check reads it")
                })?;
                forbid(row.mach_number.is_some(), criterion, "mach_number")?;
                forbid(row.mach_limit.is_some(), criterion, "mach_limit")?;
                forbid(row.momentum_flux.is_some(), criterion, "momentum_flux")?;
                forbid(
                    row.momentum_flux_limit.is_some(),
                    criterion,
                    "momentum_flux_limit",
                )?;
                forbid(
                    row.carry_over_fraction.is_some(),
                    criterion,
                    "carry_over_fraction",
                )?;
                forbid(
                    row.carry_over_limit.is_some(),
                    criterion,
                    "carry_over_limit",
                )?;
                forbid(row.recorded_ok.is_some(), criterion, "recorded_ok")?;
                let limit = row
                    .vent_velocity_limit
                    .map_or(VENT_VELOCITY_LIMIT, |value| value.value);
                require_finite("vent_velocity", value.value)?;
                require_finite("vent_velocity_limit", limit)?;
                value.value <= limit
            }
            P002Criterion::LiquidCarryOver => {
                let value = row.carry_over_fraction.ok_or_else(|| {
                    AzothError::invalid_input(
                        "carry_over_fraction",
                        "the liquid carry-over check reads it",
                    )
                })?;
                forbid(row.mach_number.is_some(), criterion, "mach_number")?;
                forbid(row.mach_limit.is_some(), criterion, "mach_limit")?;
                forbid(row.momentum_flux.is_some(), criterion, "momentum_flux")?;
                forbid(
                    row.momentum_flux_limit.is_some(),
                    criterion,
                    "momentum_flux_limit",
                )?;
                forbid(row.vent_velocity.is_some(), criterion, "vent_velocity")?;
                forbid(
                    row.vent_velocity_limit.is_some(),
                    criterion,
                    "vent_velocity_limit",
                )?;
                forbid(row.recorded_ok.is_some(), criterion, "recorded_ok")?;
                let limit = row.carry_over_limit.unwrap_or(CARRY_OVER_LIMIT);
                require_finite("carry_over_fraction", value)?;
                require_finite("carry_over_limit", limit)?;
                value <= limit
            }
            P002Criterion::DepressurisationValveSize | P002Criterion::DrainSlopeCapacity => {
                let passed = row.recorded_ok.ok_or_else(|| {
                    AzothError::invalid_input(
                        "recorded_ok",
                        "a recorded criterion reads the caller's pass flag",
                    )
                })?;
                for (present, field) in [
                    (row.mach_number.is_some(), "mach_number"),
                    (row.mach_limit.is_some(), "mach_limit"),
                    (row.momentum_flux.is_some(), "momentum_flux"),
                    (row.momentum_flux_limit.is_some(), "momentum_flux_limit"),
                    (row.vent_velocity.is_some(), "vent_velocity"),
                    (row.vent_velocity_limit.is_some(), "vent_velocity_limit"),
                    (row.carry_over_fraction.is_some(), "carry_over_fraction"),
                    (row.carry_over_limit.is_some(), "carry_over_limit"),
                ] {
                    forbid(present, criterion, field)?;
                }
                passed
            }
        };
        findings.push(P002FindingResult {
            name: row.name.clone(),
            criterion,
            compliant,
        });
    }

    let non_compliant_count = findings.iter().filter(|row| !row.compliant).count() as f64;
    Ok(NorsokP002ComplianceResult {
        findings,
        compliant: non_compliant_count == 0.0,
        non_compliant_count,
        warnings: Vec::new(),
    })
}
