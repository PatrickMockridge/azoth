"""``process.ranz_marshall_release`` - NeqSim's Ranz-Marshall release adapter.

Spec: ``specs/models/process/ranz_marshall_release.toml``

The Python twin of ``crates/azoth-process/src/models/ranz_marshall_release.rs``. The
external-film correlation supplies one relaxation time per component and the
component-selective model does the rest.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    HemReleaseDiagnostic,
    RanzMarshallReleaseResult,
    RanzMarshallReleaseStation,
)
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.process.reference.component_selective_release import (
    component_selective_release,
)

MODEL_ID = "process.ranz_marshall_release"

#: The correlation's Reynolds bound, exclusive.
MAX_REYNOLDS = 200.0
#: The correlation's Schmidt bound, exclusive.
MAX_SCHMIDT = 250.0


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def ranz_marshall_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    surface_tension: Q,
    residence_time: Q,
    morphology: str,
    sauter_mean_diameter: Q,
    continuous_phase_density: Q,
    continuous_phase_dynamic_viscosity: Q,
    relative_velocity: Q,
    component_diffusivities: list[Q],
    parameter_provenance: str,
) -> RanzMarshallReleaseResult:
    """The release rate of a Ranz-Marshall short-opening orifice.

    Raises:
        InvalidInputError: for a non-positive or empty input, a composition that is not one
            entry per component, or a mixture the databank cannot resolve.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    t = input_to_si(spec, "upstream_t", upstream_t)
    p = input_to_si(spec, "upstream_p", upstream_p)
    diameter_m = input_to_si(spec, "diameter", diameter)
    back = input_to_si(spec, "back_pressure", back_pressure)
    sigma = input_to_si(spec, "surface_tension", surface_tension)
    residence = input_to_si(spec, "residence_time", residence_time)
    d32 = input_to_si(spec, "sauter_mean_diameter", sauter_mean_diameter)
    rho_c = input_to_si(spec, "continuous_phase_density", continuous_phase_density)
    mu_c = input_to_si(
        spec, "continuous_phase_dynamic_viscosity", continuous_phase_dynamic_viscosity
    )
    u_rel = input_to_si(spec, "relative_velocity", relative_velocity)
    diffusivities = [
        input_to_si(spec, "component_diffusivities", value) for value in component_diffusivities
    ]
    apply_checks(
        checks.on_input,
        {
            "upstream_t": t,
            "upstream_p": p,
            "diameter": diameter_m,
            "discharge_coefficient": discharge_coefficient,
            "back_pressure": back,
            "surface_tension": sigma,
            "residence_time": residence,
            "sauter_mean_diameter": d32,
            "continuous_phase_density": rho_c,
            "continuous_phase_dynamic_viscosity": mu_c,
            "relative_velocity": u_rel,
        }.get,
        warnings,
    )
    if not math.isfinite(sigma) or sigma <= 0.0:
        raise InvalidInputError(
            "surface_tension",
            f"the surface tension must be finite and positive, and {sigma} is not",
        )
    if not math.isfinite(residence) or residence <= 0.0:
        raise InvalidInputError(
            "residence_time",
            f"the residence time must be finite and positive, and {residence} is not",
        )
    if not math.isfinite(d32) or d32 <= 0.0:
        raise InvalidInputError(
            "sauter_mean_diameter",
            "the Sauter mean diameter must be finite and positive",
        )
    if not math.isfinite(rho_c) or rho_c <= 0.0:
        raise InvalidInputError(
            "continuous_phase_density",
            "the continuous-phase density must be finite and positive",
        )
    if not math.isfinite(mu_c) or mu_c <= 0.0:
        raise InvalidInputError(
            "continuous_phase_dynamic_viscosity",
            "the continuous-phase viscosity must be finite and positive",
        )
    if not math.isfinite(u_rel) or u_rel < 0.0:
        raise InvalidInputError(
            "relative_velocity", "the relative velocity must be finite and non-negative"
        )
    if not parameter_provenance.strip():
        raise InvalidInputError(
            "parameter_provenance", "the parameter provenance must not be empty"
        )
    if len(diffusivities) != len(components):
        return _failure(
            "invalid",
            "RANZ_MARSHALL_COMPONENT_BASIS_INVALID",
            "Diffusivity components must exactly match the release-fluid components; "
            f"fluid has {len(components)} components and the diffusivities "
            f"{len(diffusivities)}",
            warnings,
        )
    if any(not math.isfinite(value) or value <= 0.0 for value in diffusivities):
        raise InvalidInputError(
            "component_diffusivities",
            "every component diffusivity must be finite and positive",
        )

    reynolds = rho_c * u_rel * d32 / mu_c
    if not math.isfinite(reynolds) or reynolds < 0.0 or reynolds >= MAX_REYNOLDS:
        return _failure(
            "unsupported",
            "RANZ_MARSHALL_RANGE_UNSUPPORTED",
            f"Reynolds number {reynolds:.6f} is outside 0 <= Re < 200",
            warnings,
        )
    times = []
    for index, diffusivity in enumerate(diffusivities):
        schmidt = mu_c / (rho_c * diffusivity)
        if not math.isfinite(schmidt) or schmidt <= 0.0 or schmidt >= MAX_SCHMIDT:
            return _failure(
                "unsupported",
                "RANZ_MARSHALL_RANGE_UNSUPPORTED",
                f"Schmidt number for component {index} is {schmidt:.6f} and outside 0 < Sc < 250",
                warnings,
            )
        sherwood = 2.0 + 0.6 * math.sqrt(reynolds) * schmidt ** (1.0 / 3.0)
        coefficient = sherwood * diffusivity / d32
        times.append(d32 / (6.0 * coefficient))
    if morphology == "liquid_droplets":
        return _failure(
            "unsupported",
            "LIQUID_DROPLET_HYDRODYNAMICS_UNSUPPORTED",
            "Ranz-Marshall droplet coefficients are resolved, but this release adapter only "
            "has a gas-bubble-in-liquid drift-flux closure",
            warnings,
        )
    if morphology != "gas_bubbles":
        raise InvalidInputError("morphology", f"unknown morphology {morphology!r}")

    delegate = component_selective_release(
        components,
        list(upstream_z),
        upstream_t,
        upstream_p,
        diameter,
        discharge_coefficient,
        back_pressure,
        surface_tension,
        [from_si(value, "s") for value in times],
        residence_time,
        parameter_provenance,
    )
    if delegate.status != "valid":
        code = delegate.diagnostic_codes[0]
        message = delegate.diagnostics[0].message
        return _failure(
            delegate.status,
            f"RANZ_MARSHALL_{code}",
            message,
            list(delegate.warnings),
        )
    diagnostics = [
        (
            "MODEL_ASSUMPTIONS",
            "Ranz-Marshall external-film component transfer for explicit spherical gas "
            "bubbles; equilibrium caloric properties and bounded vertical drift flux; no "
            "interfacial heat or latent kinetics, internal resistance, Stefan-flow "
            "correction, breakup, coalescence or solids",
        ),
        (
            "RANZ_MARSHALL_COMPONENT_TRANSFER",
            "External-film coefficients from the Reynolds and Schmidt numbers; no predicted "
            "residence time",
        ),
    ]
    diagnostics.extend(
        (row.code, row.message)
        for row in delegate.diagnostics
        if row.code not in ("MODEL_ASSUMPTIONS", "COMPONENT_SELECTIVE_PHASE_TRANSFER")
    )
    return RanzMarshallReleaseResult(
        status="valid",
        mass_flow_rate=delegate.mass_flow_rate,
        choked=delegate.choked,
        throat_sound_speed=None,
        component_mole_fractions=delegate.component_mole_fractions,
        component_mass_fractions=delegate.component_mass_fractions,
        stations=tuple(_row(station) for station in delegate.stations),
        diagnostics=tuple(
            HemReleaseDiagnostic(code=code, message=message) for code, message in diagnostics
        ),
        diagnostic_codes=tuple(code for code, _ in diagnostics),
        warnings=tuple(delegate.warnings),
    )


def _row(station: object) -> RanzMarshallReleaseStation:
    """One delegate station as this model's row."""
    return RanzMarshallReleaseStation(
        station=station.station,
        pressure=station.pressure,
        temperature=station.temperature,
        density=station.density,
        molar_enthalpy=station.molar_enthalpy,
        molar_entropy=station.molar_entropy,
        velocity=station.velocity,
        mass_flux=station.mass_flux,
        phase=station.phase,
        gas_mass_fraction=station.gas_mass_fraction,
        gas_density=station.gas_density,
        liquid_density=station.liquid_density,
        gas_velocity=station.gas_velocity,
        liquid_velocity=station.liquid_velocity,
    )


def _failure(
    status: str, code: str, message: str, warnings: list[Warning]
) -> RanzMarshallReleaseResult:
    return RanzMarshallReleaseResult(
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
