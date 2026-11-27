"""Spec-driven tests for the ``eos.state_at_tp`` model.

The spec's case pins the split and the aggregate properties to both implementations.
These tests pin the identities a case cannot state: the aggregate is the flash's split,
the pressure-volume and density relations hold, and the dense class distinguishes the
negative flash on a compressed fluid from a vapour.
"""

from __future__ import annotations

from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen, ureg
from azoth.core.result import PhaseClass
from azoth.eos import IdealGasModel, Mixture, state_at_tp
from azoth.eos.components import mixture_of
from azoth.eos.reference.pr_molar_volume import MOLAR_GAS_CONSTANT

MODEL_ID = "eos.state_at_tp"
Q = ureg.Quantity

SPEC = _models_gen.model(MODEL_ID)
CASES = SPEC["cases"]


def fluid() -> tuple[Mixture, IdealGasModel]:
    return mixture_of(["methane", "n-butane"])


def call(case: dict[str, Any]):
    kwargs = h.model_kwargs(SPEC, case["inputs"])
    return state_at_tp(**kwargs)


@pytest.mark.parametrize("case", CASES, ids=lambda c: c["id"])
def test_spec_case(case: dict[str, Any]) -> None:
    result = call(case)
    tolerance = case["tolerance"]
    expected = case["expected"]

    assert result.phase.value == expected["phase"]
    assert result.phase_class.value == expected["phase_class"]
    assert result.converged is expected["converged"]
    h.assert_close(
        result.vapour_fraction, expected["vapour_fraction"], tolerance, "vapour_fraction"
    )
    for name in ("x", "y"):
        actual = getattr(result, name)
        for i, (got, want) in enumerate(zip(actual, expected[name], strict=True)):
            h.assert_close(got, want, tolerance, f"{name}[{i}]")
    units = {
        "molar_mass": "kg/mol",
        "molar_volume": "m**3/mol",
        "density": "kg/m**3",
        "internal_energy": "J/mol",
        "enthalpy": "J/mol",
        "cp": "J/(mol*K)",
        "cv": "J/(mol*K)",
    }
    for name, unit in units.items():
        h.assert_close(getattr(result, name).to(unit).magnitude, expected[name], tolerance, name)
    h.assert_close(
        result.compressibility, expected["compressibility"], tolerance, "compressibility"
    )
    h.assert_consistent(result, case["id"])


def test_the_bulk_relations_hold() -> None:
    mix, ideal = fluid()
    r = state_at_tp(mix, ideal, T=Q(330.0, "K"), P=Q(2.5e6, "Pa"), z=[0.6, 0.4])
    p = 2.5e6
    t = 330.0
    v = r.molar_volume.to("m**3/mol").magnitude
    h.assert_close(
        r.internal_energy.to("J/mol").magnitude,
        r.enthalpy.to("J/mol").magnitude - p * v,
        1e-12,
        "u = h - Pv",
    )
    h.assert_close(
        r.density.to("kg/m**3").magnitude,
        r.molar_mass.to("kg/mol").magnitude / v,
        1e-12,
        "rho = M/v",
    )
    h.assert_close(r.compressibility, p * v / (MOLAR_GAS_CONSTANT * t), 1e-12, "Z")


def test_a_dense_negative_flash_is_not_called_gas() -> None:
    mix, ideal = fluid()
    # Kay pseudo-critical for this feed is about 284 K and 43 bar: 300 K/200 bar is
    # above both, 300 K/50 bar is above the pressure alone.
    dense = state_at_tp(mix, ideal, T=Q(300.0, "K"), P=Q(200e5, "Pa"), z=[0.6, 0.4])
    assert dense.phase_class in (PhaseClass.DENSE_SUPERCRITICAL, PhaseClass.DENSE_SINGLE_PHASE)
    assert dense.phase_class is not PhaseClass.SINGLE_PHASE_GAS
    above_pressure = state_at_tp(mix, ideal, T=Q(300.0, "K"), P=Q(50e5, "Pa"), z=[0.6, 0.4])
    assert above_pressure.phase_class is not PhaseClass.SINGLE_PHASE_GAS


def test_a_cap_is_a_result_when_not_strict() -> None:
    mix, ideal = fluid()
    r = state_at_tp(
        mix,
        ideal,
        T=Q(330.0, "K"),
        P=Q(2.5e6, "Pa"),
        z=[0.6, 0.4],
        tolerance=1e-300,
        strict=False,
    )
    assert r.converged is False
