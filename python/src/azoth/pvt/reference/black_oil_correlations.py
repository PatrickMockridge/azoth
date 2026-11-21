"""``pvt.black_oil_correlations`` - NeqSim's black-oil correlation table.

The Python twin of ``crates/azoth-pvt/src/black_oil_correlations.rs``: the 28 calculations of
``BlackOilCorrelations`` and ``SaturationPressureCorrelation``, the two classes' conversion
literals kept apart, and the ``SaturationPressureCorrelation`` clamp carried rather than
repaired.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import BlackOilCorrelationsResult, BlackOilResultRow
from azoth.core.units import input_records_to_si, quantity
from azoth.core.warnings import Warning
from azoth.pvt.reference import _standard as S

MODEL_ID = "pvt.black_oil_correlations"


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _finite(name: str, value: float) -> float:
    if not math.isfinite(value):
        raise InvalidInputError(
            name,
            "the correlation answered a non-finite value; the inputs are outside its state",
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


def _temperature(row: dict[str, Any], index: int) -> float:
    value = _finite(f"queries[{index}].temperature", _required(row, index, "temperature"))
    if value <= 0.0:
        raise InvalidInputError(
            f"queries[{index}].temperature",
            "an absolute temperature; zero and below are not states",
        )
    return value


def _pressure(row: dict[str, Any], index: int, name: str) -> float:
    value = _finite(f"queries[{index}].{name}", _required(row, index, name))
    if value <= 0.0:
        raise InvalidInputError(
            f"queries[{index}].{name}", "an absolute pressure; zero and below are not states"
        )
    return value


def _celsius(t_kelvin: float) -> float:
    return t_kelvin - S.CELSIUS_OFFSET_K


def _fahrenheit(t_kelvin: float) -> float:
    return S.celsius_to_fahrenheit(_celsius(t_kelvin))


def _psia(p_pa: float) -> float:
    return (p_pa / 1.0e5) * S.PSIA_PER_BAR


def _p_sat_pa(p_psia: float) -> float:
    return p_psia * S.SATURATION_BAR_PER_PSIA * 1.0e5


def _sm3_to_scf(rs: float) -> float:
    return rs / S.SATURATION_SM3_PER_SCF_STB


def _standing(rs: float, gamma_g: float, api: float, t_k: float) -> float:
    rs_field = rs / S.SM3_PER_SCF_STB
    t = _fahrenheit(t_k)
    a = 0.00091 * t - 0.0125 * api
    return 18.2 * (math.pow(rs_field / gamma_g, 0.83) * math.pow(10.0, a) - 1.4) / S.PSIA_PER_BAR


def _standing_by_specific_gravity(rs: float, gamma_g: float, gamma_o: float, t_k: float) -> float:
    rs_field = rs / S.SM3_PER_SCF_STB
    t = _fahrenheit(t_k)
    api = 141.5 / gamma_o - 131.5
    a = 0.00091 * t - 0.0125 * api
    return 18.2 * (math.pow(rs_field / gamma_g, 0.83) * math.pow(10.0, a) - 1.4) / S.PSIA_PER_BAR


def _vasquez_beggs(rs: float, gamma_g: float, api: float, t_k: float) -> float:
    rs_field = rs / S.SM3_PER_SCF_STB
    t = _fahrenheit(t_k)
    c1, c2, c3 = (0.0362, 1.0937, 25.724) if api <= 30.0 else (0.0178, 1.187, 23.931)
    pb = math.pow(rs_field / (c1 * gamma_g * math.exp(c3 * api / (t + 460.0))), 1.0 / c2)
    return pb / S.PSIA_PER_BAR


def _glaso_by_specific_gravity(rs: float, gamma_g: float, api: float, t_k: float) -> float:
    rs_field = rs / S.SM3_PER_SCF_STB
    t = _fahrenheit(t_k)
    gamma_o = 141.5 / (api + 131.5)
    a = math.pow(rs_field / gamma_g, 0.816) * math.pow(t, 0.172) / math.pow(gamma_o, 0.989)
    log_a = math.log10(a)
    log_pb = 1.7669 + 1.7447 * log_a - 0.30218 * log_a * log_a
    return math.pow(10.0, log_pb) / S.PSIA_PER_BAR


def _p_sat_standing(rs: float, gamma_g: float, api: float, t_k: float) -> float:
    rs_field = _sm3_to_scf(rs)
    t = _fahrenheit(t_k)
    x = 0.00091 * t - 0.0125 * api
    factor = math.pow(rs_field / gamma_g, 0.83) * math.pow(10.0, x)
    return max(18.2 * (factor - 1.4), 14.7)


def _p_sat_vasquez_beggs(rs: float, gamma_g: float, api: float, t_k: float) -> float:
    rs_field = _sm3_to_scf(rs)
    t = _fahrenheit(t_k)
    c1, c2, c3 = (0.0362, 1.0937, 25.724) if api <= 30.0 else (0.0178, 1.187, 23.931)
    exponent = -c3 * api / (t + S.RANKINE_OFFSET)
    return max(math.pow(rs_field / (c1 * gamma_g * math.exp(exponent)), 1.0 / c2), 14.7)


def _p_sat_glaso(rs: float, gamma_g: float, api: float, t_k: float) -> float:
    rs_field = _sm3_to_scf(rs)
    t = _fahrenheit(t_k)
    pb_star = math.pow(rs_field / gamma_g, 0.816) * math.pow(t, 0.172) / math.pow(api, 0.989)
    log_pb_star = math.log10(pb_star)
    log_pb = 1.7669 + 1.7447 * log_pb_star - 0.30218 * log_pb_star * log_pb_star
    return max(math.pow(10.0, log_pb), 14.7)


def _p_sat_petrosky_farshad(rs: float, gamma_g: float, api: float, t_k: float) -> float:
    rs_field = _sm3_to_scf(rs)
    t = _fahrenheit(t_k)
    x = 0.0007916 * math.pow(api, 1.5410) - 0.0004561 * math.pow(t, 1.3911)
    factor = math.pow(rs_field, 0.8439) * math.pow(10.0, x)
    return max(112.727 * (factor / math.pow(gamma_g, 0.8439) - 12.340), 14.7)


def _p_sat_al_marhoun(rs: float, gamma_g: float, gamma_o: float, t_k: float) -> float:
    rs_field = _sm3_to_scf(rs)
    t_r = _fahrenheit(t_k) + S.RANKINE_OFFSET
    return max(
        5.38088e-3
        * math.pow(rs_field, 0.715082)
        * math.pow(gamma_g, -1.87784)
        * math.pow(gamma_o, 3.1437)
        * math.pow(t_r, 1.32657),
        14.7,
    )


def _solution_gor_standing(p_pa: float, gamma_g: float, api: float, t_k: float) -> float:
    p_field = _psia(p_pa)
    t = _fahrenheit(t_k)
    a = 0.00091 * t - 0.0125 * api
    rs = gamma_g * math.pow((p_field / 18.2 + 1.4) * math.pow(10.0, -a), 1.2048)
    return rs * S.SM3_PER_SCF_STB


def _solution_gor_vasquez_beggs(p_pa: float, gamma_g: float, api: float, t_k: float) -> float:
    p_field = _psia(p_pa)
    t = _fahrenheit(t_k)
    c1, c2, c3 = (0.0362, 1.0937, 25.724) if api <= 30.0 else (0.0178, 1.187, 23.931)
    rs = c1 * gamma_g * math.pow(p_field, c2) * math.exp(c3 * api / (t + 460.0))
    return rs * S.SM3_PER_SCF_STB


def _oil_fvf_standing(rs: float, gamma_g: float, gamma_o: float, t_k: float) -> float:
    rs_field = rs / S.SM3_PER_SCF_STB
    t = _fahrenheit(t_k)
    f = rs_field * math.sqrt(gamma_g / gamma_o) + 1.25 * t
    return 0.9759 + 0.00012 * math.pow(f, 1.2)


def _oil_fvf_vasquez_beggs(rs: float, gamma_g: float, api: float, t_k: float) -> float:
    rs_field = rs / S.SM3_PER_SCF_STB
    t = _fahrenheit(t_k)
    c1, c2, c3 = (4.677e-4, 1.751e-5, -1.811e-8) if api <= 30.0 else (4.670e-4, 1.100e-5, 1.337e-9)
    return 1.0 + c1 * rs_field + (t - 60.0) * (api / gamma_g) * (c2 + c3 * rs_field)


def _oil_fvf_undersaturated(bob: float, co_pa: float, p_pa: float, pb_pa: float) -> float:
    co_psi = (co_pa * 1.0e5) / S.PSIA_PER_BAR
    return bob * math.exp(-co_psi * (_psia(p_pa) - _psia(pb_pa)))


def _oil_compressibility(rs: float, gamma_g: float, api: float, t_k: float, p_pa: float) -> float:
    rs_field = rs / S.SM3_PER_SCF_STB
    t = _fahrenheit(t_k)
    p_field = _psia(p_pa)
    co_psi = (-1433.0 + 5.0 * rs_field + 17.2 * t - 1180.0 * gamma_g + 12.61 * api) / (
        1.0e5 * p_field
    )
    return co_psi * S.PSIA_PER_BAR


def _dead_oil_viscosity_beggs_robinson(api: float, t_k: float) -> float:
    t = _fahrenheit(t_k)
    z = 3.0324 - 0.02023 * api
    y = math.pow(10.0, z)
    x = y * math.pow(t, -1.163)
    return math.pow(10.0, x) - 1.0


def _dead_oil_viscosity_glaso(api: float, t_k: float) -> float:
    t = _fahrenheit(t_k)
    a = 10.313 * math.log10(t) - 36.447
    return 3.141e10 * math.pow(t, -3.444) * math.pow(math.log10(api), a)


def _dead_oil_viscosity_kartoatmodjo(api: float, t_k: float) -> float:
    t = _fahrenheit(t_k)
    a = 5.7526 * math.log10(t) - 26.9718
    return 16.0e8 * math.pow(t, -2.8177) * math.pow(math.log10(api), a)


def _saturated_oil_viscosity_beggs_robinson(mu_od: float, rs: float) -> float:
    mu_field = S.pa_s_to_centipoise(mu_od)
    rs_field = rs / S.SM3_PER_SCF_STB
    a = 10.715 * math.pow(rs_field + 100.0, -0.515)
    b = 5.44 * math.pow(rs_field + 150.0, -0.338)
    return (a * math.pow(mu_field, b)) / 1000.0


def _saturated_oil_viscosity_kartoatmodjo(mu_od: float, rs: float) -> float:
    mu_field = S.pa_s_to_centipoise(mu_od)
    rs_field = rs / S.SM3_PER_SCF_STB
    a = -0.06821 + 0.9824 / (rs_field + 100.0) + 40.34e-5 * rs_field
    b = 0.2001 + 0.8428 / (rs_field + 100.0) - 0.000845 * rs_field
    return (math.pow(10.0, a) * math.pow(mu_field, b)) / 1000.0


def _undersaturated_oil_viscosity_vasquez_beggs(mu_ob: float, p_pa: float, pb_pa: float) -> float:
    mu_field = S.pa_s_to_centipoise(mu_ob)
    p = _psia(p_pa)
    pb = _psia(pb_pa)
    m = 2.6 * math.pow(p, 1.187) * math.exp(-11.513 - 8.98e-5 * p)
    return (mu_field * math.pow(p / pb, m)) / 1000.0


def _undersaturated_oil_viscosity_bergman_sutton(mu_ob: float, p_pa: float, pb_pa: float) -> float:
    mu_field = S.pa_s_to_centipoise(mu_ob)
    p = _psia(p_pa)
    pb = _psia(pb_pa)
    ln_mu = math.log(mu_field)
    a = 6.5698e-7 * ln_mu * ln_mu - 1.48211e-5 * ln_mu + 2.27877e-4
    b = 2.24623e-2 * ln_mu + 0.873204
    return (mu_field * math.exp(a * math.pow(p - pb, b))) / 1000.0


def _gas_fvf(p_pa: float, t_k: float, z: float) -> float:
    return 0.02827 * z * (_fahrenheit(t_k) + S.RANKINE_OFFSET) / _psia(p_pa)


def _gas_fvf_rb_per_mscf(p_pa: float, t_k: float, z: float) -> float:
    return 5.04 * z * (_fahrenheit(t_k) + S.RANKINE_OFFSET) / _psia(p_pa)


def _gas_viscosity_lee_gonzalez_eakin(t_k: float, rho: float, molar_mass: float) -> float:
    t_r = _fahrenheit(t_k) + S.RANKINE_OFFSET
    rho_field = S.kg_m3_to_lb_ft3(rho)
    mg = molar_mass * 1000.0
    k = (9.4 + 0.02 * mg) * math.pow(t_r, 1.5) / (209.0 + 19.0 * mg + t_r)
    x = 3.5 + 986.0 / t_r + 0.01 * mg
    y = 2.4 - 0.2 * x
    return 1.0e-4 * k * math.exp(x * math.pow(rho_field / 62.4, y))


_FIELDS = (
    "pressure",
    "solution_gor",
    "oil_fvf",
    "compressibility",
    "viscosity",
    "gas_fvf",
    "factor",
    "average",
    "minimum",
    "maximum",
    "std_dev",
    "standing",
    "vasquez_beggs",
    "glaso",
    "petrosky_farshad",
)
_PRESSURE_FIELDS = frozenset(
    {
        "pressure",
        "average",
        "minimum",
        "maximum",
        "std_dev",
        "standing",
        "vasquez_beggs",
        "glaso",
        "petrosky_farshad",
    }
)


def _row(calculation: str, **fields: float | None) -> BlackOilResultRow:
    converted: dict[str, Any] = {}
    for name in _FIELDS:
        value = fields.get(name)
        if value is None:
            converted[name] = None
        elif name == "viscosity":
            converted[name] = quantity(value, "Pa*s")
        elif name in _PRESSURE_FIELDS:
            converted[name] = quantity(value, "Pa")
        else:
            converted[name] = value
    return BlackOilResultRow(calculation=calculation, **converted)


def _one(row: dict[str, Any], index: int) -> BlackOilResultRow:
    calculation = str(row["calculation"])
    if calculation == "api_from_specific_gravity":
        sg = _positive(row, index, "gamma_o")
        return _row(calculation, factor=_finite(f"queries[{index}].factor", 141.5 / sg - 131.5))
    if calculation == "specific_gravity_from_api":
        api = _positive(row, index, "api")
        return _row(calculation, factor=_finite(f"queries[{index}].factor", 141.5 / (api + 131.5)))
    if calculation == "bubble_point_standing":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            pressure=_finite(f"queries[{index}].pressure", _standing(rs, gamma_g, api, t) * 1.0e5),
        )
    if calculation == "bubble_point_standing_by_specific_gravity":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        gamma_o = _positive(row, index, "gamma_o")
        t = _temperature(row, index)
        return _row(
            calculation,
            pressure=_finite(
                f"queries[{index}].pressure",
                _standing_by_specific_gravity(rs, gamma_g, gamma_o, t) * 1.0e5,
            ),
        )
    if calculation == "bubble_point_vasquez_beggs":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            pressure=_finite(
                f"queries[{index}].pressure", _vasquez_beggs(rs, gamma_g, api, t) * 1.0e5
            ),
        )
    if calculation == "bubble_point_glaso_by_specific_gravity":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            pressure=_finite(
                f"queries[{index}].pressure",
                _glaso_by_specific_gravity(rs, gamma_g, api, t) * 1.0e5,
            ),
        )
    if calculation == "bubble_point_standing_p_sat":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            pressure=_finite(
                f"queries[{index}].pressure", _p_sat_pa(_p_sat_standing(rs, gamma_g, api, t))
            ),
        )
    if calculation == "bubble_point_vasquez_beggs_p_sat":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            pressure=_finite(
                f"queries[{index}].pressure", _p_sat_pa(_p_sat_vasquez_beggs(rs, gamma_g, api, t))
            ),
        )
    if calculation == "bubble_point_glaso_p_sat":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            pressure=_finite(
                f"queries[{index}].pressure", _p_sat_pa(_p_sat_glaso(rs, gamma_g, api, t))
            ),
        )
    if calculation == "bubble_point_petrosky_farshad":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            pressure=_finite(
                f"queries[{index}].pressure",
                _p_sat_pa(_p_sat_petrosky_farshad(rs, gamma_g, api, t)),
            ),
        )
    if calculation == "bubble_point_al_marhoun":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        gamma_o = _positive(row, index, "gamma_o")
        t = _temperature(row, index)
        return _row(
            calculation,
            pressure=_finite(
                f"queries[{index}].pressure", _p_sat_pa(_p_sat_al_marhoun(rs, gamma_g, gamma_o, t))
            ),
        )
    if calculation == "bubble_point_statistics":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        values = [
            _p_sat_standing(rs, gamma_g, api, t),
            _p_sat_vasquez_beggs(rs, gamma_g, api, t),
            _p_sat_glaso(rs, gamma_g, api, t),
            _p_sat_petrosky_farshad(rs, gamma_g, api, t),
        ]
        mean = sum(values) / 4.0
        variance = sum((value - mean) * (value - mean) for value in values) / 4.0
        return _row(
            calculation,
            average=_finite(f"queries[{index}].average", _p_sat_pa(mean)),
            minimum=_finite(f"queries[{index}].minimum", _p_sat_pa(min(values))),
            maximum=_finite(f"queries[{index}].maximum", _p_sat_pa(max(values))),
            std_dev=_finite(f"queries[{index}].std_dev", _p_sat_pa(math.sqrt(variance))),
            standing=_finite(f"queries[{index}].standing", _p_sat_pa(values[0])),
            vasquez_beggs=_finite(f"queries[{index}].vasquez_beggs", _p_sat_pa(values[1])),
            glaso=_finite(f"queries[{index}].glaso", _p_sat_pa(values[2])),
            petrosky_farshad=_finite(f"queries[{index}].petrosky_farshad", _p_sat_pa(values[3])),
        )
    if calculation == "solution_gor_standing":
        p = _pressure(row, index, "pressure")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            solution_gor=_finite(
                f"queries[{index}].solution_gor", _solution_gor_standing(p, gamma_g, api, t)
            ),
        )
    if calculation == "solution_gor_vasquez_beggs":
        p = _pressure(row, index, "pressure")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            solution_gor=_finite(
                f"queries[{index}].solution_gor", _solution_gor_vasquez_beggs(p, gamma_g, api, t)
            ),
        )
    if calculation == "oil_fvf_standing":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        gamma_o = _positive(row, index, "gamma_o")
        t = _temperature(row, index)
        return _row(
            calculation,
            oil_fvf=_finite(
                f"queries[{index}].oil_fvf", _oil_fvf_standing(rs, gamma_g, gamma_o, t)
            ),
        )
    if calculation == "oil_fvf_vasquez_beggs":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            oil_fvf=_finite(
                f"queries[{index}].oil_fvf", _oil_fvf_vasquez_beggs(rs, gamma_g, api, t)
            ),
        )
    if calculation == "oil_fvf_undersaturated":
        bob = _positive(row, index, "bob")
        co = _finite(f"queries[{index}].compressibility", _required(row, index, "compressibility"))
        p = _pressure(row, index, "pressure")
        pb = _pressure(row, index, "pb")
        return _row(
            calculation,
            oil_fvf=_finite(f"queries[{index}].oil_fvf", _oil_fvf_undersaturated(bob, co, p, pb)),
        )
    if calculation == "oil_compressibility_vasquez_beggs":
        rs = _positive(row, index, "rs")
        gamma_g = _positive(row, index, "gamma_g")
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        p = _pressure(row, index, "pressure")
        return _row(
            calculation,
            compressibility=_finite(
                f"queries[{index}].compressibility",
                _oil_compressibility(rs, gamma_g, api, t, p) * 1.0e-5,
            ),
        )
    if calculation == "dead_oil_viscosity_beggs_robinson":
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity", _dead_oil_viscosity_beggs_robinson(api, t) / 1000.0
            ),
        )
    if calculation == "dead_oil_viscosity_glaso":
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity", _dead_oil_viscosity_glaso(api, t) / 1000.0
            ),
        )
    if calculation == "dead_oil_viscosity_kartoatmodjo":
        api = _positive(row, index, "api")
        t = _temperature(row, index)
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity", _dead_oil_viscosity_kartoatmodjo(api, t) / 1000.0
            ),
        )
    if calculation == "saturated_oil_viscosity_beggs_robinson":
        mu = _positive(row, index, "mu_od")
        rs = _positive(row, index, "rs")
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity", _saturated_oil_viscosity_beggs_robinson(mu, rs)
            ),
        )
    if calculation == "saturated_oil_viscosity_kartoatmodjo":
        mu = _positive(row, index, "mu_od")
        rs = _positive(row, index, "rs")
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity", _saturated_oil_viscosity_kartoatmodjo(mu, rs)
            ),
        )
    if calculation == "undersaturated_oil_viscosity_vasquez_beggs":
        mu = _positive(row, index, "mu_ob")
        p = _pressure(row, index, "pressure")
        pb = _pressure(row, index, "pb")
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity",
                _undersaturated_oil_viscosity_vasquez_beggs(mu, p, pb),
            ),
        )
    if calculation == "undersaturated_oil_viscosity_bergman_sutton":
        mu = _positive(row, index, "mu_ob")
        p = _pressure(row, index, "pressure")
        pb = _pressure(row, index, "pb")
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity",
                _undersaturated_oil_viscosity_bergman_sutton(mu, p, pb),
            ),
        )
    if calculation == "gas_fvf":
        p = _pressure(row, index, "pressure")
        t = _temperature(row, index)
        z = _positive(row, index, "z")
        return _row(calculation, gas_fvf=_finite(f"queries[{index}].gas_fvf", _gas_fvf(p, t, z)))
    if calculation == "gas_fvf_rb_per_mscf":
        p = _pressure(row, index, "pressure")
        t = _temperature(row, index)
        z = _positive(row, index, "z")
        return _row(
            calculation, gas_fvf=_finite(f"queries[{index}].gas_fvf", _gas_fvf_rb_per_mscf(p, t, z))
        )
    if calculation == "gas_viscosity_lee_gonzalez_eakin":
        t = _temperature(row, index)
        rho = _positive(row, index, "rho_g")
        molar_mass = _positive(row, index, "molar_mass")
        return _row(
            calculation,
            viscosity=_finite(
                f"queries[{index}].viscosity",
                _gas_viscosity_lee_gonzalez_eakin(t, rho, molar_mass) / 1000.0,
            ),
        )
    raise InvalidInputError(
        f"queries[{index}].calculation", f"{calculation!r} is not one of the black-oil calculations"
    )


def black_oil_correlations(queries: list[dict[str, Any]]) -> BlackOilCorrelationsResult:
    """The black-oil correlation table over the stated calls.

    Args:
        queries: one correlation call per row, in caller order.

    Returns:
        One result row per input row, with only the fields the calculation answers.

    Raises:
        InvalidInputError: for an unknown calculation, a missing required field, a
            non-positive value where the correlation divides by it or takes its
            logarithm, or a non-finite answer.
    """
    rows: list[dict[str, Any]] = input_records_to_si(_spec(), "queries", queries)
    warnings: list[Warning] = []
    return BlackOilCorrelationsResult(
        queries=tuple(_one(row, index) for index, row in enumerate(rows)),
        warnings=tuple(warnings),
    )
