"""``process.check_valve_leak_relief`` - the check-valve leakage relief load.

As NeqSim's ``CheckValveLeakRelief.calculate`` computes it with a fluid. Spec:
``specs/models/process/check_valve_leak_relief.toml``

The Python twin of ``crates/azoth-process/src/models/check_valve_leak_relief.rs``: a
fraction of the nominal area through ``NozzleFlow.gasMassRateKgPerS``.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import Any

from azoth.core.range import apply_checks, checks_for
from azoth.core.result import CheckValveLeakReliefResult, ReliefPhase
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.eos import components as _databank
from azoth.eos.reference.pt_flash import check_composition
from azoth.process.reference._relief import (
    DEFAULT_DOWNSTREAM_PRESSURE,
    DEFAULT_TEMPERATURE,
    gas_mass_rate,
    relief_fluid_state,
)

MODEL_ID = "process.check_valve_leak_relief"

#: The class's default leak fraction.
DEFAULT_LEAK_AREA_FRACTION = 0.01
#: TR3001's recommended 0.1% minimum, which the class clamps up to.
MINIMUM_LEAK_AREA_FRACTION = 0.001
#: The class's default discharge coefficient.
DEFAULT_DISCHARGE_COEFFICIENT = 0.85


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def check_valve_leak_relief(
    components: Sequence[str],
    mole_fractions: Sequence[float],
    nominal_diameter: Q,
    upstream_pressure: Q,
    leak_area_fraction: float | None = None,
    upstream_temperature: Q | None = None,
    downstream_pressure: Q | None = None,
    discharge_coefficient: float | None = None,
    scenario_name: str | None = None,
) -> CheckValveLeakReliefResult:
    """The leakage rate through a fraction of the nominal check-valve area.

    Raises:
        InvalidInputError: for a non-positive pressure or temperature, an out-of-range
            coefficient, a negative fraction, a composition that does not sum to one, or a
            fluid the databank cannot resolve.
    """
    spec = _spec()
    warnings: list[Warning] = []
    diameter = input_to_si(spec, "nominal_diameter", nominal_diameter)
    upstream = input_to_si(spec, "upstream_pressure", upstream_pressure)
    temperature_si = (
        None
        if upstream_temperature is None
        else input_to_si(spec, "upstream_temperature", upstream_temperature)
    )
    downstream_si = (
        None
        if downstream_pressure is None
        else input_to_si(spec, "downstream_pressure", downstream_pressure)
    )
    apply_checks(
        checks_for(spec).on_input,
        {
            "nominal_diameter": diameter,
            "upstream_pressure": upstream,
            "leak_area_fraction": leak_area_fraction,
            "upstream_temperature": temperature_si,
            "downstream_pressure": downstream_si,
            "discharge_coefficient": discharge_coefficient,
        }.get,
        warnings,
    )
    z = [float(value) for value in mole_fractions]
    check_composition(z, len(components), "mole_fractions")

    temperature = DEFAULT_TEMPERATURE if temperature_si is None else temperature_si
    downstream = DEFAULT_DOWNSTREAM_PRESSURE if downstream_si is None else downstream_si
    coefficient = (
        DEFAULT_DISCHARGE_COEFFICIENT if discharge_coefficient is None else discharge_coefficient
    )

    fraction = DEFAULT_LEAK_AREA_FRACTION if leak_area_fraction is None else leak_area_fraction
    fraction_warning = None
    if fraction < MINIMUM_LEAK_AREA_FRACTION:
        # numerics-ok: CheckValveLeakRelief.java:164 - the class clamps to its recommended minimum.
        fraction = MINIMUM_LEAK_AREA_FRACTION
        fraction_warning = "Leak area fraction below TR3001 minimum 0.1%; clamped to 0.001"

    mixture, ideal_gas = _databank.mixture_of(list(components), eos="pr")
    state = relief_fluid_state(mixture, ideal_gas, temperature, upstream, z)
    assumptions = [
        f"Leak area = {fraction * 100.0:.2f}% of nominal flow area per TR3001 SR-26466",
        "Upstream pressure taken at high-pressure source MAAP",
        f"Discharge coefficient Cd = {coefficient:.2f}",
    ]
    if fraction_warning is not None:
        assumptions.append(fraction_warning)
    assumptions.extend(state.assumptions)

    if diameter <= 0.0:
        assumptions.append("Nominal check-valve diameter not set; relief rate is zero")
        rate = 0.0
    else:
        nominal_area = math.pi / 4.0 * diameter * diameter
        leak_area = fraction * nominal_area
        rate = gas_mass_rate(
            coefficient,
            leak_area,
            upstream,
            downstream,
            temperature,
            state.molar_mass,
            state.specific_heat_ratio,
        )
    return CheckValveLeakReliefResult(
        name="Check valve leakage" if scenario_name is None else scenario_name,
        relief_rate=from_si(rate, "kg/s"),
        phase=ReliefPhase.VAPOUR,
        relief_temperature=from_si(temperature, "K"),
        molar_mass=from_si(state.molar_mass, "kg/mol"),
        compressibility=state.compressibility,
        specific_heat_ratio=state.specific_heat_ratio,
        assumptions=tuple(assumptions),
        warnings=tuple(warnings),
    )
