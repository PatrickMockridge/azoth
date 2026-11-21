"""``process.topsides_facility_cost`` - the topsides facility stack.

The Python twin of ``crates/azoth-process/src/models/topsides_facility_cost.rs``:
the module classification, the bulk and integration factors, the project stack
and the material-take-off, from the shared ``_costing`` estimate.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import (
    TopsidesBreakdownLine,
    TopsidesCapitalLine,
    TopsidesFacilityCostResult,
    TopsidesFlag,
    TopsidesMtoRow,
    TopsidesProjectLine,
    TopsidesProjectSummaryLine,
    TopsidesSummaryLine,
    TopsidesWeightLine,
)
from azoth.core.units import input_records_to_si, quantity
from azoth.core.warnings import Warning
from azoth.process.reference import _costing

MODEL_ID = "process.topsides_facility_cost"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _finite(name: str, value: float) -> float:
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )
    return value


def _number(name: str, value: Any, default: float) -> float:
    return _finite(name, default if value is None else float(value))


def _category_of(item: str, source: str) -> str:
    if source == "topsides-allowance":
        if "Structural" in item:
            return "structural"
        if "Piping" in item:
            return "piping"
        if "Electrical" in item:
            return "electrical"
        if "HVAC" in item:
            return "safety-systems"
    return "equipment"


def topsides_facility_cost(
    equipment: list[dict[str, Any]],
    facility_type: str | None = None,
    project_context: str | None = None,
    location_factor: float | None = None,
    piping_bulk_factor: float | None = None,
    electrical_instrumentation_factor: float | None = None,
    insulation_paint_fireproofing_factor: float | None = None,
    hvac_telecom_safety_factor: float | None = None,
    engineering_procurement_factor: float | None = None,
    construction_management_factor: float | None = None,
    project_contingency_factor: float | None = None,
    owner_cost_factor: float | None = None,
    include_utilities_and_offsites: bool | None = None,
    estimate_class: float | None = None,
) -> TopsidesFacilityCostResult:
    """The topsides facility estimate over the stated process rows.

    Args:
        equipment: one unit's name, process type, total module cost and weight.
        facility_type: the facility type, whose five factors are the class's table.
        project_context: the project context, whose congestion factor applies.
        location_factor: the factor applied to the module and bulk costs.
        piping_bulk_factor: the piping bulk factor.
        electrical_instrumentation_factor: the electrical and instrumentation factor.
        insulation_paint_fireproofing_factor: the insulation, paint and fireproofing factor.
        hvac_telecom_safety_factor: the HVAC, telecom and safety factor.
        engineering_procurement_factor: the engineering and procurement factor.
        construction_management_factor: the construction management factor.
        project_contingency_factor: the project contingency factor.
        owner_cost_factor: the owner cost factor.
        include_utilities_and_offsites: whether the utilities allowance is included.
        estimate_class: the AACE class, which only changes the quality flag.

    Returns:
        The capital lines, the module breakdown, the project stack, the weights,
        the material take-off and the quality flags.

    Raises:
        InvalidInputError: for an unknown facility type or context, an estimate
            class outside 1 to 5, or a non-finite value.
    """
    spec = _spec()
    rows: list[dict[str, Any]] = input_records_to_si(spec, "equipment", equipment)
    warnings: list[Warning] = []

    input_rows = [
        {
            "name": str(row["name"]),
            "equipment_type": str(row["equipment_type"]),
            "total_module_cost": _finite(
                f"equipment[{index}].total_module_cost",
                float(row["total_module_cost"]),
            ),
            "weight": _finite(f"equipment[{index}].weight", float(row["weight"])),
        }
        for index, row in enumerate(rows)
    ]
    facility_name = facility_type or "fixed_platform"
    if facility_name not in {
        "fixed_platform",
        "fpso",
        "semi_submersible",
        "onshore",
        "brownfield_tie_in",
    }:
        raise InvalidInputError("facility_type", "an unknown facility type")
    context_name = project_context or "greenfield"
    if context_name not in {"greenfield", "brownfield_modification", "host_tie_in"}:
        raise InvalidInputError("project_context", "an unknown project context")
    aace_class = _number("estimate_class", estimate_class, 4.0)
    if aace_class < 1.0 or aace_class > 5.0 or aace_class != int(aace_class):
        raise InvalidInputError("estimate_class", "an AACE class is a whole number from 1 to 5")

    result = _costing.estimate_topsides(
        {
            "equipment": input_rows,
            "facility_type": facility_name,
            "project_context": context_name,
            "location_factor": _number("location_factor", location_factor, 1.0),
            "piping_bulk_factor": _number("piping_bulk_factor", piping_bulk_factor, 0.25),
            "electrical_instrumentation_factor": _number(
                "electrical_instrumentation_factor",
                electrical_instrumentation_factor,
                0.16,
            ),
            "insulation_paint_fireproofing_factor": _number(
                "insulation_paint_fireproofing_factor",
                insulation_paint_fireproofing_factor,
                0.06,
            ),
            "hvac_telecom_safety_factor": _number(
                "hvac_telecom_safety_factor", hvac_telecom_safety_factor, 0.08
            ),
            "engineering_procurement_factor": _number(
                "engineering_procurement_factor", engineering_procurement_factor, 0.14
            ),
            "construction_management_factor": _number(
                "construction_management_factor", construction_management_factor, 0.06
            ),
            "project_contingency_factor": _number(
                "project_contingency_factor", project_contingency_factor, 0.15
            ),
            "owner_cost_factor": _number("owner_cost_factor", owner_cost_factor, 0.10),
            "include_utilities_and_offsites": bool(
                True if include_utilities_and_offsites is None else include_utilities_and_offsites
            ),
        }
    )

    flags = [TopsidesFlag(message=str(message)) for message in result["quality_flags"]]
    if aace_class > 3.0:
        flags.append(
            TopsidesFlag(
                message="Estimate is not a detailed Class 1-3 vendor or firm-quantity estimate."
            )
        )

    return TopsidesFacilityCostResult(
        capital_costs=tuple(
            TopsidesCapitalLine(name=str(name), cost=float(cost))
            for name, cost in result["capital_costs"]
        ),
        capital_cost_summary=tuple(
            TopsidesSummaryLine(name=str(name), cost=float(cost))
            for name, cost in result["capital_cost_summary"]
        ),
        capital_cost_breakdown=tuple(
            TopsidesBreakdownLine(name=str(name), cost=float(cost))
            for name, cost in result["capital_cost_breakdown"]
        ),
        project_costs=tuple(
            TopsidesProjectLine(name=str(name), cost=float(cost))
            for name, cost in result["project_costs"]
        ),
        project_cost_summary=tuple(
            TopsidesProjectSummaryLine(name=str(name), cost=float(cost))
            for name, cost in result["project_cost_summary"]
        ),
        weight_basis=tuple(
            TopsidesWeightLine(name=str(name), weight_kg=quantity(float(weight), "kg"))
            for name, weight in result["weight_basis"]
        ),
        material_take_off=tuple(
            TopsidesMtoRow(
                item=str(row["item"]),
                category=_category_of(str(row["item"]), str(row["source"] or "")),
                quantity=float(row["quantity"] or 0.0),
                unit=str(row["unit"] or ""),
                weight_kg=quantity(float(row["weight_kg"] or 0.0), "kg"),
                has_weight=row["weight_kg"] is not None,
                cost_usd=float(row["cost_usd"]),
                source=str(row["source"] or ""),
            )
            for row in result["material_take_off"]
        ),
        quality_flags=tuple(flags),
        warnings=tuple(warnings),
    )
