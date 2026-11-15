"""``process.bow_tie`` - the bow-tie risk sum.

As NeqSim's ``BowTieModel.calculateRisk`` computes it. Spec:
``specs/models/process/bow_tie.toml``

The Python twin of ``crates/azoth-process/src/models/bow_tie.rs``: the active threat sum,
the functional prevention-barrier product and the maximum severity.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import BowTieResult
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.bow_tie"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def bow_tie(
    threats: list[dict[str, object]],
    consequences: list[dict[str, object]],
    barriers: list[dict[str, object]],
) -> BowTieResult:
    """The active threat sum, the prevention-barrier product and the maximum severity.

    Args:
        threats: the causes, each ``{frequency, active}``; ``active`` absent is true.
        consequences: the outcomes, each ``{severity}``.
        barriers: the barriers, each ``{pfd, barrier_type, functional}``; only
            ``prevention`` and ``both`` multiply, and ``functional`` absent is true.

    Returns:
        The unmitigated and mitigated frequencies, the RRF and the largest severity.

    Raises:
        InvalidInputError: for a negative or non-finite frequency, a negative or
            non-whole severity, a PFD outside ``[0, 1]``, or an unknown barrier type.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []
    apply_checks(checks.on_input, lambda _: None, warnings)

    threat_rows: list[dict[str, Any]] = input_records_to_si(spec, "threats", threats)
    for index, row in enumerate(threat_rows):
        frequency = float(row["frequency"])
        if not math.isfinite(frequency) or frequency < 0.0:
            raise InvalidInputError(
                f"threats[{index}].frequency",
                f"a threat frequency is finite and at least zero, and {frequency} is not",
            )

    consequence_rows: list[dict[str, Any]] = input_records_to_si(spec, "consequences", consequences)
    for index, row in enumerate(consequence_rows):
        severity = float(row["severity"])
        if not math.isfinite(severity) or severity < 0.0 or severity != math.floor(severity):
            raise InvalidInputError(
                f"consequences[{index}].severity",
                f"a severity is a whole number at least zero, and {severity} is not",
            )

    barrier_rows: list[dict[str, Any]] = input_records_to_si(spec, "barriers", barriers)
    for index, row in enumerate(barrier_rows):
        pfd = float(row["pfd"])
        if not math.isfinite(pfd) or not 0.0 <= pfd <= 1.0:
            raise InvalidInputError(
                f"barriers[{index}].pfd", f"a barrier PFD is in [0, 1], and {pfd} is not"
            )
        barrier_type = str(row["barrier_type"])
        if barrier_type not in ("prevention", "mitigation", "both"):
            raise InvalidInputError(
                f"barriers[{index}].barrier_type",
                f"a barrier type is prevention, mitigation or both, and {barrier_type!r} is not",
            )

    unmitigated = 0.0
    for row in threat_rows:
        if row.get("active") is None or bool(row["active"]):
            unmitigated += float(row["frequency"])

    mitigated = unmitigated
    for row in barrier_rows:
        prevention = str(row["barrier_type"]) in ("prevention", "both")
        functional = row.get("functional") is None or bool(row["functional"])
        if prevention and functional:
            mitigated *= float(row["pfd"])

    max_severity = 0.0
    for row in consequence_rows:
        severity = float(row["severity"])
        if severity > max_severity:
            max_severity = severity

    total_rrf = unmitigated / mitigated if mitigated > 0.0 else 0.0

    return BowTieResult(
        unmitigated_frequency_per_year=unmitigated,
        mitigated_frequency_per_year=mitigated,
        total_rrf=total_rrf,
        max_severity=max_severity,
        warnings=tuple(warnings),
    )
