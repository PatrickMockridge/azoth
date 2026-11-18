"""``process.piping_fiv_liquid`` - the AVIFF liquid-line FIV screen.

As NeqSim's ``PipingFivScreening.screenLiquid`` computes it. Spec:
``specs/models/process/piping_fiv_liquid.toml``

The Python twin of ``crates/azoth-process/src/models/piping_fiv_liquid.rs``: the velocity
energy term, the D/t, branch and support factors, and the likelihood band.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import FivLikelihood, PipingFivLiquidResult
from azoth.core.units import Q, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.piping_fiv_liquid"

#: The class's reference liquid velocity, m/s.
REFERENCE_LIQUID_VELOCITY = 6.0
#: The D/t ratio above which the factor rises.
D_OVER_T_FLOOR = 40.0


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    lof: float
    likelihood: FivLikelihood
    d_overt: float
    d_overt_factor: float
    branch_factor: float
    support: float
    energy_term: float


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


def _band_for(lof: float) -> FivLikelihood:
    """The band a likelihood score falls in, with the class's exclusive upper bounds."""
    if lof < 0.3:
        return FivLikelihood.LOW
    if lof < 0.5:
        return FivLikelihood.MEDIUM
    if lof < 1.0:
        return FivLikelihood.HIGH
    return FivLikelihood.VERY_HIGH


def _evaluate(
    velocity: Q,
    outside_diameter: Q,
    wall_thickness: Q,
    branch_count: float,
    support_quality_factor: float,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []

    velocity_si = input_to_si(spec, "velocity", velocity)
    diameter_si = input_to_si(spec, "outside_diameter", outside_diameter)
    wall_si = input_to_si(spec, "wall_thickness", wall_thickness)
    apply_checks(
        checks_for(spec).on_input,
        {
            "velocity": velocity_si,
            "outside_diameter": diameter_si,
            "wall_thickness": wall_si,
            "branch_count": float(branch_count),
            "support_quality_factor": float(support_quality_factor),
        }.get,
        warnings,
    )
    for name, value in (
        ("velocity", velocity_si),
        ("outside_diameter", diameter_si),
        ("wall_thickness", wall_si),
        ("branch_count", float(branch_count)),
        ("support_quality_factor", float(support_quality_factor)),
    ):
        _require_finite(name, value)
    if not float(branch_count).is_integer():
        raise InvalidInputError("branch_count", "the small-bore connection count is a whole number")

    d_overt = diameter_si / wall_si
    d_overt_factor = max(d_overt / D_OVER_T_FLOOR, 1.0)
    branch_factor = 1.0 + 0.10 * float(branch_count)
    support = min(max(float(support_quality_factor), 1.0), 3.0)
    energy_term = velocity_si / REFERENCE_LIQUID_VELOCITY
    lof = energy_term * d_overt_factor * branch_factor * support

    return (
        _Solution(
            lof=lof,
            likelihood=_band_for(lof),
            d_overt=d_overt,
            d_overt_factor=d_overt_factor,
            branch_factor=branch_factor,
            support=support,
            energy_term=energy_term,
        ),
        warnings,
    )


def piping_fiv_liquid(
    velocity: Q,
    outside_diameter: Q,
    wall_thickness: Q,
    branch_count: float,
    support_quality_factor: float,
) -> PipingFivLiquidResult:
    """The liquid-line FIV likelihood of failure and its factors.

    The support-quality factor is clamped to ``[1, 3]``; the bands are exclusive upward, so
    0.3 is medium, 0.5 high and 1.0 very high. The class's recommendation text is
    presentation.

    Args:
        velocity: the liquid velocity at line conditions.
        outside_diameter: the pipe outside diameter.
        wall_thickness: the pipe wall thickness.
        branch_count: the number of small-bore connections, a whole number.
        support_quality_factor: 1 for good supports to 3 for poor ones.

    Returns:
        The likelihood, its band and every contributing factor.

    Raises:
        InvalidInputError: for a negative velocity or branch count, a non-positive diameter
            or wall, a fractional branch count, or a non-finite input.
    """
    solution, warnings = _evaluate(
        velocity,
        outside_diameter,
        wall_thickness,
        branch_count,
        support_quality_factor,
    )
    return PipingFivLiquidResult(
        lof=solution.lof,
        likelihood=solution.likelihood,
        d_overt=solution.d_overt,
        d_overt_factor=solution.d_overt_factor,
        branch_factor=solution.branch_factor,
        support_quality_factor_applied=solution.support,
        energy_term=solution.energy_term,
        warnings=tuple(warnings),
    )
