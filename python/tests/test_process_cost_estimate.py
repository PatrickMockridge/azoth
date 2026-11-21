"""``process.process_cost_estimate`` against its capture."""

from __future__ import annotations

from azoth.core.units import quantity
from azoth.process.reference.process_cost_estimate import process_cost_estimate


def close(actual: float, expected: float) -> bool:
    return abs(actual - expected) <= 1.0e-12 * max(abs(expected), 1.0)


def hours(value: object) -> float:
    return float(value.to("h").magnitude)  # type: ignore[attr-defined]


def unit(
    name: str,
    equipment_class: str,
    pec: float,
    bmc: float,
    tmc: float,
    grc: float,
    man_hours: float,
    weight: float,
    power: float | None = None,
    duty: float | None = None,
) -> dict[str, object]:
    row: dict[str, object] = {
        "name": name,
        "equipment_class": equipment_class,
        "purchased_equipment_cost": pec,
        "bare_module_cost": bmc,
        "total_module_cost": tmc,
        "grass_roots_cost": grc,
        "total_cost": tmc,
        "installation_man_hours": quantity(man_hours, "h"),
        "weight": quantity(weight, "kg"),
    }
    if power is not None:
        row["power"] = quantity(power, "kW")
    if duty is not None:
        row["duty"] = quantity(duty, "kW")
    return row


def test_the_class_process_matches_the_capture() -> None:
    result = process_cost_estimate(
        [
            unit("Feed", "Stream", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            unit(
                "InletSeparator",
                "Separator",
                2710.0997797401924,
                11158.835843080242,
                13948.544803850302,
                20922.817205775453,
                10.761660837894372,
                358.7220279298124,
            ),
            unit(
                "MainCompressor",
                "Compressor",
                236944.74435086618,
                975619.9848646914,
                1219524.9810808643,
                1829287.4716212964,
                36086.44083680164,
                601440.6806133607,
                power=243.39509113104845,
            ),
            unit(
                "AfterCooler",
                "Cooler",
                1056.0,
                4348.08,
                5435.1,
                8152.650000000001,
                5.28,
                132.0,
                duty=-339531.601308147,
            ),
            unit(
                "JTValve",
                "ThrottlingValve",
                52972.234870373686,
                218113.17707876363,
                272641.47134845454,
                408962.20702268183,
                2.2539496985037624,
                64.3985628143932,
            ),
        ],
        location_factor=1.0,
        complexity_factor=1.0,
        project_contingency=0.10,
        working_capital=0.15,
        system_total_weight=quantity(601995.8012041049, "kg"),
        capex_per_weight=1000.0,
        operating_hours=quantity(8000.0, "h"),
        electricity_cost_per_kwh=0.08,
        steam_cost_per_tonne=15.0,
        cooling_water_cost_per_m3=0.05,
        labor_cost_per_hour=50.0,
        annual_revenue=5_000_000.0,
        discount_rate=0.10,
        project_life_years=10.0,
    )
    assert close(result.total_purchased_equipment_cost, 293683.07900098007)
    assert close(result.total_bare_module_cost, 1209240.0777865353)
    assert close(result.total_module_cost, 1511550.0972331692)
    assert close(result.total_grass_roots_cost, 1738282.6118181446)
    assert close(hours(result.total_installation_man_hours), 36104.73644733804)
    assert len(result.equipment) == 5
    assert result.equipment[1].equipment_type == "Vessels"
    assert close(result.equipment[2].located_purchased_equipment_cost, 236944.74435086618)
    assert len(result.cost_by_equipment_type) == 5
    assert len(result.cost_by_discipline) == 5
    assert len(result.project_costs) == 5
    assert close(result.total_project_cost, 2172853.2647726806)
    assert close(result.operating_costs[0].cost, 155772.858323871)
    assert close(result.operating_costs[3].cost, 60462.00388932677)
    assert close(result.total_annual_operating_cost, 1851641.1872564566)
    assert close(result.payback_period_years, 0.5521234126117187)
    assert close(result.roi_percent, 181.118926884423)
    assert close(result.npv, 1.7607019385921277e7)
    assert close(result.summed_unit_total_cost, 1511550.0972331692)
    assert close(result.weight_based_capex, 6.019958012041049e8)
    assert len(result.material_take_off) == 5
    assert result.has_operating_cost
    assert result.has_economics
    assert result.has_weight_based_capex
    assert result.quality_flags == ()


def test_an_empty_roll_up_carries_its_flag_and_zero_totals() -> None:
    result = process_cost_estimate([], operating_hours=quantity(8000.0, "h"))
    assert result.total_purchased_equipment_cost == 0.0
    assert result.equipment == ()
    assert len(result.quality_flags) == 2
    assert result.has_operating_cost
    assert not result.has_economics
    assert not result.has_weight_based_capex
    assert result.payback_period_years == 0.0
    assert close(
        result.total_annual_operating_cost,
        800_000.0 + 144_000.0 + 120_000.0 + 519_200.0,
    )


def test_the_location_and_complexity_factors_scale_the_totals() -> None:
    row = unit(
        "InletSeparator",
        "Separator",
        100_000.0,
        300_000.0,
        375_000.0,
        562_500.0,
        100.0,
        5000.0,
    )
    base = process_cost_estimate([row])
    located = process_cost_estimate([row], location_factor=1.35, complexity_factor=1.2)
    assert close(
        located.total_purchased_equipment_cost,
        base.total_purchased_equipment_cost * 1.35,
    )
    assert close(
        located.total_grass_roots_cost,
        base.total_grass_roots_cost * 1.35 * 1.2,
    )
