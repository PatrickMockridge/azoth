"""``process.norsok_p002_compliance`` - the NORSOK P-002 point-check set.

As NeqSim's ``NorsokP002ComplianceChecker`` computes it. Spec:
``specs/models/process/norsok_p002_compliance.toml``

The Python twin of ``crates/azoth-process/src/models/norsok_p002_compliance.rs``: five
numeric comparisons against the class's defaults and two caller-supplied recorded checks.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    NorsokP002ComplianceResult,
    P002Criterion,
    P002FindingResult,
)
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.norsok_p002_compliance"

#: The class's flare-line Mach limit.
FLARE_MACH_LIMIT = 0.7
#: The class's blowdown momentum-flux limit, Pa.
BLOWDOWN_MOMENTUM_FLUX_LIMIT = 200000.0
#: The class's erosional momentum-flux limit, Pa.
EROSIONAL_MOMENTUM_FLUX_LIMIT = 100000.0
#: The class's vent-gas velocity limit, m/s.
VENT_VELOCITY_LIMIT = 60.0
#: The class's liquid carry-over limit, volume fraction.
CARRY_OVER_LIMIT = 1.0e-3


class _Finding(NamedTuple):
    """One check's answer, before the output row is built."""

    name: str
    criterion: P002Criterion
    compliant: bool


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


def _forbid(present: bool, criterion: P002Criterion, field: str) -> None:
    """Refuse a field the criterion does not read."""
    if present:
        raise InvalidInputError(field, f"`{criterion.value}` does not read `{field}`")


def _criterion(name: str) -> P002Criterion:
    """Parse the spec's criterion spelling."""
    try:
        return P002Criterion(name)
    except ValueError as error:
        raise InvalidInputError(
            "criterion", "the criterion must be one of the seven NORSOK P-002 checks"
        ) from error


def _evaluate(
    checks: Sequence[Mapping[str, object]],
) -> tuple[list[_Finding], list[Warning]]:
    """The class's comparisons over the caller's rows; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    rows: list[dict[str, Any]] = input_records_to_si(spec, "checks", checks)
    findings: list[_Finding] = []
    for row in rows:
        name = str(row["name"])
        criterion = _criterion(str(row["criterion"]))
        if criterion is P002Criterion.FLARE_LINE_MACH_07:
            value = row.get("mach_number")
            if value is None:
                raise InvalidInputError("mach_number", "the flare-line Mach check reads it")
            for field in (
                "momentum_flux",
                "momentum_flux_limit",
                "vent_velocity",
                "vent_velocity_limit",
                "carry_over_fraction",
                "carry_over_limit",
                "recorded_ok",
            ):
                _forbid(row.get(field) is not None, criterion, field)
            limit = row.get("mach_limit")
            limit = FLARE_MACH_LIMIT if limit is None else float(limit)
            _require_finite("mach_number", float(value))
            _require_finite("mach_limit", limit)
            compliant = float(value) <= limit
        elif criterion in (
            P002Criterion.BLOWDOWN_RHO_V2,
            P002Criterion.EROSIONAL_VELOCITY,
        ):
            value = row.get("momentum_flux")
            if value is None:
                raise InvalidInputError("momentum_flux", "the momentum-flux check reads it")
            for field in (
                "mach_number",
                "mach_limit",
                "vent_velocity",
                "vent_velocity_limit",
                "carry_over_fraction",
                "carry_over_limit",
                "recorded_ok",
            ):
                _forbid(row.get(field) is not None, criterion, field)
            default_limit = (
                BLOWDOWN_MOMENTUM_FLUX_LIMIT
                if criterion is P002Criterion.BLOWDOWN_RHO_V2
                else EROSIONAL_MOMENTUM_FLUX_LIMIT
            )
            limit = row.get("momentum_flux_limit")
            limit = default_limit if limit is None else float(limit)
            _require_finite("momentum_flux", float(value))
            _require_finite("momentum_flux_limit", limit)
            compliant = float(value) <= limit
        elif criterion is P002Criterion.VENT_GAS_VELOCITY:
            value = row.get("vent_velocity")
            if value is None:
                raise InvalidInputError("vent_velocity", "the vent-velocity check reads it")
            for field in (
                "mach_number",
                "mach_limit",
                "momentum_flux",
                "momentum_flux_limit",
                "carry_over_fraction",
                "carry_over_limit",
                "recorded_ok",
            ):
                _forbid(row.get(field) is not None, criterion, field)
            limit = row.get("vent_velocity_limit")
            limit = VENT_VELOCITY_LIMIT if limit is None else float(limit)
            _require_finite("vent_velocity", float(value))
            _require_finite("vent_velocity_limit", limit)
            compliant = float(value) <= limit
        elif criterion is P002Criterion.LIQUID_CARRY_OVER:
            value = row.get("carry_over_fraction")
            if value is None:
                raise InvalidInputError(
                    "carry_over_fraction", "the liquid carry-over check reads it"
                )
            for field in (
                "mach_number",
                "mach_limit",
                "momentum_flux",
                "momentum_flux_limit",
                "vent_velocity",
                "vent_velocity_limit",
                "recorded_ok",
            ):
                _forbid(row.get(field) is not None, criterion, field)
            limit = row.get("carry_over_limit")
            limit = CARRY_OVER_LIMIT if limit is None else float(limit)
            _require_finite("carry_over_fraction", float(value))
            _require_finite("carry_over_limit", limit)
            compliant = float(value) <= limit
        else:
            passed = row.get("recorded_ok")
            if passed is None:
                raise InvalidInputError(
                    "recorded_ok", "a recorded criterion reads the caller's pass flag"
                )
            for field in (
                "mach_number",
                "mach_limit",
                "momentum_flux",
                "momentum_flux_limit",
                "vent_velocity",
                "vent_velocity_limit",
                "carry_over_fraction",
                "carry_over_limit",
            ):
                _forbid(row.get(field) is not None, criterion, field)
            compliant = bool(passed)
        findings.append(_Finding(name=name, criterion=criterion, compliant=compliant))

    return findings, warnings


def norsok_p002_compliance(
    checks: Sequence[Mapping[str, object]],
) -> NorsokP002ComplianceResult:
    """The NORSOK P-002 point checks and their findings.

    Each numeric check is ``value <= limit``, so equality passes; the defaults are the
    class's. The two recorded criteria read only the caller's pass flag. A row states only
    the field its criterion reads. **A passing report covers the supplied checks alone**,
    not conformity with the whole standard.

    Args:
        checks: the point checks, in caller order.

    Returns:
        One finding per check, the overall compliance and the non-compliant count.

    Raises:
        InvalidInputError: for an unknown criterion, a row missing the field its criterion
            reads, a row carrying another criterion's field, or a non-finite number.
    """
    findings, warnings = _evaluate(checks)
    non_compliant_count = float(sum(1 for row in findings if not row.compliant))
    return NorsokP002ComplianceResult(
        findings=tuple(
            P002FindingResult(
                name=row.name,
                criterion=row.criterion,
                compliant=row.compliant,
            )
            for row in findings
        ),
        compliant=non_compliant_count == 0.0,
        non_compliant_count=non_compliant_count,
        warnings=tuple(warnings),
    )
