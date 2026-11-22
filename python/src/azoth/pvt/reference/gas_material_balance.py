"""``pvt.gas_material_balance`` - NeqSim's gas material balance.

The Python twin of ``crates/azoth-pvt/src/gas_material_balance.rs``: the p/Z fit, the
gravity route, the Cole plot and the Havlena-Odeh fit.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import GasMaterialBalanceResult
from azoth.core.units import input_to_si, quantity
from azoth.core.warnings import Warning, WarningCode

MODEL_ID = "pvt.gas_material_balance"
P_SC = 1.01325
T_SC = 288.71


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


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


def _positive(name: str, value: float | None) -> float:
    value = _required(name, value)
    if value <= 0.0:
        raise InvalidInputError(name, "the value must be finite and positive")
    return value


def _temperature(value: Any) -> float:
    if value is None:
        raise InvalidInputError("temperature", "the calculation needs the temperature")
    temperature = input_to_si(_spec(), "temperature", value)
    if temperature <= 0.0:
        raise InvalidInputError(
            "temperature", "an absolute temperature; zero and below are not states"
        )
    return temperature


def _bg(p_bara: float, z: float, t_k: float) -> float:
    return z * t_k / p_bara * P_SC / T_SC


def _arrays(pressure: Any, z: Any, gp: Any) -> tuple[list[float], list[float], list[float]]:
    if pressure is None:
        raise InvalidInputError("pressure", "the calculation needs the pressure history")
    if z is None:
        raise InvalidInputError("z", "the calculation needs the Z history")
    if gp is None:
        raise InvalidInputError("gp", "the calculation needs the cumulative gas history")
    pressure_si = [input_to_si(_spec(), "pressure", value) for value in pressure]
    z_values = [float(value) for value in z]
    gp_values = [float(value) for value in gp]
    if not len(pressure_si) == len(z_values) == len(gp_values):
        raise InvalidInputError("pressure", "the history arrays are different lengths")
    if len(pressure_si) < 2:
        raise InvalidInputError("pressure", "at least two history points are required")
    for index, value in enumerate(pressure_si):
        if not math.isfinite(value) or value <= 0.0:
            raise InvalidInputError(
                f"pressure[{index}]", "a material balance pressure must be finite and positive"
            )
    return pressure_si, z_values, gp_values


def _linear(x: list[float], y: list[float]) -> tuple[float, float, float]:
    n = float(len(x))
    if len(x) < 2:
        raise InvalidInputError("pressure", "a regression needs at least two points")
    sx = sy = sxx = sxy = 0.0
    for time, value in zip(x, y, strict=True):
        sx += time
        sy += value
        sxx += time * time
        sxy += time * value
    denom = n * sxx - sx * sx
    if abs(denom) < 1.0e-30:
        raise InvalidInputError(
            "gp", "the regression is singular: the cumulative gas does not vary"
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


def _through_origin(x: list[float], y: list[float]) -> tuple[float, float]:
    if not x:
        raise InvalidInputError("pressure", "the fit needs at least one point")
    sxx = sum(value * value for value in x)
    sxy = sum(a * b for a, b in zip(x, y, strict=True))
    sy = sum(y)
    if sxx < 1.0e-30:
        raise InvalidInputError(
            "pressure", "the regression is singular: the expansion does not vary"
        )
    slope = sxy / sxx
    mean = sy / len(x)
    ss_res = sum((value - slope * time) ** 2 for time, value in zip(x, y, strict=True))
    ss_tot = sum((value - mean) ** 2 for value in y)
    r_squared = 1.0 - ss_res / ss_tot if ss_tot > 1.0e-30 else 1.0
    return slope, r_squared


def _zero_ogip() -> Warning:
    return Warning(
        WarningCode.OUT_OF_VALID_RANGE,
        "the p/Z line does not decline, so the original gas in place is absent",
    )


def _row(**values: Any) -> GasMaterialBalanceResult:
    return GasMaterialBalanceResult(
        gas_fvf=values.get("gas_fvf"),
        ogip=values.get("ogip"),
        pi_over_zi=values.get("pi_over_zi"),
        slope=values.get("slope"),
        r_squared=values.get("r_squared"),
        cole_gp=values.get("cole_gp"),
        cole_f_over_eg=values.get("cole_f_over_eg"),
        warnings=tuple(values.get("warnings", ())),
    )


def gas_material_balance(
    calculation: str,
    *,
    pressure: Any = None,
    z: Any = None,
    gp: Any = None,
    we: Any = None,
    pressure_value: Any = None,
    z_value: float | None = None,
    temperature: Any = None,
    gas_gravity: float | None = None,
) -> GasMaterialBalanceResult:
    """The gas material balance over the stated inputs.

    Returns:
        The published fields; the others are absent.

    Raises:
        InvalidInputError: for an unknown calculation, a missing input, arrays of different
            lengths or fewer than two points, a non-positive pressure, Z or temperature, a
            singular regression, a non-positive Cole expansion, or a non-finite answer.
    """
    if calculation == "bg":
        p = (
            _positive(
                "pressure_value",
                None
                if pressure_value is None
                else input_to_si(_spec(), "pressure_value", pressure_value),
            )
            / 1.0e5
        )
        z = _positive("z_value", z_value)
        t = _temperature(temperature)
        return _row(gas_fvf=_finite("gas_fvf", _bg(p, z, t)))
    if calculation in ("fit_volumetric", "fit_volumetric_from_gravity"):
        pressure_si, z_values, gp_values = _arrays(pressure, z, gp)
        if calculation == "fit_volumetric_from_gravity":
            t = _temperature(temperature)
            gamma = _positive("gas_gravity", gas_gravity)
            if not 0.57 <= gamma <= 1.68:
                raise InvalidInputError(
                    "gas_gravity",
                    "the Sutton correlation is defined for a specific gravity in [0.57, 1.68]",
                )
            from azoth.pvt.reference.gas_correlations import _z_factor_sutton

            z_values = [_z_factor_sutton(p / 1.0e5, t, gamma) for p in pressure_si]
        poz = [_positive("z", value) for value in z_values]
        poz = [(pressure_si[index] / 1.0e5) / poz[index] for index in range(len(poz))]
        slope, intercept, r_squared = _linear(gp_values, poz)
        if slope < 0.0:
            ogip = _finite("ogip", -intercept / slope)
            warnings: tuple[Warning, ...] = ()
        else:
            ogip = None
            warnings = (_zero_ogip(),)
        return _row(
            ogip=ogip,
            pi_over_zi=quantity(intercept, "bar"),
            slope=_finite("slope", slope),
            r_squared=_finite("r_squared", r_squared),
            warnings=warnings,
        )
    if calculation == "cole_plot":
        pressure_si, z_values, gp_values = _arrays(pressure, z, gp)
        t = _temperature(temperature)
        bgi = _bg(pressure_si[0] / 1.0e5, z_values[0], t)
        gp_out: list[float] = []
        ratio: list[float] = []
        for index in range(1, len(pressure_si)):
            bg_cur = _bg(pressure_si[index] / 1.0e5, z_values[index], t)
            f = gp_values[index] * bg_cur
            eg = bg_cur - bgi
            if eg <= 1.0e-30:
                raise InvalidInputError(
                    f"pressure[{index}]",
                    "the Cole plot's expansion is not positive, so its ratio is not a number",
                )
            gp_out.append(_finite("cole_gp", gp_values[index]))
            ratio.append(_finite("cole_f_over_eg", f / eg))
        return _row(cole_gp=tuple(gp_out), cole_f_over_eg=tuple(ratio))
    if calculation == "fit_havlena_odeh":
        pressure_si, z_values, gp_values = _arrays(pressure, z, gp)
        if we is None:
            raise InvalidInputError("we", "the Havlena-Odeh fit needs the water influx history")
        we_values = [float(value) for value in we]
        if len(we_values) != len(pressure_si):
            raise InvalidInputError(
                "we", "the water influx history is a different length from the pressure history"
            )
        t = _temperature(temperature)
        bgi = _bg(pressure_si[0] / 1.0e5, z_values[0], t)
        x: list[float] = []
        y: list[float] = []
        for index in range(1, len(pressure_si)):
            bg_cur = _bg(pressure_si[index] / 1.0e5, z_values[index], t)
            x.append(bg_cur - bgi)
            y.append(gp_values[index] * bg_cur - we_values[index])
        slope, r_squared = _through_origin(x, y)
        return _row(
            ogip=_finite("ogip", slope),
            pi_over_zi=quantity(0.0, "bar"),
            slope=_finite("slope", slope),
            r_squared=_finite("r_squared", r_squared),
        )
    raise InvalidInputError(
        "calculation", f"{calculation!r} is not one of the gas material-balance calculations"
    )
