"""``pvt.relative_permeability`` - the Corey and LET saturation tables.

The Python twin of ``crates/azoth-pvt/src/relative_permeability.rs``: four Eclipse table
types over the two curve families, with the class's endpoint overwrites and saturation
normalisations.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import RelativePermeabilityResult, RelativePermeabilityRow
from azoth.core.units import quantity, to_count

MODEL_ID = "pvt.relative_permeability"

_TYPES = {
    "swof": ("SWOF", "Sw"),
    "sgof": ("SGOF", "Sg"),
    "sof3": ("SOF3", "So"),
    "slgof": ("SLGOF", "Sl"),
}

_DEFAULTS: dict[str, float] = {
    "swc": 0.0,
    "critical_water_saturation": -1.0,
    "sorw": 0.0,
    "sorg": 0.0,
    "sgcr": 0.0,
    "kro_max": 1.0,
    "krw_max": 1.0,
    "krg_max": 1.0,
    "no": 2.0,
    "nw": 2.0,
    "ng": 2.0,
    "nog": 2.0,
    "lo": 2.0,
    "eo": 1.0,
    "to": 2.0,
    "lw": 2.0,
    "ew": 1.0,
    "tw": 2.0,
    "lg": 2.0,
    "eg": 1.0,
    "tg": 2.0,
    "log": 2.0,
    "eog": 1.0,
    "tog": 2.0,
}


def _clamp(value: float) -> float:
    if value < 0.0:
        return 0.0
    if value > 1.0:
        return 1.0
    return value


def _linspace(start: float, end: float, count: int) -> list[float]:
    if count < 2:
        return [start]
    step = (end - start) / (count - 1)
    values = [start + index * step for index in range(count)]
    values[count - 1] = end
    return values


def _corey(saturation: float, exponent: float) -> float:
    return _clamp(saturation) ** exponent


def _let(saturation: float, l: float, e: float, t: float) -> float:
    s = _clamp(saturation)
    if s <= 0.0:
        return 0.0
    if s >= 1.0:
        return 1.0
    numerator = s**l
    denominator = numerator + e * (1.0 - s) ** t
    if denominator <= 0.0:
        return 0.0
    return numerator / denominator


def _normalize_water_for_krw(sw: float, swcr: float, sorw: float) -> float:
    denominator = 1.0 - swcr - sorw
    if denominator <= 0.0:
        return 0.0
    return _clamp((sw - swcr) / denominator)


def _normalize_water_for_krow(sw: float, swc: float, sorw: float) -> float:
    denominator = 1.0 - swc - sorw
    if denominator <= 0.0:
        return 0.0
    return _clamp((sw - swc) / denominator)


def _normalize_gas_for_krg(sg: float, sgcr: float, swc: float, sorg: float) -> float:
    denominator = 1.0 - swc - sgcr - sorg
    if denominator <= 0.0:
        return 0.0
    return _clamp((sg - sgcr) / denominator)


def _normalize_gas_for_krog(sg: float, sgcr: float, swc: float, sorg: float) -> float:
    denominator = 1.0 - swc - sorg
    if denominator <= 0.0:
        return 0.0
    return _clamp(sg / denominator)


def _parameters(values: dict[str, Any]) -> dict[str, Any]:
    out: dict[str, Any] = {"family": values.get("model_family") or "corey"}
    if out["family"] not in ("corey", "let"):
        raise InvalidInputError(
            "model_family", f"{out['family']!r} is not one of the two curve families"
        )
    for name, default in _DEFAULTS.items():
        value = values.get(name)
        out[name] = default if value is None else float(value)
        if not math.isfinite(out[name]):
            raise InvalidInputError(
                name, "a relative-permeability parameter must be finite"
            )
    return out


def _row(**values: Any) -> RelativePermeabilityRow:
    return RelativePermeabilityRow(
        sw=values.get("sw"),
        sg=values.get("sg"),
        so=values.get("so"),
        sl=values.get("sl"),
        krw=values.get("krw"),
        krg=values.get("krg"),
        krow=values.get("krow"),
        krog=values.get("krog"),
        pcow=None if values.get("pcow") is None else quantity(values["pcow"], "Pa"),
        pcog=None if values.get("pcog") is None else quantity(values["pcog"], "Pa"),
    )


def _swof(p: dict[str, Any], rows: int) -> list[RelativePermeabilityRow]:
    effective_swcr = p["swc"] if p["critical_water_saturation"] < 0.0 else p[
        "critical_water_saturation"
    ]
    max_sw = 1.0 - p["sorw"]
    sw = _linspace(p["swc"], max_sw, rows)
    krw: list[float] = []
    krow: list[float] = []
    for value in sw:
        swn_krw = _normalize_water_for_krw(value, effective_swcr, p["sorw"])
        swn_kro = _normalize_water_for_krow(value, p["swc"], p["sorw"])
        if p["family"] == "let":
            krw.append(p["krw_max"] * _let(swn_krw, p["lw"], p["ew"], p["tw"]))
            krow.append(p["kro_max"] * _let(1.0 - swn_kro, p["lo"], p["eo"], p["to"]))
        else:
            krw.append(p["krw_max"] * _corey(swn_krw, p["nw"]))
            krow.append(p["kro_max"] * _corey(1.0 - swn_kro, p["no"]))
    krw[0] = 0.0
    krow[0] = p["kro_max"]
    krw[rows - 1] = p["krw_max"]
    krow[rows - 1] = 0.0
    return [
        _row(sw=sw[i], krw=krw[i], krow=krow[i], pcow=0.0) for i in range(rows)
    ]


def _gas_curves(p: dict[str, Any], sg: list[float]) -> tuple[list[float], list[float]]:
    krg: list[float] = []
    krog: list[float] = []
    for value in sg:
        sgn_krg = _normalize_gas_for_krg(value, p["sgcr"], p["swc"], p["sorg"])
        sgn_kro = _normalize_gas_for_krog(value, p["sgcr"], p["swc"], p["sorg"])
        if p["family"] == "let":
            krg.append(p["krg_max"] * _let(sgn_krg, p["lg"], p["eg"], p["tg"]))
            krog.append(p["kro_max"] * _let(1.0 - sgn_kro, p["log"], p["eog"], p["tog"]))
        else:
            krg.append(p["krg_max"] * _corey(sgn_krg, p["ng"]))
            krog.append(p["kro_max"] * _corey(1.0 - sgn_kro, p["nog"]))
    return krg, krog


def _sgof(p: dict[str, Any], rows: int) -> list[RelativePermeabilityRow]:
    sg = _linspace(0.0, 1.0 - p["swc"] - p["sorg"], rows)
    krg, krog = _gas_curves(p, sg)
    krg[0] = 0.0
    krog[0] = p["kro_max"]
    krg[rows - 1] = p["krg_max"]
    krog[rows - 1] = 0.0
    return [
        _row(sg=sg[i], krg=krg[i], krog=krog[i], pcog=0.0) for i in range(rows)
    ]


def _sof3(p: dict[str, Any], rows: int) -> list[RelativePermeabilityRow]:
    so = _linspace(max(p["sorw"], p["sorg"]), 1.0 - p["swc"], rows)
    krow: list[float] = []
    krog: list[float] = []
    for value in so:
        son_w = _clamp((value - p["sorw"]) / (1.0 - p["swc"] - p["sorw"]))
        son_g = _clamp((value - p["sorg"]) / (1.0 - p["swc"] - p["sorg"]))
        if p["family"] == "let":
            krow.append(p["kro_max"] * _let(son_w, p["lo"], p["eo"], p["to"]))
            krog.append(p["kro_max"] * _let(son_g, p["log"], p["eog"], p["tog"]))
        else:
            krow.append(p["kro_max"] * _corey(son_w, p["no"]))
            krog.append(p["kro_max"] * _corey(son_g, p["nog"]))
    krow[0] = 0.0
    krog[0] = 0.0
    krow[rows - 1] = p["kro_max"]
    krog[rows - 1] = p["kro_max"]
    return [_row(so=so[i], krow=krow[i], krog=krog[i]) for i in range(rows)]


def _slgof(p: dict[str, Any], rows: int) -> list[RelativePermeabilityRow]:
    sl = _linspace(p["swc"] + p["sorg"], 1.0, rows)
    sg = [1.0 - value for value in sl]
    krg, krog = _gas_curves(p, sg)
    krg[0] = p["krg_max"]
    krog[0] = 0.0
    krg[rows - 1] = 0.0
    krog[rows - 1] = p["kro_max"]
    return [
        _row(sl=sl[i], krg=krg[i], krog=krog[i], pcog=0.0) for i in range(rows)
    ]


def relative_permeability(
    table_type: str | None = None,
    model_family: str | None = None,
    rows: Any = None,
    swc: float | None = None,
    critical_water_saturation: float | None = None,
    sorw: float | None = None,
    sorg: float | None = None,
    sgcr: float | None = None,
    kro_max: float | None = None,
    krw_max: float | None = None,
    krg_max: float | None = None,
    no: float | None = None,
    nw: float | None = None,
    ng: float | None = None,
    nog: float | None = None,
    lo: float | None = None,
    eo: float | None = None,
    to: float | None = None,
    lw: float | None = None,
    ew: float | None = None,
    tw: float | None = None,
    lg: float | None = None,
    eg: float | None = None,
    tg: float | None = None,
    log: float | None = None,
    eog: float | None = None,
    tog: float | None = None,
) -> RelativePermeabilityResult:
    """The relative-permeability table over the stated parameters.

    Returns:
        The table type, the saturation column's name and one row per grid point.

    Raises:
        InvalidInputError: for an unknown table type or family, fewer than three rows, a
            non-finite parameter or a non-finite table value.
    """
    name = table_type or "swof"
    if name not in _TYPES:
        raise InvalidInputError(
            "table_type", f"{name!r} is not one of the four Eclipse table types"
        )
    table_name, saturation_name = _TYPES[name]
    count = 20 if rows is None else to_count(rows, "rows")
    if count < 3:
        raise InvalidInputError(
            "rows", f"the table needs at least three rows, and it was given {count}"
        )
    values = {
        "model_family": model_family,
        "swc": swc,
        "critical_water_saturation": critical_water_saturation,
        "sorw": sorw,
        "sorg": sorg,
        "sgcr": sgcr,
        "kro_max": kro_max,
        "krw_max": krw_max,
        "krg_max": krg_max,
        "no": no,
        "nw": nw,
        "ng": ng,
        "nog": nog,
        "lo": lo,
        "eo": eo,
        "to": to,
        "lw": lw,
        "ew": ew,
        "tw": tw,
        "lg": lg,
        "eg": eg,
        "tg": tg,
        "log": log,
        "eog": eog,
        "tog": tog,
    }
    p = _parameters(values)
    if name == "swof":
        table = _swof(p, count)
    elif name == "sgof":
        table = _sgof(p, count)
    elif name == "sof3":
        table = _sof3(p, count)
    else:
        table = _slgof(p, count)
    result = RelativePermeabilityResult(
        table_type=table_name,
        saturation_name=saturation_name,
        table=tuple(table),
        warnings=(),
    )
    for row in result.table:
        for value in (
            row.sw,
            row.sg,
            row.so,
            row.sl,
            row.krw,
            row.krg,
            row.krow,
            row.krog,
        ):
            if value is not None and not math.isfinite(value):
                raise InvalidInputError(
                    "table", "a generated relative-permeability value is not finite"
                )
    return result
