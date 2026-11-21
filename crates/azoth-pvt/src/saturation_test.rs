//! `pvt.saturation_test` - NeqSim's upper saturation boundary of a fluid.
//!
//! Spec: `specs/models/pvt/saturation_test.toml`. The arithmetic is
//! `pvtsimulation/simulation/SaturationPressure.java` and `SaturationTemperature.java` at the
//! pin: a descending 10-unit scan for the uppermost two-phase crossing, bisected to `1e-5`,
//! with the classes' grid-top fallback carried as a warning.

use azoth_characterization::KijSource;
use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{Pressure, ThermodynamicTemperature};
use azoth_core::{AzothError, Result, Warning};
use azoth_eos::Cubic;

use crate::fluid::Fluid;
use crate::saturation::{upper_saturation_pressure, upper_saturation_temperature};

/// Result of `pvt.saturation_test`.
#[derive(Debug, Clone, PartialEq)]
pub struct SaturationTestResult {
    /// The upper saturation pressure, in Pa, in pressure mode.
    pub saturation_pressure: Option<f64>,
    /// The upper saturation temperature, in K, in temperature mode.
    pub saturation_temperature: Option<f64>,
    /// Caveats, including the class's unbracketed fallback.
    pub warnings: Vec<Warning>,
}

/// The upper saturation boundary of a fluid.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown `mode`, `eos` or `mixing_rule`, a missing
///   fixed temperature or pressure, search bounds outside `[30, 1200] K` or not strictly
///   increasing - and propagates the fluid build and the flashes' refusals.
#[allow(clippy::too_many_arguments)]
pub fn saturation_test(
    components: &[FluidComponent],
    mode: &str,
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    temperature: Option<ThermodynamicTemperature>,
    pressure: Option<Pressure>,
    temperature_search_min: Option<ThermodynamicTemperature>,
    temperature_search_max: Option<ThermodynamicTemperature>,
) -> Result<SaturationTestResult> {
    let fluid = Fluid::from_rows(components, cubic(eos)?, kij_source(mixing_rule)?, true)?;

    let bounds = match (temperature_search_min, temperature_search_max) {
        (None, None) => None,
        (Some(min), Some(max)) => Some((min.value, max.value)),
        _ => {
            return Err(AzothError::invalid_input(
                "temperature_search",
                "the search bounds are stated together or not at all",
            ));
        }
    };

    match mode {
        "pressure" => {
            if bounds.is_some() {
                return Err(AzothError::invalid_input(
                    "temperature_search",
                    "bounds belong to the temperature search, and this is a pressure search",
                ));
            }
            let t = temperature.ok_or_else(|| {
                AzothError::invalid_input(
                    "temperature",
                    "a pressure-mode search needs a temperature",
                )
            })?;
            let point = upper_saturation_pressure(&fluid, t.value)?;
            Ok(SaturationTestResult {
                saturation_pressure: point.pressure,
                saturation_temperature: None,
                warnings: point.warnings,
            })
        }
        "temperature" => {
            let p = pressure.ok_or_else(|| {
                AzothError::invalid_input("pressure", "a temperature-mode search needs a pressure")
            })?;
            let point = upper_saturation_temperature(&fluid, p.value, bounds)?;
            Ok(SaturationTestResult {
                saturation_pressure: None,
                saturation_temperature: point.temperature,
                warnings: point.warnings,
            })
        }
        other => Err(AzothError::invalid_input(
            "mode",
            format!("{other} is not a saturation search; use pressure or temperature"),
        )),
    }
}

fn cubic(eos: Option<&str>) -> Result<Cubic> {
    match eos.unwrap_or("srk") {
        "srk" => Ok(Cubic::Srk),
        "pr" => Ok(Cubic::Pr),
        other => Err(AzothError::invalid_input(
            "eos",
            format!("{other} is not one of srk and pr"),
        )),
    }
}

fn kij_source(mixing_rule: Option<&str>) -> Result<KijSource> {
    match mixing_rule.unwrap_or("classic") {
        "classic" => Ok(KijSource::Database),
        "zero" => Ok(KijSource::Zero),
        other => Err(AzothError::invalid_input(
            "mixing_rule",
            format!("{other} is not one of classic and zero"),
        )),
    }
}
