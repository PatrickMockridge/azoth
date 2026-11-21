"""``process.process_cost_estimate`` - the process-level cost roll-up.

The Python twin of ``crates/azoth-process/src/models/process_cost_estimate.rs``:
the totals, the type and discipline splits, the project stack, the operating
cost and the simple economics, from the shared ``_costing`` roll-up.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import (
    DisciplineCostLine,
    OperatingCostLine,
    ProcessCostEquipmentResult,
    ProcessCostEstimateResult,
    ProcessCostFlag,
    ProcessCostMtoRow,
    ProjectCostLine,
    TypeCostLine,
)
from azoth.core.units import input_records_to_si, quantity
from azoth.core.warnings import Warning
from azoth.process.reference import _costing

MODEL_ID = "process.process_cost_estimate"


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


def _si(value: Any) -> float:
    """A unit-bearing input's base-SI magnitude, or a bare number as it stands."""
    if hasattr(value, "to"):
        return float(value.to_base_units().magnitude)
    return float(value)


def process_cost_estimate(
    equipment: list[dict[str, Any]],
    location_factor: float | None = None,
    complexity_factor: float | None = None,
    project_contingency: float | None = None,
    working_capital: float | None = None,
    system_total_weight: Any = None,
    capex_per_weight: float | None = None,
    operating_hours: Any = None,
    electricity_cost_per_kwh: float | None = None,
    steam_cost_per_tonne: float | None = None,
    cooling_water_cost_per_m3: float | None = None,
    labor_cost_per_hour: float | None = None,
    annual_revenue: float | None = None,
    discount_rate: float | None = None,
    project_life_years: float | None = None,
) -> ProcessCostEstimateResult:
    """The process cost roll-up over the stated unit rows.

    Args:
        equipment: one unit's summary per row, in caller order.
        location_factor: the factor applied once to the totals.
        complexity_factor: the grass roots complexity factor.
        project_contingency: the project contingency factor.
        working_capital: the owner and working-capital factor.
        system_total_weight: the system weight, for the weight-based capex.
        capex_per_weight: the weight-based capex coefficient.
        operating_hours: annual operating hours; absent leaves OPEX at zero.
        electricity_cost_per_kwh: electricity price.
        steam_cost_per_tonne: steam price.
        cooling_water_cost_per_m3: cooling-water price.
        labor_cost_per_hour: operating labour rate.
        annual_revenue: annual revenue, for the economics.
        discount_rate: the NPV discount rate.
        project_life_years: the project life.

    Returns:
        The totals, the per-unit summaries, the splits, the project stack, the
        operating costs and the economics.

    Raises:
        InvalidInputError: for a non-finite value or a non-integral project life.
    """
    spec = _spec()
    rows: list[dict[str, Any]] = input_records_to_si(spec, "equipment", equipment)
    warnings: list[Warning] = []

    input_rows: list[dict[str, Any]] = []
    for index, row in enumerate(rows):
        input_rows.append(
            {
                "name": str(row["name"]),
                "equipment_class": str(row["equipment_class"]),
                "purchased_equipment_cost": _finite(
                    f"equipment[{index}].purchased_equipment_cost",
                    float(row["purchased_equipment_cost"]),
                ),
                "bare_module_cost": _finite(
                    f"equipment[{index}].bare_module_cost",
                    float(row["bare_module_cost"]),
                ),
                "total_module_cost": _finite(
                    f"equipment[{index}].total_module_cost",
                    float(row["total_module_cost"]),
                ),
                "grass_roots_cost": _finite(
                    f"equipment[{index}].grass_roots_cost",
                    float(row["grass_roots_cost"]),
                ),
                "total_cost": _finite(f"equipment[{index}].total_cost", float(row["total_cost"])),
                "installation_man_hours": _finite(
                    f"equipment[{index}].installation_man_hours",
                    float(row["installation_man_hours"]) / 3600.0,
                ),
                "weight": _finite(f"equipment[{index}].weight", float(row["weight"])),
                "power_kw": None
                if row.get("power") is None
                else _finite(f"equipment[{index}].power", float(row["power"]) / 1.0e3),
                "duty_kw": None
                if row.get("duty") is None
                else _finite(f"equipment[{index}].duty", float(row["duty"]) / 1.0e3),
            }
        )

    operating: dict[str, float] | None = None
    if operating_hours is not None:
        operating = {
            "hours_per_year": _finite("operating_hours", _si(operating_hours) / 3600.0),
            "electricity_cost_per_kwh": _number(
                "electricity_cost_per_kwh", electricity_cost_per_kwh, 0.08
            ),
            "steam_cost_per_tonne": _number("steam_cost_per_tonne", steam_cost_per_tonne, 15.0),
            "cooling_water_cost_per_m3": _number(
                "cooling_water_cost_per_m3", cooling_water_cost_per_m3, 0.05
            ),
            "fuel_cost_per_gj": 5.0,
        }
    life = 10
    if project_life_years is not None:
        value = _finite("project_life_years", float(project_life_years))
        if value < 0.0 or value != int(value):
            raise InvalidInputError(
                "project_life_years", "a project life is a whole number of years"
            )
        life = int(value)

    result = _costing.estimate_process(
        {
            "equipment": input_rows,
            "location_factor": _number("location_factor", location_factor, 1.0),
            "complexity_factor": _number("complexity_factor", complexity_factor, 1.0),
            "project_contingency": _number("project_contingency", project_contingency, 0.10),
            "working_capital": _number("working_capital", working_capital, 0.15),
            "system_total_weight": None
            if system_total_weight is None
            else _finite("system_total_weight", _si(system_total_weight)),
            "capex_per_weight": _number("capex_per_weight", capex_per_weight, 1000.0),
            "opex": operating,
            "labor_cost_per_hour": _number("labor_cost_per_hour", labor_cost_per_hour, 50.0),
            "annual_revenue": None
            if annual_revenue is None
            else _finite("annual_revenue", float(annual_revenue)),
            "discount_rate": _number("discount_rate", discount_rate, 0.10),
            "project_life_years": life,
        }
    )

    equipment_results = [
        ProcessCostEquipmentResult(
            name=str(row["name"]),
            equipment_class=str(row["equipment_class"]),
            equipment_type=str(row["equipment_type"]),
            purchased_equipment_cost=float(row["purchased_equipment_cost"]),
            bare_module_cost=float(row["bare_module_cost"]),
            total_module_cost=float(row["total_module_cost"]),
            grass_roots_cost=float(row["grass_roots_cost"]),
            total_cost=float(row["total_cost"]),
            installation_man_hours=quantity(float(row["installation_man_hours"]), "h"),
            weight=quantity(float(row["weight"]), "kg"),
            located_purchased_equipment_cost=float(row["located_purchased_equipment_cost"]),
            location_factor=float(row["location_factor"]),
        )
        for row in result["equipment"]
    ]
    mto = [
        ProcessCostMtoRow(
            name=str(row["name"]),
            equipment_type=str(row["equipment_type"]),
            weight_kg=quantity(float(row["weight"]), "kg"),
            cost_usd=float(row["located_purchased_equipment_cost"]),
        )
        for row in result["equipment"]
    ]

    project = result["project"]
    project_costs = [
        ProjectCostLine(name="directInstalledCost", cost=float(project["direct_installed_cost"])),
        ProjectCostLine(name="indirectCost", cost=float(project["indirect_cost"])),
        ProjectCostLine(name="projectContingency", cost=float(project["project_contingency"])),
        ProjectCostLine(name="ownerCost", cost=float(project["owner_cost"])),
        ProjectCostLine(name="totalProjectCost", cost=float(project["total_project_cost"])),
    ]
    operating_costs: list[OperatingCostLine] = []
    if result["operating_costs"] is not None:
        values = result["operating_costs"]
        operating_costs = [
            OperatingCostLine(name="Electricity", cost=float(values["electricity"])),
            OperatingCostLine(name="Steam", cost=float(values["steam"])),
            OperatingCostLine(name="Cooling Water", cost=float(values["cooling_water"])),
            OperatingCostLine(name="Maintenance", cost=float(values["maintenance"])),
            OperatingCostLine(name="Operating Labor", cost=float(values["operating_labor"])),
            OperatingCostLine(name="Supervision", cost=float(values["supervision"])),
            OperatingCostLine(name="Operating Supplies", cost=float(values["supplies"])),
            OperatingCostLine(name="Laboratory", cost=float(values["laboratory"])),
            OperatingCostLine(name="Overhead", cost=float(values["overhead"])),
        ]

    return ProcessCostEstimateResult(
        total_purchased_equipment_cost=float(result["total_purchased_equipment_cost"]),
        total_bare_module_cost=float(result["total_bare_module_cost"]),
        total_module_cost=float(result["total_module_cost"]),
        total_grass_roots_cost=float(result["total_grass_roots_cost"]),
        total_installation_man_hours=quantity(float(result["total_installation_man_hours"]), "h"),
        equipment=tuple(equipment_results),
        cost_by_equipment_type=tuple(
            TypeCostLine(name=str(name), cost=float(cost))
            for name, cost in result["cost_by_equipment_type"]
        ),
        cost_by_discipline=tuple(
            DisciplineCostLine(name=str(name), cost=float(cost))
            for name, cost in result["cost_by_discipline"]
        ),
        project_costs=tuple(project_costs),
        total_project_cost=float(project["total_project_cost"]),
        operating_costs=tuple(operating_costs),
        has_operating_cost=result["operating_costs"] is not None,
        total_annual_operating_cost=float(result["annual_operating_cost"] or 0.0),
        has_economics=result["payback_period_years"] is not None
        or result["roi_percent"] is not None
        or result["npv"] is not None,
        payback_period_years=float(result["payback_period_years"] or 0.0),
        roi_percent=float(result["roi_percent"] or 0.0),
        npv=float(result["npv"] or 0.0),
        summed_unit_total_cost=float(result["summed_unit_total_cost"]),
        has_weight_based_capex=result["weight_based_capex"] is not None,
        weight_based_capex=float(result["weight_based_capex"] or 0.0),
        material_take_off=tuple(mto),
        quality_flags=tuple(
            ProcessCostFlag(message=str(message)) for message in result["quality_flags"]
        ),
        warnings=tuple(warnings),
    )
