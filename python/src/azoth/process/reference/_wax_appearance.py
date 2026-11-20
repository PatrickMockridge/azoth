"""The wax appearance temperature of NeqSim's ``WATcalc``.

The Python twin of ``crates/azoth-process/src/models/wax_appearance.rs``:
``calcWAT`` brackets the appearance on a 10 K scan of independent TP wax flashes,
bisects the bracket to 1e-4 K, and returns the warm endpoint after re-checking both
ends. The onset is the wax **mass** fraction crossing 1e-8; the ported
``eos.tp_multiflash_wax`` reports a mole fraction, so the mass fraction is assembled
from the wax phase's molar mass and the feed's.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.eos.reference.tp_multiflash_wax import tp_multiflash_wax

#: The class's appearance threshold, a wax mass fraction of the whole fluid.
WAX_MASS_TOLERANCE = 1.0e-8
#: The class's final bracket width, K.
TEMPERATURE_TOLERANCE = 1.0e-4
#: The class's lower search bound, K.
MIN_TEMPERATURE = 100.0
#: The class's upper search bound, K.
MAX_TEMPERATURE = 1000.0
#: The class's scan step, K.
SEARCH_STEP = 10.0
#: The class's cap on TP flash evaluations.
MAX_FLASHES = 120


class _Probe:
    """One feed's wax probe, counting the class's flash evaluations."""

    def __init__(
        self,
        mixture: Any,
        names: list[str],
        eos: str,
        pressure: float,
        z: list[float],
        feed_molar_mass: float,
    ) -> None:
        self.mixture = mixture
        self.names = names
        self.eos = eos
        self.pressure = pressure
        self.z = z
        self.feed_molar_mass = feed_molar_mass
        self.evaluations = 0

    def has_wax(self, temperature: float) -> bool:
        """Whether a wax phase above the class's mass threshold exists at a temperature."""
        self.evaluations += 1
        if self.evaluations > MAX_FLASHES:
            raise InvalidInputError(
                "wax_appearance", f"exceeded {MAX_FLASHES} TP flash evaluations"
            )
        result = tp_multiflash_wax(
            self.names,
            quantity(temperature, "K"),
            quantity(self.pressure, "Pa"),
            self.z,
            eos=self.eos,
        )
        if not result.converged or result.phase_count < 3 or not result.wax_fraction > 0.0:
            return False
        composition = list(result.x[-1])
        wax_mass = result.wax_fraction * molar_mass_of(self.mixture, composition)
        fraction = wax_mass / self.feed_molar_mass
        if not math.isfinite(fraction) or fraction < 0.0:
            raise InvalidInputError("wax_appearance", "the wax mass fraction is not a fraction")
        return fraction > WAX_MASS_TOLERANCE


def wat_temperature(
    names: list[str],
    eos: str,
    start_temperature: float,
    pressure: float,
    z: list[float],
) -> float:
    """The class's ``calcWAT``: the warm endpoint of the appearance bracket, K."""
    if not math.isfinite(start_temperature) or not (
        MIN_TEMPERATURE <= start_temperature <= MAX_TEMPERATURE
    ):
        raise InvalidInputError(
            "design_temperature", "calcWAT requires a finite temperature in 100-1000 K"
        )
    if not pressure > 0.0 or not math.isfinite(pressure):
        raise InvalidInputError("pressure", "calcWAT requires a positive finite pressure")
    mixture, _ideal_gas = _components.mixture_of(names, eos=eos)
    feed_molar_mass = molar_mass_of(mixture, z)
    probe = _Probe(mixture, names, eos, pressure, z, feed_molar_mass)

    temperature = start_temperature
    lower = math.nan
    upper = math.nan
    if probe.has_wax(temperature):
        lower = temperature
        wax = True
        while wax and temperature < MAX_TEMPERATURE:
            temperature = min(MAX_TEMPERATURE, temperature + SEARCH_STEP)
            wax = probe.has_wax(temperature)
            if wax:
                lower = temperature
            else:
                upper = temperature
    else:
        upper = temperature
        wax = False
        while not wax and temperature > MIN_TEMPERATURE:
            temperature = max(MIN_TEMPERATURE, temperature - SEARCH_STEP)
            wax = probe.has_wax(temperature)
            if wax:
                lower = temperature
            else:
                upper = temperature
    if not math.isfinite(lower) or not math.isfinite(upper):
        raise InvalidInputError(
            "wax_appearance", "no wax appearance bracket in the 100-1000 K search range"
        )
    while upper - lower > TEMPERATURE_TOLERANCE:
        temperature = 0.5 * (lower + upper)
        if probe.has_wax(temperature):
            lower = temperature
        else:
            upper = temperature
    if not probe.has_wax(lower) or probe.has_wax(upper):
        raise InvalidInputError(
            "wax_appearance", "the wax appearance endpoints were not reproducible"
        )
    return upper
