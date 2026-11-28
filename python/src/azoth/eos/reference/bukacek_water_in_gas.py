"""``eos.bukacek_water_in_gas`` - the Bukacek water content of a gas.

Spec: ``specs/calcs/eos/bukacek_water_in_gas.toml``, which records the bar convention,
the fixed gas molar mass and the standard cubic metre.
"""

from __future__ import annotations

import math

from azoth._registry_gen import spec as _spec_for
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import BukacekWaterInGasResult
from azoth.core.units import input_to_si
from azoth.core.warnings import Warning

CALC_ID = "eos.bukacek_water_in_gas"

# BukacekWaterInGas.getWaterInGas's constants.
A = (-7.85823, 1.83991, -11.7811, 22.67, -15.9393, 1.77516)
TC_WATER = 667.14
PC_WATER_BAR = 220.64
PA_PER_BAR = 100_000.0


def water_in_gas_mole_fraction(temperature: float, pressure: float) -> float:
    """The water mole fraction, without the spec checks. ``pressure`` is in Pa."""
    tau = (TC_WATER - temperature) / TC_WATER
    exponent = (
        A[0] * tau
        + A[1] * tau**1.5
        + A[2] * tau**3
        + A[3] * tau**3.5
        + A[4] * tau**4
        + A[5] * tau**7.5
    ) / (1.0 - tau)
    psw_bar = PC_WATER_BAR * math.exp(exponent)
    mg_per_sm3 = 761_900.42 * psw_bar / (pressure / PA_PER_BAR) + 16.016 * math.pow(
        10.0, -1716.26 / temperature + 6.69449
    )
    molgas_sm3 = 101_325.0 / (8.3144621 * 288.15)
    return (mg_per_sm3 / 16_800.0) / molgas_sm3


def bukacek_water_in_gas(temperature: float, pressure: float) -> BukacekWaterInGasResult:
    """The Bukacek water content of a gas, as a mole fraction.

    Args:
        temperature: the gas temperature, K.
        pressure: the total pressure, Pa.

    Returns:
        The water mole fraction at the water dew point.

    Raises:
        OutOfRangeError: if ``temperature`` or ``pressure`` is not positive.

    Example:
        >>> round(bukacek_water_in_gas(288.15, 100_000.0).water_mole_fraction, 15)
        0.010403914183672
    """
    spec = _spec_for(CALC_ID)
    checks = checks_for(spec)
    warnings: list[Warning] = []

    temperature = input_to_si(spec, "temperature", temperature)
    pressure = input_to_si(spec, "pressure", pressure)
    values = {"temperature": temperature, "pressure": pressure}
    apply_checks(checks.on_input, values.get, warnings)

    water_mole_fraction = water_in_gas_mole_fraction(temperature, pressure)

    apply_checks(
        checks.derived,
        lambda name: water_mole_fraction if name == "water_mole_fraction" else None,
        warnings,
    )

    return BukacekWaterInGasResult(
        water_mole_fraction=water_mole_fraction, warnings=tuple(warnings)
    )
