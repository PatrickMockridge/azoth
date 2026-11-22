//! `pvt.gas_material_balance` - NeqSim's gas material balance.
//!
//! Spec: `specs/models/pvt/gas_material_balance.toml`. The arithmetic is
//! `pvtsimulation/reservoirproperties/materialbalance/GasMaterialBalance.java` at the pin: the
//! p/Z volumetric fit, the Cole plot and the Havlena-Odeh fit.

use azoth_core::units::ThermodynamicTemperature;
use azoth_core::warning::WarningCode;
use azoth_core::{AzothError, Result, Warning};

use crate::gas_correlations::z_factor_sutton;
use crate::standard::STANDARD_PRESSURE_BAR;

/// The standard temperature the class states, in K.
pub const T_SC: f64 = 288.71;

/// Result of `pvt.gas_material_balance`.
#[derive(Debug, Clone, PartialEq)]
pub struct GasMaterialBalanceResult {
    /// The gas formation volume factor.
    pub gas_fvf: Option<f64>,
    /// The original gas in place; absent when the p/Z line does not decline.
    pub ogip: Option<f64>,
    /// The initial p/Z intercept.
    pub pi_over_zi: Option<f64>,
    /// The p/Z line's slope.
    pub slope: Option<f64>,
    /// The fit's coefficient of determination.
    pub r_squared: Option<f64>,
    /// The Cole plot's cumulative gas, from the second history point.
    pub cole_gp: Option<Vec<f64>>,
    /// The Cole plot's F/Eg, from the second history point.
    pub cole_f_over_eg: Option<Vec<f64>>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The gas material balance over the stated inputs.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown calculation, a missing required input, arrays
///   of different lengths or fewer than two points, a non-positive pressure, Z or temperature,
///   a singular regression, a Cole plot with a non-positive expansion, or a non-finite answer.
#[allow(clippy::too_many_arguments)]
pub fn gas_material_balance(
    calculation: &str,
    pressure: Option<&[f64]>,
    z: Option<&[f64]>,
    gp: Option<&[f64]>,
    we: Option<&[f64]>,
    pressure_value: Option<f64>,
    z_value: Option<f64>,
    temperature: Option<ThermodynamicTemperature>,
    gas_gravity: Option<f64>,
) -> Result<GasMaterialBalanceResult> {
    let mut row = empty();
    match calculation {
        "bg" => {
            let p = positive("pressure_value", pressure_value)? / 1.0e5;
            let z = positive("z_value", z_value)?;
            let t = required_temperature(temperature)?;
            row.gas_fvf = Some(finite("gas_fvf", bg(p, z, t))?);
        }
        "fit_volumetric" => {
            let (pressure, z, gp) = arrays3(pressure, z, gp)?;
            let fit = fit_volumetric(&pressure, &z, &gp, &mut row)?;
            set_fit(&mut row, fit)?;
        }
        "fit_volumetric_from_gravity" => {
            let (pressure, _, gp) = arrays3(pressure, z, gp)?;
            let t = required_temperature(temperature)?;
            let gamma = gamma(gas_gravity)?;
            let z = pressure
                .iter()
                .map(|p| z_factor_sutton(*p / 1.0e5, t, gamma))
                .collect::<Vec<_>>();
            let fit = fit_volumetric(&pressure, &z, &gp, &mut row)?;
            set_fit(&mut row, fit)?;
        }
        "cole_plot" => {
            let (pressure, z, gp) = arrays3(pressure, z, gp)?;
            let t = required_temperature(temperature)?;
            let bgi = bg(pressure[0] / 1.0e5, z[0], t);
            let mut gp_out = Vec::with_capacity(pressure.len() - 1);
            let mut ratio = Vec::with_capacity(pressure.len() - 1);
            for index in 1..pressure.len() {
                let bg_cur = bg(pressure[index] / 1.0e5, z[index], t);
                let f = gp[index] * bg_cur;
                let eg = bg_cur - bgi;
                if eg <= 1.0e-30 {
                    return Err(AzothError::invalid_input(
                        format!("pressure[{index}]"),
                        "the Cole plot's expansion is not positive, so its ratio is not a number",
                    ));
                }
                gp_out.push(finite("cole_gp", gp[index])?);
                ratio.push(finite("cole_f_over_eg", f / eg)?);
            }
            row.cole_gp = Some(gp_out);
            row.cole_f_over_eg = Some(ratio);
        }
        "fit_havlena_odeh" => {
            let (pressure, z, gp) = arrays3(pressure, z, gp)?;
            let we = we_array(we, pressure.len())?;
            let t = required_temperature(temperature)?;
            let bgi = bg(pressure[0] / 1.0e5, z[0], t);
            let mut x = Vec::with_capacity(pressure.len() - 1);
            let mut y = Vec::with_capacity(pressure.len() - 1);
            for index in 1..pressure.len() {
                let bg_cur = bg(pressure[index] / 1.0e5, z[index], t);
                let f = gp[index] * bg_cur;
                let eg = bg_cur - bgi;
                x.push(eg);
                y.push(f - we[index]);
            }
            let (slope, r_squared) = through_origin(&x, &y)?;
            row.ogip = Some(finite("ogip", slope)?);
            row.pi_over_zi = Some(0.0);
            row.slope = Some(finite("slope", slope)?);
            row.r_squared = Some(finite("r_squared", r_squared)?);
        }
        other => {
            return Err(AzothError::invalid_input(
                "calculation",
                format!("{other:?} is not one of the gas material-balance calculations"),
            ));
        }
    }
    Ok(row)
}

fn empty() -> GasMaterialBalanceResult {
    GasMaterialBalanceResult {
        gas_fvf: None,
        ogip: None,
        pi_over_zi: None,
        slope: None,
        r_squared: None,
        cole_gp: None,
        cole_f_over_eg: None,
        warnings: Vec::new(),
    }
}

fn zero_ogip() -> Warning {
    Warning::new(
        WarningCode::OutOfValidRange,
        "the p/Z line does not decline, so the original gas in place is absent",
    )
}

fn fit_volumetric(
    pressure: &[f64],
    z: &[f64],
    gp: &[f64],
    row: &mut GasMaterialBalanceResult,
) -> Result<(Option<f64>, f64, f64, f64)> {
    let mut poz = Vec::with_capacity(pressure.len());
    for index in 0..pressure.len() {
        let z = positive("z", Some(z[index]))?;
        poz.push((pressure[index] / 1.0e5) / z);
    }
    let (slope, intercept, r_squared) = linear(gp, &poz)?;
    let ogip = if slope < 0.0 {
        Some(finite("ogip", -intercept / slope)?)
    } else {
        row.warnings.push(zero_ogip());
        None
    };
    Ok((ogip, intercept, slope, r_squared))
}

fn set_fit(row: &mut GasMaterialBalanceResult, fit: (Option<f64>, f64, f64, f64)) -> Result<()> {
    row.ogip = fit.0;
    row.pi_over_zi = Some(finite("pi_over_zi", fit.1 * 1.0e5)?);
    row.slope = Some(finite("slope", fit.2)?);
    row.r_squared = Some(finite("r_squared", fit.3)?);
    Ok(())
}

fn bg(p_bara: f64, z: f64, t_k: f64) -> f64 {
    z * t_k / p_bara * STANDARD_PRESSURE_BAR / T_SC
}

fn linear(x: &[f64], y: &[f64]) -> Result<(f64, f64, f64)> {
    let n = x.len() as f64;
    if x.len() < 2 {
        return Err(AzothError::invalid_input(
            "pressure",
            "a regression needs at least two points",
        ));
    }
    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    for index in 0..x.len() {
        sx += x[index];
        sy += y[index];
        sxx += x[index] * x[index];
        sxy += x[index] * y[index];
    }
    let denom = n * sxx - sx * sx;
    if denom.abs() < 1.0e-30 {
        return Err(AzothError::invalid_input(
            "gp",
            "the regression is singular: the cumulative gas does not vary",
        ));
    }
    let slope = (n * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / n;
    let mean = sy / n;
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for index in 0..x.len() {
        let predicted = slope * x[index] + intercept;
        ss_res += (y[index] - predicted) * (y[index] - predicted);
        ss_tot += (y[index] - mean) * (y[index] - mean);
    }
    let r_squared = if ss_tot > 1.0e-30 {
        1.0 - ss_res / ss_tot
    } else {
        1.0
    };
    Ok((slope, intercept, r_squared))
}

fn through_origin(x: &[f64], y: &[f64]) -> Result<(f64, f64)> {
    if x.is_empty() {
        return Err(AzothError::invalid_input(
            "pressure",
            "the fit needs at least one point",
        ));
    }
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut sy = 0.0;
    for index in 0..x.len() {
        sxx += x[index] * x[index];
        sxy += x[index] * y[index];
        sy += y[index];
    }
    if sxx < 1.0e-30 {
        return Err(AzothError::invalid_input(
            "pressure",
            "the regression is singular: the expansion does not vary",
        ));
    }
    let slope = sxy / sxx;
    let mean = sy / x.len() as f64;
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for index in 0..x.len() {
        let predicted = slope * x[index];
        ss_res += (y[index] - predicted) * (y[index] - predicted);
        ss_tot += (y[index] - mean) * (y[index] - mean);
    }
    let r_squared = if ss_tot > 1.0e-30 {
        1.0 - ss_res / ss_tot
    } else {
        1.0
    };
    Ok((slope, r_squared))
}

fn arrays3(
    pressure: Option<&[f64]>,
    z: Option<&[f64]>,
    gp: Option<&[f64]>,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let pressure = pressure.ok_or_else(|| {
        AzothError::invalid_input("pressure", "the calculation needs the pressure history")
    })?;
    let z =
        z.ok_or_else(|| AzothError::invalid_input("z", "the calculation needs the Z history"))?;
    let gp = gp.ok_or_else(|| {
        AzothError::invalid_input("gp", "the calculation needs the cumulative gas history")
    })?;
    if pressure.len() != z.len() || pressure.len() != gp.len() {
        return Err(AzothError::invalid_input(
            "pressure",
            "the history arrays are different lengths",
        ));
    }
    if pressure.len() < 2 {
        return Err(AzothError::invalid_input(
            "pressure",
            "at least two history points are required",
        ));
    }
    for (index, value) in pressure.iter().enumerate() {
        if !value.is_finite() || *value <= 0.0 {
            return Err(AzothError::invalid_input(
                format!("pressure[{index}]"),
                "a material balance pressure must be finite and positive",
            ));
        }
    }
    Ok((pressure.to_vec(), z.to_vec(), gp.to_vec()))
}

fn we_array(we: Option<&[f64]>, length: usize) -> Result<Vec<f64>> {
    let we = we.ok_or_else(|| {
        AzothError::invalid_input("we", "the Havlena-Odeh fit needs the water influx history")
    })?;
    if we.len() != length {
        return Err(AzothError::invalid_input(
            "we",
            "the water influx history is a different length from the pressure history",
        ));
    }
    Ok(we.to_vec())
}

fn finite(name: &str, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            "the calculation answered a non-finite value; the inputs are outside its state",
        ))
    }
}

fn positive(name: &str, value: Option<f64>) -> Result<f64> {
    let value =
        value.ok_or_else(|| AzothError::invalid_input(name, "the calculation needs this field"))?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            "the value must be finite and positive",
        ))
    }
}

fn required_temperature(value: Option<ThermodynamicTemperature>) -> Result<f64> {
    let value = value.ok_or_else(|| {
        AzothError::invalid_input("temperature", "the calculation needs the temperature")
    })?;
    if value.value.is_finite() && value.value > 0.0 {
        Ok(value.value)
    } else {
        Err(AzothError::invalid_input(
            "temperature",
            "an absolute temperature; zero and below are not states",
        ))
    }
}

fn gamma(value: Option<f64>) -> Result<f64> {
    let value = positive("gas_gravity", value)?;
    if (0.57..=1.68).contains(&value) {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            "gas_gravity",
            "the Sutton correlation is defined for a specific gravity in [0.57, 1.68]",
        ))
    }
}
