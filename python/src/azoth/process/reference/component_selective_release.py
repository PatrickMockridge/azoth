"""``process.component_selective_release`` - NeqSim's finite-rate drift-flux orifice release.

Spec: ``specs/models/process/component_selective_release.toml``

The Python twin of ``crates/azoth-process/src/models/component_selective_release.rs``. The
homogeneous-equilibrium scan supplies the stations; the opening's phase split relaxes from
the upstream value toward equilibrium by a first-order progress, and the shared drift-flux
closure solves the relaxed station's velocities.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    ComponentSelectiveReleaseResult,
    ComponentSelectiveReleaseStation,
    HemReleaseDiagnostic,
)
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.process.reference._release import (
    ReleaseRefusal,
    drift_flux,
    reconstructed_components,
    relax_components,
)
from azoth.process.reference.hem_release import _Failure, _states, _Station

MODEL_ID = "process.component_selective_release"


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def component_selective_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    surface_tension: Q,
    component_relaxation_times: list[Q],
    residence_time: Q,
    parameter_provenance: str,
) -> ComponentSelectiveReleaseResult:
    """The release rate of a component-selective short-opening orifice.

    Args:
        components: the fluid's substances, by name.
        upstream_z: the upstream stagnation composition, in ``components``' order.
        upstream_t: the upstream stagnation temperature.
        upstream_p: the upstream stagnation pressure.
        diameter: the circular opening's diameter.
        discharge_coefficient: the effective-area factor in ``(0, 1]``.
        back_pressure: the receiving absolute pressure.
        surface_tension: the gas/liquid interfacial tension.
        component_relaxation_times: one first-order relaxation time per component.
        residence_time: the transfer time available to the opening.
        parameter_provenance: the source or calibration identity of the relaxation inputs.

    Raises:
        InvalidInputError: for a non-positive state, opening or transfer parameter, an empty
            provenance, or a composition that is not one entry per component and sums to one.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    t = input_to_si(spec, "upstream_t", upstream_t)
    p = input_to_si(spec, "upstream_p", upstream_p)
    diameter_m = input_to_si(spec, "diameter", diameter)
    back = input_to_si(spec, "back_pressure", back_pressure)
    sigma = input_to_si(spec, "surface_tension", surface_tension)
    if len(component_relaxation_times) != len(components):
        return _failure(
            "unsupported",
            "COMPONENT_TRANSFER_REGIME_UNSUPPORTED",
            "Relaxation-time components must exactly match the fluid components",
            warnings,
        )
    times = [
        input_to_si(spec, "component_relaxation_times", value)
        for value in component_relaxation_times
    ]
    if any(not math.isfinite(value) or value <= 0.0 for value in times):
        raise InvalidInputError(
            "component_relaxation_times",
            "every component relaxation time must be finite and positive",
        )
    residence = input_to_si(spec, "residence_time", residence_time)
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
    if not parameter_provenance.strip():
        raise InvalidInputError(
            "parameter_provenance", "the parameter provenance must not be empty"
        )

    try:
        base = _states(
            components,
            list(upstream_z),
            t,
            p,
            diameter_m,
            float(discharge_coefficient),
            back,
        )
    except _Failure as failure:
        status = "unsupported" if failure.unsupported else "invalid"
        code = (
            "EQUILIBRIUM_BASE_UNSUPPORTED_PHYSICS"
            if failure.unsupported
            else "EQUILIBRIUM_BASE_CALCULATION_FAILED"
        )
        return _failure(status, code, str(failure), warnings)
    if base.status not in ("valid", "valid_with_warnings"):
        code = f"EQUILIBRIUM_BASE_{base.diagnostics[0][0]}"
        return _failure(base.status, code, base.diagnostics[0][1], warnings)

    diagnostics = [
        (
            "MODEL_ASSUMPTIONS",
            "Independent first-order component phase-partition relaxation with caller-declared "
            "times and provenance; equilibrium caloric properties and bounded vertical drift "
            "flux; no predicted transfer coefficients, interfacial heat transfer, entrainment, "
            "annular jet, pipe friction, heat transfer or solids",
        )
    ]
    diagnostics.extend(
        row
        for row in base.diagnostics
        if row[0] not in ("MODEL_ASSUMPTIONS", "THROAT_MACH_MISMATCH", "ACOUSTIC_UNAVAILABLE")
    )
    area = float(discharge_coefficient) * math.pi * diameter_m * diameter_m / 4.0
    if base.mass_flow == 0.0:
        diagnostics.append(
            ("COMPONENT_TRANSFER_NOT_APPLIED", "No forward flow; phase transfer is not resolved")
        )
        return _success(
            0.0,
            False,
            upstream_z,
            base.component_mass_fractions,
            [_row(station) for station in base.stations],
            diagnostics,
            warnings,
        )

    upstream = base.stations[0]
    try:
        relaxed = relax_components(
            upstream,
            base.stations[1],
            times,
            residence,
            list(base.component_mass_fractions),
        )
        outcome = drift_flux(relaxed, sigma)
    except ReleaseRefusal as refusal:
        if refusal.unsupported:
            return _failure(
                "unsupported", "COMPONENT_TRANSFER_REGIME_UNSUPPORTED", str(refusal), warnings
            )
        return _failure("invalid", "COMPONENT_TRANSFER_CLOSURE_FAILED", str(refusal), warnings)

    mixture, _ideal_gas = _components.mixture_of(components, eos="pr")
    mass = molar_mass_of(mixture, list(upstream_z))
    kinetic_energy = 0.5 * (
        relaxed.gas_mass_fraction * outcome.gas_velocity * outcome.gas_velocity
        + (1.0 - relaxed.gas_mass_fraction) * outcome.liquid_velocity * outcome.liquid_velocity
    )
    energy_error = (
        outcome.station.molar_enthalpy - upstream.molar_enthalpy
    ) / mass + kinetic_energy
    reconstructed = reconstructed_components(outcome.station)
    component_error = (
        1.0
        if reconstructed is None
        else max(
            abs(value - upstream_value)
            for value, upstream_value in zip(
                reconstructed, base.component_mass_fractions, strict=True
            )
        )
    )
    if component_error > 1.0e-12 or abs(energy_error) > 1.0e-6:
        return _failure(
            "invalid",
            "COMPONENT_TRANSFER_CLOSURE_FAILED",
            "Component or stagnation-energy closure failed",
            warnings,
        )
    diagnostics.append(
        (
            "COMPONENT_SELECTIVE_PHASE_TRANSFER",
            "Independent component phase-partition relaxation; no predicted transfer coefficients",
        )
    )
    diagnostics.append(
        (
            "COMPONENT_TRANSFER_CONSERVATION",
            "Reconstructed component and stagnation-energy closures hold at the opening",
        )
    )
    diagnostics.append(
        (
            "DRIFT_FLUX_CLOSURE",
            "Phase-area, drift-flux and kinetic-energy closures hold at the opening",
        )
    )
    stations = list(base.stations)
    stations[1] = outcome.station._replace(station="throat_critical")
    stations[2] = outcome.station._replace(station="orifice_exit")
    if (
        len(stations) > 3
        and stations[3].gas_phase is not None
        and stations[3].liquid_phase is not None
    ):
        try:
            relaxed_ambient = relax_components(
                upstream,
                stations[3],
                times,
                residence,
                list(base.component_mass_fractions),
            )
            ambient = drift_flux(relaxed_ambient, sigma)
        except ReleaseRefusal as refusal:
            if refusal.unsupported:
                return _failure(
                    "unsupported", "COMPONENT_TRANSFER_REGIME_UNSUPPORTED", str(refusal), warnings
                )
            return _failure("invalid", "COMPONENT_TRANSFER_CLOSURE_FAILED", str(refusal), warnings)
        stations[3] = ambient.station._replace(station="ambient_expanded")
    elif len(stations) > 3:
        diagnostics.append(
            (
                "AMBIENT_EQUILIBRIUM_ONLY",
                "Ambient-expanded diagnostic does not retain one gas and one liquid phase; "
                "the finite-rate closure is reported only at the opening",
            )
        )
    return _success(
        area * outcome.mass_flux,
        base.choked,
        upstream_z,
        base.component_mass_fractions,
        [_row(station) for station in stations],
        diagnostics,
        warnings,
    )


def _row(station: _Station) -> ComponentSelectiveReleaseStation:
    """One published station row."""
    return ComponentSelectiveReleaseStation(
        station=station.station,
        pressure=from_si(station.pressure, "Pa"),
        temperature=from_si(station.temperature, "K"),
        density=from_si(station.density, "kg/m**3"),
        molar_enthalpy=from_si(station.molar_enthalpy, "J/mol"),
        molar_entropy=from_si(station.molar_entropy, "J/(mol*K)"),
        velocity=from_si(station.velocity, "m/s"),
        mass_flux=from_si(station.mass_flux, "kg/(m**2*s)"),
        phase=station.phase,
        gas_mass_fraction=station.gas_mass_fraction,
        gas_density=(
            None if station.gas_phase is None else from_si(station.gas_phase.density, "kg/m**3")
        ),
        liquid_density=(
            None
            if station.liquid_phase is None
            else from_si(station.liquid_phase.density, "kg/m**3")
        ),
        gas_velocity=(
            None if station.gas_velocity is None else from_si(station.gas_velocity, "m/s")
        ),
        liquid_velocity=(
            None if station.liquid_velocity is None else from_si(station.liquid_velocity, "m/s")
        ),
    )


def _success(
    mass_flow: float,
    choked: bool,
    z: list[float],
    mass_fractions: tuple[float, ...],
    stations: list[ComponentSelectiveReleaseStation],
    diagnostics: list[tuple[str, str]],
    warnings: list[Warning],
) -> ComponentSelectiveReleaseResult:
    return ComponentSelectiveReleaseResult(
        status="valid",
        mass_flow_rate=from_si(mass_flow, "kg/s"),
        choked=choked,
        throat_sound_speed=None,
        component_mole_fractions=tuple(z),
        component_mass_fractions=mass_fractions,
        stations=tuple(stations),
        diagnostics=tuple(
            HemReleaseDiagnostic(code=code, message=message) for code, message in diagnostics
        ),
        diagnostic_codes=tuple(code for code, _ in diagnostics),
        warnings=tuple(warnings),
    )


def _failure(
    status: str, code: str, message: str, warnings: list[Warning]
) -> ComponentSelectiveReleaseResult:
    return ComponentSelectiveReleaseResult(
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
