"""``pvt.fetkovich_aquifer`` - the finite, constant-property water aquifer.

The Python twin of ``crates/azoth-pvt/src/fetkovich_aquifer.rs``: each prescribed
linear-pressure interval is integrated exactly, with the small-exposure series, the
within-interval flow-reversal extrema and the finite-inventory refusal.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import FetkovichAquiferResult, FetkovichState
from azoth.core.units import input_to_si, quantity, to_si

MODEL_ID = "pvt.fetkovich_aquifer"

_SMALL_EXPOSURE_SERIES_LIMIT = 1.0e-3


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


class _Aquifer:
    """The fixed aquifer parameters."""

    __slots__ = (
        "capacity",
        "initial_pressure",
        "initial_water_volume",
        "productivity",
        "total_compressibility",
    )

    def __init__(
        self,
        initial_water_volume: float,
        total_compressibility: float,
        productivity: float,
        initial_pressure: float,
    ) -> None:
        if math.isnan(initial_water_volume) or initial_water_volume < 0.0:
            raise InvalidInputError(
                "initial_water_volume",
                "the initial aquifer water volume must be nonnegative, or absent for infinite "
                "storage",
            )
        for name, value in (
            ("total_compressibility", total_compressibility),
            ("productivity", productivity),
        ):
            if not math.isfinite(value) or value < 0.0:
                raise InvalidInputError(
                    name, "the aquifer coefficient must be finite and nonnegative"
                )
        if not math.isfinite(initial_pressure) or initial_pressure <= 0.0:
            raise InvalidInputError(
                "initial_pressure", "the initial aquifer pressure must be finite and positive"
            )
        capacity = (
            0.0 if total_compressibility == 0.0 else initial_water_volume * total_compressibility
        )
        if (
            math.isfinite(initial_water_volume)
            and initial_water_volume > 0.0
            and total_compressibility > 0.0
            and (not math.isfinite(capacity) or capacity == 0.0)
        ):
            raise InvalidInputError(
                "initial_water_volume",
                "the finite aquifer storage must be representable in double arithmetic",
            )
        self.initial_water_volume = initial_water_volume
        self.total_compressibility = total_compressibility
        self.productivity = productivity
        self.initial_pressure = initial_pressure
        self.capacity = capacity

    def advance(
        self,
        state: tuple[float, float, float, float, float, float],
        duration: float,
        start: float,
        end: float,
    ) -> tuple[float, float, float, float, float, float]:
        if not math.isfinite(duration) or duration <= 0.0:
            raise InvalidInputError("duration", "the interval duration must be finite and positive")
        for name, value in (
            ("reservoir_pressure_start", start),
            ("reservoir_pressure_end", end),
        ):
            if not math.isfinite(value) or value <= 0.0:
                raise InvalidInputError(name, "the reservoir pressure must be finite and positive")
        (
            state_time,
            state_cumulative,
            state_pressure,
            _,
            _,
            _,
        ) = state
        time = state_time + duration
        influx = 0.0
        pressure = state_pressure
        minimum_influx = state_cumulative
        maximum_influx = state_cumulative
        if self.productivity > 0.0:
            if self.capacity == 0.0:
                pressure = end
            elif math.isinf(self.capacity):
                mean_reservoir_pressure = 0.5 * start + 0.5 * end
                pressure_difference = pressure - mean_reservoir_pressure
                if pressure_difference != 0.0:
                    influx = self.productivity * duration * pressure_difference
                pressure_change = end - start
                initial_difference = pressure - start
                if _same_nonzero_sign(pressure_change, initial_difference):
                    fraction = initial_difference / pressure_change
                    if 0.0 < fraction < 1.0:
                        extremum = state_cumulative + self.productivity * duration * (
                            0.5 * initial_difference * fraction
                        )
                        minimum_influx = min(minimum_influx, extremum)
                        maximum_influx = max(maximum_influx, extremum)
            else:
                exposure = self.exposure(duration)
                pressure_change = end - start
                loss = -math.expm1(-exposure)
                if exposure < _SMALL_EXPOSURE_SERIES_LIMIT:
                    ramp_per_exposure = 0.5 + exposure * (
                        -1.0 / 6.0
                        + exposure * (1.0 / 24.0 + exposure * (-1.0 / 120.0 + exposure / 720.0))
                    )
                    decay_per_exposure = 1.0 - exposure * (
                        0.5 - exposure * (1.0 / 6.0 - exposure * (1.0 / 24.0 - exposure / 120.0))
                    )
                    influx = (
                        self.productivity
                        * duration
                        * (
                            (pressure - start) * decay_per_exposure
                            - pressure_change * ramp_per_exposure
                        )
                    )
                    ramp_weight = exposure * ramp_per_exposure
                else:
                    ramp_weight = 1.0 - loss / exposure
                    influx = self.capacity * (
                        (pressure - start) * loss - pressure_change * ramp_weight
                    )
                pressure = (
                    math.exp(-exposure) * pressure
                    + (loss - ramp_weight) * start
                    + ramp_weight * end
                )
                initial_difference = state_pressure - start
                if _same_nonzero_sign(pressure_change, initial_difference):
                    fraction = _crossing_fraction(exposure, initial_difference, pressure_change)
                    if 0.0 <= fraction <= 1.0:
                        extremum = state_cumulative + self.finite_influx_at_fraction(
                            duration, exposure, initial_difference, pressure_change, fraction
                        )
                        minimum_influx = min(minimum_influx, extremum)
                        maximum_influx = max(maximum_influx, extremum)
        cumulative = state_cumulative + influx
        minimum_influx = min(minimum_influx, cumulative)
        maximum_influx = max(maximum_influx, cumulative)
        if (
            not math.isfinite(time)
            or not math.isfinite(influx)
            or not math.isfinite(cumulative)
            or not math.isfinite(minimum_influx)
            or not math.isfinite(maximum_influx)
            or not math.isfinite(pressure)
            or pressure <= 0.0
        ):
            raise InvalidInputError(
                "interval",
                "the aquifer interval produced non-finite evidence or a non-positive pressure",
            )
        if (
            math.isfinite(self.initial_water_volume)
            and maximum_influx >= self.initial_water_volume
            and self.initial_water_volume > 0.0
        ):
            raise InvalidInputError(
                "interval", "the aquifer interval exhausts the initial water inventory"
            )
        return (time, cumulative, pressure, influx, minimum_influx, maximum_influx)

    def exposure(self, duration: float) -> float:
        ratio = self.productivity / self.capacity
        if math.isfinite(ratio):
            return ratio * duration
        return math.exp(math.log(self.productivity) + math.log(duration) - math.log(self.capacity))

    def finite_influx_at_fraction(
        self,
        duration: float,
        exposure: float,
        initial_difference: float,
        pressure_change: float,
        fraction: float,
    ) -> float:
        if math.isinf(exposure):
            return self.capacity * (initial_difference - pressure_change * fraction)
        fractional_exposure = exposure * fraction
        if fractional_exposure < _SMALL_EXPOSURE_SERIES_LIMIT:
            ramp = 0.5 + fractional_exposure * (
                -1.0 / 6.0
                + fractional_exposure
                * (1.0 / 24.0 + fractional_exposure * (-1.0 / 120.0 + fractional_exposure / 720.0))
            )
            decay = 1.0 - fractional_exposure * (
                0.5
                - fractional_exposure
                * (1.0 / 6.0 - fractional_exposure * (1.0 / 24.0 - fractional_exposure / 120.0))
            )
            return (
                self.productivity
                * duration
                * fraction
                * (initial_difference * decay - pressure_change * fraction * ramp)
            )
        loss = -math.expm1(-fractional_exposure)
        ramp_weight = 1.0 - loss / fractional_exposure
        return self.capacity * (
            initial_difference * loss - pressure_change * fraction * ramp_weight
        )


def _same_nonzero_sign(first: float, second: float) -> bool:
    return (first > 0.0 and second > 0.0) or (first < 0.0 and second < 0.0)


def _crossing_fraction(exposure: float, initial_difference: float, pressure_change: float) -> float:
    if exposure == 0.0:
        return initial_difference / pressure_change
    if math.isinf(exposure):
        return 0.0
    log_ratio = (
        math.log(exposure) + math.log(abs(initial_difference)) - math.log(abs(pressure_change))
    )
    if log_ratio < -40.0:
        return initial_difference / pressure_change
    log_one_plus_ratio = log_ratio if log_ratio > 40.0 else math.log1p(math.exp(log_ratio))
    return log_one_plus_ratio / exposure


def _state(row: tuple[float, float, float, float, float, float]) -> FetkovichState:
    time, cumulative, pressure, interval, minimum, maximum = row
    return FetkovichState(
        time=quantity(time, "s"),
        cumulative_influx=quantity(cumulative, "m**3"),
        pressure=quantity(pressure, "Pa"),
        interval_influx=quantity(interval, "m**3"),
        interval_minimum_cumulative_influx=quantity(minimum, "m**3"),
        interval_maximum_cumulative_influx=quantity(maximum, "m**3"),
    )


def fetkovich_aquifer(
    total_compressibility: float | None = None,
    productivity: float | None = None,
    initial_pressure: Any = None,
    intervals: Any = None,
    initial_water_volume: Any = None,
) -> FetkovichAquiferResult:
    """The finite aquifer over a prescribed interval schedule.

    Returns:
        The capacitance, absent exactly when storage is infinite, and the initial state
        followed by one state per interval.

    Raises:
        InvalidInputError: for a negative or NaN volume, compressibility or productivity, a
            non-positive pressure or duration, an unrepresentable finite capacity, or an
            interval that exhausts the finite inventory.
    """
    spec = _spec()
    volume = (
        math.inf
        if initial_water_volume is None
        else input_to_si(spec, "initial_water_volume", initial_water_volume)
    )
    ct = _required("total_compressibility", total_compressibility)
    productivity_value = _required("productivity", productivity)
    if initial_pressure is None:
        raise InvalidInputError("initial_pressure", "the calculation needs this field")
    pressure = input_to_si(spec, "initial_pressure", initial_pressure)
    aquifer = _Aquifer(volume, ct, productivity_value, pressure)
    if intervals is None:
        raise InvalidInputError("intervals", "the calculation needs the interval schedule")
    state = (0.0, 0.0, aquifer.initial_pressure, 0.0, 0.0, 0.0)
    states = [_state(state)]
    for index, interval in enumerate(intervals):
        duration = _interval_field(spec, "duration", interval["duration"])
        start = _interval_field(
            spec, "reservoir_pressure_start", interval["reservoir_pressure_start"]
        )
        end = _interval_field(spec, "reservoir_pressure_end", interval["reservoir_pressure_end"])
        try:
            state = aquifer.advance(state, duration, start, end)
        except InvalidInputError as error:
            raise InvalidInputError(f"intervals[{index}]", str(error)) from error
        states.append(_state(state))
    return FetkovichAquiferResult(
        capacity=None if math.isinf(aquifer.capacity) else aquifer.capacity,
        states=tuple(states),
        warnings=(),
    )


def _required(name: str, value: float | None) -> float:
    if value is None:
        raise InvalidInputError(name, "the calculation needs this field")
    return float(value)


def _interval_field(spec: dict[str, Any], name: str, value: Any) -> float:
    declaration = spec["inputs"]["intervals"]["fields"][name]
    return to_si(value, declaration["unit"], f"intervals.{name}")
