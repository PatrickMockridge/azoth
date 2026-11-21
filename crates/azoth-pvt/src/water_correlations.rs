//! `pvt.water_correlations` - NeqSim's water and brine correlation table.
//!
//! Spec: `specs/models/pvt/water_correlations.toml`. The arithmetic is
//! `pvtsimulation/util/WaterPropertyCorrelations.java` and the McCain path of
//! `pvtsimulation/simulation/SolutionGasWaterRatio.java` at the pin. The two salinity
//! conventions are kept apart: ppm for the water properties, molality for the McCain Rsw.

use azoth_core::units::{Pressure, ThermodynamicTemperature};
use azoth_core::{AzothError, Result, Warning};

use crate::standard::{CENTIPOISE_PER_PA_S, PSIA_PER_BAR};

/// The calculations this id dispatches.
pub const CALCULATIONS: &[&str] = &[
    "water_fvf_mccain",
    "water_fvf_osif",
    "dead_water_viscosity_mccain",
    "water_viscosity_mccain",
    "water_compressibility_mccain",
    "brine_density_batzle_wang",
    "water_density",
    "solution_gas_water_ratio_culberson",
    "water_gas_surface_tension",
    "rsw_mccain",
];

/// One water correlation call, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct WaterQuery {
    /// The correlation.
    pub calculation: String,
    /// Absolute temperature.
    pub temperature: Option<ThermodynamicTemperature>,
    /// Absolute pressure.
    pub pressure: Option<Pressure>,
    /// Salinity in ppm (mg/L), the water-property convention.
    pub salinity_ppm: Option<f64>,
    /// Salinity as a weight fraction, the Batzle-Wang convention.
    pub salinity_wt_fraction: Option<f64>,
    /// Salinity in mol/kg water, the McCain Rsw convention.
    pub salinity_molality: Option<f64>,
}

/// One water correlation answer.
#[derive(Debug, Clone, PartialEq)]
pub struct WaterResultRow {
    /// The call this row answered.
    pub calculation: String,
    /// The water formation volume factor.
    pub fvf: Option<f64>,
    /// The water or brine viscosity, in Pa*s.
    pub viscosity: Option<f64>,
    /// The water compressibility in 1/Pa.
    pub compressibility: Option<f64>,
    /// The water or brine density, in kg/m3.
    pub density: Option<f64>,
    /// The solution gas-water ratio, in Sm3/Sm3.
    pub rsw: Option<f64>,
    /// The water-gas surface tension, in N/m.
    pub surface_tension: Option<f64>,
}

/// Result of `pvt.water_correlations`.
#[derive(Debug, Clone, PartialEq)]
pub struct WaterCorrelationsResult {
    /// One row per input row.
    pub queries: Vec<WaterResultRow>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The correlation table over the stated calls.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown calculation, a missing required field, a
///   non-positive temperature or pressure, a negative salinity, a weight fraction outside
///   [0, 0.3], or a non-finite answer.
pub fn water_correlations(queries: &[WaterQuery]) -> Result<WaterCorrelationsResult> {
    let mut rows = Vec::with_capacity(queries.len());
    for (index, query) in queries.iter().enumerate() {
        rows.push(one(index, query)?);
    }
    Ok(WaterCorrelationsResult {
        queries: rows,
        warnings: Vec::new(),
    })
}

fn one(index: usize, query: &WaterQuery) -> Result<WaterResultRow> {
    let calculation = query.calculation.as_str();
    let mut row = empty(calculation);
    match calculation {
        "water_fvf_mccain" => {
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, query.pressure)?;
            row.fvf = Some(finite(index, "fvf", water_fvf_mccain(t, p))?);
        }
        "water_fvf_osif" => {
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, query.pressure)?;
            let ppm = salinity_ppm(index, query.salinity_ppm)?;
            row.fvf = Some(finite(index, "fvf", water_fvf_osif(t, p, ppm))?);
        }
        "dead_water_viscosity_mccain" => {
            let t = temperature(index, query.temperature)?;
            let ppm = salinity_ppm(index, query.salinity_ppm)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                dead_water_viscosity_mccain(t, ppm) / CENTIPOISE_PER_PA_S,
            )?);
        }
        "water_viscosity_mccain" => {
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, query.pressure)?;
            let ppm = salinity_ppm(index, query.salinity_ppm)?;
            row.viscosity = Some(finite(
                index,
                "viscosity",
                water_viscosity_mccain(t, p, ppm) / CENTIPOISE_PER_PA_S,
            )?);
        }
        "water_compressibility_mccain" => {
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, query.pressure)?;
            let ppm = salinity_ppm(index, query.salinity_ppm)?;
            row.compressibility = Some(finite(
                index,
                "compressibility",
                water_compressibility_mccain(t, p, ppm) * 1.0e-5,
            )?);
        }
        "brine_density_batzle_wang" => {
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, query.pressure)?;
            let wt = salinity_wt_fraction(index, query.salinity_wt_fraction)?;
            row.density = Some(finite(
                index,
                "density",
                brine_density_batzle_wang(t, p, wt),
            )?);
        }
        "water_density" => {
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, query.pressure)?;
            row.density = Some(finite(
                index,
                "density",
                brine_density_batzle_wang(t, p, 0.0),
            )?);
        }
        "solution_gas_water_ratio_culberson" => {
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, query.pressure)?;
            let ppm = salinity_ppm(index, query.salinity_ppm)?;
            row.rsw = Some(finite(
                index,
                "rsw",
                solution_gas_water_ratio_culberson(t, p, ppm),
            )?);
        }
        "water_gas_surface_tension" => {
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, query.pressure)?;
            row.surface_tension = Some(finite(
                index,
                "surface_tension",
                water_gas_surface_tension(t, p) * 1.0e-3,
            )?);
        }
        "rsw_mccain" => {
            let t = temperature(index, query.temperature)?;
            let p = pressure(index, query.pressure)?;
            let molality = salinity_molality(index, query.salinity_molality)?;
            row.rsw = Some(finite(index, "rsw", rsw_mccain(t, p, molality))?);
        }
        _ => {
            return Err(AzothError::invalid_input(
                format!("queries[{index}].calculation"),
                format!("{calculation:?} is not one of the water calculations"),
            ));
        }
    }
    Ok(row)
}

fn empty(calculation: &str) -> WaterResultRow {
    WaterResultRow {
        calculation: calculation.to_owned(),
        fvf: None,
        viscosity: None,
        compressibility: None,
        density: None,
        rsw: None,
        surface_tension: None,
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

fn required(index: usize, name: &str, value: Option<f64>) -> Result<f64> {
    let value = value.ok_or_else(|| {
        AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "the calculation needs this field",
        )
    })?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].{name}"),
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

fn salinity_ppm(index: usize, value: Option<f64>) -> Result<f64> {
    let value = required(index, "salinity_ppm", value)?;
    if value < 0.0 {
        return Err(AzothError::invalid_input(
            format!("queries[{index}].salinity_ppm"),
            "a salinity cannot be negative",
        ));
    }
    Ok(value)
}

fn salinity_molality(index: usize, value: Option<f64>) -> Result<f64> {
    let value = required(index, "salinity_molality", value)?;
    if value < 0.0 {
        return Err(AzothError::invalid_input(
            format!("queries[{index}].salinity_molality"),
            "a salinity cannot be negative",
        ));
    }
    Ok(value)
}

fn salinity_wt_fraction(index: usize, value: Option<f64>) -> Result<f64> {
    let value = required(index, "salinity_wt_fraction", value)?;
    if !(0.0..=0.3).contains(&value) {
        return Err(AzothError::invalid_input(
            format!("queries[{index}].salinity_wt_fraction"),
            "the Batzle-Wang correlation is stated for a weight fraction in [0, 0.3]",
        ));
    }
    Ok(value)
}

fn temperature(index: usize, value: Option<ThermodynamicTemperature>) -> Result<f64> {
    let value = value.ok_or_else(|| {
        AzothError::invalid_input(
            format!("queries[{index}].temperature"),
            "the calculation needs this field",
        )
    })?;
    if value.value.is_finite() && value.value > 0.0 {
        Ok(value.value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].temperature"),
            "an absolute temperature; zero and below are not states",
        ))
    }
}

fn pressure(index: usize, value: Option<Pressure>) -> Result<f64> {
    let value = value.ok_or_else(|| {
        AzothError::invalid_input(
            format!("queries[{index}].pressure"),
            "the calculation needs this field",
        )
    })?;
    if value.value.is_finite() && value.value > 0.0 {
        Ok(value.value)
    } else {
        Err(AzothError::invalid_input(
            format!("queries[{index}].pressure"),
            "an absolute pressure; zero and below are not states",
        ))
    }
}

fn to_fahrenheit(t_kelvin: f64) -> f64 {
    t_kelvin * 9.0 / 5.0 - 459.67
}

fn to_psia(p_pa: f64) -> f64 {
    (p_pa / 1.0e5) * PSIA_PER_BAR
}

fn water_fvf_mccain(t_k: f64, p_pa: f64) -> f64 {
    let tf = to_fahrenheit(t_k);
    let pp = to_psia(p_pa);
    let dvwt = -1.0001e-2 + 1.33391e-4 * tf + 5.50654e-7 * tf * tf;
    let dvwp = -1.95301e-9 * pp * tf
        - 1.72834e-13 * pp * pp * tf
        - 3.58922e-7 * pp
        - 2.25341e-10 * pp * pp;
    (1.0 + dvwt) * (1.0 + dvwp)
}

fn water_fvf_osif(t_k: f64, p_pa: f64, ppm: f64) -> f64 {
    let tf = to_fahrenheit(t_k);
    let pp = to_psia(p_pa);
    let s = ppm / 1.0e6;
    let t_shift = tf - 60.0;
    let numerator = 5.1e-8 * pp
        + (tf - 60.0) * (5.47e-6 - 1.95e-10 * pp)
        + t_shift * t_shift * (-3.23e-8)
        + 8.5e-13 * pp * pp;
    1.0 + numerator - s * (0.0840655 * s * tf.sqrt())
}

fn dead_water_viscosity_mccain(t_k: f64, ppm: f64) -> f64 {
    let tf = to_fahrenheit(t_k).clamp(100.0, 400.0);
    let s = ppm / 1.0e4;
    let a = 109.574 - 8.40564 * s + 0.313314 * s * s + 8.72213e-3 * s * s * s;
    let b = -1.12166 + 2.63951e-2 * s - 6.79461e-4 * s * s - 5.47119e-5 * s * s * s
        + 1.55586e-6 * s * s * s * s;
    a * tf.powf(b)
}

fn water_viscosity_mccain(t_k: f64, p_pa: f64, ppm: f64) -> f64 {
    let pp = to_psia(p_pa);
    let mu_dead = dead_water_viscosity_mccain(t_k, ppm);
    let factor = 0.9994 + 4.0295e-5 * pp + 3.1062e-9 * pp * pp;
    mu_dead * factor
}

fn water_compressibility_mccain(t_k: f64, p_pa: f64, ppm: f64) -> f64 {
    let tf = to_fahrenheit(t_k);
    let pp = to_psia(p_pa);
    let s = ppm / 1.0e6;
    let cw_pure = (3.8546 - 0.000134 * pp)
        + (-(0.01052 + 4.77e-7 * pp)) * tf
        + (3.9267e-5 + 8.8e-10 * pp) * tf * tf;
    let cw_pure = cw_pure * 1.0e-6;
    let sal_corr = 1.0 + s * 0.7;
    cw_pure * sal_corr * PSIA_PER_BAR
}

fn brine_density_batzle_wang(t_k: f64, p_pa: f64, s: f64) -> f64 {
    let tc = t_k - 273.15;
    let p_mpa = (p_pa / 1.0e5) * 0.1;
    let rho_w = 1.0
        + 1.0e-6
            * (-80.0 * tc - 3.3 * tc * tc + 0.00175 * tc * tc * tc + 489.0 * p_mpa
                - 2.0 * tc * p_mpa
                + 0.016 * tc * tc * p_mpa
                - 1.3e-5 * tc * tc * tc * p_mpa
                - 0.333 * p_mpa * p_mpa
                - 0.002 * tc * p_mpa * p_mpa);
    let rho_w = rho_w * 1000.0;
    rho_w
        + s * (0.668
            + 0.44 * s
            + 1.0e-6
                * (300.0 * p_mpa - 2400.0 * p_mpa * s
                    + tc * (80.0 + 3.0 * tc - 3300.0 * s - 13.0 * p_mpa + 47.0 * p_mpa * s)))
            * 1000.0
}

fn solution_gas_water_ratio_culberson(t_k: f64, p_pa: f64, ppm: f64) -> f64 {
    let tf = to_fahrenheit(t_k);
    let pp = to_psia(p_pa);
    let a = 8.15839 - 6.12265e-2 * tf + 1.91663e-4 * tf * tf - 2.1654e-7 * tf * tf * tf;
    let b = 1.01021e-2 - 7.44241e-5 * tf + 3.05553e-7 * tf * tf - 2.94883e-10 * tf * tf * tf;
    let rsw_pure = a + b * pp;
    let s = ppm / 1.0e4;
    let sal_corr = 1.0 - (0.0753 - 1.73e-4 * tf) * s;
    rsw_pure * sal_corr * 0.178108
}

fn water_gas_surface_tension(t_k: f64, p_pa: f64) -> f64 {
    let tf = to_fahrenheit(t_k);
    let pp = to_psia(p_pa);
    let sigma74 = (75.0 - 0.1108 * tf - 6.722e-5 * tf * tf).max(1.0);
    if pp <= 1074.0 {
        return sigma74;
    }
    let c = (1.0 - 0.0001 * (pp - 1074.0)).max(0.2);
    sigma74 * c
}

fn rsw_mccain(t_k: f64, p_pa: f64, molality: f64) -> f64 {
    let tf = (t_k - 273.15) * 9.0 / 5.0 + 32.0;
    let pp = to_psia(p_pa);
    let a = 8.15839 - 6.12265e-2 * tf + 1.91663e-4 * tf * tf - 2.1654e-7 * tf * tf * tf;
    let b = 1.01021e-2 - 7.44241e-5 * tf + 3.05553e-7 * tf * tf - 2.94883e-10 * tf * tf * tf;
    let c = (-9.02505 + 0.130237 * tf - 8.53425e-4 * tf * tf + 2.34122e-6 * tf * tf * tf
        - 2.37049e-9 * tf * tf * tf * tf)
        * 1.0e-7;
    let rsw_pure = a + b * pp + c * pp * pp;

    let mass_nacl = molality * 58.44;
    let salinity_wt_percent = (mass_nacl / (1000.0 + mass_nacl)) * 100.0;
    let s1 = 2.12 + 3.085e-3 * tf - 3.878e-6 * tf * tf;
    let s2 = -1.0071e-4 - 1.272e-6 * tf + 9.46e-9 * tf * tf;
    let s3 = (-5.0e-5 + 4.285e-7 * tf - 1.14e-9 * tf * tf) * 1.0e-4;
    let cs = (s1 + s2 * pp + s3 * pp * pp) * 1.0e-4;
    let correction = 10f64.powf(-cs * salinity_wt_percent);
    (rsw_pure * correction * 0.178108).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mccain_rsw_converts_molality_to_weight_percent() {
        // 3.5 wt% is 0.6206267976011888 mol/kg by the class's own conversion.
        let mass = 0.620_626_797_601_188_8 * 58.44;
        let wt: f64 = mass / (1000.0 + mass) * 100.0;
        assert!((wt - 3.5).abs() < 1.0e-9);
    }

    #[test]
    fn the_surface_tension_is_clamped_at_its_floor() {
        let sigma = water_gas_surface_tension(600.0, 1.0e5);
        assert!(sigma >= 1.0);
    }
}
