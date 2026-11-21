"""The ``pvt`` namespace's front door."""

from __future__ import annotations

import pytest

import azoth.pvt
from azoth.pvt import layers


def test_the_namespace_is_importable_without_the_extension() -> None:
    assert azoth.pvt.__doc__
    assert "pvtsimulation" in azoth.pvt.__doc__


def test_an_unknown_layer_dump_is_refused() -> None:
    with pytest.raises(KeyError, match="no PVT layer dumper"):
        layers.rows("pvt.not_an_id", {})
