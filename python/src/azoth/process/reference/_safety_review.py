"""The shared internals of the ``process.*`` safety-review worksheets.

The API 14C SAFE chart's device and category vocabularies, its required-device table and
the classifier. The arithmetic is NeqSim's
``process/safety/api14c/Api14cSafeChartBuilder.java``.
"""

from __future__ import annotations

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
