"""``process.barrier_register_validation`` - the barrier register's rule review.

As NeqSim's ``BarrierRegister.validate`` and its ``SafetyBarrier``,
``PerformanceStandard`` and ``SafetyCriticalElement`` ``validate`` methods compute it.
Spec: ``specs/models/process/barrier_register_validation.toml``

The Python twin of
``crates/azoth-process/src/models/barrier_register_validation.rs``: the register,
element, barrier and standard rules, the traceability helpers and the class's iteration
quirks.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import BarrierRegisterValidationResult, RegisterFinding
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._safety_review import (
    BARRIER_STATUSES,
    BARRIER_TYPES,
    ELEMENT_TYPES,
    EVIDENCE_OWNERS,
)

MODEL_ID = "process.barrier_register_validation"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _t(value: object | None) -> str:
    return "" if value is None else str(value).strip()


def _traceable(rows: list[tuple[str, str, bool]], owner: str, owner_id: str) -> bool:
    return any(kind == owner and key == owner_id and traceable for kind, key, traceable in rows)


def _finding(scope: str, subject_id: str, rule: str) -> RegisterFinding:
    return RegisterFinding(scope=scope, subject_id=subject_id, rule=rule)


def _validate_standard(
    standard_id: str,
    standard: dict[str, Any],
    standard_traceable: bool,
    findings: list[RegisterFinding],
) -> None:
    if not standard_id:
        findings.append(_finding("standard", standard_id, "standard_id_missing"))
    if not _t(standard.get("safety_function")):
        findings.append(_finding("standard", standard_id, "standard_safety_function_missing"))
    target = standard.get("target_pfd")
    if target is not None and not math.isnan(float(target)) and not (0.0 < float(target) <= 1.0):
        findings.append(_finding("standard", standard_id, "standard_target_pfd_out_of_range"))
    availability = standard.get("required_availability")
    if (
        availability is not None
        and not math.isnan(float(availability))
        and not (0.0 <= float(availability) <= 1.0)
    ):
        findings.append(
            _finding("standard", standard_id, "standard_required_availability_out_of_range")
        )
    interval = standard.get("proof_test_interval_hours")
    if interval is not None and not math.isnan(float(interval)) and float(interval) <= 0.0:
        findings.append(
            _finding("standard", standard_id, "standard_proof_test_interval_not_positive")
        )
    if float(standard.get("acceptance_criteria_count", 0.0)) < 1.0:
        findings.append(_finding("standard", standard_id, "standard_no_acceptance_criteria"))
    if not standard_traceable:
        findings.append(_finding("standard", standard_id, "standard_no_traceable_evidence"))


def _validate_barrier(
    barrier_id: str,
    barrier: dict[str, Any],
    standards: dict[str, dict[str, Any]],
    evidence: list[tuple[str, str, bool]],
    findings: list[RegisterFinding],
) -> None:
    if not barrier_id:
        findings.append(_finding("barrier", barrier_id, "barrier_id_missing"))
    if not _t(barrier.get("name")) and not _t(barrier.get("description")):
        findings.append(_finding("barrier", barrier_id, "barrier_name_or_description_missing"))
    pfd = barrier.get("pfd")
    if pfd is not None and not math.isnan(float(pfd)) and not (0.0 <= float(pfd) <= 1.0):
        findings.append(_finding("barrier", barrier_id, "barrier_pfd_out_of_range"))
    effectiveness = barrier.get("effectiveness")
    if (
        effectiveness is not None
        and not math.isnan(float(effectiveness))
        and not (0.0 <= float(effectiveness) <= 1.0)
    ):
        findings.append(_finding("barrier", barrier_id, "barrier_effectiveness_out_of_range"))
    if str(barrier.get("status")) == "unknown":
        findings.append(_finding("barrier", barrier_id, "barrier_status_unknown"))
    if float(barrier.get("equipment_tag_count", 0.0)) < 1.0:
        findings.append(_finding("barrier", barrier_id, "barrier_no_equipment_tag"))
    standard_id = _t(barrier.get("performance_standard_id"))
    standard = standards.get(standard_id)
    if standard is None:
        findings.append(_finding("barrier", barrier_id, "barrier_no_performance_standard"))
    else:
        _validate_standard(
            standard_id,
            standard,
            _traceable(evidence, "standard", standard_id),
            findings,
        )
    own = _traceable(evidence, "barrier", barrier_id)
    through_standard = standard is not None and _traceable(evidence, "standard", standard_id)
    if not (own or through_standard):
        findings.append(_finding("barrier", barrier_id, "barrier_no_traceable_evidence"))


def barrier_register_validation(
    register_id: str,
    elements: list[dict[str, object]] | None = None,
    barriers: list[dict[str, object]] | None = None,
    standards: list[dict[str, object]] | None = None,
    links: list[dict[str, object]] | None = None,
    evidence: list[dict[str, object]] | None = None,
) -> BarrierRegisterValidationResult:
    """The register's findings.

    Args:
        register_id: the register id.
        elements: the safety-critical element rows.
        barriers: the barrier rows.
        standards: the performance-standard rows.
        links: the element-to-barrier links.
        evidence: the document-evidence rows.

    Returns:
        One finding per rule violation, in the class's order and with its duplicates.

    Raises:
        InvalidInputError: for an unknown enumeration value, a link naming an unknown
            element or barrier, or a negative count.
    """
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    element_rows: list[dict[str, Any]] = input_records_to_si(spec, "elements", elements or [])
    barrier_rows: list[dict[str, Any]] = input_records_to_si(spec, "barriers", barriers or [])
    standard_rows: list[dict[str, Any]] = input_records_to_si(spec, "standards", standards or [])
    link_rows: list[dict[str, Any]] = input_records_to_si(spec, "links", links or [])
    evidence_rows: list[dict[str, Any]] = input_records_to_si(spec, "evidence", evidence or [])

    element_map: dict[str, dict[str, Any]] = {}
    for row in element_rows:
        if str(row["element_type"]) not in ELEMENT_TYPES:
            raise InvalidInputError(
                "elements.element_type",
                f"`{row['element_type']}` is not one of {', '.join(ELEMENT_TYPES)}",
            )
        element_map[_t(row["id"])] = row
    barrier_map: dict[str, dict[str, Any]] = {}
    for row in barrier_rows:
        if str(row["barrier_type"]) not in BARRIER_TYPES:
            raise InvalidInputError(
                "barriers.barrier_type",
                f"`{row['barrier_type']}` is not one of {', '.join(BARRIER_TYPES)}",
            )
        if str(row["status"]) not in BARRIER_STATUSES:
            raise InvalidInputError(
                "barriers.status",
                f"`{row['status']}` is not one of {', '.join(BARRIER_STATUSES)}",
            )
        count = float(row["equipment_tag_count"])
        if not math.isfinite(count) or count < 0.0:
            raise InvalidInputError(
                "barriers.equipment_tag_count", "the count must be at least zero"
            )
        barrier_map[_t(row["id"])] = row
    standard_map: dict[str, dict[str, Any]] = {}
    for row in standard_rows:
        count = float(row["acceptance_criteria_count"])
        if not math.isfinite(count) or count < 0.0:
            raise InvalidInputError(
                "standards.acceptance_criteria_count",
                "the count must be at least zero",
            )
        standard_map[_t(row["id"])] = row

    element_links: dict[str, list[str]] = {}
    for index, link in enumerate(link_rows):
        element_id = _t(link["element_id"])
        barrier_id = _t(link["barrier_id"])
        if element_id not in element_map:
            raise InvalidInputError(
                f"links[{index}].element_id", f"`{element_id}` names no element row"
            )
        if barrier_id not in barrier_map:
            raise InvalidInputError(
                f"links[{index}].barrier_id", f"`{barrier_id}` names no barrier row"
            )
        ids = element_links.setdefault(element_id, [])
        if barrier_id not in ids:
            ids.append(barrier_id)

    evidence_list: list[tuple[str, str, bool]] = []
    for row in evidence_rows:
        owner = str(row["owner_kind"])
        if owner not in EVIDENCE_OWNERS:
            raise InvalidInputError(
                "evidence.owner_kind",
                f"`{owner}` is not one of {', '.join(EVIDENCE_OWNERS)}",
            )
        traceable = bool(_t(row.get("document_id")) or _t(row.get("source_reference"))) and bool(
            _t(row.get("excerpt"))
        )
        evidence_list.append((owner, _t(row["owner_id"]), traceable))

    findings: list[RegisterFinding] = []
    register_key = _t(register_id)
    if not register_key:
        findings.append(_finding("register", register_key, "register_id_missing"))
    if not element_map and not barrier_map:
        findings.append(_finding("register", register_key, "register_empty"))

    for element_id, element in element_map.items():
        if not element_id:
            findings.append(_finding("element", element_id, "element_id_missing"))
        if not _t(element.get("tag")) and not _t(element.get("name")):
            findings.append(_finding("element", element_id, "element_tag_or_name_missing"))
        linked = element_links.get(element_id, [])
        if not linked:
            findings.append(_finding("element", element_id, "element_no_barriers"))
        own = _traceable(evidence_list, "element", element_id)
        through_barrier = any(
            _traceable(evidence_list, "barrier", barrier_id)
            or _traceable(
                evidence_list,
                "standard",
                _t(barrier_map.get(barrier_id, {}).get("performance_standard_id")),
            )
            for barrier_id in linked
        )
        if not (own or through_barrier):
            findings.append(_finding("element", element_id, "element_no_traceable_evidence"))
        for barrier_id in linked:
            barrier = barrier_map.get(barrier_id)
            if barrier is not None:
                _validate_barrier(barrier_id, barrier, standard_map, evidence_list, findings)

    for barrier_id in barrier_map:
        is_linked = any(barrier_id in ids for ids in element_links.values())
        if not is_linked:
            findings.append(_finding("barrier", barrier_id, "barrier_not_linked_to_element"))

    return BarrierRegisterValidationResult(
        findings=tuple(findings),
        finding_count=float(len(findings)),
        valid=not findings,
        warnings=tuple(warnings),
    )
