"""`process.throttling_valve`'s sizing-standard surface: the wrapper, the two-phase split and
the per-standard `allow_choked` initialiser.

The arithmetic is held by the spec's cases and `test_cross_impl.py`. These are the facts a case
cannot say: which inputs the public wrapper takes, and which standards read a two-phase inlet.
"""

from __future__ import annotations

import inspect
from typing import Any

import pytest

import _helpers as h
from azoth import _models_gen
from azoth._dispatch import available, resolve, use_backend
from azoth.core.errors import InvalidInputError
from azoth.process import throttling_valve as run_valve

MODEL_ID = "process.throttling_valve"
SPEC = _models_gen.model(MODEL_ID)

RICH = {
    "components": ["methane", "ethane", "propane", "n-butane"],
    "inlet_n": 97.21961972278174,
    "inlet_z": [0.1, 0.2, 0.4, 0.3],
    "inlet_p": 182925.0,
    "inlet_t": 296.25,
    "outlet_pressure": 141325.0,
}
WELL = {
    "components": ["methane", "n-butane"],
    "inlet_n": 1.0,
    "inlet_z": [0.7, 0.3],
    "inlet_p": 2_000_000.0,
    "inlet_t": 300.0,
    "outlet_pressure": 1_500_000.0,
}


def _run(inputs: dict[str, Any], backend: str) -> Any:
    with use_backend(backend):
        return resolve(MODEL_ID)(**h.model_kwargs(SPEC, inputs))


def test_the_public_wrapper_takes_every_declared_input() -> None:
    declared = set(SPEC["inputs"])
    accepted = set(inspect.signature(run_valve).parameters)
    assert accepted == declared, sorted(declared - accepted)


@pytest.mark.parametrize("backend", sorted(available()))
@pytest.mark.parametrize("standard", ["default", "iec_60534", "iec_60534_full", "prod_choke"])
def test_the_single_phase_standards_refuse_a_two_phase_inlet(backend: str, standard: str) -> None:
    inputs = {**WELL, "sizing_standard": standard}
    with pytest.raises(InvalidInputError, match="single phase"):
        _run(inputs, backend)


@pytest.mark.parametrize("backend", sorted(available()))
@pytest.mark.parametrize("standard", ["sachdeva", "gilbert", "baxendell", "ros", "achong"])
def test_the_choke_standards_accept_a_two_phase_inlet(backend: str, standard: str) -> None:
    inputs = {**WELL, "sizing_standard": standard, "choke_diameter": 0.0127}
    result = _run(inputs, backend)
    assert result.kv > 0.0
    assert result.choked is False


@pytest.mark.parametrize("backend", sorted(available()))
def test_allow_choked_absent_follows_the_selected_standard(backend: str) -> None:
    default = _run({**RICH, "sizing_standard": "default"}, backend)
    iec = _run({**RICH, "sizing_standard": "iec_60534"}, backend)
    assert default.kv < iec.kv
    stated = _run({**RICH, "sizing_standard": "default", "allow_choked": True}, backend)
    assert stated.kv == pytest.approx(iec.kv, rel=1.0e-4)
