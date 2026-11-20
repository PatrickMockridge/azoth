//! `process.vessel_rupture` - the thin-wall vessel crossover as a registered id.
//!
//! Spec: `specs/models/process/vessel_rupture.toml`. The arithmetic is
//! `process/safety/rupture/VesselRuptureAnalyzer.java` and `MaterialStrengthCurve.java` at
//! the pin, with `process/util/fire/VesselRuptureCalculator.vonMisesStress`: the thin-wall
//! stress, the retained-strength curve, the first margin at or below zero and the linear
//! crossover.

use azoth_core::units::{
    Length, Pressure, ThermodynamicTemperature, Time, megapascals, pascals, seconds,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::material_strength::{
    api5l_strengths, normalize_grade, retained_factor, von_mises_pa,
};

/// One state of the transient.
#[derive(Debug, Clone, PartialEq)]
pub struct VesselRuptureState {
    /// The state's time.
    pub time: Time,
    /// The state's absolute internal pressure.
    pub internal_pressure: Pressure,
    /// The state's metal temperature.
    pub metal_temperature: ThermodynamicTemperature,
}

/// One row of the analyzed history.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VesselRuptureStep {
    /// The state's time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
    /// The state's thin-wall von Mises stress.
    #[serde(serialize_with = "scalar")]
    pub von_mises_stress: Pressure,
    /// The state's derated allowable stress.
    #[serde(serialize_with = "scalar")]
    pub allowable_stress: Pressure,
    /// `allowable_stress - von_mises_stress`.
    #[serde(serialize_with = "scalar")]
    pub margin: Pressure,
}

/// Result of `process.vessel_rupture`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VesselRuptureResult {
    /// Whether any step's margin is at or below zero.
    pub ruptured: bool,
    /// The interpolated first crossover time.
    #[serde(serialize_with = "optional_scalar")]
    pub rupture_time: Option<Time>,
    /// The von Mises stress at the rupture step.
    #[serde(serialize_with = "optional_scalar")]
    pub rupture_stress: Option<Pressure>,
    /// The allowable stress at the rupture step.
    #[serde(serialize_with = "optional_scalar")]
    pub rupture_allowable_stress: Option<Pressure>,
    /// The metal temperature at the rupture step.
    #[serde(serialize_with = "optional_scalar")]
    pub rupture_metal_temperature: Option<ThermodynamicTemperature>,
    /// The smallest margin over the history.
    #[serde(serialize_with = "scalar")]
    pub min_margin: Pressure,
    /// The time of the first occurrence of the smallest margin.
    #[serde(serialize_with = "scalar")]
    pub min_margin_time: Time,
    /// The class's material name.
    pub material_name: String,
    /// The ambient yield strength the material carries.
    #[serde(serialize_with = "scalar")]
    pub ambient_yield_strength: Pressure,
    /// The ambient tensile strength the allowable is built from.
    #[serde(serialize_with = "scalar")]
    pub ambient_tensile_strength: Pressure,
    /// One row per input state, in the caller's order.
    pub history: Vec<VesselRuptureStep>,
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

/// The thin-wall vessel rupture crossover.
///
/// # Errors
/// `InvalidInput` for a non-positive radius, wall, strength or temperature, a missing
/// strength or grade, an unknown grade, a contradictory material group, an empty or
/// non-increasing history, a negative pressure, a factor outside `(0, 1]`, or a non-finite
/// value.
#[allow(clippy::too_many_arguments)]
pub fn vessel_rupture(
    inner_radius: Length,
    wall_thickness: Length,
    material: &str,
    history: &[VesselRuptureState],
    yield_strength: Option<Pressure>,
    tensile_strength: Option<Pressure>,
    grade: Option<&str>,
    tensile_strength_factor: Option<f64>,
) -> Result<VesselRuptureResult> {
    let spec = &model_gen::VESSEL_RUPTURE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "inner_radius" => Some(inner_radius.value),
            "wall_thickness" => Some(wall_thickness.value),
            "yield_strength" => yield_strength.map(|value| value.value),
            "tensile_strength" => tensile_strength.map(|value| value.value),
            "tensile_strength_factor" => tensile_strength_factor,
            _ => None,
        },
        &mut warnings,
    )?;

    if inner_radius.value <= 0.0 || wall_thickness.value <= 0.0 {
        return Err(AzothError::invalid_input(
            "inner_radius",
            "the class's constructor refuses a non-positive radius or wall",
        ));
    }
    require_finite("inner_radius", inner_radius.value)?;
    require_finite("wall_thickness", wall_thickness.value)?;
    let factor = tensile_strength_factor.unwrap_or(1.0);
    require_finite("tensile_strength_factor", factor)?;
    if factor <= 0.0 || factor > 1.0 {
        return Err(AzothError::invalid_input(
            "tensile_strength_factor",
            "the class's setter refuses a factor outside (0, 1]",
        ));
    }
    if history.is_empty() {
        return Err(AzothError::invalid_input(
            "history",
            "the class refuses an empty history",
        ));
    }
    let mut previous = f64::NEG_INFINITY;
    for state in history {
        require_finite("history.time", state.time.value)?;
        require_finite("history.internal_pressure", state.internal_pressure.value)?;
        require_finite("history.metal_temperature", state.metal_temperature.value)?;
        if state.time.value < 0.0 {
            return Err(AzothError::invalid_input(
                "history.time",
                "a negative time is not an instant",
            ));
        }
        if state.time.value <= previous {
            return Err(AzothError::invalid_input(
                "history.time",
                "the class's javadoc requires strictly increasing times; the port refuses them",
            ));
        }
        previous = state.time.value;
        if state.internal_pressure.value < 0.0 {
            return Err(AzothError::invalid_input(
                "history.internal_pressure",
                "the class's von Mises helper refuses a negative pressure",
            ));
        }
        if state.metal_temperature.value <= 0.0 {
            return Err(AzothError::invalid_input(
                "history.metal_temperature",
                "a non-positive absolute temperature is not a temperature",
            ));
        }
    }

    let (material_name, ambient_yield, ambient_tensile) = match material {
        "carbon_steel" => {
            if grade.is_some() {
                return Err(AzothError::invalid_input(
                    "grade",
                    "the carbon-steel branch takes its strengths, not a grade",
                ));
            }
            let yield_value = yield_strength.ok_or_else(|| {
                AzothError::invalid_input(
                    "yield_strength",
                    "the carbon-steel branch requires its ambient yield strength",
                )
            })?;
            let tensile_value = tensile_strength.ok_or_else(|| {
                AzothError::invalid_input(
                    "tensile_strength",
                    "the carbon-steel branch requires its ambient tensile strength",
                )
            })?;
            if yield_value.value <= 0.0 || tensile_value.value <= 0.0 {
                return Err(AzothError::invalid_input(
                    "yield_strength",
                    "the class's material constructor refuses a non-positive strength",
                ));
            }
            ("CS".to_string(), yield_value.value, tensile_value.value)
        }
        "api_5l" => {
            if yield_strength.is_some() || tensile_strength.is_some() {
                return Err(AzothError::invalid_input(
                    "grade",
                    "the API 5L branch takes a grade, not explicit strengths",
                ));
            }
            let supplied = grade.ok_or_else(|| {
                AzothError::invalid_input("grade", "the API 5L branch requires a grade")
            })?;
            let normalized = normalize_grade(supplied)?;
            let (yield_value, tensile_value) = api5l_strengths(&normalized)?;
            (format!("API 5L {normalized}"), yield_value, tensile_value)
        }
        other => {
            return Err(AzothError::invalid_input(
                "material",
                format!("`{other}` is not `carbon_steel` or `api_5l`"),
            ));
        }
    };

    let radius = inner_radius.value;
    let wall = wall_thickness.value;
    let mut rows = Vec::with_capacity(history.len());
    let mut ruptured = false;
    let mut rupture_time = None;
    let mut rupture_stress = None;
    let mut rupture_allowable = None;
    let mut rupture_temperature = None;
    let mut min_margin = f64::INFINITY;
    let mut min_margin_time = history[0].time.value;
    let mut previous_margin = f64::NAN;
    let mut previous_time = f64::NAN;

    for state in history {
        let stress = von_mises_pa(state.internal_pressure.value, radius, wall);
        let allowable = ambient_tensile * retained_factor(state.metal_temperature.value) * factor;
        let margin = allowable - stress;
        rows.push(VesselRuptureStep {
            time: state.time,
            von_mises_stress: pascals(stress),
            allowable_stress: pascals(allowable),
            margin: pascals(margin),
        });
        if margin < min_margin {
            min_margin = margin;
            min_margin_time = state.time.value;
        }
        if !ruptured && margin <= 0.0 {
            ruptured = true;
            if previous_margin > 0.0 && margin != previous_margin {
                let fraction = previous_margin / (previous_margin - margin);
                rupture_time = Some(seconds(
                    previous_time + fraction * (state.time.value - previous_time),
                ));
            } else {
                rupture_time = Some(state.time);
            }
            rupture_stress = Some(pascals(stress));
            rupture_allowable = Some(pascals(allowable));
            rupture_temperature = Some(state.metal_temperature);
        }
        previous_margin = margin;
        previous_time = state.time.value;
    }

    Ok(VesselRuptureResult {
        ruptured,
        rupture_time,
        rupture_stress,
        rupture_allowable_stress: rupture_allowable,
        rupture_metal_temperature: rupture_temperature,
        min_margin: pascals(min_margin),
        min_margin_time: seconds(min_margin_time),
        material_name,
        ambient_yield_strength: megapascals(ambient_yield / 1.0e6),
        ambient_tensile_strength: megapascals(ambient_tensile / 1.0e6),
        history: rows,
        warnings,
    })
}
