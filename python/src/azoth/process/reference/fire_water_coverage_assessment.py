"""``process.fire_water_coverage_assessment`` - the fire-water coverage gate.

As NeqSim's ``FireWaterCoverageAssessment`` computes it. Spec:
``specs/models/process/fire_water_coverage_assessment.toml``

The Python twin of ``crates/azoth-process/src/models/fire_water_coverage_assessment.rs``:
the coverage, flow and pressure questions and the class's verdict.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import FireWaterCoverageAssessmentResult, SupplyVerdict
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.fire_water_coverage_assessment"


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    gap: float
    fraction: float
    deficit: float
    flow_ok: bool
    pressure_ok: bool
    verdict: SupplyVerdict


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
    protected_area: Q,
    required_flow: Q,
    spare_supply_flow: Q,
    worst_case_pressure_margin: Q,
    covered_area: Q | None = None,
    existing_delivered_flow: Q | None = None,
    required_pressure_margin: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    area = input_to_si(spec, "protected_area", protected_area)
    required = input_to_si(spec, "required_flow", required_flow)
    spare = input_to_si(spec, "spare_supply_flow", spare_supply_flow)
    worst = input_to_si(spec, "worst_case_pressure_margin", worst_case_pressure_margin)
    stated_covered = (
        None if covered_area is None else input_to_si(spec, "covered_area", covered_area)
    )
    stated_existing = (
        None
        if existing_delivered_flow is None
        else input_to_si(spec, "existing_delivered_flow", existing_delivered_flow)
    )
    stated_margin = (
        None
        if required_pressure_margin is None
        else input_to_si(spec, "required_pressure_margin", required_pressure_margin)
    )
    apply_checks(
        checks.on_input,
        {
            "protected_area": area,
            "required_flow": required,
            "covered_area": stated_covered,
            "existing_delivered_flow": stated_existing,
            "required_pressure_margin": stated_margin,
        }.get,
        warnings,
    )

    _require_finite("protected_area", area)
    _require_finite("required_flow", required)
    _require_finite("spare_supply_flow", spare)
    _require_finite("worst_case_pressure_margin", worst)
    covered = 0.0 if stated_covered is None else stated_covered
    _require_finite("covered_area", covered)
    existing = 0.0 if stated_existing is None else stated_existing
    _require_finite("existing_delivered_flow", existing)
    margin = 0.0 if stated_margin is None else stated_margin
    _require_finite("required_pressure_margin", margin)

    gap = max(0.0, area - covered)
    fraction = min(1.0, covered / area)
    deficit = max(0.0, required - existing)
    flow_ok = spare >= deficit
    pressure_ok = worst >= margin
    any_gap = gap > 0.0 or deficit > 0.0
    if not any_gap:
        verdict = SupplyVerdict.COMPLIANT
    elif not flow_ok and not pressure_ok:
        verdict = SupplyVerdict.DEDICATED_SUPPLY_REQUIRED
    elif not pressure_ok:
        verdict = SupplyVerdict.SUPPLY_PRESSURE_DEFICIT
    elif not flow_ok:
        verdict = SupplyVerdict.SUPPLY_FLOW_DEFICIT
    else:
        verdict = SupplyVerdict.COVERAGE_GAP_SUPPLY_ADEQUATE

    return (
        _Solution(
            gap=gap,
            fraction=fraction,
            deficit=deficit,
            flow_ok=flow_ok,
            pressure_ok=pressure_ok,
            verdict=verdict,
        ),
        warnings,
    )


def fire_water_coverage_assessment(
    protected_area: Q,
    required_flow: Q,
    spare_supply_flow: Q,
    worst_case_pressure_margin: Q,
    covered_area: Q | None = None,
    existing_delivered_flow: Q | None = None,
    required_pressure_margin: Q | None = None,
) -> FireWaterCoverageAssessmentResult:
    """The coverage gap, the flow deficit and the class's verdict.

    Args:
        protected_area: the protected area.
        required_flow: the area's required flow.
        spare_supply_flow: the supply available, which may be negative.
        worst_case_pressure_margin: the serving section's margin, which may be negative.
        covered_area: the area already protected; absent is 0.
        existing_delivered_flow: what the existing system delivers; absent is 0.
        required_pressure_margin: the margin the new demand needs; absent is 0.

    Returns:
        The gap, the fraction, the deficit, the two feasibility flags and the verdict.

    Raises:
        InvalidInputError: for a non-positive area or required flow, a negative covered
            area, delivered flow or required margin, or a non-finite input.
    """
    solution, warnings = _evaluate(
        protected_area,
        required_flow,
        spare_supply_flow,
        worst_case_pressure_margin,
        covered_area,
        existing_delivered_flow,
        required_pressure_margin,
    )
    return FireWaterCoverageAssessmentResult(
        coverage_gap=from_si(solution.gap, "m**2"),
        coverage_fraction=solution.fraction,
        flow_deficit=from_si(solution.deficit, "L/min"),
        flow_feasible=solution.flow_ok,
        pressure_feasible=solution.pressure_ok,
        verdict=solution.verdict,
        warnings=tuple(warnings),
    )
