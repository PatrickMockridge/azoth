"""The provenance block's own arithmetic.

The generated half is data and is checked where it is generated. What is tested here is
the merge: which warnings become ``skipped_checks``, in what order, and what the block
says when a call was clean. Those are the parts a reader relies on to tell "checked and
fine" from "never checked", so they are worth pinning directly rather than only through
a calculation that happens to exercise them.
"""

from __future__ import annotations

import importlib
import re
import sys
import tomllib
from pathlib import Path
from types import ModuleType
from typing import Any

import pytest

from azoth._provenance_gen import PROVENANCE, provenance
from azoth.core.provenance import FRAMEWORK_DOCUMENT_KEYS, PROVENANCE_KEY, Provenance
from azoth.core.warnings import Warning, WarningCode

REPO_ROOT = Path(__file__).resolve().parents[2]
TOOLS = REPO_ROOT / "tools"

#: A SHA-256, lowercase hex. A hash that is not this is not a hash.
_SHA256 = re.compile(r"^[0-9a-f]{64}$")


def provenance_tool() -> ModuleType:
    """`tools/provenance.py`, imported by name - the tools are not a package."""
    sys.path.insert(0, str(TOOLS))
    try:
        return importlib.import_module("provenance")
    finally:
        sys.path.pop(0)


STATIC: dict[str, Any] = {
    "calc_id": "hydraulics.darcy_weisbach",
    "name": "Darcy-Weisbach pressure drop",
    "spec": {"path": "specs/calcs/hydraulics/darcy_weisbach.toml", "sha256": "a" * 64},
    "code": [
        {"path": "python/src/azoth/hydraulics/reference/darcy_weisbach.py", "sha256": "b" * 64},
        {"path": "crates/azoth-hydraulics/src/darcy_weisbach.rs", "sha256": "c" * 64},
    ],
    "source": "Crane TP-410",
    "verification": "source_needed",
    "validation_cases": 1,
    "tests_active": 2,
    "tests_skipped": 2,
}


def warning(code: WarningCode, field: str | None = None) -> Warning:
    return Warning(code=code, message="a message", field=field)


def test_a_clean_call_says_so_and_names_no_skipped_check() -> None:
    block = Provenance.of(STATIC, [])

    assert block.clean
    assert block.skipped_checks == ()
    assert block.warning_codes == ()


def test_the_static_half_passes_through_unchanged() -> None:
    block = Provenance.of(STATIC, [])

    assert block.calc_id == STATIC["calc_id"]
    assert block.spec_sha256 == "a" * 64
    assert block.rust_path == STATIC["code"][1]["path"]
    assert block.verification == "source_needed"
    assert (block.validation_cases, block.tests_active, block.tests_skipped) == (1, 2, 2)


def test_only_a_skipped_check_naming_a_field_becomes_a_skipped_check() -> None:
    """Three exclusions in one case, because they are the same rule read three ways.

    A warning whose code does not mean "a check did not run" is not a skipped check; a
    warning that names no field cannot go in a tuple of field names; and two warnings
    about the same field are one skipped check, not two.
    """
    block = Provenance.of(
        STATIC,
        [
            warning(WarningCode.RANGE_CHECK_SKIPPED, "re"),
            warning(WarningCode.RANGE_CHECK_SKIPPED, "re"),
            warning(WarningCode.RANGE_CHECK_SKIPPED, None),
            warning(WarningCode.OUT_OF_VALID_RANGE, "L"),
        ],
    )

    assert block.skipped_checks == ("re",)
    assert not block.clean


def test_skipped_checks_are_ordered_by_name_and_warning_codes_by_the_vocabulary() -> None:
    """Both are ordered by something other than the order the call raised them.

    Otherwise two runs that raised the same warnings in a different order would produce
    different blocks, and the cross-language test that compares them key for key would
    be comparing the order of a warning list rather than the facts in it.
    """
    block = Provenance.of(
        STATIC,
        [
            warning(WarningCode.OUT_OF_VALID_RANGE, "v"),
            warning(WarningCode.RANGE_CHECK_SKIPPED, "v"),
            warning(WarningCode.RANGE_CHECK_SKIPPED, "D"),
        ],
    )

    assert block.skipped_checks == ("D", "v")
    assert block.warning_codes == ("OUT_OF_VALID_RANGE", "RANGE_CHECK_SKIPPED")


def test_the_document_key_is_exported_rather_than_repeated() -> None:
    """The key a result document carries the block under, and the list of such keys.

    The test that asserts a result's JSON shape needs both, and it must not hold its own
    copy: a second list is a second thing to keep in step.
    """
    assert PROVENANCE_KEY == "provenance"
    assert FRAMEWORK_DOCUMENT_KEYS == (PROVENANCE_KEY,)


def test_the_block_exists_for_every_registered_id() -> None:
    """Every calc and every model, with no id left able to produce a bare number.

    **The count is a ratchet and not a restatement**, which is why it is a number here rather than
    read from the registry: the equality above already says the two sets agree, and this says the
    set is the size it was when someone last looked. An id that leaves the registry without leaving
    a trace is otherwise a quiet subtraction.
    """
    from azoth._models_gen import MODELS
    from azoth._registry_gen import CALCS

    registered = {entry["id"] for entry in [*CALCS, *MODELS]}

    assert set(PROVENANCE) == registered
    # 260 -> 262: the sound speed and the map machine (2026-10-28); 263: the
    # equilibrium sound speed; 264: the homogeneous-equilibrium release; 265: the
    # vessel blowdown; 266: the separator capacity assessment; 267: the anti-surge
    # recycle calculator; 268: the separator performance calculator; 269: the
    # shell-and-tube thermal-hydraulic rating; 270: the tube-vibration screen; 271: the
    # LMTD correction factor; 272: the Shah condensation correlations; 273: the boiling
    # correlations; 274: the two-phase pressure drop; 275: the fouling model; 276: the tube-insert
    # enhancement; 277: the TEMA standard; 278: the zone-by-zone rating; 279: the
    # incremental zone analysis; 280: the detailed TEMA design; 281: the gas dispersion
    # analyzer; 282: the Gaussian plume; 283: the probit; 284: the jet fire; 285: the pool
    # fire; 286: the vapour-cloud explosion; 287: the BLEVE; 288: the event tree; 289: the
    # consequence analysis; 290: the passive fire protection demand; 291: the
    # fire-water demand; 292: the deluge nozzle layout; 293: the fire-monitor
    # coverage; 294: the fire-water coverage assessment; 295: the active fire
    # protection screening; 296: the API 537 flare flame; 297: the fault tree; 298: the
    # bow tie; 299: the safety instrumented function; 300: the layer of protection
    # analysis; 301: the SIL verification; 302: the NOG 070 SIL determination; 303: the
    # risk matrix; 304: the PSV sizing; 305: the dynamic fire sizing; 306: the fire-case
    # relief; 307: the overpressure acceptance; 308: the pipeline pressure protection;
    # 309: the relief disposal network; 310: the TR3001 compliance; 311: the dense-gas
    # dispersion; 312: the hazardous area; 313: the toxic thresholds; 314: the
    # blowdown pressure profile; 315: the pipe material curves; 316: the pipe fire
    # rupture; 317: the vessel rupture; 318: the blocked-outlet relief load; 319: the
    # control-valve failure load; 320: the check-valve leak load; 321: the tube-rupture
    # load; 322: the overpressure protection study; 323-326: the self-heating family;
    # 327: the HAZOP template; 328: the FMEA worksheet; 329: the consequence population;
    # 330: the MAH bow-tie skeleton; 331: the API 14C SAFE chart; 332: the barrier register
    # validation; 333: the S-001 Clause 10 review; 334: the S-001 secondary-pressure screen;
    # 335: the gas blowby; 336: the pump deadhead; 337: the runaway reaction; 338: the
    # MDMT; 339: the vacuum collapse; 340: the settle-out pressure; 341-342: the gas- and
    # liquid-line FIV screens; 343: the acoustic-induced vibration screen; 344: the
    # side-branch pulsation screen; 345: the NORSOK P-002 point checks; 346: the STS-0131
    # gate; 347: the inherent-safety index; 348: the ALARP audit; 349: the escalation graph;
    # 350: the PSV latch; 351: the dynamic PSV sizing; 352: the vessel filling;
    # 353: the depressurization; 354: the depressurization acceptance; 355: the
    # blocked-outlet overpressure; 356: the multi-vessel study; 357: the non-equilibrium
    # blowdown; 358: the ideal-gas release; 359: the legacy screening release; 360: the
    # prescribed-slip release; 361: the drift-flux release; 362: the finite-rate release;
    # 363: the component-selective release; 364: the Ranz-Marshall release; 365: the
    # bubble transfer zone; 366: the ideal-gas Fanno pipe; 367: the real-gas Fanno pipe;
    # 368: the ESD response time; 369: the mass-balance leak sensitivity;
    # 370: the trapped inventory; 371: the safety envelope; 372: the trapped-liquid
    # fire rupture; 373: the open-drain review; 374: the safety-system performance;
    # 375: the cost-estimation calculator; 376: the equipment cost estimate; 377: the process
    # cost estimate; 378: the topsides facility cost; 379: the black-oil correlation table;
    # 380: the gas correlation table; 381: the water correlation table; 382: the decline curve;
    # 383: the gas material balance; 384: the oil material balance; 385: the saturation
    # test; 386: the constant mass expansion; 387: the differential liberation; 388: the
    # constant volume depletion; 389: the separator test; 390: the swelling test; 391: the
    # minimum miscibility pressure; 392: the property sweep; 393: the PVT regression;
    # 394: the plus-molar-mass saturation tuning; 395: the Van Everdingen-Hurst aquifer;
    # 396: the Fetkovich aquifer; 397: the generalized material balance; 398: the relative
    # permeability table; 399: the CVD material balance; 400: the gas-density consistency;
    # 401: the separator path; 402: the transient drift-flux pipe.
    assert len(registered) == 402


def test_the_rust_table_carries_exactly_the_same_ids() -> None:
    """Two generated files, one list - checked, because they are two files.

    Both are emitted from the same blocks, so they agree unless a language's emitter drops
    one. Nothing else would notice: the Rust side compiles with a short table and the
    Python side passes with a long one, and the id that fell out would be found only by a
    caller who happened to ask for it.
    """
    text = (REPO_ROOT / "crates" / "azoth-core" / "src" / "provenance_gen.rs").read_text(
        encoding="utf-8"
    )
    rust_ids = re.findall(r'^\s*calc_id: "([^"]+)",$', text, flags=re.MULTILINE)

    assert len(rust_ids) == len(PROVENANCE), f"{len(rust_ids)} vs {len(PROVENANCE)}"
    assert set(rust_ids) == set(PROVENANCE)


def test_an_unknown_id_is_refused_rather_than_answered_with_an_empty_block() -> None:
    """The same rule the databank follows for a substance it cannot name.

    A block that existed but said nothing would be worse than an error: it would let a
    number be presented as if it carried its provenance when it carried a placeholder.
    """
    with pytest.raises(KeyError, match="no provenance for"):
        provenance("hydraulics.not_a_calculation")


def test_every_block_is_complete_and_well_formed() -> None:
    """The shape, for all 269, so the contract is the block's rather than one sample's."""
    for calc_id, block in PROVENANCE.items():
        assert set(block) == {
            "calc_id",
            "name",
            "spec",
            "code",
            "source",
            "verification",
            "validation_cases",
            "tests_active",
            "tests_skipped",
        }, calc_id
        assert block["calc_id"] == calc_id, calc_id
        assert block["name"], calc_id
        assert block["source"], f"{calc_id} names no source"
        assert block["verification"] in provenance_tool().VERIFICATION_STATUSES, calc_id
        assert len(block["code"]) == 2, f"{calc_id} does not name both implementations"


def test_every_embedded_hash_is_the_hash_of_the_file_it_names() -> None:
    """**The claim the block makes, checked rather than asserted.**

    A block whose hash was of something else would be worse than no hash at all: it
    would let a reader believe they had identified the code that produced a number when
    they had identified nothing. The hash is only worth carrying because it can be
    compared, so this is the test that makes it a fact about the tree.
    """
    provenance = provenance_tool()
    for calc_id, block in PROVENANCE.items():
        named = [block["spec"], *block["code"]]
        for item in named:
            assert _SHA256.match(item["sha256"]), f"{calc_id}: {item['sha256']!r} is not a hash"
            actual = provenance.sha256_of(REPO_ROOT / item["path"])
            assert actual == item["sha256"], (
                f"{calc_id}: {item['path']} is recorded as {item['sha256'][:12]} "
                f"but hashes to {actual[:12]} - regenerate with `tools/gen_registry.py`"
            )


def test_the_recorded_status_is_the_one_the_tree_derives() -> None:
    """The status is recomputed from the same evidence rather than trusted.

    Called through the tool's own `derive_verification` rather than by restating the
    cascade here, so a change to the rule cannot leave this test passing against a table
    built by the previous one.
    """
    tool = provenance_tool()
    index = tool.validation_cases()
    for tree, entry_of in (
        ("specs/calcs", tool.calc_entry),
        ("specs/models", tool.model_entry),
    ):
        for path in sorted((REPO_ROOT / tree).rglob("*.toml")):
            spec = tomllib.loads(path.read_text(encoding="utf-8"))
            record = entry_of(spec, index)
            block = PROVENANCE[spec["id"]]
            assert block["verification"] == record["verification"], spec["id"]
            assert block["validation_cases"] == record["validation_cases"], spec["id"]
            assert block["tests_active"] == record["tests_active"], spec["id"]
            assert block["tests_skipped"] == record["tests_skipped"], spec["id"]
