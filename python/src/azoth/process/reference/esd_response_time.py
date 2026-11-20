"""``process.esd_response_time`` - the ESD response-time budget.

The Python twin of ``crates/azoth-process/src/models/esd_response_time.rs``. The port
source is NeqSim's ``process/safety/esd/EsdResponseTimeSimulator.evaluate``: the three
stage sums, their total, the inclusive budget test and the echoed contributions.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import EsdContribution, EsdResponseTimeResult
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.esd_response_time"

_STAGES = ("detection", "logic", "final_element")


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def esd_response_time(
    sif_tag: str | None = None,
    contributions: list[dict[str, object]] | None = None,
    allowable_response_time: Q | None = None,
) -> EsdResponseTimeResult:
    """The ESD response-time budget.

    Args:
        sif_tag: the SIF identifier the result echoes; absent is ``ESD-SIF``.
        contributions: the timed contribution rows, each ``{name, stage, time}``; the
            list must not be empty.
        allowable_response_time: the end-to-end budget; absent is no budget, which is
            within budget and has no margin.

    Returns:
        The three stage totals, their total, the budget test and the echoed rows.

    Raises:
        InvalidInputError: for an empty list, a negative time or an unknown stage.
    """
    spec = _spec()
    warnings: list[Warning] = []
    allowable_value = (
        None
        if allowable_response_time is None
        else input_to_si(spec, "allowable_response_time", allowable_response_time)
    )
    apply_checks(
        checks_for(spec).on_input,
        {"allowable_response_time": allowable_value}.get,
        warnings,
    )

    rows = input_records_to_si(spec, "contributions", contributions or [])
    if not rows:
        raise InvalidInputError("contributions", "the class refuses an empty contribution list")

    detection = 0.0
    logic = 0.0
    final_element = 0.0
    echoed: list[EsdContribution] = []
    for index, row in enumerate(rows):
        stage = str(row["stage"])
        if stage not in _STAGES:
            raise InvalidInputError(
                f"contributions[{index}].stage",
                f"`{stage}` is not `detection`, `logic` or `final_element`",
            )
        time = float(row["time"])
        if not math.isfinite(time) or time < 0.0:
            raise InvalidInputError(
                f"contributions[{index}].time",
                "a contribution's time must be finite and at least zero",
            )
        match stage:
            case "detection":
                detection += time
            case "logic":
                logic += time
            case _:
                final_element += time
        echoed.append(EsdContribution(name=str(row["name"]), stage=stage, time=from_si(time, "s")))

    total = detection + logic + final_element
    if allowable_value is None:
        allowable = None
        margin = None
        within_budget = True
    else:
        allowable = allowable_value
        margin = allowable_value - total
        within_budget = total <= allowable_value

    return EsdResponseTimeResult(
        sif_tag="ESD-SIF" if sif_tag is None else sif_tag,
        detection_time=from_si(detection, "s"),
        logic_time=from_si(logic, "s"),
        final_element_time=from_si(final_element, "s"),
        total_response_time=from_si(total, "s"),
        allowable_response_time=None if allowable is None else from_si(allowable, "s"),
        margin=None if margin is None else from_si(margin, "s"),
        within_budget=within_budget,
        contributions=tuple(echoed),
        warnings=tuple(warnings),
    )
