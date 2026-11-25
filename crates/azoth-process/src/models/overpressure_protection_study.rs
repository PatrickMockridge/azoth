//! `process.overpressure_protection_study` - the governing case, its sizing and the
//! accumulated-pressure verdict.
//!
//! Spec: `specs/models/process/overpressure_protection_study.toml`. The arithmetic is
//! `process/safety/overpressure/OverpressureProtectionStudy.java`'s `evaluate`: the
//! governing-case selection, the three sizing branches over the ported API 520 arithmetic,
//! and the accumulated-pressure check.

use azoth_core::units::{
    Area, DynamicViscosity, MassDensity, MassRate, MolarMass, Pressure, ThermodynamicTemperature,
    kilograms_per_second, pascals, square_meters,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::overpressure_acceptance::overpressure_acceptance;
use crate::models::psv_sizing::{
    gas_area, liquid_area, relieving_pressure, select_orifice, two_phase_area,
};
use crate::models::relief_disposal_network::{ReliefCause, ReliefPhase};

/// The class's default fire overpressure basis.
const FIRE_OVERPRESSURE: f64 = 0.21;
/// The class's default non-fire overpressure basis.
const NON_FIRE_OVERPRESSURE: f64 = 0.10;
/// The class's liquid-density fallback, kg/m3.
const LIQUID_DENSITY_FALLBACK: f64 = 700.0;
/// The class's liquid-viscosity fallback, Pa s.
const LIQUID_VISCOSITY_FALLBACK: f64 = 5.0e-4;
/// The class's default relieving temperature, K.
const DEFAULT_TEMPERATURE: f64 = 288.15;
/// Square metres per square inch.
const M2_PER_IN2: f64 = 6.4516e-4;
/// The default back pressure, Pa.
const DEFAULT_BACK_PRESSURE: f64 = 101_325.0;

/// One documented relief scenario, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct OverpressureScenario {
    /// The scenario's name, echoed on the governing row.
    pub name: String,
    /// The overpressure cause, the class's spelling.
    pub cause: String,
    /// The relieving phase, the class's spelling.
    pub phase: String,
    /// The required relief rate.
    pub relief_rate: MassRate,
    /// The relieving temperature.
    pub relief_temperature: Option<ThermodynamicTemperature>,
    /// The relieving fluid's molar mass.
    pub molar_mass: Option<MolarMass>,
    /// The relieving fluid's `Z`.
    pub compressibility: Option<f64>,
    /// The relieving fluid's isentropic exponent.
    pub specific_heat_ratio: Option<f64>,
    /// The liquid's density.
    pub density: Option<MassDensity>,
    /// The liquid's dynamic viscosity.
    pub viscosity: Option<DynamicViscosity>,
    /// The two-phase inlet's gas mass fraction.
    pub gas_mass_fraction: Option<f64>,
    /// The two-phase gas density.
    pub gas_density: Option<MassDensity>,
    /// The two-phase liquid density.
    pub liquid_density: Option<MassDensity>,
    /// The latent heat, J/kg.
    pub latent_heat: Option<f64>,
    /// The liquid heat capacity, J/(kg K).
    pub liquid_heat_capacity: Option<f64>,
    /// Whether the scenario may govern.
    pub credible: Option<bool>,
}

/// Result of `process.overpressure_protection_study`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OverpressureProtectionStudyResult {
    /// The governing scenario's name.
    pub governing_name: Option<String>,
    /// The governing cause's spelling.
    pub governing_cause: Option<String>,
    /// The governing phase's spelling.
    pub governing_phase: Option<String>,
    /// The governing scenario's relief rate.
    #[serde(serialize_with = "optional_scalar")]
    pub governing_relief_rate: Option<MassRate>,
    /// The required orifice area.
    #[serde(serialize_with = "optional_scalar")]
    pub required_area: Option<Area>,
    /// The required area in square inches.
    pub required_area_in2: Option<f64>,
    /// The first standard orifice at or above the requirement.
    pub recommended_orifice: Option<String>,
    /// The selected standard orifice's area in square inches.
    pub selected_area_in2: Option<f64>,
    /// `rate selected / required`.
    #[serde(serialize_with = "optional_scalar")]
    pub selected_orifice_capacity: Option<MassRate>,
    /// Whether the selected orifice is at least the required area.
    pub capacity_adequate: bool,
    /// The peak pressure the acceptance compared.
    #[serde(serialize_with = "optional_scalar")]
    pub acceptance_peak_pressure: Option<Pressure>,
    /// `mawp accumulation_factor`.
    #[serde(serialize_with = "optional_scalar")]
    pub acceptance_allowable_pressure: Option<Pressure>,
    /// The accumulation factor.
    pub acceptance_accumulation_factor: Option<f64>,
    /// `allowable - peak`.
    #[serde(serialize_with = "optional_scalar")]
    pub acceptance_margin: Option<Pressure>,
    /// Whether the peak is at or below the allowable pressure.
    pub accepted: Option<bool>,
    /// The class's message strings, in order.
    pub messages: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The governing case, its sizing and the accumulated-pressure verdict.
///
/// # Errors
/// `InvalidInput` for a non-positive MAWP or set pressure, a negative back pressure, a
/// scenario whose cause or phase is not one of the class's, a stated non-positive property,
/// a gas or two-phase governing case with no molar mass, or a composition the acceptance
/// refuses.
pub fn overpressure_protection_study(
    mawp: Pressure,
    scenarios: &[OverpressureScenario],
    relief_set_pressure: Option<Pressure>,
    back_pressure: Option<Pressure>,
    multiple_devices: Option<bool>,
) -> Result<OverpressureProtectionStudyResult> {
    let spec = &model_gen::OVERPRESSURE_PROTECTION_STUDY_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "mawp" => Some(mawp.value),
            "relief_set_pressure" => relief_set_pressure.map(|value| value.value),
            "back_pressure" => back_pressure.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;
    let set_pressure = relief_set_pressure.unwrap_or(mawp);
    let back = back_pressure.map_or(DEFAULT_BACK_PRESSURE, |value| value.value);
    let multiple = multiple_devices.unwrap_or(false);

    // The class's builder defaults, and the refusals a non-physical stated value earns.
    for scenario in scenarios {
        let _ = ReliefCause::parse(&scenario.cause)?;
        let _ = ReliefPhase::parse(&scenario.phase)?;
        if !scenario.relief_rate.value.is_finite() || scenario.relief_rate.value < 0.0 {
            return Err(AzothError::invalid_input(
                "scenarios",
                format!(
                    "scenario `{}` states relief_rate = {}, which is not a physical rate",
                    scenario.name, scenario.relief_rate.value
                ),
            ));
        }
        for (name, value) in [
            (
                "relief_temperature",
                scenario.relief_temperature.map(|value| value.value),
            ),
            ("molar_mass", scenario.molar_mass.map(|value| value.value)),
            ("compressibility", scenario.compressibility),
            ("specific_heat_ratio", scenario.specific_heat_ratio),
            ("density", scenario.density.map(|value| value.value)),
            ("viscosity", scenario.viscosity.map(|value| value.value)),
            ("gas_density", scenario.gas_density.map(|value| value.value)),
            (
                "liquid_density",
                scenario.liquid_density.map(|value| value.value),
            ),
            ("latent_heat", scenario.latent_heat),
            ("liquid_heat_capacity", scenario.liquid_heat_capacity),
        ] {
            if let Some(value) = value {
                if !value.is_finite() || value <= 0.0 {
                    return Err(AzothError::invalid_input(
                        "scenarios",
                        format!(
                            "scenario `{}` states {name} = {value}, which must be finite and positive",
                            scenario.name
                        ),
                    ));
                }
            }
        }
        if let Some(k) = scenario.specific_heat_ratio {
            if k <= 1.0 {
                return Err(AzothError::invalid_input(
                    "scenarios",
                    format!(
                        "scenario `{}` states specific_heat_ratio = {k}, which is not above one",
                        scenario.name
                    ),
                ));
            }
        }
        if let Some(fraction) = scenario.gas_mass_fraction {
            if !(0.0..=1.0).contains(&fraction) {
                return Err(AzothError::invalid_input(
                    "scenarios",
                    format!(
                        "scenario `{}` states gas_mass_fraction = {fraction}, outside [0, 1]",
                        scenario.name
                    ),
                ));
            }
        }
    }

    let mut governing: Option<&OverpressureScenario> = None;
    for scenario in scenarios
        .iter()
        .filter(|scenario| scenario.credible.unwrap_or(true))
    {
        if governing.is_none()
            || scenario.relief_rate.value > governing.map_or(0.0, |row| row.relief_rate.value)
        {
            governing = Some(scenario);
        }
    }
    let Some(governing) = governing else {
        return Ok(empty_result(
            vec!["No credible positive relief demand defined".to_string()],
            warnings,
        ));
    };

    let cause = ReliefCause::parse(&governing.cause)?;
    let phase = ReliefPhase::parse(&governing.phase)?;
    let overpressure = if cause == ReliefCause::Fire {
        FIRE_OVERPRESSURE
    } else {
        NON_FIRE_OVERPRESSURE
    };
    let rate = governing.relief_rate.value;
    let mut messages = Vec::new();
    let mut required_area = None;
    let mut required_in2 = None;
    let mut recommended = None;
    let mut selected_in2 = None;
    let mut capacity = None;
    let mut adequate = false;

    if set_pressure.value > mawp.value {
        return Ok(empty_result(
            vec![
                format!(
                    "{}: Missing or invalid pressure, relief demand, cause or relieving-phase \
                     property basis",
                    governing.name
                ),
                "Resolve all credible relief demands before selecting an orifice".to_string(),
            ],
            warnings,
        ));
    }
    if rate <= 0.0 {
        return Ok(empty_result(
            vec![
                format!(
                    "{}: Missing or invalid pressure, relief demand, cause or relieving-phase \
                     property basis",
                    governing.name
                ),
                "Resolve all credible relief demands before selecting an orifice".to_string(),
            ],
            warnings,
        ));
    } else {
        match phase {
            ReliefPhase::Vapour => {
                let Some(molar_mass) = governing.molar_mass else {
                    return Ok(empty_result(
                        vec![
                            format!(
                                "{}: Missing or invalid pressure, relief demand, cause or \
                                 relieving-phase property basis",
                                governing.name
                            ),
                            "Resolve all credible relief demands before selecting an orifice"
                                .to_string(),
                        ],
                        warnings,
                    ));
                };
                let temperature = governing
                    .relief_temperature
                    .map_or(DEFAULT_TEMPERATURE, |value| value.value);
                let compressibility = governing.compressibility.unwrap_or(1.0);
                let specific_heat_ratio = governing.specific_heat_ratio.unwrap_or(1.3);
                let sizing = gas_area(
                    rate,
                    set_pressure.value,
                    overpressure,
                    back,
                    temperature,
                    molar_mass.value,
                    compressibility,
                    specific_heat_ratio,
                    false,
                    false,
                );
                required_area = Some(sizing.required_area);
                required_in2 = Some(sizing.required_area_in2);
                recommended = Some(sizing.recommended_orifice);
                selected_in2 = Some(sizing.selected_area_in2);
                capacity = if sizing.required_area_in2 > 0.0 {
                    Some(rate * sizing.selected_area_in2 / sizing.required_area_in2)
                } else {
                    None
                };
                adequate = sizing.selected_area_in2 >= sizing.required_area_in2;
                if !adequate {
                    messages.push(format!(
                        "{}: A single standard orifice cannot be selected for this relief demand",
                        governing.name
                    ));
                }
            }
            ReliefPhase::Liquid => {
                let stated = governing
                    .density
                    .map(|value| value.value)
                    .filter(|value| *value > 0.0)
                    .or_else(|| {
                        governing
                            .liquid_density
                            .map(|value| value.value)
                            .filter(|value| *value > 0.0)
                    });
                let density = match stated {
                    Some(density) => density,
                    None => {
                        messages.push(
                            "Liquid density not set; using default 700 kg/m3 for the liquid relief \
                             sizing"
                                .to_string(),
                        );
                        LIQUID_DENSITY_FALLBACK
                    }
                };
                let viscosity = governing
                    .viscosity
                    .map(|value| value.value)
                    .filter(|value| *value > 0.0)
                    .unwrap_or(LIQUID_VISCOSITY_FALLBACK);
                let volume_flow = rate / density;
                let sizing = liquid_area(
                    volume_flow,
                    density,
                    set_pressure.value,
                    overpressure,
                    back,
                    viscosity,
                    false,
                );
                required_area = Some(sizing.required_area);
                required_in2 = Some(sizing.required_area_in2);
                recommended = Some(sizing.recommended_orifice);
                selected_in2 = Some(sizing.selected_area_in2);
                capacity = if sizing.required_area_in2 > 0.0 {
                    Some(rate * sizing.selected_area_in2 / sizing.required_area_in2)
                } else {
                    None
                };
                adequate = sizing.selected_area_in2 >= sizing.required_area_in2;
                if !adequate {
                    messages.push(format!(
                        "{}: A single standard orifice cannot be selected for this relief demand",
                        governing.name
                    ));
                }
            }
            ReliefPhase::TwoPhase => {
                let missing = governing.gas_mass_fraction.is_none()
                    || governing.gas_density.is_none()
                    || governing.liquid_density.is_none()
                    || governing.latent_heat.is_none()
                    || governing.liquid_heat_capacity.is_none();
                if missing {
                    messages.push(
                        "Two-phase governing case is missing omega-method inputs (gas fraction, \
                         gas/liquid density, latent heat, liquid Cp); two-phase sizing skipped"
                            .to_string(),
                    );
                } else {
                    let temperature = governing
                        .relief_temperature
                        .map_or(DEFAULT_TEMPERATURE, |value| value.value);
                    let area = two_phase_area(
                        rate,
                        set_pressure.value,
                        overpressure,
                        back,
                        temperature,
                        governing.gas_mass_fraction.unwrap_or(0.0),
                        governing.gas_density.map_or(0.0, |value| value.value),
                        governing.liquid_density.map_or(0.0, |value| value.value),
                        governing.latent_heat.unwrap_or(0.0),
                        governing.liquid_heat_capacity.unwrap_or(0.0),
                    );
                    let area_in2 = area / M2_PER_IN2;
                    let (letter, selected) = select_orifice(area_in2);
                    required_area = Some(area);
                    required_in2 = Some(area_in2);
                    recommended = Some(letter);
                    selected_in2 = Some(selected);
                    capacity = if area_in2 > 0.0 {
                        Some(rate * selected / area_in2)
                    } else {
                        None
                    };
                    adequate = selected >= area_in2;
                    if !adequate {
                        messages.push(format!(
                            "{}: A single standard orifice cannot be selected for this relief demand",
                            governing.name
                        ));
                    }
                }
            }
        }
    }

    let peak = pascals(relieving_pressure(set_pressure.value, overpressure));
    let acceptance = overpressure_acceptance(peak, mawp, cause.as_str(), multiple)?;
    if !acceptance.accepted {
        let suffix = if cause == ReliefCause::Fire {
            ""
        } else if multiple {
            ", multiple devices"
        } else {
            ", single device"
        };
        messages.push(format!(
            "Relieving pressure exceeds the allowable accumulated pressure: {:.0}% of gauge MAWP \
             ({:.2} barg x {:.2}, standard atmosphere) for {}{}",
            acceptance.accumulation_factor * 100.0,
            mawp.value / 1.0e5 - 1.01325,
            acceptance.accumulation_factor,
            cause.label(),
            suffix
        ));
    }
    warnings.extend(acceptance.warnings.clone());
    Ok(OverpressureProtectionStudyResult {
        governing_name: Some(governing.name.clone()),
        governing_cause: Some(governing.cause.clone()),
        governing_phase: Some(governing.phase.clone()),
        governing_relief_rate: Some(governing.relief_rate),
        required_area: required_area.map(square_meters),
        required_area_in2: required_in2,
        recommended_orifice: recommended,
        selected_area_in2: selected_in2,
        selected_orifice_capacity: capacity.map(kilograms_per_second),
        capacity_adequate: adequate,
        acceptance_peak_pressure: Some(peak),
        acceptance_allowable_pressure: Some(acceptance.allowable_pressure),
        acceptance_accumulation_factor: Some(acceptance.accumulation_factor),
        acceptance_margin: Some(acceptance.margin),
        accepted: Some(acceptance.accepted),
        messages,
        warnings,
    })
}

/// The result when nothing governs: every optional output is absent.
fn empty_result(
    messages: Vec<String>,
    warnings: Vec<Warning>,
) -> OverpressureProtectionStudyResult {
    OverpressureProtectionStudyResult {
        governing_name: None,
        governing_cause: None,
        governing_phase: None,
        governing_relief_rate: None,
        required_area: None,
        required_area_in2: None,
        recommended_orifice: None,
        selected_area_in2: None,
        selected_orifice_capacity: None,
        capacity_adequate: false,
        acceptance_peak_pressure: None,
        acceptance_allowable_pressure: None,
        acceptance_accumulation_factor: None,
        acceptance_margin: None,
        accepted: None,
        messages,
        warnings,
    }
}
