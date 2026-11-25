"""The bridge's input codec, against declarations rather than a live spec."""

from __future__ import annotations

from typing import Any

import pytest

import _helpers
from azoth.harness.errors import UndefinedInput
from azoth.harness.inputs import decode_inputs, kwargs_for, model_kwargs

_INPUTS: dict[str, dict[str, Any]] = {
    "rho": {"unit": "kg/m**3"},
    "T": {"unit": "K"},
    "flag": {"type": "boolean"},
    "mode": {"type": "enum", "values": ["a", "b"]},
    "z": {"type": "vector", "unit": "dimensionless"},
    "names": {"type": "vector_string"},
    "rows": {
        "type": "record_list",
        "fields": {
            "x": {"unit": "m"},
            "label": {"type": "string"},
            "note": {"type": "string", "optional": True},
        },
    },
    "components": {"type": "components"},
}
ENTRY = {
    "id": "test.record",
    "inputs": {name: {**declaration, "optional": True} for name, declaration in _INPUTS.items()},
}
REQUIRED = {"id": "test.required", "inputs": {"rho": {"unit": "kg/m**3"}}}


def test_a_bare_number_is_read_in_the_specs_unit() -> None:
    decoded = decode_inputs(ENTRY, {"rho": 998.0})
    assert decoded["rho"] == 998.0


def test_a_quantity_pair_is_converted_to_the_specs_unit() -> None:
    decoded = decode_inputs(ENTRY, {"T": {"value": 250.0, "unit": "K"}})
    assert decoded["T"] == 250.0


def test_a_quantity_pair_of_the_wrong_dimension_is_refused_by_name() -> None:
    with pytest.raises(UndefinedInput, match="`T`"):
        decode_inputs(ENTRY, {"T": {"value": 20.0, "unit": "kg"}})


def test_unknown_and_missing_inputs_are_named() -> None:
    with pytest.raises(UndefinedInput, match="does not declare"):
        decode_inputs(ENTRY, {"rho": 1.0, "nope": 2.0})
    with pytest.raises(UndefinedInput, match="required inputs absent"):
        decode_inputs(REQUIRED, {})


def test_vector_and_record_list_shapes_are_preserved() -> None:
    decoded = decode_inputs(
        ENTRY,
        {
            "z": [0.9, 0.1],
            "rows": [{"x": 2.0, "label": "a"}],
        },
    )
    assert decoded["z"] == [0.9, 0.1]
    assert decoded["rows"] == [{"x": 2.0, "label": "a", "note": None}]


def test_a_record_field_carries_its_own_unit() -> None:
    with pytest.raises(UndefinedInput, match=r"rows\[0\]\.x"):
        decode_inputs(ENTRY, {"rows": [{"x": {"value": 1.0, "unit": "kg"}, "label": "a"}]})


def test_a_boolean_and_an_enum_are_checked() -> None:
    decoded = decode_inputs(ENTRY, {"flag": True, "mode": "b"})
    assert decoded == {"flag": True, "mode": "b"}
    with pytest.raises(UndefinedInput, match="true or false"):
        decode_inputs(ENTRY, {"flag": 1})
    with pytest.raises(UndefinedInput, match="not one of"):
        decode_inputs(ENTRY, {"mode": "c"})


def test_components_are_names() -> None:
    decoded = decode_inputs(ENTRY, {"components": ["methane", "n-butane"]})
    assert decoded["components"] == ["methane", "n-butane"]


def test_the_bridge_conversion_agrees_with_the_spec_helper() -> None:
    from azoth._models_gen import MODELS
    from azoth._registry_gen import CALCS

    calc = next(entry for entry in CALCS if entry["id"] == "hydraulics.reynolds_number")
    inputs = {"rho": 998.0, "v": 1.5, "D": 0.1, "mu": 1.002e-3}
    assert kwargs_for(calc, inputs) == _helpers.kwargs_for(calc, inputs)

    model = next(entry for entry in MODELS if entry["id"] == "eos.pt_flash")
    case = model["cases"][0]["inputs"]
    bridge = model_kwargs(model, case)
    helper = _helpers.model_kwargs(model, case)
    assert set(bridge) == set(helper)
    assert bridge["mixture"] == helper["mixture"]
    assert bridge["T"] == helper["T"]
    assert bridge["P"] == helper["P"]
    assert bridge["z"] == helper["z"]


def test_a_vector_string_model_keeps_its_names_as_strings() -> None:
    from azoth._models_gen import MODELS

    model = next(
        entry for entry in MODELS if entry["id"] == "characterization.set_heavy_tbp_to_plus"
    )
    case = model["cases"][0]["inputs"]
    bridge = model_kwargs(model, case)
    helper = _helpers.model_kwargs(model, case)
    assert bridge["names"] == helper["names"]
    assert all(isinstance(name, str) for name in bridge["names"])
