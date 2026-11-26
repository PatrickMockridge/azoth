"""The PVT laboratory ids agree across the two implementations at a tight tolerance.

``test_cross_impl`` holds every model case to the case's recorded oracle tolerance, which is
measured against NeqSim. This is the other question: the Rust kernel and the Python reference
are mirrors with no oracle between them, so they should agree far tighter than either agrees
with the class. A divergence here is a translation mistake, not an oracle difference.
"""

from __future__ import annotations

from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen
from azoth._dispatch import resolve, use_backend

pytestmark = pytest.mark.requires_rust

#: The pvt laboratory ids; the scalar correlation ids keep the case's own tolerance.
TIGHT_IDS = frozenset(
    {
        "pvt.saturation_test",
        "pvt.constant_mass_expansion",
        "pvt.cvd_material_balance",
        "pvt.differential_liberation",
        "pvt.gas_density_consistency",
        "pvt.constant_volume_depletion",
        "pvt.separator_path",
        "pvt.separator_test",
        "pvt.swelling_test",
        "pvt.mmp",
        "pvt.property_sweep",
    }
)

TIGHT = 1.0e-8

CASES = [
    (model, case)
    for model in _models_gen.MODELS
    if model["id"] in TIGHT_IDS
    for case in model["cases"]
]


@pytest.mark.parametrize(
    ("model", "case"),
    CASES,
    ids=lambda x: x["id"] if isinstance(x, dict) else "",
)
def test_the_pvt_mirrors_agree(model: dict[str, Any], case: dict[str, Any]) -> None:
    """Run one case through both implementations and compare at ``1e-8``."""
    kwargs = h.model_kwargs(model, case["inputs"])
    context = f"{model['id']}::{case['id']}"
    with use_backend("python"):
        py = resolve(model["id"])(**kwargs)
    with use_backend("rust"):
        rs = resolve(model["id"])(**kwargs)
    h.assert_results_equal(py, rs, TIGHT, context)
