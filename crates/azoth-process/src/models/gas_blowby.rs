//! `process.gas_blowby` - the HP-to-LP blowby screen as a registered id.
//!
//! Spec: `specs/models/process/gas_blowby.toml`. The arithmetic is
//! `process/safety/blowby/GasBlowbyAnalyzer.java` at the pin: the isentropic nozzle forms,
//! the 15 C standard volume and the relief-capacity verdict.

use azoth_core::units::{
    Area, MassRate, MolarMass, Pressure, ThermodynamicTemperature, VolumeRate,
    cubic_meters_per_second, kilograms_per_second,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's gas constant, J/(mol K).
const R_GAS: f64 = 8.314_462_618;
/// The class's standard temperature, K (15 C).
const STANDARD_TEMPERATURE_K: f64 = 288.15;
/// The class's standard pressure, Pa (1 atm).
const STANDARD_PRESSURE_PA: f64 = 101_325.0;
/// The class's default discharge coefficient.
const DEFAULT_DISCHARGE_COEFFICIENT: f64 = 0.85;

/// Whether a stated relief capacity absorbs the blowby.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BlowbyVerdict {
    /// No downstream relief capacity was stated.
    #[serde(rename = "no_relief_data")]
    NoReliefData,
    /// The capacity is at least the blowby rate.
    #[serde(rename = "relief_adequate")]
    ReliefAdequate,
    /// The capacity is below the blowby rate.
    #[serde(rename = "relief_inadequate")]
    ReliefInadequate,
}

impl BlowbyVerdict {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoReliefData => "no_relief_data",
            Self::ReliefAdequate => "relief_adequate",
            Self::ReliefInadequate => "relief_inadequate",
        }
    }
}

/// Result of `process.gas_blowby`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GasBlowbyResult {
    /// Whether the flow is choked.
    pub choked: bool,
    /// `(2/(k+1))^(k/(k-1))`.
    pub critical_pressure_ratio: f64,
    /// `p2/p1`.
    pub actual_pressure_ratio: f64,
    /// The blowby mass rate.
    #[serde(serialize_with = "scalar")]
    pub blowby_mass_rate: MassRate,
    /// The same rate at 288.15 K and 101325 Pa.
    #[serde(serialize_with = "scalar")]
    pub blowby_std_vol_rate: VolumeRate,
    /// `capacity - blowby_mass_rate`; absent when no capacity is stated.
    #[serde(serialize_with = "optional_scalar")]
    pub relief_margin: Option<MassRate>,
    /// Whether the capacity is at least the rate.
    pub relief_adequate: bool,
    /// The class's verdict.
    pub verdict: BlowbyVerdict,
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

/// The blowby rate through a fully open restriction and the relief verdict.
///
/// # Errors
/// `InvalidInput` for a non-positive source pressure, temperature, area or molar mass, a
/// negative downstream pressure or relief capacity, a specific-heat ratio at or below one, a
/// discharge coefficient outside `(0, 1]`, a downstream pressure at or above the source, or a
/// non-finite input.
#[allow(clippy::too_many_arguments)] // The signature is the spec's declared input list.
pub fn gas_blowby(
    upstream_pressure: Pressure,
    upstream_temperature: ThermodynamicTemperature,
    downstream_pressure: Pressure,
    restriction_area: Area,
    specific_heat_ratio: f64,
    molar_mass: MolarMass,
    discharge_coefficient: Option<f64>,
    downstream_relief_capacity: Option<MassRate>,
) -> Result<GasBlowbyResult> {
    let spec = &model_gen::GAS_BLOWBY_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "upstream_pressure" => Some(upstream_pressure.value),
            "upstream_temperature" => Some(upstream_temperature.value),
            "downstream_pressure" => Some(downstream_pressure.value),
            "restriction_area" => Some(restriction_area.value),
            "specific_heat_ratio" => Some(specific_heat_ratio),
            "molar_mass" => Some(molar_mass.value),
            "discharge_coefficient" => discharge_coefficient,
            "downstream_relief_capacity" => downstream_relief_capacity.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite("upstream_pressure", upstream_pressure.value)?;
    require_finite("upstream_temperature", upstream_temperature.value)?;
    require_finite("downstream_pressure", downstream_pressure.value)?;
    require_finite("restriction_area", restriction_area.value)?;
    require_finite("specific_heat_ratio", specific_heat_ratio)?;
    require_finite("molar_mass", molar_mass.value)?;
    if let Some(value) = discharge_coefficient {
        require_finite("discharge_coefficient", value)?;
    }
    if let Some(value) = downstream_relief_capacity {
        require_finite("downstream_relief_capacity", value.value)?;
    }
    if downstream_pressure.value >= upstream_pressure.value {
        return Err(AzothError::invalid_input(
            "downstream_pressure",
            "the downstream pressure must be below the upstream pressure for blowby",
        ));
    }

    let k = specific_heat_ratio;
    let m = molar_mass.value;
    let p1 = upstream_pressure.value;
    let t1 = upstream_temperature.value;
    let critical_pressure_ratio = (2.0 / (k + 1.0)).powf(k / (k - 1.0));
    let actual_pressure_ratio = downstream_pressure.value / p1;
    let choked = actual_pressure_ratio <= critical_pressure_ratio;
    let cd = discharge_coefficient.unwrap_or(DEFAULT_DISCHARGE_COEFFICIENT);
    let mass_rate = if choked {
        cd * restriction_area.value
            * p1
            * (k * m / (R_GAS * t1) * (2.0 / (k + 1.0)).powf((k + 1.0) / (k - 1.0))).sqrt()
    } else {
        // The cancellation at the critical ratio is floored at zero; the class's square root
        // would answer NaN for a rounded negative.
        let term = (actual_pressure_ratio.powf(2.0 / k)
            - actual_pressure_ratio.powf((k + 1.0) / k))
        .max(0.0);
        cd * restriction_area.value * p1 * (2.0 * m / (R_GAS * t1) * (k / (k - 1.0)) * term).sqrt()
    };
    let std_vol_rate = (mass_rate / m) * R_GAS * STANDARD_TEMPERATURE_K / STANDARD_PRESSURE_PA;

    let (relief_margin, relief_adequate, verdict) = match downstream_relief_capacity {
        Some(capacity) => {
            let margin = capacity.value - mass_rate;
            let adequate = margin >= 0.0;
            (
                Some(kilograms_per_second(margin)),
                adequate,
                if adequate {
                    BlowbyVerdict::ReliefAdequate
                } else {
                    BlowbyVerdict::ReliefInadequate
                },
            )
        }
        None => (None, false, BlowbyVerdict::NoReliefData),
    };

    Ok(GasBlowbyResult {
        choked,
        critical_pressure_ratio,
        actual_pressure_ratio,
        blowby_mass_rate: kilograms_per_second(mass_rate),
        blowby_std_vol_rate: cubic_meters_per_second(std_vol_rate),
        relief_margin,
        relief_adequate,
        verdict,
        warnings,
    })
}
