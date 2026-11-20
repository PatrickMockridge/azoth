//! The fire exposure NeqSim's `FireExposureScenario` and `FireHeatLoadCalculator` carry.
//!
//! The API 521 pool-fire heat load, the fixed flux, the generalized Stefan-Boltzmann
//! radiative flux, and the passive-protection multiplier. `process.trapped_liquid_fire_rupture`
//! is their only caller.

use azoth_core::{AzothError, Result};

/// The Stefan-Boltzmann constant, W/(m² K⁴).
const STEFAN_BOLTZMANN: f64 = 5.670374419e-8;

/// The class's fire type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FireType {
    /// API 521 pool fire.
    Api521PoolFire,
    /// A fixed heat flux.
    FixedHeatFlux,
    /// A radiative fire.
    RadiativeFire,
}

/// One fire exposure.
#[derive(Debug, Clone)]
pub(crate) struct FireExposure {
    /// The scenario's name.
    pub(crate) name: String,
    /// The scenario's type.
    pub(crate) fire_type: FireType,
    /// The exposed external area, m².
    pub(crate) exposed_area: f64,
    /// The fixed heat flux, W/m².
    pub(crate) heat_flux: f64,
    /// The API 521 environmental factor.
    pub(crate) environmental_factor: f64,
    /// The effective flame emissivity.
    pub(crate) emissivity: f64,
    /// The geometric view factor.
    pub(crate) view_factor: f64,
    /// The flame temperature, K.
    pub(crate) flame_temperature: f64,
    /// The ambient temperature, K.
    pub(crate) ambient_temperature: f64,
    /// The passive-protection heat-flux multiplier.
    pub(crate) passive_protection_factor: f64,
}

impl FireExposure {
    /// The class's API 521 pool-fire exposure.
    pub(crate) fn api521_pool_fire(exposed_area: f64, environmental_factor: f64) -> Result<Self> {
        validate_positive(exposed_area, "exposed_area")?;
        validate_positive(environmental_factor, "environmental_factor")?;
        let total_heat = api521_pool_fire_heat_load(exposed_area, environmental_factor)?;
        Ok(Self {
            name: "API 521 pool fire".to_string(),
            fire_type: FireType::Api521PoolFire,
            exposed_area,
            heat_flux: total_heat / exposed_area,
            environmental_factor,
            emissivity: 0.0,
            view_factor: 0.0,
            flame_temperature: 1200.0,
            ambient_temperature: 298.15,
            passive_protection_factor: 1.0,
        })
    }

    /// The class's fixed-flux exposure.
    pub(crate) fn fixed_heat_flux(exposed_area: f64, heat_flux: f64) -> Result<Self> {
        validate_positive(exposed_area, "exposed_area")?;
        validate_positive(heat_flux, "fire_heat_flux")?;
        Ok(Self {
            name: "Fixed heat flux".to_string(),
            fire_type: FireType::FixedHeatFlux,
            exposed_area,
            heat_flux,
            environmental_factor: 1.0,
            emissivity: 0.0,
            view_factor: 0.0,
            flame_temperature: 1200.0,
            ambient_temperature: 298.15,
            passive_protection_factor: 1.0,
        })
    }

    /// The class's radiative exposure.
    pub(crate) fn radiative_fire(
        exposed_area: f64,
        emissivity: f64,
        view_factor: f64,
        flame_temperature: f64,
        ambient_temperature: f64,
    ) -> Result<Self> {
        validate_positive(exposed_area, "exposed_area")?;
        validate_positive(ambient_temperature, "ambient_temperature")?;
        Ok(Self {
            name: "Radiative fire".to_string(),
            fire_type: FireType::RadiativeFire,
            exposed_area,
            heat_flux: 0.0,
            environmental_factor: 1.0,
            emissivity,
            view_factor,
            flame_temperature,
            ambient_temperature,
            passive_protection_factor: 1.0,
        })
    }

    /// The class's `withPassiveProtectionFactor`.
    pub(crate) fn with_passive_protection(mut self, factor: f64) -> Result<Self> {
        if !(0.0..=1.0).contains(&factor) || !factor.is_finite() {
            return Err(AzothError::invalid_input(
                "passive_protection_factor",
                "the class refuses a multiplier outside [0, 1]",
            ));
        }
        self.passive_protection_factor = factor;
        Ok(self)
    }

    /// The incident heat flux into the outer surface, W/m².
    pub(crate) fn incident_heat_flux(&self, outer_surface_temperature: f64) -> Result<f64> {
        validate_positive(outer_surface_temperature, "outer_surface_temperature")?;
        let flux = if self.fire_type == FireType::RadiativeFire {
            generalized_stefan_boltzmann_heat_flux(
                self.emissivity,
                self.view_factor,
                self.flame_temperature,
                outer_surface_temperature,
            )?
        } else {
            self.heat_flux
        };
        Ok(flux.max(0.0) * self.passive_protection_factor)
    }

    /// The total incident heat input, W.
    pub(crate) fn heat_input(&self, outer_surface_temperature: f64) -> Result<f64> {
        Ok(self.incident_heat_flux(outer_surface_temperature)? * self.exposed_area)
    }
}

/// The class's API 521 pool-fire heat load, W.
pub(crate) fn api521_pool_fire_heat_load(
    wetted_area: f64,
    environmental_factor: f64,
) -> Result<f64> {
    validate_positive(wetted_area, "wetted_area")?;
    validate_positive(environmental_factor, "environmental_factor")?;
    Ok(6.19e6 * environmental_factor * wetted_area.powf(0.82))
}

/// The class's generalized Stefan-Boltzmann radiative flux, W/m².
pub(crate) fn generalized_stefan_boltzmann_heat_flux(
    emissivity: f64,
    view_factor: f64,
    flame_temperature: f64,
    surface_temperature: f64,
) -> Result<f64> {
    if !(0.0..=1.0).contains(&emissivity) || !emissivity.is_finite() {
        return Err(AzothError::invalid_input(
            "fire_emissivity",
            "the class refuses an emissivity outside [0, 1]",
        ));
    }
    if !(0.0..=1.0).contains(&view_factor) || !view_factor.is_finite() {
        return Err(AzothError::invalid_input(
            "fire_view_factor",
            "the class refuses a view factor outside [0, 1]",
        ));
    }
    validate_positive(flame_temperature, "flame_temperature")?;
    validate_positive(surface_temperature, "outer_surface_temperature")?;
    Ok(emissivity
        * view_factor
        * STEFAN_BOLTZMANN
        * (flame_temperature.powi(4) - surface_temperature.powi(4)))
}

/// The class's positive-and-finite check.
fn validate_positive(value: f64, name: &'static str) -> Result<()> {
    if value <= 0.0 || !value.is_finite() {
        return Err(AzothError::invalid_input(
            name,
            "the class requires a positive finite value",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_521_load_and_the_radiative_flux_are_the_classes() {
        let load = api521_pool_fire_heat_load(1.0, 1.0).expect("the load evaluates");
        assert!((load - 6.19e6).abs() < 1.0e-6);
        let scenario = FireExposure::api521_pool_fire(2.0, 1.0).expect("the scenario builds");
        assert!(
            scenario
                .incident_heat_flux(300.0)
                .expect("the flux evaluates")
                > 0.0
        );
        let flux = generalized_stefan_boltzmann_heat_flux(1.0, 1.0, 500.0, 500.0)
            .expect("the flux evaluates");
        assert_eq!(flux, 0.0);
    }
}
