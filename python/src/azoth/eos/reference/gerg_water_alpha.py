"""``eos.gerg_water_alpha`` - the GERG water attraction-temperature function.

Spec: ``specs/calcs/eos/gerg_water_alpha.toml``, which records why the constructor's copy
of the component's parameters is never read and why the temperature derivatives are not
carried.
"""

from __future__ import annotations

import math

from azoth._registry_gen import spec as _spec_for
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import GergWaterAlphaResult
from azoth.core.units import input_to_si
from azoth.core.warnings import Warning

CALC_ID = "eos.gerg_water_alpha"

# AttractiveTermGERG.parametersGERG and parametersSolidGERG.
GERG_LIQUID = (0.905436, -0.213781, 0.26005)
GERG_SOLID = (0.106025, 2.683845, -4.75638)
SWITCH_TEMPERATURE = 273.16


def gerg_water_alpha(temperature: float, critical_temperature: float) -> GergWaterAlphaResult:
    """The GERG water alpha function.

    Args:
        temperature: the absolute temperature.
        critical_temperature: the component's critical temperature, K.

    Returns:
        The temperature-dependent alpha function.

    Raises:
        OutOfRangeError: if either temperature is not positive.

    Example:
        >>> gerg_water_alpha(300.0, 647.3).alpha
        1.6127672229530905
    """
    spec = _spec_for(CALC_ID)
    checks = checks_for(spec)
    warnings: list[Warning] = []

    temperature = input_to_si(spec, "temperature", temperature)
    critical_temperature = input_to_si(spec, "critical_temperature", critical_temperature)
    values = {"temperature": temperature, "critical_temperature": critical_temperature}
    apply_checks(checks.on_input, values.get, warnings)

    c = GERG_LIQUID if temperature >= SWITCH_TEMPERATURE else GERG_SOLID
    u = 1.0 - math.sqrt(temperature / critical_temperature)
    s = 1.0 + c[0] * u + c[1] * u * u + c[2] * u * u * u * u
    alpha = s * s

    apply_checks(checks.derived, lambda name: alpha if name == "alpha" else None, warnings)

    return GergWaterAlphaResult(alpha=alpha, warnings=tuple(warnings))
