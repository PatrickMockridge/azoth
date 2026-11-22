//! `pvt.constant_volume_depletion` - NeqSim's CVD.
//!
//! Spec: `specs/models/pvt/constant_volume_depletion.toml`. The arithmetic is
//! `pvtsimulation/simulation/ConstantVolumeDepletion.java` at the pin: the upward saturation
//! scan on the feed, the restore loop that removes gas until the cell volume is back to its
//! saturation value, the cumulative mole accounting and the liquid dropout.
//!
//! The restore loop's constants are the class's: a `1e-8` relative volume tolerance, 100
//! iterations, and a per-iteration removal capped at `0.95` of the gas phase. Overshoot and a
//! gas-less cell are hard failures there and here. The K-values are this id's own - the
//! per-step equilibrium after restoration - not the class's `calculateKValues()` QC method,
//! which reflashes the final inventory at every pressure.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::ThermodynamicTemperature;
use azoth_core::warning::WarningCode;
use azoth_core::{AzothError, Result, Warning};

use crate::flash::{Flash, FlashMode};
use crate::fluid::{Fluid, cubic_named, kij_named};
use crate::standard::{
    CVD_CELL_VOLUME_RELATIVE_TOLERANCE, CVD_MAX_CELL_VOLUME_ITERATIONS,
    CVD_MAX_GAS_REMOVAL_FRACTION, NEQSIM_GAS_CONSTANT, SATURATION_BISECTION_LIMIT,
    SATURATION_PRESSURE_MAX_BAR, SATURATION_PRESSURE_TOLERANCE_BAR, bar_to_pa,
};

/// One pressure of a CVD.
#[derive(Debug, Clone, PartialEq)]
pub struct CvdPoint {
    /// The pressure, in Pa.
    pub pressure: f64,
    /// The restored cell's translated volume, in m3.
    pub total_volume: f64,
    /// The total volume over the saturation volume.
    pub relative_volume: f64,
    /// The fraction of the initial moles removed through this point.
    pub cumulative_mole_fraction_depleted: f64,
    /// `|initial - remaining - removed|/initial` after restoration; absent where the
    /// accounting is at round-off, below `1e-12`.
    pub material_balance_relative_error: Option<f64>,
    /// The mixture's volume-corrected compressibility.
    pub zmix: f64,
    /// The gas phase's volume-corrected compressibility, when there is one.
    pub z_gas: Option<f64>,
    /// The oil phase's translated volume, when there is one.
    pub liquid_volume: Option<f64>,
    /// The liquid volume over the saturation volume, when there is one.
    pub liquid_volume_relative_to_vsat: Option<f64>,
}

/// Result of `pvt.constant_volume_depletion`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantVolumeDepletionResult {
    /// The saturation pressure on the feed, in Pa.
    pub saturation_pressure: f64,
    /// The translated saturation volume, in m3.
    pub saturation_volume: f64,
    /// The volume-corrected compressibility at saturation.
    pub z_saturation: f64,
    /// The feed's total amount, in mol.
    pub initial_moles: f64,
    /// The total moles removed by the restore loop, in mol.
    pub cumulative_moles_removed: f64,
    /// One row per pressure: the per-step K-values, gas `y` over oil `x`.
    pub k_values: Vec<Vec<f64>>,
    /// One row per pressure.
    pub points: Vec<CvdPoint>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// A CVD over the stated schedule.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown `eos` or `mixing_rule`, no pressures, a
///   non-positive temperature or pressure, a restore loop that overshoots the saturation
///   volume, finds no gas to remove, or fails to converge in 100 iterations - and propagates
///   the flashes' refusals.
pub fn constant_volume_depletion(
    components: &[FluidComponent],
    temperature: ThermodynamicTemperature,
    pressures: &[f64],
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    volume_correction: Option<bool>,
) -> Result<ConstantVolumeDepletionResult> {
    let feed = Fluid::from_rows(
        components,
        cubic_named(eos)?,
        kij_named(mixing_rule)?,
        volume_correction.unwrap_or(true),
    )?;
    let t = positive("temperature", temperature.value)?;
    if pressures.is_empty() {
        return Err(AzothError::invalid_input(
            "pressures",
            "a CVD needs at least one pressure",
        ));
    }
    for (index, pressure) in pressures.iter().enumerate() {
        positive(&format!("pressures[{index}]"), *pressure)?;
    }

    let mut warnings = Vec::new();
    let (saturation_pressure, saturation) = cvd_saturation(&feed, t, &mut warnings)?;
    let saturation_volume = saturation.volume();
    let z_saturation = saturation.zvolcorr();
    let initial_moles = feed.total_moles();

    let mut fluid = feed;
    let mut cumulative_removed = 0.0;
    let mut sampled_hydrocarbon_split = false;
    let mut points = Vec::with_capacity(pressures.len());
    let mut k_values = Vec::with_capacity(pressures.len());

    for pressure in pressures {
        let mut flash = fluid.flash(t, *pressure, FlashMode::TwoPhase)?;
        sampled_hydrocarbon_split |= flash.gas().is_some() && flash.oil().is_some();
        if (flash.phase_count > 1 || (sampled_hydrocarbon_split && *pressure < saturation_pressure))
            && flash.volume() > saturation_volume * (1.0 + CVD_CELL_VOLUME_RELATIVE_TOLERANCE)
        {
            let (restored, removed) = restore(&fluid, saturation_volume, t, *pressure)?;
            fluid = restored;
            cumulative_removed += removed;
            flash = fluid.flash(t, *pressure, FlashMode::TwoPhase)?;
        }

        let total_volume = flash.volume();
        let remaining = fluid.total_moles();
        let oil = flash.oil();
        points.push(CvdPoint {
            pressure: *pressure,
            total_volume,
            relative_volume: total_volume / saturation_volume,
            cumulative_mole_fraction_depleted: 1.0 - remaining / initial_moles,
            material_balance_relative_error: {
                let error = (initial_moles - remaining - cumulative_removed).abs() / initial_moles;
                // **A residual at machine epsilon is not a measurement.** The restore loop
                // converges to `1e-8`; an error four orders below that is the sum of the
                // additions, not the cell's accounting, so the row leaves it absent rather
                // than inviting a relative comparison that divides by the vanishing value.
                (error > 1.0e-12).then_some(error)
            },
            zmix: flash.zvolcorr(),
            z_gas: flash.gas().map(|gas| phase_zvolcorr(gas, *pressure, t)),
            liquid_volume: oil.map(|oil| oil.volume(flash.total_moles)),
            liquid_volume_relative_to_vsat: oil
                .map(|oil| oil.volume(flash.total_moles) / saturation_volume),
        });
        k_values.push(k_values_of(&flash, fluid.mixture.len()));
    }

    Ok(ConstantVolumeDepletionResult {
        saturation_pressure,
        saturation_volume,
        z_saturation,
        initial_moles,
        cumulative_moles_removed: cumulative_removed,
        k_values,
        points,
        warnings,
    })
}

/// One phase's `P v_corr/(R T)`.
fn phase_zvolcorr(phase: &crate::flash::Phase, pressure: f64, t: f64) -> f64 {
    pressure * phase.molar_volume / (NEQSIM_GAS_CONSTANT * t)
}

/// The per-step K-values, gas `y` over oil `x`, zero where the oil holds none; all ones for a
/// single phase.
fn k_values_of(flash: &Flash, components: usize) -> Vec<f64> {
    match (flash.gas(), flash.oil()) {
        (Some(gas), Some(oil)) => gas
            .x
            .iter()
            .zip(&oil.x)
            .map(|(vapour, liquid)| {
                if *liquid <= 1.0e-20 {
                    0.0
                } else {
                    vapour / liquid
                }
            })
            .collect(),
        _ => vec![1.0; components],
    }
}

/// The class's upward scan on the feed, left at the last midpoint.
fn cvd_saturation(fluid: &Fluid, t: f64, warnings: &mut Vec<Warning>) -> Result<(f64, Flash)> {
    let flash = |pressure_bar: f64| -> Result<Flash> {
        fluid.flash(t, bar_to_pa(pressure_bar), FlashMode::TwoPhase)
    };
    let mut pressure_bar = 1.0;
    let mut current;
    loop {
        pressure_bar += 10.0;
        current = flash(pressure_bar)?;
        if !(current.phase_count == 1 && pressure_bar < SATURATION_PRESSURE_MAX_BAR) {
            break;
        }
    }
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
    Ok((bar_to_pa(midpoint), state))
}

/// The restore loop: remove gas until the translated volume is the saturation volume.
///
/// The class returns as soon as the relative residual is inside `1e-8`; it throws on an
/// overshoot (the volume already below the target) or a cell with no gas phase, and a loop
/// that exhausts 100 iterations is a failure there too.
fn restore(fluid: &Fluid, saturation_volume: f64, t: f64, pressure: f64) -> Result<(Fluid, f64)> {
    let mut fluid = fluid.clone();
    let mut removed_total = 0.0;
    for _ in 0..CVD_MAX_CELL_VOLUME_ITERATIONS {
        let flash = fluid.flash(t, pressure, FlashMode::TwoPhase)?;
        let current_volume = flash.volume();
        let residual = (current_volume - saturation_volume) / saturation_volume;
        if residual.abs() <= CVD_CELL_VOLUME_RELATIVE_TOLERANCE {
            return Ok((fluid, removed_total));
        }
        if residual < 0.0 {
            return Err(AzothError::invalid_input(
                "volume",
                format!(
                    "the restore loop overshot the saturation volume: {current_volume} m3 \
                     against {saturation_volume} m3"
                ),
            ));
        }
        let gas = flash.gas().ok_or_else(|| {
            AzothError::invalid_input(
                "flash",
                "the cell exceeds its saturation volume and has no gas phase to remove",
            )
        })?;
        let gas_moles = gas.moles(flash.total_moles);
        let gas_volume = gas.volume(flash.total_moles);
        let to_remove = ((current_volume - saturation_volume) / gas_volume * gas_moles)
            .min(CVD_MAX_GAS_REMOVAL_FRACTION * gas_moles);
        let mut moles = fluid.moles().to_vec();
        for (index, amount) in moles.iter_mut().enumerate() {
            *amount = (*amount - to_remove * gas.x[index]).max(0.0);
        }
        fluid = fluid.with_moles(moles);
        removed_total += to_remove;
    }
    Err(AzothError::invalid_input(
        "volume",
        "the CVD gas removal did not restore the saturation volume in 100 iterations",
    ))
}

fn positive(name: &str, value: f64) -> Result<f64> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            format!("a CVD needs a finite positive value; got {value}"),
        ))
    }
}
