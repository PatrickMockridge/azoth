//! `process.check_valve_leak_relief` - the check-valve leakage relief load.
//!
//! Spec: `specs/models/process/check_valve_leak_relief.toml`. The arithmetic is
//! `process/safety/overpressure/CheckValveLeakRelief.java`'s `calculate` with a fluid: a
//! fraction of the nominal area through `NozzleFlow.gasMassRateKgPerS`.

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

/// The class's default leak fraction.
const DEFAULT_LEAK_AREA_FRACTION: f64 = 0.01;
/// TR3001's recommended 0.1% minimum, which the class clamps up to.
const MINIMUM_LEAK_AREA_FRACTION: f64 = 0.001;
/// The class's default discharge coefficient.
const DEFAULT_DISCHARGE_COEFFICIENT: f64 = 0.85;

/// Result of `process.check_valve_leak_relief`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckValveLeakReliefResult {
    /// The scenario name, echoed.
    pub name: String,
    /// The isentropic nozzle flow through the leak area.
    #[serde(serialize_with = "scalar")]
    pub relief_rate: MassRate,
    /// The relieving phase, which this cause hardcodes to vapour.
    pub phase: ReliefPhase,
    /// The upstream temperature, echoed.
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

/// The leakage rate through a fraction of the nominal check-valve area.
///
/// # Errors
/// `InvalidInput` for a non-positive pressure or temperature, an out-of-range coefficient,
/// a negative fraction, a composition that does not sum to one, or a fluid the databank
/// cannot resolve.
#[allow(clippy::too_many_arguments)] // one argument per declared input
pub fn check_valve_leak_relief(
    components: &[String],
    mole_fractions: &[f64],
    nominal_diameter: Length,
    upstream_pressure: Pressure,
    leak_area_fraction: Option<f64>,
    upstream_temperature: Option<ThermodynamicTemperature>,
    downstream_pressure: Option<Pressure>,
    discharge_coefficient: Option<f64>,
    scenario_name: Option<&str>,
) -> Result<CheckValveLeakReliefResult> {
    let spec = &model_gen::CHECK_VALVE_LEAK_RELIEF_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "nominal_diameter" => Some(nominal_diameter.value),
            "upstream_pressure" => Some(upstream_pressure.value),
            "leak_area_fraction" => leak_area_fraction,
            "upstream_temperature" => upstream_temperature.map(|value| value.value),
            "downstream_pressure" => downstream_pressure.map(|value| value.value),
            "discharge_coefficient" => discharge_coefficient,
            _ => None,
        },
        &mut warnings,
    )?;
    check_composition(mole_fractions, components.len(), "mole_fractions")?;

    let temperature = upstream_temperature.unwrap_or(kelvins(DEFAULT_TEMPERATURE));
    let downstream = downstream_pressure
        .map(|value| value.value)
        .unwrap_or(DEFAULT_DOWNSTREAM_PRESSURE);
    let coefficient = discharge_coefficient.unwrap_or(DEFAULT_DISCHARGE_COEFFICIENT);

    let mut fraction = leak_area_fraction.unwrap_or(DEFAULT_LEAK_AREA_FRACTION);
    let mut fraction_warning = None;
    if fraction < MINIMUM_LEAK_AREA_FRACTION {
        // numerics-ok: CheckValveLeakRelief.java:164 - the class clamps to its recommended minimum.
        fraction = MINIMUM_LEAK_AREA_FRACTION;
        fraction_warning =
            Some("Leak area fraction below TR3001 minimum 0.1%; clamped to 0.001".to_string());
    }

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, Cubic::Pr, None)?;
    let state = relief_fluid_state(
        &mixture,
        &ideal_gas,
        temperature,
        upstream_pressure,
        mole_fractions,
    );

    let mut assumptions = vec![
        format!(
            "Leak area = {:.2}% of nominal flow area per TR3001 SR-26466",
            fraction * 100.0
        ),
        "Upstream pressure taken at high-pressure source MAAP".to_string(),
        format!("Discharge coefficient Cd = {coefficient:.2}"),
    ];
    if let Some(warning) = fraction_warning {
        assumptions.push(warning);
    }
    assumptions.extend(state.assumptions);

    let rate = if nominal_diameter.value <= 0.0 {
        assumptions.push("Nominal check-valve diameter not set; relief rate is zero".to_string());
        0.0
    } else {
        let nominal_area = PI / 4.0 * nominal_diameter.value * nominal_diameter.value;
        let leak_area = fraction * nominal_area;
        gas_mass_rate(
            coefficient,
            leak_area,
            upstream_pressure.value,
            downstream,
            temperature.value,
            state.molar_mass,
            state.specific_heat_ratio,
        )
    };
    Ok(CheckValveLeakReliefResult {
        name: scenario_name.unwrap_or("Check valve leakage").to_string(),
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
