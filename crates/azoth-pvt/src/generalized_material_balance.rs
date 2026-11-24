//! `pvt.generalized_material_balance` - the generalized oil and gas material balance.
//!
//! Spec: `specs/models/pvt/generalized_material_balance.toml`. The arithmetic is
//! `pvtsimulation/reservoirproperties/materialbalance/GeneralizedMaterialBalance.java` at the
//! pin: the four fits over a column-scaled least squares with the class's condition-number
//! gate and its inventory checks.

use azoth_core::units::Pressure;
use azoth_core::warning::WarningCode;
use azoth_core::{AzothError, Result, Warning};
use azoth_reactions::linalg::{condition_number_2, solve_least_squares};

/// The largest permitted condition number of the scaled predictor matrix.
const MAX_CONDITION_NUMBER: f64 = 1.0e8;
/// The phase-split determinant below which the record is refused.
const MIN_SPLIT_DETERMINANT: f64 = 1.0e-10;

/// One PVT state of the balance history.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialBalanceObservation {
    /// The average reservoir pressure, in Pa.
    pub pressure: f64,
    /// The solution gas-oil ratio, Sm3/Sm3.
    pub rs: f64,
    /// The oil formation volume factor, reservoir m3/Sm3.
    pub bo: f64,
    /// The gas formation volume factor, reservoir m3/Sm3.
    pub bg: f64,
    /// The water formation volume factor, reservoir m3/Sm3.
    pub bw: f64,
    /// The vaporized oil ratio, Sm3/Sm3.
    pub rv: f64,
    /// Cumulative stock-tank oil produced minus injected, Sm3.
    pub net_oil_produced: f64,
    /// Cumulative standard gas produced minus injected, Sm3.
    pub net_gas_produced: f64,
    /// Cumulative standard water produced minus injected, Sm3.
    pub net_water_produced: f64,
    /// Cumulative aquifer influx, reservoir m3.
    pub water_influx: f64,
    /// Rock/connate-water expansion per initial hydrocarbon reservoir volume.
    pub expansion_fraction: f64,
}

/// Result of `pvt.generalized_material_balance`.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralizedMaterialBalanceResult {
    /// The initial free-oil surface component, Sm3.
    pub free_oil: f64,
    /// The initial free-gas surface component, Sm3.
    pub free_gas: f64,
    /// The total original oil in place, Sm3.
    pub ooip: f64,
    /// The total original gas in place, Sm3.
    pub ogip: f64,
    /// The initial gas-cap volume ratio; absent when the fit has no free oil.
    pub gas_cap_ratio: Option<f64>,
    /// The observed-minus-fitted volume residuals, reservoir m3.
    pub residuals: Vec<f64>,
    /// The root-mean-square volume residual, reservoir m3.
    pub rms_residual: f64,
    /// The scaled predictor matrix's 2-norm condition number.
    pub condition_number: f64,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// One PVT record, with the coefficients its phase split defines.
#[derive(Clone, Copy)]
struct Record {
    pressure: f64,
    rs: f64,
    bo: f64,
    bg: f64,
    bw: f64,
    rv: f64,
}

/// One observation's expansion and withdrawal columns.
#[derive(Clone, Copy)]
struct Observation {
    eo: f64,
    eg: f64,
    withdrawal: f64,
    net_oil: f64,
    net_gas: f64,
}

/// The generalized balance over a stated initial PVT and observation history.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown fit, a negative or non-finite production,
///   a non-positive pressure or formation volume factor, a near-critical split, too few
///   observations, a zero withdrawal or expansion signal, an ill-conditioned or singular
///   scaled solve, unphysical fitted volumes, or a history whose production exceeds the
///   fitted inventory at an observation.
#[allow(clippy::too_many_arguments)]
pub fn generalized_material_balance(
    fit: &str,
    initial_pressure: Pressure,
    initial_rs: f64,
    initial_bo: f64,
    initial_bg: f64,
    initial_bw: f64,
    initial_rv: f64,
    observations: &[MaterialBalanceObservation],
    gas_cap_ratio: Option<f64>,
) -> Result<GeneralizedMaterialBalanceResult> {
    let initial = Record {
        pressure: initial_pressure.value,
        rs: initial_rs,
        bo: initial_bo,
        bg: initial_bg,
        bw: initial_bw,
        rv: initial_rv,
    };
    coefficients(initial)?;
    let (mode, gas_per_oil) = match fit {
        "initially_oil" => (0, 0.0),
        "initially_gas" => (1, 0.0),
        "known_gas_cap" => {
            let ratio = gas_cap_ratio.ok_or_else(|| {
                AzothError::invalid_input(
                    "gas_cap_ratio",
                    "the known-gas-cap fit needs the stated ratio",
                )
            })?;
            if !ratio.is_finite() || ratio < 0.0 {
                return Err(AzothError::invalid_input(
                    "gas_cap_ratio",
                    "the gas-cap ratio must be finite and nonnegative",
                ));
            }
            (0, ratio * initial.bo / initial.bg)
        }
        "oil_and_gas" => (2, 0.0),
        other => {
            return Err(AzothError::invalid_input(
                "fit",
                format!("{other:?} is not one of the generalized material-balance fits"),
            ));
        }
    };
    let mut history = Vec::with_capacity(observations.len());
    for (index, observation) in observations.iter().enumerate() {
        history.push(observe(initial, observation).map_err(|error| {
            AzothError::invalid_input(format!("observations[{index}]"), error.to_string())
        })?);
    }
    fit_balance(initial, &history, mode, gas_per_oil)
}

/// The phase-split coefficients of one record.
fn coefficients(record: Record) -> Result<(f64, f64)> {
    if !record.pressure.is_finite()
        || record.pressure <= 0.0
        || !record.bo.is_finite()
        || record.bo <= 0.0
        || !record.bg.is_finite()
        || record.bg <= 0.0
        || !record.bw.is_finite()
        || record.bw <= 0.0
        || !record.rs.is_finite()
        || record.rs < 0.0
        || !record.rv.is_finite()
        || record.rv < 0.0
    {
        return Err(AzothError::invalid_input(
            "pvt",
            "the PVT needs a positive finite pressure and formation volume factors and \
             nonnegative finite Rs and Rv",
        ));
    }
    let determinant = 1.0 - record.rs * record.rv;
    if determinant <= MIN_SPLIT_DETERMINANT {
        return Err(AzothError::invalid_input(
            "rs_rv_split",
            "Rs*Rv must stay below one by more than 1e-10; the split is near critical or invalid",
        ));
    }
    let a = (record.bo - record.rs * record.bg) / determinant;
    let b = (record.bg - record.rv * record.bo) / determinant;
    if !a.is_finite() || !b.is_finite() {
        return Err(AzothError::invalid_input(
            "pvt",
            "an effective formation volume factor is not finite",
        ));
    }
    Ok((a, b))
}

/// One observation's expansion and withdrawal columns.
fn observe(initial: Record, observation: &MaterialBalanceObservation) -> Result<Observation> {
    for (name, value) in [
        ("net_oil_produced", observation.net_oil_produced),
        ("net_gas_produced", observation.net_gas_produced),
        ("net_water_produced", observation.net_water_produced),
        ("water_influx", observation.water_influx),
        ("expansion_fraction", observation.expansion_fraction),
    ] {
        if !value.is_finite() {
            return Err(AzothError::invalid_input(
                name,
                "the observation must be finite",
            ));
        }
    }
    let current = Record {
        pressure: observation.pressure,
        rs: observation.rs,
        bo: observation.bo,
        bg: observation.bg,
        bw: observation.bw,
        rv: observation.rv,
    };
    let (a, b) = coefficients(current)?;
    let eo = a + initial.rs * b - initial.bo + initial.bo * observation.expansion_fraction;
    let eg = initial.rv * a + b - initial.bg + initial.bg * observation.expansion_fraction;
    let withdrawal = a * observation.net_oil_produced
        + b * observation.net_gas_produced
        + current.bw * observation.net_water_produced
        - observation.water_influx;
    for (name, value) in [
        ("oil_expansion", eo),
        ("gas_expansion", eg),
        ("withdrawal", withdrawal),
    ] {
        if !value.is_finite() {
            return Err(AzothError::invalid_input(
                name,
                "an observation column is not finite",
            ));
        }
    }
    Ok(Observation {
        eo,
        eg,
        withdrawal,
        net_oil: observation.net_oil_produced,
        net_gas: observation.net_gas_produced,
    })
}

/// The scaled least-squares fit.
fn fit_balance(
    initial: Record,
    history: &[Observation],
    mode: usize,
    gas_per_oil: f64,
) -> Result<GeneralizedMaterialBalanceResult> {
    let columns = if mode == 2 { 2 } else { 1 };
    let count = history.len();
    if count < columns.max(2) {
        return Err(AzothError::invalid_input(
            "observations",
            "the fit needs at least two non-initial observations",
        ));
    }
    let mut matrix = vec![vec![0.0; columns]; count];
    let mut response = vec![0.0; count];
    let mut scale = vec![0.0_f64; columns];
    let mut response_scale: f64 = 0.0;
    for (index, row) in history.iter().enumerate() {
        matrix[index][0] = if mode == 1 {
            row.eg
        } else {
            row.eo + gas_per_oil * row.eg
        };
        if mode == 2 {
            matrix[index][1] = row.eg;
        }
        for column in 0..columns {
            if !matrix[index][column].is_finite() {
                return Err(AzothError::invalid_input(
                    "observations",
                    "an expansion predictor is not finite",
                ));
            }
            scale[column] = scale[column].max(matrix[index][column].abs());
        }
        response[index] = row.withdrawal;
        response_scale = response_scale.max(response[index].abs());
    }
    if response_scale == 0.0 {
        return Err(AzothError::invalid_input(
            "observations",
            "the history has no net withdrawal signal",
        ));
    }
    for column in 0..columns {
        if scale[column] == 0.0 {
            return Err(AzothError::invalid_input(
                "observations",
                "the history has no expansion signal for a fitted phase",
            ));
        }
        for row in &mut matrix {
            row[column] /= scale[column];
        }
    }
    for value in &mut response {
        *value /= response_scale;
    }
    let condition = condition_number_2(&matrix).map_err(|error| {
        AzothError::invalid_input("observations", format!("the scaled matrix: {error}"))
    })?;
    if !condition.is_finite() || condition > MAX_CONDITION_NUMBER {
        return Err(AzothError::invalid_input(
            "observations",
            format!(
                "the history cannot identify independent oil and gas volumes; the scaled \
                 condition number is {condition}"
            ),
        ));
    }
    let fitted = solve_least_squares(&matrix, &response).map_err(|error| {
        AzothError::invalid_input("observations", format!("the scaled least squares: {error}"))
    })?;
    let oil = if mode == 1 {
        0.0
    } else {
        fitted[0] * response_scale / scale[0]
    };
    let gas = if mode == 1 {
        fitted[0] * response_scale / scale[0]
    } else if mode == 2 {
        fitted[1] * response_scale / scale[1]
    } else {
        gas_per_oil * oil
    };
    if !oil.is_finite() || !gas.is_finite() {
        return Err(AzothError::invalid_input(
            "observations",
            "a fitted phase volume is not finite",
        ));
    }
    if oil < 0.0 || gas < 0.0 || oil + gas == 0.0 {
        return Err(AzothError::invalid_input(
            "observations",
            "the fit answered unphysical volumes; check the history, PVT and drive assumptions",
        ));
    }
    let ooip = oil + initial.rv * gas;
    let ogip = gas + initial.rs * oil;
    if !ooip.is_finite() || !ogip.is_finite() {
        return Err(AzothError::invalid_input(
            "observations",
            "a total in-place volume is not finite",
        ));
    }
    let mut residuals = vec![0.0; count];
    for (index, row) in history.iter().enumerate() {
        if row.net_oil > ooip + 1.0e-10 * ooip.max(1.0)
            || row.net_gas > ogip + 1.0e-10 * ogip.max(1.0)
        {
            return Err(AzothError::invalid_input(
                format!("observations[{index}]"),
                "the net production exceeds the fitted component inventory at this observation",
            ));
        }
        residuals[index] = row.withdrawal - oil * row.eo - gas * row.eg;
        if !residuals[index].is_finite() {
            return Err(AzothError::invalid_input(
                format!("observations[{index}]"),
                "the balance residual is not finite",
            ));
        }
    }
    let norm = residuals
        .iter()
        .fold(0.0_f64, |acc, value| acc.hypot(*value));
    let rms_residual = norm / (count as f64).sqrt();
    let mut warnings = Vec::new();
    let gas_cap_ratio = if oil > 0.0 {
        Some(gas * initial.bg / (oil * initial.bo))
    } else {
        warnings.push(Warning::new(
            WarningCode::OutOfValidRange,
            "the fit has no free oil, so the gas-cap ratio is absent rather than infinite",
        ));
        None
    };
    Ok(GeneralizedMaterialBalanceResult {
        free_oil: oil,
        free_gas: gas,
        ooip,
        ogip,
        gas_cap_ratio,
        residuals,
        rms_residual,
        condition_number: condition,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use azoth_core::units::pascals;

    fn record(pressure: f64, rs: f64, bo: f64, bg: f64, rv: f64) -> MaterialBalanceObservation {
        MaterialBalanceObservation {
            pressure,
            rs,
            bo,
            bg,
            bw: 1.02,
            rv,
            net_oil_produced: 0.0,
            net_gas_produced: 0.0,
            net_water_produced: 0.0,
            water_influx: 0.0,
            expansion_fraction: 0.0,
        }
    }

    fn history(oil: f64, gas: f64, support: bool) -> Vec<MaterialBalanceObservation> {
        let initial = Record {
            pressure: 3.0e7,
            rs: 100.0,
            bo: 1.6,
            bg: 0.004,
            bw: 1.02,
            rv: 0.0002,
        };
        let records = [
            (260.0, 80.0, 1.5, 0.005, 0.00016),
            (210.0, 60.0, 1.4, 0.008, 0.00012),
            (160.0, 40.0, 1.3, 0.012, 0.00008),
            (110.0, 20.0, 1.2, 0.020, 0.00004),
        ];
        let initial_volume = oil * initial.bo + gas * initial.bg;
        let initial_oil = oil + initial.rv * gas;
        let initial_gas = gas + initial.rs * oil;
        let mut out = Vec::new();
        for (index, (pressure, rs, bo, bg, rv)) in records.iter().enumerate() {
            let mut current = record(*pressure * 1.0e5, *rs, *bo, *bg, *rv);
            let fraction = if support {
                0.001 * (index as f64 + 1.0)
            } else {
                0.0
            };
            let influx = if support {
                0.002 * initial_volume * (index as f64 + 1.0)
            } else {
                0.0
            };
            let net_water = if support {
                -0.001 * initial_volume * (index as f64 + 1.0)
            } else {
                0.0
            };
            let remaining = initial_volume * (1.0 - fraction) - influx + net_water * current.bw;
            let oil_volume = remaining
                * if oil == 0.0 {
                    0.01 * (index as f64 + 1.0)
                } else {
                    0.58 - 0.08 * index as f64
                };
            let gas_volume = remaining - oil_volume;
            let free_oil = oil_volume / current.bo;
            let free_gas = gas_volume / current.bg;
            current.net_oil_produced = initial_oil - (free_oil + current.rv * free_gas);
            current.net_gas_produced = initial_gas - (free_gas + current.rs * free_oil);
            current.net_water_produced = net_water;
            current.water_influx = influx;
            current.expansion_fraction = fraction;
            out.push(current);
        }
        out
    }

    #[test]
    fn the_oil_and_gas_fit_recovers_the_history() {
        let result = generalized_material_balance(
            "oil_and_gas",
            pascals(3.0e7),
            100.0,
            1.6,
            0.004,
            1.02,
            0.0002,
            &history(1.0e7, 2.0e9, false),
            None,
        )
        .unwrap();
        assert!((result.free_oil / 1.0e7 - 1.0).abs() < 1.0e-9);
        assert!((result.free_gas / 2.0e9 - 1.0).abs() < 1.0e-9);
        assert!((result.ooip / 1.04e7 - 1.0).abs() < 1.0e-9);
        assert!((result.ogip / 3.0e9 - 1.0).abs() < 1.0e-9);
    }

    #[test]
    fn a_collinear_history_is_refused() {
        let mut first = record(2.0e7, 0.0, 1.1, 0.02, 0.0);
        first.net_oil_produced = 1.0e5;
        first.net_gas_produced = 1.0e6;
        let mut second = record(1.0e7, 0.0, 1.2, 0.03, 0.0);
        second.net_oil_produced = 2.0e5;
        second.net_gas_produced = 2.0e6;
        let error = generalized_material_balance(
            "oil_and_gas",
            pascals(3.0e7),
            0.0,
            1.0,
            0.01,
            1.02,
            0.0,
            &[first, second],
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("condition"));
    }
}
