"""The ``pvt`` namespace's layer dumpers, held to the ids they claim.

`tools/neqsim_layer_diff.py` compares a model's intermediates against its probe capture.
This file holds the two things that can be wrong before any comparison runs: a dumper that
names an id the registry does not have, and an id that is neither dumped nor declared
interiorless. Both are invisible in a green total, and both were the shape of the process
namespace's first layer-diff defect.
"""

from __future__ import annotations

from azoth._models_gen import MODEL_BY_ID, MODELS
from azoth.pvt import layers


def pvt_model_ids() -> set[str]:
    return {entry["id"] for entry in MODELS if str(entry["id"]).startswith("pvt.")}


def test_every_dumper_names_a_registered_model() -> None:
    assert set(layers.DUMPERS) <= set(MODEL_BY_ID)
    assert set(layers.NO_INTERIOR) <= set(MODEL_BY_ID)
    assert set(layers.DUMPERS).isdisjoint(layers.NO_INTERIOR)


def test_every_pvt_model_is_dumped_or_declared_interiorless() -> None:
    assert pvt_model_ids() <= set(layers.DUMPERS) | set(layers.NO_INTERIOR)
