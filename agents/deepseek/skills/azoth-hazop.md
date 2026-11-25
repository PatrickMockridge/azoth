# Run an azoth HAZOP

A HAZOP passes each node and guideword through the pipeline

`guideword → deviation → cause → consequence → safeguard`

and serialises the result as a worksheet. This procedure uses the DeepSeek Harness Team
board for the work and the `azoth-process`, `azoth-safety` and `azoth-scribe` presets for
the roles.

## Before you start

- Read the flowsheet through the flowsheet MCP server: `azoth://document`,
  `azoth://graph` and `azoth://diagnostics`. If the task text names the nodes, use those.
- Confirm the node list and the guideword set with the user when either is ambiguous.
- Decide where the worksheet will be written, and name that path in the scribe's task.

## The team

You are the chair. Spawn one teammate per role with the `subagent` tool and
`agent_preset`:

- `azoth-process` — deviation, cause and consequence.
- `azoth-safety` — safeguard, consequence prevented, coverage and basis.
- `azoth-scribe` — the worksheet, in the reporting skills' house style.

You sequence and record; you do not invent findings.

## The board

Create one process task per node × guideword with `team_task_create`. The title names the
node and guideword; the details name the streams, the design intent and any calculation the
role should run. Then create the safety task with `blockedBy` naming the process task, and
the scribe task with `blockedBy` naming the safety task.

Spawn the process and safety teammates once per study, not once per task, and message each
with the node/guideword they are starting on. A task change does **not** wake an idle
agent: send an explicit message after you create or update the tasks it should act on.

Update a task with `team_task_update` and the `expectedRevision` you last read. A stale
revision is refused; re-read with `team_task_get` and retry rather than overwriting another
role's update.

## What each finding must carry

- **Deviation** — the guideword applied to the node's design intent.
- **Cause** — the equipment or operation that produces it.
- **Consequence** — what happens, and how far it reaches.
- **Safeguard** — the independent protection, its action, and whether its coverage is
  adequate.
- **Basis** — the calculation id and result, or the standard or procedure, that supports
  the finding. A number without a basis is not a finding.

Use the azoth calculation tools (`azoth_catalogue`, `azoth_spec`, `azoth_calc`) when the
finding depends on a number, and record the id and the result's verification status. The
skills under `.dsh/skills` teach the method for each domain.

## The worksheet

The scribe writes one row per node × guideword with the six fields above. The chair checks
that every row is complete, that no finding was added or removed, and that the worksheet
names the flowsheet document and the calculation ids it used. Write the final worksheet to
the path chosen before the study, or return it in the conversation when no path was named.

## Limits

This is a screening study, not a design HAZOP. The library's models are `unverified` or
`partially_verified`; a result is evidence for a finding, not a design deliverable. Say so
in the worksheet when a finding relies on one.
