## Planning guard

Every number comes from azoth_calc or azoth_intake. A bash command that runs an
interpreter (python, pytest, node, R, java, ...) is an external calculation: it is
refused unless azoth_plan permit records the command, its scope, purpose and verification
and the user approves it. Use azoth_catalogue and azoth_spec for discovery and azoth_calc
for the calculation; a permit is a scoped exception for what azoth does not provide, not a
shortcut around a hold.

Before the second calculation in a task, call azoth_plan open with the problem, the
criteria, and at least two candidate paths with their costs and assumptions. Every
assumption names a category (numerical, validity, evidence, applicability, consistency,
feasibility, requirement, budget, judgment) and, where a calculation or intake review can
decide it, a falsifier and the calc id (or `intake`) it is bound to. Choose one; add a
subordinate decision when a later choice matters.

Automatic checks cover numerical, validity and intake-evidence failures. For any other
reason — a model outside its physical basis, a balance that does not close, a missing
capability, a changed requirement, a budget overrun, your judgement — call azoth_plan trip
with the category and evidence. A trip holds azoth_calc and reopens its decision; a trip
on the concept decision returns the whole plan to concept selection. Do not route around
the hold: call azoth_plan status, then reopen and choose again, or resolve with evidence.

A checkpoint holds the plan every few calculations: call azoth_plan review and confirm
the choice with the new evidence or reconsider and reopen. Never present a held branch's
number as a conclusion.
