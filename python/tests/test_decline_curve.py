"""``pvt.decline_curve`` through its public wrapper."""

from __future__ import annotations

import math

import pytest

from azoth.core.errors import InvalidInputError
from azoth.pvt import decline_curve


def test_the_arps_forms_answer() -> None:
    rate = decline_curve("rate", qi=1000.0, di=0.001, b=0.5, t=365.25)
    assert rate.rate_value == pytest.approx(714.9997484653228, rel=1.0e-9)
    cumulative = decline_curve("cumulative_production", qi=1000.0, di=0.001, b=0.5, t=3650.0)
    assert cumulative.cumulative_value == pytest.approx(1292035.3982300884, rel=1.0e-9)


def test_the_forecast_publishes_three_vectors() -> None:
    result = decline_curve(
        "forecast", qi=1000.0, di=0.001, b=0.5, t_start=0.0, t_end=3650.0, t_step=365.0
    )
    assert result.time is not None and len(result.time) == 11
    assert result.rate is not None and result.cumulative is not None
    assert result.time[10] == pytest.approx(3650.0)
    assert result.rate[1] == pytest.approx(715.1509191924157, rel=1.0e-9)
    assert result.cumulative[10] == pytest.approx(1292035.3982300884, rel=1.0e-9)


def test_the_fits_answer() -> None:
    times = [index * 365.0 for index in range(11)]
    rates = [1000.0 * math.exp(-0.001 * time) for time in times]
    result = decline_curve("fit_arps", history_time=times, history_rate=rates)
    assert result.fit_qi == pytest.approx(1000.0, rel=1.0e-9)
    assert result.fit_di == pytest.approx(0.001, rel=1.0e-6)
    assert result.fit_b == pytest.approx(0.0, abs=1.0e-12)


def test_a_zero_decline_is_absent_with_a_warning() -> None:
    result = decline_curve("time_to_rate", qi=1000.0, di=0.0, b=0.5, q_limit=50.0)
    assert result.time_to_rate is None
    assert len(result.warnings) == 1
    assert result.warnings[0].code == "OUT_OF_VALID_RANGE"


def test_a_missing_scalar_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="qi"):
        decline_curve("rate")


def test_a_short_history_is_refused() -> None:
    with pytest.raises(InvalidInputError, match="at least three"):
        decline_curve("fit_arps", history_time=[0.0, 1.0], history_rate=[100.0, 90.0])
