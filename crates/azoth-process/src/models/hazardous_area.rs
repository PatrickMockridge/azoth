//! `process.hazardous_area` - the IEC 60079-10-1 jet area classification.
//!
//! Spec: `specs/models/process/hazardous_area.toml`. The arithmetic is
//! `process/safety/dispersion/HazardousAreaCalculator.java` at the pin: the effective
//! source diameter, the mole-to-mass LFL, the de-rated distance and the zone mapping.

use azoth_core::units::{Length, MassDensity, MassRate, MolarMass, Velocity, meters};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// Molar mass of air, kg/mol, the class's constant.
const M_AIR: f64 = 0.028964;
/// The Chen & Rodi axial-decay constant, the class's `JET_DECAY_CONSTANT`.
const JET_DECAY_CONSTANT: f64 = 5.4;
/// The class's default ambient air density, kg/m3.
const DEFAULT_AMBIENT_DENSITY: f64 = 1.225;
/// The class's default factor below the LFL.
const DEFAULT_SAFETY_FACTOR: f64 = 0.5;

/// Result of `process.hazardous_area`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HazardousAreaResult {
    /// `d_e = sqrt(4 mdot / (pi rho_e U_e))`, the notional source diameter.
    #[serde(serialize_with = "scalar")]
    pub effective_diameter: Length,
    /// The class's ideal mole-to-mass LFL.
    pub lfl_mass_fraction: f64,
    /// `5.4 sqrt(rho_e/rho_a) d_e / (k Y_LFL)`.
    #[serde(serialize_with = "scalar")]
    pub hazardous_distance: Length,
    /// The class's grade label.
    pub release_grade: String,
    /// The class's zone label.
    pub zone_classification: String,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The class's grade-to-zone mapping, and its default.
fn grade(value: Option<&str>) -> Result<(&'static str, &'static str)> {
    Ok(match value.unwrap_or("secondary") {
        "continuous" => ("CONTINUOUS", "Zone 0"),
        "primary" => ("PRIMARY", "Zone 1"),
        "secondary" => ("SECONDARY", "Zone 2"),
        other => {
            return Err(AzothError::invalid_input(
                "release_grade",
                format!("a grade is `continuous`, `primary` or `secondary`, and {other:?} is not"),
            ));
        }
    })
}

fn require_positive(name: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(AzothError::invalid_input(
            name,
            format!("a finite positive value is required, and {value} is not"),
        ));
    }
    Ok(())
}

/// The hazardous distance and zone for a continuous gas jet.
///
/// # Errors
/// `InvalidInput` for a non-positive or non-finite flow, density, velocity, molar mass or
/// ambient density, an LFL outside `(0, 1)`, a safety factor outside `(0, 1]`, or an
/// unknown grade.
#[allow(clippy::too_many_arguments)] // one parameter per declared input
pub fn hazardous_area(
    mass_flow: MassRate,
    exit_density: MassDensity,
    exit_velocity: Velocity,
    lfl_volume_fraction: f64,
    gas_molar_mass: MolarMass,
    ambient_density: Option<MassDensity>,
    safety_factor: Option<f64>,
    release_grade: Option<&str>,
) -> Result<HazardousAreaResult> {
    let flow = mass_flow.value;
    let density = exit_density.value;
    let velocity = exit_velocity.value;
    let molar_mass = gas_molar_mass.value;
    let ambient = ambient_density.map_or(DEFAULT_AMBIENT_DENSITY, |value| value.value);
    let factor = safety_factor.unwrap_or(DEFAULT_SAFETY_FACTOR);
    let (grade_label, zone) = grade(release_grade)?;
    let spec = &model_gen::HAZARDOUS_AREA_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "mass_flow" => Some(flow),
            "exit_density" => Some(density),
            "exit_velocity" => Some(velocity),
            "lfl_volume_fraction" => Some(lfl_volume_fraction),
            "gas_molar_mass" => Some(molar_mass),
            "ambient_density" => Some(ambient),
            "safety_factor" => Some(factor),
            _ => None,
        },
        &mut warnings,
    )?;
    require_positive("mass_flow", flow)?;
    require_positive("exit_density", density)?;
    require_positive("exit_velocity", velocity)?;
    require_positive("gas_molar_mass", molar_mass)?;
    require_positive("ambient_density", ambient)?;
    if !lfl_volume_fraction.is_finite() || lfl_volume_fraction <= 0.0 || lfl_volume_fraction >= 1.0
    {
        return Err(AzothError::invalid_input(
            "lfl_volume_fraction",
            format!(
                "an LFL in (0, 1) is required, and {lfl_volume_fraction} is not"
            ),
        ));
    }
    if !factor.is_finite() || factor <= 0.0 || factor > 1.0 {
        return Err(AzothError::invalid_input(
            "safety_factor",
            format!("a factor in (0, 1] is required, and {factor} is not"),
        ));
    }

    let diameter = (4.0 * flow / (std::f64::consts::PI * density * velocity)).sqrt();
    let numerator = lfl_volume_fraction * molar_mass;
    let lfl_mass = numerator / (numerator + (1.0 - lfl_volume_fraction) * M_AIR);
    let distance = JET_DECAY_CONSTANT * (density / ambient).sqrt() * diameter / (factor * lfl_mass);
    Ok(HazardousAreaResult {
        effective_diameter: meters(diameter),
        lfl_mass_fraction: lfl_mass,
        hazardous_distance: meters(distance),
        release_grade: grade_label.to_owned(),
        zone_classification: zone.to_owned(),
        warnings,
    })
}
