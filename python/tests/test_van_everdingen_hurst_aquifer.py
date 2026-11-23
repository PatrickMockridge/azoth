"""``pvt.van_everdingen_hurst_aquifer`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import van_everdingen_hurst_aquifer

DROPS = [quantity(value, "Pa") for value in (0.0, 5.0e5, 1.2e6, 2.2e6, 3.5e6, 5.0e6)]


def test_the_infinite_acting_kernel_answers() -> None:
    pressure = van_everdingen_hurst_aquifer(
        "dimensionless_pressure", dimensionless_time_value=0.005
    )
    assert pressure.dimensionless_pressure == pytest.approx(0.0773837496680494, rel=1.0e-12)

    derivative = van_everdingen_hurst_aquifer(
        "dimensionless_pressure_derivative", dimensionless_time_value=0.005
    )
    assert derivative.dimensionless_pressure_derivative == pytest.approx(
        7.507012527254403, rel=1.0e-12
    )


def test_the_bounded_policy_is_the_class_legacy_maximum() -> None:
    result = van_everdingen_hurst_aquifer(
        "dimensionless_pressure",
        dimensionless_time_value=100.0,
        dimensionless_outer_radius=5.0,
    )
    assert result.dimensionless_pressure == pytest.approx(9.192771245767434, rel=1.0e-12)


def test_the_dimensionless_time_and_constant_are_si() -> None:
    time = van_everdingen_hurst_aquifer(
        "dimensionless_time",
        permeability=quantity(1.0e-13, "m**2"),
        time=quantity(8.64e7, "s"),
        porosity=0.2,
        viscosity=quantity(5.0e-4, "Pa*s"),
        total_compressibility=1.0e-9,
        reservoir_radius=quantity(3000.0, "m"),
    )
    assert time.dimensionless_time == pytest.approx(9.599999999999998, rel=1.0e-12)

    constant = van_everdingen_hurst_aquifer(
        "aquifer_constant",
        porosity=0.2,
        total_compressibility=1.0e-9,
        thickness=quantity(30.0, "m"),
        reservoir_radius=quantity(3000.0, "m"),
        encroachment_angle=180.0,
    )
    assert constant.aquifer_constant == pytest.approx(0.16964600329384885, rel=1.0e-12)


def test_carter_tracy_marches_the_capture_history() -> None:
    result = van_everdingen_hurst_aquifer(
        "carter_tracy",
        dimensionless_times=[0.0, 10.0, 20.0, 40.0, 80.0, 160.0],
        pressure_drop=DROPS,
        aquifer_constant_value=0.16964600329384885,
    )
    assert result.cumulative_influx is not None
    values = [value.to("m**3").magnitude for value in result.cumulative_influx]
    expected = [
        0.0,
        513800.1920958346,
        1622029.7705783644,
        5083532.268602057,
        14575989.592201814,
        38319577.38347466,
    ]
    for actual, wanted in zip(values, expected, strict=True):
        assert actual == pytest.approx(wanted, rel=1.0e-12, abs=1.0e-15)


def test_a_negative_drop_is_projected_to_zero() -> None:
    result = van_everdingen_hurst_aquifer(
        "carter_tracy",
        dimensionless_times=[0.0, 0.1, 0.2],
        pressure_drop=[quantity(0.0, "Pa"), quantity(1.0, "Pa"), quantity(-10.0, "Pa")],
        aquifer_constant_value=1.0,
    )
    assert result.cumulative_influx is not None
    values = [value.to("m**3").magnitude for value in result.cumulative_influx]
    assert values[1] == pytest.approx(0.31823407285179967, rel=1.0e-12)
    assert values[2] == 0.0


def test_the_influence_table_answers_pressures() -> None:
    result = van_everdingen_hurst_aquifer(
        "influence_table", dimensionless_times=[1.0, 10.0, 100.0, 1000.0]
    )
    assert result.dimensionless_pressures is not None
    assert list(result.dimensionless_pressures) == pytest.approx(
        [0.8021451666033006, 1.6508947048253173, 2.722894443143703, 3.860590595578624],
        rel=1.0e-12,
    )


def test_a_refused_state_is_named() -> None:
    with pytest.raises(InvalidInputError, match="dimensionless_time"):
        van_everdingen_hurst_aquifer(
            "dimensionless_pressure", dimensionless_time_value=-1.0
        )
    with pytest.raises(InvalidInputError, match="NaN"):
        van_everdingen_hurst_aquifer(
            "dimensionless_pressure",
            dimensionless_time_value=1.0,
            dimensionless_outer_radius=float("nan"),
        )
    with pytest.raises(InvalidInputError, match="strictly increasing"):
        van_everdingen_hurst_aquifer(
            "carter_tracy",
            dimensionless_times=[0.0, 0.0],
            pressure_drop=[quantity(0.0, "Pa"), quantity(1.0, "Pa")],
            aquifer_constant_value=1.0,
        )
