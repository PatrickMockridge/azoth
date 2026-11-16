#!/usr/bin/env python3
"""Generate the safety catalogue a front end reads.

`specs/models/process/` declares every safety procedure, and the family is already written
there: each spec's `[source] standard` is a class under `NeqSim process/safety/<family>/`, so
the tree has one place that says what family an id belongs to. This tool projects every such
spec into `crates/azoth-safety/src/forms_gen.rs`.

**The scenario form is the calculation book's own shape.** The five ids this tranche can run
carry a `CalcForm` built from the model spec's inputs and outputs with `gen_sheets`'s emitters,
because the editor should draw a field the same way in both documents. A model spec has no
`summary`, so the five one-line purposes below are authored here - the first and only copy -
and a native id without one stops the generator rather than reaching the browser blank.

    python tools/gen_safety.py            # write
    python tools/gen_safety.py --check    # fail if the tree is not what this emits
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import gen_sheets

# The generated Rust is one long template line per field; wrapping it would make the emitted
# code harder to read than the line it replaces.
# ruff: noqa: E501

ROOT = Path(__file__).resolve().parent.parent
MODEL_DIR = ROOT / "specs" / "models"
OUT = ROOT / "crates" / "azoth-safety" / "src" / "forms_gen.rs"

#: The source-path prefix every ported safety class carries, and the family segment inside it.
SAFETY = "NeqSim process/safety/"
FAMILY = re.compile(r"process/safety/([A-Za-z0-9_]+)/")

#: **Two relief ids the upstream tree files outside `process/safety/`.** `ReliefValveSizing`
#: lives in `process/util/fire/`, but both ids are overpressure screens on this surface; naming
#: them here is the one editorial fact the source path cannot state.
EXTRA_FAMILIES = {
    "process.psv_sizing": "overpressure",
    "process.psv_fire_sizing": "overpressure",
}

#: The ids a scenario can run in this tranche. `api537_flare_flame_model` is in the fire family
#: but takes a record list, which a scenario does not yet store.
NATIVE_SCENARIOS = (
    "process.bleve",
    "process.jet_fire",
    "process.pfp_demand",
    "process.pool_fire",
    "process.vce",
)

#: The one native id that is a sub-interface rather than a scenario.
BOWTIE = "process.bow_tie"

#: The purpose line a native entry's card and form show. Model specs have no summary, so this
#: is the only copy and a native id without one is a generator error.
PURPOSES = {
    BOWTIE: "Threat frequencies, prevention-barrier PFDs and the resulting mitigated frequency and RRF.",
    "process.bleve": "The BLEVE fireball's size, duration and the distance to a target flux.",
    "process.jet_fire": "Radiant flux at a distance, and the distance to a target flux, from a jet fire.",
    "process.pfp_demand": "The time a steel member survives a fire unprotected, and the protection a required survival time demands.",
    "process.pool_fire": "Pool-fire flame height, surface emissive power and the distance to a target flux.",
    "process.vce": "The TNO multi-energy overpressure and the distance to a target overpressure.",
}

#: A refusal the entry's own shape earns rather than the tranche's scope.
REFUSALS = {
    "process.api537_flare_flame_model": "this calculation takes a table of rows as input, which a scenario does not yet provide",
}


def documents() -> list[tuple[Path, dict]]:
    out = []
    for path in sorted(MODEL_DIR.rglob("*.toml")):
        spec = tomllib.loads(path.read_text(encoding="utf-8"))
        source = str((spec.get("source") or {}).get("standard", ""))
        if source.startswith(SAFETY) or spec["id"] in EXTRA_FAMILIES:
            out.append((path, spec))
    out.sort(key=lambda pair: pair[1]["id"])
    return out


def family_of(spec: dict) -> str:
    if spec["id"] in EXTRA_FAMILIES:
        return EXTRA_FAMILIES[spec["id"]]
    source = str(spec["source"]["standard"])
    match = FAMILY.search(source)
    if match is None:
        raise SystemExit(f"gen_safety: {spec['id']}'s source has no family segment: {source}")
    return match.group(1)


def sub_interface_of(spec: dict, family: str) -> str | None:
    if spec["id"] == BOWTIE:
        return "bowtie"
    if family == "fire":
        return "fire"
    return None


def native_of(spec: dict) -> bool:
    return spec["id"] == BOWTIE or spec["id"] in NATIVE_SCENARIOS


def purpose_of(spec: dict) -> str:
    purpose = " ".join(PURPOSES.get(spec["id"], "").split())
    if native_of(spec) and purpose == "":
        raise SystemExit(f"gen_safety: the native id {spec['id']} has no purpose")
    if "neqsim" in purpose.lower():
        raise SystemExit(f"gen_safety: {spec['id']}'s purpose names the upstream implementation")
    return purpose


def emit_form(spec: dict) -> str:
    """The scenario form, built with the calculation book's own emitters."""
    if gen_sheets.complex_inputs(spec) or gen_sheets.complex_outputs(spec):
        raise SystemExit(
            f"gen_safety: the native id {spec['id']} has a table-shaped input or output; "
            "a scenario stores scalar inputs only"
        )
    ins = ", ".join(gen_sheets.emit_input(entry) for entry in gen_sheets.inputs(spec))
    outs = ", ".join(gen_sheets.emit_output(entry) for entry in gen_sheets.outputs(spec))
    assumptions = gen_sheets.clean_list([str(value) for value in spec.get("assumptions", [])])
    references = gen_sheets.clean_list([str(value) for value in spec.get("references", [])])
    return (
        "Some(&CalcForm { "
        f"id: {gen_sheets.rust_str(spec['id'])}, "
        f"name: {gen_sheets.rust_str(gen_sheets.name_of(spec))}, "
        f"summary: {gen_sheets.rust_str(purpose_of(spec))}, "
        'namespace: "process", available: true, refusal: None, equation: None, '
        f"assumptions: {gen_sheets.strings(assumptions)}, "
        f"references: {gen_sheets.strings(references)}, "
        f"inputs: &[{ins}], outputs: &[{outs}], "
        "worked_example: &[], expected: &[], tolerance: 1.0e-12"
        " })"
    )


def emit_entry(spec: dict) -> str:
    family = family_of(spec)
    sub_interface = sub_interface_of(spec, family)
    native = native_of(spec)
    refusal = REFUSALS.get(spec["id"])
    if spec["id"] in NATIVE_SCENARIOS:
        form = emit_form(spec)
    else:
        form = "None"
    return (
        "    SafetyCalculation { "
        f"id: {gen_sheets.rust_str(spec['id'])}, "
        f"name: {gen_sheets.rust_str(gen_sheets.name_of(spec))}, "
        f"family: {gen_sheets.rust_str(family)}, "
        f"sub_interface: {gen_sheets.opt_str(sub_interface)}, "
        f"purpose: {gen_sheets.rust_str(purpose_of(spec))}, "
        f"native: {'true' if native else 'false'}, "
        f"refusal: {gen_sheets.opt_str(refusal)}, "
        f"form: {form} "
        "},"
    )


def render() -> str:
    docs = documents()
    if not docs:
        raise SystemExit(f"gen_safety: no safety models under {MODEL_DIR}")
    present = {spec["id"] for _, spec in docs}
    for wanted in (*NATIVE_SCENARIOS, BOWTIE):
        if wanted not in present:
            raise SystemExit(f"gen_safety: the native id {wanted} has no model spec")
    body = "\n".join(emit_entry(spec) for _, spec in docs)
    names = ("CalcForm", "CalcInput", "CalcOutput", "CalcRange", "ExampleValue")
    used = ", ".join(name for name in names if f"{name} {{" in body)
    return f"""//! GENERATED FILE - DO NOT EDIT BY HAND.
//!
//! Generated by `tools/gen_safety.py` from `specs/models/process/`: every model whose
//! `[source] standard` is under `NeqSim process/safety/`, its family taken from that path.
//! Regenerate with `python tools/gen_safety.py`; CI runs `--check` and fails on a difference.
//!
//! **A UI projection, not a second spec.** The five scenario forms reuse the calculation
//! book's shapes; prose that names the upstream implementation is dropped.

// A form's bounds are the spec's own numbers, and clippy's approximation lint is about code
// that meant a constant rather than a value a spec pinned.
#![allow(clippy::approx_constant)]

use crate::catalogue::SafetyCalculation;
use azoth_sheets::form::{{{used}}};

/// Every safety calculation, in id order.
pub static CALCULATIONS: &[SafetyCalculation] = &[
{body}
];
"""


def main() -> None:
    text = gen_sheets.formatted(render())
    if "--check" in sys.argv:
        current = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if current != text:
            raise SystemExit("gen_safety: forms_gen.rs is out of date")
        print("gen_safety: forms_gen.rs is current")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"gen_safety: wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
