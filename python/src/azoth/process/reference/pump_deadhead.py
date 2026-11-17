"""``process.pump_deadhead`` - the centrifugal deadhead screen.

As NeqSim's ``PumpDeadheadAnalyzer.analyze`` computes it. Spec:
``specs/models/process/pump_deadhead.toml``

The Python twin of ``crates/azoth-process/src/models/pump_deadhead.rs``: the deadhead
pressure, the rating verdict and the minimum-flow recirculation rise.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import DeadheadVerdict, PumpDeadheadResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.pump_deadhead"

#: The class's gravitational acceleration, m/s2.
GRAVITY = 9.80665
#: The class's default shut-off head multiple.
DEFAULT_SHUTOFF_RATIO = 1.20
#: The class's default minimum-flow efficiency.
DEFAULT_MIN_FLOW_EFFICIENCY = 0.30


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    rated: float
    deadhead: float
    margin: float | None
    exceeded: bool
    head: float | None
    rise: float | None
    temp_exceeded: bool
    verdict: DeadheadVerdict


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
    suction_pressure: Q,
    normal_discharge_pressure: Q,
    shutoff_head_ratio: float | None = None,
    protected_pressure_rating: Q | None = None,
    liquid_density: Q | None = None,
    specific_heat: Q | None = None,
    minimum_flow_efficiency: float | None = None,
    max_allowable_temperature_rise: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []

    ps = input_to_si(spec, "suction_pressure", suction_pressure)
    pn = input_to_si(spec, "normal_discharge_pressure", normal_discharge_pressure)
    ratio_input = None if shutoff_head_ratio is None else float(shutoff_head_ratio)
    rating = (
        None
        if protected_pressure_rating is None
        else input_to_si(spec, "protected_pressure_rating", protected_pressure_rating)
    )
    density = (
        None
        if liquid_density is None
        else input_to_si(spec, "liquid_density", liquid_density)
    )
    heat = (
        None
        if specific_heat is None
        else input_to_si(spec, "specific_heat", specific_heat)
    )
    efficiency_input = (
        None if minimum_flow_efficiency is None else float(minimum_flow_efficiency)
    )
    allowable = (
        None
        if max_allowable_temperature_rise is None
        else input_to_si(
            spec, "max_allowable_temperature_rise", max_allowable_temperature_rise
        )
    )
    apply_checks(
        checks_for(spec).on_input,
        {
            "suction_pressure": ps,
            "normal_discharge_pressure": pn,
            "shutoff_head_ratio": ratio_input,
            "protected_pressure_rating": rating,
            "liquid_density": density,
            "specific_heat": heat,
            "minimum_flow_efficiency": efficiency_input,
            "max_allowable_temperature_rise": allowable,
        }.get,
        warnings,
    )

    values = {
        "suction_pressure": ps,
        "normal_discharge_pressure": pn,
        "shutoff_head_ratio": ratio_input,
        "protected_pressure_rating": rating,
        "liquid_density": density,
        "specific_heat": heat,
        "minimum_flow_efficiency": efficiency_input,
        "max_allowable_temperature_rise": allowable,
    }
    for name, value in values.items():
        if value is not None:
            _require_finite(name, value)
    if pn <= ps:
        raise InvalidInputError(
            "normal_discharge_pressure",
            "the normal discharge pressure must be above the suction pressure",
        )

    rated = pn - ps
    ratio = DEFAULT_SHUTOFF_RATIO if ratio_input is None else ratio_input
    shutoff = ratio * rated
    deadhead = ps + shutoff

    if rating is None:
        margin = None
        exceeded = False
        verdict = DeadheadVerdict.NO_RATING
    else:
        margin = rating - deadhead
        exceeded = deadhead > rating
        verdict = (
            DeadheadVerdict.EXCEEDS_RATING
            if exceeded
            else DeadheadVerdict.WITHIN_RATING
        )

    if density is None or heat is None:
        head = None
        rise = None
    else:
        efficiency = (
            DEFAULT_MIN_FLOW_EFFICIENCY
            if efficiency_input is None
            else efficiency_input
        )
        head = shutoff / (density * GRAVITY)
        rise = shutoff * (1.0 - efficiency) / (density * efficiency * heat)

    temp_exceeded = (
        rise is not None and allowable is not None and rise > allowable
    )

    return (
        _Solution(
            rated=rated,
            deadhead=deadhead,
            margin=margin,
            exceeded=exceeded,
            head=head,
            rise=rise,
            temp_exceeded=temp_exceeded,
            verdict=verdict,
        ),
        warnings,
    )


def pump_deadhead(
    suction_pressure: Q,
    normal_discharge_pressure: Q,
    shutoff_head_ratio: float | None = None,
    protected_pressure_rating: Q | None = None,
    liquid_density: Q | None = None,
    specific_heat: Q | None = None,
    minimum_flow_efficiency: float | None = None,
    max_allowable_temperature_rise: Q | None = None,
) -> PumpDeadheadResult:
    """The deadhead pressure, the rating verdict and the minimum-flow rise.

    The deadhead pressure is the suction pressure plus the shut-off multiple of the rated
    differential. **Equality with the protected rating is within it**; the temperature-rise
    test is strict. Without density or specific heat the head and rise are absent, and the
    temperature verdict is false; the class's fluid-derived values are not carried.

    Args:
        suction_pressure: the pump suction pressure.
        normal_discharge_pressure: the normal discharge pressure, above the suction.
        shutoff_head_ratio: the shut-off multiple of the rated differential; absent is 1.20.
        protected_pressure_rating: the protected rating; absent publishes no rating verdict.
        liquid_density: the pumped liquid density.
        specific_heat: the liquid specific heat.
        minimum_flow_efficiency: the efficiency at minimum flow; absent is 0.30.
        max_allowable_temperature_rise: the allowable recirculation rise.

    Returns:
        The rated differential, the deadhead pressure, the rating margin and verdict, the
        shut-off head and the recirculation rise.

    Raises:
        InvalidInputError: for a negative suction pressure, a non-positive discharge
            pressure, a discharge pressure at or below the suction, a shut-off ratio below
            one, a non-positive rating, density, specific heat or allowable rise, an
            efficiency outside ``(0, 1]``, or a non-finite input.
    """
    solution, warnings = _evaluate(
        suction_pressure,
        normal_discharge_pressure,
        shutoff_head_ratio,
        protected_pressure_rating,
        liquid_density,
        specific_heat,
        minimum_flow_efficiency,
        max_allowable_temperature_rise,
    )
    return PumpDeadheadResult(
        rated_differential_pressure=from_si(solution.rated, "Pa"),
        deadhead_pressure=from_si(solution.deadhead, "Pa"),
        pressure_margin=(
            None if solution.margin is None else from_si(solution.margin, "Pa")
        ),
        pressure_rating_exceeded=solution.exceeded,
        shutoff_head=None if solution.head is None else from_si(solution.head, "m"),
        recirculation_temperature_rise=(
            None if solution.rise is None else from_si(solution.rise, "K")
        ),
        temp_rise_exceeded=solution.temp_exceeded,
        verdict=solution.verdict,
        warnings=tuple(warnings),
    )
