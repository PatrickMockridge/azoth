"""The two worked bases: one runs with declared gaps, one refuses."""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any

import pytest

from azoth.harness.api import review as harness_review
from azoth.harness.api import run
from azoth.harness.errors import IntakeRefused, UndefinedInput
from azoth.intake import load_path, review

ROOT = Path(__file__).resolve().parents[3]
EXAMPLES = ROOT / "agents" / "intake" / "examples"


def _document(name: str) -> dict[str, Any]:
    return tomllib.loads((EXAMPLES / name).read_text(encoding="utf-8"))


def test_the_flagged_basis_runs_and_carries_its_requests() -> None:
    checked = review(load_path(EXAMPLES / "gas-analysis.toml"))
    assert checked.verdict == "passed_with_flags"
    assert [request.input for request in checked.requests] == ["z", "P"]
    assert "n-heptane" in checked.requests[0].ask
    assert "laboratory pressure report" in checked.requests[1].ask

    envelope = run("eos.pt_flash", dossier=_document("gas-analysis.toml"))
    assert envelope["result"]["phase"] == "two_phase"
    assert envelope["result"]["warnings"] == []
    assert envelope["intake"]["verdict"] == "passed_with_flags"
    assert envelope["intake"]["requests"][0]["input"] == "z"


def test_the_conflicting_basis_refuses_and_cannot_be_called() -> None:
    checked = review(load_path(EXAMPLES / "gas-analysis-conflict.toml"))
    assert checked.verdict == "refused"
    assert checked.targets[0].verdict == "refused"
    assert checked.register[0].code == "CONFLICT_UNRESOLVED"
    assert checked.register[0].class_ == "hard"
    assert checked.requests[0].blocking is True
    assert "P" not in checked.targets[0].inputs

    with pytest.raises(UndefinedInput, match="required inputs absent"):
        run("eos.pt_flash", dossier=_document("gas-analysis-conflict.toml"))
    with pytest.raises(IntakeRefused):
        run(
            "eos.pt_flash",
            dossier=_document("gas-analysis-conflict.toml"),
            require_dossier=True,
        )


def test_the_review_envelope_echoes_the_dossier() -> None:
    document = harness_review(_document("gas-analysis.toml"))
    assert document["kind"] == "review"
    assert document["dossier"]["id"] == "gas-analysis"
    assert document["targets"][0]["id"] == "eos.pt_flash"
    assert document["dossier_sha256"]
