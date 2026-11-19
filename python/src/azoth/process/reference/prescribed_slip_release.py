"""``process.prescribed_slip_release`` - NeqSim's prescribed-slip orifice release.

Spec: ``specs/models/process/prescribed_slip_release.toml``

The Python twin of ``crates/azoth-process/src/models/prescribed_slip_release.rs``. The
homogeneous-equilibrium scan supplies the stations; the opening's phase velocities are
partitioned by a caller-declared gas-to-liquid ratio, and the rate is the partition's mass
flux times the effective area.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    HemReleaseDiagnostic,
    PrescribedSlipReleaseResult,
    PrescribedSlipReleaseStation,
)
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._release import ReleaseRefusal, prescribed_slip
from azoth.process.reference.hem_release import _Failure, _states, _Station

MODEL_ID = "process.prescribed_slip_release"


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def prescribed_slip_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    gas_to_liquid_slip_ratio: float,
) -> PrescribedSlipReleaseResult:
    """The release rate of a prescribed-slip short-opening orifice.

    Args:
        components: the fluid's substances, by name.
        upstream_z: the upstream stagnation composition, in ``components``' order.
        upstream_t: the upstream stagnation temperature.
        upstream_p: the upstream stagnation pressure.
        diameter: the circular opening's diameter.
        discharge_coefficient: the effective-area factor in ``(0, 1]``.
        back_pressure: the receiving absolute pressure.
        gas_to_liquid_slip_ratio: the gas velocity divided by the liquid velocity.

    Raises:
        InvalidInputError: for a non-positive state or opening, a slip ratio below one, a
            coefficient outside ``(0, 1]``, or a composition that is not one entry per
            component and sums to one.
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
            "gas_to_liquid_slip_ratio": gas_to_liquid_slip_ratio,
        }.get,
        warnings,
    )
    if not math.isfinite(gas_to_liquid_slip_ratio) or gas_to_liquid_slip_ratio < 1.0:
        raise InvalidInputError(
            "gas_to_liquid_slip_ratio",
            "the gas/liquid slip ratio must be finite and at least one, and "
            f"{gas_to_liquid_slip_ratio} is not",
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
            "Equilibrium phase state with prescribed gas/liquid velocity slip; no pipe "
            "friction, heat transfer, finite-rate phase transfer, entrainment correlation "
            "or solids",
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
            ("SLIP_NOT_APPLIED", "No forward flow; phase velocities are not resolved")
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

    try:
        outcome = prescribed_slip(base.stations[1], float(gas_to_liquid_slip_ratio))
    except ReleaseRefusal as refusal:
        if refusal.unsupported:
            return _failure("unsupported", "SLIP_REGIME_UNSUPPORTED", str(refusal), warnings)
        return _failure("invalid", "SLIP_CLOSURE_FAILED", str(refusal), warnings)

    diagnostics.append(
        (
            "PRESCRIBED_PHASE_SLIP",
            "Prescribed gas/liquid velocity ratio; no inferred slip or entrainment correlation",
        )
    )
    diagnostics.append(
        (
            "SLIP_CLOSURE",
            "Phase-area and kinetic-energy closures hold at the opening",
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
            ambient = prescribed_slip(stations[3], float(gas_to_liquid_slip_ratio))
        except ReleaseRefusal:
            pass
        else:
            stations[3] = ambient.station._replace(station="ambient_expanded")
    return _success(
        area * outcome.mass_flux,
        base.choked,
        upstream_z,
        base.component_mass_fractions,
        [_row(station) for station in stations],
        diagnostics,
        warnings,
    )


def _row(station: _Station) -> PrescribedSlipReleaseStation:
    """One published station row."""
    return PrescribedSlipReleaseStation(
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
    stations: list[PrescribedSlipReleaseStation],
    diagnostics: list[tuple[str, str]],
    warnings: list[Warning],
) -> PrescribedSlipReleaseResult:
    return PrescribedSlipReleaseResult(
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
) -> PrescribedSlipReleaseResult:
    return PrescribedSlipReleaseResult(
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
