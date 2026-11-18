"""``process.alarp_audit`` - the ALARP measure audit.

As NeqSim's ``ALARPAuditReport.evaluate`` computes it. Spec:
``specs/models/process/alarp_audit.toml``

The Python twin of ``crates/azoth-process/src/models/alarp_audit.rs``: each measure's
implied cost of averting a fatality against ``VSL x disproportion_factor``.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import AlarpAuditResult, AlarpEvaluationResult, AlarpVerdict
from azoth.core.units import input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.alarp_audit"

#: The class's default value of a statistical life, NOK.
DEFAULT_VALUE_OF_STATISTICAL_LIFE = 30.0e6
#: The class's default disproportion factor.
DEFAULT_DISPROPORTION_FACTOR = 3.0


class _Evaluation(NamedTuple):
    """One measure's answer, before the output row is built."""

    description: str
    icaf: float | None
    verdict: AlarpVerdict


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
    measures: Sequence[Mapping[str, object]],
    value_of_statistical_life: float | None = None,
    disproportion_factor: float | None = None,
) -> tuple[list[_Evaluation], float, list[Warning]]:
    """The class's arithmetic; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    vsl = (
        DEFAULT_VALUE_OF_STATISTICAL_LIFE
        if value_of_statistical_life is None
        else float(value_of_statistical_life)
    )
    factor = (
        DEFAULT_DISPROPORTION_FACTOR
        if disproportion_factor is None
        else float(disproportion_factor)
    )
    _require_finite("value_of_statistical_life", vsl)
    _require_finite("disproportion_factor", factor)
    threshold = vsl * factor
    if not math.isfinite(threshold):
        raise InvalidInputError(
            "threshold",
            "the value of a statistical life times the disproportion factor is not finite",
        )

    rows: list[dict[str, Any]] = input_records_to_si(spec, "measures", measures)
    evaluations: list[_Evaluation] = []
    for row in rows:
        risk = float(row["risk_reduction_per_year"])
        cost = float(row["annualised_cost"])
        if risk < 0.0 or cost < 0.0:
            raise InvalidInputError(
                "risk_reduction_per_year",
                "the risk reduction and the annualised cost must be non-negative",
            )
        _require_finite("risk_reduction_per_year", risk)
        _require_finite("annualised_cost", cost)
        icaf = math.inf if risk == 0.0 else cost / risk
        evaluations.append(
            _Evaluation(
                description=str(row["description"]),
                icaf=icaf if math.isfinite(icaf) else None,
                verdict=(AlarpVerdict.IMPLEMENT if icaf <= threshold else AlarpVerdict.REJECT),
            )
        )

    return evaluations, threshold, warnings


def alarp_audit(
    measures: Sequence[Mapping[str, object]],
    value_of_statistical_life: float | None = None,
    disproportion_factor: float | None = None,
) -> AlarpAuditResult:
    """Each measure's ICAF and its ALARP verdict.

    ``ICAF = annualised_cost / risk_reduction``, or infinity at exactly zero risk; a measure
    is implemented at or below ``VSL x disproportion_factor``. **An infinite ICAF is
    published as an absent one**, while the verdict still compares it. The amounts are in
    NOK and declared dimensionless; the class validates neither the VSL nor the factor.

    Args:
        measures: the candidate risk-reduction measures, in caller order.
        value_of_statistical_life: the value of a statistical life, NOK; absent is 30
            million.
        disproportion_factor: the gross-disproportion multiplier; absent is 3.

    Returns:
        One evaluation per measure, the threshold and the measure count.

    Raises:
        InvalidInputError: for a negative risk reduction or cost, a non-finite VSL, factor,
            measure or threshold.
    """
    evaluations, threshold, warnings = _evaluate(
        measures,
        value_of_statistical_life=value_of_statistical_life,
        disproportion_factor=disproportion_factor,
    )
    return AlarpAuditResult(
        evaluations=tuple(
            AlarpEvaluationResult(
                description=row.description,
                icaf=row.icaf,
                verdict=row.verdict,
            )
            for row in evaluations
        ),
        threshold=threshold,
        measure_count=float(len(evaluations)),
        warnings=tuple(warnings),
    )
