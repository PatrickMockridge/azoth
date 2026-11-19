"""``process.psv_valve`` - one step of the API 520 pressure-safety-valve latch.

As NeqSim's ``PsvValveModel.update`` and ``massFlowKgPerS`` compute it. Spec:
``specs/models/process/psv_valve.toml``

The Python twin of ``crates/azoth-process/src/models/psv_valve.rs``: the open/reseat
hysteresis, the pop count and the choked/subcritical orifice relation.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import PsvValveResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.psv_valve"

#: The class's gas constant, J/(mol K).
GAS_CONSTANT = 8.314


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    open: bool
    cycle_count: float
    mass_flow_kg_s: float


def _require_finite(name: str, value: float) -> None:
    """Refuse a value the arithmetic cannot carry to a published number."""
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    set_pressure: Q,
    blowdown_fraction: float,
    discharge_coefficient: float,
    orifice_area: Q,
    back_pressure: Q,
    upstream_pressure: Q,
    upstream_temperature: Q,
    molar_mass: Q,
    heat_capacity_ratio: float,
    compressibility: float,
    was_open: bool | None = None,
    prior_cycle_count: float | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []

    set_pressure_si = input_to_si(spec, "set_pressure", set_pressure)
    blowdown = float(blowdown_fraction)
    coefficient = float(discharge_coefficient)
    area = input_to_si(spec, "orifice_area", orifice_area)
    back = input_to_si(spec, "back_pressure", back_pressure)
    upstream = input_to_si(spec, "upstream_pressure", upstream_pressure)
    temperature = input_to_si(spec, "upstream_temperature", upstream_temperature)
    molar_mass_si = input_to_si(spec, "molar_mass", molar_mass)
    gamma = float(heat_capacity_ratio)
    z = float(compressibility)
    prior = 0.0 if prior_cycle_count is None else float(prior_cycle_count)

    apply_checks(
        checks_for(spec).on_input,
        {
            "set_pressure": set_pressure_si,
            "blowdown_fraction": blowdown,
            "discharge_coefficient": coefficient,
            "orifice_area": area,
            "back_pressure": back,
            "upstream_pressure": upstream,
            "upstream_temperature": temperature,
            "molar_mass": molar_mass_si,
            "heat_capacity_ratio": gamma,
            "compressibility": z,
            "prior_cycle_count": prior,
        }.get,
        warnings,
    )
    for name, value in (
        ("set_pressure", set_pressure_si),
        ("blowdown_fraction", blowdown),
        ("discharge_coefficient", coefficient),
        ("orifice_area", area),
        ("back_pressure", back),
        ("upstream_pressure", upstream),
        ("upstream_temperature", temperature),
        ("molar_mass", molar_mass_si),
        ("heat_capacity_ratio", gamma),
        ("compressibility", z),
        ("prior_cycle_count", prior),
    ):
        _require_finite(name, value)
    if prior != math.floor(prior):
        raise InvalidInputError(
            "prior_cycle_count",
            "the class's pop counter is a whole number, and a step cannot continue from a fraction",
        )

    if back < 0.0:
        raise InvalidInputError("back_pressure", "the class refuses a negative back pressure")
    if set_pressure_si <= back:
        raise InvalidInputError(
            "set_pressure", "the class requires the set pressure to exceed the back pressure"
        )
    if blowdown <= 0.0 or blowdown >= 1.0:
        raise InvalidInputError("blowdown_fraction", "the class refuses a fraction outside (0, 1)")
    if coefficient <= 0.0:
        raise InvalidInputError(
            "discharge_coefficient", "the class refuses a non-positive discharge coefficient"
        )
    if area <= 0.0:
        raise InvalidInputError("orifice_area", "the class refuses a non-positive orifice area")
    if upstream <= 0.0 or temperature <= 0.0:
        raise InvalidInputError(
            "upstream_pressure",
            "the class requires the upstream pressure and temperature to be positive",
        )
    if molar_mass_si <= 0.0:
        raise InvalidInputError("molar_mass", "the class refuses a non-positive molar mass")
    if gamma <= 1.0:
        raise InvalidInputError(
            "heat_capacity_ratio", "the class refuses an isentropic exponent at or below one"
        )
    if z <= 0.0:
        raise InvalidInputError(
            "compressibility", "the class refuses a non-positive compressibility"
        )

    reseat = set_pressure_si * (1.0 - blowdown)
    is_open = bool(was_open)
    cycles = prior
    if not is_open and upstream >= set_pressure_si:
        is_open = True
        cycles += 1.0
    elif is_open and upstream <= reseat:
        is_open = False

    mass_flow = 0.0
    if is_open:
        critical_ratio = math.pow(2.0 / (gamma + 1.0), gamma / (gamma - 1.0))
        pressure_ratio = back / upstream
        if pressure_ratio <= critical_ratio:
            mass_flow = (
                coefficient
                * area
                * upstream
                * math.sqrt(gamma * molar_mass_si / (z * GAS_CONSTANT * temperature))
                * math.pow(2.0 / (gamma + 1.0), (gamma + 1.0) / (2.0 * (gamma - 1.0)))
            )
        else:
            term = (2.0 * gamma / (gamma - 1.0)) * (
                math.pow(pressure_ratio, 2.0 / gamma)
                - math.pow(pressure_ratio, (gamma + 1.0) / gamma)
            )
            if term < 0.0:
                term = 0.0
            mass_flow = (
                coefficient
                * area
                * upstream
                * math.sqrt(molar_mass_si / (z * GAS_CONSTANT * temperature))
                * math.sqrt(term)
            )
        if mass_flow < 0.0 or math.isnan(mass_flow):
            mass_flow = 0.0

    return _Solution(open=is_open, cycle_count=cycles, mass_flow_kg_s=mass_flow), warnings


def psv_valve(
    set_pressure: Q,
    blowdown_fraction: float,
    discharge_coefficient: float,
    orifice_area: Q,
    back_pressure: Q,
    upstream_pressure: Q,
    upstream_temperature: Q,
    molar_mass: Q,
    heat_capacity_ratio: float,
    compressibility: float,
    was_open: bool | None = None,
    prior_cycle_count: float | None = None,
) -> PsvValveResult:
    """One step of the PSV latch and its relieving flow.

    The valve opens when closed and the upstream pressure reaches the set pressure,
    increments its pop count only on the closed-to-open transition, and closes only at or
    below the reseat pressure ``set (1 - blowdown)``. The flow uses the API 520
    compressible-orifice relation and is zero when closed.

    Args:
        set_pressure: the set (pop) pressure.
        blowdown_fraction: the blowdown fraction in ``(0, 1)``.
        discharge_coefficient: the orifice discharge coefficient.
        orifice_area: the effective orifice flow area.
        back_pressure: the downstream back pressure.
        upstream_pressure: the upstream pressure at this step.
        upstream_temperature: the upstream temperature at this step.
        molar_mass: the relieving fluid's molar mass.
        heat_capacity_ratio: the isentropic exponent ``Cp/Cv``.
        compressibility: the compressibility factor ``Z``.
        was_open: the open state before this step; absent is false.
        prior_cycle_count: the pop count before this step; absent is 0.

    Returns:
        The open state, pop count and relieving flow.

    Raises:
        InvalidInputError: for a guard the class applies or a non-integral pop count.
    """
    solution, warnings = _evaluate(
        set_pressure,
        blowdown_fraction,
        discharge_coefficient,
        orifice_area,
        back_pressure,
        upstream_pressure,
        upstream_temperature,
        molar_mass,
        heat_capacity_ratio,
        compressibility,
        was_open,
        prior_cycle_count,
    )
    return PsvValveResult(
        open=solution.open,
        cycle_count=solution.cycle_count,
        mass_flow=from_si(solution.mass_flow_kg_s, "kg/s"),
        warnings=tuple(warnings),
    )
