"""The PVT regression's parameters and the fluid they build.

Spec: ``specs/models/pvt/regression.toml``. ``PVTRegression`` retunes a characterised
fluid by applying each parameter to a clone and re-running the laboratory experiments
against the measured points. This module is that first step: the twelve carried transforms,
applied to the component rows before the mixture is assembled.

Five of the class's seventeen parameters stay owed. ``plus_molar_mass_multiplier``,
``gamma_alpha`` and ``gamma_eta`` need the mutable characterisation state the row boundary
does not carry; ``viscosity_lbc_multiplier`` and ``viscosity_pedersen_alpha`` need a
viscosity model the PFCT route never selects.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError

from ._fluid import Fluid

#: The twelve parameters the id carries, in the spec's spelling.
PARAMETERS: tuple[str, ...] = (
    "bip_methane_c7plus",
    "bip_c2c6_c7plus",
    "bip_co2_hc",
    "bip_n2_hc",
    "volume_shift_c7plus",
    "tc_multiplier_c7plus",
    "pc_multiplier_c7plus",
    "omega_multiplier_c7plus",
    "viscosity_csp_1",
    "viscosity_csp_2",
    "viscosity_csp_3",
    "viscosity_csp_4",
)

_CSP_INDEX = {
    "viscosity_csp_1": 0,
    "viscosity_csp_2": 1,
    "viscosity_csp_3": 2,
    "viscosity_csp_4": 3,
}

_PENELOUX_INTERCEPT = {"srk": 0.29441, "pr": 0.25969}


def _name(row: Mapping[str, Any]) -> str:
    return str(row["name"]).lower()


def _is_methane(row: Mapping[str, Any]) -> bool:
    return _name(row) in ("methane", "c1")


def _is_nitrogen(row: Mapping[str, Any]) -> bool:
    return _name(row) in ("nitrogen", "n2")


def _is_co2(row: Mapping[str, Any]) -> bool:
    return _name(row) in ("co2", "carbon dioxide")


def _is_c2c6(row: Mapping[str, Any]) -> bool:
    name = _name(row)
    return name in ("ethane", "c2", "propane", "c3", "c4", "c5", "c6") or any(
        part in name for part in ("butane", "pentane", "hexane")
    )


def _bip_overrides(
    rows: Sequence[Mapping[str, Any]], parameter: str, value: float
) -> list[tuple[int, int, float]]:
    plus = [index for index, row in enumerate(rows) if row["pseudo_component"]]
    pairs: list[tuple[int, int, float]] = []
    if parameter == "bip_methane_c7plus":
        index = next((i for i, row in enumerate(rows) if _is_methane(row)), None)
        if index is not None:
            pairs.extend((index, other, value) for other in plus)
    elif parameter == "bip_c2c6_c7plus":
        for index, row in enumerate(rows):
            if _is_c2c6(row):
                pairs.extend((index, other, value) for other in plus)
    elif parameter == "bip_co2_hc":
        index = next((i for i, row in enumerate(rows) if _is_co2(row)), None)
        if index is not None:
            pairs.extend((index, other, value) for other in range(len(rows)) if other != index)
    elif parameter == "bip_n2_hc":
        index = next((i for i, row in enumerate(rows) if _is_nitrogen(row)), None)
        if index is not None:
            pairs.extend((index, other, value) for other in range(len(rows)) if other != index)
    return pairs


def _field(index: int, row: dict[str, Any], field: str) -> float:
    value = row.get(field)
    if value is None:
        raise InvalidInputError(
            f"components[{index}].{field}",
            f"the parameter transforms need a pseudo row's {field}",
        )
    return float(value)


def build_fluid(
    components: Sequence[Mapping[str, Any]],
    *,
    eos: str = "srk",
    kij: str = "database",
    corrected: bool = True,
    parameters: Sequence[Mapping[str, Any]],
    values: Sequence[float],
) -> tuple[Fluid, tuple[float, float, float, float]]:
    """Apply the parameter vector to the rows, then assemble the mixture.

    Returns the fluid and the four PFCT correction factors. The transforms are the
    class's own; see the Rust twin for the volume-shift linearisation.

    Raises:
        InvalidInputError: for a value count that does not match, a non-finite value, or a
            pseudo row a multiplier cannot act on; the assembler's own refusals otherwise.
    """
    if len(parameters) != len(values):
        raise InvalidInputError(
            "parameters", f"{len(parameters)} parameter rows carry {len(values)} values"
        )
    rows = [dict(row) for row in components]
    overrides: list[tuple[int, int, float]] = []
    csp = [1.0, 1.0, 1.0, 1.0]
    for config, value in zip(parameters, values, strict=True):
        parameter = str(config["parameter"])
        if parameter not in PARAMETERS:
            raise InvalidInputError(
                "parameters", f"{parameter!r} is not one of the twelve carried parameters"
            )
        value = float(value)
        if not math.isfinite(value):
            raise InvalidInputError("parameters", f"{parameter} is not finite")
        if parameter in _CSP_INDEX:
            csp[_CSP_INDEX[parameter]] = value
            continue
        if parameter.startswith("bip_"):
            overrides.extend(_bip_overrides(rows, parameter, value))
            continue
        if parameter == "volume_shift_c7plus":
            intercept = _PENELOUX_INTERCEPT.get(eos.lower())
            if intercept is None:
                raise InvalidInputError("eos", f"{eos!r} has no Peneloux translation to scale")
            for index, row in enumerate(rows):
                if not row["pseudo_component"]:
                    continue
                rackett = row.get("racket_z")
                if rackett is None or abs(float(rackett)) < 1e-10:
                    rackett = 0.29056 - 0.08775 * _field(index, row, "acentric_factor")
                row["racket_z"] = intercept - value * (intercept - float(rackett))
            continue
        for index, row in enumerate(rows):
            if not row["pseudo_component"]:
                continue
            if parameter == "tc_multiplier_c7plus":
                row["critical_temperature"] = _field(index, row, "critical_temperature") * value
            elif parameter == "pc_multiplier_c7plus":
                row["critical_pressure"] = _field(index, row, "critical_pressure") * value
            elif parameter == "omega_multiplier_c7plus":
                row["acentric_factor"] = _field(index, row, "acentric_factor") * value
            else:
                raise InvalidInputError("parameters", f"no transform for {parameter!r}")
    fluid = Fluid.from_rows_with_kij(
        rows, eos=eos, kij=kij, kij_overrides=overrides, corrected=corrected
    )
    return fluid, (csp[0], csp[1], csp[2], csp[3])
