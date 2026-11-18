"""``process.escalation_graph`` - the domino-effect propagation.

As NeqSim's ``EscalationGraphAnalyzer.propagate`` computes it. Spec:
``specs/models/process/escalation_graph.toml``

The Python twin of ``crates/azoth-process/src/models/escalation_graph.rs``: cumulative loads
over every failed source, a threshold at or above which a target fails, and the worst case.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import EscalationGraphResult, EscalationItemResult
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.escalation_graph"


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


def _propagate(
    thresholds: dict[str, float],
    outgoing: dict[str, list[tuple[str, float]]],
    initiator: str,
) -> set[str]:
    """The failed set from one initiator, at the class's fixed point."""
    failed = {initiator}
    while True:
        cumulative: dict[str, float] = {}
        for source in failed:
            for target, load in outgoing[source]:
                cumulative[target] = cumulative.get(target, 0.0) + load
        newly_failed = {
            target
            for target, load in cumulative.items()
            if target not in failed and load >= thresholds[target]
        }
        if not newly_failed:
            return failed
        failed |= newly_failed


def _rows(order: list[str], failed: set[str]) -> tuple[EscalationItemResult, ...]:
    """The failed members in the items order."""
    return tuple(EscalationItemResult(item_id=item_id) for item_id in order if item_id in failed)


def escalation_graph(
    items: Sequence[Mapping[str, object]],
    exposures: Sequence[Mapping[str, object]],
    initiator: str,
) -> EscalationGraphResult:
    """The items that fail from an initiator, and the worst case over every item.

    A target fails when the cumulative load from every currently failed source is at or above
    its threshold. The sets are published in the items order; a duplicate item row keeps its
    first position and the later threshold. The loads and thresholds are dimensionless in the
    caller's own consistent unit.

    Args:
        items: the process items and their failure thresholds.
        exposures: the directed hazard exposures.
        initiator: the item that fails first.

    Returns:
        The failed set, its count, whether anything escalated, and the worst case.

    Raises:
        InvalidInputError: for an initiator or exposure endpoint that names no item, or a
            non-finite threshold or load.
    """
    spec = _spec()
    warnings: list[Warning] = []
    item_rows: list[dict[str, Any]] = input_records_to_si(spec, "items", items)
    exposure_rows: list[dict[str, Any]] = input_records_to_si(spec, "exposures", exposures)

    order: list[str] = []
    thresholds: dict[str, float] = {}
    for row in item_rows:
        item_id = str(row["item_id"])
        threshold = float(row["failure_threshold"])
        _require_finite("failure_threshold", threshold)
        if item_id not in thresholds:
            order.append(item_id)
        thresholds[item_id] = threshold
    outgoing: dict[str, list[tuple[str, float]]] = {item_id: [] for item_id in order}
    for row in exposure_rows:
        source = str(row["source"])
        target = str(row["target"])
        load = float(row["load"])
        _require_finite("load", load)
        if source not in thresholds or target not in thresholds:
            raise InvalidInputError(
                "item_id", "both ends of an exposure must name a registered item"
            )
        outgoing[source].append((target, load))
    if initiator not in thresholds:
        raise InvalidInputError("initiator", f"`{initiator}` names no registered item")

    failed = _propagate(thresholds, outgoing, initiator)
    failure_count = float(len(failed))

    worst: set[str] = set()
    worst_count = 0
    for item_id in order:
        candidate = _propagate(thresholds, outgoing, item_id)
        if len(candidate) > worst_count:
            worst = candidate
            worst_count = len(candidate)

    return EscalationGraphResult(
        failed_items=_rows(order, failed),
        failure_count=failure_count,
        any_escalation=failure_count > 1.0,
        worst_case_items=_rows(order, worst),
        worst_case_count=float(worst_count),
        warnings=tuple(warnings),
    )
