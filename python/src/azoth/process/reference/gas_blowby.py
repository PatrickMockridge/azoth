"""``process.gas_blowby`` - the HP-to-LP blowby screen.

As NeqSim's ``GasBlowbyAnalyzer.analyze`` computes it. Spec:
``specs/models/process/gas_blowby.toml``

The Python twin of ``crates/azoth-process/src/models/gas_blowby.rs``: the isentropic nozzle
forms, the 15 C standard volume and the relief-capacity verdict.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import BlowbyVerdict, GasBlowbyResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.gas_blowby"

#: The class's gas constant, J/(mol K).
R_GAS = 8.314462618
#: The class's standard temperature, K (15 C).
STANDARD_TEMPERATURE_K = 288.15
#: The class's standard pressure, Pa (1 atm).
STANDARD_PRESSURE_PA = 101325.0
#: The class's default discharge coefficient.
DEFAULT_DISCHARGE_COEFFICIENT = 0.85


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    choked: bool
    critical_pressure_ratio: float
    actual_pressure_ratio: float
    mass_rate: float
    std_vol_rate: float
    relief_margin: float | None
    relief_adequate: bool
    verdict: BlowbyVerdict


def _require_finite(name: str, value: float) -> None:
    """Refuse a value the arithmetic cannot carry to a published number."""
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    upstream_pressure: Q,
    upstream_temperature: Q,
    downstream_pressure: Q,
    restriction_area: Q,
    specific_heat_ratio: float,
    molar_mass: Q,
    discharge_coefficient: float | None = None,
    downstream_relief_capacity: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []

    p1 = input_to_si(spec, "upstream_pressure", upstream_pressure)
    t1 = input_to_si(spec, "upstream_temperature", upstream_temperature)
    p2 = input_to_si(spec, "downstream_pressure", downstream_pressure)
    area = input_to_si(spec, "restriction_area", restriction_area)
    k = float(specific_heat_ratio)
    m = input_to_si(spec, "molar_mass", molar_mass)
    cd = None if discharge_coefficient is None else float(discharge_coefficient)
    capacity = (
        None
        if downstream_relief_capacity is None
        else input_to_si(spec, "downstream_relief_capacity", downstream_relief_capacity)
    )
    apply_checks(
        checks_for(spec).on_input,
        {
            "upstream_pressure": p1,
            "upstream_temperature": t1,
            "downstream_pressure": p2,
            "restriction_area": area,
            "specific_heat_ratio": k,
            "molar_mass": m,
            "discharge_coefficient": cd,
            "downstream_relief_capacity": capacity,
        }.get,
        warnings,
    )

    _require_finite("upstream_pressure", p1)
    _require_finite("upstream_temperature", t1)
    _require_finite("downstream_pressure", p2)
    _require_finite("restriction_area", area)
    _require_finite("specific_heat_ratio", k)
    _require_finite("molar_mass", m)
    if cd is not None:
        _require_finite("discharge_coefficient", cd)
    if capacity is not None:
        _require_finite("downstream_relief_capacity", capacity)
    if p2 >= p1:
        raise InvalidInputError(
            "downstream_pressure",
            "the downstream pressure must be below the upstream pressure for blowby",
        )

    critical = (2.0 / (k + 1.0)) ** (k / (k - 1.0))
    actual = p2 / p1
    choked = actual <= critical
    coefficient = DEFAULT_DISCHARGE_COEFFICIENT if cd is None else cd
    if choked:
        mass_rate = (
            coefficient
            * area
            * p1
            * math.sqrt(
                k
                * m
                / (R_GAS * t1)
                * (2.0 / (k + 1.0)) ** ((k + 1.0) / (k - 1.0))
            )
        )
    else:
        term = max(0.0, actual ** (2.0 / k) - actual ** ((k + 1.0) / k))
        mass_rate = (
            coefficient
            * area
            * p1
            * math.sqrt(2.0 * m / (R_GAS * t1) * (k / (k - 1.0)) * term)
        )
    std_vol_rate = (mass_rate / m) * R_GAS * STANDARD_TEMPERATURE_K / STANDARD_PRESSURE_PA

    if capacity is None:
        relief_margin = None
        relief_adequate = False
        verdict = BlowbyVerdict.NO_RELIEF_DATA
    else:
        relief_margin = capacity - mass_rate
        relief_adequate = relief_margin >= 0.0
        verdict = (
            BlowbyVerdict.RELIEF_ADEQUATE
            if relief_adequate
            else BlowbyVerdict.RELIEF_INADEQUATE
        )

    return (
        _Solution(
            choked=choked,
            critical_pressure_ratio=critical,
            actual_pressure_ratio=actual,
            mass_rate=mass_rate,
            std_vol_rate=std_vol_rate,
            relief_margin=relief_margin,
            relief_adequate=relief_adequate,
            verdict=verdict,
        ),
        warnings,
    )


def gas_blowby(
    upstream_pressure: Q,
    upstream_temperature: Q,
    downstream_pressure: Q,
    restriction_area: Q,
    specific_heat_ratio: float,
    molar_mass: Q,
    discharge_coefficient: float | None = None,
    downstream_relief_capacity: Q | None = None,
) -> GasBlowbyResult:
    """The blowby rate through an open restriction and the relief verdict.

    The isentropic nozzle forms: the flow is choked at or below the critical ratio, and the
    slower subcritical form applies above it. The standard volume is at 288.15 K and
    101325 Pa - **15 C, not 0 C**. A stated capacity at least the blowby rate is adequate;
    a missing capacity publishes no relief data rather than a zero capacity.

    Args:
        upstream_pressure: the high-pressure source pressure.
        upstream_temperature: the upstream temperature.
        downstream_pressure: the low-pressure system's pressure, below the source.
        restriction_area: the effective opening area.
        specific_heat_ratio: the gas ``Cp/Cv``, above one.
        molar_mass: the gas molar mass.
        discharge_coefficient: the opening's coefficient; absent is 0.85.
        downstream_relief_capacity: the capacity available to absorb the blowby.

    Returns:
        The two ratio checks, the mass and standard-volume rates, the relief margin and
        the verdict.

    Raises:
        InvalidInputError: for a non-positive source pressure, temperature, area or molar
            mass, a negative downstream pressure or relief capacity, a specific-heat ratio at
            or below one, a discharge coefficient outside ``(0, 1]``, a downstream pressure
            at or above the source, or a non-finite input.
    """
    solution, warnings = _evaluate(
        upstream_pressure,
        upstream_temperature,
        downstream_pressure,
        restriction_area,
        specific_heat_ratio,
        molar_mass,
        discharge_coefficient,
        downstream_relief_capacity,
    )
    return GasBlowbyResult(
        choked=solution.choked,
        critical_pressure_ratio=solution.critical_pressure_ratio,
        actual_pressure_ratio=solution.actual_pressure_ratio,
        blowby_mass_rate=from_si(solution.mass_rate, "kg/s"),
        blowby_std_vol_rate=from_si(solution.std_vol_rate, "m**3/s"),
        relief_margin=(
            None
            if solution.relief_margin is None
            else from_si(solution.relief_margin, "kg/s")
        ),
        relief_adequate=solution.relief_adequate,
        verdict=solution.verdict,
        warnings=tuple(warnings),
    )
