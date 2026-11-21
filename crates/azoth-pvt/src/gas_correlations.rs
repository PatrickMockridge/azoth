//! `pvt.gas_correlations` - NeqSim's gas correlation table.
//!
//! Spec: `specs/models/pvt/gas_correlations.toml`. The arithmetic is
//! `pvtsimulation/util/ZFactorCorrelations.java`, `GasPseudoCriticalProperties.java`,
//! `GasPseudoPressure.java` and `reservoirproperties/CompositionEstimation.java` at the pin.
//! The two Hall-Yarborough variants are kept apart; the pseudo-pressure integration uses the
//! `GasPseudoPressure` one.

use azoth_core::units::{MolarMass, Pressure, ThermodynamicTemperature};
use azoth_core::{AzothError, Result, Warning};

use crate::standard::PSIA_PER_BAR;

/// The calculations this id dispatches.
pub const CALCULATIONS: &[&str] = &[
    "pseudo_critical_temperature_standing",
    "pseudo_critical_pressure_standing",
    "pseudo_critical_temperature_sutton",
    "pseudo_critical_pressure_sutton",
    "pseudo_critical_temperature_piper",
    "pseudo_critical_pressure_piper",
    "wichert_aziz",
    "pseudo_reduced_temperature",
    "pseudo_reduced_pressure",
    "z_factor_hall_yarborough",
    "z_factor_dranchuk_abou_kassem",
    "z_factor_papay",
    "z_factor_sutton",
    "z_factor_sour_gas",
    "gas_density",
    "gas_fvf_from_z",
    "pseudo_pressure_from_correlation",
    "delta_pseudo_pressure",
    "h2s_concentration_from_temperature",
    "h2s_concentration_from_co2",
];

/// One gas correlation call, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct GasQuery {
    /// The correlation.
    pub calculation: String,
    /// Gas specific gravity.
    pub gamma_g: Option<f64>,
    /// H2S mole fraction.
    pub y_h2s: Option<f64>,
    /// CO2 mole fraction.
    pub y_co2: Option<f64>,
    /// N2 mole fraction.
    pub y_n2: Option<f64>,
    /// Absolute temperature.
    pub temperature: Option<ThermodynamicTemperature>,
    /// Absolute pressure.
    pub pressure: Option<Pressure>,
    /// The lower pressure of the pseudo-pressure integral.
    pub reference_pressure: Option<Pressure>,
    /// Pseudocritical temperature.
    pub tpc: Option<ThermodynamicTemperature>,
    /// Pseudocritical pressure.
    pub ppc: Option<Pressure>,
    /// Pseudoreduced temperature.
    pub tpr: Option<f64>,
    /// Pseudoreduced pressure.
    pub ppr: Option<f64>,
    /// Gas molar mass.
    pub molar_mass: Option<MolarMass>,
}

/// One gas correlation answer.
#[derive(Debug, Clone, PartialEq)]
pub struct GasResultRow {
    /// The call this row answered.
    pub calculation: String,
    /// The pseudocritical or corrected pseudocritical temperature, in K.
    pub temperature: Option<f64>,
    /// The pseudocritical or corrected pseudocritical pressure, in Pa.
    pub pressure: Option<f64>,
    /// A Z factor or a reduced ratio.
    pub factor: Option<f64>,
    /// The gas density, in kg/m3.
    pub density: Option<f64>,
    /// The gas formation volume factor.
    pub gas_fvf: Option<f64>,
    /// The pseudopressure, in the class's bara^2/cP.
    pub pseudo_pressure: Option<f64>,
    /// The Haaland H2S estimate.
    pub h2s_fraction: Option<f64>,
}

/// Result of `pvt.gas_correlations`.
#[derive(Debug, Clone, PartialEq)]
pub struct GasCorrelationsResult {
    /// One row per input row.
    pub queries: Vec<GasResultRow>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The correlation table over the stated calls.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown calculation, a missing required field, a
///   gas gravity outside the correlation's range, a non-positive quantity where the
///   correlation divides by it or takes its logarithm, a reduced state the Hall-Yarborough
///   form refuses, or a non-finite answer.
pub fn gas_correlations(queries: &[GasQuery]) -> Result<GasCorrelationsResult> {
    let mut rows = Vec::with_capacity(queries.len());
    for (index, query) in queries.iter().enumerate() {
        rows.push(one(index, query)?);
    }
    Ok(GasCorrelationsResult {
        queries: rows,
        warnings: Vec::new(),
    })
}

fn one(index: usize, query: &GasQuery) -> Result<GasResultRow> {
    let calculation = query.calculation.as_str();
    let mut row = empty(calculation);
    match calculation {
        "pseudo_critical_temperature_standing" => {
            let gamma = gamma(index, query.gamma_g, 0.55, 1.8)?;
            row.temperature = Some(finite(index, "temperature", standing_tpc(gamma))?);
        }
        "pseudo_critical_pressure_standing" => {
            let gamma = gamma(index, query.gamma_g, 0.55, 1.8)?;
            row.pressure = Some(finite(index, "pressure", standing_ppc(gamma) * 1.0e5)?);
        }
        "pseudo_critical_temperature_sutton" => {
            let gamma = gamma(index, query.gamma_g, 0.57, 1.68)?;
            row.temperature = Some(finite(index, "temperature", sutton_tpc(gamma))?);
        }
        "pseudo_critical_pressure_sutton" => {
            let gamma = gamma(index, query.gamma_g, 0.57, 1.68)?;
            row.pressure = Some(finite(index, "pressure", sutton_ppc(gamma) * 1.0e5)?);
        }
        "pseudo_critical_temperature_piper" => {
            let gamma = positive(index, "gamma_g", query.gamma_g)?;
            let y_h2s = fraction(index, "y_h2s", query.y_h2s)?;
            let y_co2 = fraction(index, "y_co2", query.y_co2)?;
            let y_n2 = fraction(index, "y_n2", query.y_n2)?;
            row.temperature = Some(finite(
                index,
                "temperature",
                piper_tpc(gamma, y_h2s, y_co2, y_n2),
            )?);
        }
        "pseudo_critical_pressure_piper" => {
            let gamma = positive(index, "gamma_g", query.gamma_g)?;
            let y_h2s = fraction(index, "y_h2s", query.y_h2s)?;
            let y_co2 = fraction(index, "y_co2", query.y_co2)?;
            let y_n2 = fraction(index, "y_n2", query.y_n2)?;
            row.pressure = Some(finite(
                index,
                "pressure",
                piper_ppc(gamma, y_h2s, y_co2, y_n2) * 1.0e5,
            )?);
        }
        "wichert_aziz" => {
            let tpc = temperature(index, "tpc", query.tpc)?;
            let ppc = pressure(index, "ppc", query.ppc)?;
            let y_h2s = fraction(index, "y_h2s", query.y_h2s)?;
            let y_co2 = fraction(index, "y_co2", query.y_co2)?;
            let (tpc_corr, ppc_corr) = wichert_aziz(tpc, ppc / 1.0e5, y_h2s, y_co2);
            row.temperature = Some(finite(index, "temperature", tpc_corr)?);
            row.pressure = Some(finite(index, "pressure", ppc_corr * 1.0e5)?);
        }
        "pseudo_reduced_temperature" => {
            let t = temperature(index, "temperature", query.temperature)?;
            let tpc = temperature(index, "tpc", query.tpc)?;
            row.factor = Some(finite(index, "factor", t / tpc)?);
        }
        "pseudo_reduced_pressure" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let ppc = pressure(index, "ppc", query.ppc)?;
            row.factor = Some(finite(index, "factor", p / ppc)?);
        }
        "z_factor_hall_yarborough" => {
            let tpr = positive(index, "tpr", query.tpr)?;
            let ppr = finite_field(index, "ppr", query.ppr)?;
            if ppr < 0.0 {
                return Err(AzothError::invalid_input(
                    format!("queries[{index}].ppr"),
                    "a pseudoreduced pressure cannot be negative",
                ));
            }
            row.factor = Some(finite(index, "factor", hall_yarborough_zfactor(tpr, ppr))?);
        }
        "z_factor_dranchuk_abou_kassem" => {
            let tpr = positive(index, "tpr", query.tpr)?;
            let ppr = finite_field(index, "ppr", query.ppr)?;
            if ppr < 0.0 {
                return Err(AzothError::invalid_input(
                    format!("queries[{index}].ppr"),
                    "a pseudoreduced pressure cannot be negative",
                ));
            }
            row.factor = Some(finite(index, "factor", dranchuk_abou_kassem(tpr, ppr))?);
        }
        "z_factor_papay" => {
            let tpr = positive(index, "tpr", query.tpr)?;
            let ppr = finite_field(index, "ppr", query.ppr)?;
            if ppr < 0.0 {
                return Err(AzothError::invalid_input(
                    format!("queries[{index}].ppr"),
                    "a pseudoreduced pressure cannot be negative",
                ));
            }
            row.factor = Some(finite(index, "factor", papay(tpr, ppr))?);
        }
        "z_factor_sutton" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let t = temperature(index, "temperature", query.temperature)?;
            let gamma = gamma(index, query.gamma_g, 0.57, 1.68)?;
            row.factor = Some(finite(
                index,
                "factor",
                z_factor_sutton(p / 1.0e5, t, gamma),
            )?);
        }
        "z_factor_sour_gas" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let t = temperature(index, "temperature", query.temperature)?;
            let gamma = gamma(index, query.gamma_g, 0.57, 1.68)?;
            let y_h2s = fraction(index, "y_h2s", query.y_h2s)?;
            let y_co2 = fraction(index, "y_co2", query.y_co2)?;
            row.factor = Some(finite(
                index,
                "factor",
                z_factor_sour_gas(p / 1.0e5, t, gamma, y_h2s, y_co2),
            )?);
        }
        "gas_density" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let t = temperature(index, "temperature", query.temperature)?;
            let gamma = gamma(index, query.gamma_g, 0.57, 1.68)?;
            let mw = molar_mass_raw(index, query.molar_mass)?;
            row.density = Some(finite(
                index,
                "density",
                gas_density(p / 1.0e5, t, gamma, mw * 1000.0),
            )?);
        }
        "gas_fvf_from_z" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let t = temperature(index, "temperature", query.temperature)?;
            let gamma = gamma(index, query.gamma_g, 0.57, 1.68)?;
            row.gas_fvf = Some(finite(
                index,
                "gas_fvf",
                gas_fvf_from_z(p / 1.0e5, t, gamma),
            )?);
        }
        "pseudo_pressure_from_correlation" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let p_ref = pressure(index, "reference_pressure", query.reference_pressure)?;
            let t = temperature(index, "temperature", query.temperature)?;
            let gamma = gamma(index, query.gamma_g, 0.55, 1.8)?;
            let mw = molar_mass_raw(index, query.molar_mass)?;
            row.pseudo_pressure = Some(finite(
                index,
                "pseudo_pressure",
                pseudo_pressure(p / 1.0e5, p_ref / 1.0e5, t, gamma, mw * 1000.0)?,
            )?);
        }
        "delta_pseudo_pressure" => {
            let p = pressure(index, "pressure", query.pressure)?;
            let p_ref = pressure(index, "reference_pressure", query.reference_pressure)?;
            let t = temperature(index, "temperature", query.temperature)?;
            let gamma = gamma(index, query.gamma_g, 0.55, 1.8)?;
            let mw = molar_mass_raw(index, query.molar_mass)?;
            let value = pseudo_pressure(p / 1.0e5, p_ref / 1.0e5, t, gamma, mw * 1000.0)?;
            row.pseudo_pressure = Some(finite(index, "pseudo_pressure", value.abs())?);
        }
        "h2s_concentration_from_temperature" => {
            let t = temperature(index, "temperature", query.temperature)?;
            row.h2s_fraction = Some(finite(index, "h2s_fraction", 5.0e7 * (-6543.0 / t).exp())?);
        }
        "h2s_concentration_from_co2" => {
            let t = temperature(index, "temperature", query.temperature)?;
            let y_co2 = positive(index, "y_co2", query.y_co2)?;
            row.h2s_fraction = Some(finite(
                index,
                "h2s_fraction",
                (11.7 - 4438.3 / t + 0.7 * (y_co2 * 100.0).ln()).exp(),
            )?);
        }
        _ => {
            return Err(AzothError::invalid_input(
                format!("queries[{index}].calculation"),
                format!("{calculation:?} is not one of the gas calculations"),
            ));
        }
    }
    Ok(row)
}

fn empty(calculation: &str) -> GasResultRow {
    GasResultRow {
        calculation: calculation.to_owned(),
        temperature: None,
        pressure: None,
        factor: None,
        density: None,
        gas_fvf: None,
        pseudo_pressure: None,
        h2s_fraction: None,
    }
}

fn finite(index: usize, name: &str, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "the correlation answered a non-finite value; the inputs are outside its state",
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

fn fraction(index: usize, name: &str, value: Option<f64>) -> Result<f64> {
    let value = finite_field(index, name, value)?;
    if (0.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "a mole fraction lies in [0, 1]",
        ))
    }
}

fn gamma(index: usize, value: Option<f64>, min: f64, max: f64) -> Result<f64> {
    let value = finite_field(index, "gamma_g", value)?;
    if value < min || value > max {
        Err(AzothError::invalid_input(
            format!("queries[{index}].gamma_g"),
            format!("the correlation is defined for a gas specific gravity in [{min}, {max}]"),
        ))
    } else {
        Ok(value)
    }
}

fn missing(index: usize, name: &str) -> AzothError {
    AzothError::invalid_input(
        format!("queries[{index}].{name}"),
        "the calculation needs this field",
    )
}

fn temperature(index: usize, name: &str, value: Option<ThermodynamicTemperature>) -> Result<f64> {
    let value = value.ok_or_else(|| missing(index, name))?;
    if value.value.is_finite() && value.value > 0.0 {
        Ok(value.value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "an absolute temperature; zero and below are not states",
        ))
    }
}

fn pressure(index: usize, name: &str, value: Option<Pressure>) -> Result<f64> {
    let value = value.ok_or_else(|| missing(index, name))?;
    if value.value.is_finite() && value.value > 0.0 {
        Ok(value.value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "an absolute pressure; zero and below are not states",
        ))
    }
}

fn molar_mass_raw(index: usize, value: Option<MolarMass>) -> Result<f64> {
    let value = value.ok_or_else(|| missing(index, "molar_mass"))?;
    if value.value.is_finite() && value.value > 0.0 {
        Ok(value.value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].molar_mass"),
            "a molar mass; zero and below are not states the correlation takes",
        ))
    }
}

fn standing_tpc(gamma: f64) -> f64 {
    (168.0 + 325.0 * gamma - 12.5 * gamma * gamma) * 5.0 / 9.0
}

fn standing_ppc(gamma: f64) -> f64 {
    (677.0 + 15.0 * gamma - 37.5 * gamma * gamma) / PSIA_PER_BAR
}

fn sutton_tpc(gamma: f64) -> f64 {
    (169.2 + 349.5 * gamma - 74.0 * gamma * gamma) * 5.0 / 9.0
}

fn sutton_ppc(gamma: f64) -> f64 {
    (756.8 - 131.0 * gamma - 3.6 * gamma * gamma) / PSIA_PER_BAR
}

const TC_H2S: f64 = 672.35;
const PC_H2S: f64 = 1306.0;
const TC_CO2: f64 = 547.58;
const PC_CO2: f64 = 1071.0;
const TC_N2: f64 = 227.16;
const PC_N2: f64 = 493.1;

fn piper_jk(gamma: f64, y_h2s: f64, y_co2: f64, y_n2: f64) -> (f64, f64) {
    let mut y_hc = 1.0 - y_h2s - y_co2 - y_n2;
    if y_hc <= 0.0 {
        y_hc = 1.0;
    }
    let mut gamma_hc = (gamma - 0.9672 * y_n2 - 1.1765 * y_co2 - 1.5196 * y_h2s) / y_hc;
    if gamma_hc < 0.55 {
        gamma_hc = 0.55;
    }
    let f_h2s_j = y_h2s * (TC_H2S / PC_H2S);
    let f_co2_j = y_co2 * (TC_CO2 / PC_CO2);
    let f_n2_j = y_n2 * (TC_N2 / PC_N2);
    let j = 0.11582 - 0.45820 * f_h2s_j - 0.90348 * f_co2_j - 0.66026 * f_n2_j + 0.70729 * gamma_hc
        - 0.099397 * gamma_hc * gamma_hc;
    let f_h2s_k = y_h2s * (TC_H2S / PC_H2S).sqrt();
    let f_co2_k = y_co2 * (TC_CO2 / PC_CO2).sqrt();
    let f_n2_k = y_n2 * (TC_N2 / PC_N2).sqrt();
    let k = 3.8216 - 0.06534 * f_h2s_k - 0.42113 * f_co2_k - 0.91249 * f_n2_k + 17.438 * gamma_hc
        - 3.2191 * gamma_hc * gamma_hc;
    (j, k)
}

fn piper_tpc(gamma: f64, y_h2s: f64, y_co2: f64, y_n2: f64) -> f64 {
    let (j, k) = piper_jk(gamma, y_h2s, y_co2, y_n2);
    k * k / j * 5.0 / 9.0
}

fn piper_ppc(gamma: f64, y_h2s: f64, y_co2: f64, y_n2: f64) -> f64 {
    let (j, k) = piper_jk(gamma, y_h2s, y_co2, y_n2);
    (k * k / j) / j / PSIA_PER_BAR
}

fn wichert_aziz(tpc_k: f64, ppc_bara: f64, y_h2s: f64, y_co2: f64) -> (f64, f64) {
    let tpc_r = tpc_k * 9.0 / 5.0;
    let ppc_psia = ppc_bara * PSIA_PER_BAR;
    let a = y_h2s + y_co2;
    let b = y_h2s;
    let epsilon = 120.0 * (a.powf(0.9) - a.powf(1.6)) + 15.0 * (b.powf(0.5) - b.powi(4));
    let tpc_corr_r = tpc_r - epsilon;
    let ppc_corr_psia = ppc_psia * tpc_corr_r / (tpc_r + b * (1.0 - b) * epsilon);
    (tpc_corr_r * 5.0 / 9.0, ppc_corr_psia / PSIA_PER_BAR)
}

fn hall_yarborough_zfactor(tpr: f64, ppr: f64) -> f64 {
    if ppr < 1.0e-10 {
        return 1.0;
    }
    let t = 1.0 / tpr;
    let a = -0.06125 * ppr * t * (-1.2 * (1.0 - t) * (1.0 - t)).exp();
    let b = 14.76 * t - 9.76 * t * t + 4.58 * t * t * t;
    let c = 90.7 * t - 242.2 * t * t + 42.4 * t * t * t;
    let d = 2.18 + 2.82 * t;
    let mut y: f64 = 0.001;
    for _ in 0..200 {
        let fy = a + (y + y * y + y * y * y - y * y * y * y) / (1.0 - y).powi(3) - b * y * y
            + c * y.powf(d);
        let dfy = (1.0 + 4.0 * y + 4.0 * y * y - 4.0 * y * y * y + y * y * y * y)
            / (1.0 - y).powi(4)
            - 2.0 * b * y
            + c * d * y.powf(d - 1.0);
        let y_new = (y - fy / dfy).clamp(1.0e-12, 0.9999);
        if (y_new - y).abs() < 1.0e-13 {
            y = y_new;
            break;
        }
        y = y_new;
    }
    let z = -a / y;
    if !(0.05..=5.0).contains(&z) { 1.0 } else { z }
}

fn dranchuk_abou_kassem(tpr: f64, ppr: f64) -> f64 {
    if ppr < 1.0e-10 {
        return 1.0;
    }
    let a1 = 0.3265;
    let a2 = -1.0700;
    let a3 = -0.5339;
    let a4 = 0.01569;
    let a5 = -0.05165;
    let a6 = 0.5475;
    let a7 = -0.7361;
    let a8 = 0.1844;
    let a9 = 0.1056;
    let a10 = 0.6134;
    let a11 = 0.7210;
    let mut rho = 0.27 * ppr / tpr;
    for _ in 0..200 {
        let rr = rho;
        let rr2 = rr * rr;
        let c1 = a1 + a2 / tpr + a3 / tpr.powi(3) + a4 / tpr.powi(4) + a5 / tpr.powi(5);
        let c2 = a6 + a7 / tpr + a8 / (tpr * tpr);
        let c3 = a9 * (a7 / tpr + a8 / (tpr * tpr));
        let fz = 1.0 + c1 * rr + c2 * rr2 - c3 * rr2 * rr2 * rr
            + a10 * (1.0 + a11 * rr2) * (rr2 / tpr.powi(3)) * (-a11 * rr2).exp()
            - 0.27 * ppr / (rho * tpr);
        let dfz = c1 + 2.0 * c2 * rr - 5.0 * c3 * rr2 * rr2
            + a10
                * (rr / tpr.powi(3))
                * (2.0 + 4.0 * a11 * rr2 - 2.0 * a11 * a11 * rr2 * rr2)
                * (-a11 * rr2).exp()
            + 0.27 * ppr / (rho * rho * tpr);
        let mut new = rho - fz / dfz;
        if new < 1.0e-15 {
            new = 1.0e-15;
        }
        if (new - rho).abs() < 1.0e-12 {
            rho = new;
            break;
        }
        rho = new;
    }
    let z = 0.27 * ppr / (rho * tpr);
    if !(0.05..=5.0).contains(&z) { 1.0 } else { z }
}

fn papay(tpr: f64, ppr: f64) -> f64 {
    let z =
        1.0 - 3.52 * ppr / 10f64.powf(0.9813 * tpr) + 0.274 * ppr * ppr / 10f64.powf(0.8157 * tpr);
    z.max(0.05)
}

pub(crate) fn z_factor_sutton(p_bara: f64, t_k: f64, gamma: f64) -> f64 {
    let tpc = sutton_tpc(gamma);
    let ppc = sutton_ppc(gamma);
    hall_yarborough_zfactor(t_k / tpc, p_bara / ppc)
}

fn z_factor_sour_gas(p_bara: f64, t_k: f64, gamma: f64, y_h2s: f64, y_co2: f64) -> f64 {
    let tpc = sutton_tpc(gamma);
    let ppc = sutton_ppc(gamma);
    let (tpc_corr, ppc_corr) = wichert_aziz(tpc, ppc, y_h2s, y_co2);
    hall_yarborough_zfactor(t_k / tpc_corr, p_bara / ppc_corr)
}

fn gas_density(p_bara: f64, t_k: f64, gamma: f64, mw_g_mol: f64) -> f64 {
    let z = z_factor_sutton(p_bara, t_k, gamma);
    p_bara * mw_g_mol / (z * 0.083_144_7 * t_k)
}

fn gas_fvf_from_z(p_bara: f64, t_k: f64, gamma: f64) -> f64 {
    let z = z_factor_sutton(p_bara, t_k, gamma);
    z * t_k / p_bara * 1.01325 / 288.71
}

/// Lee-Gonzalez-Eakin viscosity in cP, in the correlation's own units.
pub(crate) fn lee_gonzalez_eakin_field(t_rankine: f64, rho_lb_ft3: f64, mg_g_mol: f64) -> f64 {
    let k = (9.4 + 0.02 * mg_g_mol) * t_rankine.powf(1.5) / (209.0 + 19.0 * mg_g_mol + t_rankine);
    let x = 3.5 + 986.0 / t_rankine + 0.01 * mg_g_mol;
    let y = 2.4 - 0.2 * x;
    1.0e-4 * k * (x * (rho_lb_ft3 / 62.4).powf(y)).exp()
}

fn hall_yarborough_pseudo(tpr: f64, ppr: f64) -> Result<f64> {
    if !tpr.is_finite() || !ppr.is_finite() || tpr <= 1.0 || !(0.0..25.0).contains(&ppr) {
        return Err(AzothError::invalid_input(
            "tpr",
            "Hall-Yarborough needs finite reduced properties with Tpr > 1.0 and 0 <= Ppr < 25.0",
        ));
    }
    if ppr < 1.0e-10 {
        return Ok(1.0);
    }
    let t = 1.0 / tpr;
    let a0 = -0.06125 * t * (-1.2 * (1.0 - t) * (1.0 - t)).exp();
    let b = 14.76 * t - 9.76 * t * t + 4.58 * t * t * t;
    let c = 90.7 * t - 242.2 * t * t + 42.4 * t * t * t;
    let d = 2.18 + 2.82 * t;
    let mut y: f64 = 0.001;
    let mut converged = false;
    for _ in 0..100 {
        let fy = a0 * ppr + (y + y * y + y * y * y - y * y * y * y) / (1.0 - y).powi(3) - b * y * y
            + c * y.powf(d);
        let dfy = (1.0 + 4.0 * y + 4.0 * y * y - 4.0 * y * y * y + y * y * y * y)
            / (1.0 - y).powi(4)
            - 2.0 * b * y
            + c * d * y.powf(d - 1.0);
        if dfy.abs() < 1.0e-30 {
            break;
        }
        let mut y_new = y - fy / dfy;
        if y_new < 0.0 {
            y_new = y / 2.0;
        }
        if y_new > 1.0 {
            y_new = (y + 1.0) / 2.0;
        }
        if (y_new - y).abs() < 1.0e-12 {
            y = y_new;
            converged = true;
            break;
        }
        y = y_new;
    }
    if !converged {
        return Err(AzothError::invalid_input(
            "tpr",
            "Hall-Yarborough did not converge at this reduced state",
        ));
    }
    let z = -a0 * ppr / y;
    if !z.is_finite() || z <= 0.0 {
        return Err(AzothError::invalid_input(
            "tpr",
            "Hall-Yarborough answered a compressibility factor that is not finite and positive",
        ));
    }
    Ok(z)
}

fn integrand_correlation(p_bara: f64, t_k: f64, gamma: f64, mw_g_mol: f64) -> Result<f64> {
    let t_rankine = t_k * 9.0 / 5.0;
    let p_psia = p_bara * PSIA_PER_BAR;
    let tpc = standing_tpc(gamma);
    let ppc = standing_ppc(gamma);
    let z = hall_yarborough_pseudo(t_k / tpc, p_bara / ppc)?;
    let rho_lb = p_psia * mw_g_mol / (z * 10.7316 * t_rankine);
    let mu = lee_gonzalez_eakin_field(t_rankine, rho_lb, mw_g_mol);
    if !mu.is_finite() || mu <= 0.0 {
        return Err(AzothError::invalid_input(
            "molar_mass",
            "the Lee-Gonzalez-Eakin viscosity must be finite and positive",
        ));
    }
    Ok(2.0 * p_bara / (mu * z))
}

fn integrate_correlation(p_low: f64, p_high: f64, t_k: f64, gamma: f64, mw: f64) -> Result<f64> {
    if (p_high - p_low).abs() < 1.0e-10 {
        return Ok(0.0);
    }
    // The class's `nSteps = 200`, which is even; the odd-step guard is dead there and here.
    let n = 200u32;
    let h = (p_high - p_low) / f64::from(n);
    let mut sum = integrand_correlation(p_low, t_k, gamma, mw)?
        + integrand_correlation(p_high, t_k, gamma, mw)?;
    for i in 1..n {
        let p = p_low + f64::from(i) * h;
        let f = integrand_correlation(p, t_k, gamma, mw)?;
        if i % 2 == 0 {
            sum += 2.0 * f;
        } else {
            sum += 4.0 * f;
        }
    }
    Ok(h / 3.0 * sum)
}

fn pseudo_pressure(
    p_bara: f64,
    p_ref_bara: f64,
    t_k: f64,
    gamma: f64,
    mw_g_mol: f64,
) -> Result<f64> {
    if !p_bara.is_finite() || p_bara <= 0.0 {
        return Err(AzothError::invalid_input(
            "pressure",
            "an absolute pressure; zero and below are not states",
        ));
    }
    if !p_ref_bara.is_finite() || p_ref_bara <= 0.0 {
        return Err(AzothError::invalid_input(
            "reference_pressure",
            "an absolute pressure; zero and below are not states",
        ));
    }
    if !t_k.is_finite() || t_k <= 0.0 {
        return Err(AzothError::invalid_input(
            "temperature",
            "an absolute temperature; zero and below are not states",
        ));
    }
    if !mw_g_mol.is_finite() || mw_g_mol <= 0.0 {
        return Err(AzothError::invalid_input(
            "molar_mass",
            "a molar mass; zero and below are not states the correlation takes",
        ));
    }
    let tpc = standing_tpc(gamma);
    let ppc = standing_ppc(gamma);
    let tpr = t_k / tpc;
    let max_ppr = p_bara.max(p_ref_bara) / ppc;
    if tpr <= 1.0 || max_ppr >= 25.0 {
        return Err(AzothError::invalid_input(
            "tpr",
            "the correlation needs Tpr > 1.0 and a maximum Ppr below 25.0",
        ));
    }
    let result = integrate_correlation(p_ref_bara, p_bara, t_k, gamma, mw_g_mol)?;
    if !result.is_finite() {
        return Err(AzothError::invalid_input(
            "pseudo_pressure",
            "the pseudopressure integral must be finite",
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sweet_gas_reference_values_are_the_tests_own() {
        assert!((standing_tpc(0.75) - 224.84375).abs() < 1.0e-12);
        assert!((standing_ppc(0.75) - 45.998_721_024_834_87).abs() < 1.0e-12);
        assert!((sutton_tpc(0.75) - 216.5).abs() < 1.0e-12);
        assert!((sutton_ppc(0.75) - 45.265_723_465_574_54).abs() < 1.0e-12);
    }

    #[test]
    fn the_ideal_gas_limits_are_one() {
        assert_eq!(hall_yarborough_zfactor(1.5, 0.0), 1.0);
        assert_eq!(dranchuk_abou_kassem(1.5, 0.0), 1.0);
        assert!((hall_yarborough_zfactor(1.5, 1.0e-12) - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn wichert_aziz_returns_the_input_pair_without_impurities() {
        let (tpc, ppc) = wichert_aziz(222.22, 45.51, 0.0, 0.0);
        assert!((tpc - 222.22).abs() < 1.0e-9);
        assert!((ppc - 45.51).abs() < 1.0e-9);
    }
}
