"""``process.safety_system_performance`` - the safety-system performance assessment.

The Python twin of ``crates/azoth-process/src/models/safety_system_performance.rs``.
The port source is NeqSim's ``process/safety/barrier/SafetySystemPerformanceAnalyzer.java``:
the five check families per barrier and matching demand, the category inference, the
related instruments and SIFs, and a stable rule on every finding.
"""

from __future__ import annotations

from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import (
    PerformanceAssessment,
    PerformanceFinding,
    PerformanceInstrument,
    PerformanceSif,
    PerformanceVerdict,
    SafetySystemPerformanceResult,
)
from azoth.core.units import from_si, input_records_to_si
from azoth.core.warnings import Warning
from azoth.process.reference.sil_verification import sil_verification

MODEL_ID = "process.safety_system_performance"

_VOTING = {
    "1oo1": (1.0, 1.0),
    "1oo2": (1.0, 2.0),
    "2oo2": (2.0, 2.0),
    "2oo3": (2.0, 3.0),
    "2oo4": (2.0, 4.0),
    "3oo4": (3.0, 4.0),
}


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def safety_system_performance(
    register_id: str,
    register_name: str | None = None,
    elements: list[dict[str, object]] | None = None,
    links: list[dict[str, object]] | None = None,
    barriers: list[dict[str, object]] | None = None,
    barrier_tags: list[dict[str, object]] | None = None,
    standards: list[dict[str, object]] | None = None,
    standard_criteria: list[dict[str, object]] | None = None,
    demands: list[dict[str, object]] | None = None,
    measurement_devices: list[dict[str, object]] | None = None,
    event_sifs: list[dict[str, object]] | None = None,
    sif_detectors: list[dict[str, object]] | None = None,
    quantitative_sifs: list[dict[str, object]] | None = None,
    protected_equipment: list[dict[str, object]] | None = None,
) -> SafetySystemPerformanceResult:
    """The safety-system performance assessment.

    Args:
        register_id: the register id the report echoes.
        register_name: the register name the report echoes.
        elements: the safety-critical elements, each ``{id, element_type}``.
        links: the element-barrier links, each ``{element_id, barrier_id}``.
        barriers: the barriers, each ``{id, status, ...}``.
        barrier_tags: the barriers' equipment tags, each ``{barrier_id, tag}``.
        standards: the performance standards.
        standard_criteria: the standards' acceptance criteria.
        demands: the demand cases.
        measurement_devices: the fire, gas and other measurement devices.
        event_sifs: the event and voting SIFs.
        sif_detectors: the event SIFs' detectors.
        quantitative_sifs: the quantitative SIL/PFD SIFs.
        protected_equipment: the quantitative SIFs' protected equipment.

    Returns:
        The assessments, findings, related instruments and SIFs, and the report's
        verdict and counts.

    Raises:
        InvalidInputError: for a row naming an object not in the register, or a
            quantitative SIF ``process.sil_verification`` refuses.
    """
    spec = _spec()
    warnings: list[Warning] = []
    element_rows = input_records_to_si(spec, "elements", elements or [])
    link_rows = input_records_to_si(spec, "links", links or [])
    barrier_rows = input_records_to_si(spec, "barriers", barriers or [])
    tag_rows = input_records_to_si(spec, "barrier_tags", barrier_tags or [])
    standard_rows = input_records_to_si(spec, "standards", standards or [])
    criteria_rows = input_records_to_si(spec, "standard_criteria", standard_criteria or [])
    demand_rows = input_records_to_si(spec, "demands", demands or [])
    device_rows = input_records_to_si(spec, "measurement_devices", measurement_devices or [])
    event_rows = input_records_to_si(spec, "event_sifs", event_sifs or [])
    detector_rows = input_records_to_si(spec, "sif_detectors", sif_detectors or [])
    quantitative_rows = input_records_to_si(spec, "quantitative_sifs", quantitative_sifs or [])
    protected_rows = input_records_to_si(spec, "protected_equipment", protected_equipment or [])

    barriers_by_id: dict[str, dict[str, Any]] = {}
    for row in barrier_rows:
        barrier_id = str(row["id"])
        barriers_by_id[barrier_id] = {
            "id": barrier_id,
            "status": str(row.get("status") or "unknown"),
            "has_traceable_evidence": bool(row.get("has_traceable_evidence") or False),
            "name": _text(row.get("name")),
            "description": _text(row.get("description")),
            "safety_function": _text(row.get("safety_function")),
            "pfd": _finite(row.get("pfd")),
            "effectiveness": _finite(row.get("effectiveness")),
            "performance_standard_id": _text(row.get("performance_standard_id")),
        }
    standards_by_id: dict[str, dict[str, Any]] = {}
    for row in standard_rows:
        standards_by_id[str(row["id"])] = {
            "id": str(row["id"]),
            "title": _text(row.get("title")),
            "safety_function": _text(row.get("safety_function")),
            "target_pfd": _finite(row.get("target_pfd")),
            "required_availability": _finite(row.get("required_availability")),
            "response_time_seconds": _finite(row.get("response_time_seconds")),
        }
    criteria: list[dict[str, Any]] = [
        {"standard_id": str(row["standard_id"]), "criterion": str(row["criterion"])}
        for row in criteria_rows
    ]
    tags: list[dict[str, str]] = [
        {"barrier_id": str(row["barrier_id"]), "tag": str(row["tag"])} for row in tag_rows
    ]
    elements_index: list[dict[str, Any]] = []
    for row in element_rows:
        element_id = str(row["id"])
        existing = next((item for item in elements_index if item["id"] == element_id), None)
        if existing is None:
            existing = {"id": element_id, "element_type": str(row["element_type"]), "barriers": []}
            elements_index.append(existing)
        else:
            existing["element_type"] = str(row["element_type"])
    for row in link_rows:
        barrier_id = str(row["barrier_id"])
        if barrier_id not in barriers_by_id:
            raise InvalidInputError("links.barrier_id", f"`{barrier_id}` names no barrier row")
        element_id = str(row["element_id"])
        element = next((item for item in elements_index if item["id"] == element_id), None)
        if element is None:
            raise InvalidInputError("links.element_id", f"`{element_id}` names no element row")
        element["barriers"].append(barrier_id)
    for row in tags:
        if row["barrier_id"] not in barriers_by_id:
            raise InvalidInputError(
                "barrier_tags.barrier_id", f"`{row['barrier_id']}` names no barrier row"
            )
    for row in criteria:
        if row["standard_id"] not in standards_by_id:
            raise InvalidInputError(
                "standard_criteria.standard_id", f"`{row['standard_id']}` names no standard row"
            )
    event_by_name = {str(row["name"]): row for row in event_rows}
    detectors = [
        {
            "sif_name": str(row["sif_name"]),
            "name": str(row["name"]),
            "bypassed": bool(row.get("bypassed") or False),
            "faulty": bool(row.get("faulty") or False),
            "tripped": bool(row.get("tripped") or False),
        }
        for row in detector_rows
    ]
    for row in detectors:
        if row["sif_name"] not in event_by_name:
            raise InvalidInputError(
                "sif_detectors.sif_name", f"`{row['sif_name']}` names no event SIF row"
            )
    quantitative: list[dict[str, Any]] = [
        {
            "id": str(row["id"]),
            "name": _text(row.get("name")),
            "description": _text(row.get("description")),
            "architecture": str(row["architecture"]),
            "claimed_sil": float(row["claimed_sil"]),
            "pfd_average": float(row["pfd_average"]),
            "test_interval_hours": _finite(row.get("test_interval_hours")),
        }
        for row in quantitative_rows
    ]
    protected: list[dict[str, str]] = [
        {"sif_id": str(row["sif_id"]), "equipment": str(row["equipment"])}
        for row in protected_rows
    ]
    for row in protected:
        if not any(sif["id"] == row["sif_id"] for sif in quantitative):
            raise InvalidInputError(
                "protected_equipment.sif_id", f"`{row['sif_id']}` names no quantitative SIF row"
            )
    devices = [
        {
            "name": str(row["name"]),
            "kind": str(row["kind"]),
            "tag": _text(row.get("tag")),
            "location": _text(row.get("location")),
            "response_time_seconds": _finite(row.get("response_time_seconds")),
        }
        for row in device_rows
    ]
    demands_index = [
        {
            "demand_id": str(row["demand_id"]),
            "barrier_id": _text(row.get("barrier_id")),
            "equipment_tag": _text(row.get("equipment_tag")),
            "scenario": _text(row.get("scenario")),
            "category": str(row.get("category") or "unknown"),
            "demand_value": _finite(row.get("demand_value")),
            "capacity_value": _finite(row.get("capacity_value")),
            "demand_unit": _text(row.get("demand_unit")),
            "required_pfd": _finite(row.get("required_pfd")),
            "actual_pfd": _finite(row.get("actual_pfd")),
            "required_availability": _finite(row.get("required_availability")),
            "actual_availability": _finite(row.get("actual_availability")),
            "required_response_time_seconds": _finite(row.get("required_response_time_seconds")),
            "actual_response_time_seconds": _finite(row.get("actual_response_time_seconds")),
            "required_effectiveness": _finite(row.get("required_effectiveness")),
            "actual_effectiveness": _finite(row.get("actual_effectiveness")),
        }
        for row in demand_rows
    ]

    register = {
        "elements": elements_index,
        "standards": standards_by_id,
        "criteria": criteria,
        "tags": tags,
        "devices": devices,
        "event_sifs": [
            {
                "name": str(row["name"]),
                "voting_notation": str(row["voting_notation"]),
                "overridden": bool(row.get("overridden") or False),
            }
            for row in event_rows
        ],
        "detectors": detectors,
        "quantitative_sifs": quantitative,
        "protected": protected,
    }

    assessments: list[PerformanceAssessment] = []
    findings: list[PerformanceFinding] = []
    instruments: list[PerformanceInstrument] = []
    sifs: list[PerformanceSif] = []
    for barrier in barriers_by_id.values():
        matching = [
            demand
            for demand in demands_index
            if _matches_demand(demand, barrier, tags)
        ]
        if not matching:
            assessments.append(
                _assess(barrier, None, register, findings, instruments, sifs)
            )
        else:
            for demand in matching:
                assessments.append(
                    _assess(barrier, demand, register, findings, instruments, sifs)
                )

    overall = PerformanceVerdict.PASS
    pass_count = 0.0
    warning_count = 0.0
    fail_count = 0.0
    for assessment in assessments:
        if assessment.verdict == "fail":
            fail_count += 1.0
            overall = PerformanceVerdict.FAIL
        elif assessment.verdict == "pass_with_warnings":
            warning_count += 1.0
            if overall == PerformanceVerdict.PASS:
                overall = PerformanceVerdict.PASS_WITH_WARNINGS
        else:
            pass_count += 1.0
    return SafetySystemPerformanceResult(
        register_id=register_id,
        register_name=register_name or "",
        overall_verdict=overall,
        assessment_count=float(len(assessments)),
        pass_count=pass_count,
        warning_count=warning_count,
        fail_count=fail_count,
        assessments=tuple(assessments),
        findings=tuple(findings),
        instruments=tuple(instruments),
        sifs=tuple(sifs),
        warnings=tuple(warnings),
    )


def _text(value: object) -> str | None:
    return None if value is None else str(value)


def _finite(value: object) -> float | None:
    if value is None:
        return None
    number = float(value)
    return number if number == number else None


def _normalise(text: str | None) -> str:
    return ("" if text is None else text).strip().lower()


def _matches_text(value: str | None, pattern: str | None) -> bool:
    left = _normalise(value)
    right = _normalise(pattern)
    if not left or not right:
        return False
    return left == right or right in left or left in right


def _select_first_finite(first: float | None, second: float | None) -> float | None:
    if first is not None and first == first:
        return first
    if second is not None and second == second:
        return second
    return None


def _push(
    findings: list[PerformanceFinding],
    barrier_id: str,
    demand_id: str | None,
    rule: str,
    severity: str,
) -> None:
    findings.append(
        PerformanceFinding(
            barrier_id=barrier_id, demand_id=demand_id, rule=rule, severity=severity
        )
    )


def _matches_demand(
    demand: dict[str, Any], barrier: dict[str, Any], tags: list[dict[str, str]]
) -> bool:
    barrier_id = demand["barrier_id"]
    if barrier_id and barrier_id == barrier["id"]:
        return True
    tag = demand["equipment_tag"]
    if not tag:
        return False
    return any(row["barrier_id"] == barrier["id"] and row["tag"] == tag for row in tags)


def _barrier_tags(register: dict[str, Any], barrier_id: str) -> list[str]:
    return [row["tag"] for row in register["tags"] if row["barrier_id"] == barrier_id]


def _effective_effectiveness(barrier: dict[str, Any]) -> float | None:
    if barrier["effectiveness"] is not None:
        return barrier["effectiveness"]
    if barrier["pfd"] is not None:
        return 1.0 - barrier["pfd"]
    return None


def _contains_any(text: str, keywords: tuple[str, ...]) -> bool:
    return any(keyword in text for keyword in keywords)


def _determine_category(
    register: dict[str, Any], barrier: dict[str, Any], demand: dict[str, Any] | None
) -> str:
    if demand is not None and demand["category"] != "unknown":
        return demand["category"]
    standard_id = barrier["performance_standard_id"]
    standard = register["standards"].get(standard_id) if standard_id else None
    parts = [
        barrier["id"],
        barrier["name"],
        barrier["description"],
        barrier["safety_function"],
    ]
    if standard is not None:
        parts.append(standard["title"])
        parts.append(standard["safety_function"])
        parts.extend(
            row["criterion"]
            for row in register["criteria"]
            if row["standard_id"] == standard["id"]
        )
    text = " ".join(part for part in parts if part).lower()
    if _contains_any(
        text, ("deluge", "firewater", "fire water", "sprinkler", "water spray")
    ):
        return "firewater_deluge"
    if _contains_any(
        text, ("fire detector", "gas detector", "f&g", "fgs", "fire and gas", "detection")
    ):
        return "fire_gas_detection"
    if _contains_any(text, ("pfp", "passive fire", "fire proof", "fireproof", "coating")):
        return "passive_fire_protection"
    if _contains_any(text, ("psd", "process shutdown", "shutdown valve", "process trip")):
        return "psd"
    if _contains_any(text, ("blowdown", "esd", "emergency shutdown")):
        return "esd_blowdown"
    if _contains_any(text, ("psv", "relief", "flare", "vent")):
        return "relief_flare"
    if _contains_any(text, ("firewall", "fire wall", "blast wall", "structural")):
        return "structural_firewall"
    if _contains_any(text, ("hipps", "high integrity pressure")):
        return "hipps"
    for element in register["elements"]:
        if barrier["id"] not in element["barriers"]:
            continue
        if element["element_type"] == "instrumented_function":
            return "esd_blowdown"
        if element["element_type"] == "fire_protection":
            return "firewater_deluge"
        if element["element_type"] == "structural":
            return "structural_firewall"
    return "unknown"


def _assess(
    barrier: dict[str, Any],
    demand: dict[str, Any] | None,
    register: dict[str, Any],
    findings: list[PerformanceFinding],
    instruments: list[PerformanceInstrument],
    sifs: list[PerformanceSif],
) -> PerformanceAssessment:
    barrier_id = barrier["id"]
    demand_id = None if demand is None else demand["demand_id"]
    category = _determine_category(register, barrier, demand)
    tags = _barrier_tags(register, barrier_id)
    standard_id = barrier["performance_standard_id"]
    standard = register["standards"].get(standard_id) if standard_id else None
    effectiveness = _effective_effectiveness(barrier)
    local: list[PerformanceFinding] = []
    local_sifs: list[PerformanceSif] = []

    # 1. Credit.
    if barrier["status"] != "available":
        _push(local, barrier_id, demand_id, "barrier_not_available", "fail")
    if not barrier["has_traceable_evidence"]:
        _push(local, barrier_id, demand_id, "barrier_untraceable", "warning")

    # 2. Performance standard.
    standard_out = None
    target_pfd = actual_pfd = None
    required_availability = actual_availability = None
    required_response = actual_response = None
    required_effectiveness = actual_effectiveness = None
    if standard is not None:
        standard_out = standard["id"]
        target = _select_first_finite(
            None if demand is None else demand["required_pfd"], standard["target_pfd"]
        )
        actual = _select_first_finite(
            None if demand is None else demand["actual_pfd"], barrier["pfd"]
        )
        if target is not None:
            target_pfd = target
            actual_pfd = actual
            if actual is None:
                _push(local, barrier_id, demand_id, "target_pfd_without_actual", "warning")
            elif actual > target:
                _push(local, barrier_id, demand_id, "pfd_exceeded", "fail")
        required = _select_first_finite(
            None if demand is None else demand["required_availability"],
            standard["required_availability"],
        )
        if required is not None:
            required_availability = required
            actual_availability = None if demand is None else demand["actual_availability"]
            if actual_availability is None:
                _push(
                    local,
                    barrier_id,
                    demand_id,
                    "required_availability_without_actual",
                    "warning",
                )
            elif actual_availability < required:
                _push(local, barrier_id, demand_id, "availability_below_required", "fail")
        required = _select_first_finite(
            None if demand is None else demand["required_response_time_seconds"],
            standard["response_time_seconds"],
        )
        if required is not None:
            required_response = required
            actual_response = (
                None if demand is None else demand["actual_response_time_seconds"]
            )
            if actual_response is None:
                _push(
                    local,
                    barrier_id,
                    demand_id,
                    "required_response_time_without_actual",
                    "warning",
                )
            elif actual_response > required:
                _push(local, barrier_id, demand_id, "response_time_exceeded", "fail")
        if demand is not None and demand["required_effectiveness"] is not None:
            required_effectiveness = demand["required_effectiveness"]
            actual_effectiveness = _select_first_finite(
                demand["actual_effectiveness"], effectiveness
            )
            if actual_effectiveness is None:
                _push(
                    local,
                    barrier_id,
                    demand_id,
                    "required_effectiveness_without_actual",
                    "warning",
                )
            elif actual_effectiveness < required_effectiveness:
                _push(local, barrier_id, demand_id, "effectiveness_below_required", "fail")
    else:
        _push(local, barrier_id, demand_id, "no_performance_standard", "warning")

    # 3. Demand capacity.
    scenario = demand_value = capacity_value = demand_unit = capacity_margin = None
    if demand is not None:
        scenario = demand["scenario"] or None
        demand_value = demand["demand_value"]
        capacity_value = demand["capacity_value"]
        demand_unit = demand["demand_unit"] or None
        if demand_value is not None and capacity_value is not None:
            capacity_margin = capacity_value - demand_value
            if capacity_margin < 0.0:
                _push(local, barrier_id, demand_id, "demand_exceeds_capacity", "fail")

    # 4. Instrumentation.
    related = [
        device
        for device in register["devices"]
        if not (category == "fire_gas_detection" and device["kind"] == "other")
        and any(
            _matches_text(device["name"], tag)
            or _matches_text(device["tag"], tag)
            or _matches_text(device["location"], tag)
            for tag in tags
        )
    ]
    max_response: float | None = None
    tags_out: list[str] = []
    for device in related:
        identifier = device["tag"] if device["tag"] and device["tag"].strip() else device["name"]
        if identifier not in tags_out:
            tags_out.append(identifier)
        if device["kind"] == "fire_detector":
            response = (
                0.0
                if device["response_time_seconds"] is None
                else device["response_time_seconds"]
            )
        elif device["kind"] == "gas_detector":
            response = (
                10.0
                if device["response_time_seconds"] is None
                else device["response_time_seconds"]
            )
        else:
            response = None
        if response is not None:
            max_response = (
                response if max_response is None else max(max_response, response)
            )
    if category == "fire_gas_detection" and not related:
        _push(local, barrier_id, demand_id, "no_fire_gas_detector", "warning")
    if demand is not None and demand["actual_response_time_seconds"] is None:
        required = demand["required_response_time_seconds"]
        if required is not None and max_response is not None and max_response > required:
            _push(local, barrier_id, demand_id, "detector_response_time_exceeded", "fail")
    for identifier in tags_out:
        instruments.append(
            PerformanceInstrument(barrier_id=barrier_id, demand_id=demand_id, tag=identifier)
        )

    event_related = [
        sif
        for sif in register["event_sifs"]
        if _matches_text(sif["name"], barrier_id)
        or _matches_text(sif["name"], barrier["name"])
        or any(
            _matches_text(detector["name"], tag)
            for detector in register["detectors"]
            if detector["sif_name"] == sif["name"]
            for tag in tags
        )
    ]
    for sif in event_related:
        detectors = [
            row for row in register["detectors"] if row["sif_name"] == sif["name"]
        ]
        count = float(len(detectors))
        bypassed = float(sum(1 for row in detectors if row["bypassed"]))
        faulty = float(sum(1 for row in detectors if row["faulty"]))
        tripped = float(sum(1 for row in detectors if row["tripped"]))
        if not any(row.identifier == sif["name"] for row in local_sifs):
            local_sifs.append(
                PerformanceSif(
                    barrier_id=barrier_id,
                    demand_id=demand_id,
                    identifier=sif["name"],
                    kind="event",
                    voting_notation=sif["voting_notation"],
                    detector_count=count,
                    bypassed_detector_count=bypassed,
                    faulty_detector_count=faulty,
                    tripped_detector_count=tripped,
                    claimed_sil=None,
                    achieved_sil=None,
                    sil_achieved=None,
                    pfd_average=None,
                    pfd_upper=None,
                    risk_reduction_factor=None,
                    architecture=None,
                    proof_test_interval_hours=None,
                    protected_equipment_count=None,
                )
            )
        required_trips, total_sensors = _VOTING[sif["voting_notation"]]
        if sif["overridden"]:
            _push(local, barrier_id, demand_id, "sif_overridden", "fail")
        if count != total_sensors:
            _push(local, barrier_id, demand_id, "sif_detector_count_mismatch", "warning")
        healthy = count - bypassed - faulty
        if healthy < required_trips:
            _push(
                local, barrier_id, demand_id, "sif_healthy_channels_insufficient", "fail"
            )
        elif bypassed > 0.0 or faulty > 0.0:
            _push(local, barrier_id, demand_id, "sif_channels_impaired", "warning")

    quantitative_related = [
        sif
        for sif in register["quantitative_sifs"]
        if _matches_text(sif["id"], barrier_id)
        or _matches_text(sif["name"], barrier_id)
        or _matches_text(sif["name"], barrier["name"])
        or _matches_text(sif["description"], barrier["safety_function"])
        or any(
            _matches_text(row["equipment"], tag)
            for row in register["protected"]
            if row["sif_id"] == sif["id"]
            for tag in tags
        )
    ]
    for sif in quantitative_related:
        identifier = sif["name"].strip() if sif["name"] else sif["id"].strip() or "quantitativeSIF"
        interval = (
            0.0 if sif["test_interval_hours"] is None else sif["test_interval_hours"]
        )
        verification = sil_verification(
            sif["pfd_average"], sif["claimed_sil"], sif["architecture"], interval
        )
        protected_count = float(
            sum(1 for row in register["protected"] if row["sif_id"] == sif["id"])
        )
        if not any(row.identifier == identifier for row in local_sifs):
            local_sifs.append(
                PerformanceSif(
                    barrier_id=barrier_id,
                    demand_id=demand_id,
                    identifier=identifier,
                    kind="quantitative",
                    voting_notation=None,
                    detector_count=None,
                    bypassed_detector_count=None,
                    faulty_detector_count=None,
                    tripped_detector_count=None,
                    claimed_sil=verification.claimed_sil,
                    achieved_sil=verification.achieved_sil,
                    sil_achieved=verification.sil_achieved,
                    pfd_average=verification.pfd_average,
                    pfd_upper=verification.pfd_upper,
                    risk_reduction_factor=(
                        1.0 / sif["pfd_average"] if sif["pfd_average"] > 0.0 else None
                    ),
                    architecture=sif["architecture"],
                    proof_test_interval_hours=interval,
                    protected_equipment_count=protected_count,
                )
            )
        if not verification.sil_achieved or verification.error_count > 0.0:
            _push(
                local, barrier_id, demand_id, "quantitative_sil_not_achieved", "fail"
            )
        for issue in verification.issues:
            if issue.severity == "warning":
                _push(
                    local,
                    barrier_id,
                    demand_id,
                    "quantitative_sif_verification_warning",
                    "warning",
                )

    # 5. Demand completeness.
    if demand is None and category in ("firewater_deluge", "passive_fire_protection"):
        _push(local, barrier_id, demand_id, "fire_protection_demand_missing", "warning")

    if any(finding.severity == "fail" for finding in local):
        verdict = "fail"
    elif any(finding.severity == "warning" for finding in local):
        verdict = "pass_with_warnings"
    else:
        verdict = "pass"
    findings.extend(local)
    sifs.extend(local_sifs)
    return PerformanceAssessment(
        barrier_id=barrier_id,
        barrier_name=barrier["name"] or "",
        demand_id=demand_id,
        category=category,
        verdict=verdict,
        barrier_status=barrier["status"],
        barrier_pfd=barrier["pfd"],
        barrier_effectiveness=effectiveness,
        has_traceable_evidence=barrier["has_traceable_evidence"],
        performance_standard_id=standard_out,
        target_pfd=target_pfd,
        actual_pfd=actual_pfd,
        required_availability=required_availability,
        actual_availability=actual_availability,
        required_response_time_seconds=(
            None if required_response is None else from_si(required_response, "s")
        ),
        actual_response_time_seconds=(
            None if actual_response is None else from_si(actual_response, "s")
        ),
        required_effectiveness=required_effectiveness,
        actual_effectiveness=actual_effectiveness,
        scenario=scenario,
        demand_value=demand_value,
        capacity_value=capacity_value,
        demand_unit=demand_unit,
        capacity_margin=capacity_margin,
        related_instrument_count=float(len(related)),
        max_instrument_response_time_seconds=(
            None if max_response is None else from_si(max_response, "s")
        ),
        related_sif_count=float(len(event_related)),
        related_quantitative_sif_count=float(len(quantitative_related)),
        finding_count=float(len(local)),
    )
