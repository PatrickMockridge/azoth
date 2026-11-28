# Working with azoth

azoth is a chemical-engineering calculation library with an agentic layer on top.

- **Skills** (90) live in [`skills/`](skills/README.md): self-contained specialisms,
  each a `SKILL.md`. To load them into a tool, run `python tools/export_skills.py
  --target <dsh|claude|opencode>`; the default, `all`, writes all three.
- **Orchestration** is in [`agents/README.md`](agents/README.md). Two agents run: a HAZOP
  team of four roles (`agents/hazop/README.md`) and an intake team of four roles
  (`agents/intake/README.md`), exported to `.claude/agents/` and `.opencode/agents/`;
  run `/hazop` or `/intake` to start one.

Setup, from a checkout:

```bash
pip install '.[agent]'          # optional: the SDK's bundled runtime, when dsh is not on PATH
python tools/launch_dsh.py --check
python tools/launch_dsh.py web --permission-mode read-only
python tools/launch_dsh.py headless --permission-mode read-only "Size a pump for 100 m³/h"
```

The launcher exports the skills, pins the checkout as DSH's workspace, loads the demo
flowsheet, and mounts the engineering persona, the calculation plugin and the flowsheet MCP
server; `--persona software` uses DSH's coding preset instead. It mounts DeepSeek, OpenAI,
Anthropic, Kimi and MiniMax routes for the model selector; `--provider` sets the default
and `--model` the id. The
[DeepSeek Harness chapter](docs/src/agentic/harness.md) is the interface reference — modes,
flags, the tools the model is handed, the bridge's JSON envelopes and the check report —
and [`agents/deepseek/README.md`](agents/deepseek/README.md) has the setup and caveats.

## The port law

azoth is a port of NeqSim, and that is the admission rule for every change. The
specification states it (`docs/src/architecture/specification.md`) and
[`skills/library/port-a-calculation`](skills/library/port-a-calculation/SKILL.md) repeats
it; this section is the version an agent reads before it writes a plan.

- **Every value names its source.** Every id, equation, input, output, algorithm parameter
  and constant declares a NeqSim class and member at the pinned commit, a publication
  NeqSim itself cites, or `caller` for a value the caller supplies. "azoth chose it" is not
  a source. The spec carries it in `source.neqsim` and `[provenance]`, and
  `tools/check_port.py` refuses a spec that does not.
- **No id without a NeqSim class.** A diagnostic variant, an aggregate, a convenience
  model or a screening label is not a port. If one is wanted it is an explicit extension
  approved before code is written; the default answer is no.
- **A study is validation, not specification.** No constant, grid, tolerance or step is
  changed to make a case study's number pass. A plan that cites a study target as the
  design input is refused; a test may cite a study as evidence, never as the specification.
- **Stop before writing the line.** The moment a proposed id, equation or value has no
  NeqSim source, stop and report it. Finishing the tranche and disclosing the deviation
  afterwards is the failure this rule exists to prevent.
- **The plan is a provenance table.** Before code, a plan lists every proposed id,
  equation, input, output and constant with its NeqSim class, member and commit or its
  citation; its "not a port" list is empty; and it states that any case study is
  validation only. A plan without the table is refused.

Where a port composes several NeqSim classes, `source.neqsim.members` names each class and
method it reads. The databank pin is `databank/manifest.toml`; a capture's pin is the jar
in its driver header. A result states which pin it was verified against.

## Executing a port

The law says what may enter a port. This section says the order that keeps a large one from
being discovered one failing comparison at a time. It is not advice; a port over roughly 500
lines that skips it is refused.

### Before code: the behaviour table

The provenance table answers *where each value comes from*. A port also needs a table of
*what the class actually does*, because the units, volumes, phase labels and post-run
mutations are where a faithful transcription diverges. The plan carries one row per item:

- every state variable and its primitive/conservative form;
- every phase property the algorithm reads, and the getter it reads it from;
- the **volume convention** for each use: shifted or unshifted, molar or mass, and which
  getter applies which (NeqSim's `getDensity()` and `getVolume()` do not agree);
- every unit conversion inside the class, including the factors in its own expressions;
- every phase label, mole/mass/volume fraction, and which of the three the class uses;
- the flash dispatch and the flag that selects it;
- every getter an output reads, **and every mutation the class makes after it stores the
  output** (a section rewritten by `updateOutletStream()` after `storeResults()` is invisible
  in the stored profile and visible in `getSections()`; the port must know which one it
  exposes);
- every default, cap, floor and fallback, with its member.

If a row cannot be filled from the class source, that is the stop-and-report rule, not a
guess. The 2026-11 `TransientPipe` port needed six corrections that this table would have
carried from the start.

### One probe, all the state

Write the probe before the first kernel and make it print everything the comparison could
need, at full precision: the input echo, the initial and final value of every output, every
getter value, every phase property the algorithm reads, every internal field the class
mutates after storing results, the step and time counters, and the whole output matrix
including the rows the class never wrote. A probe that prints only the final profiles is
not finished. Recompiling a probe to find a convention is a cycle the first probe should
have removed.

### First kernel, then the second

Validate the first kernel's **single state and single step** against the probe before
writing the march, and get its **full case** to match before writing the second kernel. The
mirror is arithmetic, not discovery: a correction made in one language and then copied is
one edit; a correction found after both exist is two edits and two test runs. The
cross-implementation comparison is the last check, not the first.

### A new id, in dependency order

The generator chain's order is in `docs/src/architecture/specification.md`; it is necessary
and not sufficient. Before the chain can run, these must exist, in this order:

1. the spec and its case, with expected values from the capture (never from either kernel);
2. the Rust result struct, its `impl CalcResult`, and the `lib.rs` re-export — the Python
   result generator reads `FIELDS` from the Rust source;
3. the public Python wrapper in the namespace's `__init__.py`, its import and its `__all__`
   entry — the stub generator imports the namespace and looks the function up;
4. a `SELF_ASSERTED_EXPECTATIONS` entry for any expectation a `TestCase` cannot carry (a
   matrix, a boolean, a record list), with the model's own test that reads it;
5. the generator chain, then `gen_ui_fixtures.py`.

Then commit, run `--checks-only`, and push; CI runs the push lane. A close-out tranche runs
once. Rerunning the same test file while editing is the symptom of a missing behaviour table
or an unvalidated first kernel; fix the table, not the loop.

### Before blaming the diff: the base red

A scoped lane exposes whatever the diff reaches, including reds already on `main`. Before
chasing a failure, run the same scoped command at the base commit in a throwaway worktree
(`git worktree add /tmp/azoth-base origin/main`). A failure there is not this diff's: state
it in the PR body as pre-existing and leave it out of the change. The 2026-11 `Fürst`
Huron-Vidal refusal and two `clippy::question_mark` lints were on `main` and cost cycles
because they were diagnosed one at a time.

## The NeqSim oracle

azoth is a port of NeqSim, so NeqSim is the differential oracle: a disagreement with
it is the interesting failure. The 60 MB jar is **not vendored** (only NeqSim's
*data* files are, under `databank/sources/neqsim/`), and the drivers beside the
captures in `validation/neqsim/` are compiled by hand — see
[`validation/README.md`](validation/README.md). This is how to get the jar back on a
machine that has none; it needs a JDK and network, and nothing else. Measured on
2026-09-23 with JDK 21 — NeqSim's Maven wrapper fetches its own Maven.

```bash
git clone https://github.com/equinor/neqsim /tmp/neqsim-check/neqsim
cd /tmp/neqsim-check/neqsim
git checkout b40bb113                 # the commit the jar is named after
./mvnw -q -DskipTests package           # → target/neqsim-3.29.0.jar
cp target/neqsim-3.29.0.jar /path/to/azoth/validation/neqsim/neqsim-b40bb11.jar
```

**The name is the commit.** Every driver header says `-cp neqsim-b40bb11.jar`, which is
`neqsim-3.29.0.jar` built at `b40bb113` and renamed for the commit it came from — so
the captures belong to that revision, and moving to a newer NeqSim is a rename that
rewrites every driver header with it. The checkout lives in `/tmp` and is disposable:
rebuilding it is this recipe.

```bash
cd validation/neqsim
../../tools/gated.sh javac -proc:none -cp neqsim-b40bb11.jar HybridEosGeReactiveProbe.java
../../tools/gated.sh java -cp .:neqsim-b40bb11.jar HybridEosGeReactiveProbe > captures/hybrid_eos_ge_reactive_probe.tsv
```

The captures are committed and CI reads them, so a new one is a capture file plus the
numbers in the case beside it, never a live JVM in the build.
