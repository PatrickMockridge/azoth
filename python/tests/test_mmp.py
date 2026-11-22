"""``pvt.mmp`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import mmp

OIL = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.70, "mol")},
    {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.30, "mol")},
]
GAS = [{"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(1.0, "mol")}]


def test_the_slim_tube_and_rising_bubble_answer() -> None:
    slim = mmp(
        OIL,
        GAS,
        temperature=quantity(323.15, "K"),
        method="slim_tube",
        recovery_threshold=0.90,
        number_of_pressure_points=3,
        min_pressure=quantity(100.0, "bar"),
        max_pressure=quantity(300.0, "bar"),
        slim_tube_nodes=6,
        pv_injected=0.4,
    )
    assert slim.mmp is None
    assert slim.mechanism == "immiscible"
    assert len(slim.points) == 3
    assert slim.points[0].recovery == pytest.approx(0.0912307824495534, rel=1.0e-6)
    assert slim.points[2].recovery == pytest.approx(0.2, rel=1.0e-6)

    rising = mmp(
        OIL,
        GAS,
        temperature=quantity(323.15, "K"),
        method="rising_bubble",
        recovery_threshold=0.90,
        number_of_pressure_points=3,
        min_pressure=quantity(100.0, "bar"),
        max_pressure=quantity(300.0, "bar"),
        slim_tube_nodes=6,
        pv_injected=0.4,
    )
    assert rising.mmp is None
    assert rising.mechanism == slim.mechanism
    assert rising.points[0].recovery == pytest.approx(slim.points[0].recovery, rel=1.0e-9)


def test_the_key_tie_line_and_a_wide_slim_tube_answer() -> None:
    tie = mmp(
        OIL,
        GAS,
        temperature=quantity(323.15, "K"),
        method="key_tie_line",
        min_pressure=quantity(100.0, "bar"),
        max_pressure=quantity(500.0, "bar"),
    )
    assert tie.mmp is not None
    assert tie.mmp.to("Pa").magnitude == pytest.approx(4.001953125e7, rel=3.0e-2)
    assert tie.mechanism == "combined"
    assert tie.points == ()

    wide = mmp(
        OIL,
        GAS,
        temperature=quantity(323.15, "K"),
        method="slim_tube",
        recovery_threshold=0.50,
        number_of_pressure_points=5,
        min_pressure=quantity(100.0, "bar"),
        max_pressure=quantity(1000.0, "bar"),
        slim_tube_nodes=6,
        pv_injected=0.4,
    )
    assert wide.mmp is not None
    assert wide.mmp.to("Pa").magnitude == pytest.approx(4.09375e7, rel=1.0e-6)
    assert wide.mechanism == "combined"


def test_a_gas_the_oil_does_not_carry_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="not a component of the oil"):
        mmp(
            OIL,
            [{"fluid": 0, "name": "CO2", "pseudo_component": False, "moles": quantity(1.0, "mol")}],
            temperature=quantity(323.15, "K"),
        )
