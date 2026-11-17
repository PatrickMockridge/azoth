"""The shared internals of the ``process.*self_heating`` family.

The two criticality screens and the transient induction solver read one geometry table, one
verdict rule and one bisection. The Python twin of
``crates/azoth-process/src/models/selfheating.rs``; the arithmetic is NeqSim's
``process/safety/selfheating/SelfHeatingGeometry.java`` and the two analyzers.
"""

from __future__ import annotations

import math
from collections.abc import Callable

#: The class's own gas constant, J/(mol K).
R_GAS = 8.314462618

#: The classical Semenov critical parameter, ``1/e``.
PSI_CRIT = 1.0 / math.e

#: The bounds and tolerance of the critical-temperature bisection, K.
T_SEARCH_MIN_K = 150.0
T_SEARCH_MAX_K = 2000.0
T_SEARCH_TOL_K = 1.0e-4

#: The bisection's iteration cap.
MAX_BISECTION_ITERATIONS = 200

#: The class's default marginal ratio.
DEFAULT_MARGINAL_RATIO = 0.7

#: The bracket-failure message, spelled as the class spells it.
BRACKET_WARNING = (
    "Critical ambient temperature could not be bracketed between 150.0 K and 2000.0 K; "
    "check the kinetic parameters"
)

#: The Semenov model's always-present note.
SEMENOV_WARNING = (
    "Semenov theory assumes a spatially uniform body temperature (small Biot number); "
    "use PorousMediaSelfHeatingAnalyzer when internal conduction controls"
)

#: The Frank-Kamenetskii model's always-present note.
FK_WARNING = (
    "Frank-Kamenetskii theory assumes a uniform boundary temperature, conduction-controlled "
    "heat loss (large Biot number) and negligible reactant consumption; use "
    "SemenovSelfHeatingAnalyzer when surface cooling controls"
)

#: The pipe-insulation configuration's note.
PIPE_INSULATION_WARNING = (
    "Pipe-insulation screening: slab half-dimension set to the full insulation thickness and "
    "boundary temperature set to the pipe wall temperature (conservative bounding assumption)"
)

#: The shape table: delta_crit, Laplacian shape factor, one-dimensional flag, dimension name.
GEOMETRIES: dict[str, tuple[float, float, bool, str]] = {
    "slab": (0.878, 0.0, True, "half-thickness"),
    "infinite_cylinder": (2.000, 1.0, True, "radius"),
    "sphere": (3.322, 2.0, True, "radius"),
    "cube": (2.520, 0.0, False, "half-thickness"),
    "equicylinder": (2.760, 1.0, False, "radius"),
}


def delta_crit_of(geometry: str) -> float:
    """The shape's published critical Frank-Kamenetskii parameter."""
    return GEOMETRIES[geometry][0]


def shape_factor_of(geometry: str) -> float:
    """The Laplacian shape factor: 0 slab, 1 cylinder, 2 sphere."""
    return GEOMETRIES[geometry][1]


def one_dimensional(geometry: str) -> bool:
    """Whether the shape reduces to a one-dimensional conduction problem."""
    return GEOMETRIES[geometry][2]


def dimension_description(geometry: str) -> str:
    """The characteristic dimension's own name for this shape."""
    return GEOMETRIES[geometry][3]


def verdict_of(ratio: float, marginal_ratio: float) -> str:
    """The verdict rule: strictly above one is ignition, marginally at the ratio or above."""
    if ratio > 1.0:
        return "self_ignition"
    if ratio >= marginal_ratio:
        return "marginal"
    return "subcritical"


def semenov_psi(
    activation_energy: float,
    volume: float,
    pre_factor: float,
    heat_transfer_coefficient: float,
    surface_area: float,
    temperature_k: float,
) -> float:
    """The Semenov criticality parameter, evaluated in logarithmic form."""
    ln_psi = (
        math.log(activation_energy)
        + math.log(volume)
        + math.log(pre_factor)
        - math.log(heat_transfer_coefficient)
        - math.log(surface_area)
        - math.log(R_GAS)
        - 2.0 * math.log(temperature_k)
        - activation_energy / (R_GAS * temperature_k)
    )
    return math.exp(ln_psi)


def fk_delta(
    activation_energy: float,
    pre_factor: float,
    dimension_m: float,
    conductivity: float,
    temperature_k: float,
) -> float:
    """The Frank-Kamenetskii criticality parameter, evaluated in logarithmic form."""
    ln_delta = (
        math.log(activation_energy)
        + math.log(pre_factor)
        + 2.0 * math.log(dimension_m)
        - math.log(conductivity)
        - math.log(R_GAS)
        - 2.0 * math.log(temperature_k)
        - activation_energy / (R_GAS * temperature_k)
    )
    return math.exp(ln_delta)


def fk_critical_dimension(
    activation_energy: float,
    pre_factor: float,
    conductivity: float,
    temperature_k: float,
    delta_crit: float,
) -> float:
    """The half-dimension at which ``delta`` reaches ``delta_crit``."""
    ln_r_crit = 0.5 * (
        math.log(delta_crit)
        + math.log(conductivity)
        + math.log(R_GAS)
        + 2.0 * math.log(temperature_k)
        + activation_energy / (R_GAS * temperature_k)
        - math.log(activation_energy)
        - math.log(pre_factor)
    )
    return math.exp(ln_r_crit)


def critical_temperature(residual: Callable[[float], float]) -> float | None:
    """The class's bisection on ``[150, 2000]`` K, ``None`` when the bracket fails."""
    low = T_SEARCH_MIN_K
    high = T_SEARCH_MAX_K
    if residual(low) > 0.0 or residual(high) < 0.0:
        return None
    iteration = 0
    while iteration < MAX_BISECTION_ITERATIONS and (high - low) > T_SEARCH_TOL_K:
        mid = 0.5 * (low + high)
        if residual(mid) < 0.0:
            low = mid
        else:
            high = mid
        iteration += 1
    return 0.5 * (low + high)
