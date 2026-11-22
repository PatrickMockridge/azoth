//! `pvt.swelling_test` - NeqSim's swelling experiment.
//!
//! Spec: `specs/models/pvt/swelling_test.toml`. The arithmetic is
//! `pvtsimulation/simulation/SwellingTest.java` at the pin: the base oil's bubble-point
//! volume, an injection of gas at each cumulative mole percentage of the original oil moles,
//! and the relative volume at each bubble point.
//!
//! The class restarts from the constructor's snapshot and adds the gas to the same system;
//! this port restarts from the row list and adds component-wise, one fresh flash per state.
//! The pinned upstream case - methane/n-decane 0.2/0.8, methane injection, 353.15 K, schedule
//! 0/10/20 %, zero kij and no volume correction - is the capture's first block.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::ThermodynamicTemperature;
use azoth_core::{AzothError, Result, Warning};

use crate::flash::FlashMode;
use crate::fluid::{Fluid, GasTemplate, cubic_named, kij_named};
use crate::saturation::bubble_point_pressure;
use crate::standard::SWELLING_DEFAULT_TEMPERATURE_K;

/// One point of a swelling experiment.
#[derive(Debug, Clone, PartialEq)]
pub struct SwellingPoint {
    /// The cumulative gas injected, as a fraction of the original oil moles.
    pub cumulative_gas_injected: f64,
    /// The bubble-point pressure, in Pa.
    pub pressure: f64,
    /// The bubble-point volume over the original oil's bubble-point volume.
    pub relative_oil_volume: f64,
}

/// Result of `pvt.swelling_test`.
#[derive(Debug, Clone, PartialEq)]
pub struct SwellingTestResult {
    /// The original oil's total amount, in mol.
    pub original_oil_moles: f64,
    /// Its bubble-point volume, in m3, translated where the fluid asks.
    pub original_oil_volume: f64,
    /// One row per schedule entry.
    pub points: Vec<SwellingPoint>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The swelling of an oil by an injection gas.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown `eos` or `mixing_rule`, an empty, negative,
///   non-finite or decreasing schedule, an injection without a gas, a gas that shares no
///   component with the oil, a non-positive temperature, or a failed bubble point.
pub fn swelling_test(
    components: &[FluidComponent],
    injection_gas: Option<&[FluidComponent]>,
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    temperature: Option<ThermodynamicTemperature>,
    cumulative_gas_injected: &[f64],
    volume_correction: Option<bool>,
) -> Result<SwellingTestResult> {
    if cumulative_gas_injected.is_empty() {
        return Err(AzothError::invalid_input(
            "cumulative_gas_injected",
            "a swelling experiment needs at least one schedule entry",
        ));
    }
    let mut previous = 0.0;
    for (index, cumulative) in cumulative_gas_injected.iter().enumerate() {
        if !cumulative.is_finite() || *cumulative < 0.0 {
            return Err(AzothError::invalid_input(
                format!("cumulative_gas_injected[{index}]"),
                format!("a cumulative injection is {cumulative}"),
            ));
        }
        if *cumulative < previous {
            return Err(AzothError::invalid_input(
                format!("cumulative_gas_injected[{index}]"),
                "the schedule is cumulative and cannot decrease",
            ));
        }
        previous = *cumulative;
    }
    let gas = injection_gas.unwrap_or(&[]);
    let injects = cumulative_gas_injected.last().copied().unwrap_or(0.0) > 0.0;
    if injects && gas.is_empty() {
        return Err(AzothError::invalid_input(
            "injection_gas",
            "the schedule injects gas and no injection gas is stated",
        ));
    }

    // With no injection at all the union is the oil; a gas template of zero total is not a
    // template.
    let (oil, template) = if gas.is_empty() {
        let fluid = Fluid::from_rows(
            components,
            cubic_named(eos)?,
            kij_named(mixing_rule)?,
            volume_correction.unwrap_or(true),
        )?;
        let count = fluid.moles().len();
        (
            fluid,
            GasTemplate {
                amounts: vec![0.0; count],
                total: 0.0,
            },
        )
    } else {
        Fluid::from_rows_pair(
            components,
            gas,
            cubic_named(eos)?,
            kij_named(mixing_rule)?,
            volume_correction.unwrap_or(true),
        )?
    };
    let t = temperature.map_or(SWELLING_DEFAULT_TEMPERATURE_K, |value| value.value);
    if !(t.is_finite() && t > 0.0) {
        return Err(AzothError::invalid_input(
            "temperature",
            format!("a swelling experiment needs a positive temperature; got {t}"),
        ));
    }

    let original_oil_moles = oil.total_moles();
    let (original_pressure, original_volume) = bubble_point_volume(&oil, t)?;
    let mut points = Vec::with_capacity(cumulative_gas_injected.len());
    let mut current = oil;
    let mut pressure = original_pressure;
    let mut volume = original_volume;
    let mut old = 0.0;
    for cumulative in cumulative_gas_injected {
        let increment = cumulative - old;
        if increment > 0.0 {
            current = current.with_added(&template, original_oil_moles * increment);
            let state = bubble_point_volume(&current, t)?;
            pressure = state.0;
            volume = state.1;
        }
        old = *cumulative;
        points.push(SwellingPoint {
            cumulative_gas_injected: *cumulative,
            pressure,
            relative_oil_volume: volume / original_volume,
        });
    }

    Ok(SwellingTestResult {
        original_oil_moles,
        original_oil_volume: original_volume,
        points,
        warnings: Vec::new(),
    })
}

/// `(bubble pressure, corrected volume)` at the oil's bubble point.
fn bubble_point_volume(fluid: &Fluid, t: f64) -> Result<(f64, f64)> {
    let pressure = bubble_point_pressure(fluid, t)?;
    let flash = fluid.flash(t, pressure, FlashMode::TwoPhase)?;
    Ok((pressure, flash.volume()))
}
