//! `process.settle_out_pressure` - the connected-volume settle-out balance.
//!
//! Spec: `specs/models/process/settle_out_pressure.toml`. The arithmetic is
//! `process/safety/settleout/SettleOutPressureAnalyzer.java` at the pin: the no-gas
//! inventory, the settle temperature and pressure, and the rating verdict.

use azoth_core::units::{
    AmountOfSubstance, Pressure, ThermodynamicTemperature, Volume, cubic_meters, kelvins, moles,
    pascals,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's gas constant, J/(mol K).
const R_GAS: f64 = 8.314_462_618;

/// One connected gas volume a caller states.
#[derive(Debug, Clone, PartialEq)]
pub struct SettleOutCompartment {
    /// The compartment's label.
    pub name: String,
    /// The internal gas volume.
    pub volume: Volume,
    /// The compartment's pressure.
    pub pressure: Pressure,
    /// The compartment's temperature.
    pub temperature: ThermodynamicTemperature,
}

/// One compartment's contribution to the balance.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SettleOutCompartmentResult {
    /// The compartment's label.
    pub name: String,
    /// The internal gas volume.
    #[serde(serialize_with = "scalar")]
    pub volume: Volume,
    /// The compartment's pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// The compartment's temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// The compartment's compressibility, 1.0.
    pub z_factor: f64,
    /// The compartment's inventory.
    #[serde(serialize_with = "scalar")]
    pub moles: AmountOfSubstance,
}

/// Whether the settle-out pressure exceeds the protected rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SettleOutVerdict {
    /// No protected rating was stated.
    #[serde(rename = "no_rating")]
    NoRating,
    /// The settle-out pressure is at or below the rating.
    #[serde(rename = "within_rating")]
    WithinRating,
    /// The settle-out pressure is above the rating.
    #[serde(rename = "exceeds_rating")]
    ExceedsRating,
}

impl SettleOutVerdict {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoRating => "no_rating",
            Self::WithinRating => "within_rating",
            Self::ExceedsRating => "exceeds_rating",
        }
    }
}

/// Result of `process.settle_out_pressure`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SettleOutPressureResult {
    /// The pressure the connected volumes settle at.
    #[serde(serialize_with = "scalar")]
    pub settle_out_pressure: Pressure,
    /// The settled temperature.
    #[serde(serialize_with = "scalar")]
    pub settle_out_temperature: ThermodynamicTemperature,
    /// The settle-out compressibility, 1.0.
    pub settle_out_z_factor: f64,
    /// The summed inventory.
    #[serde(serialize_with = "scalar")]
    pub total_moles: AmountOfSubstance,
    /// The summed volume.
    #[serde(serialize_with = "scalar")]
    pub total_volume: Volume,
    /// The largest compartment pressure.
    #[serde(serialize_with = "scalar")]
    pub max_compartment_pressure: Pressure,
    /// The smallest compartment pressure.
    #[serde(serialize_with = "scalar")]
    pub min_compartment_pressure: Pressure,
    /// `rating - settle_out_pressure`; absent when no rating is stated.
    #[serde(serialize_with = "optional_scalar")]
    pub margin_to_rating: Option<Pressure>,
    /// Whether the settle-out pressure is strictly above the rating.
    pub exceeds_rating: bool,
    /// The class's verdict.
    pub verdict: SettleOutVerdict,
    /// One row per compartment, in the caller's order.
    pub compartments: Vec<SettleOutCompartmentResult>,
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

/// The settle-out pressure of connected gas volumes and the rating verdict.
///
/// # Errors
/// `InvalidInput` for fewer than two compartments, a non-positive compartment volume,
/// pressure or temperature, a non-positive stated settle temperature or rating, or a
/// non-finite input.
pub fn settle_out_pressure(
    compartments: &[SettleOutCompartment],
    settle_out_temperature: Option<ThermodynamicTemperature>,
    protected_pressure_rating: Option<Pressure>,
) -> Result<SettleOutPressureResult> {
    let spec = &model_gen::SETTLE_OUT_PRESSURE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    if compartments.len() < 2 {
        return Err(AzothError::invalid_input(
            "compartments",
            "at least two connected gas volumes are required for a settle-out balance",
        ));
    }
    for row in compartments {
        require_finite("compartments.volume", row.volume.value)?;
        require_finite("compartments.pressure", row.pressure.value)?;
        require_finite("compartments.temperature", row.temperature.value)?;
        if row.volume.value <= 0.0 {
            return Err(AzothError::invalid_input(
                "compartments.volume",
                "a compartment volume must be positive",
            ));
        }
        if row.pressure.value <= 0.0 {
            return Err(AzothError::invalid_input(
                "compartments.pressure",
                "a compartment pressure must be positive",
            ));
        }
        if row.temperature.value <= 0.0 {
            return Err(AzothError::invalid_input(
                "compartments.temperature",
                "a compartment temperature must be positive",
            ));
        }
    }
    if let Some(temperature) = settle_out_temperature {
        require_finite("settle_out_temperature", temperature.value)?;
        if temperature.value <= 0.0 {
            return Err(AzothError::invalid_input(
                "settle_out_temperature",
                "the settle-out temperature must be positive",
            ));
        }
    }
    if let Some(rating) = protected_pressure_rating {
        require_finite("protected_pressure_rating", rating.value)?;
        if rating.value <= 0.0 {
            return Err(AzothError::invalid_input(
                "protected_pressure_rating",
                "the protected rating must be positive",
            ));
        }
    }

    let mut total_moles = 0.0;
    let mut weighted_temperature = 0.0;
    let mut total_volume = 0.0;
    let mut maximum_pressure = f64::MIN;
    let mut minimum_pressure = f64::MAX;
    let mut rows = Vec::with_capacity(compartments.len());
    for row in compartments {
        let inventory = row.pressure.value * row.volume.value / (R_GAS * row.temperature.value);
        total_moles += inventory;
        weighted_temperature += inventory * row.temperature.value;
        total_volume += row.volume.value;
        maximum_pressure = maximum_pressure.max(row.pressure.value);
        minimum_pressure = minimum_pressure.min(row.pressure.value);
        rows.push(SettleOutCompartmentResult {
            name: row.name.clone(),
            volume: row.volume,
            pressure: row.pressure,
            temperature: row.temperature,
            z_factor: 1.0,
            moles: moles(inventory),
        });
    }
    let settled_temperature =
        settle_out_temperature.map_or(weighted_temperature / total_moles, |value| value.value);
    let settled_pressure = total_moles * R_GAS * settled_temperature / total_volume;

    let (margin_to_rating, exceeds_rating, verdict) = match protected_pressure_rating {
        Some(rating) => {
            let margin = rating.value - settled_pressure;
            let exceeded = settled_pressure > rating.value;
            (
                Some(pascals(margin)),
                exceeded,
                if exceeded {
                    SettleOutVerdict::ExceedsRating
                } else {
                    SettleOutVerdict::WithinRating
                },
            )
        }
        None => (None, false, SettleOutVerdict::NoRating),
    };

    Ok(SettleOutPressureResult {
        settle_out_pressure: pascals(settled_pressure),
        settle_out_temperature: kelvins(settled_temperature),
        settle_out_z_factor: 1.0,
        total_moles: moles(total_moles),
        total_volume: cubic_meters(total_volume),
        max_compartment_pressure: pascals(maximum_pressure),
        min_compartment_pressure: pascals(minimum_pressure),
        margin_to_rating,
        exceeds_rating,
        verdict,
        compartments: rows,
        warnings,
    })
}
