//! `process.trapped_inventory` - the trapped gas and liquid inventory.
//!
//! Spec: `specs/models/process/trapped_inventory.toml`. The arithmetic is NeqSim's
//! `process/safety/inventory/TrappedInventoryCalculator.java`, through `calculate`.

use std::f64::consts::PI;

use azoth_core::units::{
    Length, Mass, MassDensity, Pressure, ThermodynamicTemperature, Volume, cubic_meters, kilograms,
    kilograms_per_cubic_meter,
};
use azoth_core::{AzothError, Result, Warning, WarningCode, apply_checks};
use azoth_eos::Cubic;
use azoth_eos::databank;
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::fluid_state;

/// One documented volume.
#[derive(Debug, Clone, PartialEq)]
pub struct TrappedInventorySegment {
    /// The segment identifier.
    pub segment_id: String,
    /// Whether the volume is equipment or a pipe.
    pub kind: SegmentKind,
    /// The equipment's internal volume; ignored for a pipe.
    pub volume: Option<Volume>,
    /// The liquid share of the volume.
    pub liquid_fill_fraction: f64,
    /// The pipe's internal diameter.
    pub internal_diameter: Option<Length>,
    /// The pipe's length.
    pub length: Option<Length>,
}

/// The segment's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SegmentKind {
    /// An equipment volume.
    #[serde(rename = "equipment")]
    Equipment,
    /// A pipe volume from its diameter and length.
    #[serde(rename = "pipe")]
    Pipe,
}

impl SegmentKind {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equipment => "equipment",
            Self::Pipe => "pipe",
        }
    }

    /// The spec's vocabulary, parsed.
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "equipment" => Ok(Self::Equipment),
            "pipe" => Ok(Self::Pipe),
            other => Err(AzothError::invalid_input(
                "segments.kind",
                format!("`{other}` is not `equipment` or `pipe`"),
            )),
        }
    }
}

/// One document evidence row.
#[derive(Debug, Clone, PartialEq)]
pub struct TrappedInventoryEvidence {
    /// The segment the evidence belongs to.
    pub segment_id: String,
    /// The document identifier.
    pub document_id: Option<String>,
    /// The source reference.
    pub source_reference: Option<String>,
    /// The excerpt the volume rests on.
    pub excerpt: Option<String>,
}

/// One segment's inventory.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrappedInventorySegmentResult {
    /// The segment identifier.
    pub segment_id: String,
    /// The segment's kind.
    pub kind: SegmentKind,
    /// The segment's internal volume.
    #[serde(serialize_with = "scalar")]
    pub total_volume: Volume,
    /// The gas share.
    #[serde(serialize_with = "scalar")]
    pub gas_volume: Volume,
    /// The liquid share.
    #[serde(serialize_with = "scalar")]
    pub liquid_volume: Volume,
    /// The segment's gas mass.
    #[serde(serialize_with = "scalar")]
    pub gas_mass: Mass,
    /// The segment's liquid mass.
    #[serde(serialize_with = "scalar")]
    pub liquid_mass: Mass,
    /// The segment's gas and liquid mass.
    #[serde(serialize_with = "scalar")]
    pub total_mass: Mass,
    /// The density the gas mass used.
    #[serde(serialize_with = "scalar")]
    pub gas_density: MassDensity,
    /// The density the liquid mass used.
    #[serde(serialize_with = "scalar")]
    pub liquid_density: MassDensity,
    /// The echoed fill fraction.
    pub liquid_fill_fraction: f64,
    /// The pipe's diameter, echoed.
    #[serde(serialize_with = "optional_scalar")]
    pub internal_diameter: Option<Length>,
    /// The pipe's length, echoed.
    #[serde(serialize_with = "optional_scalar")]
    pub length: Option<Length>,
}

/// Result of `process.trapped_inventory`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrappedInventoryResult {
    /// The gas phase's density, or the bulk density.
    #[serde(serialize_with = "scalar")]
    pub gas_density: MassDensity,
    /// The oil, aqueous or fallback density.
    #[serde(serialize_with = "scalar")]
    pub liquid_density: MassDensity,
    /// The segments' internal volumes.
    #[serde(serialize_with = "scalar")]
    pub total_volume: Volume,
    /// The gas share.
    #[serde(serialize_with = "scalar")]
    pub total_gas_volume: Volume,
    /// The liquid share.
    #[serde(serialize_with = "scalar")]
    pub total_liquid_volume: Volume,
    /// `total_gas_volume * gas_density`.
    #[serde(serialize_with = "scalar")]
    pub total_gas_mass: Mass,
    /// `total_liquid_volume * liquid_density`.
    #[serde(serialize_with = "scalar")]
    pub total_liquid_mass: Mass,
    /// The gas and liquid masses.
    #[serde(serialize_with = "scalar")]
    pub total_mass: Mass,
    /// One row per input segment.
    pub segments: Vec<TrappedInventorySegmentResult>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The trapped gas and liquid inventory.
///
/// # Errors
/// `InvalidInput` for an empty segment list, a non-positive pressure, temperature, volume or
/// fallback density, a fill outside `[0, 1]`, a wrong volume-and-geometry pair, or an
/// evidence row naming no segment.
#[allow(clippy::too_many_arguments)]
pub fn trapped_inventory(
    components: &[String],
    z: &[f64],
    pressure: Pressure,
    temperature: ThermodynamicTemperature,
    segments: &[TrappedInventorySegment],
    eos: Option<&str>,
    multiphase_check: Option<bool>,
    fallback_liquid_density: Option<MassDensity>,
    segment_evidence: Option<&[TrappedInventoryEvidence]>,
) -> Result<TrappedInventoryResult> {
    let spec = &model_gen::TRAPPED_INVENTORY_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "pressure" => Some(pressure.value),
            "temperature" => Some(temperature.value),
            "fallback_liquid_density" => fallback_liquid_density.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;
    if segments.is_empty() {
        return Err(AzothError::invalid_input(
            "segments",
            "the class refuses an inventory with no segment",
        ));
    }
    let fallback = fallback_liquid_density.map_or(800.0, |value| value.value);
    if !(fallback > 0.0 && fallback.is_finite()) {
        return Err(AzothError::invalid_input(
            "fallback_liquid_density",
            "a density is positive",
        ));
    }

    let evidence = segment_evidence.unwrap_or(&[]);
    let mut volumes = Vec::with_capacity(segments.len());
    for row in segments {
        if row.segment_id.trim().is_empty() {
            return Err(AzothError::invalid_input(
                "segments.segment_id",
                "a segment identifier must not be empty",
            ));
        }
        if !(0.0..=1.0).contains(&row.liquid_fill_fraction) || !row.liquid_fill_fraction.is_finite()
        {
            return Err(AzothError::invalid_input(
                "segments.liquid_fill_fraction",
                "the class refuses a fill fraction outside [0, 1]",
            ));
        }
        let volume = match row.kind {
            SegmentKind::Equipment => {
                let value = row.volume.ok_or_else(|| {
                    AzothError::invalid_input(
                        "segments.volume",
                        "an equipment segment requires its volume",
                    )
                })?;
                if row.internal_diameter.is_some() || row.length.is_some() {
                    return Err(AzothError::invalid_input(
                        "segments",
                        "an equipment segment states its volume, not a pipe geometry",
                    ));
                }
                if !(value.value > 0.0 && value.value.is_finite()) {
                    return Err(AzothError::invalid_input(
                        "segments.volume",
                        "a volume is positive",
                    ));
                }
                value.value
            }
            SegmentKind::Pipe => {
                if row.volume.is_some() {
                    return Err(AzothError::invalid_input(
                        "segments",
                        "a pipe states its geometry, not a volume",
                    ));
                }
                let diameter = row.internal_diameter.map(|value| value.value);
                let length = row.length.map(|value| value.value);
                let (Some(diameter), Some(length)) = (diameter, length) else {
                    return Err(AzothError::invalid_input(
                        "segments",
                        "a pipe segment requires its diameter and length",
                    ));
                };
                if !(diameter > 0.0 && diameter.is_finite() && length > 0.0 && length.is_finite()) {
                    return Err(AzothError::invalid_input(
                        "segments",
                        "a pipe's diameter and length are positive",
                    ));
                }
                PI * diameter * diameter * length / 4.0
            }
        };
        volumes.push(volume);
    }
    for row in evidence {
        if !segments
            .iter()
            .any(|segment| segment.segment_id == row.segment_id)
        {
            return Err(AzothError::invalid_input(
                "segment_evidence.segment_id",
                format!("`{}` names no segment row", row.segment_id),
            ));
        }
    }

    let cubic = match eos.unwrap_or("pr") {
        "pr" => Cubic::Pr,
        "srk" => Cubic::Srk,
        other => {
            return Err(AzothError::invalid_input(
                "eos",
                format!("`{other}` is not `pr` or `srk`"),
            ));
        }
    };
    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, cubic, None)?;
    let state = fluid_state::flash(
        &mixture,
        &ideal_gas,
        temperature.value,
        pressure.value,
        z,
        multiphase_check.unwrap_or(false),
    )?;
    let gas_density = state.gas_density();
    let liquid_density = state.liquid_density(fallback);

    let mut results = Vec::with_capacity(segments.len());
    let mut total_volume = 0.0;
    let mut total_gas_volume = 0.0;
    let mut total_liquid_volume = 0.0;
    for (row, volume) in segments.iter().zip(&volumes) {
        let gas_volume = volume * (1.0 - row.liquid_fill_fraction);
        let liquid_volume = volume * row.liquid_fill_fraction;
        let gas_mass = gas_volume * gas_density;
        let liquid_mass = liquid_volume * liquid_density;
        results.push(TrappedInventorySegmentResult {
            segment_id: row.segment_id.clone(),
            kind: row.kind,
            total_volume: cubic_meters(*volume),
            gas_volume: cubic_meters(gas_volume),
            liquid_volume: cubic_meters(liquid_volume),
            gas_mass: kilograms(gas_mass),
            liquid_mass: kilograms(liquid_mass),
            total_mass: kilograms(gas_mass + liquid_mass),
            gas_density: kilograms_per_cubic_meter(gas_density),
            liquid_density: kilograms_per_cubic_meter(liquid_density),
            liquid_fill_fraction: row.liquid_fill_fraction,
            internal_diameter: row.internal_diameter,
            length: row.length,
        });
        total_volume += volume;
        total_gas_volume += gas_volume;
        total_liquid_volume += liquid_volume;
        if !evidence
            .iter()
            .any(|item| item.segment_id == row.segment_id)
        {
            warnings.push(Warning::for_field(
                WarningCode::MissingEvidence,
                "segments",
                format!(
                    "Segment {} has no traceable document evidence.",
                    row.segment_id
                ),
            ));
        }
    }
    let total_gas_mass = total_gas_volume * gas_density;
    let total_liquid_mass = total_liquid_volume * liquid_density;
    if total_liquid_volume > 0.0 && liquid_density == fallback {
        warnings.push(Warning::for_field(
            WarningCode::FallbackToInput,
            "liquid_density",
            "No liquid phase found in representative fluid; fallback liquid density used.",
        ));
    }

    Ok(TrappedInventoryResult {
        gas_density: kilograms_per_cubic_meter(gas_density),
        liquid_density: kilograms_per_cubic_meter(liquid_density),
        total_volume: cubic_meters(total_volume),
        total_gas_volume: cubic_meters(total_gas_volume),
        total_liquid_volume: cubic_meters(total_liquid_volume),
        total_gas_mass: kilograms(total_gas_mass),
        total_liquid_mass: kilograms(total_liquid_mass),
        total_mass: kilograms(total_gas_mass + total_liquid_mass),
        segments: results,
        warnings,
    })
}
