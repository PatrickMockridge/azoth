# Orchestration

The calculus states a calculation — a unit operation, a keycard, a process. This
page states an engineering team in the same language: a composition of
role-processes communicating over message channels, and a HAZOP as one such
process. It is the same machinery as the flowsheet, applied to agents rather than
to unit operations.

## A team is a process

A **role** is a named process: a procedure quoted as `@role` and re-entered as
`*role` ([Reflection](../calculus/rho.md)). A **team** is the parallel composition of its
roles:

```
team  ::=  chair | scribe | process | safety | ...
```

Each role reads the channels it owns and writes the channels downstream roles
read, as a unit operation reads its inlets and writes its outlets
([Processes](../calculus/process.md)). The channels carry **findings**, not streams: a
deviation, a cause, a safeguard.

**Claim (a team is a composition).** A team is barbed-bisimilar to the parallel
composition of its roles, each role a process whose only free names are its
declared channels.

*Status: **executed, not proved**. DeepSeek Harness runs the team as a root agent with
role presets and a task board; the claim is what that execution is checked against, and
the proof would still be the Lean layer.*

## A HAZOP is a process

A HAZOP passes each node and guideword through a fixed pipeline:

```
guideword → deviation → cause → consequence → safeguard
```

as a process `hazop`, each stage a role reading the previous stage's channel and
writing the next. The worksheet is the serialisation of that process: `@hazop`
turns the procedure into data, and `*` re-enters it.

**Claim (a HAZOP worksheet round-trips).** Quoting a HAZOP and dropping it back is
the same HAZOP.

*Status: **specified**, and it is not independent of [rho.md](../calculus/rho.md)'s round
trip: it is that theorem at the process `hazop`. It is named here because a reader
checking the orchestration layer wants the instance, not the general statement.*

## An intake review is a process

An intake review passes the supplied material through a fixed pipeline:

```
source → claim → binding → verdict → request
```

as a process `intake`, each stage a role reading the previous stage's channel and writing
the next: the reader writes sources and claims, the assessor writes bindings, the
deterministic checker writes the verdict and the register, and the clerk writes the
requests into the report. A **hard** gap means a target has no construction and is not
run; a **soft** gap is a licensed inference or a lower source rung, and it runs flagged.
The dossier is the serialisation of that process: `@intake` turns the procedure into data,
and `*` re-enters it.

**Claim (an intake dossier round-trips).** Quoting an intake review and dropping it back is
the same review.

*Status: **specified**, and it is the round-trip claim at the process `intake`.*

## The agents

[`agents/hazop/`](../../../agents/hazop/README.md) and
[`agents/intake/`](../../../agents/intake/README.md) are the two agents: the roles above,
the skills each chains, and the pipeline, in the terms this page fixes. HAZOP runs through
`tools/launch_dsh.py web --permission-mode read-only --study hazop`, which mounts the chair
as the root preset and the process, safety and scribe roles as subagent presets over the
DeepSeek Harness Team board. Intake runs with `--study intake`, mounting the clerk and the
reader, assessor and auditor. The web profile is required: the headless one-shot composes
no preset roster. The tool schema a model is handed — `@tool` and `*tool`, the same
reflection surface a GUI side panel or a notebook offers — is
[the middleware](../architecture/middleware.md), served by `azoth mcp`.

## What is not here

The runtime is DeepSeek Harness, not an azoth module: azoth owns the specification and
the role definitions, and `tools/launch_dsh.py` composes them into a DSH profile. The
claims above remain **executed, not proved** — the same "specified until a proof exists"
discipline as the process and reflection layers, with the Lean layer still the place a
proof would live.
