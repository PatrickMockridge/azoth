#!/usr/bin/env python3
"""Launch DeepSeek Harness with azoth's skills, tools and HAZOP team.

The launcher does four things:

1. pins the checkout as DSH's first workspace, where its instructions and skills live;
2. exports the skill catalog to `.dsh/skills/`;
3. renders a `--patch` overlay for the shipped `web` or `headless` profile; and
4. starts `dsh` with that overlay.

The overlay is generated rather than committed because the web and headless profiles
compose differently, the paths are absolute, and the DSH plugin API is a developer preview.
Everything DSH-specific is in this file; the calculation logic is in `python -m azoth.harness`
and the tool bridge is `agents/deepseek/plugin/`.

Usage:
    python tools/launch_dsh.py --check
    python tools/launch_dsh.py web --permission-mode read-only
    python tools/launch_dsh.py headless --permission-mode read-only "Size a pump for 100 m3/h"
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent

sys.path.insert(0, str(Path(__file__).resolve().parent))
import export_skills  # noqa: E402

SCHEMA = "azoth.launch-check/1"
MANIFEST = "azoth.skills/1"
WORKSPACE_DOMAIN = "workspace"
WORKSPACE_VERSION = 2
WORKSPACE_TITLE = "azoth"
HAZOP_SKILL = "azoth-hazop"
HAZOP_DESCRIPTION = (
    "Run a HAZOP as an azoth team: nodes and guidewords, deviation, cause, consequence "
    "and safeguard, recorded on the DeepSeek Harness Team board. USE WHEN: a HAZOP, a "
    "node-and-guideword study, or a process-safety review is to be run."
)
VERSION_RE = re.compile(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?")


# ---------------------------------------------------------------------------------------
# commands and the toolchain


@dataclass(frozen=True)
class Command:
    """An executable and its leading arguments, e.g. `cargo run -- ...`."""

    argv: tuple[str, ...]
    kind: str = "binary"

    def __str__(self) -> str:
        return " ".join(self.argv)

    def with_args(self, *args: str) -> list[str]:
        return [*self.argv, *args]


@dataclass
class Toolchain:
    python: Path
    dsh: Command
    azoth: Command | None
    node: Path | None
    root: Path
    dsh_home: Path
    notes: list[str] = field(default_factory=list)


class WorkspaceError(Exception):
    """A DSH workspace store that cannot be pinned without guessing."""


@dataclass
class WorkspaceReport:
    status: str
    detail: str
    changed: bool = False


def _which(name: str) -> Path | None:
    found = shutil.which(name)
    return Path(found) if found else None


def _sdk_binary() -> Path | None:
    """The `dsh` executable a pip-installed SDK bundles, if this environment has one."""
    venv = Path(sys.executable).with_name("dsh")
    if venv.is_file():
        return venv
    for package in ("deepseek_harness_runtime_bin", "deepseek_harness"):
        try:
            spec = importlib.util.find_spec(package)
        except (ImportError, ValueError):
            continue
        if spec is None or spec.submodule_search_locations is None:
            continue
        for base in spec.submodule_search_locations:
            for relative in ("bin/dsh", "dsh", "runtime/dsh", "vendor/dsh"):
                candidate = Path(base) / relative
                if candidate.is_file() and os.access(candidate, os.X_OK):
                    return candidate
    return None


def resolve_python(args: argparse.Namespace) -> Path:
    for value in (args.python, os.environ.get("AZOTH_PYTHON"), sys.executable):
        if not value:
            continue
        path = Path(value).expanduser()
        if path.is_file():
            return path
        found = _which(value)
        if found:
            return found
    raise SystemExit(f"launch_dsh: python executable not found: {args.python or sys.executable}")


def resolve_dsh(args: argparse.Namespace, config: dict[str, Any]) -> Command | None:
    explicit = args.dsh or os.environ.get("AZOTH_DSH")
    if explicit:
        path = Path(explicit).expanduser()
        if path.is_file():
            return Command((str(path),))
        found = _which(explicit)
        if found:
            return Command((str(found),))
        raise SystemExit(f"launch_dsh: --dsh {explicit!r} is not an executable")
    found = _which("dsh")
    if found:
        return Command((str(found),))
    sdk = _sdk_binary()
    if sdk:
        return Command((str(sdk),))
    if args.no_npx:
        return None
    npx = _which("npx")
    if npx:
        version = str(config["harness"]["dsh_version"])
        return Command((str(npx), "--yes", f"@deepseek-ai/dsh@{version}"), kind="npx")
    return None


def resolve_azoth(args: argparse.Namespace) -> Command | None:
    explicit = args.azoth_bin or os.environ.get("AZOTH_BIN")
    if explicit:
        path = Path(explicit).expanduser()
        if path.is_file():
            return Command((str(path),))
        found = _which(explicit)
        if found:
            return Command((str(found),))
        raise SystemExit(f"launch_dsh: --azoth-bin {explicit!r} is not an executable")
    root = Path(args.root).resolve()
    for candidate in (
        root / "target" / "release" / "azoth",
        root / "target" / "debug" / "azoth",
        root / "target" / "release" / "azoth.exe",
        root / "target" / "debug" / "azoth.exe",
    ):
        if candidate.is_file():
            return Command((str(candidate),))
    found = _which("azoth")
    if found:
        return Command((str(found),))
    cargo = _which("cargo")
    if cargo:
        return Command((str(cargo), "run", "-q", "-p", "azoth-cli", "--"), kind="cargo")
    return None


def child_env(
    args: argparse.Namespace, root: Path, *, dsh_home: Path | None = None
) -> dict[str, str]:
    env = os.environ.copy()
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    source = str(root / "python" / "src")
    parts = [source, env.get("PYTHONPATH", "")]
    env["PYTHONPATH"] = os.pathsep.join(part for part in parts if part)
    env["DSH_PERMISSION_MODE"] = args.permission_mode or "read-only"
    if dsh_home is not None:
        env["DSH_HOME"] = str(dsh_home)
    return env


def resolve(
    args: argparse.Namespace, config: dict[str, Any], *, require_dsh: bool = True
) -> Toolchain:
    root = Path(args.root).resolve()
    home = Path(
        args.dsh_home
        or os.environ.get("AZOTH_DSH_HOME")
        or os.environ.get("DSH_HOME")
        or root / ".dsh-home"
    ).expanduser()
    python = resolve_python(args)
    dsh = resolve_dsh(args, config)
    if dsh is None:
        if require_dsh:
            raise SystemExit(
                "launch_dsh: no DeepSeek Harness runtime found. Install `dsh` (npm or "
                "`pip install '.[agent]'`), pass --dsh, or allow npx."
            )
        dsh = Command(("dsh",))
    azoth = resolve_azoth(args)
    node = _which(args.node or "node")
    notes: list[str] = []
    if azoth is not None and azoth.kind == "cargo":
        notes.append("azoth CLI is not built; the flowsheet MCP server will build it on launch")
    return Toolchain(
        python=python,
        dsh=dsh,
        azoth=azoth,
        node=node,
        root=root,
        dsh_home=home,
        notes=notes,
    )


# ---------------------------------------------------------------------------------------
# YAML rendering


def _scalar(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if value is None:
        return "null"
    if isinstance(value, (int, float)):
        return repr(value)
    text = str(value)
    if not text:
        return '""'
    if "\n" in text:
        raise ValueError("a multiline string is rendered by _key_value, not _scalar")
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def _key_value(prefix: str, key: str, value: Any, indent: int) -> list[str]:
    lines: list[str] = []
    if isinstance(value, dict):
        if not value:
            lines.append(f"{prefix}{key}: {{}}")
        else:
            lines.append(f"{prefix}{key}:")
            lines.extend(_render_dict(value, indent + 2))
    elif isinstance(value, list):
        if not value:
            lines.append(f"{prefix}{key}: []")
        else:
            lines.append(f"{prefix}{key}:")
            lines.extend(_render_list(value, indent + 2))
    elif isinstance(value, str) and "\n" in value:
        lines.append(f"{prefix}{key}: |")
        body = value.rstrip("\n").split("\n")
        lines.extend(" " * (indent + 2) + line for line in body)
    else:
        lines.append(f"{prefix}{key}: {_scalar(value)}")
    return lines


def _render_dict(value: dict[str, Any], indent: int) -> list[str]:
    lines: list[str] = []
    for key, item in value.items():
        lines.extend(_key_value(" " * indent, str(key), item, indent))
    return lines


def _render_list(value: list[Any], indent: int) -> list[str]:
    lines: list[str] = []
    for item in value:
        if isinstance(item, dict):
            if not item:
                lines.append(" " * indent + "- {}")
                continue
            first = True
            for key, sub in item.items():
                prefix = " " * indent + ("- " if first else "  ")
                first = False
                lines.extend(_key_value(prefix, str(key), sub, indent + 2))
        elif isinstance(item, list):
            lines.append(" " * indent + "-")
            lines.extend(_render_list(item, indent + 2))
        else:
            lines.append(" " * indent + f"- {_scalar(item)}")
    return lines


def render_yaml(rows: list[dict[str, Any]]) -> str:
    return "\n".join(_render_list(rows, 0)) + "\n"


# ---------------------------------------------------------------------------------------
# the DSH workspace


def _now() -> str:
    return datetime.now(UTC).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def _canonical(path: str | Path) -> str:
    return str(Path(path).expanduser().resolve())


def _workspace_store(store: Path) -> dict[str, Any]:
    """A store this launcher can pin without guessing, or a named refusal."""
    try:
        data = json.loads(store.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise WorkspaceError(f"{store}: {exc}") from exc
    if not isinstance(data, dict):
        raise WorkspaceError(f"{store}: the store is not a JSON object")
    unit = data.get("unit")
    if (
        not isinstance(unit, dict)
        or unit.get("name") != WORKSPACE_DOMAIN
        or unit.get("version") != WORKSPACE_VERSION
    ):
        raise WorkspaceError(f"{store}: not a {WORKSPACE_DOMAIN} version {WORKSPACE_VERSION} store")
    global_state = data.get("global")
    tables = data.get("tables")
    if not isinstance(global_state, dict) or not isinstance(tables, dict):
        raise WorkspaceError(f"{store}: `global` and `tables` are not objects")
    order = global_state.get("workspaceIds")
    workspaces = tables.get("workspaces")
    if not isinstance(order, list) or not isinstance(workspaces, dict):
        raise WorkspaceError(
            f"{store}: `workspaceIds` is not a list or `workspaces` is not an object"
        )
    if len(order) != len(set(order)):
        raise WorkspaceError(f"{store}: `workspaceIds` repeats an id")
    if set(order) != set(workspaces):
        raise WorkspaceError(f"{store}: `workspaceIds` and `workspaces` name different ids")
    for workspace_id, record in workspaces.items():
        if not isinstance(workspace_id, str) or not isinstance(record, dict):
            raise WorkspaceError(f"{store}: a workspace entry is not a record")
        if not isinstance(record.get("path"), str) or not isinstance(record.get("title"), str):
            raise WorkspaceError(f"{store}: workspace {workspace_id} has no path or title")
        if not isinstance(record.get("sessionIds"), list):
            raise WorkspaceError(f"{store}: workspace {workspace_id} has no session list")
    return data


def _write_store(store: Path, data: dict[str, Any]) -> None:
    store.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary = tempfile.mkstemp(dir=store.parent, prefix=".workspace.", suffix=".json")
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as stream:
            json.dump(data, stream, indent=2)
            stream.write("\n")
        Path(temporary).chmod(0o600)
        Path(temporary).replace(store)
    finally:
        if Path(temporary).exists():
            Path(temporary).unlink()


def ensure_workspace(root: Path, home: Path, *, write: bool) -> WorkspaceReport:
    """Make the checkout the workspace DSH opens, preserving every other record."""
    store = home / "storages" / "workspace.json"
    canonical = _canonical(root)
    if not store.is_file():
        if not write:
            return WorkspaceReport("warn", f"{canonical}: no workspace store yet; a launch pins it")
        only_id = str(uuid.uuid4())
        now = _now()
        data: dict[str, Any] = {
            "unit": {"name": WORKSPACE_DOMAIN, "version": WORKSPACE_VERSION},
            "global": {
                "initialized": True,
                "defaultWorkspaceId": only_id,
                "workspaceIds": [only_id],
                "archivedSessionIds": [],
                "pinnedSessionIds": [],
            },
            "tables": {
                "workspaces": {
                    only_id: {
                        "path": canonical,
                        "title": WORKSPACE_TITLE,
                        "sessionIds": [],
                        "createdAt": now,
                        "updatedAt": now,
                    }
                }
            },
        }
        _write_store(store, data)
        return WorkspaceReport("ok", f"{canonical}: pinned", changed=True)

    try:
        data = _workspace_store(store)
    except WorkspaceError as exc:
        if write:
            raise
        return WorkspaceReport("fail", str(exc))

    global_state = data["global"]
    order: list[str] = global_state["workspaceIds"]
    workspaces: dict[str, Any] = data["tables"]["workspaces"]
    workspace_id: str | None = next(
        (key for key, record in workspaces.items() if _canonical(record["path"]) == canonical),
        None,
    )
    detail = f"{canonical}: "
    changed = False
    if workspace_id is None:
        workspace_id = str(uuid.uuid4())
        now = _now()
        workspaces[workspace_id] = {
            "path": canonical,
            "title": WORKSPACE_TITLE,
            "sessionIds": [],
            "createdAt": now,
            "updatedAt": now,
        }
        order.insert(0, workspace_id)
        detail += "added workspace"
        changed = True
    elif order[0] != workspace_id:
        order.remove(workspace_id)
        order.insert(0, workspace_id)
        detail += "moved to first"
        changed = True
    else:
        detail += "first"

    if not global_state.get("initialized"):
        global_state["initialized"] = True
        changed = True
    if global_state.get("defaultWorkspaceId") not in order:
        global_state["defaultWorkspaceId"] = workspace_id
        changed = True

    if changed and write:
        _write_store(store, data)
        return WorkspaceReport("ok", detail, changed=True)
    if changed:
        return WorkspaceReport(
            "warn", f"{detail}; a launch moves it to the first workspace", changed=True
        )
    return WorkspaceReport("ok", detail)


# ---------------------------------------------------------------------------------------
# the overlay


def _plugin_config(tc: Toolchain, config: dict[str, Any]) -> dict[str, Any]:
    return {
        "python": str(tc.python),
        "pythonpath": [str(tc.root / "python" / "src")],
        "cwd": str(tc.root),
        "timeoutMs": int(config["plugin"]["timeout_ms"]),
        "maxOutputBytes": int(config["plugin"]["max_output_bytes"]),
    }


def _mcp_config(args: argparse.Namespace, tc: Toolchain, config: dict[str, Any]) -> dict[str, Any]:
    assert tc.azoth is not None
    command = tc.azoth.argv[0]
    leading = list(tc.azoth.argv[1:])
    arguments = [
        *leading,
        "mcp",
        "--flowsheet",
        str(Path(args.flowsheet).resolve()),
        "--palette",
        str(Path(args.palette).resolve()),
    ]
    if args.no_run:
        arguments.append("--no-run")
    return {
        "serverName": "azoth",
        "transport": "stdio",
        "command": command,
        "args": arguments,
        "cwd": str(tc.root),
        "toolCallTimeoutMs": int(config["mcp"]["tool_call_timeout_ms"]),
        "startupTimeoutMs": int(config["mcp"]["startup_timeout_ms"]),
        "closeTimeoutMs": int(config["mcp"]["close_timeout_ms"]),
        "failOnStartupError": True,
    }


def _persona(prompt: Path) -> dict[str, Any]:
    return {
        "id": "persona",
        "name": "@deepseek-ai/dsh-persona",
        "config": {"prefix": prompt.read_text(encoding="utf-8").rstrip("\n"), "suffix": ""},
    }


def _role_plugins() -> list[dict[str, Any]]:
    return [
        {
            "id": "agent-instructions",
            "name": "@deepseek-ai/dsh-agent-instructions",
            "config": {"maxBytes": 65536},
        },
        {
            "id": "skill-filesystem",
            "name": "@deepseek-ai/dsh-skill-filesystem",
            "config": {"includeDefaultRoots": True},
        },
        {
            "id": "tool-skill",
            "name": "@deepseek-ai/dsh-tool-skill",
            "config": {"catalogDescriptionMaxLength": 500},
        },
        {"id": "tool-fs", "name": "@deepseek-ai/dsh-tool-fs", "config": {"applyPatch": False}},
        {
            "id": "tool-fs-search",
            "name": "@deepseek-ai/dsh-tool-fs-search",
            "config": {"sampleOverCapGlobResults": False},
        },
        {"id": "tool-bash", "name": "@deepseek-ai/dsh-tool-bash"},
        {
            "id": "tool-todo",
            "name": "@deepseek-ai/dsh-tool-todo",
            "config": {"allowParallelInProgress": True},
        },
    ]


def _engineer_plugins(args: argparse.Namespace, config: dict[str, Any]) -> list[dict[str, Any]]:
    meta = config["presets"]["engineer"]
    prompt = Path(args.root).resolve() / "agents" / "deepseek" / str(meta["prompt"])
    plugins = _role_plugins()
    plugins.insert(1, {"id": "time-context", "name": "@deepseek-ai/dsh-time-context"})
    plugins.append({"id": "tool-ask-user", "name": "@deepseek-ai/dsh-tool-ask-user"})
    plugins.append(
        {
            "id": "compaction",
            "name": "cordis:group",
            "group": True,
            "isolate": {"compaction": True, "toolResultPruner": True},
            "config": [
                {"id": "compaction-basic", "name": "@deepseek-ai/dsh-compaction-basic"},
                {"id": "command-compact", "name": "@deepseek-ai/dsh-command-compact"},
                {
                    "id": "tool-result-pruner",
                    "name": "@deepseek-ai/dsh-compaction-tool-result-pruner",
                    "config": {"thresholdChars": 8192, "headChars": 4096, "tailChars": 1024},
                },
            ],
        }
    )
    return [_persona(prompt), *plugins]


def _engineer_preset(args: argparse.Namespace, config: dict[str, Any]) -> dict[str, Any]:
    meta = config["presets"]["engineer"]
    return {
        "id": f"preset-{meta['id']}",
        "name": "@deepseek-ai/dsh-agent-preset",
        "config": {
            "id": str(meta["id"]),
            "order": int(meta["order"]),
            "plugins": _engineer_plugins(args, config),
        },
    }


def _presets(args: argparse.Namespace, config: dict[str, Any]) -> list[dict[str, Any]]:
    roles = config["presets"]
    process = str(roles["process"]["id"])
    safety = str(roles["safety"]["id"])
    scribe = str(roles["scribe"]["id"])
    rows: list[dict[str, Any]] = []
    for slug in ("chair", "process", "safety", "scribe"):
        meta = roles[slug]
        prompt = Path(args.root).resolve() / "agents" / "deepseek" / str(meta["prompt"])
        plugins = [_persona(prompt), *_role_plugins()]
        if slug == "chair":
            plugins.extend(
                [
                    {
                        "id": "tool-subagent-control",
                        "name": "@deepseek-ai/dsh-tool-subagent-control",
                        "config": {"interrupt": False},
                    },
                    {
                        "id": "tool-subagent",
                        "name": "@deepseek-ai/dsh-tool-subagent",
                        "config": {
                            "provider": "spawn",
                            "toolName": "subagent",
                            "modelSelection": False,
                            "allowedAgentPresets": [process, safety, scribe],
                            "maxDepth": 1,
                        },
                    },
                ]
            )
        rows.append(
            {
                "id": f"preset-{meta['id']}",
                "name": "@deepseek-ai/dsh-agent-preset",
                "config": {
                    "id": str(meta["id"]),
                    "order": int(meta["order"]),
                    "plugins": plugins,
                },
            }
        )
    return rows


def build_overlay(
    args: argparse.Namespace, tc: Toolchain, config: dict[str, Any]
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    inserts: list[dict[str, Any]] = []
    mode = "headless" if args.headless else "web" if args.web else args.mode
    plugin = tc.root / "agents" / "deepseek" / "plugin" / "index.js"
    if plugin.is_file():
        inserts.append(
            {
                "id": "azoth-calc",
                "name": str(plugin),
                "config": _plugin_config(tc, config),
            }
        )
    ui = tc.root / "agents" / "deepseek" / "ui" / "index.js"
    if args.persona == "engineer" and args.study != "hazop" and mode == "web" and ui.is_file():
        inserts.append({"id": "azoth-ui", "name": str(ui), "config": {}})
    if args.flowsheet and tc.azoth is not None:
        inserts.append(
            {
                "id": "mcp-azoth-flowsheet",
                "name": "@deepseek-ai/dsh-mcp-client",
                "config": _mcp_config(args, tc, config),
            }
        )
    if inserts:
        rows.append({"insert": inserts})

    if args.base_url:
        key_env = args.api_key_env or "AZOTH_DSH_API_KEY"
        provider = "azoth-gateway"
        rows.append(
            {
                "id": "llm-pi-ai",
                "config": {
                    "providers": {
                        provider: {
                            "apiKeyEnv": key_env,
                            "api": args.api,
                            "baseURL": args.base_url,
                            "models": [{"id": args.model}],
                        }
                    }
                },
            }
        )
    else:
        provider = str(config["harness"]["default_provider"])
        model: dict[str, Any] = {"apiKeyEnv": args.api_key_env or "DEEPSEEK_API_KEY"}
        model["models"] = [{"id": args.model}]
        if args.reasoning_effort:
            model["reasoningEffort"] = args.reasoning_effort
        rows.append({"id": "llm-deepseek", "config": model})
    rows.append(
        {"id": "agent-default-model", "config": {"provider": provider, "model": args.model}}
    )

    if args.persona == "engineer" and args.study != "hazop":
        engineer = str(config["presets"]["engineer"]["id"])
        if mode == "web":
            rows.append({"id": "agent-preset-registry", "config": {"default": engineer}})
            rows.append({"insert": [_engineer_preset(args, config)]})
        else:
            # Headless composes no preset roster; its deployment persona is the hook.
            prompt = (
                Path(args.root).resolve()
                / "agents"
                / "deepseek"
                / str(config["presets"]["engineer"]["prompt"])
            )
            rows.append(
                {
                    "id": "system-prompt",
                    "config": {"personaPrefix": prompt.read_text(encoding="utf-8").rstrip("\n")},
                }
            )

    if args.study == "hazop":
        chair = str(config["presets"]["chair"]["id"])
        rows.append(
            {
                "id": "agent-team",
                "config": {
                    "maxTasks": int(config["harness"]["max_tasks"]),
                    "defaultEnabled": True,
                },
            }
        )
        presets = _presets(args, config)
        if mode == "web":
            rows.append({"id": "agent-preset-registry", "config": {"default": chair}})
            rows.append({"insert": presets})
        else:
            rows.append(
                {
                    "insert": [
                        {
                            "id": "agent-preset-registry",
                            "name": "@deepseek-ai/dsh-agent-preset-registry",
                            "config": {"default": chair},
                        },
                        *presets,
                    ]
                }
            )
    return rows


# ---------------------------------------------------------------------------------------
# skills


def _skill_files(args: argparse.Namespace, config: dict[str, Any]) -> dict[Path, str]:
    if args.skills == "none":
        return {}
    names = config["skills"]["hazop"] if args.skills == "hazop" else None
    files = export_skills.expected(
        Path(args.root).resolve(), ["dsh"], names, source_root=Path(args.root).resolve()
    )
    root = Path(args.root).resolve()
    out = root / ".dsh" / "skills"
    body = (root / "agents" / "deepseek" / "skills" / f"{HAZOP_SKILL}.md").read_text(
        encoding="utf-8"
    )
    files[out / f"{HAZOP_SKILL}.md"] = export_skills.render_extra(
        HAZOP_SKILL, HAZOP_DESCRIPTION, body
    )
    return files


def _skill_hashes(files: dict[Path, str]) -> dict[str, str]:
    return {
        path.stem: hashlib.sha256(text.encode("utf-8")).hexdigest() for path, text in files.items()
    }


def export_skills_for_launch(args: argparse.Namespace, config: dict[str, Any]) -> int:
    files = _skill_files(args, config)
    if not files:
        return 0
    out = Path(args.root).resolve() / ".dsh" / "skills"
    out.mkdir(parents=True, exist_ok=True)
    for path, text in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    manifest_path = out / ".manifest.json"
    previous: dict[str, str] = {}
    if manifest_path.is_file():
        try:
            previous = dict(json.loads(manifest_path.read_text(encoding="utf-8")).get("skills", {}))
        except (json.JSONDecodeError, AttributeError):
            previous = {}
    current = _skill_hashes(files)
    for name in previous:
        if name not in current:
            stale = out / f"{name}.md"
            if stale.is_file():
                stale.unlink()
    manifest_path.write_text(
        json.dumps({"schema": MANIFEST, "skills": current}, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return len(files)


def skills_report(
    args: argparse.Namespace, config: dict[str, Any], home: Path | None = None
) -> tuple[str, str]:
    """A check result for the exported tree, computed without writing."""
    try:
        files = _skill_files(args, config)
    except (KeyError, OSError) as exc:
        return "fail", str(exc)
    if not files:
        return "ok", "skill export disabled"
    out = Path(args.root).resolve() / ".dsh" / "skills"
    if not out.is_dir():
        return "warn", "skills are not exported yet; a launch writes them"
    stale: list[str] = []
    for path, text in files.items():
        if not path.is_file():
            stale.append(f"missing {path.name}")
        elif path.read_text(encoding="utf-8") != text:
            stale.append(f"stale {path.name}")
    expected_names = {path.stem for path in files}
    orphans = (
        sorted(path.stem for path in out.glob("*.md") if path.stem not in expected_names)
        if out.is_dir()
        else []
    )
    if stale:
        return "fail", "; ".join(stale[:5])
    if orphans:
        return "warn", f"orphaned skills: {', '.join(orphans[:5])}"
    store_home = home or Path(
        args.dsh_home
        or os.environ.get("AZOTH_DSH_HOME")
        or str(Path(args.root).resolve() / ".dsh-home")
    )
    workspace = ensure_workspace(Path(args.root), store_home, write=False)
    if workspace.status == "fail":
        return "fail", f"{len(files)} skills exported, but {workspace.detail}"
    if workspace.status == "warn":
        return "warn", f"{len(files)} skills exported; {workspace.detail}"
    return "ok", f"{len(files)} skills in {workspace.detail}"


# ---------------------------------------------------------------------------------------
# checks


@dataclass
class Check:
    name: str
    status: str
    detail: str


def _run(
    argv: list[str],
    *,
    env: dict[str, str],
    timeout: float,
    stdin: str | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        argv,
        input=stdin,
        capture_output=True,
        text=True,
        timeout=timeout,
        env=env,
        check=False,
    )


def _version(text: str) -> str | None:
    match = VERSION_RE.search(text)
    return match.group(0) if match else None


def _probe_python(args: argparse.Namespace, tc: Toolchain, env: dict[str, str]) -> Check:
    try:
        proc = _run(
            [str(tc.python), "-B", "-c", "import azoth; print(azoth.__version__)"],
            env=env,
            timeout=args.timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return Check("python", "fail", str(exc))
    if proc.returncode != 0:
        detail = proc.stderr.strip().splitlines()[-1] if proc.stderr else "import failed"
        return Check("python", "fail", detail)
    return Check("python", "ok", f"{tc.python} (azoth {proc.stdout.strip()})")


def _probe_dsh(args: argparse.Namespace, tc: Toolchain, env: dict[str, str]) -> Check:
    if tc.dsh.kind == "npx":
        return Check("dsh", "warn", f"{tc.dsh} (fetches on first launch)")
    try:
        proc = _run(tc.dsh.with_args("--version"), env=env, timeout=args.timeout)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return Check("dsh", "fail", str(exc))
    if proc.returncode != 0:
        return Check("dsh", "fail", (proc.stderr or proc.stdout).strip()[-200:])
    found = _version(proc.stdout + proc.stderr)
    if found is None:
        return Check("dsh", "warn", f"cannot parse version from {proc.stdout.strip()!r}")
    major_minor = tuple(int(part) for part in found.split("-")[0].split(".")[:2])
    if major_minor != (0, 2):
        return Check("dsh", "warn", f"{found} is outside the tested 0.2 line")
    return Check("dsh", "ok", f"{found} ({tc.dsh})")


def _probe_node(
    args: argparse.Namespace, tc: Toolchain, config: dict[str, Any], env: dict[str, str]
) -> Check:
    if tc.node is None:
        return Check("node", "fail", "node not found (only the npx path needs it)")
    try:
        proc = _run([str(tc.node), "--version"], env=env, timeout=args.timeout)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return Check("node", "fail", str(exc))
    version = proc.stdout.strip().lstrip("v")
    if not version:
        return Check("node", "warn", "cannot parse node version")
    try:
        found = tuple(int(part) for part in version.split(".")[:2])
        floor = tuple(int(part) for part in str(config["harness"]["tested_node"]).split(".")[:2])
    except ValueError:
        return Check("node", "warn", version)
    if found < floor:
        return Check(
            "node",
            "fail",
            f"{version} is below the tested floor {config['harness']['tested_node']}",
        )
    return Check("node", "ok", version)


def _probe_azoth(args: argparse.Namespace, tc: Toolchain, env: dict[str, str]) -> Check:
    if tc.azoth is None:
        if args.flowsheet or args.study == "hazop":
            return Check("azoth", "fail", "no azoth CLI; the flowsheet MCP server needs one")
        return Check("azoth", "warn", "no azoth CLI; flowsheet tools unavailable")
    if tc.azoth.kind == "cargo":
        return Check("azoth", "warn", f"{tc.azoth} (builds on launch)")
    try:
        proc = _run(tc.azoth.with_args("--version"), env=env, timeout=args.timeout)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return Check("azoth", "fail", str(exc))
    if proc.returncode != 0:
        return Check("azoth", "fail", (proc.stderr or proc.stdout).strip()[-200:])
    return Check("azoth", "ok", f"{tc.azoth} ({proc.stdout.strip()})")


def _probe_flowsheet(args: argparse.Namespace, tc: Toolchain, env: dict[str, str]) -> Check | None:
    if not args.flowsheet:
        if args.require_flowsheet or args.study == "hazop":
            return Check("flowsheet", "fail", "--flowsheet is required")
        return None
    path = Path(args.flowsheet).resolve()
    if not path.is_file():
        return Check("flowsheet", "fail", f"{path} does not exist")
    if tc.azoth is None or tc.azoth.kind == "cargo":
        return Check("flowsheet", "warn", f"{path} (not checked without a built azoth CLI)")
    try:
        proc = _run(
            tc.azoth.with_args(
                "check", "--flowsheet", str(path), "--palette", str(Path(args.palette).resolve())
            ),
            env=env,
            timeout=args.timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return Check("flowsheet", "fail", str(exc))
    if proc.returncode != 0:
        return Check("flowsheet", "fail", (proc.stderr or proc.stdout).strip()[-300:])
    return Check("flowsheet", "ok", str(path))


def _probe_mcp(args: argparse.Namespace, tc: Toolchain, env: dict[str, str]) -> Check | None:
    if not args.flowsheet or tc.azoth is None or tc.azoth.kind == "cargo":
        return None
    requests = [
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "azoth-launch-check", "version": "0.1.0"},
            },
        },
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
    ]
    payload = "\n".join(json.dumps(request) for request in requests) + "\n"
    try:
        proc = _run(
            tc.azoth.with_args(
                "mcp",
                "--flowsheet",
                str(Path(args.flowsheet).resolve()),
                "--palette",
                str(Path(args.palette).resolve()),
            ),
            env=env,
            timeout=max(args.timeout, 20),
            stdin=payload,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return Check("mcp", "fail", f"startup failed: {exc}")
    tools = 0
    name: str | None = None
    for line in proc.stdout.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if message.get("id") == 1 and "result" in message:
            name = message["result"].get("serverInfo", {}).get("name")
        if message.get("id") == 2 and "result" in message:
            tools = len(message["result"].get("tools", []))
    if tools == 0:
        return Check("mcp", "fail", "no tools/list answer (the server is not speaking MCP)")
    if name != "azoth":
        return Check("mcp", "warn", f"{tools} tools, serverInfo {name!r}")
    return Check("mcp", "ok", f"{tools} flowsheet tools")


def _probe_bridge(args: argparse.Namespace, tc: Toolchain, env: dict[str, str]) -> Check:
    if not (tc.root / "python" / "src" / "azoth" / "harness" / "__main__.py").is_file():
        return Check("bridge", "warn", "azoth.harness is not present; calc tools are off")
    try:
        proc = _run(
            [str(tc.python), "-B", "-m", "azoth.harness", "version"],
            env=env,
            timeout=args.timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return Check("bridge", "fail", str(exc))
    if proc.returncode != 0:
        return Check("bridge", "fail", (proc.stderr or proc.stdout).strip()[-200:])
    try:
        document = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return Check("bridge", "fail", "version did not print JSON")
    return Check("bridge", "ok", f"azoth {document.get('azoth_version', '?')}")


def _probe_plugin(args: argparse.Namespace, tc: Toolchain, env: dict[str, str]) -> Check:
    plugin = tc.root / "agents" / "deepseek" / "plugin"
    index = plugin / "index.js"
    bridge = plugin / "bridge.js"
    ui = tc.root / "agents" / "deepseek" / "ui"
    if not index.is_file():
        return Check("plugin", "warn", "agents/deepseek/plugin/index.js is not present")
    if tc.node is None:
        return Check("plugin", "warn", "node not found; plugin not checked")
    sources = [index, bridge]
    tests = sorted((plugin / "test").glob("*.test.mjs"))
    if ui.is_dir():
        sources.extend([ui / "index.js", ui / "client.js"])
        tests.extend(sorted((ui / "test").glob("*.test.mjs")))
    try:
        for source in sources:
            if source.is_file():
                proc = _run([str(tc.node), "--check", str(source)], env=env, timeout=args.timeout)
                if proc.returncode != 0:
                    return Check("plugin", "fail", proc.stderr.strip()[-300:])
        if tests:
            proc = _run(
                [str(tc.node), "--test", *(str(path) for path in tests)],
                env=env,
                timeout=max(args.timeout, 60),
            )
            if proc.returncode != 0:
                return Check("plugin", "fail", proc.stderr.strip()[-300:])
    except (OSError, subprocess.TimeoutExpired) as exc:
        return Check("plugin", "fail", str(exc))
    suffix = f", {len(tests)} node tests" if tests else ""
    return Check("plugin", "ok", f"{index.name}{suffix}")


def _probe_workspace(tc: Toolchain) -> Check:
    report = ensure_workspace(tc.root, tc.dsh_home, write=False)
    return Check("workspace", report.status, report.detail)


def _probe_preset(args: argparse.Namespace, tc: Toolchain, config: dict[str, Any]) -> Check | None:
    if args.persona != "engineer" or args.study == "hazop":
        return None
    meta = config["presets"].get("engineer")
    if not isinstance(meta, dict) or not meta.get("id") or not meta.get("prompt"):
        return Check("preset", "fail", "harness.toml has no [presets.engineer]")
    prompt = tc.root / "agents" / "deepseek" / str(meta["prompt"])
    if not prompt.is_file() or not prompt.read_text(encoding="utf-8").strip():
        return Check("preset", "fail", f"{prompt} is missing or empty")
    return Check("preset", "ok", f"{meta['id']} ({prompt.name})")


def _probe_ui(args: argparse.Namespace, tc: Toolchain) -> Check | None:
    if args.persona != "engineer" or args.study == "hazop" or args.mode not in ("web", "check"):
        return None
    package = tc.root / "agents" / "deepseek" / "ui"
    manifest = package / "package.json"
    client = package / "client.js"
    index = package / "index.js"
    for path in (manifest, client, index):
        if not path.is_file():
            return Check("ui", "fail", f"{path} is missing")
    try:
        document = json.loads(manifest.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        return Check("ui", "fail", f"{manifest}: {exc}")
    dsh = document.get("dsh") if isinstance(document, dict) else None
    client_meta = dsh.get("client") if isinstance(dsh, dict) else None
    if not isinstance(client_meta, dict) or client_meta.get("platform") != "web":
        return Check("ui", "fail", f"{manifest}: no dsh.client web manifest")
    exports = document.get("exports") if isinstance(document, dict) else None
    if not isinstance(exports, dict) or not isinstance(exports.get("./client"), str):
        return Check("ui", "fail", f"{manifest}: no exports['./client']")
    return Check("ui", "ok", f"{document.get('name', 'azoth-dsh-ui')} ({client.name})")


def _probe_key(args: argparse.Namespace) -> Check:
    key_env = args.api_key_env or ("AZOTH_DSH_API_KEY" if args.base_url else "DEEPSEEK_API_KEY")
    if os.environ.get(key_env):
        return Check("api-key", "ok", f"{key_env} is set")
    if args.base_url:
        return Check("api-key", "warn", f"{key_env} is not set (only needed by the gateway)")
    return Check("api-key", "warn", f"{key_env} is not set; save a key in DSH or export it")


def run_checks(args: argparse.Namespace, config: dict[str, Any]) -> list[Check]:
    tc = resolve(args, config, require_dsh=False)
    with tempfile.TemporaryDirectory(prefix="azoth-dsh-check-") as home:
        env = child_env(args, tc.root, dsh_home=Path(home))
        checks: list[Check] = []
        root = tc.root
        missing = [
            path
            for path in ("skills.toml", "python/src/azoth", "agents/deepseek", "Cargo.toml")
            if not (root / path).exists()
        ]
        checks.append(
            Check(
                "repo",
                "fail" if missing else "ok",
                f"missing {missing}" if missing else str(root),
            )
        )
        checks.append(_probe_python(args, tc, env))
        checks.append(_probe_dsh(args, tc, env))
        if tc.dsh.kind == "npx":
            checks.append(_probe_node(args, tc, config, env))
        checks.append(_probe_azoth(args, tc, env))
        flowsheet = _probe_flowsheet(args, tc, env)
        if flowsheet is not None:
            checks.append(flowsheet)
        mcp = _probe_mcp(args, tc, env)
        if mcp is not None:
            checks.append(mcp)
        checks.append(_probe_bridge(args, tc, env))
        checks.append(_probe_plugin(args, tc, env))
        checks.append(_probe_workspace(tc))
        preset = _probe_preset(args, tc, config)
        if preset is not None:
            checks.append(preset)
        ui = _probe_ui(args, tc)
        if ui is not None:
            checks.append(ui)
        status, detail = skills_report(args, config, tc.dsh_home)
        checks.append(Check("skills", status, detail))
        checks.append(_probe_key(args))
        return checks


# ---------------------------------------------------------------------------------------
# launch


def _task(args: argparse.Namespace) -> str:
    words = list(args.task)
    if words == ["-"]:
        return sys.stdin.read()
    return " ".join(words).strip()


def _launch_argv(
    args: argparse.Namespace, tc: Toolchain, patch: Path, *, task: str | None = None
) -> list[str]:
    argv = [*tc.dsh.argv, "--profile", args.mode, "--patch", str(patch)]
    if args.mode == "web":
        if args.port:
            argv.extend(["--port", str(args.port)])
        if args.no_open:
            argv.append("--no-open")
        if args.host:
            argv.extend(["--host", args.host])
    else:
        if task is None:
            task = _task(args)
        if args.json:
            argv.append("--json")
        if args.session_id:
            argv.extend(["--session-id", args.session_id])
        argv.append(task)
    return argv


def _launch(args: argparse.Namespace, config: dict[str, Any]) -> int:
    if args.permission_mode is None:
        raise SystemExit("launch_dsh: choose --permission-mode read-only or workspace-write")
    tc = resolve(args, config)
    if args.study == "hazop" and not args.flowsheet:
        raise SystemExit("launch_dsh: --study hazop needs --flowsheet")
    try:
        workspace = ensure_workspace(tc.root, tc.dsh_home, write=True)
    except WorkspaceError as exc:
        raise SystemExit(f"launch_dsh: cannot pin the workspace: {exc}") from exc
    if workspace.changed:
        print(f"launch_dsh: {workspace.detail}")
    task = _task(args) if args.mode == "headless" else None
    if args.mode == "headless" and not task:
        raise SystemExit("launch_dsh: headless mode needs a task")
    written = export_skills_for_launch(args, config)
    if written:
        print(f"launch_dsh: exported {written} skill(s) to {tc.root / '.dsh' / 'skills'}")

    patch_dir = tc.root / ".dsh" / "patches"
    patch_dir.mkdir(parents=True, exist_ok=True)
    patch = patch_dir / f"azoth-{args.mode}.yml"
    patch.write_text(render_yaml(build_overlay(args, tc, config)), encoding="utf-8")

    argv = _launch_argv(args, tc, patch, task=task)
    env = child_env(args, tc.root, dsh_home=tc.dsh_home)
    if args.verbose:
        print("launch_dsh:", " ".join(argv))
        for note in tc.notes:
            print("launch_dsh: note:", note)
    try:
        return subprocess.run(argv, env=env, check=False).returncode or 0
    except FileNotFoundError as exc:
        raise SystemExit(f"launch_dsh: {exc}") from exc


# ---------------------------------------------------------------------------------------
# CLI


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("mode", nargs="?", choices=("web", "headless", "check"), default="web")
    p.add_argument("task", nargs="*", help="headless task; `-` reads stdin")
    p.add_argument("--check", action="store_true", help="probe without launching")
    p.add_argument("--headless", action="store_true", help="headless one-shot")
    p.add_argument("--web", action="store_true", help="interactive web UI")
    p.add_argument("--flowsheet")
    p.add_argument("--no-flowsheet", action="store_true")
    p.add_argument("--no-run", action="store_true")
    p.add_argument("--palette", default=str(ROOT / "specs" / "unit_ops"))
    p.add_argument("--require-flowsheet", action="store_true")
    p.add_argument("--root", default=str(ROOT))
    p.add_argument("--dsh")
    p.add_argument("--no-npx", action="store_true")
    p.add_argument("--python")
    p.add_argument("--azoth-bin")
    p.add_argument("--node")
    p.add_argument("--dsh-home")
    p.add_argument("--model")
    p.add_argument("--base-url")
    p.add_argument(
        "--api", choices=("openai-completions", "openai-responses"), default="openai-completions"
    )
    p.add_argument("--api-key-env")
    p.add_argument("--reasoning-effort", choices=("off", "low", "high", "max"))
    p.add_argument("--persona", choices=("engineer", "software"), default="engineer")
    p.add_argument("--study", choices=("none", "hazop"), default="none")
    p.add_argument("--skills", choices=("full", "hazop", "none"), default="full")
    p.add_argument("--permission-mode", choices=("read-only", "workspace-write"), default=None)
    p.add_argument("--port", type=int)
    p.add_argument("--no-open", action="store_true")
    p.add_argument("--host")
    p.add_argument("--allow-remote", action="store_true")
    p.add_argument("--session-id")
    p.add_argument("--json", action="store_true")
    p.add_argument("--print-config", action="store_true")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--verbose", action="store_true")
    p.add_argument("--timeout", type=float, default=15.0)
    return p


def _argv_with_mode(argv: list[str]) -> list[str]:
    """Let `--headless "task"` mean `headless "task"`, not a task parsed as the mode."""
    words = list(argv)
    bare = [word for word in words if not word.startswith("-")]
    for flag, mode in (("--headless", "headless"), ("--web", "web"), ("--check", "check")):
        if flag in words and (not bare or bare[0] not in ("web", "headless", "check")):
            words.insert(0, mode)
            break
    return words


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_intermixed_args(_argv_with_mode(sys.argv[1:] if argv is None else argv))
    if args.check:
        args.mode = "check"
    if args.headless:
        args.mode = "headless"
    if args.web:
        args.mode = "web"
    if args.host and not args.allow_remote and args.host not in ("127.0.0.1", "localhost"):
        raise SystemExit("launch_dsh: --host needs --allow-remote; the default is loopback")
    args.root = str(Path(args.root).resolve())
    config_path = Path(args.root) / "agents" / "deepseek" / "harness.toml"
    if not config_path.is_file():
        raise SystemExit(f"launch_dsh: {config_path} does not exist")
    config = tomllib.loads(config_path.read_text(encoding="utf-8"))
    if args.model is None:
        args.model = str(config["harness"]["default_model"])
    if args.flowsheet and args.no_flowsheet:
        raise SystemExit("launch_dsh: --flowsheet and --no-flowsheet are mutually exclusive")
    if args.flowsheet is None and not args.no_flowsheet:
        demo = Path(args.root) / "specs" / "flowsheets" / "demo.toml"
        if demo.is_file():
            args.flowsheet = str(demo)

    if args.mode == "check" or args.print_config or args.dry_run:
        if args.mode == "check":
            checks = run_checks(args, config)
            if args.json:
                print(
                    json.dumps(
                        {
                            "schema": SCHEMA,
                            "ok": all(check.status != "fail" for check in checks),
                            "checks": [check.__dict__ for check in checks],
                        },
                        indent=2,
                    )
                )
            else:
                for check in checks:
                    print(f"{check.status:4} {check.name:10} {check.detail}")
            return 1 if any(check.status == "fail" for check in checks) else 0
        tc = resolve(args, config, require_dsh=False)
        overlay = render_yaml(build_overlay(args, tc, config))
        print(overlay)
        if args.dry_run:
            patch = Path(args.root) / ".dsh" / "patches" / f"azoth-{args.mode}.yml"
            print("# " + " ".join(_launch_argv(args, tc, patch)))
            for check in run_checks(args, config):
                print(f"# {check.status:4} {check.name:10} {check.detail}")
        return 0

    return _launch(args, config)


if __name__ == "__main__":
    raise SystemExit(main())
