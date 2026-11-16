"""``process.tube_rupture_relief`` - the heat-exchanger tube-rupture relief load.

As NeqSim's ``TubeRuptureRelief.calculate`` computes it with a fluid. Spec:
``specs/models/process/tube_rupture_relief.toml``

The Python twin of ``crates/azoth-process/src/models/tube_rupture_relief.rs``: the open
ends' bore through ``NozzleFlow.gasMassRateKgPerS``.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import Any

from azoth.core.range import apply_checks, checks_for
from azoth.core.result import ReliefPhase, TubeRuptureReliefResult
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

MODEL_ID = "process.tube_rupture_relief"

#: The class's default open-end count.
DEFAULT_OPEN_ENDS = 2.0
#: The class's default discharge coefficient.
DEFAULT_DISCHARGE_COEFFICIENT = 0.85


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def tube_rupture_relief(
    components: Sequence[str],
    mole_fractions: Sequence[float],
    tube_inner_diameter: Q,
    high_pressure: Q,
    number_of_open_ends: float | None = None,
    high_temperature: Q | None = None,
    low_pressure: Q | None = None,
    discharge_coefficient: float | None = None,
    scenario_name: str | None = None,
) -> TubeRuptureReliefResult:
    """The full-bore tube-rupture rate through one or two open ends.

    Raises:
        InvalidInputError: for a non-positive pressure or temperature, an out-of-range
            coefficient, a count below one, a composition that does not sum to one, or a
            fluid the databank cannot resolve.
    """
    spec = _spec()
    warnings: list[Warning] = []
    diameter = input_to_si(spec, "tube_inner_diameter", tube_inner_diameter)
    high = input_to_si(spec, "high_pressure", high_pressure)
    temperature_si = (
        None
        if high_temperature is None
        else input_to_si(spec, "high_temperature", high_temperature)
    )
    low_si = None if low_pressure is None else input_to_si(spec, "low_pressure", low_pressure)
    apply_checks(
        checks_for(spec).on_input,
        {
            "tube_inner_diameter": diameter,
            "high_pressure": high,
            "number_of_open_ends": number_of_open_ends,
            "high_temperature": temperature_si,
            "low_pressure": low_si,
            "discharge_coefficient": discharge_coefficient,
        }.get,
        warnings,
    )
    z = [float(value) for value in mole_fractions]
    check_composition(z, len(components), "mole_fractions")

    temperature = DEFAULT_TEMPERATURE if temperature_si is None else temperature_si
    low = DEFAULT_DOWNSTREAM_PRESSURE if low_si is None else low_si
    coefficient = (
        DEFAULT_DISCHARGE_COEFFICIENT if discharge_coefficient is None else discharge_coefficient
    )
    ends = DEFAULT_OPEN_ENDS if number_of_open_ends is None else number_of_open_ends

    mixture, ideal_gas = _databank.mixture_of(list(components), eos="pr")
    state = relief_fluid_state(mixture, ideal_gas, temperature, high, z)
    assumptions = [
        f"{ends:.0f} open ends (full-bore single-tube rupture) per TR3001 SR-26616",
        "High-pressure fluid at maximum operating pressure",
        "Vaporization/flashing on the low-pressure side not modelled (screening estimate)",
        f"Discharge coefficient Cd = {coefficient:.2f}",
    ]
    assumptions.extend(state.assumptions)

    if diameter <= 0.0:
        assumptions.append("Tube inner diameter not set; relief rate is zero")
        rate = 0.0
    else:
        tube_area = math.pi / 4.0 * diameter * diameter
        area = ends * tube_area
        rate = gas_mass_rate(
            coefficient,
            area,
            high,
            low,
            temperature,
            state.molar_mass,
            state.specific_heat_ratio,
        )
    return TubeRuptureReliefResult(
        name="Tube rupture" if scenario_name is None else scenario_name,
        relief_rate=from_si(rate, "kg/s"),
        phase=ReliefPhase.VAPOUR,
        relief_temperature=from_si(temperature, "K"),
        molar_mass=from_si(state.molar_mass, "kg/mol"),
        compressibility=state.compressibility,
        specific_heat_ratio=state.specific_heat_ratio,
        assumptions=tuple(assumptions),
        warnings=tuple(warnings),
    )
