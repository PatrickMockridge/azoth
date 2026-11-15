"""``process.risk_matrix`` - the risk matrix's static arithmetic.

As NeqSim's ``RiskMatrix`` computes it. Spec: ``specs/models/process/risk_matrix.toml``

The Python twin of ``crates/azoth-process/src/models/risk_matrix.rs``: the bands, the
score and level, the MTBF sentinel and the two cost terms.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import RiskAssessmentRow, RiskMatrixResult
from azoth.core.units import input_records_to_si, quantity
from azoth.core.warnings import Warning

MODEL_ID = "process.risk_matrix"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _probability_category(failures_per_year: float) -> tuple[str, float]:
    if failures_per_year < 0.1:
        return "very_low", 1.0
    if failures_per_year < 0.5:
        return "low", 2.0
    if failures_per_year < 1.0:
        return "medium", 3.0
    if failures_per_year < 2.0:
        return "high", 4.0
    return "very_high", 5.0


def _consequence_category(loss_percent: float) -> tuple[str, float]:
    if loss_percent < 5.0:
        return "negligible", 1.0
    if loss_percent < 20.0:
        return "minor", 2.0
    if loss_percent < 50.0:
        return "moderate", 3.0
    if loss_percent < 80.0:
        return "major", 4.0
    return "catastrophic", 5.0


def _risk_level(score: float) -> str:
    if score <= 4.0:
        return "low"
    if score <= 9.0:
        return "medium"
    if score <= 15.0:
        return "high"
    return "critical"


def risk_matrix(
    equipment: list[dict[str, object]],
    product_price_per_kg: float | None = None,
    downtime_cost_per_hour: float | None = None,
) -> RiskMatrixResult:
    """The bands, the score and level, the MTBF, the downtime and the costs.

    Raises:
        InvalidInputError: for a negative or non-finite frequency, repair time or mass
            loss, a loss percentage outside ``[0, 100]``, or a negative price or
            downtime cost.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    price = 0.5 if product_price_per_kg is None else float(product_price_per_kg)
    downtime_cost = 5000.0 if downtime_cost_per_hour is None else float(downtime_cost_per_hour)
    apply_checks(
        checks.on_input,
        {
            "product_price_per_kg": product_price_per_kg,
            "downtime_cost_per_hour": downtime_cost_per_hour,
        }.get,
        warnings,
    )
    for name, value in (
        ("product_price_per_kg", price),
        ("downtime_cost_per_hour", downtime_cost),
    ):
        if not math.isfinite(value) or value < 0.0:
            raise InvalidInputError(
                name, f"a non-negative finite value is required, and {value} is not"
            )

    rows: list[dict[str, Any]] = input_records_to_si(spec, "equipment", equipment)
    assessments: list[RiskAssessmentRow] = []
    total_annual_risk_cost = 0.0
    for index, row in enumerate(rows):
        failures_per_year = float(row["failures_per_year"])
        mttr_hours = float(row["mttr_hours"])
        loss_percent = float(row["production_loss_percent"])
        loss_kg_s = float(row["production_loss_kg_per_hr"])
        if not math.isfinite(failures_per_year) or failures_per_year < 0.0:
            raise InvalidInputError(
                f"equipment[{index}].failures_per_year",
                f"a failure frequency is finite and at least zero, and {failures_per_year} is not",
            )
        if not math.isfinite(mttr_hours) or mttr_hours < 0.0:
            raise InvalidInputError(
                f"equipment[{index}].mttr_hours",
                f"a repair time is finite and at least zero, and {mttr_hours} is not",
            )
        if not math.isfinite(loss_percent) or not 0.0 <= loss_percent <= 100.0:
            raise InvalidInputError(
                f"equipment[{index}].production_loss_percent",
                f"a production loss is in [0, 100] percent, and {loss_percent} is not",
            )
        if not math.isfinite(loss_kg_s) or loss_kg_s < 0.0:
            raise InvalidInputError(
                f"equipment[{index}].production_loss_kg_per_hr",
                "a mass loss is finite and at least zero",
            )

        probability_category, probability_level = _probability_category(failures_per_year)
        consequence_category, consequence_level = _consequence_category(loss_percent)
        risk_score = probability_level * consequence_level
        mtbf_hours = 8760.0 / failures_per_year if failures_per_year > 0.0 else 1.0e6
        expected_downtime = failures_per_year * mttr_hours
        loss_kg_hr = loss_kg_s * 3600.0
        cost_per_failure = loss_kg_hr * mttr_hours * price + mttr_hours * downtime_cost
        annual_risk_cost = cost_per_failure * failures_per_year
        total_annual_risk_cost += annual_risk_cost

        assessments.append(
            RiskAssessmentRow(
                name=str(row["name"]),
                failures_per_year=failures_per_year,
                mtbf_hours=mtbf_hours,
                mttr_hours=mttr_hours,
                expected_downtime_hours_per_year=expected_downtime,
                probability_category=probability_category,
                probability_level=probability_level,
                production_loss_percent=loss_percent,
                production_loss_kg_hr=quantity(loss_kg_s, "kg/s"),
                consequence_category=consequence_category,
                consequence_level=consequence_level,
                risk_score=risk_score,
                risk_level=_risk_level(risk_score),
                cost_per_failure=cost_per_failure,
                annual_risk_cost=annual_risk_cost,
            )
        )

    return RiskMatrixResult(
        assessments=tuple(assessments),
        total_annual_risk_cost=total_annual_risk_cost,
        warnings=tuple(warnings),
    )
