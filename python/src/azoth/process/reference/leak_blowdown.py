"""``process.leak_blowdown`` - NeqSim's legacy vessel-blowdown time loop.

Spec: ``specs/models/process/leak_blowdown.toml``

The Python twin of ``crates/azoth-process/src/models/leak_blowdown.rs``, written to mirror it.
The port source is ``LeakModel.calculateSourceTerm(duration, timeStep)``: the inventory at
fixed volume and internal energy, the substep loop that removes at most one percent of the
mass at a time, the simplified orifice, the capped jet velocity, the unclamped momentum and
the VU flash that solves the remaining state. The modern ``calculateReleaseFlow`` path is
:mod:`azoth.process.reference.hem_release`.

The class's silent fallbacks - ``gamma = 1.3``, methane's molar mass, ``rho = 1``, a NaN
rate as zero - are refused here, and so is any state that is not a single gas phase.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import LeakBlowdownDiagnostic, LeakBlowdownPoint, LeakBlowdownResult
from azoth.core.units import Q, from_si, input_to_si, quantity
from azoth.core.warnings import Warning
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import phase_state, reduced_parameters
from azoth.eos.reference.molar_enthalpy_entropy import molar_enthalpy_entropy
from azoth.eos.reference.pr_molar_volume import MOLAR_GAS_CONSTANT
from azoth.eos.reference.pt_flash import check_composition, pt_flash
from azoth.eos.reference.vu_flash import vu_flash
from azoth.eos.reference.vu_flash_single_comp import vu_flash_single_comp

MODEL_ID = "process.leak_blowdown"

#: The class's fixed gas constant, J/(kmol K).
R = 8314.0
#: The largest mass one substep may remove, as a fraction of the current inventory.
SUBSTEP_MASS_FRACTION = 0.01
#: The class's substep cap.
MAX_SUBSTEPS = 1_000_000
#: The receiving-pressure event's absolute bracket, Pa.
RECEIVING_PRESSURE_FLOOR = 1e-3
#: The receiving-pressure event's relative bracket.
RECEIVING_PRESSURE_RELATIVE = 1e-9
#: The closure check's relative tolerance.
CLOSURE_TOLERANCE = 1e-7
#: The declared-point cap.
MAX_POINTS = 100_000
#: The velocity cap, as a multiple of the ideal-gas sound speed.
VELOCITY_CAP = 1.2


class _Failure(Exception):
    """A state failure the class would throw out of the loop; the port reports it."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


class _Point(NamedTuple):
    time: float
    mass_flow_rate: float
    temperature: float
    pressure: float
    vapour_mole_fraction: float
    jet_velocity: float
    jet_momentum: float
    droplet_smd: float


class _Phase(NamedTuple):
    density: float
    cp: float
    cv: float
    molar_enthalpy: float


class _State(NamedTuple):
    temperature: float
    pressure: float
    density: float
    gamma: float
    molar_enthalpy: float
    compressibility: float

    def molar_internal_energy(self) -> float:
        return self.molar_enthalpy - self.compressibility * MOLAR_GAS_CONSTANT * self.temperature


class _Advanced(NamedTuple):
    state: _State
    moles: float
    mass: float
    energy: float


class _Outcome(NamedTuple):
    status: str
    scenario_name: str
    hole_diameter: float
    number_of_points: int
    points: tuple[_Point, ...]
    total_mass_released: float | None
    peak_mass_flow_rate: float | None
    time_to_empty: float | None
    diagnostics: tuple[tuple[str, str], ...]
    initial_mass: float
    initial_density: float
    initial_gamma: float
    gammas: tuple[float, ...]


def leak_blowdown(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    duration: Q,
    vessel_volume: Q | None = None,
    hole_diameter: Q | None = None,
    discharge_coefficient: float | None = None,
    back_pressure: Q | None = None,
    time_step: Q | None = None,
    scenario_name: str | None = None,
) -> LeakBlowdownResult:
    """Blow a vessel down through a circular opening."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    t = input_to_si(spec, "initial_t", initial_t)
    p = input_to_si(spec, "initial_p", initial_p)
    span = input_to_si(spec, "duration", duration)
    volume = 1.0 if vessel_volume is None else input_to_si(spec, "vessel_volume", vessel_volume)
    diameter = 0.01 if hole_diameter is None else input_to_si(spec, "hole_diameter", hole_diameter)
    coefficient = 0.62 if discharge_coefficient is None else float(discharge_coefficient)
    back = 101325.0 if back_pressure is None else input_to_si(spec, "back_pressure", back_pressure)
    step = 1.0 if time_step is None else input_to_si(spec, "time_step", time_step)
    scenario = "Release" if scenario_name is None else scenario_name
    apply_checks(
        checks.on_input,
        {
            "initial_t": t,
            "initial_p": p,
            "duration": span,
            "vessel_volume": volume,
            "hole_diameter": diameter,
            "discharge_coefficient": coefficient,
            "back_pressure": back,
            "time_step": step,
        }.get,
        warnings,
    )
    if not 0.0 < coefficient <= 1.0:
        raise InvalidInputError(
            "discharge_coefficient",
            f"the effective-area factor must be in (0, 1], and {coefficient} is not",
        )
    if not span > 0.0 or not step > 0.0:
        raise InvalidInputError(
            "duration", f"a positive duration and step are required, and they are {span} and {step}"
        )
    declared = math.ceil(span / step)
    if declared < 0 or declared > MAX_POINTS - 1:
        raise InvalidInputError(
            "duration",
            f"the declared point count is {declared}+1, and the port's cap is {MAX_POINTS}",
        )

    try:
        outcome = _states(
            components,
            list(initial_z),
            t,
            p,
            span,
            volume,
            diameter,
            coefficient,
            back,
            step,
            scenario,
        )
    except _Failure as failure:
        return LeakBlowdownResult(
            status="invalid",
            scenario_name=scenario,
            hole_diameter=from_si(diameter, "m"),
            number_of_points=declared + 1,
            recorded_points=0,
            points=(),
            total_mass_released=None,
            peak_mass_flow_rate=None,
            time_to_empty=None,
            diagnostics=(LeakBlowdownDiagnostic(code=failure.code, message=str(failure)),),
            diagnostic_codes=(failure.code,),
            warnings=tuple(warnings),
        )

    return LeakBlowdownResult(
        status=outcome.status,
        scenario_name=outcome.scenario_name,
        hole_diameter=from_si(outcome.hole_diameter, "m"),
        number_of_points=outcome.number_of_points,
        recorded_points=len(outcome.points),
        points=tuple(
            LeakBlowdownPoint(
                time=from_si(row.time, "s"),
                mass_flow_rate=from_si(row.mass_flow_rate, "kg/s"),
                temperature=from_si(row.temperature, "K"),
                pressure=from_si(row.pressure, "Pa"),
                vapour_mole_fraction=row.vapour_mole_fraction,
                jet_velocity=from_si(row.jet_velocity, "m/s"),
                jet_momentum=from_si(row.jet_momentum, "N"),
                droplet_smd=from_si(row.droplet_smd, "m"),
            )
            for row in outcome.points
        ),
        total_mass_released=(
            None
            if outcome.total_mass_released is None
            else from_si(outcome.total_mass_released, "kg")
        ),
        peak_mass_flow_rate=(
            None
            if outcome.peak_mass_flow_rate is None
            else from_si(outcome.peak_mass_flow_rate, "kg/s")
        ),
        time_to_empty=(
            None if outcome.time_to_empty is None else from_si(outcome.time_to_empty, "s")
        ),
        diagnostics=tuple(
            LeakBlowdownDiagnostic(code=code, message=message)
            for code, message in outcome.diagnostics
        ),
        diagnostic_codes=tuple(code for code, _ in outcome.diagnostics),
        warnings=tuple(warnings),
    )


def _states(
    components: list[str],
    z: list[float],
    t: float,
    p: float,
    duration: float,
    volume: float,
    diameter: float,
    coefficient: float,
    back: float,
    step: float,
    scenario: str,
) -> _Outcome:
    """The whole calculation in SI magnitudes, for the public function and the layer diff."""
    mixture, ideal_gas = _components.mixture_of(components, eos="pr")
    if len(z) != len(mixture.components):
        raise InvalidInputError(
            "initial_z",
            f"a mixture of {len(mixture.components)} components needs that many mole "
            f"fractions, and {z!r} has {len(z)}",
        )
    check_composition(list(z), len(mixture.components), "initial_z")
    if any(not math.isfinite(value) for value in z):
        raise InvalidInputError("initial_z", "a mole fraction must be finite")

    diagnostics: list[tuple[str, str]] = [
        (
            "MODEL_ASSUMPTIONS",
            "Rigid adiabatic vessel; equilibrium flashes; simplified orifice; no slip "
            "or friction in the jet",
        )
    ]
    if p <= back:
        diagnostics.append(
            ("NO_FORWARD_FLOW", "The initial pressure is at or below the receiving pressure")
        )

    state = _flash_state(mixture, ideal_gas, t, p, z)
    # The class scales the inventory by the **EOS** volume, ``V / (Z R T / P)``, not by
    # the physical-properties density the rate uses. The two differ by the volume shift.
    initial_molar_volume = state.compressibility * MOLAR_GAS_CONSTANT * t / p
    if not initial_molar_volume > 0.0 or not math.isfinite(initial_molar_volume):
        raise _Failure(
            "DENSITY_UNAVAILABLE", f"the initial molar volume is {initial_molar_volume} m3/mol"
        )
    initial_gamma = state.gamma
    initial_density = state.density

    molar_mass = molar_mass_of(mixture, z)
    molar_mass_kmol = 1000.0 * molar_mass
    area = math.pi * diameter * diameter / 4.0
    number_of_points = math.ceil(duration / step) + 1
    moles = volume / initial_molar_volume
    mass = moles * molar_mass
    initial_mass = mass
    if not initial_mass > 0.0 or not math.isfinite(initial_mass):
        raise _Failure(
            "DENSITY_UNAVAILABLE",
            f"the initial inventory is {initial_mass} kg for a {volume} m3 vessel",
        )
    energy = moles * state.molar_internal_energy()
    initial_energy = energy
    total = 0.0
    released_energy = 0.0
    peak = 0.0
    points: list[_Point] = []
    gammas: list[float] = []
    substeps = 0
    stopped = state.pressure <= back
    elapsed = 0.0

    for index in range(number_of_points):
        sample_time = duration if index + 1 == number_of_points else index * step
        while not stopped and elapsed < sample_time:
            substeps += 1
            if substeps > MAX_SUBSTEPS:
                raise _Failure(
                    "SUBSTEP_LIMIT", f"the run reached the class's {MAX_SUBSTEPS} substeps"
                )
            rate = _screening_rate(state, molar_mass_kmol, back, area, coefficient)
            if rate == 0.0:
                stopped = True
                break
            sub_step = min(sample_time - elapsed, SUBSTEP_MASS_FRACTION * mass / rate)
            if not sub_step > 0.0 or elapsed + sub_step <= elapsed:
                raise _Failure(
                    "TIMESTEP_UNREPRESENTABLE",
                    f"a substep of {sub_step} s at {elapsed} s is not representable",
                )
            next_state = _advance(
                mixture,
                ideal_gas,
                state,
                z,
                volume,
                moles,
                mass,
                energy,
                molar_mass,
                rate * sub_step,
                state.pressure,
            )
            tolerance = max(RECEIVING_PRESSURE_FLOOR, back * RECEIVING_PRESSURE_RELATIVE)
            if next_state.state.pressure < back:
                lower = 0.0
                upper = sub_step
                located = False
                for _ in range(64):
                    sub_step = 0.5 * (lower + upper)
                    next_state = _advance(
                        mixture,
                        ideal_gas,
                        state,
                        z,
                        volume,
                        moles,
                        mass,
                        energy,
                        molar_mass,
                        rate * sub_step,
                        state.pressure,
                    )
                    difference = next_state.state.pressure - back
                    if 0.0 <= difference <= tolerance:
                        located = True
                        break
                    if difference >= 0.0:
                        lower = sub_step
                    else:
                        upper = sub_step
                if not located:
                    raise _Failure(
                        "RECEIVING_PRESSURE_EVENT_FAILED",
                        "the receiving-pressure crossing could not be bracketed",
                    )
                stopped = True
            removed = rate * sub_step
            total += removed
            released_energy += removed * state.molar_enthalpy / molar_mass
            elapsed += sub_step
            state = next_state.state
            moles = next_state.moles
            mass = next_state.mass
            energy = next_state.energy
            stopped = stopped or state.pressure <= back + tolerance
        elapsed = sample_time
        rate = 0.0 if stopped else _screening_rate(state, molar_mass_kmol, back, area, coefficient)
        peak = max(peak, rate)
        points.append(
            _record_point(state, rate, sample_time, molar_mass_kmol, back, area, coefficient)
        )
        gammas.append(state.gamma)

    if not _closes(mass + total, initial_mass, initial_mass):
        raise _Failure(
            "CLOSURE_FAILED",
            f"the final inventory {mass} kg plus {total} kg released does not close on "
            f"{initial_mass} kg",
        )
    energy_scale = max(abs(initial_energy), abs(released_energy))
    if not _closes(energy + released_energy, initial_energy, energy_scale):
        raise _Failure(
            "CLOSURE_FAILED",
            f"the final energy {energy} J plus {released_energy} J released does not close",
        )

    time_to_empty = total / (0.5 * peak) if peak > 0.0 else 0.0
    return _Outcome(
        status="valid",
        scenario_name=scenario,
        hole_diameter=diameter,
        number_of_points=number_of_points,
        points=tuple(points),
        total_mass_released=total,
        peak_mass_flow_rate=peak,
        time_to_empty=time_to_empty,
        diagnostics=tuple(diagnostics),
        initial_mass=initial_mass,
        initial_density=initial_density,
        initial_gamma=initial_gamma,
        gammas=tuple(gammas),
    )


def _screening_rate(
    state: _State, molar_mass_kmol: float, back: float, area: float, coefficient: float
) -> float:
    if state.pressure <= back:
        mdot = 0.0
    else:
        critical = (2.0 / (state.gamma + 1.0)) ** (state.gamma / (state.gamma - 1.0))
        ratio = back / state.pressure
        if ratio <= critical:
            factor = (2.0 / (state.gamma + 1.0)) ** (
                (state.gamma + 1.0) / (2.0 * (state.gamma - 1.0))
            )
            mdot = (
                coefficient
                * area
                * state.pressure
                * math.sqrt(state.gamma * molar_mass_kmol / (R * state.temperature))
                * factor
            )
        else:
            drop = state.pressure - back
            mdot = coefficient * area * math.sqrt(2.0 * state.density * drop)
    if not math.isfinite(mdot) or mdot < 0.0:
        raise _Failure("FLASH_FAILED", f"the rate at {state.pressure} Pa is {mdot} kg/s")
    return mdot


def _record_point(
    state: _State,
    rate: float,
    time: float,
    molar_mass_kmol: float,
    back: float,
    area: float,
    coefficient: float,
) -> _Point:
    if rate == 0.0:
        velocity = 0.0
        momentum = 0.0
    else:
        raw = rate / (state.density * area * coefficient)
        sound = math.sqrt(state.gamma * R * state.temperature / molar_mass_kmol)
        velocity = min(raw, VELOCITY_CAP * sound)
        momentum = rate * velocity + (state.pressure - back) * area
    return _Point(
        time=time,
        mass_flow_rate=rate,
        temperature=state.temperature,
        pressure=state.pressure,
        vapour_mole_fraction=1.0,
        jet_velocity=velocity,
        jet_momentum=momentum,
        droplet_smd=0.0,
    )


def _advance(
    mixture: Any,
    ideal_gas: Any,
    before: _State,
    z: list[float],
    volume: float,
    moles: float,
    mass: float,
    energy: float,
    molar_mass: float,
    removed: float,
    previous_pressure: float,
) -> _Advanced:
    new_moles = moles * (1.0 - removed / mass)
    if not math.isfinite(new_moles) or new_moles <= 0.0:
        raise _Failure(
            "VU_CLOSURE_FAILED",
            f"removing {removed} kg of a {mass} kg inventory leaves {new_moles} mol",
        )
    target_energy = energy - removed * before.molar_enthalpy / molar_mass
    molar_volume = volume / new_moles
    molar_energy = target_energy / new_moles
    if (
        not math.isfinite(molar_volume)
        or molar_volume <= 0.0
        or not math.isfinite(molar_energy)
        or not math.isfinite(target_energy)
    ):
        raise _Failure(
            "VU_CLOSURE_FAILED",
            f"the VU target is {molar_volume} m3/mol at {molar_energy} J/mol",
        )
    try:
        pressure, temperature, _phase = _vu_at(
            mixture, ideal_gas, previous_pressure, molar_volume, molar_energy, z
        )
    except Exception as error:  # the class's VU flash throws out of the loop
        raise _Failure(
            "VU_CLOSURE_FAILED",
            f"the VU flash at {molar_volume} m3/mol and {molar_energy} J/mol refused: {error}",
        ) from error
    state = _flash_state(mixture, ideal_gas, temperature, pressure, z)
    return _Advanced(
        state=state,
        moles=new_moles,
        mass=new_moles * molar_mass,
        energy=target_energy,
    )


def _vu_at(
    mixture: Any, ideal_gas: Any, previous_pressure: float, v: float, u: float, z: list[float]
) -> tuple[float, float, str]:
    """The class's VU dispatch: the pure route first, then the mixture iteration."""
    if len(mixture.components) == 1:
        try:
            single = vu_flash_single_comp(
                mixture,
                ideal_gas,
                quantity(previous_pressure, "Pa"),
                quantity(v, "m**3/mol"),
                quantity(u, "J/mol"),
            )
            return previous_pressure, single.T.to("K").magnitude, "two_phase"
        except Exception:  # the class falls through on any refusal
            pass
    flash = vu_flash(mixture, ideal_gas, quantity(v, "m**3/mol"), quantity(u, "J/mol"), z)
    return flash.P.to("Pa").magnitude, flash.T.to("K").magnitude, flash.phase


def _closes(actual: float, expected: float, scale: float) -> bool:
    return (
        math.isfinite(actual)
        and math.isfinite(expected)
        and abs(actual - expected) <= CLOSURE_TOLERANCE * max(abs(scale), 1e-10)
    )


def _flash_state(
    mixture: Any, ideal_gas: Any, temperature: float, pressure: float, z: list[float]
) -> _State:
    reduced = reduced_parameters(mixture, temperature, pressure)
    kij = mixture.effective_kij(reduced.t_kelvin)
    flash = pt_flash(mixture, quantity(temperature, "K"), quantity(pressure, "Pa"), z)
    if flash.phase != "all_vapour":
        raise _Failure(
            "PHASE_BOUNDARY",
            f"the flash at {pressure:.6f} Pa found a non-gas state, and the class requires a "
            "single gas phase",
        )
    root = phase_state(reduced, kij, z, liquid=False).z
    phase = _phase_of(mixture, ideal_gas, reduced, kij, temperature, pressure, z, root)
    if not phase.density > 0.0 or not math.isfinite(phase.density):
        raise _Failure(
            "DENSITY_UNAVAILABLE", f"the density at {pressure} Pa is {phase.density} kg/m3"
        )
    gamma = phase.cp / phase.cv
    if not math.isfinite(gamma) or not 1.0 < gamma <= 2.0:
        raise _Failure(
            "GAMMA_OUT_OF_RANGE",
            f"the capacity ratio at {pressure} Pa is {gamma}, and the port refuses one outside "
            "(1, 2]",
        )
    return _State(
        temperature=temperature,
        pressure=pressure,
        density=phase.density,
        gamma=gamma,
        molar_enthalpy=phase.molar_enthalpy,
        compressibility=root,
    )


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
        raise _Failure(
            "DENSITY_UNAVAILABLE",
            f"the corrected molar volume at {p} Pa is {molar_volume} m3/mol",
        )
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, composition, root)
    enthalpy = molar_enthalpy_entropy(
        mixture, ideal_gas, quantity(t, "K"), quantity(p, "Pa"), composition, root
    )
    return _Phase(
        density=molar_mass / molar_volume,
        cp=capacity.cp,
        cv=capacity.cv,
        molar_enthalpy=float(enthalpy.h.to("J/mol").magnitude),
    )


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)
