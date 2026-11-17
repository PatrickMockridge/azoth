"""``process.fmea_worksheet`` - the FMEA risk-priority ranking.

As NeqSim's ``FMEAWorksheet`` computes it. Spec: ``specs/models/process/fmea_worksheet.toml``

The Python twin of ``crates/azoth-process/src/models/fmea_worksheet.rs``: ``RPN = S O D``,
the stable descending ranking and the inclusive threshold filter.
"""

from __future__ import annotations

from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import FmeaRankedEntry, FmeaWorksheetResult
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.fmea_worksheet"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _whole(value: float) -> bool:
    return value.is_integer()


def fmea_worksheet(
    entries: list[dict[str, object]],
    rpn_threshold: float,
) -> FmeaWorksheetResult:
    """The RPN ranking and the entries at or above the threshold.

    Args:
        entries: the failure-mode rows, each ``{component, failure_mode, effect, cause,
            severity, occurrence, detection, mitigation}``.
        rpn_threshold: the RPN at which an entry is kept, inclusive.

    Returns:
        The descending ranking, the entries at or above the threshold in caller order, the
        largest RPN and the entry count.

    Raises:
        InvalidInputError: for an S, O or D outside ``[1, 10]`` or not a whole number, or
            for a threshold that is not a whole number.
    """
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    rows = input_records_to_si(spec, "entries", entries)
    threshold = float(rpn_threshold)
    for index, row in enumerate(rows):
        for name in ("severity", "occurrence", "detection"):
            score = float(row[name])
            if not _whole(score) or not 1.0 <= score <= 10.0:
                raise InvalidInputError(
                    f"entries[{index}].{name}",
                    "S, O, D must each be in [1, 10]",
                )
    if not _whole(threshold):
        raise InvalidInputError("rpn_threshold", "the threshold must be a whole number")

    caller_order: list[FmeaRankedEntry] = []
    for row in rows:
        caller_order.append(
            FmeaRankedEntry(
                component=str(row["component"]),
                failure_mode=str(row["failure_mode"]),
                severity=float(row["severity"]),
                occurrence=float(row["occurrence"]),
                detection=float(row["detection"]),
                rpn=float(row["severity"]) * float(row["occurrence"]) * float(row["detection"]),
                mitigation=None if row.get("mitigation") is None else str(row["mitigation"]),
            )
        )
    # Python's sort is stable, and `reverse=True` preserves that, so a tie keeps caller order.
    ranked = sorted(caller_order, key=lambda row: row.rpn, reverse=True)
    above = [row for row in caller_order if row.rpn >= threshold]
    return FmeaWorksheetResult(
        ranked_entries=tuple(ranked),
        entries_above_threshold=tuple(above),
        highest_rpn=ranked[0].rpn if ranked else None,
        entry_count=float(len(ranked)),
        warnings=tuple(warnings),
    )
