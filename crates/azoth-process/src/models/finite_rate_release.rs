//! `process.finite_rate_release` - NeqSim's finite-rate drift-flux orifice release.
//!
//! Spec: `specs/models/process/finite_rate_release.toml`. The port source is
//! `process/safety/release/FiniteRateDriftFluxReleaseModel.java`, through its `calculate`.
//!
//! The homogeneous-equilibrium scan supplies the stations; the opening's phase split relaxes
//! from the upstream value toward equilibrium by a first-order progress, and the shared
//! drift-flux closure solves the relaxed station's velocities.

use azoth_core::units::{
    Length, MassDensity, MassFlux, MassRate, MolarEnergy, MolarHeatCapacity, Pressure,
    ThermodynamicTemperature, Velocity, joules_per_mole, joules_per_mole_kelvin, kelvins,
    kilograms_per_cubic_meter, kilograms_per_second, kilograms_per_square_meter_second,
    meters_per_second, pascals,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::hem_release::{
    HemReleaseDiagnostic, ReleasePhase, ReleaseStation, TraceStation, trace,
};
use crate::models::release::{ReleaseRefusal, drift_flux, relax_uniform};

/// One row of `stations`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FiniteRateReleaseStation {
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

/// Result of `process.finite_rate_release`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FiniteRateReleaseResult {
    /// `valid`, `invalid` or `unsupported`.
    pub status: String,
    /// The closure's release rate; absent when the result is not usable.
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
    /// The base stations with the opening relaxed and solved.
    pub stations: Vec<FiniteRateReleaseStation>,
    /// The model's assumptions and the base's retained records.
    pub diagnostics: Vec<HemReleaseDiagnostic>,
    /// The diagnostics' machine codes alone.
    pub diagnostic_codes: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// NeqSim's finite-rate drift-flux orifice release.
///
/// # Errors
/// A non-positive input, an empty provenance, a composition that is not one entry per
/// component, or a mixture the databank cannot resolve.
#[allow(clippy::too_many_arguments)] // one argument per spec input
pub fn finite_rate_release(
    components: &[String],
    upstream_z: &[f64],
    upstream_t: ThermodynamicTemperature,
    upstream_p: Pressure,
    diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
    surface_tension: f64,
    phase_transfer_relaxation_time: f64,
    residence_time: f64,
    parameter_provenance: &str,
) -> Result<FiniteRateReleaseResult> {
    let spec = &model_gen::FINITE_RATE_RELEASE_SPEC;
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
            "phase_transfer_relaxation_time" => Some(phase_transfer_relaxation_time),
            "residence_time" => Some(residence_time),
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
    if !phase_transfer_relaxation_time.is_finite() || phase_transfer_relaxation_time <= 0.0 {
        return Err(AzothError::invalid_input(
            "phase_transfer_relaxation_time",
            format!(
                "the relaxation time must be finite and positive, and \
                 {phase_transfer_relaxation_time} is not"
            ),
        ));
    }
    if !residence_time.is_finite() || residence_time <= 0.0 {
        return Err(AzothError::invalid_input(
            "residence_time",
            format!("the residence time must be finite and positive, and {residence_time} is not"),
        ));
    }
    if parameter_provenance.trim().is_empty() {
        return Err(AzothError::invalid_input(
            "parameter_provenance",
            "the parameter provenance must not be empty".to_string(),
        ));
    }

    let base = trace(
        components,
        upstream_z,
        upstream_t,
        upstream_p,
        diameter,
        discharge_coefficient,
        back_pressure,
    )?;
    if base.status != "valid" && base.status != "valid_with_warnings" {
        let cause = base
            .diagnostics
            .first()
            .map_or(("UNKNOWN", String::new()), |row| {
                (row.code.as_str(), row.message.clone())
            });
        return Ok(failure(
            &base.status,
            &format!("EQUILIBRIUM_BASE_{}", cause.0),
            cause.1,
            base.warnings,
        ));
    }
    let mut diagnostics = vec![HemReleaseDiagnostic {
        code: "MODEL_ASSUMPTIONS".to_string(),
        message: "Uniform first-order gas/liquid phase-split relaxation with caller-declared \
                  times and provenance; equilibrium caloric properties and bounded vertical \
                  drift flux; no component-selective kinetics, predictive residence time, \
                  entrainment, annular jet, pipe friction, heat transfer or solids"
            .to_string(),
    }];
    for row in &base.diagnostics {
        if row.code != "MODEL_ASSUMPTIONS"
            && row.code != "THROAT_MACH_MISMATCH"
            && row.code != "ACOUSTIC_UNAVAILABLE"
        {
            diagnostics.push(row.clone());
        }
    }
    let area = discharge_coefficient * std::f64::consts::PI * diameter.value * diameter.value / 4.0;
    if base.mass_flow == Some(0.0) {
        diagnostics.push(HemReleaseDiagnostic {
            code: "FINITE_RATE_NOT_APPLIED".to_string(),
            message: "No forward flow; phase transfer is not resolved".to_string(),
        });
        return Ok(success(
            Some(0.0),
            false,
            &base.component_mole_fractions,
            base.component_mass_fractions,
            base.stations.iter().map(row).collect(),
            diagnostics,
            base.warnings,
        ));
    }

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, _) = azoth_eos::databank::mixture_of(&names, azoth_eos::cubic::Cubic::Pr, None)?;
    let molar_mass = azoth_eos::entropy_state::molar_mass_of(&mixture, upstream_z)?;
    let upstream = &base.stations[0];
    if upstream.liquid_phase.is_none() {
        return Ok(refused(
            ReleaseRefusal::Unsupported("Liquid phase is required".to_string()),
            base.warnings,
        ));
    }
    let progress = 1.0 - (-residence_time / phase_transfer_relaxation_time).exp();
    let equilibrium = &base.stations[1];
    let relaxed = match relax_uniform(upstream.gas_mass_fraction, equilibrium, progress) {
        Ok(state) => state,
        Err(refusal) => return Ok(refused(refusal, base.warnings)),
    };
    let outcome = match drift_flux(&relaxed, surface_tension) {
        Ok(outcome) => outcome,
        Err(refusal) => return Ok(refused(refusal, base.warnings)),
    };
    let kinetic_energy = 0.5
        * (relaxed.gas_mass_fraction * outcome.gas_velocity * outcome.gas_velocity
            + (1.0 - relaxed.gas_mass_fraction)
                * outcome.liquid_velocity
                * outcome.liquid_velocity);
    let upstream_enthalpy = upstream.molar_enthalpy;
    let energy_error =
        (outcome.state.molar_enthalpy - upstream_enthalpy) / molar_mass + kinetic_energy;
    if energy_error.abs() > 1.0e-6 {
        return Ok(refused(
            ReleaseRefusal::Invalid("Component or stagnation-energy closure failed".to_string()),
            base.warnings,
        ));
    }
    diagnostics.push(HemReleaseDiagnostic {
        code: "FINITE_RATE_PHASE_TRANSFER".to_string(),
        message: "First-order phase-split relaxation; no component-selective kinetics".to_string(),
    });
    diagnostics.push(HemReleaseDiagnostic {
        code: "FINITE_RATE_CONSERVATION".to_string(),
        message: "Component and stagnation-energy closures hold at the opening".to_string(),
    });
    diagnostics.push(HemReleaseDiagnostic {
        code: "DRIFT_FLUX_CLOSURE".to_string(),
        message: "Phase-area, drift-flux and kinetic-energy closures hold at the opening"
            .to_string(),
    });
    let mut stations = base.stations.clone();
    let mut throat = outcome.state.clone();
    throat.station = ReleaseStation::ThroatCritical;
    let mut exit = outcome.state.clone();
    exit.station = ReleaseStation::OrificeExit;
    stations[1] = throat;
    stations[2] = exit;
    if let Some(ambient) = stations.get(3).cloned() {
        if ambient.gas_phase.is_some() && ambient.liquid_phase.is_some() {
            let relaxed_ambient =
                match relax_uniform(upstream.gas_mass_fraction, &ambient, progress) {
                    Ok(state) => state,
                    Err(refusal) => return Ok(refused(refusal, base.warnings)),
                };
            match drift_flux(&relaxed_ambient, surface_tension) {
                Ok(partitioned) => {
                    let mut row = partitioned.state;
                    row.station = ReleaseStation::AmbientExpanded;
                    stations[3] = row;
                }
                Err(refusal) => return Ok(refused(refusal, base.warnings)),
            }
        } else {
            diagnostics.push(HemReleaseDiagnostic {
                code: "AMBIENT_EQUILIBRIUM_ONLY".to_string(),
                message: "Ambient-expanded diagnostic does not retain one gas and one liquid \
                          phase; the finite-rate closure is reported only at the opening"
                    .to_string(),
            });
        }
    }
    let rate = area * outcome.mass_flux;
    Ok(success(
        Some(rate),
        base.choked,
        &base.component_mole_fractions,
        base.component_mass_fractions,
        stations.iter().map(row).collect(),
        diagnostics,
        base.warnings,
    ))
}

/// One published station row.
fn row(station: &TraceStation) -> FiniteRateReleaseStation {
    FiniteRateReleaseStation {
        station: station.station,
        pressure: pascals(station.pressure),
        temperature: kelvins(station.temperature),
        density: kilograms_per_cubic_meter(station.density),
        molar_enthalpy: joules_per_mole(station.molar_enthalpy),
        molar_entropy: joules_per_mole_kelvin(station.molar_entropy),
        velocity: meters_per_second(station.velocity),
        mass_flux: kilograms_per_square_meter_second(station.mass_flux),
        phase: station.phase,
        gas_mass_fraction: station.gas_mass_fraction,
        gas_density: station
            .gas_phase
            .as_ref()
            .map(|phase| kilograms_per_cubic_meter(phase.density)),
        liquid_density: station
            .liquid_phase
            .as_ref()
            .map(|phase| kilograms_per_cubic_meter(phase.density)),
        gas_velocity: station.gas_velocity.map(meters_per_second),
        liquid_velocity: station.liquid_velocity.map(meters_per_second),
    }
}

/// The class's success result.
#[allow(clippy::too_many_arguments)] // one argument per result field
fn success(
    flow: Option<f64>,
    choked: bool,
    z: &[f64],
    mass_fractions: Vec<f64>,
    stations: Vec<FiniteRateReleaseStation>,
    diagnostics: Vec<HemReleaseDiagnostic>,
    warnings: Vec<Warning>,
) -> FiniteRateReleaseResult {
    FiniteRateReleaseResult {
        status: "valid".to_string(),
        mass_flow_rate: flow.map(kilograms_per_second),
        choked,
        throat_sound_speed: None,
        component_mole_fractions: z.to_vec(),
        component_mass_fractions: mass_fractions,
        stations,
        diagnostic_codes: diagnostics.iter().map(|row| row.code.clone()).collect(),
        diagnostics,
        warnings,
    }
}

/// The class's result for a relaxation or closure refusal.
fn refused(refusal: ReleaseRefusal, warnings: Vec<Warning>) -> FiniteRateReleaseResult {
    match refusal {
        ReleaseRefusal::Unsupported(message) => failure(
            "unsupported",
            "FINITE_RATE_REGIME_UNSUPPORTED",
            message,
            warnings,
        ),
        ReleaseRefusal::Invalid(message) => {
            failure("invalid", "FINITE_RATE_CLOSURE_FAILED", message, warnings)
        }
    }
}

/// The class's failure result: no rate, no stations, one diagnostic.
fn failure(
    status: &str,
    code: &str,
    message: String,
    warnings: Vec<Warning>,
) -> FiniteRateReleaseResult {
    FiniteRateReleaseResult {
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
