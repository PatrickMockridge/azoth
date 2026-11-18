//! `process.flow_induced_pulsation_screening` - the side-branch resonance screen.
//!
//! Spec: `specs/models/process/flow_induced_pulsation_screening.toml`. The arithmetic is
//! `process/safety/vibration/FlowInducedPulsationScreening.java` at the pin: the effective
//! width, the acoustic eigenfrequencies, the lock-in envelopes and the severity band.

use azoth_core::units::{
    Frequency, Length, MassDensity, Pressure, Velocity, meters, meters_per_second, pascals,
    per_second,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;
use std::f64::consts::PI;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::piping_fiv_gas::FivLikelihood;

/// The class's default screening Strouhal number for the side-branch modes.
const DEFAULT_STROUHAL_NUMBER: f64 = 0.37;
/// The class's default number of acoustic modes.
const DEFAULT_MODE_COUNT: f64 = 5.0;
/// Half-width of the lock-in envelope, as a fraction.
const LOCK_IN_ENVELOPE_FRACTION: f64 = 0.20;
/// The reference momentum flux used to grade the severity, Pa.
const REFERENCE_RHO_V2: f64 = 20_000.0;
/// Severity below which a locked mode is not worth pursuing.
const NEGLIGIBLE_SEVERITY: f64 = 0.05;

/// One evaluated acoustic mode.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PulsationModeResult {
    /// The mode index, zero for the fundamental.
    pub mode_index: f64,
    /// The acoustic eigenfrequency.
    #[serde(serialize_with = "scalar")]
    pub frequency: Frequency,
    /// The lower lock-in envelope.
    #[serde(serialize_with = "scalar")]
    pub envelope_low: Frequency,
    /// The upper lock-in envelope.
    #[serde(serialize_with = "scalar")]
    pub envelope_high: Frequency,
    /// Whether the shedding frequency is inside this mode's envelope.
    pub locked_in: bool,
    /// The velocity that places the shedding frequency on this mode.
    #[serde(serialize_with = "scalar")]
    pub resonance_velocity: Velocity,
    /// The lower velocity of this mode's lock-in band.
    #[serde(serialize_with = "scalar")]
    pub resonance_velocity_low: Velocity,
    /// The upper velocity of this mode's lock-in band.
    #[serde(serialize_with = "scalar")]
    pub resonance_velocity_high: Velocity,
}

/// Result of `process.flow_induced_pulsation_screening`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlowInducedPulsationResult {
    /// The AVIFF likelihood band.
    pub likelihood: FivLikelihood,
    /// Whether any mode is locked in.
    pub any_mode_locked_in: bool,
    /// The vortex-shedding frequency.
    #[serde(serialize_with = "scalar")]
    pub shedding_frequency: Frequency,
    /// The lowest resonance velocity over all modes.
    #[serde(serialize_with = "scalar")]
    pub lowest_resonance_velocity: Velocity,
    /// The effective branch-mouth width.
    #[serde(serialize_with = "scalar")]
    pub effective_width: Length,
    /// The branch-to-run diameter ratio.
    pub diameter_ratio: f64,
    /// The main-line momentum flux.
    #[serde(serialize_with = "scalar")]
    pub rho_v2: Pressure,
    /// The momentum flux normalised by the reference.
    pub energy_term: f64,
    /// The diameter-ratio coupling factor.
    pub coupling_factor: f64,
    /// The strongest locked mode's weight.
    pub mode_weight: f64,
    /// The normalised severity.
    pub severity: f64,
    /// One row per evaluated mode, in mode order.
    pub modes: Vec<PulsationModeResult>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The class's band from the resonance state and severity.
fn band_for(any_mode_locked_in: bool, severity: f64) -> FivLikelihood {
    if !any_mode_locked_in || severity < NEGLIGIBLE_SEVERITY {
        FivLikelihood::Low
    } else if severity < 0.3 {
        FivLikelihood::Medium
    } else if severity < 1.0 {
        FivLikelihood::High
    } else {
        FivLikelihood::VeryHigh
    }
}

/// The acoustic eigenfrequency of one mode.
fn eigen_frequency(mode_index: u64, speed_of_sound: f64, acoustic_length: f64, open: bool) -> f64 {
    if open {
        (mode_index as f64 + 1.0) * speed_of_sound / (2.0 * acoustic_length)
    } else {
        (2 * mode_index + 1) as f64 * speed_of_sound / (4.0 * acoustic_length)
    }
}

/// Refuse a value the arithmetic cannot carry to a published number.
fn require_finite(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

/// The side-branch pulsation likelihood and every evaluated mode.
///
/// # Errors
/// `InvalidInput` for a non-positive acoustic length, diameter, speed of sound or Strouhal
/// number, a negative edge radius, velocity or density, a mode count below one or fractional,
/// an unknown termination, or a non-finite input.
#[allow(clippy::too_many_arguments)] // The signature is the spec's declared input list.
pub fn flow_induced_pulsation_screening(
    acoustic_length: Length,
    branch_internal_diameter: Length,
    run_internal_diameter: Length,
    main_line_velocity: Velocity,
    main_line_density: MassDensity,
    speed_of_sound: Velocity,
    edge_radius: Option<Length>,
    termination: Option<&str>,
    strouhal_number: Option<f64>,
    mode_count: Option<f64>,
) -> Result<FlowInducedPulsationResult> {
    let spec = &model_gen::FLOW_INDUCED_PULSATION_SCREENING_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "acoustic_length" => Some(acoustic_length.value),
            "branch_internal_diameter" => Some(branch_internal_diameter.value),
            "edge_radius" => Some(edge_radius.map_or(0.0, |value| value.value)),
            "run_internal_diameter" => Some(run_internal_diameter.value),
            "main_line_velocity" => Some(main_line_velocity.value),
            "main_line_density" => Some(main_line_density.value),
            "speed_of_sound" => Some(speed_of_sound.value),
            "strouhal_number" => Some(strouhal_number.unwrap_or(DEFAULT_STROUHAL_NUMBER)),
            "mode_count" => Some(mode_count.unwrap_or(DEFAULT_MODE_COUNT)),
            _ => None,
        },
        &mut warnings,
    )?;

    let edge = edge_radius.map_or(0.0, |value| value.value);
    let strouhal = strouhal_number.unwrap_or(DEFAULT_STROUHAL_NUMBER);
    let count = mode_count.unwrap_or(DEFAULT_MODE_COUNT);
    for (name, value) in [
        ("acoustic_length", acoustic_length.value),
        ("branch_internal_diameter", branch_internal_diameter.value),
        ("edge_radius", edge),
        ("run_internal_diameter", run_internal_diameter.value),
        ("main_line_velocity", main_line_velocity.value),
        ("main_line_density", main_line_density.value),
        ("speed_of_sound", speed_of_sound.value),
        ("strouhal_number", strouhal),
        ("mode_count", count),
    ] {
        require_finite(name, value)?;
    }
    if count.fract() != 0.0 {
        return Err(AzothError::invalid_input(
            "mode_count",
            "the acoustic mode count is a whole number",
        ));
    }
    let open = match termination.unwrap_or("closed") {
        "closed" => false,
        "open" => true,
        _ => {
            return Err(AzothError::invalid_input(
                "termination",
                "the acoustic termination must be `closed` or `open`",
            ));
        }
    };

    let effective_width = PI * branch_internal_diameter.value / 4.0 + edge;
    let shedding_frequency = strouhal * main_line_velocity.value / effective_width;
    let diameter_ratio = branch_internal_diameter.value / run_internal_diameter.value;
    let rho_v2 = main_line_density.value * main_line_velocity.value * main_line_velocity.value;

    let mut modes = Vec::new();
    let mut any_locked_in = false;
    let mut strongest_weight: f64 = 0.0;
    let mut lowest_resonance_velocity = f64::NAN;
    let count = count as u64;
    for mode_index in 0..count {
        let frequency = eigen_frequency(
            mode_index,
            speed_of_sound.value,
            acoustic_length.value,
            open,
        );
        let envelope_low = (1.0 - LOCK_IN_ENVELOPE_FRACTION) * frequency;
        let envelope_high = (1.0 + LOCK_IN_ENVELOPE_FRACTION) * frequency;
        let locked_in = shedding_frequency >= envelope_low && shedding_frequency <= envelope_high;
        let resonance_velocity = frequency * effective_width / strouhal;
        if locked_in {
            any_locked_in = true;
            strongest_weight = strongest_weight.max(1.0 / (2 * mode_index + 1) as f64);
        }
        if !lowest_resonance_velocity.is_finite() || resonance_velocity < lowest_resonance_velocity
        {
            lowest_resonance_velocity = resonance_velocity;
        }
        modes.push(PulsationModeResult {
            mode_index: mode_index as f64,
            frequency: per_second(frequency),
            envelope_low: per_second(envelope_low),
            envelope_high: per_second(envelope_high),
            locked_in,
            resonance_velocity: meters_per_second(resonance_velocity),
            resonance_velocity_low: meters_per_second(
                (1.0 - LOCK_IN_ENVELOPE_FRACTION) * resonance_velocity,
            ),
            resonance_velocity_high: meters_per_second(
                (1.0 + LOCK_IN_ENVELOPE_FRACTION) * resonance_velocity,
            ),
        });
    }

    let energy_term = rho_v2 / REFERENCE_RHO_V2;
    let coupling_factor = (diameter_ratio / 0.5).min(1.0);
    let severity = energy_term * coupling_factor * strongest_weight;

    Ok(FlowInducedPulsationResult {
        likelihood: band_for(any_locked_in, severity),
        any_mode_locked_in: any_locked_in,
        shedding_frequency: per_second(shedding_frequency),
        lowest_resonance_velocity: meters_per_second(lowest_resonance_velocity),
        effective_width: meters(effective_width),
        diameter_ratio,
        rho_v2: pascals(rho_v2),
        energy_term,
        coupling_factor,
        mode_weight: strongest_weight,
        severity,
        modes,
        warnings,
    })
}
