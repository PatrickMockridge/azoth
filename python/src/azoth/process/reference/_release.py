"""Shared arithmetic for the derived release source-term models.

The Python twin of ``crates/azoth-process/src/models/release.rs``. The slip, drift-flux and
finite-rate models all post-process the equilibrium stations of ``process.hem_release``;
this module holds the basis they share.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, replace
from typing import Any


class ReleaseRefusal(Exception):
    """A failure a derived release model reports inside its result."""

    def __init__(self, message: str, *, unsupported: bool) -> None:
        super().__init__(message)
        self.unsupported = unsupported


@dataclass(frozen=True, slots=True)
class PhaseBasis:
    """The resolved gas and liquid basis of one station."""

    gas_fraction: float
    liquid_fraction: float
    gas_density: float
    liquid_density: float


def phase_basis(station: Any) -> PhaseBasis:
    """The gas and liquid basis, or the class's refusal."""
    gas = station.gas_phase
    liquid = station.liquid_phase
    if gas is None or liquid is None:
        raise ReleaseRefusal(
            "Exactly one gas and one liquid phase are required at the opening",
            unsupported=True,
        )
    gas_fraction = station.gas_mass_fraction
    liquid_fraction = 1.0 - gas_fraction
    if gas_fraction <= 1.0e-10 or liquid_fraction <= 1.0e-10:
        raise ReleaseRefusal(
            "Both phases must have positive resolved mass fraction", unsupported=True
        )
    if (
        not math.isfinite(gas.density)
        or gas.density <= 0.0
        or not math.isfinite(liquid.density)
        or liquid.density <= 0.0
    ):
        raise ReleaseRefusal(
            "Phase density and mass-fraction bases do not match", unsupported=False
        )
    return PhaseBasis(
        gas_fraction=gas_fraction,
        liquid_fraction=liquid_fraction,
        gas_density=gas.density,
        liquid_density=liquid.density,
    )


@dataclass(frozen=True, slots=True)
class SlipOutcome:
    """One station's slip hydrodynamics."""

    station: Any
    mass_flux: float
    gas_velocity: float
    liquid_velocity: float
    gas_area_fraction: float
    phase_area_sum: float
    kinetic_energy_error: float


def prescribed_slip(station: Any, gas_to_liquid_slip_ratio: float) -> SlipOutcome:
    """The prescribed-slip partition of one station."""
    basis = phase_basis(station)
    homogeneous_velocity = station.velocity
    if not math.isfinite(homogeneous_velocity) or homogeneous_velocity <= 0.0:
        raise ReleaseRefusal("homogeneous throat velocity must be positive", unsupported=False)
    ratio_squared = gas_to_liquid_slip_ratio * gas_to_liquid_slip_ratio
    denominator = basis.gas_fraction * ratio_squared + basis.liquid_fraction
    liquid_velocity = homogeneous_velocity / math.sqrt(denominator)
    gas_velocity = gas_to_liquid_slip_ratio * liquid_velocity
    specific_area = basis.gas_fraction / (basis.gas_density * gas_velocity) + (
        basis.liquid_fraction / (basis.liquid_density * liquid_velocity)
    )
    mass_flux = 1.0 / specific_area
    gas_area_fraction = mass_flux * basis.gas_fraction / (basis.gas_density * gas_velocity)
    liquid_area_fraction = (
        mass_flux * basis.liquid_fraction / (basis.liquid_density * liquid_velocity)
    )
    phase_area_sum = gas_area_fraction + liquid_area_fraction
    phase_kinetic_energy = 0.5 * (
        basis.gas_fraction * gas_velocity * gas_velocity
        + basis.liquid_fraction * liquid_velocity * liquid_velocity
    )
    homogeneous_kinetic_energy = 0.5 * homogeneous_velocity * homogeneous_velocity
    kinetic_energy_error = phase_kinetic_energy - homogeneous_kinetic_energy
    if (
        not math.isfinite(mass_flux)
        or mass_flux <= 0.0
        or abs(phase_area_sum - 1.0) > 1.0e-10
        or abs(kinetic_energy_error) > 1.0e-9 * max(1.0, homogeneous_kinetic_energy)
    ):
        raise ReleaseRefusal("Phase area or kinetic-energy closure failed", unsupported=False)
    bulk_velocity = mass_flux / station.density
    partitioned = station._replace(
        velocity=bulk_velocity,
        mass_flux=station.density * bulk_velocity,
        gas_velocity=gas_velocity,
        liquid_velocity=liquid_velocity,
    )
    return SlipOutcome(
        station=partitioned,
        mass_flux=mass_flux,
        gas_velocity=gas_velocity,
        liquid_velocity=liquid_velocity,
        gas_area_fraction=gas_area_fraction,
        phase_area_sum=phase_area_sum,
        kinetic_energy_error=kinetic_energy_error,
    )


#: The class's distribution parameter.
DRIFT_DISTRIBUTION_PARAMETER = 1.2
#: The class's Harmathy coefficient.
HARMATHY_COEFFICIENT = 1.53
#: The class's bounded bubbly/dispersed gas area fraction.
MAX_GAS_AREA_FRACTION = 0.80
#: The class's largest bracketed slip ratio.
MAX_SLIP_RATIO = 128.0
#: The closure's bisection cap.
DRIFT_BISECTIONS = 80
#: The closure's gravity.
GRAVITY = 9.80665


@dataclass(frozen=True, slots=True)
class DriftOutcome:
    """One station's drift-flux hydrodynamics."""

    station: Any
    mass_flux: float
    slip_ratio: float
    gas_velocity: float
    liquid_velocity: float
    gas_area_fraction: float
    phase_area_sum: float
    kinetic_energy_error: float
    drift_velocity: float
    drift_residual: float


@dataclass(frozen=True, slots=True)
class _DriftRaw:
    mass_flux: float
    gas_velocity: float
    liquid_velocity: float
    gas_area_fraction: float
    phase_area_sum: float
    kinetic_energy_error: float
    drift_residual: float


def _drift_raw(
    station: Any, basis: PhaseBasis, slip_ratio: float, drift_velocity: float
) -> _DriftRaw:
    """The drift-flux closure at one slip ratio."""
    homogeneous_velocity = station.velocity
    if not math.isfinite(homogeneous_velocity) or homogeneous_velocity <= 0.0:
        raise ReleaseRefusal("homogeneous throat velocity must be positive", unsupported=False)
    denominator = basis.gas_fraction * slip_ratio * slip_ratio + basis.liquid_fraction
    liquid_velocity = homogeneous_velocity / math.sqrt(denominator)
    gas_velocity = slip_ratio * liquid_velocity
    specific_area = basis.gas_fraction / (basis.gas_density * gas_velocity) + (
        basis.liquid_fraction / (basis.liquid_density * liquid_velocity)
    )
    mass_flux = 1.0 / specific_area
    gas_area_fraction = mass_flux * basis.gas_fraction / (basis.gas_density * gas_velocity)
    liquid_area_fraction = (
        mass_flux * basis.liquid_fraction / (basis.liquid_density * liquid_velocity)
    )
    phase_area_sum = gas_area_fraction + liquid_area_fraction
    volumetric_flux = mass_flux * (
        basis.gas_fraction / basis.gas_density + basis.liquid_fraction / basis.liquid_density
    )
    drift_residual = gas_velocity - DRIFT_DISTRIBUTION_PARAMETER * volumetric_flux - drift_velocity
    phase_kinetic_energy = 0.5 * (
        basis.gas_fraction * gas_velocity * gas_velocity
        + basis.liquid_fraction * liquid_velocity * liquid_velocity
    )
    homogeneous_kinetic_energy = 0.5 * homogeneous_velocity * homogeneous_velocity
    kinetic_energy_error = phase_kinetic_energy - homogeneous_kinetic_energy
    if (
        not math.isfinite(mass_flux)
        or mass_flux <= 0.0
        or not math.isfinite(drift_residual)
        or abs(phase_area_sum - 1.0) > 1.0e-10
        or abs(kinetic_energy_error) > 1.0e-9 * max(1.0, homogeneous_kinetic_energy)
    ):
        raise ReleaseRefusal(
            "Phase area, drift-flux or kinetic-energy closure failed", unsupported=False
        )
    return _DriftRaw(
        mass_flux=mass_flux,
        gas_velocity=gas_velocity,
        liquid_velocity=liquid_velocity,
        gas_area_fraction=gas_area_fraction,
        phase_area_sum=phase_area_sum,
        kinetic_energy_error=kinetic_energy_error,
        drift_residual=drift_residual,
    )


def drift_flux(station: Any, surface_tension: float) -> DriftOutcome:
    """The vertical drift-flux partition of one station."""
    basis = phase_basis(station)
    if basis.liquid_density <= basis.gas_density:
        raise ReleaseRefusal(
            "Harmathy closure requires liquid density greater than gas density",
            unsupported=True,
        )
    drift_velocity = (
        HARMATHY_COEFFICIENT
        * (
            GRAVITY
            * surface_tension
            * (basis.liquid_density - basis.gas_density)
            / (basis.liquid_density * basis.liquid_density)
        )
        ** 0.25
    )
    if not math.isfinite(drift_velocity) or drift_velocity <= 0.0:
        raise ReleaseRefusal("Harmathy drift velocity must be positive", unsupported=False)
    lower = 1.0
    lower_residual = _drift_raw(station, basis, lower, drift_velocity).drift_residual
    upper = 2.0
    upper_residual = _drift_raw(station, basis, upper, drift_velocity).drift_residual
    while upper_residual <= 0.0 and upper < MAX_SLIP_RATIO:
        upper *= 2.0
        upper_residual = _drift_raw(station, basis, upper, drift_velocity).drift_residual
    if not lower_residual < 0.0 or not upper_residual > 0.0:
        raise ReleaseRefusal(
            "Unable to bracket positive drift-flux slip solution", unsupported=False
        )
    for _ in range(DRIFT_BISECTIONS):
        middle = 0.5 * (lower + upper)
        middle_residual = _drift_raw(station, basis, middle, drift_velocity).drift_residual
        if middle_residual > 0.0:
            upper = middle
        else:
            lower = middle
    slip_ratio = 0.5 * (lower + upper)
    raw = _drift_raw(station, basis, slip_ratio, drift_velocity)
    if raw.gas_area_fraction > MAX_GAS_AREA_FRACTION:
        raise ReleaseRefusal(
            f"Predicted gas area fraction {raw.gas_area_fraction:.6f} exceeds the bounded "
            f"bubbly/dispersed screening limit {MAX_GAS_AREA_FRACTION}",
            unsupported=True,
        )
    bulk_velocity = raw.mass_flux / station.density
    partitioned = station._replace(
        velocity=bulk_velocity,
        mass_flux=station.density * bulk_velocity,
        gas_velocity=raw.gas_velocity,
        liquid_velocity=raw.liquid_velocity,
    )
    return DriftOutcome(
        station=partitioned,
        mass_flux=raw.mass_flux,
        slip_ratio=slip_ratio,
        gas_velocity=raw.gas_velocity,
        liquid_velocity=raw.liquid_velocity,
        gas_area_fraction=raw.gas_area_fraction,
        phase_area_sum=raw.phase_area_sum,
        kinetic_energy_error=raw.kinetic_energy_error,
        drift_velocity=drift_velocity,
        drift_residual=raw.drift_residual,
    )


def relax_uniform(upstream_gas_fraction: float, equilibrium: Any, progress: float) -> Any:
    """The uniform first-order relaxation of the equilibrium station."""
    basis = phase_basis(equilibrium)
    relaxed = upstream_gas_fraction + progress * (basis.gas_fraction - upstream_gas_fraction)
    if relaxed <= 1.0e-10 or relaxed >= 1.0 - 1.0e-10:
        raise ReleaseRefusal(
            "Relaxed state must retain positive gas and liquid mass fractions",
            unsupported=True,
        )
    density = 1.0 / (relaxed / basis.gas_density + (1.0 - relaxed) / basis.liquid_density)
    gas = replace(equilibrium.gas_phase, mass_fraction=relaxed)
    liquid = replace(equilibrium.liquid_phase, mass_fraction=1.0 - relaxed)
    return equilibrium._replace(
        density=density,
        gas_mass_fraction=relaxed,
        gas_phase=gas,
        liquid_phase=liquid,
    )
