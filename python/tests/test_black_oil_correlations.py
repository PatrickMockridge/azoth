"""``pvt.black_oil_correlations`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import black_oil_correlations

RS_BLACK_OIL = 89.05399999999999
RS_P_SAT = 89.053803339515
T_K = 366.4833333333333


def test_the_wrapper_publishes_quantities() -> None:
    result = black_oil_correlations(
        [
            {"calculation": "api_from_specific_gravity", "gamma_o": 0.85},
            {
                "calculation": "bubble_point_standing",
                "rs": RS_BLACK_OIL,
                "gamma_g": 0.8,
                "api": 35.0,
                "temperature": quantity(T_K, "K"),
            },
        ]
    )
    assert result.is_clean
    assert result.queries[0].factor == pytest.approx(34.970588235294116)
    assert result.queries[1].pressure is not None
    assert result.queries[1].pressure.to("Pa").magnitude == pytest.approx(
        14401383.868223343, rel=1.0e-9
    )


def test_the_two_standing_forms_keep_their_own_literals() -> None:
    common = {"rs": RS_P_SAT, "gamma_g": 0.8, "api": 35.0, "temperature": quantity(T_K, "K")}
    black_oil = black_oil_correlations([{"calculation": "bubble_point_standing", **common}])
    p_sat = black_oil_correlations([{"calculation": "bubble_point_standing_p_sat", **common}])
    first = black_oil.queries[0].pressure
    second = p_sat.queries[0].pressure
    assert first is not None and second is not None
    difference = abs(first.to("Pa").magnitude - second.to("Pa").magnitude)
    assert 0.0 < difference < 1.0e-4 * second.to("Pa").magnitude


def test_the_p_sat_clamp_is_carried() -> None:
    result = black_oil_correlations(
        [
            {
                "calculation": "bubble_point_standing_p_sat",
                "rs": 0.17810760667903,
                "gamma_g": 0.8,
                "api": 35.0,
                "temperature": quantity(T_K, "K"),
            }
        ]
    )
    pressure = result.queries[0].pressure
    assert pressure is not None
    assert pressure.to("Pa").magnitude == pytest.approx(101352.972, rel=1.0e-9)


def test_a_missing_field_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"queries\[0\]\.api"):
        black_oil_correlations(
            [
                {
                    "calculation": "bubble_point_standing",
                    "rs": RS_BLACK_OIL,
                    "gamma_g": 0.8,
                    "temperature": quantity(T_K, "K"),
                }
            ]
        )


def test_a_non_positive_api_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"queries\[0\]\.api"):
        black_oil_correlations(
            [
                {
                    "calculation": "dead_oil_viscosity_glaso",
                    "api": 0.0,
                    "temperature": quantity(T_K, "K"),
                }
            ]
        )


def test_an_unknown_calculation_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"queries\[0\]\.calculation"):
        black_oil_correlations([{"calculation": "not_a_calculation"}])
