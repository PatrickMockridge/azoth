"""The intermediates the PVT reference kernels already compute, as named rows.

    from azoth.pvt import layers
    layers.rows("pvt.differential_liberation", {...})

A case compares the experiment's published arrays, and an array that is out says nothing
about which step moved first. The probe captures the same per-step quantities - saturation
pressure, phase compositions, Z roots, corrected volumes, densities, viscosities and the
cumulative moles a ledger removed - and those already exist inside the reference kernels.
This module exposes them by name so ``tools/neqsim_layer_diff.py`` can put the two side by
side.

The key names are the capture's, so a capture and a dump meet without a mapping table. A key
a dump does not carry is not compared; a dumper whose key set shares nothing with its
capture is an error rather than a pass.

No model gains an output here: this is an introspection surface, and a caller who wants an
answer calls the id.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any

#: A model's inputs, keyed as a case states them, to its intermediate rows.
Dumper = Callable[[Mapping[str, Any]], dict[str, float]]

#: Models whose per-step arithmetic is dumped by a function here.
DUMPERS: dict[str, Dumper] = {}

#: Record-list models with no interior to dump: every published row is the answer. The value
#: says why, and the layer-diff test counts each as evidence rather than silently skipping it.
NO_INTERIOR: dict[str, str] = {
    "pvt.black_oil_correlations": (
        "each row is one correlation's answer; no state lies between inputs and value"
    ),
    "pvt.gas_correlations": (
        "each row is one correlation's answer; no state lies between inputs and value"
    ),
    "pvt.water_correlations": (
        "each row is one correlation's answer; no state lies between inputs and value"
    ),
    "pvt.decline_curve": (
        "each call is one closed-form answer; the fits' grid search is the answer, not an interior"
    ),
}

#: Capture blocks that are evidence rather than a case - a refusal, a fallback or a fluid the
#: id cannot express - and the count of rows in each. A block is not excused by widening a
#: tolerance; it is a measured row this id does not publish.
UNCASED_ROWS: dict[str, int] = {}


def rows(model_id: str, inputs: Mapping[str, Any]) -> dict[str, float]:
    """One model's named intermediates at one state.

    Raises:
        KeyError: for a model with no dumper, naming what is known.
    """
    dumper = DUMPERS.get(model_id)
    if dumper is None:
        raise KeyError(f"no PVT layer dumper for {model_id!r}; known: {sorted(DUMPERS)}")
    return dumper(inputs)
