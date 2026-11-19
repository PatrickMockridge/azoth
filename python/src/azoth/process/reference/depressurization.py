"""``process.depressurization`` - the equilibrium vessel blowdown.

As NeqSim's ``DepressurizationSimulator.run`` computes it. Spec:
``specs/models/process/depressurization.toml``

The Python twin of ``crates/azoth-process/src/models/depressurization.rs``: the mole
alignment, the discharge selection, the mass and energy balance, the lumped wall, the fire
boundary, the VU flash and the API 521 summary.
"""

from __future__ import annotations

import math
from itertools import pairwise
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import DepressurizationPoint, DepressurizationResult
from azoth.core.units import Q, from_si, input_to_si, quantity
from azoth.core.warnings import Warning
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import reduced_parameters
from azoth.eos.reference.molar_enthalpy_entropy import molar_enthalpy_entropy
from azoth.eos.reference.pt_flash import check_composition, pt_flash
from azoth.eos.reference.vu_flash import vu_flash
from azoth.eos.reference.vu_flash_single_comp import vu_flash_single_comp

MODEL_ID = "process.depressurization"

#: The class's gas constant, J/(mol K).
GAS_CONSTANT = 8.314
#: Stefan-Boltzmann constant, W/(m2 K4).
STEFAN_BOLTZMANN = 5.670374419e-8
#: The density ceiling above which a phase is not the vapour leaving the valve, kg/m3.
VAPOUR_PHASE_MAX_DENSITY = 300.0
#: The largest share of the vapour phase one step removes.
MAX_VAPOUR_FRACTION_PER_STEP = 0.25
#: The class's fallback capacity ratio.
FALLBACK_GAMMA = 1.3
#: The API 521 criterion horizon, s.
CRITERION_HORIZON_S = 900.0
#: The 7 barg absolute pressure, Pa.
SEVEN_BARG_PA = 8.0e5

PRESETS = {
    "pool_fire_peak": (0.9, 1100.0, 30.0),
    "pool_fire_background": (0.9, 950.0, 30.0),
    "jet_fire_peak": (1.0, 1300.0, 100.0),
    "jet_fire_background": (0.9, 1100.0, 50.0),
}


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    points: list[tuple[float, float, float, float, float, float]]
    initial_pressure: float
    fire_heat_input: float
    fire_heat_routed_to_wall: bool
    vu_flash_fallback_count: float
    vapour_withdrawal_used: bool
    time_to_half_pressure: float | None
    time_to_7_barg: float | None
    min_fluid_temperature: float
    min_wall_temperature: float
    max_pressure: float
    max_fluid_temperature: float
    max_wall_temperature: float
    half_pressure_criterion_met: bool
    seven_barg_criterion_met: bool
    pressure_monotonic_non_increasing: bool
    mass_monotonic_non_increasing: bool
    recorded_points: float


class _Phase(NamedTuple):
    """One phase's properties at its own composition and root."""

    composition: list[float]
    molar_mass: float
    density: float
    cp: float
    cv: float
    molar_enthalpy: float


class _Flash(NamedTuple):
    """The flashed state's phases and its bulk properties."""

    vapour_fraction: float | None
    phases: list[_Phase]
    bulk_molar_mass: float
    bulk_density: float
    bulk_cp: float
    bulk_cv: float
    bulk_molar_enthalpy: float


def _phase_props(
    mixture: Any,
    ideal_gas: Any,
    reduced: Any,
    kij: Any,
    t: float,
    p: float,
    composition: list[float],
    root: float,
) -> _Phase:
    from azoth.eos.reference.pr_molar_volume import MOLAR_GAS_CONSTANT

    molar_mass = molar_mass_of(mixture, composition)
    shift = mixture.volume_shift(composition)
    molar_volume = root * MOLAR_GAS_CONSTANT * t / p - shift
    if not molar_volume > 0.0 or not math.isfinite(molar_volume):
        raise InvalidInputError("initial_p", f"the corrected molar volume is {molar_volume} m3/mol")
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, composition, root)
    enthalpy = molar_enthalpy_entropy(
        mixture, ideal_gas, quantity(t, "K"), quantity(p, "Pa"), composition, root
    )
    return _Phase(
        composition=list(composition),
        molar_mass=molar_mass,
        density=molar_mass / molar_volume,
        cp=capacity.cp,
        cv=capacity.cv,
        molar_enthalpy=enthalpy.h.to("J/mol").magnitude,
    )


def _flash_info(mixture: Any, ideal_gas: Any, t: float, p: float, z: list[float]) -> _Flash:
    from azoth.eos.reference._mixture_state import phase_state

    reduced = reduced_parameters(mixture, t, p)
    kij = mixture.effective_kij(reduced.t_kelvin)
    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), z)
    if flash.phase == "two_phase":
        beta = flash.vapour_fraction
        if beta is None:
            raise InvalidInputError(
                "initial_p", "the flash split the feed and reported no vapour fraction"
            )
        vapour = _phase_props(mixture, ideal_gas, reduced, kij, t, p, list(flash.y), flash.z_vapour)
        liquid = _phase_props(mixture, ideal_gas, reduced, kij, t, p, list(flash.x), flash.z_liquid)
        mass = beta * vapour.molar_mass + (1.0 - beta) * liquid.molar_mass
        volume = beta * (vapour.molar_mass / vapour.density) + (1.0 - beta) * (
            liquid.molar_mass / liquid.density
        )
        return _Flash(
            vapour_fraction=beta,
            phases=[liquid, vapour],
            bulk_molar_mass=mass,
            bulk_density=mass / volume,
            bulk_cp=beta * vapour.cp + (1.0 - beta) * liquid.cp,
            bulk_cv=beta * vapour.cv + (1.0 - beta) * liquid.cv,
            bulk_molar_enthalpy=beta * vapour.molar_enthalpy + (1.0 - beta) * liquid.molar_enthalpy,
        )
    if flash.phase == "trivial":
        raise InvalidInputError(
            "initial_p", "the flash found a trivial solution at the initial state"
        )
    root = phase_state(reduced, kij, z, liquid=flash.phase != "all_vapour").z
    phase = _phase_props(mixture, ideal_gas, reduced, kij, t, p, z, root)
    return _Flash(
        vapour_fraction=None,
        phases=[phase],
        bulk_molar_mass=phase.molar_mass,
        bulk_density=phase.density,
        bulk_cp=phase.cp,
        bulk_cv=phase.cv,
        bulk_molar_enthalpy=phase.molar_enthalpy,
    )


def _vu_at(
    mixture: Any, ideal_gas: Any, previous_pressure: float, v: float, u: float, z: list[float]
) -> tuple[float, float, str, float | None, list[float], list[float]]:
    """The VU flash's pressure, temperature, phase, split and phase compositions."""
    if len(mixture.components) == 1:
        try:
            single = vu_flash_single_comp(
                mixture,
                ideal_gas,
                quantity(previous_pressure, "Pa"),
                quantity(v, "m**3/mol"),
                quantity(u, "J/mol"),
            )
            return (
                previous_pressure,
                single.T.to("K").magnitude,
                "two_phase",
                single.vapour_fraction,
                [1.0],
                [1.0],
            )
        except Exception:  # the class falls through on any refusal
            pass
    flash = vu_flash(mixture, ideal_gas, quantity(v, "m**3/mol"), quantity(u, "J/mol"), z)
    return (
        flash.P.to("Pa").magnitude,
        flash.T.to("K").magnitude,
        flash.phase,
        flash.vapour_fraction,
        list(flash.x),
        list(flash.y),
    )


def _info_from_flash(
    mixture: Any,
    ideal_gas: Any,
    t: float,
    p: float,
    phase: str,
    beta: float | None,
    x: list[float],
    y: list[float],
    z: list[float],
) -> _Flash:
    """The state a VU flash resolved, from the flash's own split rather than a re-flash."""
    from azoth.eos.reference._mixture_state import phase_state

    reduced = reduced_parameters(mixture, t, p)
    kij = mixture.effective_kij(reduced.t_kelvin)
    if phase == "two_phase" and beta is not None:
        vapour_root = phase_state(reduced, kij, y, liquid=False).z
        liquid_root = phase_state(reduced, kij, x, liquid=True).z
        vapour = _phase_props(mixture, ideal_gas, reduced, kij, t, p, y, vapour_root)
        liquid = _phase_props(mixture, ideal_gas, reduced, kij, t, p, x, liquid_root)
        mass = beta * vapour.molar_mass + (1.0 - beta) * liquid.molar_mass
        volume = beta * (vapour.molar_mass / vapour.density) + (1.0 - beta) * (
            liquid.molar_mass / liquid.density
        )
        return _Flash(
            vapour_fraction=beta,
            phases=[liquid, vapour],
            bulk_molar_mass=mass,
            bulk_density=mass / volume,
            bulk_cp=beta * vapour.cp + (1.0 - beta) * liquid.cp,
            bulk_cv=beta * vapour.cv + (1.0 - beta) * liquid.cv,
            bulk_molar_enthalpy=beta * vapour.molar_enthalpy + (1.0 - beta) * liquid.molar_enthalpy,
        )
    root = phase_state(reduced, kij, z, liquid=phase != "all_vapour").z
    single = _phase_props(mixture, ideal_gas, reduced, kij, t, p, z, root)
    return _Flash(
        vapour_fraction=None,
        phases=[single],
        bulk_molar_mass=single.molar_mass,
        bulk_density=single.density,
        bulk_cp=single.cp,
        bulk_cv=single.cv,
        bulk_molar_enthalpy=single.molar_enthalpy,
    )


def _orifice_rate(
    area: float,
    coefficient: float,
    pressure: float,
    temperature: float,
    molar_mass: float,
    gamma: float,
    compressibility: float,
    back_pressure: float,
) -> float:
    critical_ratio = (2.0 / (gamma + 1.0)) ** (gamma / (gamma - 1.0))
    pressure_ratio = back_pressure / pressure
    if pressure_ratio <= critical_ratio:
        rate = (
            coefficient
            * area
            * pressure
            * math.sqrt(gamma * molar_mass / (compressibility * GAS_CONSTANT * temperature))
            * (2.0 / (gamma + 1.0)) ** ((gamma + 1.0) / (2.0 * (gamma - 1.0)))
        )
    else:
        term = (2.0 * gamma / (gamma - 1.0)) * (
            pressure_ratio ** (2.0 / gamma) - pressure_ratio ** ((gamma + 1.0) / gamma)
        )
        if term < 0.0:
            term = 0.0
        rate = (
            coefficient
            * area
            * pressure
            * math.sqrt(molar_mass / (compressibility * GAS_CONSTANT * temperature))
            * math.sqrt(term)
        )
    if rate < 0.0 or math.isnan(rate):
        rate = 0.0
    return float(rate)


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    orifice_diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    fire_heat_input: Q | None = None,
    fire_heat_input_to_wall: bool | None = None,
    fire_exposed_area: Q | None = None,
    fire_preset: str | None = None,
    fire_emissivity: float | None = None,
    fire_flame_temperature: Q | None = None,
    fire_convective_coefficient: Q | None = None,
    wall_mass: Q | None = None,
    wall_area: Q | None = None,
    wall_specific_heat: Q | None = None,
    wall_heat_transfer_coefficient: Q | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    stop_pressure: Q | None = None,
    withdrawal_mode: str | None = None,
) -> tuple[_Solution, list[Warning]]:
    spec = _spec()
    warnings: list[Warning] = []
    initial_t_si = input_to_si(spec, "initial_t", initial_t)
    initial_p_si = input_to_si(spec, "initial_p", initial_p)
    volume = input_to_si(spec, "vessel_volume", vessel_volume)
    diameter = input_to_si(spec, "orifice_diameter", orifice_diameter)
    coefficient = float(discharge_coefficient)
    back = input_to_si(spec, "back_pressure", back_pressure)
    duty = 0.0 if fire_heat_input is None else input_to_si(spec, "fire_heat_input", fire_heat_input)
    route_to_wall = bool(fire_heat_input_to_wall)
    exposed = (
        0.0
        if fire_exposed_area is None
        else input_to_si(spec, "fire_exposed_area", fire_exposed_area)
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
    step_seconds = 1.0 if time_step is None else input_to_si(spec, "time_step", time_step)
    span = 900.0 if max_time is None else input_to_si(spec, "max_time", max_time)
    stop = 1.5e5 if stop_pressure is None else input_to_si(spec, "stop_pressure", stop_pressure)
    mode = "auto" if withdrawal_mode is None else withdrawal_mode
    if mode not in ("auto", "bulk", "vapour"):
        raise InvalidInputError("withdrawal_mode", "the mode must be auto, bulk or vapour")

    explicit_fire = (
        fire_emissivity is not None
        or fire_flame_temperature is not None
        or fire_convective_coefficient is not None
    )
    if fire_preset is not None:
        if explicit_fire:
            raise InvalidInputError(
                "fire_preset",
                "the preset and the explicit flame parameters are two statements of one boundary",
            )
        if fire_preset not in PRESETS:
            raise InvalidInputError("fire_preset", "the preset is not one of the four fire presets")
        emissivity, flame_temperature, convection = PRESETS[fire_preset]
    else:
        emissivity = 0.9 if fire_emissivity is None else float(fire_emissivity)
        flame_temperature = (
            1100.0
            if fire_flame_temperature is None
            else input_to_si(spec, "fire_flame_temperature", fire_flame_temperature)
        )
        convection = (
            30.0
            if fire_convective_coefficient is None
            else input_to_si(spec, "fire_convective_coefficient", fire_convective_coefficient)
        )

    apply_checks(
        checks_for(spec).on_input,
        {
            "initial_t": initial_t_si,
            "initial_p": initial_p_si,
            "vessel_volume": volume,
            "orifice_diameter": diameter,
            "discharge_coefficient": coefficient,
            "back_pressure": back,
            "time_step": step_seconds,
            "max_time": span,
            "stop_pressure": stop,
        }.get,
        warnings,
    )
    for name, value in (
        ("initial_t", initial_t_si),
        ("initial_p", initial_p_si),
        ("vessel_volume", volume),
        ("orifice_diameter", diameter),
        ("discharge_coefficient", coefficient),
        ("back_pressure", back),
        ("fire_heat_input", duty),
        ("fire_exposed_area", exposed),
        ("fire_emissivity", emissivity),
        ("fire_flame_temperature", flame_temperature),
        ("fire_convective_coefficient", convection),
        ("wall_mass", wall_metal),
        ("wall_area", wall_face),
        ("wall_specific_heat", wall_capacity),
        ("wall_heat_transfer_coefficient", wall_film),
        ("time_step", step_seconds),
        ("max_time", span),
        ("stop_pressure", stop),
    ):
        if not math.isfinite(value):
            raise InvalidInputError(
                name, "a non-finite value is not a number this calculation publishes"
            )
    if volume <= 0.0 or diameter <= 0.0 or initial_t_si <= 0.0 or initial_p_si <= 0.0:
        raise InvalidInputError(
            "vessel_volume",
            "the class requires the volume, orifice and initial state to be positive",
        )
    if coefficient <= 0.0 or coefficient > 1.0:
        raise InvalidInputError(
            "discharge_coefficient", "the port refuses a coefficient outside (0, 1]"
        )
    if back < 0.0:
        raise InvalidInputError("back_pressure", "a negative receiving pressure is not a pressure")
    if step_seconds <= 0.0 or span <= 0.0 or stop <= 0.0:
        raise InvalidInputError(
            "time_step", "the class refuses a non-positive step, horizon or stop pressure"
        )

    mixture, ideal_gas = _components.mixture_of([str(name) for name in components], eos="pr")
    check_composition([float(value) for value in initial_z], len(mixture.components), "initial_z")
    composition = [float(value) for value in initial_z]
    p0 = initial_p_si
    state = _flash_info(mixture, ideal_gas, initial_t_si, initial_p_si, composition)
    pressure = initial_p_si
    temperature = initial_t_si
    mass = state.bulk_density * volume
    moles = mass / state.bulk_molar_mass
    energy = state.bulk_molar_enthalpy * moles - pressure * volume
    wall_temperature = temperature
    fallbacks = 0.0
    vapour_used = False
    time = 0.0
    area = math.pi * 0.25 * diameter * diameter
    points: list[tuple[float, float, float, float, float, float]] = [
        (time, pressure, temperature, mass, wall_temperature, 0.0)
    ]
    min_fluid_temperature = temperature
    min_wall_temperature = wall_temperature
    max_pressure = pressure
    max_fluid_temperature = temperature
    max_wall_temperature = wall_temperature

    while time < span and pressure > stop and mass > 1.0e-6:
        selected: int | None = None
        if mode != "bulk" and state.vapour_fraction is not None and len(state.phases) > 1:
            lightest = min(range(len(state.phases)), key=lambda i: state.phases[i].density)
            if state.phases[lightest].density <= VAPOUR_PHASE_MAX_DENSITY:
                selected = lightest
        if selected is not None:
            phase = state.phases[selected]
            molar_mass = phase.molar_mass
            cp, cv, density, enthalpy = phase.cp, phase.cv, phase.density, phase.molar_enthalpy
        else:
            molar_mass = state.bulk_molar_mass
            cp, cv, density, enthalpy = (
                state.bulk_cp,
                state.bulk_cv,
                state.bulk_density,
                state.bulk_molar_enthalpy,
            )
        gamma = cp / cv if cv > 0.0 else FALLBACK_GAMMA
        compressibility = pressure * molar_mass / (density * GAS_CONSTANT * temperature)
        if compressibility <= 0.0 or not math.isfinite(compressibility):
            compressibility = 1.0
        rate = _orifice_rate(
            area,
            coefficient,
            pressure,
            temperature,
            molar_mass,
            gamma,
            compressibility,
            back,
        )
        dm = min(rate * step_seconds, mass)
        specific_enthalpy = enthalpy / molar_mass

        fire_through_wall = route_to_wall and wall_metal > 0.0 and wall_face > 0.0
        if exposed > 0.0:
            surface = wall_temperature if fire_through_wall else temperature
            radiative = emissivity * STEFAN_BOLTZMANN * (flame_temperature**4 - surface**4)
            convective = convection * (flame_temperature - surface)
            effective_fire = max(radiative + convective, 0.0) * exposed
        else:
            effective_fire = duty
        direct_fire = 0.0 if fire_through_wall else effective_fire
        q_wall = (
            wall_film * wall_face * (wall_temperature - temperature) if wall_metal > 0.0 else 0.0
        )
        if wall_metal > 0.0:
            external = effective_fire if fire_through_wall else 0.0
            wall_temperature += (external - q_wall) * step_seconds / (wall_metal * wall_capacity)
        u_before = energy
        new_moles = moles
        new_composition = list(composition)
        if selected is not None:
            vapour_used = True
            beta = state.vapour_fraction
            assert beta is not None
            phase_moles = (beta if selected == 1 else 1.0 - beta) * moles
            requested = dm / molar_mass
            removed = min(requested, phase_moles * MAX_VAPOUR_FRACTION_PER_STEP)
            dm = removed * molar_mass
            new_moles = max(moles - removed, 0.0)
            if new_moles > 0.0:
                new_composition = [
                    (moles * bulk - removed * phase_fraction) / new_moles
                    for bulk, phase_fraction in zip(
                        composition, state.phases[selected].composition, strict=True
                    )
                ]
                total = sum(new_composition)
                if total > 0.0:
                    new_composition = [value / total for value in new_composition]
            energy = (
                u_before
                - specific_enthalpy * dm
                + direct_fire * step_seconds
                + q_wall * step_seconds
            )
            new_mass = new_moles * molar_mass_of(mixture, new_composition)
        else:
            new_mass = mass - dm
            new_moles *= new_mass / mass
            energy = (
                u_before
                - specific_enthalpy * dm
                + direct_fire * step_seconds
                + q_wall * step_seconds
            )
        if new_moles <= 0.0 or new_mass <= 0.0:
            break
        molar_volume = volume / new_moles
        molar_energy = energy / new_moles
        try:
            pressure, temperature, phase_name, beta_out, x_out, y_out = _vu_at(
                mixture, ideal_gas, pressure, molar_volume, molar_energy, new_composition
            )
        except Exception:  # the class's ideal-gas fallback
            fallbacks += 1.0
            pressure = (
                new_mass * GAS_CONSTANT * temperature / (molar_mass * volume) * compressibility
            )
            pressure = max(pressure, back)
            state = _flash_info(mixture, ideal_gas, temperature, pressure, new_composition)
        else:
            state = _info_from_flash(
                mixture,
                ideal_gas,
                temperature,
                pressure,
                phase_name,
                beta_out,
                x_out,
                y_out,
                new_composition,
            )
        composition = new_composition
        moles = new_moles
        mass = new_moles * molar_mass_of(mixture, composition) if selected is not None else new_mass
        time += step_seconds
        min_fluid_temperature = min(min_fluid_temperature, temperature)
        min_wall_temperature = min(min_wall_temperature, wall_temperature)
        max_pressure = max(max_pressure, pressure)
        max_fluid_temperature = max(max_fluid_temperature, temperature)
        max_wall_temperature = max(max_wall_temperature, wall_temperature)
        points.append((time, pressure, temperature, mass, wall_temperature, rate))

    pressure_monotonic = True
    mass_monotonic = True
    for previous, current in pairwise(points):
        tolerance = max(1.0e-3, abs(previous[1]) * 1.0e-4)
        if current[1] > previous[1] + tolerance:
            pressure_monotonic = False
        mass_tolerance = max(1.0e-8, abs(previous[3]) * 1.0e-9)
        if current[3] > previous[3] + mass_tolerance:
            mass_monotonic = False
    half = 0.5 * p0
    time_to_half: float | None = None
    time_to_seven: float | None = None
    for previous, current in pairwise(points):
        if time_to_half is None and previous[1] > half and current[1] <= half:
            time_to_half = _crossing(previous, current, half)
        if time_to_seven is None and previous[1] > SEVEN_BARG_PA and current[1] <= SEVEN_BARG_PA:
            time_to_seven = _crossing(previous, current, SEVEN_BARG_PA)

    return (
        _Solution(
            points=points,
            initial_pressure=p0,
            fire_heat_input=duty,
            fire_heat_routed_to_wall=route_to_wall and wall_metal > 0.0 and wall_face > 0.0,
            vu_flash_fallback_count=fallbacks,
            vapour_withdrawal_used=vapour_used,
            time_to_half_pressure=time_to_half,
            time_to_7_barg=time_to_seven,
            min_fluid_temperature=min_fluid_temperature,
            min_wall_temperature=min_wall_temperature,
            max_pressure=max_pressure,
            max_fluid_temperature=max_fluid_temperature,
            max_wall_temperature=max_wall_temperature,
            half_pressure_criterion_met=time_to_half is not None
            and time_to_half <= CRITERION_HORIZON_S,
            seven_barg_criterion_met=time_to_seven is not None
            and time_to_seven <= CRITERION_HORIZON_S,
            pressure_monotonic_non_increasing=pressure_monotonic,
            mass_monotonic_non_increasing=mass_monotonic,
            recorded_points=float(len(points)),
        ),
        warnings,
    )


def _crossing(
    previous: tuple[float, float, float, float, float, float],
    current: tuple[float, float, float, float, float, float],
    target: float,
) -> float:
    drop = previous[1] - current[1]
    if abs(drop) < 1.0e-12:
        return current[0]
    fraction = min(max((previous[1] - target) / drop, 0.0), 1.0)
    return previous[0] + fraction * (current[0] - previous[0])


def depressurization(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    orifice_diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    fire_heat_input: Q | None = None,
    fire_heat_input_to_wall: bool | None = None,
    fire_exposed_area: Q | None = None,
    fire_preset: str | None = None,
    fire_emissivity: float | None = None,
    fire_flame_temperature: Q | None = None,
    fire_convective_coefficient: Q | None = None,
    wall_mass: Q | None = None,
    wall_area: Q | None = None,
    wall_specific_heat: Q | None = None,
    wall_heat_transfer_coefficient: Q | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    stop_pressure: Q | None = None,
    withdrawal_mode: str | None = None,
) -> DepressurizationResult:
    """The equilibrium vessel blowdown and its API 521 summary.

    AUTO and VAPOUR take the lightest phase of a multiphase inventory when its density is at
    most 300 kg/m3 and remove at most a quarter of it per step; the fire boundary is the
    constant duty or a temperature-dependent flux over the exposed area.

    Args:
        components: the inventory's substances.
        initial_z: the inventory's composition.
        initial_t: the inventory temperature.
        initial_p: the inventory pressure.
        vessel_volume: the vessel's internal volume.
        orifice_diameter: the blowdown opening's diameter.
        discharge_coefficient: the orifice coefficient in (0, 1].
        back_pressure: the receiving absolute pressure.
        fire_heat_input: the constant fire duty; absent is 0.
        fire_heat_input_to_wall: whether the fire heats the wall first; absent is false.
        fire_exposed_area: the temperature-dependent fire's exposed area; absent is 0.
        fire_preset: a screening fire preset; absent uses the explicit trio.
        fire_emissivity: the flame emissivity; absent is 0.9.
        fire_flame_temperature: the flame temperature; absent is 1100 K.
        fire_convective_coefficient: the flame convection coefficient; absent is 30.
        wall_mass: the lumped wall mass; absent is 0.
        wall_area: the wall's internal area; absent is 0.
        wall_specific_heat: the wall's specific heat; absent is 470.
        wall_heat_transfer_coefficient: the inside film coefficient; absent is 50.
        time_step: the explicit step; absent is 1 s.
        max_time: the horizon; absent is 900 s.
        stop_pressure: the stop pressure; absent is 1.5e5 Pa.
        withdrawal_mode: auto, bulk or vapour; absent is auto.

    Returns:
        The recorded history, the criteria and the flags.

    Raises:
        InvalidInputError: for a non-positive dimension, an invalid coefficient, a fire
            ambiguity, an unknown mode or preset, or a feed state the flash cannot resolve.
    """
    solution, warnings = _evaluate(
        components,
        initial_z,
        initial_t,
        initial_p,
        vessel_volume,
        orifice_diameter,
        discharge_coefficient,
        back_pressure,
        fire_heat_input,
        fire_heat_input_to_wall,
        fire_exposed_area,
        fire_preset,
        fire_emissivity,
        fire_flame_temperature,
        fire_convective_coefficient,
        wall_mass,
        wall_area,
        wall_specific_heat,
        wall_heat_transfer_coefficient,
        time_step,
        max_time,
        stop_pressure,
        withdrawal_mode,
    )
    return DepressurizationResult(
        points=tuple(
            DepressurizationPoint(
                time=from_si(point[0], "s"),
                pressure=from_si(point[1], "Pa"),
                temperature=from_si(point[2], "K"),
                mass=from_si(point[3], "kg"),
                wall_temperature=from_si(point[4], "K"),
                mass_flow=from_si(point[5], "kg/s"),
            )
            for point in solution.points
        ),
        initial_pressure=from_si(solution.initial_pressure, "Pa"),
        fire_heat_input=from_si(solution.fire_heat_input, "W"),
        fire_heat_routed_to_wall=solution.fire_heat_routed_to_wall,
        vu_flash_fallback_count=solution.vu_flash_fallback_count,
        vapour_withdrawal_used=solution.vapour_withdrawal_used,
        time_to_half_pressure=(
            None
            if solution.time_to_half_pressure is None
            else from_si(solution.time_to_half_pressure, "s")
        ),
        time_to_7_barg=(
            None if solution.time_to_7_barg is None else from_si(solution.time_to_7_barg, "s")
        ),
        min_fluid_temperature=from_si(solution.min_fluid_temperature, "K"),
        min_wall_temperature=from_si(solution.min_wall_temperature, "K"),
        max_pressure=from_si(solution.max_pressure, "Pa"),
        max_fluid_temperature=from_si(solution.max_fluid_temperature, "K"),
        max_wall_temperature=from_si(solution.max_wall_temperature, "K"),
        half_pressure_criterion_met=solution.half_pressure_criterion_met,
        seven_barg_criterion_met=solution.seven_barg_criterion_met,
        pressure_monotonic_non_increasing=solution.pressure_monotonic_non_increasing,
        mass_monotonic_non_increasing=solution.mass_monotonic_non_increasing,
        recorded_points=solution.recorded_points,
        warnings=tuple(warnings),
    )
