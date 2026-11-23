"""The Marquardt loop, its Ridders derivative and its floor."""

from __future__ import annotations

from typing import Any

import pytest

from azoth.core.errors import InvalidInputError
from azoth.pvt.reference.regression import levenberg_marquardt


def _one_parameter(parameters: list[float]) -> list[float]:
    return [parameters[0], 2.0 * parameters[0], 3.0 * parameters[0]]


def test_one_parameter_recovers_its_value_and_stops_on_the_floor() -> None:
    result = levenberg_marquardt(
        [(0.0, 10.0)], [1.0], 50, [2.0, 4.0, 6.0], [1.0, 1.0, 1.0], _one_parameter
    )
    assert result.parameters[0] == pytest.approx(2.0, abs=1e-6)
    assert result.reason == "chi_square_tolerance"
    assert result.iterations == 5


def test_two_parameters_recover_their_values() -> None:
    x = [0.0, 1.0, 2.0, 3.0]
    result = levenberg_marquardt(
        [(-10.0, 10.0), (-10.0, 10.0)],
        [0.0, 0.0],
        200,
        [1.0, 3.0, 5.0, 7.0],
        [1.0, 1.0, 1.0, 1.0],
        lambda parameters: [parameters[0] + parameters[1] * value for value in x],
    )
    # The class stops on chi-square at the floor, not on parameter exactness.
    assert result.parameters[0] == pytest.approx(1.0, abs=1e-4)
    assert result.parameters[1] == pytest.approx(2.0, abs=1e-4)
    assert result.iterations >= 5


def test_a_bound_pinned_parameter_is_singular() -> None:
    result = levenberg_marquardt(
        [(1.0, 1.0)], [1.0], 50, [2.0], [1.0], lambda parameters: [parameters[0]]
    )
    assert result.reason == "singular_matrix"
    assert result.iterations == 1


def test_an_exactly_fit_model_still_takes_the_floor() -> None:
    result = levenberg_marquardt(
        [(0.0, 10.0)], [1.0], 50, [1.0, 2.0, 3.0], [1.0, 1.0, 1.0], _one_parameter
    )
    assert result.iterations == 5
    assert result.chi_square < 1e-12


def test_an_evaluation_refusal_propagates() -> None:
    def refuse(_: Any) -> list[float]:
        raise InvalidInputError("state", "the flash refused")

    with pytest.raises(InvalidInputError, match="refused"):
        levenberg_marquardt([(0.0, 1.0)], [0.5], 10, [1.0], [1.0], refuse)
