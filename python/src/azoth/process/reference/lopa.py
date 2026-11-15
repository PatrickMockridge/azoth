"""``process.lopa`` - the layer-of-protection analysis result arithmetic.

As NeqSim's ``LOPAResult`` computes it. Spec: ``specs/models/process/lopa.toml``

The Python twin of ``crates/azoth-process/src/models/lopa.rs``: the target status and gap,
the RRFs, the required additional SIL, the layer rows and the STS0131 bands.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    LopaLayerContribution,
    LopaResult,
    Sts0131PressureCategory,
)
from azoth.core.units import Q, input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.lopa"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _required_sil(target_pfd: float) -> int:
    if target_pfd >= 0.01:
        return 1
    if target_pfd >= 0.001:
        return 2
    if target_pfd >= 0.0001:
        return 3
    return 4


def _sts0131_category(
    event: float, design: float, test: float
) -> tuple[Sts0131PressureCategory, float]:
    if event > 2.0 * design:
        return Sts0131PressureCategory.ABOVE_TWO_TIMES_DESIGN_PRESSURE, 1.0e-5
    if event > test:
        return Sts0131PressureCategory.ABOVE_TEST_TO_TWO_TIMES_DESIGN_PRESSURE, 1.0e-4
    if event > design:
        return Sts0131PressureCategory.ABOVE_DESIGN_TO_TEST_PRESSURE, 1.0e-3
    return Sts0131PressureCategory.BELOW_OR_AT_DESIGN_PRESSURE, 1.0e-2


def lopa(
    initiating_event_frequency_per_year: float,
    target_frequency_per_year: float,
    mitigated_frequency_per_year: float,
    layers: list[dict[str, object]],
    event_pressure: Q | None = None,
    design_pressure: Q | None = None,
    test_pressure: Q | None = None,
) -> LopaResult:
    """The target status and gap, the RRFs, the required SIL, the layer rows and STS0131.

    Raises:
        InvalidInputError: for a negative or non-finite frequency, a PFD outside
            ``[0, 1]``, a negative layer frequency, a partial pressure triple, a
            non-positive pressure, or a test pressure below the design pressure.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    initiating = float(initiating_event_frequency_per_year)
    target = float(target_frequency_per_year)
    mitigated = float(mitigated_frequency_per_year)
    apply_checks(
        checks.on_input,
        {
            "initiating_event_frequency_per_year": initiating,
            "target_frequency_per_year": target,
            "mitigated_frequency_per_year": mitigated,
            "event_pressure": None if event_pressure is None else event_pressure.to("Pa").magnitude,
            "design_pressure": (
                None if design_pressure is None else design_pressure.to("Pa").magnitude
            ),
            "test_pressure": (None if test_pressure is None else test_pressure.to("Pa").magnitude),
        }.get,
        warnings,
    )
    for name, value in (
        ("initiating_event_frequency_per_year", initiating),
        ("target_frequency_per_year", target),
        ("mitigated_frequency_per_year", mitigated),
    ):
        if not math.isfinite(value):
            raise InvalidInputError(
                name, "a non-finite value is not a number this calculation publishes"
            )

    rows: list[dict[str, Any]] = input_records_to_si(spec, "layers", layers)
    for index, row in enumerate(rows):
        pfd = float(row["pfd"])
        if not math.isfinite(pfd) or not 0.0 <= pfd <= 1.0:
            raise InvalidInputError(
                f"layers[{index}].pfd", f"a layer PFD is in [0, 1], and {pfd} is not"
            )
        for field in ("frequency_before_per_year", "frequency_after_per_year"):
            value = float(row[field])
            if not math.isfinite(value) or value < 0.0:
                raise InvalidInputError(
                    f"layers[{index}].{field}",
                    f"a layer frequency is finite and at least zero, and {value} is not",
                )

    pressures = (event_pressure, design_pressure, test_pressure)
    if all(pressure is None for pressure in pressures):
        sts0131: tuple[Sts0131PressureCategory, float] | None = None
    elif any(pressure is None for pressure in pressures):
        raise InvalidInputError(
            "event_pressure", "the STS0131 triple is all three pressures or none"
        )
    else:
        event = float(event_pressure.to("Pa").magnitude)  # type: ignore[union-attr]
        design = float(design_pressure.to("Pa").magnitude)  # type: ignore[union-attr]
        test = float(test_pressure.to("Pa").magnitude)  # type: ignore[union-attr]
        for name, value in (
            ("event_pressure", event),
            ("design_pressure", design),
            ("test_pressure", test),
        ):
            if not math.isfinite(value) or value <= 0.0:
                raise InvalidInputError(
                    name, f"an STS0131 pressure is positive, and {value} is not"
                )
        if test < design:
            raise InvalidInputError(
                "test_pressure", "the test pressure is at least the design pressure"
            )
        sts0131 = _sts0131_category(event, design, test)

    target_met = mitigated <= target
    gap_to_target = mitigated - target
    total_rrf = initiating / mitigated if mitigated > 0.0 else 0.0
    if target_met:
        required_additional_rrf: float | None = 0.0
    elif target > 0.0:
        required_additional_rrf = mitigated / target
    else:
        required_additional_rrf = None
    if target_met:
        required_additional_sil = 0.0
    else:
        rrf = required_additional_rrf if required_additional_rrf is not None else math.inf
        required_additional_sil = float(_required_sil(1.0 / rrf))

    contributions = tuple(
        LopaLayerContribution(
            name=str(row["name"]),
            pfd=float(row["pfd"]),
            frequency_before_per_year=float(row["frequency_before_per_year"]),
            frequency_after_per_year=float(row["frequency_after_per_year"]),
            contribution_per_year=float(row["frequency_before_per_year"])
            - float(row["frequency_after_per_year"]),
            rrf=(1.0 / float(row["pfd"])) if float(row["pfd"]) > 0.0 else None,
        )
        for row in rows
    )

    category = None if sts0131 is None else sts0131[0]
    target_frequency = None if sts0131 is None else sts0131[1]

    return LopaResult(
        target_met=target_met,
        gap_to_target=gap_to_target,
        total_rrf=total_rrf,
        required_additional_rrf=required_additional_rrf,
        required_additional_sil=required_additional_sil,
        layer_contributions=contributions,
        sts0131_pressure_category=category,
        sts0131_target_frequency_per_year=target_frequency,
        warnings=tuple(warnings),
    )
