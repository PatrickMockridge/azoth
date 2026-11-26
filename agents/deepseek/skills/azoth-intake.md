# Run an azoth intake review

An intake review turns supplied material into a dossier — sources, claims and bindings —
checks it with `azoth_intake`, and reports the gaps before a calculation runs:

`source → claim → binding → verdict → request`

A **hard gap** means the target cannot be constructed: a required input with no claim,
two sources in conflict, an ambiguous unit, an unlicensed inference. A **soft gap** means
it was constructed by a licensed inference or a lower source rung; it runs, is flagged,
and raises a request.

## Before you start

- Identify the target id(s) from the user's request. If more than one is plausible, ask.
- Call `azoth_spec` for every target. The assessor binds against the inputs, units,
  optional flags and valid range that declaration carries.
- Decide where the dossier and report go. In a read-only session the dossier lives in the
  conversation; `azoth_intake` and `azoth_calc` take it inline or read a path in the
  workspace.

## The team

You are the clerk. Spawn one teammate per role with the `subagent` tool and
`agent_preset`:

- `azoth-intake-reader` — sources and claims, with verbatim quotes and locators.
- `azoth-intake-assessor` — bindings to claim, rule or absent.
- `azoth-intake-auditor` — an independent read of the quotes and the register.

You sequence and record; you do not extract, bind or judge the evidence.

## The board

Create the reader task first, the assessor task with `blockedBy` naming it, and the audit
task with `blockedBy` naming the assessor's. Message each role when its task is ready: a
task change does not wake an idle agent. Update with the `expectedRevision` you read; a
stale revision is refused.

## The dossier

The reader returns `sources` and `claims`; the assessor returns `targets`. A source is
`prompt`, `file` or `calculation`, with a locator and a bounded excerpt. A claim carries
the value as written, the unit as written, a verbatim quote and the source id. A target
binds every declared input to exactly one of:

- `claim` — the supplied value;
- `rule` — one of the three licensed rules, with a `basis`;
- `absent` — no value, and why;
- `conflicted` — two sources disagree, and no resolution is recorded.

The licensed rules are the whole set:

- `composition_remainder` — the balance of a composition assigned to a component the
  same source names as unreported. The shortfall must be in the rule's band.
- `derived_by_id` — an input constructed by another registered id from supplied inputs.
  The derivation is run, not accepted as a typed-in number.
- `spec_default` — the numeric default the spec itself declares for an optional input.
  No shipped spec declares one today; a prose default is not read.

No rule guesses a typical value, reads a prose default, or resolves a conflict.

## The verdict

Call `azoth_intake` with the dossier. It returns the verdict, the ranked register and one
request per gap. The verdict is the tool's, not yours:

- `passed` — every required input is supplied.
- `passed_with_flags` — every required input is constructed, and at least one soft gap is
  declared. The target may run.
- `refused` — at least one hard gap. Do not call `azoth_calc` for it.

Pass the dossier to `azoth_calc` as `dossier` (or `dossier_path`) for a target that
passed. A dossier replaces `inputs`; do not send both. The result carries a sibling
`intake` block, and the library's own warnings and provenance are unchanged.

## The report

Return, in this order:

1. The targets and their verdicts.
2. The ranked register: hard gaps first, then soft, each with its code, input and detail.
3. **Data requests** — every request the tool raised, with what would close it.
4. For each target that ran: the calculation id, its result and its verification status.
5. The clerical limit: this review checks that a value was supplied and bound, not that
   the quoted number is the right physics.

## Limits

This tranche is advisory: `azoth_calc` does not mechanically refuse a refused target, and
the study procedure is what stops it. A dossier's prompt excerpt is caller-supplied; the
checker proves the quote is in the excerpt, not that the excerpt is the user's prompt.
The auditor's independent re-read is the defence for both.
