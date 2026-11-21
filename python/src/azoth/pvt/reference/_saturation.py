"""The two saturation searches, and the bubble point the swelling test restarts from.

The Python twin of ``crates/azoth-pvt/src/saturation.rs``: a 10-unit descending scan for the
uppermost crossing, a bisection to ``1e-5``, and the classes' grid-top fallback carried as a
warning rather than turned into a refusal.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.core.warnings import Warning, WarningCode
from azoth.eos import bubble_pressure

from . import _flash

SATURATION_PRESSURE_MIN_BAR = 1.0
SATURATION_PRESSURE_MAX_BAR = 1000.0
SATURATION_PRESSURE_STEP_BAR = 10.0
SATURATION_PRESSURE_TOLERANCE_BAR = 1.0e-5
SATURATION_TEMPERATURE_MIN_K = 30.0
SATURATION_TEMPERATURE_MAX_K = 1200.0
SATURATION_TEMPERATURE_STEP_K = 10.0
SATURATION_TEMPERATURE_TOLERANCE_K = 1.0e-5
SATURATION_BISECTION_LIMIT = 500


def bar_to_pa(bar: float) -> float:
    return bar * 1.0e5


@dataclass
class SaturationPoint:
    """An upper saturation boundary."""

    pressure: float | None = None
    temperature: float | None = None
    warnings: list[Warning] = field(default_factory=list)


def upper_saturation_pressure(fluid: Any, t: float) -> SaturationPoint:
    """The uppermost saturation pressure at a temperature."""

    def two_phase(pressure_bar: float) -> bool:
        return bool(fluid.flash(t, bar_to_pa(pressure_bar), _flash.MULTIPHASE).phase_count > 1)

    higher_bar = SATURATION_PRESSURE_MAX_BAR
    higher_two = two_phase(higher_bar)
    steps = int(
        (SATURATION_PRESSURE_MAX_BAR - SATURATION_PRESSURE_MIN_BAR) // SATURATION_PRESSURE_STEP_BAR
    )
    for step in range(steps, -1, -1):
        trial_bar = SATURATION_PRESSURE_MIN_BAR + step * SATURATION_PRESSURE_STEP_BAR
        if trial_bar >= higher_bar:
            continue
        trial_two = two_phase(trial_bar)
        if trial_two and not higher_two:
            return SaturationPoint(
                pressure=bar_to_pa(_refine_pressure(two_phase, trial_bar, higher_bar))
            )
        higher_bar = trial_bar
        higher_two = trial_two
    return SaturationPoint(
        pressure=bar_to_pa(SATURATION_PRESSURE_MAX_BAR),
        warnings=[
            Warning(
                WarningCode.OUT_OF_VALID_RANGE,
                "no two-phase region is bracketed between "
                f"{SATURATION_PRESSURE_MIN_BAR} and {SATURATION_PRESSURE_MAX_BAR} bar, so the "
                "search's top is the answer",
                "saturation_pressure",
            )
        ],
    )


def _refine_pressure(two_phase: Any, two_phase_pressure: float, single_pressure: float) -> float:
    low = two_phase_pressure
    high = single_pressure
    iteration = 0
    while True:
        iteration += 1
        trial = (low + high) / 2.0
        if two_phase(trial):
            low = trial
        else:
            high = trial
        if (
            abs(high - low) <= SATURATION_PRESSURE_TOLERANCE_BAR
            or iteration >= SATURATION_BISECTION_LIMIT
        ):
            break
    return high


def upper_saturation_temperature(
    fluid: Any, p: float, bounds: tuple[float, float] | None = None
) -> SaturationPoint:
    """The uppermost saturation temperature at a pressure, optionally inside stated bounds."""
    minimum, maximum = bounds or (SATURATION_TEMPERATURE_MIN_K, SATURATION_TEMPERATURE_MAX_K)
    if not (
        minimum >= SATURATION_TEMPERATURE_MIN_K
        and maximum <= SATURATION_TEMPERATURE_MAX_K
        and minimum < maximum
    ):
        raise InvalidInputError(
            "temperature_search",
            f"the bounds must satisfy {SATURATION_TEMPERATURE_MIN_K} <= minimum < maximum <= "
            f"{SATURATION_TEMPERATURE_MAX_K} K; got {minimum} and {maximum}",
        )

    def two_phase(temperature: float) -> bool:
        return bool(fluid.flash(temperature, p, _flash.MULTIPHASE).phase_count > 1)

    bounded = minimum != SATURATION_TEMPERATURE_MIN_K or maximum != SATURATION_TEMPERATURE_MAX_K
    if bounded:
        found = _search_temperature(two_phase, minimum, maximum, True)
        if found is not None:
            return SaturationPoint(temperature=found)
    found = _search_temperature(
        two_phase,
        SATURATION_TEMPERATURE_MIN_K,
        SATURATION_TEMPERATURE_MAX_K,
        False,
    )
    if found is not None:
        return SaturationPoint(temperature=found)
    return SaturationPoint(
        temperature=SATURATION_TEMPERATURE_MAX_K,
        warnings=[
            Warning(
                WarningCode.OUT_OF_VALID_RANGE,
                "no two-phase region is bracketed between "
                f"{SATURATION_TEMPERATURE_MIN_K} and {SATURATION_TEMPERATURE_MAX_K} K, so the "
                "search's top is the answer",
                "saturation_temperature",
            )
        ],
    )


def _search_temperature(
    two_phase: Any, minimum: float, maximum: float, require_single_upper_endpoint: bool
) -> float | None:
    higher = maximum
    higher_two = two_phase(higher)
    if require_single_upper_endpoint and higher_two:
        return None
    trial = SATURATION_TEMPERATURE_MIN_K + SATURATION_TEMPERATURE_STEP_K * math.floor(
        (maximum - SATURATION_TEMPERATURE_MIN_K) / SATURATION_TEMPERATURE_STEP_K
    )
    if trial >= higher:
        trial -= SATURATION_TEMPERATURE_STEP_K
    while higher > minimum:
        trial = max(minimum, trial)
        trial_two = two_phase(trial)
        if trial_two and not higher_two:
            return _refine_temperature(two_phase, trial, higher)
        higher = trial
        higher_two = trial_two
        trial -= SATURATION_TEMPERATURE_STEP_K
    return None


def _refine_temperature(
    two_phase: Any, two_phase_temperature: float, single_temperature: float
) -> float:
    low = two_phase_temperature
    high = single_temperature
    iteration = 0
    while True:
        iteration += 1
        trial = (low + high) / 2.0
        if two_phase(trial):
            low = trial
        else:
            high = trial
        if (
            abs(high - low) <= SATURATION_TEMPERATURE_TOLERANCE_K
            or iteration >= SATURATION_BISECTION_LIMIT
        ):
            break
    return high


def bubble_point_pressure(fluid: Any, t: float) -> float:
    """The bubble-point pressure of the fluid at a temperature, in Pa."""
    return bubble_pressure(fluid.mixture, quantity(t, "K"), fluid.z()).pressure.to("Pa").magnitude
