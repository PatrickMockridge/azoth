"""``pvt.separator_test`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import separator_test

BINARY = [
    {"fluid": 0, "name": "methane", "pseudo_component": False, "moles": quantity(0.70, "mol")},
    {"fluid": 0, "name": "nC10", "pseudo_component": False, "moles": quantity(0.30, "mol")},
]
UPSTREAM = [
    {"fluid": 0, "name": name, "pseudo_component": False, "moles": quantity(moles, "mol")}
    for name, moles in [
        ("nitrogen", 0.5),
        ("CO2", 2.0),
        ("methane", 45.0),
        ("ethane", 8.0),
        ("propane", 5.0),
        ("i-butane", 1.5),
        ("n-butane", 2.5),
        ("i-pentane", 1.0),
        ("n-pentane", 1.5),
        ("n-hexane", 3.0),
        ("n-heptane", 30.0),
    ]
]
BINARY_STAGES = [
    {"pressure": quantity(50.0, "bar"), "temperature": quantity(323.15, "K")},
    {"pressure": quantity(20.0, "bar"), "temperature": quantity(313.15, "K")},
    {"pressure": quantity(5.0, "bar"), "temperature": quantity(303.15, "K")},
    {"pressure": quantity(1.01325, "bar"), "temperature": quantity(288.15, "K")},
]
TRAIN_STAGES = [
    {"pressure": quantity(50.0, "bar"), "temperature": quantity(313.15, "K")},
    {"pressure": quantity(10.0, "bar"), "temperature": quantity(303.15, "K")},
    {"pressure": quantity(1.01325, "bar"), "temperature": quantity(288.15, "K")},
]


def test_the_continuous_and_staged_modes_answer() -> None:
    continuous = separator_test(BINARY, "continuous", stages=BINARY_STAGES)
    assert continuous.stages[0].stage_gor == pytest.approx(210.19952704510527, rel=1.0e-6)
    assert continuous.stages[0].oil_fvf == pytest.approx(1.0848181343719043, rel=1.0e-6)
    assert continuous.stages[3].stage_gor == pytest.approx(233.68586355180275, rel=1.0e-6)

    staged = separator_test(BINARY, "staged", stages=BINARY_STAGES)
    assert staged.stages[0].stage_gor == pytest.approx(210.0998570385, rel=1.0e-6)
    assert staged.stages[3].stage_gor == pytest.approx(1.893169014075703, rel=1.0e-6)


def test_the_train_answers_its_totals_and_stages() -> None:
    result = separator_test(
        UPSTREAM,
        "train",
        stages=TRAIN_STAGES,
        reservoir_pressure=quantity(300.0, "bar"),
        reservoir_temperature=quantity(373.15, "K"),
    )
    assert result.total_gor == pytest.approx(239.47388503058167, rel=1.0e-6)
    assert result.bo == pytest.approx(1.8173491732538267, rel=1.0e-6)
    assert result.rs == pytest.approx(result.total_gor)
    assert result.stock_tank_oil_density is not None
    assert result.stock_tank_oil_density.to("kg/m**3").magnitude == pytest.approx(
        685.0479302241214, rel=1.0e-6
    )
    assert result.stock_tank_api == pytest.approx(74.84833529641193, rel=1.0e-6)

    stage = result.stages[0]
    assert stage.gas_volume is not None
    assert stage.gas_volume.to("m**3").magnitude == pytest.approx(0.9700963855280732, rel=1.0e-6)
    assert stage.gas_density is not None
    assert stage.gas_density.to("kg/m**3").magnitude == pytest.approx(43.10040883414266, rel=1.0e-6)
    # The id's stage density, not the class's one-stage-lagged cache read.
    assert stage.oil_density is not None
    assert stage.oil_density.to("kg/m**3").magnitude == pytest.approx(602.4395773, rel=1.0e-6)


def test_the_first_stage_optimisation_answers() -> None:
    result = separator_test(
        UPSTREAM,
        "train",
        stages=TRAIN_STAGES,
        reservoir_pressure=quantity(300.0, "bar"),
        reservoir_temperature=quantity(373.15, "K"),
        optimize_first_stage=True,
        first_stage_pressure_min=quantity(5.0, "bar"),
        first_stage_pressure_max=quantity(20.0, "bar"),
        first_stage_pressure_steps=3,
        first_stage_temperature_min=quantity(293.15, "K"),
        first_stage_temperature_max=quantity(313.15, "K"),
        first_stage_temperature_steps=2,
    )
    assert result.optimal_stage_pressure is not None
    assert result.optimal_stage_pressure.to("Pa").magnitude == pytest.approx(2.0e6)
    assert result.optimal_stage_temperature is not None
    assert result.optimal_stage_temperature.to("K").magnitude == pytest.approx(293.15)
    assert result.bo_at_optimum == pytest.approx(1.8041071234377508, rel=5.0e-2)
    assert result.gor_at_optimum == pytest.approx(236.48088841268523, rel=5.0e-2)


def test_the_mode_and_the_reservoir_conditions_are_checked() -> None:
    with pytest.raises(InvalidInputError, match="at least one stage"):
        separator_test(BINARY, "continuous", stages=[])

    with pytest.raises(InvalidInputError, match="continuous, staged or train"):
        separator_test(BINARY, "bogus", stages=BINARY_STAGES)

    with pytest.raises(InvalidInputError, match="reservoir pressure"):
        separator_test(UPSTREAM, "train", stages=TRAIN_STAGES)
