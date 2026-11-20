"""The fire exposure NeqSim's ``FireExposureScenario`` and ``FireHeatLoadCalculator`` carry.

The Python twin of ``crates/azoth-process/src/models/fire_exposure.rs``: the API 521
pool-fire heat load, the fixed flux, the generalized Stefan-Boltzmann radiative flux, and
the passive-protection multiplier.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, replace

from azoth.core.errors import InvalidInputError

#: The Stefan-Boltzmann constant, W/(m2 K4).
STEFAN_BOLTZMANN = 5.670374419e-8


@dataclass(frozen=True)
class FireExposure:
    """One fire exposure."""

    name: str
    fire_type: str
    exposed_area: float
    heat_flux: float
    environmental_factor: float
    emissivity: float
    view_factor: float
    flame_temperature: float
    ambient_temperature: float
    passive_protection_factor: float


def _validate_positive(value: float, name: str) -> None:
    """The class's positive-and-finite check."""
    if value <= 0.0 or not math.isfinite(value):
        raise InvalidInputError(name, "the class requires a positive finite value")


def api521_pool_fire(exposed_area: float, environmental_factor: float) -> FireExposure:
    """The class's API 521 pool-fire exposure."""
    _validate_positive(exposed_area, "exposed_area")
    _validate_positive(environmental_factor, "environmental_factor")
    total_heat = api521_pool_fire_heat_load(exposed_area, environmental_factor)
    return FireExposure(
        name="API 521 pool fire",
        fire_type="api_521_pool_fire",
        exposed_area=exposed_area,
        heat_flux=total_heat / exposed_area,
        environmental_factor=environmental_factor,
        emissivity=0.0,
        view_factor=0.0,
        flame_temperature=1200.0,
        ambient_temperature=298.15,
        passive_protection_factor=1.0,
    )


def fixed_heat_flux(exposed_area: float, heat_flux: float) -> FireExposure:
    """The class's fixed-flux exposure."""
    _validate_positive(exposed_area, "exposed_area")
    _validate_positive(heat_flux, "fire_heat_flux")
    return FireExposure(
        name="Fixed heat flux",
        fire_type="fixed_heat_flux",
        exposed_area=exposed_area,
        heat_flux=heat_flux,
        environmental_factor=1.0,
        emissivity=0.0,
        view_factor=0.0,
        flame_temperature=1200.0,
        ambient_temperature=298.15,
        passive_protection_factor=1.0,
    )


def radiative_fire(
    exposed_area: float,
    emissivity: float,
    view_factor: float,
    flame_temperature: float,
    ambient_temperature: float,
) -> FireExposure:
    """The class's radiative exposure."""
    _validate_positive(exposed_area, "exposed_area")
    _validate_positive(ambient_temperature, "ambient_temperature")
    return FireExposure(
        name="Radiative fire",
        fire_type="radiative_fire",
        exposed_area=exposed_area,
        heat_flux=0.0,
        environmental_factor=1.0,
        emissivity=emissivity,
        view_factor=view_factor,
        flame_temperature=flame_temperature,
        ambient_temperature=ambient_temperature,
        passive_protection_factor=1.0,
    )


def with_passive_protection(scenario: FireExposure, factor: float) -> FireExposure:
    """The class's ``withPassiveProtectionFactor``."""
    if not (0.0 <= factor <= 1.0) or not math.isfinite(factor):
        raise InvalidInputError(
            "passive_protection_factor", "the class refuses a multiplier outside [0, 1]"
        )
    return replace(scenario, passive_protection_factor=factor)


def incident_heat_flux(scenario: FireExposure, outer_surface_temperature: float) -> float:
    """The incident heat flux into the outer surface, W/m2."""
    _validate_positive(outer_surface_temperature, "outer_surface_temperature")
    if scenario.fire_type == "radiative_fire":
        flux = generalized_stefan_boltzmann_heat_flux(
            scenario.emissivity,
            scenario.view_factor,
            scenario.flame_temperature,
            outer_surface_temperature,
        )
    else:
        flux = scenario.heat_flux
    return max(0.0, flux) * scenario.passive_protection_factor


def heat_input(scenario: FireExposure, outer_surface_temperature: float) -> float:
    """The total incident heat input, W."""
    return incident_heat_flux(scenario, outer_surface_temperature) * float(scenario.exposed_area)


def api521_pool_fire_heat_load(wetted_area: float, environmental_factor: float) -> float:
    """The class's API 521 pool-fire heat load, W."""
    _validate_positive(wetted_area, "wetted_area")
    _validate_positive(environmental_factor, "environmental_factor")
    return float(6.19e6 * environmental_factor * wetted_area**0.82)


def generalized_stefan_boltzmann_heat_flux(
    emissivity: float,
    view_factor: float,
    flame_temperature: float,
    surface_temperature: float,
) -> float:
    """The class's generalized Stefan-Boltzmann radiative flux, W/m2."""
    if not (0.0 <= emissivity <= 1.0) or not math.isfinite(emissivity):
        raise InvalidInputError("fire_emissivity", "the class refuses an emissivity outside [0, 1]")
    if not (0.0 <= view_factor <= 1.0) or not math.isfinite(view_factor):
        raise InvalidInputError(
            "fire_view_factor", "the class refuses a view factor outside [0, 1]"
        )
    _validate_positive(flame_temperature, "flame_temperature")
    _validate_positive(surface_temperature, "outer_surface_temperature")
    return (
        emissivity
        * view_factor
        * STEFAN_BOLTZMANN
        * (flame_temperature**4 - surface_temperature**4)
    )
