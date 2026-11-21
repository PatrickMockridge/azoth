"""``pvt.gas_material_balance`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import gas_material_balance

PRESSURE = [
    quantity(300.0, "bar"),
    quantity(250.0, "bar"),
    quantity(200.0, "bar"),
    quantity(150.0, "bar"),
    quantity(100.0, "bar"),
]
Z = [0.9, 0.88, 0.85, 0.82, 0.80]
GP = [0.0, 1.0e6, 2.0e6, 3.0e6, 4.0e6]


def test_the_bg_and_the_pz_fit_answer() -> None:
    single = gas_material_balance(
        "bg",
        pressure_value=quantity(300.0, "bar"),
        z_value=0.9,
        temperature=quantity(350.0, "K"),
    )
    assert single.gas_fvf == pytest.approx(0.0036850559384849855, rel=1.0e-9)

    fit = gas_material_balance(
        "fit_volumetric",
        pressure=PRESSURE,
        z=Z,
        gp=GP,
        temperature=quantity(350.0, "K"),
    )
    assert fit.ogip is not None
    assert fit.pi_over_zi is not None
    assert fit.pi_over_zi.to("bar").magnitude == pytest.approx(335.6951871657755, rel=1.0e-9)


def test_the_cole_plot_answers_two_vectors() -> None:
    result = gas_material_balance(
        "cole_plot",
        pressure=PRESSURE,
        z=Z,
        gp=GP,
        temperature=quantity(350.0, "K"),
    )
    assert result.cole_gp is not None and len(result.cole_gp) == 4
    assert result.cole_f_over_eg is not None
    assert result.cole_f_over_eg[0] == pytest.approx(6769230.769230775, rel=1.0e-9)


def test_a_flat_pz_line_leaves_the_ogip_absent() -> None:
    result = gas_material_balance(
        "fit_volumetric",
        pressure=[quantity(300.0, "bar"), quantity(250.0, "bar")],
        z=[0.9, 0.75],
        gp=[0.0, 1.0e6],
    )
    assert result.ogip is None
    assert len(result.warnings) == 1


def test_a_short_history_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="at least two"):
        gas_material_balance(
            "fit_volumetric",
            pressure=[quantity(300.0, "bar")],
            z=[0.9],
            gp=[0.0],
        )


def test_an_unknown_calculation_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="calculation"):
        gas_material_balance("not_a_calculation")
