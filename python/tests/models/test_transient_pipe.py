"""Spec-driven tests for the ``eos.transient_pipe`` model.

The spec's cases pin both implementations to the NeqSim capture. These tests pin what a
case cannot state: the refusals for the unsupported three-phase, accumulation-zone and
transient-boundary branches, and the two backends' agreement on both cases.
"""

from __future__ import annotations

from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen, ureg, use_backend
from azoth.core.errors import InvalidInputError
from azoth.core.result import TransientPipeResult
from azoth.eos import transient_pipe

MODEL_ID = "eos.transient_pipe"
Q = ureg.Quantity

SPEC = _models_gen.model(MODEL_ID)
CASES = SPEC["cases"]


def call(case: dict[str, Any]) -> TransientPipeResult:
    return transient_pipe(**h.model_kwargs(SPEC, case["inputs"]))


@pytest.mark.parametrize("case", CASES, ids=lambda c: c["id"])
def test_spec_case(case: dict[str, Any]) -> None:
    result = call(case)
    tolerance = case["tolerance"]
    expected = case["expected"]
    for name in (
        "pressure_profile",
        "temperature_profile",
        "liquid_holdup_profile",
        "gas_velocity_profile",
        "liquid_velocity_profile",
    ):
        unit = {
            "pressure_profile": "Pa",
            "temperature_profile": "K",
            "gas_velocity_profile": "m/s",
            "liquid_velocity_profile": "m/s",
        }.get(name)
        actual = [
            value.to(unit).magnitude if unit is not None else value
            for value in getattr(result, name)
        ]
        assert len(actual) == len(expected[name]), f"{case['id']}: {name} length"
        for index, (got, want) in enumerate(zip(actual, expected[name], strict=True)):
            h.assert_close(got, want, tolerance, f"{case['id']} ({name}[{index}])")
    assert len(result.pressure_history) == len(expected["pressure_history"])
    for row, (actual_row, expected_row) in enumerate(
        zip(result.pressure_history, expected["pressure_history"], strict=True)
    ):
        assert len(actual_row) == len(expected_row)
        for column, (got, want) in enumerate(zip(actual_row, expected_row, strict=True)):
            h.assert_close(
                got.to("Pa").magnitude,
                want,
                tolerance,
                f"{case['id']} (pressure_history[{row}][{column}])",
            )
    h.assert_close(
        result.mass_inventory.to("kg").magnitude,
        expected["mass_inventory"],
        tolerance,
        f"{case['id']} (mass_inventory)",
    )
    assert result.simulation_time >= Q(case["inputs"]["max_simulation_time"], "s")
    assert result.total_time_steps > 0
    h.assert_consistent(result, case["id"])


@pytest.mark.parametrize("case", CASES, ids=lambda c: c["id"])
def test_the_backends_agree(case: dict[str, Any]) -> None:
    kwargs = h.model_kwargs(SPEC, case["inputs"])
    with use_backend("python"):
        python = transient_pipe(**kwargs)
    with use_backend("rust"):
        rust = transient_pipe(**kwargs)
    h.assert_results_equal(python, rust, case["tolerance"], f"{MODEL_ID}::{case['id']}")


def test_the_three_phase_flash_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="oil and an aqueous"):
        transient_pipe(
            ["methane", "n-heptane", "water"],
            [0.5, 0.3, 0.2],
            Q(5.0, "kg/s"),
            Q(5.0e6, "Pa"),
            Q(300.0, "K"),
            Q(100.0, "m"),
            Q(0.1, "m"),
            5,
            Q(1.0e-4, "m"),
            "constant_flow",
            "constant_pressure",
            outlet_pressure=Q(4.0e6, "Pa"),
            max_simulation_time=Q(1.0, "s"),
            cfl=0.5,
            thermodynamic_update_interval=10,
            eos="srk",
        )


def test_a_low_point_is_refused() -> None:
    case = CASES[0]
    kwargs = h.model_kwargs(SPEC, case["inputs"])
    kwargs["elevation_profile"] = [0.0, -1.0, 0.0, 0.0, 0.0]
    with pytest.raises(InvalidInputError, match="accumulation"):
        transient_pipe(**kwargs)


def test_a_boundary_outside_the_three_is_refused() -> None:
    case = CASES[0]
    kwargs = h.model_kwargs(SPEC, case["inputs"])
    kwargs["inlet_boundary"] = "transient_flow"
    with pytest.raises(InvalidInputError, match="transient_flow"):
        transient_pipe(**kwargs)
