"""``process.control_valve_failure_relief`` - the fail-open control-valve relief load.

As NeqSim's ``ControlValveFailureRelief.calculate`` computes it with a fluid. Spec:
``specs/models/process/control_valve_failure_relief.toml``

The Python twin of ``crates/azoth-process/src/models/control_valve_failure_relief.rs``: the
effective area (explicit or from Cv) through ``NozzleFlow.gasMassRateKgPerS``.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import Any

from azoth.core.range import apply_checks, checks_for
from azoth.core.result import ControlValveFailureReliefResult, ReliefPhase
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

MODEL_ID = "process.control_valve_failure_relief"

#: The class's ``CV_TO_AREA_CONSTANT``, ``Cv = 24.6 Cd A_in2``.
CV_TO_AREA_CONSTANT = 24.6
#: The class's square inches per square metre.
IN2_PER_M2 = 1550.0031
#: The class's default discharge coefficient.
DEFAULT_DISCHARGE_COEFFICIENT = 0.85
#: The class's own assumption sentence.
BASIS = "Fail-open control valve, upstream at maximum operating pressure per TR3001 SR-26501"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def control_valve_failure_relief(
    components: Sequence[str],
    mole_fractions: Sequence[float],
    upstream_pressure: Q,
    upstream_temperature: Q | None = None,
    downstream_pressure: Q | None = None,
    effective_area: Q | None = None,
    cv: float | None = None,
    cv_erosion_margin: float | None = None,
    discharge_coefficient: float | None = None,
    scenario_name: str | None = None,
) -> ControlValveFailureReliefResult:
    """The fail-open rate from an explicit area or a Cv with an erosion margin.

    Raises:
        InvalidInputError: for a non-positive pressure or temperature, an out-of-range
            coefficient, a composition that does not sum to one, or a fluid the databank
            cannot resolve.
    """
    spec = _spec()
    warnings: list[Warning] = []
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
    area_si = (
        None if effective_area is None else input_to_si(spec, "effective_area", effective_area)
    )
    apply_checks(
        checks_for(spec).on_input,
        {
            "upstream_pressure": upstream,
            "upstream_temperature": temperature_si,
            "downstream_pressure": downstream_si,
            "effective_area": area_si,
            "cv": cv,
            "cv_erosion_margin": cv_erosion_margin,
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
    margin = 1.0 if cv_erosion_margin is None else cv_erosion_margin

    mixture, ideal_gas = _databank.mixture_of(list(components), eos="pr")
    state = relief_fluid_state(mixture, ideal_gas, temperature, upstream, z)
    assumptions = [BASIS, f"Discharge coefficient Cd = {coefficient:.2f}"]
    assumptions.extend(state.assumptions)

    area = math.nan if area_si is None else area_si
    if not math.isfinite(area) or area <= 0.0:
        if cv is not None and cv > 0.0:
            area = (cv * margin / (CV_TO_AREA_CONSTANT * coefficient)) / IN2_PER_M2
            if margin > 1.0:
                assumptions.append(
                    f"Cv increased by erosion/vibration margin {margin:.2f} per TR3001 SR-26501"
                )
            assumptions.append(
                "Effective area derived from Cv with screening relation Cv = 24.6 Cd A_in2"
            )
        else:
            assumptions.append("Neither effective area nor Cv set; relief rate is zero")
            area = 0.0

    rate = (
        gas_mass_rate(
            coefficient,
            area,
            upstream,
            downstream,
            temperature,
            state.molar_mass,
            state.specific_heat_ratio,
        )
        if area > 0.0
        else 0.0
    )
    return ControlValveFailureReliefResult(
        name="Control valve failure" if scenario_name is None else scenario_name,
        relief_rate=from_si(rate, "kg/s"),
        phase=ReliefPhase.VAPOUR,
        relief_temperature=from_si(temperature, "K"),
        molar_mass=from_si(state.molar_mass, "kg/mol"),
        compressibility=state.compressibility,
        specific_heat_ratio=state.specific_heat_ratio,
        assumptions=tuple(assumptions),
        warnings=tuple(warnings),
    )
