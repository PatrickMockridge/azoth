//! `process.sts0131_gate` - the STS-0131 acceptance gate.
//!
//! Spec: `specs/models/process/sts0131_gate.toml`. The arithmetic is
//! `process/safety/compliance/Sts0131Gate.java` at the pin: the PSV area margin, the MDMT
//! comparison and the four pass-through checks, with no warning arm.

use azoth_core::units::{Area, ThermodynamicTemperature};
use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;

/// The STS-0131 check a finding belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Sts0131CheckType {
    /// A depressurisation acceptance result.
    #[serde(rename = "depressurisation")]
    Depressurisation,
    /// A PSV sizing margin.
    #[serde(rename = "psv_sizing_margin")]
    PsvSizingMargin,
    /// The minimum wall temperature against the design MDMT.
    #[serde(rename = "mdmt")]
    Mdmt,
    /// A trapped-liquid fire rupture prediction.
    #[serde(rename = "trapped_liquid_rupture")]
    TrappedLiquidRupture,
    /// A NOG 070 SIL determination.
    #[serde(rename = "sil")]
    Sil,
    /// A caller-named check.
    #[serde(rename = "custom")]
    Custom,
}

impl Sts0131CheckType {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Depressurisation => "depressurisation",
            Self::PsvSizingMargin => "psv_sizing_margin",
            Self::Mdmt => "mdmt",
            Self::TrappedLiquidRupture => "trapped_liquid_rupture",
            Self::Sil => "sil",
            Self::Custom => "custom",
        }
    }

    /// Parse the spec's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "depressurisation" => Some(Self::Depressurisation),
            "psv_sizing_margin" => Some(Self::PsvSizingMargin),
            "mdmt" => Some(Self::Mdmt),
            "trapped_liquid_rupture" => Some(Self::TrappedLiquidRupture),
            "sil" => Some(Self::Sil),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// The severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Sts0131Severity {
    /// The check passed.
    #[serde(rename = "pass")]
    Pass,
    /// The check is short of its criterion but not critical; no builder emits it.
    #[serde(rename = "warning")]
    Warning,
    /// The check failed.
    #[serde(rename = "fail")]
    Fail,
}

impl Sts0131Severity {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warning => "warning",
            Self::Fail => "fail",
        }
    }
}

/// One caller-stated check.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sts0131CheckRow {
    /// The check type, the spec's spelling.
    pub check_type: String,
    /// The custom check's name.
    pub name: Option<String>,
    /// The depressurisation acceptance.
    pub acceptable: Option<bool>,
    /// The required PSV area.
    pub required_area: Option<Area>,
    /// The selected PSV area.
    pub selected_area: Option<Area>,
    /// The required area margin.
    pub required_margin: Option<f64>,
    /// The minimum wall temperature.
    pub minimum_wall_temperature: Option<ThermodynamicTemperature>,
    /// The design MDMT.
    pub design_mdmt: Option<ThermodynamicTemperature>,
    /// The trapped-liquid rupture prediction.
    pub rupture_predicted: Option<bool>,
    /// The NOG 070 SIL compliance.
    pub sil_compliant: Option<bool>,
    /// The custom check's pass flag.
    pub custom_ok: Option<bool>,
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sts0131FindingResult {
    /// The check.
    pub check_type: Sts0131CheckType,
    /// The severity.
    pub severity: Sts0131Severity,
    /// The custom check's name.
    pub name: Option<String>,
    /// The PSV area margin.
    pub margin: Option<f64>,
}

/// Result of `process.sts0131_gate`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sts0131GateResult {
    /// One finding per check, in caller order.
    pub findings: Vec<Sts0131FindingResult>,
    /// Whether no finding fails.
    pub acceptable: bool,
    /// The number of failing findings.
    pub failure_count: f64,
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

/// Refuse a field the check does not read.
fn forbid(condition: bool, check: Sts0131CheckType, field: &'static str) -> Result<()> {
    if condition {
        Err(AzothError::invalid_input(
            field,
            format!("`{}` does not read `{field}`", check.as_str()),
        ))
    } else {
        Ok(())
    }
}

/// The STS-0131 gate findings over the caller's checks.
///
/// # Errors
/// `InvalidInput` for an unknown check type, a row missing the field its check reads, a row
/// carrying another check's field, a non-positive PSV area, or a non-finite number.
pub fn sts0131_gate(checks: &[Sts0131CheckRow]) -> Result<Sts0131GateResult> {
    let mut findings = Vec::with_capacity(checks.len());
    for row in checks {
        let check = Sts0131CheckType::parse(&row.check_type).ok_or_else(|| {
            AzothError::invalid_input(
                "check_type",
                "the check must be one of the six STS-0131 checks",
            )
        })?;
        let built_in = check != Sts0131CheckType::Custom;
        forbid(built_in && row.name.is_some(), check, "name")?;
        let (severity, margin) = match check {
            Sts0131CheckType::Depressurisation => {
                let passed = row.acceptable.ok_or_else(|| {
                    AzothError::invalid_input("acceptable", "the depressurisation check reads it")
                })?;
                for (present, field) in [
                    (row.required_area.is_some(), "required_area"),
                    (row.selected_area.is_some(), "selected_area"),
                    (row.required_margin.is_some(), "required_margin"),
                    (
                        row.minimum_wall_temperature.is_some(),
                        "minimum_wall_temperature",
                    ),
                    (row.design_mdmt.is_some(), "design_mdmt"),
                    (row.rupture_predicted.is_some(), "rupture_predicted"),
                    (row.sil_compliant.is_some(), "sil_compliant"),
                    (row.custom_ok.is_some(), "custom_ok"),
                ] {
                    forbid(present, check, field)?;
                }
                (severity(passed), None)
            }
            Sts0131CheckType::PsvSizingMargin => {
                let required = row.required_area.ok_or_else(|| {
                    AzothError::invalid_input("required_area", "the PSV margin check reads it")
                })?;
                let selected = row.selected_area.ok_or_else(|| {
                    AzothError::invalid_input("selected_area", "the PSV margin check reads it")
                })?;
                let required_margin = row.required_margin.ok_or_else(|| {
                    AzothError::invalid_input("required_margin", "the PSV margin check reads it")
                })?;
                if required.value <= 0.0 || selected.value <= 0.0 {
                    return Err(AzothError::invalid_input(
                        "required_area",
                        "the PSV areas must be positive",
                    ));
                }
                for (present, field) in [
                    (row.acceptable.is_some(), "acceptable"),
                    (
                        row.minimum_wall_temperature.is_some(),
                        "minimum_wall_temperature",
                    ),
                    (row.design_mdmt.is_some(), "design_mdmt"),
                    (row.rupture_predicted.is_some(), "rupture_predicted"),
                    (row.sil_compliant.is_some(), "sil_compliant"),
                    (row.custom_ok.is_some(), "custom_ok"),
                ] {
                    forbid(present, check, field)?;
                }
                require_finite("required_area", required.value)?;
                require_finite("selected_area", selected.value)?;
                require_finite("required_margin", required_margin)?;
                let value = (selected.value - required.value) / required.value;
                (severity(value >= required_margin), Some(value))
            }
            Sts0131CheckType::Mdmt => {
                let minimum = row.minimum_wall_temperature.ok_or_else(|| {
                    AzothError::invalid_input("minimum_wall_temperature", "the MDMT check reads it")
                })?;
                let design = row.design_mdmt.ok_or_else(|| {
                    AzothError::invalid_input("design_mdmt", "the MDMT check reads it")
                })?;
                for (present, field) in [
                    (row.acceptable.is_some(), "acceptable"),
                    (row.required_area.is_some(), "required_area"),
                    (row.selected_area.is_some(), "selected_area"),
                    (row.required_margin.is_some(), "required_margin"),
                    (row.rupture_predicted.is_some(), "rupture_predicted"),
                    (row.sil_compliant.is_some(), "sil_compliant"),
                    (row.custom_ok.is_some(), "custom_ok"),
                ] {
                    forbid(present, check, field)?;
                }
                require_finite("minimum_wall_temperature", minimum.value)?;
                require_finite("design_mdmt", design.value)?;
                (severity(minimum.value >= design.value), None)
            }
            Sts0131CheckType::TrappedLiquidRupture => {
                let ruptured = row.rupture_predicted.ok_or_else(|| {
                    AzothError::invalid_input(
                        "rupture_predicted",
                        "the trapped-liquid check reads it",
                    )
                })?;
                for (present, field) in [
                    (row.acceptable.is_some(), "acceptable"),
                    (row.required_area.is_some(), "required_area"),
                    (row.selected_area.is_some(), "selected_area"),
                    (row.required_margin.is_some(), "required_margin"),
                    (
                        row.minimum_wall_temperature.is_some(),
                        "minimum_wall_temperature",
                    ),
                    (row.design_mdmt.is_some(), "design_mdmt"),
                    (row.sil_compliant.is_some(), "sil_compliant"),
                    (row.custom_ok.is_some(), "custom_ok"),
                ] {
                    forbid(present, check, field)?;
                }
                (severity(!ruptured), None)
            }
            Sts0131CheckType::Sil => {
                let compliant = row.sil_compliant.ok_or_else(|| {
                    AzothError::invalid_input("sil_compliant", "the SIL check reads it")
                })?;
                for (present, field) in [
                    (row.acceptable.is_some(), "acceptable"),
                    (row.required_area.is_some(), "required_area"),
                    (row.selected_area.is_some(), "selected_area"),
                    (row.required_margin.is_some(), "required_margin"),
                    (
                        row.minimum_wall_temperature.is_some(),
                        "minimum_wall_temperature",
                    ),
                    (row.design_mdmt.is_some(), "design_mdmt"),
                    (row.rupture_predicted.is_some(), "rupture_predicted"),
                    (row.custom_ok.is_some(), "custom_ok"),
                ] {
                    forbid(present, check, field)?;
                }
                (severity(compliant), None)
            }
            Sts0131CheckType::Custom => {
                let passed = row.custom_ok.ok_or_else(|| {
                    AzothError::invalid_input("custom_ok", "the custom check reads it")
                })?;
                if row.name.is_none() {
                    return Err(AzothError::invalid_input(
                        "name",
                        "the custom check reads its name",
                    ));
                }
                for (present, field) in [
                    (row.acceptable.is_some(), "acceptable"),
                    (row.required_area.is_some(), "required_area"),
                    (row.selected_area.is_some(), "selected_area"),
                    (row.required_margin.is_some(), "required_margin"),
                    (
                        row.minimum_wall_temperature.is_some(),
                        "minimum_wall_temperature",
                    ),
                    (row.design_mdmt.is_some(), "design_mdmt"),
                    (row.rupture_predicted.is_some(), "rupture_predicted"),
                    (row.sil_compliant.is_some(), "sil_compliant"),
                ] {
                    forbid(present, check, field)?;
                }
                (severity(passed), None)
            }
        };
        findings.push(Sts0131FindingResult {
            check_type: check,
            severity,
            name: if built_in { None } else { row.name.clone() },
            margin,
        });
    }

    let failure_count = findings
        .iter()
        .filter(|row| row.severity == Sts0131Severity::Fail)
        .count() as f64;
    Ok(Sts0131GateResult {
        acceptable: failure_count == 0.0,
        failure_count,
        findings,
        warnings: Vec::new(),
    })
}

/// The severity of a pass or fail.
fn severity(passed: bool) -> Sts0131Severity {
    if passed {
        Sts0131Severity::Pass
    } else {
        Sts0131Severity::Fail
    }
}
