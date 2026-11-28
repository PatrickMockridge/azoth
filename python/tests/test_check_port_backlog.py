"""Tests for ``tools/check_port_backlog.py`` and ``tools/gen_port_checklist.py``.

The checker is what makes the inventory full: every physics class is in a row or a spec,
every row's size is the metric's, and a covered row is stale. Each refusal is pinned on a
synthetic tree.
"""

from __future__ import annotations

import importlib
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))

check = importlib.import_module("check_port_backlog")
checklist = importlib.import_module("gen_port_checklist")

CLASSES = """\
[[pin]]
short = "abc"
commit = "abc1234"
classes = ["Alpha", "Beta"]
"""

SCOPE = """\
commit = "abc1234"

[[class]]
name = "Alpha"
package = "neqsim.thermo"
kind = "class"
loc = 100
refs = ["Beta"]

[[class]]
name = "Beta"
package = "neqsim.thermo"
kind = "class"
loc = 200
refs = []
"""


def row(
    row_id: str,
    tier: int,
    state: str,
    classes: list[str],
    loc: int = 100,
    deps: int = 1,
    size: str = "S",
    blocked: str = "",
) -> str:
    lines = [
        "[[package]]",
        f'id = "{row_id}"',
        f"tier = {tier}",
        'title = "X"',
        f'state = "{state}"',
        f"loc = {loc}",
        f"deps = {deps}",
        'capture = ""',
        "new_solver = false",
        f'size = "{size}"',
    ]
    if blocked:
        lines.append(f'blocked_by = "{blocked}"')
    lines.append("classes = [")
    lines.extend(f'    "{name}",' for name in classes)
    lines.append("]")
    return "\n".join(lines)


def tree(tmp_path: Path, backlog: str) -> Path:
    (tmp_path / "data" / "neqsim").mkdir(parents=True)
    (tmp_path / "databank").mkdir()
    (tmp_path / "specs").mkdir()
    (tmp_path / "data" / "neqsim" / "classes.toml").write_text(CLASSES, encoding="utf-8")
    (tmp_path / "data" / "neqsim" / "port_scope.toml").write_text(SCOPE, encoding="utf-8")
    (tmp_path / "databank" / "port_backlog.toml").write_text(backlog, encoding="utf-8")
    (tmp_path / "databank" / "manifest.toml").write_text(
        '[[upstreams]]\n[[upstreams.files]]\nid = "file"\n'
        '[[upstreams.files.columns]]\nname = "X"\ndisposition = "used"\nreason = "used: x"\n',
        encoding="utf-8",
    )
    return tmp_path


def baseline() -> str:
    return "\n\n".join(
        [
            row("tier1.alpha", 1, "port", ["Alpha"]),
            row("tier0.beta", 0, "internal", ["Beta"], loc=200, deps=0),
        ]
    )


def test_a_complete_inventory_passes(tmp_path: Path) -> None:
    assert check.check(tree(tmp_path, baseline())) == []


def test_an_unclassified_row_is_refused(tmp_path: Path) -> None:
    backlog = baseline().replace("tier = 1", "tier = -1", 1)
    problems = check.check(tree(tmp_path, backlog))
    assert any("unclassified" in problem for problem in problems)


def test_a_class_in_no_row_and_no_spec_is_refused(tmp_path: Path) -> None:
    backlog = row("tier1.alpha", 1, "port", ["Alpha"])
    problems = check.check(tree(tmp_path, backlog))
    assert any("Beta" in problem and "no row" in problem for problem in problems)


def test_a_class_not_at_the_pin_is_refused(tmp_path: Path) -> None:
    backlog = baseline().replace('"Alpha",', '"Gamma",', 1)
    problems = check.check(tree(tmp_path, backlog))
    assert any("Gamma" in problem and "pin" in problem for problem in problems)


def test_a_class_in_two_rows_is_refused(tmp_path: Path) -> None:
    backlog = baseline().replace('"Beta",', '"Beta", "Alpha",', 1)
    problems = check.check(tree(tmp_path, backlog))
    assert any("already owned" in problem for problem in problems)


def test_a_fully_covered_row_is_stale(tmp_path: Path) -> None:
    root = tree(tmp_path, baseline())
    (root / "specs" / "alpha.toml").write_text(
        '[source.neqsim]\nclass = "Alpha"\nmembers = ["Alpha.run"]\ncommit = "abc1234"\n',
        encoding="utf-8",
    )
    problems = check.check(root)
    assert any("stale" in problem for problem in problems)


def test_a_size_that_does_not_match_the_metric_is_refused(tmp_path: Path) -> None:
    backlog = baseline().replace('size = "S"', 'size = "L"', 1)
    problems = check.check(tree(tmp_path, backlog))
    assert any("size" in problem for problem in problems)


def test_an_unreachable_row_needs_a_reason(tmp_path: Path) -> None:
    backlog = row("tier1.alpha", 1, "unreachable", ["Alpha"])
    problems = check.check(tree(tmp_path, backlog))
    assert any("blocked_by" in problem for problem in problems)


def test_the_checklist_renders_the_open_row(tmp_path: Path) -> None:
    rendered = checklist.render(tree(tmp_path, baseline()))
    assert "- [ ] [S] X — `Alpha`" in rendered
    assert "- Beta" not in rendered
    assert "internal — 1 classes" in rendered
