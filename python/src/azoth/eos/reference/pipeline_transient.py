"""``eos.pipeline_transient`` - a segmented, compressible HEM pipeline stage.

Spec: ``specs/models/eos/pipeline_transient.toml``. The conserved state per cell is the
component densities ``c_i``, the momentum ``rho u`` and the total energy ``rho E``. Each
cell's ``(v, u, z)`` is inverted by :func:`azoth.eos.reference.vu_flash.vu_flash`; the
faces carry a Rusanov flux with the frozen homogeneous wave speed; the march is explicit
Euler with Darcy friction and the caller's wall flux.

This is azoth's own line, not a NeqSim ``TransientPipe`` port: NeqSim's class carries one
composition for the whole line and cannot close per component.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from dataclasses import dataclass

from azoth import _models_gen
from azoth.core.errors import InvalidInputError, OutOfRangeError, SolverNotConvergedError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    Phase,
    PipelineTransientPoint,
    PipelineTransientResult,
)
from azoth.core.units import Q, from_si, input_to_si, quantity as q
from azoth.core.warnings import Warning
from azoth.eos.mixture import Mixture
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import reduced_parameters
from azoth.eos.reference.molar_enthalpy_entropy import IdealGasModel
from azoth.eos.reference.state_at_tp import state_at_tp
from azoth.eos.reference.vu_flash import vu_flash

MODEL_ID = "eos.pipeline_transient"
MOLAR_GAS_CONSTANT = 8.31446261815324
MAX_HALVINGS = 40
MIN_STEP = 1.0e-9
_Flux = tuple[list[float], float, float]


@dataclass(frozen=True, slots=True)
class _Cell:
    """One cell's derived state, and the quantities a face flux reads."""

    c: list[float]
    momentum: float
    energy: float
    pressure: float
    temperature: float
    velocity: float
    density: float
    beta: float
    sound_speed: float
    liquid_fraction: float


def _masses(mixture: Mixture) -> list[float]:
    masses: list[float] = []
    for component in mixture.components:
        if component.molar_mass is None:
            raise InvalidInputError(
                "component", "molar mass", "a card-added component needs its own molar mass"
            )
        masses.append(float(component.molar_mass.to("kg/mol").magnitude))
    return masses


def _derive(
    mixture: Mixture,
    ideal_gas: IdealGasModel,
    masses: list[float],
    c: list[float],
    momentum: float,
    energy: float,
) -> _Cell:
    """Invert one cell's conserved state to pressure, temperature and the split."""
    total = sum(c)
    if not total > 0.0 or not math.isfinite(total):
        raise InvalidInputError("initial_component_density", "a cell holds no component")
    z = [value / total for value in c]
    density = sum(c[i] * masses[i] for i in range(len(c)))
    if not density > 0.0:
        raise InvalidInputError(
            "initial_component_density", "a cell's mass density is not positive"
        )
    velocity = momentum / density
    volume = 1.0 / total
    specific_internal = energy / density - 0.5 * velocity * velocity
    molar_internal = specific_internal * density / total
    if not math.isfinite(molar_internal):
        raise InvalidInputError("initial_total_energy", "a cell's internal energy is not finite")

    flash = vu_flash(
        mixture,
        ideal_gas,
        q(volume, "m**3/mol"),
        q(molar_internal, "J/mol"),
        z,
    )
    pressure = float(flash.P.to("Pa").magnitude)
    temperature = float(flash.T.to("K").magnitude)
    reduced = reduced_parameters(mixture, temperature, pressure)
    kij = mixture.effective_kij(temperature)

    if flash.phase is Phase.TWO_PHASE:
        beta = flash.vapour_fraction
        if beta is None:  # pragma: no cover - a split always reports beta
            raise InvalidInputError("vapour_fraction", "a split reported no vapour fraction")
        liquid = phase_capacity(
            mixture, ideal_gas, reduced, kij, temperature, pressure, list(flash.x), flash.z_liquid
        )
        vapour = phase_capacity(
            mixture, ideal_gas, reduced, kij, temperature, pressure, list(flash.y), flash.z_vapour
        )
        cp = (1.0 - beta) * liquid.cp + beta * vapour.cp
        cv = (1.0 - beta) * liquid.cv + beta * vapour.cv
        liquid_volume = flash.z_liquid * MOLAR_GAS_CONSTANT * temperature / pressure
        vapour_volume = flash.z_vapour * MOLAR_GAS_CONSTANT * temperature / pressure
        mixture_volume = (1.0 - beta) * liquid_volume + beta * vapour_volume
        liquid_fraction = (1.0 - beta) * liquid_volume / mixture_volume
    else:
        root = flash.z_liquid if flash.phase is Phase.ALL_LIQUID else flash.z_vapour
        capacity = phase_capacity(mixture, ideal_gas, reduced, kij, temperature, pressure, z, root)
        cp, cv = capacity.cp, capacity.cv
        beta = 0.0 if flash.phase is Phase.ALL_LIQUID else 1.0
        liquid_fraction = 1.0 - beta

    gamma = cp / cv
    sound_speed = math.sqrt(max(gamma * pressure / density, 1.0e-12))
    return _Cell(
        c=list(c),
        momentum=momentum,
        energy=energy,
        pressure=pressure,
        temperature=temperature,
        velocity=velocity,
        density=density,
        beta=beta,
        sound_speed=sound_speed,
        liquid_fraction=liquid_fraction,
    )


def _ghost(
    mixture: Mixture,
    ideal_gas: IdealGasModel,
    c: list[float],
    cell: _Cell,
    pressure: Q,
    temperature: Q | None,
) -> _Cell:
    """A constant-pressure ghost at the boundary, on the adjacent composition."""
    total = sum(c)
    z = [value / total for value in c]
    t_g = float(temperature.to("K").magnitude) if temperature is not None else cell.temperature
    state = state_at_tp(mixture, ideal_gas, q(t_g, "K"), pressure, z)
    if not state.converged:
        raise SolverNotConvergedError(0, 0.0, 1.0)
    volume = float(state.molar_volume.to("m**3/mol").magnitude)
    if not volume > 0.0:
        raise InvalidInputError("right_boundary_pressure", "the ghost volume is not positive")
    density = float(state.density.to("kg/m**3").magnitude)
    molar_mass = float(state.molar_mass.to("kg/mol").magnitude)
    molar_internal = float(state.internal_energy.to("J/mol").magnitude)
    velocity = cell.velocity
    specific_internal = molar_internal / molar_mass
    cp = float(state.cp.to("J/(mol*K)").magnitude)
    cv = float(state.cv.to("J/(mol*K)").magnitude)
    p = float(pressure.to("Pa").magnitude)
    gamma = cp / cv
    if state.phase is Phase.ALL_LIQUID:
        beta = 0.0
    elif state.phase is Phase.ALL_VAPOUR:
        beta = 1.0
    else:
        beta = float(state.vapour_fraction or 0.0)
    return _Cell(
        c=[value / volume for value in z],
        momentum=density * velocity,
        energy=density * (specific_internal + 0.5 * velocity * velocity),
        pressure=p,
        temperature=t_g,
        velocity=velocity,
        density=density,
        beta=beta,
        sound_speed=math.sqrt(max(gamma * p / density, 1.0e-12)),
        liquid_fraction=1.0 - beta,
    )


def _face(left: _Cell, right: _Cell) -> tuple[list[float], float, float]:
    """The Rusanov flux in the +x direction at a face."""
    wave = max(abs(left.velocity) + left.sound_speed, abs(right.velocity) + right.sound_speed)
    c = [
        0.5 * (left.c[j] * left.velocity + right.c[j] * right.velocity)
        - 0.5 * wave * (right.c[j] - left.c[j])
        for j in range(len(left.c))
    ]
    momentum = 0.5 * (
        (left.momentum * left.velocity + left.pressure)
        + (right.momentum * right.velocity + right.pressure)
    ) - 0.5 * wave * (right.momentum - left.momentum)
    energy = 0.5 * (
        ((left.energy + left.pressure) * left.velocity)
        + ((right.energy + right.pressure) * right.velocity)
    ) - 0.5 * wave * (right.energy - left.energy)
    return c, momentum, energy


def pipeline_transient(
    mixture: Mixture,
    ideal_gas: IdealGasModel,
    length: Q,
    diameter: Q,
    cells: float,
    friction_factor: float,
    wall_heat_flux: Q | None,
    left_boundary: str,
    right_boundary: str,
    left_boundary_pressure: Q | None,
    right_boundary_pressure: Q | None,
    left_boundary_temperature: Q | None,
    right_boundary_temperature: Q | None,
    initial_component_density: Sequence[Sequence[Q]],
    initial_momentum: Sequence[Q],
    initial_total_energy: Sequence[Q],
    time_step: Q,
    duration: Q,
    cfl_number: float | None = None,
) -> PipelineTransientResult:
    """Advance one stage of a segmented compressible line.

    The state is conserved; a stage is one call, and the caller chains blowdown, hold
    and restart stages. See the spec for the flux, the boundaries and the failure
    policy.
    """
    spec = _models_gen.model(MODEL_ID)
    checks = checks_for(spec)
    warnings: list[Warning] = []
    length_si = input_to_si(spec, "length", length)
    diameter_si = input_to_si(spec, "diameter", diameter)
    step_si = input_to_si(spec, "time_step", time_step)
    duration_si = input_to_si(spec, "duration", duration)
    apply_checks(
        checks.on_input,
        {
            "length": length_si,
            "diameter": diameter_si,
            "time_step": step_si,
            "duration": duration_si,
        }.get,
        warnings,
    )

    requested = float(cells)
    count = round(requested)
    if abs(requested - count) > 1.0e-09:
        raise InvalidInputError("cells", f"`cells` must be a whole number, got {requested}")
    if count < 2:
        raise OutOfRangeError(
            "cells", float(count), "a finite-volume line needs at least two cells"
        )
    if left_boundary not in ("closed", "constant_pressure"):
        raise InvalidInputError("left_boundary", f"unknown boundary `{left_boundary}`")
    if right_boundary not in ("closed", "constant_pressure"):
        raise InvalidInputError("right_boundary", f"unknown boundary `{right_boundary}`")
    if left_boundary == "constant_pressure" and left_boundary_pressure is None:
        raise InvalidInputError(
            "left_boundary_pressure", "a constant-pressure inlet needs a pressure"
        )
    if right_boundary == "constant_pressure" and right_boundary_pressure is None:
        raise InvalidInputError(
            "right_boundary_pressure", "a constant-pressure outlet needs a pressure"
        )
    if len(initial_component_density) != count:
        raise InvalidInputError(
            "initial_component_density",
            f"{count} cells need {count} rows, got {len(initial_component_density)}",
        )
    n = len(mixture)
    for row in initial_component_density:
        if len(row) != n:
            raise InvalidInputError(
                "initial_component_density",
                f"a row needs one entry per component ({n}), got {len(row)}",
            )
    masses = _masses(mixture)
    c = [
        [float(value.to("mol/m**3").magnitude) for value in row]
        for row in initial_component_density
    ]
    momentum = [float(value.to("kg/(m**2*s)").magnitude) for value in initial_momentum]
    energy = [float(value.to("Pa").magnitude) for value in initial_total_energy]
    if len(momentum) != count or len(energy) != count:
        raise InvalidInputError("initial_momentum", f"{count} cells need {count} values")

    cfl = 0.5 if cfl_number is None else float(cfl_number)
    if not 0.0 < cfl <= 1.0:
        raise OutOfRangeError("cfl_number", cfl, "the CFL fraction is in (0, 1]")
    friction = float(friction_factor)
    heat_flux = 0.0 if wall_heat_flux is None else float(wall_heat_flux.to("W/m**2").magnitude)
    dx = length_si / count
    area = math.pi * diameter_si * diameter_si / 4.0

    cells_state = [
        _derive(mixture, ideal_gas, masses, c[i], momentum[i], energy[i]) for i in range(count)
    ]
    fastest = max(abs(cell.velocity) + cell.sound_speed for cell in cells_state)
    if step_si > cfl * dx / fastest:
        raise OutOfRangeError(
            "time_step",
            step_si,
            f"exceeds the CFL limit {cfl * dx / fastest:.6g} s for the initial state",
        )

    def holdup() -> tuple[list[float], float]:
        component = [sum(c[i][j] for i in range(count)) * area * dx for j in range(n)]
        return component, sum(component[j] * masses[j] for j in range(n))

    initial_component, initial_mass = holdup()
    boundary_component = [0.0] * n
    boundary_mass = 0.0
    history = [
        _point(
            0.0,
            initial_mass,
            cells_state[0],
            cells_state[-1],
            cells_state,
            area,
            dx,
            left_boundary,
            right_boundary,
            left_boundary_pressure,
            right_boundary_pressure,
        )
    ]
    elapsed = 0.0
    steps = 0
    rejections = 0
    smallest = step_si
    while elapsed < duration_si - 1.0e-12:
        dt = min(step_si, duration_si - elapsed)
        accepted = False
        for _attempt in range(MAX_HALVINGS + 1):
            try:
                new_c, new_momentum, new_energy, left_flux, right_flux = _step(
                    mixture,
                    ideal_gas,
                    cells_state,
                    c,
                    momentum,
                    energy,
                    dx,
                    diameter_si,
                    friction,
                    heat_flux,
                    left_boundary,
                    right_boundary,
                    left_boundary_pressure,
                    right_boundary_pressure,
                    left_boundary_temperature,
                    right_boundary_temperature,
                    dt,
                )
                new_states = [
                    _derive(mixture, ideal_gas, masses, new_c[i], new_momentum[i], new_energy[i])
                    for i in range(count)
                ]
                accepted = True
                break
            except (
                InvalidInputError,
                OutOfRangeError,
                SolverNotConvergedError,
                ValueError,
                ZeroDivisionError,
                OverflowError,
            ):
                dt *= 0.5
                rejections += 1
                smallest = min(smallest, dt)
                if dt < MIN_STEP:
                    raise SolverNotConvergedError(steps, dt, MIN_STEP) from None

        if not accepted:  # pragma: no cover - the halving loop either accepts or raises
            raise SolverNotConvergedError(steps, dt, MIN_STEP)

        elapsed += dt
        smallest = min(smallest, dt)
        c, momentum, energy = new_c, new_momentum, new_energy
        cells_state = new_states
        steps += 1
        for j in range(n):
            boundary_component[j] += (right_flux[0][j] - left_flux[0][j]) * area * dt
        right_mass_flux = sum(right_flux[0][j] * masses[j] for j in range(n))
        left_mass_flux = sum(left_flux[0][j] * masses[j] for j in range(n))
        boundary_mass += (right_mass_flux - left_mass_flux) * area * dt
        _, current_mass = holdup()
        history.append(
            _point(
                elapsed,
                current_mass,
                cells_state[0],
                cells_state[-1],
                cells_state,
                area,
                dx,
                left_boundary,
                right_boundary,
                left_boundary_pressure,
                right_boundary_pressure,
            )
        )

    final_component, final_mass = holdup()
    final_states = cells_state
    return PipelineTransientResult(
        final_component_density=tuple(
            tuple(from_si(value, "mol/m**3") for value in row) for row in c
        ),
        final_velocity=tuple(from_si(cell.velocity, "m/s") for cell in final_states),
        final_pressure=tuple(from_si(cell.pressure, "Pa") for cell in final_states),
        final_temperature=tuple(from_si(cell.temperature, "K") for cell in final_states),
        final_vapour_fraction=tuple(cell.beta for cell in final_states),
        final_density=tuple(from_si(cell.density, "kg/m**3") for cell in final_states),
        final_liquid_volume_fraction=tuple(cell.liquid_fraction for cell in final_states),
        boundary_component_outflow=tuple(from_si(value, "mol") for value in boundary_component),
        initial_component_holdup=tuple(from_si(value, "mol") for value in initial_component),
        final_component_holdup=tuple(from_si(value, "mol") for value in final_component),
        boundary_mass_outflow=from_si(boundary_mass, "kg"),
        initial_mass=from_si(initial_mass, "kg"),
        final_mass=from_si(final_mass, "kg"),
        steps=steps,
        rejections=rejections,
        min_time_step=from_si(smallest, "s"),
        history=tuple(history),
        warnings=tuple(warnings),
    )


def _point(
    time: float,
    mass: float,
    first: _Cell,
    last: _Cell,
    states: list[_Cell],
    area: float,
    dx: float,
    left_boundary: str,
    right_boundary: str,
    left_pressure: Q | None,
    right_pressure: Q | None,
) -> PipelineTransientPoint:
    p_in = (
        float(left_pressure.to("Pa").magnitude)
        if left_boundary == "constant_pressure" and left_pressure is not None
        else first.pressure
    )
    p_out = (
        float(right_pressure.to("Pa").magnitude)
        if right_boundary == "constant_pressure" and right_pressure is not None
        else last.pressure
    )
    liquid = sum(cell.liquid_fraction for cell in states) * area * dx
    return PipelineTransientPoint(
        time=from_si(time, "s"),
        mass=from_si(mass, "kg"),
        p_in=from_si(p_in, "Pa"),
        p_out=from_si(p_out, "Pa"),
        t_out=from_si(last.temperature, "K"),
        beta_out=last.beta,
        liquid_inventory=from_si(liquid, "m**3"),
    )


def _step(
    mixture: Mixture,
    ideal_gas: IdealGasModel,
    states: list[_Cell],
    c: list[list[float]],
    momentum: list[float],
    energy: list[float],
    dx: float,
    diameter: float,
    friction: float,
    heat_flux: float,
    left_boundary: str,
    right_boundary: str,
    left_pressure: Q | None,
    right_pressure: Q | None,
    left_temperature: Q | None,
    right_temperature: Q | None,
    dt: float,
) -> tuple[list[list[float]], list[float], list[float], _Flux, _Flux]:
    """One explicit Euler step; returns the new state and the two boundary fluxes."""
    count = len(states)
    n = len(states[0].c)
    if left_boundary == "closed":
        left_flux: _Flux = (
            [0.0] * n,
            states[0].pressure,
            0.0,
        )
    else:
        assert left_pressure is not None
        ghost = _ghost(mixture, ideal_gas, c[0], states[0], left_pressure, left_temperature)
        left_flux = _face(ghost, states[0])
    if right_boundary == "closed":
        right_flux: _Flux = (
            [0.0] * n,
            states[-1].pressure,
            0.0,
        )
    else:
        assert right_pressure is not None
        ghost = _ghost(mixture, ideal_gas, c[-1], states[-1], right_pressure, right_temperature)
        right_flux = _face(states[-1], ghost)

    faces: list[_Flux] = [left_flux]
    for i in range(1, count):
        faces.append(_face(states[i - 1], states[i]))
    faces.append(right_flux)

    new_c: list[list[float]] = []
    new_momentum: list[float] = []
    new_energy: list[float] = []
    for i in range(count):
        left = faces[i]
        right = faces[i + 1]
        row = [c[i][j] - dt / dx * (right[0][j] - left[0][j]) for j in range(n)]
        if any(value < -1.0e-12 or not math.isfinite(value) for value in row):
            raise ValueError("a component density went negative")
        new_c.append([max(value, 0.0) for value in row])
        cell = states[i]
        friction_source = (
            -friction * cell.density * abs(cell.velocity) * cell.velocity / (2.0 * diameter)
        )
        new_momentum.append(momentum[i] - dt / dx * (right[1] - left[1]) + dt * friction_source)
        heat_source = 4.0 * heat_flux / diameter
        new_energy.append(energy[i] - dt / dx * (right[2] - left[2]) + dt * heat_source)
        if not math.isfinite(new_energy[-1]):
            raise ValueError("the total energy is not finite")
    return new_c, new_momentum, new_energy, left_flux, right_flux
