"""``eos.cpa_water_caloric_alpha`` - the caloric-fitted SRK-CPA water alpha function.

Spec: ``specs/calcs/eos/cpa_water_caloric_alpha.toml``, which records that the
coefficients are the class's own calibration, not the cited paper's parameter set.
"""

from __future__ import annotations

import math

from azoth._registry_gen import spec as _spec_for
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import CpaWaterCaloricAlphaResult
from azoth.core.units import input_to_si
from azoth.core.warnings import Warning

CALC_ID = "eos.cpa_water_caloric_alpha"

CPA_CALORIC = (
    0.6670190973128074,
    -0.25656681951681287,
    1.3687767086793399,
    -3.1891110889699097,
    5.218580308820144,
)


def cpa_water_caloric_alpha(
    temperature: float, critical_temperature: float
) -> CpaWaterCaloricAlphaResult:
    """The caloric-fitted SRK-CPA water alpha function.

    Args:
        temperature: the absolute temperature.
        critical_temperature: the water component's critical temperature, K.

    Returns:
        The temperature-dependent alpha function.

    Raises:
        OutOfRangeError: if either temperature is not positive.

    Example:
        >>> cpa_water_caloric_alpha(298.15, 647.3).alpha
        1.4813963230257607
    """
    spec = _spec_for(CALC_ID)
    checks = checks_for(spec)
    warnings: list[Warning] = []

    temperature = input_to_si(spec, "temperature", temperature)
    critical_temperature = input_to_si(spec, "critical_temperature", critical_temperature)
    values = {"temperature": temperature, "critical_temperature": critical_temperature}
    apply_checks(checks.on_input, values.get, warnings)

    u = 1.0 - math.sqrt(temperature / critical_temperature)
    c = CPA_CALORIC
    s = (
        1.0
        + c[0] * u
        + c[1] * u * u
        + c[2] * u * u * u
        + c[3] * u * u * u * u
        + c[4] * u * u * u * u * u
    )
    alpha = s * s

    apply_checks(checks.derived, lambda name: alpha if name == "alpha" else None, warnings)

    return CpaWaterCaloricAlphaResult(alpha=alpha, warnings=tuple(warnings))
