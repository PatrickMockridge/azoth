"""``pvt.saturation_test`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import saturation_test

METHANE_NC10 = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.70, "mol")},
    {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.30, "mol")},
]
METHANE_NBUTANE = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.95, "mol")},
    {"fluid": 0, "name": "n-butane", "pseudo_component": False, "moles": quantity(0.05, "mol")},
]
METHANE = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(1.0, "mol")}
]


def test_the_upper_saturation_pressures_answer() -> None:
    result = saturation_test(METHANE_NC10, "pressure", temperature=quantity(323.15, "K"))
    assert result.saturation_pressure is not None
    assert result.saturation_pressure.to("Pa").magnitude == pytest.approx(
        2.8955195999145508e7, rel=1.0e-6
    )
    assert result.saturation_temperature is None

    result = saturation_test(METHANE_NBUTANE, "pressure", temperature=quantity(275.0, "K"))
    assert result.saturation_pressure is not None
    assert result.saturation_pressure.to("Pa").magnitude == pytest.approx(
        7495956.039428711, rel=1.0e-6
    )


def test_the_upper_saturation_temperature_answers() -> None:
    result = saturation_test(METHANE_NC10, "temperature", pressure=quantity(52.1, "bar"))
    assert result.saturation_temperature is not None
    assert result.saturation_temperature.to("K").magnitude == pytest.approx(
        557.1245384216309, rel=1.0e-6
    )
    assert result.saturation_pressure is None


def test_an_unbracketed_search_answers_the_grid_top_with_a_warning() -> None:
    pressure = saturation_test(METHANE, "pressure", temperature=quantity(300.0, "K"))
    assert pressure.saturation_pressure is not None
    assert pressure.saturation_pressure.to("Pa").magnitude == 1.0e8
    assert len(pressure.warnings) == 1

    temperature = saturation_test(METHANE, "temperature", pressure=quantity(100.0, "bar"))
    assert temperature.saturation_temperature is not None
    assert temperature.saturation_temperature.to("K").magnitude == 1200.0
    assert len(temperature.warnings) == 1


def test_the_bounds_belong_to_the_temperature_search_only() -> None:
    with pytest.raises(InvalidInputError, match="pressure search"):
        saturation_test(
            METHANE,
            "pressure",
            temperature=quantity(300.0, "K"),
            temperature_search_min=quantity(200.0, "K"),
            temperature_search_max=quantity(400.0, "K"),
        )

    with pytest.raises(InvalidInputError, match="together"):
        saturation_test(
            METHANE,
            "temperature",
            pressure=quantity(50.0, "bar"),
            temperature_search_min=quantity(200.0, "K"),
        )

    with pytest.raises(InvalidInputError, match="30"):
        saturation_test(
            METHANE,
            "temperature",
            pressure=quantity(50.0, "bar"),
            temperature_search_min=quantity(20.0, "K"),
            temperature_search_max=quantity(400.0, "K"),
        )


def test_an_unknown_mode_or_mixing_rule_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="saturation search"):
        saturation_test(METHANE, "both")

    with pytest.raises(InvalidInputError, match="classic and zero"):
        saturation_test(METHANE, "pressure", mixing_rule="bogus", temperature=quantity(300.0, "K"))
