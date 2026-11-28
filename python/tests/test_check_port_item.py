"""Tests for ``tools/check_port_item.py``: a new spec class stays inside its item.

Each case builds a two-commit repository: the base spec names one class, the second
commit adds another. The backlog row and the commit trailer decide whether it passes.
"""

from __future__ import annotations

import importlib
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))

check = importlib.import_module("check_port_item")

BASE_SPEC = """\
id = "eos.example"

[source.neqsim]
class = "Alpha"
members = ["Alpha.run"]
commit = "abc1234"
"""

NEXT_SPEC = """\
id = "eos.example"

[source.neqsim]
class = "Beta"
members = ["Beta.run"]
commit = "abc1234"
"""

BACKLOG = """\
[[package]]
id = "tier1.beta"
tier = 1
title = "X"
state = "port"
loc = 100
deps = 1
capture = ""
new_solver = false
size = "S"
classes = ["%s"]
"""


def git(root: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(root), *arguments], capture_output=True, text=True, check=True
    )


def repository(tmp_path: Path, classes: str) -> Path:
    root = tmp_path / "repo"
    (root / "specs" / "calcs" / "eos").mkdir(parents=True)
    (root / "databank").mkdir()
    (root / "databank" / "port_backlog.toml").write_text(BACKLOG % classes, encoding="utf-8")
    (root / "specs" / "calcs" / "eos" / "example.toml").write_text(BASE_SPEC, encoding="utf-8")
    git(root, "init")
    git(root, "config", "user.email", "test@example.com")
    git(root, "config", "user.name", "Test")
    git(root, "add", "-A")
    git(root, "commit", "-m", "base")
    (root / "specs" / "calcs" / "eos" / "example.toml").write_text(NEXT_SPEC, encoding="utf-8")
    return root


def commit(root: Path, message: str) -> None:
    git(root, "add", "-A")
    git(root, "commit", "-m", message)


def test_a_new_class_with_its_trailer_passes(tmp_path: Path) -> None:
    root = repository(tmp_path, "Beta")
    commit(root, "add beta\n\nPort-Item: tier1.beta")
    assert check.check(root, "HEAD^..HEAD") == []


def test_a_new_class_with_no_trailer_is_refused(tmp_path: Path) -> None:
    root = repository(tmp_path, "Beta")
    commit(root, "add beta")
    problems = check.check(root, "HEAD^..HEAD")
    assert any("no Port-Item" in problem for problem in problems)


def test_a_new_class_outside_the_item_is_refused(tmp_path: Path) -> None:
    root = repository(tmp_path, "Gamma")
    commit(root, "add beta\n\nPort-Item: tier1.beta")
    problems = check.check(root, "HEAD^..HEAD")
    assert any("outside the Port-Item scope" in problem for problem in problems)


def test_a_trailer_naming_no_row_is_refused(tmp_path: Path) -> None:
    root = repository(tmp_path, "Beta")
    commit(root, "add beta\n\nPort-Item: tier9.missing")
    problems = check.check(root, "HEAD^..HEAD")
    assert any("names no backlog row" in problem for problem in problems)


def test_a_change_with_no_new_class_passes(tmp_path: Path) -> None:
    root = repository(tmp_path, "Beta")
    path = root / "specs" / "calcs" / "eos" / "example.toml"
    path.write_text(BASE_SPEC + "\n# a correction\n", encoding="utf-8")
    commit(root, "comment only")
    assert check.check(root, "HEAD^..HEAD") == []


def test_a_row_removed_by_the_same_change_still_scopes_the_trailer(tmp_path: Path) -> None:
    """A completed row is deleted; the trailer must still resolve against the base."""
    root = repository(tmp_path, "Beta")
    path = root / "specs" / "calcs" / "eos" / "example.toml"
    path.write_text(NEXT_SPEC, encoding="utf-8")
    (root / "databank" / "port_backlog.toml").write_text("", encoding="utf-8")
    commit(root, "add beta and remove the row\n\nPort-Item: tier1.beta")
    assert check.check(root, "HEAD^..HEAD") == []


def test_one_class_may_be_named_by_two_new_specs(tmp_path: Path) -> None:
    """A class with two calculations is one owner and several ids, both new here."""
    root = repository(tmp_path, "Beta")
    path = root / "specs" / "calcs" / "eos" / "example.toml"
    path.write_text(NEXT_SPEC, encoding="utf-8")
    second = root / "specs" / "calcs" / "eos" / "second.toml"
    second.write_text(
        NEXT_SPEC.replace('id = "eos.example"', 'id = "eos.second"'), encoding="utf-8"
    )
    commit(root, "add beta in two specs\n\nPort-Item: tier1.beta")
    assert check.check(root, "HEAD^..HEAD") == []


def test_a_class_named_by_a_spec_outside_the_change_is_refused(tmp_path: Path) -> None:
    root = repository(tmp_path, "Beta")
    path = root / "specs" / "calcs" / "eos" / "example.toml"
    path.write_text(NEXT_SPEC, encoding="utf-8")
    commit(root, "add beta\n\nPort-Item: tier1.beta")
    second = root / "specs" / "calcs" / "eos" / "second.toml"
    second.write_text(
        NEXT_SPEC.replace('id = "eos.example"', 'id = "eos.second"'), encoding="utf-8"
    )
    commit(root, "add the second spec\n\nPort-Item: tier1.beta")
    problems = check.check(root, "HEAD^..HEAD")
    assert any("is already named by" in problem for problem in problems)
