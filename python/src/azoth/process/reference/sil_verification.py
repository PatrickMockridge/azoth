"""``process.sil_verification`` - the SIL verification arithmetic.

As NeqSim's ``SILVerificationResult`` computes it. Spec:
``specs/models/process/sil_verification.toml``

The Python twin of ``crates/azoth-process/src/models/sil_verification.rs``.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import SilIssue, SilVerificationResult
from azoth.core.warnings import Warning

MODEL_ID = "process.sil_verification"

_HARDWARE_FAULT_TOLERANCE = {
    "1oo1": 0.0,
    "1oo2": 1.0,
    "2oo2": 0.0,
    "2oo3": 1.0,
    "1oo3": 2.0,
}


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _required_sil(target_pfd: float) -> int:
    if target_pfd >= 0.01:
        return 1
    if target_pfd >= 0.001:
        return 2
    if target_pfd >= 0.0001:
        return 3
    return 4


def _max_pfd_for_sil(sil: int) -> float:
    return {1: 0.1, 2: 0.01, 3: 0.001, 4: 0.0001}.get(sil, 1.0)


def sil_verification(
    pfd_average: float,
    claimed_sil: float,
    architecture: str,
    test_interval_hours: float,
) -> SilVerificationResult:
    """The achieved SIL, the upper PFD, the architecture table and the issue set.

    Raises:
        InvalidInputError: for a negative or non-finite PFD, a claimed SIL outside 1..4
            or not whole, or a negative or non-finite proof-test interval.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    pfd = float(pfd_average)
    sil_value = float(claimed_sil)
    interval = float(test_interval_hours)
    apply_checks(
        checks.on_input,
        {
            "pfd_average": pfd,
            "claimed_sil": sil_value,
            "test_interval_hours": interval,
        }.get,
        warnings,
    )
    for name, value in (
        ("pfd_average", pfd),
        ("claimed_sil", sil_value),
        ("test_interval_hours", interval),
    ):
        if not math.isfinite(value):
            raise InvalidInputError(
                name, "a non-finite value is not a number this calculation publishes"
            )
    if sil_value != math.floor(sil_value) or not 1.0 <= sil_value <= 4.0:
        raise InvalidInputError(
            "claimed_sil",
            f"a claimed SIL is a whole number in [1, 4], and {sil_value} is not",
        )
    claimed = int(sil_value)

    achieved = _required_sil(pfd)
    fault_tolerance = _HARDWARE_FAULT_TOLERANCE.get(architecture, 0.0)
    coverage = 0.6 if ("oo2" in architecture or "oo3" in architecture) else 0.5
    proof_years = interval / 8760.0
    issues: list[SilIssue] = []

    if pfd > _max_pfd_for_sil(claimed):
        issues.append(SilIssue(severity="error", category="pfd_exceeded"))
    if claimed == 4 and fault_tolerance < 2.0:
        issues.append(SilIssue(severity="error", category="architecture"))
    elif claimed == 3 and fault_tolerance < 1.0:
        issues.append(SilIssue(severity="warning", category="architecture"))
    if proof_years > 5.0:
        issues.append(SilIssue(severity="warning", category="proof_test_interval"))
    if claimed >= 3 and coverage < 0.9:
        issues.append(SilIssue(severity="warning", category="diagnostic_coverage"))

    error_count = sum(1 for issue in issues if issue.severity == "error")
    warning_count = sum(1 for issue in issues if issue.severity == "warning")

    return SilVerificationResult(
        claimed_sil=sil_value,
        achieved_sil=float(achieved),
        sil_achieved=achieved >= claimed,
        pfd_average=pfd,
        pfd_upper=pfd * 1.5,
        hardware_fault_tolerance=fault_tolerance,
        systematic_capability=sil_value,
        diagnostic_coverage=coverage,
        issue_count=float(len(issues)),
        error_count=float(error_count),
        warning_count=float(warning_count),
        verification_passed=error_count == 0,
        issues=tuple(issues),
        warnings=tuple(warnings),
    )
