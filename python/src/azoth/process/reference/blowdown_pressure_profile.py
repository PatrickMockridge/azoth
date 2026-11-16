"""``process.blowdown_pressure_profile`` - the profile lookup.

As NeqSim's ``BlowdownPressureProfile`` computes it. Spec:
``specs/models/process/blowdown_pressure_profile.toml``

The Python twin of ``crates/azoth-process/src/models/blowdown_pressure_profile.rs``: the
piecewise step or linear lookup, its absolute ``1e-10`` second window, both endpoint clamps
and the gauge offset.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import BlowdownPressureProfileResult
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.blowdown_pressure_profile"

#: The class's absolute near-equality on seconds.
_NEARLY_EQUAL_SECONDS = 1.0e-10
#: The class's default absolute-to-gauge offset, bar.
_DEFAULT_GAUGE_OFFSET_BAR = 1.0
#: Pascals in a bar.
_PA_PER_BAR = 100_000.0


class _Solution(NamedTuple):
    """The two pressures in SI (Pa), before the units are attached."""

    pressure_absolute: float
    pressure_gauge: float


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
    points: list[dict[str, object]],
    time: Q,
    interpolation_mode: str | None = None,
    gauge_offset: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    rows = input_records_to_si(spec, "points", points)
    query = input_to_si(spec, "time", time)
    stated_offset = None if gauge_offset is None else input_to_si(spec, "gauge_offset", gauge_offset)
    apply_checks(
        checks.on_input,
        {"time": query, "gauge_offset": stated_offset}.get,
        warnings,
    )

    _require_finite("time", query)
    offset_pa = (
        _DEFAULT_GAUGE_OFFSET_BAR * _PA_PER_BAR if stated_offset is None else stated_offset
    )
    _require_finite("gauge_offset", offset_pa)
    mode = "step_previous" if interpolation_mode is None else interpolation_mode
    if mode not in ("step_previous", "linear"):
        raise InvalidInputError(
            "interpolation_mode", f"`{mode}` is not `step_previous` or `linear`"
        )
    linear = mode == "linear"

    if len(rows) < 2:
        raise InvalidInputError(
            "points", "the class refuses a profile with fewer than two time/pressure points"
        )
    previous = -math.inf
    for row in rows:
        row_time = float(row["time"])
        row_pressure = float(row["pressure"])
        _require_finite("points.time", row_time)
        _require_finite("points.pressure", row_pressure)
        if row_time < 0.0:
            raise InvalidInputError("points.time", "the class refuses a negative profile time")
        if row_pressure <= 0.0:
            raise InvalidInputError(
                "points.pressure", "the class refuses a non-positive absolute pressure"
            )
        if row_time <= previous:
            raise InvalidInputError(
                "points.time", "the class requires strictly increasing profile times"
            )
        previous = row_time

    absolute_pa = _lookup(rows, query, linear) * _PA_PER_BAR
    return _Solution(absolute_pa, absolute_pa - offset_pa), warnings


def _lookup(rows: list[dict[str, object]], query: float, linear: bool) -> float:
    """The class's piecewise lookup, in bara, from the SI rows."""
    bara = [float(row["pressure"]) / _PA_PER_BAR for row in rows]
    times = [float(row["time"]) for row in rows]
    last = len(times) - 1
    if query <= times[0]:
        return bara[0]
    if query >= times[last]:
        return bara[last]
    answer = bara[last]
    for index in range(1, len(times)):
        if abs(query - times[index]) < _NEARLY_EQUAL_SECONDS:
            answer = bara[index]
            break
        if query < times[index]:
            if linear:
                fraction = (query - times[index - 1]) / (times[index] - times[index - 1])
                answer = bara[index - 1] + fraction * (bara[index] - bara[index - 1])
            else:
                answer = bara[index - 1]
            break
    return answer


def blowdown_pressure_profile(
    points: list[dict[str, object]],
    time: Q,
    interpolation_mode: str | None = None,
    gauge_offset: Q | None = None,
) -> BlowdownPressureProfileResult:
    """The profile's absolute and gauge pressure at one query time.

    Args:
        points: the profile's ``{time, pressure}`` rows, in increasing time.
        time: the query time.
        interpolation_mode: ``step_previous`` (absent) or ``linear``.
        gauge_offset: the absolute-to-gauge offset; absent is 1 bar.

    Returns:
        The absolute pressure and the absolute minus the offset.

    Raises:
        InvalidInputError: for fewer than two points, a negative or non-finite time, a
            non-positive or non-finite pressure, times that are not strictly increasing, a
            negative query time, a negative gauge offset, an unknown mode, or a non-finite
            value.
    """
    solution, warnings = _evaluate(points, time, interpolation_mode, gauge_offset)
    return BlowdownPressureProfileResult(
        pressure_absolute=from_si(solution.pressure_absolute, "bar"),
        pressure_gauge=from_si(solution.pressure_gauge, "bar"),
        warnings=tuple(warnings),
    )
