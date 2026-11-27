"""Spec-driven tests for the ``eos.pt_flash_check`` model.

The spec's cases pin the two implementations to each other and to ``eos.pt_flash``'s
arithmetic. These tests pin the diagnostic contract a case cannot state: the split is
the same flash, a tolerance the loop cannot meet is a result under ``strict = false``,
and it raises under ``strict = true``.
"""

from __future__ import annotations

from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen, ureg
from azoth.core.errors import InvalidInputError, SolverNotConvergedError
from azoth.core.result import PtFlashCheckResult
from azoth.core.warnings import WarningCode
from azoth.eos import from_names, pt_flash, pt_flash_check

MODEL_ID = "eos.pt_flash_check"
Q = ureg.Quantity

SPEC = _models_gen.model(MODEL_ID)
CASES = SPEC["cases"]


def call(case: dict[str, Any]) -> PtFlashCheckResult:
    return pt_flash_check(**h.model_kwargs(SPEC, case["inputs"]))


@pytest.mark.parametrize("case", CASES, ids=lambda c: c["id"])
def test_spec_case(case: dict[str, Any]) -> None:
    result = call(case)
    tolerance = case["tolerance"]
    expected = case["expected"]
    for name in ("x", "y", "k", "ln_phi_liquid", "ln_phi_vapour"):
        actual = getattr(result, name)
        assert len(actual) == len(expected[name]), f"{case['id']}: {name} length"
        for i, (got, want) in enumerate(zip(actual, expected[name], strict=True)):
            h.assert_close(got, want, tolerance, f"{case['id']} ({name}[{i}])")
    for name in ("vapour_fraction", "z_liquid", "z_vapour", "min_t_over_tc"):
        h.assert_close(getattr(result, name), expected[name], tolerance, name)
    assert result.converged is expected["converged"]
    assert result.iterations == expected["iterations"]
    h.assert_consistent(result, case["id"])


def test_the_split_is_the_pt_flash_split() -> None:
    fluid = from_names(["methane", "n-butane"])
    ordinary = pt_flash(fluid, T=Q(330.0, "K"), P=Q(2.5e6, "Pa"), z=[0.6, 0.4])
    checked = pt_flash_check(fluid, T=Q(330.0, "K"), P=Q(2.5e6, "Pa"), z=[0.6, 0.4])
    assert checked.converged is True
    assert checked.phase == ordinary.phase
    assert checked.iterations == ordinary.iterations
    assert checked.vapour_fraction == ordinary.vapour_fraction
    assert checked.x == ordinary.x
    assert checked.y == ordinary.y
    assert checked.k == ordinary.k


def test_a_tolerance_the_loop_cannot_meet_is_a_result_when_not_strict() -> None:
    fluid = from_names(["methane", "n-butane"])
    result = pt_flash_check(
        fluid,
        T=Q(330.0, "K"),
        P=Q(2.5e6, "Pa"),
        z=[0.6, 0.4],
        tolerance=1e-300,
        strict=False,
    )
    assert result.converged is False
    assert any(w.code is WarningCode.SOLVER_NOT_CONVERGED for w in result.warnings)
    with pytest.raises(SolverNotConvergedError):
        pt_flash_check(
            fluid,
            T=Q(330.0, "K"),
            P=Q(2.5e6, "Pa"),
            z=[0.6, 0.4],
            tolerance=1e-300,
            strict=True,
        )


@pytest.mark.parametrize("tolerance", [0.0, -1.0, float("nan"), float("inf")])
def test_a_bad_tolerance_is_refused(tolerance: float) -> None:
    fluid = from_names(["methane", "n-butane"])
    with pytest.raises(InvalidInputError):
        pt_flash_check(
            fluid,
            T=Q(330.0, "K"),
            P=Q(2.5e6, "Pa"),
            z=[0.6, 0.4],
            tolerance=tolerance,
            strict=False,
        )
