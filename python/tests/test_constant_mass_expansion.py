"""``pvt.constant_mass_expansion`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import constant_mass_expansion

ROWS = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.70, "mol")},
    {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.30, "mol")},
]
PRESSURES = [
    quantity(300.0, "bar"),
    quantity(250.0, "bar"),
    quantity(200.0, "bar"),
    quantity(150.0, "bar"),
    quantity(100.0, "bar"),
    quantity(50.0, "bar"),
    quantity(10.0, "bar"),
]


def test_the_saturation_and_single_phase_point_answer() -> None:
    result = constant_mass_expansion(ROWS, temperature=quantity(323.15, "K"), pressures=PRESSURES)
    assert result.saturation_pressure.to("Pa").magnitude == pytest.approx(
        2.8955063438415527e7, rel=1.0e-4
    )
    assert result.saturation_volume.to("m**3").magnitude == pytest.approx(
        1.1262213155362104e-4, rel=1.0e-4
    )
    assert result.z_saturation == pytest.approx(1.2136960689968699, rel=1.0e-4)
    assert result.saturation_isothermal_compressibility == pytest.approx(
        3.6950566080912616e-9, rel=1.0e-3
    )

    point = result.points[0]
    assert point.relative_volume == pytest.approx(0.9962118444465242, rel=1.0e-6)
    assert point.density.to("kg/m**3").magnitude == pytest.approx(551.7836891574023, rel=1.0e-6)
    assert point.bg == pytest.approx(0.004132467194523274, rel=1.0e-6)
    assert point.viscosity is not None
    assert point.isothermal_compressibility is not None
    assert point.y_factor is None
    assert point.liquid_relative_volume is None


def test_the_two_phase_points_carry_the_class_quantities() -> None:
    result = constant_mass_expansion(ROWS, temperature=quantity(323.15, "K"), pressures=PRESSURES)
    point = result.points[1]
    assert point.density.to("kg/m**3").magnitude == pytest.approx(176.16920906600248, rel=1.0e-6)
    assert point.gas_volume.to("m**3").magnitude == pytest.approx(1.6226118566811178e-5, rel=1.0e-6)
    assert point.z_gas == pytest.approx(0.9266794787321436, rel=1.0e-6)
    assert point.oil_density is not None
    assert point.oil_density.to("kg/m**3").magnitude == pytest.approx(576.4180638703791, rel=1.0e-6)
    assert point.liquid_relative_volume == pytest.approx(0.9134080080494633, rel=1.0e-4)
    assert point.y_factor == pytest.approx(2.6734567057644845, rel=1.0e-4)
    assert point.viscosity is None
    assert point.isothermal_compressibility is None


def test_turning_the_volume_correction_off_moves_the_density() -> None:
    corrected = constant_mass_expansion(
        ROWS, temperature=quantity(323.15, "K"), pressures=PRESSURES
    )
    raw = constant_mass_expansion(
        ROWS,
        temperature=quantity(323.15, "K"),
        pressures=PRESSURES,
        volume_correction=False,
    )
    assert (
        raw.points[0].density.to("kg/m**3").magnitude
        != corrected.points[0].density.to("kg/m**3").magnitude
    )


def test_a_pressure_schedule_and_a_known_cubic_are_required() -> None:
    with pytest.raises(InvalidInputError, match="at least one pressure"):
        constant_mass_expansion(ROWS, temperature=quantity(323.15, "K"), pressures=[])

    with pytest.raises(InvalidInputError, match="srk and pr"):
        constant_mass_expansion(
            ROWS,
            eos="bogus",
            temperature=quantity(323.15, "K"),
            pressures=PRESSURES,
        )
