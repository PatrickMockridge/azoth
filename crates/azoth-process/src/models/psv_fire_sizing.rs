//! `process.psv_fire_sizing` - the API 521 dynamic fire-case sizing.
//!
//! Spec: `specs/models/process/psv_fire_sizing.toml`. The arithmetic is
//! `process/util/fire/ReliefValveSizing.java` at the pin: the design flow, the
//! one-minute heat-up, the 21 % gas area and the blowdown/Cv/orifice helpers.

use azoth_core::units::{
    Area, Mass, MolarMass, Power, Pressure, SpecificHeatCapacity, ThermodynamicTemperature, Time,
    Volume, pascals, square_meters,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;

/// The standard API 520 orifice ladder, D to T.
const ORIFICES: [(&str, f64); 14] = [
    ("D", 0.110),
    ("E", 0.196),
    ("F", 0.307),
    ("G", 0.503),
    ("H", 0.785),
    ("J", 1.287),
    ("K", 1.838),
    ("L", 2.853),
    ("M", 3.600),
    ("N", 4.340),
    ("P", 6.380),
    ("Q", 11.05),
    ("R", 16.0),
    ("T", 26.0),
];

fn select_orifice(required_area_in2: f64) -> (String, f64) {
    for (letter, area) in ORIFICES {
        if area >= required_area_in2 {
            return (letter.to_string(), area);
        }
    }
    ("T".to_string(), 26.0)
}

/// The design flow the class sizes the gas branch with.
struct DesignFlow {
    mass_flow: f64,
    temperature: f64,
    back_pressure: f64,
    overpressure_fraction: f64,
}

fn design_flow(
    initial_mass: f64,
    initial_temperature: f64,
    fire_heat_input: f64,
    heat_capacity: f64,
    blowdown_time: f64,
) -> DesignFlow {
    let overpressure_fraction = 0.21;
    let delta_t = fire_heat_input * 60.0 / (initial_mass * heat_capacity);
    let relieving_temperature = initial_temperature + delta_t;
    let peak_mass_flow = fire_heat_input / (heat_capacity * 50.0);
    let average_mass_flow = initial_mass / blowdown_time;
    let design_mass_flow = peak_mass_flow.max(average_mass_flow);
    DesignFlow {
        mass_flow: design_mass_flow,
        temperature: relieving_temperature,
        back_pressure: 101_325.0,
        overpressure_fraction,
    }
}

#[allow(clippy::too_many_arguments)] // one parameter per quantity the class method reads
fn gas_area(
    mass_flow: f64,
    set_pressure: f64,
    overpressure_fraction: f64,
    back_pressure: f64,
    temperature: f64,
    molar_mass: f64,
    compressibility: f64,
    specific_heat_ratio: f64,
    balanced_bellows: bool,
    has_rupture_disk: bool,
) -> (f64, f64, String, f64, f64, f64, f64) {
    let k = specific_heat_ratio;
    let relieving_pressure = set_pressure * (1.0 + overpressure_fraction);
    let back_pressure_fraction = back_pressure / relieving_pressure;
    let kd = 0.975;
    let kc = if has_rupture_disk { 0.9 } else { 1.0 };
    let critical_ratio = (2.0 / (k + 1.0)).powf(k / (k - 1.0));
    let kb = if balanced_bellows {
        if back_pressure_fraction > critical_ratio {
            let r = back_pressure_fraction;
            let value = ((k / (k - 1.0)) * (r.powf(2.0 / k) - r.powf((k + 1.0) / k))
                / (1.0 - critical_ratio))
                .sqrt();
            value.min(1.0)
        } else {
            1.0
        }
    } else if back_pressure_fraction > 0.5 {
        (1.0 - (back_pressure_fraction - 0.5) * 2.0).max(0.1)
    } else {
        1.0
    };
    let c = 520.0 * (k * (2.0 / (k + 1.0)).powf((k + 1.0) / (k - 1.0))).sqrt();
    let molar_mass_g_per_mol = molar_mass * 1000.0;
    let pressure_psia = relieving_pressure / 6894.76;
    let mass_flow_lb_per_hr = mass_flow * 7936.64;
    let temperature_rankine = temperature * 1.8;
    let area_in2 = mass_flow_lb_per_hr / (c * kd * kb * kc * pressure_psia)
        * (temperature_rankine * compressibility / molar_mass_g_per_mol).sqrt();
    let (recommended_orifice, selected_area_in2) = select_orifice(area_in2);
    (
        area_in2 * 6.4516e-4,
        area_in2,
        recommended_orifice,
        selected_area_in2,
        back_pressure_fraction,
        kb,
        kc,
    )
}

/// Result of `process.psv_fire_sizing`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PsvFireSizingResult {
    /// The class's 0.21.
    pub overpressure_fraction: f64,
    /// The required orifice area.
    #[serde(serialize_with = "scalar")]
    pub required_area: Area,
    /// The required area in in².
    pub required_area_in2: f64,
    /// The selected ladder letter.
    pub recommended_orifice: String,
    /// The selected ladder area in in².
    pub selected_area_in2: f64,
    /// `101325 / (set_pressure 1.21)`.
    pub back_pressure_fraction: f64,
    /// The gas `Kd`.
    pub discharge_coefficient: f64,
    /// The gas `Kb`.
    pub back_pressure_correction: f64,
    /// The gas `Kc`.
    pub combination_correction: f64,
    /// `set_pressure (1 - blowdown_percent / 100)`.
    #[serde(serialize_with = "optional_scalar")]
    pub blowdown_pressure: Option<Pressure>,
    /// `24.6 cv_discharge_coefficient A_in2`.
    pub flow_coefficient: Option<f64>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The dynamic fire design flow, the 21 % area and the three helpers.
///
/// # Errors
/// `InvalidInput` for a non-positive mass, time, heat capacity, pressure, temperature,
/// molar mass or compressibility; an exponent at or below one; a negative fire heat
/// input; a blowdown percentage outside `[0, 100]`; or an area and coefficient that are
/// not stated together.
#[allow(clippy::too_many_arguments)] // one parameter per declared input, and there are fourteen
pub fn psv_fire_sizing(
    initial_mass: Mass,
    initial_pressure: Pressure,
    set_pressure: Pressure,
    initial_temperature: ThermodynamicTemperature,
    fire_heat_input: Power,
    vessel_volume: Volume,
    molar_mass: MolarMass,
    specific_heat_ratio: f64,
    compressibility: f64,
    heat_capacity: SpecificHeatCapacity,
    blowdown_time: Time,
    blowdown_percent: Option<f64>,
    orifice_area: Option<Area>,
    cv_discharge_coefficient: Option<f64>,
) -> Result<PsvFireSizingResult> {
    let spec = &model_gen::PSV_FIRE_SIZING_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "initial_mass" => Some(initial_mass.value),
            "initial_pressure" => Some(initial_pressure.value),
            "set_pressure" => Some(set_pressure.value),
            "initial_temperature" => Some(initial_temperature.value),
            "fire_heat_input" => Some(fire_heat_input.value),
            "vessel_volume" => Some(vessel_volume.value),
            "molar_mass" => Some(molar_mass.value),
            "specific_heat_ratio" => Some(specific_heat_ratio),
            "compressibility" => Some(compressibility),
            "heat_capacity" => Some(heat_capacity.value),
            "blowdown_time" => Some(blowdown_time.value),
            "blowdown_percent" => blowdown_percent,
            "orifice_area" => orifice_area.map(|value| value.value),
            "cv_discharge_coefficient" => cv_discharge_coefficient,
            _ => None,
        },
        &mut warnings,
    )?;
    for (name, value) in [
        ("initial_mass", initial_mass.value),
        ("set_pressure", set_pressure.value),
        ("initial_temperature", initial_temperature.value),
        ("molar_mass", molar_mass.value),
        ("compressibility", compressibility),
        ("heat_capacity", heat_capacity.value),
        ("blowdown_time", blowdown_time.value),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(AzothError::invalid_input(
                name,
                format!("a finite positive value is required, and {value} is not"),
            ));
        }
    }
    if !fire_heat_input.value.is_finite() || fire_heat_input.value < 0.0 {
        return Err(AzothError::invalid_input(
            "fire_heat_input",
            format!(
                "a finite non-negative heat input is required, and {} is not",
                fire_heat_input.value
            ),
        ));
    }
    if !specific_heat_ratio.is_finite() || specific_heat_ratio <= 1.0 {
        return Err(AzothError::invalid_input(
            "specific_heat_ratio",
            format!("an exponent above one is required, and {specific_heat_ratio} is not"),
        ));
    }
    if let Some(percent) = blowdown_percent
        && (!percent.is_finite() || !(0.0..=100.0).contains(&percent))
    {
        return Err(AzothError::invalid_input(
            "blowdown_percent",
            format!("a percentage in [0, 100] is required, and {percent} is not"),
        ));
    }

    let flow = design_flow(
        initial_mass.value,
        initial_temperature.value,
        fire_heat_input.value,
        heat_capacity.value,
        blowdown_time.value,
    );
    let (area_m2, area_in2, recommended, selected, back_pressure_fraction, kb, kc) = gas_area(
        flow.mass_flow,
        set_pressure.value,
        flow.overpressure_fraction,
        flow.back_pressure,
        flow.temperature,
        molar_mass.value,
        compressibility,
        specific_heat_ratio,
        false,
        false,
    );
    let blowdown_pressure =
        blowdown_percent.map(|percent| pascals(set_pressure.value * (1.0 - percent / 100.0)));
    let flow_coefficient = match (orifice_area, cv_discharge_coefficient) {
        (Some(area), Some(coefficient)) => Some(24.6 * coefficient * (area.value / 6.4516e-4)),
        (None, None) => None,
        (Some(_), None) => {
            return Err(AzothError::invalid_input(
                "cv_discharge_coefficient",
                "is required with orifice_area",
            ));
        }
        (None, Some(_)) => {
            return Err(AzothError::invalid_input(
                "orifice_area",
                "is required with cv_discharge_coefficient",
            ));
        }
    };
    Ok(PsvFireSizingResult {
        overpressure_fraction: flow.overpressure_fraction,
        required_area: square_meters(area_m2),
        required_area_in2: area_in2,
        recommended_orifice: recommended,
        selected_area_in2: selected,
        back_pressure_fraction,
        discharge_coefficient: 0.975,
        back_pressure_correction: kb,
        combination_correction: kc,
        blowdown_pressure,
        flow_coefficient,
        warnings,
    })
}
