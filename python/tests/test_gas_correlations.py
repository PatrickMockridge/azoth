"""``pvt.gas_correlations`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import gas_correlations


def test_the_wrapper_publishes_quantities() -> None:
    result = gas_correlations(
        [
            {"calculation": "pseudo_critical_temperature_standing", "gamma_g": 0.75},
            {
                "calculation": "gas_density",
                "pressure": quantity(13790000.0, "Pa"),
                "temperature": quantity(366.48, "K"),
                "gamma_g": 0.65,
                "molar_mass": quantity(0.01604, "kg/mol"),
            },
        ]
    )
    assert result.is_clean
    temperature = result.queries[0].temperature
    density = result.queries[1].density
    assert temperature is not None and density is not None
    assert temperature.to("K").magnitude == pytest.approx(224.84375, rel=1.0e-12)
    assert density.to("kg/m**3").magnitude == pytest.approx(80.85998616241369, rel=1.0e-9)


def test_the_pseudo_pressure_is_the_captured_value() -> None:
    result = gas_correlations(
        [
            {
                "calculation": "pseudo_pressure_from_correlation",
                "pressure": quantity(20684000.0, "Pa"),
                "reference_pressure": quantity(101325.0, "Pa"),
                "temperature": quantity(366.48, "K"),
                "gamma_g": 0.65,
                "molar_mass": quantity(0.01604, "kg/mol"),
            }
        ]
    )
    value = result.queries[0].pseudo_pressure
    assert value is not None
    assert value == pytest.approx(2865418.4358585384, rel=1.0e-9)


def test_a_gas_gravity_outside_the_range_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"queries\[0\]\.gamma_g"):
        gas_correlations([{"calculation": "pseudo_critical_temperature_standing", "gamma_g": 0.54}])


def test_a_zero_reference_pressure_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"queries\[0\]\.reference_pressure"):
        gas_correlations(
            [
                {
                    "calculation": "pseudo_pressure_from_correlation",
                    "pressure": quantity(20684000.0, "Pa"),
                    "reference_pressure": quantity(0.0, "Pa"),
                    "temperature": quantity(366.48, "K"),
                    "gamma_g": 0.65,
                    "molar_mass": quantity(0.01604, "kg/mol"),
                }
            ]
        )


def test_an_unknown_calculation_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"queries\[0\]\.calculation"):
        gas_correlations([{"calculation": "not_a_calculation"}])
