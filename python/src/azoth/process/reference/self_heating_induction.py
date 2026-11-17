"""``process.self_heating_induction`` - the one-dimensional induction-time march.

As NeqSim's ``SelfHeatingInductionSolver.solve`` computes it. Spec:
``specs/models/process/self_heating_induction.toml``

The Python twin of ``crates/azoth-process/src/models/self_heating_induction.rs``.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    SelfHeatingGeometry,
    SelfHeatingInductionResult,
    SelfHeatingTimePoint,
)
from azoth.core.units import Q, from_si, input_to_si, to_count
from azoth.core.warnings import Warning
from azoth.process.reference._selfheating import (
    GEOMETRIES,
    R_GAS,
    one_dimensional,
    shape_factor_of,
)

MODEL_ID = "process.self_heating_induction"

#: The class's step cap.
MAX_STEPS = 20_000_000

#: The retained-history cap, after which the list is decimated and the stride doubles.
MAX_HISTORY_POINTS = 500

#: The safety factor on the conduction stability limit.
CONDUCTION_SAFETY = 0.4

#: The maximum fraction of the Frank-Kamenetskii temperature scale per step.
REACTION_STEP_FRACTION = 0.02

#: The default ignition rise, K.
DEFAULT_IGNITION_RISE_K = 100.0

#: The default simulated-time cap, s (30 days).
DEFAULT_MAX_TIME_S = 30.0 * 24.0 * 3600.0


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _max_of(values: list[float]) -> float:
    peak = values[0]
    for value in values[1:]:
        if value > peak:
            peak = value
    return peak


def _decimate(history: list[SelfHeatingTimePoint]) -> None:
    kept = history[::2]
    history.clear()
    history.extend(kept)


def self_heating_induction(
    geometry: str,
    characteristic_dimension: Q,
    effective_conductivity: Q,
    volumetric_heat_capacity: float,
    activation_energy: Q,
    volumetric_pre_factor: float,
    boundary_temperature: Q,
    initial_temperature: Q | None = None,
    ignition_rise: Q | None = None,
    max_time: Q | None = None,
    node_count: float | None = None,
) -> SelfHeatingInductionResult:
    """The one-dimensional induction-time march for a self-heating body.

    Raises:
        InvalidInputError: for an unknown or non-one-dimensional geometry, a non-positive
            dimension, conductivity, heat capacity, activation energy, pre-factor or
            boundary temperature, a node count below 5, or a non-positive ignition rise or
            maximum time.
    """
    spec = _spec()
    warnings: list[Warning] = []
    dimension = input_to_si(spec, "characteristic_dimension", characteristic_dimension)
    conductivity = input_to_si(spec, "effective_conductivity", effective_conductivity)
    heat_capacity = float(volumetric_heat_capacity)
    energy = input_to_si(spec, "activation_energy", activation_energy)
    pre_factor = float(volumetric_pre_factor)
    boundary = input_to_si(spec, "boundary_temperature", boundary_temperature)
    start = (
        boundary
        if initial_temperature is None
        else input_to_si(spec, "initial_temperature", initial_temperature)
    )
    rise = (
        DEFAULT_IGNITION_RISE_K
        if ignition_rise is None
        else input_to_si(spec, "ignition_rise", ignition_rise)
    )
    time_cap = DEFAULT_MAX_TIME_S if max_time is None else input_to_si(spec, "max_time", max_time)
    count = None if node_count is None else to_count(node_count, "node_count")
    apply_checks(
        checks_for(spec).on_input,
        {
            "characteristic_dimension": dimension,
            "effective_conductivity": conductivity,
            "volumetric_heat_capacity": heat_capacity,
            "activation_energy": energy,
            "volumetric_pre_factor": pre_factor,
            "boundary_temperature": boundary,
            "initial_temperature": None if initial_temperature is None else start,
            "ignition_rise": None if ignition_rise is None else rise,
            "max_time": None if max_time is None else time_cap,
            "node_count": None if count is None else float(count),
        }.get,
        warnings,
    )

    shape = str(geometry)
    if shape not in GEOMETRIES:
        raise InvalidInputError(
            "geometry",
            f"`{shape}` is not `slab`, `infinite_cylinder` or `sphere`",
        )
    if not one_dimensional(shape):
        raise InvalidInputError(
            "geometry",
            f"`{shape}` is not one-dimensional; use `slab`, `infinite_cylinder` or `sphere`",
        )
    if count is None:
        count = 41
    elif count < 5:
        raise InvalidInputError("node_count", "a node count must be a whole number of at least 5")

    j = shape_factor_of(shape)
    dx = dimension / (count - 1)
    alpha = conductivity / heat_capacity
    temperature = [start] * count
    temperature[count - 1] = boundary
    dt_conduction = CONDUCTION_SAFETY * dx * dx / (2.0 * (j + 1.0) * alpha)
    ignition_temperature = boundary + rise

    history = [
        SelfHeatingTimePoint(
            time=from_si(0.0, "s"),
            centre_temperature=from_si(temperature[0], "K"),
            max_temperature=from_si(_max_of(temperature), "K"),
        )
    ]
    history_stride = 1
    steps_since_sample = 0
    time = 0.0
    ignited = False
    steady = False
    induction_time: float | None = None
    next_temperature = [0.0] * count
    step = 0
    while step < MAX_STEPS:
        peak = _max_of(temperature)
        if peak >= ignition_temperature:
            ignited = True
            induction_time = time
            break
        if time >= time_cap:
            break

        peak_source = pre_factor * math.exp(-energy / (R_GAS * peak))
        dt = dt_conduction
        if peak_source > 0.0:
            fk_scale = R_GAS * peak * peak / energy
            dt_reaction = REACTION_STEP_FRACTION * fk_scale * heat_capacity / peak_source
            dt = min(dt, dt_reaction)
        if time + dt > time_cap:
            dt = time_cap - time

        max_rate = 0.0
        for k in range(count - 1):
            if k == 0:
                laplacian = 2.0 * (j + 1.0) * (temperature[1] - temperature[0]) / (dx * dx)
            else:
                second = (temperature[k - 1] - 2.0 * temperature[k] + temperature[k + 1]) / (
                    dx * dx
                )
                first = (temperature[k + 1] - temperature[k - 1]) / (2.0 * dx)
                laplacian = second + j * first / (k * dx)
            source = pre_factor * math.exp(-energy / (R_GAS * temperature[k]))
            rate = (conductivity * laplacian + source) / heat_capacity
            next_temperature[k] = temperature[k] + dt * rate
            max_rate = max(max_rate, abs(rate))
        next_temperature[count - 1] = boundary

        temperature = list(next_temperature)
        time += dt

        steps_since_sample += 1
        if steps_since_sample >= history_stride:
            steps_since_sample = 0
            history.append(
                SelfHeatingTimePoint(
                    time=from_si(time, "s"),
                    centre_temperature=from_si(temperature[0], "K"),
                    max_temperature=from_si(_max_of(temperature), "K"),
                )
            )
            if len(history) >= MAX_HISTORY_POINTS:
                _decimate(history)
                history_stride *= 2

        if max_rate * (time_cap - time) < 0.1 and time > 0.0:
            steady = True
            break
        step += 1

    step_limit_hit = step >= MAX_STEPS
    messages = []
    if step_limit_hit:
        messages.append(
            f"Step limit of {MAX_STEPS} reached before ignition or steady state; "
            "reduce the node count or the maximum simulated time"
        )

    final_peak = _max_of(temperature)
    history.append(
        SelfHeatingTimePoint(
            time=from_si(time, "s"),
            centre_temperature=from_si(temperature[0], "K"),
            max_temperature=from_si(final_peak, "K"),
        )
    )

    if not ignited and steady:
        messages.append(
            "A steady temperature profile was reached, so the body is subcritical at this "
            "boundary temperature"
        )
    if not ignited and not steady and not step_limit_hit:
        messages.append(
            "Maximum simulated time reached without ignition or a clear steady state; "
            "extend the simulated time before concluding the case is safe"
        )

    return SelfHeatingInductionResult(
        geometry=SelfHeatingGeometry(shape),
        characteristic_dimension=from_si(dimension, "m"),
        boundary_temperature=from_si(boundary, "K"),
        initial_temperature=from_si(start, "K"),
        ignition_rise=from_si(rise, "K"),
        ignited=ignited,
        induction_time=None if induction_time is None else from_si(induction_time, "s"),
        peak_temperature=from_si(final_peak, "K"),
        peak_temperature_rise=from_si(final_peak - boundary, "K"),
        steady_state_reached=steady,
        simulated_time=from_si(time, "s"),
        history=tuple(history),
        messages=tuple(messages),
        warnings=tuple(warnings),
    )
