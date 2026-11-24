"""``pvt.generalized_material_balance`` through its public wrapper."""

from __future__ import annotations

from typing import Any

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import generalized_material_balance

INITIAL: dict[str, Any] = {
    "initial_pressure": quantity(3.0e7, "Pa"),
    "initial_rs": 100.0,
    "initial_bo": 1.6,
    "initial_bg": 0.004,
    "initial_bw": 1.02,
    "initial_rv": 0.0002,
}


def column(name: str, value: float) -> Any:
    if name == "pressure":
        return quantity(value, "Pa")
    if name in ("net_oil_produced", "net_gas_produced", "net_water_produced", "water_influx"):
        return quantity(value, "m**3")
    return value


def history(oil: float, gas: float, support: bool) -> list[dict[str, object]]:
    bo, bg, rv = 1.6, 0.004, 0.0002
    records = [
        (260.0, 80.0, 1.5, 0.005, 0.00016),
        (210.0, 60.0, 1.4, 0.008, 0.00012),
        (160.0, 40.0, 1.3, 0.012, 0.00008),
        (110.0, 20.0, 1.2, 0.020, 0.00004),
    ]
    initial_volume = oil * bo + gas * bg
    initial_oil = oil + rv * gas
    initial_gas = gas + 100.0 * oil
    out: list[dict[str, object]] = []
    for index, (pressure, rs, current_bo, current_bg, current_rv) in enumerate(records):
        fraction = 0.001 * (index + 1) if support else 0.0
        influx = 0.002 * initial_volume * (index + 1) if support else 0.0
        net_water = -0.001 * initial_volume * (index + 1) if support else 0.0
        remaining = initial_volume * (1.0 - fraction) - influx + net_water * 1.02
        oil_volume = remaining * (0.01 * (index + 1) if oil == 0.0 else 0.58 - 0.08 * index)
        gas_volume = remaining - oil_volume
        free_oil = oil_volume / current_bo
        free_gas = gas_volume / current_bg
        row = {
            "pressure": pressure * 1.0e5,
            "rs": rs,
            "bo": current_bo,
            "bg": current_bg,
            "bw": 1.02,
            "rv": current_rv,
            "net_oil_produced": initial_oil - (free_oil + current_rv * free_gas),
            "net_gas_produced": initial_gas - (free_gas + rs * free_oil),
            "net_water_produced": net_water,
            "water_influx": influx,
            "expansion_fraction": fraction,
        }
        out.append({name: column(name, value) for name, value in row.items()})
    return out


def test_the_oil_and_gas_fit_is_the_capture() -> None:
    result = generalized_material_balance(
        "oil_and_gas", observations=history(1.0e7, 2.0e9, False), **INITIAL
    )
    assert result.free_oil.to("m**3").magnitude == pytest.approx(9999999.999999976, rel=1.0e-9)
    assert result.free_gas.to("m**3").magnitude == pytest.approx(2000000000.0000012, rel=1.0e-9)
    assert result.gas_cap_ratio == pytest.approx(0.5000000000000014, rel=1.0e-9)
    assert result.condition_number == pytest.approx(10.34514772279802, rel=1.0e-9)


def test_the_known_gas_cap_folds_the_ratio_into_one_predictor() -> None:
    result = generalized_material_balance(
        "known_gas_cap",
        gas_cap_ratio=0.5,
        observations=history(1.0e7, 2.0e9, True),
        **INITIAL,
    )
    assert result.free_oil.to("m**3").magnitude == pytest.approx(1.0e7, rel=1.0e-9)
    assert result.free_gas.to("m**3").magnitude == pytest.approx(2.0e9, rel=1.0e-9)
    assert result.condition_number == pytest.approx(1.0, rel=1.0e-12)


def test_the_initially_gas_fit_leaves_the_ratio_absent_with_a_warning() -> None:
    result = generalized_material_balance(
        "initially_gas", observations=history(0.0, 2.0e9, False), **INITIAL
    )
    assert result.free_oil.to("m**3").magnitude == 0.0
    assert result.gas_cap_ratio is None
    assert len(result.warnings) == 1


def test_a_refused_state_is_named() -> None:
    with pytest.raises(InvalidInputError, match="at least two"):
        generalized_material_balance(
            "oil_and_gas", observations=history(1.0e7, 2.0e9, False)[:1], **INITIAL
        )
    with pytest.raises(InvalidInputError, match="Rs\\*Rv"):
        generalized_material_balance(
            "oil_and_gas",
            observations=history(1.0e7, 2.0e9, False),
            **{**INITIAL, "initial_rv": 0.01},
        )
