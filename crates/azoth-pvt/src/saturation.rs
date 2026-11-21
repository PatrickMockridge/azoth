//! The two saturation searches, and the bubble point the swelling test restarts from.
//!
//! The searches are `SaturationPressure.calcSaturationPressure` and
//! `SaturationTemperature.calcSaturationTemperature` at the pin, arithmetic and order: a
//! 10-unit descending scan for the **uppermost** crossing, then a bisection to `1e-5`. An
//! interval that brackets no crossing is not a refusal in the class - it answers the top of
//! the grid - and the port carries that with a warning rather than inventing a failure.

use azoth_core::units::kelvins;
use azoth_core::warning::WarningCode;
use azoth_core::{Result, Warning};

use crate::flash::FlashMode;
use crate::fluid::Fluid;
use crate::standard::{
    SATURATION_BISECTION_LIMIT, SATURATION_PRESSURE_MAX_BAR, SATURATION_PRESSURE_MIN_BAR,
    SATURATION_PRESSURE_STEP_BAR, SATURATION_PRESSURE_TOLERANCE_BAR, SATURATION_TEMPERATURE_MAX_K,
    SATURATION_TEMPERATURE_MIN_K, SATURATION_TEMPERATURE_STEP_K,
    SATURATION_TEMPERATURE_TOLERANCE_K, bar_to_pa,
};

/// An upper saturation boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct SaturationPoint {
    /// The boundary pressure, in Pa, in pressure mode.
    pub pressure: Option<f64>,
    /// The boundary temperature, in K, in temperature mode.
    pub temperature: Option<f64>,
    /// Caveats, including the class's unbracketed fallback.
    pub warnings: Vec<Warning>,
}

/// The uppermost saturation pressure at a temperature.
///
/// # Errors
/// Propagates the flash's refusals.
pub fn upper_saturation_pressure(fluid: &Fluid, t: f64) -> Result<SaturationPoint> {
    let two_phase = |pressure_bar: f64| -> Result<bool> {
        Ok(fluid
            .flash(t, bar_to_pa(pressure_bar), FlashMode::Multiphase)?
            .phase_count
            > 1)
    };

    let mut higher_bar = SATURATION_PRESSURE_MAX_BAR;
    let mut higher_two = two_phase(higher_bar)?;
    let steps = ((SATURATION_PRESSURE_MAX_BAR - SATURATION_PRESSURE_MIN_BAR)
        / SATURATION_PRESSURE_STEP_BAR)
        .floor() as i64;

    for step in (0..=steps).rev() {
        let trial_bar = SATURATION_PRESSURE_MIN_BAR + step as f64 * SATURATION_PRESSURE_STEP_BAR;
        if trial_bar >= higher_bar {
            continue;
        }
        let trial_two = two_phase(trial_bar)?;
        if trial_two && !higher_two {
            let pressure = refine_pressure(&two_phase, trial_bar, higher_bar)?;
            return Ok(SaturationPoint {
                pressure: Some(bar_to_pa(pressure)),
                temperature: None,
                warnings: Vec::new(),
            });
        }
        higher_bar = trial_bar;
        higher_two = trial_two;
    }

    // The class returns the top of the grid when no boundary is bracketed.
    Ok(SaturationPoint {
        pressure: Some(bar_to_pa(SATURATION_PRESSURE_MAX_BAR)),
        temperature: None,
        warnings: vec![Warning::for_field(
            WarningCode::OutOfValidRange,
            "saturation_pressure",
            format!(
                "no two-phase region is bracketed between {} and {} bar, so the search's top \
                 is the answer",
                SATURATION_PRESSURE_MIN_BAR, SATURATION_PRESSURE_MAX_BAR
            ),
        )],
    })
}

fn refine_pressure(
    two_phase: &impl Fn(f64) -> Result<bool>,
    two_phase_pressure: f64,
    single_pressure: f64,
) -> Result<f64> {
    let mut min = two_phase_pressure;
    let mut max = single_pressure;
    let mut iteration = 0;
    loop {
        iteration += 1;
        let trial = (min + max) / 2.0;
        if two_phase(trial)? {
            min = trial;
        } else {
            max = trial;
        }
        if (max - min).abs() <= SATURATION_PRESSURE_TOLERANCE_BAR
            || iteration >= SATURATION_BISECTION_LIMIT
        {
            break;
        }
    }
    let _ = two_phase(max)?;
    Ok(max)
}

/// The uppermost saturation temperature at a pressure, optionally inside stated bounds.
///
/// # Errors
/// * [`azoth_core::AzothError::InvalidInput`] for a bound outside `[30, 1200] K`, a lower bound
///   not below the upper, or a non-positive pressure.
pub fn upper_saturation_temperature(
    fluid: &Fluid,
    p: f64,
    bounds: Option<(f64, f64)>,
) -> Result<SaturationPoint> {
    let (min, max) = bounds.unwrap_or((SATURATION_TEMPERATURE_MIN_K, SATURATION_TEMPERATURE_MAX_K));
    if !(min.is_finite()
        && max.is_finite()
        && min >= SATURATION_TEMPERATURE_MIN_K
        && max <= SATURATION_TEMPERATURE_MAX_K
        && min < max)
    {
        return Err(azoth_core::AzothError::invalid_input(
            "temperature_search",
            format!(
                "the bounds must satisfy {} <= minimum < maximum <= {} K; got {min} and {max}",
                SATURATION_TEMPERATURE_MIN_K, SATURATION_TEMPERATURE_MAX_K
            ),
        ));
    }
    let two_phase = |temperature: f64| -> Result<bool> {
        Ok(fluid
            .flash(temperature, p, FlashMode::Multiphase)?
            .phase_count
            > 1)
    };

    // The class treats the default pair as no bounds at all, because its fields are those
    // values until a caller sets them.
    let bounded = min != SATURATION_TEMPERATURE_MIN_K || max != SATURATION_TEMPERATURE_MAX_K;
    if bounded {
        if let Some(temperature) = search_temperature(&two_phase, min, max, true)? {
            return Ok(SaturationPoint {
                pressure: None,
                temperature: Some(temperature),
                warnings: Vec::new(),
            });
        }
    }
    if let Some(temperature) = search_temperature(
        &two_phase,
        SATURATION_TEMPERATURE_MIN_K,
        SATURATION_TEMPERATURE_MAX_K,
        false,
    )? {
        return Ok(SaturationPoint {
            pressure: None,
            temperature: Some(temperature),
            warnings: Vec::new(),
        });
    }

    let _ = two_phase(SATURATION_TEMPERATURE_MAX_K)?;
    Ok(SaturationPoint {
        pressure: None,
        temperature: Some(SATURATION_TEMPERATURE_MAX_K),
        warnings: vec![Warning::for_field(
            WarningCode::OutOfValidRange,
            "saturation_temperature",
            format!(
                "no two-phase region is bracketed between {} and {} K, so the search's top is \
                 the answer",
                SATURATION_TEMPERATURE_MIN_K, SATURATION_TEMPERATURE_MAX_K
            ),
        )],
    })
}

fn search_temperature(
    two_phase: &impl Fn(f64) -> Result<bool>,
    minimum: f64,
    maximum: f64,
    require_single_upper_endpoint: bool,
) -> Result<Option<f64>> {
    let mut higher = maximum;
    let mut higher_two = two_phase(higher)?;
    if require_single_upper_endpoint && higher_two {
        return Ok(None);
    }
    let mut trial = SATURATION_TEMPERATURE_MIN_K
        + SATURATION_TEMPERATURE_STEP_K
            * ((maximum - SATURATION_TEMPERATURE_MIN_K) / SATURATION_TEMPERATURE_STEP_K).floor();
    if trial >= higher {
        trial -= SATURATION_TEMPERATURE_STEP_K;
    }
    while higher > minimum {
        trial = trial.max(minimum);
        let trial_two = two_phase(trial)?;
        if trial_two && !higher_two {
            return Ok(Some(refine_temperature(two_phase, trial, higher)?));
        }
        higher = trial;
        higher_two = trial_two;
        trial -= SATURATION_TEMPERATURE_STEP_K;
    }
    Ok(None)
}

fn refine_temperature(
    two_phase: &impl Fn(f64) -> Result<bool>,
    two_phase_temperature: f64,
    single_temperature: f64,
) -> Result<f64> {
    let mut min = two_phase_temperature;
    let mut max = single_temperature;
    let mut iteration = 0;
    loop {
        iteration += 1;
        let trial = (min + max) / 2.0;
        if two_phase(trial)? {
            min = trial;
        } else {
            max = trial;
        }
        if (max - min).abs() <= SATURATION_TEMPERATURE_TOLERANCE_K
            || iteration >= SATURATION_BISECTION_LIMIT
        {
            break;
        }
    }
    let _ = two_phase(max)?;
    Ok(max)
}

/// The bubble-point pressure of the fluid at a temperature, in Pa.
///
/// `SwellingTest` restarts from `bubblePointPressureFlash(false)`; this is azoth's own
/// bubble-point solve, which finds the same root.
///
/// # Errors
/// Propagates [`azoth_eos::bubble_pressure`]'s refusals.
pub fn bubble_point_pressure(fluid: &Fluid, t: f64) -> Result<f64> {
    Ok(
        azoth_eos::bubble_pressure(&fluid.mixture, kelvins(t), &fluid.z())?
            .pressure
            .value,
    )
}
