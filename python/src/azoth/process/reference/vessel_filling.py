"""``process.vessel_filling`` - the gas-charging transient.

As NeqSim's ``VesselFillingSimulator.run`` computes it. Spec:
``specs/models/process/vessel_filling.toml``

The Python twin of ``crates/azoth-process/src/models/vessel_filling.rs``: the inlet
stagnation enthalpy, the mole alignment, the VU-flash march, the lumped wall, the Woodfield
film coefficient and the liner temperature limits.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import VesselFillingPoint, VesselFillingResult
from azoth.core.units import Q, from_si, input_to_si, quantity
from azoth.core.warnings import Warning
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import Evaluations, flashed_state, molar_mass_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import reduced_parameters
from azoth.eos.reference.pt_flash import check_composition, pt_flash
from azoth.eos.reference.thermal_conductivity import thermal_conductivity
from azoth.eos.reference.viscosity import viscosity
from azoth.eos.reference.vu_flash import vu_flash
from azoth.eos.reference.vu_flash_single_comp import vu_flash_single_comp

MODEL_ID = "process.vessel_filling"

#: The class's gas constant, J/(mol K).
GAS_CONSTANT = 8.314
#: Standard gravity for the Woodfield Rayleigh number, m/s2.
GRAVITY = 9.80665


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    points: list[tuple[float, float, float, float, float]]
    inlet_temperature: float
    inlet_mass_flow: float
    vu_flash_fallback_count: float
    max_fluid_temperature: float
    min_fluid_temperature: float
    max_pressure: float
    final_pressure: float
    final_mass: float
    liner_over_temperature: bool
    liner_under_temperature: bool
    liner_limits_met: bool
    recorded_points: float


def _vu_at(
    mixture: Any, ideal_gas: Any, previous_pressure: float, v: float, u: float, z: list[float]
) -> tuple[float, float]:
    """The class's VU dispatch: the pure single-component route, then the mixture route."""
    if len(mixture.components) == 1:
        try:
            single = vu_flash_single_comp(
                mixture,
                ideal_gas,
                quantity(previous_pressure, "Pa"),
                quantity(v, "m**3/mol"),
                quantity(u, "J/mol"),
            )
            return previous_pressure, single.T.to("K").magnitude
        except Exception:  # the class falls through on any refusal
            pass
    flash = vu_flash(
        mixture,
        ideal_gas,
        quantity(v, "m**3/mol"),
        quantity(u, "J/mol"),
        z,
    )
    return flash.P.to("Pa").magnitude, flash.T.to("K").magnitude


def _bulk_cp(mixture: Any, ideal_gas: Any, t: float, p: float, z: list[float]) -> float:
    reduced = reduced_parameters(mixture, t, p)
    kij = mixture.effective_kij(reduced.t_kelvin)
    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), z)
    if flash.phase == "two_phase":
        beta = flash.vapour_fraction
        vapour = phase_capacity(
            mixture, ideal_gas, reduced, kij, t, p, list(flash.y), flash.z_vapour
        )
        liquid = phase_capacity(
            mixture, ideal_gas, reduced, kij, t, p, list(flash.x), flash.z_liquid
        )
        if beta is None:
            raise InvalidInputError("initial_p", "the flash reported no vapour fraction")
        return float(beta * vapour.cp + (1.0 - beta) * liquid.cp)
    from azoth.eos.reference._mixture_state import phase_state

    root = phase_state(reduced, kij, z, liquid=flash.phase != "all_vapour").z
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, z, root)
    return float(capacity.cp)


def _woodfield(
    mixture: Any,
    ideal_gas: Any,
    z: list[float],
    molar_mass: float,
    temperature: float,
    pressure: float,
    density: float,
    inlet_mass_flow: float,
    inlet_temperature: float,
    nozzle: float,
    height: float,
) -> float | None:
    try:
        mu = float(
            viscosity(mixture, quantity(temperature, "K"), quantity(pressure, "Pa"), z)
            .mu.to("Pa*s")
            .magnitude
        )
        k = float(
            thermal_conductivity(
                mixture, ideal_gas, quantity(temperature, "K"), quantity(pressure, "Pa"), z
            )
            .k.to("W/(m*K)")
            .magnitude
        )
        cp_molar = _bulk_cp(mixture, ideal_gas, temperature, pressure, z)
    except Exception:  # the class falls back to the fixed coefficient
        return None
    if mu <= 0.0 or k <= 0.0 or cp_molar <= 0.0 or molar_mass <= 0.0 or density <= 0.0:
        return None
    cp_mass = cp_molar / molar_mass
    alpha = k / (density * cp_mass)
    nu = mu / density
    beta = 1.0 / temperature
    nozzle_area = math.pi * 0.25 * nozzle * nozzle
    inlet_velocity = inlet_mass_flow / (density * nozzle_area)
    reynolds = density * abs(inlet_velocity) * nozzle / mu
    rayleigh = GRAVITY * beta * abs(inlet_temperature - temperature) * height**3 / (nu * alpha)
    nusselt = 0.56 * reynolds**0.67 + 0.104 * rayleigh**0.352
    coefficient = nusselt * k / height
    return coefficient if coefficient > 0.0 and math.isfinite(coefficient) else None


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
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
    target_pressure: Q | None = None,
    wall_mass: Q | None = None,
    wall_area: Q | None = None,
    wall_specific_heat: Q | None = None,
    wall_heat_transfer_coefficient: Q | None = None,
    woodfield_nozzle_diameter: Q | None = None,
    vessel_height: Q | None = None,
    liner_min_temperature: Q | None = None,
    liner_max_temperature: Q | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []
    initial_t_si = input_to_si(spec, "initial_t", initial_t)
    initial_p_si = input_to_si(spec, "initial_p", initial_p)
    volume = input_to_si(spec, "vessel_volume", vessel_volume)
    inlet_t_si = input_to_si(spec, "inlet_temperature", inlet_temperature)
    supply = input_to_si(spec, "supply_pressure", supply_pressure)
    flow = input_to_si(spec, "inlet_mass_flow", inlet_mass_flow)
    target = (
        math.inf
        if target_pressure is None
        else input_to_si(spec, "target_pressure", target_pressure)
    )
    wall_metal = 0.0 if wall_mass is None else input_to_si(spec, "wall_mass", wall_mass)
    wall_face = 0.0 if wall_area is None else input_to_si(spec, "wall_area", wall_area)
    wall_capacity = (
        470.0
        if wall_specific_heat is None
        else input_to_si(spec, "wall_specific_heat", wall_specific_heat)
    )
    wall_film = (
        50.0
        if wall_heat_transfer_coefficient is None
        else input_to_si(spec, "wall_heat_transfer_coefficient", wall_heat_transfer_coefficient)
    )
    nozzle = (
        None
        if woodfield_nozzle_diameter is None
        else input_to_si(spec, "woodfield_nozzle_diameter", woodfield_nozzle_diameter)
    )
    height = None if vessel_height is None else input_to_si(spec, "vessel_height", vessel_height)
    liner_min = (
        -math.inf
        if liner_min_temperature is None
        else input_to_si(spec, "liner_min_temperature", liner_min_temperature)
    )
    liner_max = (
        math.inf
        if liner_max_temperature is None
        else input_to_si(spec, "liner_max_temperature", liner_max_temperature)
    )
    step_seconds = 1.0 if time_step is None else input_to_si(spec, "time_step", time_step)
    span = 600.0 if max_time is None else input_to_si(spec, "max_time", max_time)

    apply_checks(
        checks_for(spec).on_input,
        {
            "initial_t": initial_t_si,
            "initial_p": initial_p_si,
            "vessel_volume": volume,
            "inlet_temperature": inlet_t_si,
            "supply_pressure": supply,
            "inlet_mass_flow": flow,
            "target_pressure": 1.0 if target_pressure is None else target,
            "woodfield_nozzle_diameter": 1.0 if nozzle is None else nozzle,
            "vessel_height": 1.0 if height is None else height,
            "time_step": step_seconds,
            "max_time": span,
        }.get,
        warnings,
    )
    for name, value in (
        ("initial_t", initial_t_si),
        ("initial_p", initial_p_si),
        ("vessel_volume", volume),
        ("inlet_temperature", inlet_t_si),
        ("supply_pressure", supply),
        ("inlet_mass_flow", flow),
        ("wall_mass", wall_metal),
        ("wall_area", wall_face),
        ("wall_specific_heat", wall_capacity),
        ("wall_heat_transfer_coefficient", wall_film),
        ("woodfield_nozzle_diameter", 1.0 if nozzle is None else nozzle),
        ("vessel_height", 1.0 if height is None else height),
        ("time_step", step_seconds),
        ("max_time", span),
    ):
        if not math.isfinite(value):
            raise InvalidInputError(
                name, "a non-finite value is not a number this calculation publishes"
            )
    if volume <= 0.0 or inlet_t_si <= 0.0 or supply <= 0.0 or flow <= 0.0:
        raise InvalidInputError(
            "vessel_volume",
            "the class requires the volume and inlet conditions to be positive",
        )
    if target_pressure is not None and (target <= 0.0 or not math.isfinite(target)):
        raise InvalidInputError(
            "target_pressure", "the class refuses a non-positive target pressure"
        )
    if (nozzle is None) != (height is None):
        raise InvalidInputError(
            "woodfield_nozzle_diameter",
            "the Woodfield correlation needs both the inlet nozzle diameter and the vessel height",
        )
    if (nozzle is not None and nozzle <= 0.0) or (height is not None and height <= 0.0):
        raise InvalidInputError(
            "woodfield_nozzle_diameter",
            "the class refuses a non-positive nozzle diameter or vessel height",
        )
    if liner_max_temperature is not None and liner_max <= liner_min:
        raise InvalidInputError(
            "liner_max_temperature",
            "the class requires the upper liner limit to exceed the lower one",
        )
    if step_seconds <= 0.0 or span <= 0.0:
        raise InvalidInputError("time_step", "the class refuses a non-positive step or horizon")
    if initial_t_si <= 0.0 or initial_p_si <= 0.0:
        raise InvalidInputError(
            "initial_t", "the initial temperature and pressure must be positive"
        )

    mixture, ideal_gas = _components.mixture_of([str(name) for name in components], eos="pr")
    check_composition([float(value) for value in initial_z], len(mixture.components), "initial_z")
    z = [float(value) for value in initial_z]
    molar_mass = molar_mass_of(mixture, z)
    evaluations = Evaluations()
    initial = flashed_state(
        mixture, ideal_gas, z, molar_mass, initial_p_si, initial_t_si, evaluations
    )
    inlet = flashed_state(mixture, ideal_gas, z, molar_mass, supply, inlet_t_si, evaluations)
    inlet_enthalpy = inlet.molar_enthalpy / molar_mass

    pressure = initial_p_si
    temperature = initial_t_si
    mass = initial.density * volume
    moles = mass / molar_mass
    energy = initial.molar_enthalpy * moles - pressure * volume
    wall_temperature = temperature
    fallbacks = 0.0
    time = 0.0
    points: list[tuple[float, float, float, float, float]] = [
        (time, pressure, temperature, mass, wall_temperature)
    ]
    max_fluid_temperature = temperature
    min_fluid_temperature = temperature
    max_pressure = pressure

    while time < span and pressure < target and mass > 0.0:
        dm = flow * step_seconds
        new_mass = mass + dm
        density = mass / volume
        coefficient = wall_film
        if nozzle is not None and height is not None:
            woodfield = _woodfield(
                mixture,
                ideal_gas,
                z,
                molar_mass,
                temperature,
                pressure,
                density,
                flow,
                inlet_t_si,
                nozzle,
                height,
            )
            if woodfield is not None:
                coefficient = woodfield
        q_wall = (
            coefficient * wall_face * (wall_temperature - temperature) if wall_metal > 0.0 else 0.0
        )
        d_u = inlet_enthalpy * dm + q_wall * step_seconds
        if wall_metal > 0.0:
            wall_temperature += (-q_wall) * step_seconds / (wall_metal * wall_capacity)
        new_energy = energy + d_u
        if mass > 0.0:
            moles *= new_mass / mass
        new_moles = new_mass / molar_mass
        molar_volume = volume / new_moles
        molar_energy = new_energy / new_moles
        try:
            pressure, temperature = _vu_at(
                mixture, ideal_gas, pressure, molar_volume, molar_energy, z
            )
        except Exception:  # the class's ideal-gas fallback
            fallbacks += 1.0
            pressure = new_mass * GAS_CONSTANT * temperature / (molar_mass * volume)
        mass = new_mass
        energy = new_energy
        time += step_seconds
        max_fluid_temperature = max(max_fluid_temperature, temperature)
        min_fluid_temperature = min(min_fluid_temperature, temperature)
        max_pressure = max(max_pressure, pressure)
        points.append((time, pressure, temperature, mass, wall_temperature))

    liner_over = any(point[2] > liner_max for point in points)
    liner_under = any(point[2] < liner_min for point in points)
    return (
        _Solution(
            points=points,
            inlet_temperature=inlet_t_si,
            inlet_mass_flow=flow,
            vu_flash_fallback_count=fallbacks,
            max_fluid_temperature=max_fluid_temperature,
            min_fluid_temperature=min_fluid_temperature,
            max_pressure=max_pressure,
            final_pressure=points[-1][1],
            final_mass=points[-1][3],
            liner_over_temperature=liner_over,
            liner_under_temperature=liner_under,
            liner_limits_met=not liner_over and not liner_under,
            recorded_points=float(len(points)),
        ),
        warnings,
    )


def vessel_filling(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    inlet_temperature: Q,
    supply_pressure: Q,
    inlet_mass_flow: Q,
    target_pressure: Q | None = None,
    wall_mass: Q | None = None,
    wall_area: Q | None = None,
    wall_specific_heat: Q | None = None,
    wall_heat_transfer_coefficient: Q | None = None,
    woodfield_nozzle_diameter: Q | None = None,
    vessel_height: Q | None = None,
    liner_min_temperature: Q | None = None,
    liner_max_temperature: Q | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
) -> VesselFillingResult:
    """The gas-charging transient of a vessel.

    The inlet specific enthalpy is flashed once at the supply state; the vessel's moles are
    aligned to ``density x volume``; each step adds ``mdot dt``, exchanges heat with the
    lumped wall, scales the moles and VU-flashes at the vessel volume and the new internal
    energy. The Woodfield film coefficient is used when both the nozzle diameter and the
    vessel height are given.

    Args:
        components: the residual gas's substances.
        initial_z: the residual gas's composition.
        initial_t: the residual gas temperature.
        initial_p: the residual gas pressure.
        vessel_volume: the vessel's internal volume.
        inlet_temperature: the supply stagnation temperature.
        supply_pressure: the supply pressure for the inlet enthalpy.
        inlet_mass_flow: the charging mass flow.
        target_pressure: the stop pressure; absent charges for the whole horizon.
        wall_mass: the lumped wall mass; absent is 0, which disables the wall.
        wall_area: the wall's internal area; absent is 0.
        wall_specific_heat: the wall's specific heat; absent is 470.
        wall_heat_transfer_coefficient: the fixed film coefficient; absent is 50.
        woodfield_nozzle_diameter: the inlet nozzle diameter; absent uses the fixed value.
        vessel_height: the vessel height; absent uses the fixed value.
        liner_min_temperature: the lower liner limit; absent is no limit.
        liner_max_temperature: the upper liner limit; absent is no limit.
        time_step: the explicit step; absent is 1 s.
        max_time: the horizon; absent is 600 s.

    Returns:
        The recorded history and its extremes.

    Raises:
        InvalidInputError: for a non-positive dimension, a liner limit order, one Woodfield
            dimension without the other, or a feed state the flash cannot resolve.
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
        target_pressure,
        wall_mass,
        wall_area,
        wall_specific_heat,
        wall_heat_transfer_coefficient,
        woodfield_nozzle_diameter,
        vessel_height,
        liner_min_temperature,
        liner_max_temperature,
        time_step,
        max_time,
    )
    return VesselFillingResult(
        points=tuple(
            VesselFillingPoint(
                time=from_si(point[0], "s"),
                pressure=from_si(point[1], "Pa"),
                temperature=from_si(point[2], "K"),
                mass=from_si(point[3], "kg"),
                wall_temperature=from_si(point[4], "K"),
            )
            for point in solution.points
        ),
        inlet_temperature=from_si(solution.inlet_temperature, "K"),
        inlet_mass_flow=from_si(solution.inlet_mass_flow, "kg/s"),
        vu_flash_fallback_count=solution.vu_flash_fallback_count,
        max_fluid_temperature=from_si(solution.max_fluid_temperature, "K"),
        min_fluid_temperature=from_si(solution.min_fluid_temperature, "K"),
        max_pressure=from_si(solution.max_pressure, "Pa"),
        final_pressure=from_si(solution.final_pressure, "Pa"),
        final_mass=from_si(solution.final_mass, "kg"),
        liner_over_temperature=solution.liner_over_temperature,
        liner_under_temperature=solution.liner_under_temperature,
        liner_limits_met=solution.liner_limits_met,
        recorded_points=solution.recorded_points,
        warnings=tuple(warnings),
    )
