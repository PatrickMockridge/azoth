"""``pvt.water_correlations`` - NeqSim's water and brine correlation table.

The Python twin of ``crates/azoth-pvt/src/water_correlations.rs``: the water FVFs, the
Meehan viscosity pair, McCain's compressibility, Batzle-Wang's density and surface tension,
and the two solution gas-water ratios.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import WaterCorrelationsResult, WaterResultRow
from azoth.core.units import input_records_to_si, quantity
from azoth.core.warnings import Warning
from azoth.pvt.reference import _standard as S

MODEL_ID = "pvt.water_correlations"


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _finite(name: str, value: float) -> float:
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "the correlation answered a non-finite value; the inputs are outside its state"
        )
    return value


def _required(row: dict[str, Any], index: int, name: str) -> float:
    value = row.get(name)
    if value is None:
        raise InvalidInputError(f"queries[{index}].{name}", "the calculation needs this field")
    return float(value)


def _salinity_ppm(row: dict[str, Any], index: int) -> float:
    value = _finite(f"queries[{index}].salinity_ppm", _required(row, index, "salinity_ppm"))
    if value < 0.0:
        raise InvalidInputError(f"queries[{index}].salinity_ppm", "a salinity cannot be negative")
    return value


def _salinity_molality(row: dict[str, Any], index: int) -> float:
    value = _finite(
        f"queries[{index}].salinity_molality", _required(row, index, "salinity_molality")
    )
    if value < 0.0:
        raise InvalidInputError(
            f"queries[{index}].salinity_molality", "a salinity cannot be negative"
        )
    return value


def _salinity_wt_fraction(row: dict[str, Any], index: int) -> float:
    value = _finite(
        f"queries[{index}].salinity_wt_fraction",
        _required(row, index, "salinity_wt_fraction"),
    )
    if not 0.0 <= value <= 0.3:
        raise InvalidInputError(
            f"queries[{index}].salinity_wt_fraction",
            "the Batzle-Wang correlation is stated for a weight fraction in [0, 0.3]",
        )
    return value


def _temperature(row: dict[str, Any], index: int) -> float:
    value = _finite(f"queries[{index}].temperature", _required(row, index, "temperature"))
    if value <= 0.0:
        raise InvalidInputError(
            f"queries[{index}].temperature",
            "an absolute temperature; zero and below are not states",
        )
    return value


def _pressure(row: dict[str, Any], index: int) -> float:
    value = _finite(f"queries[{index}].pressure", _required(row, index, "pressure"))
    if value <= 0.0:
        raise InvalidInputError(
            f"queries[{index}].pressure", "an absolute pressure; zero and below are not states"
        )
    return value


def _to_fahrenheit(t_kelvin: float) -> float:
    return t_kelvin * 9.0 / 5.0 - 459.67


def _to_psia(p_pa: float) -> float:
    return (p_pa / 1.0e5) * S.PSIA_PER_BAR


def _water_fvf_mccain(t_k: float, p_pa: float) -> float:
    tf = _to_fahrenheit(t_k)
    pp = _to_psia(p_pa)
    dvwt = -1.0001e-2 + 1.33391e-4 * tf + 5.50654e-7 * tf * tf
    dvwp = (
        -1.95301e-9 * pp * tf - 1.72834e-13 * pp * pp * tf - 3.58922e-7 * pp - 2.25341e-10 * pp * pp
    )
    return (1.0 + dvwt) * (1.0 + dvwp)


def _water_fvf_osif(t_k: float, p_pa: float, ppm: float) -> float:
    tf = _to_fahrenheit(t_k)
    pp = _to_psia(p_pa)
    s = ppm / 1.0e6
    t_shift = tf - 60.0
    numerator = (
        5.1e-8 * pp
        + (tf - 60.0) * (5.47e-6 - 1.95e-10 * pp)
        + t_shift * t_shift * (-3.23e-8)
        + 8.5e-13 * pp * pp
    )
    return 1.0 + numerator - s * (0.0840655 * s * math.sqrt(tf))


def _dead_water_viscosity_mccain(t_k: float, ppm: float) -> float:
    tf = min(max(_to_fahrenheit(t_k), 100.0), 400.0)
    s = ppm / 1.0e4
    a = 109.574 - 8.40564 * s + 0.313314 * s * s + 8.72213e-3 * s * s * s
    b = (
        -1.12166
        + 2.63951e-2 * s
        - 6.79461e-4 * s * s
        - 5.47119e-5 * s * s * s
        + 1.55586e-6 * s * s * s * s
    )
    return a * math.pow(tf, b)


def _water_viscosity_mccain(t_k: float, p_pa: float, ppm: float) -> float:
    pp = _to_psia(p_pa)
    mu_dead = _dead_water_viscosity_mccain(t_k, ppm)
    factor = 0.9994 + 4.0295e-5 * pp + 3.1062e-9 * pp * pp
    return mu_dead * factor


def _water_compressibility_mccain(t_k: float, p_pa: float, ppm: float) -> float:
    tf = _to_fahrenheit(t_k)
    pp = _to_psia(p_pa)
    s = ppm / 1.0e6
    cw_pure = (
        (3.8546 - 0.000134 * pp)
        + (-(0.01052 + 4.77e-7 * pp)) * tf
        + (3.9267e-5 + 8.8e-10 * pp) * tf * tf
    )
    cw_pure = cw_pure * 1.0e-6
    return cw_pure * (1.0 + s * 0.7) * S.PSIA_PER_BAR


def _brine_density_batzle_wang(t_k: float, p_pa: float, s: float) -> float:
    tc = t_k - 273.15
    p_mpa = (p_pa / 1.0e5) * 0.1
    rho_w = 1.0 + 1.0e-6 * (
        -80.0 * tc
        - 3.3 * tc * tc
        + 0.00175 * tc * tc * tc
        + 489.0 * p_mpa
        - 2.0 * tc * p_mpa
        + 0.016 * tc * tc * p_mpa
        - 1.3e-5 * tc * tc * tc * p_mpa
        - 0.333 * p_mpa * p_mpa
        - 0.002 * tc * p_mpa * p_mpa
    )
    rho_w = rho_w * 1000.0
    return (
        rho_w
        + s
        * (
            0.668
            + 0.44 * s
            + 1.0e-6
            * (
                300.0 * p_mpa
                - 2400.0 * p_mpa * s
                + tc * (80.0 + 3.0 * tc - 3300.0 * s - 13.0 * p_mpa + 47.0 * p_mpa * s)
            )
        )
        * 1000.0
    )


def _solution_gas_water_ratio_culberson(t_k: float, p_pa: float, ppm: float) -> float:
    tf = _to_fahrenheit(t_k)
    pp = _to_psia(p_pa)
    a = 8.15839 - 6.12265e-2 * tf + 1.91663e-4 * tf * tf - 2.1654e-7 * tf * tf * tf
    b = 1.01021e-2 - 7.44241e-5 * tf + 3.05553e-7 * tf * tf - 2.94883e-10 * tf * tf * tf
    rsw_pure = a + b * pp
    s = ppm / 1.0e4
    return rsw_pure * (1.0 - (0.0753 - 1.73e-4 * tf) * s) * 0.178108


def _water_gas_surface_tension(t_k: float, p_pa: float) -> float:
    tf = _to_fahrenheit(t_k)
    pp = _to_psia(p_pa)
    sigma74 = max(75.0 - 0.1108 * tf - 6.722e-5 * tf * tf, 1.0)
    if pp <= 1074.0:
        return sigma74
    return sigma74 * max(1.0 - 0.0001 * (pp - 1074.0), 0.2)


def _rsw_mccain(t_k: float, p_pa: float, molality: float) -> float:
    tf = (t_k - 273.15) * 9.0 / 5.0 + 32.0
    pp = _to_psia(p_pa)
    a = 8.15839 - 6.12265e-2 * tf + 1.91663e-4 * tf * tf - 2.1654e-7 * tf * tf * tf
    b = 1.01021e-2 - 7.44241e-5 * tf + 3.05553e-7 * tf * tf - 2.94883e-10 * tf * tf * tf
    c = (
        -9.02505
        + 0.130237 * tf
        - 8.53425e-4 * tf * tf
        + 2.34122e-6 * tf * tf * tf
        - 2.37049e-9 * tf * tf * tf * tf
    ) * 1.0e-7
    rsw_pure = a + b * pp + c * pp * pp
    mass_nacl = molality * 58.44
    salinity_wt_percent = (mass_nacl / (1000.0 + mass_nacl)) * 100.0
    s1 = 2.12 + 3.085e-3 * tf - 3.878e-6 * tf * tf
    s2 = -1.0071e-4 - 1.272e-6 * tf + 9.46e-9 * tf * tf
    s3 = (-5.0e-5 + 4.285e-7 * tf - 1.14e-9 * tf * tf) * 1.0e-4
    cs = (s1 + s2 * pp + s3 * pp * pp) * 1.0e-4
    return max(rsw_pure * math.pow(10.0, -cs * salinity_wt_percent) * 0.178108, 0.0)


_FIELDS = (
    "fvf",
    "viscosity",
    "compressibility",
    "density",
    "rsw",
    "surface_tension",
)


def _row(calculation: str, **fields: float | None) -> WaterResultRow:
    converted: dict[str, Any] = {}
    for name in _FIELDS:
        value = fields.get(name)
        if value is None:
            converted[name] = None
        elif name == "viscosity":
            converted[name] = quantity(value, "Pa*s")
        elif name == "density":
            converted[name] = quantity(value, "kg/m**3")
        elif name == "surface_tension":
            converted[name] = quantity(value, "N/m")
        else:
            converted[name] = value
    return WaterResultRow(calculation=calculation, **converted)


def _one(row: dict[str, Any], index: int) -> WaterResultRow:
    calculation = str(row["calculation"])
    if calculation == "water_fvf_mccain":
        t = _temperature(row, index)
        p = _pressure(row, index)
        return _row(calculation, fvf=_finite(f"queries[{index}].fvf", _water_fvf_mccain(t, p)))
    if calculation == "water_fvf_osif":
        t = _temperature(row, index)
        p = _pressure(row, index)
        ppm = _salinity_ppm(row, index)
        return _row(calculation, fvf=_finite(f"queries[{index}].fvf", _water_fvf_osif(t, p, ppm)))
    if calculation == "dead_water_viscosity_mccain":
        t = _temperature(row, index)
        ppm = _salinity_ppm(row, index)
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity",
                _dead_water_viscosity_mccain(t, ppm) / 1000.0,
            ),
        )
    if calculation == "water_viscosity_mccain":
        t = _temperature(row, index)
        p = _pressure(row, index)
        ppm = _salinity_ppm(row, index)
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity",
                _water_viscosity_mccain(t, p, ppm) / 1000.0,
            ),
        )
    if calculation == "water_compressibility_mccain":
        t = _temperature(row, index)
        p = _pressure(row, index)
        ppm = _salinity_ppm(row, index)
        return _row(
            calculation,
            compressibility=_finite(
                f"queries[{index}].compressibility",
                _water_compressibility_mccain(t, p, ppm) * 1.0e-5,
            ),
        )
    if calculation == "brine_density_batzle_wang":
        t = _temperature(row, index)
        p = _pressure(row, index)
        wt = _salinity_wt_fraction(row, index)
        return _row(
            calculation,
            density=_finite(f"queries[{index}].density", _brine_density_batzle_wang(t, p, wt)),
        )
    if calculation == "water_density":
        t = _temperature(row, index)
        p = _pressure(row, index)
        return _row(
            calculation,
            density=_finite(f"queries[{index}].density", _brine_density_batzle_wang(t, p, 0.0)),
        )
    if calculation == "solution_gas_water_ratio_culberson":
        t = _temperature(row, index)
        p = _pressure(row, index)
        ppm = _salinity_ppm(row, index)
        return _row(
            calculation,
            rsw=_finite(
                f"queries[{index}].rsw",
                _solution_gas_water_ratio_culberson(t, p, ppm),
            ),
        )
    if calculation == "water_gas_surface_tension":
        t = _temperature(row, index)
        p = _pressure(row, index)
        return _row(
            calculation,
            surface_tension=_finite(
                f"queries[{index}].surface_tension",
                _water_gas_surface_tension(t, p) * 1.0e-3,
            ),
        )
    if calculation == "rsw_mccain":
        t = _temperature(row, index)
        p = _pressure(row, index)
        molality = _salinity_molality(row, index)
        return _row(calculation, rsw=_finite(f"queries[{index}].rsw", _rsw_mccain(t, p, molality)))
    raise InvalidInputError(
        f"queries[{index}].calculation", f"{calculation!r} is not one of the water calculations"
    )


def water_correlations(queries: list[dict[str, Any]]) -> WaterCorrelationsResult:
    """The water correlation table over the stated calls.

    Args:
        queries: one water correlation call per row, in caller order.

    Returns:
        One result row per input row, with only the fields the calculation answers.

    Raises:
        InvalidInputError: for an unknown calculation, a missing required field, a
            non-positive temperature or pressure, a negative salinity, a weight fraction
            outside [0, 0.3], or a non-finite answer.
    """
    rows: list[dict[str, Any]] = input_records_to_si(_spec(), "queries", queries)
    warnings: list[Warning] = []
    return WaterCorrelationsResult(
        queries=tuple(_one(row, index) for index, row in enumerate(rows)),
        warnings=tuple(warnings),
    )
