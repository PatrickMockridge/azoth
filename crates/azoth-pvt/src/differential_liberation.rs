//! `pvt.differential_liberation` - NeqSim's DLE.
//!
//! Spec: `specs/models/pvt/differential_liberation.toml`. The arithmetic is
//! `pvtsimulation/simulation/DifferentialLiberation.java` at the pin: a pressure schedule over
//! a cell whose gas phase is removed after each two-phase step, the remaining solution gas
//! read against the stock-tank oil volume from a final 288.15 K / 1.01325 bar flash.
//!
//! Two class quirks travel: the saturation scan's first loop increments the pressure without
//! a flash and is therefore dead when the scan is entered from a two-phase state (which is the
//! only way `runCalc` enters it), and the saturation volume is the phase carrying the material
//! at the last bisection midpoint, which our flash can label a trace incipient gas.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::ThermodynamicTemperature;
use azoth_core::warning::WarningCode;
use azoth_core::{AzothError, Result, Warning};

use crate::flash::{Flash, FlashMode, Phase};
use crate::fluid::{Fluid, cubic_named, kij_named};
use crate::ledger;
use crate::standard::{
    NEQSIM_AIR_MOLAR_MASS_KG_PER_MOL, SATURATION_BISECTION_LIMIT, SATURATION_PRESSURE_MAX_BAR,
    SATURATION_PRESSURE_TOLERANCE_BAR, STANDARD_PRESSURE_BAR, STANDARD_PRESSURE_PA,
    STANDARD_TEMPERATURE_K, bar_to_pa,
};

/// One pressure of a DLE.
#[derive(Debug, Clone, PartialEq)]
pub struct DlePoint {
    /// The pressure, in Pa.
    pub pressure: f64,
    /// The corrected total volume, in m3.
    pub total_volume: f64,
    /// The total volume over the saturation volume; absent without a saturation state.
    pub relative_volume: Option<f64>,
    /// The oil phase's corrected volume, in m3.
    pub liquid_volume: f64,
    /// The liquid volume over the saturation volume; absent without one.
    pub liquid_volume_relative_to_vsat: Option<f64>,
    /// The oil density, in kg/m3.
    pub oil_density: f64,
    /// The removed gas's corrected volume, when the point is two-phase.
    pub gas_volume: Option<f64>,
    /// That gas at the standard state, in m3.
    pub gas_standard_volume: Option<f64>,
    /// The gas phase's compressibility.
    pub z_gas: Option<f64>,
    /// The gas phase's molar mass over 0.028.
    pub relative_gas_gravity: Option<f64>,
    /// The oil formation volume factor; absent without a stock-tank volume.
    pub bo: Option<f64>,
    /// The gas formation volume factor; absent in a single phase.
    pub bg: Option<f64>,
    /// The remaining solution gas per stock-tank oil volume; absent without one.
    pub rs: Option<f64>,
}

/// Result of `pvt.differential_liberation`.
#[derive(Debug, Clone, PartialEq)]
pub struct DifferentialLiberationResult {
    /// The saturation pressure on the partially depleted cell, in Pa.
    pub saturation_pressure: Option<f64>,
    /// The saturation volume, in m3, translated.
    pub saturation_volume: Option<f64>,
    /// The stock-tank oil volume from the final standard flash, in m3.
    pub voil_std: Option<f64>,
    /// One row per pressure.
    pub points: Vec<DlePoint>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// A DLE over the stated schedule.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown `eos` or `mixing_rule`, no pressures, a
///   non-positive temperature or pressure, or a two-phase point with no gas phase - and
///   propagates the flashes' refusals.
pub fn differential_liberation(
    components: &[FluidComponent],
    temperature: ThermodynamicTemperature,
    pressures: &[f64],
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    volume_correction: Option<bool>,
) -> Result<DifferentialLiberationResult> {
    let feed = Fluid::from_rows(
        components,
        cubic_named(eos)?,
        kij_named(mixing_rule)?,
        volume_correction.unwrap_or(true),
    )?;
    differential_liberation_of(&feed, temperature, pressures)
}

/// The same DLE over a fluid the caller has already built.
///
/// # Errors
/// The same as [`differential_liberation`], less the `eos`/`mixing_rule` refusals.
pub fn differential_liberation_of(
    feed: &Fluid,
    temperature: ThermodynamicTemperature,
    pressures: &[f64],
) -> Result<DifferentialLiberationResult> {
    let t = positive("temperature", temperature.value)?;
    if pressures.is_empty() {
        return Err(AzothError::invalid_input(
            "pressures",
            "a DLE needs at least one pressure",
        ));
    }
    for (index, pressure) in pressures.iter().enumerate() {
        positive(&format!("pressures[{index}]"), *pressure)?;
    }

    let mut warnings = Vec::new();
    let mut fluid = feed.clone();
    let mut saturation: Option<(f64, f64)> = None;
    let mut total_gas_standard = 0.0;
    let mut rows: Vec<Row> = Vec::with_capacity(pressures.len());

    for pressure in pressures {
        let flash = fluid.flash(t, *pressure, FlashMode::TwoPhase)?;
        let volume = flash.volume();
        if !(volume.is_finite() && volume > 0.0) {
            return Err(AzothError::invalid_input(
                "volume",
                format!("the flash at {} Pa has no volume", pressure),
            ));
        }
        let mut row = Row {
            pressure: *pressure,
            total_volume: volume,
            liquid_volume: volume,
            oil_density: flash.mass() / volume,
            gas_volume: None,
            gas_standard_volume: None,
            z_gas: None,
            relative_gas_gravity: None,
        };
        if flash.phase_count > 1 {
            if saturation.is_none() {
                saturation = Some(dle_saturation(&fluid, t, &mut warnings)?);
            }
            let gas = flash.gas().ok_or_else(|| {
                AzothError::invalid_input(
                    "flash",
                    "a two-phase DLE point has no gas phase, and there is nothing to remove",
                )
            })?;
            let gas_standard = gas_standard_volume(gas, flash.total_moles, *pressure, t);
            total_gas_standard += gas_standard;
            row.z_gas = Some(gas.z_factor);
            row.relative_gas_gravity = Some(gas.molar_mass / NEQSIM_AIR_MOLAR_MASS_KG_PER_MOL);
            if let Some(oil) = flash.oil() {
                row.oil_density = oil.density();
                row.liquid_volume = oil.volume(flash.total_moles);
            } else {
                let phase = flash.phases.first().ok_or_else(|| {
                    AzothError::invalid_input("flash", "the point's flash answered no phase")
                })?;
                row.oil_density = phase.density();
                row.liquid_volume = phase.volume(flash.total_moles);
            }
            row.gas_volume = Some(gas.volume(flash.total_moles));
            row.gas_standard_volume = Some(gas_standard);
            fluid = ledger::remove_phase(&fluid, gas)?;
        } else {
            let phase = flash.phases.first().ok_or_else(|| {
                AzothError::invalid_input("flash", "the point's flash answered no phase")
            })?;
            row.oil_density = phase.density();
            row.liquid_volume = phase.volume(flash.total_moles);
        }
        rows.push(row);
    }

    let standard = fluid.flash(
        STANDARD_TEMPERATURE_K,
        STANDARD_PRESSURE_PA,
        FlashMode::TwoPhase,
    )?;
    let voil_std = standard.oil().map(|oil| oil.volume(standard.total_moles));
    if let Some(gas) = standard.gas() {
        total_gas_standard += gas.volume(standard.total_moles);
    } else {
        warnings.push(Warning::new(
            WarningCode::OutOfValidRange,
            "the final standard flash leaves no gas phase; only the removed gas is counted",
        ));
    }
    if voil_std.is_none() {
        warnings.push(Warning::new(
            WarningCode::OutOfValidRange,
            "the final standard flash leaves no oil phase, so Bo, Rs and the stock-tank volume \
             are absent",
        ));
    }
    if saturation.is_none() {
        warnings.push(Warning::new(
            WarningCode::OutOfValidRange,
            "no pressure on the schedule is two-phase, so the saturation state is absent",
        ));
    }

    let mut running = 0.0;
    let mut points = Vec::with_capacity(rows.len());
    for row in rows {
        let relative_volume = saturation.map(|(_, volume)| row.total_volume / volume);
        // The class's array entry is zero for a single-phase point - uninitialised, not a
        // volume - so the row leaves it absent where the step removed no gas.
        let liquid_relative = match (saturation, row.gas_standard_volume) {
            (Some((_, volume)), Some(_)) => Some(row.liquid_volume / volume),
            _ => None,
        };
        let bo = voil_std.map(|volume| row.liquid_volume / volume);
        running += row.gas_standard_volume.unwrap_or(0.0);
        let rs = voil_std.map(|volume| (total_gas_standard - running) / volume);
        let bg = match (row.gas_volume, row.gas_standard_volume, row.z_gas) {
            (Some(volume), Some(standard), Some(z)) if z > 1.0e-10 => Some(volume / standard),
            _ => None,
        };
        points.push(DlePoint {
            pressure: row.pressure,
            total_volume: row.total_volume,
            relative_volume,
            liquid_volume: row.liquid_volume,
            liquid_volume_relative_to_vsat: liquid_relative,
            oil_density: row.oil_density,
            gas_volume: row.gas_volume,
            gas_standard_volume: row.gas_standard_volume,
            z_gas: row.z_gas,
            relative_gas_gravity: row.relative_gas_gravity,
            bo,
            bg,
            rs,
        });
    }

    Ok(DifferentialLiberationResult {
        saturation_pressure: saturation.map(|(pressure, _)| pressure),
        saturation_volume: saturation.map(|(_, volume)| volume),
        voil_std,
        points,
        warnings,
    })
}

struct Row {
    pressure: f64,
    total_volume: f64,
    liquid_volume: f64,
    oil_density: f64,
    gas_volume: Option<f64>,
    gas_standard_volume: Option<f64>,
    z_gas: Option<f64>,
    relative_gas_gravity: Option<f64>,
}

/// The class's standard volume for a removed gas, in m3.
fn gas_standard_volume(gas: &Phase, total_moles: f64, pressure: f64, t: f64) -> f64 {
    gas.volume(total_moles) * (pressure / 1.0e5) / STANDARD_PRESSURE_BAR / gas.z_factor
        * STANDARD_TEMPERATURE_K
        / t
}

/// The class's dead-first-loop scan, left at the last midpoint.
fn dle_saturation(fluid: &Fluid, t: f64, warnings: &mut Vec<Warning>) -> Result<(f64, f64)> {
    let flash = |pressure_bar: f64| -> Result<Flash> {
        fluid.flash(t, bar_to_pa(pressure_bar), FlashMode::TwoPhase)
    };
    // `calcSaturationConditions` is entered only from a two-phase state, so its first loop
    // increments once without a flash and exits on the phase count it did not change.
    let mut pressure_bar = 11.0;
    let mut current;
    loop {
        pressure_bar += 10.0;
        current = flash(pressure_bar)?;
        if !(current.phase_count > 1 && pressure_bar < SATURATION_PRESSURE_MAX_BAR) {
            break;
        }
    }

    let mut min = pressure_bar - 10.0;
    let mut max = pressure_bar;
    let mut iteration = 0;
    let (midpoint, state) = loop {
        let midpoint = (min + max) / 2.0;
        let state = flash(midpoint)?;
        if state.phase_count > 1 {
            min = midpoint;
        } else {
            max = midpoint;
        }
        iteration += 1;
        if (max - min).abs() <= SATURATION_PRESSURE_TOLERANCE_BAR
            || iteration >= SATURATION_BISECTION_LIMIT
        {
            break (midpoint, state);
        }
    };
    if (max - min).abs() > SATURATION_PRESSURE_TOLERANCE_BAR {
        warnings.push(Warning::new(
            WarningCode::SolverNotConverged,
            "the saturation bisection reached its cap; the last midpoint is the answer",
        ));
    }
    // The phase carrying the material, for the reason the CCE's saturation state uses it.
    let phase = state
        .phases
        .iter()
        .max_by(|left, right| left.fraction.total_cmp(&right.fraction))
        .ok_or_else(|| {
            AzothError::invalid_input("flash", "the saturation flash answered no phase")
        })?;
    Ok((bar_to_pa(midpoint), phase.volume(state.total_moles)))
}

fn positive(name: &str, value: f64) -> Result<f64> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            format!("a DLE needs a finite positive value; got {value}"),
        ))
    }
}
