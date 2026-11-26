"""The intake review and the dossier run through the bridge CLI."""

from __future__ import annotations

import io
import json
import tomllib
from pathlib import Path
from typing import Any

import pytest

from azoth.harness.cli import main

ROOT = Path(__file__).resolve().parents[3]
EXAMPLE = ROOT / "agents" / "intake" / "examples" / "gas-analysis.toml"
CONFLICT = ROOT / "agents" / "intake" / "examples" / "gas-analysis-conflict.toml"


def _run(capsys: pytest.CaptureFixture[str], *argv: str) -> tuple[int, dict[str, Any]]:
    code = main(list(argv))
    document = json.loads(capsys.readouterr().out)
    return code, document


def _document(path: Path = EXAMPLE) -> dict[str, Any]:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def test_review_is_a_successful_call_even_when_refused(
    capsys: pytest.CaptureFixture[str],
) -> None:
    code, document = _run(capsys, "review", "--dossier-json", json.dumps(_document(CONFLICT)))
    assert code == 0
    assert document["kind"] == "review"
    assert document["verdict"] == "refused"
    assert document["register"][0]["class"] == "hard"


def test_review_filters_one_target(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(
        capsys,
        "review",
        "--dossier-json",
        json.dumps(_document()),
        "--id",
        "eos.pt_flash",
    )
    assert code == 0
    assert document["id"] == "eos.pt_flash"
    assert len(document["targets"]) == 1


def test_run_with_a_dossier_carries_the_intake_block(
    capsys: pytest.CaptureFixture[str],
) -> None:
    code, document = _run(capsys, "run", "eos.pt_flash", "--dossier-json", json.dumps(_document()))
    assert code == 0
    assert document["kind"] == "result"
    assert document["result"]["phase"] == "two_phase"
    assert document["intake"]["verdict"] == "passed_with_flags"
    assert "intake" not in document["result"]


def test_run_strict_refuses_a_refused_dossier(
    capsys: pytest.CaptureFixture[str],
) -> None:
    code, document = _run(
        capsys,
        "run",
        "eos.pt_flash",
        "--dossier-json",
        json.dumps(_document(CONFLICT)),
        "--require-dossier",
    )
    assert code == 3
    assert document["error"]["type"] == "IntakeRefused"


def test_a_toml_dossier_is_read_from_the_workspace(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "dossier.toml").write_bytes(EXAMPLE.read_bytes())
    monkeypatch.chdir(tmp_path)
    code, document = _run(capsys, "run", "eos.pt_flash", "--dossier-file", "dossier.toml")
    assert code == 0
    assert document["intake"]["verdict"] == "passed_with_flags"


def test_a_dossier_outside_the_workspace_is_refused(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    outside = tmp_path / "dossier.toml"
    outside.write_bytes(EXAMPLE.read_bytes())
    code, document = _run(capsys, "run", "eos.pt_flash", "--dossier-file", str(outside))
    assert code == 3
    assert document["error"]["type"] == "DossierInvalid"


def test_a_malformed_dossier_is_refused(capsys: pytest.CaptureFixture[str]) -> None:
    code, document = _run(capsys, "review", "--dossier-json", json.dumps({"id": "x"}))
    assert code == 3
    assert document["error"]["type"] == "DossierInvalid"


def test_review_reads_stdin(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    payload = json.dumps(_document()).encode("utf-8")

    class FakeStdin:
        buffer = io.BytesIO(payload)

    monkeypatch.setattr("sys.stdin", FakeStdin())
    code = main(["review", "--dossier-json", "-"])
    document = json.loads(capsys.readouterr().out)
    assert code == 0
    assert document["verdict"] == "passed_with_flags"
