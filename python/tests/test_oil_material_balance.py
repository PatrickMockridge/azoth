"""``pvt.oil_material_balance`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.pvt import oil_material_balance


def test_the_terms_answer() -> None:
    withdrawal = oil_material_balance(
        "withdrawal_f", np=1.0e6, rp=500.0, bo=1.2, rs=300.0, bg=0.005, wp=1.0e5, bw=1.01
    )
    assert withdrawal.f == pytest.approx(2301000.0, rel=1.0e-9)
    expansion = oil_material_balance("eo", bo=1.2, boi=1.3, rsi=400.0, rs=300.0, bg=0.005)
    assert expansion.eo_value == pytest.approx(0.4, rel=1.0e-9)


def test_the_fits_answer() -> None:
    depletion = oil_material_balance(
        "fit_depletion_drive",
        f_history=[1.0e6, 2.0e6, 3.0e6],
        eo_history=[0.1, 0.2, 0.3],
    )
    assert depletion.ooip == pytest.approx(1.0e7, rel=1.0e-9)
    assert depletion.m_fit == pytest.approx(0.0, abs=1.0e-12)

    gas_cap = oil_material_balance(
        "fit_gas_cap_drive",
        f_history=[1.03e6, 2.036e6, 3.06e6],
        eo_history=[0.1, 0.2, 0.3],
        eg_history=[0.015, 0.018, 0.03],
    )
    assert gas_cap.ooip == pytest.approx(1.0e7, rel=1.0e-9)
    assert gas_cap.m_fit == pytest.approx(0.2, rel=1.0e-9)

    water = oil_material_balance(
        "fit_water_drive",
        f_history=[1.0e6, 2.101e6, 3.202e6],
        eo_history=[0.1, 0.2, 0.3],
        we_history=[0.0, 1.0e5, 2.0e5],
        bw=1.01,
    )
    assert water.ooip == pytest.approx(1.0e7, rel=1.0e-9)


def test_the_drive_indices_answer() -> None:
    result = oil_material_balance(
        "drive_indices",
        n=1.0e7,
        m=0.2,
        eo_term=0.2,
        eg_term=0.02,
        efw_term=0.001,
        we_value=1.0e5,
        bw=1.01,
        f_value=3.0e6,
    )
    assert result.ddi == pytest.approx(0.6666666666666666, rel=1.0e-9)
    assert result.sdi == pytest.approx(0.013333333333333334, rel=1.0e-9)
    assert result.wdi == pytest.approx(0.033666666666666664, rel=1.0e-9)
    assert result.edi == pytest.approx(0.0033333333333333335, rel=1.0e-9)


def test_a_zero_eo_in_the_water_drive_plot_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"eo_history\[1\]"):
        oil_material_balance(
            "fit_water_drive",
            f_history=[1.0e6, 2.0e6],
            eo_history=[0.1, 0.0],
            we_history=[0.0, 1.0e5],
            bw=1.01,
        )


def test_an_unknown_calculation_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="calculation"):
        oil_material_balance("not_a_calculation")
