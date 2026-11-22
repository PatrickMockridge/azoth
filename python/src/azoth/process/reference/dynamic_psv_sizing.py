"""``process.dynamic_psv_sizing`` - steady API 521 area against a cycling-PSV transient.

As NeqSim's ``DynamicPsvSizingStudy.run`` computes it. Spec:
``specs/models/process/dynamic_psv_sizing.toml``

The Python twin of ``crates/azoth-process/src/models/dynamic_psv_sizing.rs``: one TP flash
for the gas properties, the steady gas area, and a bisection over the orifice area whose
inner march is the class's ideal-gas vessel with the PSV latch.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import DynamicPsvSizingResult
from azoth.core.units import Q, from_si, input_to_si, quantity
from azoth.core.warnings import Warning
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import phase_state, reduced_parameters
from azoth.eos.reference.pr_molar_volume import MOLAR_GAS_CONSTANT
from azoth.eos.reference.pt_flash import check_composition, pt_flash
from azoth.process.reference.psv_sizing import _gas_area

MODEL_ID = "process.dynamic_psv_sizing"

#: The class's gas constant, J/(mol K).
GAS_CONSTANT = 8.314
#: The class's fallback capacity ratio.
FALLBACK_GAMMA = 1.3
#: The class's bisection iterations.
BISECTION_STEPS = 40


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    steady_required_area: float
    dynamic_required_area: float
    oversizing_ratio: float | None
    allowable_pressure: float
    peak_dynamic_pressure: float
    steady_relief_rate: float


class _Phase(NamedTuple):
    """One phase's molar mass, corrected molar volume and capacities."""

    molar_mass: float
    molar_volume: float
    cp: float
    cv: float


def _phase_of(
    mixture: Any,
    ideal_gas: Any,
    reduced: Any,
    kij: Any,
    t: float,
    p: float,
    composition: list[float],
    root: float,
) -> _Phase:
    molar_mass = molar_mass_of(mixture, composition)
    shift = mixture.volume_shift(composition)
    molar_volume = root * MOLAR_GAS_CONSTANT * t / p - shift
    if not molar_volume > 0.0 or not math.isfinite(molar_volume):
        raise InvalidInputError("initial_p", f"the corrected molar volume is {molar_volume} m3/mol")
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, composition, root)
    return _Phase(
        molar_mass=molar_mass,
        molar_volume=molar_volume,
        cp=capacity.cp,
        cv=capacity.cv,
    )


def _bulk_properties(
    mixture: Any, ideal_gas: Any, t: float, p: float, z: list[float]
) -> tuple[float, float]:
    """The class's bulk density and capacity ratio after one TP flash."""
    reduced = reduced_parameters(mixture, t, p)
    kij = mixture.effective_kij(reduced.t_kelvin)
    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), z)
    if flash.phase == "two_phase":
        beta = flash.vapour_fraction
        if beta is None:
            raise InvalidInputError(
                "initial_p", "the flash split the feed and reported no vapour fraction"
            )
        vapour = _phase_of(mixture, ideal_gas, reduced, kij, t, p, list(flash.y), flash.z_vapour)
        liquid = _phase_of(mixture, ideal_gas, reduced, kij, t, p, list(flash.x), flash.z_liquid)
        mass = beta * vapour.molar_mass + (1.0 - beta) * liquid.molar_mass
        volume = beta * vapour.molar_volume + (1.0 - beta) * liquid.molar_volume
        cp = beta * vapour.cp + (1.0 - beta) * liquid.cp
        cv = beta * vapour.cv + (1.0 - beta) * liquid.cv
        return mass / volume, cp / cv
    if flash.phase == "trivial":
        raise InvalidInputError(
            "initial_p", "the flash found a trivial solution at the initial state"
        )
    root = phase_state(reduced, kij, z, liquid=flash.phase != "all_vapour").z
    phase = _phase_of(mixture, ideal_gas, reduced, kij, t, p, z, root)
    return phase.molar_mass / phase.molar_volume, phase.cp / phase.cv


def _valve_step(
    set_pressure: float,
    blowdown_fraction: float,
    discharge_coefficient: float,
    orifice_area: float,
    back_pressure: float,
    upstream_pressure: float,
    upstream_temperature: float,
    molar_mass: float,
    heat_capacity_ratio: float,
    compressibility: float,
    was_open: bool,
    prior_cycle_count: float,
) -> tuple[bool, float, float]:
    """The latch and the flow, over SI values; the same arithmetic as the valve model."""
    reseat = set_pressure * (1.0 - blowdown_fraction)
    is_open = was_open
    cycles = prior_cycle_count
    if not is_open and upstream_pressure >= set_pressure:
        is_open = True
        cycles += 1.0
    elif is_open and upstream_pressure <= reseat:
        is_open = False
    mass_flow = 0.0
    if is_open:
        gamma = heat_capacity_ratio
        critical_ratio = math.pow(2.0 / (gamma + 1.0), gamma / (gamma - 1.0))
        pressure_ratio = back_pressure / upstream_pressure
        if pressure_ratio <= critical_ratio:
            mass_flow = (
                discharge_coefficient
                * orifice_area
                * upstream_pressure
                * math.sqrt(
                    gamma * molar_mass / (compressibility * GAS_CONSTANT * upstream_temperature)
                )
                * math.pow(2.0 / (gamma + 1.0), (gamma + 1.0) / (2.0 * (gamma - 1.0)))
            )
        else:
            term = (2.0 * gamma / (gamma - 1.0)) * (
                math.pow(pressure_ratio, 2.0 / gamma)
                - math.pow(pressure_ratio, (gamma + 1.0) / gamma)
            )
            if term < 0.0:
                term = 0.0
            mass_flow = (
                discharge_coefficient
                * orifice_area
                * upstream_pressure
                * math.sqrt(molar_mass / (compressibility * GAS_CONSTANT * upstream_temperature))
                * math.sqrt(term)
            )
        if mass_flow < 0.0 or math.isnan(mass_flow):
            mass_flow = 0.0
    return is_open, cycles, mass_flow


def _peak_pressure(
    area: float,
    set_pressure: float,
    blowdown_fraction: float,
    discharge_coefficient: float,
    back_pressure: float,
    fire_heat_input: float,
    vessel_volume: float,
    initial_pressure_fraction: float,
    initial_temperature: float,
    molar_mass: float,
    gamma: float,
    compressibility: float,
    time_step: float,
    max_time: float,
) -> float:
    """The peak vessel pressure over the class's ideal-gas PSV-cycling march."""
    cv_mass = (GAS_CONSTANT / molar_mass) / (gamma - 1.0)
    cp_mass = gamma * cv_mass
    temperature = initial_temperature
    start = set_pressure * initial_pressure_fraction
    moles = start * vessel_volume / (compressibility * GAS_CONSTANT * temperature)
    mass = moles * molar_mass
    peak = start
    time = 0.0
    is_open = False
    cycles = 0.0
    while time < max_time and mass > 1.0e-6:
        pressure = moles * compressibility * GAS_CONSTANT * temperature / vessel_volume
        if pressure > peak:
            peak = pressure
        is_open, cycles, mdot = _valve_step(
            set_pressure,
            blowdown_fraction,
            discharge_coefficient,
            area,
            back_pressure,
            pressure,
            temperature,
            molar_mass,
            gamma,
            compressibility,
            is_open,
            cycles,
        )
        dm = min(mdot * time_step, mass)
        u_old = mass * cv_mass * temperature
        u_new = u_old + fire_heat_input * time_step - cp_mass * temperature * dm
        mass_new = mass - dm
        if mass_new <= 1.0e-9:
            break
        temperature = u_new / (mass_new * cv_mass)
        if temperature <= 0.0 or not math.isfinite(temperature):
            break
        mass = mass_new
        moles = mass / molar_mass
        time += time_step
    return peak


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
    fire_heat_input: Q,
    set_pressure: Q,
    overpressure_fraction: float,
    back_pressure: Q,
    blowdown_fraction: float | None = None,
    discharge_coefficient: float | None = None,
    latent_heat: float | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    initial_pressure_fraction: float | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []
    initial_t_si = input_to_si(spec, "initial_t", initial_t)
    initial_p_si = input_to_si(spec, "initial_p", initial_p)
    volume = input_to_si(spec, "vessel_volume", vessel_volume)
    fire = input_to_si(spec, "fire_heat_input", fire_heat_input)
    set_si = input_to_si(spec, "set_pressure", set_pressure)
    overpressure = float(overpressure_fraction)
    back = input_to_si(spec, "back_pressure", back_pressure)
    blowdown = 0.1 if blowdown_fraction is None else float(blowdown_fraction)
    coefficient = 0.975 if discharge_coefficient is None else float(discharge_coefficient)
    latent = 350_000.0 if latent_heat is None else float(latent_heat)
    step_seconds = 0.5 if time_step is None else input_to_si(spec, "time_step", time_step)
    span = 1200.0 if max_time is None else input_to_si(spec, "max_time", max_time)
    start_fraction = 0.9 if initial_pressure_fraction is None else float(initial_pressure_fraction)

    apply_checks(
        checks_for(spec).on_input,
        {
            "initial_t": initial_t_si,
            "initial_p": initial_p_si,
            "vessel_volume": volume,
            "fire_heat_input": fire,
            "set_pressure": set_si,
            "overpressure_fraction": overpressure,
            "back_pressure": back,
            "blowdown_fraction": blowdown,
            "discharge_coefficient": coefficient,
            "latent_heat": latent,
            "time_step": step_seconds,
            "max_time": span,
            "initial_pressure_fraction": start_fraction,
        }.get,
        warnings,
    )
    for name, value in (
        ("initial_t", initial_t_si),
        ("initial_p", initial_p_si),
        ("vessel_volume", volume),
        ("fire_heat_input", fire),
        ("set_pressure", set_si),
        ("overpressure_fraction", overpressure),
        ("back_pressure", back),
        ("blowdown_fraction", blowdown),
        ("discharge_coefficient", coefficient),
        ("latent_heat", latent),
        ("time_step", step_seconds),
        ("max_time", span),
        ("initial_pressure_fraction", start_fraction),
    ):
        if not math.isfinite(value):
            raise InvalidInputError(
                name, "a non-finite value is not a number this calculation publishes"
            )
    if volume <= 0.0 or fire <= 0.0 or set_si <= 0.0 or overpressure <= 0.0 or back <= 0.0:
        raise InvalidInputError(
            "vessel_volume",
            "the class requires the volume, fire heat, pressures and overpressure to be positive",
        )
    if blowdown <= 0.0 or blowdown >= 1.0:
        raise InvalidInputError("blowdown_fraction", "the class refuses a fraction outside (0, 1)")
    if coefficient <= 0.0 or latent <= 0.0 or step_seconds <= 0.0 or span <= 0.0:
        raise InvalidInputError(
            "discharge_coefficient",
            "the coefficient, latent heat, step and horizon must be positive",
        )
    if initial_t_si <= 0.0 or initial_p_si <= 0.0:
        raise InvalidInputError(
            "initial_t", "the initial temperature and pressure must be positive"
        )

    mixture, ideal_gas = _components.mixture_of([str(name) for name in components], eos="pr")
    check_composition([float(value) for value in initial_z], len(mixture.components), "initial_z")
    z = [float(value) for value in initial_z]
    if any(not math.isfinite(value) for value in initial_z):
        raise InvalidInputError("initial_z", "an initial composition must be finite")
    density, ratio = _bulk_properties(mixture, ideal_gas, initial_t_si, initial_p_si, z)
    molar_mass = molar_mass_of(mixture, z)
    gamma = ratio if ratio > 1.0 and math.isfinite(ratio) else FALLBACK_GAMMA
    compressibility = (
        set_si * molar_mass / (density * GAS_CONSTANT * initial_t_si)
        if density > 0.0 and molar_mass > 0.0
        else 1.0
    )
    if compressibility <= 0.0 or not math.isfinite(compressibility):
        compressibility = 1.0

    allowable = set_si * (1.0 + overpressure)
    relief_rate = fire / latent
    steady_area = float(
        _gas_area(
            relief_rate,
            set_si,
            overpressure,
            back,
            initial_t_si,
            molar_mass,
            compressibility,
            gamma,
            False,
            False,
        )["required_area"]
    )

    def peak(area: float) -> float:
        return _peak_pressure(
            area,
            set_si,
            blowdown,
            coefficient,
            back,
            fire,
            volume,
            start_fraction,
            initial_t_si,
            molar_mass,
            gamma,
            compressibility,
            step_seconds,
            span,
        )

    low = steady_area * 1.0e-3
    high = steady_area * 3.0
    peak_high = peak(high)
    if peak_high > allowable:
        high = steady_area * 10.0
        peak_high = peak(high)
    dynamic_area = high
    peak_dynamic = peak_high
    for _ in range(BISECTION_STEPS):
        mid = 0.5 * (low + high)
        candidate = peak(mid)
        if candidate <= allowable:
            high = mid
            dynamic_area = mid
            peak_dynamic = candidate
        else:
            low = mid
        if (high - low) <= 1.0e-9 * steady_area:
            break

    return (
        _Solution(
            steady_required_area=steady_area,
            dynamic_required_area=dynamic_area,
            oversizing_ratio=steady_area / dynamic_area if dynamic_area > 0.0 else None,
            allowable_pressure=allowable,
            peak_dynamic_pressure=peak_dynamic,
            steady_relief_rate=relief_rate,
        ),
        warnings,
    )


def dynamic_psv_sizing(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    fire_heat_input: Q,
    set_pressure: Q,
    overpressure_fraction: float,
    back_pressure: Q,
    blowdown_fraction: float | None = None,
    discharge_coefficient: float | None = None,
    latent_heat: float | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    initial_pressure_fraction: float | None = None,
) -> DynamicPsvSizingResult:
    """The steady API 521 area and the dynamically required area for a cycling PSV.

    The dynamic march is an ideal-gas lumped vessel seeded from one TP flash; its start
    pressure is ``set_pressure x initial_pressure_fraction``, so the fluid's stated pressure
    enters only through the flash and the compressibility.

    Args:
        components: the vessel gas's substances.
        initial_z: the gas's composition.
        initial_t: the gas temperature at the initial flash.
        initial_p: the gas pressure at the initial flash.
        vessel_volume: the vessel's internal volume.
        fire_heat_input: the constant fire heat input.
        set_pressure: the PSV set pressure.
        overpressure_fraction: the allowable accumulation fraction.
        back_pressure: the downstream back pressure.
        blowdown_fraction: the PSV blowdown fraction; absent is 0.1.
        discharge_coefficient: the PSV discharge coefficient; absent is 0.975.
        latent_heat: the latent heat in J/kg; absent is 350000.
        time_step: the march's step; absent is 0.5 s.
        max_time: the march's horizon; absent is 1200 s.
        initial_pressure_fraction: the start pressure fraction; absent is 0.9.

    Returns:
        The steady and dynamic areas, the oversizing ratio and the peak pressure.

    Raises:
        InvalidInputError: for a non-positive dimension, an invalid fraction or a feed state
            the flash cannot resolve.
    """
    solution, warnings = _evaluate(
        components,
        initial_z,
        initial_t,
        initial_p,
        vessel_volume,
        fire_heat_input,
        set_pressure,
        overpressure_fraction,
        back_pressure,
        blowdown_fraction,
        discharge_coefficient,
        latent_heat,
        time_step,
        max_time,
        initial_pressure_fraction,
    )
    return DynamicPsvSizingResult(
        steady_required_area=from_si(solution.steady_required_area, "m**2"),
        dynamic_required_area=from_si(solution.dynamic_required_area, "m**2"),
        oversizing_ratio=solution.oversizing_ratio,
        allowable_pressure=from_si(solution.allowable_pressure, "Pa"),
        peak_dynamic_pressure=from_si(solution.peak_dynamic_pressure, "Pa"),
        steady_relief_rate=from_si(solution.steady_relief_rate, "kg/s"),
        warnings=tuple(warnings),
    )
