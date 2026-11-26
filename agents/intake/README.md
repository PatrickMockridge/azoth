# Intake orchestrator

The second agent: four roles checking the supplied evidence before a calculation runs,
through the pipeline

`source → claim → binding → verdict → request`

specified in [`docs/src/agentic/orchestration.md`](../../docs/src/agentic/orchestration.md).
The dossier is the serialisation of the run — `@intake` — so a run re-entered with `*` is
the same run.

Roles, and what each carries:

- **clerk** — owns the dossier and the Team board, sequences the other roles, and calls
  `azoth_intake` for the verdict. It does not extract, bind or judge evidence.
- **reader** — extracts only what a source says: a bounded excerpt, a verbatim quote, the
  value and unit as written, and the source ladder's `used`/`blocked`/`absent` outcomes.
  It chains `azoth-units-vocabulary`.
- **assessor** — reads `azoth_spec` for each target and binds each declared input to a
  claim, one of the three licensed rules, or an explicit absence. It chains
  `azoth-run-calculation` and `azoth-provenance-verification`.
- **auditor** — independently re-reads every quote, hunts contradictions and unrecorded
  assumptions, checks that the register covers every input, and signs or returns the
  dossier. It chains `azoth-provenance-verification`.

The verdict is the deterministic `azoth_intake` tool's, not a role's: it returns `passed`,
`passed_with_flags` or `refused`, a ranked register of hard and soft gaps, and one request
per gap. A refused target is not run.

Runnable through `tools/launch_dsh.py web --permission-mode read-only --study intake`:
the clerk is the root preset, the three roles are subagent presets, and
[`agents/deepseek/skills/azoth-intake.md`](../deepseek/skills/azoth-intake.md) carries the
procedure. The runtime is a dev preview; this file fixes the agent, and the runtime
executes it.

The review is clerical, not a verification of the source: it checks that a value was
supplied, quoted and bound, not that the quoted number is the right physics. The
procedure says so in the report.

Setup: [`agents/README.md`](../README.md) and [`agents/deepseek/README.md`](../deepseek/README.md).
