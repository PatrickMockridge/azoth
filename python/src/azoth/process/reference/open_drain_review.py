"""``process.open_drain_review`` - the NORSOK S-001 open-drain review.

The Python twin of ``crates/azoth-process/src/models/open_drain_review.rs``. The port
source is NeqSim's ``process/safety/opendrain/OpenDrainReviewEngine.java``: nineteen
assessments per item in the class's order, the evidence confidence and the verdict
cascade.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    OpenDrainAssessment,
    OpenDrainResult,
    OpenDrainReviewResult,
    OpenDrainVerdict,
)
from azoth.core.units import Q, input_records_to_si, input_to_si
from azoth.core.warnings import Warning
from azoth.process.reference import _review_evidence as ev

MODEL_ID = "process.open_drain_review"

_BASE_SCORE = 0.20


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def open_drain_review(
    _project_name: str | None = None,
    default_liquid_leak_rate: Q | float | None = None,
    items: list[dict[str, Any]] | None = None,
    evidence: list[dict[str, Any]] | None = None,
    design_standards: list[dict[str, Any]] | None = None,
    references: list[dict[str, Any]] | None = None,
) -> OpenDrainReviewResult:
    """The NORSOK S-001 Clause 9 open-drain review.

    Args:
        project_name: the project name the report echoes.
        default_liquid_leak_rate: the default worst credible process fire leak rate;
            absent is ``5`` kg/s.
        items: the review items, each ``{area_id, area_type, drain_system_type}``.
        evidence: the key-value rows, each ``{area_id, key, value}``. **An unparseable
            value yields the default at that key.**
        design_standards: the design standards as rows, each ``{area_id, standard}``.
        references: the technical-document references, each ``{area_id, reference}``.

    Returns:
        The nineteenth assessments per item, the item verdicts and confidences, and
        the report's verdict and counts.

    Raises:
        InvalidInputError: for a duplicate area id, a duplicate ``(area id, key)`` or a
            row naming no item.
    """
    spec = _spec()
    warnings: list[Warning] = []
    default_leak = (
        5.0
        if default_liquid_leak_rate is None
        else input_to_si(spec, "default_liquid_leak_rate", default_liquid_leak_rate)
    )
    apply_checks(
        checks_for(spec).on_input,
        {"default_liquid_leak_rate": default_leak}.get,
        warnings,
    )

    reviewed: list[dict[str, Any]] = []
    seen: set[str] = set()
    item_rows: list[dict[str, Any]] = input_records_to_si(spec, "items", items or [])
    for row in item_rows:
        area_id = str(row["area_id"])
        if area_id in seen:
            raise InvalidInputError("items.area_id", f"`{area_id}` appears more than once")
        seen.add(area_id)
        reviewed.append(
            {
                "area_id": area_id,
                "area_type": _optional_text(row.get("area_type")),
                "drain_system_type": _optional_text(row.get("drain_system_type")),
                "evidence": {},
                "references": [],
            }
        )

    index = {item["area_id"]: item for item in reviewed}
    evidence_rows: list[dict[str, Any]] = input_records_to_si(spec, "evidence", evidence or [])
    for row in evidence_rows:
        area_id = str(row["area_id"])
        item = index.get(area_id)
        if item is None:
            raise InvalidInputError("evidence.area_id", f"`{area_id}` names no item row")
        key = str(row["key"])
        if key in item["evidence"]:
            raise InvalidInputError(
                "evidence.key", f"`{key}` appears more than once for `{area_id}`"
            )
        item["evidence"][key] = _optional_text(row.get("value"))
    standard_rows: list[dict[str, Any]] = input_records_to_si(
        spec, "design_standards", design_standards or []
    )
    for row in standard_rows:
        item = index.get(str(row["area_id"]))
        if item is None:
            raise InvalidInputError(
                "design_standards.area_id", f"`{row['area_id']}` names no item row"
            )
        item["evidence"].setdefault("designStandards", []).append(str(row["standard"]))
    reference_rows: list[dict[str, Any]] = input_records_to_si(spec, "references", references or [])
    for row in reference_rows:
        item = index.get(str(row["area_id"]))
        if item is None:
            raise InvalidInputError("references.area_id", f"`{row['area_id']}` names no item row")
        item["references"].append(str(row["reference"]))

    results: list[OpenDrainResult] = []
    assessments: list[OpenDrainAssessment] = []
    for item in reviewed:
        rows = _evaluate(item, default_leak)
        results.append(
            OpenDrainResult(
                area_id=item["area_id"],
                verdict=_verdict(rows),
                confidence=_confidence(item),
            )
        )
        assessments.extend(rows)

    overall = OpenDrainVerdict.PASS
    if any(row.verdict is OpenDrainVerdict.FAIL for row in results):
        overall = OpenDrainVerdict.FAIL
    elif any(row.verdict is OpenDrainVerdict.PASS_WITH_WARNINGS for row in results):
        overall = OpenDrainVerdict.PASS_WITH_WARNINGS
    return OpenDrainReviewResult(
        overall_verdict=overall,
        item_count=float(len(results)),
        failed_item_count=float(sum(r.verdict is OpenDrainVerdict.FAIL for r in results)),
        warning_item_count=float(
            sum(r.verdict is OpenDrainVerdict.PASS_WITH_WARNINGS for r in results)
        ),
        assessment_count=float(len(assessments)),
        fail_assessment_count=float(sum(a.status == "fail" for a in assessments)),
        warning_assessment_count=float(sum(a.status == "warning" for a in assessments)),
        results=tuple(results),
        assessments=tuple(assessments),
        warnings=tuple(warnings),
    )


def _optional_text(value: object) -> str | None:
    return None if value is None else str(value)


def _push(
    rows: list[OpenDrainAssessment],
    item: dict[str, Any],
    requirement_id: str,
    clause: str,
    status: str,
    severity: str,
) -> None:
    rows.append(
        OpenDrainAssessment(
            area_id=item["area_id"],
            requirement_id=requirement_id,
            clause=clause,
            status=status,
            severity=severity,
        )
    )


def _required_bool(
    rows: list[OpenDrainAssessment],
    item: dict[str, Any],
    requirement_id: str,
    clause: str,
    key: str,
) -> None:
    value = ev.get_bool(item["evidence"], (key,))
    if value is None:
        _push(rows, item, requirement_id, clause, "warning", "high")
    elif value:
        _push(rows, item, requirement_id, clause, "pass", "low")
    else:
        _push(rows, item, requirement_id, clause, "fail", "critical")


def _evaluate(item: dict[str, Any], default_leak: float) -> list[OpenDrainAssessment]:
    rows: list[OpenDrainAssessment] = []
    evidence = item["evidence"]

    # 1. Design-standard evidence.
    standards = ev.get_string_list(
        evidence, ("designStandards", "standards", "drainageSystemStandards")
    )
    combined = " ".join(standards).lower()
    if standards and "p-002" in combined and "13702" in combined:
        _push(rows, item, "OD-S001-9.4.1-STANDARDS", "9.4.1", "pass", "low")
    else:
        _push(rows, item, "OD-S001-9.4.1-STANDARDS", "9.4.1", "warning", "medium")

    # 2. Spill collection.
    if not _requires_drainage_review(item):
        _push(rows, item, "OD-S001-9.4.1-COLLECTION", "9.4.1", "not_applicable", "low")
    else:
        _required_bool(rows, item, "OD-S001-9.4.1-COLLECTION", "9.4.1", "hasOpenDrainMeasures")

    # 3. Capacity.
    fire_water = ev.get_double(
        evidence, 0.0, ("fireWaterCapacityKgPerS", "fireWaterRateKgPerS", "delugeLoadKgPerS")
    )
    leak_rate = ev.get_double(
        evidence,
        default_leak,
        ("liquidLeakRateKgPerS", "worstCredibleLeakKgPerS", "processFireLeakKgPerS"),
    )
    required = max(0.0, fire_water) + max(0.0, leak_rate)
    capacity_keys = ("drainageCapacityKgPerS", "openDrainCapacityKgPerS", "capacityKgPerS")
    if not ev.has_any(evidence, capacity_keys):
        _push(rows, item, "OD-S001-9.4.1-CAPACITY", "9.4.1", "warning", "high")
    else:
        capacity = ev.get_double(evidence, math.nan, capacity_keys)
        if capacity + 1.0e-12 < required:
            _push(rows, item, "OD-S001-9.4.1-CAPACITY", "9.4.1", "fail", "critical")
        else:
            _push(rows, item, "OD-S001-9.4.1-CAPACITY", "9.4.1", "pass", "low")

    # 4. Backflow.
    _required_bool(rows, item, "OD-S001-9.4.1-BACKFLOW", "9.4.1", "backflowPrevented")

    # 5. Bunding.
    if ev.get_bool_default(
        evidence,
        False,
        ("hasFlammableLiquidTanks", "tankArea", "flammableLiquidVesselsPresent"),
    ):
        _required_bool(rows, item, "OD-S001-9.4.1-BUNDING", "9.4.1", "bundingAndDrainageAdequate")
    else:
        _push(rows, item, "OD-S001-9.4.1-BUNDING", "9.4.1", "not_applicable", "low")

    # 6. Floater.
    if ev.get_bool_default(evidence, False, ("isFloater", "floatingFacility", "floater")):
        _required_bool(rows, item, "OD-S001-9.4.1-FLOATER", "9.4.1", "preventsCrossAreaSpreading")
    else:
        _push(rows, item, "OD-S001-9.4.1-FLOATER", "9.4.1", "not_applicable", "low")

    # 7. Hazardous drain tank inerting.
    tank = ev.get_bool_default(
        evidence,
        False,
        (
            "hazardousAreaCollectionTank",
            "hazardousDrainTank",
            "collectionTankInHazardousService",
        ),
    ) or (
        ev.get_bool_default(evidence, False, ("hazardousArea",))
        and ev.get_bool_default(evidence, False, ("collectionTankPresent",))
    )
    if tank:
        _required_bool(rows, item, "OD-S001-9.4.2-INERTING", "9.4.2", "purgedOrInerted")
    else:
        _push(rows, item, "OD-S001-9.4.2-INERTING", "9.4.2", "not_applicable", "low")

    # 8. Closed/open interaction.
    _required_bool(
        rows,
        item,
        "OD-S001-9.4.2-CLOSED-OPEN",
        "9.4.2",
        "closedOpenDrainInteractionPrevented",
    )

    # 9. Hazardous/non-hazardous separation.
    separated = ev.get_bool(evidence, ("hazardousNonHazardousPhysicallySeparated",))
    combined_bool = ev.get_bool(
        evidence, ("hazardousNonHazardousCombined", "hazardousNonHazardousShareSump")
    )
    if combined_bool is True and separated is not True:
        _push(rows, item, "OD-S001-9.4.2-HAZ-NONHAZ", "9.4.2", "fail", "critical")
    elif separated is None and combined_bool is None:
        _push(rows, item, "OD-S001-9.4.2-HAZ-NONHAZ", "9.4.2", "warning", "high")
    else:
        _push(rows, item, "OD-S001-9.4.2-HAZ-NONHAZ", "9.4.2", "pass", "low")

    # 10. Common sump.
    if ev.get_bool_default(evidence, False, ("commonCaissonOrSump", "sharedCaisson", "sharedSump")):
        _required_bool(
            rows,
            item,
            "OD-S001-9.4.2-COMMON-SUMP",
            "9.4.2",
            "nonHazardousBackflowPrevented",
        )
    else:
        _push(rows, item, "OD-S001-9.4.2-COMMON-SUMP", "9.4.2", "not_applicable", "low")

    # 11-13. Seals, vent and fire seals.
    _required_bool(
        rows,
        item,
        "OD-S001-9.4.2-SEAL-BACKPRESSURE",
        "9.4.2",
        "sealDesignedForMaxBackpressure",
    )
    _required_bool(rows, item, "OD-S001-9.4.2-VENT", "9.4.2", "ventTerminatedSafe")
    if ev.get_bool_default(
        evidence, False, ("multipleFireAreas", "crossesFireAreas", "fireAreaInterface")
    ):
        _required_bool(
            rows,
            item,
            "OD-S001-9.4.2-FIRE-SEALS",
            "9.4.2",
            "liquidFireSealsBetweenFireAreas",
        )
    else:
        _push(rows, item, "OD-S001-9.4.2-FIRE-SEALS", "9.4.2", "not_applicable", "low")

    # 14. Helideck.
    if _is_helideck(item):
        _required_bool(rows, item, "OD-S001-9.4.2-HELIDECK", "9.4.2", "dedicatedPipeDrainage")
    else:
        _push(rows, item, "OD-S001-9.4.2-HELIDECK", "9.4.2", "not_applicable", "low")

    # 15. Drilling/production separation.
    if ev.get_bool_default(
        evidence, False, ("drillingArea", "drillingDrain")
    ) or ev.contains_ignore_case(item["area_type"] or "", "drilling"):
        connected = ev.get_bool(
            evidence,
            ("drillingProductionOpenDrainConnected", "connectedToProductionOpenDrain"),
        )
        if connected is None:
            _push(rows, item, "OD-S001-9.4.2-DRILLING-PROD", "9.4.2", "warning", "high")
        elif connected:
            _push(rows, item, "OD-S001-9.4.2-DRILLING-PROD", "9.4.2", "fail", "critical")
        else:
            _push(rows, item, "OD-S001-9.4.2-DRILLING-PROD", "9.4.2", "pass", "low")
    else:
        _push(rows, item, "OD-S001-9.4.2-DRILLING-PROD", "9.4.2", "not_applicable", "low")

    # 16. Temporary storage.
    if ev.get_bool_default(evidence, False, ("temporaryFlammableLiquidStorage", "toteTankStorage")):
        _required_bool(rows, item, "OD-S001-9.4.2-TOTE", "9.4.2", "adequateCollectionDrainForTote")
    else:
        _push(rows, item, "OD-S001-9.4.2-TOTE", "9.4.2", "not_applicable", "low")

    # 17. Fire division.
    if ev.get_bool_default(
        evidence,
        False,
        (
            "drainBoxInFireDivision",
            "drainPipePenetratesFireDivision",
            "fireDivisionPenetration",
        ),
    ):
        _required_bool(
            rows,
            item,
            "OD-S001-9.4.3-FIRE-DIVISION",
            "9.4.3",
            "fireDivisionIntegrityMaintained",
        )
    else:
        _push(rows, item, "OD-S001-9.4.3-FIRE-DIVISION", "9.4.3", "not_applicable", "low")

    # 18. Utility independence.
    dependency = ev.get_bool(
        evidence,
        ("openDrainDependsOnUtility", "dependsOnUtility", "requiresUtilityForDrainage"),
    )
    if dependency is None:
        _push(rows, item, "OD-S001-9.4.1-UTILITY", "9.4.1", "info", "low")
    elif dependency:
        _push(rows, item, "OD-S001-9.4.1-UTILITY", "9.4.1", "fail", "high")
    else:
        _push(rows, item, "OD-S001-9.4.1-UTILITY", "9.4.1", "pass", "low")

    # 19. Operational evidence.
    operational_keys = (
        "observedBackflowEvents",
        "sumpHighLevelEvents",
        "observedBackpressureBara",
        "maximumAllowedBackpressureBara",
        "tagreaderSource",
    )
    if not ev.has_any(evidence, operational_keys):
        _push(rows, item, "OD-OPS-TAGREADER", "Operational evidence", "info", "low")
    else:
        backflow = ev.get_double(evidence, 0.0, ("observedBackflowEvents",))
        high_level = ev.get_double(evidence, 0.0, ("sumpHighLevelEvents",))
        observed = ev.get_double(evidence, math.nan, ("observedBackpressureBara",))
        maximum = ev.get_double(evidence, math.nan, ("maximumAllowedBackpressureBara",))
        over_backpressure = (
            not math.isnan(observed) and not math.isnan(maximum) and observed > maximum
        )
        if backflow > 0.0 or over_backpressure:
            _push(rows, item, "OD-OPS-TAGREADER", "Operational evidence", "fail", "high")
        elif high_level > 0.0:
            _push(rows, item, "OD-OPS-TAGREADER", "Operational evidence", "warning", "medium")
        else:
            _push(rows, item, "OD-OPS-TAGREADER", "Operational evidence", "pass", "low")
    return rows


def _requires_drainage_review(item: dict[str, Any]) -> bool:
    source = ev.get_bool(
        item["evidence"],
        ("sourceHasFlammableOrHazardousLiquid", "flammableOrHazardousLiquidSource"),
    )
    if source is not None:
        return source
    searchable = f"{item['area_type'] or ''} {item['drain_system_type'] or ''}".lower()
    return (
        any(
            token in searchable
            for token in (
                "process",
                "hazard",
                "hydrocarbon",
                "tank",
                "well",
                "drilling",
                "helideck",
                "storage",
            )
        )
        or not searchable.strip()
    )


def _is_helideck(item: dict[str, Any]) -> bool:
    return (
        ev.get_bool_default(item["evidence"], False, ("helideckDrain", "helideckArea"))
        or ev.contains_ignore_case(item["area_type"] or "", "helideck")
        or ev.contains_ignore_case(item["area_id"], "helideck")
    )


def _confidence(item: dict[str, Any]) -> float:
    score = _BASE_SCORE
    if item["references"]:
        score += 0.20
    if ev.has_any(
        item["evidence"],
        ("drainageCapacityKgPerS", "openDrainCapacityKgPerS", "capacityKgPerS"),
    ):
        score += 0.15
    if ev.has_any(
        item["evidence"],
        ("hasOpenDrainMeasures", "backflowPrevented", "closedOpenDrainInteractionPrevented"),
    ):
        score += 0.20
    if ev.has_any(item["evidence"], ("designStandards", "standards", "drainageSystemStandards")):
        score += 0.10
    if ev.has_any(
        item["evidence"],
        ("observedBackflowEvents", "sumpHighLevelEvents", "tagreaderSource"),
    ):
        score += 0.15
    return min(1.0, score)


def _verdict(rows: list[OpenDrainAssessment]) -> OpenDrainVerdict:
    if any(row.status == "fail" for row in rows):
        return OpenDrainVerdict.FAIL
    if any(row.status == "warning" for row in rows):
        return OpenDrainVerdict.PASS_WITH_WARNINGS
    return OpenDrainVerdict.PASS
