"""``process.mah_bow_tie_skeleton`` - the ISO 17776 MAH catalogue skeleton.

As NeqSim's ``MahBowTieBuilder.build`` and ``MahCatalogue`` produce it. Spec:
``specs/models/process/mah_bow_tie_skeleton.toml``

The Python twin of ``crates/azoth-process/src/models/mah_bow_tie_skeleton.rs``: the
default threats, consequences and barrier classes at the class's placeholder constants.
"""

from __future__ import annotations

from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    MahBarrier,
    MahBowTieSkeletonResult,
    MahConsequence,
    MahThreat,
)
from azoth.core.warnings import Warning
from azoth.process.reference._hazid import MAH_CATALOGUE

MODEL_ID = "process.mah_bow_tie_skeleton"

THREAT_FREQUENCY_PER_YEAR = 0.001
CONSEQUENCE_SEVERITY = 4.0
BARRIER_PFD = 0.01


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def mah_bow_tie_skeleton(mah_type: str) -> MahBowTieSkeletonResult:
    """The catalogue skeleton for one major accident hazard.

    Args:
        mah_type: one of the eleven ISO 17776 categories.

    Returns:
        The default threats, consequences and barrier classes, their counts, and the
        class's placeholders.

    Raises:
        InvalidInputError: for a type that is not one of the eleven categories.
    """
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    if mah_type not in MAH_CATALOGUE:
        raise InvalidInputError(
            "mah_type",
            f"`{mah_type}` is not one of {', '.join(MAH_CATALOGUE)}",
        )
    _description, threats, consequences, barriers = MAH_CATALOGUE[mah_type]

    threat_rows = [
        MahThreat(
            id=f"T{index + 1}",
            description=threat,
            frequency_per_year=THREAT_FREQUENCY_PER_YEAR,
        )
        for index, threat in enumerate(threats)
    ]
    consequence_rows = [
        MahConsequence(
            id=f"C{index + 1}",
            description=consequence,
            severity=CONSEQUENCE_SEVERITY,
        )
        for index, consequence in enumerate(consequences)
    ]
    barrier_rows = [
        MahBarrier(id=f"B{index + 1}", description=barrier, pfd=BARRIER_PFD)
        for index, barrier in enumerate(barriers)
    ]

    return MahBowTieSkeletonResult(
        threats=tuple(threat_rows),
        consequences=tuple(consequence_rows),
        barriers=tuple(barrier_rows),
        threat_count=float(len(threat_rows)),
        consequence_count=float(len(consequence_rows)),
        barrier_count=float(len(barrier_rows)),
        warnings=tuple(warnings),
    )
