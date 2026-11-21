"""``pvt.gas_correlations`` - NeqSim's gas correlation table.

The Python twin of ``crates/azoth-pvt/src/gas_correlations.rs``: the pseudo-critical
correlations, the two Hall-Yarborough variants, the pseudo-pressure integral and the Haaland
H2S estimates.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import GasCorrelationsResult, GasResultRow
from azoth.core.units import input_records_to_si, quantity
from azoth.core.warnings import Warning
from azoth.pvt.reference import _standard as S

MODEL_ID = "pvt.gas_correlations"

_TC_H2S = 672.35
_PC_H2S = 1306.0
_TC_CO2 = 547.58
_PC_CO2 = 1071.0
_TC_N2 = 227.16
_PC_N2 = 493.1


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


def _positive(row: dict[str, Any], index: int, name: str) -> float:
    value = _finite(f"queries[{index}].{name}", _required(row, index, name))
    if value <= 0.0:
        raise InvalidInputError(
            f"queries[{index}].{name}",
            "the correlation divides by this value or takes its logarithm, and it must be positive",
        )
    return value


def _fraction(row: dict[str, Any], index: int, name: str) -> float:
    value = _finite(f"queries[{index}].{name}", _required(row, index, name))
    if not 0.0 <= value <= 1.0:
        raise InvalidInputError(f"queries[{index}].{name}", "a mole fraction lies in [0, 1]")
    return value


def _gamma(row: dict[str, Any], index: int, minimum: float, maximum: float) -> float:
    value = _finite(f"queries[{index}].gamma_g", _required(row, index, "gamma_g"))
    if value < minimum or value > maximum:
        raise InvalidInputError(
            f"queries[{index}].gamma_g",
            f"the correlation is defined for a gas specific gravity in [{minimum}, {maximum}]",
        )
    return value


def _temperature(row: dict[str, Any], index: int, name: str) -> float:
    value = _finite(f"queries[{index}].{name}", _required(row, index, name))
    if value <= 0.0:
        raise InvalidInputError(
            f"queries[{index}].{name}", "an absolute temperature; zero and below are not states"
        )
    return value


def _pressure(row: dict[str, Any], index: int, name: str) -> float:
    value = _finite(f"queries[{index}].{name}", _required(row, index, name))
    if value <= 0.0:
        raise InvalidInputError(
            f"queries[{index}].{name}", "an absolute pressure; zero and below are not states"
        )
    return value


def _molar_mass(row: dict[str, Any], index: int) -> float:
    value = _finite(f"queries[{index}].molar_mass", _required(row, index, "molar_mass"))
    if value <= 0.0:
        raise InvalidInputError(
            f"queries[{index}].molar_mass",
            "a molar mass; zero and below are not states the correlation takes",
        )
    return value


def _standing_tpc(gamma: float) -> float:
    return (168.0 + 325.0 * gamma - 12.5 * gamma * gamma) * 5.0 / 9.0


def _standing_ppc(gamma: float) -> float:
    return (677.0 + 15.0 * gamma - 37.5 * gamma * gamma) / S.PSIA_PER_BAR


def _sutton_tpc(gamma: float) -> float:
    return (169.2 + 349.5 * gamma - 74.0 * gamma * gamma) * 5.0 / 9.0


def _sutton_ppc(gamma: float) -> float:
    return (756.8 - 131.0 * gamma - 3.6 * gamma * gamma) / S.PSIA_PER_BAR


def _piper_jk(gamma: float, y_h2s: float, y_co2: float, y_n2: float) -> tuple[float, float]:
    y_hc = 1.0 - y_h2s - y_co2 - y_n2
    if y_hc <= 0.0:
        y_hc = 1.0
    gamma_hc = (gamma - 0.9672 * y_n2 - 1.1765 * y_co2 - 1.5196 * y_h2s) / y_hc
    if gamma_hc < 0.55:
        gamma_hc = 0.55
    f_h2s_j = y_h2s * (_TC_H2S / _PC_H2S)
    f_co2_j = y_co2 * (_TC_CO2 / _PC_CO2)
    f_n2_j = y_n2 * (_TC_N2 / _PC_N2)
    j = (
        0.11582
        - 0.45820 * f_h2s_j
        - 0.90348 * f_co2_j
        - 0.66026 * f_n2_j
        + 0.70729 * gamma_hc
        - 0.099397 * gamma_hc * gamma_hc
    )
    f_h2s_k = y_h2s * math.sqrt(_TC_H2S / _PC_H2S)
    f_co2_k = y_co2 * math.sqrt(_TC_CO2 / _PC_CO2)
    f_n2_k = y_n2 * math.sqrt(_TC_N2 / _PC_N2)
    k = (
        3.8216
        - 0.06534 * f_h2s_k
        - 0.42113 * f_co2_k
        - 0.91249 * f_n2_k
        + 17.438 * gamma_hc
        - 3.2191 * gamma_hc * gamma_hc
    )
    return j, k


def _piper_tpc(gamma: float, y_h2s: float, y_co2: float, y_n2: float) -> float:
    j, k = _piper_jk(gamma, y_h2s, y_co2, y_n2)
    return k * k / j * 5.0 / 9.0


def _piper_ppc(gamma: float, y_h2s: float, y_co2: float, y_n2: float) -> float:
    j, k = _piper_jk(gamma, y_h2s, y_co2, y_n2)
    return (k * k / j) / j / S.PSIA_PER_BAR


def _wichert_aziz(tpc_k: float, ppc_bara: float, y_h2s: float, y_co2: float) -> tuple[float, float]:
    tpc_r = tpc_k * 9.0 / 5.0
    ppc_psia = ppc_bara * S.PSIA_PER_BAR
    a = y_h2s + y_co2
    b = y_h2s
    epsilon = 120.0 * (math.pow(a, 0.9) - math.pow(a, 1.6)) + 15.0 * (
        math.pow(b, 0.5) - math.pow(b, 4.0)
    )
    tpc_corr_r = tpc_r - epsilon
    ppc_corr_psia = ppc_psia * tpc_corr_r / (tpc_r + b * (1.0 - b) * epsilon)
    return tpc_corr_r * 5.0 / 9.0, ppc_corr_psia / S.PSIA_PER_BAR


def _hall_yarborough_zfactor(tpr: float, ppr: float) -> float:
    if ppr < 1.0e-10:
        return 1.0
    t = 1.0 / tpr
    a = -0.06125 * ppr * t * math.exp(-1.2 * (1.0 - t) * (1.0 - t))
    b = 14.76 * t - 9.76 * t * t + 4.58 * t * t * t
    c = 90.7 * t - 242.2 * t * t + 42.4 * t * t * t
    d = 2.18 + 2.82 * t
    y = 0.001
    for _ in range(200):
        fy = (
            a
            + (y + y * y + y * y * y - y * y * y * y) / math.pow(1.0 - y, 3)
            - b * y * y
            + c * math.pow(y, d)
        )
        dfy = (
            (1.0 + 4.0 * y + 4.0 * y * y - 4.0 * y * y * y + y * y * y * y) / math.pow(1.0 - y, 4)
            - 2.0 * b * y
            + c * d * math.pow(y, d - 1.0)
        )
        y_new = y - fy / dfy
        if y_new < 1.0e-12:
            y_new = 1.0e-12
        if y_new > 0.9999:
            y_new = 0.9999
        if abs(y_new - y) < 1.0e-13:
            y = y_new
            break
        y = y_new
    z = -a / y
    return 1.0 if z < 0.05 or z > 5.0 else z


def _dranchuk_abou_kassem(tpr: float, ppr: float) -> float:
    if ppr < 1.0e-10:
        return 1.0
    a1, a2, a3, a4, a5 = 0.3265, -1.0700, -0.5339, 0.01569, -0.05165
    a6, a7, a8, a9, a10, a11 = 0.5475, -0.7361, 0.1844, 0.1056, 0.6134, 0.7210
    rho = 0.27 * ppr / tpr
    for _ in range(200):
        rr = rho
        rr2 = rr * rr
        c1 = a1 + a2 / tpr + a3 / math.pow(tpr, 3) + a4 / math.pow(tpr, 4) + a5 / math.pow(tpr, 5)
        c2 = a6 + a7 / tpr + a8 / (tpr * tpr)
        c3 = a9 * (a7 / tpr + a8 / (tpr * tpr))
        fz = (
            1.0
            + c1 * rr
            + c2 * rr2
            - c3 * rr2 * rr2 * rr
            + a10 * (1.0 + a11 * rr2) * (rr2 / math.pow(tpr, 3)) * math.exp(-a11 * rr2)
            - 0.27 * ppr / (rho * tpr)
        )
        dfz = (
            c1
            + 2.0 * c2 * rr
            - 5.0 * c3 * rr2 * rr2
            + a10
            * (rr / math.pow(tpr, 3))
            * (2.0 + 4.0 * a11 * rr2 - 2.0 * a11 * a11 * rr2 * rr2)
            * math.exp(-a11 * rr2)
            + 0.27 * ppr / (rho * rho * tpr)
        )
        rho_new = rho - fz / dfz
        if rho_new < 1.0e-15:
            rho_new = 1.0e-15
        if abs(rho_new - rho) < 1.0e-12:
            rho = rho_new
            break
        rho = rho_new
    z = 0.27 * ppr / (rho * tpr)
    return 1.0 if z < 0.05 or z > 5.0 else z


def _papay(tpr: float, ppr: float) -> float:
    z = (
        1.0
        - 3.52 * ppr / math.pow(10.0, 0.9813 * tpr)
        + 0.274 * ppr * ppr / math.pow(10.0, 0.8157 * tpr)
    )
    return max(z, 0.05)


def _z_factor_sutton(p_bara: float, t_k: float, gamma: float) -> float:
    return _hall_yarborough_zfactor(t_k / _sutton_tpc(gamma), p_bara / _sutton_ppc(gamma))


def _z_factor_sour_gas(
    p_bara: float, t_k: float, gamma: float, y_h2s: float, y_co2: float
) -> float:
    tpc_corr, ppc_corr = _wichert_aziz(_sutton_tpc(gamma), _sutton_ppc(gamma), y_h2s, y_co2)
    return _hall_yarborough_zfactor(t_k / tpc_corr, p_bara / ppc_corr)


def _gas_density(p_bara: float, t_k: float, gamma: float, mw_g_mol: float) -> float:
    z = _z_factor_sutton(p_bara, t_k, gamma)
    return p_bara * mw_g_mol / (z * 0.0831447 * t_k)


def _gas_fvf_from_z(p_bara: float, t_k: float, gamma: float) -> float:
    z = _z_factor_sutton(p_bara, t_k, gamma)
    return z * t_k / p_bara * 1.01325 / 288.71


def _lee_gonzalez_eakin_field(t_rankine: float, rho_lb_ft3: float, mg_g_mol: float) -> float:
    k = (9.4 + 0.02 * mg_g_mol) * math.pow(t_rankine, 1.5) / (209.0 + 19.0 * mg_g_mol + t_rankine)
    x = 3.5 + 986.0 / t_rankine + 0.01 * mg_g_mol
    y = 2.4 - 0.2 * x
    return 1.0e-4 * k * math.exp(x * math.pow(rho_lb_ft3 / 62.4, y))


def _hall_yarborough_pseudo(tpr: float, ppr: float) -> float:
    if not (math.isfinite(tpr) and math.isfinite(ppr) and tpr > 1.0 and 0.0 <= ppr < 25.0):
        raise InvalidInputError(
            "tpr",
            "Hall-Yarborough needs finite reduced properties with Tpr > 1.0 and 0 <= Ppr < 25.0",
        )
    if ppr < 1.0e-10:
        return 1.0
    t = 1.0 / tpr
    a0 = -0.06125 * t * math.exp(-1.2 * (1.0 - t) * (1.0 - t))
    b = 14.76 * t - 9.76 * t * t + 4.58 * t * t * t
    c = 90.7 * t - 242.2 * t * t + 42.4 * t * t * t
    d = 2.18 + 2.82 * t
    y = 0.001
    converged = False
    for _ in range(100):
        fy = (
            a0 * ppr
            + (y + y * y + y * y * y - y * y * y * y) / math.pow(1.0 - y, 3)
            - b * y * y
            + c * math.pow(y, d)
        )
        dfy = (
            (1.0 + 4.0 * y + 4.0 * y * y - 4.0 * y * y * y + y * y * y * y) / math.pow(1.0 - y, 4)
            - 2.0 * b * y
            + c * d * math.pow(y, d - 1.0)
        )
        if abs(dfy) < 1.0e-30:
            break
        y_new = y - fy / dfy
        if y_new < 0.0:
            y_new = y / 2.0
        if y_new > 1.0:
            y_new = (y + 1.0) / 2.0
        if abs(y_new - y) < 1.0e-12:
            y = y_new
            converged = True
            break
        y = y_new
    if not converged:
        raise InvalidInputError("tpr", "Hall-Yarborough did not converge at this reduced state")
    z = -a0 * ppr / y
    if not math.isfinite(z) or z <= 0.0:
        raise InvalidInputError(
            "tpr",
            "Hall-Yarborough answered a compressibility factor that is not finite and positive",
        )
    return z


def _integrand(p_bara: float, t_k: float, gamma: float, mw_g_mol: float) -> float:
    t_rankine = t_k * 9.0 / 5.0
    p_psia = p_bara * S.PSIA_PER_BAR
    z = _hall_yarborough_pseudo(t_k / _standing_tpc(gamma), p_bara / _standing_ppc(gamma))
    rho_lb = p_psia * mw_g_mol / (z * 10.7316 * t_rankine)
    mu = _lee_gonzalez_eakin_field(t_rankine, rho_lb, mw_g_mol)
    if not math.isfinite(mu) or mu <= 0.0:
        raise InvalidInputError(
            "molar_mass", "the Lee-Gonzalez-Eakin viscosity must be finite and positive"
        )
    return 2.0 * p_bara / (mu * z)


def _integrate(p_low: float, p_high: float, t_k: float, gamma: float, mw: float) -> float:
    if abs(p_high - p_low) < 1.0e-10:
        return 0.0
    n = 200
    h = (p_high - p_low) / n
    total = _integrand(p_low, t_k, gamma, mw) + _integrand(p_high, t_k, gamma, mw)
    for i in range(1, n):
        p = p_low + i * h
        f = _integrand(p, t_k, gamma, mw)
        total += 2.0 * f if i % 2 == 0 else 4.0 * f
    return h / 3.0 * total


def _pseudo_pressure(
    p_bara: float, p_ref_bara: float, t_k: float, gamma: float, mw_g_mol: float
) -> float:
    if not math.isfinite(p_bara) or p_bara <= 0.0:
        raise InvalidInputError("pressure", "an absolute pressure; zero and below are not states")
    if not math.isfinite(p_ref_bara) or p_ref_bara <= 0.0:
        raise InvalidInputError(
            "reference_pressure", "an absolute pressure; zero and below are not states"
        )
    if not math.isfinite(t_k) or t_k <= 0.0:
        raise InvalidInputError(
            "temperature", "an absolute temperature; zero and below are not states"
        )
    if not math.isfinite(mw_g_mol) or mw_g_mol <= 0.0:
        raise InvalidInputError(
            "molar_mass", "a molar mass; zero and below are not states the correlation takes"
        )
    tpr = t_k / _standing_tpc(gamma)
    max_ppr = max(p_bara, p_ref_bara) / _standing_ppc(gamma)
    if tpr <= 1.0 or max_ppr >= 25.0:
        raise InvalidInputError(
            "tpr", "the correlation needs Tpr > 1.0 and a maximum Ppr below 25.0"
        )
    result = _integrate(p_ref_bara, p_bara, t_k, gamma, mw_g_mol)
    if not math.isfinite(result):
        raise InvalidInputError("pseudo_pressure", "the pseudopressure integral must be finite")
    return result


_FIELDS = (
    "temperature",
    "pressure",
    "factor",
    "density",
    "gas_fvf",
    "pseudo_pressure",
    "h2s_fraction",
)


def _row(calculation: str, **fields: float | None) -> GasResultRow:
    converted: dict[str, Any] = {}
    for name in _FIELDS:
        value = fields.get(name)
        if value is None:
            converted[name] = None
        elif name == "temperature":
            converted[name] = quantity(value, "K")
        elif name == "pressure":
            converted[name] = quantity(value, "Pa")
        elif name == "density":
            converted[name] = quantity(value, "kg/m**3")
        else:
            converted[name] = value
    return GasResultRow(calculation=calculation, **converted)


def _one(row: dict[str, Any], index: int) -> GasResultRow:
    calculation = str(row["calculation"])
    if calculation == "pseudo_critical_temperature_standing":
        gamma = _gamma(row, index, 0.55, 1.8)
        return _row(
            calculation, temperature=_finite(f"queries[{index}].temperature", _standing_tpc(gamma))
        )
    if calculation == "pseudo_critical_pressure_standing":
        gamma = _gamma(row, index, 0.55, 1.8)
        return _row(
            calculation,
            pressure=_finite(f"queries[{index}].pressure", _standing_ppc(gamma) * 1.0e5),
        )
    if calculation == "pseudo_critical_temperature_sutton":
        gamma = _gamma(row, index, 0.57, 1.68)
        return _row(
            calculation, temperature=_finite(f"queries[{index}].temperature", _sutton_tpc(gamma))
        )
    if calculation == "pseudo_critical_pressure_sutton":
        gamma = _gamma(row, index, 0.57, 1.68)
        return _row(
            calculation, pressure=_finite(f"queries[{index}].pressure", _sutton_ppc(gamma) * 1.0e5)
        )
    if calculation == "pseudo_critical_temperature_piper":
        gamma = _positive(row, index, "gamma_g")
        y_h2s = _fraction(row, index, "y_h2s")
        y_co2 = _fraction(row, index, "y_co2")
        y_n2 = _fraction(row, index, "y_n2")
        return _row(
            calculation,
            temperature=_finite(
                f"queries[{index}].temperature", _piper_tpc(gamma, y_h2s, y_co2, y_n2)
            ),
        )
    if calculation == "pseudo_critical_pressure_piper":
        gamma = _positive(row, index, "gamma_g")
        y_h2s = _fraction(row, index, "y_h2s")
        y_co2 = _fraction(row, index, "y_co2")
        y_n2 = _fraction(row, index, "y_n2")
        return _row(
            calculation,
            pressure=_finite(
                f"queries[{index}].pressure", _piper_ppc(gamma, y_h2s, y_co2, y_n2) * 1.0e5
            ),
        )
    if calculation == "wichert_aziz":
        tpc = _temperature(row, index, "tpc")
        ppc = _pressure(row, index, "ppc") / 1.0e5
        y_h2s = _fraction(row, index, "y_h2s")
        y_co2 = _fraction(row, index, "y_co2")
        tpc_corr, ppc_corr = _wichert_aziz(tpc, ppc, y_h2s, y_co2)
        return _row(
            calculation,
            temperature=_finite(f"queries[{index}].temperature", tpc_corr),
            pressure=_finite(f"queries[{index}].pressure", ppc_corr * 1.0e5),
        )
    if calculation == "pseudo_reduced_temperature":
        t = _temperature(row, index, "temperature")
        tpc = _temperature(row, index, "tpc")
        return _row(calculation, factor=_finite(f"queries[{index}].factor", t / tpc))
    if calculation == "pseudo_reduced_pressure":
        p = _pressure(row, index, "pressure")
        ppc = _pressure(row, index, "ppc")
        return _row(calculation, factor=_finite(f"queries[{index}].factor", p / ppc))
    if calculation == "z_factor_hall_yarborough":
        tpr = _positive(row, index, "tpr")
        ppr = _finite(f"queries[{index}].ppr", _required(row, index, "ppr"))
        if ppr < 0.0:
            raise InvalidInputError(
                f"queries[{index}].ppr", "a pseudoreduced pressure cannot be negative"
            )
        return _row(
            calculation,
            factor=_finite(f"queries[{index}].factor", _hall_yarborough_zfactor(tpr, ppr)),
        )
    if calculation == "z_factor_dranchuk_abou_kassem":
        tpr = _positive(row, index, "tpr")
        ppr = _finite(f"queries[{index}].ppr", _required(row, index, "ppr"))
        if ppr < 0.0:
            raise InvalidInputError(
                f"queries[{index}].ppr", "a pseudoreduced pressure cannot be negative"
            )
        return _row(
            calculation, factor=_finite(f"queries[{index}].factor", _dranchuk_abou_kassem(tpr, ppr))
        )
    if calculation == "z_factor_papay":
        tpr = _positive(row, index, "tpr")
        ppr = _finite(f"queries[{index}].ppr", _required(row, index, "ppr"))
        if ppr < 0.0:
            raise InvalidInputError(
                f"queries[{index}].ppr", "a pseudoreduced pressure cannot be negative"
            )
        return _row(calculation, factor=_finite(f"queries[{index}].factor", _papay(tpr, ppr)))
    if calculation == "z_factor_sutton":
        p = _pressure(row, index, "pressure") / 1.0e5
        t = _temperature(row, index, "temperature")
        gamma = _gamma(row, index, 0.57, 1.68)
        return _row(
            calculation, factor=_finite(f"queries[{index}].factor", _z_factor_sutton(p, t, gamma))
        )
    if calculation == "z_factor_sour_gas":
        p = _pressure(row, index, "pressure") / 1.0e5
        t = _temperature(row, index, "temperature")
        gamma = _gamma(row, index, 0.57, 1.68)
        y_h2s = _fraction(row, index, "y_h2s")
        y_co2 = _fraction(row, index, "y_co2")
        return _row(
            calculation,
            factor=_finite(
                f"queries[{index}].factor", _z_factor_sour_gas(p, t, gamma, y_h2s, y_co2)
            ),
        )
    if calculation == "gas_density":
        p = _pressure(row, index, "pressure") / 1.0e5
        t = _temperature(row, index, "temperature")
        gamma = _gamma(row, index, 0.57, 1.68)
        mw = _molar_mass(row, index) * 1000.0
        return _row(
            calculation, density=_finite(f"queries[{index}].density", _gas_density(p, t, gamma, mw))
        )
    if calculation == "gas_fvf_from_z":
        p = _pressure(row, index, "pressure") / 1.0e5
        t = _temperature(row, index, "temperature")
        gamma = _gamma(row, index, 0.57, 1.68)
        return _row(
            calculation, gas_fvf=_finite(f"queries[{index}].gas_fvf", _gas_fvf_from_z(p, t, gamma))
        )
    if calculation == "pseudo_pressure_from_correlation":
        p = _pressure(row, index, "pressure") / 1.0e5
        p_ref = _pressure(row, index, "reference_pressure") / 1.0e5
        t = _temperature(row, index, "temperature")
        gamma = _gamma(row, index, 0.55, 1.8)
        mw = _molar_mass(row, index) * 1000.0
        return _row(
            calculation,
            pseudo_pressure=_finite(
                f"queries[{index}].pseudo_pressure", _pseudo_pressure(p, p_ref, t, gamma, mw)
            ),
        )
    if calculation == "delta_pseudo_pressure":
        p = _pressure(row, index, "pressure") / 1.0e5
        p_ref = _pressure(row, index, "reference_pressure") / 1.0e5
        t = _temperature(row, index, "temperature")
        gamma = _gamma(row, index, 0.55, 1.8)
        mw = _molar_mass(row, index) * 1000.0
        value = _pseudo_pressure(p, p_ref, t, gamma, mw)
        return _row(
            calculation, pseudo_pressure=_finite(f"queries[{index}].pseudo_pressure", abs(value))
        )
    if calculation == "h2s_concentration_from_temperature":
        t = _temperature(row, index, "temperature")
        return _row(
            calculation,
            h2s_fraction=_finite(f"queries[{index}].h2s_fraction", 5.0e7 * math.exp(-6543.0 / t)),
        )
    if calculation == "h2s_concentration_from_co2":
        t = _temperature(row, index, "temperature")
        y_co2 = _positive(row, index, "y_co2")
        return _row(
            calculation,
            h2s_fraction=_finite(
                f"queries[{index}].h2s_fraction",
                math.exp(11.7 - 4438.3 / t + 0.7 * math.log(y_co2 * 100.0)),
            ),
        )
    raise InvalidInputError(
        f"queries[{index}].calculation", f"{calculation!r} is not one of the gas calculations"
    )


def gas_correlations(queries: list[dict[str, Any]]) -> GasCorrelationsResult:
    """The gas correlation table over the stated calls.

    Args:
        queries: one gas correlation call per row, in caller order.

    Returns:
        One result row per input row, with only the fields the calculation answers.

    Raises:
        InvalidInputError: for an unknown calculation, a missing required field, a gas
            gravity outside the correlation's range, a non-positive quantity where the
            correlation divides by it or takes its logarithm, a reduced state
            Hall-Yarborough refuses, or a non-finite answer.
    """
    rows: list[dict[str, Any]] = input_records_to_si(_spec(), "queries", queries)
    warnings: list[Warning] = []
    return GasCorrelationsResult(
        queries=tuple(_one(row, index) for index, row in enumerate(rows)),
        warnings=tuple(warnings),
    )
