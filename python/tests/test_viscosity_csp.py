"""The four CSP correction factors behind ``eos.viscosity``'s boundary."""

from __future__ import annotations

import math
from typing import Any

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.eos import from_names
from azoth.eos.reference.viscosity import _viscosity_with_csp, viscosity

METHANE_TC = 190.56
METHANE_PC = 4.599e6
METHANE_M = 0.016043


def _mixture() -> Any:
    return from_names(["n-heptane"], eos="srk")


def _mu(result: Any) -> float:
    return float(result.mu.to("Pa*s").magnitude)


def test_unit_factors_are_the_published_correlation() -> None:
    mixture = _mixture()
    published = viscosity(mixture, quantity(320.0, "K"), quantity(5.0e5, "Pa"), [1.0])
    unit = _viscosity_with_csp(
        mixture, quantity(320.0, "K"), quantity(5.0e5, "Pa"), [1.0], (1.0, 1.0, 1.0, 1.0)
    )
    assert _mu(published) == _mu(unit)


@pytest.mark.parametrize("index,factor", [(0, 2.0), (1, 2.0), (2, 2.0)])
def test_each_factor_scales_its_own_exponent(index: int, factor: float) -> None:
    mixture = _mixture()
    unit = _viscosity_with_csp(
        mixture, quantity(320.0, "K"), quantity(5.0e5, "Pa"), [1.0], (1.0, 1.0, 1.0, 1.0)
    )
    correction = [1.0, 1.0, 1.0, 1.0]
    correction[index] = factor
    scaled = _viscosity_with_csp(
        mixture,
        quantity(320.0, "K"),
        quantity(5.0e5, "Pa"),
        [1.0],
        (correction[0], correction[1], correction[2], correction[3]),
    )
    component = mixture.components[0]
    tc = component.Tc.to_base_units().magnitude
    pc = component.Pc.to_base_units().magnitude
    molar_mass = component.molar_mass.to_base_units().magnitude * 1e3
    expected = (
        (tc / METHANE_TC) ** (-1.0 / 6.0 * (factor - 1.0))
        if index == 0
        else (pc / METHANE_PC) ** (2.0 / 3.0 * (factor - 1.0))
        if index == 1
        else (molar_mass / (METHANE_M * 1e3)) ** (0.5 * (factor - 1.0))
    )
    assert math.isclose(_mu(scaled) / _mu(unit), expected, rel_tol=1e-12)


def test_a_non_finite_factor_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="finite"):
        _viscosity_with_csp(
            _mixture(),
            quantity(320.0, "K"),
            quantity(5.0e5, "Pa"),
            [1.0],
            (1.0, math.nan, 1.0, 1.0),
        )
