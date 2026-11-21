//! `pvt.black_oil_correlations` - NeqSim's black-oil correlation table.
//!
//! Spec: `specs/models/pvt/black_oil_correlations.toml`. The arithmetic is
//! `pvtsimulation/util/BlackOilCorrelations.java` and
//! `pvtsimulation/util/SaturationPressureCorrelation.java` at the pin. The two classes convert the
//! same quantities with different literals; the `_p_sat` calculations use the second family.

use azoth_core::units::{
    DynamicViscosity, MassDensity, MolarMass, Pressure, ThermodynamicTemperature,
};
use azoth_core::{AzothError, Result, Warning};

use crate::standard::{
    CELSIUS_OFFSET_K, PSIA_PER_BAR, RANKINE_OFFSET, SATURATION_BAR_PER_PSIA,
    SATURATION_SM3_PER_SCF_STB, SM3_PER_SCF_STB, celsius_to_fahrenheit, kg_m3_to_lb_ft3,
    pa_s_to_centipoise,
};

/// The calculations this id dispatches.
pub const CALCULATIONS: &[&str] = &[
    "api_from_specific_gravity",
    "specific_gravity_from_api",
    "bubble_point_standing",
    "bubble_point_standing_by_specific_gravity",
    "bubble_point_vasquez_beggs",
    "bubble_point_glaso_by_specific_gravity",
    "bubble_point_standing_p_sat",
    "bubble_point_vasquez_beggs_p_sat",
    "bubble_point_glaso_p_sat",
    "bubble_point_petrosky_farshad",
    "bubble_point_al_marhoun",
    "bubble_point_statistics",
    "solution_gor_standing",
    "solution_gor_vasquez_beggs",
    "oil_fvf_standing",
    "oil_fvf_vasquez_beggs",
    "oil_fvf_undersaturated",
    "oil_compressibility_vasquez_beggs",
    "dead_oil_viscosity_beggs_robinson",
    "dead_oil_viscosity_glaso",
    "dead_oil_viscosity_kartoatmodjo",
    "saturated_oil_viscosity_beggs_robinson",
    "saturated_oil_viscosity_kartoatmodjo",
    "undersaturated_oil_viscosity_vasquez_beggs",
    "undersaturated_oil_viscosity_bergman_sutton",
    "gas_fvf",
    "gas_fvf_rb_per_mscf",
    "gas_viscosity_lee_gonzalez_eakin",
];

/// One correlation call, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct BlackOilQuery {
    /// The correlation.
    pub calculation: String,
    /// Solution gas-oil ratio, in m3/m3.
    pub rs: Option<f64>,
    /// Gas specific gravity.
    pub gamma_g: Option<f64>,
    /// Oil specific gravity.
    pub gamma_o: Option<f64>,
    /// Oil API gravity.
    pub api: Option<f64>,
    /// Absolute temperature.
    pub temperature: Option<ThermodynamicTemperature>,
    /// Absolute pressure.
    pub pressure: Option<Pressure>,
    /// Oil formation volume factor at the bubble point.
    pub bob: Option<f64>,
    /// Oil compressibility, in 1/Pa.
    pub compressibility: Option<f64>,
    /// Bubble point pressure.
    pub pb: Option<Pressure>,
    /// Dead-oil viscosity.
    pub mu_od: Option<DynamicViscosity>,
    /// Saturated-oil viscosity at the bubble point.
    pub mu_ob: Option<DynamicViscosity>,
    /// Gas density.
    pub rho_g: Option<MassDensity>,
    /// Gas molar mass.
    pub molar_mass: Option<MolarMass>,
    /// Gas compressibility factor.
    pub z: Option<f64>,
}

/// One correlation answer.
#[derive(Debug, Clone, PartialEq)]
pub struct BlackOilResultRow {
    /// The call this row answered.
    pub calculation: String,
    /// The bubble point, in Pa.
    pub pressure: Option<f64>,
    /// The solution gas-oil ratio, in m3/m3.
    pub solution_gor: Option<f64>,
    /// The oil formation volume factor.
    pub oil_fvf: Option<f64>,
    /// The oil compressibility, in 1/Pa.
    pub compressibility: Option<f64>,
    /// The oil or gas viscosity, in Pa*s.
    pub viscosity: Option<f64>,
    /// The gas formation volume factor.
    pub gas_fvf: Option<f64>,
    /// The API gravity, specific gravity or another dimensionless conversion.
    pub factor: Option<f64>,
    /// The arithmetic mean of the four saturation-pressure correlations, in Pa.
    pub average: Option<f64>,
    /// The smallest saturation-pressure answer, in Pa.
    pub minimum: Option<f64>,
    /// The largest saturation-pressure answer, in Pa.
    pub maximum: Option<f64>,
    /// The population standard deviation, in Pa.
    pub std_dev: Option<f64>,
    /// Standing's answer, in Pa.
    pub standing: Option<f64>,
    /// Vasquez-Beggs's answer, in Pa.
    pub vasquez_beggs: Option<f64>,
    /// Glaso's answer, in Pa.
    pub glaso: Option<f64>,
    /// Petrosky-Farshad's answer, in Pa.
    pub petrosky_farshad: Option<f64>,
}

/// Result of `pvt.black_oil_correlations`.
#[derive(Debug, Clone, PartialEq)]
pub struct BlackOilCorrelationsResult {
    /// One row per input row.
    pub queries: Vec<BlackOilResultRow>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The correlation table over the stated calls.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown calculation, a missing required field, a
///   non-positive value where the correlation divides by it or takes its logarithm, or a
///   non-finite answer.
pub fn black_oil_correlations(queries: &[BlackOilQuery]) -> Result<BlackOilCorrelationsResult> {
    let mut rows = Vec::with_capacity(queries.len());
    for (index, query) in queries.iter().enumerate() {
        rows.push(one(index, query)?);
    }
    Ok(BlackOilCorrelationsResult {
        queries: rows,
        warnings: Vec::new(),
    })
}

fn one(index: usize, query: &BlackOilQuery) -> Result<BlackOilResultRow> {
    let calculation = query.calculation.as_str();
    let mut row = empty(calculation);
    match calculation {
        "api_from_specific_gravity" => {
            let sg = positive(index, "gamma_o", query.gamma_o)?;
            row.factor = Some(finite(index, "factor", 141.5 / sg - 131.5)?);
        }
        "specific_gravity_from_api" => {
            let api = positive(index, "api", query.api)?;
            row.factor = Some(finite(index, "factor", 141.5 / (api + 131.5))?);
        }
        "bubble_point_standing" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                standing(rs, gamma_g, api, t) * 1.0e5,
            )?);
        }
        "bubble_point_standing_by_specific_gravity" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let gamma_o = positive(index, "gamma_o", query.gamma_o)?;
            let t = temperature(index, query.temperature)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                standing_by_specific_gravity(rs, gamma_g, gamma_o, t) * 1.0e5,
            )?);
        }
        "bubble_point_vasquez_beggs" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                vasquez_beggs(rs, gamma_g, api, t) * 1.0e5,
            )?);
        }
        "bubble_point_glaso_by_specific_gravity" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                glaso_by_specific_gravity(rs, gamma_g, api, t) * 1.0e5,
            )?);
        }
        "bubble_point_standing_p_sat" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                p_sat_pa(p_sat_standing(rs, gamma_g, api, t)),
            )?);
        }
        "bubble_point_vasquez_beggs_p_sat" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                p_sat_pa(p_sat_vasquez_beggs(rs, gamma_g, api, t)),
            )?);
        }
        "bubble_point_glaso_p_sat" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                p_sat_pa(p_sat_glaso(rs, gamma_g, api, t)),
            )?);
        }
        "bubble_point_petrosky_farshad" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                p_sat_pa(p_sat_petrosky_farshad(rs, gamma_g, api, t)),
            )?);
        }
        "bubble_point_al_marhoun" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let gamma_o = positive(index, "gamma_o", query.gamma_o)?;
            let t = temperature(index, query.temperature)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                p_sat_pa(p_sat_al_marhoun(rs, gamma_g, gamma_o, t)),
            )?);
        }
        "bubble_point_statistics" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            let standing = p_sat_standing(rs, gamma_g, api, t);
            let vasquez = p_sat_vasquez_beggs(rs, gamma_g, api, t);
            let glaso = p_sat_glaso(rs, gamma_g, api, t);
            let petrosky = p_sat_petrosky_farshad(rs, gamma_g, api, t);
            let values = [standing, vasquez, glaso, petrosky];
            let mean = values.iter().sum::<f64>() / 4.0;
            let variance = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / 4.0;
            let to_pa = p_sat_pa;
            row.average = Some(finite(index, "average", to_pa(mean))?);
            row.minimum = Some(finite(
                index,
                "minimum",
                to_pa(values[0].min(values[1]).min(values[2]).min(values[3])),
            )?);
            row.maximum = Some(finite(
                index,
                "maximum",
                to_pa(values[0].max(values[1]).max(values[2]).max(values[3])),
            )?);
            row.std_dev = Some(finite(index, "std_dev", to_pa(variance.sqrt()))?);
            row.standing = Some(finite(index, "standing", to_pa(standing))?);
            row.vasquez_beggs = Some(finite(index, "vasquez_beggs", to_pa(vasquez))?);
            row.glaso = Some(finite(index, "glaso", to_pa(glaso))?);
            row.petrosky_farshad = Some(finite(index, "petrosky_farshad", to_pa(petrosky))?);
        }
        "solution_gor_standing" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.solution_gor = Some(finite(
                index,
                "solution_gor",
                solution_gor_standing(p, gamma_g, api, t),
            )?);
        }
        "solution_gor_vasquez_beggs" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.solution_gor = Some(finite(
                index,
                "solution_gor",
                solution_gor_vasquez_beggs(p, gamma_g, api, t),
            )?);
        }
        "oil_fvf_standing" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let gamma_o = positive(index, "gamma_o", query.gamma_o)?;
            let t = temperature(index, query.temperature)?;
            row.oil_fvf = Some(finite(
                index,
                "oil_fvf",
                oil_fvf_standing(rs, gamma_g, gamma_o, t),
            )?);
        }
        "oil_fvf_vasquez_beggs" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.oil_fvf = Some(finite(
                index,
                "oil_fvf",
                oil_fvf_vasquez_beggs(rs, gamma_g, api, t),
            )?);
        }
        "oil_fvf_undersaturated" => {
            let bob = positive(index, "bob", query.bob)?;
            let co = finite_field(index, "compressibility", query.compressibility)?;
            let p = pressure(index, "pressure", query.pressure)?;
            let pb = pressure(index, "pb", query.pb)?;
            row.oil_fvf = Some(finite(
                index,
                "oil_fvf",
                oil_fvf_undersaturated(bob, co, p, pb),
            )?);
        }
        "oil_compressibility_vasquez_beggs" => {
            let rs = positive(index, "rs", query.rs)?;
            let gamma_g = positive(index, "gamma_g", query.gamma_g)?;
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, "pressure", query.pressure)?;
            row.compressibility = Some(finite(
                index,
                "compressibility",
                oil_compressibility(rs, gamma_g, api, t, p) * 1.0e-5,
            )?);
        }
        "dead_oil_viscosity_beggs_robinson" => {
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                dead_oil_viscosity_beggs_robinson(api, t) / 1000.0,
            )?);
        }
        "dead_oil_viscosity_glaso" => {
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                dead_oil_viscosity_glaso(api, t) / 1000.0,
            )?);
        }
        "dead_oil_viscosity_kartoatmodjo" => {
            let api = positive(index, "api", query.api)?;
            let t = temperature(index, query.temperature)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                dead_oil_viscosity_kartoatmodjo(api, t) / 1000.0,
            )?);
        }
        "saturated_oil_viscosity_beggs_robinson" => {
            let mu = viscosity(index, "mu_od", query.mu_od)?;
            let rs = positive(index, "rs", query.rs)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                saturated_oil_viscosity_beggs_robinson(mu, rs),
            )?);
        }
        "saturated_oil_viscosity_kartoatmodjo" => {
            let mu = viscosity(index, "mu_od", query.mu_od)?;
            let rs = positive(index, "rs", query.rs)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                saturated_oil_viscosity_kartoatmodjo(mu, rs),
            )?);
        }
        "undersaturated_oil_viscosity_vasquez_beggs" => {
            let mu = viscosity(index, "mu_ob", query.mu_ob)?;
            let p = pressure(index, "pressure", query.pressure)?;
            let pb = pressure(index, "pb", query.pb)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                undersaturated_oil_viscosity_vasquez_beggs(mu, p, pb),
            )?);
        }
        "undersaturated_oil_viscosity_bergman_sutton" => {
            let mu = viscosity(index, "mu_ob", query.mu_ob)?;
            let p = pressure(index, "pressure", query.pressure)?;
            let pb = pressure(index, "pb", query.pb)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                undersaturated_oil_viscosity_bergman_sutton(mu, p, pb),
            )?);
        }
        "gas_fvf" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let t = temperature(index, query.temperature)?;
            let z = positive(index, "z", query.z)?;
            row.gas_fvf = Some(finite(index, "gas_fvf", gas_fvf(p, t, z))?);
        }
        "gas_fvf_rb_per_mscf" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let t = temperature(index, query.temperature)?;
            let z = positive(index, "z", query.z)?;
            row.gas_fvf = Some(finite(index, "gas_fvf", gas_fvf_rb_per_mscf(p, t, z))?);
        }
        "gas_viscosity_lee_gonzalez_eakin" => {
            let t = temperature(index, query.temperature)?;
            let rho = density(index, "rho_g", query.rho_g)?;
            let molar_mass = molar_mass(index, "molar_mass", query.molar_mass)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                gas_viscosity_lee_gonzalez_eakin(t, rho, molar_mass) / 1000.0,
            )?);
        }
        _ => {
            return Err(AzothError::invalid_input(
                format!("queries[{index}].calculation"),
                format!("{calculation:?} is not one of the black-oil calculations"),
            ));
        }
    }
    Ok(row)
}

fn empty(calculation: &str) -> BlackOilResultRow {
    BlackOilResultRow {
        calculation: calculation.to_owned(),
        pressure: None,
        solution_gor: None,
        oil_fvf: None,
        compressibility: None,
        viscosity: None,
        gas_fvf: None,
        factor: None,
        average: None,
        minimum: None,
        maximum: None,
        std_dev: None,
        standing: None,
        vasquez_beggs: None,
        glaso: None,
        petrosky_farshad: None,
    }
}

fn finite(index: usize, name: &str, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "the correlation answered a non-finite value; the inputs are outside the state it can answer",
        ))
    }
}

fn finite_field(index: usize, name: &str, value: Option<f64>) -> Result<f64> {
    let value = value.ok_or_else(|| missing(index, name))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

fn positive(index: usize, name: &str, value: Option<f64>) -> Result<f64> {
    let value = finite_field(index, name, value)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "the correlation divides by this value or takes its logarithm, and it must be positive",
        ))
    }
}

fn missing(index: usize, name: &str) -> AzothError {
    AzothError::invalid_input(
        format!("queries[{index}].{name}"),
        "the calculation needs this field",
    )
}

fn temperature(index: usize, value: Option<ThermodynamicTemperature>) -> Result<f64> {
    let value = value.ok_or_else(|| missing(index, "temperature"))?;
    let k = value.value;
    if k.is_finite() && k > 0.0 {
        Ok(k)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].temperature"),
            "an absolute temperature; zero and below are not states",
        ))
    }
}

fn pressure(index: usize, name: &str, value: Option<Pressure>) -> Result<f64> {
    let value = value.ok_or_else(|| missing(index, name))?;
    let p = value.value;
    if p.is_finite() && p > 0.0 {
        Ok(p)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "an absolute pressure; zero and below are not states",
        ))
    }
}

fn viscosity(index: usize, name: &str, value: Option<DynamicViscosity>) -> Result<f64> {
    let value = value.ok_or_else(|| missing(index, name))?;
    let mu = value.value;
    if mu.is_finite() && mu > 0.0 {
        Ok(mu)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "a viscosity; zero and below are not states the correlation takes",
        ))
    }
}

fn density(index: usize, name: &str, value: Option<MassDensity>) -> Result<f64> {
    let value = value.ok_or_else(|| missing(index, name))?;
    let rho = value.value;
    if rho.is_finite() && rho > 0.0 {
        Ok(rho)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "a density; zero and below are not states the correlation takes",
        ))
    }
}

fn molar_mass(index: usize, name: &str, value: Option<MolarMass>) -> Result<f64> {
    let value = value.ok_or_else(|| missing(index, name))?;
    let m = value.value;
    if m.is_finite() && m > 0.0 {
        Ok(m)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "a molar mass; zero and below are not states the correlation takes",
        ))
    }
}

fn celsius(t_kelvin: f64) -> f64 {
    t_kelvin - CELSIUS_OFFSET_K
}

fn fahrenheit(t_kelvin: f64) -> f64 {
    celsius_to_fahrenheit(celsius(t_kelvin))
}

fn psia(p_pa: f64) -> f64 {
    (p_pa / 1.0e5) * PSIA_PER_BAR
}

fn p_sat_psia(p_psia: f64) -> f64 {
    p_psia.max(14.7)
}

fn p_sat_pa(p_psia: f64) -> f64 {
    p_psia * SATURATION_BAR_PER_PSIA * 1.0e5
}

fn sm3_to_scf(rs_sm3: f64) -> f64 {
    rs_sm3 / SATURATION_SM3_PER_SCF_STB
}

fn standing(rs: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let rs_field = rs / SM3_PER_SCF_STB;
    let t = fahrenheit(t_k);
    let a = 0.00091 * t - 0.0125 * api;
    let pb = 18.2 * ((rs_field / gamma_g).powf(0.83) * 10f64.powf(a) - 1.4);
    pb / PSIA_PER_BAR
}

fn standing_by_specific_gravity(rs: f64, gamma_g: f64, gamma_o: f64, t_k: f64) -> f64 {
    let rs_field = rs / SM3_PER_SCF_STB;
    let t = fahrenheit(t_k);
    let api = 141.5 / gamma_o - 131.5;
    let a = 0.00091 * t - 0.0125 * api;
    let pb = 18.2 * ((rs_field / gamma_g).powf(0.83) * 10f64.powf(a) - 1.4);
    pb / PSIA_PER_BAR
}

fn vasquez_beggs(rs: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let rs_field = rs / SM3_PER_SCF_STB;
    let t = fahrenheit(t_k);
    let (c1, c2, c3) = if api <= 30.0 {
        (0.0362, 1.0937, 25.724)
    } else {
        (0.0178, 1.187, 23.931)
    };
    let pb = (rs_field / (c1 * gamma_g * (c3 * api / (t + 460.0)).exp())).powf(1.0 / c2);
    pb / PSIA_PER_BAR
}

fn glaso_by_specific_gravity(rs: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let rs_field = rs / SM3_PER_SCF_STB;
    let t = fahrenheit(t_k);
    let gamma_o = 141.5 / (api + 131.5);
    let a = (rs_field / gamma_g).powf(0.816) * t.powf(0.172) / gamma_o.powf(0.989);
    let log_a = a.log10();
    let log_pb = 1.7669 + 1.7447 * log_a - 0.30218 * log_a * log_a;
    let pb = 10f64.powf(log_pb);
    pb / PSIA_PER_BAR
}

fn p_sat_standing(rs: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let rs_field = sm3_to_scf(rs);
    let t = fahrenheit(t_k);
    let x = 0.00091 * t - 0.0125 * api;
    let factor = (rs_field / gamma_g).powf(0.83) * 10f64.powf(x);
    p_sat_psia(18.2 * (factor - 1.4))
}

fn p_sat_vasquez_beggs(rs: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let rs_field = sm3_to_scf(rs);
    let t = fahrenheit(t_k);
    let (c1, c2, c3) = if api <= 30.0 {
        (0.0362, 1.0937, 25.724)
    } else {
        (0.0178, 1.187, 23.931)
    };
    let exponent = -c3 * api / (t + RANKINE_OFFSET);
    p_sat_psia((rs_field / (c1 * gamma_g * exponent.exp())).powf(1.0 / c2))
}

fn p_sat_glaso(rs: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let rs_field = sm3_to_scf(rs);
    let t = fahrenheit(t_k);
    let pb_star = (rs_field / gamma_g).powf(0.816) * t.powf(0.172) / api.powf(0.989);
    let log_pb_star = pb_star.log10();
    let log_pb = 1.7669 + 1.7447 * log_pb_star - 0.30218 * log_pb_star * log_pb_star;
    p_sat_psia(10f64.powf(log_pb))
}

fn p_sat_petrosky_farshad(rs: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let rs_field = sm3_to_scf(rs);
    let t = fahrenheit(t_k);
    let x = 0.0007916 * api.powf(1.5410) - 0.0004561 * t.powf(1.3911);
    let factor = rs_field.powf(0.8439) * 10f64.powf(x);
    p_sat_psia(112.727 * (factor / gamma_g.powf(0.8439) - 12.340))
}

fn p_sat_al_marhoun(rs: f64, gamma_g: f64, gamma_o: f64, t_k: f64) -> f64 {
    let rs_field = sm3_to_scf(rs);
    let t_r = fahrenheit(t_k) + RANKINE_OFFSET;
    let a = 5.38088e-3;
    let b = 0.715082;
    let c = -1.87784;
    let d = 3.1437;
    let e = 1.32657;
    p_sat_psia(a * rs_field.powf(b) * gamma_g.powf(c) * gamma_o.powf(d) * t_r.powf(e))
}

fn solution_gor_standing(p_pa: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let p_field = psia(p_pa);
    let t = fahrenheit(t_k);
    let a = 0.00091 * t - 0.0125 * api;
    let rs = gamma_g * ((p_field / 18.2 + 1.4) * 10f64.powf(-a)).powf(1.2048);
    rs * SM3_PER_SCF_STB
}

fn solution_gor_vasquez_beggs(p_pa: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let p_field = psia(p_pa);
    let t = fahrenheit(t_k);
    let (c1, c2, c3) = if api <= 30.0 {
        (0.0362, 1.0937, 25.724)
    } else {
        (0.0178, 1.187, 23.931)
    };
    let rs = c1 * gamma_g * p_field.powf(c2) * (c3 * api / (t + 460.0)).exp();
    rs * SM3_PER_SCF_STB
}

fn oil_fvf_standing(rs: f64, gamma_g: f64, gamma_o: f64, t_k: f64) -> f64 {
    let rs_field = rs / SM3_PER_SCF_STB;
    let t = fahrenheit(t_k);
    let f = rs_field * (gamma_g / gamma_o).sqrt() + 1.25 * t;
    0.9759 + 0.00012 * f.powf(1.2)
}

fn oil_fvf_vasquez_beggs(rs: f64, gamma_g: f64, api: f64, t_k: f64) -> f64 {
    let rs_field = rs / SM3_PER_SCF_STB;
    let t = fahrenheit(t_k);
    let (c1, c2, c3) = if api <= 30.0 {
        (4.677e-4, 1.751e-5, -1.811e-8)
    } else {
        (4.670e-4, 1.100e-5, 1.337e-9)
    };
    1.0 + c1 * rs_field + (t - 60.0) * (api / gamma_g) * (c2 + c3 * rs_field)
}

fn oil_fvf_undersaturated(bob: f64, co_pa: f64, p_pa: f64, pb_pa: f64) -> f64 {
    let co_psi = (co_pa * 1.0e5) / PSIA_PER_BAR;
    let p_psi = psia(p_pa);
    let pb_psi = psia(pb_pa);
    bob * (-co_psi * (p_psi - pb_psi)).exp()
}

fn oil_compressibility(rs: f64, gamma_g: f64, api: f64, t_k: f64, p_pa: f64) -> f64 {
    let rs_field = rs / SM3_PER_SCF_STB;
    let t = fahrenheit(t_k);
    let p_field = psia(p_pa);
    let co_psi =
        (-1433.0 + 5.0 * rs_field + 17.2 * t - 1180.0 * gamma_g + 12.61 * api) / (1.0e5 * p_field);
    co_psi * PSIA_PER_BAR
}

fn dead_oil_viscosity_beggs_robinson(api: f64, t_k: f64) -> f64 {
    let t = fahrenheit(t_k);
    let z = 3.0324 - 0.02023 * api;
    let y = 10f64.powf(z);
    let x = y * t.powf(-1.163);
    10f64.powf(x) - 1.0
}

fn dead_oil_viscosity_glaso(api: f64, t_k: f64) -> f64 {
    let t = fahrenheit(t_k);
    let a = 10.313 * t.log10() - 36.447;
    3.141e10 * t.powf(-3.444) * api.log10().powf(a)
}

fn dead_oil_viscosity_kartoatmodjo(api: f64, t_k: f64) -> f64 {
    let t = fahrenheit(t_k);
    let a = 5.7526 * t.log10() - 26.9718;
    16.0e8 * t.powf(-2.8177) * api.log10().powf(a)
}

fn saturated_oil_viscosity_beggs_robinson(mu_od_pa_s: f64, rs: f64) -> f64 {
    let mu_field = pa_s_to_centipoise(mu_od_pa_s);
    let rs_field = rs / SM3_PER_SCF_STB;
    let a = 10.715 * (rs_field + 100.0).powf(-0.515);
    let b = 5.44 * (rs_field + 150.0).powf(-0.338);
    (a * mu_field.powf(b)) / 1000.0
}

fn saturated_oil_viscosity_kartoatmodjo(mu_od_pa_s: f64, rs: f64) -> f64 {
    let mu_field = pa_s_to_centipoise(mu_od_pa_s);
    let rs_field = rs / SM3_PER_SCF_STB;
    let a = -0.06821 + 0.9824 / (rs_field + 100.0) + 40.34e-5 * rs_field;
    let b = 0.2001 + 0.8428 / (rs_field + 100.0) - 0.000845 * rs_field;
    (10f64.powf(a) * mu_field.powf(b)) / 1000.0
}

fn undersaturated_oil_viscosity_vasquez_beggs(mu_ob_pa_s: f64, p_pa: f64, pb_pa: f64) -> f64 {
    let mu_field = pa_s_to_centipoise(mu_ob_pa_s);
    let p = psia(p_pa);
    let pb = psia(pb_pa);
    let m = 2.6 * p.powf(1.187) * (-11.513 - 8.98e-5 * p).exp();
    (mu_field * (p / pb).powf(m)) / 1000.0
}

fn undersaturated_oil_viscosity_bergman_sutton(mu_ob_pa_s: f64, p_pa: f64, pb_pa: f64) -> f64 {
    let mu_field = pa_s_to_centipoise(mu_ob_pa_s);
    let p = psia(p_pa);
    let pb = psia(pb_pa);
    let ln_mu = mu_field.ln();
    let a = 6.5698e-7 * ln_mu * ln_mu - 1.48211e-5 * ln_mu + 2.27877e-4;
    let b = 2.24623e-2 * ln_mu + 0.873204;
    (mu_field * (a * (p - pb).powf(b)).exp()) / 1000.0
}

fn gas_fvf(p_pa: f64, t_k: f64, z: f64) -> f64 {
    let p_field = psia(p_pa);
    let t_r = fahrenheit(t_k) + RANKINE_OFFSET;
    0.02827 * z * t_r / p_field
}

fn gas_fvf_rb_per_mscf(p_pa: f64, t_k: f64, z: f64) -> f64 {
    let p_field = psia(p_pa);
    let t_r = fahrenheit(t_k) + RANKINE_OFFSET;
    5.04 * z * t_r / p_field
}

fn gas_viscosity_lee_gonzalez_eakin(t_k: f64, rho_kg_m3: f64, molar_mass_kg_mol: f64) -> f64 {
    let t_r = fahrenheit(t_k) + RANKINE_OFFSET;
    let rho_field = kg_m3_to_lb_ft3(rho_kg_m3);
    let mg = molar_mass_kg_mol * 1000.0;
    let k = (9.4 + 0.02 * mg) * t_r.powf(1.5) / (209.0 + 19.0 * mg + t_r);
    let x = 3.5 + 986.0 / t_r + 0.01 * mg;
    let y = 2.4 - 0.2 * x;
    1.0e-4 * k * (x * (rho_field / 62.4).powf(y)).exp()
}
