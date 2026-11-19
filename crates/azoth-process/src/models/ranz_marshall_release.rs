//! `process.ranz_marshall_release` - NeqSim's Ranz-Marshall release adapter.
//!
//! Spec: `specs/models/process/ranz_marshall_release.toml`. The port source is
//! `process/safety/release/RanzMarshallFiniteRateReleaseModel.java`, through its `calculate`.
//!
//! The external-film correlation supplies one relaxation time per component, and the
//! component-selective model does the rest: `Sh_i = 2 + 0.6 sqrt(Re) cbrt(Sc_i)`,
//! `k_i = Sh_i D_i / d32`, `tau_i = d32 / (6 k_i)`.

use azoth_core::units::{
    Length, MassDensity, MassFlux, MassRate, MolarEnergy, MolarHeatCapacity, Pressure,
    ThermodynamicTemperature, Velocity,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::component_selective_release::{
    ComponentSelectiveReleaseStation, component_selective_release,
};
use crate::models::hem_release::{HemReleaseDiagnostic, ReleasePhase, ReleaseStation};

/// The correlation's Reynolds bound, exclusive.
const MAX_REYNOLDS: f64 = 200.0;
/// The correlation's Schmidt bound, exclusive.
const MAX_SCHMIDT: f64 = 250.0;

/// One row of `stations`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RanzMarshallReleaseStation {
    /// Which station this row is.
    pub station: ReleaseStation,
    /// Absolute pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// Temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// The relaxed harmonic mixture density at the opening.
    #[serde(serialize_with = "scalar")]
    pub density: MassDensity,
    /// Molar enthalpy.
    #[serde(serialize_with = "scalar")]
    pub molar_enthalpy: MolarEnergy,
    /// Molar entropy.
    #[serde(serialize_with = "scalar")]
    pub molar_entropy: MolarHeatCapacity,
    /// The station's bulk velocity.
    #[serde(serialize_with = "scalar")]
    pub velocity: Velocity,
    /// The station's mass flux, before the discharge coefficient.
    #[serde(serialize_with = "scalar")]
    pub mass_flux: MassFlux,
    /// What the base flash found.
    pub phase: ReleasePhase,
    /// The relaxed gas phase's share of the mass.
    pub gas_mass_fraction: f64,
    /// The gas phase's own density.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_density: Option<MassDensity>,
    /// The liquid phase's own density.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_density: Option<MassDensity>,
    /// The solved gas velocity.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_velocity: Option<Velocity>,
    /// The solved liquid velocity.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_velocity: Option<Velocity>,
}

/// Result of `process.ranz_marshall_release`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RanzMarshallReleaseResult {
    /// `valid`, `invalid` or `unsupported`.
    pub status: String,
    /// The delegate's release rate; absent when the result is not usable.
    #[serde(serialize_with = "optional_scalar")]
    pub mass_flow_rate: Option<MassRate>,
    /// The base scan's choked flag.
    pub choked: bool,
    /// Absent: the derived model drops the base's throat sound speed.
    #[serde(serialize_with = "optional_scalar")]
    pub throat_sound_speed: Option<Velocity>,
    /// The upstream overall mole fractions.
    pub component_mole_fractions: Vec<f64>,
    /// The upstream overall mass fractions.
    pub component_mass_fractions: Vec<f64>,
    /// The delegate's stations.
    pub stations: Vec<RanzMarshallReleaseStation>,
    /// The model's assumptions and the delegate's retained records.
    pub diagnostics: Vec<HemReleaseDiagnostic>,
    /// The diagnostics' machine codes alone.
    pub diagnostic_codes: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// NeqSim's Ranz-Marshall release adapter.
///
/// # Errors
/// A non-positive or empty input, a composition that is not one entry per component, or a
/// mixture the databank cannot resolve.
#[allow(clippy::too_many_arguments)] // one argument per spec input
pub fn ranz_marshall_release(
    components: &[String],
    upstream_z: &[f64],
    upstream_t: ThermodynamicTemperature,
    upstream_p: Pressure,
    diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
    surface_tension: f64,
    residence_time: f64,
    morphology: &str,
    sauter_mean_diameter: f64,
    continuous_phase_density: f64,
    continuous_phase_dynamic_viscosity: f64,
    relative_velocity: f64,
    component_diffusivities: &[f64],
    parameter_provenance: &str,
) -> Result<RanzMarshallReleaseResult> {
    let spec = &model_gen::RANZ_MARSHALL_RELEASE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "upstream_t" => Some(upstream_t.value),
            "upstream_p" => Some(upstream_p.value),
            "diameter" => Some(diameter.value),
            "discharge_coefficient" => Some(discharge_coefficient),
            "back_pressure" => Some(back_pressure.value),
            "surface_tension" => Some(surface_tension),
            "residence_time" => Some(residence_time),
            "sauter_mean_diameter" => Some(sauter_mean_diameter),
            "continuous_phase_density" => Some(continuous_phase_density),
            "continuous_phase_dynamic_viscosity" => Some(continuous_phase_dynamic_viscosity),
            "relative_velocity" => Some(relative_velocity),
            _ => None,
        },
        &mut warnings,
    )?;
    if !surface_tension.is_finite() || surface_tension <= 0.0 {
        return Err(AzothError::invalid_input(
            "surface_tension",
            format!(
                "the surface tension must be finite and positive, and {surface_tension} is not"
            ),
        ));
    }
    if !residence_time.is_finite() || residence_time <= 0.0 {
        return Err(AzothError::invalid_input(
            "residence_time",
            format!("the residence time must be finite and positive, and {residence_time} is not"),
        ));
    }
    if !sauter_mean_diameter.is_finite() || sauter_mean_diameter <= 0.0 {
        return Err(AzothError::invalid_input(
            "sauter_mean_diameter",
            "the Sauter mean diameter must be finite and positive".to_string(),
        ));
    }
    if !continuous_phase_density.is_finite() || continuous_phase_density <= 0.0 {
        return Err(AzothError::invalid_input(
            "continuous_phase_density",
            "the continuous-phase density must be finite and positive".to_string(),
        ));
    }
    if !continuous_phase_dynamic_viscosity.is_finite() || continuous_phase_dynamic_viscosity <= 0.0
    {
        return Err(AzothError::invalid_input(
            "continuous_phase_dynamic_viscosity",
            "the continuous-phase viscosity must be finite and positive".to_string(),
        ));
    }
    if !relative_velocity.is_finite() || relative_velocity < 0.0 {
        return Err(AzothError::invalid_input(
            "relative_velocity",
            "the relative velocity must be finite and non-negative".to_string(),
        ));
    }
    if parameter_provenance.trim().is_empty() {
        return Err(AzothError::invalid_input(
            "parameter_provenance",
            "the parameter provenance must not be empty".to_string(),
        ));
    }

    if component_diffusivities.len() != components.len() {
        return Ok(failure(
            "invalid",
            "RANZ_MARSHALL_COMPONENT_BASIS_INVALID",
            format!(
                "Diffusivity components must exactly match the release-fluid components; \
                 fluid has {} components and the diffusivities {}",
                components.len(),
                component_diffusivities.len()
            ),
            warnings,
        ));
    }
    if component_diffusivities
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(AzothError::invalid_input(
            "component_diffusivities",
            "every component diffusivity must be finite and positive".to_string(),
        ));
    }

    let reynolds = continuous_phase_density * relative_velocity * sauter_mean_diameter
        / continuous_phase_dynamic_viscosity;
    if !reynolds.is_finite() || reynolds < 0.0 || reynolds >= MAX_REYNOLDS {
        return Ok(failure(
            "unsupported",
            "RANZ_MARSHALL_RANGE_UNSUPPORTED",
            format!("Reynolds number {reynolds:.6} is outside 0 <= Re < 200"),
            warnings,
        ));
    }
    let mut times = Vec::with_capacity(component_diffusivities.len());
    for (index, diffusivity) in component_diffusivities.iter().enumerate() {
        let schmidt = continuous_phase_dynamic_viscosity / (continuous_phase_density * diffusivity);
        if !schmidt.is_finite() || schmidt <= 0.0 || schmidt >= MAX_SCHMIDT {
            return Ok(failure(
                "unsupported",
                "RANZ_MARSHALL_RANGE_UNSUPPORTED",
                format!(
                    "Schmidt number for component {index} is {schmidt:.6} and outside 0 < Sc < \
                     250"
                ),
                warnings,
            ));
        }
        let sherwood = 2.0 + 0.6 * reynolds.sqrt() * schmidt.cbrt();
        let coefficient = sherwood * diffusivity / sauter_mean_diameter;
        times.push(sauter_mean_diameter / (6.0 * coefficient));
    }
    if morphology == "liquid_droplets" {
        return Ok(failure(
            "unsupported",
            "LIQUID_DROPLET_HYDRODYNAMICS_UNSUPPORTED",
            "Ranz-Marshall droplet coefficients are resolved, but this release adapter only \
             has a gas-bubble-in-liquid drift-flux closure"
                .to_string(),
            warnings,
        ));
    }
    if morphology != "gas_bubbles" {
        return Err(AzothError::invalid_input(
            "morphology",
            format!("unknown morphology {morphology:?}"),
        ));
    }

    let delegate = component_selective_release(
        components,
        upstream_z,
        upstream_t,
        upstream_p,
        diameter,
        discharge_coefficient,
        back_pressure,
        surface_tension,
        &times,
        residence_time,
        parameter_provenance,
    )?;
    if delegate.status != "valid" {
        let code = delegate
            .diagnostic_codes
            .first()
            .map_or("UNKNOWN", String::as_str);
        let message = delegate
            .diagnostics
            .first()
            .map_or(String::new(), |row| row.message.clone());
        let status = delegate.status.clone();
        return Ok(failure(
            &status,
            &format!("RANZ_MARSHALL_{code}"),
            message,
            delegate.warnings,
        ));
    }
    let mut diagnostics = vec![HemReleaseDiagnostic {
        code: "MODEL_ASSUMPTIONS".to_string(),
        message: "Ranz-Marshall external-film component transfer for explicit spherical gas \
                  bubbles; equilibrium caloric properties and bounded vertical drift flux; no \
                  interfacial heat or latent kinetics, internal resistance, Stefan-flow \
                  correction, breakup, coalescence or solids"
            .to_string(),
    }];
    diagnostics.push(HemReleaseDiagnostic {
        code: "RANZ_MARSHALL_COMPONENT_TRANSFER".to_string(),
        message: "External-film coefficients from the Reynolds and Schmidt numbers; no \
                  predicted residence time"
            .to_string(),
    });
    for row in &delegate.diagnostics {
        if row.code != "MODEL_ASSUMPTIONS" && row.code != "COMPONENT_SELECTIVE_PHASE_TRANSFER" {
            diagnostics.push(row.clone());
        }
    }
    Ok(RanzMarshallReleaseResult {
        status: "valid".to_string(),
        mass_flow_rate: delegate.mass_flow_rate,
        choked: delegate.choked,
        throat_sound_speed: None,
        component_mole_fractions: delegate.component_mole_fractions,
        component_mass_fractions: delegate.component_mass_fractions,
        stations: delegate.stations.iter().map(station).collect(),
        diagnostic_codes: diagnostics.iter().map(|row| row.code.clone()).collect(),
        diagnostics,
        warnings: delegate.warnings,
    })
}

/// One delegate station as this model's row.
fn station(row: &ComponentSelectiveReleaseStation) -> RanzMarshallReleaseStation {
    RanzMarshallReleaseStation {
        station: row.station,
        pressure: row.pressure,
        temperature: row.temperature,
        density: row.density,
        molar_enthalpy: row.molar_enthalpy,
        molar_entropy: row.molar_entropy,
        velocity: row.velocity,
        mass_flux: row.mass_flux,
        phase: row.phase,
        gas_mass_fraction: row.gas_mass_fraction,
        gas_density: row.gas_density,
        liquid_density: row.liquid_density,
        gas_velocity: row.gas_velocity,
        liquid_velocity: row.liquid_velocity,
    }
}

/// The class's failure result: no rate, no stations, one diagnostic.
fn failure(
    status: &str,
    code: &str,
    message: String,
    warnings: Vec<Warning>,
) -> RanzMarshallReleaseResult {
    RanzMarshallReleaseResult {
        status: status.to_string(),
        mass_flow_rate: None,
        choked: false,
        throat_sound_speed: None,
        component_mole_fractions: Vec::new(),
        component_mass_fractions: Vec::new(),
        stations: Vec::new(),
        diagnostics: vec![HemReleaseDiagnostic {
            code: code.to_string(),
            message,
        }],
        diagnostic_codes: vec![code.to_string()],
        warnings,
    }
}
