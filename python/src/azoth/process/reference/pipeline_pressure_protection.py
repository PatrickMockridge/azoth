"""``process.pipeline_pressure_protection`` - the two-barrier pipeline screen.

As NeqSim's ``PipelinePressureProtectionCalculator`` computes it. Spec:
``specs/models/process/pipeline_pressure_protection.toml``

The Python twin of ``crates/azoth-process/src/models/pipeline_pressure_protection.rs``:
the incidental pressure, the design margin and the three verdicts, in bar.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import PipelinePressureProtectionResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.pipeline_pressure_protection"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def pipeline_pressure_protection(
    max_source_pressure: Q,
    design_pressure: Q,
    incidental_factor: float,
    barrier1_setpoint: Q,
    barrier2_setpoint: Q,
) -> PipelinePressureProtectionResult:
    """The incidental pressure, the design margin and the protection verdict.

    Raises:
        InvalidInputError: for a non-positive or non-finite pressure, or an incidental
            factor below one.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []
    source_si = input_to_si(spec, "max_source_pressure", max_source_pressure)
    design_si = input_to_si(spec, "design_pressure", design_pressure)
    barrier1_si = input_to_si(spec, "barrier1_setpoint", barrier1_setpoint)
    barrier2_si = input_to_si(spec, "barrier2_setpoint", barrier2_setpoint)
    apply_checks(
        checks.on_input,
        {
            "max_source_pressure": source_si,
            "design_pressure": design_si,
            "incidental_factor": incidental_factor,
            "barrier1_setpoint": barrier1_si,
            "barrier2_setpoint": barrier2_si,
        }.get,
        warnings,
    )
    for name, value in (
        ("max_source_pressure", source_si),
        ("design_pressure", design_si),
        ("barrier1_setpoint", barrier1_si),
        ("barrier2_setpoint", barrier2_si),
    ):
        if not math.isfinite(value) or value <= 0.0:
            raise InvalidInputError(
                name, f"a finite positive pressure is required, and {value} is not"
            )
    if not math.isfinite(incidental_factor) or incidental_factor < 1.0:
        raise InvalidInputError(
            "incidental_factor",
            f"a factor of at least one is required, and {incidental_factor} is not",
        )

    source_bar = source_si / 1.0e5
    design_bar = design_si / 1.0e5
    barrier1_bar = barrier1_si / 1.0e5
    barrier2_bar = barrier2_si / 1.0e5
    max_incidental = incidental_factor * design_bar
    margin = design_bar - source_bar
    fully_rated = source_bar <= design_bar
    barrier1_adequate = barrier1_bar <= design_bar
    barrier2_adequate = barrier2_bar <= max_incidental
    adequate = fully_rated or (barrier1_adequate and barrier2_adequate)
    if fully_rated:
        verdict = "FULLY_RATED"
    elif adequate:
        verdict = "TWO_BARRIERS_REQUIRED"
    else:
        verdict = "INSUFFICIENT"
    return PipelinePressureProtectionResult(
        max_incidental_pressure=from_si(max_incidental * 1.0e5, "bar"),
        design_margin=from_si(margin * 1.0e5, "bar"),
        fully_rated=fully_rated,
        barrier1_adequate=barrier1_adequate,
        barrier2_adequate=barrier2_adequate,
        protection_adequate=adequate,
        verdict=verdict,
        warnings=tuple(warnings),
    )
