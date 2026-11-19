"""The cross-implementation cases too slow for the lanes.

The real-gas Fanno march runs thousands of PH flashes per state in the Python mirror, so
its cases are held out of ``test_cross_impl`` and run here under ``slow_sweep``: the
nightly carries them, and a caller who wants them runs this module directly.
"""

from __future__ import annotations

from typing import Any

import pytest

import azoth._models_gen as _models_gen
from test_cross_impl import SLOW_MODELS, assert_model_agrees

pytestmark = [pytest.mark.requires_rust, pytest.mark.slow_sweep]


def _slow_cases() -> list[tuple[dict[str, Any], dict[str, Any]]]:
    return [
        (model, case)
        for model in _models_gen.MODELS
        if model["id"] in SLOW_MODELS
        for case in model["cases"]
    ]


@pytest.mark.parametrize(
    ("model", "case"),
    _slow_cases(),
    ids=lambda x: x["id"] if isinstance(x, dict) else "",
)
def test_the_slow_model_cases_agree(model: dict[str, Any], case: dict[str, Any]) -> None:
    """Run one slow model case through both implementations and compare."""
    assert_model_agrees(model, case)
