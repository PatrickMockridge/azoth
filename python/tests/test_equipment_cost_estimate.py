"""``process.equipment_cost_estimate`` against its capture."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError, OutOfRangeError
from azoth.core.units import quantity
from azoth.process.reference.equipment_cost_estimate import equipment_cost_estimate


def close(actual: float, expected: float) -> bool:
    return abs(actual - expected) <= 1.0e-12 * max(abs(expected), 1.0)


def hours(value: object) -> float:
    return float(value.to("h").magnitude)  # type: ignore[attr-defined]


def test_a_trayed_column_matches_the_capture() -> None:
    result = equipment_cost_estimate(
        [
            {
                "equipment": "column",
                "name": "Depropaniser",
                "column_type": "trayed",
                "tray_type": "sieve",
                "diameter": quantity(2.0, "m"),
                "height": quantity(25.0, "m"),
                "number_of_trays": 30.0,
                "design_pressure_barg": quantity(15.0, "bar"),
                "include_reboiler": True,
                "include_condenser": True,
                "reboiler_duty": quantity(2000.0, "kW"),
                "condenser_duty": quantity(1500.0, "kW"),
            }
        ]
    )
    row = result.equipment[0]
    assert row.name == "Depropaniser"
    assert row.equipment_type == "column"
    assert close(row.purchased_equipment_cost, 186272.12543695548)
    assert row.bare_module_cost == 0.0
    assert row.total_cost == 0.0
    assert hours(row.installation_man_hours) == 0.0
    assert result.material_take_off == ()
    assert result.quality_flags == ()


def test_the_stainless_column_carries_the_material_factor_twice() -> None:
    carbon = {
        "equipment": "column",
        "diameter": quantity(2.0, "m"),
        "height": quantity(25.0, "m"),
        "number_of_trays": 30.0,
        "design_pressure_barg": quantity(15.0, "bar"),
        "include_reboiler": True,
        "include_condenser": True,
        "reboiler_duty": quantity(2000.0, "kW"),
        "condenser_duty": quantity(1500.0, "kW"),
    }
    stainless = dict(carbon, material="ss316")
    result = equipment_cost_estimate([carbon, stainless])
    assert close(result.equipment[1].purchased_equipment_cost, 391171.4634176065)
    assert close(result.equipment[1].material_factor, 2.1)
    assert close(
        result.equipment[1].purchased_equipment_cost / result.equipment[0].purchased_equipment_cost,
        2.1,
    )


def test_the_packed_and_zero_columns_match_the_capture() -> None:
    result = equipment_cost_estimate(
        [
            {
                "equipment": "column",
                "column_type": "packed",
                "packing_type": "structured",
                "diameter": quantity(1.5, "m"),
                "height": quantity(15.0, "m"),
                "packing_height": quantity(10.0, "m"),
                "design_pressure_barg": quantity(15.0, "bar"),
                "include_reboiler": True,
                "include_condenser": True,
                "reboiler_duty": quantity(1000.0, "kW"),
                "condenser_duty": quantity(800.0, "kW"),
            },
            {
                "equipment": "column",
                "diameter": quantity(0.0, "m"),
                "height": quantity(0.0, "m"),
                "include_reboiler": False,
                "include_condenser": False,
            },
        ]
    )
    assert close(result.equipment[0].purchased_equipment_cost, 149981.67037473136)
    assert result.equipment[1].purchased_equipment_cost == 0.0
    assert len(result.quality_flags) == 1
    assert result.quality_flags[0].equipment_index == 1.0


def test_the_absorber_states_match_the_capture() -> None:
    result = equipment_cost_estimate(
        [
            {
                "equipment": "absorber",
                "absorber_type": "packed",
                "packing_type": "structured",
                "diameter": quantity(2.0, "m"),
                "height": quantity(15.0, "m"),
                "packing_height": quantity(10.0, "m"),
                "design_pressure_barg": quantity(60.0, "bar"),
            },
            {
                "equipment": "absorber",
                "absorber_type": "trayed",
                "tray_type": "valve",
                "diameter": quantity(2.0, "m"),
                "height": quantity(15.0, "m"),
                "number_of_stages": 12.0,
                "design_pressure_barg": quantity(60.0, "bar"),
                "include_liquid_distributor": False,
                "include_mist_eliminator": False,
            },
        ]
    )
    assert close(result.equipment[0].purchased_equipment_cost, 155322.29397076587)
    assert close(result.equipment[1].purchased_equipment_cost, 62090.37414978155)


def test_the_separator_carries_its_stack_and_bill_of_materials() -> None:
    result = equipment_cost_estimate(
        [
            {
                "equipment": "separator",
                "name": "InletSeparator",
                "inner_diameter": quantity(0.8361865022733448, "m"),
                "tangent_length": quantity(1.6723730045466896, "m"),
                "mechanical_design_present": True,
                "max_design_pressure": quantity(110.00000000000001, "bar"),
                "weight_total": quantity(3315.9396795551834, "kg"),
                "include_material_take_off": True,
                "shell_weight": quantity(2098.6959997184704, "kg"),
                "piping_weight": quantity(839.4783998873882, "kg"),
                "electro_instrument_weight": quantity(167.89567997747764, "kg"),
                "structural_weight": quantity(209.86959997184704, "kg"),
            }
        ]
    )
    row = result.equipment[0]
    assert close(row.purchased_equipment_cost, 4185.028871129008)
    assert close(row.bare_module_cost, 17231.85637687369)
    assert close(row.total_module_cost, 21539.820471092113)
    assert close(row.grass_roots_cost, 32309.73070663817)
    assert close(hours(row.installation_man_hours), 99.47819038665551)
    assert len(result.material_take_off) == 4
    assert close(result.material_take_off[0].cost_usd, 16789.567997747763)
    assert result.material_take_off[3].item == "Structural Steel"
    assert result.quality_flags == ()


def test_the_estimate_refuses_what_it_cannot_express() -> None:
    with pytest.raises(InvalidInputError):
        equipment_cost_estimate([{"equipment": "reactor"}])
    with pytest.raises(InvalidInputError):
        equipment_cost_estimate([{"equipment": "column", "height": quantity(10.0, "m")}])
    with pytest.raises(InvalidInputError):
        equipment_cost_estimate([{"equipment": "separator"}])
    with pytest.raises(InvalidInputError):
        equipment_cost_estimate(
            [
                {
                    "equipment": "column",
                    "diameter": quantity(1.0, "m"),
                    "height": quantity(10.0, "m"),
                    "material": "unobtainium",
                }
            ]
        )
    with pytest.raises(InvalidInputError):
        equipment_cost_estimate(
            [
                {
                    "equipment": "column",
                    "diameter": quantity(1.0, "m"),
                    "height": quantity(10.0, "m"),
                    "cepci": 0.0,
                }
            ]
        )
    with pytest.raises(InvalidInputError):
        equipment_cost_estimate(
            [
                {
                    "equipment": "column",
                    "diameter": quantity(1.0, "m"),
                    "height": quantity(10.0, "m"),
                    "number_of_trays": 20.5,
                }
            ]
        )
    with pytest.raises((InvalidInputError, OutOfRangeError)):
        equipment_cost_estimate(
            [{"equipment": "separator", "vessel_volume": quantity(float("inf"), "m**3")}]
        )
