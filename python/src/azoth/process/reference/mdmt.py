"""``process.mdmt`` - the UCS-66 minimum design metal temperature.

As NeqSim's ``MDMTCalculator`` computes it. Spec: ``specs/models/process/mdmt.toml``

The Python twin of ``crates/azoth-process/src/models/mdmt.rs``: the four analytic curves,
the stress-ratio reduction and the cold-service verdict.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import MdmtResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.mdmt"

#: The class's lower thickness clamp, mm.
MINIMUM_THICKNESS_MM = 6.0
#: The class's upper thickness clamp, mm.
MAXIMUM_THICKNESS_MM = 100.0
#: The class's full stress-ratio reduction, K.
FULL_REDUCTION_K = 55.0
#: Kelvin offset.
CELSIUS_OFFSET = 273.15


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    mdmt_k: float
    acceptable: bool


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
    material_curve: str,
    governing_thickness: Q,
    stress_ratio: float | None = None,
    min_operating_temperature: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []

    thickness = input_to_si(spec, "governing_thickness", governing_thickness)
    ratio_input = None if stress_ratio is None else float(stress_ratio)
    minimum = (
        None
        if min_operating_temperature is None
        else input_to_si(spec, "min_operating_temperature", min_operating_temperature)
    )
    apply_checks(
        checks_for(spec).on_input,
        {
            "governing_thickness": thickness,
            "stress_ratio": ratio_input,
        }.get,
        warnings,
    )
    _require_finite("governing_thickness", thickness)
    if ratio_input is not None:
        _require_finite("stress_ratio", ratio_input)
    if minimum is None:
        raise InvalidInputError(
            "min_operating_temperature", "the cold-service temperature is required"
        )
    _require_finite("min_operating_temperature", minimum)

    thickness_mm = min(max(thickness * 1000.0, MINIMUM_THICKNESS_MM), MAXIMUM_THICKNESS_MM)
    logarithm = math.log(thickness_mm / MINIMUM_THICKNESS_MM)
    if material_curve == "a":
        base = -29.0 + 18.0 * logarithm
    elif material_curve == "b":
        base = -48.0 + 22.0 * logarithm
    elif material_curve == "c":
        base = -55.0 + 20.0 * logarithm
    elif material_curve == "d":
        base = -65.0 + 18.0 * logarithm
    else:
        raise InvalidInputError(
            "material_curve", "the curve must be one of a, b, c or d"
        )
    ratio = 1.0 if ratio_input is None else ratio_input
    if ratio <= 0.35:
        reduction = FULL_REDUCTION_K
    elif ratio >= 1.0:
        reduction = 0.0
    else:
        reduction = FULL_REDUCTION_K * (1.0 - ratio) / 0.65
    mdmt_k = base - reduction + CELSIUS_OFFSET

    return _Solution(mdmt_k=mdmt_k, acceptable=minimum >= mdmt_k), warnings


def mdmt(
    material_curve: str,
    governing_thickness: Q,
    min_operating_temperature: Q,
    stress_ratio: float | None = None,
) -> MdmtResult:
    """The UCS-66 curve's MDMT and the cold-service verdict.

    The thickness is clamped to 6-100 mm before the curve is read. The four curves are the
    class's analytic approximations of ASME Fig. UCS-66, **not the figure's table**. The
    stress-ratio reduction is 55 K at or below 0.35, zero at or above 1.0, and linear
    between. The verdict is ``min_operating_temperature >= MDMT``.

    Args:
        material_curve: the UCS-66 curve, one of ``a``, ``b``, ``c`` or ``d``.
        governing_thickness: the governing thickness; clamped to 6-100 mm.
        stress_ratio: the applied-to-allowable ratio; absent is 1.0.
        min_operating_temperature: the coldest metal temperature.

    Returns:
        The MDMT in kelvin and whether the cold service is acceptable.

    Raises:
        InvalidInputError: for an unknown curve, a non-positive thickness, a stress ratio
            outside ``(0, 1]``, or a non-finite input.
    """
    solution, warnings = _evaluate(
        material_curve,
        governing_thickness,
        stress_ratio=stress_ratio,
        min_operating_temperature=min_operating_temperature,
    )
    return MdmtResult(
        mdmt=from_si(solution.mdmt_k, "K"),
        acceptable=solution.acceptable,
        warnings=tuple(warnings),
    )
