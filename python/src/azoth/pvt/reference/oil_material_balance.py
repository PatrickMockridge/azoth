"""``pvt.oil_material_balance`` - NeqSim's oil material balance.

The Python twin of ``crates/azoth-pvt/src/oil_material_balance.rs``: the four terms, the
three fits and the drive indices.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import OilMaterialBalanceResult

MODEL_ID = "pvt.oil_material_balance"


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _finite(name: str, value: float) -> float:
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "the calculation answered a non-finite value; the inputs are outside its state"
        )
    return value


def _scalar(name: str, value: float | None) -> float:
    if value is None:
        raise InvalidInputError(name, "the calculation needs this field")
    value = float(value)
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )
    return value


def _positive(name: str, value: float | None) -> float:
    value = _scalar(name, value)
    if value <= 0.0:
        raise InvalidInputError(name, "the value must be positive")
    return value


def _arrays(f: Any, eo: Any) -> tuple[list[float], list[float]]:
    if f is None:
        raise InvalidInputError("f_history", "the fit needs the withdrawal history")
    if eo is None:
        raise InvalidInputError("eo_history", "the fit needs the Eo history")
    f_values = [float(value) for value in f]
    eo_values = [float(value) for value in eo]
    if len(f_values) != len(eo_values):
        raise InvalidInputError("f_history", "the history arrays are different lengths")
    if len(f_values) < 2:
        raise InvalidInputError("f_history", "at least two history points are required")
    return f_values, eo_values


def _through_origin(x: list[float], y: list[float]) -> tuple[float, float]:
    sxx = sum(value * value for value in x)
    sxy = sum(a * b for a, b in zip(x, y, strict=True))
    sy = sum(y)
    if sxx < 1.0e-30:
        raise InvalidInputError(
            "eo_history", "the regression is singular: the expansion does not vary"
        )
    slope = sxy / sxx
    mean = sy / len(x)
    ss_res = sum((value - slope * time) ** 2 for time, value in zip(x, y, strict=True))
    ss_tot = sum((value - mean) ** 2 for value in y)
    r_squared = 1.0 - ss_res / ss_tot if ss_tot > 1.0e-30 else 1.0
    return slope, r_squared


def _linear(x: list[float], y: list[float]) -> tuple[float, float, float]:
    n = float(len(x))
    sx = sy = sxx = sxy = 0.0
    for time, value in zip(x, y, strict=True):
        sx += time
        sy += value
        sxx += time * time
        sxy += time * value
    denom = n * sxx - sx * sx
    if abs(denom) < 1.0e-30:
        raise InvalidInputError(
            "we_history", "the regression is singular: the influx does not vary"
        )
    slope = (n * sxy - sx * sy) / denom
    intercept = (sy - slope * sx) / n
    mean = sy / n
    ss_res = sum(
        (value - (slope * time + intercept)) ** 2 for time, value in zip(x, y, strict=True)
    )
    ss_tot = sum((value - mean) ** 2 for value in y)
    r_squared = 1.0 - ss_res / ss_tot if ss_tot > 1.0e-30 else 1.0
    return slope, intercept, r_squared


def _two_predictor(x1: list[float], x2: list[float], y: list[float]) -> tuple[float, float, float]:
    s11 = s12 = s22 = s1y = s2y = 0.0
    for first, second, value in zip(x1, x2, y, strict=True):
        s11 += first * first
        s12 += first * second
        s22 += second * second
        s1y += first * value
        s2y += second * value
    det = s11 * s22 - s12 * s12
    if abs(det) < 1.0e-30:
        raise InvalidInputError(
            "eo_history", "the two-predictor regression is singular: the predictors are collinear"
        )
    a = (s1y * s22 - s2y * s12) / det
    b = (s11 * s2y - s12 * s1y) / det
    mean = sum(y) / len(y)
    ss_res = sum(
        (value - (a * first + b * second)) ** 2
        for first, second, value in zip(x1, x2, y, strict=True)
    )
    ss_tot = sum((value - mean) ** 2 for value in y)
    r_squared = 1.0 - ss_res / ss_tot if ss_tot > 1.0e-30 else 1.0
    return a, b, r_squared


def _row(**values: Any) -> OilMaterialBalanceResult:
    return OilMaterialBalanceResult(
        f=values.get("f"),
        eo_value=values.get("eo_value"),
        eg_value=values.get("eg_value"),
        efw_value=values.get("efw_value"),
        ooip=values.get("ooip"),
        m_fit=values.get("m_fit"),
        r_squared=values.get("r_squared"),
        ddi=values.get("ddi"),
        sdi=values.get("sdi"),
        wdi=values.get("wdi"),
        edi=values.get("edi"),
        warnings=tuple(values.get("warnings", ())),
    )


def oil_material_balance(
    calculation: str,
    *,
    np: float | None = None,
    rp: float | None = None,
    bo: float | None = None,
    rs: float | None = None,
    bg: float | None = None,
    wp: float | None = None,
    bw: float | None = None,
    boi: float | None = None,
    rsi: float | None = None,
    bgi: float | None = None,
    cw: float | None = None,
    swi: float | None = None,
    cf: float | None = None,
    delta_p: float | None = None,
    m: float | None = None,
    n: float | None = None,
    eo_term: float | None = None,
    eg_term: float | None = None,
    efw_term: float | None = None,
    we_value: float | None = None,
    f_value: float | None = None,
    f_history: Any = None,
    eo_history: Any = None,
    eg_history: Any = None,
    we_history: Any = None,
) -> OilMaterialBalanceResult:
    """The oil material balance over the stated inputs.

    Returns:
        The published fields; the others are absent.

    Raises:
        InvalidInputError: for an unknown calculation, a missing scalar, a history of
            different lengths or fewer than two points, a zero Eo in the water-drive plot, a
            water saturation at or above one, a singular regression, or a non-finite answer.
    """
    if calculation == "withdrawal_f":
        np_value = _scalar("np", np)
        rp_value = _scalar("rp", rp)
        bo_value = _scalar("bo", bo)
        rs_value = _scalar("rs", rs)
        bg_value = _scalar("bg", bg)
        wp_value = _scalar("wp", wp)
        bw_value = _scalar("bw", bw)
        return _row(
            f=_finite(
                "f", np_value * (bo_value + (rp_value - rs_value) * bg_value) + wp_value * bw_value
            )
        )
    if calculation == "eo":
        return _row(
            eo_value=_finite(
                "eo_value",
                (_scalar("bo", bo) - _scalar("boi", boi))
                + (_scalar("rsi", rsi) - _scalar("rs", rs)) * _scalar("bg", bg),
            )
        )
    if calculation == "eg":
        return _row(
            eg_value=_finite(
                "eg_value",
                _scalar("boi", boi) * (_scalar("bg", bg) / _positive("bgi", bgi) - 1.0),
            )
        )
    if calculation == "efw":
        swi_value = _scalar("swi", swi)
        if not 0.0 <= swi_value < 1.0:
            raise InvalidInputError("swi", "the initial water saturation is a fraction below one")
        return _row(
            efw_value=_finite(
                "efw_value",
                _scalar("boi", boi)
                * (1.0 + _scalar("m", m))
                * (_scalar("cw", cw) * swi_value + _scalar("cf", cf))
                / (1.0 - swi_value)
                * _scalar("delta_p", delta_p),
            )
        )
    if calculation == "fit_depletion_drive":
        f, eo = _arrays(f_history, eo_history)
        ooip, r_squared = _through_origin(eo, f)
        return _row(
            ooip=_finite("ooip", ooip), m_fit=0.0, r_squared=_finite("r_squared", r_squared)
        )
    if calculation == "fit_gas_cap_drive":
        f, eo = _arrays(f_history, eo_history)
        if eg_history is None:
            raise InvalidInputError("eg_history", "the gas-cap fit needs the Eg history")
        eg = [float(value) for value in eg_history]
        if len(eg) != len(f):
            raise InvalidInputError(
                "eg_history", "the Eg history is a different length from the withdrawal history"
            )
        first, second, r_squared = _two_predictor(eo, eg, f)
        m_fit = second / first if abs(first) > 1.0e-30 else 0.0
        return _row(
            ooip=_finite("ooip", first),
            m_fit=_finite("m_fit", m_fit),
            r_squared=_finite("r_squared", r_squared),
        )
    if calculation == "fit_water_drive":
        f, eo = _arrays(f_history, eo_history)
        if we_history is None:
            raise InvalidInputError("we_history", "the water-drive fit needs the We history")
        we = [float(value) for value in we_history]
        if len(we) != len(f):
            raise InvalidInputError(
                "we_history", "the We history is a different length from the withdrawal history"
            )
        bw_value = _scalar("bw", bw)
        x: list[float] = []
        y: list[float] = []
        for index in range(len(f)):
            if abs(eo[index]) < 1.0e-30:
                raise InvalidInputError(
                    f"eo_history[{index}]", "Eo must be non-zero for the water-drive plot"
                )
            x.append(we[index] * bw_value / eo[index])
            y.append(f[index] / eo[index])
        _, intercept, r_squared = _linear(x, y)
        return _row(
            ooip=_finite("ooip", intercept),
            m_fit=0.0,
            r_squared=_finite("r_squared", r_squared),
        )
    if calculation == "drive_indices":
        n_value = _scalar("n", n)
        m_value = _scalar("m", m)
        eo_value = _scalar("eo_term", eo_term)
        eg_value = _scalar("eg_term", eg_term)
        efw_value = _scalar("efw_term", efw_term)
        we_number = _scalar("we_value", we_value)
        bw_value = _scalar("bw", bw)
        f_value_number = _scalar("f_value", f_value)
        if abs(f_value_number) < 1.0e-30:
            return _row(ddi=0.0, sdi=0.0, wdi=0.0, edi=0.0)
        return _row(
            ddi=_finite("ddi", n_value * eo_value / f_value_number),
            sdi=_finite("sdi", n_value * m_value * eg_value / f_value_number),
            wdi=_finite("wdi", we_number * bw_value / f_value_number),
            edi=_finite("edi", n_value * efw_value / f_value_number),
        )
    raise InvalidInputError(
        "calculation", f"{calculation!r} is not one of the oil material-balance calculations"
    )
