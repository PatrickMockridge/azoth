"""The safety catalogue, held to the model specs it is projected from.

`tools/gen_safety.py` reads `specs/models/process/` and emits the entries the safety workspace
and the overview carry. The family is the source path, so this test also holds the two relief
ids the upstream tree files outside `process/safety/` to their stated exception.
"""

from __future__ import annotations

import importlib
import shutil
import sys
from pathlib import Path
from types import ModuleType
from typing import Any

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]


def tool() -> ModuleType:
    sys.path.insert(0, str(REPO_ROOT / "tools"))
    try:
        return importlib.import_module("gen_safety")
    finally:
        sys.path.pop(0)


def test_the_generated_file_is_what_the_models_emit(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # The check formats its Rust through `rustfmt`, which the Python job may not carry; the
    # Rust-test job runs the same check with the toolchain.
    if shutil.which("rustfmt") is None:
        pytest.skip("rustfmt is not on this runner; rust-test runs the generated-file check")
    module: Any = tool()
    monkeypatch.setattr(sys, "argv", ["gen_safety.py", "--check"])
    module.main()


def test_the_catalogue_is_every_safety_model_and_nothing_else() -> None:
    module: Any = tool()
    specs = [spec for _, spec in module.documents()]
    ids = [spec["id"] for spec in specs]
    assert len(ids) == len(set(ids))
    assert len(ids) == 39
    families = [module.family_of(spec) for spec in specs]
    assert families.count("dispersion") == 6
    assert families.count("fire") == 6
    assert families.count("firewater") == 5
    assert families.count("overpressure") == 7
    assert families.count("rupture") == 4
    assert families.count("release") == 2
    assert families.count("risk") == 8
    assert families.count("qra") == 1


def test_the_native_scenarios_are_five_and_the_bow_tie_is_its_own_view() -> None:
    module: Any = tool()
    by_id = {spec["id"]: spec for _, spec in module.documents()}
    native = sorted(module.NATIVE_SCENARIOS)
    assert native == [
        "process.bleve",
        "process.jet_fire",
        "process.pfp_demand",
        "process.pool_fire",
        "process.vce",
    ]
    for wanted in native:
        assert module.emit_form(by_id[wanted]).startswith("Some(&CalcForm {")
    assert module.sub_interface_of(by_id[module.BOWTIE], "risk") == "bowtie"
    assert module.sub_interface_of(by_id["process.jet_fire"], "fire") == "fire"
    assert module.sub_interface_of(by_id["process.gas_dispersion"], "dispersion") is None


def test_every_native_id_says_what_it_is_for() -> None:
    module: Any = tool()
    for _, spec in module.documents():
        purpose = module.purpose_of(spec)
        if module.native_of(spec):
            assert purpose != "", spec["id"]
        assert "neqsim" not in purpose.lower(), spec["id"]
        assert len(purpose) <= 300, spec["id"]


def test_a_native_id_without_a_purpose_is_refused() -> None:
    module: Any = tool()
    spec = next(spec for _, spec in module.documents() if spec["id"] == "process.jet_fire")
    saved = module.PURPOSES.pop("process.jet_fire")
    try:
        with pytest.raises(SystemExit):
            module.purpose_of(spec)
    finally:
        module.PURPOSES["process.jet_fire"] = saved


def test_the_two_relief_ids_are_named_outside_the_safety_source() -> None:
    module: Any = tool()
    by_id = {spec["id"]: spec for _, spec in module.documents()}
    for wanted in ("process.psv_sizing", "process.psv_fire_sizing"):
        spec = by_id[wanted]
        assert not str(spec["source"]["standard"]).startswith(module.SAFETY)
        assert module.family_of(spec) == "overpressure"


def test_the_catalogue_never_names_the_upstream_implementation() -> None:
    module: Any = tool()
    # The module header says where the entries come from; the entries themselves are what a
    # browser can show, so that is what is held to the sanitising rule.
    body = module.render().partition("pub static CALCULATIONS")[2]
    assert "neqsim" not in body.lower()
