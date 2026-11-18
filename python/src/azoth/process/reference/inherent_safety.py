"""``process.inherent_safety`` - the Kletz four-pillar inherent-safety index.

As NeqSim's ``InherentSafetyEvaluator.overallIndex`` computes it. Spec:
``specs/models/process/inherent_safety.toml``

The Python twin of ``crates/azoth-process/src/models/inherent_safety.rs``: four pillar
scores and their simple mean.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import InherentSafetyResult
from azoth.core.warnings import Warning

MODEL_ID = "process.inherent_safety"


class _Solution(NamedTuple):
    """Every number the calculation produces, before the result is built."""

    overall_index: float


def _require_finite(name: str, value: float) -> None:
    """Refuse a value the arithmetic cannot carry to a published number."""
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    substitute_score: float | None = None,
    minimize_score: float | None = None,
    moderate_score: float | None = None,
    simplify_score: float | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []
    values = {
        "substitute_score": None if substitute_score is None else float(substitute_score),
        "minimize_score": None if minimize_score is None else float(minimize_score),
        "moderate_score": None if moderate_score is None else float(moderate_score),
        "simplify_score": None if simplify_score is None else float(simplify_score),
    }
    apply_checks(checks_for(spec).on_input, values.get, warnings)
    for name, value in values.items():
        if value is not None:
            _require_finite(name, value)

    return (
        _Solution(overall_index=sum(value or 0.0 for value in values.values()) / 4.0),
        warnings,
    )


def inherent_safety(
    substitute_score: float | None = None,
    minimize_score: float | None = None,
    moderate_score: float | None = None,
    simplify_score: float | None = None,
) -> InherentSafetyResult:
    """The mean of the four Kletz inherent-safety pillars.

    Each score is in ``[0, 10]`` and 10 is the best; **an absent pillar is zero**, the
    class's zero-initialised array. Notes and the report are presentation.

    Args:
        substitute_score: the substitute pillar's score.
        minimize_score: the minimize pillar's score.
        moderate_score: the moderate pillar's score.
        simplify_score: the simplify pillar's score.

    Returns:
        The simple mean of the four scores.

    Raises:
        InvalidInputError: for a score outside ``[0, 10]`` or a non-finite input.
    """
    solution, warnings = _evaluate(
        substitute_score,
        minimize_score,
        moderate_score,
        simplify_score,
    )
    return InherentSafetyResult(
        overall_index=solution.overall_index,
        warnings=tuple(warnings),
    )
