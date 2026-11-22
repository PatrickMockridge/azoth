"""``pvt.property_sweep`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import property_sweep

BINARY = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.70, "mol")},
    {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.30, "mol")},
]
METHANE = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(1.0, "mol")}
]


def test_the_density_sweep_answers() -> None:
    result = property_sweep(
        BINARY,
        "density",
        temperatures=[quantity(323.15, "K"), quantity(313.15, "K"), quantity(303.15, "K")],
        pressures=[quantity(5.0, "bar")] * 3,
    )
    point = result.points[0]
    assert point.gas_density is not None
    assert point.gas_density.to("kg/m**3").magnitude == pytest.approx(3.050545344825013, rel=1e-6)
    assert point.oil_density is not None
    assert point.oil_density.to("kg/m**3").magnitude == pytest.approx(726.5941347934073, rel=1e-6)
    assert point.aqueous_density is None
    assert point.gas_phase_available
    assert point.oil_phase_available
    assert not point.aqueous_phase_available

    gas = property_sweep(
        METHANE,
        "density",
        temperatures=[quantity(300.0, "K"), quantity(290.0, "K")],
        pressures=[quantity(5.0, "bar")] * 2,
    )
    assert gas.points[0].gas_density is not None
    assert gas.points[0].oil_density is None
    assert gas.points[0].gas_phase_available
    assert not gas.points[0].oil_phase_available


def test_the_viscosity_sweep_answers() -> None:
    result = property_sweep(
        BINARY,
        "viscosity",
        temperatures=[quantity(323.15, "K"), quantity(313.15, "K"), quantity(303.15, "K")],
        pressures=[quantity(5.0, "bar")] * 3,
    )
    point = result.points[0]
    assert point.gas_viscosity is not None
    assert point.gas_viscosity.to("Pa*s").magnitude == pytest.approx(
        1.2037860833403275e-5, rel=1e-6
    )
    assert point.oil_viscosity is not None
    assert point.oil_viscosity.to("Pa*s").magnitude == pytest.approx(6.239054659190214e-4, rel=1e-6)
    assert point.gas_density is None

    gas = property_sweep(
        METHANE,
        "viscosity",
        temperatures=[quantity(300.0, "K")],
        pressures=[quantity(5.0, "bar")],
    )
    assert gas.points[0].gas_viscosity is not None
    assert gas.points[0].oil_viscosity is None


def test_the_wax_modes_are_owed_and_the_grid_is_checked() -> None:
    with pytest.raises(InvalidInputError, match="owed"):
        property_sweep(
            BINARY,
            "wax_fraction",
            temperatures=[quantity(300.0, "K")],
            pressures=[quantity(5.0, "bar")],
        )

    with pytest.raises(InvalidInputError, match="temperatures and"):
        property_sweep(
            BINARY,
            "density",
            temperatures=[quantity(300.0, "K"), quantity(290.0, "K")],
            pressures=[quantity(5.0, "bar")],
        )
