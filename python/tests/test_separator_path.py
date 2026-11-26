"""``pvt.separator_path`` through its public wrapper."""

from __future__ import annotations

from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen
from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import separator_path

MODEL_ID = "pvt.separator_path"
SPEC = _models_gen.model(MODEL_ID)
CASES = SPEC["cases"]


def call(case: dict[str, Any]) -> Any:
    """Run one case declared in the model spec."""
    return separator_path(**h.model_kwargs(SPEC, case["inputs"]))


@pytest.mark.parametrize("case", CASES, ids=lambda c: c["id"])
def test_spec_case(case: dict[str, Any]) -> None:
    """Run one case declared in the model spec."""
    result = call(case)
    expected = case["expected"]
    h.assert_close(
        result.final_pressure.to("Pa").magnitude,
        expected["final_pressure"],
        case["tolerance"],
        f"{case['id']} (final_pressure)",
    )
    h.assert_close(
        result.final_temperature.to("K").magnitude,
        expected["final_temperature"],
        case["tolerance"],
        f"{case['id']} (final_temperature)",
    )
    for index, want in enumerate(expected["volumes"]):
        if want == 0.0:
            assert result.volumes[index].to("m**3").magnitude == 0.0
        else:
            h.assert_close(
                result.volumes[index].to("m**3").magnitude,
                want,
                case["tolerance"],
                f"{case['id']} (volume {index})",
            )
    if "gas_oil_ratio" in expected:
        assert result.gas_oil_ratio == pytest.approx(
            expected["gas_oil_ratio"], rel=case["tolerance"]
        )
    else:
        assert result.gas_oil_ratio is None


def test_the_one_stage_component_inventory_is_the_capture() -> None:
    case = next(case for case in CASES if case["id"] == "one_stage_stock_tank")
    result = call(case)
    gas = [value.to("mol").magnitude for value in result.component_moles[0]]
    oil = [value.to("mol").magnitude for value in result.component_moles[1]]
    assert gas[0] == pytest.approx(69.93189933043215, rel=1.0e-6)
    assert gas[3] == pytest.approx(0.08721319284227026, rel=1.0e-6)
    assert oil[0] == pytest.approx(0.06810066955169695, rel=1.0e-6)
    assert oil[3] == pytest.approx(14.912786807177062, rel=1.0e-6)


def test_a_gas_only_path_has_no_gas_oil_ratio() -> None:
    case = next(case for case in CASES if case["id"] == "gas_only")
    result = call(case)
    assert result.gas_oil_ratio is None
    assert len(result.warnings) == 1


def test_increasing_pressures_are_refused() -> None:
    with pytest.raises(InvalidInputError, match="strictly decreasing"):
        separator_path(
            [
                {
                    "fluid": 0,
                    "name": "methane",
                    "pseudo_component": False,
                    "moles": quantity(1.0, "mol"),
                }
            ],
            [quantity(1.0e6, "Pa"), quantity(2.0e6, "Pa")],
            [quantity(300.0, "K"), quantity(288.15, "K")],
        )
