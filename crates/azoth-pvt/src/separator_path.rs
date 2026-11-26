//! `pvt.separator_path` - the staged separator cascade onto one declared final basis.
//!
//! Spec: `specs/models/pvt/separator_path.toml`. The arithmetic is
//! `pvtsimulation/util/PvtSeparatorPath.java` at the pin: each stage flashes the carried
//! fluid, the gas leaves and is flashed at the path's final pressure and temperature, and
//! the oil and aqueous phases carry to the next stage.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{Pressure, ThermodynamicTemperature};
use azoth_core::{AzothError, Result, Warning, WarningCode};
use azoth_eos::hydrate_inhibitor_wt::PhaseLabel;

use crate::flash::{Flash, FlashMode};
use crate::fluid::{Fluid, cubic_named, kij_named};

/// The three product phases, in the class's gas/oil/aqueous order.
const PHASES: usize = 3;

/// One stage path's final-condition products and native component amounts.
#[derive(Debug, Clone, PartialEq)]
pub struct SeparatorPathResult {
    /// The declared final volume-basis pressure, in Pa.
    pub final_pressure: f64,
    /// The declared final volume-basis temperature, in K.
    pub final_temperature: f64,
    /// The gas, oil and aqueous corrected volumes, in m3, in that order.
    pub volumes: Vec<f64>,
    /// The three phases' component amounts, in mol, phase by component.
    pub component_moles: Vec<Vec<f64>>,
    /// The gas/oil volume ratio at the final basis; absent when no oil phase is present.
    pub gas_oil_ratio: Option<f64>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

fn phase_index(label: PhaseLabel) -> Option<usize> {
    match label {
        PhaseLabel::Gas => Some(0),
        PhaseLabel::Oil => Some(1),
        PhaseLabel::Aqueous => Some(2),
    }
}

fn accumulate(
    flash: &Flash,
    volumes: &mut [f64; PHASES],
    component_moles: &mut [Vec<f64>],
) -> Result<()> {
    let total = flash.total_moles;
    for phase in &flash.phases {
        let index = phase_index(phase.label).ok_or_else(|| {
            AzothError::invalid_input(
                "components",
                "an unsupported or nonphysical separator product phase",
            )
        })?;
        let volume = phase.volume(total);
        if !volume.is_finite() || volume <= 0.0 {
            return Err(AzothError::invalid_input(
                "components",
                "an unsupported or nonphysical separator product phase",
            ));
        }
        volumes[index] += volume;
        for (component, fraction) in phase.x.iter().enumerate() {
            let amount = phase.fraction * total * fraction;
            if !amount.is_finite() || amount < 0.0 {
                return Err(AzothError::invalid_input(
                    "components",
                    "an invalid separator product component amount",
                ));
            }
            component_moles[index][component] += amount;
        }
    }
    Ok(())
}

/// Flash once and return the phase amounts, in the mixture's component order.
fn phase_moles(flash: &Flash, label: PhaseLabel) -> Option<Vec<f64>> {
    let phase = flash.phases.iter().find(|phase| phase.label == label)?;
    Some(
        phase
            .x
            .iter()
            .map(|fraction| phase.fraction * flash.total_moles * fraction)
            .collect(),
    )
}

/// A staged separator path onto its declared final volume basis.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for no stages, a non-positive or non-finite pressure or
///   temperature, pressures that are not strictly decreasing, an unsupported product phase,
///   a component balance that does not close, or a non-finite accumulated volume - and
///   propagates the fluid build and the flashes' refusals.
#[allow(clippy::too_many_arguments)]
pub fn separator_path(
    components: &[FluidComponent],
    pressures: &[Pressure],
    temperatures: &[ThermodynamicTemperature],
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    volume_correction: Option<bool>,
    multiphase: Option<bool>,
) -> Result<SeparatorPathResult> {
    if pressures.is_empty() || pressures.len() != temperatures.len() {
        return Err(AzothError::invalid_input(
            "pressures",
            "equal nonempty pressure and temperature arrays are required",
        ));
    }
    for (index, (pressure, temperature)) in pressures.iter().zip(temperatures).enumerate() {
        if !pressure.value.is_finite()
            || pressure.value <= 0.0
            || !temperature.value.is_finite()
            || temperature.value <= 0.0
            || (index > 0 && pressure.value >= pressures[index - 1].value)
        {
            return Err(AzothError::invalid_input(
                "pressures",
                "finite positive temperatures and strictly decreasing positive pressures are required",
            ));
        }
    }

    let fluid = Fluid::from_rows(
        components,
        cubic_named(eos)?,
        kij_named(mixing_rule)?,
        volume_correction.unwrap_or(true),
    )?;
    let initial = fluid.moles().to_vec();
    let count = initial.len();
    let mode = if multiphase.unwrap_or(false) {
        FlashMode::Multiphase
    } else {
        FlashMode::TwoPhase
    };
    let last = pressures.len() - 1;
    let final_pressure = pressures[last].value;
    let final_temperature = temperatures[last].value;

    let mut volumes = [0.0_f64; PHASES];
    let mut totals = vec![vec![0.0_f64; count]; PHASES];
    let mut current = fluid.clone();
    for stage in 0..=last {
        let flash = current.flash(temperatures[stage].value, pressures[stage].value, mode)?;
        if stage == last {
            accumulate(&flash, &mut volumes, &mut totals)?;
            break;
        }
        if flash.gas().is_some() {
            let moles =
                phase_moles(&flash, PhaseLabel::Gas).expect("the flash reported a gas phase");
            let gas = current.with_moles(moles);
            let condensed = gas.flash(final_temperature, final_pressure, mode)?;
            accumulate(&condensed, &mut volumes, &mut totals)?;
        }
        let oil = phase_moles(&flash, PhaseLabel::Oil);
        let aqueous = phase_moles(&flash, PhaseLabel::Aqueous);
        let mut remaining = vec![0.0_f64; count];
        let mut any = false;
        for phase in [oil, aqueous].into_iter().flatten() {
            for (component, amount) in phase.iter().enumerate() {
                remaining[component] += amount;
            }
            any = true;
        }
        if !any {
            break;
        }
        current = current.with_moles(remaining);
    }

    for (component, amount) in initial.iter().enumerate() {
        let accounted: f64 = (0..PHASES).map(|phase| totals[phase][component]).sum();
        if !accounted.is_finite() || (amount - accounted).abs() > 1.0e-8 * amount.max(1.0) {
            return Err(AzothError::invalid_input(
                "components",
                "the separator native component balance failed",
            ));
        }
    }
    if volumes.iter().any(|volume| !volume.is_finite()) {
        return Err(AzothError::invalid_input(
            "components",
            "a non-finite accumulated separator product volume",
        ));
    }

    let mut warnings = Vec::new();
    let gas_oil_ratio = if volumes[1] > 0.0 {
        Some(volumes[0] / volumes[1])
    } else {
        warnings.push(Warning::new(
            WarningCode::OutOfValidRange,
            "the path has no stock-tank oil, so its gas/oil ratio is absent rather than NaN",
        ));
        None
    };
    Ok(SeparatorPathResult {
        final_pressure,
        final_temperature,
        volumes: volumes.to_vec(),
        component_moles: totals,
        gas_oil_ratio,
        warnings,
    })
}
