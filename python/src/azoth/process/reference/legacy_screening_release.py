"""``process.legacy_screening_release`` - NeqSim's historical orifice screening release.

Spec: ``specs/models/process/legacy_screening_release.toml``

The Python twin of ``crates/azoth-process/src/models/legacy_screening_release.rs``, written
to mirror it. **Compatibility surface, not a physical model**: the historical screening
equation, its gamma default and the aliased stations are reproduced as the class has them.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    HemReleaseDiagnostic,
    LegacyScreeningReleaseResult,
    LegacyScreeningReleaseStation,
)
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import (
    EntropyState,
    Evaluations,
    flashed_state,
    molar_mass_of,
)
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import phase_state, reduced_parameters
from azoth.eos.reference.pt_flash import check_composition

MODEL_ID = "process.legacy_screening_release"

#: The class's historical gas constant, J/(kmol K).
R = 8314.0
#: The fallback density's gas constant, J/(mol K), as the class writes it.
R_MOLAR = 8.314
#: The class's historical default heat-capacity ratio.
DEFAULT_GAMMA = 1.3


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


def legacy_screening_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
) -> LegacyScreeningReleaseResult:
    """The historical screening release rate through a circular opening.

    Args:
        components: the inventory's substances, by name.
        upstream_z: the upstream composition, in ``components``' order.
        upstream_t: the upstream temperature.
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
            "SCREENING_ONLY",
            "Compatibility result from the historical screening equation; not a qualified "
            "physical release model",
        ),
        (
            "UNRESOLVED_STATIONS",
            "Legacy screening does not solve throat or exit thermodynamics; the opening "
            "states alias upstream",
        ),
        (
            "LEGACY_FALLBACK_POLICY",
            "The delegated method retains historical density, molar-mass and gamma defaults "
            "when properties fail",
        ),
    ]

    try:
        upstream = flashed_state(mixture, ideal_gas, list(upstream_z), mass, p, t, Evaluations())
    except Exception as error:
        return _failure("invalid", "LEGACY_SCREENING_ADAPTER_FAILED", str(error), warnings)

    gamma = DEFAULT_GAMMA
    gamma_defaulted = False
    try:
        reduced = reduced_parameters(mixture, t, p)
        kij = mixture.effective_kij(reduced.t_kelvin)
        root = phase_state(reduced, kij, list(upstream_z), liquid=upstream.phase == "all_liquid").z
        capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, list(upstream_z), root)
        candidate = capacity.cp / capacity.cv
        if math.isfinite(candidate) and 1.0 < candidate <= 2.0:
            gamma = candidate
        else:
            gamma_defaulted = True
    except Exception:
        gamma_defaulted = True

    # **The screening path's density is the ideal-gas fallback.** The class flashes a
    # clone without initialising its physical properties, so its density read is NaN and
    # its own fallback ``P MW/(8.314 T)`` supplies the value. The station keeps the EOS
    # density.
    rate_density = p * mass / (R_MOLAR * t)
    area = math.pi * diameter_m * diameter_m / 4.0
    if p <= back:
        rate = 0.0
    else:
        rate = _screening_rate(
            p, t, rate_density, gamma, 1000.0 * mass, discharge_coefficient, area, back
        )
    if not math.isfinite(rate) or rate < 0.0:
        return _failure(
            "invalid",
            "LEGACY_SCREENING_ADAPTER_FAILED",
            f"Legacy method returned an invalid release rate: {rate} kg/s",
            warnings,
        )
    effective_area = discharge_coefficient * math.pi * diameter_m * diameter_m / 4.0
    velocity = 0.0 if rate <= 0.0 else rate / (upstream.density * effective_area)

    if gamma_defaulted:
        diagnostics.append(("LEGACY_GAMMA_DEFAULT", "Historical gamma default 1.3 was used"))
    critical = (2.0 / (gamma + 1.0)) ** (gamma / (gamma - 1.0))
    choked = back / p <= critical
    if rate == 0.0:
        choked = False
        diagnostics.append(
            (
                "NO_FORWARD_FLOW_OR_SCREENING_ZERO",
                "Legacy screening returned zero; inspect upstream pressure and retained "
                "fallback behavior",
            )
        )

    stations = (
        _row("upstream_stagnation", upstream, 0.0),
        _row("throat_critical", upstream, velocity),
        _row("orifice_exit", upstream, velocity),
    )
    return LegacyScreeningReleaseResult(
        status="valid_with_warnings",
        mass_flow_rate=from_si(rate, "kg/s"),
        choked=choked,
        throat_sound_speed=None,
        component_mole_fractions=tuple(upstream_z),
        component_mass_fractions=mass_fractions,
        stations=stations,
        diagnostics=tuple(
            HemReleaseDiagnostic(code=code, message=message) for code, message in diagnostics
        ),
        diagnostic_codes=tuple(code for code, _ in diagnostics),
        warnings=tuple(warnings),
    )


def _screening_rate(
    pressure: float,
    temperature: float,
    density: float,
    gamma: float,
    molar_mass: float,
    coefficient: float,
    area: float,
    back: float,
) -> float:
    """The historical screening rate, kg/s."""
    critical = (2.0 / (gamma + 1.0)) ** (gamma / (gamma - 1.0))
    ratio = back / pressure
    if ratio <= critical:
        factor = (2.0 / (gamma + 1.0)) ** ((gamma + 1.0) / (2.0 * (gamma - 1.0)))
        return float(
            coefficient
            * area
            * pressure
            * math.sqrt(gamma * molar_mass / (R * temperature))
            * factor
        )
    return float(coefficient * area * math.sqrt(max(0.0, 2.0 * density * (pressure - back))))


def _row(station: str, state: EntropyState, velocity: float) -> LegacyScreeningReleaseStation:
    """One published station row."""
    density = from_si(state.density, "kg/m**3")
    return LegacyScreeningReleaseStation(
        station=station,
        pressure=from_si(state.pressure, "Pa"),
        temperature=from_si(state.temperature, "K"),
        density=density,
        molar_enthalpy=from_si(state.molar_enthalpy, "J/mol"),
        molar_entropy=from_si(state.molar_entropy, "J/(mol*K)"),
        velocity=from_si(velocity, "m/s"),
        mass_flux=from_si(state.density * velocity, "kg/(m**2*s)"),
        phase="all_vapour" if state.phase == "trivial" else state.phase,
        gas_mass_fraction=state.gas_mass_fraction,
        gas_density=(
            None if state.gas_phase is None else from_si(state.gas_phase.density, "kg/m**3")
        ),
        liquid_density=(
            None if state.liquid_phase is None else from_si(state.liquid_phase.density, "kg/m**3")
        ),
        gas_velocity=None,
        liquid_velocity=None,
    )


def _failure(
    status: str, code: str, message: str, warnings: list[Warning]
) -> LegacyScreeningReleaseResult:
    return LegacyScreeningReleaseResult(
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
