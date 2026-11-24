"""Shared cost-estimation arithmetic.

The Python twin of ``crates/azoth-process/src/costing/``. Costs are USD
magnitudes carried as bare floats; the vocabulary has no currency unit, so the
currency, cost year and location basis travel beside them in the basis record.
"""

from __future__ import annotations

import math
from typing import Any

CEPCI_2019 = 607.5
CEPCI_2020 = 596.2
CEPCI_2021 = 708.0
CEPCI_2022 = 816.0
CEPCI_2023 = 800.0
CEPCI_2024 = 820.0
CEPCI_2025 = 840.0

BARE_MODULE_B1 = 1.89
BARE_MODULE_B2 = 1.35
CONTINGENCY_DEFAULT = 0.15
ENGINEERING_DEFAULT = 0.10
GRASS_ROOTS_FACTOR = 1.5
INSTALLATION_FACTOR_STORED = 3.0
COST_PER_WEIGHT_UNIT = 8.0

COMPRESSOR_MIN_KW = 450.0
COMPRESSOR_MAX_KW = 3000.0
PUMP_MIN_KW = 1.0
PUMP_MAX_KW = 300.0

_CEPCI_BY_YEAR = {
    2019: CEPCI_2019,
    2020: CEPCI_2020,
    2021: CEPCI_2021,
    2022: CEPCI_2022,
    2023: CEPCI_2023,
    2024: CEPCI_2024,
    2025: CEPCI_2025,
}

_MATERIAL_FACTORS = {
    "carbon steel": 1.0,
    "cs": 1.0,
    "ss304": 1.8,
    "304ss": 1.8,
    "ss316": 2.1,
    "316ss": 2.1,
    "ss316l": 2.3,
    "316lss": 2.3,
    "monel": 3.2,
    "hastelloy": 3.8,
    "hastelloy c": 3.8,
    "inconel": 3.5,
    "titanium": 4.5,
    "nickel": 3.0,
}

_MATERIAL_NAMES = {
    "carbon steel": "Carbon Steel",
    "cs": "Carbon Steel",
    "ss304": "SS304",
    "304ss": "SS304",
    "ss316": "SS316",
    "316ss": "SS316",
    "ss316l": "SS316L",
    "316lss": "SS316L",
    "monel": "Monel",
    "hastelloy": "Hastelloy C",
    "hastelloy c": "Hastelloy C",
    "inconel": "Inconel",
    "titanium": "Titanium",
    "nickel": "Nickel",
}

_EXCHANGE_RATES = {
    "USD": 1.0,
    "EUR": 0.92,
    "NOK": 11.0,
    "GBP": 0.79,
    "CNY": 7.25,
    "JPY": 155.0,
}

#: The AACE classes: number, maturity, low accuracy, high accuracy, default method.
ESTIMATE_CLASSES: dict[int, tuple[str, float, float, str]] = {
    1: ("Final", -0.10, 0.15, "vendor-quote"),
    2: ("Detailed", -0.15, 0.20, "detailed-quantity-takeoff"),
    3: ("Budget/FEED", -0.20, 0.30, "sized-equipment-and-bulk-mto"),
    4: ("Study", -0.30, 0.50, "equipment-factored"),
    5: ("Concept", -0.50, 1.00, "capacity-factored"),
}


def default_basis() -> dict[str, Any]:
    """The class's default estimate basis, Class 4 on a 2025 US Gulf Coast."""
    return {
        "estimate_class": 4,
        "estimating_method": ESTIMATE_CLASSES[4][3],
        "data_source": "correlation",
        "currency_code": "USD",
        "cost_year": 2025,
        "location_factor": 1.0,
        "location_basis": "US Gulf Coast",
        "notes": "Equipment-factored NeqSim screening/study estimate.",
    }


def cepci_for_year(year: int) -> float | None:
    """The CEPCI for a stated year."""
    return _CEPCI_BY_YEAR.get(year)


def pressure_factor(design_pressure_barg: float) -> float:
    """The vessel pressure-factor step table."""
    if design_pressure_barg <= 5.0:
        return 1.0
    if design_pressure_barg <= 10.0:
        return 1.05
    if design_pressure_barg <= 20.0:
        return 1.15
    if design_pressure_barg <= 50.0:
        return 1.30
    if design_pressure_barg <= 100.0:
        return 1.50
    if design_pressure_barg <= 150.0:
        return 1.65
    if design_pressure_barg <= 200.0:
        return 1.90
    return 2.5


def material_factor(material: str) -> float | None:
    """The material factor, or ``None`` where the class keeps its previous one."""
    return _MATERIAL_FACTORS.get(material.strip().lower())


def material_name(material: str) -> str | None:
    """The canonical material name."""
    return _MATERIAL_NAMES.get(material.strip().lower())


def location_factor(region: str) -> float:
    """The location factor, the class's ordered containment chain."""
    name = region.lower()
    if "gulf coast" in name or "usgc" in name:
        return 1.0
    if "midwest" in name:
        return 1.05
    if "west coast" in name or "california" in name:
        return 1.15
    if "north sea" in name or "norway" in name or "norwegian" in name:
        return 1.35
    if "western europe" in name or "uk" in name or "netherlands" in name or "germany" in name:
        return 1.20
    if (
        "middle east" in name
        or "qatar" in name
        or "saudi" in name
        or "uae" in name
        or "abu dhabi" in name
    ):
        return 1.10
    if "southeast asia" in name or "singapore" in name or "malaysia" in name or "indonesia" in name:
        return 0.90
    if "china" in name:
        return 0.75
    if "australia" in name:
        return 1.30
    if "brazil" in name:
        return 1.15
    if "west africa" in name or "nigeria" in name or "angola" in name:
        return 1.40
    return 1.0


def exchange_rate(currency: str) -> float | None:
    """The default exchange rate from USD."""
    return _EXCHANGE_RATES.get(currency.strip().upper())


def cepci_ratio(current: float) -> float:
    """The current index over the correlation reference, 607.5."""
    return current / CEPCI_2019


# ---------------------------------------------------------------------------
# Correlations.
# ---------------------------------------------------------------------------


def turton_parallel(
    capacity: float,
    min_capacity: float,
    max_capacity: float,
    k1: float,
    k2: float,
    k3: float,
    type_factor: float,
    ratio: float,
) -> float:
    """A Turton capacity correlation evaluated as equal parallel units."""
    units = max(math.ceil(capacity / max_capacity), 1.0)
    per_unit = max(capacity / units, min_capacity)
    log_s = math.log10(per_unit)
    log_cp = k1 + k2 * log_s + k3 * log_s * log_s
    return units * math.pow(10.0, log_cp) * type_factor * ratio


def vertical_vessel_by_volume(volume_m3: float, ratio: float) -> float:
    if volume_m3 <= 0.0:
        return 0.0
    volume = min(max(volume_m3, 0.3), 520.0)
    log_v = math.log10(volume)
    log_cp = 3.4974 + 0.4485 * log_v + 0.1074 * log_v * log_v
    return math.pow(10.0, log_cp) * ratio


def vertical_vessel_by_weight(weight_kg: float, ratio: float) -> float:
    if weight_kg <= 0.0:
        return 0.0
    log_w = math.log10(max(weight_kg, 250.0))
    log_cp = 3.4974 + 0.4485 * log_w + 0.1074 * log_w * log_w
    return math.pow(10.0, log_cp) * ratio


def horizontal_vessel_by_volume(volume_m3: float, ratio: float) -> float:
    if volume_m3 <= 0.0:
        return 0.0
    volume = min(max(volume_m3, 0.1), 628.0)
    log_v = math.log10(volume)
    log_cp = 3.5565 + 0.3776 * log_v + 0.0905 * log_v * log_v
    return math.pow(10.0, log_cp) * ratio


def horizontal_vessel_by_weight(weight_kg: float, ratio: float) -> float:
    if weight_kg <= 0.0:
        return 0.0
    log_w = math.log10(max(weight_kg, 250.0))
    log_cp = 3.5565 + 0.3776 * log_w + 0.0905 * log_w * log_w
    return math.pow(10.0, log_cp) * ratio


def shell_tube_heat_exchanger(area_m2: float, ratio: float) -> float:
    if area_m2 <= 0.0:
        return 0.0
    log_a = math.log10(max(area_m2, 10.0))
    log_cp = 4.3247 - 0.3030 * log_a + 0.1634 * log_a * log_a
    return math.pow(10.0, log_cp) * ratio


def plate_heat_exchanger(area_m2: float, ratio: float) -> float:
    if area_m2 <= 0.0:
        return 0.0
    return 1000.0 * math.pow(area_m2, 0.68) * ratio


def air_cooler(area_m2: float, ratio: float) -> float:
    if area_m2 <= 0.0:
        return 0.0
    log_a = math.log10(max(area_m2, 10.0))
    log_cp = 4.0336 + 0.2341 * log_a + 0.0497 * log_a * log_a
    return math.pow(10.0, log_cp) * ratio


def sieve_trays(diameter_m: float, number_of_trays: float, ratio: float) -> float:
    if diameter_m <= 0.0 or number_of_trays <= 0.0:
        return 0.0
    d = min(max(diameter_m, 0.3), 5.0)
    return 468.0 * math.exp(0.1739 * d) * number_of_trays * ratio


def valve_trays(diameter_m: float, number_of_trays: float, ratio: float) -> float:
    return sieve_trays(diameter_m, number_of_trays, ratio) * 1.2


def bubble_cap_trays(diameter_m: float, number_of_trays: float, ratio: float) -> float:
    return sieve_trays(diameter_m, number_of_trays, ratio) * 1.5


def packing(volume_m3: float, packing_type: str, ratio: float) -> float:
    if volume_m3 <= 0.0:
        return 0.0
    if packing_type.lower() == "metal":
        per_m3 = 8000.0
    elif packing_type.lower() == "ceramic":
        per_m3 = 3000.0
    else:
        per_m3 = 2000.0
    return per_m3 * volume_m3 * ratio


def centrifugal_pump(power_kw: float, ratio: float) -> float:
    if power_kw <= 0.0:
        return 0.0
    return turton_parallel(power_kw, PUMP_MIN_KW, PUMP_MAX_KW, 3.3892, 0.0536, 0.1538, 1.0, ratio)


def centrifugal_compressor(power_kw: float, ratio: float) -> float:
    if power_kw <= 0.0:
        return 0.0
    return turton_parallel(
        power_kw,
        COMPRESSOR_MIN_KW,
        COMPRESSOR_MAX_KW,
        2.2897,
        1.3604,
        -0.1027,
        1.0,
        ratio,
    )


def reciprocating_compressor(power_kw: float, ratio: float) -> float:
    if power_kw <= 0.0:
        return 0.0
    return turton_parallel(
        power_kw,
        COMPRESSOR_MIN_KW,
        COMPRESSOR_MAX_KW,
        2.2897,
        1.3604,
        -0.1027,
        1.3,
        ratio,
    )


def piping_cost(
    diameter_m: float,
    length_m: float,
    schedule: int,
    material_factor_value: float,
    ratio: float,
) -> float:
    if diameter_m <= 0.0 or length_m <= 0.0:
        return 0.0
    diameter_in = diameter_m / 0.0254
    base_per_m = 50.0 * math.pow(diameter_in, 1.5)
    schedule_factor = 1.5 if schedule == 80 else 2.5 if schedule == 160 else 1.0
    return base_per_m * length_m * schedule_factor * material_factor_value * ratio


def control_valve(cv: float, material_factor_value: float, ratio: float) -> float:
    if cv <= 0.0:
        return 0.0
    return 2500.0 * math.pow(cv, 0.55) * material_factor_value * ratio


def installation_man_hours(weight_kg: float, equipment_type: str) -> float:
    if weight_kg <= 0.0:
        return 0.0
    lowered = equipment_type.lower()
    if lowered == "vessel":
        per_tonne = 30.0
    elif lowered == "exchanger":
        per_tonne = 40.0
    elif lowered == "pump":
        per_tonne = 50.0
    elif lowered == "compressor":
        per_tonne = 60.0
    else:
        per_tonne = 35.0
    return per_tonne * weight_kg / 1000.0


def piping_man_hours(weight_kg: float) -> float:
    return 100.0 * weight_kg / 1000.0


def vessel_bom(
    shell_weight_kg: float,
    heads_weight_kg: float,
    nozzle_count: float,
    internals_weight_kg: float,
    material: str,
    material_factor_value: float,
    ratio: float,
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if shell_weight_kg > 0.0:
        rows.append(
            {
                "item": "Vessel Shell",
                "material": material,
                "quantity": None,
                "unit": None,
                "weight_kg": shell_weight_kg,
                "cost_usd": vertical_vessel_by_weight(shell_weight_kg, ratio) * 0.6,
                "source": None,
            }
        )
    if heads_weight_kg > 0.0:
        rows.append(
            {
                "item": "Vessel Heads (2)",
                "material": material,
                "quantity": None,
                "unit": None,
                "weight_kg": heads_weight_kg,
                "cost_usd": heads_weight_kg * 8.0 * material_factor_value,
                "source": None,
            }
        )
    if nozzle_count > 0.0:
        rows.append(
            {
                "item": "Nozzles",
                "material": None,
                "quantity": nozzle_count,
                "unit": None,
                "weight_kg": None,
                "cost_usd": 500.0 * nozzle_count * material_factor_value,
                "source": None,
            }
        )
    if internals_weight_kg > 0.0:
        rows.append(
            {
                "item": "Internals",
                "material": material,
                "quantity": None,
                "unit": None,
                "weight_kg": internals_weight_kg,
                "cost_usd": internals_weight_kg * 15.0 * material_factor_value,
                "source": None,
            }
        )
    return rows


def weight_bom(
    shell_weight_kg: float,
    internals_weight_kg: float,
    nozzle_weight_kg: float,
    piping_weight_kg: float,
    electro_instrument_weight_kg: float,
    structural_weight_kg: float,
    material: str,
    cost_per_weight_unit: float,
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []

    def push(item: str, row_material: str | None, weight: float, multiplier: float) -> None:
        if weight > 0.0:
            rows.append(
                {
                    "item": item,
                    "material": row_material,
                    "quantity": None,
                    "unit": "kg",
                    "weight_kg": weight,
                    "cost_usd": weight * cost_per_weight_unit * multiplier,
                    "source": None,
                }
            )

    push("Vessel Shell", material, shell_weight_kg, 1.0)
    push("Internals", material, internals_weight_kg, 1.5)
    push("Nozzles", material, nozzle_weight_kg, 2.0)
    push("Piping", material, piping_weight_kg, 1.2)
    push("Electrical & Instrumentation", None, electro_instrument_weight_kg, 3.0)
    push("Structural Steel", "Carbon Steel", structural_weight_kg, 0.8)
    return rows


# ---------------------------------------------------------------------------
# The module stack.
# ---------------------------------------------------------------------------


def bare_module(purchased_cost: float, pressure_factor_value: float, fm: float) -> float:
    return purchased_cost * (BARE_MODULE_B1 + BARE_MODULE_B2 * pressure_factor_value * fm)


def total_module(bare_module_cost: float, contingency: float, engineering: float) -> float:
    return bare_module_cost * (1.0 + contingency + engineering)


def grass_roots(total_module_cost: float) -> float:
    return total_module_cost * GRASS_ROOTS_FACTOR


def equipment_stack(
    purchased_equipment_cost: float,
    design_present: bool,
    max_design_pressure_barg: float,
    weight_kg: float,
    equipment_type: str,
    material_factor_value: float,
    contingency: float,
    engineering: float,
) -> dict[str, float]:
    if not design_present:
        return {
            "purchased_equipment_cost": purchased_equipment_cost,
            "bare_module_cost": 0.0,
            "total_module_cost": 0.0,
            "grass_roots_cost": 0.0,
            "installation_man_hours": 0.0,
        }
    bmc = bare_module(
        purchased_equipment_cost,
        pressure_factor(max_design_pressure_barg),
        material_factor_value,
    )
    tmc = total_module(bmc, contingency, engineering)
    return {
        "purchased_equipment_cost": purchased_equipment_cost,
        "bare_module_cost": bmc,
        "total_module_cost": tmc,
        "grass_roots_cost": grass_roots(tmc),
        "installation_man_hours": installation_man_hours(weight_kg, equipment_type),
    }


def calculator_summary(
    purchased_cost: float,
    design_pressure_barg: float,
    weight_kg: float,
    equipment_type: str,
    material_factor_value: float,
    location_factor_value: float,
    contingency: float,
    engineering: float,
) -> dict[str, float]:
    pec = purchased_cost * material_factor_value * location_factor_value
    bmc = bare_module(pec, pressure_factor(design_pressure_barg), 1.0)
    tmc = total_module(bmc, contingency, engineering)
    return {
        "purchased_equipment_cost": pec,
        "bare_module_cost": bmc,
        "total_module_cost": tmc,
        "grass_roots_cost": grass_roots(tmc),
        "installation_man_hours": installation_man_hours(weight_kg, equipment_type),
    }


# ---------------------------------------------------------------------------
# Equipment.
# ---------------------------------------------------------------------------

EQUIPMENT_TYPES: dict[str, str] = {
    "separator": "vessel",
    "reactor": "reactor",
    "column": "column",
    "absorber": "absorber",
    "compressor": "compressor",
    "pump": "pump",
    "expander": "expander",
    "heat_exchanger": "exchanger",
    "bahx": "BAHX",
    "tank": "tank",
    "filter": "filter",
    "manifold": "manifold",
    "mercury_removal": "vessel",
    "electrolyzer": "electrolyzer",
    "psa": "psa-cascade",
    "ejector": "ejector",
    "pipe": "piping",
    "valve": "valve",
    "mixer": "mixer",
    "splitter": "splitter",
    "well": "well",
}

_MODULE_ONLY_TOTAL = {
    "separator",
    "column",
    "compressor",
    "pump",
    "heat_exchanger",
    "mercury_removal",
    "pipe",
    "valve",
}


def _f(spec: dict[str, Any], key: str, default: float = 0.0) -> float:
    value = spec.get(key)
    return default if value is None else float(value)


def _b(spec: dict[str, Any], key: str, default: bool = False) -> bool:
    value = spec.get(key)
    return default if value is None else bool(value)


def _s(spec: dict[str, Any], key: str, default: str = "") -> str:
    value = spec.get(key)
    return default if value is None else str(value)


def _empty_extras() -> dict[str, float | None]:
    return {
        "equipment_cost": None,
        "installed_cost": None,
        "specific_cost_per_m2": None,
        "specific_capex_per_kw": None,
        "annual_maintenance_cost": None,
        "sorbent_replacement_cost": None,
        "annual_operating_cost": None,
        "calculated_weight_kg": None,
    }


def _trays(tray_type: str, diameter_m: float, count: float, ratio: float) -> float:
    if tray_type.lower() == "valve":
        return valve_trays(diameter_m, count, ratio)
    if tray_type.lower() == "bubble-cap":
        return bubble_cap_trays(diameter_m, count, ratio)
    return sieve_trays(diameter_m, count, ratio)


def _column_pressure_factor(design_pressure_barg: float) -> float:
    if design_pressure_barg <= 5.0:
        return 1.0
    if design_pressure_barg <= 20.0:
        return 1.2
    if design_pressure_barg <= 50.0:
        return 1.5
    if design_pressure_barg <= 100.0:
        return 2.0
    return 2.5


def _driver_cost(driver_type: str, power_kw: float, ratio: float) -> float:
    if power_kw <= 0.0:
        return 0.0
    lowered = driver_type.lower()
    if lowered == "gas-turbine":
        return 4000.0 * math.pow(power_kw, 0.8) * ratio
    if lowered == "steam-turbine":
        return 2000.0 * math.pow(power_kw, 0.75) * ratio
    if lowered == "gas-engine":
        return 500.0 * math.pow(power_kw, 0.85) * ratio
    return 300.0 * math.pow(power_kw / 0.95, 0.75) * ratio


def _tema_factor(tema_type: str) -> float:
    front = tema_type[0] if tema_type else "A"
    rear = tema_type[2] if len(tema_type) > 2 else "S"
    factor = 1.1 if front == "B" else 1.2 if front == "C" else 1.3 if front == "N" else 1.0
    factor *= {
        "P": 1.2,
        "S": 1.15,
        "T": 1.25,
        "U": 0.9,
        "W": 1.1,
    }.get(rear, 1.0)
    return factor


def _grade_factor(grade: str) -> float:
    upper = grade.upper()
    if "DUPLEX" in upper or "22CR" in upper or "25CR" in upper:
        return 2.2
    if "316" in upper or "SS" in upper or "STAINLESS" in upper:
        return 1.8
    return 1.0


def _valve_pressure_class_factor(pressure_class: float) -> float:
    return {
        150.0: 1.0,
        300.0: 1.2,
        600.0: 1.5,
        900.0: 2.0,
        1500.0: 2.5,
        2500.0: 3.5,
    }.get(pressure_class, 1.0)


def _mixer_pressure_class_factor(pressure_class: float) -> float:
    if pressure_class <= 150.0:
        return 1.0
    if pressure_class <= 300.0:
        return 1.1
    if pressure_class <= 600.0:
        return 1.3
    if pressure_class <= 900.0:
        return 1.5
    if pressure_class <= 1500.0:
        return 1.8
    return 2.2


def _pipe_schedule_for_cost(schedule: str) -> int:
    if schedule in {"80", "80S"}:
        return 80
    if schedule == "160":
        return 160
    return 40


def _pipe_schedule_weight_factor(schedule: str) -> float:
    if schedule in {"10", "10S"}:
        return 0.7
    if schedule in {"80", "80S"}:
        return 1.4
    if schedule == "160":
        return 2.0
    if schedule == "XXS":
        return 2.5
    return 1.0


def _pipe_weight(spec: dict[str, Any]) -> float:
    nominal = _f(spec, "nominal_diameter_in")
    if nominal <= 2.0:
        base = 0.003
    elif nominal <= 6.0:
        base = 0.0055
    elif nominal <= 12.0:
        base = 0.0095
    else:
        base = 0.0125
    thickness = base * _pipe_schedule_weight_factor(_s(spec, "schedule", "40"))
    outer = nominal * 0.0254
    inner = outer - 2.0 * thickness
    return math.pi / 4.0 * (outer * outer - inner * inner) * _f(spec, "length_m") * 7850.0


def _header_cost(
    header_diameter_m: float,
    header_length_m: float,
    branch_diameter_m: float,
    branch_count: float,
    material_factor_value: float,
    cepci: float,
) -> float:
    if header_diameter_m <= 0.0:
        return 0.0
    ratio = cepci_ratio(cepci)
    header_per_m = piping_cost(header_diameter_m, 1.0, 40, material_factor_value, ratio)
    branch_per_m = piping_cost(branch_diameter_m, 1.0, 40, material_factor_value, ratio)
    header = header_per_m * header_length_m
    branch = branch_per_m * 0.5 * branch_count
    return header + branch + (header + branch) * 0.5


def _equipment_pec(
    spec: dict[str, Any], common: dict[str, Any], extras: dict[str, float | None]
) -> float:
    kind = _s(spec, "kind")
    ratio = cepci_ratio(float(common["cepci"]))
    fm = float(common["material_factor"])
    weight_total = float(common["weight_total_kg"])
    design_present = bool(common["design_present"])

    if kind == "separator":
        volume = 0.0
        diameter = _f(spec, "inner_diameter_m")
        length = _f(spec, "tangent_length_m")
        if diameter > 0.0 and length > 0.0:
            volume = math.pi / 4.0 * diameter * diameter * length
        if volume <= 0.0:
            volume = _f(spec, "vessel_volume_m3")
        # The class prices by the separator's orientation, whose default is horizontal.
        return 0.0 if volume <= 0.0 else horizontal_vessel_by_volume(volume, ratio)

    if kind == "reactor":
        diameter = _f(spec, "vessel_diameter_m")
        length = _f(spec, "vessel_length_m")
        volume = (
            math.pi / 4.0 * diameter * diameter * length if diameter > 0.0 and length > 0.0 else 0.0
        )
        vessel = vertical_vessel_by_volume(volume, ratio)
        pressure = pressure_factor(max(_f(spec, "design_pressure_bara") - 1.01325, 0.0))
        catalyst = _f(spec, "catalyst_mass_kg") * _f(spec, "catalyst_cost_per_kg", 8.0)
        internals = vessel * _f(spec, "internals_fraction", 0.45) + max(
            _f(spec, "number_of_beds", 1.0), 1.0
        ) * _f(spec, "bed_allowance_usd", 25000.0)
        fallback = _f(spec, "total_equipped_weight_kg") * 20.0
        return max(vessel * pressure + catalyst + internals, fallback)

    if kind == "column":
        diameter = _f(spec, "diameter_m")
        height = _f(spec, "height_m")
        area = math.pi / 4.0 * diameter * diameter
        shell = (
            vertical_vessel_by_volume(area * height, ratio)
            * _column_pressure_factor(_f(spec, "design_pressure_barg"))
            * fm
        )
        if _s(spec, "column_type").lower() == "packed":
            internals = (
                packing(area * _f(spec, "packing_height_m"), _s(spec, "packing_type"), ratio) * fm
                + 1500.0 * math.pow(area, 0.6) * 2.0 * ratio * fm
            )
        else:
            internals = (
                _trays(
                    _s(spec, "tray_type"),
                    diameter,
                    _f(spec, "number_of_trays"),
                    ratio,
                )
                * fm
            )
        reboiler = 0.0
        if _b(spec, "include_reboiler") and _f(spec, "reboiler_duty_kw") > 0.0:
            reboiler_area = _f(spec, "reboiler_duty_kw") * 1000.0 / (1000.0 * 30.0)
            reboiler = shell_tube_heat_exchanger(reboiler_area, ratio) * 1.3 * fm
        condenser = 0.0
        if _b(spec, "include_condenser") and _f(spec, "condenser_duty_kw") > 0.0:
            condenser_area = _f(spec, "condenser_duty_kw") * 1000.0 / (800.0 * 20.0)
            condenser = shell_tube_heat_exchanger(condenser_area, ratio) * fm
        return shell + internals + reboiler + condenser

    if kind == "absorber":
        diameter = _f(spec, "diameter_m")
        height = _f(spec, "height_m")
        area = math.pi / 4.0 * diameter * diameter
        shell = vertical_vessel_by_volume(area * height, ratio)
        absorber_type = _s(spec, "absorber_type").lower()
        if absorber_type == "trayed":
            internals = _trays(
                _s(spec, "tray_type"),
                diameter,
                _f(spec, "number_of_stages"),
                ratio,
            )
        elif absorber_type == "spray":
            internals = (5000.0 + 2000.0 * area) * ratio
        else:
            internals = packing(
                area * _f(spec, "packing_height_m"), _s(spec, "packing_type"), ratio
            )
        total = (shell + internals) * fm
        if _b(spec, "include_liquid_distributor", True):
            total += 3000.0 * area * fm * ratio
        if _b(spec, "include_mist_eliminator", True):
            total += 1500.0 * area * fm * ratio
        if _b(spec, "include_reboiler"):
            reboiler_area = _f(spec, "reboiler_duty_kw") * 1000.0 / (800.0 * 30.0)
            total += shell_tube_heat_exchanger(reboiler_area, ratio) * fm
        if _b(spec, "include_reflux_system"):
            drum_volume = 0.1 * area * height
            drum = horizontal_vessel_by_weight(drum_volume * 100.0, ratio)
            total += (drum + centrifugal_pump(10.0, ratio)) * fm
        return total

    if kind == "compressor":
        power = _f(spec, "power_kw")
        if power <= 0.0:
            return weight_total * 20.0
        compressor_type = _s(spec, "compressor_type").lower()
        if compressor_type == "reciprocating":
            cost = reciprocating_compressor(power, ratio)
        elif compressor_type == "screw":
            cost = centrifugal_compressor(power, ratio) * 0.8
        elif compressor_type == "axial":
            cost = centrifugal_compressor(power, ratio) * 1.3
        else:
            cost = centrifugal_compressor(power, ratio)
        stages = _f(spec, "number_of_stages", 1.0)
        if stages > 1.0:
            cost *= 1.0 + (stages - 1.0) * 0.25
        if _b(spec, "include_driver", True):
            cost += _driver_cost(_s(spec, "driver_type", "electric-motor"), power, ratio)
        if _b(spec, "include_intercoolers") and stages > 1.0:
            area = power * 0.7 * 1000.0 / (500.0 * 30.0)
            cost += shell_tube_heat_exchanger(area, ratio) * (stages - 1.0)
        return cost

    if kind == "pump":
        power = _f(spec, "power_kw")
        if power <= 0.0:
            return weight_total * 15.0
        pump_type = _s(spec, "pump_type").lower()
        if pump_type == "reciprocating":
            cost = centrifugal_pump(power, ratio) * 1.5
        elif pump_type == "gear":
            cost = 400.0 * math.pow(max(power, 1.0), 0.7) * ratio
        elif pump_type == "screw":
            cost = centrifugal_pump(power, ratio) * 2.0
        elif pump_type == "diaphragm":
            cost = 600.0 * math.pow(max(power, 1.0), 0.6) * ratio
        else:
            cost = centrifugal_pump(power, ratio)
        if _b(spec, "api_rated"):
            cost *= 1.5
        seal = _s(spec, "seal_type", "single-mechanical").lower()
        cost *= (
            1.3
            if seal == "double-mechanical"
            else 0.8
            if seal == "packed"
            else 2.0
            if seal in {"mag-drive", "magnetic-drive"}
            else 2.5
            if seal == "canned"
            else 1.0
        )
        if _b(spec, "include_motor", True):
            cost += 200.0 * math.pow(power / 0.9, 0.75) * ratio
        return cost

    if kind == "expander":
        power = _f(spec, "shaft_power_kw")
        if power <= 0.0:
            return weight_total * 30.0 if weight_total > 0.0 else 0.0
        radial = 3000.0 * math.pow(max(power, 50.0), 0.75) * ratio
        expander_type = _s(spec, "expander_type").lower()
        cost = (
            radial * 1.2
            if expander_type == "axial"
            else radial * 1.1
            if expander_type == "mixed-flow"
            else radial
        )
        if _b(spec, "cryogenic_service"):
            cost *= 1.4
        cost *= fm
        if _b(spec, "include_load", True):
            load_type = _s(spec, "load_type", "generator").lower()
            if load_type == "generator":
                cost += 400.0 * math.pow(power, 0.7) * ratio
            elif load_type == "compressor":
                cost += centrifugal_compressor(power, ratio)
            else:
                cost += 150.0 * math.pow(power, 0.6) * ratio
        if _b(spec, "include_gearbox"):
            cost += 200.0 * math.pow(max(power, 10.0), 0.65) * ratio
        if _b(spec, "include_lube_oil_system", True):
            cost += radial * 0.05
        if _b(spec, "include_control_system", True):
            cost += (15000.0 + 10.0 * power) * ratio
        return cost

    if kind == "heat_exchanger":
        area = _f(spec, "area_m2")
        if area <= 0.0:
            area = abs(_f(spec, "duty_kw")) * 1000.0 / (500.0 * 30.0)
        if area <= 0.0:
            return weight_total * 8.0
        exchanger_type = _s(spec, "exchanger_type").lower()
        if exchanger_type == "plate":
            cost = plate_heat_exchanger(area, ratio)
        elif exchanger_type in {"air-cooler", "aircooler"}:
            cost = air_cooler(area, ratio)
        elif exchanger_type == "double-pipe":
            cost = shell_tube_heat_exchanger(area, ratio) * 0.6
        else:
            cost = shell_tube_heat_exchanger(area, ratio)
        cost *= _tema_factor(_s(spec, "tema_type", "AES"))
        shell_passes = _f(spec, "shell_passes", 1.0)
        if shell_passes > 1.0:
            cost *= 1.0 + 0.1 * (shell_passes - 1.0)
        return cost

    if kind == "bahx":
        core_weight = _f(spec, "core_weight_kg")
        total_weight = _f(spec, "total_weight_kg")
        area = _f(spec, "area_m2")
        header_weight = total_weight - core_weight
        core_cost = core_weight * (8.0 + 25.0)
        header_cost = header_weight * (8.0 + 15.0)
        brazing = area * 120.0
        testing = (core_cost + header_cost) * 0.12
        equipment = (core_cost + header_cost + brazing + testing) * 1.15
        extras["equipment_cost"] = equipment
        extras["installed_cost"] = equipment * 3.5 * 1.10
        extras["specific_cost_per_m2"] = equipment / area if area > 0.0 else None
        extras["annual_maintenance_cost"] = equipment * 0.02
        return equipment

    if kind == "tank":
        diameter = _f(spec, "diameter_m")
        height = _f(spec, "height_m")
        volume = (
            math.pi / 4.0 * diameter * diameter * height
            if design_present
            else _f(spec, "volume_m3")
        )
        tank_type = _s(spec, "tank_type", "fixed-cone-roof")
        floating = _b(spec, "floating_roof") or tank_type.lower() == "floating-roof"
        design_pressure = _f(spec, "design_pressure_barg")
        if tank_type.lower() == "spherical":
            cost = 2500.0 * math.pow(max(volume, 50.0), 0.65)
            if design_pressure > 1.0:
                cost *= 1.0 + 0.05 * design_pressure
            base = cost * ratio
        elif tank_type.lower() == "horizontal":
            cost = 600.0 * math.pow(max(volume, 10.0), 0.55)
            if design_pressure > 0.0:
                cost *= pressure_factor(design_pressure)
            base = cost * ratio
        elif floating:
            base = 700.0 * math.pow(max(volume, 500.0), 0.52) * ratio
        else:
            cost = 475.0 * math.pow(max(volume, 100.0), 0.507)
            if tank_type.lower() == "fixed-dome-roof":
                cost *= 1.15
            base = cost * ratio
        base *= fm
        if 0.07 < design_pressure < 1.0:
            base *= 1.0 + design_pressure * 0.3
        if _b(spec, "include_foundation", True):
            foundation = math.pi / 4.0 * diameter * diameter * 300.0
            if volume > 10000.0:
                foundation *= 1.2
            base += foundation * ratio
        if _b(spec, "include_heating_coils"):
            base += volume * 50.0 * fm * ratio
        if _b(spec, "include_insulation"):
            surface = math.pi * diameter * height + 2.0 * math.pi / 4.0 * diameter * diameter
            insulation = surface * 80.0
            insulation *= 1.0 + (_f(spec, "insulation_thickness_mm", 100.0) - 50.0) / 100.0
            base += insulation * ratio
        return base

    if kind == "filter":
        diameter = _f(spec, "inner_diameter_m")
        length = _f(spec, "vessel_length_m")
        volume = (
            math.pi / 4.0 * diameter * diameter * length if diameter > 0.0 and length > 0.0 else 0.0
        )
        vessel = horizontal_vessel_by_volume(volume, ratio)
        elements = max(_f(spec, "required_elements"), 1.0) * _f(spec, "element_cost_usd", 450.0)
        correlation = (vessel + elements) * (1.0 + _f(spec, "auxiliaries_fraction", 0.35))
        fallback = _f(spec, "empty_vessel_weight_kg") * _f(
            spec, "fallback_steel_rate_usd_per_kg", 6.0
        ) + _f(spec, "required_elements") * _f(spec, "fallback_element_cost_usd", 2500.0)
        cost = max(correlation, fallback)
        cost *= _grade_factor(_s(spec, "material_grade"))
        cost *= pressure_factor(max(_f(spec, "design_pressure_bara") - 1.01325, 0.0))
        return cost

    if kind == "manifold":
        branches = max(_f(spec, "inlets") + _f(spec, "outlets"), 1.0)
        weight = _f(spec, "weight_kg")
        if weight <= 0.0:
            header_diameter = max(_f(spec, "header_diameter_m"), 0.1)
            branch_diameter = max(_f(spec, "branch_diameter_m"), 0.05)
            header_length = max(branches * 1.2, 4.0)
            header_thickness = max(header_diameter * 0.04, 0.008)
            branch_thickness = max(branch_diameter * 0.04, 0.006)
            header_weight = math.pi * header_diameter * header_thickness * header_length * 7850.0
            branch_weight = math.pi * branch_diameter * branch_thickness * 1.5 * 7850.0 * branches
            weight = (header_weight + branch_weight) * 2.2
        location = _s(spec, "location").lower()
        location_value = 2.6 if location == "subsea" else 0.75 if location == "onshore" else 1.0
        manifold_type = _s(spec, "manifold_type").lower()
        type_value = (
            1.2
            if manifold_type in {"pigging", "test"}
            else 1.1
            if manifold_type == "injection"
            else 1.0
        )
        header_diameter = _f(spec, "header_diameter_m")
        pressure_value = 1.8 if header_diameter >= 0.4 else 1.4 if header_diameter >= 0.25 else 1.0
        fabrication = (
            weight
            * _f(spec, "base_cost_per_kg", 65.0)
            * location_value
            * _grade_factor(_s(spec, "material_grade"))
            * type_value
        )
        branch = branches * _f(spec, "branch_allowance_usd", 18000.0) * pressure_value
        subsea = (
            250000.0 + max(_f(spec, "water_depth_m"), 0.0) * 350.0 if location == "subsea" else 0.0
        )
        return fabrication + branch + subsea

    if kind == "mercury_removal":
        steel = (
            _f(spec, "vessel_shell_weight_kg")
            + _f(spec, "internals_weight_kg")
            + _f(spec, "nozzle_weight_kg")
        )
        sorbent = _f(spec, "sorbent_charge_weight_kg") * _f(
            spec, "sorbent_unit_price_usd_per_kg", 25.0
        )
        extras["sorbent_replacement_cost"] = sorbent * 1.25
        return steel * _f(spec, "steel_cost_per_kg", 8.0) * fm + sorbent

    if kind == "electrolyzer":
        power = _f(spec, "power_kw")
        if power <= 0.0:
            return 0.0
        technology = _s(spec, "technology", "PEM").upper()
        specific = (
            800.0
            if technology == "ALKALINE"
            else 2500.0
            if technology == "SOEC"
            else 1500.0
            if technology == "AEM"
            else 1250.0
        )
        extras["specific_capex_per_kw"] = specific
        cost = power * specific * math.pow(power / 1000.0, -0.15)
        if not _b(spec, "include_balance_of_plant", True):
            cost *= 0.65
        return cost * (float(common["cepci"]) / 800.0)

    if kind == "psa":
        bed_volume = 12.566
        bed_diameter_value = spec.get("bed_diameter_m")
        bed_length_value = spec.get("bed_length_m")
        if (
            bed_diameter_value is not None
            and bed_length_value is not None
            and float(bed_diameter_value) > 0.0
            and float(bed_length_value) > 0.0
        ):
            bed_volume = (
                math.pi
                / 4.0
                * float(bed_diameter_value)
                * float(bed_diameter_value)
                * float(bed_length_value)
            )
        scale = math.pow(bed_volume / 12.566, 0.6)
        price = 10.0 if _s(spec, "sorbent", "ACTIVATED_CARBON").upper() == "ZEOLITE_13X" else 4.0
        per_bed = 250000.0 * scale + 60000.0 + _f(spec, "sorbent_mass_per_bed_kg") * price
        cost = _f(spec, "number_of_beds") * per_bed
        if not _b(spec, "include_balance_of_plant", True):
            cost *= 0.75
        return cost * (float(common["cepci"]) / 800.0)

    if kind == "ejector":
        compression = (
            (_f(spec, "discharge_pressure_bara") * 1000.0) / _f(spec, "suction_pressure_mbar")
            if _f(spec, "suction_pressure_mbar") > 0.0
            else 0.0
        )
        stages = _f(spec, "number_of_stages", 1.0)
        stage_factor = 1.0 + (stages - 1.0) * 0.7
        capacity = max(_f(spec, "suction_capacity_kg_per_h"), 10.0)

        def steam() -> float:
            return (
                500.0 * math.pow(capacity, 0.6) * math.pow(compression, 0.3) * stage_factor * ratio
            )

        ejector_type = _s(spec, "ejector_type", "steam").lower()
        if ejector_type == "gas":
            cost = steam() * 1.3
        elif ejector_type == "liquid":
            cost = 300.0 * math.pow(capacity, 0.5) * ratio
        elif ejector_type == "hybrid":
            cost = steam() + 800.0 * math.pow(capacity, 0.55) * ratio
        else:
            cost = steam()
        cost *= fm
        if _b(spec, "include_intercondensers") and stages > 1.0:
            cost += (stages - 1.0) * shell_tube_heat_exchanger(capacity * 0.1, ratio) * fm
        if _b(spec, "include_aftercondenser", True):
            cost += shell_tube_heat_exchanger(capacity * 0.15, ratio) * fm
        return cost

    if kind == "pipe":
        nominal = _f(spec, "nominal_diameter_in")
        schedule = _s(spec, "schedule", "40")
        cost = piping_cost(
            nominal * 0.0254,
            _f(spec, "length_m"),
            _pipe_schedule_for_cost(schedule),
            fm,
            ratio,
        )
        cost *= fm
        if _b(spec, "include_fittings", True):
            total = int(_f(spec, "fittings_per_hundred_m") * _f(spec, "length_m") / 100.0)
            average = 30.0 + 15.0 * math.pow(nominal, 0.8)
            cost += total * average * fm * ratio
        flanges = _f(spec, "number_of_flanges", 2.0)
        if flanges > 0.0:
            cost += flanges * (50.0 + 30.0 * math.pow(nominal, 1.2)) * fm * ratio
        installation = _s(spec, "installation_type", "above-ground").lower()
        cost *= 1.5 if installation == "buried" else 3.0 if installation == "subsea" else 1.0
        if _b(spec, "include_insulation"):
            surface = math.pi * nominal * 0.0254 * _f(spec, "length_m")
            insulation = surface * 30.0
            insulation *= 1.0 + _f(spec, "insulation_thickness_mm", 50.0) / 100.0
            cost += insulation * ratio
        extras["calculated_weight_kg"] = _pipe_weight(spec)
        return cost

    if kind == "valve":
        valve_type = _s(spec, "valve_type", "control").lower()
        if valve_type == "control":
            cost = control_valve(_f(spec, "valve_cv"), fm, ratio)
        else:
            gate = (40.0 + 20.0 * math.pow(_f(spec, "nominal_size_in"), 1.5)) * ratio
            cost = (
                gate
                if valve_type == "gate"
                else gate * 1.5
                if valve_type == "ball"
                else gate * 1.3
                if valve_type == "globe"
                else gate * 0.8
                if valve_type == "check"
                else gate * 0.6
                if valve_type == "butterfly"
                else control_valve(_f(spec, "valve_cv"), fm, ratio)
            )
        cost *= _valve_pressure_class_factor(_f(spec, "pressure_class", 300.0))
        cost *= fm
        if _b(spec, "include_actuator", True) and valve_type == "control":
            actuator = _s(spec, "actuator_type", "pneumatic").lower()
            nominal = _f(spec, "nominal_size_in")
            base = (
                500.0 + 100.0 * math.pow(nominal, 0.8)
                if actuator == "electric"
                else 800.0 + 150.0 * math.pow(nominal, 0.8)
                if actuator == "hydraulic"
                else 50.0 + 10.0 * nominal
                if actuator == "manual"
                else 300.0 + 80.0 * math.pow(nominal, 0.8)
            )
            cost += base * ratio
        return cost

    if kind == "mixer":
        pressure = _mixer_pressure_class_factor(_f(spec, "pressure_class", 300.0))
        header = spec.get("header_cost_usd")
        if header is not None and float(header) > 0.0:
            return float(header) * fm * pressure
        diameter = _f(spec, "pipe_diameter_in")
        elements = _f(spec, "number_of_elements", 6.0)
        mixer_type = _s(spec, "mixer_type", "static").lower()
        static = 100.0 * math.pow(diameter, 1.5) * math.pow(elements, 0.5) * ratio
        if mixer_type == "inline":
            cost = static * 2.5
        elif mixer_type == "tee":
            cost = (50.0 + 20.0 * math.pow(diameter, 1.2)) * ratio
        elif mixer_type == "vessel":
            diameter_m = diameter * 0.0254
            height_m = diameter_m * 2.0
            volume = math.pi / 4.0 * diameter_m * diameter_m * height_m
            cost = 500.0 * math.pow(max(volume * 1000.0, 100.0), 0.55) * ratio
        else:
            cost = static
        cost *= fm
        cost *= pressure
        if _b(spec, "flanged_connections", True):
            price = 50.0 + 30.0 * math.pow(diameter, 1.2)
            cost += 2.0 * price * fm * pressure * ratio
        return cost

    if kind == "splitter":
        pressure = _mixer_pressure_class_factor(_f(spec, "pressure_class", 300.0))
        header = spec.get("header_cost_usd")
        if header is not None and float(header) > 0.0:
            return float(header) * fm * pressure
        inlet = _f(spec, "inlet_diameter_in")
        outlet = _f(spec, "outlet_diameter_in")
        outlets = _f(spec, "number_of_outlets", 2.0)
        splitter_type = _s(spec, "splitter_type", "manifold").lower()

        def manifold() -> float:
            base = 200.0 + 50.0 * math.pow(inlet, 1.3)
            outlet_cost = outlets * 30.0 * math.pow(outlet, 1.2)
            return (base + outlet_cost) * ratio

        if splitter_type == "header":
            cost = manifold() * 1.3
        elif splitter_type == "tee":
            cost = (50.0 + 25.0 * math.pow(inlet, 1.2)) * ratio
        elif splitter_type == "vessel":
            diameter_m = inlet * 0.0254 * 2.0
            height_m = diameter_m * 1.5
            volume = math.pi / 4.0 * diameter_m * diameter_m * height_m
            vessel = 400.0 * math.pow(max(volume * 1000.0, 50.0), 0.55)
            nozzles = (1.0 + outlets) * 100.0 * math.pow(outlet, 0.8)
            cost = (vessel + nozzles) * ratio
        else:
            cost = manifold()
        cost *= fm
        cost *= pressure
        if _b(spec, "include_control_valves"):
            cost += outlets * control_valve(100.0 * outlet, fm, ratio) * fm * pressure
        if _b(spec, "include_flow_meters"):
            cost += outlets * (500.0 + 100.0 * outlet) * ratio
        return cost

    if kind == "well":
        return max(_f(spec, "well_capex_usd", 75.0e6), 0.0)

    raise ValueError(f"unknown equipment kind {kind!r}")


def estimate_equipment(spec: dict[str, Any], common: dict[str, Any]) -> dict[str, Any]:
    """Evaluate one equipment row."""
    kind = _s(spec, "kind")
    if kind not in EQUIPMENT_TYPES:
        raise ValueError(f"unknown equipment kind {kind!r}")
    equipment_type = EQUIPMENT_TYPES[kind]
    extras = _empty_extras()
    pec = _equipment_pec(spec, common, extras)
    fm = float(common["material_factor"])
    design_present = bool(common["design_present"])

    if kind == "well":
        stack = {
            "purchased_equipment_cost": pec,
            "bare_module_cost": pec,
            "total_module_cost": pec,
            "grass_roots_cost": pec,
            "installation_man_hours": 0.0,
        }
        total_cost = pec
    else:
        stack = equipment_stack(
            pec,
            design_present,
            float(common["max_design_pressure_barg"]),
            float(common["weight_total_kg"]),
            equipment_type,
            fm,
            float(common["contingency"]),
            float(common["engineering"]),
        )
        if kind in _MODULE_ONLY_TOTAL or stack["total_module_cost"] > 0.0:
            total_cost = stack["total_module_cost"]
        elif design_present:
            total_cost = float(common["weight_total_kg"]) * COST_PER_WEIGHT_UNIT
        else:
            total_cost = 0.0

    flags: list[str] = []
    if pec <= 0.0:
        flags.append("The purchased cost is zero; check the capacity basis.")
    return {
        "kind": kind,
        "equipment_type": equipment_type,
        "stack": stack,
        "total_cost": total_cost,
        "material_factor": fm,
        "extras": extras,
        "mto": [],
        "quality_flags": flags,
    }


def annual_operating_cost(
    spec: dict[str, Any], stack: dict[str, float], opex: dict[str, float]
) -> float:
    """The annual operating cost, for the kinds that override the base 5 % of PEC."""
    kind = _s(spec, "kind")
    pec = stack["purchased_equipment_cost"]
    hours = opex["hours_per_year"]
    if kind == "compressor":
        power = _f(spec, "power_kw")
        if power <= 0.0:
            return 0.0
        driver = _s(spec, "driver_type", "electric-motor").lower()
        if driver == "gas-turbine":
            return power / 0.30 * hours * 3600.0 / 1e9 * opex["fuel_cost_per_gj"]
        if driver == "gas-engine":
            return power / 0.38 * hours * 3600.0 / 1e9 * opex["fuel_cost_per_gj"]
        if driver == "steam-turbine":
            return power * hours * 3.0 * 10.0 / 1000.0
        return power / 0.95 * hours * opex["electricity_cost_per_kwh"]
    if kind == "pump":
        power = _f(spec, "power_kw")
        return 0.0 if power <= 0.0 else power / 0.9 * hours * opex["electricity_cost_per_kwh"]
    if kind == "expander":
        cost = pec * 0.03
        if _b(spec, "include_lube_oil_system", True):
            cost += _f(spec, "shaft_power_kw") / 1000.0 * 0.5 * hours * 3.0
        return cost
    if kind == "tank":
        cost = pec * 0.02
        if _b(spec, "include_heating_coils"):
            cost += _f(spec, "volume_m3") * 0.01 * hours * 0.08
        return cost
    if kind == "column":
        cost = 0.0
        if _b(spec, "include_reboiler") and _f(spec, "reboiler_duty_kw") > 0.0:
            steam_flow = _f(spec, "reboiler_duty_kw") * 3600.0 / 2760.0
            cost += steam_flow * hours / 1000.0 * opex["steam_cost_per_tonne"]
        if _b(spec, "include_condenser") and _f(spec, "condenser_duty_kw") > 0.0:
            flow = _f(spec, "condenser_duty_kw") * 3600.0 / (4.18 * 10.0)
            cost += flow * hours / 1000.0 * opex["cooling_water_cost_per_m3"]
        return cost
    if kind == "absorber":
        cost = 0.0
        if _b(spec, "include_reboiler"):
            steam = _f(spec, "reboiler_duty_kw") * 3600.0 / 2200.0
            cost += steam * hours * opex["steam_cost_per_tonne"] / 1000.0
        if _b(spec, "include_reflux_system"):
            cost += 10.0 * hours * opex["electricity_cost_per_kwh"]
        return cost + pec * 0.03
    if kind == "ejector":
        cost = 0.0
        if _s(spec, "ejector_type", "steam").lower() == "steam":
            compression = (
                _f(spec, "discharge_pressure_bara") * 1000.0 / _f(spec, "suction_pressure_mbar")
                if _f(spec, "suction_pressure_mbar") > 0.0
                else 0.0
            )
            factor = 3.0 + 2.0 * math.log(compression)
            steam = _f(spec, "suction_capacity_kg_per_h") * factor
            cost += steam * hours * opex["steam_cost_per_tonne"] / 1000.0
        if _b(spec, "include_intercondensers") or _b(spec, "include_aftercondenser", True):
            duty = _f(spec, "suction_capacity_kg_per_h") * 2.5
            flow = duty * 3600.0 / (4.18 * 10.0) / 1000.0
            cost += flow * hours * opex["cooling_water_cost_per_m3"]
        return cost + pec * 0.02
    if kind == "mercury_removal":
        replacement = (
            _f(spec, "sorbent_charge_weight_kg")
            * _f(spec, "sorbent_unit_price_usd_per_kg", 25.0)
            * 1.25
        )
        return stack["total_module_cost"] * 0.03 + replacement / 5.0
    return pec * 0.05


# ---------------------------------------------------------------------------
# Process roll-up.
# ---------------------------------------------------------------------------

_TYPE_CHAIN: tuple[tuple[tuple[str, ...], str], ...] = (
    (("Separator", "Scrubber"), "Vessels"),
    (("Compressor",), "Compressors"),
    (("Pump",), "Pumps"),
    (("Expander",), "Expanders"),
    (("HeatExchanger", "Cooler", "Heater"), "Heat Exchangers"),
    (("Valve",), "Valves"),
    (("Tank",), "Tanks"),
    (("Column", "Distillation", "Absorber"), "Columns"),
    (("Mixer", "Splitter"), "Piping"),
    (("Pipeline", "Pipe"), "Pipelines"),
    (("Well",), "Wells"),
    (("Reservoir",), "Reservoir"),
    (("Stream",), "Streams"),
)


def classify_equipment(equipment_class: str) -> str:
    """The class-name category, the class's ordered containment chain."""
    for needles, name in _TYPE_CHAIN:
        if any(needle in equipment_class for needle in needles):
            return name
    return "Other"


def _add_to_map(rows: list[tuple[str, float]], key: str, value: float) -> None:
    for index, (name, current) in enumerate(rows):
        if name == key:
            rows[index] = (name, current + value)
            return
    rows.append((key, value))


def estimate_process(input_data: dict[str, Any]) -> dict[str, Any]:
    """The process roll-up over per-unit rows."""
    equipment = list(input_data.get("equipment") or [])
    location = float(input_data.get("location_factor", 1.0))
    complexity = float(input_data.get("complexity_factor", 1.0))
    project_contingency = float(input_data.get("project_contingency", 0.10))
    working_capital = float(input_data.get("working_capital", 0.15))
    opex = input_data.get("opex")
    labor_rate = float(input_data.get("labor_cost_per_hour", 50.0))

    result: dict[str, Any] = {
        "total_purchased_equipment_cost": 0.0,
        "total_bare_module_cost": 0.0,
        "total_module_cost": 0.0,
        "total_grass_roots_cost": 0.0,
        "total_installation_man_hours": 0.0,
        "equipment": [],
        "cost_by_equipment_type": [],
        "cost_by_discipline": [],
        "project": {},
        "operating_costs": None,
        "annual_operating_cost": None,
        "payback_period_years": None,
        "roi_percent": None,
        "npv": None,
        "summed_unit_total_cost": 0.0,
        "weight_based_capex": None,
        "material_take_off": [],
        "quality_flags": [],
    }
    if not equipment:
        result["quality_flags"].append(
            "No equipment cost summaries were available. State at least one unit."
        )

    total_pec = total_bmc = total_tmc = total_man_hours = 0.0
    by_type: list[tuple[str, float]] = []
    for row in equipment:
        equipment_class = str(row["equipment_class"])
        equipment_type = classify_equipment(equipment_class)
        pec = float(row["purchased_equipment_cost"])
        bmc = float(row["bare_module_cost"])
        tmc = float(row["total_module_cost"])
        total_pec += pec
        total_bmc += bmc
        total_tmc += tmc
        total_man_hours += float(row["installation_man_hours"])
        located = pec * location
        _add_to_map(by_type, equipment_type, located)
        result["equipment"].append(
            {
                "name": str(row["name"]),
                "equipment_class": equipment_class,
                "equipment_type": equipment_type,
                "located_purchased_equipment_cost": located,
                "purchased_equipment_cost": pec,
                "bare_module_cost": bmc,
                "total_module_cost": tmc,
                "grass_roots_cost": float(row["grass_roots_cost"]),
                "total_cost": float(row["total_cost"]),
                "installation_man_hours": float(row["installation_man_hours"]),
                "weight": float(row["weight"]),
                "location_factor": location,
            }
        )
        result["summed_unit_total_cost"] += float(row["total_cost"])
        result["material_take_off"].append(
            {
                "item": str(row["name"]),
                "material": None,
                "quantity": float(row["weight"]),
                "unit": "kg",
                "weight_kg": float(row["weight"]),
                "cost_usd": located,
                "source": "process-cost-estimate",
            }
        )

    result["total_purchased_equipment_cost"] = total_pec * location
    result["total_bare_module_cost"] = total_bmc * location
    result["total_module_cost"] = total_tmc * location
    result["total_installation_man_hours"] = total_man_hours
    result["cost_by_equipment_type"] = by_type

    tmc_total = result["total_module_cost"]
    result["total_grass_roots_cost"] = (
        tmc_total + 0.10 * tmc_total + 0.05 * tmc_total
    ) * complexity
    result["cost_by_discipline"] = [
        ("Process Equipment", result["total_purchased_equipment_cost"]),
        ("Piping & Valves", result["total_bare_module_cost"] * 0.25 / 0.30),
        (
            "Electrical & Instrumentation",
            result["total_bare_module_cost"] * 0.15 / 0.30,
        ),
        ("Civil & Structural", result["total_bare_module_cost"] * 0.10 / 0.30),
        (
            "Other (HVAC, Paint, Insulation)",
            result["total_bare_module_cost"] * 0.20 / 0.30,
        ),
    ]

    direct = tmc_total
    indirect = max(result["total_grass_roots_cost"] - tmc_total, 0.0)
    contingency = (direct + indirect) * project_contingency
    owner = (direct + indirect) * working_capital
    result["project"] = {
        "direct_installed_cost": direct,
        "indirect_cost": indirect,
        "project_contingency": contingency,
        "owner_cost": owner,
        "total_project_cost": direct + indirect + contingency + owner,
    }

    if opex is not None:
        operating = {
            "electricity": 0.0,
            "steam": 0.0,
            "cooling_water": 0.0,
            "maintenance": 0.0,
            "operating_labor": 0.0,
            "supervision": 0.0,
            "supplies": 0.0,
            "laboratory": 0.0,
            "overhead": 0.0,
        }
        hours = float(opex["hours_per_year"])
        for row in equipment:
            equipment_class = str(row["equipment_class"])
            if ("Compressor" in equipment_class or "Pump" in equipment_class) and row.get(
                "power_kw"
            ) is not None:
                operating["electricity"] += (
                    float(row["power_kw"]) * hours * float(opex["electricity_cost_per_kwh"])
                )
            if ("Heater" in equipment_class or "Reboiler" in equipment_class) and row.get(
                "duty_kw"
            ) is not None:
                steam_flow = abs(float(row["duty_kw"])) * 3.6 / 2200.0
                operating["steam"] += (
                    steam_flow * hours * float(opex["steam_cost_per_tonne"]) / 1000.0
                )
            if ("Cooler" in equipment_class or "Condenser" in equipment_class) and row.get(
                "duty_kw"
            ) is not None:
                flow = abs(float(row["duty_kw"])) * 3.6 / (4.18 * 10.0) / 1000.0
                operating["cooling_water"] += (
                    flow * hours * float(opex["cooling_water_cost_per_m3"])
                )
        operating["maintenance"] = result["total_module_cost"] * 0.04
        major = len(equipment)
        operators = max(major // 5, 2)
        operating["operating_labor"] = operators * 3 * hours / 3.0 * labor_rate
        operating["supervision"] = operating["operating_labor"] * 0.18
        operating["supplies"] = operating["maintenance"] * 0.12
        operating["laboratory"] = operating["operating_labor"] * 0.15
        operating["overhead"] = (
            operating["operating_labor"] + operating["supervision"] + operating["maintenance"]
        ) * 0.55
        annual = sum(operating.values())
        result["operating_costs"] = operating
        result["annual_operating_cost"] = annual
        revenue = input_data.get("annual_revenue")
        if revenue is not None:
            profit = float(revenue) - annual
            result["payback_period_years"] = (
                result["total_grass_roots_cost"] / profit if profit > 0.0 else None
            )
            result["roi_percent"] = (
                0.0
                if result["total_grass_roots_cost"] <= 0.0
                else profit / result["total_grass_roots_cost"] * 100.0
            )
            npv = -result["total_grass_roots_cost"]
            discount = float(input_data.get("discount_rate", 0.10))
            for year in range(1, int(input_data.get("project_life_years", 10)) + 1):
                npv += profit / math.pow(1.0 + discount, year)
            result["npv"] = npv

    system_weight = input_data.get("system_total_weight")
    if system_weight is not None:
        result["weight_based_capex"] = float(system_weight) * float(
            input_data.get("capex_per_weight", 1000.0)
        )
    if result["total_purchased_equipment_cost"] <= 0.0:
        result["quality_flags"].append("No positive purchased equipment cost was available.")
    return result


# ---------------------------------------------------------------------------
# Topsides.
# ---------------------------------------------------------------------------

_FACILITY_FACTORS: dict[str, tuple[str, float, float, float, float, float]] = {
    "fixed_platform": ("Fixed platform", 0.18, 0.20, 0.12, 0.90, 0.18),
    "fpso": ("FPSO", 0.22, 0.14, 0.10, 0.70, 0.22),
    "semi_submersible": ("Semi-submersible", 0.24, 0.18, 0.12, 0.80, 0.22),
    "onshore": ("Onshore", 0.08, 0.02, 0.04, 0.35, 0.25),
    "brownfield_tie_in": ("Brownfield tie-in", 0.26, 0.12, 0.20, 0.45, 0.12),
}

_PROJECT_CONTEXT: dict[str, tuple[str, float]] = {
    "greenfield": ("Greenfield", 1.0),
    "brownfield_modification": ("Brownfield modification", 1.35),
    "host_tie_in": ("Host tie-in", 1.50),
}


def _classify_module(equipment_type: str) -> str:
    if "Compressor" in equipment_type or "Expander" in equipment_type:
        return "Compression and Rotating Equipment"
    if "Pump" in equipment_type:
        return "Pumping and Utility Packages"
    if "Heat Exchanger" in equipment_type:
        return "Heat Transfer and Cooling"
    if "Valve" in equipment_type or "Piping" in equipment_type or "Pipeline" in equipment_type:
        return "Piping and Flow Control"
    if "Column" in equipment_type or "Vessel" in equipment_type or "Tank" in equipment_type:
        return "Separation and Treatment Modules"
    return "Miscellaneous Process Packages"


def _normalize_key(label: str) -> str:
    parts = [
        part
        for part in "".join(
            character if character.isalnum() else " " for character in label
        ).split()
    ]
    if not parts:
        return "unknown"
    key = parts[0][0].lower() + parts[0][1:]
    for part in parts[1:]:
        key += part[0].upper() + part[1:]
    return key


def estimate_topsides(input_data: dict[str, Any]) -> dict[str, Any]:
    """The topsides facility stack over process rows."""
    equipment = list(input_data.get("equipment") or [])
    facility_key = str(input_data.get("facility_type", "fixed_platform"))
    (
        facility_display,
        module_integration,
        installation,
        hook_up,
        structural_factor,
        utility_factor,
    ) = _FACILITY_FACTORS[facility_key]
    context_key = str(input_data.get("project_context", "greenfield"))
    context_display, congestion = _PROJECT_CONTEXT[context_key]
    location = float(input_data.get("location_factor", 1.0))
    piping_bulk_factor = float(input_data.get("piping_bulk_factor", 0.25))
    electrical_factor = float(input_data.get("electrical_instrumentation_factor", 0.16))
    insulation_factor = float(input_data.get("insulation_paint_fireproofing_factor", 0.06))
    hvac_factor = float(input_data.get("hvac_telecom_safety_factor", 0.08))
    engineering_factor = float(input_data.get("engineering_procurement_factor", 0.14))
    construction_factor = float(input_data.get("construction_management_factor", 0.06))
    contingency_factor = float(input_data.get("project_contingency_factor", 0.15))
    owner_factor = float(input_data.get("owner_cost_factor", 0.10))
    include_utilities = bool(input_data.get("include_utilities_and_offsites", True))

    quality_flags: list[str] = []
    material_take_off: list[dict[str, Any]] = []
    module_costs: list[tuple[str, float]] = []
    equipment_weight = 0.0
    process_module_cost = 0.0
    excluded_scope_cost = 0.0

    for row in equipment:
        equipment_type = str(row["equipment_type"])
        if "Well" in equipment_type or "Reservoir" in equipment_type:
            excluded_scope_cost += float(row["total_module_cost"])
            quality_flags.append(
                "Excluded non-topsides item from topsides module build-up: " + str(row["name"])
            )
            continue
        located = float(row["total_module_cost"]) * location
        module = _classify_module(equipment_type)
        _add_to_map(module_costs, module, located)
        process_module_cost += located
        weight = max(float(row["weight"]), 0.0)
        equipment_weight += weight
        material_take_off.append(
            {
                "item": str(row["name"]),
                "material": equipment_type,
                "quantity": float(row["weight"]),
                "unit": "kg",
                "weight_kg": float(row["weight"]),
                "cost_usd": located,
                "source": "process-cost-estimate",
            }
        )
        if float(row["weight"]) <= 0.0 and float(row["total_module_cost"]) > 0.0:
            quality_flags.append("Equipment cost has no positive weight basis: " + str(row["name"]))

    breakdown = [("module." + _normalize_key(module), value) for module, value in module_costs]

    structural_weight = equipment_weight * structural_factor
    piping_weight = equipment_weight * 0.35
    structural_steel = structural_weight * 10.0 * location
    piping_bulk = process_module_cost * piping_bulk_factor
    electrical = process_module_cost * electrical_factor
    insulation = process_module_cost * insulation_factor
    hvac = process_module_cost * hvac_factor
    utility = process_module_cost * utility_factor if include_utilities else 0.0
    module_integration_cost = process_module_cost * module_integration
    bulk = structural_steel + piping_bulk + electrical + insulation + hvac + utility
    direct_fabricated = process_module_cost + bulk + module_integration_cost
    offshore_installation = direct_fabricated * installation * congestion
    hook_up_cost = direct_fabricated * hook_up * congestion
    direct_field = direct_fabricated + offshore_installation + hook_up_cost
    engineering = direct_field * engineering_factor
    construction = direct_field * construction_factor
    contingency = direct_field * contingency_factor
    owner = direct_field * owner_factor
    total = direct_field + engineering + construction + contingency + owner

    material_take_off.extend(
        [
            {
                "item": "Structural steel allowance",
                "material": "Carbon Steel",
                "quantity": structural_weight,
                "unit": "kg",
                "weight_kg": structural_weight,
                "cost_usd": structural_steel,
                "source": "topsides-allowance",
            },
            {
                "item": "Piping bulk allowance",
                "material": "Mixed piping classes",
                "quantity": piping_weight,
                "unit": "kg",
                "weight_kg": piping_weight,
                "cost_usd": piping_bulk,
                "source": "topsides-allowance",
            },
            {
                "item": "Electrical and instrumentation allowance",
                "material": "Cable, trays, instruments",
                "quantity": 1.0,
                "unit": "allowance",
                "weight_kg": None,
                "cost_usd": electrical,
                "source": "topsides-allowance",
            },
            {
                "item": "HVAC, telecom, fire and gas allowance",
                "material": "Package allowance",
                "quantity": 1.0,
                "unit": "allowance",
                "weight_kg": None,
                "cost_usd": hvac,
                "source": "topsides-allowance",
            },
        ]
    )

    quality_flags.extend(
        [
            "Facility type factor set: " + facility_display,
            "Project context factor set: " + context_display,
        ]
    )
    if process_module_cost <= 0.0:
        quality_flags.append("No positive topsides process module cost was calculated.")
    if equipment_weight <= 0.0:
        quality_flags.append(
            "No positive equipment dry weight was available; bulk MTO weights are allowance based."
        )
    if include_utilities:
        quality_flags.append(
            "Utilities and offsites are factor allowances unless explicit utility "
            "modules are included."
        )
    if context_key != "greenfield":
        quality_flags.append(
            "Brownfield or host tie-in congestion factor applied; verify with "
            "site-specific survey data."
        )

    return {
        "capital_costs": [
            ("processEquipmentModules", process_module_cost),
            ("structuralSteel", structural_steel),
            ("pipingBulk", piping_bulk),
            ("electricalAndInstrumentation", electrical),
            ("insulationPaintFireproofing", insulation),
            ("hvacTelecomAndSafetySystems", hvac),
            ("utilitiesAndOffsites", utility),
            ("moduleFabricationAndIntegration", module_integration_cost),
            ("offshoreInstallation", offshore_installation),
            ("hookUpAndCommissioning", hook_up_cost),
        ],
        "capital_cost_summary": [
            ("directFieldCost", direct_field),
            ("excludedNonTopsidesScope", excluded_scope_cost * location),
        ],
        "capital_cost_breakdown": breakdown,
        "project_costs": [
            ("engineeringProcurementProjectManagement", engineering),
            ("constructionManagement", construction),
            ("projectContingency", contingency),
            ("ownerCost", owner),
        ],
        "project_cost_summary": [("totalTopsidesCapex", total)],
        "weight_basis": [
            ("equipmentDryWeight", equipment_weight),
            ("structuralSteelWeight", structural_weight),
            ("pipingBulkWeight", piping_weight),
            (
                "totalEstimatedDryWeight",
                equipment_weight + structural_weight + piping_weight,
            ),
        ],
        "material_take_off": material_take_off,
        "quality_flags": quality_flags,
    }
