"""``process.equipment_cost_estimate`` - one equipment item's cost stack.

The Python twin of ``crates/azoth-process/src/models/equipment_cost_estimate.rs``:
the selected kind's purchased cost, the shared module stack, the labour hours and
the weight bill of materials, from the shared ``_costing`` arithmetic.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import (
    EquipmentCostEstimateResult,
    EquipmentCostResultRow,
    EquipmentMtoRow,
    EquipmentQualityFlag,
)
from azoth.core.units import input_records_to_si, quantity
from azoth.core.warnings import Warning
from azoth.process.reference import _costing

MODEL_ID = "process.equipment_cost_estimate"

_KINDS = ("separator", "column", "absorber", "reactor", "compressor", "pump", "expander")

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


def _require(index: int, row: dict[str, Any], kind: str, field: str) -> Any:
    value = row.get(field)
    if value is None:
        raise InvalidInputError(f"equipment[{index}].{field}", f"a {kind} row requires `{field}`")
    return value


def _whole(index: int, field: str, value: float) -> float:
    value = _finite(f"equipment[{index}].{field}", value)
    if value != int(value):
        raise InvalidInputError(f"equipment[{index}].{field}", "a count is a whole number")
    return value


def _number(row: dict[str, Any], field: str, default: float = 0.0) -> float:
    value = row.get(field)
    return default if value is None else float(value)


def _boolean(row: dict[str, Any], field: str, default: bool = False) -> bool:
    value = row.get(field)
    return default if value is None else bool(value)


def _spec_for_row(index: int, row: dict[str, Any], kind: str) -> dict[str, Any]:
    if kind == "separator":
        volume = row.get("vessel_volume")
        diameter = row.get("inner_diameter")
        length = row.get("tangent_length")
        if volume is None and (diameter is None or length is None):
            raise InvalidInputError(
                f"equipment[{index}].inner_diameter",
                "a separator requires a vessel volume or both a diameter and a tangent length",
            )
        return {
            "kind": "separator",
            "inner_diameter_m": 0.0 if diameter is None else float(diameter),
            "tangent_length_m": 0.0 if length is None else float(length),
            "vessel_volume_m3": None if volume is None else float(volume),
        }
    if kind == "column":
        diameter = _finite(
            f"equipment[{index}].diameter",
            float(_require(index, row, kind, "diameter")),
        )
        height = _finite(
            f"equipment[{index}].height",
            float(_require(index, row, kind, "height")),
        )
        return {
            "kind": "column",
            "column_type": str(row.get("column_type") or "trayed"),
            "tray_type": str(row.get("tray_type") or "sieve"),
            "packing_type": str(row.get("packing_type") or "structured"),
            "diameter_m": diameter,
            "height_m": height,
            "number_of_trays": 0.0
            if row.get("number_of_trays") is None
            else _whole(index, "number_of_trays", float(row["number_of_trays"])),
            "packing_height_m": _number(row, "packing_height"),
            "design_pressure_barg": _number(row, "design_pressure_barg"),
            "include_reboiler": _boolean(row, "include_reboiler"),
            "include_condenser": _boolean(row, "include_condenser"),
            "reboiler_duty_kw": _number(row, "reboiler_duty") / 1.0e3,
            "condenser_duty_kw": _number(row, "condenser_duty") / 1.0e3,
        }
    diameter = _finite(
        f"equipment[{index}].diameter", float(_require(index, row, kind, "diameter"))
    )
    height = _finite(f"equipment[{index}].height", float(_require(index, row, kind, "height")))
    return {
        "kind": "absorber",
        "absorber_type": str(row.get("absorber_type") or "packed"),
        "tray_type": str(row.get("tray_type") or "sieve"),
        "packing_type": str(row.get("packing_type") or "structured"),
        "diameter_m": diameter,
        "height_m": height,
        "number_of_stages": 0.0
        if row.get("number_of_stages") is None
        else _whole(index, "number_of_stages", float(row["number_of_stages"])),
        "packing_height_m": _number(row, "packing_height"),
        "include_liquid_distributor": _boolean(row, "include_liquid_distributor", True),
        "include_mist_eliminator": _boolean(row, "include_mist_eliminator", True),
        "include_reboiler": _boolean(row, "include_reboiler"),
        "include_reflux_system": _boolean(row, "include_reflux_system"),
        "reboiler_duty_kw": _number(row, "reboiler_duty") / 1.0e3,
    }


def _machine_spec(index: int, row: dict[str, Any], kind: str) -> dict[str, Any]:
    """The reactor and machine rows, whose fields differ by kind."""
    if kind == "reactor":
        return {
            "kind": "reactor",
            "vessel_diameter_m": _finite(
                f"equipment[{index}].vessel_diameter",
                float(_require(index, row, kind, "vessel_diameter")),
            ),
            "vessel_length_m": _finite(
                f"equipment[{index}].vessel_length",
                float(_require(index, row, kind, "vessel_length")),
            ),
            "design_pressure_bara": _number(row, "design_pressure_bara") / 1.0e5,
            "catalyst_mass_kg": _number(row, "catalyst_mass"),
            "number_of_beds": 1.0
            if row.get("number_of_beds") is None
            else _whole(index, "number_of_beds", float(row["number_of_beds"])),
            "total_equipped_weight_kg": _number(row, "total_equipped_weight"),
            "catalyst_cost_per_kg": _number(row, "catalyst_cost_per_kg", 8.0),
            "internals_fraction": _number(row, "internals_fraction", 0.45),
            "bed_allowance_usd": _number(row, "bed_allowance", 25000.0),
        }
    if kind == "compressor":
        return {
            "kind": "compressor",
            "power_kw": _number(row, "power") / 1.0e3,
            "compressor_type": str(row.get("compressor_type") or "centrifugal"),
            "driver_type": str(row.get("driver_type") or "electric-motor"),
            "include_driver": _boolean(row, "include_driver", True),
            "number_of_stages": 1.0
            if row.get("number_of_stages") is None
            else _whole(index, "number_of_stages", float(row["number_of_stages"])),
            "include_intercoolers": _boolean(row, "include_intercoolers"),
        }
    if kind == "pump":
        return {
            "kind": "pump",
            "power_kw": _number(row, "power") / 1.0e3,
            "pump_type": str(row.get("pump_type") or "centrifugal"),
            "include_motor": _boolean(row, "include_motor", True),
            "api_rated": _boolean(row, "api_rated"),
            "seal_type": str(row.get("seal_type") or "single-mechanical"),
        }
    return {
        "kind": "expander",
        "shaft_power_kw": _number(row, "shaft_power") / 1.0e3,
        "expander_type": str(row.get("expander_type") or "radial-inflow"),
        "load_type": str(row.get("load_type") or "generator"),
        "include_load": _boolean(row, "include_load", True),
        "include_gearbox": _boolean(row, "include_gearbox"),
        "include_lube_oil_system": _boolean(row, "include_lube_oil_system", True),
        "include_control_system": _boolean(row, "include_control_system", True),
        "cryogenic_service": _boolean(row, "cryogenic_service"),
    }


def equipment_cost_estimate(
    equipment: list[dict[str, Any]],
) -> EquipmentCostEstimateResult:
    """The purchased cost, stack, labour and bill of materials of each stated item.

    Args:
        equipment: one item per row, ``equipment`` selecting the kind and the
            kind's fields.

    Returns:
        The per-item rows, the weighted bill of materials and the quality flags.

    Raises:
        InvalidInputError: for an unknown kind or material, a missing field the
            kind needs, a non-integral count, a non-positive cost index, or a
            non-finite value.
    """
    spec = _spec()
    rows: list[dict[str, Any]] = input_records_to_si(spec, "equipment", equipment)
    warnings: list[Warning] = []
    results: list[EquipmentCostResultRow] = []
    mto: list[EquipmentMtoRow] = []
    flags: list[EquipmentQualityFlag] = []

    for index, row in enumerate(rows):
        kind = str(row["equipment"])
        if kind not in _KINDS:
            raise InvalidInputError(
                f"equipment[{index}].equipment",
                f"{kind!r} is not a kind this port has reached",
            )
        cepci = _finite(
            f"equipment[{index}].cepci",
            840.0 if row.get("cepci") is None else float(row["cepci"]),
        )
        if cepci <= 0.0:
            raise InvalidInputError(f"equipment[{index}].cepci", "a cost index is positive")
        if row.get("material_factor") is not None:
            material_factor = _finite(
                f"equipment[{index}].material_factor", float(row["material_factor"])
            )
        elif row.get("material") is not None:
            material = str(row["material"])
            if material not in _MATERIALS:
                raise InvalidInputError(
                    f"equipment[{index}].material",
                    f"{material!r} is not a material with a factor",
                )
            factor = _costing.material_factor(_MATERIALS[material])
            material_factor = 1.0 if factor is None else float(factor)
        else:
            material_factor = 1.0
        contingency = _finite(
            f"equipment[{index}].contingency_factor",
            0.15 if row.get("contingency_factor") is None else float(row["contingency_factor"]),
        )
        engineering = _finite(
            f"equipment[{index}].engineering_factor",
            0.10 if row.get("engineering_factor") is None else float(row["engineering_factor"]),
        )
        max_pressure = (
            0.0
            if row.get("max_design_pressure") is None
            else _finite(
                f"equipment[{index}].max_design_pressure",
                float(row["max_design_pressure"]) / 1.0e5,
            )
        )
        weight = (
            0.0
            if row.get("weight_total") is None
            else _finite(f"equipment[{index}].weight_total", float(row["weight_total"]))
        )
        common = {
            "material_factor": material_factor,
            "cepci": cepci,
            "design_present": bool(row.get("mechanical_design_present") or False),
            "max_design_pressure_barg": max_pressure,
            "weight_total_kg": weight,
            "contingency": contingency,
            "engineering": engineering,
        }
        item = (
            _spec_for_row(index, row, kind)
            if kind in ("separator", "column", "absorber")
            else _machine_spec(index, row, kind)
        )
        # The design pressures arrive in Pa; the kinds read them in bar.
        if "design_pressure_barg" in item:
            item["design_pressure_barg"] = _number(row, "design_pressure_barg") / 1.0e5
        evaluated = _costing.estimate_equipment(item, common)
        stack = evaluated["stack"]
        extras = evaluated["extras"]

        operating = 0.0
        if row.get("operating_hours") is not None:
            operating = _costing.annual_operating_cost(
                item,
                stack,
                {
                    "hours_per_year": _finite(
                        f"equipment[{index}].operating_hours",
                        float(row["operating_hours"]) / 3600.0,
                    ),
                    "electricity_cost_per_kwh": _number(row, "electricity_cost_per_kwh", 0.08),
                    "steam_cost_per_tonne": _number(row, "steam_cost_per_tonne", 15.0),
                    "cooling_water_cost_per_m3": _number(row, "cooling_water_cost_per_m3", 0.05),
                    "fuel_cost_per_gj": _number(row, "fuel_cost_per_gj", 5.0),
                },
            )

        results.append(
            EquipmentCostResultRow(
                name=str(row.get("name") or ""),
                equipment_type=str(evaluated["equipment_type"]),
                purchased_equipment_cost=float(stack["purchased_equipment_cost"]),
                bare_module_cost=float(stack["bare_module_cost"]),
                total_module_cost=float(stack["total_module_cost"]),
                grass_roots_cost=float(stack["grass_roots_cost"]),
                total_cost=float(evaluated["total_cost"]),
                installation_man_hours=quantity(float(stack["installation_man_hours"]), "h"),
                material_factor=material_factor,
                equipment_cost=float(extras["equipment_cost"] or 0.0),
                installed_cost=float(extras["installed_cost"] or 0.0),
                specific_cost_per_m2=float(extras["specific_cost_per_m2"] or 0.0),
                specific_capex_per_kw=float(extras["specific_capex_per_kw"] or 0.0),
                annual_maintenance_cost=float(extras["annual_maintenance_cost"] or 0.0),
                sorbent_replacement_cost=float(extras["sorbent_replacement_cost"] or 0.0),
                annual_operating_cost=operating,
                calculated_weight=quantity(float(extras["calculated_weight_kg"] or 0.0), "kg"),
            )
        )
        for message in evaluated["quality_flags"]:
            flags.append(EquipmentQualityFlag(equipment_index=float(index), message=str(message)))

        if bool(row.get("include_material_take_off") or False):
            material = _MATERIALS.get(str(row.get("material") or ""), "Carbon Steel")
            lines = _costing.weight_bom(
                _number(row, "shell_weight"),
                _number(row, "internals_weight"),
                _number(row, "nozzle_weight"),
                _number(row, "piping_weight"),
                _number(row, "electro_instrument_weight"),
                _number(row, "structural_weight"),
                material,
                _costing.COST_PER_WEIGHT_UNIT,
            )
            mto.extend(
                EquipmentMtoRow(
                    equipment_index=float(index),
                    item=str(line["item"]),
                    material=str(line["material"] or ""),
                    quantity=float(line["quantity"] or 0.0),
                    has_quantity=line["quantity"] is not None,
                    unit=str(line["unit"] or ""),
                    weight_kg=quantity(float(line["weight_kg"] or 0.0), "kg"),
                    has_weight=line["weight_kg"] is not None,
                    cost_usd=float(line["cost_usd"]),
                    source=str(line["source"] or ""),
                )
                for line in lines
            )

    return EquipmentCostEstimateResult(
        equipment=tuple(results),
        material_take_off=tuple(mto),
        quality_flags=tuple(flags),
        warnings=tuple(warnings),
    )
