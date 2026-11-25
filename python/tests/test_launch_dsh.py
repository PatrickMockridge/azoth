"""The DeepSeek Harness launcher's generated overlay and read-only check.

The launcher is the one place DSH-specific configuration is written, so these tests hold
the pieces that can be checked without DSH: the overlay rows, the skill export and its
manifest, and a check that does not touch the tree. The real boot is `launch_dsh.py
--check` against an installed DSH, which CI does not have.
"""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path
from typing import Any, cast

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))

import launch_dsh  # type: ignore[import-not-found]  # noqa: E402


def _root(tmp_path: Path) -> Path:
    """A checkout-shaped tree with one catalog skill and the harness assets."""
    root = tmp_path / "repo"
    (root / "python" / "src" / "azoth").mkdir(parents=True)
    (root / "Cargo.toml").write_text("", encoding="utf-8")
    (root / "skills.toml").write_text(
        "\n".join(
            [
                "[[skill]]",
                'name = "azoth-test"',
                'version = "0.1.0"',
                'description = "A test skill. USE WHEN: testing."',
                'calculation_basis = "advisory"',
                'path = "skills/library/test/SKILL.md"',
                'tags = ["test"]',
                "",
            ]
        ),
        encoding="utf-8",
    )
    body = root / "skills" / "library" / "test" / "SKILL.md"
    body.parent.mkdir(parents=True)
    body.write_text("# Test\n", encoding="utf-8")
    harness = root / "agents" / "deepseek"
    (harness / "skills").mkdir(parents=True)
    (harness / "prompts").mkdir()
    (harness / "harness.toml").write_text(
        "\n".join(
            [
                "[harness]",
                'tested_dsh = "0.2.2-rc.5"',
                'dsh_version = "0.2.2-rc.5"',
                'tested_node = "22.19"',
                'default_model = "deepseek-v4-flash"',
                'default_provider = "deepseek-official"',
                "max_tasks = 256",
                "",
                "[plugin]",
                "timeout_ms = 120000",
                "max_output_bytes = 4194304",
                "",
                "[mcp]",
                "tool_call_timeout_ms = 60000",
                "startup_timeout_ms = 30000",
                "close_timeout_ms = 5000",
                "",
                "[skills]",
                'hazop = ["azoth-test"]',
                "",
                "[presets.chair]",
                'id = "azoth-hazop-chair"',
                "order = 10",
                'prompt = "prompts/chair.md"',
                "",
                "[presets.process]",
                'id = "azoth-process"',
                "order = 11",
                'prompt = "prompts/process.md"',
                "",
                "[presets.safety]",
                'id = "azoth-safety"',
                "order = 12",
                'prompt = "prompts/safety.md"',
                "",
                "[presets.scribe]",
                'id = "azoth-scribe"',
                "order = 13",
                'prompt = "prompts/scribe.md"',
                "",
            ]
        ),
        encoding="utf-8",
    )
    (harness / "skills" / "azoth-hazop.md").write_bytes(
        (ROOT / "agents" / "deepseek" / "skills" / "azoth-hazop.md").read_bytes()
    )
    for name in ("chair", "process", "safety", "scribe"):
        (harness / "prompts" / f"{name}.md").write_bytes(
            (ROOT / "agents" / "deepseek" / "prompts" / f"{name}.md").read_bytes()
        )
    return root


def _args(root: Path, *extra: str) -> tuple[Any, dict[str, Any]]:
    args = launch_dsh.parser().parse_args([*extra])
    args.root = str(root)
    config = tomllib.loads((root / "agents" / "deepseek" / "harness.toml").read_text())
    if args.model is None:
        args.model = str(config["harness"]["default_model"])
    return args, config


def _toolchain(root: Path) -> launch_dsh.Toolchain:
    return launch_dsh.Toolchain(
        python=Path(sys.executable),
        dsh=launch_dsh.Command(("dsh",)),
        azoth=launch_dsh.Command(("azoth",)),
        node=None,
        root=root,
        dsh_home=root / ".dsh-home",
    )


def _rows(root: Path, *extra: str) -> list[dict[str, Any]]:
    args, config = _args(root, *extra)
    return cast("list[dict[str, Any]]", launch_dsh.build_overlay(args, _toolchain(root), config))


def _find(rows: list[dict[str, Any]], row_id: str) -> dict[str, Any] | None:
    for row in rows:
        if row.get("id") == row_id:
            return row
        for inserted in row.get("insert", []):
            if inserted.get("id") == row_id:
                return cast("dict[str, Any]", inserted)
    return None


def test_general_web_overlay_has_no_team_or_presets(tmp_path: Path) -> None:
    rows = _rows(_root(tmp_path), "--flowsheet", "case.toml")
    assert _find(rows, "mcp-azoth-flowsheet") is not None
    assert _find(rows, "llm-deepseek") is not None
    assert _find(rows, "agent-preset-registry") is None
    assert _find(rows, "preset-azoth-hazop-chair") is None


def test_study_overlay_wires_the_team_and_four_presets(tmp_path: Path) -> None:
    rows = _rows(_root(tmp_path), "--study", "hazop", "--flowsheet", "case.toml")
    team = _find(rows, "agent-team")
    assert team is not None
    assert team["config"]["defaultEnabled"] is True
    registry = _find(rows, "agent-preset-registry")
    assert registry is not None
    assert registry["config"]["default"] == "azoth-hazop-chair"
    chair = _find(rows, "preset-azoth-hazop-chair")
    assert chair is not None
    subagent = next(
        plugin for plugin in chair["config"]["plugins"] if plugin["id"] == "tool-subagent"
    )
    assert subagent["config"]["allowedAgentPresets"] == [
        "azoth-process",
        "azoth-safety",
        "azoth-scribe",
    ]
    for preset in ("azoth-process", "azoth-safety", "azoth-scribe"):
        assert _find(rows, f"preset-{preset}") is not None


def test_headless_study_inserts_the_registry(tmp_path: Path) -> None:
    rows = _rows(_root(tmp_path), "--headless", "--study", "hazop", "--flowsheet", "case.toml")
    inserted = [
        entry
        for row in rows
        for entry in row.get("insert", [])
        if entry.get("id") == "agent-preset-registry"
    ]
    assert inserted, "headless has no shipped registry, so it must be inserted"


def test_a_gateway_replaces_the_hosted_provider(tmp_path: Path) -> None:
    rows = _rows(
        _root(tmp_path),
        "--base-url",
        "http://127.0.0.1:8000/v1",
        "--model",
        "local-model",
    )
    gateway = _find(rows, "llm-pi-ai")
    assert gateway is not None
    route = gateway["config"]["providers"]["azoth-gateway"]
    assert route["baseURL"] == "http://127.0.0.1:8000/v1"
    assert route["models"] == [{"id": "local-model"}]
    default = _find(rows, "agent-default-model")
    assert default is not None
    assert default["config"] == {"provider": "azoth-gateway", "model": "local-model"}
    assert _find(rows, "llm-deepseek") is None


def test_render_yaml_quotes_names_and_keeps_prompts_as_blocks(tmp_path: Path) -> None:
    text = launch_dsh.render_yaml(_rows(_root(tmp_path), "--study", "hazop"))
    assert "- insert:" in text
    assert 'name: "@deepseek-ai/dsh-agent-preset"' in text
    assert "prefix: |" in text
    assert "allowedAgentPresets:" in text


def test_skill_export_writes_a_manifest_and_removes_a_stale_entry(tmp_path: Path) -> None:
    root = _root(tmp_path)
    args, config = _args(root, "--skills", "hazop")
    assert launch_dsh.export_skills_for_launch(args, config) == 2
    skills = root / ".dsh" / "skills"
    assert (skills / "azoth-test.md").is_file()
    assert (skills / "azoth-hazop.md").is_file()
    manifest = json.loads((skills / ".manifest.json").read_text())
    assert set(manifest["skills"]) == {"azoth-test", "azoth-hazop"}

    stale = skills / "old-skill.md"
    stale.write_text("# old\n", encoding="utf-8")
    manifest["skills"]["old-skill"] = "x"
    (skills / ".manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
    launch_dsh.export_skills_for_launch(args, config)
    assert not stale.exists()


def test_skills_report_warns_before_the_first_export(tmp_path: Path) -> None:
    root = _root(tmp_path)
    args, config = _args(root, "--skills", "hazop")
    status, detail = launch_dsh.skills_report(args, config)
    assert status == "warn"
    assert "not exported" in detail
    (root / ".dsh" / "skills").mkdir(parents=True)
    status, detail = launch_dsh.skills_report(args, config)
    assert status == "fail"
    assert "missing" in detail


def test_check_is_read_only_and_reports_the_fake_runtime(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    root = _root(tmp_path)
    args, config = _args(root, "--skills", "hazop")
    launch_dsh.export_skills_for_launch(args, config)

    fake = tmp_path / "dsh"
    fake.write_text('#!/bin/sh\nif [ "$1" = "--version" ]; then echo 0.2.2; exit 0; fi\nexit 0\n')
    fake.chmod(0o755)

    before = {path.relative_to(root) for path in root.rglob("*")}
    code = launch_dsh.main(
        ["--check", "--json", "--root", str(root), "--dsh", str(fake), "--skills", "hazop"]
    )
    after = {path.relative_to(root) for path in root.rglob("*")}
    assert before == after, "the check wrote into the tree"
    report = json.loads(capsys.readouterr().out)
    assert report["schema"] == "azoth.launch-check/1"
    assert report["checks"]
    assert all("status" in check for check in report["checks"])
    assert code in (0, 1)


def test_print_config_does_not_write(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    root = _root(tmp_path)
    code = launch_dsh.main(["--print-config", "--root", str(root)])
    assert code == 0
    assert "- id:" in capsys.readouterr().out
    assert not (root / ".dsh" / "patches").exists()
