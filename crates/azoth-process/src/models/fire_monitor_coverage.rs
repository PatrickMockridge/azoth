//! `process.fire_monitor_coverage` - the fire-monitor screen as a registered id.
//!
//! Spec: `specs/models/process/fire_monitor_coverage.toml`. The arithmetic is
//! `process/safety/firewater/FireMonitorCoverage.java` at the pin: the density, the wind
//! drift and the class's verdict.

use azoth_core::units::{
    Area, Length, Velocity, VolumeRate, liters_per_minute, meters, square_meters,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;
use std::f64::consts::PI;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's default monitor count.
const DEFAULT_MONITOR_COUNT: f64 = 1.0;
/// The class's default monitor flow, L/min.
const DEFAULT_MONITOR_FLOW_LPM: f64 = 2400.0;
/// The class's default effective throw, m.
const DEFAULT_EFFECTIVE_THROW_M: f64 = 40.0;
/// The class's default sweep angle, degrees.
const DEFAULT_SWEEP_ANGLE_DEG: f64 = 90.0;
/// The class's default wind speed, m/s.
const DEFAULT_WIND_SPEED_M_S: f64 = 0.0;
/// The class's default fall height, m.
const DEFAULT_FALL_HEIGHT_M: f64 = 10.0;
/// The class's default droplet terminal velocity, m/s.
const DEFAULT_TERMINAL_VELOCITY_M_S: f64 = 8.0;

/// The class's verdicts, in its priority order.
const VERDICT_OBSTRUCTED: &str = "NOT SUITABLE - line of sight to part of the target is \
     obstructed; a monitor cannot wet surfaces it cannot see";
const VERDICT_STILL_AIR: &str = "NOT SUITABLE - required density not achieved even in still air";
const VERDICT_MARGINAL: &str = "MARGINAL - density achieved in still air only; wind drift takes \
     it below the requirement";
const VERDICT_FEASIBLE: &str =
    "FEASIBLE ON DENSITY - carry forward to a trajectory and shadowing study";

/// Result of `process.fire_monitor_coverage`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FireMonitorCoverageResult {
    /// The monitors' total flow.
    #[serde(serialize_with = "scalar")]
    pub total_flow: VolumeRate,
    /// The swept area of the monitors' cones.
    #[serde(serialize_with = "scalar")]
    pub swept_area: Area,
    /// The nominal density in `L/(min m2)`; dimensionless.
    pub nominal_density: f64,
    /// The wind drift displacement.
    #[serde(serialize_with = "scalar")]
    pub drift_displacement: Length,
    /// The fraction of the target the monitors still cover in wind.
    pub wind_coverage_fraction: f64,
    /// The nominal density times the coverage fraction.
    pub effective_density: f64,
    /// Whether the nominal density meets the requirement.
    pub meets_density_still_air: bool,
    /// Whether the effective density meets the requirement.
    pub meets_density_in_wind: bool,
    /// The flow that would meet the requirement; absent when the drift leaves no coverage.
    #[serde(serialize_with = "optional_scalar")]
    pub flow_needed_for_compliance: Option<VolumeRate>,
    /// The class's classification.
    pub verdict: String,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
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

/// The density, the wind drift, the coverage fraction and the class's verdict.
///
/// # Errors
/// `InvalidInput` for a non-positive area, density, flow, throw, fall height or terminal
/// velocity, a count below one or not whole, an angle outside `(0, 360]`, a negative wind
/// speed or dimension, or a non-finite input.
#[allow(clippy::too_many_arguments)]
pub fn fire_monitor_coverage(
    target_area: Area,
    required_density: f64,
    monitor_count: Option<f64>,
    monitor_flow: Option<VolumeRate>,
    effective_throw: Option<Length>,
    sweep_angle: Option<f64>,
    wind_speed: Option<Velocity>,
    fall_height: Option<Length>,
    droplet_terminal_velocity: Option<Velocity>,
    characteristic_dimension: Option<Length>,
    line_of_sight_obstructed: Option<bool>,
) -> Result<FireMonitorCoverageResult> {
    let spec = &model_gen::FIRE_MONITOR_COVERAGE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "target_area" => Some(target_area.value),
            "required_density" => Some(required_density),
            "monitor_count" => monitor_count,
            "monitor_flow" => monitor_flow.map(|value| value.value),
            "effective_throw" => effective_throw.map(|value| value.value),
            "sweep_angle" => sweep_angle,
            "wind_speed" => wind_speed.map(|value| value.value),
            "fall_height" => fall_height.map(|value| value.value),
            "droplet_terminal_velocity" => droplet_terminal_velocity.map(|value| value.value),
            "characteristic_dimension" => characteristic_dimension.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite("target_area", target_area.value)?;
    require_finite("required_density", required_density)?;
    let count = monitor_count.unwrap_or(DEFAULT_MONITOR_COUNT);
    require_finite("monitor_count", count)?;
    if count.fract() != 0.0 {
        return Err(AzothError::invalid_input(
            "monitor_count",
            "the class's `setMonitors` takes an integer count; a fractional monitor is not a \
             placement",
        ));
    }
    let flow_lpm = monitor_flow
        .map(|value| value.value * 60_000.0)
        .unwrap_or(DEFAULT_MONITOR_FLOW_LPM);
    require_finite("monitor_flow", flow_lpm)?;
    let throw_m = effective_throw
        .map(|value| value.value)
        .unwrap_or(DEFAULT_EFFECTIVE_THROW_M);
    require_finite("effective_throw", throw_m)?;
    let sweep = sweep_angle.unwrap_or(DEFAULT_SWEEP_ANGLE_DEG);
    require_finite("sweep_angle", sweep)?;
    let wind = wind_speed
        .map(|value| value.value)
        .unwrap_or(DEFAULT_WIND_SPEED_M_S);
    require_finite("wind_speed", wind)?;
    let fall = fall_height
        .map(|value| value.value)
        .unwrap_or(DEFAULT_FALL_HEIGHT_M);
    require_finite("fall_height", fall)?;
    let terminal = droplet_terminal_velocity
        .map(|value| value.value)
        .unwrap_or(DEFAULT_TERMINAL_VELOCITY_M_S);
    require_finite("droplet_terminal_velocity", terminal)?;
    let stated_dimension = characteristic_dimension.map(|value| value.value);
    if let Some(dimension) = stated_dimension {
        require_finite("characteristic_dimension", dimension)?;
    }
    let obstructed = line_of_sight_obstructed.unwrap_or(false);

    // The class's operation order, to the letter.
    let total_flow_lpm = count * flow_lpm;
    let swept_area = count * (PI * throw_m * throw_m * sweep / 360.0);
    let nominal_density = total_flow_lpm / target_area.value;
    let drift = fall * wind / terminal;
    let dimension = stated_dimension.unwrap_or_else(|| target_area.value.sqrt());
    let fraction = (1.0 - drift / dimension).clamp(0.0, 1.0);
    let effective_density = nominal_density * fraction;
    let still_air = nominal_density >= required_density;
    let in_wind = effective_density >= required_density;
    let needed = if fraction <= 0.0 {
        None
    } else {
        Some(required_density * target_area.value / fraction)
    };
    let verdict = if obstructed {
        VERDICT_OBSTRUCTED
    } else if !still_air {
        VERDICT_STILL_AIR
    } else if !in_wind {
        VERDICT_MARGINAL
    } else {
        VERDICT_FEASIBLE
    };

    Ok(FireMonitorCoverageResult {
        total_flow: liters_per_minute(total_flow_lpm),
        swept_area: square_meters(swept_area),
        nominal_density,
        drift_displacement: meters(drift),
        wind_coverage_fraction: fraction,
        effective_density,
        meets_density_still_air: still_air,
        meets_density_in_wind: in_wind,
        flow_needed_for_compliance: needed.map(liters_per_minute),
        verdict: verdict.to_string(),
        warnings,
    })
}
