"""``pvt.fetkovich_aquifer`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import fetkovich_aquifer


def interval(duration: float, start: float, end: float) -> dict[str, object]:
    return {
        "duration": quantity(duration, "s"),
        "reservoir_pressure_start": quantity(start, "Pa"),
        "reservoir_pressure_end": quantity(end, "Pa"),
    }


def test_the_constant_pressure_identity() -> None:
    result = fetkovich_aquifer(
        1.0e-9,
        2.0e-7,
        quantity(3.0e7, "Pa"),
        [interval(12000.0, 2.5e7, 2.5e7)],
        initial_water_volume=quantity(1.0e6, "m**3"),
    )
    assert result.capacity == pytest.approx(0.001, rel=1.0e-15)
    state = result.states[1]
    assert state.cumulative_influx.to("m**3").magnitude == pytest.approx(
        4546.410233552938, rel=1.0e-12
    )
    assert state.pressure.to("Pa").magnitude == pytest.approx(25453589.766447064, rel=1.0e-12)


def test_the_interior_extrema_are_retained() -> None:
    result = fetkovich_aquifer(
        1.0e-9,
        2.0e-7,
        quantity(3.0e7, "Pa"),
        [interval(10000.0, 2.5e7, 4.0e7)],
        initial_water_volume=quantity(1.0e6, "m**3"),
    )
    state = result.states[1]
    assert state.interval_minimum_cumulative_influx.to("m**3").magnitude == pytest.approx(
        -4191.69104045766, rel=1.0e-10
    )
    assert state.interval_maximum_cumulative_influx.to("m**3").magnitude == pytest.approx(
        1168.8078217550703, rel=1.0e-10
    )


def test_infinite_storage_has_no_capacity() -> None:
    result = fetkovich_aquifer(
        1.0e-9,
        2.0e-7,
        quantity(3.0e7, "Pa"),
        [interval(1000.0, 3.0e7, 1.0e7)],
    )
    assert result.capacity is None
    assert result.states[1].cumulative_influx.to("m**3").magnitude == pytest.approx(
        2000.0, rel=1.0e-12
    )
    assert result.states[1].pressure.to("Pa").magnitude == pytest.approx(3.0e7)


def test_a_multi_interval_schedule_keeps_the_signed_influx() -> None:
    result = fetkovich_aquifer(
        1.0e-9,
        2.0e-7,
        quantity(3.0e7, "Pa"),
        [interval(100000.0, 3.5e7, 3.5e7), interval(100000.0, 3.0e7, 3.0e7)],
        initial_water_volume=quantity(1.0e6, "m**3"),
    )
    assert len(result.states) == 3
    assert result.states[1].cumulative_influx.to("m**3").magnitude < 0.0
    assert result.states[2].interval_influx.to("m**3").magnitude > 0.0


def test_a_refused_state_is_named() -> None:
    with pytest.raises(InvalidInputError, match=r"intervals\[0\]"):
        fetkovich_aquifer(
            1.0e-6,
            1.0e-5,
            quantity(3.0e7, "Pa"),
            [interval(100.0, 2.5e7, 2.5e7)],
            initial_water_volume=quantity(1.0, "m**3"),
        )
    with pytest.raises(InvalidInputError, match="initial_water_volume"):
        fetkovich_aquifer(
            1.0e-9,
            2.0e-7,
            quantity(3.0e7, "Pa"),
            [],
            initial_water_volume=quantity(-1.0, "m**3"),
        )
    with pytest.raises(InvalidInputError, match="duration"):
        fetkovich_aquifer(
            1.0e-9,
            2.0e-7,
            quantity(3.0e7, "Pa"),
            [interval(0.0, 3.0e7, 2.0e7)],
            initial_water_volume=quantity(1.0e6, "m**3"),
        )
