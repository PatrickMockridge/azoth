"""The dossier parser and the authoring schema."""

from __future__ import annotations

import copy
import json
import tomllib
from pathlib import Path
from typing import Any

import jsonschema
import pytest

from azoth.intake import DossierInvalid, canonical, from_document, load_path

ROOT = Path(__file__).resolve().parents[3]
EXAMPLES = ROOT / "agents" / "intake" / "examples"
SCHEMA = json.loads((ROOT / "specs" / "schema" / "intake.schema.json").read_text())


def _document(name: str) -> dict[str, Any]:
    return tomllib.loads((EXAMPLES / name).read_text(encoding="utf-8"))


def test_the_examples_validate_against_the_schema() -> None:
    validator = jsonschema.Draft202012Validator(SCHEMA)
    for name in ("gas-analysis.toml", "gas-analysis-conflict.toml"):
        validator.validate(_document(name))


def test_the_parser_reads_the_example() -> None:
    dossier = load_path(EXAMPLES / "gas-analysis.toml")
    assert dossier.id == "gas-analysis"
    assert [source.id for source in dossier.sources] == ["S-1", "S-2"]
    assert dossier.claim("C-1").unit == "mol%"
    assert dossier.targets[0].id == "eos.pt_flash"
    assert {item.name for item in dossier.targets[0].inputs} == {
        "components",
        "P",
        "T",
        "z",
    }


def test_unknown_keys_are_refused() -> None:
    document = _document("gas-analysis.toml")
    document["verdict"] = "passed"
    with pytest.raises(DossierInvalid, match="unknown keys"):
        from_document(document)


def test_a_bad_schema_version_is_refused() -> None:
    document = _document("gas-analysis.toml")
    document["schema"] = "azoth.intake/2"
    with pytest.raises(DossierInvalid, match=r"expected 'azoth\.intake/1'"):
        from_document(document)


def test_duplicate_claim_ids_are_refused() -> None:
    document = _document("gas-analysis.toml")
    document["claims"].append(copy.deepcopy(document["claims"][0]))
    with pytest.raises(DossierInvalid, match="two claims share an id"):
        from_document(document)


def test_an_unknown_source_is_refused() -> None:
    document = _document("gas-analysis.toml")
    document["claims"][0]["source"] = "S-9"
    with pytest.raises(DossierInvalid, match="unknown source"):
        from_document(document)


def test_an_unknown_claim_in_a_binding_is_refused() -> None:
    document = _document("gas-analysis.toml")
    document["targets"][0]["inputs"][0]["claim"] = "C-99"
    with pytest.raises(DossierInvalid, match="unknown claim"):
        from_document(document)


def test_a_binding_must_be_exactly_one_of_the_four_forms() -> None:
    document = _document("gas-analysis.toml")
    document["targets"][0]["inputs"][0] = {
        "name": "components",
        "claim": "C-3",
        "absent": True,
    }
    with pytest.raises(DossierInvalid, match="one of claim, rule, absent or conflicted"):
        from_document(document)


def test_the_canonical_document_is_stable() -> None:
    document = _document("gas-analysis.toml")
    assert canonical(document) == canonical(copy.deepcopy(document))
