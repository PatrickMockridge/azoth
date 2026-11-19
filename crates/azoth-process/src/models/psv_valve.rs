//! `process.psv_valve` - one step of the API 520 pressure-safety-valve latch.
//!
//! Spec: `specs/models/process/psv_valve.toml`. The arithmetic is
//! `process/safety/depressurization/PsvValveModel.java` at the pin: the open/reseat
//! hysteresis, the pop count and the choked/subcritical orifice relation.

use azoth_core::units::{
    Area, MassRate, MolarMass, Pressure, ThermodynamicTemperature, kilograms_per_second,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's gas constant, J/(mol K).
const GAS_CONSTANT: f64 = 8.314;

/// Result of `process.psv_valve`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PsvValveResult {
    /// The valve's open state after the update.
    pub open: bool,
    /// The pop count after the update.
    pub cycle_count: f64,
    /// The relieving mass flow at the upstream state; zero when closed.
    #[serde(serialize_with = "scalar")]
    pub mass_flow: MassRate,
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

/// The latch and the flow, over SI values, shared with the dynamic sizing study.
///
/// Returns `(open, cycle_count, mass_flow)`. The caller has already applied the class's
/// guards; this is the arithmetic alone.
#[allow(clippy::too_many_arguments)] // one parameter per quantity the class reads
pub(crate) fn step(
    set_pressure: f64,
    blowdown_fraction: f64,
    discharge_coefficient: f64,
    orifice_area: f64,
    back_pressure: f64,
    upstream_pressure: f64,
    upstream_temperature: f64,
    molar_mass: f64,
    heat_capacity_ratio: f64,
    compressibility: f64,
    was_open: bool,
    prior_cycle_count: f64,
) -> (bool, f64, f64) {
    let reseat = set_pressure * (1.0 - blowdown_fraction);
    let mut open = was_open;
    let mut cycle_count = prior_cycle_count;
    if !open && upstream_pressure >= set_pressure {
        open = true;
        cycle_count += 1.0;
    } else if open && upstream_pressure <= reseat {
        open = false;
    }

    let mut mass_flow = 0.0;
    if open {
        let gamma = heat_capacity_ratio;
        let critical_ratio = (2.0 / (gamma + 1.0)).powf(gamma / (gamma - 1.0));
        let pressure_ratio = back_pressure / upstream_pressure;
        mass_flow = if pressure_ratio <= critical_ratio {
            discharge_coefficient
                * orifice_area
                * upstream_pressure
                * (gamma * molar_mass / (compressibility * GAS_CONSTANT * upstream_temperature))
                    .sqrt()
                * (2.0 / (gamma + 1.0)).powf((gamma + 1.0) / (2.0 * (gamma - 1.0)))
        } else {
            let mut term = (2.0 * gamma / (gamma - 1.0))
                * (pressure_ratio.powf(2.0 / gamma) - pressure_ratio.powf((gamma + 1.0) / gamma));
            if term < 0.0 {
                term = 0.0;
            }
            discharge_coefficient
                * orifice_area
                * upstream_pressure
                * (molar_mass / (compressibility * GAS_CONSTANT * upstream_temperature)).sqrt()
                * term.sqrt()
        };
        if mass_flow < 0.0 || mass_flow.is_nan() {
            mass_flow = 0.0;
        }
    }
    (open, cycle_count, mass_flow)
}

/// One step of the PSV latch and its relieving flow.
///
/// # Errors
/// `InvalidInput` for the class's constructor and mass-flow guards, a non-integral pop
/// count, or a non-finite input.
#[allow(clippy::too_many_arguments)] // one parameter per quantity the class reads
pub fn psv_valve(
    set_pressure: Pressure,
    blowdown_fraction: f64,
    discharge_coefficient: f64,
    orifice_area: Area,
    back_pressure: Pressure,
    upstream_pressure: Pressure,
    upstream_temperature: ThermodynamicTemperature,
    molar_mass: MolarMass,
    heat_capacity_ratio: f64,
    compressibility: f64,
    was_open: Option<bool>,
    prior_cycle_count: Option<f64>,
) -> Result<PsvValveResult> {
    let spec = &model_gen::PSV_VALVE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "set_pressure" => Some(set_pressure.value),
            "blowdown_fraction" => Some(blowdown_fraction),
            "discharge_coefficient" => Some(discharge_coefficient),
            "orifice_area" => Some(orifice_area.value),
            "back_pressure" => Some(back_pressure.value),
            "upstream_pressure" => Some(upstream_pressure.value),
            "upstream_temperature" => Some(upstream_temperature.value),
            "molar_mass" => Some(molar_mass.value),
            "heat_capacity_ratio" => Some(heat_capacity_ratio),
            "compressibility" => Some(compressibility),
            "prior_cycle_count" => Some(prior_cycle_count.unwrap_or(0.0)),
            _ => None,
        },
        &mut warnings,
    )?;

    for (name, value) in [
        ("set_pressure", set_pressure.value),
        ("blowdown_fraction", blowdown_fraction),
        ("discharge_coefficient", discharge_coefficient),
        ("orifice_area", orifice_area.value),
        ("back_pressure", back_pressure.value),
        ("upstream_pressure", upstream_pressure.value),
        ("upstream_temperature", upstream_temperature.value),
        ("molar_mass", molar_mass.value),
        ("heat_capacity_ratio", heat_capacity_ratio),
        ("compressibility", compressibility),
    ] {
        require_finite(name, value)?;
    }
    let prior_cycle_count = prior_cycle_count.unwrap_or(0.0);
    require_finite("prior_cycle_count", prior_cycle_count)?;
    if prior_cycle_count.fract() != 0.0 {
        return Err(AzothError::invalid_input(
            "prior_cycle_count",
            "the class's pop counter is a whole number, and a step cannot continue from a fraction of one",
        ));
    }

    if back_pressure.value < 0.0 {
        return Err(AzothError::invalid_input(
            "back_pressure",
            "the class refuses a negative back pressure",
        ));
    }
    if set_pressure.value <= back_pressure.value {
        return Err(AzothError::invalid_input(
            "set_pressure",
            "the class requires the set pressure to exceed the back pressure",
        ));
    }
    if blowdown_fraction <= 0.0 || blowdown_fraction >= 1.0 {
        return Err(AzothError::invalid_input(
            "blowdown_fraction",
            "the class refuses a fraction outside (0, 1)",
        ));
    }
    if discharge_coefficient <= 0.0 {
        return Err(AzothError::invalid_input(
            "discharge_coefficient",
            "the class refuses a non-positive discharge coefficient",
        ));
    }
    if orifice_area.value <= 0.0 {
        return Err(AzothError::invalid_input(
            "orifice_area",
            "the class refuses a non-positive orifice area",
        ));
    }
    if upstream_pressure.value <= 0.0 || upstream_temperature.value <= 0.0 {
        return Err(AzothError::invalid_input(
            "upstream_pressure",
            "the class requires the upstream pressure and temperature to be positive",
        ));
    }
    if molar_mass.value <= 0.0 {
        return Err(AzothError::invalid_input(
            "molar_mass",
            "the class refuses a non-positive molar mass",
        ));
    }
    if heat_capacity_ratio <= 1.0 {
        return Err(AzothError::invalid_input(
            "heat_capacity_ratio",
            "the class refuses an isentropic exponent at or below one",
        ));
    }
    if compressibility <= 0.0 {
        return Err(AzothError::invalid_input(
            "compressibility",
            "the class refuses a non-positive compressibility",
        ));
    }

    let (open, cycle_count, mass_flow) = step(
        set_pressure.value,
        blowdown_fraction,
        discharge_coefficient,
        orifice_area.value,
        back_pressure.value,
        upstream_pressure.value,
        upstream_temperature.value,
        molar_mass.value,
        heat_capacity_ratio,
        compressibility,
        was_open.unwrap_or(false),
        prior_cycle_count,
    );

    Ok(PsvValveResult {
        open,
        cycle_count,
        mass_flow: kilograms_per_second(mass_flow),
        warnings,
    })
}
