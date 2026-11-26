"""The source's unit spellings, converted or refused."""

from __future__ import annotations

import pytest

from azoth.intake.units import UnitGap, normalize


def test_mole_percent_is_a_fraction_with_a_mole_basis() -> None:
    value = normalize(90.0, "mol%", "dimensionless", "z", expect_mole=True)
    assert value.value == pytest.approx(0.9)
    assert value.basis == "mole"


def test_mass_percent_on_a_mole_fraction_is_refused() -> None:
    with pytest.raises(UnitGap, match="mass percent"):
        normalize(90.0, "wt%", "dimensionless", "z", expect_mole=True)


def test_gauge_pressure_is_converted_to_absolute() -> None:
    value = normalize(50.0, "barg", "Pa", "P")
    assert value.value == pytest.approx(5_101_325.0)
    value = normalize(14.6959487755, "psig", "Pa", "P")
    assert value.value == pytest.approx(2 * 101_325.0, rel=1e-9)


def test_a_bare_bar_is_ambiguous() -> None:
    with pytest.raises(UnitGap, match="gauge or absolute"):
        normalize(50.0, "bar", "Pa", "P")


def test_a_missing_unit_on_a_dimensioned_input_is_refused() -> None:
    with pytest.raises(UnitGap, match="without a unit"):
        normalize(300.0, None, "K", "T")


def test_a_dimensionless_input_may_be_a_bare_number() -> None:
    assert normalize(0.5, None, "dimensionless", "fraction").value == 0.5


def test_temperature_offsets_are_not_applied_to_a_difference() -> None:
    assert normalize(25.0, "degC", "K", "T").value == pytest.approx(298.15)
    assert normalize(25.0, "degC", "K", "delta_t", interval=True).value == pytest.approx(25.0)
    assert normalize(9.0, "degF", "K", "delta_t", interval=True).value == pytest.approx(5.0)


def test_a_wrong_dimension_is_refused() -> None:
    with pytest.raises(UnitGap, match="does not convert"):
        normalize(1.0, "m", "Pa", "P")
