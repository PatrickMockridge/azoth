"""The deterministic gate: hard versus soft, and the run-path block."""

from __future__ import annotations

import copy
import tomllib
from pathlib import Path
from typing import Any

import pytest

from azoth.harness.api import review as harness_review
from azoth.harness.api import run
from azoth.harness.errors import (
    DossierRequired,
    InputsMismatch,
    IntakeRefused,
    UndefinedInput,
)
from azoth.intake import Review, from_document, review

ROOT = Path(__file__).resolve().parents[3]
EXAMPLES = ROOT / "agents" / "intake" / "examples"


def _document(name: str = "gas-analysis.toml") -> dict[str, Any]:
    return tomllib.loads((EXAMPLES / name).read_text(encoding="utf-8"))


def _codes(checked: Review) -> list[tuple[str, str, str]]:
    return [(gap.input, gap.class_, gap.code) for gap in checked.register]


def test_the_flagged_example_passes_with_two_declared_gaps() -> None:
    checked = review(from_document(_document()))
    assert checked.verdict == "passed_with_flags"
    assert _codes(checked) == [
        ("z", "soft", "INFERRED_COMPOSITION_REMAINDER"),
        ("P", "soft", "LOWER_RUNG_USED"),
    ]
    assert checked.targets[0].inputs["P"] == pytest.approx(5_101_325.0)
    assert checked.targets[0].inputs["z"][-1] == pytest.approx(0.01)


def test_the_witness_pair_differs_in_one_claim() -> None:
    passing = _document()
    refusing = copy.deepcopy(passing)
    # The same numbers, but the source no longer names the component it is the balance of.
    refusing["claims"][1]["quote"] = "balance not separately analysed"
    refusing["sources"][0]["excerpt"] = refusing["sources"][0]["excerpt"].replace(
        "balance n-heptane (C7), not separately analysed",
        "balance not separately analysed",
    )
    assert review(from_document(passing)).verdict == "passed_with_flags"
    checked = review(from_document(refusing))
    assert checked.verdict == "refused"
    assert ("z", "hard", "RULE_PRECONDITION") in _codes(checked)


def test_a_conflicting_pressure_refuses_the_target() -> None:
    checked = review(from_document(_document("gas-analysis-conflict.toml")))
    assert checked.verdict == "refused"
    assert checked.targets[0].verdict == "refused"
    assert ("P", "hard", "CONFLICT_UNRESOLVED") in _codes(checked)
    assert "P" not in checked.targets[0].inputs
    assert any(request.blocking and request.input == "P" for request in checked.requests)


def test_a_quote_that_does_not_contain_the_value_is_a_hard_gap() -> None:
    document = _document()
    document["claims"][4]["value"] = 301.0
    checked = review(from_document(document))
    assert checked.verdict == "refused"
    assert ("T", "hard", "QUOTE_DOES_NOT_SUPPORT_VALUE") in _codes(checked)


def test_a_bare_bar_is_ambiguous() -> None:
    document = _document()
    document["claims"][3]["unit"] = "bar"
    checked = review(from_document(document))
    assert checked.verdict == "refused"
    assert ("P", "hard", "UNIT_AMBIGUOUS") in _codes(checked)


def test_a_required_input_absent_is_refused() -> None:
    document = _document()
    document["targets"][0]["inputs"][1] = {"name": "P", "absent": True}
    checked = review(from_document(document))
    assert checked.verdict == "refused"
    assert ("P", "hard", "REQUIRED_INPUT_ABSENT") in _codes(checked)


def test_an_unlicensed_rule_is_refused() -> None:
    document = _document()
    document["targets"][0]["inputs"][3]["rule"] = "typical_from_standard"
    checked = review(from_document(document))
    assert checked.verdict == "refused"
    assert ("z", "hard", "RULE_UNLICENSED") in _codes(checked)


def test_a_derived_input_is_run_not_typed_in() -> None:
    document = {
        "id": "derived",
        "sources": [
            {
                "id": "S-1",
                "kind": "prompt",
                "locator": "user message",
                "excerpt": "rho 998 v 1.5 D 0.1 mu 0.001002 roughness 0.0001",
            }
        ],
        "claims": [
            {"id": "C-1", "value": 998.0, "unit": "kg/m**3", "quote": "rho 998", "source": "S-1"},
            {"id": "C-2", "value": 1.5, "unit": "m/s", "quote": "v 1.5", "source": "S-1"},
            {"id": "C-3", "value": 0.1, "unit": "m", "quote": "D 0.1", "source": "S-1"},
            {
                "id": "C-4",
                "value": 0.001002,
                "unit": "Pa*s",
                "quote": "mu 0.001002",
                "source": "S-1",
            },
            {
                "id": "C-5",
                "value": 0.0001,
                "unit": "dimensionless",
                "quote": "roughness 0.0001",
                "source": "S-1",
            },
        ],
        "targets": [
            {
                "id": "hydraulics.friction_factor_colebrook",
                "inputs": [
                    {
                        "name": "re",
                        "rule": "derived_by_id",
                        "basis": {
                            "id": "hydraulics.reynolds_number",
                            "output": "re",
                            "inputs": {
                                "rho": {"claim": "C-1"},
                                "v": {"claim": "C-2"},
                                "D": {"claim": "C-3"},
                                "mu": {"claim": "C-4"},
                            },
                        },
                    },
                    {"name": "relative_roughness", "claim": "C-5"},
                ],
            }
        ],
    }
    checked = review(from_document(document))
    assert checked.verdict == "passed_with_flags"
    assert checked.targets[0].inputs["re"] == pytest.approx(149_401.1976, rel=1e-6)
    inferred = next(origin for origin in checked.targets[0].origins if origin.input == "re")
    assert inferred.rule == "derived_by_id"
    assert inferred.provenance is not None
    assert inferred.provenance["calc_id"] == "hydraulics.reynolds_number"


def test_the_run_envelope_carries_a_sibling_block_and_changes_no_result_field() -> None:
    document = _document()
    constructed = review(from_document(document)).targets[0].inputs
    plain = run("eos.pt_flash", constructed)
    reviewed = run("eos.pt_flash", dossier=document)
    assert set(reviewed["result"]) == set(plain["result"])
    assert "intake" not in reviewed["result"]
    assert reviewed["intake"]["verdict"] == "passed_with_flags"
    assert reviewed["intake"]["enforcement"] == "advisory"
    assert reviewed["intake"]["target"] == "eos.pt_flash"
    assert len(reviewed["intake"]["dossier_sha256"]) == 64


def test_the_advisory_mode_runs_a_refused_target_and_says_so() -> None:
    document = _document()
    document["claims"][4]["value"] = 301.0  # the quote still says 300 K
    envelope = run("eos.pt_flash", dossier=document)
    assert envelope["intake"]["verdict"] == "refused"
    assert envelope["intake"]["enforcement"] == "advisory"
    assert envelope["result"]["provenance"]["calc_id"] == "eos.pt_flash"


def test_the_strict_mode_stops_a_refused_target_before_the_kernel() -> None:
    document = _document("gas-analysis-conflict.toml")
    with pytest.raises(IntakeRefused, match="CONFLICT_UNRESOLVED"):
        run("eos.pt_flash", dossier=document, require_dossier=True)


def test_an_absent_required_value_cannot_be_run() -> None:
    document = _document("gas-analysis-conflict.toml")
    with pytest.raises(UndefinedInput, match="required inputs absent"):
        run("eos.pt_flash", dossier=document)


def test_the_strict_mode_requires_a_dossier() -> None:
    with pytest.raises(DossierRequired):
        run("eos.pt_flash", {}, require_dossier=True)


def test_a_dossier_replaces_inputs() -> None:
    with pytest.raises(InputsMismatch):
        run("eos.pt_flash", {}, dossier=_document())


def test_the_harness_review_names_the_verdict() -> None:
    document = harness_review(_document())
    assert document["kind"] == "review"
    assert document["verdict"] == "passed_with_flags"
    assert document["register"][0]["class"] == "soft"
    assert document["requests"][0]["ask"]
