"""Shared arithmetic for the derived release source-term models.

The Python twin of ``crates/azoth-process/src/models/release.rs``. The slip, drift-flux and
finite-rate models all post-process the equilibrium stations of ``process.hem_release``;
this module holds the basis they share.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
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
