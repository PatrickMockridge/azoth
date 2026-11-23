//! `pvt.fetkovich_aquifer` - the finite, constant-property water aquifer.
//!
//! Spec: `specs/models/pvt/fetkovich_aquifer.toml`. The arithmetic is
//! `pvtsimulation/reservoirproperties/materialbalance/FetkovichAquifer.java` at the pin: each
//! prescribed linear-pressure interval is integrated exactly, with the small-exposure series,
//! the within-interval flow-reversal extrema and the finite-inventory refusal.

use azoth_core::units::{Pressure, Volume};
use azoth_core::{AzothError, Result, Warning};

/// Below this dimensionless exposure the class uses its cancellation-free series.
const SMALL_EXPOSURE_SERIES_LIMIT: f64 = 1.0e-3;

/// One prescribed interval of the march.
#[derive(Debug, Clone, PartialEq)]
pub struct FetkovichInterval {
    /// The interval duration, in s.
    pub duration: f64,
    /// The reservoir pressure at the interval start, in Pa.
    pub reservoir_pressure_start: f64,
    /// The reservoir pressure at the interval end, in Pa.
    pub reservoir_pressure_end: f64,
}

/// One aquifer state as the march answers it.
#[derive(Debug, Clone, PartialEq)]
pub struct FetkovichState {
    /// The elapsed time, in s.
    pub time: f64,
    /// The cumulative water influx, in m3.
    pub cumulative_influx: f64,
    /// The aquifer pressure, in Pa.
    pub pressure: f64,
    /// The influx over this interval, in m3.
    pub interval_influx: f64,
    /// The smallest cumulative influx reached inside this interval, in m3.
    pub interval_minimum_cumulative_influx: f64,
    /// The largest cumulative influx reached inside this interval, in m3.
    pub interval_maximum_cumulative_influx: f64,
}

/// Result of `pvt.fetkovich_aquifer`.
#[derive(Debug, Clone, PartialEq)]
pub struct FetkovichAquiferResult {
    /// The volume-pressure capacitance, m3/Pa; absent exactly when storage is infinite.
    pub capacity: Option<f64>,
    /// The initial state followed by one state per interval.
    pub states: Vec<FetkovichState>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The fixed aquifer parameters.
struct Aquifer {
    initial_water_volume: f64,
    productivity: f64,
    initial_pressure: f64,
    capacity: f64,
}

/// A mutable copy of one state during the march.
#[derive(Clone, Copy)]
struct State {
    time: f64,
    cumulative_influx: f64,
    pressure: f64,
    interval_influx: f64,
    interval_minimum_cumulative_influx: f64,
    interval_maximum_cumulative_influx: f64,
}

/// The finite aquifer over a prescribed interval schedule.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for a negative or NaN water volume, compressibility or
///   productivity, a non-positive initial pressure, an unrepresentable finite capacity, a
///   non-positive interval duration or pressure, an interval that exhausts the finite
///   inventory (naming the interval), or non-finite evidence.
pub fn fetkovich_aquifer(
    initial_water_volume: Option<Volume>,
    total_compressibility: f64,
    productivity: f64,
    initial_pressure: Pressure,
    intervals: &[FetkovichInterval],
) -> Result<FetkovichAquiferResult> {
    let volume = initial_water_volume.map_or(f64::INFINITY, |quantity| quantity.value);
    let aquifer = Aquifer::new(
        volume,
        total_compressibility,
        productivity,
        initial_pressure.value,
    )?;
    let mut state = State {
        time: 0.0,
        cumulative_influx: 0.0,
        pressure: aquifer.initial_pressure,
        interval_influx: 0.0,
        interval_minimum_cumulative_influx: 0.0,
        interval_maximum_cumulative_influx: 0.0,
    };
    let mut states = vec![state.into_state()];
    for (index, interval) in intervals.iter().enumerate() {
        state = aquifer.advance(state, interval).map_err(|error| {
            AzothError::invalid_input(format!("intervals[{index}]"), error.to_string())
        })?;
        states.push(state.into_state());
    }
    Ok(FetkovichAquiferResult {
        capacity: if aquifer.capacity.is_finite() {
            Some(aquifer.capacity)
        } else {
            None
        },
        states,
        warnings: Vec::new(),
    })
}

impl State {
    fn into_state(self) -> FetkovichState {
        FetkovichState {
            time: self.time,
            cumulative_influx: self.cumulative_influx,
            pressure: self.pressure,
            interval_influx: self.interval_influx,
            interval_minimum_cumulative_influx: self.interval_minimum_cumulative_influx,
            interval_maximum_cumulative_influx: self.interval_maximum_cumulative_influx,
        }
    }
}

impl Aquifer {
    fn new(
        initial_water_volume: f64,
        total_compressibility: f64,
        productivity: f64,
        initial_pressure: f64,
    ) -> Result<Self> {
        if initial_water_volume.is_nan() || initial_water_volume < 0.0 {
            return Err(AzothError::invalid_input(
                "initial_water_volume",
                "the initial aquifer water volume must be nonnegative, or absent for infinite storage",
            ));
        }
        for (name, value) in [
            ("total_compressibility", total_compressibility),
            ("productivity", productivity),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(AzothError::invalid_input(
                    name,
                    "the aquifer coefficient must be finite and nonnegative",
                ));
            }
        }
        if !initial_pressure.is_finite() || initial_pressure <= 0.0 {
            return Err(AzothError::invalid_input(
                "initial_pressure",
                "the initial aquifer pressure must be finite and positive",
            ));
        }
        let capacity = if total_compressibility == 0.0 {
            0.0
        } else {
            initial_water_volume * total_compressibility
        };
        if initial_water_volume.is_finite()
            && initial_water_volume > 0.0
            && total_compressibility > 0.0
            && (!capacity.is_finite() || capacity == 0.0)
        {
            return Err(AzothError::invalid_input(
                "initial_water_volume",
                "the finite aquifer storage must be representable in double arithmetic",
            ));
        }
        Ok(Self {
            initial_water_volume,
            productivity,
            initial_pressure,
            capacity,
        })
    }

    fn advance(&self, state: State, interval: &FetkovichInterval) -> Result<State> {
        if !interval.duration.is_finite() || interval.duration <= 0.0 {
            return Err(AzothError::invalid_input(
                "duration",
                "the interval duration must be finite and positive",
            ));
        }
        for (name, value) in [
            (
                "reservoir_pressure_start",
                interval.reservoir_pressure_start,
            ),
            ("reservoir_pressure_end", interval.reservoir_pressure_end),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(AzothError::invalid_input(
                    name,
                    "the reservoir pressure must be finite and positive",
                ));
            }
        }
        let time = state.time + interval.duration;
        let mut influx = 0.0;
        let mut pressure = state.pressure;
        let mut minimum_influx = state.cumulative_influx;
        let mut maximum_influx = state.cumulative_influx;
        if self.productivity > 0.0 {
            if self.capacity == 0.0 {
                pressure = interval.reservoir_pressure_end;
            } else if self.capacity.is_infinite() {
                let mean_reservoir_pressure =
                    0.5 * interval.reservoir_pressure_start + 0.5 * interval.reservoir_pressure_end;
                let pressure_difference = pressure - mean_reservoir_pressure;
                if pressure_difference != 0.0 {
                    influx = self.productivity * interval.duration * pressure_difference;
                }
                let pressure_change =
                    interval.reservoir_pressure_end - interval.reservoir_pressure_start;
                let initial_difference = pressure - interval.reservoir_pressure_start;
                if same_nonzero_sign(pressure_change, initial_difference) {
                    let fraction = initial_difference / pressure_change;
                    if fraction > 0.0 && fraction < 1.0 {
                        let extremum = state.cumulative_influx
                            + self.productivity
                                * interval.duration
                                * (0.5 * initial_difference * fraction);
                        minimum_influx = minimum_influx.min(extremum);
                        maximum_influx = maximum_influx.max(extremum);
                    }
                }
            } else {
                let exposure = self.exposure(interval.duration);
                let pressure_change =
                    interval.reservoir_pressure_end - interval.reservoir_pressure_start;
                let loss = -(-exposure).exp_m1();
                let ramp_weight;
                if exposure < SMALL_EXPOSURE_SERIES_LIMIT {
                    let ramp_per_exposure = 0.5
                        + exposure
                            * (-1.0 / 6.0
                                + exposure
                                    * (1.0 / 24.0 + exposure * (-1.0 / 120.0 + exposure / 720.0)));
                    let decay_per_exposure = 1.0
                        - exposure
                            * (0.5
                                - exposure
                                    * (1.0 / 6.0 - exposure * (1.0 / 24.0 - exposure / 120.0)));
                    influx = self.productivity
                        * interval.duration
                        * ((pressure - interval.reservoir_pressure_start) * decay_per_exposure
                            - pressure_change * ramp_per_exposure);
                    ramp_weight = exposure * ramp_per_exposure;
                } else {
                    ramp_weight = 1.0 - loss / exposure;
                    influx = self.capacity
                        * ((pressure - interval.reservoir_pressure_start) * loss
                            - pressure_change * ramp_weight);
                }
                pressure = (-exposure).exp() * pressure
                    + (loss - ramp_weight) * interval.reservoir_pressure_start
                    + ramp_weight * interval.reservoir_pressure_end;
                let initial_difference = state.pressure - interval.reservoir_pressure_start;
                if same_nonzero_sign(pressure_change, initial_difference) {
                    let fraction = crossing_fraction(exposure, initial_difference, pressure_change);
                    if (0.0..=1.0).contains(&fraction) {
                        let extremum = state.cumulative_influx
                            + self.finite_influx_at_fraction(
                                interval.duration,
                                exposure,
                                initial_difference,
                                pressure_change,
                                fraction,
                            );
                        minimum_influx = minimum_influx.min(extremum);
                        maximum_influx = maximum_influx.max(extremum);
                    }
                }
            }
        }
        let cumulative = state.cumulative_influx + influx;
        minimum_influx = minimum_influx.min(cumulative);
        maximum_influx = maximum_influx.max(cumulative);
        if !time.is_finite()
            || !influx.is_finite()
            || !cumulative.is_finite()
            || !minimum_influx.is_finite()
            || !maximum_influx.is_finite()
            || !pressure.is_finite()
            || pressure <= 0.0
        {
            return Err(AzothError::invalid_input(
                "interval",
                "the aquifer interval produced non-finite evidence or a non-positive pressure",
            ));
        }
        if self.initial_water_volume.is_finite()
            && maximum_influx >= self.initial_water_volume
            && self.initial_water_volume > 0.0
        {
            return Err(AzothError::invalid_input(
                "interval",
                "the aquifer interval exhausts the initial water inventory",
            ));
        }
        Ok(State {
            time,
            cumulative_influx: cumulative,
            pressure,
            interval_influx: influx,
            interval_minimum_cumulative_influx: minimum_influx,
            interval_maximum_cumulative_influx: maximum_influx,
        })
    }

    fn exposure(&self, duration: f64) -> f64 {
        let ratio = self.productivity / self.capacity;
        if ratio.is_finite() {
            return ratio * duration;
        }
        (self.productivity.ln() + duration.ln() - self.capacity.ln()).exp()
    }

    fn finite_influx_at_fraction(
        &self,
        duration: f64,
        exposure: f64,
        initial_difference: f64,
        pressure_change: f64,
        fraction: f64,
    ) -> f64 {
        if exposure.is_infinite() {
            return self.capacity * (initial_difference - pressure_change * fraction);
        }
        let fractional_exposure = exposure * fraction;
        if fractional_exposure < SMALL_EXPOSURE_SERIES_LIMIT {
            let ramp = 0.5
                + fractional_exposure
                    * (-1.0 / 6.0
                        + fractional_exposure
                            * (1.0 / 24.0
                                + fractional_exposure
                                    * (-1.0 / 120.0 + fractional_exposure / 720.0)));
            let decay = 1.0
                - fractional_exposure
                    * (0.5
                        - fractional_exposure
                            * (1.0 / 6.0
                                - fractional_exposure
                                    * (1.0 / 24.0 - fractional_exposure / 120.0)));
            return self.productivity
                * duration
                * fraction
                * (initial_difference * decay - pressure_change * fraction * ramp);
        }
        let loss = -(-fractional_exposure).exp_m1();
        let ramp_weight = 1.0 - loss / fractional_exposure;
        self.capacity * (initial_difference * loss - pressure_change * fraction * ramp_weight)
    }
}

/// Whether two values are strictly the same nonzero sign.
fn same_nonzero_sign(first: f64, second: f64) -> bool {
    (first > 0.0 && second > 0.0) || (first < 0.0 && second < 0.0)
}

/// The possible zero-flow time as a fraction of a prescribed linear-pressure interval.
fn crossing_fraction(exposure: f64, initial_difference: f64, pressure_change: f64) -> f64 {
    if exposure == 0.0 {
        return initial_difference / pressure_change;
    }
    if exposure.is_infinite() {
        return 0.0;
    }
    let log_ratio = exposure.ln() + initial_difference.abs().ln() - pressure_change.abs().ln();
    if log_ratio < -40.0 {
        return initial_difference / pressure_change;
    }
    let log_one_plus_ratio = if log_ratio > 40.0 {
        log_ratio
    } else {
        log_ratio.exp().ln_1p()
    };
    log_one_plus_ratio / exposure
}

#[cfg(test)]
mod tests {
    use super::*;
    use azoth_core::units::{cubic_meters, pascals};

    fn interval(duration: f64, start: f64, end: f64) -> FetkovichInterval {
        FetkovichInterval {
            duration,
            reservoir_pressure_start: start,
            reservoir_pressure_end: end,
        }
    }

    #[test]
    fn the_constant_pressure_interval_is_the_exponential_identity() {
        let result = fetkovich_aquifer(
            Some(cubic_meters(1.0e6)),
            1.0e-9,
            2.0e-7,
            pascals(3.0e7),
            &[interval(12000.0, 2.5e7, 2.5e7)],
        )
        .unwrap();
        let state = &result.states[1];
        assert!((state.cumulative_influx / 4546.410233552938 - 1.0).abs() < 1.0e-12);
        assert!((state.pressure / 2.5453589766447064e7 - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn the_interior_extrema_are_sampled() {
        let result = fetkovich_aquifer(
            Some(cubic_meters(1.0e6)),
            1.0e-9,
            2.0e-7,
            pascals(3.0e7),
            &[interval(10000.0, 2.5e7, 4.0e7)],
        )
        .unwrap();
        let state = &result.states[1];
        assert!(
            (state.interval_maximum_cumulative_influx / 1168.8078217550703 - 1.0).abs() < 1.0e-10
        );
        assert!((state.cumulative_influx / -4191.69104045766 - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn infinite_storage_has_no_capacity_and_a_linear_rate() {
        let result = fetkovich_aquifer(
            None,
            1.0e-9,
            2.0e-7,
            pascals(3.0e7),
            &[interval(1000.0, 3.0e7, 1.0e7)],
        )
        .unwrap();
        assert!(result.capacity.is_none());
        assert!((result.states[1].cumulative_influx - 2000.0).abs() < 1.0e-9);
        assert!(result.states[1].pressure == 3.0e7);
    }

    #[test]
    fn an_exhausting_interval_is_refused() {
        let error = fetkovich_aquifer(
            Some(cubic_meters(1.0)),
            1.0e-6,
            1.0e-5,
            pascals(3.0e7),
            &[interval(100.0, 2.5e7, 2.5e7)],
        )
        .unwrap_err();
        assert!(error.to_string().contains("intervals[0]"));
    }
}
