"""``process.depressurization_acceptance`` - the STS-0131 acceptance of a stored blowdown.

As NeqSim's ``STS0131AcceptanceCriteria.evaluate`` computes it. Spec:
``specs/models/process/depressurization_acceptance.toml``

The Python twin of ``crates/azoth-process/src/models/depressurization_acceptance.rs``: the
limiting time, the interpolated state, the peak rate and the three configured criteria.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import DepressurizationAcceptanceResult
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.depressurization_acceptance"


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    limiting_time: float
    pressure_at_limiting_time: float
    remaining_mass_at_limiting_time: float
    peak_discharge_rate: float
    pressure_configured: bool
    pressure_criterion_met: bool
    mass_configured: bool
    mass_criterion_met: bool
    fire_rate_configured: bool
    fire_rate_criterion_met: bool
    acceptable: bool


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _interpolate(times: list[float], values: list[float], target: float) -> float:
    if target <= times[0]:
        return values[0]
    for index in range(1, len(times)):
        upper = times[index]
        if target <= upper:
            lower = times[index - 1]
            fraction = (target - lower) / (upper - lower) if upper > lower else 0.0
            return values[index - 1] + fraction * (values[index] - values[index - 1])
    return values[-1]


def _evaluate(
    points: list[dict[str, object]],
    time_to_escape: Q | None = None,
    estimated_time_to_rupture: Q | None = None,
    maximum_pressure_at_rupture: Q | None = None,
    maximum_remaining_mass: Q | None = None,
    maximum_escalated_fire_rate: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    rows: list[dict[str, Any]] = input_records_to_si(spec, "points", points)
    if not rows:
        raise InvalidInputError("points", "the class requires at least one time sample")
    for row in rows:
        for field in ("time", "pressure", "mass", "mass_flow"):
            value = float(row[field])
            if not math.isfinite(value):
                raise InvalidInputError(
                    field, "a non-finite sample is not a state this calculation compares"
                )

    times = [float(row["time"]) for row in rows]
    pressures = [float(row["pressure"]) for row in rows]
    masses = [float(row["mass"]) for row in rows]

    def positive(value: Q | None, name: str) -> float | None:
        if value is None:
            return None
        seconds_value = input_to_si(spec, name, value)
        return seconds_value if math.isfinite(seconds_value) and seconds_value > 0.0 else None

    limiting = times[-1]
    for candidate in (
        positive(time_to_escape, "time_to_escape"),
        positive(estimated_time_to_rupture, "estimated_time_to_rupture"),
    ):
        if candidate is not None:
            limiting = min(limiting, candidate)
    limiting = max(limiting, 0.0)
    pressure_value = _interpolate(times, pressures, limiting)
    mass_value = _interpolate(times, masses, limiting)
    peak = 0.0
    for row in rows:
        if float(row["time"]) <= limiting:
            peak = max(peak, float(row["mass_flow"]))

    pressure_limit = (
        None
        if maximum_pressure_at_rupture is None
        else input_to_si(spec, "maximum_pressure_at_rupture", maximum_pressure_at_rupture)
    )
    mass_limit = (
        None
        if maximum_remaining_mass is None
        else input_to_si(spec, "maximum_remaining_mass", maximum_remaining_mass)
    )
    fire_limit = (
        None
        if maximum_escalated_fire_rate is None
        else input_to_si(spec, "maximum_escalated_fire_rate", maximum_escalated_fire_rate)
    )
    pressure_met = pressure_limit is None or pressure_value <= pressure_limit
    mass_met = mass_limit is None or mass_value <= mass_limit
    fire_met = fire_limit is None or peak <= fire_limit

    return (
        _Solution(
            limiting_time=limiting,
            pressure_at_limiting_time=pressure_value,
            remaining_mass_at_limiting_time=mass_value,
            peak_discharge_rate=peak,
            pressure_configured=pressure_limit is not None,
            pressure_criterion_met=pressure_met,
            mass_configured=mass_limit is not None,
            mass_criterion_met=mass_met,
            fire_rate_configured=fire_limit is not None,
            fire_rate_criterion_met=fire_met,
            acceptable=pressure_met and mass_met and fire_met,
        ),
        warnings,
    )


def depressurization_acceptance(
    points: list[dict[str, object]],
    time_to_escape: Q | None = None,
    estimated_time_to_rupture: Q | None = None,
    maximum_pressure_at_rupture: Q | None = None,
    maximum_remaining_mass: Q | None = None,
    maximum_escalated_fire_rate: Q | None = None,
) -> DepressurizationAcceptanceResult:
    """The STS-0131 acceptance of a stored blowdown trajectory.

    The limiting time is the last sample's time reduced by the positive escape and rupture
    times; pressure and mass are linearly interpolated, the peak rate is the maximum at or
    before that time, and each criterion passes when unconfigured or at or below its limit.

    Args:
        points: the trajectory, at least one row, in time order.
        time_to_escape: the personnel escape time; absent is no limit.
        estimated_time_to_rupture: the item's estimated time to rupture; absent is no limit.
        maximum_pressure_at_rupture: the pressure limit; absent is unconfigured.
        maximum_remaining_mass: the inventory limit; absent is unconfigured.
        maximum_escalated_fire_rate: the fire-rate limit; absent is unconfigured.

    Returns:
        The limiting state, the peak rate and the criteria.

    Raises:
        InvalidInputError: for an empty trajectory or a non-finite sample.
    """
    solution, warnings = _evaluate(
        points,
        time_to_escape,
        estimated_time_to_rupture,
        maximum_pressure_at_rupture,
        maximum_remaining_mass,
        maximum_escalated_fire_rate,
    )
    return DepressurizationAcceptanceResult(
        limiting_time=from_si(solution.limiting_time, "s"),
        pressure_at_limiting_time=from_si(solution.pressure_at_limiting_time, "Pa"),
        remaining_mass_at_limiting_time=from_si(solution.remaining_mass_at_limiting_time, "kg"),
        peak_discharge_rate=from_si(solution.peak_discharge_rate, "kg/s"),
        pressure_configured=solution.pressure_configured,
        pressure_criterion_met=solution.pressure_criterion_met,
        mass_configured=solution.mass_configured,
        mass_criterion_met=solution.mass_criterion_met,
        fire_rate_configured=solution.fire_rate_configured,
        fire_rate_criterion_met=solution.fire_rate_criterion_met,
        acceptable=solution.acceptable,
        warnings=tuple(warnings),
    )
