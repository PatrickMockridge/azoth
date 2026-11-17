//! `process.self_heating_induction` - the one-dimensional induction-time march.
//!
//! Spec: `specs/models/process/self_heating_induction.toml`. The arithmetic is NeqSim's
//! `process/safety/selfheating/SelfHeatingInductionSolver.java`'s `solve`: explicit Euler
//! on a conduction equation with an Arrhenius source, its adaptive step, its three
//! termination states and its decimated temperature history.

use azoth_core::units::{
    Length, MolarEnergy, TemperatureInterval, ThermalConductivity, ThermodynamicTemperature, Time,
    kelvin_intervals, kelvins, seconds,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::selfheating::{R_GAS, SelfHeatingGeometry};

/// The class's step cap.
const MAX_STEPS: usize = 20_000_000;

/// The retained-history cap, after which the list is decimated and the stride doubles.
const MAX_HISTORY_POINTS: usize = 500;

/// The safety factor on the conduction stability limit.
const CONDUCTION_SAFETY: f64 = 0.4;

/// The maximum fraction of the Frank-Kamenetskii temperature scale per step.
const REACTION_STEP_FRACTION: f64 = 0.02;

/// The default ignition rise, K.
const DEFAULT_IGNITION_RISE_K: f64 = 100.0;

/// The default simulated-time cap, s (30 days).
const DEFAULT_MAX_TIME_S: f64 = 30.0 * 24.0 * 3600.0;

/// One retained temperature sample.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelfHeatingTimePoint {
    /// The sample's time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
    /// The centre-node temperature.
    #[serde(serialize_with = "scalar")]
    pub centre_temperature: ThermodynamicTemperature,
    /// The peak node temperature.
    #[serde(serialize_with = "scalar")]
    pub max_temperature: ThermodynamicTemperature,
}

/// Result of `process.self_heating_induction`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelfHeatingInductionResult {
    /// The body shape the march used.
    pub geometry: SelfHeatingGeometry,
    /// The characteristic half-dimension.
    #[serde(serialize_with = "scalar")]
    pub characteristic_dimension: Length,
    /// The surface temperature held fixed.
    #[serde(serialize_with = "scalar")]
    pub boundary_temperature: ThermodynamicTemperature,
    /// The uniform temperature the body started at.
    #[serde(serialize_with = "scalar")]
    pub initial_temperature: ThermodynamicTemperature,
    /// The rise above the boundary that defines ignition.
    #[serde(serialize_with = "scalar")]
    pub ignition_rise: TemperatureInterval,
    /// Whether the peak reached the ignition rise.
    pub ignited: bool,
    /// The time at which the peak reached it; absent when the body did not ignite.
    #[serde(serialize_with = "optional_scalar")]
    pub induction_time: Option<Time>,
    /// The final peak temperature.
    #[serde(serialize_with = "scalar")]
    pub peak_temperature: ThermodynamicTemperature,
    /// `peak_temperature - boundary_temperature`.
    #[serde(serialize_with = "scalar")]
    pub peak_temperature_rise: TemperatureInterval,
    /// Whether the steady heuristic ended the run.
    pub steady_state_reached: bool,
    /// The simulated time the run reached.
    #[serde(serialize_with = "scalar")]
    pub simulated_time: Time,
    /// The retained temperature samples, in the class's order.
    pub history: Vec<SelfHeatingTimePoint>,
    /// The class's warning strings, in order.
    pub messages: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn max_of(values: &[f64]) -> f64 {
    let mut max = values[0];
    for value in &values[1..] {
        if *value > max {
            max = *value;
        }
    }
    max
}

fn decimate(history: &mut Vec<SelfHeatingTimePoint>) {
    let kept: Vec<SelfHeatingTimePoint> = history.iter().step_by(2).cloned().collect();
    *history = kept;
}

/// The one-dimensional induction-time march for a self-heating body.
///
/// # Errors
/// `InvalidInput` for an unknown or non-one-dimensional geometry, a non-positive
/// dimension, conductivity, heat capacity, activation energy, pre-factor or boundary
/// temperature, a node count below 5 that is not a whole number, or a non-positive
/// ignition rise or maximum time.
#[allow(clippy::too_many_arguments)] // one argument per declared input
pub fn self_heating_induction(
    geometry: &str,
    characteristic_dimension: Length,
    effective_conductivity: ThermalConductivity,
    volumetric_heat_capacity: f64,
    activation_energy: MolarEnergy,
    volumetric_pre_factor: f64,
    boundary_temperature: ThermodynamicTemperature,
    initial_temperature: Option<ThermodynamicTemperature>,
    ignition_rise: Option<TemperatureInterval>,
    max_time: Option<Time>,
    node_count: Option<f64>,
) -> Result<SelfHeatingInductionResult> {
    let spec = &model_gen::SELF_HEATING_INDUCTION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "characteristic_dimension" => Some(characteristic_dimension.value),
            "effective_conductivity" => Some(effective_conductivity.value),
            "volumetric_heat_capacity" => Some(volumetric_heat_capacity),
            "activation_energy" => Some(activation_energy.value),
            "volumetric_pre_factor" => Some(volumetric_pre_factor),
            "boundary_temperature" => Some(boundary_temperature.value),
            "initial_temperature" => initial_temperature.map(|value| value.value),
            "ignition_rise" => ignition_rise.map(|value| value.value),
            "max_time" => max_time.map(|value| value.value),
            "node_count" => node_count,
            _ => None,
        },
        &mut warnings,
    )?;

    let shape = SelfHeatingGeometry::parse(geometry)?;
    if !shape.one_dimensional() {
        return Err(AzothError::invalid_input(
            "geometry",
            format!(
                "`{}` is not one-dimensional; use `slab`, `infinite_cylinder` or `sphere`",
                shape.as_str()
            ),
        ));
    }
    let count = match node_count {
        Some(value) if value.is_finite() && value.fract() == 0.0 && value >= 5.0 => value as usize,
        Some(_) => {
            return Err(AzothError::invalid_input(
                "node_count",
                "a node count must be a whole number of at least 5",
            ));
        }
        None => 41,
    };

    let dimension = characteristic_dimension.value;
    let conductivity = effective_conductivity.value;
    let heat_capacity = volumetric_heat_capacity;
    let energy = activation_energy.value;
    let pre_factor = volumetric_pre_factor;
    let boundary = boundary_temperature.value;
    let rise = ignition_rise.map_or(DEFAULT_IGNITION_RISE_K, |value| value.value);
    let max_time_s = max_time.map_or(DEFAULT_MAX_TIME_S, |value| value.value);
    let start = initial_temperature.map_or(boundary, |value| value.value);

    let shape_factor = shape.shape_factor();
    let dx = dimension / (count - 1) as f64;
    let alpha = conductivity / heat_capacity;
    let mut temperature = vec![start; count];
    temperature[count - 1] = boundary;
    let dt_conduction = CONDUCTION_SAFETY * dx * dx / (2.0 * (shape_factor + 1.0) * alpha);
    let ignition_temperature = boundary + rise;

    let mut history = vec![SelfHeatingTimePoint {
        time: seconds(0.0),
        centre_temperature: kelvins(temperature[0]),
        max_temperature: kelvins(max_of(&temperature)),
    }];
    let mut history_stride = 1usize;
    let mut steps_since_sample = 0usize;
    let mut time = 0.0;
    let mut ignited = false;
    let mut steady = false;
    let mut induction_time = None;
    let mut next = vec![0.0; count];
    let mut step = 0usize;
    while step < MAX_STEPS {
        let peak = max_of(&temperature);
        if peak >= ignition_temperature {
            ignited = true;
            induction_time = Some(time);
            break;
        }
        if time >= max_time_s {
            break;
        }

        let peak_source = pre_factor * (-energy / (R_GAS * peak)).exp();
        let mut dt = dt_conduction;
        if peak_source > 0.0 {
            let fk_scale = R_GAS * peak * peak / energy;
            let dt_reaction = REACTION_STEP_FRACTION * fk_scale * heat_capacity / peak_source;
            dt = dt.min(dt_reaction);
        }
        if time + dt > max_time_s {
            dt = max_time_s - time;
        }

        let mut max_rate = 0.0_f64;
        for k in 0..count - 1 {
            let laplacian = if k == 0 {
                2.0 * (shape_factor + 1.0) * (temperature[1] - temperature[0]) / (dx * dx)
            } else {
                let second =
                    (temperature[k - 1] - 2.0 * temperature[k] + temperature[k + 1]) / (dx * dx);
                let first = (temperature[k + 1] - temperature[k - 1]) / (2.0 * dx);
                second + shape_factor * first / (k as f64 * dx)
            };
            let source = pre_factor * (-energy / (R_GAS * temperature[k])).exp();
            let rate = (conductivity * laplacian + source) / heat_capacity;
            next[k] = temperature[k] + dt * rate;
            max_rate = max_rate.max(rate.abs());
        }
        next[count - 1] = boundary;

        temperature.copy_from_slice(&next);
        time += dt;

        steps_since_sample += 1;
        if steps_since_sample >= history_stride {
            steps_since_sample = 0;
            history.push(SelfHeatingTimePoint {
                time: seconds(time),
                centre_temperature: kelvins(temperature[0]),
                max_temperature: kelvins(max_of(&temperature)),
            });
            if history.len() >= MAX_HISTORY_POINTS {
                decimate(&mut history);
                history_stride *= 2;
            }
        }

        if max_rate * (max_time_s - time) < 0.1 && time > 0.0 {
            steady = true;
            break;
        }
        step += 1;
    }
    let step_limit_hit = step >= MAX_STEPS;
    let mut messages = Vec::new();
    if step_limit_hit {
        messages.push(format!(
            "Step limit of {MAX_STEPS} reached before ignition or steady state; reduce the node count or the maximum simulated time"
        ));
    }

    let final_peak = max_of(&temperature);
    history.push(SelfHeatingTimePoint {
        time: seconds(time),
        centre_temperature: kelvins(temperature[0]),
        max_temperature: kelvins(final_peak),
    });

    if !ignited && steady {
        messages.push(
            "A steady temperature profile was reached, so the body is subcritical at this boundary temperature"
                .to_string(),
        );
    }
    if !ignited && !steady && !step_limit_hit {
        messages.push(
            "Maximum simulated time reached without ignition or a clear steady state; extend the simulated time before concluding the case is safe"
                .to_string(),
        );
    }

    Ok(SelfHeatingInductionResult {
        geometry: shape,
        characteristic_dimension,
        boundary_temperature,
        initial_temperature: kelvins(start),
        ignition_rise: kelvin_intervals(rise),
        ignited,
        induction_time: induction_time.map(seconds),
        peak_temperature: kelvins(final_peak),
        peak_temperature_rise: kelvin_intervals(final_peak - boundary),
        steady_state_reached: steady,
        simulated_time: seconds(time),
        history,
        messages,
        warnings,
    })
}
