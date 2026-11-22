"""``pvt.decline_curve`` - NeqSim's decline-curve analysis.

The Python twin of ``crates/azoth-pvt/src/decline_curve.rs``: the Arps family, the two
rate-time fits and the Duong model. Rates, times and cumulative volumes are unit-agnostic
and cross as dimensionless magnitudes.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import DeclineCurveResult
from azoth.core.warnings import Warning, WarningCode

MODEL_ID = "pvt.decline_curve"


def _finite(name: str, value: float) -> float:
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "the calculation answered a non-finite value; the inputs are outside its state"
        )
    return value


def _required(name: str, value: float | None) -> float:
    if value is None:
        raise InvalidInputError(name, "the calculation needs this field")
    value = float(value)
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )
    return value


def _non_negative(name: str, value: float | None) -> float:
    value = _required(name, value)
    if value < 0.0:
        raise InvalidInputError(name, "the value cannot be negative")
    return value


def _positive(name: str, value: float | None) -> float:
    value = _required(name, value)
    if value <= 0.0:
        raise InvalidInputError(name, "the value must be positive")
    return value


def _whole(name: str, value: float | None, length: int) -> int:
    number = _required(name, value)
    if number < 0.0 or number != int(number) or number >= length:
        raise InvalidInputError(name, "the index is a whole number inside the history")
    return int(number)


def _zero_decline() -> Warning:
    return Warning(
        WarningCode.OUT_OF_VALID_RANGE,
        "the decline is zero, so the economic limit is never reached; "
        "the time and the EUR are absent",
    )


def _rate(qi: float, di: float, b: float, t: float) -> float:
    if t <= 0.0:
        return qi
    if b < 1.0e-10:
        return qi * math.exp(-di * t)
    return qi * math.pow(1.0 + b * di * t, -1.0 / b)


def _cumulative(qi: float, di: float, b: float, t: float) -> float:
    if t <= 0.0:
        return 0.0
    if di < 1.0e-15:
        return qi * t
    if b < 1.0e-10:
        return (qi / di) * (1.0 - math.exp(-di * t))
    if abs(b - 1.0) < 1.0e-10:
        return (qi / di) * math.log(1.0 + di * t)
    return (qi / ((1.0 - b) * di)) * (1.0 - math.pow(1.0 + b * di * t, (b - 1.0) / b))


def _time_to_rate(qi: float, di: float, b: float, q_limit: float) -> float | None:
    if q_limit >= qi:
        return 0.0
    if di < 1.0e-15:
        return None
    if b < 1.0e-10:
        return -math.log(q_limit / qi) / di
    return (math.pow(qi / q_limit, b) - 1.0) / (b * di)


def _linearize_arps(t: list[float], q: list[float], b: float) -> tuple[float, float] | None:
    n = float(len(t))
    sx = sy = sxx = sxy = 0.0
    if b < 1.0e-6:
        for time, rate in zip(t, q, strict=True):
            y = math.log(rate)
            sx += time
            sy += y
            sxx += time * time
            sxy += time * y
        denom = n * sxx - sx * sx
        if abs(denom) < 1.0e-30:
            return None
        slope = (n * sxy - sx * sy) / denom
        intercept = (sy - slope * sx) / n
        di = -slope
        qi = math.exp(intercept)
        return None if di <= 0.0 or qi <= 0.0 else (qi, di)
    for time, rate in zip(t, q, strict=True):
        y = math.pow(rate, -b)
        sx += time
        sy += y
        sxx += time * time
        sxy += time * y
    denom = n * sxx - sx * sx
    if abs(denom) < 1.0e-30:
        return None
    slope = (n * sxy - sx * sy) / denom
    intercept = (sy - slope * sx) / n
    if intercept <= 0.0 or slope <= 0.0:
        return None
    qi = math.pow(intercept, -1.0 / b)
    di = slope / (intercept * b)
    return None if qi <= 0.0 or di <= 0.0 else (qi, di)


def _fit_arps(
    t: list[float], q: list[float], start: int, end: int
) -> tuple[float, float, float, float]:
    count = end - start + 1
    if count < 3:
        raise InvalidInputError("history_time", "a fit needs at least three points")
    t0 = t[start]
    times = [t[start + index] - t0 for index in range(count)]
    rates = [q[start + index] for index in range(count)]
    if any(rate <= 0.0 for rate in rates):
        raise InvalidInputError("history_rate", "every rate in the fit must be positive")
    best_b = 0.0
    best_qi = rates[0]
    best_di = 0.001
    best_sse = float("inf")
    for pass_index in range(2):
        low, high = (
            (0.0, 1.0) if pass_index == 0 else (max(best_b - 0.05, 0.0), min(best_b + 0.05, 1.0))
        )
        for step in range(101):
            b_try = low + (high - low) * step / 100.0
            linear = _linearize_arps(times, rates, b_try)
            if linear is None:
                continue
            qi, di = linear
            sse = sum(
                (_rate(qi, di, b_try, time) - rate) ** 2
                for time, rate in zip(times, rates, strict=True)
            )
            if sse < best_sse:
                best_sse = sse
                best_b = b_try
                best_qi = qi
                best_di = di
    mean = sum(rates) / count
    ss_tot = sum((rate - mean) ** 2 for rate in rates)
    r_squared = 1.0 - best_sse / ss_tot if ss_tot > 1.0e-30 else 1.0
    return best_qi, best_di, best_b, r_squared


def _estimate_hyperbolic(
    q1: float, t1: float, q2: float, t2: float, q3: float, t3: float
) -> tuple[float, float, float]:
    if t3 <= t1:
        raise InvalidInputError("t3", "the third point's time must be greater than the first's")
    di_est = math.log(q1 / q3) / (t3 - t1)
    if di_est <= 0.0:
        di_est = 0.001
    best_b = 0.0
    best_error = float("inf")
    for index in range(101):
        b_try = index / 100.0
        r = math.pow(q1 / q3, b_try)
        d_try = di_est if abs(r - 1.0) < 1.0e-12 else (r - 1.0) / (b_try * (t3 - t1) + 1.0e-30)
        if d_try <= 0.0:
            continue
        qi_try = q1 * math.pow(1.0 + b_try * d_try * t1, 1.0 / (b_try + 1.0e-30))
        predicted = _rate(qi_try, d_try, b_try, t2)
        error = abs(predicted - q2) / q2
        if error < best_error:
            best_error = error
            best_b = b_try
    r = math.pow(q1 / q3, best_b)
    di_final = (
        math.log(q1 / q3) / (t3 - t1) if best_b < 1.0e-10 else (r - 1.0) / (best_b * (t3 - t1))
    )
    if di_final <= 0.0:
        di_final = di_est
    qi_final = (
        q1 * math.exp(di_final * t1)
        if best_b < 1.0e-10
        else q1 * math.pow(1.0 + best_b * di_final * t1, 1.0 / best_b)
    )
    return qi_final, di_final, best_b


def _duong_time(a: float, m: float, t: float) -> float:
    if t <= 0.0:
        return 1.0
    exponent = a / (1.0 - m) * (math.pow(t, 1.0 - m) - 1.0)
    return math.pow(t, -m) * math.exp(exponent)


def _cumulative_duong(q1: float, a: float, m: float, t: float) -> float:
    if t <= 0.0:
        return 0.0
    exponent = a / (1.0 - m) * (math.pow(t, 1.0 - m) - 1.0)
    return q1 / a * math.exp(exponent)


def _fit_duong(t: list[float], q: list[float]) -> tuple[float, float, float, float]:
    n = len(t)
    for index in range(n):
        if t[index] <= 0.0 or q[index] <= 0.0:
            raise InvalidInputError(
                "history_time", "the Duong fit needs strictly positive times and rates"
            )
        if index > 0 and t[index] <= t[index - 1]:
            raise InvalidInputError("history_time", "the Duong fit needs strictly increasing times")
    gp = [0.0] * n
    gp[0] = q[0] * t[0]
    for index in range(1, n):
        gp[index] = gp[index - 1] + 0.5 * (q[index] + q[index - 1]) * (t[index] - t[index - 1])
    sx = sy = sxx = sxy = 0.0
    for index in range(n):
        x = math.log(t[index])
        y = math.log(q[index] / gp[index])
        sx += x
        sy += y
        sxx += x * x
        sxy += x * y
    count = float(n)
    denom = count * sxx - sx * sx
    if abs(denom) < 1.0e-30:
        raise InvalidInputError("history_time", "the Duong regression is singular")
    slope = (count * sxy - sx * sy) / denom
    intercept = (sy - slope * sx) / count
    m = -slope
    a = math.exp(intercept)
    stt = stq = 0.0
    for time, rate in zip(t, q, strict=True):
        time_function = _duong_time(a, m, time)
        stt += time_function * time_function
        stq += time_function * rate
    q1 = stq / stt if stt > 1.0e-30 else q[0]
    mean = sum(q) / count
    ss_res = sum(
        (rate - q1 * _duong_time(a, m, time)) ** 2 for time, rate in zip(t, q, strict=True)
    )
    ss_tot = sum((rate - mean) ** 2 for rate in q)
    r_squared = 1.0 - ss_res / ss_tot if ss_tot > 1.0e-30 else 1.0
    return q1, a, m, r_squared


_FIELDS = (
    "time",
    "rate",
    "cumulative",
    "rate_value",
    "cumulative_value",
    "nominal",
    "effective",
    "instantaneous_decline_rate",
    "time_to_rate",
    "eur",
    "remaining",
    "exponential_decline",
    "fit_qi",
    "fit_di",
    "fit_b",
    "fit_r_squared",
    "duong_q1",
    "duong_a",
    "duong_m",
    "duong_time",
    "summary_effective",
    "summary_rate_1yr",
    "summary_rate_5yr",
    "summary_rate_10yr",
    "summary_cum_1yr",
    "summary_cum_5yr",
    "summary_cum_10yr",
    "summary_time_to_limit",
    "summary_time_to_limit_years",
    "summary_eur",
)


def _row(**values: Any) -> DeclineCurveResult:
    fields = {name: values.get(name) for name in _FIELDS}
    return DeclineCurveResult(**fields, warnings=tuple(values.get("warnings", ())))


def decline_curve(
    calculation: str,
    *,
    qi: float | None = None,
    di: float | None = None,
    b: float | None = None,
    t: float | None = None,
    q_limit: float | None = None,
    d_nominal: float | None = None,
    d_effective: float | None = None,
    q1: float | None = None,
    t1: float | None = None,
    q2: float | None = None,
    t2: float | None = None,
    q3: float | None = None,
    t3: float | None = None,
    t_start: float | None = None,
    t_end: float | None = None,
    t_step: float | None = None,
    t_current: float | None = None,
    a: float | None = None,
    m: float | None = None,
    start_index: float | None = None,
    end_index: float | None = None,
    history_time: list[float] | None = None,
    history_rate: list[float] | None = None,
) -> DeclineCurveResult:
    """The decline calculation over the stated inputs.

    Returns:
        One result with the fields the calculation answers; the others are absent.

    Raises:
        InvalidInputError: for an unknown calculation, a missing required scalar, a
            non-positive rate where a fit needs one, a bad window, fewer than three
            history points, `t2 <= t1`, a non-positive time step, or a non-finite answer.
    """
    if calculation == "rate":
        return _row(
            rate_value=_finite(
                "rate_value",
                _rate(
                    _required("qi", qi), _required("di", di), _required("b", b), _required("t", t)
                ),
            )
        )
    if calculation == "rate_exponential":
        return _row(
            rate_value=_finite(
                "rate_value",
                _rate(_non_negative("qi", qi), _non_negative("di", di), 0.0, _non_negative("t", t)),
            )
        )
    if calculation == "rate_hyperbolic":
        return _row(
            rate_value=_finite(
                "rate_value",
                _rate(
                    _required("qi", qi), _required("di", di), _required("b", b), _required("t", t)
                ),
            )
        )
    if calculation == "rate_harmonic":
        return _row(
            rate_value=_finite(
                "rate_value",
                _rate(_required("qi", qi), _required("di", di), 1.0, _required("t", t)),
            )
        )
    if calculation == "cumulative_production":
        return _row(
            cumulative_value=_finite(
                "cumulative_value",
                _cumulative(
                    _required("qi", qi), _required("di", di), _required("b", b), _required("t", t)
                ),
            )
        )
    if calculation == "cumulative_exponential":
        return _row(
            cumulative_value=_finite(
                "cumulative_value",
                _cumulative(
                    _non_negative("qi", qi), _non_negative("di", di), 0.0, _non_negative("t", t)
                ),
            )
        )
    if calculation == "cumulative_harmonic":
        return _row(
            cumulative_value=_finite(
                "cumulative_value",
                _cumulative(_required("qi", qi), _required("di", di), 1.0, _required("t", t)),
            )
        )
    if calculation == "nominal_to_effective_annual":
        d = _non_negative("d_nominal", d_nominal)
        return _row(effective=_finite("effective", 1.0 - math.exp(-d * 365.25)))
    if calculation == "effective_to_nominal_annual":
        d = _required("d_effective", d_effective)
        if not 0.0 < d < 1.0:
            raise InvalidInputError(
                "d_effective", "the effective decline is a fraction strictly between 0 and 1"
            )
        return _row(nominal=_finite("nominal", -math.log(1.0 - d) / 365.25))
    if calculation == "instantaneous_decline_rate":
        decline = _required("di", di)
        exponent = _required("b", b)
        time = _required("t", t)
        value = decline if exponent < 1.0e-10 else decline / (1.0 + exponent * decline * time)
        return _row(instantaneous_decline_rate=_finite("instantaneous_decline_rate", value))
    if calculation in ("time_to_rate", "eur", "remaining_reserves"):
        first = _positive("qi", qi)
        decline = _non_negative("di", di)
        exponent = _non_negative("b", b)
        limit = _positive("q_limit", q_limit)
        limit_time = _time_to_rate(first, decline, exponent, limit)
        if limit_time is None:
            return _row(warnings=(_zero_decline(),))
        if calculation == "time_to_rate":
            return _row(time_to_rate=_finite("time_to_rate", limit_time))
        if calculation == "eur":
            return _row(eur=_finite("eur", _cumulative(first, decline, exponent, limit_time)))
        current = _non_negative("t_current", t_current)
        return _row(
            remaining=_finite(
                "remaining",
                _cumulative(first, decline, exponent, limit_time)
                - _cumulative(first, decline, exponent, current),
            )
        )
    if calculation == "forecast":
        first = _required("qi", qi)
        decline = _required("di", di)
        exponent = _required("b", b)
        start = _required("t_start", t_start)
        end = _required("t_end", t_end)
        step = _positive("t_step", t_step)
        if end < start:
            raise InvalidInputError("t_end", "the forecast's end is before its start")
        points = math.ceil((end - start) / step) + 1
        if points <= 0:
            raise InvalidInputError("t_end", "the forecast asks for no points")
        times: list[float] = []
        rates: list[float] = []
        cumulative: list[float] = []
        for index in range(points):
            elapsed = start + index * step
            if elapsed > end:
                elapsed = end
            times.append(_finite("time", elapsed))
            rates.append(_finite("rate", _rate(first, decline, exponent, elapsed)))
            cumulative.append(_finite("cumulative", _cumulative(first, decline, exponent, elapsed)))
        return _row(time=tuple(times), rate=tuple(rates), cumulative=tuple(cumulative))
    if calculation == "estimate_exponential_decline":
        first = _positive("q1", q1)
        first_t = _required("t1", t1)
        second = _positive("q2", q2)
        second_t = _required("t2", t2)
        if second_t <= first_t:
            raise InvalidInputError(
                "t2", "the second point's time must be greater than the first's"
            )
        if second >= first:
            raise InvalidInputError("q2", "the second rate must be below the first")
        return _row(
            exponential_decline=_finite(
                "exponential_decline", math.log(first / second) / (second_t - first_t)
            )
        )
    if calculation == "estimate_hyperbolic_parameters":
        estimates = _estimate_hyperbolic(
            _positive("q1", q1),
            _required("t1", t1),
            _positive("q2", q2),
            _required("t2", t2),
            _positive("q3", q3),
            _required("t3", t3),
        )
        return _row(
            fit_qi=_finite("fit_qi", estimates[0]),
            fit_di=_finite("fit_di", estimates[1]),
            fit_b=_finite("fit_b", estimates[2]),
        )
    if calculation in ("fit_arps", "fit_arps_window", "fit_duong"):
        times, rates = _history(history_time, history_rate)
        if calculation == "fit_arps":
            start, end = 0, len(times) - 1
        elif calculation == "fit_arps_window":
            start = _whole("start_index", start_index, len(times))
            end = _whole("end_index", end_index, len(times))
            if start >= end:
                raise InvalidInputError(
                    "start_index", "the window's first index must be below its last"
                )
        else:
            fitted = _fit_duong(times, rates)
            return _row(
                duong_q1=_finite("duong_q1", fitted[0]),
                duong_a=_finite("duong_a", fitted[1]),
                duong_m=_finite("duong_m", fitted[2]),
                fit_r_squared=_finite("fit_r_squared", fitted[3]),
            )
        fitted = _fit_arps(times, rates, start, end)
        return _row(
            fit_qi=_finite("fit_qi", fitted[0]),
            fit_di=_finite("fit_di", fitted[1]),
            fit_b=_finite("fit_b", fitted[2]),
            fit_r_squared=_finite("fit_r_squared", fitted[3]),
        )
    if calculation == "eur_from_fit":
        first = _positive("fit_qi", qi)
        decline = _non_negative("fit_di", di)
        exponent = _non_negative("fit_b", b)
        limit = _positive("q_limit", q_limit)
        limit_time = _time_to_rate(first, decline, exponent, limit)
        if limit_time is None:
            return _row(warnings=(_zero_decline(),))
        return _row(eur=_finite("eur", _cumulative(first, decline, exponent, limit_time)))
    if calculation == "duong_time_function":
        return _row(
            duong_time=_finite(
                "duong_time",
                _duong_time(_positive("a", a), _required("m", m), _non_negative("t", t)),
            )
        )
    if calculation == "rate_duong":
        return _row(
            rate_value=_finite(
                "rate_value",
                _positive("q1", q1)
                * _duong_time(_positive("a", a), _required("m", m), _non_negative("t", t)),
            )
        )
    if calculation == "cumulative_duong":
        return _row(
            cumulative_value=_finite(
                "cumulative_value",
                _cumulative_duong(
                    _positive("q1", q1), _positive("a", a), _required("m", m), _non_negative("t", t)
                ),
            )
        )
    if calculation == "summary":
        first = _positive("qi", qi)
        decline = _non_negative("di", di)
        exponent = _non_negative("b", b)
        limit = _positive("q_limit", q_limit)
        limit_time = _time_to_rate(first, decline, exponent, limit)
        summary_fields: dict[str, Any] = {
            "summary_effective": _finite("summary_effective", 1.0 - math.exp(-decline * 365.25)),
            "summary_rate_1yr": _finite(
                "summary_rate_1yr", _rate(first, decline, exponent, 365.25)
            ),
            "summary_rate_5yr": _finite(
                "summary_rate_5yr", _rate(first, decline, exponent, 5.0 * 365.25)
            ),
            "summary_rate_10yr": _finite(
                "summary_rate_10yr", _rate(first, decline, exponent, 10.0 * 365.25)
            ),
            "summary_cum_1yr": _finite(
                "summary_cum_1yr", _cumulative(first, decline, exponent, 365.25)
            ),
            "summary_cum_5yr": _finite(
                "summary_cum_5yr", _cumulative(first, decline, exponent, 5.0 * 365.25)
            ),
            "summary_cum_10yr": _finite(
                "summary_cum_10yr", _cumulative(first, decline, exponent, 10.0 * 365.25)
            ),
        }
        if limit_time is not None:
            summary_fields["summary_time_to_limit"] = _finite("summary_time_to_limit", limit_time)
            summary_fields["summary_time_to_limit_years"] = _finite(
                "summary_time_to_limit_years", limit_time / 365.25
            )
            summary_fields["summary_eur"] = _finite(
                "summary_eur", _cumulative(first, decline, exponent, limit_time)
            )
        else:
            summary_fields["warnings"] = (_zero_decline(),)
        return _row(**summary_fields)
    raise InvalidInputError(
        "calculation", f"{calculation!r} is not one of the decline calculations"
    )


def _history(
    times: list[float] | None, rates: list[float] | None
) -> tuple[list[float], list[float]]:
    if times is None:
        raise InvalidInputError("history_time", "the fit needs the history's times")
    if rates is None:
        raise InvalidInputError("history_rate", "the fit needs the history's rates")
    if len(times) != len(rates):
        raise InvalidInputError("history_rate", "the times and rates are different lengths")
    if len(times) < 3:
        raise InvalidInputError("history_time", "a fit needs at least three points")
    schedule = [float(value) for value in times]
    for index, value in enumerate(schedule):
        if not math.isfinite(value) or value < 0.0:
            raise InvalidInputError(
                "history_time", "the history's times must be finite and non-negative"
            )
        if index > 0 and value <= schedule[index - 1]:
            raise InvalidInputError(
                "history_time", "the history's times must be strictly increasing"
            )
    return schedule, [float(value) for value in rates]
