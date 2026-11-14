"""``process.active_fire_protection_screening`` - the protection hierarchy screen.

As NeqSim's ``ActiveFireProtectionScreening`` computes it. Spec:
``specs/models/process/active_fire_protection_screening.toml``

The Python twin of
``crates/azoth-process/src/models/active_fire_protection_screening.rs``: the extinguishing
classification, the blowdown margin and the selective saving.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import ActiveFireProtectionScreeningResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.active_fire_protection_screening"

#: The class's two extinguishing scenarios.
_EXTINGUISHING = ("liquid_pool_fire", "ordinary_combustible_fire")
#: The class's four scenarios.
_SCENARIOS = (
    "gas_jet_fire",
    "two_phase_jet_fire",
    "liquid_pool_fire",
    "ordinary_combustible_fire",
)


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    extinguishing: bool
    faster: bool
    margin: float
    saving: float


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
    scenario: str,
    blowdown_time_to_target: Q,
    bare_steel_time_to_critical: Q,
    dedicated_object_demand: Q,
    general_area_demand: Q,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    blowdown = input_to_si(spec, "blowdown_time_to_target", blowdown_time_to_target)
    bare = input_to_si(spec, "bare_steel_time_to_critical", bare_steel_time_to_critical)
    dedicated = input_to_si(spec, "dedicated_object_demand", dedicated_object_demand)
    general = input_to_si(spec, "general_area_demand", general_area_demand)
    apply_checks(
        checks.on_input,
        {
            "blowdown_time_to_target": blowdown,
            "bare_steel_time_to_critical": bare,
            "dedicated_object_demand": dedicated,
            "general_area_demand": general,
        }.get,
        warnings,
    )

    if scenario not in _SCENARIOS:
        raise InvalidInputError(
            "scenario",
            f"the scenario is one of the class's four fire types, and {scenario!r} is not",
        )
    _require_finite("blowdown_time_to_target", blowdown)
    _require_finite("bare_steel_time_to_critical", bare)
    _require_finite("dedicated_object_demand", dedicated)
    _require_finite("general_area_demand", general)

    extinguishing = scenario in _EXTINGUISHING
    faster = blowdown < bare
    margin = bare - blowdown
    saving = 1.0 - dedicated / general

    return (
        _Solution(
            extinguishing=extinguishing,
            faster=faster,
            margin=margin,
            saving=saving,
        ),
        warnings,
    )


def active_fire_protection_screening(
    scenario: str,
    blowdown_time_to_target: Q,
    bare_steel_time_to_critical: Q,
    dedicated_object_demand: Q,
    general_area_demand: Q,
) -> ActiveFireProtectionScreeningResult:
    """The extinguishing classification, the blowdown margin and the selective saving.

    Args:
        scenario: one of the class's four fire types.
        blowdown_time_to_target: the blowdown time to its target.
        bare_steel_time_to_critical: the bare steel's time to critical.
        dedicated_object_demand: the dedicated object's fire-water demand.
        general_area_demand: the area's fire-water demand.

    Returns:
        Whether water extinguishes the fire, whether blowdown is faster than the steel, the
        margin and the selective protection saving.

    Raises:
        InvalidInputError: for a non-positive blowdown or time to critical, a negative
            dedicated demand, a non-positive general demand, an unknown scenario, or a
            non-finite input.
    """
    solution, warnings = _evaluate(
        scenario,
        blowdown_time_to_target,
        bare_steel_time_to_critical,
        dedicated_object_demand,
        general_area_demand,
    )
    return ActiveFireProtectionScreeningResult(
        water_extinguishing=solution.extinguishing,
        blowdown_faster_than_failure=solution.faster,
        blowdown_margin=from_si(solution.margin, "s"),
        selective_protection_saving_fraction=solution.saving,
        warnings=tuple(warnings),
    )
