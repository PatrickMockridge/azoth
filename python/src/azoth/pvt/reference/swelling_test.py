"""``pvt.swelling_test`` - NeqSim's swelling experiment.

The Python twin of ``crates/azoth-pvt/src/swelling_test.rs``: the base oil's bubble-point
volume, and the relative volume after each cumulative gas injection.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any

import pint

from azoth.core.errors import InvalidInputError
from azoth.core.result import SwellingPoint, SwellingTestResult
from azoth.core.units import input_to_si, quantity

from . import _flash, _fluid, _saturation

MODEL_ID = "pvt.swelling_test"
DEFAULT_TEMPERATURE = 298.15


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _cubic(eos: str | None) -> str:
    name = eos or "srk"
    if name not in ("srk", "pr"):
        raise InvalidInputError("eos", f"{name!r} is not one of srk and pr")
    return name


def _kij(mixing_rule: str | None) -> str:
    rule = mixing_rule or "classic"
    if rule not in ("classic", "zero"):
        raise InvalidInputError("mixing_rule", f"{rule!r} is not one of classic and zero")
    return "database" if rule == "classic" else "zero"


def _bubble_point_volume(fluid: _fluid.Fluid, t: float) -> tuple[float, float]:
    pressure = _saturation.bubble_point_pressure(fluid, t)
    flash = fluid.flash(t, pressure, _flash.TWO_PHASE)
    return pressure, flash.volume()


def swelling_test(
    components: Sequence[Mapping[str, Any]],
    injection_gas: Sequence[Mapping[str, Any]] | None = None,
    *,
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    cumulative_gas_injected: Sequence[Any] | None = None,
    volume_correction: bool | None = None,
) -> SwellingTestResult:
    """The swelling of an oil by an injection gas.

    Raises:
        InvalidInputError: for an empty, negative, non-finite or decreasing schedule, an
            injection without a gas, a non-positive temperature, or a failed bubble point.
    """
    if cumulative_gas_injected is None or len(cumulative_gas_injected) == 0:
        raise InvalidInputError(
            "cumulative_gas_injected", "a swelling experiment needs at least one schedule entry"
        )
    schedule = [
        float(value.to_base_units().magnitude) if isinstance(value, pint.Quantity) else float(value)
        for value in cumulative_gas_injected
    ]
    previous = 0.0
    for index, cumulative in enumerate(schedule):
        if not math.isfinite(cumulative) or cumulative < 0.0:
            raise InvalidInputError(
                f"cumulative_gas_injected[{index}]", f"a cumulative injection is {cumulative}"
            )
        if cumulative < previous:
            raise InvalidInputError(
                f"cumulative_gas_injected[{index}]",
                "the schedule is cumulative and cannot decrease",
            )
        previous = cumulative

    injects = schedule[-1] > 0.0
    gas = list(injection_gas or [])
    if injects and not gas:
        raise InvalidInputError(
            "injection_gas", "the schedule injects gas and no injection gas is stated"
        )
    corrected = True if volume_correction is None else volume_correction
    if gas:
        oil, template = _fluid.Fluid.from_rows_pair(
            components,
            gas,
            eos=_cubic(eos),
            kij=_kij(mixing_rule),
            corrected=corrected,
        )
    else:
        oil = _fluid.Fluid.from_rows(
            components, eos=_cubic(eos), kij=_kij(mixing_rule), corrected=corrected
        )
        template = _fluid.GasTemplate(amounts=[0.0] * len(oil.moles), total=0.0)

    if temperature is None:
        t = DEFAULT_TEMPERATURE
    else:
        t = float(input_to_si(_spec(), "temperature", temperature))
    if not t > 0.0:
        raise InvalidInputError(
            "temperature", f"a swelling experiment needs a positive temperature; got {t}"
        )

    original_moles = oil.total_moles()
    original_pressure, original_volume = _bubble_point_volume(oil, t)
    points: list[SwellingPoint] = []
    current = oil
    pressure = original_pressure
    volume = original_volume
    old = 0.0
    for cumulative in schedule:
        increment = cumulative - old
        if increment > 0.0:
            current = current.with_added(template, original_moles * increment)
            pressure, volume = _bubble_point_volume(current, t)
        old = cumulative
        points.append(
            SwellingPoint(
                cumulative_gas_injected=cumulative,
                pressure=quantity(pressure, "Pa"),
                relative_oil_volume=volume / original_volume,
            )
        )

    return SwellingTestResult(
        original_oil_moles=quantity(original_moles, "mol"),
        original_oil_volume=quantity(original_volume, "m**3"),
        points=tuple(points),
        warnings=(),
    )
