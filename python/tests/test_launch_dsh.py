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
                "max_tasks = 256",
                "",
                "[plugin]",
                "timeout_ms = 120000",
                "max_output_bytes = 4194304",
                "",
                "[providers.deepseek]",
                'label = "DeepSeek"',
                'route = "llm-deepseek"',
                'provider = "deepseek-official"',
                'api_key_env = "DEEPSEEK_API_KEY"',
                'default_model = "deepseek-v4-flash"',
                'models = ["deepseek-v4-flash", "deepseek-v4-pro"]',
                "",
                "[providers.openai]",
                'label = "OpenAI"',
                'route = "llm-pi-ai"',
                'provider = "openai"',
                'api_key_env = "OPENAI_API_KEY"',
                'default_model = "gpt-5.5"',
                'models = ["gpt-5.5", "gpt-5.5-pro"]',
                "",
                "[providers.anthropic]",
                'label = "Anthropic"',
                'route = "llm-pi-ai"',
                'provider = "anthropic"',
                'api_key_env = "ANTHROPIC_API_KEY"',
                'default_model = "claude-sonnet-5"',
                'models = ["claude-sonnet-5"]',
                "",
                "[providers.kimi]",
                'label = "Kimi (Moonshot)"',
                'route = "llm-pi-ai"',
                'provider = "moonshotai"',
                'api_key_env = "MOONSHOT_API_KEY"',
                'default_model = "kimi-k2.6"',
                'models = ["kimi-k2.6"]',
                "",
                "[providers.minimax]",
                'label = "MiniMax"',
                'route = "llm-pi-ai"',
                'provider = "minimax"',
                'api_key_env = "MINIMAX_API_KEY"',
                'default_model = "MiniMax-M2.7"',
                'models = ["MiniMax-M2.7"]',
                "",
                "[mcp]",
                "tool_call_timeout_ms = 60000",
                "startup_timeout_ms = 30000",
                "close_timeout_ms = 5000",
                "",
                "[skills]",
                'hazop = ["azoth-test"]',
                'intake = ["azoth-test"]',
                "",
                "[studies.hazop]",
                'root = "chair"',
                'roles = ["chair", "process", "safety", "scribe"]',
                'skill = "azoth-hazop"',
                'description = "Run a HAZOP test team."',
                "requires_flowsheet = true",
                "web_only = true",
                "",
                "[studies.intake]",
                'root = "intake-clerk"',
                'roles = ["intake-clerk", "intake-reader", "intake-assessor", "intake-auditor"]',
                'skill = "azoth-intake"',
                'description = "Review supplied claims."',
                "requires_flowsheet = false",
                "web_only = true",
                "root_asks_user = true",
                "",
                "[presets.engineer]",
                'id = "azoth-engineer"',
                "order = 5",
                'prompt = "prompts/engineer.md"',
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
                "[presets.intake-clerk]",
                'id = "azoth-intake-clerk"',
                "order = 20",
                'prompt = "prompts/intake-clerk.md"',
                "",
                "[presets.intake-reader]",
                'id = "azoth-intake-reader"',
                "order = 21",
                'prompt = "prompts/intake-reader.md"',
                "",
                "[presets.intake-assessor]",
                'id = "azoth-intake-assessor"',
                "order = 22",
                'prompt = "prompts/intake-assessor.md"',
                "",
                "[presets.intake-auditor]",
                'id = "azoth-intake-auditor"',
                "order = 23",
                'prompt = "prompts/intake-auditor.md"',
                "",
            ]
        ),
        encoding="utf-8",
    )
    for skill in ("azoth-hazop", "azoth-intake"):
        (harness / "skills" / f"{skill}.md").write_bytes(
            (ROOT / "agents" / "deepseek" / "skills" / f"{skill}.md").read_bytes()
        )
    for name in (
        "engineer",
        "chair",
        "process",
        "safety",
        "scribe",
        "intake-clerk",
        "intake-reader",
        "intake-assessor",
        "intake-auditor",
    ):
        (harness / "prompts" / f"{name}.md").write_bytes(
            (ROOT / "agents" / "deepseek" / "prompts" / f"{name}.md").read_bytes()
        )
    ui = harness / "ui"
    ui.mkdir()
    (ui / "index.js").write_text("export function apply() {}\n", encoding="utf-8")
    (ui / "client.js").write_text(
        "window.__ModuleLoader__.load({ id: 'azoth-dsh-ui', factory() { return {} } })\n",
        encoding="utf-8",
    )
    (ui / "package.json").write_text(
        json.dumps(
            {
                "name": "azoth-dsh-ui",
                "exports": {".": "./index.js", "./client": "./client.js"},
                "dsh": {"client": {"platform": "web"}},
            }
        ),
        encoding="utf-8",
    )
    demo = root / "specs" / "flowsheets" / "demo.toml"
    demo.parent.mkdir(parents=True)
    demo.write_text("", encoding="utf-8")
    return root


def _args(root: Path, *extra: str) -> tuple[Any, dict[str, Any]]:
    args = launch_dsh.parser().parse_args([*extra])
    args.root = str(root)
    config = tomllib.loads((root / "agents" / "deepseek" / "harness.toml").read_text())
    if args.provider is None:
        args.provider = "deepseek"
    if args.model is None:
        args.model = str(config["providers"][args.provider]["default_model"])
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


@pytest.fixture(autouse=True)
def _no_provider_keys(monkeypatch: pytest.MonkeyPatch) -> None:
    for name in (
        "DEEPSEEK_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "MOONSHOT_API_KEY",
        "MINIMAX_API_KEY",
    ):
        monkeypatch.delenv(name, raising=False)


def _store_credentials(home: Path, *refs: str) -> None:
    home.mkdir(parents=True, exist_ok=True)
    (home / ".credentials.yaml").write_text(
        "version: 1\nrecords: {}\nrefs:\n" + "".join(f"  {ref}: {{}}\n" for ref in refs),
        encoding="utf-8",
    )


def test_engineer_web_overlay_has_the_engineer_preset_but_no_team(tmp_path: Path) -> None:
    rows = _rows(_root(tmp_path), "--flowsheet", "case.toml")
    assert _find(rows, "mcp-azoth-flowsheet") is not None
    assert _find(rows, "llm-deepseek") is not None
    assert _find(rows, "azoth-ui") is not None
    registry = _find(rows, "agent-preset-registry")
    assert registry is not None
    assert registry["config"]["default"] == "azoth-engineer"
    preset = _find(rows, "preset-azoth-engineer")
    assert preset is not None
    plugins = {plugin["id"]: plugin for plugin in preset["config"]["plugins"]}
    assert plugins["tool-fs-search"]["config"]["sampleOverCapGlobResults"] is False
    assert plugins["tool-todo"]["config"]["allowParallelInProgress"] is True
    assert _find(rows, "agent-team") is None
    assert _find(rows, "preset-azoth-hazop-chair") is None


def test_headless_engineer_sets_the_deployment_persona(tmp_path: Path) -> None:
    rows = _rows(_root(tmp_path), "--headless", "--flowsheet", "case.toml")
    persona = _find(rows, "system-prompt")
    assert persona is not None
    assert "process-engineering assistant" in persona["config"]["personaPrefix"]
    assert _find(rows, "agent-preset-registry") is None
    assert _find(rows, "preset-azoth-engineer") is None


@pytest.mark.parametrize(
    ("choice", "provider", "model"),
    [
        ("openai", "openai", "gpt-5.5"),
        ("anthropic", "anthropic", "claude-sonnet-5"),
        ("kimi", "moonshotai", "kimi-k2.6"),
        ("minimax", "minimax", "MiniMax-M2.7"),
    ],
)
def test_a_provider_choice_sets_the_default_and_mounts_the_routes(
    tmp_path: Path, choice: str, provider: str, model: str
) -> None:
    rows = _rows(_root(tmp_path), "--provider", choice, "--flowsheet", "case.toml")
    default = _find(rows, "agent-default-model")
    assert default is not None
    assert default["config"] == {"provider": provider, "model": model}
    routes = _find(rows, "llm-pi-ai")
    assert routes is not None
    assert set(routes["config"]["providers"]) == {provider}
    assert _find(rows, "llm-deepseek") is not None


def test_the_default_provider_is_the_first_with_a_key(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    _, config = _args(_root(tmp_path))
    for name in (
        "DEEPSEEK_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "MOONSHOT_API_KEY",
        "MINIMAX_API_KEY",
    ):
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv("OPENAI_API_KEY", "x")
    assert launch_dsh.default_provider(config) == "openai"
    monkeypatch.delenv("OPENAI_API_KEY")
    monkeypatch.setenv("ANTHROPIC_API_KEY", "x")
    assert launch_dsh.default_provider(config) == "anthropic"


def test_the_default_provider_falls_back_to_deepseek(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    _, config = _args(_root(tmp_path))
    for name in (
        "DEEPSEEK_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "MOONSHOT_API_KEY",
        "MINIMAX_API_KEY",
    ):
        monkeypatch.delenv(name, raising=False)
    assert launch_dsh.default_provider(config) == "deepseek"


def test_a_stored_key_is_available(tmp_path: Path) -> None:
    root = _root(tmp_path)
    _, config = _args(root)
    home = tmp_path / "home"
    _store_credentials(home, "DEEPSEEK_API_KEY")
    assert launch_dsh.available_providers(config, home) == ["deepseek"]
    assert launch_dsh.default_provider(config, home) == "deepseek"


def test_a_stored_key_outranks_a_later_env_key(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = _root(tmp_path)
    _, config = _args(root)
    home = tmp_path / "home"
    _store_credentials(home, "DEEPSEEK_API_KEY")
    monkeypatch.setenv("OPENAI_API_KEY", "x")
    assert launch_dsh.available_providers(config, home) == ["deepseek", "openai"]
    assert launch_dsh.default_provider(config, home) == "deepseek"


def test_routes_follow_the_available_keys(tmp_path: Path) -> None:
    root = _root(tmp_path)
    _store_credentials(root / ".dsh-home", "DEEPSEEK_API_KEY")
    rows = _rows(root)
    assert _find(rows, "llm-deepseek") is not None
    assert _find(rows, "llm-pi-ai") is None
    ui = _find(rows, "azoth-ui")
    assert ui is not None
    assert ui["config"]["missingKey"] is False


def test_no_key_mounts_the_official_route_for_onboarding(tmp_path: Path) -> None:
    root = _root(tmp_path)
    rows = _rows(root)
    assert _find(rows, "llm-deepseek") is not None
    assert _find(rows, "llm-pi-ai") is None
    ui = _find(rows, "azoth-ui")
    assert ui is not None
    assert ui["config"]["missingKey"] is True


def test_a_missing_key_names_the_env_and_settings(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    args, config = _args(_root(tmp_path))
    monkeypatch.delenv("DEEPSEEK_API_KEY", raising=False)
    warning = launch_dsh.key_warning(args, config)
    assert warning is not None
    assert "DEEPSEEK_API_KEY" in warning
    assert "Settings -> Models" in warning
    monkeypatch.setenv("DEEPSEEK_API_KEY", "x")
    assert launch_dsh.key_warning(args, config) is None


def test_export_writes_the_user_root_too(tmp_path: Path) -> None:
    root = _root(tmp_path)
    args, config = _args(root, "--skills", "hazop")
    args.root = str(root)
    home = tmp_path / "home"
    written = launch_dsh.export_skills_for_launch(args, config, home)
    assert written >= 1
    assert (home / "skills" / "azoth-test.md").is_file()
    assert (root / ".dsh" / "skills" / "azoth-test.md").is_file()


def test_software_persona_leaves_the_shipped_standard_root(tmp_path: Path) -> None:
    rows = _rows(_root(tmp_path), "--persona", "software", "--flowsheet", "case.toml")
    assert _find(rows, "azoth-ui") is None
    assert _find(rows, "agent-preset-registry") is None
    assert _find(rows, "preset-azoth-engineer") is None
    assert _find(rows, "llm-deepseek") is not None


def test_study_overlay_wires_the_team_and_four_presets(tmp_path: Path) -> None:
    rows = _rows(_root(tmp_path), "--study", "hazop", "--flowsheet", "case.toml")
    team = _find(rows, "agent-team")
    assert team is not None
    assert team["config"]["defaultEnabled"] is True
    registry = _find(rows, "agent-preset-registry")
    assert registry is not None
    assert registry["config"]["default"] == "azoth-hazop-chair"
    assert _find(rows, "azoth-ui") is None
    assert _find(rows, "preset-azoth-engineer") is None
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


def test_intake_study_overlay_wires_the_clerk_and_three_roles(tmp_path: Path) -> None:
    rows = _rows(_root(tmp_path), "--study", "intake")
    team = _find(rows, "agent-team")
    assert team is not None
    assert team["config"]["defaultEnabled"] is True
    registry = _find(rows, "agent-preset-registry")
    assert registry is not None
    assert registry["config"]["default"] == "azoth-intake-clerk"
    assert _find(rows, "azoth-ui") is None
    assert _find(rows, "preset-azoth-engineer") is None
    clerk = _find(rows, "preset-azoth-intake-clerk")
    assert clerk is not None
    plugins = {plugin["id"]: plugin for plugin in clerk["config"]["plugins"]}
    assert plugins["tool-subagent"]["config"]["allowedAgentPresets"] == [
        "azoth-intake-reader",
        "azoth-intake-assessor",
        "azoth-intake-auditor",
    ]
    assert "tool-ask-user" in plugins
    for preset in (
        "azoth-intake-clerk",
        "azoth-intake-reader",
        "azoth-intake-assessor",
        "azoth-intake-auditor",
    ):
        assert _find(rows, f"preset-{preset}") is not None


def test_headless_study_is_refused(tmp_path: Path) -> None:
    root = _root(tmp_path)
    with pytest.raises(SystemExit, match="web profile"):
        launch_dsh.main(
            ["--headless", "--study", "hazop", "--flowsheet", "case.toml", "--root", str(root)]
        )


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
    assert launch_dsh.export_skills_for_launch(args, config) == 3
    skills = root / ".dsh" / "skills"
    assert (skills / "azoth-test.md").is_file()
    assert (skills / "azoth-hazop.md").is_file()
    assert (skills / "azoth-intake.md").is_file()
    manifest = json.loads((skills / ".manifest.json").read_text())
    assert set(manifest["skills"]) == {"azoth-test", "azoth-hazop", "azoth-intake"}

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
    assert {"workspace", "preset", "ui"} <= {check["name"] for check in report["checks"]}
    assert code in (0, 1)


def test_print_config_does_not_write(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    root = _root(tmp_path)
    code = launch_dsh.main(["--print-config", "--root", str(root)])
    assert code == 0
    output = capsys.readouterr().out
    assert "- id:" in output
    assert "preset-azoth-engineer" in output
    assert "azoth-ui" in output
    assert "mcp-azoth-flowsheet" in output
    assert not (root / ".dsh" / "patches").exists()


def test_print_config_can_leave_out_the_demo_and_the_engineer_ui(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    root = _root(tmp_path)
    code = launch_dsh.main(
        ["--print-config", "--no-flowsheet", "--persona", "software", "--root", str(root)]
    )
    assert code == 0
    output = capsys.readouterr().out
    assert "mcp-azoth-flowsheet" not in output
    assert "preset-azoth-engineer" not in output
    assert "azoth-ui" not in output


def test_flowsheet_and_no_flowsheet_are_mutually_exclusive(tmp_path: Path) -> None:
    root = _root(tmp_path)
    with pytest.raises(SystemExit, match="mutually exclusive"):
        launch_dsh.main(
            ["--print-config", "--flowsheet", "case.toml", "--no-flowsheet", "--root", str(root)]
        )


def test_launch_requires_an_explicit_permission_mode(tmp_path: Path) -> None:
    root = _root(tmp_path)
    with pytest.raises(SystemExit, match="permission-mode"):
        launch_dsh.main(["web", "--root", str(root)])


def test_headless_flag_accepts_a_task_with_options_between(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    root = _root(tmp_path)
    code = launch_dsh.main(
        [
            "--headless",
            "--print-config",
            "--permission-mode",
            "read-only",
            "--root",
            str(root),
            "hello",
            "world",
        ]
    )
    assert code == 0
    assert "- id:" in capsys.readouterr().out


def _workspace_state(path: str, workspace_id: str) -> dict[str, Any]:
    now = "2026-01-01T00:00:00.000Z"
    return {
        "unit": {"name": "workspace", "version": 2},
        "global": {
            "initialized": True,
            "defaultWorkspaceId": workspace_id,
            "workspaceIds": [workspace_id],
            "archivedSessionIds": [],
            "pinnedSessionIds": [],
        },
        "tables": {
            "workspaces": {
                workspace_id: {
                    "path": path,
                    "title": "elsewhere",
                    "sessionIds": [],
                    "createdAt": now,
                    "updatedAt": now,
                }
            }
        },
    }


def test_workspace_pins_the_checkout_first_and_is_idempotent(tmp_path: Path) -> None:
    root = _root(tmp_path)
    home = tmp_path / "home"
    first = launch_dsh.ensure_workspace(root, home, write=True)
    assert first.changed
    store = json.loads((home / "storages" / "workspace.json").read_text())
    workspace_id = store["global"]["workspaceIds"][0]
    assert store["global"]["defaultWorkspaceId"] == workspace_id
    assert store["tables"]["workspaces"][workspace_id]["path"] == str(root.resolve())
    second = launch_dsh.ensure_workspace(root, home, write=True)
    assert not second.changed
    assert json.loads((home / "storages" / "workspace.json").read_text()) == store


def test_workspace_preserves_another_workspace_without_reordering(tmp_path: Path) -> None:
    root = _root(tmp_path)
    home = tmp_path / "home"
    elsewhere = home / "storages" / "workspace.json"
    elsewhere.parent.mkdir(parents=True)
    elsewhere.write_text(json.dumps(_workspace_state("/tmp/elsewhere", "other")), encoding="utf-8")
    check = launch_dsh.ensure_workspace(root, home, write=False)
    assert check.status == "warn"
    assert check.changed
    launch_dsh.ensure_workspace(root, home, write=True)
    store = json.loads(elsewhere.read_text())
    assert len(store["global"]["workspaceIds"]) == 2
    assert "other" in store["global"]["workspaceIds"]
    assert store["global"]["workspaceIds"][0] == "other"
    assert store["tables"]["workspaces"]["other"]["path"] == "/tmp/elsewhere"


def test_explicit_workspace_is_created_and_opened(tmp_path: Path) -> None:
    root = _root(tmp_path)
    home = tmp_path / "home"
    target = tmp_path / "Documents" / "DHS" / "dws"
    report = launch_dsh.ensure_workspace(root, home, write=True, workspace=target)
    assert report.changed
    assert target.is_dir()
    store = json.loads((home / "storages" / "workspace.json").read_text())
    first = store["global"]["workspaceIds"][0]
    assert store["global"]["defaultWorkspaceId"] == first
    assert store["tables"]["workspaces"][first]["path"] == str(target.resolve())


def test_the_checkout_is_added_beside_an_explicit_workspace(tmp_path: Path) -> None:
    root = _root(tmp_path)
    home = tmp_path / "home"
    target = tmp_path / "elsewhere"
    launch_dsh.ensure_workspace(root, home, write=True, workspace=target)
    launch_dsh.ensure_workspace(root, home, write=True)
    store = json.loads((home / "storages" / "workspace.json").read_text())
    paths = [record["path"] for record in store["tables"]["workspaces"].values()]
    assert paths[0] == str(target.resolve())
    assert str(root.resolve()) in paths


def test_workspace_refuses_a_corrupt_store(tmp_path: Path) -> None:
    root = _root(tmp_path)
    home = tmp_path / "home"
    store = home / "storages" / "workspace.json"
    store.parent.mkdir(parents=True)
    store.write_text("{", encoding="utf-8")
    report = launch_dsh.ensure_workspace(root, home, write=False)
    assert report.status == "fail"
    with pytest.raises(launch_dsh.WorkspaceError):
        launch_dsh.ensure_workspace(root, home, write=True)


def test_workspace_is_not_written_by_a_read(tmp_path: Path) -> None:
    root = _root(tmp_path)
    home = tmp_path / "home"
    report = launch_dsh.ensure_workspace(root, home, write=False)
    assert report.status == "warn"
    assert not (home / "storages" / "workspace.json").exists()
