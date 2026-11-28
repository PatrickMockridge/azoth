"""Tests for ``tools/check_port.py``.

The checker is the gate that refuses a spec without a NeqSim source. These tests pin each
refusal on a synthetic spec, and one end-to-end tree proves the file plumbing.
"""

from __future__ import annotations

import importlib
import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))

check_port = importlib.import_module("check_port")

INDEX = {"abc1234": {"NeqSimClass", "OtherClass"}}
COMMIT = "abc1234def5678"


def spec() -> dict[str, Any]:
    return {
        "id": "eos.example",
        "source": {
            "standard": "NeqSim `NeqSimClass`",
            "neqsim": {
                "class": "NeqSimClass",
                "members": ["NeqSimClass.run"],
                "commit": COMMIT,
            },
        },
        "inputs": {"T": {}, "P": {}},
        "outputs": {"value": {}},
        "algorithm": {"scheme": "x", "tolerance": 1e-08, "inner": {"max_iterations": 10}},
        "provenance": {
            "inputs": {"default": "caller"},
            "outputs": {"default": "NeqSim NeqSimClass.run"},
            "algorithm": {
                "scheme": {"source": "NeqSim NeqSimClass.run"},
                "tolerance": {"source": "NeqSim NeqSimClass.tolerance"},
                "inner.max_iterations": {"source": "NeqSim NeqSimClass.maxIterations"},
            },
            "constants": [],
        },
    }


def problems(document: dict[str, Any]) -> list[str]:
    result: list[str] = check_port.check_neqsim(document, INDEX) + check_port.check_values(document)
    return result


def test_a_complete_spec_passes() -> None:
    assert problems(spec()) == []


def test_a_spec_without_a_neqsim_source_is_refused() -> None:
    document = spec()
    del document["source"]["neqsim"]
    assert any("source.neqsim" in problem for problem in problems(document))


def test_a_class_absent_from_the_pin_is_refused() -> None:
    document = spec()
    document["source"]["neqsim"]["class"] = "NotAClass"
    assert any("NotAClass" in problem for problem in problems(document))


def test_an_unknown_pin_is_refused() -> None:
    document = spec()
    document["source"]["neqsim"]["commit"] = "ffffffff"
    assert any("not an indexed" in problem for problem in problems(document))


def test_a_spec_without_provenance_is_refused() -> None:
    document = spec()
    del document["provenance"]
    assert "no `[provenance]` table" in problems(document)


def test_an_input_with_no_default_or_override_is_refused() -> None:
    document = spec()
    document["provenance"]["inputs"] = {"default": "", "overrides": {"T": {"source": "caller"}}}
    assert any("no row for ['P']" in problem for problem in problems(document))


def test_an_override_for_an_undeclared_input_is_refused() -> None:
    document = spec()
    document["provenance"]["inputs"]["overrides"] = {"Q": {"source": "caller"}}
    assert any("undeclared ['Q']" in problem for problem in problems(document))


def test_an_algorithm_leaf_with_no_provenance_is_refused() -> None:
    document = spec()
    del document["provenance"]["algorithm"]["tolerance"]
    assert any("tolerance" in problem for problem in problems(document))


def test_an_override_without_a_source_is_refused() -> None:
    document = spec()
    document["provenance"]["inputs"]["overrides"] = {"T": {}}
    assert any("has no source" in problem for problem in problems(document))


def test_a_constant_row_without_a_source_is_refused() -> None:
    document = spec()
    document["provenance"]["constants"] = [{"name": "c", "value": 1.0}]
    assert any("has no 'source'" in problem for problem in problems(document))


def test_the_tree_check_reads_both_trees(tmp_path: Path) -> None:
    (tmp_path / "data" / "neqsim").mkdir(parents=True)
    (tmp_path / "data" / "neqsim" / "classes.toml").write_text(
        f'[[pin]]\nshort = "abc1234"\ncommit = "{COMMIT}"\nclasses = ["NeqSimClass"]\n'
    )
    (tmp_path / "specs" / "models").mkdir(parents=True)
    (tmp_path / "specs" / "models" / "example.toml").write_text(
        """
id = "eos.example"
[source]
standard = "NeqSim `NeqSimClass`"
[source.neqsim]
class = "NeqSimClass"
members = ["NeqSimClass.run"]
commit = "abc1234def5678"
[inputs.T]
[inputs.P]
[outputs.value]
[algorithm]
scheme = "x"
tolerance = 1e-8
[provenance.inputs]
default = "caller"
[provenance.outputs]
default = "NeqSim NeqSimClass.run"
[provenance.algorithm.scheme]
source = "NeqSim NeqSimClass.run"
[provenance.algorithm.tolerance]
source = "NeqSim NeqSimClass.tolerance"
"""
    )
    assert check_port.check(tmp_path) == []
    document = (tmp_path / "specs" / "models" / "example.toml").read_text()
    (tmp_path / "specs" / "models" / "example.toml").write_text(
        document.replace('default = "caller"', 'default = ""')
    )
    assert check_port.check(tmp_path)


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__]))
