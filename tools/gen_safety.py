#!/usr/bin/env python3
"""Generate the safety catalogue a front end reads.

`specs/models/process/` declares every safety procedure, and the family is already written
there: each spec's `[source] standard` is a class under `NeqSim process/safety/<family>/`, so
the tree has one place that says what family an id belongs to. This tool projects every such
spec into `crates/azoth-safety/src/forms_gen.rs`.

**The scenario form is the calculation book's own shape.** Every id this workspace can run
carries a `CalcForm` built from the model spec's inputs and outputs with `gen_sheets`'s
emitters, because the editor should draw a field the same way in both documents. A model spec
has no `summary`, so the one-line purposes below are authored here - the first and only copy -
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
    "process.gas_blowby": "equipmentsafety",
    "process.pump_deadhead": "equipmentsafety",
    "process.runaway_reaction": "equipmentsafety",
    "process.mdmt": "equipmentsafety",
    "process.vacuum_collapse": "equipmentsafety",
    "process.settle_out_pressure": "equipmentsafety",
    # **Three small source packages are one assessment family.** `inherent/`, `alarp/` and
    # `escalation/` each hold one worksheet on the safety case rather than a family of its
    # own; the spec's `[source]` still names each package, and the grouping is id-keyed so a
    # future class under those paths is not silently pulled in.
    "process.inherent_safety": "safetyassessment",
    "process.alarp_audit": "safetyassessment",
    "process.escalation_graph": "safetyassessment",
}

#: The ids a scenario can run: every safety model whose inputs and outputs are all scalar,
#: plus the five fire models. A model with a table-shaped input or output is listed with the
#: refusal `refusal_of` derives from that shape.
NATIVE_SCENARIOS = (
    "process.acoustic_induced_vibration_screening",
    "process.active_fire_protection_screening",
    "process.bleve",
    "process.deluge_nozzle_layout",
    "process.fire_case_relief",
    "process.fire_monitor_coverage",
    "process.fire_water_coverage_assessment",
    "process.gas_blowby",
    "process.gaussian_plume",
    "process.hazardous_area",
    "process.heavy_gas_dispersion",
    "process.inherent_safety",
    "process.jet_fire",
    "process.mdmt",
    "process.nog070_sil_determination",
    "process.overpressure_acceptance",
    "process.pfp_demand",
    "process.pipeline_pressure_protection",
    "process.piping_fiv_gas",
    "process.piping_fiv_liquid",
    "process.pool_fire",
    "process.probit",
    "process.psv_fire_sizing",
    "process.psv_sizing",
    "process.psv_valve",
    "process.pump_deadhead",
    "process.runaway_reaction",
    "process.s001_secondary_pressure_protection",
    "process.safety_instrumented_function",
    "process.toxic_library",
    "process.vce",
)

#: The one native id that is a sub-interface rather than a scenario.
BOWTIE = "process.bow_tie"

#: The family registry: the one place a family's label, its deck copy and its blurb are
#: written. The order is the tab order, then the listed families alphabetically. A deck is
#: the copy the workspace draws; the families without one are listed in the catalogue and
#: linked to their manual pages.
#:
#: **The family set is the specs', and this table must cover it exactly.** A family the
#: specs name and this table omits would be a model with nowhere to appear; one this table
#: names and the specs do not is dead copy. `render` refuses either rather than drawing a
#: partial surface.
FAMILIES = (
    {
        "id": "depressurization",
        "label": "Depressurization",
        "blurb": "Depressurization and blowdown, the vessel filling and blocked-outlet overpressure screens.",
        "deck": {
            "tab": "Depressurization",
            "title": "Depressurization models",
            "heading": "Depressurization scenarios",
            "search": "Search depressurization models",
            "step": "Add a pressure-safety-valve step, a vessel filling or a blocked-outlet overpressure screen.",
        },
    },
    {
        "id": "dispersion",
        "label": "Dispersion",
        "blurb": "Gaussian plume, hazardous area, dense gas, probit and toxic thresholds.",
        "deck": {
            "tab": "Dispersion",
            "title": "Dispersion models",
            "heading": "Dispersion scenarios",
            "search": "Search dispersion models",
            "step": "Add a plume, a hazardous-area screen, a dense-gas distance, a probit or a toxic-threshold lookup.",
        },
    },
    {
        "id": "equipmentsafety",
        "label": "Equipment and system safety",
        "blurb": "Gas blowby, pump deadhead, runaway reaction and minimum design metal temperature.",
        "deck": {
            "tab": "Equipment safety",
            "title": "Equipment safety models",
            "heading": "Equipment safety scenarios",
            "search": "Search equipment safety models",
            "step": "Add a gas blowby, a pump deadhead, a runaway reaction or a minimum design metal temperature.",
        },
    },
    {
        "id": "fire",
        "label": "Fire and explosion",
        "blurb": "Jet fire, pool fire, BLEVE, vapour-cloud explosion and passive fire protection.",
        "deck": {
            "tab": "Fire",
            "title": "Fire models",
            "heading": "Fire scenarios",
            "search": "Search fire models",
            "step": "Add a jet fire, a pool fire, a BLEVE, a vapour-cloud explosion or a protection demand.",
        },
    },
    {
        "id": "firewater",
        "label": "Firewater",
        "blurb": "Protection screening, deluge layout, monitor coverage and supply gates.",
        "deck": {
            "tab": "Firewater",
            "title": "Firewater models",
            "heading": "Firewater scenarios",
            "search": "Search firewater models",
            "step": "Add a protection screen, a deluge layout, a monitor coverage or a supply assessment.",
        },
    },
    {
        "id": "overpressure",
        "label": "Overpressure and relief",
        "blurb": "Fire-case relief, relief-valve sizing, acceptance and pipeline protection.",
        "deck": {
            "tab": "Relief",
            "title": "Relief models",
            "heading": "Relief scenarios",
            "search": "Search relief models",
            "step": "Add a fire-case relief, a relief-valve sizing, an acceptance check or a pipeline protection screen.",
        },
    },
    {
        "id": "processsafetysystem",
        "label": "Process safety systems",
        "blurb": "The NORSOK S-001 secondary-pressure protection screen.",
        "deck": {
            "tab": "Process safety",
            "title": "Process safety system models",
            "heading": "Process safety system scenarios",
            "search": "Search process safety system models",
            "step": "Add a NORSOK S-001 secondary-pressure protection screen.",
        },
    },
    {
        "id": "risk",
        "label": "Risk and SIL",
        "blurb": "NOG 070 minimum SIL and the instrumented function's PFD arithmetic.",
        "deck": {
            "tab": "Risk",
            "title": "Risk and SIL models",
            "heading": "Risk scenarios",
            "search": "Search risk models",
            "step": "Add a NOG 070 determination or an instrumented-function assessment.",
        },
    },
    {
        "id": "safetyassessment",
        "label": "Safety assessment",
        "blurb": "The Kletz inherent-safety index.",
        "deck": {
            "tab": "Assessment",
            "title": "Safety assessment models",
            "heading": "Safety assessment scenarios",
            "search": "Search safety assessment models",
            "step": "Add an inherent-safety evaluation.",
        },
    },
    {
        "id": "vibration",
        "label": "Vibration",
        "blurb": "Gas and liquid flow-induced vibration and acoustic-induced vibration.",
        "deck": {
            "tab": "Vibration",
            "title": "Vibration models",
            "heading": "Vibration scenarios",
            "search": "Search vibration models",
            "step": "Add a gas or liquid flow-induced vibration screen or an acoustic-induced vibration screen.",
        },
    },
    {
        "id": "api14c",
        "label": "API 14C",
        "blurb": "The SAFE chart's device rows and its missing-device findings.",
        "deck": None,
    },
    {
        "id": "barrier",
        "label": "Barriers",
        "blurb": "The barrier register's findings and the safety system's performance.",
        "deck": None,
    },
    {
        "id": "compliance",
        "label": "Compliance",
        "blurb": "The NORSOK P-002 and STS-0131 compliance gates.",
        "deck": None,
    },
    {
        "id": "envelope",
        "label": "Safety envelope",
        "blurb": "The process safety envelope and the deviations it reports.",
        "deck": None,
    },
    {
        "id": "esd",
        "label": "Emergency shutdown",
        "blurb": "The emergency shutdown response time against its target.",
        "deck": None,
    },
    {
        "id": "hazid",
        "label": "Hazard identification",
        "blurb": "The HAZOP, FMEA and MAH worksheets.",
        "deck": None,
    },
    {
        "id": "inventory",
        "label": "Inventory",
        "blurb": "The trapped inventory in isolated sections.",
        "deck": None,
    },
    {
        "id": "leakdetection",
        "label": "Leak detection",
        "blurb": "The mass-balance leak detection and its threshold.",
        "deck": None,
    },
    {
        "id": "opendrain",
        "label": "Open drain",
        "blurb": "The open-drain review's rows and findings.",
        "deck": None,
    },
    {
        "id": "qra",
        "label": "Quantitative risk",
        "blurb": "Consequence analysis for a quantitative risk assessment.",
        "deck": None,
    },
    {
        "id": "release",
        "label": "Release",
        "blurb": "Release-rate models from the ideal-gas forms to the finite-rate and drift-flux forms.",
        "deck": None,
    },
    {
        "id": "rupture",
        "label": "Rupture",
        "blurb": "Vessel and pipe rupture, and the trapped-liquid fire rupture.",
        "deck": None,
    },
    {
        "id": "selfheating",
        "label": "Self-heating",
        "blurb": "The Semenov, porous-media and basket-test self-heating screens.",
        "deck": None,
    },
)

#: The families with a deck of scenario cards, in tab order.
SCENARIO_FAMILIES = tuple(entry["id"] for entry in FAMILIES if entry["deck"] is not None)

#: The purpose line a native entry's card and form show. Model specs have no summary, so this
#: is the only copy and a native id without one is a generator error.
PURPOSES = {
    BOWTIE: "Threat frequencies, prevention-barrier PFDs and the resulting mitigated frequency and RRF.",
    "process.acoustic_induced_vibration_screening": "The sound power level downstream of a pressure-reducing device, the design-line allowable and the screening margin.",
    "process.inherent_safety": "The four Kletz pillar scores and their average inherent-safety index.",
    "process.piping_fiv_gas": "The flow-induced-vibration likelihood of failure for a gas line from its momentum flux, diameter ratio, branch count, pulsation source and support quality.",
    "process.piping_fiv_liquid": "The flow-induced-vibration likelihood of failure for a liquid line from its velocity, diameter ratio, branch count and support quality.",
    "process.active_fire_protection_screening": "Whether water can extinguish the fire, the blowdown margin over the steel's failure and the selective-protection saving.",
    "process.bleve": "The BLEVE fireball's size, duration and the distance to a target flux.",
    "process.deluge_nozzle_layout": "The deluge nozzle count from the flow and coverage criteria, the grid spacing and the delivered density.",
    "process.fire_case_relief": "The fire-case relief rate from the wetted area or vessel geometry, the heat input and the latent heat.",
    "process.fire_monitor_coverage": "The monitor flow, swept area, wind drift and the effective density against its requirement.",
    "process.fire_water_coverage_assessment": "The coverage gap, flow deficit and the supply adequacy verdict against the required pressure margin.",
    "process.gas_blowby": "The choked or subcritical blowby rate through an open restriction and whether a stated relief capacity absorbs it.",
    "process.gaussian_plume": "Ground-level concentration at a downwind point and the distance to a target concentration, from a Gaussian plume.",
    "process.hazardous_area": "The notional source diameter, the mass-fraction LFL, the de-rated hazardous distance and the zone.",
    "process.heavy_gas_dispersion": "The signed reduced gravity, the Britter parameter and the distance to a concentration ratio.",
    "process.jet_fire": "Radiant flux at a distance, and the distance to a target flux, from a jet fire.",
    "process.nog070_sil_determination": "The achieved SIL against the NOG 070 minimum, and the PFD reduction a non-compliant SIF needs.",
    "process.overpressure_acceptance": "The accumulation factor, the allowable pressure and whether the peak is accepted.",
    "process.pfp_demand": "The time a steel member survives a fire unprotected, and the protection a required survival time demands.",
    "process.pipeline_pressure_protection": "The incidental pressure, the design margin and whether the segment is fully rated or needs two barriers.",
    "process.s001_secondary_pressure_protection": "The pressure band's default target frequency, the test and event pressure checks, the demand-versus-target test and the leakage and proof-test criteria.",
    "process.pool_fire": "Pool-fire flame height, surface emissive power and the distance to a target flux.",
    "process.probit": "The probit from a dose, or the probability at a stated probit.",
    "process.psv_fire_sizing": "The fire-case relief area, the selected API 520 orifice and the blowdown pressure.",
    "process.psv_sizing": "The API 520 gas, liquid or two-phase relief area and the selected orifice.",
    "process.psv_valve": "One step of a pressure-safety-valve latch: whether it opens or reseats, its pop count and the API 520 relieving flow.",
    "process.pump_deadhead": "The shut-off deadhead pressure against the protected rating, and the minimum-flow recirculation temperature rise.",
    "process.runaway_reaction": "The adiabatic temperature rise, MTSR, margin, time to maximum rate and the two-phase relief flag.",
    "process.mdmt": "The UCS-66 curve's minimum design metal temperature and whether the cold service is acceptable.",
    "process.vacuum_collapse": "The constant-volume cooling sweep's minimum pressure against the external rating, and the make-up gas.",
    "process.settle_out_pressure": "The settle-out pressure of connected gas volumes against the protected rating, with each compartment's inventory.",
    "process.safety_instrumented_function": "The SIF's RRF, required PFD and SIL, architecture PFDs and spurious-trip availability.",
    "process.toxic_library": "The IDLH, ERPG-2 and AEGL-2 thresholds for a chemical, and a ppm concentration converted to kg/m3.",
    "process.vce": "The TNO multi-energy overpressure and the distance to a target overpressure.",
}

#: A refusal the entry's own shape earns rather than the workspace's scope.
INPUT_REFUSAL = (
    "this calculation takes a table of rows as input, which a scenario does not yet provide"
)
FLUID_REFUSAL = "this calculation takes a fluid as input, which a scenario does not yet provide"
OUTPUT_REFUSAL = "this calculation returns a table of rows, which this workspace does not yet draw"
BOTH_REFUSAL = (
    "this calculation takes a table of rows as input and returns one as output, "
    "which a scenario does not yet provide or draw"
)


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
    # The bow-tie is in the risk family and is its own view, so this test comes first.
    if spec["id"] == BOWTIE:
        return "bowtie"
    if family in SCENARIO_FAMILIES:
        return family
    return None


def native_of(spec: dict) -> bool:
    return spec["id"] == BOWTIE or spec["id"] in NATIVE_SCENARIOS


def refusal_of(spec: dict) -> str | None:
    """Why a non-native entry cannot run, derived from the shape that stops it.

    The final branch is deliberate: a safety model that is scalar-shaped and has no arm is a
    gap to close, not an entry to list without an explanation.
    """
    if native_of(spec):
        return None
    takes = gen_sheets.complex_inputs(spec)
    returns = gen_sheets.complex_outputs(spec)
    if takes and returns:
        return BOTH_REFUSAL
    if takes:
        # A fluid is not a table, and the relief loads are the first entries whose complex
        # input is a component list rather than rows.
        kinds = {declaration.get("type") for declaration in spec["inputs"].values()}
        if "components" in kinds and "record_list" not in kinds:
            return FLUID_REFUSAL
        return INPUT_REFUSAL
    if returns:
        return OUTPUT_REFUSAL
    raise SystemExit(
        f"gen_safety: the non-native id {spec['id']} is scalar-shaped; "
        "give it a native arm and a purpose or stop listing it"
    )


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
    refusal = refusal_of(spec)
    form = emit_form(spec) if spec["id"] in NATIVE_SCENARIOS else "None"
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


def emit_family(entry: dict) -> str:
    deck = entry["deck"]
    deck_text = (
        "None"
        if deck is None
        else (
            "Some(SafetyDeck { "
            f"tab: {gen_sheets.rust_str(deck['tab'])}, "
            f"title: {gen_sheets.rust_str(deck['title'])}, "
            f"heading: {gen_sheets.rust_str(deck['heading'])}, "
            f"search: {gen_sheets.rust_str(deck['search'])}, "
            f"step: {gen_sheets.rust_str(deck['step'])}, "
            "})"
        )
    )
    return (
        "    SafetyFamily { "
        f"id: {gen_sheets.rust_str(entry['id'])}, "
        f"label: {gen_sheets.rust_str(entry['label'])}, "
        f"blurb: {gen_sheets.rust_str(entry['blurb'])}, "
        f"deck: {deck_text} "
        "},"
    )


def check_families(specs: list[dict]) -> None:
    """The registry covers the spec tree exactly, and its copy is showable."""
    named = [family_of(spec) for spec in specs]
    declared = [entry["id"] for entry in FAMILIES]
    if len(declared) != len(set(declared)):
        raise SystemExit("gen_safety: a family id appears twice in FAMILIES")
    missing = sorted(set(named) - set(declared))
    unknown = sorted(set(declared) - set(named))
    if missing or unknown:
        raise SystemExit(
            f"gen_safety: family registry mismatch: missing {missing}, unknown {unknown}"
        )
    for entry in FAMILIES:
        for field in ("id", "label", "blurb"):
            if not str(entry[field]).strip():
                raise SystemExit(f"gen_safety: the family {entry['id']} has an empty {field}")
        deck = entry["deck"]
        if deck is not None:
            for field in ("tab", "title", "heading", "search", "step"):
                if not str(deck[field]).strip():
                    raise SystemExit(
                        f"gen_safety: the deck {entry['id']} has an empty {field}"
                    )
        text = " ".join(
            str(value)
            for value in (
                entry["label"],
                entry["blurb"],
                *((deck or {}).values()),
            )
        )
        if "neqsim" in text.lower():
            raise SystemExit(f"gen_safety: the family {entry['id']} names the upstream")
    deck_families = set(SCENARIO_FAMILIES)
    for spec in specs:
        family = family_of(spec)
        if native_of(spec) and spec["id"] != BOWTIE and family not in deck_families:
            raise SystemExit(
                f"gen_safety: the native id {spec['id']} is in {family}, which has no deck"
            )
    for family in deck_families:
        if not any(native_of(spec) and family_of(spec) == family for spec in specs):
            raise SystemExit(f"gen_safety: the deck {family} has no native scenario")


def render() -> str:
    docs = documents()
    if not docs:
        raise SystemExit(f"gen_safety: no safety models under {MODEL_DIR}")
    present = {spec["id"] for _, spec in docs}
    for wanted in (*NATIVE_SCENARIOS, BOWTIE):
        if wanted not in present:
            raise SystemExit(f"gen_safety: the native id {wanted} has no model spec")
    check_families([spec for _, spec in docs])
    body = "\n".join(emit_entry(spec) for _, spec in docs)
    families = "\n".join(emit_family(entry) for entry in FAMILIES)
    names = ("CalcForm", "CalcInput", "CalcOutput", "CalcRange", "ExampleValue")
    used = ", ".join(name for name in names if f"{name} {{" in body)
    return f"""//! GENERATED FILE - DO NOT EDIT BY HAND.
//!
//! Generated by `tools/gen_safety.py` from `specs/models/process/`: every model whose
//! `[source] standard` is under `NeqSim process/safety/`, its family taken from that path,
//! and the family registry the workspace draws its tabs and catalogue from.
//! Regenerate with `python tools/gen_safety.py`; CI runs `--check` and fails on a difference.
//!
//! **A UI projection, not a second spec.** The scenario forms reuse the calculation book's
//! shapes; prose that names the upstream implementation is dropped.

// A form's bounds are the spec's own numbers, and clippy's approximation lint is about code
// that meant a constant rather than a value a spec pinned.
#![allow(clippy::approx_constant)]

use crate::catalogue::{{SafetyCalculation, SafetyDeck, SafetyFamily}};
use azoth_sheets::form::{{{used}}};

/// Every safety calculation, in id order.
pub static CALCULATIONS: &[SafetyCalculation] = &[
{body}
];

/// Every safety family, in registry order: the deck families in tab order, then the listed.
pub static FAMILIES: &[SafetyFamily] = &[
{families}
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
