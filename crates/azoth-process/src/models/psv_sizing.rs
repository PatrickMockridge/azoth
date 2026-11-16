//! `process.psv_sizing` - the API 520 relief-orifice arithmetic.
//!
//! Spec: `specs/models/process/psv_sizing.toml`. The arithmetic is
//! `process/util/fire/ReliefValveSizing.java` at the pin: the gas, liquid and
//! two-phase area methods, the D to T ladder and the capacity helper whose unit
//! basis disagrees with the sizing path.

use azoth_core::units::{
    Area, DynamicViscosity, MassDensity, MassRate, MolarMass, Pressure, SpecificHeatCapacity,
    ThermodynamicTemperature, VolumeRate, kilograms_per_second, pascals, square_meters,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;

/// The service branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Service {
    /// Compressible gas.
    Gas,
    /// Incompressible liquid.
    Liquid,
    /// Flashing two-phase, Leung's omega method.
    TwoPhase,
}

impl Service {
    fn parse(name: &str) -> Result<Self> {
        match name {
            "gas" => Ok(Self::Gas),
            "liquid" => Ok(Self::Liquid),
            "two_phase" => Ok(Self::TwoPhase),
            _ => Err(AzothError::invalid_input(
                "service",
                format!("`{name}` is not `gas`, `liquid` or `two_phase`"),
            )),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Gas => "gas",
            Self::Liquid => "liquid",
            Self::TwoPhase => "two_phase",
        }
    }
}

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

/// The first ladder row at or above the requirement, else T at 26.0 in².
pub(crate) fn select_orifice(required_area_in2: f64) -> (String, f64) {
    for (letter, area) in ORIFICES {
        if area >= required_area_in2 {
            return (letter.to_string(), area);
        }
    }
    ("T".to_string(), 26.0)
}

/// The gas area and its corrections.
pub(crate) struct GasArea {
    pub(crate) required_area: f64,
    pub(crate) required_area_in2: f64,
    pub(crate) recommended_orifice: String,
    pub(crate) selected_area_in2: f64,
    pub(crate) back_pressure_fraction: f64,
    pub(crate) back_pressure_correction: f64,
    pub(crate) combination_correction: f64,
}

#[allow(clippy::too_many_arguments)] // one parameter per quantity the class method reads
pub(crate) fn gas_area(
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
) -> GasArea {
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
    GasArea {
        required_area: area_in2 * 6.4516e-4,
        required_area_in2: area_in2,
        recommended_orifice,
        selected_area_in2,
        back_pressure_fraction,
        back_pressure_correction: kb,
        combination_correction: kc,
    }
}

/// The liquid area and its corrections.
pub(crate) struct LiquidArea {
    pub(crate) required_area: f64,
    pub(crate) required_area_in2: f64,
    pub(crate) recommended_orifice: String,
    pub(crate) selected_area_in2: f64,
    pub(crate) back_pressure_fraction: f64,
    pub(crate) back_pressure_correction: f64,
    pub(crate) viscosity_correction: f64,
    pub(crate) mass_flow: f64,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn liquid_area(
    volume_flow: f64,
    liquid_density: f64,
    set_pressure: f64,
    overpressure_fraction: f64,
    back_pressure: f64,
    viscosity: f64,
    balanced_bellows: bool,
) -> LiquidArea {
    let relieving_pressure = set_pressure * (1.0 + overpressure_fraction);
    let specific_gravity = liquid_density / 999.0;
    let flow_gpm = volume_flow * 15850.3;
    let p1_psig = (relieving_pressure - 101_325.0) / 6894.76;
    let p2_psig = (back_pressure - 101_325.0) / 6894.76;
    let kd = 0.65;
    let kw = if balanced_bellows && back_pressure / relieving_pressure > 0.5 {
        (1.0 - 0.5 * (back_pressure / relieving_pressure - 0.5)).max(0.1)
    } else {
        1.0
    };
    let mut dp = p1_psig - p2_psig;
    if dp <= 0.0 {
        dp = 1.0;
    }
    let initial_area_in2 = flow_gpm / (38.0 * kd * kw) * (specific_gravity / dp).sqrt();
    let mut kv = 1.0;
    if viscosity > 0.0 && initial_area_in2 > 0.0 {
        let area_si = initial_area_in2 * 6.4516e-4;
        let equivalent_diameter = (4.0 * area_si / std::f64::consts::PI).sqrt();
        let velocity = volume_flow / area_si;
        let reynolds = liquid_density * velocity * equivalent_diameter / viscosity;
        if reynolds > 0.0 && reynolds < 100_000.0 {
            kv = 1.0 / (0.9935 + 2.878 / reynolds.sqrt() + 342.75 / reynolds.powf(1.5));
            kv = kv.clamp(0.2, 1.0);
        }
    }
    let kp = if overpressure_fraction < 0.25 {
        (0.6 + 4.0 * overpressure_fraction).min(1.0)
    } else {
        1.0
    };
    let area_in2 = flow_gpm / (38.0 * kd * kw * kv * kp) * (specific_gravity / dp).sqrt();
    let (recommended_orifice, selected_area_in2) = select_orifice(area_in2);
    LiquidArea {
        required_area: area_in2 * 6.4516e-4,
        required_area_in2: area_in2,
        recommended_orifice,
        selected_area_in2,
        back_pressure_fraction: back_pressure / relieving_pressure,
        back_pressure_correction: kw,
        viscosity_correction: kv,
        mass_flow: volume_flow * liquid_density,
    }
}

#[allow(clippy::too_many_arguments)] // one parameter per quantity the class method reads
pub(crate) fn two_phase_area(
    mass_flow: f64,
    set_pressure: f64,
    overpressure_fraction: f64,
    back_pressure: f64,
    temperature: f64,
    gas_fraction: f64,
    gas_density: f64,
    liquid_density: f64,
    latent_heat: f64,
    liquid_heat_capacity: f64,
) -> f64 {
    let p0 = set_pressure * (1.0 + overpressure_fraction);
    let v_g = 1.0 / gas_density;
    let v_l = 1.0 / liquid_density;
    let mix = gas_fraction * v_g + (1.0 - gas_fraction) * v_l;
    let omega = gas_fraction * v_g / mix
        + liquid_heat_capacity * temperature * p0 * (v_g - v_l).powi(2)
            / (latent_heat.powi(2) * mix);
    let eta_c = if omega <= 1.0 {
        0.55 + 0.217 * omega.ln()
    } else {
        0.85 / (1.0 + omega)
    };
    let eta_c = eta_c.max(back_pressure / p0);
    let g2 = p0 / mix * (2.0 * (omega * (1.0 / eta_c).ln() + (1.0 - omega) * (1.0 - eta_c)));
    let mass_flux = g2.max(0.0).sqrt();
    if mass_flux <= 0.0 {
        0.0
    } else {
        mass_flow / (0.85 * mass_flux)
    }
}

/// The capacity helper, which is not the sizing inverse.
#[allow(clippy::too_many_arguments)] // one parameter per quantity the class method reads
fn capacity(
    area: f64,
    set_pressure: f64,
    overpressure_fraction: f64,
    back_pressure: f64,
    temperature: f64,
    molar_mass: f64,
    compressibility: f64,
    specific_heat_ratio: f64,
    discharge_coefficient: f64,
) -> f64 {
    let k = specific_heat_ratio;
    let relieving_pressure = set_pressure * (1.0 + overpressure_fraction);
    let critical_ratio = (2.0 / (k + 1.0)).powf(k / (k - 1.0));
    let pressure_ratio = back_pressure / relieving_pressure;
    let c = if pressure_ratio <= critical_ratio {
        (k * (2.0 / (k + 1.0)).powf((k + 1.0) / (k - 1.0))).sqrt()
    } else {
        (2.0 * k / (k - 1.0)
            * (pressure_ratio.powf(2.0 / k) - pressure_ratio.powf((k + 1.0) / k)).max(0.0))
        .sqrt()
    };
    discharge_coefficient
        * area
        * relieving_pressure
        * c
        * (molar_mass / (compressibility * 8314.0 * temperature)).sqrt()
}

/// Result of `process.psv_sizing`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PsvSizingResult {
    /// `set_pressure (1 + overpressure_fraction)`.
    #[serde(serialize_with = "scalar")]
    pub relieving_pressure: Pressure,
    /// `back_pressure / relieving_pressure`.
    pub back_pressure_fraction: f64,
    /// The class's `Kd`.
    pub discharge_coefficient: f64,
    /// The required orifice area.
    #[serde(serialize_with = "scalar")]
    pub required_area: Area,
    /// The required area in in², gas and liquid only.
    pub required_area_in2: Option<f64>,
    /// The selected ladder letter, gas and liquid only.
    pub recommended_orifice: Option<String>,
    /// The selected ladder area in in², gas and liquid only.
    pub selected_area_in2: Option<f64>,
    /// The gas `Kb` or the liquid `Kw`.
    pub back_pressure_correction: Option<f64>,
    /// The gas `Kc`.
    pub combination_correction: Option<f64>,
    /// The liquid `Kv`.
    pub viscosity_correction: Option<f64>,
    /// The liquid mass flow.
    #[serde(serialize_with = "optional_scalar")]
    pub mass_flow_rate: Option<MassRate>,
    /// The capacity helper's result.
    #[serde(serialize_with = "optional_scalar")]
    pub mass_flow_capacity: Option<MassRate>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn required(value: Option<f64>, name: &str, service: Service) -> Result<f64> {
    value.ok_or_else(|| {
        AzothError::invalid_input(name, format!("required for the {} service", service.name()))
    })
}

fn refuse(value: Option<f64>, name: &str, service: Service) -> Result<()> {
    if value.is_some() {
        return Err(AzothError::invalid_input(
            name,
            format!("is not read by the {} service", service.name()),
        ));
    }
    Ok(())
}

/// The gas, liquid or two-phase required area, the ladder and the capacity helper.
///
/// # Errors
/// `InvalidInput` for a missing branch input, an input the branch does not read, a
/// back pressure at or above the relieving pressure, or a value its range refuses.
#[allow(clippy::too_many_arguments)] // one parameter per declared input, and there are nineteen
pub fn psv_sizing(
    service: &str,
    set_pressure: Pressure,
    overpressure_fraction: f64,
    back_pressure: Pressure,
    is_balanced_bellows: Option<bool>,
    has_rupture_disk: Option<bool>,
    mass_flow: Option<MassRate>,
    temperature: Option<ThermodynamicTemperature>,
    molar_mass: Option<MolarMass>,
    compressibility: Option<f64>,
    specific_heat_ratio: Option<f64>,
    volume_flow: Option<VolumeRate>,
    liquid_density: Option<MassDensity>,
    viscosity: Option<DynamicViscosity>,
    gas_fraction: Option<f64>,
    gas_density: Option<MassDensity>,
    latent_heat: Option<f64>,
    liquid_heat_capacity: Option<SpecificHeatCapacity>,
    orifice_area: Option<Area>,
    capacity_discharge_coefficient: Option<f64>,
) -> Result<PsvSizingResult> {
    let spec = &model_gen::PSV_SIZING_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "set_pressure" => Some(set_pressure.value),
            "overpressure_fraction" => Some(overpressure_fraction),
            "back_pressure" => Some(back_pressure.value),
            "mass_flow" => mass_flow.map(|value| value.value),
            "temperature" => temperature.map(|value| value.value),
            "molar_mass" => molar_mass.map(|value| value.value),
            "compressibility" => compressibility,
            "specific_heat_ratio" => specific_heat_ratio,
            "volume_flow" => volume_flow.map(|value| value.value),
            "liquid_density" => liquid_density.map(|value| value.value),
            "viscosity" => viscosity.map(|value| value.value),
            "gas_fraction" => gas_fraction,
            "gas_density" => gas_density.map(|value| value.value),
            "latent_heat" => latent_heat,
            "liquid_heat_capacity" => liquid_heat_capacity.map(|value| value.value),
            "orifice_area" => orifice_area.map(|value| value.value),
            "capacity_discharge_coefficient" => capacity_discharge_coefficient,
            _ => None,
        },
        &mut warnings,
    )?;

    let branch = Service::parse(service)?;
    let relieving_pressure = set_pressure.value * (1.0 + overpressure_fraction);
    for (name, value) in [
        ("set_pressure", set_pressure.value),
        ("overpressure_fraction", overpressure_fraction),
        ("back_pressure", back_pressure.value),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(AzothError::invalid_input(
                name,
                format!("a finite non-negative value is required, and {value} is not"),
            ));
        }
    }
    if !back_pressure.value.is_finite() || back_pressure.value >= relieving_pressure {
        return Err(AzothError::invalid_input(
            "back_pressure",
            format!(
                "a back pressure below the relieving pressure {relieving_pressure} Pa is required, and {} Pa is not",
                back_pressure.value
            ),
        ));
    }

    let balanced_bellows = is_balanced_bellows.unwrap_or(false);
    let rupture_disk = has_rupture_disk.unwrap_or(false);
    let orifice_area = orifice_area.map(|value| value.value);
    let capacity_cd = capacity_discharge_coefficient;

    match branch {
        Service::Gas => {
            refuse(volume_flow.map(|value| value.value), "volume_flow", branch)?;
            refuse(viscosity.map(|value| value.value), "viscosity", branch)?;
            refuse(gas_fraction, "gas_fraction", branch)?;
            refuse(gas_density.map(|value| value.value), "gas_density", branch)?;
            refuse(latent_heat, "latent_heat", branch)?;
            refuse(
                liquid_heat_capacity.map(|value| value.value),
                "liquid_heat_capacity",
                branch,
            )?;
            refuse(
                liquid_density.map(|value| value.value),
                "liquid_density",
                branch,
            )?;
            let mass_flow = required(mass_flow.map(|value| value.value), "mass_flow", branch)?;
            let temperature =
                required(temperature.map(|value| value.value), "temperature", branch)?;
            let molar_mass = required(molar_mass.map(|value| value.value), "molar_mass", branch)?;
            let compressibility = required(compressibility, "compressibility", branch)?;
            let specific_heat_ratio = required(specific_heat_ratio, "specific_heat_ratio", branch)?;
            let gas = gas_area(
                mass_flow,
                set_pressure.value,
                overpressure_fraction,
                back_pressure.value,
                temperature,
                molar_mass,
                compressibility,
                specific_heat_ratio,
                balanced_bellows,
                rupture_disk,
            );
            let mass_flow_capacity = match (orifice_area, capacity_cd) {
                (Some(area), Some(coefficient)) => Some(kilograms_per_second(capacity(
                    area,
                    set_pressure.value,
                    overpressure_fraction,
                    back_pressure.value,
                    temperature,
                    molar_mass,
                    compressibility,
                    specific_heat_ratio,
                    coefficient,
                ))),
                (None, None) => None,
                (Some(_), None) => {
                    return Err(AzothError::invalid_input(
                        "capacity_discharge_coefficient",
                        "is required with orifice_area",
                    ));
                }
                (None, Some(_)) => {
                    return Err(AzothError::invalid_input(
                        "orifice_area",
                        "is required with capacity_discharge_coefficient",
                    ));
                }
            };
            Ok(PsvSizingResult {
                relieving_pressure: pascals(relieving_pressure),
                back_pressure_fraction: gas.back_pressure_fraction,
                discharge_coefficient: 0.975,
                required_area: square_meters(gas.required_area),
                required_area_in2: Some(gas.required_area_in2),
                recommended_orifice: Some(gas.recommended_orifice),
                selected_area_in2: Some(gas.selected_area_in2),
                back_pressure_correction: Some(gas.back_pressure_correction),
                combination_correction: Some(gas.combination_correction),
                viscosity_correction: None,
                mass_flow_rate: None,
                mass_flow_capacity,
                warnings,
            })
        }
        Service::Liquid => {
            if rupture_disk {
                return Err(AzothError::invalid_input(
                    "has_rupture_disk",
                    "is not read by the liquid service",
                ));
            }
            refuse(mass_flow.map(|value| value.value), "mass_flow", branch)?;
            refuse(temperature.map(|value| value.value), "temperature", branch)?;
            refuse(molar_mass.map(|value| value.value), "molar_mass", branch)?;
            refuse(compressibility, "compressibility", branch)?;
            refuse(specific_heat_ratio, "specific_heat_ratio", branch)?;
            refuse(gas_fraction, "gas_fraction", branch)?;
            refuse(gas_density.map(|value| value.value), "gas_density", branch)?;
            refuse(latent_heat, "latent_heat", branch)?;
            refuse(
                liquid_heat_capacity.map(|value| value.value),
                "liquid_heat_capacity",
                branch,
            )?;
            refuse(orifice_area, "orifice_area", branch)?;
            refuse(capacity_cd, "capacity_discharge_coefficient", branch)?;
            let volume_flow =
                required(volume_flow.map(|value| value.value), "volume_flow", branch)?;
            let liquid_density = required(
                liquid_density.map(|value| value.value),
                "liquid_density",
                branch,
            )?;
            let viscosity = required(viscosity.map(|value| value.value), "viscosity", branch)?;
            let liquid = liquid_area(
                volume_flow,
                liquid_density,
                set_pressure.value,
                overpressure_fraction,
                back_pressure.value,
                viscosity,
                balanced_bellows,
            );
            Ok(PsvSizingResult {
                relieving_pressure: pascals(relieving_pressure),
                back_pressure_fraction: liquid.back_pressure_fraction,
                discharge_coefficient: 0.65,
                required_area: square_meters(liquid.required_area),
                required_area_in2: Some(liquid.required_area_in2),
                recommended_orifice: Some(liquid.recommended_orifice),
                selected_area_in2: Some(liquid.selected_area_in2),
                back_pressure_correction: Some(liquid.back_pressure_correction),
                combination_correction: None,
                viscosity_correction: Some(liquid.viscosity_correction),
                mass_flow_rate: Some(kilograms_per_second(liquid.mass_flow)),
                mass_flow_capacity: None,
                warnings,
            })
        }
        Service::TwoPhase => {
            if balanced_bellows {
                return Err(AzothError::invalid_input(
                    "is_balanced_bellows",
                    "is not read by the two_phase service",
                ));
            }
            if rupture_disk {
                return Err(AzothError::invalid_input(
                    "has_rupture_disk",
                    "is not read by the two_phase service",
                ));
            }
            refuse(volume_flow.map(|value| value.value), "volume_flow", branch)?;
            refuse(viscosity.map(|value| value.value), "viscosity", branch)?;
            refuse(molar_mass.map(|value| value.value), "molar_mass", branch)?;
            refuse(compressibility, "compressibility", branch)?;
            refuse(specific_heat_ratio, "specific_heat_ratio", branch)?;
            refuse(orifice_area, "orifice_area", branch)?;
            refuse(capacity_cd, "capacity_discharge_coefficient", branch)?;
            let mass_flow = required(mass_flow.map(|value| value.value), "mass_flow", branch)?;
            let temperature =
                required(temperature.map(|value| value.value), "temperature", branch)?;
            let liquid_density = required(
                liquid_density.map(|value| value.value),
                "liquid_density",
                branch,
            )?;
            let gas_fraction = required(gas_fraction, "gas_fraction", branch)?;
            let gas_density =
                required(gas_density.map(|value| value.value), "gas_density", branch)?;
            let latent_heat = required(latent_heat, "latent_heat", branch)?;
            let liquid_heat_capacity = required(
                liquid_heat_capacity.map(|value| value.value),
                "liquid_heat_capacity",
                branch,
            )?;
            let area = two_phase_area(
                mass_flow,
                set_pressure.value,
                overpressure_fraction,
                back_pressure.value,
                temperature,
                gas_fraction,
                gas_density,
                liquid_density,
                latent_heat,
                liquid_heat_capacity,
            );
            Ok(PsvSizingResult {
                relieving_pressure: pascals(relieving_pressure),
                back_pressure_fraction: back_pressure.value / relieving_pressure,
                discharge_coefficient: 0.85,
                required_area: square_meters(area),
                required_area_in2: None,
                recommended_orifice: None,
                selected_area_in2: None,
                back_pressure_correction: None,
                combination_correction: None,
                viscosity_correction: None,
                mass_flow_rate: None,
                mass_flow_capacity: None,
                warnings,
            })
        }
    }
}
