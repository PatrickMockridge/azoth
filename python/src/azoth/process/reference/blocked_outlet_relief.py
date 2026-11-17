"""``process.blocked_outlet_relief`` - the blocked-outlet relief load.

As NeqSim's ``BlockedOutletRelief.calculate`` computes it with a fluid. Spec:
``specs/models/process/blocked_outlet_relief.toml``

The Python twin of ``crates/azoth-process/src/models/blocked_outlet_relief.rs``: the relief
rate is the full inflow, and the relieving state is ``ReliefFluidState.evaluate``.
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

from azoth.core.range import apply_checks, checks_for
from azoth.core.result import BlockedOutletReliefResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.eos import components as _databank
from azoth.eos.reference.pt_flash import check_composition
from azoth.process.reference._relief import relief_fluid_state

MODEL_ID = "process.blocked_outlet_relief"

#: The class's own assumption sentence.
BASIS = "Relief rate set equal to full inflow per TR3001 SR-26460"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def blocked_outlet_relief(
    components: Sequence[str],
    mole_fractions: Sequence[float],
    inflow_rate: Q,
    relief_pressure: Q,
    relief_temperature: Q,
    scenario_name: str | None = None,
) -> BlockedOutletReliefResult:
    """The full-inflow relief rate and the relieving-fluid state at it.

    Raises:
        InvalidInputError: for a negative inflow, a non-positive pressure or temperature, a
            composition that does not sum to one, or a fluid the databank cannot resolve.
    """
    spec = _spec()
    warnings: list[Warning] = []
    inflow = input_to_si(spec, "inflow_rate", inflow_rate)
    pressure = input_to_si(spec, "relief_pressure", relief_pressure)
    temperature = input_to_si(spec, "relief_temperature", relief_temperature)
    apply_checks(
        checks_for(spec).on_input,
        {
            "inflow_rate": inflow,
            "relief_pressure": pressure,
            "relief_temperature": temperature,
        }.get,
        warnings,
    )
    z = [float(value) for value in mole_fractions]
    check_composition(z, len(components), "mole_fractions")

    mixture, ideal_gas = _databank.mixture_of(list(components), eos="pr")
    state = relief_fluid_state(mixture, ideal_gas, temperature, pressure, z)
    assumptions = [BASIS]
    if inflow <= 0.0:
        assumptions.append("Inflow rate not set or non-positive; relief rate is zero")
    assumptions.extend(state.assumptions)
    return BlockedOutletReliefResult(
        name="Blocked outlet" if scenario_name is None else scenario_name,
        relief_rate=from_si(inflow, "kg/s"),
        phase=state.phase,
        relief_temperature=from_si(temperature, "K"),
        molar_mass=from_si(state.molar_mass, "kg/mol"),
        compressibility=state.compressibility,
        specific_heat_ratio=state.specific_heat_ratio,
        assumptions=tuple(assumptions),
        warnings=tuple(warnings),
    )
