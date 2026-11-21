"""``process.cost_estimation`` against its capture."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.process.reference.cost_estimation import cost_estimation


def close(actual: float, expected: float) -> bool:
    return abs(actual - expected) <= 1.0e-12 * max(abs(expected), 1.0)


def hours(value: object) -> float:
    return float(value.to("h").magnitude)  # type: ignore[attr-defined]


def test_factor_boundaries_match_the_capture() -> None:
    result = cost_estimation(
        [
            {"calculation": "pressure_factor", "design_pressure": quantity(5.0, "bar")},
            {"calculation": "pressure_factor", "design_pressure": quantity(50.0, "bar")},
            {"calculation": "material_factor", "material": "ss316"},
            {"calculation": "location_factor", "region": "north_sea"},
            {"calculation": "currency_conversion", "amount": 1.0e6, "currency": "eur"},
            {"calculation": "piping_man_hours", "weight": quantity(5000.0, "kg")},
        ]
    )
    rows = result.queries
    assert close(rows[0].pressure_factor, 1.0)
    assert close(rows[1].pressure_factor, 1.3)
    assert close(rows[2].material_factor, 2.1)
    assert close(rows[3].location_factor, 1.35)
    assert close(rows[4].exchange_rate, 0.92)
    assert close(rows[4].converted_amount, 920_000.0)
    assert close(rows[4].cost_in_usd, 1_000_000.0)
    assert close(hours(rows[5].installation_man_hours), 500.0)
    assert close(rows[0].installation_factor, 3.0)


def test_correlations_match_the_capture() -> None:
    result = cost_estimation(
        [
            {
                "calculation": "correlation",
                "correlation": "vertical_vessel_by_volume",
                "capacity": 100.0,
            },
            {"calculation": "correlation", "correlation": "shell_tube", "capacity": 50.0},
            {
                "calculation": "correlation",
                "correlation": "centrifugal_compressor",
                "capacity": 1500.0,
            },
            {
                "calculation": "correlation",
                "correlation": "piping",
                "diameter": quantity(0.1524, "m"),
                "piping_length": quantity(100.0, "m"),
                "piping_schedule": "80",
                "material": "ss316",
            },
            {
                "calculation": "correlation",
                "correlation": "control_valve",
                "valve_cv": 150.0,
                "material": "ss316",
            },
        ]
    )
    rows = result.queries
    assert close(rows[0].purchased_equipment_cost, 92200.44216287117)
    assert close(rows[1].purchased_equipment_cost, 26442.288629302795)
    assert close(rows[2].purchased_equipment_cost, 519033.7321337709)
    assert close(rows[3].purchased_equipment_cost, 320066.6597236687)
    assert close(rows[4].purchased_equipment_cost, 114220.09043285412)


def test_the_module_stack_and_summary_match_the_capture() -> None:
    result = cost_estimation(
        [
            {
                "calculation": "module_stack",
                "purchased_cost": 100_000.0,
                "design_pressure": quantity(10.0, "bar"),
                "material": "ss316",
            },
            {
                "calculation": "summary",
                "purchased_cost": 50_000.0,
                "design_pressure": quantity(50.0, "bar"),
                "weight": quantity(1000.0, "kg"),
                "equipment_type": "vessel",
            },
            {
                "calculation": "installation_man_hours",
                "weight": quantity(5000.0, "kg"),
                "equipment_type": "compressor",
            },
        ]
    )
    rows = result.queries
    assert close(rows[0].pressure_factor, 1.05)
    assert close(rows[0].bare_module_cost, 486675.00000000006)
    assert close(rows[0].total_module_cost, 608343.7500000001)
    assert close(rows[0].grass_roots_cost, 912515.6250000002)
    assert close(rows[1].purchased_equipment_cost, 50_000.0)
    assert close(rows[1].bare_module_cost, 182_250.0)
    assert close(rows[1].total_module_cost, 227_812.5)
    assert close(rows[1].grass_roots_cost, 341_718.75)
    assert close(hours(rows[1].installation_man_hours), 30.0)
    assert close(hours(rows[2].installation_man_hours), 300.0)


def test_the_vessel_bom_matches_the_capture() -> None:
    result = cost_estimation(
        [
            {
                "calculation": "vessel_bom",
                "shell_weight": quantity(1000.0, "kg"),
                "heads_weight": quantity(200.0, "kg"),
                "nozzle_count": 4.0,
                "internals_weight": quantity(300.0, "kg"),
                "material": "carbon_steel",
            }
        ]
    )
    rows = result.bom
    assert len(rows) == 4
    assert rows[0].item == "Vessel Shell"
    assert close(rows[0].cost_usd, 535037.5394461319)
    assert rows[0].has_weight
    assert not rows[0].has_quantity
    assert close(rows[1].cost_usd, 1600.0)
    assert rows[2].item == "Nozzles"
    assert rows[2].has_quantity
    assert not rows[2].has_weight
    assert close(rows[3].cost_usd, 4500.0)


def test_the_calculator_refuses_what_the_class_cannot_express() -> None:
    with pytest.raises(InvalidInputError):
        cost_estimation([{"calculation": "nonsense", "capacity": 1.0}])
    with pytest.raises(InvalidInputError):
        cost_estimation(
            [{"calculation": "correlation", "correlation": "vertical_vessel_by_volume"}]
        )
    with pytest.raises(InvalidInputError):
        cost_estimation(
            [
                {
                    "calculation": "correlation",
                    "correlation": "sieve_trays",
                    "diameter": quantity(2.0, "m"),
                    "tray_count": 20.5,
                }
            ]
        )
    with pytest.raises(InvalidInputError):
        cost_estimation([{"calculation": "material_factor", "current_cepci": 0.0}])
    with pytest.raises(InvalidInputError):
        cost_estimation([{"calculation": "material_factor", "material": "unobtainium"}])
    with pytest.raises(InvalidInputError):
        cost_estimation([{"calculation": "module_stack", "purchased_cost": float("inf")}])
    with pytest.raises(InvalidInputError):
        cost_estimation([{"calculation": "module_stack", "purchased_cost": -1.0}])
