"""``pvt.gas_density_consistency`` through its public wrapper."""

from __future__ import annotations

from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen
from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import gas_density_consistency

MODEL_ID = "pvt.gas_density_consistency"
SPEC = _models_gen.model(MODEL_ID)
CASES = SPEC["cases"]


def call(case: dict[str, Any]) -> Any:
    """Run one case declared in the model spec."""
    return gas_density_consistency(**h.model_kwargs(SPEC, case["inputs"]))


@pytest.mark.parametrize("case", CASES, ids=lambda c: c["id"])
def test_spec_case(case: dict[str, Any]) -> None:
    """Run one case declared in the model spec and compare every row."""
    result = call(case)
    expected = case["expected"]["rows"]
    assert len(result.rows) == len(expected)
    for index, (row, want) in enumerate(zip(result.rows, expected, strict=True)):
        for field in (
            "implied_density",
            "implied_lower",
            "implied_upper",
            "observed_minus_implied",
        ):
            h.assert_close(
                getattr(row, field).to("kg/m**3").magnitude,
                want[field],
                case["tolerance"],
                f"{case['id']} row {index} ({field})",
            )
        assert row.compatible is want["compatible"]
    assert result.all_compatible is all(row["compatible"] for row in expected)


def test_the_point_row_is_exactly_twenty() -> None:
    result = gas_density_consistency(
        [
            {
                "pressure": quantity(831446.261815324, "Pa"),
                "pressure_bound": quantity(0.0, "Pa"),
                "temperature": quantity(100.0, "K"),
                "temperature_bound": quantity(0.0, "K"),
                "compressibility": 1.0,
                "compressibility_bound": 0.0,
                "molar_mass": quantity(0.02, "kg/mol"),
                "molar_mass_bound": quantity(0.0, "kg/mol"),
                "density": quantity(20.0, "kg/m**3"),
                "density_bound": quantity(0.0, "kg/m**3"),
            }
        ]
    )
    assert result.rows[0].implied_density.to("kg/m**3").magnitude == pytest.approx(
        20.000000000000004, rel=1.0e-15
    )
    assert result.rows[0].compatible
    assert result.all_compatible


def test_a_negative_bound_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="nonnegative bound"):
        gas_density_consistency(
            [
                {
                    "pressure": quantity(100.0, "Pa"),
                    "pressure_bound": quantity(-1.0, "Pa"),
                    "temperature": quantity(300.0, "K"),
                    "temperature_bound": quantity(0.0, "K"),
                    "compressibility": 1.0,
                    "compressibility_bound": 0.0,
                    "molar_mass": quantity(0.02, "kg/mol"),
                    "molar_mass_bound": quantity(0.0, "kg/mol"),
                    "density": quantity(20.0, "kg/m**3"),
                    "density_bound": quantity(0.0, "kg/m**3"),
                }
            ]
        )
