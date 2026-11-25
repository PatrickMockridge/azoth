# The agentic layer

azoth is a library, and an agent that uses it has to know how — which units, which
calculation ids, what a result's warnings mean, when to reach for a keycard. That
knowledge is written down as **skills**: self-contained specialisms an agent loads when a
task matches, each a `SKILL.md` with runnable examples and tests. [Skills](./skills.md)
is their shape; [the roadmap](./roadmap.md) maps them to the port tranches.

The layer is native to the repository rather than an external project. Skills live under
[`skills/`](../../../skills/README.md), their machine-readable metadata in
[`skills.toml`](../../../skills.toml), and
[`tools/validate_skills.py`](../../../tools/validate_skills.py) holds every skill to the
shape.

**The one agent is HAZOP** — a team of four roles (chair, process, safety, scribe) chained
through `guideword → deviation → cause → consequence → safeguard`. Its definition is
[`agents/hazop/README.md`](../../../agents/hazop/README.md), exported to `.claude/agents/`
and `.opencode/agents/`; the formal statement of a team as a composition of role-processes
is [Orchestration](./orchestration.md).

## Run it

```bash
python tools/launch_dsh.py --check
python tools/launch_dsh.py web --permission-mode read-only
```

The launcher exports the skills to `.dsh/skills/`, pins the checkout as the DSH workspace,
generates the overlay, and mounts the engineering preset, the welcome card, the calculation
plugin and the demo flowsheet. `--check` is the read-only probe; `--study hazop` (web
profile) enables the four role presets and Agent Teams; `--persona software` uses DSH's
coding preset.
[DeepSeek Harness](./harness.md) is the interface reference — modes, flags, the tools the
model is handed, the bridge's JSON envelopes and the check report — and
[`agents/deepseek/README.md`](../../../agents/deepseek/README.md) is the setup page.

The distribution name `azoth-engine` is on PyPI; the launcher and the skills are
repository features, because `tools/launch_dsh.py` is a repository tool. The
`agent` extra supplies the SDK's bundled runtime when `dsh` is not on PATH.
