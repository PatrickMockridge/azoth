"""``process.bubble_transfer_zone_release`` - NeqSim's predictive bubble transfer zone.

Spec: ``specs/models/process/bubble_transfer_zone_release.toml``

The Python twin of ``crates/azoth-process/src/models/bubble_transfer_zone_release.rs``. The
drift-flux base supplies the phase velocities; a fixed point predicts the bubble diameter,
the relative velocity and the residence time, and the Ranz-Marshall adapter solves each
iterate.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    BubbleTransferZoneReleaseResult,
    BubbleTransferZoneReleaseStation,
    HemReleaseDiagnostic,
)
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.process.reference.drift_flux_release import drift_flux_release
from azoth.process.reference.ranz_marshall_release import ranz_marshall_release

MODEL_ID = "process.bubble_transfer_zone_release"

#: The class's fixed-point iteration cap.
MAX_FIXED_POINT_ITERATIONS = 24
#: The class's fixed-point relative tolerance.
FIXED_POINT_RELATIVE_TOLERANCE = 1.0e-8
#: The class's gravity.
GRAVITY = 9.80665
#: The Eotvos bound.
MAX_EOTVOS_NUMBER = 4.0
#: The Weber bound.
MAX_WEBER_NUMBER = 3.0
#: The hydraulic-diameter fraction.
MAX_BUBBLE_TO_HYDRAULIC_DIAMETER = 0.25


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


class _PredictionRefusal(Exception):
    """A prediction refusal, carrying whether it is a regime gap."""

    def __init__(self, message: str, *, unsupported: bool) -> None:
        super().__init__(message)
        self.unsupported = unsupported


def bubble_transfer_zone_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    surface_tension: Q,
    transfer_zone_hydraulic_diameter: Q,
    transfer_zone_vertical_length: Q,
    continuous_phase_dynamic_viscosity: Q,
    component_diffusivities: list[Q],
    parameter_provenance: str,
) -> BubbleTransferZoneReleaseResult:
    """The release rate of a predictive bubble transfer-zone orifice.

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
    hydraulic = input_to_si(
        spec, "transfer_zone_hydraulic_diameter", transfer_zone_hydraulic_diameter
    )
    vertical = input_to_si(spec, "transfer_zone_vertical_length", transfer_zone_vertical_length)
    mu_c = input_to_si(
        spec, "continuous_phase_dynamic_viscosity", continuous_phase_dynamic_viscosity
    )
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
            "transfer_zone_hydraulic_diameter": hydraulic,
            "transfer_zone_vertical_length": vertical,
            "continuous_phase_dynamic_viscosity": mu_c,
        }.get,
        warnings,
    )
    for name, value in (
        ("surface_tension", sigma),
        ("transfer_zone_hydraulic_diameter", hydraulic),
        ("transfer_zone_vertical_length", vertical),
        ("continuous_phase_dynamic_viscosity", mu_c),
    ):
        if not math.isfinite(value) or value <= 0.0:
            raise InvalidInputError(name, f"{name} must be finite and positive, and {value} is not")
    if not parameter_provenance.strip():
        raise InvalidInputError(
            "parameter_provenance", "the parameter provenance must not be empty"
        )
    if len(diffusivities) != len(components):
        return _failure(
            "invalid",
            "TRANSFER_ZONE_COMPONENT_BASIS_INVALID",
            "Diffusivity components must exactly match the release-fluid components",
            warnings,
        )
    if any(not math.isfinite(value) or value <= 0.0 for value in diffusivities):
        raise InvalidInputError(
            "component_diffusivities",
            "every component diffusivity must be finite and positive",
        )

    base = drift_flux_release(
        components,
        list(upstream_z),
        upstream_t,
        upstream_p,
        diameter,
        discharge_coefficient,
        back_pressure,
        surface_tension,
    )
    if base.status != "valid":
        return _failure(
            base.status,
            f"TRANSFER_ZONE_BASE_{base.diagnostic_codes[0]}",
            base.diagnostics[0].message,
            list(base.warnings),
        )
    if base.mass_flow_rate is not None and base.mass_flow_rate.to("kg/s").magnitude == 0.0:
        return _success(
            0.0,
            False,
            upstream_z,
            base.component_mass_fractions,
            [_row(station) for station in base.stations],
            [("TRANSFER_ZONE_NOT_APPLIED", "No forward release flow")],
            list(base.warnings),
        )

    throat = base.stations[1]
    gas_density = 0.0 if throat.gas_density is None else throat.gas_density.to("kg/m**3").magnitude
    liquid_density = (
        0.0 if throat.liquid_density is None else throat.liquid_density.to("kg/m**3").magnitude
    )
    gas_velocity = 0.0 if throat.gas_velocity is None else throat.gas_velocity.to("m/s").magnitude
    liquid_velocity = (
        0.0 if throat.liquid_velocity is None else throat.liquid_velocity.to("m/s").magnitude
    )

    previous: tuple[float, ...] | None = None
    previous_rate = math.nan
    for iteration in range(1, MAX_FIXED_POINT_ITERATIONS + 1):
        try:
            prediction = _predict(
                gas_density,
                liquid_density,
                gas_velocity,
                liquid_velocity,
                sigma,
                hydraulic,
                vertical,
                diameter_m,
            )
        except _PredictionRefusal as refusal:
            if refusal.unsupported:
                return _failure(
                    "unsupported",
                    "TRANSFER_ZONE_REGIME_UNSUPPORTED",
                    str(refusal),
                    warnings,
                )
            return _failure("invalid", "TRANSFER_ZONE_CLOSURE_FAILED", str(refusal), warnings)
        d32, relative, residence, liquid_rho = prediction
        delegate = ranz_marshall_release(
            components,
            list(upstream_z),
            upstream_t,
            upstream_p,
            diameter,
            discharge_coefficient,
            back_pressure,
            surface_tension,
            from_si(residence, "s"),
            "gas_bubbles",
            from_si(d32, "m"),
            from_si(liquid_rho, "kg/m**3"),
            continuous_phase_dynamic_viscosity,
            from_si(relative, "m/s"),
            [from_si(value, "m**2/s") for value in diffusivities],
            parameter_provenance,
        )
        if delegate.status != "valid":
            return _failure(
                delegate.status,
                f"TRANSFER_ZONE_{delegate.diagnostic_codes[0]}",
                delegate.diagnostics[0].message,
                list(delegate.warnings),
            )
        rate = (
            0.0 if delegate.mass_flow_rate is None else delegate.mass_flow_rate.to("kg/s").magnitude
        )
        if (
            previous is not None
            and _converged(previous, prediction)
            and (_relative(previous_rate, rate) <= FIXED_POINT_RELATIVE_TOLERANCE)
        ):
            diagnostics = [
                (
                    "MODEL_ASSUMPTIONS",
                    "Predictive vertical spherical-bubble transfer zone with bounded "
                    "Eotvos/Weber/geometry diameter, resolved drift-flux phase velocities, gas "
                    "residence time and Ranz-Marshall external-film transfer; no population "
                    "balance, entrainment, annular flow, interfacial heat transfer or solids",
                ),
                (
                    "PREDICTIVE_BUBBLE_TRANSFER_ZONE",
                    "Diameter, relative velocity and residence time from the bounded "
                    "transfer-zone prediction",
                ),
            ]
            diagnostics.extend(
                (row.code, row.message)
                for row in delegate.diagnostics
                if row.code != "MODEL_ASSUMPTIONS"
            )
            return _success(
                rate,
                delegate.choked,
                upstream_z,
                delegate.component_mass_fractions,
                [_row(station) for station in delegate.stations],
                diagnostics,
                list(delegate.warnings),
            )
        previous = prediction
        previous_rate = rate
        next_throat = delegate.stations[1]
        gas_density = (
            0.0
            if next_throat.gas_density is None
            else next_throat.gas_density.to("kg/m**3").magnitude
        )
        liquid_density = (
            0.0
            if next_throat.liquid_density is None
            else next_throat.liquid_density.to("kg/m**3").magnitude
        )
        gas_velocity = (
            0.0
            if next_throat.gas_velocity is None
            else next_throat.gas_velocity.to("m/s").magnitude
        )
        liquid_velocity = (
            0.0
            if next_throat.liquid_velocity is None
            else next_throat.liquid_velocity.to("m/s").magnitude
        )
        _ = iteration
    return _failure(
        "invalid",
        "TRANSFER_ZONE_NOT_CONVERGED",
        "Bubble diameter, phase velocity, residence time and release rate did not converge "
        "within 24 fixed-point iterations",
        warnings,
    )


def _predict(
    gas_density: float,
    liquid_density: float,
    gas_velocity: float,
    liquid_velocity: float,
    surface_tension: float,
    hydraulic_diameter: float,
    vertical_length: float,
    opening_diameter: float,
) -> tuple[float, float, float, float]:
    """The predicted diameter, relative velocity, residence time and liquid density."""
    if gas_density <= 0.0 or liquid_density <= 0.0 or gas_velocity <= 0.0 or liquid_velocity <= 0.0:
        raise _PredictionRefusal(
            "Resolved one-gas/one-liquid phase velocities are required", unsupported=True
        )
    if liquid_density <= gas_density:
        raise _PredictionRefusal(
            "Transfer-zone closure requires liquid density greater than gas density",
            unsupported=True,
        )
    relative_velocity = abs(gas_velocity - liquid_velocity)
    eotvos_diameter = math.sqrt(
        MAX_EOTVOS_NUMBER * surface_tension / (GRAVITY * (liquid_density - gas_density))
    )
    weber_diameter = (
        math.inf
        if relative_velocity == 0.0
        else MAX_WEBER_NUMBER
        * surface_tension
        / (liquid_density * relative_velocity * relative_velocity)
    )
    hydraulic_limit = MAX_BUBBLE_TO_HYDRAULIC_DIAMETER * hydraulic_diameter
    diameter = min(eotvos_diameter, weber_diameter, hydraulic_limit, opening_diameter)
    if not math.isfinite(diameter) or diameter <= 0.0:
        raise _PredictionRefusal("predicted bubble diameter must be positive", unsupported=False)
    residence = vertical_length / gas_velocity
    if not math.isfinite(residence) or residence <= 0.0:
        raise _PredictionRefusal(
            "predicted bubble residence time must be positive", unsupported=False
        )
    return (diameter, relative_velocity, residence, liquid_density)


def _converged(previous: tuple[float, ...], current: tuple[float, ...]) -> bool:
    return all(
        _relative(first, second) <= FIXED_POINT_RELATIVE_TOLERANCE
        for first, second in zip(previous, current, strict=True)
    )


def _relative(first: float, second: float) -> float:
    if not math.isfinite(first) or not math.isfinite(second):
        return math.inf
    return abs(first - second) / max(1.0e-15, abs(first), abs(second))


def _row(station: Any) -> BubbleTransferZoneReleaseStation:
    """One delegate station as this model's row."""
    return BubbleTransferZoneReleaseStation(
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


def _success(
    mass_flow: float,
    choked: bool,
    z: list[float],
    mass_fractions: tuple[float, ...],
    stations: list[BubbleTransferZoneReleaseStation],
    diagnostics: list[tuple[str, str]],
    warnings: list[Warning],
) -> BubbleTransferZoneReleaseResult:
    return BubbleTransferZoneReleaseResult(
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
) -> BubbleTransferZoneReleaseResult:
    return BubbleTransferZoneReleaseResult(
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
