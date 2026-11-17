"""``process.s001_clause10_review`` - the NORSOK S-001 Clause 10 review.

As NeqSim's ``ProcessSafetySystemReviewEngine`` computes it. Spec:
``specs/models/process/s001_clause10_review.toml``

The Python twin of ``crates/azoth-process/src/models/s001_clause10_review.rs``: the
fifteen checks, the coverage result, the confidence and the verdict cascade.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    OverallVerdict,
    ReviewAssessment,
    ReviewCoverage,
    ReviewVerdict,
    S001Clause10ReviewResult,
)
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._safety_review import s001_evaluate

MODEL_ID = "process.s001_clause10_review"

_PASS = "pass"
_WARNING = "warning"
_FAIL = "fail"
_INFO = "info"
_NOT_APPLICABLE = "not_applicable"

_LOW = "low"
_MEDIUM = "medium"
_HIGH = "high"
_CRITICAL = "critical"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _parse_java_double(raw: str) -> float | None:
    """Java's ``Double.parseDouble`` on a trimmed string, with the type suffix it accepts."""
    text = raw.strip()
    lower = text.lower()
    if lower == "nan":
        return math.nan
    if lower in ("infinity", "+infinity"):
        return math.inf
    if lower == "-infinity":
        return -math.inf
    if text and text[-1] in "fFdD":
        text = text[:-1]
    try:
        return float(text)
    except ValueError:
        return None


class _Item:
    __slots__ = ("function_id", "function_type", "references", "values")

    def __init__(self, function_id: str, function_type: str) -> None:
        self.function_id = function_id
        self.function_type = function_type
        self.values: dict[str, str | None] = {}
        self.references: list[str] = []

    def contains_type(self, tokens: tuple[str, ...]) -> bool:
        kind = self.function_type.lower()
        return any(token in kind for token in tokens)

    def has(self, key: str) -> bool:
        return key in self.values

    def has_any(self, keys: tuple[str, ...]) -> bool:
        return any(key in self.values for key in keys)

    def get_bool(self, keys: tuple[str, ...]) -> bool | None:
        for key in keys:
            if key not in self.values:
                continue
            value = self.values[key]
            if value is None:
                continue
            text = value.strip()
            lower = text.lower()
            if lower in ("true", "yes", "y") or text == "1":
                return True
            if lower in ("false", "no", "n") or text == "0":
                return False
        return None

    def get_bool_default(self, default: bool, keys: tuple[str, ...]) -> bool:
        value = self.get_bool(keys)
        return default if value is None else value

    def get_double(self, default: float, keys: tuple[str, ...]) -> float:
        for key in keys:
            if key not in self.values:
                continue
            value = self.values[key]
            if value is None:
                return default
            parsed = _parse_java_double(value)
            return default if parsed is None else parsed
        return default


def _is_psd(item: _Item) -> bool:
    return item.contains_type(("psd", "process shutdown", "shutdown")) or item.has_any(
        ("psdValveTag", "shutdownActionDefined", "psdValveFailsSafe")
    )


def _is_psv(item: _Item) -> bool:
    return item.contains_type(("psv", "relief", "safety valve")) or item.has_any(
        ("psvTag", "psvSizingBasisDocumented", "requiredReliefLoadKgPerS")
    )


def _is_alarm(item: _Item) -> bool:
    return item.contains_type(("alarm", "operator action")) or item.has_any(
        ("alarmTag", "alarmActionDefined", "operatorActionDefined")
    )


def _is_logic(item: _Item) -> bool:
    return item.contains_type(("sif", "logic", "sis", "psd")) or item.has_any(
        ("logicSolverCertified", "logicSolverIndependent", "causeAndEffectTested")
    )


def _is_secondary(item: _Item) -> bool:
    return item.contains_type(
        ("secondary_pressure", "secondary pressure", "10.4.7", "overpressure")
    ) or item.has_any(("maximumEventPressureBara", "eventPressureBara", "demandFrequencyPerYear"))


def _is_action_or_instrumented(item: _Item) -> bool:
    return (
        _is_psd(item)
        or _is_alarm(item)
        or _is_logic(item)
        or _is_secondary(item)
        or item.has_any(("requiredResponseTimeSeconds", "actualResponseTimeSeconds"))
    )


def _has_type(items: list[_Item], token: str) -> bool:
    lower = token.lower()
    for item in items:
        if lower in item.function_type.lower():
            return True
        matched = {
            "PSD": _is_psd,
            "PSV": _is_psv,
            "ALARM": _is_alarm,
            "SIF": _is_logic,
            "SECONDARY_PRESSURE_PROTECTION": _is_secondary,
        }.get(token)
        if matched is not None and matched(item):
            return True
    return False


def _push(
    assessments: list[ReviewAssessment],
    function_id: str,
    requirement_id: str,
    clause: str,
    status: str,
    severity: str,
) -> None:
    assessments.append(
        ReviewAssessment(
            function_id=function_id,
            requirement_id=requirement_id,
            clause=clause,
            status=status,
            severity=severity,
        )
    )


def _required_bool(
    item: _Item,
    assessments: list[ReviewAssessment],
    requirement_id: str,
    clause: str,
    keys: tuple[str, ...],
) -> None:
    value = item.get_bool(keys)
    if value is True:
        _push(assessments, item.function_id, requirement_id, clause, _PASS, _LOW)
    elif value is False:
        _push(assessments, item.function_id, requirement_id, clause, _FAIL, _CRITICAL)
    else:
        _push(assessments, item.function_id, requirement_id, clause, _WARNING, _HIGH)


def _not_applicable(
    item: _Item, assessments: list[ReviewAssessment], requirement_id: str, clause: str
) -> None:
    _push(assessments, item.function_id, requirement_id, clause, _NOT_APPLICABLE, _LOW)


def _compare_times(
    item: _Item,
    assessments: list[ReviewAssessment],
    requirement_id: str,
    clause: str,
    actual_key: str,
    limit_key: str,
) -> None:
    actual = item.get_double(math.nan, (actual_key,))
    limit = item.get_double(math.nan, (limit_key,))
    if not math.isfinite(actual) or not math.isfinite(limit):
        _push(assessments, item.function_id, requirement_id, clause, _WARNING, _MEDIUM)
    elif actual <= limit:
        _push(assessments, item.function_id, requirement_id, clause, _PASS, _LOW)
    else:
        _push(assessments, item.function_id, requirement_id, clause, _FAIL, _HIGH)


def _compare_minimum(
    item: _Item,
    assessments: list[ReviewAssessment],
    requirement_id: str,
    clause: str,
    minimum_key: str,
    provided_key: str,
) -> None:
    minimum = item.get_double(math.nan, (minimum_key,))
    provided = item.get_double(math.nan, (provided_key,))
    if not math.isfinite(minimum) or not math.isfinite(provided):
        _push(assessments, item.function_id, requirement_id, clause, _WARNING, _MEDIUM)
    elif provided + 1.0e-12 >= minimum:
        _push(assessments, item.function_id, requirement_id, clause, _PASS, _LOW)
    else:
        _push(assessments, item.function_id, requirement_id, clause, _FAIL, _HIGH)


def _finite(value: float) -> float | None:
    return value if math.isfinite(value) else None


def _evaluate_secondary(item: _Item, assessments: list[ReviewAssessment]) -> None:
    event = item.get_double(
        math.nan,
        (
            "maximumEventPressureBara",
            "eventPressureBara",
            "protectedSystemMaximumPressureBara",
        ),
    )
    design = item.get_double(
        math.nan, ("designPressureBara", "protectedEquipmentDesignPressureBara")
    )
    test = item.get_double(
        math.nan, ("testPressureBara", "hydrotestPressureBara", "pressureTestBara")
    )
    demand = item.get_double(
        math.nan,
        ("demandFrequencyPerYear", "annualDemandFrequency", "eventFrequencyPerYear"),
    )
    target = item.get_double(math.nan, ("targetFrequencyPerYear", "maximumAllowedFrequencyPerYear"))
    assessed = item.get_bool(
        ("reliefLeakageAssessed", "psvLeakageAssessed", "reliefValveLeakageDocumented")
    )
    safe = item.get_bool(
        ("reliefLeakageToSafeLocation", "psvLeakageToSafeLocation", "leakageRoutedSafe")
    )
    proof = item.get_double(
        math.nan, ("proofTestIntervalMonths", "testIntervalMonths", "proofTestMonths")
    )
    empty = (
        not math.isfinite(event)
        and not math.isfinite(design)
        and not math.isfinite(test)
        and not math.isfinite(demand)
        and not math.isfinite(proof)
        and assessed is None
        and safe is None
    )
    if empty:
        _push(
            assessments,
            item.function_id,
            "PSS-10.4.7-SECONDARY",
            "10.4.7",
            _WARNING,
            _HIGH,
        )
        return
    outcome = s001_evaluate(
        _finite(event),
        _finite(design),
        _finite(test),
        _finite(demand),
        _finite(target),
        assessed,
        safe,
        _finite(proof),
    )
    if outcome["acceptable"]:
        _push(assessments, item.function_id, "PSS-10.4.7-SECONDARY", "10.4.7", _PASS, _LOW)
    elif not outcome["pressure_basis_complete"] or not outcome["frequency_configured"]:
        _push(assessments, item.function_id, "PSS-10.4.7-SECONDARY", "10.4.7", _WARNING, _HIGH)
    else:
        _push(assessments, item.function_id, "PSS-10.4.7-SECONDARY", "10.4.7", _FAIL, _CRITICAL)


def _evaluate_item(item: _Item) -> list[ReviewAssessment]:
    assessments: list[ReviewAssessment] = []

    # Safety lifecycle.
    for requirement, keys in (
        (
            "PSS-LIFECYCLE-HAZID-HAZOP-LOPA",
            (
                "hazidHazopLopaCompleted",
                "hazardAnalysisCompleted",
                "riskAssessmentCompleted",
                "lopaCompleted",
            ),
        ),
        (
            "PSS-LIFECYCLE-SRS",
            (
                "srsDefinesRequiredFunctions",
                "srsDocumented",
                "srsReferenceDocumented",
                "silRequirementsDocumented",
            ),
        ),
        (
            "PSS-LIFECYCLE-DESIGN-IMPLEMENTATION",
            (
                "sisEsdFgsDesignImplemented",
                "safetySystemDesignImplemented",
                "designImplementationVerified",
                "causeAndEffectImplemented",
            ),
        ),
        (
            "PSS-LIFECYCLE-VERIFY-TEST-OPERATE",
            (
                "verificationTestingOperationConfirmed",
                "verificationPlanDocumented",
                "proofTestProgramDocumented",
                "operationsMaintenancePlanDocumented",
            ),
        ),
    ):
        _required_bool(item, assessments, requirement, "Safety lifecycle", keys)

    # Role and interfaces.
    _required_bool(
        item,
        assessments,
        "PSS-10.1-ROLE",
        "10.1",
        ("processSafetyRoleDefined", "roleDefined", "safetyFunctionDefined"),
    )
    _required_bool(
        item,
        assessments,
        "PSS-10.1-INTERFACES",
        "10.1",
        ("interfacesDefined", "systemInterfacesDefined", "interfaceRegisterDocumented"),
    )

    # Means of protection.
    _required_bool(
        item,
        assessments,
        "PSS-10.3-PROTECTION-LAYERS",
        "10.3",
        (
            "protectionLayersDocumented",
            "meansOfProtectionDocumented",
            "protectionStrategyDocumented",
        ),
    )
    _required_bool(
        item,
        assessments,
        "PSS-10.3-DESIGN-BASIS",
        "10.3",
        ("designBasisDocumented", "scenarioBasisDocumented", "hazardBasisDocumented"),
    )

    # Process-safety principles.
    _required_bool(
        item,
        assessments,
        "PSS-10.4.1-PRINCIPLES",
        "10.4.1",
        (
            "processSafetyPrinciplesDocumented",
            "safeStateDefined",
            "failSafeStateDocumented",
        ),
    )
    _required_bool(
        item,
        assessments,
        "PSS-10.4.1-BYPASS",
        "10.4.1",
        (
            "bypassManagementDocumented",
            "overrideManagementDocumented",
            "impairmentManagementDocumented",
        ),
    )

    # PSD valves.
    if _is_psd(item):
        _required_bool(
            item,
            assessments,
            "PSS-10.4.2-SHUTDOWN-ACTION",
            "10.4.2",
            ("shutdownActionDefined", "psdActionDefined", "executiveActionDefined"),
        )
        _required_bool(
            item,
            assessments,
            "PSS-10.4.2-FAILSAFE",
            "10.4.2",
            ("psdValveFailsSafe", "valveFailsSafe", "finalElementFailsSafe"),
        )
        _required_bool(
            item,
            assessments,
            "PSS-10.4.2-ISOLATION",
            "10.4.2",
            (
                "psdValveIsolationAdequate",
                "shutdownIsolationAdequate",
                "leakTightClassDocumented",
            ),
        )
    else:
        _not_applicable(item, assessments, "PSS-10.4.2-PSD", "10.4.2")

    # PSV protection.
    if _is_psv(item):
        _required_bool(
            item,
            assessments,
            "PSS-10.4.3-SIZING-BASIS",
            "10.4.3",
            (
                "psvSizingBasisDocumented",
                "reliefScenarioDocumented",
                "reliefLoadDocumented",
            ),
        )
        _required_bool(
            item,
            assessments,
            "PSS-10.4.3-PROTECTED-EQUIPMENT",
            "10.4.3",
            ("protectedEquipmentDocumented", "protectedEquipmentTagDocumented"),
        )
        required = item.get_double(
            math.nan,
            (
                "requiredReliefLoadKgPerS",
                "reliefLoadKgPerS",
                "requiredCapacityKgPerS",
            ),
        )
        capacity = item.get_double(
            math.nan,
            ("psvCapacityKgPerS", "ratedCapacityKgPerS", "reliefCapacityKgPerS"),
        )
        if math.isfinite(required) and math.isfinite(capacity):
            status = _PASS if capacity + 1.0e-12 >= required else _FAIL
            severity = _LOW if status == _PASS else _CRITICAL
            _push(assessments, item.function_id, "PSS-10.4.3-CAPACITY", "10.4.3", status, severity)
        else:
            _push(
                assessments,
                item.function_id,
                "PSS-10.4.3-CAPACITY",
                "10.4.3",
                _WARNING,
                _HIGH,
            )
    else:
        _not_applicable(item, assessments, "PSS-10.4.3-PSV", "10.4.3")

    # Alarm actions.
    if _is_alarm(item):
        _required_bool(
            item,
            assessments,
            "PSS-10.4.4-ACTION",
            "10.4.4",
            ("alarmActionDefined", "operatorActionDefined", "alarmResponseDefined"),
        )
        _required_bool(
            item,
            assessments,
            "PSS-10.4.4-SETPOINT",
            "10.4.4",
            ("alarmSetpointDocumented", "tripSetpointDocumented", "actionLimitDocumented"),
        )
        _compare_times(
            item,
            assessments,
            "PSS-10.4.4-OPERATOR-TIME",
            "10.4.4",
            "operatorResponseTimeSeconds",
            "availableOperatorResponseTimeSeconds",
        )
    else:
        _not_applicable(item, assessments, "PSS-10.4.4-ALARM", "10.4.4")

    # Response time.
    if _is_action_or_instrumented(item):
        _compare_times(
            item,
            assessments,
            "PSS-10.4.5-RESPONSE",
            "10.4.5",
            "actualResponseTimeSeconds",
            "requiredResponseTimeSeconds",
        )
    else:
        _not_applicable(item, assessments, "PSS-10.4.5-RESPONSE", "10.4.5")

    # Logic solver.
    if _is_logic(item):
        _required_bool(
            item,
            assessments,
            "PSS-10.4.6-CERTIFIED",
            "10.4.6",
            ("logicSolverCertified", "logicSolverSuitable", "sisLogicSolverCertified"),
        )
        _required_bool(
            item,
            assessments,
            "PSS-10.4.6-INDEPENDENCE",
            "10.4.6",
            (
                "logicSolverIndependent",
                "independentFromControlSystem",
                "sisBpcsIndependenceDocumented",
            ),
        )
        _required_bool(
            item,
            assessments,
            "PSS-10.4.6-TESTED",
            "10.4.6",
            ("causeAndEffectTested", "logicFunctionTested", "sifProofTested"),
        )
    else:
        _not_applicable(item, assessments, "PSS-10.4.6-LOGIC", "10.4.6")

    # Secondary pressure protection.
    if _is_secondary(item):
        _evaluate_secondary(item, assessments)
    else:
        _not_applicable(item, assessments, "PSS-10.4.7-SECONDARY", "10.4.7")

    # Utility dependencies.
    _required_bool(
        item,
        assessments,
        "PSS-10.4.8-UTILITY-LIST",
        "10.4.8",
        (
            "requiredUtilitiesIdentified",
            "utilityDependenciesIdentified",
            "utilityMatrixDocumented",
        ),
    )
    utility_dependent = item.get_bool(
        ("utilityDependent", "dependsOnUtility", "requiresUtilityForAction")
    )
    if utility_dependent is True:
        _required_bool(
            item,
            assessments,
            "PSS-10.4.8-UTILITY-LOSS",
            "10.4.8",
            (
                "failSafeOnUtilityLoss",
                "safeOnUtilityFailure",
                "utilityFailureSafeStateDocumented",
            ),
        )
    else:
        _push(assessments, item.function_id, "PSS-10.4.8-UTILITY-LOSS", "10.4.8", _INFO, _LOW)

    # PSD principles.
    if _is_psd(item):
        _required_bool(
            item,
            assessments,
            "PSS-10.4.9-PSD-INDEPENDENCE",
            "10.4.9",
            (
                "psdIndependentFromControl",
                "psdIndependenceDocumented",
                "independentFromControlSystem",
            ),
        )
        _required_bool(
            item,
            assessments,
            "PSS-10.4.9-PSD-MANUAL",
            "10.4.9",
            ("manualShutdownAvailable", "manualInitiationDocumented", "manualPsdAvailable"),
        )
    else:
        _not_applicable(item, assessments, "PSS-10.4.9-PSD", "10.4.9")

    # Survivability.
    _required_bool(
        item,
        assessments,
        "PSS-10.4.10-SURVIVABILITY",
        "10.4.10",
        (
            "survivabilityRequirementDocumented",
            "fireBlastSurvivabilityDocumented",
            "survivabilityDocumented",
        ),
    )
    _compare_minimum(
        item,
        assessments,
        "PSS-10.4.10-ENDURANCE",
        "10.4.10",
        "requiredSurvivabilityTimeMin",
        "survivabilityTimeMin",
    )

    # Operational evidence.
    if item.has_any(
        (
            "tagreaderSource",
            "instrumentDataSource",
            "bypassActive",
            "overrideActive",
            "proofTestOverdue",
            "tripDemandFailures",
            "observedResponseTimeSeconds",
        )
    ):
        bypass = item.get_bool_default(False, ("bypassActive", "sifBypassed", "psdBypassed"))
        override = item.get_bool_default(
            False, ("overrideActive", "sifOverridden", "psdOverridden")
        )
        overdue = item.get_bool_default(False, ("proofTestOverdue", "testOverdue"))
        failures = item.get_double(0.0, ("tripDemandFailures", "failedDemandCount"))
        if bypass or override:
            _push(
                assessments,
                item.function_id,
                "PSS-OPS-INSTRUMENT-DATA",
                "Operational evidence",
                _FAIL,
                _HIGH,
            )
        elif overdue or failures > 0.0:
            _push(
                assessments,
                item.function_id,
                "PSS-OPS-INSTRUMENT-DATA",
                "Operational evidence",
                _WARNING,
                _MEDIUM,
            )
        else:
            _push(
                assessments,
                item.function_id,
                "PSS-OPS-INSTRUMENT-DATA",
                "Operational evidence",
                _PASS,
                _LOW,
            )
    else:
        _push(
            assessments,
            item.function_id,
            "PSS-OPS-INSTRUMENT-DATA",
            "Operational evidence",
            _INFO,
            _LOW,
        )

    # Evidence quality.
    if not item.references:
        _push(
            assessments,
            item.function_id,
            "PSS-EVIDENCE-SOURCES",
            "Evidence quality",
            _WARNING,
            _MEDIUM,
        )
    else:
        _push(
            assessments,
            item.function_id,
            "PSS-EVIDENCE-SOURCES",
            "Evidence quality",
            _PASS,
            _LOW,
        )

    return assessments


def _confidence(item: _Item) -> float:
    score = 0.15
    if item.references:
        score += 0.20
    if item.has_any(("designBasisDocumented", "scenarioBasisDocumented")):
        score += 0.15
    if item.has_any(
        (
            "hazidHazopLopaCompleted",
            "srsDefinesRequiredFunctions",
            "verificationTestingOperationConfirmed",
        )
    ):
        score += 0.10
    if item.has_any(
        (
            "requiredResponseTimeSeconds",
            "actualResponseTimeSeconds",
            "operatorResponseTimeSeconds",
        )
    ):
        score += 0.15
    if item.has_any(("logicSolverCertified", "psvCapacityKgPerS", "maximumEventPressureBara")):
        score += 0.15
    if item.has_any(("tagreaderSource", "instrumentDataSource", "bypassActive")):
        score += 0.10
    return min(1.0, score)


def _verdict(assessments: list[ReviewAssessment]) -> str:
    if any(assessment.status == _FAIL for assessment in assessments):
        return "fail"
    if any(assessment.status == _WARNING for assessment in assessments):
        return "pass_with_warnings"
    return "pass"


_COVERAGE = (
    ("psd", "PSD", "PSS-COVERAGE-PSD", "10.4.2"),
    ("psv", "PSV", "PSS-COVERAGE-PSV", "10.4.3"),
    ("alarm", "ALARM", "PSS-COVERAGE-ALARM", "10.4.4"),
    ("sif", "SIF", "PSS-COVERAGE-SIF", "10.4.6"),
    (
        "secondary_pressure_protection",
        "SECONDARY_PRESSURE_PROTECTION",
        "PSS-COVERAGE-SECONDARY",
        "10.4.7",
    ),
)


def s001_clause10_review(
    items: list[dict[str, object]] | None = None,
    evidence: list[dict[str, object]] | None = None,
    references: list[dict[str, object]] | None = None,
) -> S001Clause10ReviewResult:
    """The Clause 10 review.

    Args:
        items: the function rows, each ``{function_id, function_type, equipment_tag}``.
        evidence: the key-value rows, each ``{function_id, key, value}``.
        references: the reference rows, each ``{function_id, reference}``.

    Returns:
        The assessments, verdicts, coverage rows, overall verdict and the counts.

    Raises:
        InvalidInputError: for a duplicate function id, a duplicate evidence key, or an
            evidence or reference row naming no item.
    """
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    item_rows = input_records_to_si(spec, "items", items or [])
    evidence_rows = input_records_to_si(spec, "evidence", evidence or [])
    reference_rows = input_records_to_si(spec, "references", references or [])

    reviewed: list[_Item] = []
    for row in item_rows:
        function_id = str(row["function_id"])
        if any(item.function_id == function_id for item in reviewed):
            raise InvalidInputError("items.function_id", f"`{function_id}` appears more than once")
        reviewed.append(
            _Item(
                function_id,
                "" if row.get("function_type") is None else str(row["function_type"]),
            )
        )

    for index, row in enumerate(evidence_rows):
        item = next(
            (entry for entry in reviewed if entry.function_id == str(row["function_id"])), None
        )
        if item is None:
            raise InvalidInputError(
                f"evidence[{index}].function_id",
                f"`{row['function_id']}` names no item row",
            )
        key = str(row["key"])
        if key in item.values:
            raise InvalidInputError(
                f"evidence[{index}].key",
                f"`{key}` appears more than once for `{row['function_id']}`",
            )
        value = row.get("value")
        item.values[key] = None if value is None else str(value)

    for index, row in enumerate(reference_rows):
        item = next(
            (entry for entry in reviewed if entry.function_id == str(row["function_id"])), None
        )
        if item is None:
            raise InvalidInputError(
                f"references[{index}].function_id",
                f"`{row['function_id']}` names no item row",
            )
        item.references.append(str(row["reference"]))

    coverage = [
        ReviewCoverage(
            function_type=function_type,
            status="pass" if _has_type(reviewed, token) else "warning",
        )
        for function_type, token, _, _ in _COVERAGE
    ]
    assessments: list[ReviewAssessment] = [
        ReviewAssessment(
            function_id="CLAUSE10-COVERAGE",
            requirement_id=requirement_id,
            clause=clause,
            status=row.status,
            severity=_LOW if row.status == "pass" else _MEDIUM,
        )
        for row, (_, _, requirement_id, clause) in zip(coverage, _COVERAGE, strict=True)
    ]
    verdicts: list[ReviewVerdict] = [
        ReviewVerdict(
            function_id="CLAUSE10-COVERAGE",
            verdict=_verdict(assessments),
            confidence=1.0,
        )
    ]
    for item in reviewed:
        item_assessments = _evaluate_item(item)
        verdicts.append(
            ReviewVerdict(
                function_id=item.function_id,
                verdict=_verdict(item_assessments),
                confidence=_confidence(item),
            )
        )
        assessments.extend(item_assessments)

    if any(row.verdict == "fail" for row in verdicts):
        overall = OverallVerdict.FAIL
    elif any(row.verdict == "pass_with_warnings" for row in verdicts):
        overall = OverallVerdict.PASS_WITH_WARNINGS
    else:
        overall = OverallVerdict.PASS

    return S001Clause10ReviewResult(
        assessments=tuple(assessments),
        verdicts=tuple(verdicts),
        coverage=tuple(coverage),
        overall_verdict=overall,
        item_count=float(len(verdicts)),
        failed_item_count=float(sum(1 for row in verdicts if row.verdict == "fail")),
        warning_item_count=float(sum(1 for row in verdicts if row.verdict == "pass_with_warnings")),
        assessment_count=float(len(assessments)),
        fail_assessment_count=float(sum(1 for row in assessments if row.status == _FAIL)),
        warning_assessment_count=float(sum(1 for row in assessments if row.status == _WARNING)),
        warnings=tuple(warnings),
    )
