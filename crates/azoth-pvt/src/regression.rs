//! The PVT regression's parameters and the fluid they build.
//!
//! Spec: `specs/models/pvt/regression.toml`. `PVTRegression` retunes a characterised fluid
//! by applying each [`RegressionParameter`] to a clone and re-running the laboratory
//! experiments against the measured points. This module is that first step: the twelve
//! carried transforms, applied to the component rows before the mixture is assembled.
//!
//! **Five of the class's seventeen parameters stay owed.** `PLUS_MOLAR_MASS_MULTIPLIER`,
//! `GAMMA_ALPHA` and `GAMMA_ETA` need the mutable characterisation state the row boundary
//! does not carry; `VISCOSITY_LBC_MULTIPLIER` and `VISCOSITY_PEDERSEN_ALPHA` need a viscosity
//! model the PFCT route never selects. They are named here rather than silently absent.

use azoth_characterization::KijSource;
use azoth_characterization::component_row::FluidComponent;
use azoth_core::{AzothError, Result};
use azoth_eos::Cubic;

use crate::fluid::Fluid;

/// One `RegressionParameter` constant, by the name the spec's enum states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionParameter {
    /// `BIP_METHANE_C7PLUS`.
    BipMethaneC7Plus,
    /// `BIP_C2C6_C7PLUS`.
    BipC2C6C7Plus,
    /// `BIP_CO2_HC`.
    BipCo2Hc,
    /// `BIP_N2_HC`.
    BipN2Hc,
    /// `VOLUME_SHIFT_C7PLUS`.
    VolumeShiftC7Plus,
    /// `TC_MULTIPLIER_C7PLUS`.
    TcMultiplierC7Plus,
    /// `PC_MULTIPLIER_C7PLUS`.
    PcMultiplierC7Plus,
    /// `OMEGA_MULTIPLIER_C7PLUS`.
    OmegaMultiplierC7Plus,
    /// `VISCOSITY_CSP_1`.
    ViscosityCsp1,
    /// `VISCOSITY_CSP_2`.
    ViscosityCsp2,
    /// `VISCOSITY_CSP_3`.
    ViscosityCsp3,
    /// `VISCOSITY_CSP_4`.
    ViscosityCsp4,
}

impl RegressionParameter {
    /// The spec's enum spelling.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::BipMethaneC7Plus => "bip_methane_c7plus",
            Self::BipC2C6C7Plus => "bip_c2c6_c7plus",
            Self::BipCo2Hc => "bip_co2_hc",
            Self::BipN2Hc => "bip_n2_hc",
            Self::VolumeShiftC7Plus => "volume_shift_c7plus",
            Self::TcMultiplierC7Plus => "tc_multiplier_c7plus",
            Self::PcMultiplierC7Plus => "pc_multiplier_c7plus",
            Self::OmegaMultiplierC7Plus => "omega_multiplier_c7plus",
            Self::ViscosityCsp1 => "viscosity_csp_1",
            Self::ViscosityCsp2 => "viscosity_csp_2",
            Self::ViscosityCsp3 => "viscosity_csp_3",
            Self::ViscosityCsp4 => "viscosity_csp_4",
        }
    }

    /// The constant the spec's enum spelling names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "bip_methane_c7plus" => Self::BipMethaneC7Plus,
            "bip_c2c6_c7plus" => Self::BipC2C6C7Plus,
            "bip_co2_hc" => Self::BipCo2Hc,
            "bip_n2_hc" => Self::BipN2Hc,
            "volume_shift_c7plus" => Self::VolumeShiftC7Plus,
            "tc_multiplier_c7plus" => Self::TcMultiplierC7Plus,
            "pc_multiplier_c7plus" => Self::PcMultiplierC7Plus,
            "omega_multiplier_c7plus" => Self::OmegaMultiplierC7Plus,
            "viscosity_csp_1" => Self::ViscosityCsp1,
            "viscosity_csp_2" => Self::ViscosityCsp2,
            "viscosity_csp_3" => Self::ViscosityCsp3,
            "viscosity_csp_4" => Self::ViscosityCsp4,
            _ => return None,
        })
    }

    /// The CSP index this parameter scales, if it is one of the four.
    #[must_use]
    pub fn csp_index(self) -> Option<usize> {
        match self {
            Self::ViscosityCsp1 => Some(0),
            Self::ViscosityCsp2 => Some(1),
            Self::ViscosityCsp3 => Some(2),
            Self::ViscosityCsp4 => Some(3),
            _ => None,
        }
    }
}

/// One fitting parameter with the bounds and initial guess the class's config carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegressionParameterConfig {
    /// Which constant.
    pub parameter: RegressionParameter,
    /// The initial guess, which the optimizer evaluates without clamping.
    pub initial: f64,
    /// The lower bound the optimizer clamps its steps onto.
    pub lower: f64,
    /// The upper bound.
    pub upper: f64,
}

/// A fluid built for one parameter vector, and the four CSP factors it carries.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionFluid {
    /// The assembled fluid.
    pub fluid: Fluid,
    /// The four PFCT correction factors, in the class's order.
    pub csp: [f64; 4],
}

/// Apply the parameter vector to the rows, then assemble the mixture.
///
/// The transforms are the class's own:
/// * the four BIPs override named pairs, a later parameter winning on a shared pair;
/// * the three multipliers scale every pseudo row's `Tc`, `Pc` or acentric factor;
/// * the volume-shift multiplier scales the Peneloux constant, which is linear in the
///   Rackett compressibility: `z' = A - value*(A - z)` with `A = 0.29441` for SRK or
///   `0.25969` for PR;
/// * the four CSP factors are carried beside the fluid, not applied to it.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for a value count that does not match the parameter
///   list, a non-finite value, or a pseudo row the multiplier cannot act on.
/// * The assembler's own refusals.
pub fn build_fluid(
    rows: &[FluidComponent],
    cubic: Cubic,
    kij: KijSource,
    corrected: bool,
    parameters: &[RegressionParameterConfig],
    values: &[f64],
) -> Result<RegressionFluid> {
    if parameters.len() != values.len() {
        return Err(AzothError::invalid_input(
            "parameters",
            format!(
                "{} parameter rows carry {} values",
                parameters.len(),
                values.len()
            ),
        ));
    }
    let mut rows = rows.to_vec();
    let mut overrides: Vec<(usize, usize, f64)> = Vec::new();
    let mut csp = [1.0; 4];
    for (config, &value) in parameters.iter().zip(values) {
        if !value.is_finite() {
            return Err(AzothError::invalid_input(
                "parameters",
                format!("{} is not finite", config.parameter.name()),
            ));
        }
        if let Some(index) = config.parameter.csp_index() {
            csp[index] = value;
            continue;
        }
        match config.parameter {
            RegressionParameter::BipMethaneC7Plus
            | RegressionParameter::BipC2C6C7Plus
            | RegressionParameter::BipCo2Hc
            | RegressionParameter::BipN2Hc => {
                overrides.extend(bip_overrides(&rows, config.parameter, value));
            }
            RegressionParameter::VolumeShiftC7Plus => {
                let intercept = match cubic {
                    Cubic::Srk => 0.29441,
                    Cubic::Pr => 0.25969,
                    other => {
                        return Err(AzothError::invalid_input(
                            "cubic",
                            format!("{other:?} has no Peneloux translation to scale"),
                        ));
                    }
                };
                for (index, row) in rows.iter_mut().enumerate() {
                    if !row.pseudo_component {
                        continue;
                    }
                    let z = row.racket_z.filter(|value| value.abs() >= 1e-10).unwrap_or(
                        0.29056
                            - 0.08775
                                * row.acentric_factor.ok_or_else(|| {
                                    AzothError::invalid_input(
                                        format!("components[{index}].acentric_factor"),
                                        "a pseudo row without a Rackett compressibility needs \
                                         its acentric factor for the default",
                                    )
                                })?,
                    );
                    row.racket_z = Some(intercept - value * (intercept - z));
                }
            }
            RegressionParameter::TcMultiplierC7Plus
            | RegressionParameter::PcMultiplierC7Plus
            | RegressionParameter::OmegaMultiplierC7Plus => {
                for (index, row) in rows.iter_mut().enumerate() {
                    if !row.pseudo_component {
                        continue;
                    }
                    match config.parameter {
                        RegressionParameter::TcMultiplierC7Plus => {
                            let field = row.critical_temperature.as_mut().ok_or_else(|| {
                                AzothError::invalid_input(
                                    format!("components[{index}].critical_temperature"),
                                    "the Tc multiplier needs a pseudo row's critical temperature",
                                )
                            })?;
                            *field *= value;
                        }
                        RegressionParameter::PcMultiplierC7Plus => {
                            let field = row.critical_pressure.as_mut().ok_or_else(|| {
                                AzothError::invalid_input(
                                    format!("components[{index}].critical_pressure"),
                                    "the Pc multiplier needs a pseudo row's critical pressure",
                                )
                            })?;
                            *field *= value;
                        }
                        _ => {
                            let field = row.acentric_factor.as_mut().ok_or_else(|| {
                                AzothError::invalid_input(
                                    format!("components[{index}].acentric_factor"),
                                    "the acentric multiplier needs a pseudo row's acentric factor",
                                )
                            })?;
                            *field *= value;
                        }
                    }
                }
            }
            RegressionParameter::ViscosityCsp1
            | RegressionParameter::ViscosityCsp2
            | RegressionParameter::ViscosityCsp3
            | RegressionParameter::ViscosityCsp4 => unreachable!("handled by csp_index"),
        }
    }
    let fluid = Fluid::from_rows_with_kij(&rows, cubic, kij, &overrides, corrected)?;
    Ok(RegressionFluid { fluid, csp })
}

fn name(row: &FluidComponent) -> String {
    row.name.to_ascii_lowercase()
}

fn first(rows: &[FluidComponent], matches: impl Fn(&FluidComponent) -> bool) -> Option<usize> {
    rows.iter().position(matches)
}

fn is_methane(row: &FluidComponent) -> bool {
    matches!(name(row).as_str(), "methane" | "c1")
}

fn is_nitrogen(row: &FluidComponent) -> bool {
    matches!(name(row).as_str(), "nitrogen" | "n2")
}

fn is_co2(row: &FluidComponent) -> bool {
    matches!(name(row).as_str(), "co2" | "carbon dioxide")
}

fn is_c2c6(row: &FluidComponent) -> bool {
    let name = name(row);
    matches!(
        name.as_str(),
        "ethane" | "c2" | "propane" | "c3" | "c4" | "c5" | "c6"
    ) || name.contains("butane")
        || name.contains("pentane")
        || name.contains("hexane")
}

/// The pairs one BIP parameter sets, in row indices.
fn bip_overrides(
    rows: &[FluidComponent],
    parameter: RegressionParameter,
    value: f64,
) -> Vec<(usize, usize, f64)> {
    let mut pairs = Vec::new();
    let plus: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.pseudo_component)
        .map(|(index, _)| index)
        .collect();
    match parameter {
        RegressionParameter::BipMethaneC7Plus => {
            if let Some(index) = first(rows, is_methane) {
                pairs.extend(plus.iter().map(|&other| (index, other, value)));
            }
        }
        RegressionParameter::BipC2C6C7Plus => {
            for (index, row) in rows.iter().enumerate() {
                if is_c2c6(row) {
                    pairs.extend(plus.iter().map(|&other| (index, other, value)));
                }
            }
        }
        RegressionParameter::BipCo2Hc => {
            if let Some(index) = first(rows, is_co2) {
                pairs.extend(
                    (0..rows.len())
                        .filter(|&other| other != index)
                        .map(|other| (index, other, value)),
                );
            }
        }
        RegressionParameter::BipN2Hc => {
            if let Some(index) = first(rows, is_nitrogen) {
                pairs.extend(
                    (0..rows.len())
                        .filter(|&other| other != index)
                        .map(|other| (index, other, value)),
                );
            }
        }
        _ => {}
    }
    pairs
}
