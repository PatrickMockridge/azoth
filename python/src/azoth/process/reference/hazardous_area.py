"""``process.hazardous_area`` - the IEC 60079-10-1 jet area classification.

Spec: ``specs/models/process/hazardous_area.toml``

The Python twin of ``crates/azoth-process/src/models/hazardous_area.rs``: the effective
source diameter, the mole-to-mass LFL, the de-rated distance and the zone mapping.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import HazardousAreaResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.hazardous_area"

#: Molar mass of air, kg/mol, the class's constant.
M_AIR = 0.028964
#: The Chen & Rodi axial-decay constant, the class's ``JET_DECAY_CONSTANT``.
JET_DECAY_CONSTANT = 5.4
#: The class's default ambient air density, kg/m3.
DEFAULT_AMBIENT_DENSITY = 1.225
#: The class's default factor below the LFL.
DEFAULT_SAFETY_FACTOR = 0.5

_GRADES: dict[str, tuple[str, str]] = {
    "continuous": ("CONTINUOUS", "Zone 0"),
    "primary": ("PRIMARY", "Zone 1"),
    "secondary": ("SECONDARY", "Zone 2"),
}


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _effective_diameter(mass_flow: float, exit_density: float, exit_velocity: float) -> float:
    """The notional source diameter, in m."""
    return math.sqrt(4.0 * mass_flow / (math.pi * exit_density * exit_velocity))


def _lfl_mass_fraction(lfl_volume_fraction: float, gas_molar_mass: float) -> float:
    """The class's ideal mole-to-mass LFL."""
    numerator = lfl_volume_fraction * gas_molar_mass
    return numerator / (numerator + (1.0 - lfl_volume_fraction) * M_AIR)


def _hazardous_distance(
    exit_density: float,
    ambient_density: float,
    diameter: float,
    safety_factor: float,
    lfl_mass_fraction: float,
) -> float:
    """The distance to the de-rated LFL, in m."""
    return (
        JET_DECAY_CONSTANT
        * math.sqrt(exit_density / ambient_density)
        * diameter
        / (safety_factor * lfl_mass_fraction)
    )


def _grade(value: str | None) -> tuple[str, str]:
    """The class's grade-to-zone mapping, and its default."""
    key = "secondary" if value is None else value
    try:
        return _GRADES[key]
    except KeyError:
        raise InvalidInputError(
            "release_grade",
            f"a grade is `continuous`, `primary` or `secondary`, and {key!r} is not",
        ) from None


def _require_positive(name: str, value: float) -> None:
    if not math.isfinite(value) or value <= 0.0:
        raise InvalidInputError(name, f"a finite positive value is required, and {value} is not")


def hazardous_area(
    mass_flow: Q,
    exit_density: Q,
    exit_velocity: Q,
    lfl_volume_fraction: float,
    gas_molar_mass: Q,
    ambient_density: Q | None = None,
    safety_factor: float | None = None,
    release_grade: str | None = None,
) -> HazardousAreaResult:
    """The hazardous distance and zone for a continuous gas jet.

    Raises:
        InvalidInputError: for a non-positive flow, density, velocity, molar mass or
            ambient density, an LFL outside ``(0, 1)``, a safety factor outside
            ``(0, 1]``, or an unknown grade.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    flow_si = input_to_si(spec, "mass_flow", mass_flow)
    density_si = input_to_si(spec, "exit_density", exit_density)
    velocity_si = input_to_si(spec, "exit_velocity", exit_velocity)
    molar_si = input_to_si(spec, "gas_molar_mass", gas_molar_mass)
    ambient_si = (
        DEFAULT_AMBIENT_DENSITY
        if ambient_density is None
        else input_to_si(spec, "ambient_density", ambient_density)
    )
    factor = DEFAULT_SAFETY_FACTOR if safety_factor is None else float(safety_factor)
    grade_label, zone = _grade(release_grade)

    apply_checks(
        checks.on_input,
        {
            "mass_flow": flow_si,
            "exit_density": density_si,
            "exit_velocity": velocity_si,
            "lfl_volume_fraction": lfl_volume_fraction,
            "gas_molar_mass": molar_si,
            "ambient_density": ambient_si,
            "safety_factor": factor,
        }.get,
        warnings,
    )
    _require_positive("mass_flow", flow_si)
    _require_positive("exit_density", density_si)
    _require_positive("exit_velocity", velocity_si)
    _require_positive("gas_molar_mass", molar_si)
    _require_positive("ambient_density", ambient_si)
    if (
        not math.isfinite(lfl_volume_fraction)
        or lfl_volume_fraction <= 0.0
        or lfl_volume_fraction >= 1.0
    ):
        raise InvalidInputError(
            "lfl_volume_fraction",
            f"an LFL in (0, 1) is required, and {lfl_volume_fraction} is not",
        )
    if not math.isfinite(factor) or factor <= 0.0 or factor > 1.0:
        raise InvalidInputError(
            "safety_factor", f"a factor in (0, 1] is required, and {factor} is not"
        )

    diameter = _effective_diameter(flow_si, density_si, velocity_si)
    lfl_mass = _lfl_mass_fraction(lfl_volume_fraction, molar_si)
    distance = _hazardous_distance(density_si, ambient_si, diameter, factor, lfl_mass)
    return HazardousAreaResult(
        effective_diameter=from_si(diameter, "m"),
        lfl_mass_fraction=lfl_mass,
        hazardous_distance=from_si(distance, "m"),
        release_grade=grade_label,
        zone_classification=zone,
        warnings=tuple(warnings),
    )
