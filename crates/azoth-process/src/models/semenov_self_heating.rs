//! `process.semenov_self_heating` - the Semenov criticality screen.
//!
//! Spec: `specs/models/process/semenov_self_heating.toml`. The arithmetic is NeqSim's
//! `process/safety/selfheating/SemenovSelfHeatingAnalyzer.java`'s `analyze`: the Semenov
//! parameter against `1/e`, the critical-temperature bisection and the verdict.

use azoth_core::units::{
    Area, HeatTransfer, MolarEnergy, TemperatureInterval, ThermodynamicTemperature, Volume,
    kelvin_intervals, kelvins,
};
use azoth_core::{Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::selfheating::{
    BRACKET_WARNING, DEFAULT_MARGINAL_RATIO, PSI_CRIT, R_GAS, SEMENOV_WARNING, SelfHeatingVerdict,
    critical_temperature, semenov_psi, verdict_of,
};

/// Result of `process.semenov_self_heating`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SemenovSelfHeatingResult {
    /// The Semenov criticality parameter.
    pub psi: f64,
    /// The critical parameter, `1/e`.
    pub psi_crit: f64,
    /// `psi / psi_crit`.
    pub psi_ratio: f64,
    /// The ambient temperature at which `psi` reaches its critical value.
    #[serde(serialize_with = "optional_scalar")]
    pub critical_temperature: Option<ThermodynamicTemperature>,
    /// `critical_temperature - ambient_temperature`.
    #[serde(serialize_with = "optional_scalar")]
    pub temperature_margin: Option<TemperatureInterval>,
    /// The steady self-heating excess at criticality, `R Ta^2/E`.
    #[serde(serialize_with = "scalar")]
    pub critical_temperature_rise: TemperatureInterval,
    /// The screening verdict.
    pub verdict: SelfHeatingVerdict,
    /// The class's warning strings, in order.
    pub messages: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The Semenov criticality screen for a thermally thin body.
///
/// # Errors
/// `InvalidInput` for a non-positive volume, surface area, heat-transfer coefficient,
/// activation energy, pre-factor or ambient temperature, or a marginal ratio outside (0, 1).
pub fn semenov_self_heating(
    volume: Volume,
    surface_area: Area,
    heat_transfer_coefficient: HeatTransfer,
    activation_energy: MolarEnergy,
    volumetric_pre_factor: f64,
    ambient_temperature: ThermodynamicTemperature,
    marginal_ratio: Option<f64>,
) -> Result<SemenovSelfHeatingResult> {
    let spec = &model_gen::SEMENOV_SELF_HEATING_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "volume" => Some(volume.value),
            "surface_area" => Some(surface_area.value),
            "heat_transfer_coefficient" => Some(heat_transfer_coefficient.value),
            "activation_energy" => Some(activation_energy.value),
            "volumetric_pre_factor" => Some(volumetric_pre_factor),
            "ambient_temperature" => Some(ambient_temperature.value),
            "marginal_ratio" => marginal_ratio,
            _ => None,
        },
        &mut warnings,
    )?;

    let marginal = marginal_ratio.unwrap_or(DEFAULT_MARGINAL_RATIO);
    let energy = activation_energy.value;
    let ambient = ambient_temperature.value;
    let psi = semenov_psi(
        energy,
        volume.value,
        volumetric_pre_factor,
        heat_transfer_coefficient.value,
        surface_area.value,
        ambient,
    );
    let psi_ratio = psi / PSI_CRIT;
    let critical = critical_temperature(|temperature| {
        semenov_psi(
            energy,
            volume.value,
            volumetric_pre_factor,
            heat_transfer_coefficient.value,
            surface_area.value,
            temperature,
        ) - PSI_CRIT
    });

    let mut messages = Vec::new();
    if critical.is_none() {
        messages.push(BRACKET_WARNING.to_string());
    }
    messages.push(SEMENOV_WARNING.to_string());

    Ok(SemenovSelfHeatingResult {
        psi,
        psi_crit: PSI_CRIT,
        psi_ratio,
        critical_temperature: critical.map(kelvins),
        temperature_margin: critical.map(|temperature| kelvin_intervals(temperature - ambient)),
        critical_temperature_rise: kelvin_intervals(R_GAS * ambient * ambient / energy),
        verdict: verdict_of(psi_ratio, marginal),
        messages,
        warnings,
    })
}
