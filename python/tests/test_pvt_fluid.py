"""The PVT laboratory experiments' fluid and flash machinery.

The Python twins of ``crates/azoth-pvt/tests/fluid.rs`` and the phase order the classes'
``phase(0) = gas`` depends on.
"""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt.reference import _flash, _fluid, _volume

OIL = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.2, "mol")},
    {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.8, "mol")},
]
GAS = [{"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(1.0, "mol")}]


def test_the_union_merges_a_gas_component_into_the_oils_row() -> None:
    fluid, template = _fluid.Fluid.from_rows_pair(OIL, GAS)
    assert fluid.moles == pytest.approx([0.2, 0.8])
    assert template.amounts == pytest.approx([1.0, 0.0])
    swelled = fluid.with_added(template, 0.1)
    assert swelled.moles == pytest.approx([0.3, 0.8])


def test_the_union_appends_a_gas_component_the_oil_does_not_have() -> None:
    gas = [
        {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.5, "mol")},
        {"fluid": 0, "name": "ethane", "pseudo_component": False, "moles": quantity(0.5, "mol")},
    ]
    fluid, template = _fluid.Fluid.from_rows_pair(OIL, gas)
    assert fluid.moles == pytest.approx([0.2, 0.8, 0.0])
    assert template.amounts == pytest.approx([0.5, 0.0, 0.5])


def test_two_fluids_in_one_row_list_are_refused() -> None:
    rows = [
        {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(1.0, "mol")},
        {"fluid": 1, "name": "nC10", "pseudo_component": False, "moles": quantity(1.0, "mol")},
    ]
    with pytest.raises(InvalidInputError, match="more than one fluid"):
        _fluid.Fluid.from_rows(rows)


def test_the_volume_shift_is_the_srk_one_for_an_srk_fluid() -> None:
    fluid = _fluid.Fluid.from_rows(OIL, eos="srk")
    x = fluid.z()
    shift = _volume.volume_shift(fluid.mixture, x)
    expected = sum(
        x[index] * component.volume_shift_srk
        for index, component in enumerate(fluid.mixture.components)
    )
    assert shift == pytest.approx(expected)
    # The two cubics' shifts differ; this is the mistake the selection exists to prevent.
    pr = sum(
        x[index] * component.volume_shift
        for index, component in enumerate(fluid.mixture.components)
    )
    assert shift != pytest.approx(pr)


def test_the_flash_orders_its_phases_gas_first() -> None:
    fluid = _fluid.Fluid.from_rows(OIL, eos="srk")
    flash = fluid.flash(323.15, 5.0e6, _flash.TWO_PHASE)
    assert flash.gas() is not None
    assert flash.oil() is not None
    assert flash.phases[0].label == "gas"
    assert flash.phases[0].fraction + flash.phases[1].fraction == pytest.approx(1.0)
