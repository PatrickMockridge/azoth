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
use azoth_core::{AzothError, Result, Warning};
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

/// One experiment the regression fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExperimentType {
    /// A constant-composition expansion.
    Cce,
    /// A constant-volume depletion.
    Cvd,
    /// A differential liberation.
    Dle,
    /// One phase's viscosity at one state.
    Viscosity,
}

/// One CCE's schedule and its measured relative volumes.
#[derive(Debug, Clone, PartialEq)]
pub struct CceExperiment {
    /// The experiment's temperature, in K.
    pub temperature: f64,
    /// The schedule, in Pa.
    pub pressures: Vec<f64>,
    /// The measured relative volumes, aligned with `pressures`.
    pub relative_volumes: Vec<f64>,
}

/// One CVD's schedule and its measured dropout and gas Z.
#[derive(Debug, Clone, PartialEq)]
pub struct CvdExperiment {
    /// The experiment's temperature, in K.
    pub temperature: f64,
    /// The schedule, in Pa.
    pub pressures: Vec<f64>,
    /// The measured liquid volume over the saturation volume, aligned with `pressures`.
    pub liquid_dropout: Vec<f64>,
    /// The measured gas compressibility, aligned with `pressures`.
    pub z_factor: Vec<f64>,
}

/// One DLE's schedule and its measured Rs, Bo and oil density.
#[derive(Debug, Clone, PartialEq)]
pub struct DleExperiment {
    /// The experiment's temperature, in K.
    pub temperature: f64,
    /// The schedule, in Pa.
    pub pressures: Vec<f64>,
    /// The measured solution gas-oil ratio.
    pub rs: Vec<f64>,
    /// The measured formation volume factor.
    pub bo: Vec<f64>,
    /// The measured oil density, in kg/m3.
    pub oil_density: Vec<f64>,
}

/// Which phase a viscosity observation names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViscosityPhase {
    /// The gas phase.
    Gas,
    /// The oil, or liquid, phase.
    Oil,
    /// The aqueous, or water, phase.
    Aqueous,
}

impl ViscosityPhase {
    /// The spec's enum spelling.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Gas => "gas",
            Self::Oil => "oil",
            Self::Aqueous => "aqueous",
        }
    }

    /// The constant the spec's enum spelling names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "gas" | "vapor" | "vapour" => Self::Gas,
            "oil" | "liquid" => Self::Oil,
            "aqueous" | "water" => Self::Aqueous,
            _ => return None,
        })
    }
}

/// One measured viscosity at one state.
#[derive(Debug, Clone, PartialEq)]
pub struct ViscosityPoint {
    /// The state's temperature, in K.
    pub temperature: f64,
    /// The state's pressure, in Pa.
    pub pressure: f64,
    /// The measured dynamic viscosity, in Pa*s.
    pub viscosity: f64,
    /// Which phase the measurement names.
    pub phase: ViscosityPhase,
}

/// The class's per-experiment weights, which divide the sample standard deviations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExperimentWeights {
    /// The CCE weight; the class's default is 1.
    pub cce: f64,
    /// The CVD weight; the class's default is 1.
    pub cvd: f64,
    /// The DLE weight; the class's default is 1.
    pub dle: f64,
    /// The viscosity weight; the class's default is 0.5.
    pub viscosity: f64,
}

impl Default for ExperimentWeights {
    fn default() -> Self {
        Self {
            cce: 1.0,
            cvd: 1.0,
            dle: 1.0,
            viscosity: 0.5,
        }
    }
}

/// Everything the fit is held to.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionData {
    /// The CCE experiments, in registration order.
    pub cce: Vec<CceExperiment>,
    /// The CVD experiments, in registration order.
    pub cvd: Vec<CvdExperiment>,
    /// The DLE experiments, in registration order.
    pub dle: Vec<DleExperiment>,
    /// The viscosity points, in registration order.
    pub viscosity: Vec<ViscosityPoint>,
    /// The per-experiment weights.
    pub weights: ExperimentWeights,
}

/// Which CVD value a sample fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CvdProperty {
    /// The liquid volume over the saturation volume.
    LiquidDropout,
    /// The gas compressibility.
    GasZ,
}

/// Which DLE value a sample fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DleProperty {
    /// The solution gas-oil ratio.
    Rs,
    /// The formation volume factor.
    Bo,
    /// The oil density.
    OilDensity,
}

/// One sample of the fit: an observed value, its standard deviation and where it is read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// The measured value.
    pub observed: f64,
    /// The class's standard deviation, after the experiment's weight divides it.
    pub sigma: f64,
    /// The model value's address.
    pub kind: SampleKind,
}

/// Where a sample's model value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    /// One CCE point.
    Cce {
        /// Which experiment.
        experiment: usize,
        /// Which point.
        point: usize,
    },
    /// One CVD row's dropout or gas Z.
    Cvd {
        /// Which experiment.
        experiment: usize,
        /// Which row.
        row: usize,
        /// Which value.
        property: CvdProperty,
    },
    /// One DLE row's Rs, Bo or density.
    Dle {
        /// Which experiment.
        experiment: usize,
        /// Which row.
        row: usize,
        /// Which value.
        property: DleProperty,
    },
    /// One viscosity point.
    Viscosity {
        /// Which point.
        point: usize,
    },
}

fn finite_positive(name: &str, value: f64) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        return Ok(());
    }
    Err(AzothError::invalid_input(
        name,
        format!("a positive finite value is needed; got {value}"),
    ))
}

fn weight(name: &str, value: f64) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        return Ok(());
    }
    Err(AzothError::invalid_input(
        name,
        format!("a weight must be finite and above zero; got {value}"),
    ))
}

impl RegressionData {
    /// The samples in the class's registration order, with its standard deviations.
    ///
    /// # Errors
    /// * [`AzothError::InvalidInput`] for a weight at or below zero, an observation vector
    ///   whose length is not the schedule's, a non-positive temperature or pressure, or a
    ///   non-positive CCE relative volume - which the objective divides by.
    pub fn samples(&self) -> Result<Vec<Sample>> {
        weight("cce_weight", self.weights.cce)?;
        weight("cvd_weight", self.weights.cvd)?;
        weight("dle_weight", self.weights.dle)?;
        weight("viscosity_weight", self.weights.viscosity)?;
        let mut samples = Vec::new();
        for (experiment, data) in self.cce.iter().enumerate() {
            finite_positive("cce.temperature", data.temperature)?;
            if data.pressures.len() != data.relative_volumes.len() {
                return Err(AzothError::invalid_input(
                    "cce",
                    "the schedule and its relative volumes differ in length",
                ));
            }
            for (point, (&pressure, &relative_volume)) in data
                .pressures
                .iter()
                .zip(&data.relative_volumes)
                .enumerate()
            {
                finite_positive(&format!("cce.pressures[{point}]"), pressure)?;
                finite_positive(&format!("cce.relative_volumes[{point}]"), relative_volume)?;
                samples.push(Sample {
                    observed: relative_volume,
                    sigma: 0.01 / self.weights.cce,
                    kind: SampleKind::Cce { experiment, point },
                });
            }
        }
        for (experiment, data) in self.cvd.iter().enumerate() {
            finite_positive("cvd.temperature", data.temperature)?;
            if data.pressures.len() != data.liquid_dropout.len()
                || data.pressures.len() != data.z_factor.len()
            {
                return Err(AzothError::invalid_input(
                    "cvd",
                    "the schedule, its dropout and its Z differ in length",
                ));
            }
            for row in 0..data.pressures.len() {
                finite_positive(&format!("cvd.pressures[{row}]"), data.pressures[row])?;
                for (property, observed) in [
                    (CvdProperty::LiquidDropout, data.liquid_dropout[row]),
                    (CvdProperty::GasZ, data.z_factor[row]),
                ] {
                    samples.push(Sample {
                        observed,
                        sigma: 0.01 / self.weights.cvd,
                        kind: SampleKind::Cvd {
                            experiment,
                            row,
                            property,
                        },
                    });
                }
            }
        }
        for (experiment, data) in self.dle.iter().enumerate() {
            finite_positive("dle.temperature", data.temperature)?;
            if data.pressures.len() != data.rs.len()
                || data.pressures.len() != data.bo.len()
                || data.pressures.len() != data.oil_density.len()
            {
                return Err(AzothError::invalid_input(
                    "dle",
                    "the schedule and its three observations differ in length",
                ));
            }
            for row in 0..data.pressures.len() {
                finite_positive(&format!("dle.pressures[{row}]"), data.pressures[row])?;
                for (property, observed) in [
                    (DleProperty::Rs, data.rs[row]),
                    (DleProperty::Bo, data.bo[row]),
                    (DleProperty::OilDensity, data.oil_density[row]),
                ] {
                    samples.push(Sample {
                        observed,
                        sigma: (observed.abs() * 0.01).max(1e-3) / self.weights.dle,
                        kind: SampleKind::Dle {
                            experiment,
                            row,
                            property,
                        },
                    });
                }
            }
        }
        for (point, data) in self.viscosity.iter().enumerate() {
            finite_positive(&format!("viscosity[{point}].temperature"), data.temperature)?;
            finite_positive(&format!("viscosity[{point}].pressure"), data.pressure)?;
            samples.push(Sample {
                observed: data.viscosity,
                sigma: (data.viscosity.abs() * 0.02).max(1e-10) / self.weights.viscosity,
                kind: SampleKind::Viscosity { point },
            });
        }
        Ok(samples)
    }
}

/// The model value one sample reads.
///
/// # Errors
/// Propagates the kernels' refusals.
pub fn sample_value(
    fluid: &Fluid,
    csp: [f64; 4],
    sample: &Sample,
    data: &RegressionData,
) -> Result<f64> {
    match sample.kind {
        SampleKind::Cce { experiment, point } => {
            let data = &data.cce[experiment];
            let result = crate::constant_mass_expansion_of(
                fluid,
                azoth_core::units::kelvins(data.temperature),
                &[data.pressures[point]],
            )?;
            Ok(result.points[0].relative_volume)
        }
        SampleKind::Cvd {
            experiment,
            row,
            property,
        } => {
            let data = &data.cvd[experiment];
            let result = crate::constant_volume_depletion_of(
                fluid,
                azoth_core::units::kelvins(data.temperature),
                &data.pressures,
            )?;
            let point = &result.points[row];
            Ok(match property {
                CvdProperty::LiquidDropout => point.liquid_volume_relative_to_vsat.unwrap_or(0.0),
                CvdProperty::GasZ => point.z_gas.unwrap_or(0.0),
            })
        }
        SampleKind::Dle {
            experiment,
            row,
            property,
        } => {
            let data = &data.dle[experiment];
            let result = crate::differential_liberation_of(
                fluid,
                azoth_core::units::kelvins(data.temperature),
                &data.pressures,
            )?;
            let point = &result.points[row];
            Ok(match property {
                DleProperty::Rs => point.rs.unwrap_or(0.0),
                DleProperty::Bo => point.bo.unwrap_or(1.0),
                DleProperty::OilDensity => point.oil_density,
            })
        }
        SampleKind::Viscosity { point } => {
            let data = &data.viscosity[point];
            let flash = fluid.flash(data.temperature, data.pressure, crate::FlashMode::TwoPhase)?;
            let phase = match data.phase {
                ViscosityPhase::Gas => flash.gas(),
                ViscosityPhase::Oil => flash.oil(),
                ViscosityPhase::Aqueous => flash.aqueous(),
            };
            match phase {
                None => Ok(0.0),
                Some(phase) => crate::transport::phase_viscosity_with_csp(
                    &fluid.mixture,
                    data.temperature,
                    data.pressure,
                    &phase.x,
                    csp,
                ),
            }
        }
    }
}

/// Every sample's model value, with each CVD and DLE experiment evaluated once.
///
/// The class's observation clones share one full-experiment result per parameter vector.
/// This is the same: a schedule is solved once, not once per dependent row, which is the
/// difference between a six-second fit and an eighty-second one.
///
/// # Errors
/// Propagates the kernels' refusals.
pub fn sample_values(
    fluid: &Fluid,
    csp: [f64; 4],
    samples: &[Sample],
    data: &RegressionData,
) -> Result<Vec<f64>> {
    let cvd_results = data
        .cvd
        .iter()
        .map(|experiment| {
            crate::constant_volume_depletion_of(
                fluid,
                azoth_core::units::kelvins(experiment.temperature),
                &experiment.pressures,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let dle_results = data
        .dle
        .iter()
        .map(|experiment| {
            crate::differential_liberation_of(
                fluid,
                azoth_core::units::kelvins(experiment.temperature),
                &experiment.pressures,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let mut values = Vec::with_capacity(samples.len());
    for sample in samples {
        values.push(match sample.kind {
            SampleKind::Cce { experiment, point } => {
                let data = &data.cce[experiment];
                let result = crate::constant_mass_expansion_of(
                    fluid,
                    azoth_core::units::kelvins(data.temperature),
                    &[data.pressures[point]],
                )?;
                result.points[0].relative_volume
            }
            SampleKind::Cvd {
                experiment,
                row,
                property,
            } => {
                let point = &cvd_results[experiment].points[row];
                match property {
                    CvdProperty::LiquidDropout => {
                        point.liquid_volume_relative_to_vsat.unwrap_or(0.0)
                    }
                    CvdProperty::GasZ => point.z_gas.unwrap_or(0.0),
                }
            }
            SampleKind::Dle {
                experiment,
                row,
                property,
            } => {
                let point = &dle_results[experiment].points[row];
                match property {
                    DleProperty::Rs => point.rs.unwrap_or(0.0),
                    DleProperty::Bo => point.bo.unwrap_or(1.0),
                    DleProperty::OilDensity => point.oil_density,
                }
            }
            SampleKind::Viscosity { point } => {
                let point = &data.viscosity[point];
                let flash = fluid.flash(
                    point.temperature,
                    point.pressure,
                    crate::FlashMode::TwoPhase,
                )?;
                let phase = match point.phase {
                    ViscosityPhase::Gas => flash.gas(),
                    ViscosityPhase::Oil => flash.oil(),
                    ViscosityPhase::Aqueous => flash.aqueous(),
                };
                match phase {
                    None => 0.0,
                    Some(phase) => crate::transport::phase_viscosity_with_csp(
                        &fluid.mixture,
                        point.temperature,
                        point.pressure,
                        &phase.x,
                        csp,
                    )?,
                }
            }
        });
    }
    Ok(values)
}

/// The weighted chi-square the optimizer minimises.
///
/// # Errors
/// Propagates [`RegressionData::samples`] and [`sample_value`].
pub fn chi_square(
    fluid: &Fluid,
    csp: [f64; 4],
    samples: &[Sample],
    data: &RegressionData,
) -> Result<f64> {
    let mut sum = 0.0;
    for sample in samples {
        let value = sample_value(fluid, csp, sample, data)?;
        let residual = (value - sample.observed) / sample.sigma;
        sum += residual * residual;
    }
    Ok(sum)
}

/// The class's second ledger: unweighted squared relative residuals, each divided by its
/// experiment's point count.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Objectives {
    /// The CCE objective.
    pub cce: Option<f64>,
    /// The CVD objective.
    pub cvd: Option<f64>,
    /// The DLE objective.
    pub dle: Option<f64>,
    /// The viscosity objective.
    pub viscosity: Option<f64>,
}

impl Objectives {
    /// The sum in the class's declaration order.
    #[must_use]
    pub fn total(&self) -> f64 {
        self.cce.unwrap_or(0.0)
            + self.cvd.unwrap_or(0.0)
            + self.dle.unwrap_or(0.0)
            + self.viscosity.unwrap_or(0.0)
    }
}

/// The second ledger, computed from the tuned fluid.
///
/// # Errors
/// Propagates the kernels' refusals.
pub fn objectives(fluid: &Fluid, csp: [f64; 4], data: &RegressionData) -> Result<Objectives> {
    let mut values = Objectives::default();
    if !data.cce.is_empty() {
        // The class flattens every CCE experiment onto the first point's temperature.
        let temperature = data.cce[0].temperature;
        let mut pressures = Vec::new();
        for experiment in &data.cce {
            pressures.extend(&experiment.pressures);
        }
        let result = crate::constant_mass_expansion_of(
            fluid,
            azoth_core::units::kelvins(temperature),
            &pressures,
        )?;
        let mut total = 0.0;
        let mut index = 0;
        for experiment in &data.cce {
            for observed in &experiment.relative_volumes {
                let calculated = result.points[index].relative_volume;
                total += ((calculated - observed) / observed).powi(2);
                index += 1;
            }
        }
        if index > 0 {
            values.cce = Some(total / index as f64);
        }
    }
    if !data.cvd.is_empty() {
        let mut total = 0.0;
        let mut points = 0;
        for experiment in &data.cvd {
            let result = crate::constant_volume_depletion_of(
                fluid,
                azoth_core::units::kelvins(experiment.temperature),
                &experiment.pressures,
            )?;
            for (row, point) in result.points.iter().enumerate() {
                let dropout = point.liquid_volume_relative_to_vsat.unwrap_or(0.0);
                let observed = experiment.liquid_dropout[row];
                total += ((dropout - observed) / observed.max(0.01)).powi(2);
                let z = point.z_gas.unwrap_or(0.0);
                let observed_z = experiment.z_factor[row];
                total += ((z - observed_z) / observed_z.abs().max(1e-12)).powi(2);
            }
            points += experiment.pressures.len();
        }
        if points > 0 {
            values.cvd = Some(total / (2 * points) as f64);
        }
    }
    if !data.dle.is_empty() {
        let mut total = 0.0;
        let mut points = 0;
        for experiment in &data.dle {
            let result = crate::differential_liberation_of(
                fluid,
                azoth_core::units::kelvins(experiment.temperature),
                &experiment.pressures,
            )?;
            for (row, point) in result.points.iter().enumerate() {
                let calculated = [
                    point.rs.unwrap_or(0.0),
                    point.bo.unwrap_or(1.0),
                    point.oil_density,
                ];
                let observed = [
                    experiment.rs[row],
                    experiment.bo[row],
                    experiment.oil_density[row],
                ];
                for (calculated, observed) in calculated.into_iter().zip(observed) {
                    if observed > 0.0 {
                        total += ((calculated - observed) / observed).powi(2);
                    }
                }
            }
            points += experiment.pressures.len();
        }
        if points > 0 {
            values.dle = Some(total / (3 * points) as f64);
        }
    }
    if !data.viscosity.is_empty() {
        let mut total = 0.0;
        for point in &data.viscosity {
            let flash = fluid.flash(
                point.temperature,
                point.pressure,
                crate::FlashMode::TwoPhase,
            )?;
            let phase = match point.phase {
                ViscosityPhase::Gas => flash.gas(),
                ViscosityPhase::Oil => flash.oil(),
                ViscosityPhase::Aqueous => flash.aqueous(),
            };
            let calculated = match phase {
                None => 0.0,
                Some(phase) => crate::transport::phase_viscosity_with_csp(
                    &fluid.mixture,
                    point.temperature,
                    point.pressure,
                    &phase.x,
                    csp,
                )?,
            };
            if point.viscosity > 0.0 {
                total += ((calculated - point.viscosity) / point.viscosity).powi(2);
            }
        }
        values.viscosity = Some(total / data.viscosity.len() as f64);
    }
    Ok(values)
}

/// Why the optimizer stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvergenceReason {
    /// The weighted chi-square reached `1e-6`.
    ChiSquareTolerance,
    /// The weighted gradient norm reached `1e-6`.
    GradientTolerance,
    /// The iteration cap was reached first.
    MaxIterationsReached,
    /// The normal matrix could not be solved.
    SingularMatrix,
}

impl ConvergenceReason {
    /// The spec's enum spelling.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::ChiSquareTolerance => "chi_square_tolerance",
            Self::GradientTolerance => "gradient_tolerance",
            Self::MaxIterationsReached => "max_iterations_reached",
            Self::SingularMatrix => "singular_matrix",
        }
    }

    /// The constant the spec's enum spelling names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "chi_square_tolerance" => Self::ChiSquareTolerance,
            "gradient_tolerance" => Self::GradientTolerance,
            "max_iterations_reached" => Self::MaxIterationsReached,
            "singular_matrix" => Self::SingularMatrix,
            _ => return None,
        })
    }
}

/// The optimizer's answer.
#[derive(Debug, Clone, PartialEq)]
pub struct LmResult {
    /// The parameter vector at the last evaluation.
    pub parameters: Vec<f64>,
    /// The number of steps taken.
    pub iterations: usize,
    /// Why it stopped.
    pub reason: ConvergenceReason,
    /// The final weighted chi-square.
    pub chi_square: f64,
    /// The final weighted gradient norm.
    pub gradient_norm: f64,
}

/// The class's constants.
pub const LM_CHI_SQUARE_TOLERANCE: f64 = 1.0e-6;
/// The class's gradient tolerance.
pub const LM_GRADIENT_TOLERANCE: f64 = 1.0e-6;
/// The class's five-iteration floor.
pub const LM_MINIMUM_ITERATIONS: usize = 5;
/// The class's default cap.
pub const LM_MAXIMUM_ITERATIONS: usize = 100;

const RIDDERS_CON: f64 = 1.4;
const RIDDERS_NTAB: usize = 10;
const RIDDERS_SAFE: f64 = 2.0;
const RIDDERS_BIG: f64 = 1.0e30;

fn clamp(value: f64, bound: (f64, f64)) -> f64 {
    value.max(bound.0).min(bound.1)
}

/// The weighted chi-square of one model vector.
fn weighted_chi(values: &[f64], observed: &[f64], sigma: &[f64]) -> f64 {
    values
        .iter()
        .zip(observed)
        .zip(sigma)
        .map(|((value, observed), sigma)| ((observed - value) / sigma).powi(2))
        .sum()
}

/// Ridders' derivative of one sample's model value in one parameter.
///
/// `evaluate` returns every sample's value at a parameter vector. Probes are clamped onto
/// the declared bounds, as the class's `setFittingParams` clamps them.
fn ridders<F>(
    evaluate: &mut F,
    bounds: &[(f64, f64)],
    parameters: &[f64],
    sample: usize,
    index: usize,
) -> Result<f64>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>>,
{
    let base = parameters[index];
    let bound = bounds[index];
    let mut h = base.abs() / 1.0e3;
    if h == 0.0 {
        h = 1.0e-10;
    }
    let probe = |evaluate: &mut F, offset: f64| -> Result<f64> {
        let mut probe = parameters.to_vec();
        probe[index] = clamp(base + offset, bound);
        Ok(evaluate(&probe)?[sample])
    };
    let mut tableau = [[0.0f64; RIDDERS_NTAB]; RIDDERS_NTAB];
    let val1 = probe(evaluate, h)?;
    let val2 = probe(evaluate, -h)?;
    tableau[0][0] = (val1 - val2) / (2.0 * h);
    let mut answer = tableau[0][0];
    let mut error = RIDDERS_BIG;
    let con2 = RIDDERS_CON * RIDDERS_CON;
    for level in 1..RIDDERS_NTAB {
        h /= RIDDERS_CON;
        let val1 = probe(evaluate, h)?;
        let val2 = probe(evaluate, -h)?;
        tableau[0][level] = (val1 - val2) / (2.0 * h);
        let mut factor = con2;
        for higher in 1..=level {
            tableau[higher][level] = (tableau[higher - 1][level] * factor
                - tableau[higher - 1][level - 1])
                / (factor - 1.0);
            factor *= con2;
            let errt = (tableau[higher][level] - tableau[higher - 1][level])
                .abs()
                .max((tableau[higher][level] - tableau[higher - 1][level - 1]).abs());
            if errt <= error {
                error = errt;
                answer = tableau[higher][level];
            }
        }
        if (tableau[level][level] - tableau[level - 1][level - 1]).abs() >= RIDDERS_SAFE * error {
            break;
        }
    }
    Ok(answer)
}

/// Solve `matrix * x = rhs` by Gaussian elimination with partial pivoting.
fn solve_system(matrix: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    let n = rhs.len();
    let mut a = matrix.to_vec();
    let mut b = rhs.to_vec();
    for column in 0..n {
        let mut pivot = column;
        for row in (column + 1)..n {
            if a[row][column].abs() > a[pivot][column].abs() {
                pivot = row;
            }
        }
        if a[pivot][column] == 0.0 {
            return None;
        }
        a.swap(column, pivot);
        b.swap(column, pivot);
        for row in (column + 1)..n {
            let factor = a[row][column] / a[column][column];
            for entry in column..n {
                a[row][entry] -= factor * a[column][entry];
            }
            b[row] -= factor * b[column];
        }
    }
    let mut solution = vec![0.0; n];
    for row in (0..n).rev() {
        let mut sum = b[row];
        for column in (row + 1)..n {
            sum -= a[row][column] * solution[column];
        }
        solution[row] = sum / a[row][row];
    }
    Some(solution)
}

/// The class's Levenberg-Marquardt loop.
///
/// `evaluate` returns every sample's model value at a parameter vector. The initial guess
/// is evaluated without clamping; steps and derivative probes are clamped onto `bounds`.
///
/// # Errors
/// Propagates `evaluate`'s refusals.
pub fn levenberg_marquardt<F>(
    bounds: &[(f64, f64)],
    initial: &[f64],
    max_iterations: usize,
    observed: &[f64],
    sigma: &[f64],
    mut evaluate: F,
) -> Result<LmResult>
where
    F: FnMut(&[f64]) -> Result<Vec<f64>>,
{
    if bounds.len() != initial.len() || observed.len() != sigma.len() {
        return Err(AzothError::invalid_input(
            "regression",
            "the parameter, observation and sigma vectors disagree in length",
        ));
    }
    let mut parameters = initial.to_vec();
    let mut damping = 10.0;
    let mut iterations = 0;
    let mut reason = ConvergenceReason::MaxIterationsReached;
    loop {
        let values = evaluate(&parameters)?;
        let chi = weighted_chi(&values, observed, sigma);
        let count = parameters.len();
        let mut derivatives = vec![vec![0.0f64; count]; observed.len()];
        for sample in 0..observed.len() {
            for index in 0..count {
                derivatives[sample][index] =
                    ridders(&mut evaluate, bounds, &parameters, sample, index)?;
            }
        }
        let mut alpha = vec![vec![0.0f64; count]; count];
        let mut beta = vec![0.0f64; count];
        for first in 0..count {
            for second in 0..count {
                let mut sum = 0.0;
                for sample in 0..observed.len() {
                    sum += derivatives[sample][first] * derivatives[sample][second]
                        / sigma[sample].powi(2);
                }
                alpha[first][second] = if first == second {
                    sum * (1.0 + damping)
                } else {
                    sum
                };
            }
            let mut sum = 0.0;
            for sample in 0..observed.len() {
                sum += (observed[sample] - values[sample]) / sigma[sample].powi(2)
                    * derivatives[sample][first];
            }
            beta[first] = sum;
        }
        let gradient_norm = beta.iter().map(|value| value * value).sum::<f64>().sqrt();
        if iterations >= LM_MINIMUM_ITERATIONS && chi.abs() <= LM_CHI_SQUARE_TOLERANCE {
            reason = ConvergenceReason::ChiSquareTolerance;
            break;
        }
        if iterations >= LM_MINIMUM_ITERATIONS && gradient_norm <= LM_GRADIENT_TOLERANCE {
            reason = ConvergenceReason::GradientTolerance;
            break;
        }
        if iterations >= max_iterations && iterations >= LM_MINIMUM_ITERATIONS {
            break;
        }
        iterations += 1;
        let Some(step) = solve_system(&alpha, &beta) else {
            reason = ConvergenceReason::SingularMatrix;
            break;
        };
        let old_parameters = parameters.clone();
        let mut next: Vec<f64> = parameters
            .iter()
            .zip(&step)
            .zip(bounds)
            .map(|((value, step), bound)| clamp(value + step, *bound))
            .collect();
        let new_chi = weighted_chi(&evaluate(&next)?, observed, sigma);
        if !(new_chi < chi) || !new_chi.is_finite() {
            next = old_parameters;
            damping *= 10.0;
        } else {
            damping /= 10.0;
        }
        parameters = next;
        // The class re-initialises at the current parameters; the next iteration's top does
        // the same, so no state is carried between steps.
    }
    let values = evaluate(&parameters)?;
    let chi = weighted_chi(&values, observed, sigma);
    let count = parameters.len();
    let mut gradient = vec![0.0; count];
    for sample in 0..observed.len() {
        for index in 0..count {
            let derivative = ridders(&mut evaluate, bounds, &parameters, sample, index)?;
            gradient[index] +=
                (observed[sample] - values[sample]) / sigma[sample].powi(2) * derivative;
        }
    }
    let gradient_norm = gradient
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    Ok(LmResult {
        parameters,
        iterations,
        reason,
        chi_square: chi,
        gradient_norm,
    })
}

/// The whole fit: the optimizer, the final fluid, both ledgers and the class's simplified
/// uncertainty.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionFit {
    /// The fitted values, in the parameter list's order.
    pub values: Vec<f64>,
    /// The fluid at the fitted values.
    pub fluid: Fluid,
    /// Its four PFCT correction factors.
    pub csp: [f64; 4],
    /// The final objectives.
    pub objectives: Objectives,
    /// The final weighted chi-square.
    pub chi_square: f64,
    /// The steps taken.
    pub iterations: usize,
    /// Why the optimizer stopped.
    pub reason: ConvergenceReason,
    /// The final weighted gradient norm.
    pub gradient_norm: f64,
    /// Each parameter's simplified standard error.
    pub standard_errors: Vec<f64>,
    /// Each parameter's 95 % half-width.
    pub half_widths_95: Vec<f64>,
    /// The observation count less the parameter count, floored at one.
    pub degrees_of_freedom: usize,
    /// The final chi-square, which the class calls the residual variance.
    pub residual_variance: f64,
}

/// Run `PVTRegression`'s fit over a fluid the caller describes.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for no parameters, no observations, a non-finite bound or
///   a lower bound above its upper.
/// * Propagates the kernels' and the assembler's refusals.
pub fn fit(
    rows: &[FluidComponent],
    cubic: Cubic,
    kij: KijSource,
    corrected: bool,
    parameters: &[RegressionParameterConfig],
    data: &RegressionData,
    max_iterations: usize,
) -> Result<RegressionFit> {
    if parameters.is_empty() {
        return Err(AzothError::invalid_input(
            "parameters",
            "a regression needs at least one parameter",
        ));
    }
    for config in parameters {
        if !(config.lower.is_finite() && config.upper.is_finite() && config.initial.is_finite())
            || config.lower > config.upper
        {
            return Err(AzothError::invalid_input(
                "parameters",
                format!(
                    "{} needs finite bounds with lower <= upper and a finite guess",
                    config.parameter.name()
                ),
            ));
        }
    }
    let samples = data.samples()?;
    if samples.is_empty() {
        return Err(AzothError::invalid_input(
            "observations",
            "a regression needs at least one observation",
        ));
    }
    let observed: Vec<f64> = samples.iter().map(|sample| sample.observed).collect();
    let sigma: Vec<f64> = samples.iter().map(|sample| sample.sigma).collect();
    let bounds: Vec<(f64, f64)> = parameters
        .iter()
        .map(|config| (config.lower, config.upper))
        .collect();
    let initial: Vec<f64> = parameters.iter().map(|config| config.initial).collect();
    let result = levenberg_marquardt(
        &bounds,
        &initial,
        max_iterations,
        &observed,
        &sigma,
        |values| {
            let built = build_fluid(rows, cubic, kij, corrected, parameters, values)?;
            sample_values(&built.fluid, built.csp, &samples, data)
        },
    )?;
    let built = build_fluid(rows, cubic, kij, corrected, parameters, &result.parameters)?;
    let objectives = objectives(&built.fluid, built.csp, data)?;
    let observations: usize = data
        .cce
        .iter()
        .map(|experiment| experiment.pressures.len())
        .sum::<usize>()
        + data
            .cvd
            .iter()
            .map(|experiment| 2 * experiment.pressures.len())
            .sum::<usize>()
        + data
            .dle
            .iter()
            .map(|experiment| 3 * experiment.pressures.len())
            .sum::<usize>()
        + data.viscosity.len();
    let degrees_of_freedom = observations.saturating_sub(parameters.len()).max(1);
    let residual_variance = result.chi_square;
    let mut standard_errors = Vec::with_capacity(parameters.len());
    let mut half_widths_95 = Vec::with_capacity(parameters.len());
    for config in parameters {
        let error = (residual_variance / degrees_of_freedom as f64).sqrt()
            * (config.upper - config.lower)
            * 0.1;
        standard_errors.push(error);
        half_widths_95.push(1.96 * error);
    }
    Ok(RegressionFit {
        values: result.parameters,
        fluid: built.fluid,
        csp: built.csp,
        objectives,
        chi_square: result.chi_square,
        iterations: result.iterations,
        reason: result.reason,
        gradient_norm: result.gradient_norm,
        standard_errors,
        half_widths_95,
        degrees_of_freedom,
        residual_variance,
    })
}

/// One parameter row as the spec states it.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionParameterRow {
    /// The spec's enum spelling.
    pub parameter: String,
    /// The initial guess.
    pub initial: f64,
    /// The lower bound.
    pub lower: f64,
    /// The upper bound.
    pub upper: f64,
}

/// One CCE observation row.
#[derive(Debug, Clone, PartialEq)]
pub struct CceRow {
    /// Which experiment the row belongs to.
    pub experiment: f64,
    /// The experiment's temperature, in K.
    pub temperature: f64,
    /// The point's pressure, in Pa.
    pub pressure: f64,
    /// The measured relative volume.
    pub relative_volume: f64,
}

/// One CVD observation row.
#[derive(Debug, Clone, PartialEq)]
pub struct CvdRow {
    /// Which experiment the row belongs to.
    pub experiment: f64,
    /// The experiment's temperature, in K.
    pub temperature: f64,
    /// The point's pressure, in Pa.
    pub pressure: f64,
    /// The measured liquid volume over the saturation volume.
    pub liquid_dropout: f64,
    /// The measured gas compressibility.
    pub z_factor: f64,
}

/// One DLE observation row.
#[derive(Debug, Clone, PartialEq)]
pub struct DleRow {
    /// Which experiment the row belongs to.
    pub experiment: f64,
    /// The experiment's temperature, in K.
    pub temperature: f64,
    /// The point's pressure, in Pa.
    pub pressure: f64,
    /// The measured solution gas-oil ratio.
    pub rs: f64,
    /// The measured formation volume factor.
    pub bo: f64,
    /// The measured oil density, in kg/m3.
    pub oil_density: f64,
}

/// One viscosity observation row.
#[derive(Debug, Clone, PartialEq)]
pub struct ViscosityRow {
    /// The state's temperature, in K.
    pub temperature: f64,
    /// The state's pressure, in Pa.
    pub pressure: f64,
    /// The measured dynamic viscosity, in Pa*s.
    pub viscosity: f64,
    /// The spec's phase enum spelling.
    pub phase: String,
}

/// One fitted parameter as the id publishes it.
#[derive(Debug, Clone, PartialEq)]
pub struct FittedParameterRow {
    /// The spec's enum spelling.
    pub parameter: String,
    /// The fitted value.
    pub value: f64,
    /// The lower bound.
    pub lower: f64,
    /// The upper bound.
    pub upper: f64,
}

/// One objective as the id publishes it.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionObjectiveRow {
    /// The spec's enum spelling.
    pub experiment: String,
    /// The unweighted objective.
    pub value: f64,
}

/// The id's answer.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionResult {
    /// One row per fitted parameter, in the caller's order.
    pub fitted_parameters: Vec<FittedParameterRow>,
    /// Each parameter's simplified standard error, aligned with `fitted_parameters`.
    pub standard_errors: Vec<f64>,
    /// Each parameter's 95 % half-width, aligned with `fitted_parameters`.
    pub half_widths_95: Vec<f64>,
    /// One row per experiment that had observations, in `ExperimentType` declaration order.
    pub objectives: Vec<RegressionObjectiveRow>,
    /// The final weighted chi-square.
    pub final_chi_square: f64,
    /// The objectives' sum.
    pub total_objective: f64,
    /// The observation count less the parameter count, floored at one.
    pub degrees_of_freedom: f64,
    /// The class's residual variance, which is the chi-square itself.
    pub residual_variance: f64,
    /// `sqrt(chi-square) * 100`.
    pub average_absolute_deviation_percent: f64,
    /// The steps taken.
    pub iterations: f64,
    /// Why the optimizer stopped.
    pub convergence_reason: ConvergenceReason,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

fn experiment_index(value: f64, label: &str) -> Result<usize> {
    if value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= 1.0e9 {
        return Ok(value as usize);
    }
    Err(AzothError::invalid_input(
        label,
        format!("an experiment index is a whole number from zero; got {value}"),
    ))
}

/// Row indices grouped by experiment, in first-appearance order, refusing a gap.
fn groups<T>(
    rows: &[T],
    index: impl Fn(&T) -> f64,
    label: &str,
) -> Result<Vec<(usize, Vec<usize>)>> {
    let mut out: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut closed = std::collections::HashSet::new();
    for (position, row) in rows.iter().enumerate() {
        let experiment = experiment_index(index(row), label)?;
        match out.last_mut() {
            Some((held, members)) if *held == experiment => members.push(position),
            _ => {
                if !closed.insert(experiment) {
                    return Err(AzothError::invalid_input(
                        label,
                        format!("experiment {experiment} is not contiguous"),
                    ));
                }
                out.push((experiment, vec![position]));
            }
        }
    }
    Ok(out)
}

fn same_temperature(values: &[f64], label: &str) -> Result<f64> {
    let first = values.first().copied().unwrap_or(0.0);
    if values.iter().any(|value| (value - first).abs() > 1e-9) {
        return Err(AzothError::invalid_input(
            label,
            "one experiment carries more than one temperature",
        ));
    }
    Ok(first)
}

/// `PVTRegression` as the id exposes it: rows in, the fit and its two ledgers out.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown parameter or phase name, a malformed or
///   non-contiguous experiment index, or an experiment with no rows.
/// * Propagates [`fit`]'s refusals.
#[allow(clippy::too_many_arguments)] // The signature is the spec's declared inputs.
pub fn regression(
    components: &[FluidComponent],
    parameters: &[RegressionParameterRow],
    cce: Option<&[CceRow]>,
    cvd: Option<&[CvdRow]>,
    dle: Option<&[DleRow]>,
    viscosity: Option<&[ViscosityRow]>,
    cce_weight: Option<f64>,
    cvd_weight: Option<f64>,
    dle_weight: Option<f64>,
    viscosity_weight: Option<f64>,
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    volume_correction: Option<bool>,
    max_iterations: Option<u32>,
) -> Result<RegressionResult> {
    let mut configs = Vec::with_capacity(parameters.len());
    for row in parameters {
        let parameter = RegressionParameter::from_name(&row.parameter).ok_or_else(|| {
            AzothError::invalid_input(
                "parameters",
                format!(
                    "{:?} is not one of the twelve carried parameters",
                    row.parameter
                ),
            )
        })?;
        configs.push(RegressionParameterConfig {
            parameter,
            initial: row.initial,
            lower: row.lower,
            upper: row.upper,
        });
    }
    let mut data = RegressionData {
        cce: Vec::new(),
        cvd: Vec::new(),
        dle: Vec::new(),
        viscosity: Vec::new(),
        weights: ExperimentWeights {
            cce: cce_weight.unwrap_or(1.0),
            cvd: cvd_weight.unwrap_or(1.0),
            dle: dle_weight.unwrap_or(1.0),
            viscosity: viscosity_weight.unwrap_or(0.5),
        },
    };
    if let Some(rows) = cce {
        for (_, members) in groups(rows, |row| row.experiment, "cce")? {
            let temperature = same_temperature(
                &members
                    .iter()
                    .map(|&at| rows[at].temperature)
                    .collect::<Vec<_>>(),
                "cce",
            )?;
            data.cce.push(CceExperiment {
                temperature,
                pressures: members.iter().map(|&at| rows[at].pressure).collect(),
                relative_volumes: members.iter().map(|&at| rows[at].relative_volume).collect(),
            });
        }
    }
    if let Some(rows) = cvd {
        for (_, members) in groups(rows, |row| row.experiment, "cvd")? {
            let temperature = same_temperature(
                &members
                    .iter()
                    .map(|&at| rows[at].temperature)
                    .collect::<Vec<_>>(),
                "cvd",
            )?;
            data.cvd.push(CvdExperiment {
                temperature,
                pressures: members.iter().map(|&at| rows[at].pressure).collect(),
                liquid_dropout: members.iter().map(|&at| rows[at].liquid_dropout).collect(),
                z_factor: members.iter().map(|&at| rows[at].z_factor).collect(),
            });
        }
    }
    if let Some(rows) = dle {
        for (_, members) in groups(rows, |row| row.experiment, "dle")? {
            let temperature = same_temperature(
                &members
                    .iter()
                    .map(|&at| rows[at].temperature)
                    .collect::<Vec<_>>(),
                "dle",
            )?;
            data.dle.push(DleExperiment {
                temperature,
                pressures: members.iter().map(|&at| rows[at].pressure).collect(),
                rs: members.iter().map(|&at| rows[at].rs).collect(),
                bo: members.iter().map(|&at| rows[at].bo).collect(),
                oil_density: members.iter().map(|&at| rows[at].oil_density).collect(),
            });
        }
    }
    if let Some(rows) = viscosity {
        for row in rows {
            let phase = ViscosityPhase::from_name(&row.phase).ok_or_else(|| {
                AzothError::invalid_input(
                    "viscosity",
                    format!("{:?} is not a phase the class names", row.phase),
                )
            })?;
            data.viscosity.push(ViscosityPoint {
                temperature: row.temperature,
                pressure: row.pressure,
                viscosity: row.viscosity,
                phase,
            });
        }
    }
    let result = fit(
        components,
        crate::fluid::cubic_named(eos)?,
        crate::fluid::kij_named(mixing_rule)?,
        volume_correction.unwrap_or(true),
        &configs,
        &data,
        max_iterations.map_or(LM_MAXIMUM_ITERATIONS, |value| value as usize),
    )?;
    let mut fitted_parameters = Vec::with_capacity(configs.len());
    for (index, config) in configs.iter().enumerate() {
        fitted_parameters.push(FittedParameterRow {
            parameter: config.parameter.name().to_owned(),
            value: result.values[index],
            lower: config.lower,
            upper: config.upper,
        });
    }
    let mut objective_rows = Vec::new();
    for (experiment, value) in [
        ("cce", result.objectives.cce),
        ("cvd", result.objectives.cvd),
        ("dle", result.objectives.dle),
        ("viscosity", result.objectives.viscosity),
    ] {
        if let Some(value) = value {
            objective_rows.push(RegressionObjectiveRow {
                experiment: experiment.to_owned(),
                value,
            });
        }
    }
    Ok(RegressionResult {
        fitted_parameters,
        standard_errors: result.standard_errors,
        half_widths_95: result.half_widths_95,
        objectives: objective_rows,
        final_chi_square: result.chi_square,
        total_objective: result.objectives.total(),
        degrees_of_freedom: result.degrees_of_freedom as f64,
        residual_variance: result.residual_variance,
        average_absolute_deviation_percent: result.chi_square.sqrt() * 100.0,
        iterations: result.iterations as f64,
        convergence_reason: result.reason,
        warnings: Vec::new(),
    })
}
