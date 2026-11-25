# DeepSeek Harness

DeepSeek Harness (`dsh`) is the runtime the agentic layer executes on. It is a
plugin-first agent harness from DeepSeek AI, MIT-licensed and a `0.2` developer preview;
azoth supplies the skills, the calculation tools and the HAZOP team's role definitions.

## What the launcher does

`tools/launch_dsh.py` does three things:

1. writes the skill catalog to `.dsh/skills/`, the project root DSH scans;
2. renders a `--patch` overlay for the shipped `web` or `headless` profile; and
3. starts `dsh` with that overlay.

The overlay is generated rather than committed because the two profiles compose
differently, the paths are absolute, and the plugin API is a preview. All DSH-specific
configuration is in that one generator, so a version bump is one edit.

```bash
python tools/launch_dsh.py --check --flowsheet F
python tools/launch_dsh.py web --flowsheet F
python tools/launch_dsh.py headless --flowsheet F "Run a HAZOP on F"
```

`--check` is read-only and probes the Python bridge, the `azoth` CLI and its MCP
handshake, the Node plugin and its tests, the exported skill hashes, and the model key.
`--print-config` prints the overlay; `--dry-run` adds the argv.

## The two tool surfaces

- **The calculation plugin** registers `azoth_catalogue`, `azoth_spec` and `azoth_calc`
  in DeepSeek Harness. It calls `python -m azoth.harness`, which resolves a registered
  calc or model id, decodes its spec inputs, runs the public function on the selected
  backend, and returns `to_dict()` with warnings and provenance.
- **The flowsheet MCP server** is the existing `azoth mcp --flowsheet F`, mounted through
  `@deepseek-ai/dsh-mcp-client`. Its 20 edit commands are served as `mcp__azoth__*`, with
  the four `azoth://` resources. The server holds one document in memory and never writes
  it.

The plugin imports no `@deepseek-ai/*` package: the tool registry accepts a plain tool
definition, so an absolute-path mount needs no node_modules. The engineering logic stays in
Python, where it is typed and tested with the library.

## Skills and the team

The 90 catalog skills export unchanged; `--skills hazop` writes a curated subset.
`agents/deepseek/skills/azoth-hazop.md` is a harness-only procedure, not a catalog skill,
and is what `/azoth-hazop` loads.

`--study hazop` enables the Team board and mounts four presets: `azoth-hazop-chair` as the
root, and `azoth-process`, `azoth-safety` and `azoth-scribe` as subagents. The chair
creates a task per node × guideword and sequences the pipeline
`guideword → deviation → cause → consequence → safeguard`; the scribe renders the
worksheet. [Orchestration](./orchestration.md) states the formal composition; the
execution is a mapping of it, not a proof.

## Security and limits

The launcher sets `workspace-write` and never mounts `sdk-minimal`, whose
`danger-full-access` pin is the wrong default. A non-loopback host requires
`--allow-remote`. The calculation tool accepts registry ids only — no `eval`, no shell,
no import by a model-supplied string; a keycard path must be under the working directory.
Inputs are capped at 1 MB, outputs at 8 MB, and a call times out at 120 s.

The library's models are `unverified` or `partially_verified`; a result is evidence for a
finding, not a design deliverable. The provenance block says which.

## Licence

DeepSeek Harness is MIT. azoth and the launcher/plugin are AGPL-3.0-or-later. No DSH source
is vendored, and no third-party AGPL DSH plugin is used. The hosted DeepSeek API has its
own terms; a local open-weight deployment has the model's.
