"""Spec-driven tests for the ``eos.pipeline_transient`` model.

The spec's case pins the two implementations to each other. These tests add the two
things a case cannot state: the conservation identities the stage promises, and the CFL
refusal.
"""

from __future__ import annotations

from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen
from azoth.core.errors import OutOfRangeError
from azoth.core.result import PipelineTransientResult
from azoth.eos import pipeline_transient

MODEL_ID = "eos.pipeline_transient"

SPEC = _models_gen.model(MODEL_ID)
CASES = SPEC["cases"]


def call(case: dict[str, Any]) -> PipelineTransientResult:
    return pipeline_transient(**h.model_kwargs(SPEC, case["inputs"]))


@pytest.mark.parametrize("case", CASES, ids=lambda c: c["id"])
def test_spec_case(case: dict[str, Any]) -> None:
    result = call(case)
    tolerance = case["tolerance"]
    expected = case["expected"]
    units = {
        "final_velocity": "m/s",
        "final_pressure": "Pa",
        "final_temperature": "K",
        "final_vapour_fraction": None,
        "final_density": "kg/m**3",
        "final_liquid_volume_fraction": None,
        "boundary_component_outflow": "mol",
        "initial_component_holdup": "mol",
        "final_component_holdup": "mol",
    }
    for name, unit in units.items():
        actual = getattr(result, name)
        want = expected[name]
        assert len(actual) == len(want), f"{case['id']}: {name} length"
        for i, (got, expected_value) in enumerate(zip(actual, want, strict=True)):
            value = got.to(unit).magnitude if unit is not None else got
            h.assert_close(value, expected_value, tolerance, f"{case['id']} ({name}[{i}])")
    for name, unit in (
        ("min_time_step", "s"),
        ("initial_mass", "kg"),
        ("final_mass", "kg"),
        ("boundary_mass_outflow", "kg"),
    ):
        h.assert_close(getattr(result, name).to(unit).magnitude, expected[name], tolerance, name)
    assert result.steps == expected["steps"]
    assert result.rejections == expected["rejections"]
    h.assert_consistent(result, case["id"])


def test_mass_and_component_inventories_close() -> None:
    result = call(CASES[0])
    h.assert_close(
        result.initial_mass.to("kg").magnitude - result.final_mass.to("kg").magnitude,
        result.boundary_mass_outflow.to("kg").magnitude,
        1e-9,
        "mass balance",
    )
    for i, (initial, final, outflow) in enumerate(
        zip(
            result.initial_component_holdup,
            result.final_component_holdup,
            result.boundary_component_outflow,
            strict=True,
        )
    ):
        h.assert_close(
            initial.to("mol").magnitude - final.to("mol").magnitude,
            outflow.to("mol").magnitude,
            1e-9,
            f"component {i} balance",
        )


def test_a_step_above_the_initial_cfl_limit_is_refused() -> None:
    inputs = dict(CASES[0]["inputs"])
    inputs["time_step"] = 1000.0
    with pytest.raises(OutOfRangeError, match="time_step"):
        pipeline_transient(**h.model_kwargs(SPEC, inputs))
