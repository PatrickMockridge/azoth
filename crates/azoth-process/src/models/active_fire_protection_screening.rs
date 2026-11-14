//! `process.active_fire_protection_screening` - the protection hierarchy as a registered id.
//!
//! Spec: `specs/models/process/active_fire_protection_screening.toml`. The arithmetic is
//! `process/safety/firewater/ActiveFireProtectionScreening.java` at the pin: the
//! extinguishing classification, the blowdown margin and the selective saving.

use azoth_core::units::{Time, VolumeRate, seconds};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's four fire scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FireScenario {
    /// A pressurised gas jet.
    GasJet,
    /// A two-phase jet.
    TwoPhaseJet,
    /// A liquid pool.
    LiquidPool,
    /// An ordinary combustible.
    OrdinaryCombustible,
}

impl FireScenario {
    /// The stated scenario.
    fn parse(value: &str) -> Result<Self> {
        match value {
            "gas_jet_fire" => Ok(Self::GasJet),
            "two_phase_jet_fire" => Ok(Self::TwoPhaseJet),
            "liquid_pool_fire" => Ok(Self::LiquidPool),
            "ordinary_combustible_fire" => Ok(Self::OrdinaryCombustible),
            other => Err(AzothError::invalid_input(
                "scenario",
                format!("the scenario is one of the class's four fire types, and {other:?} is not"),
            )),
        }
    }
}

/// Result of `process.active_fire_protection_screening`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActiveFireProtectionScreeningResult {
    /// Whether water application can extinguish the fire.
    pub water_extinguishing: bool,
    /// Whether blowdown is faster than the steel's failure.
    pub blowdown_faster_than_failure: bool,
    /// `bare - blowdown`; positive when blowdown wins.
    #[serde(serialize_with = "scalar")]
    pub blowdown_margin: Time,
    /// The fraction of the area requirement selective protection does not discharge.
    pub selective_protection_saving_fraction: f64,
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

/// The extinguishing classification, the blowdown margin and the selective saving.
///
/// # Errors
/// `InvalidInput` for a non-positive blowdown or time to critical, a negative dedicated
/// demand, a non-positive general demand, an unknown scenario, or a non-finite input.
pub fn active_fire_protection_screening(
    scenario: &str,
    blowdown_time_to_target: Time,
    bare_steel_time_to_critical: Time,
    dedicated_object_demand: VolumeRate,
    general_area_demand: VolumeRate,
) -> Result<ActiveFireProtectionScreeningResult> {
    let spec = &model_gen::ACTIVE_FIRE_PROTECTION_SCREENING_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "blowdown_time_to_target" => Some(blowdown_time_to_target.value),
            "bare_steel_time_to_critical" => Some(bare_steel_time_to_critical.value),
            "dedicated_object_demand" => Some(dedicated_object_demand.value),
            "general_area_demand" => Some(general_area_demand.value),
            _ => None,
        },
        &mut warnings,
    )?;

    let fire = FireScenario::parse(scenario)?;
    require_finite("blowdown_time_to_target", blowdown_time_to_target.value)?;
    require_finite(
        "bare_steel_time_to_critical",
        bare_steel_time_to_critical.value,
    )?;
    require_finite("dedicated_object_demand", dedicated_object_demand.value)?;
    require_finite("general_area_demand", general_area_demand.value)?;

    // The class's operation order, to the letter.
    let extinguishing = matches!(
        fire,
        FireScenario::LiquidPool | FireScenario::OrdinaryCombustible
    );
    let faster = blowdown_time_to_target.value < bare_steel_time_to_critical.value;
    let margin = bare_steel_time_to_critical.value - blowdown_time_to_target.value;
    let saving = 1.0 - dedicated_object_demand.value / general_area_demand.value;

    Ok(ActiveFireProtectionScreeningResult {
        water_extinguishing: extinguishing,
        blowdown_faster_than_failure: faster,
        blowdown_margin: seconds(margin),
        selective_protection_saving_fraction: saving,
        warnings,
    })
}
