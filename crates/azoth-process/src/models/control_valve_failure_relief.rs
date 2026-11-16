//! `process.control_valve_failure_relief` - the fail-open control-valve relief load.
//!
//! Spec: `specs/models/process/control_valve_failure_relief.toml`. The arithmetic is
//! `process/safety/overpressure/ControlValveFailureRelief.java`'s `calculate` with a fluid:
//! the effective area (explicit or from Cv) through `NozzleFlow.gasMassRateKgPerS`.

use azoth_core::units::{
    Area, MassRate, MolarMass, Pressure, ThermodynamicTemperature, kelvins, kilograms_per_mole,
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

/// The class's `CV_TO_AREA_CONSTANT`, `Cv = 24.6 Cd A_in2`.
const CV_TO_AREA_CONSTANT: f64 = 24.6;
/// The class's square inches per square metre.
const IN2_PER_M2: f64 = 1550.0031;
/// The class's default discharge coefficient.
const DEFAULT_DISCHARGE_COEFFICIENT: f64 = 0.85;
/// The class's own assumption sentence.
const BASIS: &str =
    "Fail-open control valve, upstream at maximum operating pressure per TR3001 SR-26501";

/// Result of `process.control_valve_failure_relief`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ControlValveFailureReliefResult {
    /// The scenario name, echoed.
    pub name: String,
    /// The isentropic nozzle flow through the effective area.
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

/// The fail-open rate from an explicit area or a Cv with an erosion margin.
///
/// # Errors
/// `InvalidInput` for a non-positive pressure or temperature, an out-of-range coefficient,
/// a composition that does not sum to one, or a fluid the databank cannot resolve.
#[allow(clippy::too_many_arguments)] // one argument per declared input
pub fn control_valve_failure_relief(
    components: &[String],
    mole_fractions: &[f64],
    upstream_pressure: Pressure,
    upstream_temperature: Option<ThermodynamicTemperature>,
    downstream_pressure: Option<Pressure>,
    effective_area: Option<Area>,
    cv: Option<f64>,
    cv_erosion_margin: Option<f64>,
    discharge_coefficient: Option<f64>,
    scenario_name: Option<&str>,
) -> Result<ControlValveFailureReliefResult> {
    let spec = &model_gen::CONTROL_VALVE_FAILURE_RELIEF_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "upstream_pressure" => Some(upstream_pressure.value),
            "upstream_temperature" => upstream_temperature.map(|value| value.value),
            "downstream_pressure" => downstream_pressure.map(|value| value.value),
            "effective_area" => effective_area.map(|value| value.value),
            "cv" => cv,
            "cv_erosion_margin" => cv_erosion_margin,
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
    let margin = cv_erosion_margin.unwrap_or(1.0);

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
        BASIS.to_string(),
        format!("Discharge coefficient Cd = {coefficient:.2}"),
    ];
    assumptions.extend(state.assumptions);

    let mut area = effective_area.map(|value| value.value).unwrap_or(f64::NAN);
    if !area.is_finite() || area <= 0.0 {
        if let Some(cv) = cv.filter(|value| *value > 0.0) {
            area = (cv * margin / (CV_TO_AREA_CONSTANT * coefficient)) / IN2_PER_M2;
            if margin > 1.0 {
                assumptions.push(format!(
                    "Cv increased by erosion/vibration margin {margin:.2} per TR3001 SR-26501"
                ));
            }
            assumptions.push(
                "Effective area derived from Cv with screening relation Cv = 24.6 Cd A_in2"
                    .to_string(),
            );
        } else {
            assumptions.push("Neither effective area nor Cv set; relief rate is zero".to_string());
            area = 0.0;
        }
    }

    let rate = if area > 0.0 {
        gas_mass_rate(
            coefficient,
            area,
            upstream_pressure.value,
            downstream,
            temperature.value,
            state.molar_mass,
            state.specific_heat_ratio,
        )
    } else {
        0.0
    };
    Ok(ControlValveFailureReliefResult {
        name: scenario_name.unwrap_or("Control valve failure").to_string(),
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
