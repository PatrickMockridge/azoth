"""The shared cost arithmetic, held to the same identities as the Rust twin."""

from __future__ import annotations

import math

import pytest

from azoth.process.reference import _costing as costing


def close(actual: float, expected: float) -> bool:
    return abs(actual - expected) <= 1.0e-9 * max(abs(expected), 1.0)


def common(material: str = "Carbon Steel", design_present: bool = True) -> dict[str, object]:
    return {
        "material_factor": costing.material_factor(material),
        "cepci": costing.CEPCI_2025,
        "design_present": design_present,
        "max_design_pressure_barg": 10.0,
        "weight_total_kg": 5000.0,
        "contingency": 0.15,
        "engineering": 0.10,
    }


def test_factor_tables_are_the_classes() -> None:
    assert costing.pressure_factor(5.0) == 1.0
    assert costing.pressure_factor(5.0001) == 1.05
    assert costing.pressure_factor(20.0) == 1.15
    assert costing.pressure_factor(50.0) == 1.30
    assert costing.pressure_factor(200.0) == 1.90
    assert costing.pressure_factor(201.0) == 2.5

    assert costing.material_factor("Carbon Steel") == 1.0
    assert costing.material_factor("SS316") == 2.1
    assert costing.material_factor("titanium") == 4.5
    assert costing.material_factor("unobtainium") is None

    assert costing.location_factor("US Gulf Coast") == 1.0
    assert costing.location_factor("North Sea / Norway") == 1.35
    assert costing.location_factor("China") == 0.75
    assert costing.location_factor("Middle East") == 1.10

    assert costing.exchange_rate("USD") == 1.0
    assert costing.exchange_rate("EUR") == 0.92
    assert costing.exchange_rate("NOK") == 11.0
    assert costing.exchange_rate("XXX") is None

    assert costing.cepci_for_year(2019) == 607.5
    assert costing.cepci_for_year(2025) == 840.0
    assert costing.cepci_for_year(2018) is None


def test_vessel_correlations_hold_their_clamps() -> None:
    assert close(
        costing.vertical_vessel_by_volume(0.1, 1.0),
        costing.vertical_vessel_by_volume(0.3, 1.0),
    )
    assert close(
        costing.vertical_vessel_by_volume(600.0, 1.0),
        costing.vertical_vessel_by_volume(520.0, 1.0),
    )
    log_v = math.log10(100.0)
    expected = math.pow(10.0, 3.5565 + 0.3776 * log_v + 0.0905 * log_v * log_v)
    assert close(costing.horizontal_vessel_by_volume(100.0, 1.0), expected)


def test_parallel_units_split_a_large_duty() -> None:
    assert close(
        costing.centrifugal_compressor(6000.0, 1.0),
        2.0 * costing.centrifugal_compressor(3000.0, 1.0),
    )
    assert close(
        costing.centrifugal_pump(600.0, 1.0),
        2.0 * costing.centrifugal_pump(300.0, 1.0),
    )
    assert costing.centrifugal_pump(0.0, 1.0) == 0.0


def test_the_module_stack_is_the_classes() -> None:
    bmc = costing.bare_module(1000.0, 1.05, 2.1)
    assert close(bmc, 1000.0 * (1.89 + 1.35 * 1.05 * 2.1))
    tmc = costing.total_module(bmc, 0.15, 0.10)
    assert close(tmc, bmc * 1.25)
    assert close(costing.grass_roots(tmc), tmc * 1.5)

    stack = costing.equipment_stack(1000.0, True, 10.0, 5000.0, "vessel", 2.1, 0.15, 0.10)
    assert close(stack["bare_module_cost"], bmc)
    assert close(stack["total_module_cost"], tmc)
    assert close(stack["installation_man_hours"], 150.0)

    no_design = costing.equipment_stack(1000.0, False, 10.0, 5000.0, "vessel", 2.1, 0.15, 0.10)
    assert no_design["purchased_equipment_cost"] == 1000.0
    assert no_design["bare_module_cost"] == 0.0

    summary = costing.calculator_summary(1000.0, 10.0, 5000.0, "vessel", 2.1, 1.35, 0.15, 0.10)
    pec = 1000.0 * 2.1 * 1.35
    assert close(summary["purchased_equipment_cost"], pec)
    assert close(summary["bare_module_cost"], pec * (1.89 + 1.35 * 1.05))


def test_a_column_carries_the_material_factor_twice() -> None:
    spec = {
        "kind": "column",
        "column_type": "trayed",
        "tray_type": "sieve",
        "packing_type": "metal",
        "diameter_m": 2.0,
        "height_m": 20.0,
        "number_of_trays": 30.0,
        "packing_height_m": 0.0,
        "design_pressure_barg": 15.0,
        "include_reboiler": True,
        "include_condenser": True,
        "reboiler_duty_kw": 2000.0,
        "condenser_duty_kw": 1500.0,
    }
    carbon = costing.estimate_equipment(spec, common("Carbon Steel"))
    stainless = costing.estimate_equipment(spec, common("SS316"))
    assert close(
        stainless["stack"]["purchased_equipment_cost"],
        carbon["stack"]["purchased_equipment_cost"] * 2.1,
    )
    assert close(
        stainless["stack"]["bare_module_cost"] / stainless["stack"]["purchased_equipment_cost"],
        1.89 + 1.35 * 1.05 * 2.1,
    )


def test_a_control_valve_carries_the_material_factor_three_times() -> None:
    spec = {
        "kind": "valve",
        "valve_type": "control",
        "nominal_size_in": 4.0,
        "valve_cv": 150.0,
        "pressure_class": 600.0,
        "include_actuator": False,
    }
    carbon = costing.estimate_equipment(spec, common("Carbon Steel"))
    stainless = costing.estimate_equipment(spec, common("SS316"))
    assert close(
        stainless["stack"]["purchased_equipment_cost"],
        carbon["stack"]["purchased_equipment_cost"] * 2.1 * 2.1,
    )


def test_a_well_is_all_in_and_an_unknown_kind_is_refused() -> None:
    result = costing.estimate_equipment(
        {"kind": "well", "well_capex_usd": 80.0e6}, common(design_present=False)
    )
    assert result["stack"]["purchased_equipment_cost"] == 80.0e6
    assert result["stack"]["bare_module_cost"] == 80.0e6
    assert result["total_cost"] == 80.0e6
    with pytest.raises(ValueError):
        costing.estimate_equipment({"kind": "unobtainium"}, common())


def test_the_process_roll_up_is_the_classes() -> None:
    rows = [
        {
            "name": "InletSeparator",
            "equipment_class": "Separator",
            "purchased_equipment_cost": 100_000.0,
            "bare_module_cost": 300_000.0,
            "total_module_cost": 375_000.0,
            "grass_roots_cost": 562_500.0,
            "total_cost": 375_000.0,
            "installation_man_hours": 100.0,
            "weight": 5000.0,
        },
        {
            "name": "MainCompressor",
            "equipment_class": "Compressor",
            "purchased_equipment_cost": 200_000.0,
            "bare_module_cost": 600_000.0,
            "total_module_cost": 750_000.0,
            "grass_roots_cost": 1_125_000.0,
            "total_cost": 750_000.0,
            "installation_man_hours": 200.0,
            "weight": 10_000.0,
            "power_kw": 1000.0,
        },
    ]
    result = costing.estimate_process(
        {
            "equipment": rows,
            "location_factor": 1.35,
            "complexity_factor": 1.2,
            "opex": {
                "hours_per_year": 8000.0,
                "electricity_cost_per_kwh": 0.10,
                "steam_cost_per_tonne": 15.0,
                "cooling_water_cost_per_m3": 0.05,
                "fuel_cost_per_gj": 5.0,
            },
            "annual_revenue": 5_000_000.0,
        }
    )
    assert close(result["total_purchased_equipment_cost"], 300_000.0 * 1.35)
    assert close(result["total_module_cost"], 1_125_000.0 * 1.35)
    assert close(result["total_grass_roots_cost"], (1_125_000.0 * 1.35 * 1.15) * 1.2)
    assert close(result["total_installation_man_hours"], 300.0)
    assert len(result["cost_by_equipment_type"]) == 2
    project = result["project"]
    assert close(
        project["total_project_cost"],
        project["direct_installed_cost"]
        + project["indirect_cost"]
        + project["project_contingency"]
        + project["owner_cost"],
    )
    operating = result["operating_costs"]
    assert close(operating["electricity"], 1000.0 * 8000.0 * 0.10)
    assert close(operating["maintenance"], result["total_module_cost"] * 0.04)
    assert result["payback_period_years"] is not None
    assert result["npv"] is not None


def test_the_topsides_stack_is_the_classes() -> None:
    result = costing.estimate_topsides(
        {
            "equipment": [
                {
                    "name": "InletSeparator",
                    "equipment_type": "Vessels",
                    "total_module_cost": 100_000.0,
                    "weight": 5000.0,
                },
                {
                    "name": "Well1",
                    "equipment_type": "Wells",
                    "total_module_cost": 80_000_000.0,
                    "weight": 0.0,
                },
            ],
            "facility_type": "fixed_platform",
            "project_context": "greenfield",
            "location_factor": 1.35,
        }
    )
    assert close(result["capital_costs"][0][1], 135_000.0)
    assert close(result["capital_cost_summary"][1][1], 80_000_000.0 * 1.35)
    assert ("module.separationAndTreatmentModules", 135_000.0) in result["capital_cost_breakdown"]
    assert len(result["material_take_off"]) == 5
