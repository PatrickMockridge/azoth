"""The shared internals of the ``process.*`` safety-review worksheets.

The API 14C SAFE chart's device and category vocabularies, its required-device table and
the classifier. The arithmetic is NeqSim's
``process/safety/api14c/Api14cSafeChartBuilder.java``.
"""

from __future__ import annotations

import math

#: The class's enum order, which is ``Api14cSafetyAnalysisTable.deviceOrder()``.
API14C_DEVICES: tuple[str, ...] = (
    "psh",
    "psl",
    "lsh",
    "lsl",
    "tsh",
    "tsl",
    "usv",
    "sdv",
    "bdv",
    "psv",
    "fsv",
    "fire",
    "gas",
)

#: The category vocabulary.
API14C_CATEGORIES: tuple[str, ...] = (
    "pressure_vessel",
    "atmospheric_vessel",
    "fired_vessel",
    "pipeline_segment",
    "compressor",
    "pump",
    "heat_exchanger",
    "wellhead",
)

#: The required devices per category, in the device declaration order.
API14C_REQUIRED: dict[str, tuple[str, ...]] = {
    "pressure_vessel": ("psh", "psl", "lsh", "lsl", "sdv", "psv"),
    "atmospheric_vessel": ("lsh", "psv", "fsv"),
    "fired_vessel": ("psh", "lsl", "tsh", "sdv", "bdv", "psv", "fire"),
    "pipeline_segment": ("psh", "psl", "sdv", "fsv"),
    "compressor": ("psh", "psl", "tsh", "sdv", "bdv", "psv"),
    "pump": ("psh", "sdv", "psv", "fsv"),
    "heat_exchanger": ("psh", "tsh", "psv"),
    "wellhead": ("psh", "psl", "usv", "sdv", "psv"),
}


#: The element types the register accepts.
ELEMENT_TYPES: tuple[str, ...] = (
    "process_equipment",
    "instrumented_function",
    "fire_protection",
    "structural",
    "utility",
    "other",
)

#: The barrier types.
BARRIER_TYPES: tuple[str, ...] = ("prevention", "mitigation", "both")

#: The barrier statuses.
BARRIER_STATUSES: tuple[str, ...] = (
    "available",
    "impaired",
    "bypassed",
    "out_of_service",
    "unknown",
)

#: Which record a piece of evidence is attached to.
EVIDENCE_OWNERS: tuple[str, ...] = ("element", "barrier", "standard")


def _positive(value: float | None) -> float | None:
    return None if value is None or not math.isfinite(value) or value <= 0.0 else value


def s001_default_target(
    maximum_event_pressure: float | None,
    design_pressure: float | None,
    test_pressure: float | None,
) -> float:
    """The class's default target frequency by pressure band, NaN without a complete basis."""
    event = _positive(maximum_event_pressure)
    design = _positive(design_pressure)
    test = _positive(test_pressure)
    if event is None or design is None or test is None:
        return math.nan
    if event > 2.0 * design:
        return 1.0e-5
    if event > test:
        return 1.0e-4
    if event > design:
        return 1.0e-3
    return 1.0e-2


def s001_evaluate(
    maximum_event_pressure: float | None,
    design_pressure: float | None,
    test_pressure: float | None,
    demand_frequency_per_year: float | None,
    target_frequency_per_year: float | None,
    relief_leakage_assessed: bool | None,
    relief_leakage_to_safe_location: bool | None,
    proof_test_interval_months: float | None,
) -> dict[str, object]:
    """The class's evaluation of the S-001 criteria."""
    pressure_basis_complete = (
        _positive(maximum_event_pressure) is not None
        and _positive(design_pressure) is not None
        and _positive(test_pressure) is not None
    )
    test_pressure_valid = not pressure_basis_complete or (
        float(test_pressure) >= float(design_pressure)  # type: ignore[arg-type]
    )
    pressure_within_test = not pressure_basis_complete or (
        float(maximum_event_pressure) <= float(test_pressure)  # type: ignore[arg-type]
    )
    stated_target = (
        target_frequency_per_year
        if target_frequency_per_year is not None and math.isfinite(target_frequency_per_year)
        else None
    )
    resolved_target = (
        stated_target
        if stated_target is not None
        else s001_default_target(maximum_event_pressure, design_pressure, test_pressure)
    )
    frequency_configured = (
        demand_frequency_per_year is not None
        and math.isfinite(demand_frequency_per_year)
        and demand_frequency_per_year >= 0.0
        and math.isfinite(resolved_target)
    )
    frequency_criterion_met = not frequency_configured or (
        float(demand_frequency_per_year) <= resolved_target  # type: ignore[arg-type]
    )
    leakage_assessed = relief_leakage_assessed is True
    leakage_routed_safe = relief_leakage_to_safe_location is True
    proof_test_configured = proof_test_interval_months is not None and math.isfinite(
        proof_test_interval_months
    )
    proof_test_criterion_met = not proof_test_configured or (
        float(proof_test_interval_months) <= 12.0  # type: ignore[arg-type]
    )
    acceptable = (
        pressure_basis_complete
        and test_pressure_valid
        and pressure_within_test
        and frequency_configured
        and frequency_criterion_met
        and leakage_assessed
        and leakage_routed_safe
        and proof_test_criterion_met
    )
    return {
        "target_frequency_per_year": resolved_target if math.isfinite(resolved_target) else None,
        "pressure_basis_complete": pressure_basis_complete,
        "test_pressure_valid": test_pressure_valid,
        "pressure_within_test": pressure_within_test,
        "frequency_configured": frequency_configured,
        "frequency_criterion_met": frequency_criterion_met,
        "leakage_assessed": leakage_assessed,
        "leakage_routed_safe": leakage_routed_safe,
        "proof_test_configured": proof_test_configured,
        "proof_test_criterion_met": proof_test_criterion_met,
        "acceptable": acceptable,
    }


def classify_api14c(unit_class: str) -> str | None:
    """The class's equipment classifier: a lower-case substring test on the simple name."""
    name = unit_class.lower()
    if (
        "scrubber" in name
        or "separator" in name
        or "flashdrum" in name
        or "knockout" in name
        or ("flash" in name and "drum" in name)
    ):
        return "pressure_vessel"
    if "tank" in name:
        return "atmospheric_vessel"
    if "heater" in name or "furnace" in name or "reformer" in name:
        return "fired_vessel"
    if "pipe" in name or "flowline" in name or "riser" in name:
        return "pipeline_segment"
    if "compressor" in name or "expander" in name:
        return "compressor"
    if "pump" in name:
        return "pump"
    if (
        "heatexchanger" in name
        or "cooler" in name
        or "condenser" in name
        or "reboiler" in name
        or "aircooler" in name
    ):
        return "heat_exchanger"
    if "well" in name:
        return "wellhead"
    return None
