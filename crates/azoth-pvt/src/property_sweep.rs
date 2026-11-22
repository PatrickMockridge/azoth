//! `pvt.property_sweep` - NeqSim's phase-property sweeps.
//!
//! Spec: `specs/models/pvt/property_sweep.toml`. The arithmetic is `DensitySim.java` and
//! `ViscositySim.java` at the pin: a flash at each temperature/pressure pair, the phase
//! densities or the phase viscosities, and the availability flags the viscosity sweep carries.
//!
//! **The wax half of the sweep is owed.** `WaxFractionSim` and `ViscosityWaxOilSim` need
//! NeqSim's TBP wax model (`getWaxModel().addTBPWax()`), whose fluid cannot cross as
//! characterization rows; the class's own model also refuses a databank n-alkane wax former
//! ("Invalid reference-liquid fugacity coefficient for nC20"), so no expressible wax state
//! exists to hold it to. `wax_tbp_model` and `wax_model_parity` are recorded in `ROADMAP.md`
//! rather than a field being invented here.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::{AzothError, Result, Warning};

use crate::flash::FlashMode;
use crate::fluid::{Fluid, cubic_named, kij_named};
use crate::transport::{TransportKind, phase_viscosity};

/// One temperature/pressure point of a sweep.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyPoint {
    /// The temperature, in K.
    pub temperature: f64,
    /// The pressure, in Pa.
    pub pressure: f64,
    /// The gas phase's density, in kg/m3, in the density mode.
    pub gas_density: Option<f64>,
    /// The oil phase's density, in kg/m3, in the density mode.
    pub oil_density: Option<f64>,
    /// The aqueous phase's density, in kg/m3, in the density mode.
    pub aqueous_density: Option<f64>,
    /// The gas phase's viscosity, in Pa s, in the viscosity mode.
    pub gas_viscosity: Option<f64>,
    /// The oil phase's viscosity, in Pa s, in the viscosity mode.
    pub oil_viscosity: Option<f64>,
    /// The aqueous phase's viscosity, in Pa s, in the viscosity mode.
    pub aqueous_viscosity: Option<f64>,
    /// Whether a gas phase is present.
    pub gas_phase_available: bool,
    /// Whether an oil phase is present.
    pub oil_phase_available: bool,
    /// Whether an aqueous phase is present.
    pub aqueous_phase_available: bool,
}

/// Result of `pvt.property_sweep`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertySweepResult {
    /// One row per temperature/pressure pair.
    pub points: Vec<PropertyPoint>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// A density or viscosity sweep over a grid of states.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown `mode`, `eos` or `mixing_rule`, empty or
///   unequal temperature and pressure arrays, a non-positive temperature or pressure, or an
///   unavailable phase property - and propagates the flashes' refusals.
pub fn property_sweep(
    components: &[FluidComponent],
    mode: &str,
    temperatures: &[f64],
    pressures: &[f64],
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    volume_correction: Option<bool>,
) -> Result<PropertySweepResult> {
    let fluid = Fluid::from_rows(
        components,
        cubic_named(eos)?,
        kij_named(mixing_rule)?,
        volume_correction.unwrap_or(true),
    )?;
    if temperatures.is_empty() {
        return Err(AzothError::invalid_input(
            "temperatures",
            "a property sweep needs at least one state",
        ));
    }
    if temperatures.len() != pressures.len() {
        return Err(AzothError::invalid_input(
            "pressures",
            format!(
                "the schedule has {} temperatures and {} pressures",
                temperatures.len(),
                pressures.len()
            ),
        ));
    }
    if mode != "density" && mode != "viscosity" {
        return Err(AzothError::invalid_input(
            "mode",
            format!(
                "{mode} is not one of density and viscosity; the wax modes are owed with the \
                 class's TBP wax model"
            ),
        ));
    }
    let mut points = Vec::with_capacity(temperatures.len());
    for (temperature, pressure) in temperatures.iter().zip(pressures) {
        if !(temperature.is_finite() && *temperature > 0.0) {
            return Err(AzothError::invalid_input(
                "temperatures",
                format!("a sweep temperature is {temperature}"),
            ));
        }
        if !(pressure.is_finite() && *pressure > 0.0) {
            return Err(AzothError::invalid_input(
                "pressures",
                format!("a sweep pressure is {pressure}"),
            ));
        }
        let flash = fluid.flash(*temperature, *pressure, FlashMode::TwoPhase)?;
        let oil = flash.oil();
        let aqueous = flash.aqueous();
        let gas = flash.gas();
        let (gas_density, oil_density, aqueous_density) = if mode == "density" {
            (
                gas.map(crate::flash::Phase::density),
                oil.map(crate::flash::Phase::density),
                aqueous.map(crate::flash::Phase::density),
            )
        } else {
            (None, None, None)
        };
        let viscosity =
            |phase: Option<&crate::flash::Phase>, kind: TransportKind| -> Result<Option<f64>> {
                phase
                    .map(|phase| {
                        phase_viscosity(&fluid.mixture, *temperature, *pressure, &phase.x, kind)
                    })
                    .transpose()
            };
        let (gas_viscosity, oil_viscosity, aqueous_viscosity) = if mode == "viscosity" {
            (
                viscosity(gas, TransportKind::Gas)?,
                viscosity(oil, TransportKind::Oil)?,
                viscosity(aqueous, TransportKind::Aqueous)?,
            )
        } else {
            (None, None, None)
        };
        points.push(PropertyPoint {
            temperature: *temperature,
            pressure: *pressure,
            gas_density,
            oil_density,
            aqueous_density,
            gas_viscosity,
            oil_viscosity,
            aqueous_viscosity,
            gas_phase_available: gas.is_some(),
            oil_phase_available: oil.is_some(),
            aqueous_phase_available: aqueous.is_some(),
        });
    }
    Ok(PropertySweepResult {
        points,
        warnings: Vec::new(),
    })
}
