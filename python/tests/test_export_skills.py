"""The skill exporter writes one frontmatter file per catalog entry, per target."""

import subprocess
import sys
import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
TOOL = ROOT / "tools" / "export_skills.py"
CATALOG = ROOT / "skills.toml"

sys.path.insert(0, str(ROOT / "tools"))
from export_skills import expected, render_extra  # type: ignore[import-not-found]  # noqa: E402


def _names() -> list[str]:
    document = tomllib.loads(CATALOG.read_text(encoding="utf-8"))
    return [str(entry["name"]) for entry in document.get("skill", [])]


def test_export_writes_one_skill_per_entry_per_target(tmp_path: Path) -> None:
    result = subprocess.run(
        [sys.executable, str(TOOL), "--target", "all", "--root", str(tmp_path)],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    names = _names()
    dsh = sorted(p.name for p in (tmp_path / ".dsh" / "skills").glob("*.md"))
    assert dsh == sorted(f"{n}.md" for n in names)
    for tool in ("claude", "opencode"):
        written = sorted(
            p.parent.name for p in (tmp_path / f".{tool}" / "skills").glob("*/SKILL.md")
        )
        assert written == sorted(names)


def test_frontmatter_names_and_describes_the_skill(tmp_path: Path) -> None:
    subprocess.run(
        [sys.executable, str(TOOL), "--target", "claude", "--root", str(tmp_path)],
        capture_output=True,
        check=True,
    )
    document = tomllib.loads(CATALOG.read_text(encoding="utf-8"))
    entry = document["skill"][0]
    path = tmp_path / ".claude" / "skills" / entry["name"] / "SKILL.md"
    text = path.read_text(encoding="utf-8")
    assert text.startswith("---\n")
    assert f"name: {entry['name']}\n" in text
    assert "description: " in text


def test_expected_renders_a_named_subset_without_writing(tmp_path: Path) -> None:
    files = expected(ROOT, ["dsh"], names=["azoth-run-calculation"])
    assert set(files) == {ROOT / ".dsh" / "skills" / "azoth-run-calculation.md"}
    text = files[next(iter(files))]
    assert text.startswith("---\nname: azoth-run-calculation\n")
    assert "USE WHEN:" in text
    assert not (tmp_path / "unused").exists()


def test_expected_refuses_a_name_not_in_the_catalog() -> None:
    with pytest.raises(KeyError, match="not in the catalog"):
        expected(ROOT, ["dsh"], names=["azoth-not-a-skill"])


def test_render_extra_carries_the_harness_invocation_fields() -> None:
    text = render_extra("azoth-hazop", "Run a HAZOP. USE WHEN: a study starts.", "# Procedure\n")
    assert text.startswith("---\nname: azoth-hazop\n")
    assert "user-invocable: true\n" in text
    assert text.endswith("# Procedure\n")
