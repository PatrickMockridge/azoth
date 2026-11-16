//! `process.pipe_fire_rupture_material` - the material curves as a registered id.
//!
//! Spec: `specs/models/process/pipe_fire_rupture_material.toml`. The arithmetic is
//! `process/safety/rupture/PipeFireRuptureMaterial.java` at the pin: the six workbook
//! tables, their clamped interpolation, the Sellars-Tegart strain rate and the class's
//! temperature-correction factor.

use azoth_core::units::{
    Frequency, MassDensity, Pressure, SpecificHeatCapacity, ThermalConductivity,
    ThermodynamicTemperature, joules_per_kilogram_kelvin, kilograms_per_cubic_meter, megapascals,
    per_second, watts_per_meter_kelvin,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's absolute-temperature offset.
const KELVIN_OFFSET: f64 = 273.15;
/// The class's own absolute-temperature offset, applied after the conversion.
const CLASS_OFFSET: f64 = 273.0;
/// The class's per-minute conversion for the strain rate.
const SECONDS_PER_MINUTE: f64 = 60.0;
/// The workbook temperature grid, degC.
const GRID_C: [f64; 15] = [
    0.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 750.0, 800.0, 900.0, 1000.0, 1100.0,
    1150.0, 1350.0,
];
/// The SS316 and 6Mo workbook heat capacity, J/(kg K), on `GRID_C`.
const STAINLESS_CP: [f64; 15] = [
    479.816_560_509_554_17,
    495.064_968_152_866_24,
    511.329_936_305_732_5,
    520.478_980_891_719_7,
    528.611_464_968_152_9,
    538.777_070_063_694_3,
    549.959_235_668_789_8,
    560.124_840_764_331_2,
    564.0,
    568.257_324_840_764_3,
    574.356_687_898_089_1,
    574.356_687_898_089_1,
    574.356_687_898_089_1,
    574.356_687_898_089_1,
    574.0,
];
/// The SS316 and 6Mo workbook thermal conductivity, W/(m K), on `GRID_C`.
const STAINLESS_K: [f64; 15] = [
    13.5, 14.9, 16.7, 18.3, 19.8, 21.3, 22.7, 24.2, 24.8, 25.6, 27.1, 28.6, 30.5, 34.2, 34.2,
];

/// One temperature-dependent material table.
struct MaterialTable {
    /// The class's display name.
    name: &'static str,
    /// Heat capacity, J/(kg K).
    heat_capacity: &'static [f64],
    /// Thermal conductivity, W/(m K).
    thermal_conductivity: &'static [f64],
    /// Ultimate tensile strength, MPa.
    ultimate_tensile_strength: &'static [f64],
    /// Strain-rate multiplier.
    strain_effect: &'static [f64],
    /// Accumulated-strain limit.
    rupture_strain_limit: &'static [f64],
    /// Steel density, kg/m3.
    density_kg_m3: f64,
    /// Sellars-Tegart pre-exponential constant.
    a: f64,
    /// Sellars-Tegart alpha, 1/MPa.
    alpha: f64,
    /// Sellars-Tegart exponent.
    n: f64,
    /// Activation term Q/R, K.
    activation_temperature_k: f64,
    /// The correction rate, per minute.
    correction_rate_per_minute: f64,
    /// The correction limit, degC.
    correction_limit_c: f64,
}

const DUPLEX_22CR: MaterialTable = MaterialTable {
    name: "22Cr duplex",
    heat_capacity: &[
        480.0, 500.0, 530.0, 550.0, 590.0, 635.0, 670.0, 710.0, 730.0, 750.0, 790.0, 840.0, 870.0,
        879.9, 900.0,
    ],
    thermal_conductivity: &[
        15.0, 16.0, 17.0, 18.0, 20.0, 24.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0,
    ],
    ultimate_tensile_strength: &[
        650.0, 605.0, 553.0, 540.0, 533.0, 462.0, 371.0, 247.0, 202.0, 157.0, 78.0, 28.0, 12.0,
        4.0, 0.0,
    ],
    strain_effect: &[
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.7, 1.0, 1.4, 2.3, 2.3, 2.3,
    ],
    rupture_strain_limit: &[
        0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.23, 0.26, 0.33, 0.5, 0.5, 0.5,
    ],
    density_kg_m3: 7850.0,
    a: 12_311_757_039.742_393,
    alpha: 0.03,
    n: 2.156_323_825_544_742,
    activation_temperature_k: 34_115.772_963_646_705,
    correction_rate_per_minute: 0.03,
    correction_limit_c: 900.0,
};

const SS_316: MaterialTable = MaterialTable {
    name: "SS 316",
    heat_capacity: &STAINLESS_CP,
    thermal_conductivity: &STAINLESS_K,
    ultimate_tensile_strength: &[
        485.0, 467.0, 429.0, 426.0, 421.0, 398.0, 363.0, 277.0, 208.5, 140.0, 80.0, 45.0, 20.0,
        10.0, 0.0,
    ],
    strain_effect: &[
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.7, 0.5, 0.3, 0.25, 0.3, 0.3, 0.3,
    ],
    rupture_strain_limit: &[
        0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.16, 0.17, 0.18, 0.2, 0.23, 0.25, 0.3,
    ],
    density_kg_m3: 7850.0,
    a: 23_449_524_917_238.52,
    alpha: 0.015,
    n: 3.904_587_450_236_608_2,
    activation_temperature_k: 42_315.658_241_317_32,
    correction_rate_per_minute: 0.05,
    correction_limit_c: 900.0,
};

const CS_235: MaterialTable = MaterialTable {
    name: "CS 235 ASME A106-B",
    heat_capacity: &[
        450.0, 480.0, 510.0, 550.0, 600.0, 660.0, 750.0, 900.0, 1450.0, 820.0, 540.0, 540.0, 540.0,
        540.0, 540.0,
    ],
    thermal_conductivity: &[
        54.2, 50.95, 47.45, 43.7, 40.45, 37.2, 33.95, 30.7, 28.0, 27.4, 27.4, 27.4, 27.4, 27.4,
        27.4,
    ],
    ultimate_tensile_strength: &[
        420.0, 407.0, 397.0, 382.0, 370.0, 308.0, 189.0, 92.0, 81.5, 71.0, 53.0, 29.0, 17.0, 4.0,
        0.0,
    ],
    strain_effect: &[
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.7, 0.5, 0.5, 0.4, 0.4, 0.3, 0.3,
    ],
    rupture_strain_limit: &[
        0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.2, 0.2, 0.2, 0.2, 0.3,
    ],
    density_kg_m3: 7850.0,
    a: 4_425_186_183_349.352,
    alpha: 0.03,
    n: 3.722_240_193_350_131_2,
    activation_temperature_k: 41_462.451_924_325_49,
    correction_rate_per_minute: 0.05,
    correction_limit_c: 900.0,
};

const CS_360_X52: MaterialTable = MaterialTable {
    name: "CS 360 API 5L-X52",
    heat_capacity: &[
        450.0, 480.0, 510.0, 550.0, 600.0, 660.0, 750.0, 900.0, 1450.0, 820.0, 540.0, 540.0, 540.0,
        540.0, 540.0,
    ],
    thermal_conductivity: &[
        54.2, 50.95, 47.45, 43.7, 40.45, 37.2, 33.95, 30.7, 28.0, 27.4, 27.4, 27.4, 27.4, 27.4,
        27.4,
    ],
    ultimate_tensile_strength: &[
        545.0, 529.0, 515.0, 496.0, 480.0, 400.0, 245.0, 120.0, 99.0, 78.0, 60.0, 38.0, 22.0, 5.0,
        0.0,
    ],
    strain_effect: &[
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.4, 0.6, 0.8, 0.9, 1.0, 1.0,
    ],
    rupture_strain_limit: &[
        0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.2, 0.2, 0.2, 0.2, 0.3,
    ],
    density_kg_m3: 7850.0,
    a: 7_508_371_948_335.949,
    alpha: 0.015,
    n: 4.346_909_864_208_215,
    activation_temperature_k: 38_301.221_625_835_93,
    correction_rate_per_minute: 0.05,
    correction_limit_c: 900.0,
};

const SUPERDUPLEX: MaterialTable = MaterialTable {
    name: "Superduplex",
    heat_capacity: &[
        480.0, 500.0, 530.0, 550.0, 590.0, 635.0, 670.0, 710.0, 730.0, 750.0, 790.0, 840.0, 870.0,
        879.9, 900.0,
    ],
    thermal_conductivity: &[
        15.0, 16.0, 17.0, 18.0, 20.0, 24.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0,
    ],
    ultimate_tensile_strength: &[
        750.0, 698.0, 638.0, 638.0, 613.0, 531.0, 427.0, 284.0, 234.5, 185.0, 100.0, 31.0, 13.0,
        5.0, 0.0,
    ],
    strain_effect: &[
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.7, 1.0, 1.2, 2.2, 2.3, 2.2,
    ],
    rupture_strain_limit: &[
        0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.23, 0.26, 0.33, 0.5, 0.5, 0.5,
    ],
    density_kg_m3: 7850.0,
    a: 352_941_684_892.519_5,
    alpha: 0.03,
    n: 2.148_699_199_752_026_6,
    activation_temperature_k: 39_301.049_471_841_41,
    correction_rate_per_minute: 0.03,
    correction_limit_c: 900.0,
};

const MO_6: MaterialTable = MaterialTable {
    name: "6Mo",
    heat_capacity: &STAINLESS_CP,
    thermal_conductivity: &STAINLESS_K,
    ultimate_tensile_strength: &[
        650.0, 646.0, 589.0, 557.0, 546.0, 528.0, 480.0, 380.0, 315.0, 250.0, 135.0, 90.0, 40.0,
        20.0, 0.0,
    ],
    strain_effect: &[
        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.7, 0.5, 0.3, 0.3, 0.3, 0.25, 0.3,
    ],
    rupture_strain_limit: &[
        0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.2, 0.2, 0.2, 0.2, 0.3,
    ],
    density_kg_m3: 7850.0,
    a: 2_443_499_691_962.783_7,
    alpha: 0.015,
    n: 3.080_826_295_438_379_6,
    activation_temperature_k: 41_637.042_647_031_3,
    correction_rate_per_minute: 0.05,
    correction_limit_c: 900.0,
};

/// One point of a caller-supplied material curve.
#[derive(Debug, Clone, PartialEq)]
pub struct PipeFireRuptureMaterialPoint {
    /// The point's metal temperature.
    pub temperature: ThermodynamicTemperature,
    /// The point's heat capacity.
    pub heat_capacity: SpecificHeatCapacity,
    /// The point's thermal conductivity.
    pub thermal_conductivity: ThermalConductivity,
    /// The point's ultimate tensile strength.
    pub ultimate_tensile_strength: Pressure,
    /// The point's strain-rate multiplier.
    pub strain_effect_factor: f64,
    /// The point's accumulated-strain limit.
    pub rupture_strain_limit: f64,
}

/// Result of `process.pipe_fire_rupture_material`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PipeFireRuptureMaterialResult {
    /// The built-in table's name or the supplied label.
    pub resolved_material_name: String,
    /// The material density.
    #[serde(serialize_with = "scalar")]
    pub density: MassDensity,
    /// The heat capacity at the evaluation temperature.
    #[serde(serialize_with = "scalar")]
    pub heat_capacity: SpecificHeatCapacity,
    /// The thermal conductivity at the evaluation temperature.
    #[serde(serialize_with = "scalar")]
    pub thermal_conductivity: ThermalConductivity,
    /// The ultimate tensile strength at the evaluation temperature.
    #[serde(serialize_with = "scalar")]
    pub ultimate_tensile_strength: Pressure,
    /// The strain-rate multiplier at the evaluation temperature.
    pub strain_effect_factor: f64,
    /// The accumulated-strain limit at the evaluation temperature.
    pub rupture_strain_limit: f64,
    /// The class's temperature-correction factor.
    pub temperature_correction_factor: f64,
    /// The Sellars-Tegart strain rate.
    #[serde(serialize_with = "scalar")]
    pub strain_rate: Frequency,
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

/// Refuse a non-positive constant.
fn require_positive(name: &'static str, value: f64) -> Result<()> {
    require_finite(name, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name,
            "the class's constructor refuses a non-positive value",
        ))
    }
}

/// The class's endpoint-clamped linear interpolation.
fn interpolate(value: f64, xs: &[f64], ys: &[f64]) -> f64 {
    if value <= xs[0] {
        return ys[0];
    }
    for index in 1..xs.len() {
        if value <= xs[index] {
            let fraction = (value - xs[index - 1]) / (xs[index] - xs[index - 1]);
            return ys[index - 1] + fraction * (ys[index] - ys[index - 1]);
        }
    }
    ys[ys.len() - 1]
}

/// The class's lowercased `contains` selector, in its order.
fn built_in(name: &str) -> Result<&'static MaterialTable> {
    let normalized = name.to_lowercase();
    if normalized.contains("superduplex") || normalized.contains("25cr") {
        Ok(&SUPERDUPLEX)
    } else if normalized.contains("22cr") || normalized.contains("duplex") {
        Ok(&DUPLEX_22CR)
    } else if normalized.contains("316") {
        Ok(&SS_316)
    } else if normalized.contains("360") || normalized.contains("x52") || normalized.contains("5l")
    {
        Ok(&CS_360_X52)
    } else if normalized.contains("235") || normalized.contains("a106") {
        Ok(&CS_235)
    } else if normalized.contains("6mo") || normalized.contains("6 mo") {
        Ok(&MO_6)
    } else {
        Err(AzothError::invalid_input(
            "material_name",
            format!("`{name}` matches no built-in pipe fire-rupture material"),
        ))
    }
}

/// The material's properties and Sellars-Tegart strain rate at one state.
///
/// # Errors
/// `InvalidInput` for an unknown name, a curve and constants that are not all-or-none, a
/// grid under two points or over the spec's row guard, a non-increasing or non-finite grid,
/// a non-positive density or constitutive constant, a non-positive temperature, an unknown
/// interpolation, or a non-finite value.
#[allow(clippy::too_many_arguments)]
pub fn pipe_fire_rupture_material(
    material_name: &str,
    evaluation_temperature: ThermodynamicTemperature,
    evaluation_stress: Pressure,
    curve: Option<&[PipeFireRuptureMaterialPoint]>,
    density: Option<MassDensity>,
    sellars_tegart_a: Option<f64>,
    sellars_tegart_alpha: Option<f64>,
    sellars_tegart_n: Option<f64>,
    activation_temperature: Option<ThermodynamicTemperature>,
    correction_strain_rate: Option<Frequency>,
    correction_limit_temperature: Option<ThermodynamicTemperature>,
) -> Result<PipeFireRuptureMaterialResult> {
    let spec = &model_gen::PIPE_FIRE_RUPTURE_MATERIAL_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "evaluation_temperature" => Some(evaluation_temperature.value),
            "evaluation_stress" => Some(evaluation_stress.value),
            "density" => density.map(|value| value.value),
            "sellars_tegart_a" => sellars_tegart_a,
            "sellars_tegart_alpha" => sellars_tegart_alpha,
            "sellars_tegart_n" => sellars_tegart_n,
            "activation_temperature" => activation_temperature.map(|value| value.value),
            "correction_strain_rate" => correction_strain_rate.map(|value| value.value),
            "correction_limit_temperature" => correction_limit_temperature.map(|value| value.value),
            _ => None,
        },
        &mut warnings,
    )?;

    let label = material_name.trim();
    if label.is_empty() {
        return Err(AzothError::invalid_input(
            "material_name",
            "the class's constructor refuses an empty material name",
        ));
    }
    require_finite("evaluation_temperature", evaluation_temperature.value)?;
    if evaluation_temperature.value <= 0.0 {
        return Err(AzothError::invalid_input(
            "evaluation_temperature",
            "a non-positive absolute temperature is not a temperature",
        ));
    }
    require_finite("evaluation_stress", evaluation_stress.value)?;

    let groups = [
        density.is_some(),
        sellars_tegart_a.is_some(),
        sellars_tegart_alpha.is_some(),
        sellars_tegart_n.is_some(),
        activation_temperature.is_some(),
        correction_strain_rate.is_some(),
        correction_limit_temperature.is_some(),
    ];
    let supplied = groups.iter().filter(|value| **value).count();
    if curve.is_none() && supplied != 0 {
        return Err(AzothError::invalid_input(
            "curve",
            "the six constants are carried with a curve; a built-in name carries neither",
        ));
    }
    if curve.is_some() && supplied != groups.len() {
        return Err(AzothError::invalid_input(
            "curve",
            "a caller-supplied curve requires all six constants",
        ));
    }

    let evaluation_c = evaluation_temperature.value - KELVIN_OFFSET;
    let stress_mpa = evaluation_stress.value / 1.0e6;
    require_finite("evaluation_temperature", evaluation_c)?;

    let (
        resolved_name,
        density_kg_m3,
        grid,
        heat_capacity,
        thermal_conductivity,
        ultimate_tensile_strength,
        strain_effect,
        rupture_strain_limit,
        a,
        alpha,
        n,
        activation_k,
        correction_rate_per_minute,
        correction_limit_c,
    ) = if let Some(points) = curve {
        let capacity = spec
            .algorithm
            .map_or(0, |algorithm| algorithm.max_iterations) as usize;
        if points.len() < 2 {
            return Err(AzothError::invalid_input(
                "curve",
                "a material curve needs at least two points",
            ));
        }
        if points.len() > capacity {
            return Err(AzothError::invalid_input(
                "curve",
                format!(
                    "{} points exceeds the {capacity} this procedure carries",
                    points.len()
                ),
            ));
        }
        let mut grid = Vec::with_capacity(points.len());
        let mut cp = Vec::with_capacity(points.len());
        let mut k = Vec::with_capacity(points.len());
        let mut uts = Vec::with_capacity(points.len());
        let mut effect = Vec::with_capacity(points.len());
        let mut limit = Vec::with_capacity(points.len());
        let mut previous = f64::NEG_INFINITY;
        for point in points {
            require_finite("curve.temperature", point.temperature.value)?;
            if point.temperature.value <= 0.0 {
                return Err(AzothError::invalid_input(
                    "curve.temperature",
                    "a non-positive absolute temperature is not a temperature",
                ));
            }
            let temperature_c = point.temperature.value - KELVIN_OFFSET;
            if temperature_c <= previous {
                return Err(AzothError::invalid_input(
                    "curve.temperature",
                    "the class requires strictly increasing temperatures",
                ));
            }
            previous = temperature_c;
            require_finite("curve.heat_capacity", point.heat_capacity.value)?;
            require_finite(
                "curve.thermal_conductivity",
                point.thermal_conductivity.value,
            )?;
            require_finite(
                "curve.ultimate_tensile_strength",
                point.ultimate_tensile_strength.value,
            )?;
            require_finite("curve.strain_effect_factor", point.strain_effect_factor)?;
            require_finite("curve.rupture_strain_limit", point.rupture_strain_limit)?;
            grid.push(temperature_c);
            cp.push(point.heat_capacity.value);
            k.push(point.thermal_conductivity.value);
            uts.push(point.ultimate_tensile_strength.value / 1.0e6);
            effect.push(point.strain_effect_factor);
            limit.push(point.rupture_strain_limit);
        }
        let density = density.expect("the all-or-none group was checked");
        let a = sellars_tegart_a.expect("the all-or-none group was checked");
        let alpha = sellars_tegart_alpha.expect("the all-or-none group was checked");
        let n = sellars_tegart_n.expect("the all-or-none group was checked");
        let activation = activation_temperature.expect("the all-or-none group was checked");
        let correction = correction_strain_rate.expect("the all-or-none group was checked");
        let limit_temperature =
            correction_limit_temperature.expect("the all-or-none group was checked");
        require_positive("density", density.value)?;
        require_positive("sellars_tegart_a", a)?;
        require_positive("sellars_tegart_alpha", alpha)?;
        require_positive("sellars_tegart_n", n)?;
        require_positive("activation_temperature", activation.value)?;
        require_positive("correction_strain_rate", correction.value)?;
        require_finite("correction_limit_temperature", limit_temperature.value)?;
        (
            label.to_string(),
            density.value,
            grid,
            cp,
            k,
            uts,
            effect,
            limit,
            a,
            alpha,
            n,
            activation.value,
            correction.value * SECONDS_PER_MINUTE,
            limit_temperature.value - KELVIN_OFFSET,
        )
    } else {
        let table = built_in(label)?;
        (
            table.name.to_string(),
            table.density_kg_m3,
            GRID_C.to_vec(),
            table.heat_capacity.to_vec(),
            table.thermal_conductivity.to_vec(),
            table.ultimate_tensile_strength.to_vec(),
            table.strain_effect.to_vec(),
            table.rupture_strain_limit.to_vec(),
            table.a,
            table.alpha,
            table.n,
            table.activation_temperature_k,
            table.correction_rate_per_minute,
            table.correction_limit_c,
        )
    };

    let heat_capacity_at = interpolate(evaluation_c, &grid, &heat_capacity);
    let conductivity_at = interpolate(evaluation_c, &grid, &thermal_conductivity);
    let uts_at = interpolate(evaluation_c, &grid, &ultimate_tensile_strength);
    let effect_at = interpolate(evaluation_c, &grid, &strain_effect);
    let limit_at = interpolate(evaluation_c, &grid, &rupture_strain_limit);

    let absolute_temperature = evaluation_c + CLASS_OFFSET;
    let correction = if evaluation_c >= correction_limit_c {
        1.0
    } else {
        let hyperbolic_sine = (alpha * uts_at.max(0.0)).sinh();
        if hyperbolic_sine <= 0.0 {
            1.0
        } else {
            absolute_temperature / activation_k
                * ((hyperbolic_sine.ln() * n) - correction_rate_per_minute.ln() + a.ln())
        }
    };
    let effective_stress = stress_mpa.max(0.0);
    let strain_rate_per_minute = effect_at
        * a
        * (alpha * effective_stress).sinh().powf(n)
        * (-correction * activation_k / absolute_temperature).exp();

    Ok(PipeFireRuptureMaterialResult {
        resolved_material_name: resolved_name,
        density: kilograms_per_cubic_meter(density_kg_m3),
        heat_capacity: joules_per_kilogram_kelvin(heat_capacity_at),
        thermal_conductivity: watts_per_meter_kelvin(conductivity_at),
        ultimate_tensile_strength: megapascals(uts_at),
        strain_effect_factor: effect_at,
        rupture_strain_limit: limit_at,
        temperature_correction_factor: correction,
        strain_rate: per_second(strain_rate_per_minute / SECONDS_PER_MINUTE),
        warnings,
    })
}
