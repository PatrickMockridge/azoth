# DeepSeek Harness

The engineering front end for azoth: a launcher, a calculation plugin, the skill catalog
and a HAZOP team. DeepSeek Harness (`dsh`) is a developer preview from DeepSeek AI, built
on Cordis; this directory holds the azoth side of it.

## Run it

```bash
python tools/launch_dsh.py --check --flowsheet specs/flowsheets/demo.toml
python tools/launch_dsh.py web --flowsheet specs/flowsheets/demo.toml
python tools/launch_dsh.py headless --study hazop --flowsheet F "Run a HAZOP on F"
```

`--check` is read-only. It probes the Python bridge, the `azoth` CLI and the flowsheet MCP
handshake, the Node plugin and its tests, the exported skill hashes, and whether the model
key is set. `--print-config` prints the generated overlay; `--dry-run` adds the exact argv.
A real launch exports the skills to `.dsh/skills/`, writes the overlay to
`.dsh/patches/azoth-<mode>.yml`, and starts the shipped `web` or `headless` profile with
`--patch`.

Defaults: `DEEPSEEK_API_KEY`, `deepseek-v4-flash`, `DSH_HOME=<root>/.dsh-home`. A hosted
key is one route; `--base-url` adds an OpenAI-compatible `llm-pi-ai` provider instead, for
vLLM, SGLang or a gateway serving open-weight weights. `--dsh`/`AZOTH_DSH` names an
installed runtime; the Python SDK's bundled runtime is the fallback; `npx` is the last
resort and the only path that needs Node.

## The two tool surfaces

- **`azoth_catalogue`, `azoth_spec`, `azoth_calc`** — the plugin in [`plugin/`](plugin/).
  It calls `python -m azoth.harness`, which runs any of the 98 calculations and 300 models
  by id. Inputs are numbers in the unit `azoth_spec` declares, or `{value, unit}`; the
  result is the library's `to_dict()` with warnings and the provenance block.
- **`mcp__azoth__*`** — the existing `azoth mcp --flowsheet F` server, mounted through
  `@deepseek-ai/dsh-mcp-client`. It exposes the 20 flowsheet edit commands and the
  `azoth://document`, `azoth://diagnostics`, `azoth://report` and `azoth://catalogue`
  resources. The server holds the document in memory and never writes it.

The plugin imports nothing from `@deepseek-ai/*`: DeepSeek Harness accepts a plain tool
definition, so it mounts by absolute path without a package install. `plugin/bridge.js` is
pure Node and is tested with `node --test`; the engineering logic is Python.

## Skills and the HAZOP team

`tools/export_skills.py` writes the 90 catalog skills to `.dsh/skills/`, the project root
DeepSeek Harness scans at rank 100. `--skills hazop` exports the curated subset in
[`harness.toml`](harness.toml) instead. [`skills/azoth-hazop.md`](skills/azoth-hazop.md) is
a harness-only procedure, not a catalog skill: it is what `/azoth-hazop` loads.

`--study hazop` enables Agent Teams and sets the root preset to `azoth-hazop-chair`. The
chair creates one Team task per node × guideword, spawns the `azoth-process`,
`azoth-safety` and `azoth-scribe` presets through the `subagent` tool, and the scribe
renders the worksheet. The formal statement of a team as a composition of role-processes
is [`docs/src/agentic/orchestration.md`](../../docs/src/agentic/orchestration.md); this is
its execution mapping, not a proof.

## Security and limits

- The launcher sets `DSH_PERMISSION_MODE=workspace-write` and never mounts
  `sdk-minimal`, whose `danger-full-access` pin is the wrong default here. A non-loopback
  `--host` requires `--allow-remote`.
- `azoth_calc` accepts registry ids only; there is no `eval`, no shell and no import by a
  model-supplied string. A keycard path must be under the working directory.
- The bridge caps inputs at 1 MB and outputs at 8 MB; the plugin times a call out at 120 s.
- The catalog's models are `unverified` or `partially_verified`. A result is evidence for a
  finding, not a design deliverable; the provenance block says which.

## Licence

DeepSeek Harness is MIT. azoth, this directory's launcher and the plugin are
AGPL-3.0-or-later. No DeepSeek Harness source is vendored, and no third-party AGPL DSH
plugin is used. The hosted DeepSeek API has its own terms; a local open-weight deployment
has the model's.

The plugin and overlay target the `0.2` line (`0.2.2-rc.5` on Node 22; `0.2.3-alpha.2`
needs Node 24.22.3+). The version is declared in [`harness.toml`](harness.toml); the API is
a preview, so `--check` reports what is installed rather than assuming it.
