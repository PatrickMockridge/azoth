//! `pvt.decline_curve` - NeqSim's decline-curve analysis.
//!
//! Spec: `specs/models/pvt/decline_curve.toml`. The arithmetic is
//! `pvtsimulation/util/DeclineCurveAnalysis.java` at the pin: the Arps family, the two
//! rate-time fits and the Duong model. Rates, times and cumulative volumes are unit-agnostic
//! and cross as dimensionless magnitudes.

use azoth_core::warning::WarningCode;
use azoth_core::{AzothError, Result, Warning};

/// Result of `pvt.decline_curve`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclineCurveResult {
    /// The forecast's times.
    pub time: Option<Vec<f64>>,
    /// The forecast's rates.
    pub rate: Option<Vec<f64>>,
    /// The forecast's cumulative production.
    pub cumulative: Option<Vec<f64>>,
    /// The rate at the stated time.
    pub rate_value: Option<f64>,
    /// The cumulative production at the stated time.
    pub cumulative_value: Option<f64>,
    /// The nominal decline rate, 1/day.
    pub nominal: Option<f64>,
    /// The effective annual decline.
    pub effective: Option<f64>,
    /// The instantaneous decline rate at the stated time.
    pub instantaneous_decline_rate: Option<f64>,
    /// The time to reach the economic limit; absent when the decline is zero.
    pub time_to_rate: Option<f64>,
    /// The estimated ultimate recovery; absent when the decline is zero.
    pub eur: Option<f64>,
    /// The remaining reserves to the economic limit.
    pub remaining: Option<f64>,
    /// The exponential decline rate from two points, 1/day.
    pub exponential_decline: Option<f64>,
    /// The fitted initial rate.
    pub fit_qi: Option<f64>,
    /// The fitted nominal decline rate.
    pub fit_di: Option<f64>,
    /// The fitted Arps exponent.
    pub fit_b: Option<f64>,
    /// The fit's coefficient of determination.
    pub fit_r_squared: Option<f64>,
    /// The fitted Duong rate at unit time.
    pub duong_q1: Option<f64>,
    /// The fitted Duong intercept.
    pub duong_a: Option<f64>,
    /// The fitted Duong slope.
    pub duong_m: Option<f64>,
    /// The Duong time function at the stated time.
    pub duong_time: Option<f64>,
    /// The summary's effective annual decline.
    pub summary_effective: Option<f64>,
    /// The summary's rate at one year.
    pub summary_rate_1yr: Option<f64>,
    /// The summary's rate at five years.
    pub summary_rate_5yr: Option<f64>,
    /// The summary's rate at ten years.
    pub summary_rate_10yr: Option<f64>,
    /// The summary's cumulative at one year.
    pub summary_cum_1yr: Option<f64>,
    /// The summary's cumulative at five years.
    pub summary_cum_5yr: Option<f64>,
    /// The summary's cumulative at ten years.
    pub summary_cum_10yr: Option<f64>,
    /// The summary's time to the economic limit, days.
    pub summary_time_to_limit: Option<f64>,
    /// The summary's time to the economic limit, years.
    pub summary_time_to_limit_years: Option<f64>,
    /// The summary's EUR.
    pub summary_eur: Option<f64>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The decline calculation over the stated inputs.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown calculation, a missing required scalar, a
///   non-positive rate where a fit needs one, a bad window, fewer than three history points,
///   `t2 <= t1`, a non-positive time step, or a non-finite answer.
#[allow(clippy::too_many_arguments)]
pub fn decline_curve(
    calculation: &str,
    qi: Option<f64>,
    di: Option<f64>,
    b: Option<f64>,
    t: Option<f64>,
    q_limit: Option<f64>,
    d_nominal: Option<f64>,
    d_effective: Option<f64>,
    q1: Option<f64>,
    t1: Option<f64>,
    q2: Option<f64>,
    t2: Option<f64>,
    q3: Option<f64>,
    t3: Option<f64>,
    t_start: Option<f64>,
    t_end: Option<f64>,
    t_step: Option<f64>,
    t_current: Option<f64>,
    a: Option<f64>,
    m: Option<f64>,
    start_index: Option<f64>,
    end_index: Option<f64>,
    history_time: Option<&[f64]>,
    history_rate: Option<&[f64]>,
) -> Result<DeclineCurveResult> {
    let mut row = empty();
    match calculation {
        "rate" => {
            let qi = scalar("qi", qi)?;
            let di = scalar("di", di)?;
            let b = scalar("b", b)?;
            let t = scalar("t", t)?;
            row.rate_value = Some(finite("rate_value", rate(qi, di, b, t))?);
        }
        "rate_exponential" => {
            let qi = non_negative("qi", qi)?;
            let di = non_negative("di", di)?;
            let t = non_negative("t", t)?;
            row.rate_value = Some(finite("rate_value", rate(qi, di, 0.0, t))?);
        }
        "rate_hyperbolic" => {
            let qi = scalar("qi", qi)?;
            let di = scalar("di", di)?;
            let b = scalar("b", b)?;
            let t = scalar("t", t)?;
            row.rate_value = Some(finite("rate_value", rate(qi, di, b, t))?);
        }
        "rate_harmonic" => {
            let qi = scalar("qi", qi)?;
            let di = scalar("di", di)?;
            let t = scalar("t", t)?;
            row.rate_value = Some(finite("rate_value", rate(qi, di, 1.0, t))?);
        }
        "cumulative_production" => {
            let qi = scalar("qi", qi)?;
            let di = scalar("di", di)?;
            let b = scalar("b", b)?;
            let t = scalar("t", t)?;
            row.cumulative_value = Some(finite("cumulative_value", cumulative(qi, di, b, t))?);
        }
        "cumulative_exponential" => {
            let qi = non_negative("qi", qi)?;
            let di = non_negative("di", di)?;
            let t = non_negative("t", t)?;
            row.cumulative_value = Some(finite("cumulative_value", cumulative(qi, di, 0.0, t))?);
        }
        "cumulative_harmonic" => {
            let qi = scalar("qi", qi)?;
            let di = scalar("di", di)?;
            let t = scalar("t", t)?;
            row.cumulative_value = Some(finite("cumulative_value", cumulative(qi, di, 1.0, t))?);
        }
        "nominal_to_effective_annual" => {
            let d = non_negative("d_nominal", d_nominal)?;
            row.effective = Some(finite("effective", 1.0 - (-d * 365.25).exp())?);
        }
        "effective_to_nominal_annual" => {
            let d = required("d_effective", d_effective)?;
            if !(d > 0.0 && d < 1.0) {
                return Err(AzothError::invalid_input(
                    "d_effective",
                    "the effective decline is a fraction strictly between 0 and 1",
                ));
            }
            row.nominal = Some(finite("nominal", -(1.0 - d).ln() / 365.25)?);
        }
        "instantaneous_decline_rate" => {
            let di = scalar("di", di)?;
            let b = scalar("b", b)?;
            let t = scalar("t", t)?;
            let value = if b < 1.0e-10 {
                di
            } else {
                di / (1.0 + b * di * t)
            };
            row.instantaneous_decline_rate = Some(finite("instantaneous_decline_rate", value)?);
        }
        "time_to_rate" => {
            let qi = positive("qi", qi)?;
            let di = non_negative("di", di)?;
            let b = non_negative("b", b)?;
            let limit = positive("q_limit", q_limit)?;
            match time_to_rate(qi, di, b, limit) {
                Some(value) => row.time_to_rate = Some(finite("time_to_rate", value)?),
                None => row.warnings.push(zero_decline()),
            }
        }
        "eur" => {
            let qi = positive("qi", qi)?;
            let di = non_negative("di", di)?;
            let b = non_negative("b", b)?;
            let limit = positive("q_limit", q_limit)?;
            match time_to_rate(qi, di, b, limit) {
                Some(time) => row.eur = Some(finite("eur", cumulative(qi, di, b, time))?),
                None => row.warnings.push(zero_decline()),
            }
        }
        "remaining_reserves" => {
            let qi = positive("qi", qi)?;
            let di = non_negative("di", di)?;
            let b = non_negative("b", b)?;
            let current = non_negative("t_current", t_current)?;
            let limit = positive("q_limit", q_limit)?;
            match time_to_rate(qi, di, b, limit) {
                Some(time) => {
                    let total = cumulative(qi, di, b, time);
                    row.remaining =
                        Some(finite("remaining", total - cumulative(qi, di, b, current))?);
                }
                None => row.warnings.push(zero_decline()),
            }
        }
        "forecast" => {
            let qi = scalar("qi", qi)?;
            let di = scalar("di", di)?;
            let b = scalar("b", b)?;
            let start = scalar("t_start", t_start)?;
            let end = scalar("t_end", t_end)?;
            let step = positive("t_step", t_step)?;
            if end < start {
                return Err(AzothError::invalid_input(
                    "t_end",
                    "the forecast's end is before its start",
                ));
            }
            let points = ((end - start) / step).ceil() as i64 + 1;
            if points <= 0 {
                return Err(AzothError::invalid_input(
                    "t_end",
                    "the forecast asks for no points",
                ));
            }
            let mut times = Vec::with_capacity(points as usize);
            let mut rates = Vec::with_capacity(points as usize);
            let mut cumulative_values = Vec::with_capacity(points as usize);
            for index in 0..points {
                let mut time = start + index as f64 * step;
                if time > end {
                    time = end;
                }
                times.push(finite("time", time)?);
                rates.push(finite("rate", rate(qi, di, b, time))?);
                cumulative_values.push(finite("cumulative", cumulative(qi, di, b, time))?);
            }
            row.time = Some(times);
            row.rate = Some(rates);
            row.cumulative = Some(cumulative_values);
        }
        "estimate_exponential_decline" => {
            let first = positive("q1", q1)?;
            let first_t = scalar("t1", t1)?;
            let second = positive("q2", q2)?;
            let second_t = scalar("t2", t2)?;
            if second_t <= first_t {
                return Err(AzothError::invalid_input(
                    "t2",
                    "the second point's time must be greater than the first's",
                ));
            }
            if second >= first {
                return Err(AzothError::invalid_input(
                    "q2",
                    "the second rate must be below the first",
                ));
            }
            row.exponential_decline = Some(finite(
                "exponential_decline",
                (first / second).ln() / (second_t - first_t),
            )?);
        }
        "estimate_hyperbolic_parameters" => {
            let (qi, di, b) = estimate_hyperbolic(
                positive("q1", q1)?,
                scalar("t1", t1)?,
                positive("q2", q2)?,
                scalar("t2", t2)?,
                positive("q3", q3)?,
                scalar("t3", t3)?,
            )?;
            row.fit_qi = Some(finite("fit_qi", qi)?);
            row.fit_di = Some(finite("fit_di", di)?);
            row.fit_b = Some(finite("fit_b", b)?);
        }
        "fit_arps" => {
            let (times, rates) = history(history_time, history_rate)?;
            let fit = fit_arps(&times, &rates, 0, times.len() - 1)?;
            set_arps_fit(&mut row, fit)?;
        }
        "fit_arps_window" => {
            let (times, rates) = history(history_time, history_rate)?;
            let start = whole("start_index", start_index, times.len())?;
            let end = whole("end_index", end_index, times.len())?;
            if start >= end {
                return Err(AzothError::invalid_input(
                    "start_index",
                    "the window's first index must be below its last",
                ));
            }
            let fit = fit_arps(&times, &rates, start, end)?;
            set_arps_fit(&mut row, fit)?;
        }
        "eur_from_fit" => {
            let first = positive("fit_qi", qi)?;
            let decline = non_negative("fit_di", di)?;
            let exponent = non_negative("fit_b", b)?;
            let limit = positive("q_limit", q_limit)?;
            match time_to_rate(first, decline, exponent, limit) {
                Some(time) => {
                    row.eur = Some(finite("eur", cumulative(first, decline, exponent, time))?)
                }
                None => row.warnings.push(zero_decline()),
            }
        }
        "duong_time_function" => {
            let intercept = positive("a", a)?;
            let slope = scalar("m", m)?;
            let time = non_negative("t", t)?;
            row.duong_time = Some(finite(
                "duong_time",
                duong_time_function(intercept, slope, time),
            )?);
        }
        "rate_duong" => {
            let rate_at_unit = positive("q1", q1)?;
            let intercept = positive("a", a)?;
            let slope = scalar("m", m)?;
            let time = non_negative("t", t)?;
            row.rate_value = Some(finite(
                "rate_value",
                rate_at_unit * duong_time_function(intercept, slope, time),
            )?);
        }
        "cumulative_duong" => {
            let rate_at_unit = positive("q1", q1)?;
            let intercept = positive("a", a)?;
            let slope = scalar("m", m)?;
            let time = non_negative("t", t)?;
            row.cumulative_value = Some(finite(
                "cumulative_value",
                cumulative_duong(rate_at_unit, intercept, slope, time),
            )?);
        }
        "fit_duong" => {
            let (times, rates) = history(history_time, history_rate)?;
            let fit = fit_duong(&times, &rates)?;
            row.duong_q1 = Some(finite("duong_q1", fit.0)?);
            row.duong_a = Some(finite("duong_a", fit.1)?);
            row.duong_m = Some(finite("duong_m", fit.2)?);
            row.fit_r_squared = Some(finite("fit_r_squared", fit.3)?);
        }
        "summary" => {
            let qi = positive("qi", qi)?;
            let di = non_negative("di", di)?;
            let b = non_negative("b", b)?;
            let limit = positive("q_limit", q_limit)?;
            let time = time_to_rate(qi, di, b, limit);
            let (time_value, eur_value) = match time {
                Some(value) => (value, cumulative(qi, di, b, value)),
                None => {
                    row.warnings.push(zero_decline());
                    (f64::NAN, f64::NAN)
                }
            };
            row.summary_effective = Some(finite("summary_effective", 1.0 - (-di * 365.25).exp())?);
            row.summary_rate_1yr = Some(finite("summary_rate_1yr", rate(qi, di, b, 365.25))?);
            row.summary_rate_5yr = Some(finite("summary_rate_5yr", rate(qi, di, b, 5.0 * 365.25))?);
            row.summary_rate_10yr =
                Some(finite("summary_rate_10yr", rate(qi, di, b, 10.0 * 365.25))?);
            row.summary_cum_1yr = Some(finite("summary_cum_1yr", cumulative(qi, di, b, 365.25))?);
            row.summary_cum_5yr = Some(finite(
                "summary_cum_5yr",
                cumulative(qi, di, b, 5.0 * 365.25),
            )?);
            row.summary_cum_10yr = Some(finite(
                "summary_cum_10yr",
                cumulative(qi, di, b, 10.0 * 365.25),
            )?);
            if time_value.is_finite() {
                row.summary_time_to_limit = Some(finite("summary_time_to_limit", time_value)?);
                row.summary_time_to_limit_years =
                    Some(finite("summary_time_to_limit_years", time_value / 365.25)?);
                row.summary_eur = Some(finite("summary_eur", eur_value)?);
            }
        }
        other => {
            return Err(AzothError::invalid_input(
                "calculation",
                format!("{other:?} is not one of the decline calculations"),
            ));
        }
    }
    Ok(row)
}

fn empty() -> DeclineCurveResult {
    DeclineCurveResult {
        time: None,
        rate: None,
        cumulative: None,
        rate_value: None,
        cumulative_value: None,
        nominal: None,
        effective: None,
        instantaneous_decline_rate: None,
        time_to_rate: None,
        eur: None,
        remaining: None,
        exponential_decline: None,
        fit_qi: None,
        fit_di: None,
        fit_b: None,
        fit_r_squared: None,
        duong_q1: None,
        duong_a: None,
        duong_m: None,
        duong_time: None,
        summary_effective: None,
        summary_rate_1yr: None,
        summary_rate_5yr: None,
        summary_rate_10yr: None,
        summary_cum_1yr: None,
        summary_cum_5yr: None,
        summary_cum_10yr: None,
        summary_time_to_limit: None,
        summary_time_to_limit_years: None,
        summary_eur: None,
        warnings: Vec::new(),
    }
}

fn zero_decline() -> Warning {
    Warning::new(
        WarningCode::OutOfValidRange,
        "the decline is zero, so the economic limit is never reached; the time and the EUR are absent",
    )
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

fn required(name: &str, value: Option<f64>) -> Result<f64> {
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

fn scalar(name: &str, value: Option<f64>) -> Result<f64> {
    required(name, value)
}

fn non_negative(name: &str, value: Option<f64>) -> Result<f64> {
    let value = required(name, value)?;
    if value < 0.0 {
        return Err(AzothError::invalid_input(
            name,
            "the value cannot be negative",
        ));
    }
    Ok(value)
}

fn positive(name: &str, value: Option<f64>) -> Result<f64> {
    let value = required(name, value)?;
    if value <= 0.0 {
        return Err(AzothError::invalid_input(
            name,
            "the value must be positive",
        ));
    }
    Ok(value)
}

fn whole(name: &str, value: Option<f64>, length: usize) -> Result<usize> {
    let value = required(name, value)?;
    if value < 0.0 || value.fract() != 0.0 || value >= length as f64 {
        return Err(AzothError::invalid_input(
            name,
            "the index is a whole number inside the history",
        ));
    }
    Ok(value as usize)
}

fn history(times: Option<&[f64]>, rates: Option<&[f64]>) -> Result<(Vec<f64>, Vec<f64>)> {
    let times = times.ok_or_else(|| {
        AzothError::invalid_input("history_time", "the fit needs the history's times")
    })?;
    let rates = rates.ok_or_else(|| {
        AzothError::invalid_input("history_rate", "the fit needs the history's rates")
    })?;
    if times.len() != rates.len() {
        return Err(AzothError::invalid_input(
            "history_rate",
            "the times and rates are different lengths",
        ));
    }
    if times.len() < 3 {
        return Err(AzothError::invalid_input(
            "history_time",
            "a fit needs at least three points",
        ));
    }
    for index in 0..times.len() {
        if !times[index].is_finite() || times[index] < 0.0 {
            return Err(AzothError::invalid_input(
                "history_time",
                "the history's times must be finite and non-negative",
            ));
        }
        if index > 0 && times[index] <= times[index - 1] {
            return Err(AzothError::invalid_input(
                "history_time",
                "the history's times must be strictly increasing",
            ));
        }
    }
    Ok((times.to_vec(), rates.to_vec()))
}

fn rate(qi: f64, di: f64, b: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return qi;
    }
    if b < 1.0e-10 {
        return qi * (-di * t).exp();
    }
    qi * (1.0 + b * di * t).powf(-1.0 / b)
}

fn cumulative(qi: f64, di: f64, b: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    if di < 1.0e-15 {
        return qi * t;
    }
    if b < 1.0e-10 {
        return (qi / di) * (1.0 - (-di * t).exp());
    }
    if (b - 1.0).abs() < 1.0e-10 {
        return (qi / di) * (1.0 + di * t).ln();
    }
    (qi / ((1.0 - b) * di)) * (1.0 - (1.0 + b * di * t).powf((b - 1.0) / b))
}

fn time_to_rate(qi: f64, di: f64, b: f64, q_limit: f64) -> Option<f64> {
    if q_limit >= qi {
        return Some(0.0);
    }
    if di < 1.0e-15 {
        return None;
    }
    if b < 1.0e-10 {
        return Some(-(q_limit / qi).ln() / di);
    }
    Some(((qi / q_limit).powf(b) - 1.0) / (b * di))
}

fn linearize_arps(t: &[f64], q: &[f64], b: f64) -> Option<(f64, f64)> {
    let n = t.len() as f64;
    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    if b < 1.0e-6 {
        for index in 0..t.len() {
            let y = q[index].ln();
            sx += t[index];
            sy += y;
            sxx += t[index] * t[index];
            sxy += t[index] * y;
        }
        let denom = n * sxx - sx * sx;
        if denom.abs() < 1.0e-30 {
            return None;
        }
        let slope = (n * sxy - sx * sy) / denom;
        let intercept = (sy - slope * sx) / n;
        let di = -slope;
        let qi = intercept.exp();
        if di <= 0.0 || qi <= 0.0 {
            return None;
        }
        return Some((qi, di));
    }
    for index in 0..t.len() {
        let y = q[index].powf(-b);
        sx += t[index];
        sy += y;
        sxx += t[index] * t[index];
        sxy += t[index] * y;
    }
    let denom = n * sxx - sx * sx;
    if denom.abs() < 1.0e-30 {
        return None;
    }
    let slope = (n * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / n;
    if intercept <= 0.0 || slope <= 0.0 {
        return None;
    }
    let qi = intercept.powf(-1.0 / b);
    let di = slope / (intercept * b);
    if qi <= 0.0 || di <= 0.0 {
        return None;
    }
    Some((qi, di))
}

fn fit_arps(t: &[f64], q: &[f64], start: usize, end: usize) -> Result<(f64, f64, f64, f64)> {
    let n = end - start + 1;
    if n < 3 {
        return Err(AzothError::invalid_input(
            "history_time",
            "a fit needs at least three points",
        ));
    }
    let t0 = t[start];
    let mut times = Vec::with_capacity(n);
    let mut rates = Vec::with_capacity(n);
    for index in 0..n {
        times.push(t[start + index] - t0);
        rates.push(q[start + index]);
        if rates[index] <= 0.0 {
            return Err(AzothError::invalid_input(
                "history_rate",
                "every rate in the fit must be positive",
            ));
        }
    }
    let mut best_b: f64 = 0.0;
    let mut best_qi = rates[0];
    let mut best_di = 0.001;
    let mut best_sse = f64::MAX;
    for pass in 0..2 {
        let (low, high) = if pass == 0 {
            (0.0, 1.0)
        } else {
            ((best_b - 0.05).max(0.0), (best_b + 0.05).min(1.0))
        };
        for step in 0..=100 {
            let b_try = low + (high - low) * step as f64 / 100.0;
            let Some((qi, di)) = linearize_arps(&times, &rates, b_try) else {
                continue;
            };
            let mut sse = 0.0;
            for index in 0..n {
                let predicted = rate(qi, di, b_try, times[index]);
                let residual = predicted - rates[index];
                sse += residual * residual;
            }
            if sse < best_sse {
                best_sse = sse;
                best_b = b_try;
                best_qi = qi;
                best_di = di;
            }
        }
    }
    let mean = rates.iter().sum::<f64>() / n as f64;
    let ss_tot: f64 = rates
        .iter()
        .map(|value| (value - mean) * (value - mean))
        .sum();
    let r_squared = if ss_tot > 1.0e-30 {
        1.0 - best_sse / ss_tot
    } else {
        1.0
    };
    Ok((best_qi, best_di, best_b, r_squared))
}

fn set_arps_fit(row: &mut DeclineCurveResult, fit: (f64, f64, f64, f64)) -> Result<()> {
    row.fit_qi = Some(finite("fit_qi", fit.0)?);
    row.fit_di = Some(finite("fit_di", fit.1)?);
    row.fit_b = Some(finite("fit_b", fit.2)?);
    row.fit_r_squared = Some(finite("fit_r_squared", fit.3)?);
    Ok(())
}

fn estimate_hyperbolic(
    q1: f64,
    t1: f64,
    q2: f64,
    t2: f64,
    q3: f64,
    t3: f64,
) -> Result<(f64, f64, f64)> {
    if t3 <= t1 {
        return Err(AzothError::invalid_input(
            "t3",
            "the third point's time must be greater than the first's",
        ));
    }
    let mut di_est = (q1 / q3).ln() / (t3 - t1);
    if di_est <= 0.0 {
        di_est = 0.001;
    }
    let mut best_b = 0.0;
    let mut best_error = f64::MAX;
    for index in 0..=100 {
        let b_try = index as f64 / 100.0;
        let r = (q1 / q3).powf(b_try);
        let d_try = if (r - 1.0).abs() < 1.0e-12 {
            di_est
        } else {
            (r - 1.0) / (b_try * (t3 - t1) + 1.0e-30)
        };
        if d_try <= 0.0 {
            continue;
        }
        let qi_try = q1 * (1.0 + b_try * d_try * t1).powf(1.0 / (b_try + 1.0e-30));
        let predicted = rate(qi_try, d_try, b_try, t2);
        let error = (predicted - q2).abs() / q2;
        if error < best_error {
            best_error = error;
            best_b = b_try;
        }
    }
    let r = (q1 / q3).powf(best_b);
    let mut di_final = if best_b < 1.0e-10 {
        (q1 / q3).ln() / (t3 - t1)
    } else {
        (r - 1.0) / (best_b * (t3 - t1))
    };
    if di_final <= 0.0 {
        di_final = di_est;
    }
    let qi_final = if best_b < 1.0e-10 {
        q1 * (di_final * t1).exp()
    } else {
        q1 * (1.0 + best_b * di_final * t1).powf(1.0 / best_b)
    };
    Ok((qi_final, di_final, best_b))
}

fn duong_time_function(a: f64, m: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return 1.0;
    }
    let exponent = a / (1.0 - m) * (t.powf(1.0 - m) - 1.0);
    t.powf(-m) * exponent.exp()
}

fn cumulative_duong(q1: f64, a: f64, m: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    let exponent = a / (1.0 - m) * (t.powf(1.0 - m) - 1.0);
    q1 / a * exponent.exp()
}

fn fit_duong(t: &[f64], q: &[f64]) -> Result<(f64, f64, f64, f64)> {
    let n = t.len();
    for index in 0..n {
        if t[index] <= 0.0 || q[index] <= 0.0 {
            return Err(AzothError::invalid_input(
                "history_time",
                "the Duong fit needs strictly positive times and rates",
            ));
        }
        if index > 0 && t[index] <= t[index - 1] {
            return Err(AzothError::invalid_input(
                "history_time",
                "the Duong fit needs strictly increasing times",
            ));
        }
    }
    let mut gp = vec![0.0; n];
    gp[0] = q[0] * t[0];
    for index in 1..n {
        gp[index] = gp[index - 1] + 0.5 * (q[index] + q[index - 1]) * (t[index] - t[index - 1]);
    }
    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    for index in 0..n {
        let x = t[index].ln();
        let y = (q[index] / gp[index]).ln();
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
    }
    let count = n as f64;
    let denom = count * sxx - sx * sx;
    if denom.abs() < 1.0e-30 {
        return Err(AzothError::invalid_input(
            "history_time",
            "the Duong regression is singular",
        ));
    }
    let slope = (count * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / count;
    let m = -slope;
    let a = intercept.exp();
    let mut stt = 0.0;
    let mut stq = 0.0;
    for index in 0..n {
        let time_function = duong_time_function(a, m, t[index]);
        stt += time_function * time_function;
        stq += time_function * q[index];
    }
    let q1 = if stt > 1.0e-30 { stq / stt } else { q[0] };
    let mean = q.iter().sum::<f64>() / count;
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for index in 0..n {
        let predicted = q1 * duong_time_function(a, m, t[index]);
        ss_res += (q[index] - predicted) * (q[index] - predicted);
        ss_tot += (q[index] - mean) * (q[index] - mean);
    }
    let r_squared = if ss_tot > 1.0e-30 {
        1.0 - ss_res / ss_tot
    } else {
        1.0
    };
    Ok((q1, a, m, r_squared))
}
