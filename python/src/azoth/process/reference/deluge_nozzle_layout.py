"""``process.deluge_nozzle_layout`` - the deluge nozzle net screen.

As NeqSim's ``DelugeNozzleLayout`` computes it. Spec:
``specs/models/process/deluge_nozzle_layout.toml``

The Python twin of ``crates/azoth-process/src/models/deluge_nozzle_layout.rs``: the flow
and coverage criteria, the larger count, the grid and the delivered density.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import DelugeNozzleLayoutResult, GoverningCriterion
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.deluge_nozzle_layout"

#: The class's default operating pressure, Pa.
_DEFAULT_OPERATING_PRESSURE_PA = 5.0e5
#: The class's default coverage efficiency.
_DEFAULT_COVERAGE_EFFICIENCY = 1.0


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    required_flow: float
    flow_per_nozzle: float
    coverage_per_nozzle: float
    count_from_flow: float
    count_from_coverage: float
    count: float
    governing: GoverningCriterion
    grid: float
    delivered_flow: float
    delivered_density: float
    pressure_adequate: bool
    density_met: bool
    clearance_adequate: bool


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
    required_density: float,
    nozzle_k: float,
    nozzle_min_pressure: Q,
    nozzle_max_spacing: Q,
    nozzle_clearance: Q,
    available_clearance: Q,
    operating_pressure: Q | None = None,
    coverage_efficiency: float | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    area = input_to_si(spec, "protected_area", protected_area)
    density = float(required_density)
    k = float(nozzle_k)
    min_pressure = input_to_si(spec, "nozzle_min_pressure", nozzle_min_pressure)
    spacing = input_to_si(spec, "nozzle_max_spacing", nozzle_max_spacing)
    clearance = input_to_si(spec, "nozzle_clearance", nozzle_clearance)
    available = input_to_si(spec, "available_clearance", available_clearance)
    stated_pressure = (
        None
        if operating_pressure is None
        else input_to_si(spec, "operating_pressure", operating_pressure)
    )
    stated_efficiency = None if coverage_efficiency is None else float(coverage_efficiency)
    apply_checks(
        checks.on_input,
        {
            "protected_area": area,
            "required_density": density,
            "nozzle_k": k,
            "nozzle_min_pressure": min_pressure,
            "nozzle_max_spacing": spacing,
            "nozzle_clearance": clearance,
            "available_clearance": available,
            "operating_pressure": stated_pressure,
            "coverage_efficiency": stated_efficiency,
        }.get,
        warnings,
    )

    _require_finite("protected_area", area)
    _require_finite("required_density", density)
    _require_finite("nozzle_k", k)
    _require_finite("nozzle_min_pressure", min_pressure)
    _require_finite("nozzle_max_spacing", spacing)
    _require_finite("nozzle_clearance", clearance)
    _require_finite("available_clearance", available)
    pressure = _DEFAULT_OPERATING_PRESSURE_PA if stated_pressure is None else stated_pressure
    _require_finite("operating_pressure", pressure)
    efficiency = (
        _DEFAULT_COVERAGE_EFFICIENCY if stated_efficiency is None else stated_efficiency
    )
    _require_finite("coverage_efficiency", efficiency)

    # **The orifice law is in bar**: `input_to_si` gives pascals, and the K factor is
    # `(L/min)/sqrt(bar)`.
    required_flow = area * density
    flow_per_nozzle = k * math.sqrt(pressure / 1.0e5)
    coverage_per_nozzle = spacing * spacing * efficiency
    count_from_flow = math.ceil(required_flow / flow_per_nozzle)
    count_from_coverage = math.ceil(area / coverage_per_nozzle)
    count = max(count_from_flow, count_from_coverage)
    governing = (
        GoverningCriterion.COVERAGE
        if count_from_coverage > count_from_flow
        else GoverningCriterion.FLOW
    )
    grid = math.sqrt(area / count)
    delivered_flow = count * flow_per_nozzle
    delivered_density = delivered_flow / area

    return (
        _Solution(
            # The class works in L/min; the solution is SI, and `from_si` re-expresses it.
            required_flow=required_flow / 60000.0,
            flow_per_nozzle=flow_per_nozzle / 60000.0,
            coverage_per_nozzle=coverage_per_nozzle,
            count_from_flow=float(count_from_flow),
            count_from_coverage=float(count_from_coverage),
            count=float(count),
            governing=governing,
            grid=grid,
            delivered_flow=delivered_flow / 60000.0,
            delivered_density=delivered_density,
            pressure_adequate=pressure >= min_pressure,
            density_met=delivered_density >= density,
            clearance_adequate=available >= clearance,
        ),
        warnings,
    )


def deluge_nozzle_layout(
    protected_area: Q,
    required_density: float,
    nozzle_k: float,
    nozzle_min_pressure: Q,
    nozzle_max_spacing: Q,
    nozzle_clearance: Q,
    available_clearance: Q,
    operating_pressure: Q | None = None,
    coverage_efficiency: float | None = None,
) -> DelugeNozzleLayoutResult:
    """The two criteria, the larger count, the grid and the delivered density.

    Args:
        protected_area: the protected area.
        required_density: the required application density in ``L/(min m2)``.
        nozzle_k: the nozzle's discharge coefficient in ``(L/min)/sqrt(bar)``.
        nozzle_min_pressure: the nozzle's minimum operating pressure.
        nozzle_max_spacing: the nozzle's maximum spacing.
        nozzle_clearance: the clear depth the nozzle needs.
        available_clearance: the clear depth available.
        operating_pressure: the header pressure; absent is 5 bar.
        coverage_efficiency: the wetted fraction of the coverage square; absent is 1.

    Returns:
        The flows, the two counts, the governing criterion, the grid and the deliverable.

    Raises:
        InvalidInputError: for a non-positive area, density, coefficient, minimum pressure,
            spacing or operating pressure, an efficiency outside ``(0, 1]``, a negative
            clearance, or a non-finite input.
    """
    solution, warnings = _evaluate(
        protected_area,
        required_density,
        nozzle_k,
        nozzle_min_pressure,
        nozzle_max_spacing,
        nozzle_clearance,
        available_clearance,
        operating_pressure,
        coverage_efficiency,
    )
    return DelugeNozzleLayoutResult(
        required_flow=from_si(solution.required_flow, "L/min"),
        flow_per_nozzle=from_si(solution.flow_per_nozzle, "L/min"),
        coverage_per_nozzle=from_si(solution.coverage_per_nozzle, "m**2"),
        nozzle_count_from_flow=solution.count_from_flow,
        nozzle_count_from_coverage=solution.count_from_coverage,
        nozzle_count=solution.count,
        governing_criterion=solution.governing,
        grid_spacing=from_si(solution.grid, "m"),
        delivered_flow=from_si(solution.delivered_flow, "L/min"),
        delivered_density=solution.delivered_density,
        pressure_adequate=solution.pressure_adequate,
        density_met=solution.density_met,
        clearance_adequate=solution.clearance_adequate,
        warnings=tuple(warnings),
    )
