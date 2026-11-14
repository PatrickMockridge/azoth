"""``process.fire_monitor_coverage`` - the fire-monitor coverage screen.

As NeqSim's ``FireMonitorCoverage`` computes it. Spec:
``specs/models/process/fire_monitor_coverage.toml``

The Python twin of ``crates/azoth-process/src/models/fire_monitor_coverage.rs``: the
density, the wind drift, the coverage fraction and the class's verdict.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import FireMonitorCoverageResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.fire_monitor_coverage"

_DEFAULT_MONITOR_COUNT = 1.0
_DEFAULT_MONITOR_FLOW_LPM = 2400.0
_DEFAULT_EFFECTIVE_THROW_M = 40.0
_DEFAULT_SWEEP_ANGLE_DEG = 90.0
_DEFAULT_WIND_SPEED_M_S = 0.0
_DEFAULT_FALL_HEIGHT_M = 10.0
_DEFAULT_TERMINAL_VELOCITY_M_S = 8.0

_VERDICT_OBSTRUCTED = (
    "NOT SUITABLE - line of sight to part of the target is obstructed; a monitor cannot wet "
    "surfaces it cannot see"
)
_VERDICT_STILL_AIR = "NOT SUITABLE - required density not achieved even in still air"
_VERDICT_MARGINAL = (
    "MARGINAL - density achieved in still air only; wind drift takes it below the requirement"
)
_VERDICT_FEASIBLE = "FEASIBLE ON DENSITY - carry forward to a trajectory and shadowing study"


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    total_flow: float
    swept_area: float
    nominal_density: float
    drift: float
    fraction: float
    effective_density: float
    still_air: bool
    in_wind: bool
    needed: float | None
    verdict: str


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
    target_area: Q,
    required_density: float,
    monitor_count: float | None = None,
    monitor_flow: Q | None = None,
    effective_throw: Q | None = None,
    sweep_angle: float | None = None,
    wind_speed: Q | None = None,
    fall_height: Q | None = None,
    droplet_terminal_velocity: Q | None = None,
    characteristic_dimension: Q | None = None,
    line_of_sight_obstructed: bool | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    area = input_to_si(spec, "target_area", target_area)
    density = float(required_density)
    stated_count = None if monitor_count is None else float(monitor_count)
    stated_flow = None if monitor_flow is None else input_to_si(spec, "monitor_flow", monitor_flow)
    stated_throw = (
        None if effective_throw is None else input_to_si(spec, "effective_throw", effective_throw)
    )
    stated_sweep = None if sweep_angle is None else float(sweep_angle)
    stated_wind = None if wind_speed is None else input_to_si(spec, "wind_speed", wind_speed)
    stated_fall = None if fall_height is None else input_to_si(spec, "fall_height", fall_height)
    stated_terminal = (
        None
        if droplet_terminal_velocity is None
        else input_to_si(spec, "droplet_terminal_velocity", droplet_terminal_velocity)
    )
    stated_dimension = (
        None
        if characteristic_dimension is None
        else input_to_si(spec, "characteristic_dimension", characteristic_dimension)
    )
    apply_checks(
        checks.on_input,
        {
            "target_area": area,
            "required_density": density,
            "monitor_count": stated_count,
            "monitor_flow": stated_flow,
            "effective_throw": stated_throw,
            "sweep_angle": stated_sweep,
            "wind_speed": stated_wind,
            "fall_height": stated_fall,
            "droplet_terminal_velocity": stated_terminal,
            "characteristic_dimension": stated_dimension,
        }.get,
        warnings,
    )

    _require_finite("target_area", area)
    _require_finite("required_density", density)
    count = _DEFAULT_MONITOR_COUNT if stated_count is None else stated_count
    _require_finite("monitor_count", count)
    if count != math.floor(count):
        raise InvalidInputError(
            "monitor_count",
            "the class's `setMonitors` takes an integer count; a fractional monitor is not a "
            "placement",
        )
    flow_lpm = _DEFAULT_MONITOR_FLOW_LPM if stated_flow is None else stated_flow * 60000.0
    _require_finite("monitor_flow", flow_lpm)
    throw_m = _DEFAULT_EFFECTIVE_THROW_M if stated_throw is None else stated_throw
    _require_finite("effective_throw", throw_m)
    sweep = _DEFAULT_SWEEP_ANGLE_DEG if stated_sweep is None else stated_sweep
    _require_finite("sweep_angle", sweep)
    wind = _DEFAULT_WIND_SPEED_M_S if stated_wind is None else stated_wind
    _require_finite("wind_speed", wind)
    fall = _DEFAULT_FALL_HEIGHT_M if stated_fall is None else stated_fall
    _require_finite("fall_height", fall)
    terminal = _DEFAULT_TERMINAL_VELOCITY_M_S if stated_terminal is None else stated_terminal
    _require_finite("droplet_terminal_velocity", terminal)
    if stated_dimension is not None:
        _require_finite("characteristic_dimension", stated_dimension)
    obstructed = bool(line_of_sight_obstructed)

    total_flow_lpm = count * flow_lpm
    swept_area = count * (math.pi * throw_m * throw_m * sweep / 360.0)
    nominal_density = total_flow_lpm / area
    drift = fall * wind / terminal
    dimension = math.sqrt(area) if stated_dimension is None else stated_dimension
    fraction = min(1.0, max(0.0, 1.0 - drift / dimension))
    effective_density = nominal_density * fraction
    still_air = nominal_density >= density
    in_wind = effective_density >= density
    needed = None if fraction <= 0.0 else density * area / fraction
    if obstructed:
        verdict = _VERDICT_OBSTRUCTED
    elif not still_air:
        verdict = _VERDICT_STILL_AIR
    elif not in_wind:
        verdict = _VERDICT_MARGINAL
    else:
        verdict = _VERDICT_FEASIBLE

    return (
        _Solution(
            total_flow=total_flow_lpm / 60000.0,
            swept_area=swept_area,
            nominal_density=nominal_density,
            drift=drift,
            fraction=fraction,
            effective_density=effective_density,
            still_air=still_air,
            in_wind=in_wind,
            needed=None if needed is None else needed / 60000.0,
            verdict=verdict,
        ),
        warnings,
    )


def fire_monitor_coverage(
    target_area: Q,
    required_density: float,
    monitor_count: float | None = None,
    monitor_flow: Q | None = None,
    effective_throw: Q | None = None,
    sweep_angle: float | None = None,
    wind_speed: Q | None = None,
    fall_height: Q | None = None,
    droplet_terminal_velocity: Q | None = None,
    characteristic_dimension: Q | None = None,
    line_of_sight_obstructed: bool | None = None,
) -> FireMonitorCoverageResult:
    """The density, the wind drift, the coverage fraction and the class's verdict.

    Args:
        target_area: the target area.
        required_density: the required density in ``L/(min m2)``.
        monitor_count: the whole monitor count; absent is 1.
        monitor_flow: one monitor's flow; absent is 2400 L/min.
        effective_throw: the throw; absent is 40 m.
        sweep_angle: the sweep in degrees; absent is 90.
        wind_speed: the wind speed; absent is 0.
        fall_height: the fall height; absent is 10 m.
        droplet_terminal_velocity: the terminal velocity; absent is 8 m/s.
        characteristic_dimension: the dimension along the wind; absent is
            ``sqrt(target_area)``.
        line_of_sight_obstructed: whether the sight is blocked; absent is false.

    Returns:
        The flows, the densities, the drift, the fraction and the verdict.

    Raises:
        InvalidInputError: for a non-positive area, density, flow, throw, fall height or
            terminal velocity, a count below one or not whole, an angle outside
            ``(0, 360]``, a negative wind speed or dimension, or a non-finite input.
    """
    solution, warnings = _evaluate(
        target_area,
        required_density,
        monitor_count,
        monitor_flow,
        effective_throw,
        sweep_angle,
        wind_speed,
        fall_height,
        droplet_terminal_velocity,
        characteristic_dimension,
        line_of_sight_obstructed,
    )
    return FireMonitorCoverageResult(
        total_flow=from_si(solution.total_flow, "L/min"),
        swept_area=from_si(solution.swept_area, "m**2"),
        nominal_density=solution.nominal_density,
        drift_displacement=from_si(solution.drift, "m"),
        wind_coverage_fraction=solution.fraction,
        effective_density=solution.effective_density,
        meets_density_still_air=solution.still_air,
        meets_density_in_wind=solution.in_wind,
        flow_needed_for_compliance=(
            None if solution.needed is None else from_si(solution.needed, "L/min")
        ),
        verdict=solution.verdict,
        warnings=tuple(warnings),
    )
