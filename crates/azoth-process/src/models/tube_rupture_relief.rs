//! `process.tube_rupture_relief` - the heat-exchanger tube-rupture relief load.
//!
//! Spec: `specs/models/process/tube_rupture_relief.toml`. The arithmetic is
//! `process/safety/overpressure/TubeRuptureRelief.java`'s `calculate` with a fluid: the
//! open ends' bore through `NozzleFlow.gasMassRateKgPerS`.

use std::f64::consts::PI;

use azoth_core::units::{
    Length, MassRate, MolarMass, Pressure, ThermodynamicTemperature, kelvins, kilograms_per_mole,
    kilograms_per_second,
};
use azoth_core::{Result, Warning, apply_checks};
use azoth_eos::cubic::Cubic;
use azoth_eos::databank;
use azoth_eos::phase_boundary::check_composition;
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::overpressure::{
    DEFAULT_DOWNSTREAM_PRESSURE, DEFAULT_TEMPERATURE, gas_mass_rate, relief_fluid_state,
};
use crate::models::relief_disposal_network::ReliefPhase;

/// The class's default open-end count.
const DEFAULT_OPEN_ENDS: f64 = 2.0;
/// The class's default discharge coefficient.
const DEFAULT_DISCHARGE_COEFFICIENT: f64 = 0.85;

/// Result of `process.tube_rupture_relief`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TubeRuptureReliefResult {
    /// The scenario name, echoed.
    pub name: String,
    /// The isentropic nozzle flow through the rupture area.
    #[serde(serialize_with = "scalar")]
    pub relief_rate: MassRate,
    /// The relieving phase, which this cause hardcodes to vapour.
    pub phase: ReliefPhase,
    /// The high-pressure side's temperature, echoed.
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

/// The full-bore tube-rupture rate through one or two open ends.
///
/// # Errors
/// `InvalidInput` for a non-positive pressure or temperature, an out-of-range coefficient,
/// a count below one, a composition that does not sum to one, or a fluid the databank
/// cannot resolve.
#[allow(clippy::too_many_arguments)] // one argument per declared input
pub fn tube_rupture_relief(
    components: &[String],
    mole_fractions: &[f64],
    tube_inner_diameter: Length,
    high_pressure: Pressure,
    number_of_open_ends: Option<f64>,
    high_temperature: Option<ThermodynamicTemperature>,
    low_pressure: Option<Pressure>,
    discharge_coefficient: Option<f64>,
    scenario_name: Option<&str>,
) -> Result<TubeRuptureReliefResult> {
    let spec = &model_gen::TUBE_RUPTURE_RELIEF_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "tube_inner_diameter" => Some(tube_inner_diameter.value),
            "high_pressure" => Some(high_pressure.value),
            "number_of_open_ends" => number_of_open_ends,
            "high_temperature" => high_temperature.map(|value| value.value),
            "low_pressure" => low_pressure.map(|value| value.value),
            "discharge_coefficient" => discharge_coefficient,
            _ => None,
        },
        &mut warnings,
    )?;
    check_composition(mole_fractions, components.len(), "mole_fractions")?;

    let temperature = high_temperature.unwrap_or(kelvins(DEFAULT_TEMPERATURE));
    let low = low_pressure
        .map(|value| value.value)
        .unwrap_or(DEFAULT_DOWNSTREAM_PRESSURE);
    let coefficient = discharge_coefficient.unwrap_or(DEFAULT_DISCHARGE_COEFFICIENT);
    let ends = number_of_open_ends.unwrap_or(DEFAULT_OPEN_ENDS);

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, Cubic::Pr, None)?;
    let state = relief_fluid_state(
        &mixture,
        &ideal_gas,
        temperature,
        high_pressure,
        mole_fractions,
    );

    let mut assumptions = vec![
        format!("{ends:.0} open ends (full-bore single-tube rupture) per TR3001 SR-26616"),
        "High-pressure fluid at maximum operating pressure".to_string(),
        "Vaporization/flashing on the low-pressure side not modelled (screening estimate)"
            .to_string(),
        format!("Discharge coefficient Cd = {coefficient:.2}"),
    ];
    assumptions.extend(state.assumptions);

    let rate = if tube_inner_diameter.value <= 0.0 {
        assumptions.push("Tube inner diameter not set; relief rate is zero".to_string());
        0.0
    } else {
        let tube_area = PI / 4.0 * tube_inner_diameter.value * tube_inner_diameter.value;
        let area = ends * tube_area;
        gas_mass_rate(
            coefficient,
            area,
            high_pressure.value,
            low,
            temperature.value,
            state.molar_mass,
            state.specific_heat_ratio,
        )
    };
    Ok(TubeRuptureReliefResult {
        name: scenario_name.unwrap_or("Tube rupture").to_string(),
        relief_rate: kilograms_per_second(rate),
        phase: ReliefPhase::Vapour,
        relief_temperature: temperature,
        molar_mass: kilograms_per_mole(state.molar_mass),
        compressibility: state.compressibility,
        specific_heat_ratio: state.specific_heat_ratio,
        assumptions,
        warnings,
    })
}
