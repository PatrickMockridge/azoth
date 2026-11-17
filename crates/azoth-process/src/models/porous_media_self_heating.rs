//! `process.porous_media_self_heating` - the Frank-Kamenetskii criticality screen.
//!
//! Spec: `specs/models/process/porous_media_self_heating.toml`. The arithmetic is NeqSim's
//! `process/safety/selfheating/PorousMediaSelfHeatingAnalyzer.java`'s `analyze`: the
//! criticality parameter against the shape's critical value, the critical boundary
//! temperature, the critical half-dimension and the verdict.

use azoth_core::units::{
    Length, MolarEnergy, TemperatureInterval, ThermalConductivity, ThermodynamicTemperature,
    kelvin_intervals, kelvins, meters,
};
use azoth_core::{Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::selfheating::{
    BRACKET_WARNING, DEFAULT_MARGINAL_RATIO, FK_WARNING, PIPE_INSULATION_WARNING, R_GAS,
    SelfHeatingGeometry, SelfHeatingVerdict, critical_temperature, fk_critical_dimension, fk_delta,
    verdict_of,
};

/// Result of `process.porous_media_self_heating`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PorousMediaSelfHeatingResult {
    /// The body shape the screen used.
    pub geometry: SelfHeatingGeometry,
    /// The characteristic half-dimension.
    #[serde(serialize_with = "scalar")]
    pub characteristic_dimension: Length,
    /// The effective thermal conductivity.
    #[serde(serialize_with = "scalar")]
    pub effective_conductivity: ThermalConductivity,
    /// The apparent activation energy.
    #[serde(serialize_with = "scalar")]
    pub activation_energy: MolarEnergy,
    /// The volumetric heat-release pre-factor, in W/m3.
    pub volumetric_pre_factor: f64,
    /// The boundary temperature the body is held at.
    #[serde(serialize_with = "scalar")]
    pub boundary_temperature: ThermodynamicTemperature,
    /// The Frank-Kamenetskii criticality parameter.
    pub delta: f64,
    /// The shape's critical parameter.
    pub delta_crit: f64,
    /// `delta / delta_crit`.
    pub delta_ratio: f64,
    /// The boundary temperature at which `delta` reaches its critical value.
    #[serde(serialize_with = "optional_scalar")]
    pub critical_temperature: Option<ThermodynamicTemperature>,
    /// The half-dimension at which `delta` reaches its critical value.
    #[serde(serialize_with = "scalar")]
    pub critical_dimension: Length,
    /// `critical_temperature - boundary_temperature`.
    #[serde(serialize_with = "optional_scalar")]
    pub temperature_margin: Option<TemperatureInterval>,
    /// `critical_dimension - characteristic_dimension`.
    #[serde(serialize_with = "scalar")]
    pub dimension_margin: Length,
    /// The Frank-Kamenetskii temperature scale, `R Tb^2/E`.
    #[serde(serialize_with = "scalar")]
    pub fk_temperature_scale: TemperatureInterval,
    /// The screening verdict.
    pub verdict: SelfHeatingVerdict,
    /// The class's warning strings, in order.
    pub messages: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The Frank-Kamenetskii criticality screen for a porous body.
///
/// # Errors
/// `InvalidInput` for an unknown geometry, a non-positive dimension, conductivity,
/// activation energy, pre-factor or boundary temperature, or a marginal ratio outside
/// (0, 1).
#[allow(clippy::too_many_arguments)] // one argument per declared input
pub fn porous_media_self_heating(
    geometry: &str,
    characteristic_dimension: Length,
    effective_conductivity: ThermalConductivity,
    activation_energy: MolarEnergy,
    volumetric_pre_factor: f64,
    boundary_temperature: ThermodynamicTemperature,
    marginal_ratio: Option<f64>,
    pipe_insulation_screening: Option<bool>,
) -> Result<PorousMediaSelfHeatingResult> {
    let spec = &model_gen::POROUS_MEDIA_SELF_HEATING_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "characteristic_dimension" => Some(characteristic_dimension.value),
            "effective_conductivity" => Some(effective_conductivity.value),
            "activation_energy" => Some(activation_energy.value),
            "volumetric_pre_factor" => Some(volumetric_pre_factor),
            "boundary_temperature" => Some(boundary_temperature.value),
            "marginal_ratio" => marginal_ratio,
            _ => None,
        },
        &mut warnings,
    )?;

    let insulation = pipe_insulation_screening.unwrap_or(false);
    let shape = SelfHeatingGeometry::parse(geometry)?;
    let shape = if insulation {
        SelfHeatingGeometry::Slab
    } else {
        shape
    };
    let marginal = marginal_ratio.unwrap_or(DEFAULT_MARGINAL_RATIO);
    let energy = activation_energy.value;
    let boundary = boundary_temperature.value;
    let dimension = characteristic_dimension.value;
    let conductivity = effective_conductivity.value;
    let delta_crit = shape.delta_crit();
    let delta = fk_delta(
        energy,
        volumetric_pre_factor,
        dimension,
        conductivity,
        boundary,
    );
    let delta_ratio = delta / delta_crit;
    let critical = critical_temperature(|temperature| {
        fk_delta(
            energy,
            volumetric_pre_factor,
            dimension,
            conductivity,
            temperature,
        ) - delta_crit
    });
    let critical_dimension = fk_critical_dimension(
        energy,
        volumetric_pre_factor,
        conductivity,
        boundary,
        delta_crit,
    );

    let mut messages = Vec::new();
    if insulation {
        messages.push(PIPE_INSULATION_WARNING.to_string());
    }
    if critical.is_none() {
        messages.push(BRACKET_WARNING.to_string());
    }
    messages.push(FK_WARNING.to_string());

    Ok(PorousMediaSelfHeatingResult {
        geometry: shape,
        characteristic_dimension,
        effective_conductivity,
        activation_energy,
        volumetric_pre_factor,
        boundary_temperature,
        delta,
        delta_crit,
        delta_ratio,
        critical_temperature: critical.map(kelvins),
        critical_dimension: meters(critical_dimension),
        temperature_margin: critical.map(|temperature| kelvin_intervals(temperature - boundary)),
        dimension_margin: meters(critical_dimension - dimension),
        fk_temperature_scale: kelvin_intervals(R_GAS * boundary * boundary / energy),
        verdict: verdict_of(delta_ratio, marginal),
        messages,
        warnings,
    })
}
