"""``process.real_gas_fanno_release`` - NeqSim's real-gas Fanno pipe release.

Spec: ``specs/models/process/real_gas_fanno_release.toml``

The Python twin of ``crates/azoth-process/src/models/real_gas_fanno_release.rs``. A
one-sided adiabatic constant-area pipe of a single equilibrium gas, marched in ten axial
steps with a predictor-corrector pressure gradient and a PH flash at every energy state.
**The march is slow in pure Python** - thousands of PH flashes per state - so its
cross-implementation cases run in the slow sweep, not the lane. The Rust twin is
`process.real_gas_fanno_release` and the case is held at `1e-3`.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    HemReleaseDiagnostic,
    RealGasFannoReleaseResult,
    RealGasFannoReleaseStation,
)
from azoth.core.units import Q, from_si, input_to_si, quantity
from azoth.core.warnings import Warning
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import phase_state, reduced_parameters
from azoth.eos.reference.molar_enthalpy_entropy import molar_enthalpy_entropy
from azoth.eos.reference.ph_flash import ph_flash
from azoth.eos.reference.pr_molar_volume import MOLAR_GAS_CONSTANT
from azoth.eos.reference.ps_flash import ps_flash
from azoth.eos.reference.pt_flash import check_composition, pt_flash

MODEL_ID = "process.real_gas_fanno_release"

AXIAL_STEPS = 10
ROOT_ITERATIONS = 16
PROPERTY_ITERATIONS = 20
SONIC_LIMIT = 0.9995
PROPERTY_TOLERANCE = 2.0e-8
FLUX_CLOSURE = 2.0e-7
HYDRATE_FORMERS = (
    "methane",
    "ethane",
    "propane",
    "i-butane",
    "n-butane",
    "CO2",
    "H2S",
    "nitrogen",
)


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


@dataclass(frozen=True, slots=True)
class _Gas:
    pressure: float
    temperature: float
    density: float
    molar_enthalpy: float
    molar_entropy: float
    velocity: float
    mach: float


@dataclass(frozen=True, slots=True)
class _Trace:
    completed: bool
    inlet: _Gas | None
    exit: _Gas | None
    failure: str


def real_gas_fanno_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    flow_path_length: Q,
    darcy_friction_factor: float,
) -> RealGasFannoReleaseResult:
    """The release rate of a real-gas Fanno pipe.

    Raises:
        InvalidInputError: for a non-positive input, a one-sided flow path, or a composition
            that is not one entry per component.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    t0 = input_to_si(spec, "upstream_t", upstream_t)
    p0 = input_to_si(spec, "upstream_p", upstream_p)
    diameter_m = input_to_si(spec, "diameter", diameter)
    back = input_to_si(spec, "back_pressure", back_pressure)
    length = input_to_si(spec, "flow_path_length", flow_path_length)
    friction = float(darcy_friction_factor)
    apply_checks(
        checks.on_input,
        {
            "upstream_t": t0,
            "upstream_p": p0,
            "diameter": diameter_m,
            "discharge_coefficient": discharge_coefficient,
            "back_pressure": back,
            "flow_path_length": length,
            "darcy_friction_factor": friction,
        }.get,
        warnings,
    )
    has_length = length > 0.0
    has_friction = friction > 0.0
    if has_length != has_friction:
        raise InvalidInputError(
            "flow_path_length",
            "the pipe length and Darcy friction factor must both be zero or both be positive",
        )
    if not has_length:
        return _failure(
            "unsupported",
            "UNSUPPORTED_REAL_GAS_FANNO_REGIME",
            "Real-gas Fanno model requires positive pipe length and friction",
            warnings,
        )

    mixture, ideal_gas = _components.mixture_of(components, eos="pr")
    if len(upstream_z) != len(mixture.components):
        raise InvalidInputError(
            "upstream_z",
            f"a mixture of {len(mixture.components)} components needs that many mole "
            f"fractions, and {upstream_z!r} has {len(upstream_z)}",
        )
    check_composition(list(upstream_z), len(mixture.components), "upstream_z")
    mass = molar_mass_of(mixture, list(upstream_z))
    names = mixture.names if mixture.names is not None else tuple(components)
    gate = _solid_gate(mixture, names, t0, list(upstream_z))
    if gate is not None:
        return _failure("invalid", "SOLID_RISK_ASSESSMENT_FAILED", gate, warnings)

    diagnostics = [
        (
            "REAL_GAS_FANNO_ASSUMPTIONS",
            "Steady adiabatic constant-area single-equilibrium-gas flow with EOS properties",
        ),
        (
            "QUASI_STEADY_ONE_SIDED_PIPE",
            "One pipe segment from stagnation boundary to release plane; transient waves and "
            "the opposite rupture side are excluded",
        ),
        (
            "SPECIFIED_DARCY_FRICTION",
            "Specified Darcy friction; no roughness or Reynolds correlation inferred",
        ),
        (
            "SOLID_RISK_ASSESSED",
            "UPSTREAM_STAGNATION: Mixture-specific solid and hydrate applicability checks passed",
        ),
    ]

    flash = pt_flash(mixture, quantity(t0, "K"), quantity(p0, "Pa"), list(upstream_z))
    if flash.phase != "all_vapour":
        return _failure(
            "unsupported",
            "UNSUPPORTED_REAL_GAS_FANNO_REGIME",
            "The real-gas Fanno model requires one equilibrium gas phase",
            warnings,
        )
    try:
        upstream = _gas_state(mixture, ideal_gas, list(upstream_z), mass, p0, t0, 0.0)
    except Exception as error:
        return _failure("unsupported", "UNSUPPORTED_REAL_GAS_FANNO_REGIME", str(error), warnings)
    if p0 <= back:
        diagnostics.append(
            ("NO_FORWARD_FLOW", "Upstream pressure is at or below receiving pressure")
        )
        throat = _replace(upstream, velocity=0.0)
        return _success(
            0.0,
            False,
            None,
            None,
            None,
            upstream_z,
            _component_mass_fractions(mixture, list(upstream_z)),
            [
                _row(upstream, "upstream_stagnation"),
                _row(throat, "throat_critical"),
                _row(throat, "orifice_exit"),
            ],
            diagnostics,
            warnings,
        )

    h0 = upstream.molar_enthalpy
    s0 = upstream.molar_entropy
    try:
        sound0 = _sound(mixture, ideal_gas, list(upstream_z), p0, t0)
    except Exception as error:
        return _failure("invalid", "REAL_GAS_FANNO_CALCULATION_FAILED", str(error), warnings)
    upper_flux = upstream.density * sound0 * 1.5
    low_flux = upper_flux * 1.0e-4
    high_flux = upper_flux
    step_length = length / AXIAL_STEPS

    def trace(flux: float) -> _Trace:
        return _trace(
            mixture,
            ideal_gas,
            list(upstream_z),
            mass,
            h0,
            s0,
            flux,
            step_length,
            friction,
            diameter_m,
            p0,
        )

    low_trace = trace(low_flux)
    if not low_trace.completed:
        return _failure(
            "invalid",
            "REAL_GAS_FANNO_CALCULATION_FAILED",
            f"Cannot establish the low-flux pipe solution: {low_trace.failure}",
            warnings,
        )
    critical = low_trace
    for _ in range(ROOT_ITERATIONS):
        trial_flux = 0.5 * (low_flux + high_flux)
        trial = trace(trial_flux)
        if trial.completed and trial.exit is not None and trial.exit.mach < SONIC_LIMIT:
            low_flux = trial_flux
            critical = trial
        else:
            high_flux = trial_flux
    if critical.exit is None:
        return _failure(
            "invalid",
            "REAL_GAS_FANNO_CALCULATION_FAILED",
            "The critical trace has no exit state",
            warnings,
        )
    choked = back <= critical.exit.pressure
    if choked:
        accepted_flux = low_flux
        accepted = critical.exit
    else:
        lower = upper_flux * 1.0e-4
        upper = low_flux
        accepted = critical.exit
        accepted_flux = low_flux
        for _ in range(ROOT_ITERATIONS):
            trial_flux = 0.5 * (lower + upper)
            trial = trace(trial_flux)
            if trial.completed and trial.exit is not None and trial.exit.pressure >= back:
                lower = trial_flux
                accepted = trial.exit
                accepted_flux = trial_flux
            else:
                upper = trial_flux
    energy_error = abs(h0 / mass - accepted.molar_enthalpy / mass - 0.5 * accepted.velocity**2)
    if energy_error > max(0.2, abs(h0 / mass) * 1.0e-6):
        return _failure(
            "invalid",
            "REAL_GAS_FANNO_CALCULATION_FAILED",
            f"Stagnation-energy closure failed: {energy_error} J/kg",
            warnings,
        )
    flux_error = abs(accepted_flux - accepted.density * accepted.velocity) / max(
        1.0, abs(accepted_flux)
    )
    if flux_error > FLUX_CLOSURE:
        return _failure(
            "invalid",
            "REAL_GAS_FANNO_CALCULATION_FAILED",
            f"Mass-flux closure failed: {flux_error}",
            warnings,
        )
    ambient = accepted
    if choked:
        try:
            ps_result = ps_flash(
                mixture,
                ideal_gas,
                quantity(back, "Pa"),
                quantity(s0, "J/(mol*K)"),
                list(upstream_z),
            )
            temperature = ps_result.T.to("K").magnitude
            reduced = reduced_parameters(mixture, temperature, back)
            kij = mixture.effective_kij(reduced.t_kelvin)
            root = phase_state(reduced, kij, list(upstream_z), liquid=False).z
            caloric = molar_enthalpy_entropy(
                mixture,
                ideal_gas,
                quantity(temperature, "K"),
                quantity(back, "Pa"),
                list(upstream_z),
                root,
            )
            enthalpy = caloric.h.to("J/mol").magnitude
            velocity = math.sqrt(max(0.0, 2.0 * (h0 - enthalpy) / mass))
            ambient = _Gas(
                pressure=back,
                temperature=temperature,
                density=back * mass / (MOLAR_GAS_CONSTANT * temperature),
                molar_enthalpy=enthalpy,
                molar_entropy=s0,
                velocity=velocity,
                mach=0.0,
            )
        except Exception as error:
            return _failure("invalid", "REAL_GAS_FANNO_CALCULATION_FAILED", str(error), warnings)
    try:
        sound = _sound(
            mixture, ideal_gas, list(upstream_z), accepted.pressure, accepted.temperature
        )
    except Exception as error:
        return _failure("invalid", "REAL_GAS_FANNO_CALCULATION_FAILED", str(error), warnings)
    area = math.pi * diameter_m * diameter_m / 4.0
    rate = discharge_coefficient * area * accepted_flux
    diagnostics.append(
        (
            "REAL_GAS_FANNO_SOLUTION",
            "EOS march solved; the exit Mach is reported on the result",
        )
    )
    return _success(
        rate,
        choked,
        sound,
        friction * length / diameter_m,
        (critical.inlet.mach if critical.inlet is not None else 0.0, accepted.mach),
        upstream_z,
        _component_mass_fractions(mixture, list(upstream_z)),
        [
            _row(upstream, "upstream_stagnation"),
            _row(accepted, "throat_critical"),
            _row(accepted, "orifice_exit"),
            _row(ambient, "ambient_expanded"),
        ],
        diagnostics,
        warnings,
    )


def _gas_state(
    mixture: Any,
    ideal_gas: Any,
    z: list[float],
    mass: float,
    pressure: float,
    temperature: float,
    mass_flux: float,
) -> _Gas:
    reduced = reduced_parameters(mixture, temperature, pressure)
    kij = mixture.effective_kij(reduced.t_kelvin)
    root = phase_state(reduced, kij, z, liquid=False).z
    density = mass / (root * MOLAR_GAS_CONSTANT * temperature / pressure)
    caloric = molar_enthalpy_entropy(
        mixture, ideal_gas, quantity(temperature, "K"), quantity(pressure, "Pa"), z, root
    )
    sound = _sound(mixture, ideal_gas, z, pressure, temperature)
    velocity = mass_flux / density
    return _Gas(
        pressure=pressure,
        temperature=temperature,
        density=density,
        molar_enthalpy=caloric.h.to("J/mol").magnitude,
        molar_entropy=caloric.s.to("J/(mol*K)").magnitude,
        velocity=velocity,
        mach=velocity / sound,
    )


def _energy_state(
    mixture: Any,
    ideal_gas: Any,
    z: list[float],
    mass: float,
    h0: float,
    mass_flux: float,
    pressure: float,
) -> _Gas:
    if not pressure > 0.0 or not math.isfinite(pressure):
        raise ValueError("the energy-state pressure is not positive")
    density = mass / (MOLAR_GAS_CONSTANT * 300.0 / pressure)
    temperature = 300.0
    for _ in range(PROPERTY_ITERATIONS):
        velocity = mass_flux / density
        static_enthalpy = h0 - 0.5 * velocity * velocity * mass
        flash = ph_flash(
            mixture,
            ideal_gas,
            quantity(pressure, "Pa"),
            quantity(static_enthalpy, "J/mol"),
            z,
        )
        if flash.phase != "all_vapour":
            raise ValueError("The real-gas Fanno model requires one equilibrium gas phase")
        temperature = flash.T.to("K").magnitude
        reduced = reduced_parameters(mixture, temperature, pressure)
        kij = mixture.effective_kij(reduced.t_kelvin)
        root = phase_state(reduced, kij, z, liquid=False).z
        updated = mass / (root * MOLAR_GAS_CONSTANT * temperature / pressure)
        converged = abs(updated - density) <= PROPERTY_TOLERANCE * max(1.0, updated)
        density = updated
        if converged:
            break
    return _gas_state(mixture, ideal_gas, z, mass, pressure, temperature, mass_flux)


def _inlet_state(
    mixture: Any,
    ideal_gas: Any,
    z: list[float],
    mass: float,
    h0: float,
    s0: float,
    mass_flux: float,
    pressure0: float,
    upstream: _Gas,
) -> _Gas:
    sound0 = _sound(mixture, ideal_gas, z, pressure0, upstream.temperature)
    if mass_flux <= upstream.density * sound0 * 1.0e-6:
        velocity = mass_flux / upstream.density
        return _replace(upstream, velocity=velocity, mach=velocity / sound0)
    high_pressure = pressure0
    high = _energy_state(mixture, ideal_gas, z, mass, h0, mass_flux, high_pressure)
    high_error = high.molar_entropy - s0
    low_pressure = pressure0
    low = high
    low_error = high_error
    for index in range(1, 37):
        if low_error * high_error <= 0.0:
            break
        low_pressure = pressure0 * math.exp(-math.log(200.0) * index / 36.0)
        low = _energy_state(mixture, ideal_gas, z, mass, h0, mass_flux, low_pressure)
        low_error = low.molar_entropy - s0
    if low_error * high_error > 0.0:
        raise ValueError("Cannot bracket the subsonic isentropic pipe inlet")
    for _ in range(ROOT_ITERATIONS):
        pressure = 0.5 * (low_pressure + high_pressure)
        middle = _energy_state(mixture, ideal_gas, z, mass, h0, mass_flux, pressure)
        error = middle.molar_entropy - s0
        if error * low_error <= 0.0:
            high_pressure = pressure
            high = middle
            high_error = error
        else:
            low_pressure = pressure
            low = middle
            low_error = error
    result = low if abs(low_error) < abs(high_error) else high
    if result.mach >= 1.0:
        raise ValueError("Pipe inlet is not subsonic")
    return result


def _pressure_gradient(
    mixture: Any,
    ideal_gas: Any,
    z: list[float],
    mass: float,
    h0: float,
    mass_flux: float,
    friction: float,
    diameter: float,
    pressure0: float,
    state: _Gas,
) -> float:
    pressure = state.pressure
    step = max(20.0, pressure * 2.0e-5)
    lower = _energy_state(mixture, ideal_gas, z, mass, h0, mass_flux, pressure - step)
    upper = _energy_state(
        mixture,
        ideal_gas,
        z,
        mass,
        h0,
        mass_flux,
        min(pressure + step, pressure0 * (1.0 - 1.0e-12)),
    )
    velocity_derivative = (upper.velocity - lower.velocity) / (upper.pressure - lower.pressure)
    denominator = 1.0 + mass_flux * velocity_derivative
    if not math.isfinite(denominator) or denominator <= 2.0e-4:
        raise ValueError("Steady momentum equation reached its sonic singularity")
    return -friction * mass_flux * mass_flux / (2.0 * diameter * state.density * denominator)


def _trace(
    mixture: Any,
    ideal_gas: Any,
    z: list[float],
    mass: float,
    h0: float,
    s0: float,
    mass_flux: float,
    step_length: float,
    friction: float,
    diameter: float,
    pressure0: float,
) -> _Trace:
    try:
        upstream = _gas_state(mixture, ideal_gas, z, mass, pressure0, 300.0, 0.0)
        inlet = _inlet_state(mixture, ideal_gas, z, mass, h0, s0, mass_flux, pressure0, upstream)
    except Exception as error:
        return _Trace(False, None, None, str(error))
    state = inlet
    for _ in range(AXIAL_STEPS):
        if state.mach >= SONIC_LIMIT:
            return _Trace(False, inlet, state, "sonic limit reached before the pipe exit")
        try:
            gradient = _pressure_gradient(
                mixture,
                ideal_gas,
                z,
                mass,
                h0,
                mass_flux,
                friction,
                diameter,
                pressure0,
                state,
            )
        except Exception as error:
            return _Trace(False, inlet, state, str(error))
        predicted_pressure = state.pressure + gradient * step_length
        if not predicted_pressure > 0.0 or predicted_pressure >= state.pressure:
            return _Trace(False, inlet, state, "pressure step did not decrease pressure")
        try:
            predicted = _energy_state(
                mixture, ideal_gas, z, mass, h0, mass_flux, predicted_pressure
            )
        except Exception as error:
            return _Trace(False, inlet, state, str(error))
        if predicted.mach >= 1.0:
            return _Trace(False, inlet, predicted, "predictor crossed the sonic limit")
        try:
            corrected_gradient = 0.5 * (
                gradient
                + _pressure_gradient(
                    mixture,
                    ideal_gas,
                    z,
                    mass,
                    h0,
                    mass_flux,
                    friction,
                    diameter,
                    pressure0,
                    predicted,
                )
            )
        except Exception as error:
            return _Trace(False, inlet, predicted, str(error))
        corrected_pressure = state.pressure + corrected_gradient * step_length
        if not corrected_pressure > 0.0 or corrected_pressure >= state.pressure:
            return _Trace(False, inlet, state, "corrected pressure step did not decrease pressure")
        try:
            state = _energy_state(mixture, ideal_gas, z, mass, h0, mass_flux, corrected_pressure)
        except Exception as error:
            return _Trace(False, inlet, state, str(error))
    completed = state.mach < 1.0
    return _Trace(completed, inlet, state, "" if completed else "exit is sonic")


def _sound(
    mixture: Any,
    ideal_gas: Any,
    z: list[float],
    pressure: float,
    temperature: float,
) -> float:
    reduced = reduced_parameters(mixture, temperature, pressure)
    kij = mixture.effective_kij(reduced.t_kelvin)
    root = phase_state(reduced, kij, z, liquid=False).z
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, temperature, pressure, z, root)
    mass = molar_mass_of(mixture, z)
    volume = root * MOLAR_GAS_CONSTANT * temperature / pressure
    gamma = capacity.cp / capacity.cv
    return volume * math.sqrt(-gamma * capacity.d_p_d_v / mass)


def _replace(state: _Gas, **changes: float) -> _Gas:
    values = {
        "pressure": state.pressure,
        "temperature": state.temperature,
        "density": state.density,
        "molar_enthalpy": state.molar_enthalpy,
        "molar_entropy": state.molar_entropy,
        "velocity": state.velocity,
        "mach": state.mach,
    }
    values.update(changes)
    return _Gas(**values)


def _solid_gate(mixture: Any, names: Any, temperature: float, z: list[float]) -> str | None:
    for index, (component, fraction) in enumerate(zip(mixture.components, z, strict=True)):
        if fraction <= 1.0e-12:
            continue
        triple_value = getattr(component, "triple_point_temperature", None)
        if triple_value is not None and hasattr(triple_value, "to"):
            triple = triple_value.to("K").magnitude
        else:
            triple = triple_value
        if triple is not None and 0.0 < triple < 1000.0 and temperature <= triple:
            return (
                f"component {index} is at or below its triple point, and this port does "
                "not reproduce the per-component solid flash"
            )
    water = 0.0
    for name, fraction in zip(names, z, strict=True):
        if name == "water":
            water = fraction
    if water > 1.0e-12:
        for name, fraction in zip(names, z, strict=True):
            if name in HYDRATE_FORMERS and fraction > 1.0e-12:
                return "a water-bearing hydrate-forming state is not reproduced by this port"
    return None


def _component_mass_fractions(mixture: Any, z: list[float]) -> tuple[float, ...]:
    masses: list[float] = []
    total = 0.0
    for index, (component, fraction) in enumerate(zip(mixture.components, z, strict=True)):
        molar_mass = component.molar_mass
        if molar_mass is None:
            raise InvalidInputError("molar_mass", f"component {index} carries no molar mass")
        mass = fraction * molar_mass.to("kg/mol").magnitude
        total += mass
        masses.append(mass)
    if not total > 0.0 or not math.isfinite(total):
        raise InvalidInputError("upstream_z", f"the mixture's total mass is {total} kg/mol")
    return tuple(mass / total for mass in masses)


def _row(state: _Gas, station: str) -> RealGasFannoReleaseStation:
    density = from_si(state.density, "kg/m**3")
    return RealGasFannoReleaseStation(
        station=station,
        pressure=from_si(state.pressure, "Pa"),
        temperature=from_si(state.temperature, "K"),
        density=density,
        molar_enthalpy=from_si(state.molar_enthalpy, "J/mol"),
        molar_entropy=from_si(state.molar_entropy, "J/(mol*K)"),
        velocity=from_si(state.velocity, "m/s"),
        mass_flux=from_si(state.density * state.velocity, "kg/(m**2*s)"),
        phase="all_vapour",
        gas_mass_fraction=1.0,
        gas_density=density,
        liquid_density=None,
        gas_velocity=None,
        liquid_velocity=None,
    )


def _success(
    mass_flow: float,
    choked: bool,
    sound: float | None,
    friction_length: float | None,
    machs: tuple[float, float] | None,
    z: list[float],
    mass_fractions: tuple[float, ...],
    stations: list[RealGasFannoReleaseStation],
    diagnostics: list[tuple[str, str]],
    warnings: list[Warning],
) -> RealGasFannoReleaseResult:
    return RealGasFannoReleaseResult(
        status="valid",
        mass_flow_rate=from_si(mass_flow, "kg/s"),
        choked=choked,
        throat_sound_speed=None if sound is None else from_si(sound, "m/s"),
        component_mole_fractions=tuple(z),
        component_mass_fractions=mass_fractions,
        friction_length=friction_length,
        inlet_mach=None if machs is None else machs[0],
        exit_mach=None if machs is None else machs[1],
        stations=tuple(stations),
        diagnostics=tuple(
            HemReleaseDiagnostic(code=code, message=message) for code, message in diagnostics
        ),
        diagnostic_codes=tuple(code for code, _ in diagnostics),
        warnings=tuple(warnings),
    )


def _failure(
    status: str, code: str, message: str, warnings: list[Warning]
) -> RealGasFannoReleaseResult:
    return RealGasFannoReleaseResult(
        status=status,
        mass_flow_rate=None,
        choked=False,
        throat_sound_speed=None,
        component_mole_fractions=(),
        component_mass_fractions=(),
        friction_length=None,
        inlet_mach=None,
        exit_mach=None,
        stations=(),
        diagnostics=(HemReleaseDiagnostic(code=code, message=message),),
        diagnostic_codes=(code,),
        warnings=tuple(warnings),
    )
