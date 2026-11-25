# Working with azoth

azoth is a chemical-engineering calculation library with an agentic layer on top.

- **Skills** (90) live in [`skills/`](skills/README.md): self-contained specialisms,
  each a `SKILL.md`. To load them into a tool, run `python tools/export_skills.py
  --target <dsh|claude|opencode>`; the default, `all`, writes all three.
- **Orchestration** is in [`agents/README.md`](agents/README.md). The one agent is a
  HAZOP team of four roles, defined in `agents/hazop/README.md` and exported to
  `.claude/agents/` and `.opencode/agents/`; run `/hazop` to start a study.

Setup, from a checkout:

```bash
pip install '.[agent]'          # optional: the SDK's bundled runtime, when dsh is not on PATH
python tools/launch_dsh.py --check --flowsheet specs/flowsheets/demo.toml
python tools/launch_dsh.py web --flowsheet specs/flowsheets/demo.toml
python tools/launch_dsh.py headless --study hazop --flowsheet F "Run a HAZOP on F"
```

The launcher exports the skills, writes the DSH overlay, and mounts the calculation plugin
and the flowsheet MCP server. The
[DeepSeek Harness chapter](docs/src/agentic/harness.md) is the interface reference — modes,
flags, the tools the model is handed, the bridge's JSON envelopes and the check report —
and [`agents/deepseek/README.md`](agents/deepseek/README.md) has the setup and caveats.

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
