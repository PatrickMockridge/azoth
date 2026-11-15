"""``process.tr3001_compliance`` - the six TR3001 overpressure checks.

As NeqSim's ``TR3001ComplianceChecker`` computes it. Spec:
``specs/models/process/tr3001_compliance.toml``

The Python twin of ``crates/azoth-process/src/models/tr3001_compliance.rs``: the
governing-case rule and the six findings in the class's order.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import Tr3001ComplianceResult, Tr3001Finding
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.tr3001_compliance"

_CAUSES = (
    "blocked_outlet",
    "check_valve_leakage",
    "gas_blow_by",
    "liquid_blow_by",
    "volatile_liquid_ingress",
    "inadvertent_valve_opening",
    "control_valve_failure",
    "fire",
    "thermal_expansion",
    "choke_collapse",
    "tube_rupture",
    "vacuum",
    "other",
)


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def tr3001_compliance(
    scenarios: list[dict[str, object]],
    capacity_adequate: bool,
    required_area_in2: float | None = None,
    acceptance_accepted: bool | None = None,
    accumulation_fraction: float | None = None,
) -> Tr3001ComplianceResult:
    """The six checks over a study's scenario rows and sizing verdict.

    Raises:
        InvalidInputError: for a negative or non-finite rate or area, an accumulation
            fraction outside ``[0, 1]``, a cause the class does not name, or an
            acceptance group that is half stated.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []
    apply_checks(
        checks.on_input,
        {
            "required_area_in2": required_area_in2,
            "accumulation_fraction": accumulation_fraction,
        }.get,
        warnings,
    )
    if (acceptance_accepted is None) != (accumulation_fraction is None):
        raise InvalidInputError(
            "acceptance_accepted",
            "the acceptance verdict and the accumulation fraction are stated together",
        )
    if required_area_in2 is not None and (
        not math.isfinite(required_area_in2) or required_area_in2 < 0.0
    ):
        raise InvalidInputError(
            "required_area_in2",
            f"a finite non-negative area is required, and {required_area_in2} is not",
        )

    rows = input_records_to_si(spec, "scenarios", scenarios)
    parsed: list[tuple[str, float, bool, bool]] = []
    for index, row in enumerate(rows):
        rate = float(row["relief_rate"])
        if not math.isfinite(rate) or rate < 0.0:
            raise InvalidInputError(
                f"scenarios[{index}].relief_rate",
                f"a finite non-negative rate is required, and {rate} is not",
            )
        cause = str(row["cause"])
        if cause not in _CAUSES:
            raise InvalidInputError(
                "cause", f"{cause!r} is not one of the class's overpressure causes"
            )
        parsed.append((cause, rate, bool(row["dynamically_determined"]), bool(row["credible"])))

    credible = [row for row in parsed if row[3]]
    governing = None
    for row in credible:
        if governing is None or row[1] > governing[1]:
            governing = row

    findings: list[Tr3001Finding] = [
        Tr3001Finding(
            check="credible_scenarios",
            status="pass" if credible else "fail",
        ),
        Tr3001Finding(
            check="governing_case",
            status="pass" if governing is not None else "fail",
        ),
    ]
    if governing is not None:
        if required_area_in2 is None:
            status = "needs_review"
        elif capacity_adequate:
            status = "pass"
        else:
            status = "fail"
        findings.append(Tr3001Finding(check="capacity", status=status))
    if acceptance_accepted is not None:
        findings.append(
            Tr3001Finding(
                check="acceptance",
                status="pass" if acceptance_accepted else "fail",
            )
        )
    if governing is not None:
        if governing[0] == "fire":
            status = (
                "pass"
                if accumulation_fraction is not None and accumulation_fraction >= 0.20
                else "needs_review"
            )
            findings.append(Tr3001Finding(check="fire_basis", status=status))
        if governing[0] in ("fire", "blocked_outlet"):
            findings.append(
                Tr3001Finding(
                    check="dynamic_determination",
                    status="pass" if governing[2] else "needs_review",
                )
            )

    pass_count = sum(1 for row in findings if row.status == "pass")
    review_count = sum(1 for row in findings if row.status == "needs_review")
    fail_count = sum(1 for row in findings if row.status == "fail")
    return Tr3001ComplianceResult(
        findings=tuple(findings),
        pass_count=float(pass_count),
        review_count=float(review_count),
        fail_count=float(fail_count),
        compliant=fail_count == 0,
        warnings=tuple(warnings),
    )
