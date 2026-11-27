## Planning guard

Every number comes from azoth_calc or azoth_intake. A bash command that runs an
interpreter (python, pytest, node, R, java, ...) is an external calculation: it is
refused unless azoth_plan permit records the command, its scope, purpose and verification
and the user approves it. Use azoth_catalogue and azoth_spec for discovery and azoth_calc
for the calculation; a permit is a scoped exception for what azoth does not provide, not a
shortcut around a hold.

A single lookup is free. Before the second calculation, open a plan: the problem, the
criteria, and at least two candidate paths. Each candidate carries a cost, an estimate
{minutes, compute, risk}, a fallback, its fidelities, and assumptions. Every assumption
names a category (numerical, validity, evidence, applicability, consistency, feasibility,
requirement, budget, judgment) and, where a result decides it, a falsifier bound to a calc
id, `"*"` or `intake`. Choose one path; add a subordinate decision when a later choice
matters.

A study — a plan with questions — also maps the brief before execution:

- questions: for each question the user asked, the cheapest fidelity that answers it
  (thermo, steady, transient, field, olga) and the candidate that answers it. `choose`
  refuses a path that does not answer a primary question.
- basis: composition and consistency for thermo/steady; flow, diameter, pressure drop,
  composition and consistency for transport/transient/field/solver. Record each check with
  azoth_plan basis and its evidence; a failed check trips the path.
- solver feasibility, before a solver step: dt_s, cost_per_step_ms, inversion
  conditioning, the singular closures (dP -> 0, beta -> 0/1), estimated minutes, the
  fallback and the success test. A step also names the question it answers and a time box;
  an estimate over the box is refused unless accept_overrun is recorded.

steps carry a kind, a time box and a success test. A step over its box holds the next
calculation: extend it or start its fallback with azoth_plan review. After
review_after_minutes or review_after_calcs, review the chosen path with at_risk,
remaining_minutes and evidence; confirm resets the checkpoint, reconsider reopens it.

Record every failed attempt with azoth_plan fail: symptom, evidence, diagnosis, artefact.
Three failures with the same signature on a step or candidate trip the path, because three
"bugs" read together are usually one wrong formulation. Do not fix the next symptom until
the ledger says the formulation is not the problem.

When the user supplies an expected value, record it with azoth_plan target and compare at
least two models against it with azoth_plan compare, one agreeing and one disagreeing. The
spread is the result; a target met by one model choice is not a validated fluid.

A long case goes to a background job; record it with azoth_plan job and read it with
job_output/job_list rather than sleeping on it.

Automatic checks cover numerical, validity, intake-evidence and external-command failures.
For any other reason call azoth_plan trip. A trip holds the calculation and returns to its
decision; a root trip returns the whole plan to concept selection. Do not route around the
hold: status, then reopen and choose again, or resolve with evidence. Never present a held
branch's number as a conclusion.
