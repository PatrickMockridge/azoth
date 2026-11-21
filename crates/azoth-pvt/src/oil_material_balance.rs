//! `pvt.oil_material_balance` - NeqSim's oil material balance.
//!
//! Spec: `specs/models/pvt/oil_material_balance.toml`. The arithmetic is
//! `pvtsimulation/reservoirproperties/materialbalance/OilMaterialBalance.java` at the pin: the
//! four material-balance terms, the three fits and the drive indices.

use azoth_core::{AzothError, Result, Warning};

/// Result of `pvt.oil_material_balance`.
#[derive(Debug, Clone, PartialEq)]
pub struct OilMaterialBalanceResult {
    /// The reservoir withdrawal term F.
    pub f: Option<f64>,
    /// The oil and dissolved-gas expansion term Eo.
    pub eo_value: Option<f64>,
    /// The gas-cap expansion term Eg.
    pub eg_value: Option<f64>,
    /// The connate-water and rock expansion term Efw.
    pub efw_value: Option<f64>,
    /// The fitted original oil in place.
    pub ooip: Option<f64>,
    /// The fitted gas-cap ratio.
    pub m_fit: Option<f64>,
    /// The fit's coefficient of determination.
    pub r_squared: Option<f64>,
    /// The depletion drive index.
    pub ddi: Option<f64>,
    /// The segregation (gas-cap) drive index.
    pub sdi: Option<f64>,
    /// The water drive index.
    pub wdi: Option<f64>,
    /// The expansion drive index.
    pub edi: Option<f64>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The oil material balance over the stated inputs.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown calculation, a missing required scalar, a
///   history of different lengths or fewer than two points, a non-positive Eo in the water-drive
///   plot, a singular regression, a water saturation at or above one, or a non-finite answer.
#[allow(clippy::too_many_arguments)]
pub fn oil_material_balance(
    calculation: &str,
    np: Option<f64>,
    rp: Option<f64>,
    bo: Option<f64>,
    rs: Option<f64>,
    bg: Option<f64>,
    wp: Option<f64>,
    bw: Option<f64>,
    boi: Option<f64>,
    rsi: Option<f64>,
    bgi: Option<f64>,
    cw: Option<f64>,
    swi: Option<f64>,
    cf: Option<f64>,
    delta_p: Option<f64>,
    m: Option<f64>,
    n: Option<f64>,
    eo_term: Option<f64>,
    eg_term: Option<f64>,
    efw_term: Option<f64>,
    we_value: Option<f64>,
    f_value: Option<f64>,
    f_history: Option<&[f64]>,
    eo_history: Option<&[f64]>,
    eg_history: Option<&[f64]>,
    we_history: Option<&[f64]>,
) -> Result<OilMaterialBalanceResult> {
    let mut row = empty();
    match calculation {
        "withdrawal_f" => {
            let np = scalar("np", np)?;
            let rp = scalar("rp", rp)?;
            let bo = scalar("bo", bo)?;
            let rs = scalar("rs", rs)?;
            let bg = scalar("bg", bg)?;
            let wp = scalar("wp", wp)?;
            let bw = scalar("bw", bw)?;
            row.f = Some(finite("f", np * (bo + (rp - rs) * bg) + wp * bw)?);
        }
        "eo" => {
            let bo = scalar("bo", bo)?;
            let boi = scalar("boi", boi)?;
            let rsi = scalar("rsi", rsi)?;
            let rs = scalar("rs", rs)?;
            let bg = scalar("bg", bg)?;
            row.eo_value = Some(finite("eo_value", (bo - boi) + (rsi - rs) * bg)?);
        }
        "eg" => {
            let boi = scalar("boi", boi)?;
            let bg = scalar("bg", bg)?;
            let bgi = positive("bgi", bgi)?;
            row.eg_value = Some(finite("eg_value", boi * (bg / bgi - 1.0))?);
        }
        "efw" => {
            let boi = scalar("boi", boi)?;
            let m = scalar("m", m)?;
            let cw = scalar("cw", cw)?;
            let swi = scalar("swi", swi)?;
            let cf = scalar("cf", cf)?;
            let delta_p = scalar("delta_p", delta_p)?;
            if !(0.0..1.0).contains(&swi) {
                return Err(AzothError::invalid_input(
                    "swi",
                    "the initial water saturation is a fraction below one",
                ));
            }
            row.efw_value = Some(finite(
                "efw_value",
                boi * (1.0 + m) * (cw * swi + cf) / (1.0 - swi) * delta_p,
            )?);
        }
        "fit_depletion_drive" => {
            let (f, eo) = arrays2(f_history, eo_history)?;
            let (ooip, r_squared) = through_origin(&eo, &f)?;
            row.ooip = Some(finite("ooip", ooip)?);
            row.m_fit = Some(0.0);
            row.r_squared = Some(finite("r_squared", r_squared)?);
        }
        "fit_gas_cap_drive" => {
            let (f, eo) = arrays2(f_history, eo_history)?;
            let eg = eg_history.ok_or_else(|| {
                AzothError::invalid_input("eg_history", "the gas-cap fit needs the Eg history")
            })?;
            if eg.len() != f.len() {
                return Err(AzothError::invalid_input(
                    "eg_history",
                    "the Eg history is a different length from the withdrawal history",
                ));
            }
            let (first, second, r_squared) = two_predictor(&eo, eg, &f)?;
            row.ooip = Some(finite("ooip", first)?);
            let m_fit = if first.abs() > 1.0e-30 {
                second / first
            } else {
                0.0
            };
            row.m_fit = Some(finite("m_fit", m_fit)?);
            row.r_squared = Some(finite("r_squared", r_squared)?);
        }
        "fit_water_drive" => {
            let (f, eo) = arrays2(f_history, eo_history)?;
            let we = we_history.ok_or_else(|| {
                AzothError::invalid_input("we_history", "the water-drive fit needs the We history")
            })?;
            if we.len() != f.len() {
                return Err(AzothError::invalid_input(
                    "we_history",
                    "the We history is a different length from the withdrawal history",
                ));
            }
            let bw = scalar("bw", bw)?;
            let mut x = Vec::with_capacity(f.len());
            let mut y = Vec::with_capacity(f.len());
            for index in 0..f.len() {
                if eo[index].abs() < 1.0e-30 {
                    return Err(AzothError::invalid_input(
                        format!("eo_history[{index}]"),
                        "Eo must be non-zero for the water-drive plot",
                    ));
                }
                x.push(we[index] * bw / eo[index]);
                y.push(f[index] / eo[index]);
            }
            let (_, intercept, r_squared) = linear(&x, &y)?;
            row.ooip = Some(finite("ooip", intercept)?);
            row.m_fit = Some(0.0);
            row.r_squared = Some(finite("r_squared", r_squared)?);
        }
        "drive_indices" => {
            let n = scalar("n", n)?;
            let m = scalar("m", m)?;
            let eo_term = scalar("eo_term", eo_term)?;
            let eg_term = scalar("eg_term", eg_term)?;
            let efw_term = scalar("efw_term", efw_term)?;
            let we = scalar("we_value", we_value)?;
            let bw = scalar("bw", bw)?;
            let f = scalar("f_value", f_value)?;
            if f.abs() < 1.0e-30 {
                row.ddi = Some(0.0);
                row.sdi = Some(0.0);
                row.wdi = Some(0.0);
                row.edi = Some(0.0);
            } else {
                row.ddi = Some(finite("ddi", n * eo_term / f)?);
                row.sdi = Some(finite("sdi", n * m * eg_term / f)?);
                row.wdi = Some(finite("wdi", we * bw / f)?);
                row.edi = Some(finite("edi", n * efw_term / f)?);
            }
        }
        other => {
            return Err(AzothError::invalid_input(
                "calculation",
                format!("{other:?} is not one of the oil material-balance calculations"),
            ));
        }
    }
    Ok(row)
}

fn empty() -> OilMaterialBalanceResult {
    OilMaterialBalanceResult {
        f: None,
        eo_value: None,
        eg_value: None,
        efw_value: None,
        ooip: None,
        m_fit: None,
        r_squared: None,
        ddi: None,
        sdi: None,
        wdi: None,
        edi: None,
        warnings: Vec::new(),
    }
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

fn scalar(name: &str, value: Option<f64>) -> Result<f64> {
    let value =
        value.ok_or_else(|| AzothError::invalid_input(name, "the calculation needs this field"))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

fn positive(name: &str, value: Option<f64>) -> Result<f64> {
    let value = scalar(name, value)?;
    if value > 0.0 {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            "the value must be positive",
        ))
    }
}

fn arrays2(f: Option<&[f64]>, eo: Option<&[f64]>) -> Result<(Vec<f64>, Vec<f64>)> {
    let f = f.ok_or_else(|| {
        AzothError::invalid_input("f_history", "the fit needs the withdrawal history")
    })?;
    let eo =
        eo.ok_or_else(|| AzothError::invalid_input("eo_history", "the fit needs the Eo history"))?;
    if f.len() != eo.len() {
        return Err(AzothError::invalid_input(
            "f_history",
            "the history arrays are different lengths",
        ));
    }
    if f.len() < 2 {
        return Err(AzothError::invalid_input(
            "f_history",
            "at least two history points are required",
        ));
    }
    Ok((f.to_vec(), eo.to_vec()))
}

fn through_origin(x: &[f64], y: &[f64]) -> Result<(f64, f64)> {
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
            "eo_history",
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

fn linear(x: &[f64], y: &[f64]) -> Result<(f64, f64, f64)> {
    let n = x.len() as f64;
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
            "we_history",
            "the regression is singular: the influx does not vary",
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

fn two_predictor(x1: &[f64], x2: &[f64], y: &[f64]) -> Result<(f64, f64, f64)> {
    let mut s11 = 0.0;
    let mut s12 = 0.0;
    let mut s22 = 0.0;
    let mut s1y = 0.0;
    let mut s2y = 0.0;
    for index in 0..x1.len() {
        s11 += x1[index] * x1[index];
        s12 += x1[index] * x2[index];
        s22 += x2[index] * x2[index];
        s1y += x1[index] * y[index];
        s2y += x2[index] * y[index];
    }
    let det = s11 * s22 - s12 * s12;
    if det.abs() < 1.0e-30 {
        return Err(AzothError::invalid_input(
            "eo_history",
            "the two-predictor regression is singular: the predictors are collinear",
        ));
    }
    let a = (s1y * s22 - s2y * s12) / det;
    let b = (s11 * s2y - s12 * s1y) / det;
    let mean = y.iter().sum::<f64>() / y.len() as f64;
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for index in 0..x1.len() {
        let predicted = a * x1[index] + b * x2[index];
        ss_res += (y[index] - predicted) * (y[index] - predicted);
        ss_tot += (y[index] - mean) * (y[index] - mean);
    }
    let r_squared = if ss_tot > 1.0e-30 {
        1.0 - ss_res / ss_tot
    } else {
        1.0
    };
    Ok((a, b, r_squared))
}
