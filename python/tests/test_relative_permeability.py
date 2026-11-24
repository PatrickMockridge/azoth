"""``pvt.relative_permeability`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.pvt import relative_permeability


def test_the_default_table_is_swof_at_twenty_rows() -> None:
    result = relative_permeability()
    assert result.table_type == "SWOF"
    assert result.saturation_name == "Sw"
    assert len(result.table) == 20
    assert result.table[0].sw == 0.0
    assert result.table[0].krw == 0.0
    assert result.table[0].krow == 1.0
    assert result.table[19].sw == 1.0
    assert result.table[19].krw == 1.0
    assert result.table[19].krow == 0.0
    assert result.table[0].pcow is not None


def test_the_sgof_table_is_the_capture() -> None:
    result = relative_permeability(table_type="sgof", rows=5)
    assert result.table_type == "SGOF"
    assert result.saturation_name == "Sg"
    assert result.table[1].krg == pytest.approx(0.0625, rel=1.0e-12)
    assert result.table[1].krog == pytest.approx(0.5625, rel=1.0e-12)
    assert result.table[4].krg == 1.0


def test_the_let_table_is_the_capture() -> None:
    result = relative_permeability(
        model_family="let",
        rows=5,
        swc=0.15,
        sorw=0.2,
        krw_max=0.3,
        lo=2.5,
        eo=1.25,
        to=1.75,
        lw=1.2,
        ew=1.5,
        tw=2.0,
    )
    assert result.table[1].krw == pytest.approx(0.05501216576859834, rel=1.0e-12)
    assert result.table[1].krow == pytest.approx(0.8151257300970868, rel=1.0e-12)
    assert result.table[4].krw == 0.3
    assert result.table[4].krow == 0.0


def test_a_descending_grid_is_published() -> None:
    result = relative_permeability(rows=5, swc=0.9, sorw=0.2)
    first = result.table[0].sw
    last = result.table[4].sw
    assert first is not None and last is not None
    assert first > last
    assert result.table[0].krw == 0.0
    assert result.table[4].krow == 0.0


def test_a_refused_state_is_named() -> None:
    with pytest.raises(InvalidInputError, match="three rows"):
        relative_permeability(rows=2)
    with pytest.raises(InvalidInputError, match="not_a_table"):
        relative_permeability(table_type="not_a_table")
