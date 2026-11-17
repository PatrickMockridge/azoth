//! `process.blocked_outlet_relief` - the blocked-outlet relief load.
//!
//! Spec: `specs/models/process/blocked_outlet_relief.toml`. The arithmetic is
//! `process/safety/overpressure/BlockedOutletRelief.java`'s `calculate` with a fluid: the
//! relief rate is the full inflow, and the relieving state is `ReliefFluidState.evaluate`.

use azoth_core::units::{
    MassRate, MolarMass, Pressure, ThermodynamicTemperature, kilograms_per_mole,
    kilograms_per_second,
};
use azoth_core::{Result, Warning, apply_checks};
use azoth_eos::cubic::Cubic;
use azoth_eos::databank;
use azoth_eos::phase_boundary::check_composition;
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::overpressure::relief_fluid_state;
use crate::models::relief_disposal_network::ReliefPhase;

/// The class's own assumption sentence.
const BASIS: &str = "Relief rate set equal to full inflow per TR3001 SR-26460";

/// Result of `process.blocked_outlet_relief`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlockedOutletReliefResult {
    /// The scenario name, echoed.
    pub name: String,
    /// The inflow rate, which is the relief rate.
    #[serde(serialize_with = "scalar")]
    pub relief_rate: MassRate,
    /// The relieving phase the flash classified.
    pub phase: ReliefPhase,
    /// The relieving temperature, echoed.
    #[serde(serialize_with = "scalar")]
    pub relief_temperature: ThermodynamicTemperature,
    /// The relieving fluid's molar mass.
    #[serde(serialize_with = "scalar")]
    pub molar_mass: MolarMass,
    /// The relieving fluid's `Z`.
    pub compressibility: f64,
    /// The relieving fluid's `getKappa`, the isentropic exponent.
    pub specific_heat_ratio: f64,
    /// The class's assumption strings, in order.
    pub assumptions: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The full-inflow relief rate and the relieving-fluid state at it.
///
/// # Errors
/// `InvalidInput` for a negative inflow, a non-positive pressure or temperature, a
/// composition that does not sum to one, or a fluid the databank cannot resolve.
#[allow(clippy::too_many_arguments)] // one argument per declared input
pub fn blocked_outlet_relief(
    components: &[String],
    mole_fractions: &[f64],
    inflow_rate: MassRate,
    relief_pressure: Pressure,
    relief_temperature: ThermodynamicTemperature,
    scenario_name: Option<&str>,
) -> Result<BlockedOutletReliefResult> {
    let spec = &model_gen::BLOCKED_OUTLET_RELIEF_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "inflow_rate" => Some(inflow_rate.value),
            "relief_pressure" => Some(relief_pressure.value),
            "relief_temperature" => Some(relief_temperature.value),
            _ => None,
        },
        &mut warnings,
    )?;
    check_composition(mole_fractions, components.len(), "mole_fractions")?;

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, Cubic::Pr, None)?;
    let state = relief_fluid_state(
        &mixture,
        &ideal_gas,
        relief_temperature,
        relief_pressure,
        mole_fractions,
    );
    let mut assumptions = vec![BASIS.to_string()];
    if inflow_rate.value <= 0.0 {
        assumptions.push("Inflow rate not set or non-positive; relief rate is zero".to_string());
    }
    assumptions.extend(state.assumptions);
    Ok(BlockedOutletReliefResult {
        name: scenario_name.unwrap_or("Blocked outlet").to_string(),
        relief_rate: kilograms_per_second(inflow_rate.value),
        phase: state.phase,
        relief_temperature,
        molar_mass: kilograms_per_mole(state.molar_mass),
        compressibility: state.compressibility,
        specific_heat_ratio: state.specific_heat_ratio,
        assumptions,
        warnings,
    })
}
