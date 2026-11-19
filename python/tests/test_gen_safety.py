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
    assert len(ids) == 82
    families = [module.family_of(spec) for spec in specs]
    assert families.count("depressurization") == 8
    assert families.count("dispersion") == 6
    assert families.count("equipmentsafety") == 6
    assert families.count("fire") == 6
    assert families.count("firewater") == 5
    assert families.count("overpressure") == 12
    assert families.count("rupture") == 4
    assert families.count("release") == 5
    assert families.count("risk") == 8
    assert families.count("qra") == 1
    assert families.count("selfheating") == 4
    assert families.count("hazid") == 4
    assert families.count("api14c") == 1
    assert families.count("barrier") == 1
    assert families.count("processsafetysystem") == 2
    assert families.count("compliance") == 2
    assert families.count("safetyassessment") == 3
    assert families.count("vibration") == 4


def test_the_native_scenarios_are_the_scalar_safety_models() -> None:
    module: Any = tool()
    by_id = {spec["id"]: spec for _, spec in module.documents()}
    native = sorted(module.NATIVE_SCENARIOS)
    assert native == [
        "process.acoustic_induced_vibration_screening",
        "process.active_fire_protection_screening",
        "process.bleve",
        "process.deluge_nozzle_layout",
        "process.fire_case_relief",
        "process.fire_monitor_coverage",
        "process.fire_water_coverage_assessment",
        "process.gas_blowby",
        "process.gaussian_plume",
        "process.hazardous_area",
        "process.heavy_gas_dispersion",
        "process.inherent_safety",
        "process.jet_fire",
        "process.mdmt",
        "process.nog070_sil_determination",
        "process.overpressure_acceptance",
        "process.pfp_demand",
        "process.pipeline_pressure_protection",
        "process.piping_fiv_gas",
        "process.piping_fiv_liquid",
        "process.pool_fire",
        "process.probit",
        "process.psv_fire_sizing",
        "process.psv_sizing",
        "process.psv_valve",
        "process.pump_deadhead",
        "process.runaway_reaction",
        "process.s001_secondary_pressure_protection",
        "process.safety_instrumented_function",
        "process.toxic_library",
        "process.vce",
    ]
    for wanted in native:
        assert module.emit_form(by_id[wanted]).startswith("Some(&CalcForm {")
    assert module.sub_interface_of(by_id[module.BOWTIE], "risk") == "bowtie"
    assert module.sub_interface_of(by_id["process.jet_fire"], "fire") == "fire"
    assert module.sub_interface_of(by_id["process.gas_dispersion"], "dispersion") == "dispersion"
    assert (
        module.sub_interface_of(by_id["process.gas_blowby"], "equipmentsafety") == "equipmentsafety"
    )
    assert module.sub_interface_of(by_id["process.fire_water_demand"], "firewater") == "firewater"
    assert module.sub_interface_of(by_id["process.psv_sizing"], "overpressure") == "overpressure"
    assert module.sub_interface_of(by_id["process.safety_instrumented_function"], "risk") == "risk"
    assert (
        module.sub_interface_of(
            by_id["process.s001_secondary_pressure_protection"], "processsafetysystem"
        )
        == "processsafetysystem"
    )
    assert module.sub_interface_of(by_id["process.hem_release"], "release") is None
    assert (
        module.sub_interface_of(by_id["process.psv_valve"], "depressurization")
        == "depressurization"
    )
    assert module.sub_interface_of(by_id["process.piping_fiv_gas"], "vibration") == "vibration"
    assert (
        module.sub_interface_of(by_id["process.flow_induced_pulsation_screening"], "vibration")
        == "vibration"
    )
    assert (
        module.sub_interface_of(by_id["process.inherent_safety"], "safetyassessment")
        == "safetyassessment"
    )
    assert module.sub_interface_of(by_id["process.norsok_p002_compliance"], "compliance") is None


def test_every_non_native_entry_has_a_refusal_and_no_form() -> None:
    module: Any = tool()
    entries = module.render()
    assert entries.count("native: false") == 50
    assert entries.count("refusal: Some(") == 50
    for _, spec in module.documents():
        if module.native_of(spec):
            assert module.refusal_of(spec) is None, spec["id"]
        else:
            assert module.refusal_of(spec) is not None, spec["id"]


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
