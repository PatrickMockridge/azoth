//! `process.deluge_nozzle_layout` - the deluge nozzle net screen as a registered id.
//!
//! Spec: `specs/models/process/deluge_nozzle_layout.toml`. The arithmetic is
//! `process/safety/firewater/DelugeNozzleLayout.java` at the pin: the flow and coverage
//! criteria, the larger count, the grid and the delivered density.

use azoth_core::units::{
    Area, Length, Pressure, VolumeRate, bars, liters_per_minute, meters, square_meters,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's default operating pressure, Pa.
const DEFAULT_OPERATING_PRESSURE_PA: f64 = 5.0e5;
/// The class's default coverage efficiency.
const DEFAULT_COVERAGE_EFFICIENCY: f64 = 1.0;

/// Which criterion set the nozzle count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GoverningCriterion {
    /// The flow criterion.
    #[serde(rename = "flow")]
    Flow,
    /// The coverage criterion.
    #[serde(rename = "coverage")]
    Coverage,
}

impl GoverningCriterion {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flow => "flow",
            Self::Coverage => "coverage",
        }
    }
}

/// Result of `process.deluge_nozzle_layout`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DelugeNozzleLayoutResult {
    /// The required flow.
    #[serde(serialize_with = "scalar")]
    pub required_flow: VolumeRate,
    /// One nozzle's flow at the operating pressure.
    #[serde(serialize_with = "scalar")]
    pub flow_per_nozzle: VolumeRate,
    /// One nozzle's coverage square.
    #[serde(serialize_with = "scalar")]
    pub coverage_per_nozzle: Area,
    /// The count the flow criterion asks for.
    pub nozzle_count_from_flow: f64,
    /// The count the coverage criterion asks for.
    pub nozzle_count_from_coverage: f64,
    /// The governing count.
    pub nozzle_count: f64,
    /// Which criterion governed.
    pub governing_criterion: GoverningCriterion,
    /// The grid spacing.
    #[serde(serialize_with = "scalar")]
    pub grid_spacing: Length,
    /// The delivered flow.
    #[serde(serialize_with = "scalar")]
    pub delivered_flow: VolumeRate,
    /// The delivered density in `L/(min m2)`; dimensionless.
    pub delivered_density: f64,
    /// Whether the operating pressure meets the nozzle minimum.
    pub pressure_adequate: bool,
    /// Whether the delivered density meets the requirement.
    pub density_met: bool,
    /// Whether the available clearance meets the nozzle requirement.
    pub clearance_adequate: bool,
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

/// The flow and coverage criteria, the larger count, the grid and the delivered density.
///
/// # Errors
/// `InvalidInput` for a non-positive area, density, discharge coefficient, minimum pressure,
/// spacing or operating pressure, an efficiency outside `(0, 1]`, a negative clearance, or
/// a non-finite input.
#[allow(clippy::too_many_arguments)]
pub fn deluge_nozzle_layout(
    protected_area: Area,
    required_density: f64,
    nozzle_k: f64,
    nozzle_min_pressure: Pressure,
    nozzle_max_spacing: Length,
    nozzle_clearance: Length,
    available_clearance: Length,
    operating_pressure: Option<Pressure>,
    coverage_efficiency: Option<f64>,
) -> Result<DelugeNozzleLayoutResult> {
    let spec = &model_gen::DELUGE_NOZZLE_LAYOUT_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "protected_area" => Some(protected_area.value),
            "required_density" => Some(required_density),
            "nozzle_k" => Some(nozzle_k),
            "nozzle_min_pressure" => Some(nozzle_min_pressure.value),
            "nozzle_max_spacing" => Some(nozzle_max_spacing.value),
            "nozzle_clearance" => Some(nozzle_clearance.value),
            "available_clearance" => Some(available_clearance.value),
            "operating_pressure" => operating_pressure.map(|value| value.value),
            "coverage_efficiency" => coverage_efficiency,
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite("protected_area", protected_area.value)?;
    require_finite("required_density", required_density)?;
    require_finite("nozzle_k", nozzle_k)?;
    require_finite("nozzle_min_pressure", nozzle_min_pressure.value)?;
    require_finite("nozzle_max_spacing", nozzle_max_spacing.value)?;
    require_finite("nozzle_clearance", nozzle_clearance.value)?;
    require_finite("available_clearance", available_clearance.value)?;
    let pressure = operating_pressure
        .map(|value| value.value)
        .unwrap_or(DEFAULT_OPERATING_PRESSURE_PA);
    require_finite("operating_pressure", pressure)?;
    let efficiency = coverage_efficiency.unwrap_or(DEFAULT_COVERAGE_EFFICIENCY);
    require_finite("coverage_efficiency", efficiency)?;

    // The class's operation order, to the letter. **The orifice law is in bar**: the SI
    // magnitudes are pascals, and the K factor is `(L/min)/sqrt(bar)`.
    let pressure_bar = pressure / 1.0e5;
    let required_flow = protected_area.value * required_density;
    let flow_per_nozzle = nozzle_k * pressure_bar.sqrt();
    let coverage_per_nozzle = nozzle_max_spacing.value * nozzle_max_spacing.value * efficiency;
    let count_from_flow = (required_flow / flow_per_nozzle).ceil();
    let count_from_coverage = (protected_area.value / coverage_per_nozzle).ceil();
    let count = count_from_flow.max(count_from_coverage);
    let governing = if count_from_coverage > count_from_flow {
        GoverningCriterion::Coverage
    } else {
        GoverningCriterion::Flow
    };
    let grid = (protected_area.value / count).sqrt();
    let delivered_flow = count * flow_per_nozzle;
    let delivered_density = delivered_flow / protected_area.value;

    Ok(DelugeNozzleLayoutResult {
        required_flow: liters_per_minute(required_flow),
        flow_per_nozzle: liters_per_minute(flow_per_nozzle),
        coverage_per_nozzle: square_meters(coverage_per_nozzle),
        nozzle_count_from_flow: count_from_flow,
        nozzle_count_from_coverage: count_from_coverage,
        nozzle_count: count,
        governing_criterion: governing,
        grid_spacing: meters(grid),
        delivered_flow: liters_per_minute(delivered_flow),
        delivered_density,
        pressure_adequate: pressure >= nozzle_min_pressure.value,
        density_met: delivered_density >= required_density,
        clearance_adequate: available_clearance.value >= nozzle_clearance.value,
        warnings,
    })
}
