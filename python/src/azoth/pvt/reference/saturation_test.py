"""``pvt.saturation_test`` - NeqSim's upper saturation boundary of a fluid.

The Python twin of ``crates/azoth-pvt/src/saturation_test.rs``: the descending scan, the
bisection and the grid-top fallback with its warning.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import SaturationTestResult
from azoth.core.units import input_to_si, quantity

from . import _fluid, _saturation

MODEL_ID = "pvt.saturation_test"


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _scalar(name: str, value: Any) -> float | None:
    if value is None:
        return None
    return float(input_to_si(_spec(), name, value))


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


def saturation_test(
    components: Sequence[Mapping[str, Any]],
    mode: str,
    *,
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    pressure: Any = None,
    temperature_search_min: Any = None,
    temperature_search_max: Any = None,
) -> SaturationTestResult:
    """The upper saturation boundary of a fluid.

    Returns:
        ``saturation_pressure`` in pressure mode or ``saturation_temperature`` in temperature
        mode; the other is absent. An unbracketed search returns the grid top with a warning.

    Raises:
        InvalidInputError: for an unknown mode, ``eos`` or ``mixing_rule``, a missing fixed
            temperature or pressure, one search bound without the other, bounds outside
            ``[30, 1200] K``, or bounds on a pressure search.
    """
    fluid = _fluid.Fluid.from_rows(
        components, eos=_cubic(eos), kij=_kij(mixing_rule), corrected=True
    )
    low = _scalar("temperature_search_min", temperature_search_min)
    high = _scalar("temperature_search_max", temperature_search_max)
    if (low is None) != (high is None):
        raise InvalidInputError(
            "temperature_search", "the search bounds are stated together or not at all"
        )
    bounds = None if low is None or high is None else (low, high)

    if mode == "pressure":
        if bounds is not None:
            raise InvalidInputError(
                "temperature_search",
                "bounds belong to the temperature search, and this is a pressure search",
            )
        fixed = _scalar("temperature", temperature)
        if fixed is None:
            raise InvalidInputError("temperature", "a pressure-mode search needs a temperature")
        point = _saturation.upper_saturation_pressure(fluid, fixed)
        return SaturationTestResult(
            saturation_pressure=(
                None if point.pressure is None else quantity(point.pressure, "Pa")
            ),
            saturation_temperature=None,
            warnings=tuple(point.warnings),
        )
    if mode == "temperature":
        fixed = _scalar("pressure", pressure)
        if fixed is None:
            raise InvalidInputError("pressure", "a temperature-mode search needs a pressure")
        point = _saturation.upper_saturation_temperature(fluid, fixed, bounds)
        return SaturationTestResult(
            saturation_pressure=None,
            saturation_temperature=(
                None if point.temperature is None else quantity(point.temperature, "K")
            ),
            warnings=tuple(point.warnings),
        )
    raise InvalidInputError(
        "mode", f"{mode!r} is not a saturation search; use pressure or temperature"
    )
