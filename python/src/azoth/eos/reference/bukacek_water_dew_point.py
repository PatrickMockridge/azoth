"""``eos.bukacek_water_dew_point`` - the Bukacek water dew point.

Spec: ``specs/calcs/eos/bukacek_water_dew_point.toml``, which records why a capped state
is not asserted.
"""

from __future__ import annotations

from azoth._registry_gen import spec as _spec_for
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import BukacekWaterDewPointResult
from azoth.core.units import from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.eos.reference.bukacek_water_in_gas import water_in_gas_mole_fraction

CALC_ID = "eos.bukacek_water_dew_point"

# BukacekWaterInGas.waterDewPointTemperature's loop constants.
START_TEMPERATURE = 273.15
STEP = 1e5
RELATIVE_TOLERANCE = 1e-8
MAX_ITERATIONS = 1000


def bukacek_water_dew_point(
    water_mole_fraction: float, pressure: float
) -> BukacekWaterDewPointResult:
    """The Bukacek water dew-point temperature.

    Args:
        water_mole_fraction: the water mole fraction whose dew point is wanted.
        pressure: the total pressure, Pa.

    Returns:
        The water dew-point temperature, K.

    Raises:
        OutOfRangeError: if ``water_mole_fraction`` or ``pressure`` is not positive.

    Example:
        >>> round(bukacek_water_dew_point(20e-6, 7_000_000.0).temperature, 9)
        248.335109008
    """
    spec = _spec_for(CALC_ID)
    checks = checks_for(spec)
    warnings: list[Warning] = []

    pressure = input_to_si(spec, "pressure", pressure)
    values = {"water_mole_fraction": water_mole_fraction, "pressure": pressure}
    apply_checks(checks.on_input, values.get, warnings)

    temperature = START_TEMPERATURE
    for _ in range(MAX_ITERATIONS):
        x = water_in_gas_mole_fraction(temperature, pressure)
        temperature -= (x - water_mole_fraction) * STEP
        if abs((x - water_mole_fraction) / water_mole_fraction) <= RELATIVE_TOLERANCE:
            break

    apply_checks(
        checks.derived,
        lambda name: temperature if name == "temperature" else None,
        warnings,
    )

    return BukacekWaterDewPointResult(
        temperature=from_si(temperature, "K"), warnings=tuple(warnings)
    )
