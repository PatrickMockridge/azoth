"""The JSON CLI the DSH plugin calls."""

from __future__ import annotations

import io
import json
import sys
from pathlib import Path
from typing import Any

import pytest

from azoth._models_gen import MODELS
from azoth.harness.cli import main


def _run(capsys: pytest.CaptureFixture[str], *argv: str) -> tuple[int, dict[str, Any]]:
    code = main(list(argv))
    document = json.loads(capsys.readouterr().out)
    return code, document


def test_version_reports_the_registry_and_backend(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(capsys, "version")
    assert code == 0
    assert document["kind"] == "version"
    assert document["calcs"] == 98
    assert document["models"] == len(MODELS)
    assert document["backend"] in ("python", "rust")


def test_catalogue_filters_and_pages(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(capsys, "catalogue", "--query", "darcy", "--limit", "1")
    assert code == 0
    assert document["total"] == 1
    assert document["items"][0]["id"] == "hydraulics.darcy_weisbach"


def test_catalogue_finds_an_output_name(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(capsys, "catalogue", "--query", "cricondenbar")
    assert code == 0
    assert document["total"] == 1
    item = document["items"][0]
    assert item["id"] == "eos.pt_phase_envelope"
    assert "cricondenbar_pressure" in item["outputs"]
    assert item["matched"] == ["cricondenbar"]


def test_catalogue_ranks_a_multi_term_query(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(capsys, "catalogue", "--query", "phase envelope cricondenbar dew point")
    assert code == 0
    assert document["total"] > 1
    assert document["items"][0]["id"] == "eos.pt_phase_envelope"
    assert document["items"][0]["matched"] == ["phase", "envelope", "cricondenbar", "dew"]


def test_catalogue_searches_an_input_name(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(capsys, "catalogue", "--query", "gas_flow")
    assert code == 0
    item = next(i for i in document["items"] if i["id"] == "eos.beggs_brill_segment")
    assert "gas_flow" in item["inputs"]
    assert item["matched"] == ["gas_flow"]


def test_a_calc_runs_and_carries_provenance(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(
        capsys,
        "run",
        "hydraulics.reynolds_number",
        "--inputs-json",
        json.dumps({"rho": 998.0, "v": 1.5, "D": 0.1, "mu": 1.002e-3}),
    )
    assert code == 0
    assert document["kind"] == "result"
    assert document["result"]["re"] == pytest.approx(149401.1976, rel=1e-9)
    assert document["result"]["provenance"]["calc_id"] == "hydraulics.reynolds_number"
    assert document["result"]["provenance"]["verification"] == "partially_verified"


def test_a_model_builds_its_mixture(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(
        capsys,
        "run",
        "eos.pt_flash",
        "--inputs-json",
        json.dumps(
            {
                "components": ["methane", "n-butane"],
                "T": {"value": 250.0, "unit": "K"},
                "P": {"value": 20.0, "unit": "bar"},
                "z": [0.9, 0.1],
            }
        ),
    )
    assert code == 0
    assert document["result"]["phase"] == "two_phase"
    assert document["result"]["provenance"]["calc_id"] == "eos.pt_flash"


def test_inputs_may_come_on_stdin(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    payload = json.dumps({"rho": 998.0, "v": 1.5, "D": 0.1, "mu": 1.002e-3}).encode()

    class FakeStdin:
        buffer = io.BytesIO(payload)

    monkeypatch.setattr(sys, "stdin", FakeStdin())
    code, document = _run(capsys, "run", "hydraulics.reynolds_number", "--inputs-json", "-")
    assert code == 0
    assert document["kind"] == "result"


def test_the_backend_can_be_named(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(
        capsys,
        "run",
        "hydraulics.reynolds_number",
        "--inputs-json",
        json.dumps({"rho": 998.0, "v": 1.5, "D": 0.1, "mu": 1.002e-3}),
        "--backend",
        "python",
    )
    assert code == 0
    assert document["backend"] == "python"


def test_an_unknown_id_is_a_named_refusal(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(capsys, "spec", "eos.not_a_calc")
    assert code == 2
    assert document["error"]["type"] == "UnknownId"


def test_a_bad_unit_is_an_input_refusal(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(
        capsys,
        "run",
        "eos.pt_flash",
        "--inputs-json",
        json.dumps(
            {
                "components": ["methane"],
                "T": 250.0,
                "P": {"value": 1, "unit": "kg"},
                "z": [1.0],
            }
        ),
    )
    assert code == 3
    assert document["error"]["field"] == "P"


def test_a_keycard_outside_the_working_directory_is_refused(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    keycard = tmp_path / "card.toml"
    keycard.write_text("schema_version = 1\n", encoding="utf-8")
    code, document = _run(
        capsys,
        "run",
        "eos.pt_flash",
        "--inputs-json",
        json.dumps({"components": ["methane"], "T": 250.0, "P": 2.0e6, "z": [1.0]}),
        "--keycard",
        str(keycard),
    )
    assert code == 3
    assert document["error"]["field"] == "keycard"
