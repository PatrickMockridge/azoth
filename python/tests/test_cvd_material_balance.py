"""``pvt.cvd_material_balance`` through its public wrapper."""

from __future__ import annotations

from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen
from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import cvd_material_balance

MODEL_ID = "pvt.cvd_material_balance"
SPEC = _models_gen.model(MODEL_ID)
CASES = SPEC["cases"]


def call(case: dict[str, Any]) -> Any:
    """Run one case declared in the model spec."""
    return cvd_material_balance(**h.model_kwargs(SPEC, case["inputs"]))


@pytest.mark.parametrize("case", CASES, ids=lambda c: c["id"])
def test_spec_case(case: dict[str, Any]) -> None:
    """Run one case declared in the model spec and compare every stage field."""
    result = call(case)
    expected = case["expected"]["stages"]
    assert len(result.stages) == len(expected)
    for index, (stage, want) in enumerate(zip(result.stages, expected, strict=True)):
        for field in (
            "total_moles",
            "vapor_moles",
            "liquid_moles",
            "stage_withdrawn_moles",
            "component_closure",
            "remaining_closure",
            "produced_closure",
            "vapor_closure",
        ):
            h.assert_close(
                getattr(stage, field),
                want[field],
                case["tolerance"],
                f"{case['id']} stage {index} ({field})",
                absolute_floor=1.0e-12,
            )
        assert stage.liquid_state == want["liquid_state"]
        assert stage.compatible is want["compatible"]
    assert result.compatible is all(stage["compatible"] for stage in expected)


def test_the_analytic_binary_matrices_are_the_capture() -> None:
    case = next(case for case in CASES if case["id"] == "analytic_binary")
    result = call(case)
    assert result.produced_component_moles[1] == pytest.approx(
        (0.18000000000000002, 0.020000000000000004), rel=1.0e-15
    )
    assert result.remaining_component_moles[2] == pytest.approx((0.44, 0.16), rel=1.0e-15)
    assert result.liquid_component_moles[1] == pytest.approx(
        (0.25999999999999995, 0.13999999999999999), rel=1.0e-15
    )
    assert result.liquid_component_moles[2] == pytest.approx((0.26, 0.14), rel=1.0e-15)


def test_the_ns1_inventory_is_the_capture() -> None:
    case = next(case for case in CASES if case["id"] == "ns1_table1")
    result = call(case)
    assert result.stages[1].liquid_moles == pytest.approx(0.11367529644728536, rel=1.0e-14)
    assert result.produced_component_moles[1][0] == pytest.approx(0.00216576, rel=1.0e-14)
    assert result.liquid_component_moles[1][0] == pytest.approx(0.0024282071147348488, rel=1.0e-14)
    assert result.liquid_component_moles[1][10] == pytest.approx(0.027147868379731703, rel=1.0e-14)


def test_a_composition_that_does_not_sum_to_one_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="sum to one"):
        cvd_material_balance(
            ["A", "B"],
            [
                {
                    "pressure": quantity(100.0e5, "Pa"),
                    "gas_z": 1.0,
                    "liquid_volume_fraction": 0.0,
                    "cumulative_produced": 0.0,
                },
                {
                    "pressure": quantity(50.0e5, "Pa"),
                    "gas_z": 1.0,
                    "liquid_volume_fraction": 0.2,
                    "cumulative_produced": 0.2,
                },
            ],
            [[0.5, 0.5], [0.8, 0.3]],
            1.0e-12,
            1.0e-8,
        )


def test_the_first_row_must_be_the_saturation_reference() -> None:
    with pytest.raises(InvalidInputError, match="saturation reference"):
        cvd_material_balance(
            ["A"],
            [
                {
                    "pressure": quantity(100.0, "Pa"),
                    "gas_z": 1.0,
                    "liquid_volume_fraction": 0.1,
                    "cumulative_produced": 0.0,
                },
                {
                    "pressure": quantity(50.0, "Pa"),
                    "gas_z": 1.0,
                    "liquid_volume_fraction": 0.2,
                    "cumulative_produced": 0.2,
                },
            ],
            [[1.0], [1.0]],
            1.0e-12,
            1.0e-8,
        )
