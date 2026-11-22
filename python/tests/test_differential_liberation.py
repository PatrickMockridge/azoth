"""``pvt.differential_liberation`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import differential_liberation

ROWS = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.70, "mol")},
    {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.30, "mol")},
]
PRESSURES = [
    quantity(250.0, "bar"),
    quantity(200.0, "bar"),
    quantity(150.0, "bar"),
    quantity(100.0, "bar"),
    quantity(70.0, "bar"),
    quantity(50.0, "bar"),
    quantity(30.0, "bar"),
    quantity(20.0, "bar"),
    quantity(10.0, "bar"),
    quantity(1.0, "bar"),
]


def test_the_first_and_last_points_answer() -> None:
    result = differential_liberation(ROWS, temperature=quantity(323.15, "K"), pressures=PRESSURES)
    assert result.saturation_pressure is not None
    assert result.saturation_pressure.to("Pa").magnitude == pytest.approx(
        2.8955063438415527e7, rel=1.0e-4
    )
    assert result.voil_std is not None
    assert result.voil_std.to("m**3").magnitude == pytest.approx(5.609020215003901e-5, rel=1.0e-6)

    point = result.points[0]
    assert point.bo is not None
    assert point.bo == pytest.approx(1.5791783988447246, rel=1.0e-6)
    assert point.rs is not None
    assert point.rs == pytest.approx(225.43729834885008, rel=1.0e-6)
    assert point.bg is not None
    assert point.bg == pytest.approx(0.0042120322308083985, rel=1.0e-6)
    assert point.oil_density.to("kg/m**3").magnitude == pytest.approx(576.4180638703791, rel=1.0e-6)

    point = result.points[-1]
    assert point.rs == pytest.approx(0.0, abs=1.0e-12)
    assert point.bg == pytest.approx(1.1347113330119678, rel=1.0e-6)


def test_a_single_phase_point_leaves_its_gas_fields_absent() -> None:
    rows = [
        {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.60, "mol")},
        {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.40, "mol")},
    ]
    result = differential_liberation(
        rows,
        temperature=quantity(323.15, "K"),
        pressures=[quantity(250.0, "bar"), quantity(200.0, "bar")],
    )
    point = result.points[0]
    assert point.gas_volume is None
    assert point.gas_standard_volume is None
    assert point.bg is None
    assert point.liquid_volume_relative_to_vsat is None
    assert point.bo is not None


def test_a_schedule_is_required() -> None:
    with pytest.raises(InvalidInputError, match="at least one pressure"):
        differential_liberation(ROWS, temperature=quantity(323.15, "K"), pressures=[])

    with pytest.raises(InvalidInputError, match="srk and pr"):
        differential_liberation(
            ROWS,
            eos="bogus",
            temperature=quantity(323.15, "K"),
            pressures=PRESSURES,
        )
