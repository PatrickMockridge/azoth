# DeepSeek Harness

The engineering entry point for azoth: a launcher, a calculation plugin, a planning guard,
a welcome card, the skill catalog and two engineering teams, HAZOP and intake. DeepSeek
Harness (`dsh`) is a developer preview from DeepSeek AI, built on Cordis; this directory
holds the azoth side of it.

## Run it

```bash
python tools/launch_dsh.py --check
python tools/launch_dsh.py web --permission-mode read-only
python tools/launch_dsh.py web --permission-mode read-only --provider openai --model gpt-5.5
python tools/launch_dsh.py headless --permission-mode read-only "Size a pump for 100 m³/h"
python tools/launch_dsh.py web --permission-mode read-only --study hazop --flowsheet F
```

`--check` is read-only. It probes the Python bridge, the `azoth` CLI and the flowsheet MCP
handshake, the Node plugin and welcome UI, the exported skill hashes, the DSH workspace,
the engineering preset, the selected provider, and which provider keys are set.
`--print-config` prints the
generated overlay; `--dry-run` adds the exact argv. A real launch exports the skills to
`.dsh/skills/` and `$DSH_HOME/skills`, writes the overlay to
`.dsh/patches/azoth-<mode>.yml`, registers the checkout (or `--workspace PATH`, created
when absent) without reordering the store, and starts the shipped `web` or `headless`
profile with `--patch`.

Defaults: the first provider whose key is exported or stored in the DSH credential store
(`deepseek` when none is), the provider's `DEEPSEEK_API_KEY`/`default_model`,
`DSH_HOME=<root>/.dsh-home`, the demo flowsheet `specs/flowsheets/demo.toml`, and
`--persona engineer`. A launch whose default route has no key prints the env var and the
Settings → Models remedy. `--persona software` leaves DSH's coding preset in place.
`--no-flowsheet` starts without a case. `--permission-mode` is required for a launch:
`read-only` keeps calculations and in-memory flowsheet edits available while refusing
file writes; `workspace-write` permits them. `--dsh`/`AZOTH_DSH` names an installed
runtime; the Python SDK's bundled runtime is the fallback; `npx` is the last resort and
the only path that needs Node.

The launcher mounts five provider routes so the model selector can switch without a
relaunch:

| `--provider` | Route | Model default | Key |
|---|---|---|---|
| `deepseek` | `deepseek-official` | `deepseek-v4-flash` | `DEEPSEEK_API_KEY` |
| `openai` | OpenAI catalog | `gpt-5.5` | `OPENAI_API_KEY` |
| `anthropic` | Anthropic catalog | `claude-sonnet-5` | `ANTHROPIC_API_KEY` |
| `kimi` | Moonshot catalog | `kimi-k2.6` | `MOONSHOT_API_KEY` |
| `minimax` | MiniMax catalog | `MiniMax-M2.7` | `MINIMAX_API_KEY` |

`--provider` sets the session default; when omitted, the first provider with an exported
or stored key is the default. `--model` overrides the id. Only providers with a key are
mounted, so the selector matches the available keys; with no key at all the official
DeepSeek route is mounted for DSH's onboarding step. `--base-url` replaces the native
routes with an OpenAI-compatible gateway, vLLM/SGLang endpoint or any model the catalog
does not list.

The first web session shows a compact card: azoth is a chemical-engineering calculation
library, FEED means Front-End Engineering Design (not a browser front end), the demo
flowsheet is loaded, and results are screening values. Three buttons fill the composer
with a pump sizing, the demo separator, and relief and fire questions.

## The tool surfaces

- **`azoth_help`, `azoth_catalogue`, `azoth_spec`, `azoth_intake`, `azoth_calc`** — the
  plugin in [`plugin/`](plugin/). `azoth_help` defines FEED, model, catalogue, keycard,
  dossier, hard gap and soft gap; `azoth_intake` checks a dossier before a calculation; the
  others call `python -m azoth.harness`, which runs any of the 97 calculations and 305
  models by id. Inputs are numbers in the unit `azoth_spec` declares, or `{value, unit}`;
  a dossier replaces `inputs` and adds a sibling `intake` block. The result is the
  library's `to_dict()` with warnings and the provenance block, rendered with what its
  verification means.
- **`azoth_plan`** — the planning guard in [`plugin/guard.js`](plugin/guard.js). It records
  a decision tree: `open` requires at least two candidate paths, each with a cost, an
  estimate, a fallback and assumptions; `choose` fixes the path; `add` attaches a
  subordinate decision; `step` records ordered work with a kind, a time box and a success
  test. An assumption names a category — numerical, validity, evidence, applicability,
  consistency, feasibility, requirement, budget or judgment — and, where a result can
  decide it, a falsifier bound to a calculation id, `"*"` or `intake`. A study maps each
  question to a fidelity and an answer; `basis` records the consistency checks; a solver
  step needs a feasibility record. `fail` logs a failure, and three with one signature
  trip the path; `target` and `compare` record a supplied validation value and its spread
  across at least two models. `azoth_calc` and `azoth_intake` results are checked
  automatically for the numerical, validity and evidence signals; `trip` records any other
  reason. A trip holds the calculation until `reopen` returns to the decision or `resolve`
  records evidence that the assumption survived. `permit` records an external bash
  calculation — command, scope, purpose and verification — and a matching command is
  refused until the user approves it. `review` is the periodic checkpoint; `job` records a
  background job; `status` returns the record. The thresholds and the warning-category map
  are `[plan]` in [`harness.toml`](harness.toml).
- **`mcp__azoth__*`** — the existing `azoth mcp --flowsheet F` server, mounted through
  `@deepseek-ai/dsh-mcp-client`. It exposes the 20 flowsheet edit commands and the
  `azoth://document`, `azoth://diagnostics`, `azoth://report` and `azoth://catalogue`
  resources. The server holds the document in memory and never writes it.

The plugins import nothing from `@deepseek-ai/*`: DeepSeek Harness accepts a plain tool
definition, so they mount by absolute path without a package install. `plugin/bridge.js`
is pure Node and is tested with `node --test`; the engineering logic is Python.

### The bridge

The plugin is a shell around `python -m azoth.harness`:

```bash
python -m azoth.harness catalogue --query darcy
python -m azoth.harness spec hydraulics.darcy_weisbach
python -m azoth.harness run hydraulics.reynolds_number \
  --inputs-json '{"rho":998,"v":1.5,"D":0.1,"mu":0.001002}'
python -m azoth.harness review --dossier-file agents/intake/examples/gas-analysis.toml
python -m azoth.harness run eos.pt_flash --dossier-file agents/intake/examples/gas-analysis.toml
python -m azoth.harness version
```

It writes one JSON envelope on stdout, schema `azoth.harness/1`, with `kind` of
`catalogue`, `spec`, `review`, `result` or `error`; a result is the library's `to_dict()`,
the provenance block included, with an `intake` sibling when a dossier was supplied. The
exit status is `0` for success, `2` for an unknown id, `3` for an invalid input or a
malformed dossier, `4` for a calculation error and `5` for an internal failure. Inputs cap
at 1 MB and the CLI's output at 8 MB. [The harness
chapter](../../docs/src/agentic/harness.md) states the full contract.

## Skills and the two teams

`tools/export_skills.py` writes the 90 catalog skills to `.dsh/skills/`, the project root
DSH scans at rank 100 once the checkout is the session workspace. `--skills hazop` or
`--skills intake` exports the curated subset in [`harness.toml`](harness.toml) instead.
[`skills/azoth-hazop.md`](skills/azoth-hazop.md) and
[`skills/azoth-intake.md`](skills/azoth-intake.md) are harness-only procedures, not catalog
skills: they are what `/azoth-hazop` and `/intake` load.

`--study hazop` (web only) enables Agent Teams and sets the root preset to
`azoth-hazop-chair`. The chair creates one Team task per node × guideword, spawns the
`azoth-process`, `azoth-safety` and `azoth-scribe` presets through the `subagent` tool, and
the scribe renders the worksheet. `--study intake` sets the root to `azoth-intake-clerk`
and spawns `azoth-intake-reader`, `azoth-intake-assessor` and `azoth-intake-auditor`; the
clerk calls `azoth_intake` for a verdict and runs no refused target. The headless one-shot
composes no preset roster, so it refuses either study. The formal statement of a team as a
composition of role-processes is
[`docs/src/agentic/orchestration.md`](../../docs/src/agentic/orchestration.md); this is its
execution mapping, not a proof.

A complete worked output is
[`case-studies/azoth-feed-08`](../../case-studies/azoth-feed-08/README.md), the notebook-08 FEED
study and HAZOP close-out.

## Security and limits

- A launch requires `--permission-mode read-only` or `workspace-write`; there is no
  default. The launcher sets `DSH_PERMISSION_MODE` from it and never mounts `sdk-minimal`,
  whose `danger-full-access` pin is the wrong default here. A non-loopback `--host`
  requires `--allow-remote`.
- `azoth_calc` and `azoth_intake` accept registry ids and a dossier only; there is no
  `eval`, no shell and no import by a model-supplied string. A keycard path and a dossier
  path must be under the working directory.
- The bridge caps inputs at 1 MB and outputs at 8 MB; the plugin times a call out at 120 s.
- The planning guard gates `azoth_calc` and every bash command that runs an interpreter.
  Read-only commands pass; `python -m azoth.harness` and `azoth` are azoth and pass the
  plan holds; any other interpreter is an external calculation that needs an
  `azoth_plan permit` and the user's approval, or `--allow-external-compute` at launch.
  Every external run is bannered and logged. The guard is per session, so a spawned
  subagent opens its own plan.
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
