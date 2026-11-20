"""``process.trapped_liquid_fire_rupture`` - the trapped-liquid fire rupture study.

The Python twin of ``crates/azoth-process/src/models/trapped_liquid_fire_rupture.rs``.
The port source is ``TrappedLiquidFireRuptureStudy.run``, with the wall, fire and
material helper classes.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import RupturePoint, TrappedLiquidFireRuptureResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning, WarningCode
from azoth.eos import components as _components
from azoth.process.reference import (
    _fire_exposure,
    _fluid_state,
    _material_strength,
    _transient_wall,
)

MODEL_ID = "process.trapped_liquid_fire_rupture"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _scalar(spec: dict[str, Any], field: str, value: Any) -> float:
    """A scalar the caller may state as a number (already SI) or a quantity."""
    if hasattr(value, "to") or hasattr(value, "magnitude"):
        return input_to_si(spec, field, value)
    return float(value)


def _optional(spec: dict[str, Any], field: str, value: Any) -> float | None:
    return None if value is None else _scalar(spec, field, value)


def trapped_liquid_fire_rupture(
    components: list[str],
    z: list[float],
    inventory_pressure: Q,
    inventory_temperature: Q,
    inventory_liquid_mass: Q,
    inventory_total_mass: Q,
    inventory_total_volume: Q,
    pipe_internal_diameter: Q,
    wall_thickness: Q,
    exposed_length: Q,
    eos: str | None = None,
    multiphase_check: bool | None = None,
    segment_id: str | None = None,
    fire_type: str | None = None,
    environmental_factor: float | None = None,
    fire_heat_flux: float | None = None,
    fire_emissivity: float | None = None,
    fire_view_factor: float | None = None,
    flame_temperature: Q | None = None,
    ambient_temperature: Q | None = None,
    passive_protection_factor: float | None = None,
    fire_exposed_area: float | None = None,
    api5l_grade: str | None = None,
    ambient_tensile_strength: Q | None = None,
    flange_class: str | None = None,
    relief_set_pressure: Q | None = None,
    relief_limits_pressure: bool | None = None,
    max_time: Q | None = None,
    time_step: Q | None = None,
    wall_density: Q | None = None,
    wall_heat_capacity: float | None = None,
    wall_conductivity: float | None = None,
    liquid_heat_capacity: float | None = None,
    liquid_expansion: float | None = None,
    liquid_bulk_modulus: Q | None = None,
    inner_htc: float | None = None,
    outer_htc: float | None = None,
    tensile_strength_factor: float | None = None,
    wall_nodes: int | None = None,
    vapor_pocket_detection: bool | None = None,
) -> TrappedLiquidFireRuptureResult:
    """The trapped-liquid fire rupture study.

    Raises:
        InvalidInputError: for a missing fire parameter, a non-positive geometry, time,
            wall or liquid property, fewer than three nodes, an unknown grade or fire
            type, or a factor outside its range.
    """
    spec = _spec()
    warnings: list[Warning] = []
    pressure0 = input_to_si(spec, "inventory_pressure", inventory_pressure)
    temperature0 = input_to_si(spec, "inventory_temperature", inventory_temperature)
    liquid_mass_input = input_to_si(spec, "inventory_liquid_mass", inventory_liquid_mass)
    total_mass = input_to_si(spec, "inventory_total_mass", inventory_total_mass)
    total_volume = input_to_si(spec, "inventory_total_volume", inventory_total_volume)
    diameter = input_to_si(spec, "pipe_internal_diameter", pipe_internal_diameter)
    wall = input_to_si(spec, "wall_thickness", wall_thickness)
    length = input_to_si(spec, "exposed_length", exposed_length)
    max_time_value = _optional(spec, "max_time", max_time)
    time_step_value = _optional(spec, "time_step", time_step)
    factor = _optional(spec, "tensile_strength_factor", tensile_strength_factor)
    apply_checks(
        checks_for(spec).on_input,
        {
            "inventory_pressure": pressure0,
            "inventory_temperature": temperature0,
            "inventory_total_mass": total_mass,
            "inventory_total_volume": total_volume,
            "pipe_internal_diameter": diameter,
            "wall_thickness": wall,
            "exposed_length": length,
            "max_time": max_time_value,
            "time_step": time_step_value,
            "tensile_strength_factor": factor,
            "passive_protection_factor": passive_protection_factor,
        }.get,
        warnings,
    )

    outer_diameter = diameter + 2.0 * wall
    area = math.pi * outer_diameter * length
    exposed_area = area if fire_exposed_area is None else fire_exposed_area
    ambient = (
        298.15
        if ambient_temperature is None
        else input_to_si(spec, "ambient_temperature", ambient_temperature)
    )
    kind = "api_521_pool_fire" if fire_type is None else fire_type
    environmental = (
        1.0
        if environmental_factor is None
        else _scalar(spec, "environmental_factor", environmental_factor)
    )
    if kind == "api_521_pool_fire":
        fire = _fire_exposure.api521_pool_fire(exposed_area, environmental)
    elif kind == "fixed_heat_flux":
        if fire_heat_flux is None:
            raise InvalidInputError("fire_heat_flux", "a fixed flux is required")
        flux = _scalar(spec, "fire_heat_flux", fire_heat_flux)
        fire = _fire_exposure.fixed_heat_flux(exposed_area, flux)
    elif kind == "radiative_fire":
        if fire_emissivity is None or fire_view_factor is None:
            raise InvalidInputError(
                "fire_emissivity", "a radiative fire states its emissivity and view factor"
            )
        flame = (
            1200.0
            if flame_temperature is None
            else input_to_si(spec, "flame_temperature", flame_temperature)
        )
        fire = _fire_exposure.radiative_fire(
            exposed_area,
            _scalar(spec, "fire_emissivity", fire_emissivity),
            _scalar(spec, "fire_view_factor", fire_view_factor),
            flame,
            ambient,
        )
    else:
        raise InvalidInputError(
            "fire_type",
            f"`{kind}` is not `api_521_pool_fire`, `fixed_heat_flux` or `radiative_fire`",
        )
    if passive_protection_factor is not None:
        fire = _fire_exposure.with_passive_protection(
            fire, _scalar(spec, "passive_protection_factor", passive_protection_factor)
        )

    if ambient_tensile_strength is not None:
        tensile = input_to_si(spec, "ambient_tensile_strength", ambient_tensile_strength)
    else:
        grade = _material_strength.normalize_grade("B" if api5l_grade is None else api5l_grade)
        tensile = _material_strength.api5l_strengths(grade)[1]
    factor_value = 0.75 if factor is None else factor
    if not factor_value > 0.0 or not math.isfinite(factor_value):
        raise InvalidInputError("tensile_strength_factor", "the class requires a positive factor")
    flange = 0 if flange_class is None else int(flange_class)
    flange_ambient = _material_strength.flange_class_rating_pa(flange)

    liquid_mass = max(liquid_mass_input, total_mass)
    if liquid_mass_input <= 0.0:
        warnings.append(
            Warning(
                WarningCode.FALLBACK_TO_INPUT,
                "No liquid inventory was reported; total mass was used for thermal capacity.",
                "inventory_liquid_mass",
            )
        )
    step = 2.0 if time_step_value is None else time_step_value
    horizon = 1800.0 if max_time_value is None else max_time_value
    if not (step > 0.0 and math.isfinite(step) and horizon > 0.0 and math.isfinite(horizon)):
        raise InvalidInputError("time_step", "the time control is positive and finite")
    nodes = 15 if wall_nodes is None else int(wall_nodes)
    if nodes < 3:
        raise InvalidInputError("wall_nodes", "the class requires at least three nodes")
    wall_rho = 7850.0 if wall_density is None else input_to_si(spec, "wall_density", wall_density)
    wall_cp = (
        500.0
        if wall_heat_capacity is None
        else _scalar(spec, "wall_heat_capacity", wall_heat_capacity)
    )
    wall_k = (
        45.0 if wall_conductivity is None else _scalar(spec, "wall_conductivity", wall_conductivity)
    )
    liquid_cp = (
        2200.0
        if liquid_heat_capacity is None
        else _scalar(spec, "liquid_heat_capacity", liquid_heat_capacity)
    )
    expansion = (
        7.0e-4 if liquid_expansion is None else _scalar(spec, "liquid_expansion", liquid_expansion)
    )
    bulk = (
        1.2e9
        if liquid_bulk_modulus is None
        else input_to_si(spec, "liquid_bulk_modulus", liquid_bulk_modulus)
    )
    inner = 500.0 if inner_htc is None else _scalar(spec, "inner_htc", inner_htc)
    outer = 35.0 if outer_htc is None else _scalar(spec, "outer_htc", outer_htc)
    wall_model = _transient_wall.TransientWall(wall, wall_k, wall_rho, wall_cp, temperature0, nodes)

    eos_name = "pr" if eos is None else eos
    mixture, ideal_gas = _components.mixture_of([str(name) for name in components], eos=eos_name)

    steps = math.ceil(horizon / step)
    trajectory: list[RupturePoint] = []
    pressure = pressure0
    liquid_temperature = temperature0
    time_to_relief = None
    time_to_vapor = None
    time_to_pipe = None
    time_to_flange = None
    vacuum = True if vapor_pocket_detection is None else vapor_pocket_detection
    relief_limit = bool(relief_limits_pressure)
    set_pressure = (
        None
        if relief_set_pressure is None
        else input_to_si(spec, "relief_set_pressure", relief_set_pressure)
    )

    for index in range(steps + 1):
        time = min(index * step, horizon)
        outer_wall = wall_model.outer_wall_temperature()
        inner_wall = wall_model.inner_wall_temperature()
        allowable = _material_strength.allowable_rupture_stress_pa(
            tensile, outer_wall, factor_value
        )
        stress = _material_strength.von_mises_pa(pressure, diameter / 2.0, wall)
        flange_rating = _material_strength.temperature_reduced_flange_rating_pa(
            flange_ambient, outer_wall
        )
        trajectory.append(
            RupturePoint(
                time=from_si(time, "s"),
                pressure=from_si(pressure, "Pa"),
                liquid_temperature=from_si(liquid_temperature, "K"),
                inner_wall_temperature=from_si(inner_wall, "K"),
                outer_wall_temperature=from_si(outer_wall, "K"),
                von_mises_stress=from_si(stress, "Pa"),
                allowable_stress=from_si(allowable, "Pa"),
                flange_rating=from_si(flange_rating, "Pa"),
            )
        )
        if set_pressure is not None and pressure >= set_pressure and time_to_relief is None:
            time_to_relief = time
        if vacuum and time_to_vapor is None:
            try:
                state = _fluid_state.flash(
                    mixture,
                    ideal_gas,
                    liquid_temperature,
                    pressure,
                    [float(value) for value in z],
                    bool(multiphase_check),
                )
                if state.gas_mole_fraction() > 1.0e-10:
                    time_to_vapor = time
            except Exception:
                pass
        if stress >= allowable and time_to_pipe is None:
            time_to_pipe = time
        if pressure >= flange_rating and time_to_flange is None:
            time_to_flange = time
        if stress >= allowable or pressure >= flange_rating:
            break
        if index == steps:
            break

        previous_mean = wall_model.mean_wall_temperature()
        flux = _fire_exposure.incident_heat_flux(fire, outer_wall)
        wall_model.advance_time_step(
            step, liquid_temperature, inner, fire.ambient_temperature, outer, flux
        )
        absorbed = wall_model.heat_absorbed(area, previous_mean)
        incident = flux * area * step
        heat_to_liquid = max(0.0, incident - max(0.0, absorbed))
        liquid_temperature += heat_to_liquid / (liquid_mass * liquid_cp)
        pressure = pressure0 + bulk * expansion * max(0.0, liquid_temperature - temperature0)
        if set_pressure is not None and pressure >= set_pressure and relief_limit:
            pressure = set_pressure

    mode = "none"
    best = math.inf
    if time_to_pipe is not None and time_to_pipe < best:
        best = time_to_pipe
        mode = "pipe_rupture"
    if time_to_flange is not None and time_to_flange < best:
        mode = "flange_failure"
    if mode == "none" and time_to_relief is not None:
        mode = "relief_set_pressure"
    if mode == "none" and time_to_vapor is not None:
        mode = "vapor_pocket"
    minimum = None
    if time_to_pipe is not None and time_to_flange is not None:
        minimum = min(time_to_pipe, time_to_flange)
    elif time_to_pipe is not None:
        minimum = time_to_pipe
    elif time_to_flange is not None:
        minimum = time_to_flange
    last = trajectory[-1]

    return TrappedLiquidFireRuptureResult(
        segment_id="trapped-segment" if segment_id is None else segment_id,
        trajectory=tuple(trajectory),
        limiting_failure_mode=mode,
        rupture_predicted=mode in ("pipe_rupture", "flange_failure"),
        time_to_relief_set=None if time_to_relief is None else from_si(time_to_relief, "s"),
        time_to_vapor_pocket=None if time_to_vapor is None else from_si(time_to_vapor, "s"),
        time_to_pipe_rupture=None if time_to_pipe is None else from_si(time_to_pipe, "s"),
        time_to_flange_failure=None if time_to_flange is None else from_si(time_to_flange, "s"),
        minimum_failure_time=None if minimum is None else from_si(minimum, "s"),
        final_pressure=last.pressure,
        final_liquid_temperature=last.liquid_temperature,
        final_outer_wall_temperature=last.outer_wall_temperature,
        standards_applied=(
            "API 521 / ISO 23251 fire exposure and relief screening basis",
            "ASME B31.3/B31.4 pressure piping stress screening",
            "ASME B16.5 flange class pressure-temperature screening",
            "CCPS/TNO consequence handoff basis for post-rupture source terms",
        ),
        recommendations=(
            "Verify trapped inventory and isolation boundaries from current P&IDs, line lists, "
            "and valve positions.",
            "Use certified material, flange, bolt, and gasket pressure-temperature data for final "
            "design decisions.",
            "Evaluate PSV, thermal relief, drain/vent, or procedural safeguards for blocked-in "
            "liquid segments exposed to fire.",
            "Use the source-term handoff for consequence screening if pipe or flange failure is "
            "predicted.",
        ),
        warnings=tuple(warnings),
    )
