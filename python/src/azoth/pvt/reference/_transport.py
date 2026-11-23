"""A phase's viscosity, by the correlation NeqSim's phase type selects.

The Python twin of ``crates/azoth-pvt/src/transport.rs``. A gas and a liquid phase both
default to ``PFCTViscosityMethodHeavyOil`` at the pin, so :func:`azoth.eos.viscosity` is the
right call for both; an aqueous phase uses the water form. A negative answer is floored at
``1e-5 Pa s`` exactly where ``PhysicalProperties`` floors it.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.eos import aqueous_viscosity, viscosity

#: ``PhysicalProperties.getViscosity``'s floor for a negative answer, Pa*s.
VISCOSITY_FLOOR_PA_S = 1.0e-5

GAS = "gas"
OIL = "oil"
AQUEOUS = "aqueous"


def phase_viscosity(mixture: Any, t: float, p: float, x: list[float], kind: str) -> float:
    """One phase's dynamic viscosity, in Pa*s."""
    if kind in (GAS, OIL):
        value = viscosity(mixture, quantity(t, "K"), quantity(p, "Pa"), x).mu.to("Pa*s").magnitude
    elif kind == AQUEOUS:
        value = (
            aqueous_viscosity(mixture, quantity(t, "K"), quantity(p, "Pa"), x)
            .viscosity.to("Pa*s")
            .magnitude
        )
    else:
        raise InvalidInputError("phase", f"{kind!r} is not a phase kind")
    return _floor(value)


def phase_viscosity_with_csp(
    mixture: Any, t: float, p: float, x: list[float], correction: tuple[float, ...]
) -> float:
    """The same with the PVT regression's four PFCT correction factors.

    Every phase takes the PFCT form once a factor is set, aqueous included: the class
    forces ``PFCT`` on each stored phase slot before it writes the factor.
    """
    from azoth.eos.reference.viscosity import _viscosity_with_csp

    value = (
        _viscosity_with_csp(
            mixture,
            quantity(t, "K"),
            quantity(p, "Pa"),
            x,
            (correction[0], correction[1], correction[2], correction[3]),
        )
        .mu.to("Pa*s")
        .magnitude
    )
    return _floor(value)


def _floor(value: float) -> float:
    if math.isnan(value):
        raise InvalidInputError("viscosity", "the correlation answered a NaN at this state")
    return VISCOSITY_FLOOR_PA_S if value < 0.0 else value
