"""``process.sts0131_gate`` - the STS-0131 acceptance gate.

As NeqSim's ``Sts0131Gate`` computes it. Spec: ``specs/models/process/sts0131_gate.toml``

The Python twin of ``crates/azoth-process/src/models/sts0131_gate.rs``: the PSV area margin,
the MDMT comparison and the four pass-through checks, with no warning arm.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    Sts0131CheckType,
    Sts0131FindingResult,
    Sts0131GateResult,
    Sts0131Severity,
)
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.sts0131_gate"


class _Finding(NamedTuple):
    """One check's answer, before the output row is built."""

    check_type: Sts0131CheckType
    severity: Sts0131Severity
    name: str | None
    margin: float | None


def _require_finite(name: str, value: float) -> None:
    """Refuse a value the arithmetic cannot carry to a published comparison."""
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation compares"
        )


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _forbid(present: bool, check: Sts0131CheckType, field: str) -> None:
    """Refuse a field the check does not read."""
    if present:
        raise InvalidInputError(field, f"`{check.value}` does not read `{field}`")


def _check_type(name: str) -> Sts0131CheckType:
    """Parse the spec's check-type spelling."""
    try:
        return Sts0131CheckType(name)
    except ValueError as error:
        raise InvalidInputError(
            "check_type", "the check must be one of the six STS-0131 checks"
        ) from error


def _severity(passed: bool) -> Sts0131Severity:
    """The severity of a pass or fail."""
    return Sts0131Severity.PASS if passed else Sts0131Severity.FAIL


def _evaluate(
    checks: Sequence[Mapping[str, object]],
) -> tuple[list[_Finding], list[Warning]]:
    """The class's gate over the caller's rows; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    rows: list[dict[str, Any]] = input_records_to_si(spec, "checks", checks)
    findings: list[_Finding] = []
    for row in rows:
        check = _check_type(str(row["check_type"]))
        built_in = check is not Sts0131CheckType.CUSTOM
        _forbid(built_in and row.get("name") is not None, check, "name")
        if check is Sts0131CheckType.DEPRESSURISATION:
            passed = row.get("acceptable")
            if passed is None:
                raise InvalidInputError("acceptable", "the depressurisation check reads it")
            for field in (
                "required_area",
                "selected_area",
                "required_margin",
                "minimum_wall_temperature",
                "design_mdmt",
                "rupture_predicted",
                "sil_compliant",
                "custom_ok",
            ):
                _forbid(row.get(field) is not None, check, field)
            severity, margin = _severity(bool(passed)), None
        elif check is Sts0131CheckType.PSV_SIZING_MARGIN:
            required = row.get("required_area")
            selected = row.get("selected_area")
            required_margin = row.get("required_margin")
            if required is None:
                raise InvalidInputError("required_area", "the PSV margin check reads it")
            if selected is None:
                raise InvalidInputError("selected_area", "the PSV margin check reads it")
            if required_margin is None:
                raise InvalidInputError("required_margin", "the PSV margin check reads it")
            if float(required) <= 0.0 or float(selected) <= 0.0:
                raise InvalidInputError("required_area", "the PSV areas must be positive")
            for field in (
                "acceptable",
                "minimum_wall_temperature",
                "design_mdmt",
                "rupture_predicted",
                "sil_compliant",
                "custom_ok",
            ):
                _forbid(row.get(field) is not None, check, field)
            _require_finite("required_area", float(required))
            _require_finite("selected_area", float(selected))
            _require_finite("required_margin", float(required_margin))
            margin = (float(selected) - float(required)) / float(required)
            severity = _severity(margin >= float(required_margin))
        elif check is Sts0131CheckType.MDMT:
            minimum = row.get("minimum_wall_temperature")
            design = row.get("design_mdmt")
            if minimum is None:
                raise InvalidInputError("minimum_wall_temperature", "the MDMT check reads it")
            if design is None:
                raise InvalidInputError("design_mdmt", "the MDMT check reads it")
            for field in (
                "acceptable",
                "required_area",
                "selected_area",
                "required_margin",
                "rupture_predicted",
                "sil_compliant",
                "custom_ok",
            ):
                _forbid(row.get(field) is not None, check, field)
            _require_finite("minimum_wall_temperature", float(minimum))
            _require_finite("design_mdmt", float(design))
            severity, margin = _severity(float(minimum) >= float(design)), None
        elif check is Sts0131CheckType.TRAPPED_LIQUID_RUPTURE:
            ruptured = row.get("rupture_predicted")
            if ruptured is None:
                raise InvalidInputError("rupture_predicted", "the trapped-liquid check reads it")
            for field in (
                "acceptable",
                "required_area",
                "selected_area",
                "required_margin",
                "minimum_wall_temperature",
                "design_mdmt",
                "sil_compliant",
                "custom_ok",
            ):
                _forbid(row.get(field) is not None, check, field)
            severity, margin = _severity(not bool(ruptured)), None
        elif check is Sts0131CheckType.SIL:
            compliant = row.get("sil_compliant")
            if compliant is None:
                raise InvalidInputError("sil_compliant", "the SIL check reads it")
            for field in (
                "acceptable",
                "required_area",
                "selected_area",
                "required_margin",
                "minimum_wall_temperature",
                "design_mdmt",
                "rupture_predicted",
                "custom_ok",
            ):
                _forbid(row.get(field) is not None, check, field)
            severity, margin = _severity(bool(compliant)), None
        else:
            passed = row.get("custom_ok")
            if passed is None:
                raise InvalidInputError("custom_ok", "the custom check reads it")
            if row.get("name") is None:
                raise InvalidInputError("name", "the custom check reads its name")
            for field in (
                "acceptable",
                "required_area",
                "selected_area",
                "required_margin",
                "minimum_wall_temperature",
                "design_mdmt",
                "rupture_predicted",
                "sil_compliant",
            ):
                _forbid(row.get(field) is not None, check, field)
            severity, margin = _severity(bool(passed)), None
        findings.append(
            _Finding(
                check_type=check,
                severity=severity,
                name=None if built_in else str(row["name"]),
                margin=margin,
            )
        )

    return findings, warnings


def sts0131_gate(
    checks: Sequence[Mapping[str, object]],
) -> Sts0131GateResult:
    """The STS-0131 gate's findings and its overall acceptability.

    The PSV margin is ``(selected - required)/required`` and passes at or above the stated
    margin; the MDMT check passes at or above the design MDMT; the trapped-liquid check
    passes when no rupture is predicted. A row states only the field its check reads. The
    class's ``WARNING`` severity is emitted by no builder.

    Args:
        checks: the checks, in caller order.

    Returns:
        One finding per check, the overall acceptability and the failure count.

    Raises:
        InvalidInputError: for an unknown check type, a row missing the field its check
            reads, a row carrying another check's field, a non-positive PSV area, or a
            non-finite number.
    """
    findings, warnings = _evaluate(checks)
    failure_count = float(sum(1 for row in findings if row.severity is Sts0131Severity.FAIL))
    return Sts0131GateResult(
        findings=tuple(
            Sts0131FindingResult(
                check_type=row.check_type,
                severity=row.severity,
                name=row.name,
                margin=row.margin,
            )
            for row in findings
        ),
        acceptable=failure_count == 0.0,
        failure_count=failure_count,
        warnings=tuple(warnings),
    )
