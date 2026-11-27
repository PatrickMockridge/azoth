"""Spec-driven tests for the ``eos.umr_pru_flash`` model.

The spec's case pins the two implementations to each other. These tests pin what a case
cannot state: the flash equations at the answer, the cap that is a result, and the
quadratic co-volume ``SystemUMRPRUEos`` installs with ``setBmixType(1)``.
"""

from __future__ import annotations

import math
from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen, ureg
from azoth.core.result import UmrPruFlashResult
from azoth.eos import umr_pru_flash
from azoth.eos.components import umr_pru_mixture_of
from azoth.eos.reference._mixture_state import reduced_parameters, umr_ader

MODEL_ID = "eos.umr_pru_flash"
Q = ureg.Quantity

SPEC = _models_gen.model(MODEL_ID)
CASES = SPEC["cases"]


def call(case: dict[str, Any]) -> UmrPruFlashResult:
    return umr_pru_flash(**h.model_kwargs(SPEC, case["inputs"]))


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
    assert result.iterations == expected["iterations"]
    h.assert_consistent(result, case["id"])


def test_the_split_satisfies_the_flash_equations() -> None:
    result = umr_pru_flash(
        ["methane", "n-butane"], T=Q(330.0, "K"), P=Q(25.0e5, "Pa"), z=[0.6, 0.4]
    )
    beta = result.vapour_fraction
    assert beta is not None
    for i, (x, y) in enumerate(zip(result.x, result.y, strict=True)):
        assert x * (1.0 - beta) + y * beta == pytest.approx([0.6, 0.4][i], abs=1e-12)
        assert math.log(x) + result.ln_phi_liquid[i] == pytest.approx(
            math.log(y) + result.ln_phi_vapour[i], abs=1e-9
        )
    assert sum(result.x) == pytest.approx(1.0, abs=1e-12)
    assert sum(result.y) == pytest.approx(1.0, abs=1e-12)


def test_the_co_volume_is_neqsims_quadratic_rule() -> None:
    mixture, _ = umr_pru_mixture_of(["methane", "n-butane"])
    reduced = reduced_parameters(mixture, 330.0, 25.0e5)
    assert reduced.umr_bmix_type == 1
    x = [0.6, 0.4]
    _, _, b_der, _ = umr_ader(reduced, x)
    # For a homogeneous co-volume rule, b_mix = sum_i x_i b_der_i.
    b_mix = sum(xi * b for xi, b in zip(x, b_der, strict=True))
    b = reduced.b

    def bij(i: int, j: int) -> float:
        return ((math.sqrt(b[i]) + math.sqrt(b[j])) / 2.0) ** 2

    expected = x[0] * x[0] * bij(0, 0) + 2.0 * x[0] * x[1] * bij(0, 1) + x[1] * x[1] * bij(1, 1)
    assert b_mix == pytest.approx(expected, abs=1e-14)
    assert b_mix != pytest.approx(sum(xi * bi for xi, bi in zip(x, b, strict=True)), abs=1e-6)
