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
