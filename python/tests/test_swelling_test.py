"""``pvt.swelling_test`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import swelling_test

OIL = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.2, "mol")},
    {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.8, "mol")},
]
GAS = [{"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(1.0, "mol")}]


def test_the_pinned_upstream_case_answers() -> None:
    result = swelling_test(
        OIL,
        injection_gas=GAS,
        mixing_rule="zero",
        volume_correction=False,
        temperature=quantity(353.15, "K"),
        cumulative_gas_injected=[0.0, 0.10, 0.20],
    )
    assert result.original_oil_moles.to("mol").magnitude == pytest.approx(1.0)
    assert result.points[0].pressure.to("Pa").magnitude == pytest.approx(
        5128370.972140508, rel=1e-6
    )
    assert result.points[0].relative_oil_volume == pytest.approx(1.0)
    assert result.points[1].pressure.to("Pa").magnitude == pytest.approx(7320109.92148898, rel=1e-6)
    assert result.points[1].relative_oil_volume == pytest.approx(1.0301020971426877, rel=1e-6)
    assert result.points[2].pressure.to("Pa").magnitude == pytest.approx(
        9314150.739597373, rel=1e-6
    )
    assert result.points[2].relative_oil_volume == pytest.approx(1.0602317415732485, rel=1e-6)


def test_a_mixed_injection_gas_and_a_zero_schedule_answer() -> None:
    oil = [
        {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.5, "mol")},
        {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.5, "mol")},
    ]
    gas = [
        {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.7, "mol")},
        {"fluid": 0, "name": "ethane", "pseudo_component": False, "moles": quantity(0.3, "mol")},
    ]
    result = swelling_test(
        oil,
        injection_gas=gas,
        mixing_rule="zero",
        volume_correction=False,
        temperature=quantity(323.15, "K"),
        cumulative_gas_injected=[0.0, 0.05, 0.15, 0.25],
    )
    assert result.points[3].relative_oil_volume == pytest.approx(1.1115123217160363, rel=1e-6)

    zero = swelling_test(
        oil,
        mixing_rule="zero",
        volume_correction=False,
        temperature=quantity(323.15, "K"),
        cumulative_gas_injected=[0.0, 0.0],
    )
    assert zero.points[1].relative_oil_volume == pytest.approx(1.0)
    assert zero.points[1].pressure.to("Pa").magnitude == pytest.approx(
        1.4411248035344983e7, rel=1e-6
    )


def test_a_decreasing_schedule_or_a_gasless_injection_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="cannot decrease"):
        swelling_test(
            OIL,
            mixing_rule="zero",
            temperature=quantity(353.15, "K"),
            cumulative_gas_injected=[0.1, 0.05],
        )

    with pytest.raises(InvalidInputError, match="no injection gas"):
        swelling_test(
            OIL,
            mixing_rule="zero",
            temperature=quantity(353.15, "K"),
            cumulative_gas_injected=[0.0, 0.1],
        )
