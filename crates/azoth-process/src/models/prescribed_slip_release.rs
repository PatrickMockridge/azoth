//! `process.prescribed_slip_release` - NeqSim's prescribed-slip orifice release.
//!
//! Spec: `specs/models/process/prescribed_slip_release.toml`. The port source is
//! `process/safety/release/SlipCorrectedHomogeneousEquilibriumReleaseModel.java`, through its
//! `calculate`.
//!
//! The homogeneous-equilibrium scan supplies the stations; the opening's phase velocities are
//! partitioned by a caller-declared gas-to-liquid ratio, and the rate is the partition's mass
//! flux times the effective area.

use azoth_core::units::{
    Length, MassDensity, MassFlux, MassRate, MolarEnergy, MolarHeatCapacity, Pressure,
    ThermodynamicTemperature, Velocity, joules_per_mole, joules_per_mole_kelvin, kelvins,
    kilograms_per_cubic_meter, kilograms_per_second, kilograms_per_square_meter_second,
    meters_per_second, pascals,
};
use azoth_core::{Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::hem_release::{
    HemReleaseDiagnostic, ReleasePhase, ReleaseStation, TraceStation, trace,
};
use crate::models::release::{ReleaseRefusal, prescribed_slip};

/// One row of `stations`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PrescribedSlipReleaseStation {
    /// Which station this row is.
    pub station: ReleaseStation,
    /// Absolute pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// Temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// Mass over the EOS total volume.
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
    /// The gas phase's share of the mass.
    pub gas_mass_fraction: f64,
    /// The gas phase's own density.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_density: Option<MassDensity>,
    /// The liquid phase's own density.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_density: Option<MassDensity>,
    /// The partitioned gas velocity.
    #[serde(serialize_with = "optional_scalar")]
    pub gas_velocity: Option<Velocity>,
    /// The partitioned liquid velocity.
    #[serde(serialize_with = "optional_scalar")]
    pub liquid_velocity: Option<Velocity>,
}

/// Result of `process.prescribed_slip_release`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PrescribedSlipReleaseResult {
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
    /// The base stations with the opening's phase velocities partitioned.
    pub stations: Vec<PrescribedSlipReleaseStation>,
    /// The model's assumptions and the base's retained records.
    pub diagnostics: Vec<HemReleaseDiagnostic>,
    /// The diagnostics' machine codes alone.
    pub diagnostic_codes: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// NeqSim's prescribed-slip orifice release.
///
/// # Errors
/// A non-positive input, a slip ratio below one, a composition that is not one entry per
/// component, or a mixture the databank cannot resolve.
pub fn prescribed_slip_release(
    components: &[String],
    upstream_z: &[f64],
    upstream_t: ThermodynamicTemperature,
    upstream_p: Pressure,
    diameter: Length,
    discharge_coefficient: f64,
    back_pressure: Pressure,
    gas_to_liquid_slip_ratio: f64,
) -> Result<PrescribedSlipReleaseResult> {
    let spec = &model_gen::PRESCRIBED_SLIP_RELEASE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "upstream_t" => Some(upstream_t.value),
            "upstream_p" => Some(upstream_p.value),
            "diameter" => Some(diameter.value),
            "discharge_coefficient" => Some(discharge_coefficient),
            "back_pressure" => Some(back_pressure.value),
            "gas_to_liquid_slip_ratio" => Some(gas_to_liquid_slip_ratio),
            _ => None,
        },
        &mut warnings,
    )?;
    if !gas_to_liquid_slip_ratio.is_finite() || gas_to_liquid_slip_ratio < 1.0 {
        return Err(azoth_core::AzothError::invalid_input(
            "gas_to_liquid_slip_ratio",
            format!(
                "the gas/liquid slip ratio must be finite and at least one, and \
                 {gas_to_liquid_slip_ratio} is not"
            ),
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
        message: "Equilibrium phase state with prescribed gas/liquid velocity slip; no pipe \
                  friction, heat transfer, finite-rate phase transfer, entrainment correlation \
                  or solids"
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
            code: "SLIP_NOT_APPLIED".to_string(),
            message: "No forward flow; phase velocities are not resolved".to_string(),
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

    let opening = &base.stations[1];
    let outcome = match prescribed_slip(opening, gas_to_liquid_slip_ratio) {
        Ok(outcome) => outcome,
        Err(refusal) => return Ok(refused(refusal, base.warnings)),
    };
    diagnostics.push(HemReleaseDiagnostic {
        code: "PRESCRIBED_PHASE_SLIP".to_string(),
        message: "Prescribed gas/liquid velocity ratio; no inferred slip or entrainment \
                  correlation"
            .to_string(),
    });
    diagnostics.push(HemReleaseDiagnostic {
        code: "SLIP_CLOSURE".to_string(),
        message: "Phase-area and kinetic-energy closures hold at the opening".to_string(),
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
            if let Ok(partitioned) = prescribed_slip(&ambient, gas_to_liquid_slip_ratio) {
                let mut row = partitioned.state;
                row.station = ReleaseStation::AmbientExpanded;
                stations[3] = row;
            }
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
fn row(station: &TraceStation) -> PrescribedSlipReleaseStation {
    PrescribedSlipReleaseStation {
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
    stations: Vec<PrescribedSlipReleaseStation>,
    diagnostics: Vec<HemReleaseDiagnostic>,
    warnings: Vec<Warning>,
) -> PrescribedSlipReleaseResult {
    PrescribedSlipReleaseResult {
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

/// The class's result for a partition refusal.
fn refused(refusal: ReleaseRefusal, warnings: Vec<Warning>) -> PrescribedSlipReleaseResult {
    match refusal {
        ReleaseRefusal::Unsupported(message) => {
            failure("unsupported", "SLIP_REGIME_UNSUPPORTED", message, warnings)
        }
        ReleaseRefusal::Invalid(message) => {
            failure("invalid", "SLIP_CLOSURE_FAILED", message, warnings)
        }
    }
}

/// The class's failure result: no rate, no stations, one diagnostic.
fn failure(
    status: &str,
    code: &str,
    message: String,
    warnings: Vec<Warning>,
) -> PrescribedSlipReleaseResult {
    PrescribedSlipReleaseResult {
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
