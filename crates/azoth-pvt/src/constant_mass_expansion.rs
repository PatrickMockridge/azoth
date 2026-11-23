//! `pvt.constant_mass_expansion` - NeqSim's CCE.
//!
//! Spec: `specs/models/pvt/constant_mass_expansion.toml`. The arithmetic is
//! `pvtsimulation/simulation/ConstantMassExpansion.java` at the pin: the upward saturation
//! scan, the relative volume against the saturation volume, the Y-factor, the isothermal
//! compressibility and the class's `Bg`.
//!
//! **The class's saturation state is the last bisection midpoint**, not the refined
//! single-phase edge the `SaturationPressure` class returns; the two differ by up to `1e-5`
//! bar and the capture holds this id to the midpoint. Its saturation and total volumes are
//! uncorrected, while the phase densities and the gas volume are the volume-corrected ones.
//! The `1e5` in the class's `Bg` compensates for NeqSim's `1e5`-scaled internal volume in the
//! same expression's standard volume, so in SI the ratio is the physical one.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{ThermodynamicTemperature, kelvins, pascals};
use azoth_core::warning::WarningCode;
use azoth_core::{AzothError, Result, Warning};

use crate::flash::{Flash, FlashMode, Phase};
use crate::fluid::{Fluid, cubic_named, kij_named};
use crate::standard::{
    NEQSIM_GAS_CONSTANT, SATURATION_BISECTION_LIMIT, SATURATION_PRESSURE_MAX_BAR,
    SATURATION_PRESSURE_TOLERANCE_BAR, STANDARD_PRESSURE_PA, STANDARD_TEMPERATURE_K, bar_to_pa,
};
use crate::transport::{TransportKind, phase_viscosity};
use crate::volume;

/// One pressure of a CCE.
#[derive(Debug, Clone, PartialEq)]
pub struct CcePoint {
    /// The pressure, in Pa.
    pub pressure: f64,
    /// The untranslated total volume, in m3.
    pub total_volume: f64,
    /// The total volume over the saturation volume.
    pub relative_volume: f64,
    /// Phase 0's mass density, in kg/m3.
    pub density: f64,
    /// Phase 0's corrected extensive volume, the class's "gas volume", in m3.
    pub gas_volume: f64,
    /// The standard-state volume at 288.15 K and 1.01325 bar, in m3.
    pub gas_standard_volume: f64,
    /// The class's `Bg`, the corrected phase volume over the standard volume.
    pub bg: f64,
    /// Phase 0's cubic compressibility.
    pub z_gas: f64,
    /// The oil phase's density, when the flash labels one.
    pub oil_density: Option<f64>,
    /// The oil phase's viscosity, when the flash labels one.
    pub oil_viscosity: Option<f64>,
    /// The oil phase's untranslated volume over the saturation volume.
    pub liquid_relative_volume: Option<f64>,
    /// The class's Y-factor, present only in the two-phase region.
    pub y_factor: Option<f64>,
    /// Phase 0's viscosity, present only for a single-phase point.
    pub viscosity: Option<f64>,
    /// Phase 0's isothermal compressibility, in 1/Pa, present only for a single phase.
    pub isothermal_compressibility: Option<f64>,
}

/// Result of `pvt.constant_mass_expansion`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantMassExpansionResult {
    /// The saturation pressure at the last bisection midpoint, in Pa.
    pub saturation_pressure: f64,
    /// The untranslated saturation volume, in m3.
    pub saturation_volume: f64,
    /// `sum_k beta_k Z_k` at the saturation state.
    pub z_saturation: f64,
    /// The saturation state's isothermal compressibility, in 1/Pa.
    pub saturation_isothermal_compressibility: f64,
    /// One row per pressure.
    pub points: Vec<CcePoint>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The CCE of a fluid over a pressure schedule.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown `eos` or `mixing_rule`, no pressures, a
///   non-positive temperature or pressure, or a phase whose properties cannot be read - and
///   propagates the flash's refusals.
pub fn constant_mass_expansion(
    components: &[FluidComponent],
    temperature: ThermodynamicTemperature,
    pressures: &[f64],
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    volume_correction: Option<bool>,
) -> Result<ConstantMassExpansionResult> {
    let fluid = Fluid::from_rows(
        components,
        cubic_named(eos)?,
        kij_named(mixing_rule)?,
        volume_correction.unwrap_or(true),
    )?;
    constant_mass_expansion_of(&fluid, temperature, pressures)
}

/// The same CCE over a fluid the caller has already built.
///
/// The PVT regression builds one fluid per parameter vector and asks this kernel for the
/// schedule, rather than rebuilding the rows for every point.
///
/// # Errors
/// The same as [`constant_mass_expansion`], less the `eos`/`mixing_rule` refusals.
pub fn constant_mass_expansion_of(
    fluid: &Fluid,
    temperature: ThermodynamicTemperature,
    pressures: &[f64],
) -> Result<ConstantMassExpansionResult> {
    let t = positive("temperature", temperature.value)?;
    if pressures.is_empty() {
        return Err(AzothError::invalid_input(
            "pressures",
            "a CCE needs at least one pressure",
        ));
    }
    for (index, pressure) in pressures.iter().enumerate() {
        positive(&format!("pressures[{index}]"), *pressure)?;
    }

    let (saturation_pressure, saturation, warnings) = saturation_state(fluid, t)?;
    let saturation_volume = saturation.volume_uncorrected();
    let z_saturation = saturation.z_mix();
    // **The saturation state's phase 0 is the phase carrying the material.** NeqSim's flash
    // at the class's midpoint is single-phase there, and its phase 0 is the liquid; our flash
    // can leave a trace incipient gas at the same pressure, whose compressibility is not the
    // state's. The largest phase is the one the class is looking at.
    let first = saturation
        .phases
        .iter()
        .max_by(|left, right| left.fraction.total_cmp(&right.fraction))
        .ok_or_else(|| {
            AzothError::invalid_input("flash", "the saturation flash answered no phase")
        })?;
    let reduced = fluid
        .mixture
        .reduced_parameters(kelvins(t), pascals(saturation_pressure))?;
    let saturation_isothermal_compressibility = volume::isothermal_compressibility(
        &fluid.mixture,
        &reduced,
        &first.x,
        first.z_factor,
        t,
        saturation_pressure,
    );
    let mut points = Vec::with_capacity(pressures.len());
    for pressure in pressures {
        let flash = fluid.flash(t, *pressure, FlashMode::TwoPhase)?;
        let phase = flash.phases.first().ok_or_else(|| {
            AzothError::invalid_input("flash", "the point's flash answered no phase")
        })?;
        let total_volume = flash.volume_uncorrected();
        let relative_volume = total_volume / saturation_volume;
        let gas_volume = phase.volume(flash.total_moles);
        let gas_standard_volume =
            phase.moles(flash.total_moles) * NEQSIM_GAS_CONSTANT * STANDARD_TEMPERATURE_K
                / STANDARD_PRESSURE_PA;
        let oil = flash.oil();

        let (viscosity, isothermal_compressibility, liquid_relative_volume, y_factor) =
            if flash.phase_count > 1 {
                let oil = oil.ok_or_else(|| {
                    AzothError::invalid_input(
                        "flash",
                        "a two-phase CCE point has no oil phase, so its liquid volume is absent",
                    )
                })?;
                let liquid = oil.volume_uncorrected(flash.total_moles) / saturation_volume;
                let y = ((saturation_pressure - pressure) / pressure)
                    / ((total_volume - saturation_volume) / saturation_volume);
                (None, None, Some(liquid), Some(y))
            } else {
                let reduced = fluid
                    .mixture
                    .reduced_parameters(kelvins(t), pascals(*pressure))?;
                let compressibility = volume::isothermal_compressibility(
                    &fluid.mixture,
                    &reduced,
                    &phase.x,
                    phase.z_factor,
                    t,
                    *pressure,
                );
                let viscosity = phase_viscosity(
                    &fluid.mixture,
                    t,
                    *pressure,
                    &phase.x,
                    transport_kind(phase),
                )?;
                (Some(viscosity), Some(compressibility), None, None)
            };

        let (oil_density, oil_viscosity) = match oil {
            Some(oil) => (
                Some(oil.density()),
                Some(phase_viscosity(
                    &fluid.mixture,
                    t,
                    *pressure,
                    &oil.x,
                    TransportKind::Oil,
                )?),
            ),
            None => (None, None),
        };

        points.push(CcePoint {
            pressure: *pressure,
            total_volume,
            relative_volume,
            density: phase.density(),
            gas_volume,
            gas_standard_volume,
            bg: gas_volume / gas_standard_volume,
            z_gas: phase.z_factor,
            oil_density,
            oil_viscosity,
            liquid_relative_volume,
            y_factor,
            viscosity,
            isothermal_compressibility,
        });
    }

    Ok(ConstantMassExpansionResult {
        saturation_pressure,
        saturation_volume,
        z_saturation,
        saturation_isothermal_compressibility,
        points,
        warnings,
    })
}

fn transport_kind(phase: &Phase) -> TransportKind {
    match phase.label {
        azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Gas => TransportKind::Gas,
        azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Oil => TransportKind::Oil,
        azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Aqueous => TransportKind::Aqueous,
    }
}

/// The class's upward scan and bisection, left at the last midpoint.
///
/// The two loops are the class's do-while sequence: 10-bar steps while single phase, then
/// while two phases, then a bisection whose final state is the last midpoint evaluated.
fn saturation_state(fluid: &Fluid, t: f64) -> Result<(f64, Flash, Vec<Warning>)> {
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
    let warnings = if (max - min).abs() > SATURATION_PRESSURE_TOLERANCE_BAR {
        vec![Warning::new(
            WarningCode::SolverNotConverged,
            "the saturation bisection reached its cap; the last midpoint is the answer",
        )]
    } else {
        Vec::new()
    };
    Ok((bar_to_pa(midpoint), state, warnings))
}

fn positive(name: &str, value: f64) -> Result<f64> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            format!("a CCE needs a finite positive value; got {value}"),
        ))
    }
}
