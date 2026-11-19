"""``process.blocked_outlet_overpressure`` - the blocked-outlet pressure build-up.

As NeqSim's ``BlockedOutletOverpressureAnalyzer.run`` computes it. Spec:
``specs/models/process/blocked_outlet_overpressure.toml``

The Python twin of ``crates/azoth-process/src/models/blocked_outlet_overpressure.rs``: a
vessel filling run whose target is the relief set pressure, then the first crossing.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.result import BlockedOutletOverpressureResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.process.reference.vessel_filling import _evaluate as fill_evaluate

MODEL_ID = "process.blocked_outlet_overpressure"


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    relief_set_pressure: float
    max_pressure: float
    final_pressure: float
    final_mass: float
    inlet_mass_flow: float
    simulated_horizon: float
    time_to_relief_set: float | None
    relief_demand: bool


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    inlet_temperature: Q,
    supply_pressure: Q,
    inlet_mass_flow: Q,
    relief_set_pressure: Q,
    time_step: Q | None = None,
    max_time: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    relief = input_to_si(spec, "relief_set_pressure", relief_set_pressure)
    step = 1.0 if time_step is None else input_to_si(spec, "time_step", time_step)
    span = 3600.0 if max_time is None else input_to_si(spec, "max_time", max_time)
    if relief <= 0.0 or not math.isfinite(relief):
        raise InvalidInputError(
            "relief_set_pressure", "the class refuses a non-positive set pressure"
        )
    if step <= 0.0 or span <= 0.0 or not math.isfinite(step) or not math.isfinite(span):
        raise InvalidInputError("time_step", "the class refuses a non-positive step or horizon")

    filling, warnings = fill_evaluate(
        components,
        initial_z,
        initial_t,
        initial_p,
        vessel_volume,
        inlet_temperature,
        supply_pressure,
        inlet_mass_flow,
        relief_set_pressure,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        time_step if time_step is not None else _seconds_quantity(step),
        max_time if max_time is not None else _seconds_quantity(span),
    )
    time_to_relief = None
    for point in filling.points:
        if point[1] >= relief:
            time_to_relief = point[0]
            break
    return (
        _Solution(
            relief_set_pressure=relief,
            max_pressure=filling.max_pressure,
            final_pressure=filling.final_pressure,
            final_mass=filling.final_mass,
            inlet_mass_flow=input_to_si(spec, "inlet_mass_flow", inlet_mass_flow),
            simulated_horizon=span,
            time_to_relief_set=time_to_relief,
            relief_demand=time_to_relief is not None,
        ),
        warnings,
    )


def _seconds_quantity(value: float) -> Q:
    from azoth.core.units import quantity

    return quantity(value, "s")


def blocked_outlet_overpressure(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    inlet_temperature: Q,
    supply_pressure: Q,
    inlet_mass_flow: Q,
    relief_set_pressure: Q,
    time_step: Q | None = None,
    max_time: Q | None = None,
) -> BlockedOutletOverpressureResult:
    """The blocked-outlet pressure build-up to the relief set pressure.

    A vessel filling run whose target is the relief set pressure; the time to demand is the
    first recorded sample at or above it, with no interpolation.

    Args:
        components: the trapped gas's substances.
        initial_z: the trapped gas's composition.
        initial_t: the trapped gas temperature.
        initial_p: the trapped gas pressure.
        vessel_volume: the vessel's internal volume.
        inlet_temperature: the continued feed's stagnation temperature.
        supply_pressure: the feed's supply pressure.
        inlet_mass_flow: the continued feed mass flow.
        relief_set_pressure: the relief device's set pressure.
        time_step: the explicit step; absent is 1 s.
        max_time: the horizon; absent is 3600 s.

    Returns:
        The relief set pressure, the pressure and mass summary, the time to demand and the
        demand flag.

    Raises:
        InvalidInputError: for a non-positive dimension or set pressure, or a feed state the
            filling flash cannot resolve.
    """
    solution, warnings = _evaluate(
        components,
        initial_z,
        initial_t,
        initial_p,
        vessel_volume,
        inlet_temperature,
        supply_pressure,
        inlet_mass_flow,
        relief_set_pressure,
        time_step,
        max_time,
    )
    return BlockedOutletOverpressureResult(
        relief_set_pressure=from_si(solution.relief_set_pressure, "Pa"),
        max_pressure=from_si(solution.max_pressure, "Pa"),
        final_pressure=from_si(solution.final_pressure, "Pa"),
        final_mass=from_si(solution.final_mass, "kg"),
        inlet_mass_flow=from_si(solution.inlet_mass_flow, "kg/s"),
        simulated_horizon=from_si(solution.simulated_horizon, "s"),
        time_to_relief_set=(
            None
            if solution.time_to_relief_set is None
            else from_si(solution.time_to_relief_set, "s")
        ),
        relief_demand=solution.relief_demand,
        warnings=tuple(warnings),
    )
