"""``process.ideal_gas_fanno_release`` - NeqSim's ideal-gas Fanno pipe release.

Spec: ``specs/models/process/ideal_gas_fanno_release.toml``

The Python twin of ``crates/azoth-process/src/models/ideal_gas_fanno_release.rs``. A
quasi-steady adiabatic constant-area pipe of a calorically perfect gas: the Fanno function
and two 120-iteration bisections locate the exit Mach, and the class's ``1/M``
stagnation-pressure ratio is kept.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    HemReleaseDiagnostic,
    IdealGasFannoReleaseResult,
    IdealGasFannoReleaseStation,
)
from azoth.core.units import Q, from_si, input_to_si, quantity
from azoth.core.warnings import Warning
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import phase_state, reduced_parameters
from azoth.eos.reference.molar_enthalpy_entropy import molar_enthalpy_entropy
from azoth.eos.reference.pr_molar_volume import MOLAR_GAS_CONSTANT
from azoth.eos.reference.pt_flash import check_composition, pt_flash

MODEL_ID = "process.ideal_gas_fanno_release"

#: The smallest solved Mach.
MIN_MACH = 1.0e-10
#: The bisection cap.
BISECTION_ITERATIONS = 120
#: The mass-flux closure.
FLUX_CLOSURE = 1.0e-9
#: The hydrate formers.
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


def ideal_gas_fanno_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    flow_path_length: Q,
    darcy_friction_factor: float,
) -> IdealGasFannoReleaseResult:
    """The release rate of an ideal-gas Fanno pipe.

    Raises:
        InvalidInputError: for a non-positive state or opening, a path that is not both
            positive or both zero, or a composition that is not one entry per component.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    t = input_to_si(spec, "upstream_t", upstream_t)
    p = input_to_si(spec, "upstream_p", upstream_p)
    diameter_m = input_to_si(spec, "diameter", diameter)
    back = input_to_si(spec, "back_pressure", back_pressure)
    length = input_to_si(spec, "flow_path_length", flow_path_length)
    friction = float(darcy_friction_factor)
    apply_checks(
        checks.on_input,
        {
            "upstream_t": t,
            "upstream_p": p,
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
            "UNSUPPORTED_FANNO_REGIME",
            "Fanno model requires positive pipe length and Darcy friction",
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

    diagnostics = [
        (
            "FANNO_MODEL_ASSUMPTIONS",
            "Steady adiabatic constant-area calorically perfect gas with specified constant "
            "Darcy friction",
        ),
        (
            "QUASI_STEADY_ONE_SIDED_PIPE",
            "One pipe segment from stagnation boundary to release plane; transient waves and "
            "the opposite rupture side are excluded",
        ),
    ]
    names = mixture.names if mixture.names is not None else tuple(components)
    gate = _solid_gate(mixture, names, t, list(upstream_z))
    if gate is not None:
        return _failure("invalid", "SOLID_RISK_ASSESSMENT_FAILED", gate, warnings)
    diagnostics.append(
        (
            "SOLID_RISK_ASSESSED",
            "UPSTREAM_STAGNATION: Mixture-specific solid and hydrate applicability checks passed",
        )
    )

    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), list(upstream_z))
    if flash.phase != "all_vapour":
        return _failure(
            "unsupported",
            "UNSUPPORTED_FANNO_REGIME",
            "The Fanno pipe model requires one gas phase",
            warnings,
        )
    reduced = reduced_parameters(mixture, t, p)
    kij = mixture.effective_kij(reduced.t_kelvin)
    root = phase_state(reduced, kij, list(upstream_z), liquid=False).z
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, list(upstream_z), root)
    gamma = capacity.cp / capacity.cv
    if not math.isfinite(gamma) or not 1.0 < gamma <= 2.0:
        return _failure(
            "invalid", "FANNO_CALCULATION_FAILED", "Heat-capacity ratio must be in (1, 2]", warnings
        )
    caloric = molar_enthalpy_entropy(
        mixture, ideal_gas, quantity(t, "K"), quantity(p, "Pa"), list(upstream_z), root
    )
    enthalpy0 = caloric.h.to("J/mol").magnitude
    entropy0 = caloric.s.to("J/(mol*K)").magnitude
    specific_gas_constant = MOLAR_GAS_CONSTANT / mass
    density0 = p / (specific_gas_constant * t)
    stagnation = (p, t, density0, enthalpy0, entropy0, 0.0)
    if p <= back:
        diagnostics.append(
            ("NO_FORWARD_FLOW", "Upstream pressure is at or below receiving pressure")
        )
        return _success(
            0.0,
            False,
            None,
            None,
            None,
            upstream_z,
            _component_mass_fractions(mixture, list(upstream_z)),
            [
                _row("upstream_stagnation", stagnation),
                _row("throat_critical", stagnation),
                _row("orifice_exit", stagnation),
            ],
            diagnostics,
            warnings,
        )

    friction_length = friction * length / diameter_m
    inlet_at_choke = _inlet_mach(1.0, friction_length, gamma)
    critical = _state(p, t, mass, gamma, enthalpy0, entropy0, inlet_at_choke, 1.0)
    choked = back <= critical[0]
    exit_mach = (
        1.0
        if choked
        else _unchoked_exit_mach(back, p, t, mass, gamma, enthalpy0, entropy0, friction_length)
    )
    inlet = _inlet_mach(exit_mach, friction_length, gamma)
    exit = _state(p, t, mass, gamma, enthalpy0, entropy0, inlet, exit_mach)
    inlet_flux = _mass_flux(p, t, gamma, specific_gas_constant, inlet)
    relative = abs(exit[2] * exit[5] - inlet_flux) / max(1.0, abs(inlet_flux))
    if relative > FLUX_CLOSURE:
        return _failure(
            "invalid",
            "FANNO_CALCULATION_FAILED",
            f"Fanno mass-flux closure failed: {relative}",
            warnings,
        )
    exit_tuple = (exit[0], exit[1], exit[2], exit[3], exit[4], exit[5])
    ambient = (
        _expanded(exit[6], t, mass, gamma, enthalpy0, entropy0, back) if choked else exit_tuple
    )
    area = math.pi * diameter_m * diameter_m / 4.0
    rate = discharge_coefficient * area * exit_tuple[2] * exit_tuple[5]
    sound = math.sqrt(gamma * specific_gas_constant * exit_tuple[1])
    diagnostics.append(
        (
            "FANNO_SOLUTION",
            "Darcy friction length solved; the exit Mach is reported on the result",
        )
    )
    return _success(
        rate,
        choked,
        sound,
        friction_length,
        (inlet, exit_mach),
        upstream_z,
        _component_mass_fractions(mixture, list(upstream_z)),
        [
            _row("upstream_stagnation", stagnation),
            _row("throat_critical", exit_tuple),
            _row("orifice_exit", exit_tuple),
            _row("ambient_expanded", ambient),
        ],
        diagnostics,
        warnings,
    )


def _fanno_function(mach: float, gamma: float) -> float:
    mach2 = mach * mach
    return (1.0 - mach2) / (gamma * mach2) + (gamma + 1.0) / (2.0 * gamma) * math.log(
        (gamma + 1.0) * mach2 / (2.0 + (gamma - 1.0) * mach2)
    )


def _state(
    pressure0: float,
    temperature0: float,
    molar_mass: float,
    gamma: float,
    enthalpy0: float,
    entropy0: float,
    inlet_mach: float,
    exit_mach: float,
) -> tuple[float, float, float, float, float, float]:
    ratio_exit = (1.0 / exit_mach) * (
        (2.0 + (gamma - 1.0) * exit_mach * exit_mach) / (gamma + 1.0)
    ) ** ((gamma + 1.0) / (2.0 * (gamma - 1.0)))
    ratio_inlet = (1.0 / inlet_mach) * (
        (2.0 + (gamma - 1.0) * inlet_mach * inlet_mach) / (gamma + 1.0)
    ) ** ((gamma + 1.0) / (2.0 * (gamma - 1.0)))
    pressure0_exit = pressure0 * ratio_exit / ratio_inlet
    stagnation_factor = 1.0 + 0.5 * (gamma - 1.0) * exit_mach * exit_mach
    pressure = pressure0_exit / stagnation_factor ** (gamma / (gamma - 1.0))
    temperature = temperature0 / stagnation_factor
    specific_gas_constant = MOLAR_GAS_CONSTANT / molar_mass
    sound = math.sqrt(gamma * specific_gas_constant * temperature)
    velocity = exit_mach * sound
    density = pressure / (specific_gas_constant * temperature)
    heat_capacity = gamma * specific_gas_constant / (gamma - 1.0)
    entropy = (
        entropy0
        + heat_capacity * math.log(temperature / temperature0)
        - specific_gas_constant * math.log(pressure / pressure0)
    )
    return (
        pressure,
        temperature,
        density,
        enthalpy0 - 0.5 * velocity * velocity * molar_mass,
        entropy * molar_mass,
        velocity,
        pressure0_exit,
    )


def _inlet_mach(exit_mach: float, friction_length: float, gamma: float) -> float:
    target = _fanno_function(exit_mach, gamma) + friction_length
    if not math.isfinite(target) or target >= _fanno_function(MIN_MACH, gamma):
        raise InvalidInputError(
            "flow_path_length",
            "Fanno friction length exceeds the bounded Mach solver range",
        )
    low = MIN_MACH
    high = exit_mach
    for _ in range(BISECTION_ITERATIONS):
        middle = 0.5 * (low + high)
        if _fanno_function(middle, gamma) > target:
            low = middle
        else:
            high = middle
    return 0.5 * (low + high)


def _unchoked_exit_mach(
    back_pressure: float,
    pressure0: float,
    temperature0: float,
    molar_mass: float,
    gamma: float,
    enthalpy0: float,
    entropy0: float,
    friction_length: float,
) -> float:
    low = MIN_MACH
    high = 1.0
    for _ in range(BISECTION_ITERATIONS):
        middle = 0.5 * (low + high)
        inlet = _inlet_mach(middle, friction_length, gamma)
        candidate = _state(
            pressure0, temperature0, molar_mass, gamma, enthalpy0, entropy0, inlet, middle
        )
        if candidate[0] > back_pressure:
            low = middle
        else:
            high = middle
    return 0.5 * (low + high)


def _mass_flux(
    pressure0: float, temperature0: float, gamma: float, gas_constant: float, mach: float
) -> float:
    factor = 1.0 + 0.5 * (gamma - 1.0) * mach * mach
    temperature = temperature0 / factor
    pressure = pressure0 / factor ** (gamma / (gamma - 1.0))
    return (
        pressure
        / (gas_constant * temperature)
        * mach
        * math.sqrt(gamma * gas_constant * temperature)
    )


def _expanded(
    pressure0: float,
    temperature0: float,
    molar_mass: float,
    gamma: float,
    enthalpy0: float,
    entropy0: float,
    pressure: float,
) -> tuple[float, float, float, float, float, float]:
    pressure_ratio = pressure / pressure0
    temperature = temperature0 * pressure_ratio ** ((gamma - 1.0) / gamma)
    specific_gas_constant = MOLAR_GAS_CONSTANT / molar_mass
    heat_capacity = gamma * specific_gas_constant / (gamma - 1.0)
    velocity = math.sqrt(max(0.0, 2.0 * heat_capacity * (temperature0 - temperature)))
    density = pressure / (specific_gas_constant * temperature)
    return (
        pressure,
        temperature,
        density,
        enthalpy0 - 0.5 * velocity * velocity * molar_mass,
        entropy0,
        velocity,
    )


def _solid_gate(
    mixture: object, names: tuple[str, ...] | list[str], temperature: float, z: list[float]
) -> str | None:
    for index, (component, fraction) in enumerate(zip(mixture.components, z, strict=True)):
        if fraction <= 1.0e-12:
            continue
        triple = getattr(component, "triple_point_temperature", None)
        if hasattr(triple, "to"):
            triple = triple.to("K").magnitude
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


def _component_mass_fractions(mixture: object, z: list[float]) -> tuple[float, ...]:
    masses: list[float] = []
    total = 0.0
    for index, (component, fraction) in enumerate(zip(mixture.components, z, strict=True)):
        if component.molar_mass is None:
            raise InvalidInputError("molar_mass", f"component {index} carries no molar mass")
        mass = fraction * component.molar_mass.to("kg/mol").magnitude
        total += mass
        masses.append(mass)
    if not total > 0.0 or not math.isfinite(total):
        raise InvalidInputError("upstream_z", f"the mixture's total mass is {total} kg/mol")
    return tuple(mass / total for mass in masses)


def _row(
    station: str, state: tuple[float, float, float, float, float, float]
) -> IdealGasFannoReleaseStation:
    pressure, temperature, density, enthalpy, entropy, velocity = state
    gas_density = from_si(density, "kg/m**3")
    return IdealGasFannoReleaseStation(
        station=station,
        pressure=from_si(pressure, "Pa"),
        temperature=from_si(temperature, "K"),
        density=gas_density,
        molar_enthalpy=from_si(enthalpy, "J/mol"),
        molar_entropy=from_si(entropy, "J/(mol*K)"),
        velocity=from_si(velocity, "m/s"),
        mass_flux=from_si(density * velocity, "kg/(m**2*s)"),
        phase="all_vapour",
        gas_mass_fraction=1.0,
        gas_density=gas_density,
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
    stations: list[IdealGasFannoReleaseStation],
    diagnostics: list[tuple[str, str]],
    warnings: list[Warning],
) -> IdealGasFannoReleaseResult:
    return IdealGasFannoReleaseResult(
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
) -> IdealGasFannoReleaseResult:
    return IdealGasFannoReleaseResult(
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
