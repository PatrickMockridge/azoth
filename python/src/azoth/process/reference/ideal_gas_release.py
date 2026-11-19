"""``process.ideal_gas_release`` - NeqSim's ideal-gas isentropic orifice release.

Spec: ``specs/models/process/ideal_gas_release.toml``

The Python twin of ``crates/azoth-process/src/models/ideal_gas_release.rs``, written to
mirror it. One gas phase, a constant heat-capacity ratio, and an isentropic expansion from
the stagnation state to the critical ratio or the receiving pressure. The upstream density
is ideal-gas while the stagnation enthalpy and entropy are the EOS flash's.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    HemReleaseDiagnostic,
    IdealGasReleaseResult,
    IdealGasReleaseStation,
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

MODEL_ID = "process.ideal_gas_release"


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _component_mass_fractions(mixture: Any, z: list[float]) -> tuple[float, ...]:
    """The upstream overall mass fractions."""
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


def ideal_gas_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
) -> IdealGasReleaseResult:
    """The release rate of an ideal-gas isentropic short-opening orifice.

    Args:
        components: the gas's substances, by name.
        upstream_z: the upstream stagnation composition, in ``components``' order.
        upstream_t: the upstream stagnation temperature.
        upstream_p: the upstream stagnation pressure.
        diameter: the circular opening's diameter.
        discharge_coefficient: the effective-area factor in ``(0, 1]``.
        back_pressure: the receiving absolute pressure.

    Raises:
        InvalidInputError: for a non-positive state or opening, a coefficient outside
            ``(0, 1]``, or a composition that is not one entry per component and sums to one.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    t = input_to_si(spec, "upstream_t", upstream_t)
    p = input_to_si(spec, "upstream_p", upstream_p)
    diameter_m = input_to_si(spec, "diameter", diameter)
    back = input_to_si(spec, "back_pressure", back_pressure)
    apply_checks(
        checks.on_input,
        {
            "upstream_t": t,
            "upstream_p": p,
            "diameter": diameter_m,
            "discharge_coefficient": discharge_coefficient,
            "back_pressure": back,
        }.get,
        warnings,
    )
    if not 0.0 < discharge_coefficient <= 1.0:
        raise InvalidInputError(
            "discharge_coefficient",
            f"the effective-area factor must be in (0, 1], and {discharge_coefficient} is not",
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
    mass_fractions = _component_mass_fractions(mixture, list(upstream_z))

    diagnostics = [
        (
            "IDEAL_GAS_ASSUMPTIONS",
            "Calorically perfect gas, constant gamma, isentropic acceleration; no "
            "real-gas departure or phase change",
        ),
        (
            "NO_PROPERTY_FALLBACK",
            "Mixture molar mass and gamma are required properties; no default is substituted",
        ),
        ("EXIT_ALIASES_THROAT", "Zero-length opening: exit equals accepted throat"),
    ]

    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), list(upstream_z))
    if flash.phase != "all_vapour":
        return _failure(
            "unsupported",
            "UNSUPPORTED_IDEAL_GAS_REGIME",
            "The ideal-gas adapter requires one gas phase",
            warnings,
        )

    reduced = reduced_parameters(mixture, t, p)
    kij = mixture.effective_kij(reduced.t_kelvin)
    root = phase_state(reduced, kij, list(upstream_z), liquid=False).z
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, list(upstream_z), root)
    gamma = capacity.cp / capacity.cv
    if not math.isfinite(gamma) or not 1.0 < gamma <= 2.0:
        return _failure(
            "invalid",
            "IDEAL_GAS_CALCULATION_FAILED",
            "Heat-capacity ratio must be in (1, 2]",
            warnings,
        )
    caloric = molar_enthalpy_entropy(
        mixture, ideal_gas, quantity(t, "K"), quantity(p, "Pa"), list(upstream_z), root
    )
    enthalpy0 = caloric.h.to("J/mol").magnitude / mass
    entropy0 = caloric.s.to("J/(mol*K)").magnitude / mass
    density0 = p * mass / (MOLAR_GAS_CONSTANT * t)

    stagnation = (p, t, density0, enthalpy0, entropy0, 0.0)
    if p <= back:
        diagnostics.append(
            ("NO_FORWARD_FLOW", "Upstream pressure is at or below receiving pressure")
        )
        stations = tuple(
            _row(name, stagnation, mass)
            for name in (
                "upstream_stagnation",
                "throat_critical",
                "orifice_exit",
            )
        )
        return _success(
            "valid", 0.0, False, None, upstream_z, mass_fractions, stations, diagnostics, warnings
        )

    critical_ratio = (2.0 / (gamma + 1.0)) ** (gamma / (gamma - 1.0))
    back_ratio = back / p
    choked = back_ratio <= critical_ratio
    throat_pressure = p * critical_ratio if choked else back
    throat = _expanded(t, p, enthalpy0, entropy0, mass, gamma, throat_pressure)
    ambient = _expanded(t, p, enthalpy0, entropy0, mass, gamma, back) if choked else throat
    stations = (
        _row("upstream_stagnation", stagnation, mass),
        _row("throat_critical", throat, mass),
        _row("orifice_exit", throat, mass),
        _row("ambient_expanded", ambient, mass),
    )
    area = math.pi * diameter_m * diameter_m / 4.0
    mass_flow = discharge_coefficient * area * throat[2] * throat[5]
    sound_speed = math.sqrt(gamma * MOLAR_GAS_CONSTANT * throat[1] / mass)
    return _success(
        "valid",
        mass_flow,
        choked,
        sound_speed,
        upstream_z,
        mass_fractions,
        stations,
        diagnostics,
        warnings,
    )


def _expanded(
    t0: float,
    p0: float,
    enthalpy0: float,
    entropy0: float,
    molar_mass: float,
    gamma: float,
    pressure: float,
) -> tuple[float, float, float, float, float]:
    """One isentropic expansion from the stagnation state.

    Returns ``(pressure, temperature, density, specific enthalpy, specific entropy,
    velocity)``.
    """
    ratio = pressure / p0
    temperature = t0 * ratio ** ((gamma - 1.0) / gamma)
    specific_gas_constant = MOLAR_GAS_CONSTANT / molar_mass
    heat_capacity = gamma * specific_gas_constant / (gamma - 1.0)
    velocity = math.sqrt(max(0.0, 2.0 * heat_capacity * (t0 - temperature)))
    density = pressure * molar_mass / (MOLAR_GAS_CONSTANT * temperature)
    enthalpy = enthalpy0 - 0.5 * velocity * velocity
    return (pressure, temperature, density, enthalpy, entropy0, velocity)


def _row(
    station: str,
    state: tuple[float, float, float, float, float, float],
    molar_mass: float,
) -> IdealGasReleaseStation:
    """One published station row from a ``(P, T, rho, h, s, u)`` state."""
    pressure, temperature, density, enthalpy, entropy, velocity = state
    gas_density = from_si(density, "kg/m**3")
    return IdealGasReleaseStation(
        station=station,
        pressure=from_si(pressure, "Pa"),
        temperature=from_si(temperature, "K"),
        density=gas_density,
        molar_enthalpy=from_si(enthalpy * molar_mass, "J/mol"),
        molar_entropy=from_si(entropy * molar_mass, "J/(mol*K)"),
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
    status: str,
    mass_flow: float | None,
    choked: bool,
    sound_speed: float | None,
    z: list[float],
    mass_fractions: tuple[float, ...],
    stations: tuple[IdealGasReleaseStation, ...],
    diagnostics: list[tuple[str, str]],
    warnings: list[Warning],
) -> IdealGasReleaseResult:
    return IdealGasReleaseResult(
        status=status,
        mass_flow_rate=None if mass_flow is None else from_si(mass_flow, "kg/s"),
        choked=choked,
        throat_sound_speed=None if sound_speed is None else from_si(sound_speed, "m/s"),
        component_mole_fractions=tuple(z),
        component_mass_fractions=mass_fractions,
        stations=stations,
        diagnostics=tuple(
            HemReleaseDiagnostic(code=code, message=message) for code, message in diagnostics
        ),
        diagnostic_codes=tuple(code for code, _ in diagnostics),
        warnings=tuple(warnings),
    )


def _failure(
    status: str,
    code: str,
    message: str,
    warnings: list[Warning],
) -> IdealGasReleaseResult:
    return IdealGasReleaseResult(
        status=status,
        mass_flow_rate=None,
        choked=False,
        throat_sound_speed=None,
        component_mole_fractions=(),
        component_mass_fractions=(),
        stations=(),
        diagnostics=(HemReleaseDiagnostic(code=code, message=message),),
        diagnostic_codes=(code,),
        warnings=tuple(warnings),
    )
