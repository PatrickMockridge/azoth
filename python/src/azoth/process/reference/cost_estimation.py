"""``process.cost_estimation`` - the cost-estimation calculator.

The Python twin of ``crates/azoth-process/src/models/cost_estimation.rs``: the
factor tables, the Turton correlation dispatch, the module stack, the labour
rates and the vessel bill of materials, from the shared ``_costing`` arithmetic.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import CostBomRow, CostEstimationResult, CostEstimationResultRow
from azoth.core.units import from_si, input_records_to_si, quantity
from azoth.core.warnings import Warning
from azoth.process.reference import _costing

MODEL_ID = "process.cost_estimation"

_CALCULATIONS = (
    "correlation",
    "module_stack",
    "summary",
    "installation_man_hours",
    "piping_man_hours",
    "currency_conversion",
    "pressure_factor",
    "material_factor",
    "location_factor",
    "vessel_bom",
)

_CORRELATIONS = (
    "vertical_vessel_by_volume",
    "horizontal_vessel_by_volume",
    "vertical_vessel_by_weight",
    "horizontal_vessel_by_weight",
    "shell_tube",
    "plate",
    "air_cooler",
    "sieve_trays",
    "valve_trays",
    "bubble_cap_trays",
    "packing",
    "centrifugal_pump",
    "centrifugal_compressor",
    "reciprocating_compressor",
    "piping",
    "control_valve",
)

_MATERIALS = {
    "carbon_steel": "Carbon Steel",
    "ss304": "SS304",
    "ss316": "SS316",
    "ss316l": "SS316L",
    "monel": "Monel",
    "hastelloy_c": "Hastelloy C",
    "inconel": "Inconel",
    "titanium": "Titanium",
    "nickel": "Nickel",
}

_REGIONS = {
    "us_gulf_coast": "US Gulf Coast",
    "us_midwest": "US Midwest",
    "us_west_coast": "US West Coast",
    "western_europe": "Western Europe",
    "north_sea": "North Sea / Norway",
    "middle_east": "Middle East",
    "southeast_asia": "Southeast Asia",
    "china": "China",
    "australia": "Australia",
    "brazil": "Brazil",
    "west_africa": "West Africa",
}


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


def _require(index: int, row: dict[str, Any], field: str, why: str) -> Any:
    value = row.get(field)
    if value is None:
        raise InvalidInputError(f"queries[{index}].{field}", f"a {why} row requires `{field}`")
    return value


def _whole(index: int, field: str, value: float) -> float:
    value = _finite(f"queries[{index}].{field}", value)
    if value != int(value):
        raise InvalidInputError(f"queries[{index}].{field}", "a count is a whole number")
    return value


def _correlation_answer(
    index: int, row: dict[str, Any], material_factor: float, ratio: float
) -> float:
    correlation = str(_require(index, row, "correlation", "correlation"))
    if correlation not in _CORRELATIONS:
        raise InvalidInputError(
            f"queries[{index}].correlation",
            f"{correlation!r} is not one of the calculator's correlations",
        )

    def capacity() -> float:
        return _finite(
            f"queries[{index}].capacity",
            float(_require(index, row, "capacity", "correlation")),
        )

    if correlation == "vertical_vessel_by_volume":
        return float(_costing.vertical_vessel_by_volume(capacity(), ratio))
    if correlation == "horizontal_vessel_by_volume":
        return float(_costing.horizontal_vessel_by_volume(capacity(), ratio))
    if correlation == "vertical_vessel_by_weight":
        return float(_costing.vertical_vessel_by_weight(capacity(), ratio))
    if correlation == "horizontal_vessel_by_weight":
        return float(_costing.horizontal_vessel_by_weight(capacity(), ratio))
    if correlation == "shell_tube":
        return float(_costing.shell_tube_heat_exchanger(capacity(), ratio))
    if correlation == "plate":
        return float(_costing.plate_heat_exchanger(capacity(), ratio))
    if correlation == "air_cooler":
        return float(_costing.air_cooler(capacity(), ratio))
    if correlation in {"sieve_trays", "valve_trays", "bubble_cap_trays"}:
        diameter = _finite(
            f"queries[{index}].diameter", float(_require(index, row, "diameter", "correlation"))
        )
        count = _whole(
            index, "tray_count", float(_require(index, row, "tray_count", "correlation"))
        )
        if correlation == "valve_trays":
            return float(_costing.valve_trays(diameter, count, ratio))
        if correlation == "bubble_cap_trays":
            return float(_costing.bubble_cap_trays(diameter, count, ratio))
        return float(_costing.sieve_trays(diameter, count, ratio))
    if correlation == "packing":
        return float(_costing.packing(capacity(), str(row.get("packing_type") or "plastic"), ratio))
    if correlation == "centrifugal_pump":
        return float(_costing.centrifugal_pump(capacity(), ratio))
    if correlation == "centrifugal_compressor":
        return float(_costing.centrifugal_compressor(capacity(), ratio))
    if correlation == "reciprocating_compressor":
        return float(_costing.reciprocating_compressor(capacity(), ratio))
    if correlation == "piping":
        diameter = _finite(
            f"queries[{index}].diameter", float(_require(index, row, "diameter", "correlation"))
        )
        length = _finite(
            f"queries[{index}].piping_length",
            float(_require(index, row, "piping_length", "correlation")),
        )
        schedule = str(row.get("piping_schedule") or "40")
        if schedule not in {"40", "80", "160"}:
            raise InvalidInputError(
                f"queries[{index}].piping_schedule",
                f"{schedule!r} is not one of the piping schedules",
            )
        return float(_costing.piping_cost(diameter, length, int(schedule), material_factor, ratio))
    if correlation == "control_valve":
        cv = _finite(
            f"queries[{index}].valve_cv", float(_require(index, row, "valve_cv", "correlation"))
        )
        return float(_costing.control_valve(cv, material_factor, ratio))
    raise InvalidInputError(
        f"queries[{index}].correlation",
        "the correlation is not one this kernel dispatches",
    )


def _bom_rows(rows: list[dict[str, Any]]) -> list[CostBomRow]:
    return [
        CostBomRow(
            item=str(row["item"]),
            material=str(row["material"] or ""),
            quantity=float(row["quantity"] or 0.0),
            has_quantity=row["quantity"] is not None,
            unit=str(row["unit"] or ""),
            weight_kg=from_si(float(row["weight_kg"] or 0.0), "kg"),
            has_weight=row["weight_kg"] is not None,
            cost_usd=float(row["cost_usd"]),
            source=str(row["source"] or ""),
        )
        for row in rows
    ]


def cost_estimation(queries: list[dict[str, Any]]) -> CostEstimationResult:
    """The calculator over the stated queries.

    Args:
        queries: one calculator call per row, ``calculation`` selecting the fields
            the row must carry.

    Returns:
        One row per query and the vessel bill of materials.

    Raises:
        InvalidInputError: for an unknown call, correlation, material, region or
            currency, a missing field the call requires, a non-integral count, a
            non-positive cost index or exchange rate, or a non-finite value.
    """
    spec = _spec()
    rows: list[dict[str, Any]] = input_records_to_si(spec, "queries", queries)
    warnings: list[Warning] = []
    results: list[CostEstimationResultRow] = []
    bom: list[CostBomRow] = []

    for index, row in enumerate(rows):
        calculation = str(row["calculation"])
        if calculation not in _CALCULATIONS:
            raise InvalidInputError(
                f"queries[{index}].calculation",
                f"{calculation!r} is not one of the calculator's calls",
            )
        current_cepci = _finite(
            f"queries[{index}].current_cepci",
            840.0 if row.get("current_cepci") is None else float(row["current_cepci"]),
        )
        reference_cepci = _finite(
            f"queries[{index}].reference_cepci",
            607.5 if row.get("reference_cepci") is None else float(row["reference_cepci"]),
        )
        if current_cepci <= 0.0 or reference_cepci <= 0.0:
            raise InvalidInputError(f"queries[{index}].current_cepci", "a cost index is positive")
        ratio = current_cepci / reference_cepci

        if row.get("material_factor") is not None:
            material_factor = _finite(
                f"queries[{index}].material_factor", float(row["material_factor"])
            )
        elif row.get("material") is not None:
            material = str(row["material"])
            if material not in _MATERIALS:
                raise InvalidInputError(
                    f"queries[{index}].material",
                    f"{material!r} is not a material with a factor",
                )
            factor = _costing.material_factor(_MATERIALS[material])
            material_factor = 1.0 if factor is None else float(factor)
        else:
            material_factor = 1.0

        if row.get("region") is not None:
            region = str(row["region"])
            if region not in _REGIONS:
                raise InvalidInputError(
                    f"queries[{index}].region", f"{region!r} is not a location region"
                )
            location = _costing.location_factor(_REGIONS[region])
        else:
            location = 1.0

        if row.get("exchange_rate") is not None:
            exchange = _finite(f"queries[{index}].exchange_rate", float(row["exchange_rate"]))
        elif row.get("currency") is not None:
            rate = _costing.exchange_rate(str(row["currency"]))
            if rate is None:
                raise InvalidInputError(
                    f"queries[{index}].currency",
                    f"{row['currency']!r} is not a currency with a default rate",
                )
            exchange = rate
        else:
            exchange = 1.0
        if exchange <= 0.0:
            raise InvalidInputError(
                f"queries[{index}].exchange_rate", "an exchange rate is positive"
            )

        contingency = _finite(
            f"queries[{index}].contingency_factor",
            0.15 if row.get("contingency_factor") is None else float(row["contingency_factor"]),
        )
        engineering = _finite(
            f"queries[{index}].engineering_factor",
            0.10 if row.get("engineering_factor") is None else float(row["engineering_factor"]),
        )
        design_pressure = (
            None
            if row.get("design_pressure") is None
            else _finite(
                f"queries[{index}].design_pressure",
                float(row["design_pressure"]) / 1.0e5,
            )
        )
        pressure = (
            _finite(f"queries[{index}].pressure_factor", float(row["pressure_factor"]))
            if row.get("pressure_factor") is not None
            else _costing.pressure_factor(design_pressure or 0.0)
        )

        pec = 0.0
        bmc = 0.0
        tmc = 0.0
        grc = 0.0
        man_hours = 0.0
        converted = 0.0
        cost_in_usd = 0.0

        if calculation == "correlation":
            pec = _correlation_answer(index, row, material_factor, ratio)
        elif calculation == "module_stack":
            purchased = _finite(
                f"queries[{index}].purchased_cost",
                float(_require(index, row, "purchased_cost", "module_stack")),
            )
            if purchased < 0.0:
                raise InvalidInputError(
                    f"queries[{index}].purchased_cost",
                    "a purchased cost is at least zero",
                )
            pec = purchased
            bmc = _costing.bare_module(purchased, pressure, material_factor)
            tmc = _costing.total_module(bmc, contingency, engineering)
            grc = _costing.grass_roots(tmc)
        elif calculation == "summary":
            purchased = _finite(
                f"queries[{index}].purchased_cost",
                float(_require(index, row, "purchased_cost", "summary")),
            )
            weight = _finite(
                f"queries[{index}].weight",
                float(_require(index, row, "weight", "summary")),
            )
            equipment_type = str(row.get("equipment_type") or "vessel")
            stack = _costing.calculator_summary(
                purchased,
                design_pressure or 0.0,
                weight,
                equipment_type,
                material_factor,
                location,
                contingency,
                engineering,
            )
            pec = stack["purchased_equipment_cost"]
            bmc = stack["bare_module_cost"]
            tmc = stack["total_module_cost"]
            grc = stack["grass_roots_cost"]
            man_hours = stack["installation_man_hours"]
        elif calculation == "installation_man_hours":
            weight = _finite(
                f"queries[{index}].weight",
                float(_require(index, row, "weight", "installation_man_hours")),
            )
            man_hours = _costing.installation_man_hours(
                weight, str(row.get("equipment_type") or "vessel")
            )
        elif calculation == "piping_man_hours":
            weight = _finite(
                f"queries[{index}].weight",
                float(_require(index, row, "weight", "piping_man_hours")),
            )
            man_hours = _costing.piping_man_hours(weight)
        elif calculation == "currency_conversion":
            amount = _finite(
                f"queries[{index}].amount",
                float(_require(index, row, "amount", "currency_conversion")),
            )
            direction = str(row.get("currency_direction") or "from_usd")
            if direction == "from_usd":
                converted = amount * exchange
                cost_in_usd = amount
            elif direction == "to_usd":
                converted = amount / exchange
                cost_in_usd = converted
            else:
                raise InvalidInputError(
                    f"queries[{index}].currency_direction",
                    f"{direction!r} is not `from_usd` or `to_usd`",
                )
        elif calculation == "vessel_bom":
            nozzle_count = _whole(
                index,
                "nozzle_count",
                float(_require(index, row, "nozzle_count", "vessel_bom")),
            )
            shell = _finite(
                f"queries[{index}].shell_weight",
                float(_require(index, row, "shell_weight", "vessel_bom")),
            )
            heads = _finite(
                f"queries[{index}].heads_weight",
                float(_require(index, row, "heads_weight", "vessel_bom")),
            )
            internals = _finite(
                f"queries[{index}].internals_weight",
                float(_require(index, row, "internals_weight", "vessel_bom")),
            )
            material = _MATERIALS.get(str(row.get("material") or ""), "Carbon Steel")
            bom.extend(
                _bom_rows(
                    _costing.vessel_bom(
                        shell,
                        heads,
                        nozzle_count,
                        internals,
                        material,
                        material_factor,
                        ratio,
                    )
                )
            )

        results.append(
            CostEstimationResultRow(
                calculation=calculation,
                correlation=str(row.get("correlation") or ""),
                purchased_equipment_cost=pec,
                pressure_factor=pressure,
                material_factor=material_factor,
                location_factor=location,
                exchange_rate=exchange,
                bare_module_cost=bmc,
                total_module_cost=tmc,
                grass_roots_cost=grc,
                installation_man_hours=quantity(man_hours, "h"),
                converted_amount=converted,
                cost_in_usd=cost_in_usd,
                current_cepci=current_cepci,
                reference_cepci=reference_cepci,
                cepci_ratio=ratio,
                contingency_factor=contingency,
                engineering_factor=engineering,
                installation_factor=_costing.INSTALLATION_FACTOR_STORED,
            )
        )

    return CostEstimationResult(queries=tuple(results), bom=tuple(bom), warnings=tuple(warnings))
