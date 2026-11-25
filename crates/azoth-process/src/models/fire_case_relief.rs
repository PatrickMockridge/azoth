//! `process.fire_case_relief` - the API 521 fire-case relief load.
//!
//! Spec: `specs/models/process/fire_case_relief.toml`. The arithmetic is
//! `process/safety/overpressure/FireCaseRelief.java`'s no-fluid path: the wetted
//! area, Table 4's heat input and `Q / latent_heat`.

use azoth_core::units::{
    Area, Length, MassRate, ThermodynamicTemperature, kilograms_per_hour, kilograms_per_second,
};
use azoth_core::{Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;

/// `Q = C1 F A_ft2^0.82`, the class's Table 4 correlation with F always 1.0. The
/// 21 000 coefficient is credited only with both drainage and firefighting; else 34 500.
fn fire_heat_input(area_m2: f64, has_drainage: bool, has_fire_fighting: bool) -> f64 {
    let area_ft2 = area_m2 * 10.7639;
    let coefficient = if has_drainage && has_fire_fighting {
        21_000.0
    } else {
        34_500.0
    };
    coefficient * area_ft2.powf(0.82) * 0.293_071_07
}

/// Result of `process.fire_case_relief`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FireCaseReliefResult {
    /// The latent heat used, in J/kg.
    pub latent_heat: f64,
    /// `heat_input / latent_heat`.
    #[serde(serialize_with = "scalar")]
    pub relief_rate: MassRate,
    /// The relief rate times 3600.
    #[serde(serialize_with = "scalar")]
    pub relief_rate_kg_per_hr: MassRate,
    /// The relieving temperature, echoed when stated.
    #[serde(serialize_with = "optional_scalar")]
    pub relieving_temperature: Option<ThermodynamicTemperature>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The wetted area, the Table 4 heat input and `Q / latent_heat`.
///
/// # Errors
/// `InvalidInput` for a negative area, diameter, height or latent heat, or a
/// non-positive relieving temperature.
pub fn fire_case_relief(
    wetted_area: Option<Area>,
    vessel_diameter: Option<Length>,
    wetted_height: Option<Length>,
    has_drainage: bool,
    has_fire_fighting: bool,
    latent_heat: Option<f64>,
    relieving_temperature: Option<ThermodynamicTemperature>,
) -> Result<FireCaseReliefResult> {
    let spec = &model_gen::FIRE_CASE_RELIEF_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "wetted_area" => wetted_area.map(|value| value.value),
            "vessel_diameter" => vessel_diameter.map(|value| value.value),
            "wetted_height" => wetted_height.map(|value| value.value),
            "latent_heat" => latent_heat,
            "relieving_temperature" => relieving_temperature.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    let area = wetted_area
        .map(|value| value.value)
        .filter(|value| *value > 0.0)
        .or_else(|| {
            let diameter = vessel_diameter.map(|value| value.value).unwrap_or(0.0);
            let height = wetted_height.map(|value| value.value).unwrap_or(0.0);
            if diameter > 0.0 && height > 0.0 {
                Some(std::f64::consts::PI * diameter * height)
            } else {
                None
            }
        })
        .unwrap_or(0.0);
    let heat_input_watts = if area > 0.0 {
        fire_heat_input(area, has_drainage, has_fire_fighting)
    } else {
        0.0
    };
    let latent = latent_heat
        .filter(|value| *value > 0.0)
        .unwrap_or(350_000.0);
    let relief_rate = heat_input_watts / latent;
    Ok(FireCaseReliefResult {
        latent_heat: latent,
        relief_rate: kilograms_per_second(relief_rate),
        relief_rate_kg_per_hr: kilograms_per_hour(relief_rate * 3600.0),
        relieving_temperature,
        warnings,
    })
}
