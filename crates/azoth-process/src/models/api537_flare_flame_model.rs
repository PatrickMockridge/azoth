//! `process.api537_flare_flame_model` - the API 537 flare flame as a registered id.
//!
//! Spec: `specs/models/process/api537_flare_flame_model.toml`. The arithmetic is
//! `process/safety/fire/Api537FlareFlameModel.java` at the pin: Kent's flame length, the
//! tilt, the point-source flux, the sterile zones and the noise.

use azoth_core::units::{
    HeatFluxDensity, Length, MassRate, Power, ThermodynamicTemperature, Velocity, kelvins, meters,
    watts, watts_per_square_meter,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;
use std::f64::consts::PI;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The Stefan-Boltzmann constant, W/(m2 K4).
const SIGMA: f64 = 5.670_374_419e-8;
/// The class's default stack height, m.
const DEFAULT_STACK_HEIGHT_M: f64 = 30.0;
/// The class's default wind speed, m/s.
const DEFAULT_WIND_SPEED_M_S: f64 = 0.0;
/// The class's default transmissivity.
const DEFAULT_TRANSMISSIVITY: f64 = 1.0;
/// The class's default acoustic efficiency.
const DEFAULT_ACOUSTIC_EFFICIENCY: f64 = 3.0e-6;

/// One flux level whose sterile radius is asked.
#[derive(Debug, Clone, PartialEq)]
pub struct FlareTargetFlux {
    /// The target flux.
    pub flux: HeatFluxDensity,
}

/// One sterile radius answer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SterileRadius {
    /// The target flux the radius is for.
    #[serde(serialize_with = "scalar")]
    pub flux: HeatFluxDensity,
    /// The sterile radius.
    #[serde(serialize_with = "scalar")]
    pub radius: Length,
}

/// Result of `process.api537_flare_flame_model`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Api537FlareFlameModelResult {
    /// `mass_flow heat_of_combustion`.
    #[serde(serialize_with = "scalar")]
    pub heat_release: Power,
    /// `radiative_fraction heat_release`.
    #[serde(serialize_with = "scalar")]
    pub radiative_power: Power,
    /// Kent's flame length.
    #[serde(serialize_with = "scalar")]
    pub flame_length: Length,
    /// The tilt from vertical in radians; dimensionless.
    pub flame_tilt: f64,
    /// The flame centre's horizontal offset.
    #[serde(serialize_with = "scalar")]
    pub flame_center_horizontal: Length,
    /// The flame centre's height.
    #[serde(serialize_with = "scalar")]
    pub flame_center_height: Length,
    /// The flame tip's horizontal offset.
    #[serde(serialize_with = "scalar")]
    pub flame_tip_horizontal: Length,
    /// The flame tip's height.
    #[serde(serialize_with = "scalar")]
    pub flame_tip_height: Length,
    /// The incident flux at the stated ground distance.
    #[serde(serialize_with = "scalar")]
    pub heat_flux_at_ground_distance: HeatFluxDensity,
    /// One row per target flux, in the caller's order.
    pub sterile_radii: Vec<SterileRadius>,
    /// The sound power level in dB; dimensionless.
    pub sound_power_level: f64,
    /// The sound pressure level at the stated distance in dB; dimensionless.
    pub sound_pressure_level: f64,
    /// The equivalent flame temperature.
    #[serde(serialize_with = "scalar")]
    pub equivalent_flame_temperature: ThermodynamicTemperature,
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

/// Kent's flame length, tilt, flux, sterile zones and noise.
///
/// # Errors
/// `InvalidInput` for a negative mass flow, a non-positive heat of combustion or exit
/// velocity, a radiative fraction outside `[0, 1]`, a negative distance, stack height or
/// efficiency, a transmissivity outside `[0, 1]`, a non-positive target flux, or a
/// non-finite input.
#[allow(clippy::too_many_arguments)]
pub fn api537_flare_flame_model(
    mass_flow: MassRate,
    heat_of_combustion: f64,
    radiative_fraction: f64,
    exit_velocity: Velocity,
    ground_distance: Length,
    sound_distance: Length,
    target_fluxes: &[FlareTargetFlux],
    stack_height: Option<Length>,
    wind_speed: Option<Velocity>,
    transmissivity: Option<f64>,
    acoustic_efficiency: Option<f64>,
) -> Result<Api537FlareFlameModelResult> {
    let spec = &model_gen::API537_FLARE_FLAME_MODEL_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "mass_flow" => Some(mass_flow.value),
            "heat_of_combustion" => Some(heat_of_combustion),
            "radiative_fraction" => Some(radiative_fraction),
            "exit_velocity" => Some(exit_velocity.value),
            "ground_distance" => Some(ground_distance.value),
            "sound_distance" => Some(sound_distance.value),
            "stack_height" => stack_height.map(|value| value.value),
            "wind_speed" => wind_speed.map(|value| value.value),
            "transmissivity" => transmissivity,
            "acoustic_efficiency" => acoustic_efficiency,
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite("mass_flow", mass_flow.value)?;
    require_finite("heat_of_combustion", heat_of_combustion)?;
    require_finite("radiative_fraction", radiative_fraction)?;
    require_finite("exit_velocity", exit_velocity.value)?;
    require_finite("ground_distance", ground_distance.value)?;
    require_finite("sound_distance", sound_distance.value)?;
    let stack = stack_height.map_or(DEFAULT_STACK_HEIGHT_M, |value| value.value);
    require_finite("stack_height", stack)?;
    let wind = wind_speed.map_or(DEFAULT_WIND_SPEED_M_S, |value| value.value);
    require_finite("wind_speed", wind)?;
    let tau = transmissivity.unwrap_or(DEFAULT_TRANSMISSIVITY);
    require_finite("transmissivity", tau)?;
    let eta = acoustic_efficiency.unwrap_or(DEFAULT_ACOUSTIC_EFFICIENCY);
    require_finite("acoustic_efficiency", eta)?;
    for target in target_fluxes {
        require_finite("target_fluxes.flux", target.flux.value)?;
        if target.flux.value <= 0.0 {
            return Err(AzothError::invalid_input(
                "target_fluxes.flux",
                "the class's `sterileZoneRadiusM` refuses a non-positive target flux",
            ));
        }
    }

    // The class's clamps, mirrored; the declared ranges make them unreachable through the
    // front door.
    let wind = wind.max(0.0);
    let tau = tau.clamp(0.0, 1.0);
    let eta = eta.max(0.0);

    // The class's operation order, to the letter.
    let heat_release = mass_flow.value * heat_of_combustion;
    let radiative_power = radiative_fraction * heat_release;
    let length = if heat_release <= 0.0 {
        0.0
    } else {
        0.00326 * heat_release.powf(0.478)
    };
    let tilt = wind.atan2(exit_velocity.value);
    let center_horizontal = 0.5 * length * tilt.sin();
    let center_height = stack + 0.5 * length * tilt.cos();
    let tip_horizontal = length * tilt.sin();
    let tip_height = stack + length * tilt.cos();
    let dx = ground_distance.value - center_horizontal;
    let dz = center_height;
    let mut r2 = dx * dx + dz * dz;
    if r2 < 1.0e-6 {
        r2 = 1.0e-6;
    }
    let flux = tau * radiative_power / (4.0 * PI * r2);
    let mut sterile = Vec::with_capacity(target_fluxes.len());
    for target in target_fluxes {
        let r_sphere = (tau * radiative_power / (4.0 * PI * target.flux.value)).sqrt();
        let radius = if r_sphere <= dz {
            0.0
        } else {
            let horizontal = (r_sphere * r_sphere - dz * dz).sqrt();
            (center_horizontal + horizontal).max(0.0)
        };
        sterile.push(SterileRadius {
            flux: target.flux,
            radius: meters(radius),
        });
    }
    let acoustic_power = eta * heat_release;
    let pwl = if acoustic_power <= 0.0 {
        0.0
    } else {
        10.0 * (acoustic_power / 1.0e-12).log10()
    };
    let spl = pwl - 20.0 * sound_distance.value.max(1.0).log10() - 11.0;
    let surface = PI * (length * 0.12).max(0.1) * length;
    let temperature = if surface <= 0.0 {
        0.0
    } else {
        (radiative_power / surface / SIGMA).powf(0.25)
    };

    Ok(Api537FlareFlameModelResult {
        heat_release: watts(heat_release),
        radiative_power: watts(radiative_power),
        flame_length: meters(length),
        flame_tilt: tilt,
        flame_center_horizontal: meters(center_horizontal),
        flame_center_height: meters(center_height),
        flame_tip_horizontal: meters(tip_horizontal),
        flame_tip_height: meters(tip_height),
        heat_flux_at_ground_distance: watts_per_square_meter(flux),
        sterile_radii: sterile,
        sound_power_level: pwl,
        sound_pressure_level: spl,
        equivalent_flame_temperature: kelvins(temperature),
        warnings,
    })
}
