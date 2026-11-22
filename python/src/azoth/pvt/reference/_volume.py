"""Molar volume, density and the Peneloux translation, in SI.

The Python twin of ``crates/azoth-pvt/src/volume.rs``. **The translation is chosen by the
cubic**: ``Mixture.volume_shift`` sums the Peng-Robinson components' shifts, and a
Soave-Redlich-Kwong fluid must sum ``Component.volume_shift_srk`` instead.
"""

from __future__ import annotations

from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.eos.reference._mixture_state import mixture_parameters

#: NeqSim's gas constant, as the classes state it.
R = 8.3144621


def volume_shift(mixture: Any, x: list[float]) -> float:
    """The composition-weighted volume translation of a mixture, in m3/mol."""
    if mixture.cubic.name == "srk":
        return float(
            sum(
                x[index] * component.volume_shift_srk
                for index, component in enumerate(mixture.components)
            )
        )
    return float(
        sum(x[index] * component.volume_shift for index, component in enumerate(mixture.components))
    )


def molar_volume(z: float, t: float, p: float) -> float:
    """The untranslated molar volume ``v = Z R T / P``, in m3/mol."""
    return z * R * t / p


def corrected_molar_volume(
    mixture: Any, x: list[float], z: float, t: float, p: float, corrected: bool
) -> float:
    """The molar volume after the Peneloux translation, or before it when correction is off."""
    volume = molar_volume(z, t, p)
    return volume - volume_shift(mixture, x) if corrected else volume


def phase_molar_mass(mixture: Any, x: list[float]) -> float:
    """A phase's molar mass, in kg/mol."""
    mass = 0.0
    for index, component in enumerate(mixture.components):
        if component.molar_mass is None:
            raise InvalidInputError(
                "components", f"component {index} carries no molar mass, so the phase has none"
            )
        mass += x[index] * component.molar_mass.to("kg/mol").magnitude
    return mass


def phase_density(
    mixture: Any, x: list[float], z: float, t: float, p: float, corrected: bool
) -> float:
    """A phase's mass density, in kg/m3."""
    volume = corrected_molar_volume(mixture, x, z, t, p, corrected)
    if not (volume > 0.0):
        raise InvalidInputError("volume", f"the translated molar volume is {volume} m3/mol")
    return phase_molar_mass(mixture, x) / volume


def zvolcorr(z: float, t: float, p: float, corrected: bool, shift: float) -> float:
    """``P V_corr/(n R T)``, the volume-corrected compressibility."""
    untranslated = molar_volume(z, t, p)
    return (untranslated - (shift if corrected else 0.0)) / untranslated * z


def dp_dv_molar(mixture: Any, reduced: Any, x: list[float], z: float, t: float, p: float) -> float:
    """``dP/dv|T`` at a composition, in Pa*mol/m3."""
    a_reduced, b_reduced = mixture_parameters(reduced.a, reduced.b, mixture.kij, x)
    a = a_reduced * R * R * t * t / p
    b = b_reduced * R * t / p
    v = molar_volume(z, t, p)
    delta1 = mixture.cubic.delta1
    delta2 = mixture.cubic.delta2
    return float(
        -R * t / ((v - b) * (v - b))
        + a * (2.0 * v + (delta1 + delta2) * b) / ((v + delta1 * b) ** 2 * (v + delta2 * b) ** 2)
    )


def isothermal_compressibility(
    mixture: Any, reduced: Any, x: list[float], z: float, t: float, p: float
) -> float:
    """The isothermal compressibility ``-1/(V dP/dV|Tn)``, in 1/Pa."""
    volume = molar_volume(z, t, p)
    return -1.0 / (volume * dp_dv_molar(mixture, reduced, x, z, t, p))
