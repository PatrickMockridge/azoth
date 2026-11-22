"""``pvt.constant_volume_depletion`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import constant_volume_depletion

ROWS = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.70, "mol")},
    {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.30, "mol")},
]
PRESSURES = [
    quantity(400.0, "bar"),
    quantity(350.0, "bar"),
    quantity(300.0, "bar"),
    quantity(250.0, "bar"),
    quantity(200.0, "bar"),
    quantity(150.0, "bar"),
    quantity(100.0, "bar"),
    quantity(50.0, "bar"),
]


def test_the_saturation_and_the_restored_points_answer() -> None:
    result = constant_volume_depletion(ROWS, temperature=quantity(323.15, "K"), pressures=PRESSURES)
    assert result.saturation_pressure.to("Pa").magnitude == pytest.approx(
        2.8955063438415527e7, rel=1.0e-4
    )
    assert result.cumulative_moles_removed.to("mol").magnitude == pytest.approx(
        0.5597746791158182, rel=1.0e-4
    )

    point = result.points[3]
    assert point.relative_volume == pytest.approx(1.00000000001827, rel=1.0e-4)
    assert point.cumulative_mole_fraction_depleted == pytest.approx(0.06770259075210731, rel=1.0e-4)
    assert point.material_balance_relative_error is None
    assert point.z_gas is not None
    assert point.z_gas == pytest.approx(0.9159261004210869, rel=1.0e-4)
    assert point.liquid_volume_relative_to_vsat == pytest.approx(0.9025692156128022, rel=1.0e-4)

    point = result.points[-1]
    assert point.cumulative_mole_fraction_depleted == pytest.approx(0.5597746791158184, rel=1.0e-4)
    assert point.liquid_volume_relative_to_vsat == pytest.approx(0.6341464599978935, rel=1.0e-4)


def test_the_k_values_are_one_row_per_pressure() -> None:
    result = constant_volume_depletion(ROWS, temperature=quantity(323.15, "K"), pressures=PRESSURES)
    assert len(result.k_values) == len(PRESSURES)
    assert all(len(row) == 2 for row in result.k_values)
    assert all(value >= 0.0 for row in result.k_values for value in row)


def test_a_schedule_and_a_known_cubic_are_required() -> None:
    with pytest.raises(InvalidInputError, match="at least one pressure"):
        constant_volume_depletion(ROWS, temperature=quantity(323.15, "K"), pressures=[])

    with pytest.raises(InvalidInputError, match="srk and pr"):
        constant_volume_depletion(
            ROWS,
            eos="bogus",
            temperature=quantity(323.15, "K"),
            pressures=PRESSURES,
        )
