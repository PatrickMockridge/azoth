//! `process.safety_instrumented_function` - the SIF's IEC 61508 arithmetic.
//!
//! Spec: `specs/models/process/safety_instrumented_function.toml`. The arithmetic is
//! `process/safety/risk/sis/SafetyInstrumentedFunction.java` at the pin: the RRF, the
//! required PFD and SIL, the three architecture PFDs and the spurious-trip availability.

use azoth_core::{AzothError, Result, Warning, WarningCode, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;

/// The PFD band's lower bound per SIL, indexed by the SIL.
const MIN_PFD: [f64; 5] = [0.0, 0.01, 0.001, 0.0001, 0.00001];
/// The PFD band's upper bound per SIL, indexed by the SIL.
const MAX_PFD: [f64; 5] = [1.0, 0.1, 0.01, 0.001, 0.0001];

/// Result of `process.safety_instrumented_function`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SafetyInstrumentedFunctionResult {
    /// `1 / PFD`; absent at a PFD of zero.
    pub risk_reduction_factor: Option<f64>,
    /// `unmitigated_frequency PFD`.
    pub mitigated_frequency_per_year: f64,
    /// `target_frequency / unmitigated_frequency`, or 1.0 at a zero unmitigated frequency.
    pub required_pfd: f64,
    /// The SIL the required PFD maps to.
    pub required_sil: f64,
    /// The largest PFD the claimed SIL allows.
    pub max_pfd_for_claimed_sil: f64,
    /// The proof-test interval in years.
    pub proof_test_interval_years: f64,
    /// Whether the PFD lies inside the claimed SIL's band.
    pub pfd_within_claimed_sil: bool,
    /// `lambda test_interval / 2`; absent without a failure rate.
    pub pfd_1oo1: Option<f64>,
    /// `(lambda test_interval)^2 / 3`; absent without a failure rate.
    pub pfd_1oo2: Option<f64>,
    /// `(lambda test_interval)^2`; absent without a failure rate.
    pub pfd_2oo3: Option<f64>,
    /// `base (1 - rate mttr / 8760)`; absent without a base availability or an MTTR.
    pub availability_with_spurious_trips: Option<f64>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn require_finite(name: &str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name.to_string(),
            "a non-finite value is not a number this calculation publishes",
        ))
    }
}

/// The class's `getRequiredSil`: the minimum SIL a target PFD asks for.
pub(crate) fn required_sil(target_pfd: f64) -> usize {
    if target_pfd >= 0.01 {
        1
    } else if target_pfd >= 0.001 {
        2
    } else if target_pfd >= 0.0001 {
        3
    } else {
        4
    }
}

/// The class's `getMaxPfdForSil`, which answers 1.0 for a SIL it does not know.
fn max_pfd_for_sil(sil: usize) -> f64 {
    match sil {
        1 => 0.1,
        2 => 0.01,
        3 => 0.001,
        4 => 0.0001,
        _ => 1.0,
    }
}

/// The RRF, the required PFD and SIL, the architecture PFDs and the spurious-trip
/// availability.
///
/// # Errors
/// `InvalidInput` for a PFD outside `[0, 1]`, a claimed SIL outside 1..4 or not whole, a
/// negative frequency, interval, failure rate, spurious rate or repair time, a base
/// availability outside `[0, 1]`, or a non-finite value.
#[allow(clippy::too_many_arguments)]
pub fn safety_instrumented_function(
    pfd_average: f64,
    claimed_sil: f64,
    unmitigated_frequency_per_year: f64,
    target_frequency_per_year: f64,
    test_interval_hours: f64,
    lambda_du_per_hour: Option<f64>,
    spurious_trip_rate_per_year: Option<f64>,
    mttr_hours: Option<f64>,
    base_availability: Option<f64>,
) -> Result<SafetyInstrumentedFunctionResult> {
    let spec = &model_gen::SAFETY_INSTRUMENTED_FUNCTION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "pfd_average" => Some(pfd_average),
            "claimed_sil" => Some(claimed_sil),
            "unmitigated_frequency_per_year" => Some(unmitigated_frequency_per_year),
            "target_frequency_per_year" => Some(target_frequency_per_year),
            "test_interval_hours" => Some(test_interval_hours),
            "lambda_du_per_hour" => lambda_du_per_hour,
            "spurious_trip_rate_per_year" => spurious_trip_rate_per_year,
            "mttr_hours" => mttr_hours,
            "base_availability" => base_availability,
            _ => None,
        },
        &mut warnings,
    )?;

    require_finite("pfd_average", pfd_average)?;
    require_finite("claimed_sil", claimed_sil)?;
    require_finite(
        "unmitigated_frequency_per_year",
        unmitigated_frequency_per_year,
    )?;
    require_finite("target_frequency_per_year", target_frequency_per_year)?;
    require_finite("test_interval_hours", test_interval_hours)?;
    if claimed_sil.fract() != 0.0 || !(1.0..=4.0).contains(&claimed_sil) {
        return Err(AzothError::invalid_input(
            "claimed_sil",
            format!("a claimed SIL is a whole number in [1, 4], and {claimed_sil} is not"),
        ));
    }
    let sil = claimed_sil as usize;
    if let Some(value) = lambda_du_per_hour {
        require_finite("lambda_du_per_hour", value)?;
    }
    if let Some(value) = spurious_trip_rate_per_year {
        require_finite("spurious_trip_rate_per_year", value)?;
    }
    if let Some(value) = mttr_hours {
        require_finite("mttr_hours", value)?;
    }
    if let Some(value) = base_availability {
        require_finite("base_availability", value)?;
    }

    let risk_reduction_factor = if pfd_average > 0.0 {
        Some(1.0 / pfd_average)
    } else {
        None
    };
    let mitigated_frequency_per_year = unmitigated_frequency_per_year * pfd_average;
    let required_pfd = if unmitigated_frequency_per_year <= 0.0 {
        1.0
    } else {
        target_frequency_per_year / unmitigated_frequency_per_year
    };
    let required_sil = required_sil(required_pfd) as f64;
    let max_pfd = max_pfd_for_sil(sil);
    let proof_test_interval_years = test_interval_hours / 8760.0;
    let pfd_within_claimed_sil = pfd_average >= MIN_PFD[sil] && pfd_average <= MAX_PFD[sil];
    if !pfd_within_claimed_sil {
        warnings.push(Warning::for_field(
            WarningCode::OutOfValidRange,
            "pfd_average",
            format!(
                "a PFD of {pfd_average:.2e} lies outside SIL {sil}'s band \
                 [{:.2e}, {:.2e}]; the class prints the same warning and publishes the value",
                MIN_PFD[sil], MAX_PFD[sil]
            ),
        ));
    }

    let (pfd_1oo1, pfd_1oo2, pfd_2oo3) = match lambda_du_per_hour {
        Some(lambda) => {
            let lambda_ti = lambda * test_interval_hours;
            (
                Some(lambda_ti / 2.0),
                Some(lambda_ti * lambda_ti / 3.0),
                Some(lambda_ti * lambda_ti),
            )
        }
        None => (None, None, None),
    };
    let availability_with_spurious_trips = match (base_availability, mttr_hours) {
        (Some(base), Some(mttr)) => {
            let rate = spurious_trip_rate_per_year.unwrap_or(0.0);
            Some(if rate > 0.0 {
                base * (1.0 - rate * mttr / 8760.0)
            } else {
                base
            })
        }
        _ => None,
    };

    Ok(SafetyInstrumentedFunctionResult {
        risk_reduction_factor,
        mitigated_frequency_per_year,
        required_pfd,
        required_sil,
        max_pfd_for_claimed_sil: max_pfd,
        proof_test_interval_years,
        pfd_within_claimed_sil,
        pfd_1oo1,
        pfd_1oo2,
        pfd_2oo3,
        availability_with_spurious_trips,
        warnings,
    })
}
