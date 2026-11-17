//! `process.vacuum_collapse` - the blocked-in cooldown sweep as a registered id.
//!
//! Spec: `specs/models/process/vacuum_collapse.toml`. The arithmetic is
//! `process/safety/vacuum/VacuumCollapseAnalyzer.java` at the pin: the constant-volume
//! cooling sweep, the minimum pressure and the make-up gas.

use azoth_core::units::{
    AmountOfSubstance, Mass, MolarMass, Pressure, ThermodynamicTemperature, Volume,
    cubic_meters_per_mole, kelvins, kilograms, moles, pascals,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use azoth_eos::Phase;
use azoth_eos::cubic::Cubic;
use azoth_eos::databank;
use azoth_eos::phase_boundary::check_composition;
use azoth_eos::pt_flash::pt_flash;
use azoth_eos::tv_flash::tv_flash;
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's gas constant, J/(mol K).
const R_GAS: f64 = 8.314_462_618;
/// The class's default atmospheric pressure, Pa.
const DEFAULT_ATMOSPHERIC_PRESSURE_PA: f64 = 101_325.0;
/// The class's default make-up gas molar mass, kg/mol (nitrogen).
const DEFAULT_MAKEUP_MOLAR_MASS: f64 = 0.028_013_4;
/// The class's default cooling steps.
const DEFAULT_COOLING_STEPS: f64 = 20.0;

/// Whether the cooling sweep's minimum pressure stays within the external rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum VacuumVerdict {
    /// The pressure stays at or above atmosphere.
    #[serde(rename = "no_vacuum")]
    NoVacuum,
    /// A vacuum develops but stays within the external-pressure rating.
    #[serde(rename = "vacuum_within_rating")]
    VacuumWithinRating,
    /// The pressure falls below the external-pressure rating.
    #[serde(rename = "vacuum_exceeds_rating")]
    VacuumExceedsRating,
}

impl VacuumVerdict {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoVacuum => "no_vacuum",
            Self::VacuumWithinRating => "vacuum_within_rating",
            Self::VacuumExceedsRating => "vacuum_exceeds_rating",
        }
    }
}

/// Result of `process.vacuum_collapse`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VacuumCollapseResult {
    /// The pressure at the cold end, the last converged step's.
    #[serde(serialize_with = "scalar")]
    pub final_pressure: Pressure,
    /// The smallest pressure on the cooling curve.
    #[serde(serialize_with = "scalar")]
    pub minimum_pressure: Pressure,
    /// The temperature at which the smallest pressure is reached.
    #[serde(serialize_with = "scalar")]
    pub minimum_pressure_temperature: ThermodynamicTemperature,
    /// `max(0, atmospheric_pressure - minimum_pressure)`.
    #[serde(serialize_with = "scalar")]
    pub vacuum_depth: Pressure,
    /// `minimum_pressure - external_pressure_rating`.
    #[serde(serialize_with = "scalar")]
    pub margin_to_rating: Pressure,
    /// Whether the minimum pressure is below atmosphere.
    pub vacuum_present: bool,
    /// Whether the minimum pressure is at or above the external rating.
    pub within_rating: bool,
    /// The make-up moles needed to hold the setpoint.
    #[serde(serialize_with = "scalar")]
    pub makeup_gas_moles: AmountOfSubstance,
    /// The make-up mass.
    #[serde(serialize_with = "scalar")]
    pub makeup_gas_mass: Mass,
    /// The class's verdict.
    pub verdict: VacuumVerdict,
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

/// The constant-volume cooling sweep, its minimum pressure and the make-up gas.
///
/// # Errors
/// `InvalidInput` for a composition that does not match the components or sums to one, a
/// non-positive vessel volume, pressure or temperature, a cold end at or above the initial
/// temperature, a non-positive make-up pressure or molar mass, a non-integer step count
/// below one, a flash state the databank cannot resolve, or a non-finite input.
#[allow(clippy::too_many_arguments)] // The signature is the spec's declared input list.
pub fn vacuum_collapse(
    components: &[String],
    mole_fractions: &[f64],
    vessel_volume: Volume,
    initial_pressure: Pressure,
    initial_temperature: ThermodynamicTemperature,
    cold_end_temperature: ThermodynamicTemperature,
    atmospheric_pressure: Option<Pressure>,
    external_pressure_rating: Option<Pressure>,
    makeup_setpoint: Option<Pressure>,
    makeup_gas_molar_mass: Option<MolarMass>,
    cooling_steps: Option<f64>,
) -> Result<VacuumCollapseResult> {
    let spec = &model_gen::VACUUM_COLLAPSE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "vessel_volume" => Some(vessel_volume.value),
            "initial_pressure" => Some(initial_pressure.value),
            "initial_temperature" => Some(initial_temperature.value),
            "cooling_steps" => cooling_steps,
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite("vessel_volume", vessel_volume.value)?;
    require_finite("initial_pressure", initial_pressure.value)?;
    require_finite("initial_temperature", initial_temperature.value)?;
    require_finite("cold_end_temperature", cold_end_temperature.value)?;
    for (name, value) in [
        (
            "atmospheric_pressure",
            atmospheric_pressure.map(|value| value.value),
        ),
        (
            "external_pressure_rating",
            external_pressure_rating.map(|value| value.value),
        ),
        ("makeup_setpoint", makeup_setpoint.map(|value| value.value)),
        (
            "makeup_gas_molar_mass",
            makeup_gas_molar_mass.map(|value| value.value),
        ),
        ("cooling_steps", cooling_steps),
    ] {
        if let Some(value) = value {
            require_finite(name, value)?;
        }
    }
    if cold_end_temperature.value >= initial_temperature.value {
        return Err(AzothError::invalid_input(
            "cold_end_temperature",
            "the cold-end temperature must be below the initial temperature for a cooling case",
        ));
    }
    let steps = cooling_steps.unwrap_or(DEFAULT_COOLING_STEPS);
    if steps < 1.0 || (steps - steps.round()).abs() > 1.0e-9 {
        return Err(AzothError::invalid_input(
            "cooling_steps",
            "the step count must be a positive integer",
        ));
    }

    let atmospheric =
        atmospheric_pressure.map_or(DEFAULT_ATMOSPHERIC_PRESSURE_PA, |value| value.value);
    if atmospheric <= 0.0 {
        return Err(AzothError::invalid_input(
            "atmospheric_pressure",
            "the atmospheric pressure must be positive",
        ));
    }
    let rating = external_pressure_rating.map_or(0.0, |value| value.value);
    if rating < 0.0 {
        return Err(AzothError::invalid_input(
            "external_pressure_rating",
            "the external-pressure rating must be at least zero",
        ));
    }
    let setpoint = makeup_setpoint.map_or(atmospheric, |value| value.value);
    if setpoint <= 0.0 {
        return Err(AzothError::invalid_input(
            "makeup_setpoint",
            "the make-up setpoint must be positive",
        ));
    }
    let makeup_molar_mass =
        makeup_gas_molar_mass.map_or(DEFAULT_MAKEUP_MOLAR_MASS, |value| value.value);
    if makeup_molar_mass <= 0.0 {
        return Err(AzothError::invalid_input(
            "makeup_gas_molar_mass",
            "the make-up gas molar mass must be positive",
        ));
    }

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, Cubic::Pr, None)?;
    check_composition(mole_fractions, mixture.len(), "mole_fractions")?;

    let t0 = initial_temperature.value;
    let p0 = initial_pressure.value;
    let flash = pt_flash(&mixture, kelvins(t0), pascals(p0), mole_fractions)?;
    let compressibility = match flash.phase {
        Phase::TwoPhase => {
            let beta = flash.vapour_fraction.ok_or_else(|| {
                AzothError::invalid_input(
                    "mole_fractions",
                    "a two-phase split has no vapour fraction",
                )
            })?;
            (1.0 - beta) * flash.z_liquid + beta * flash.z_vapour
        }
        Phase::AllLiquid => flash.z_liquid,
        Phase::AllVapour | Phase::Trivial => flash.z_vapour,
    };
    let molar_volume = compressibility * R_GAS * t0 / p0;

    let cold_end = cold_end_temperature.value;
    let mut final_pressure = p0;
    let mut minimum_pressure = p0;
    let mut minimum_temperature = t0;
    let count = steps as u32;
    for index in 1..=count {
        let temperature = t0 + (cold_end - t0) * f64::from(index) / f64::from(count);
        match tv_flash(
            &mixture,
            &ideal_gas,
            kelvins(temperature),
            cubic_meters_per_mole(molar_volume),
            mole_fractions,
        ) {
            Ok(state) => {
                let pressure = state.pressure.value;
                final_pressure = pressure;
                if pressure < minimum_pressure {
                    minimum_pressure = pressure;
                    minimum_temperature = temperature;
                }
            }
            Err(error) => {
                if index == 1 {
                    return Err(error);
                }
                break;
            }
        }
    }

    let vacuum_depth = (atmospheric - minimum_pressure).max(0.0);
    let margin = minimum_pressure - rating;
    let vacuum_present = minimum_pressure < atmospheric;
    let within_rating = minimum_pressure >= rating;
    let verdict = if !vacuum_present {
        VacuumVerdict::NoVacuum
    } else if within_rating {
        VacuumVerdict::VacuumWithinRating
    } else {
        VacuumVerdict::VacuumExceedsRating
    };
    let makeup_moles = if minimum_pressure < setpoint {
        vessel_volume.value * (setpoint - minimum_pressure) / (R_GAS * minimum_temperature)
    } else {
        0.0
    };

    Ok(VacuumCollapseResult {
        final_pressure: pascals(final_pressure),
        minimum_pressure: pascals(minimum_pressure),
        minimum_pressure_temperature: kelvins(minimum_temperature),
        vacuum_depth: pascals(vacuum_depth),
        margin_to_rating: pascals(margin),
        vacuum_present,
        within_rating,
        makeup_gas_moles: moles(makeup_moles),
        makeup_gas_mass: kilograms(makeup_moles * makeup_molar_mass),
        verdict,
        warnings,
    })
}
