"""The shared internals of the ``process.*`` HAZID worksheets.

The guide-word and parameter orders, the equipment-type seed table and the class's
``TBD`` marker. The Python twin of ``crates/azoth-process/src/models/hazid.rs``; the
arithmetic is NeqSim's ``process/safety/hazid/HAZOPTemplate.java``.
"""

from __future__ import annotations

GUIDE_WORDS = ("no", "more", "less", "reverse", "as_well_as", "part_of", "other_than")
PARAMETERS = ("flow", "pressure", "temperature", "level", "composition", "reaction")

TBD = "TBD"

_SEED_TABLE: tuple[tuple[tuple[str, ...], tuple[tuple[str, str], ...]], ...] = (
    (
        ("separator", "scrubber"),
        (
            ("no", "flow"),
            ("more", "level"),
            ("less", "level"),
            ("more", "pressure"),
            ("less", "pressure"),
        ),
    ),
    (
        ("compressor", "expander"),
        (("no", "flow"), ("reverse", "flow"), ("more", "pressure"), ("more", "temperature")),
    ),
    (("pump",), (("no", "flow"), ("reverse", "flow"), ("more", "pressure"))),
    (("valve",), (("no", "flow"), ("more", "flow"), ("less", "flow"))),
    (
        ("cooler", "heater", "heatexchanger", "reboiler", "condenser"),
        (("more", "temperature"), ("less", "temperature"), ("no", "flow")),
    ),
    (("pipe",), (("no", "flow"), ("less", "flow"), ("less", "pressure"))),
)

_DEFAULT_SEEDS = (
    ("no", "flow"),
    ("more", "flow"),
    ("more", "pressure"),
    ("less", "pressure"),
)


def seed_rows(equipment_type: str) -> tuple[tuple[str, str], ...]:
    """The class's equipment-type seed set, by lower-case substring in its own order."""
    kind = equipment_type.lower()
    for markers, rows in _SEED_TABLE:
        if any(marker in kind for marker in markers):
            return rows
    return _DEFAULT_SEEDS


#: The eleven generic mappings, in the class's declaration order:
#: ``(guide_word, parameter, consequence, calculator, standard, safeguard)``.
GENERIC_MAPPINGS: tuple[tuple[str, str, str, str, str, str], ...] = (
    (
        "more",
        "pressure",
        "Overpressure of equipment beyond MAWP from blocked outlet, gas blow-by or settle-out",
        "ReliefValveSizing, SettleOutPressureAnalyzer, GasBlowbyAnalyzer",
        "API 520 / API 521",
        "Pressure safety valve (PSV) and/or high-integrity pressure protection system (HIPPS)",
    ),
    (
        "less",
        "pressure",
        "Vacuum collapse of a vessel from cooling, draining or blocked make-up gas",
        "VacuumCollapseAnalyzer",
        "API 521 / ASME BPVC",
        "Vacuum breaker / vacuum relief valve",
    ),
    (
        "less",
        "temperature",
        "Autorefrigeration / low-temperature brittle fracture from depressurisation or J-T cooling",
        "MDMTCalculator, DepressurizationSimulator",
        "ASME UCS-66 / API 521",
        "Low-temperature trip, material selection for MDMT",
    ),
    (
        "more",
        "temperature",
        "External fire heat input causing thermal expansion, overpressure or wall rupture",
        "TrappedLiquidFireRuptureStudy, PipeFireRuptureStudy, PfpDemandCalculator",
        "API 521",
        "Thermal safety valve (TSV), passive fire protection (PFP), depressurisation",
    ),
    (
        "no",
        "flow",
        "Pump deadhead — blocked discharge causing temperature rise and shutoff overpressure",
        "PumpDeadheadAnalyzer",
        "API 610",
        "Minimum-flow recycle line, low-flow trip",
    ),
    (
        "more",
        "flow",
        "Compressor surge or high pipe velocity causing erosion and mechanical damage",
        "getUtilizationSnapshot (surge / velocity constraints)",
        "API 14E / API 617",
        "Anti-surge controller, flow restriction / velocity limit",
    ),
    (
        "less",
        "flow",
        "Low flow causing deadhead, overheating or loss of cooling duty",
        "PumpDeadheadAnalyzer",
        "API 610",
        "Minimum-flow recycle, low-flow alarm",
    ),
    (
        "reverse",
        "flow",
        "Backflow / check-valve slam generating hydraulic surge (water hammer)",
        "WaterHammerStudy",
        "API 521",
        "Check valve, slow-closing actuator, surge analysis",
    ),
    (
        "as_well_as",
        "composition",
        "Contamination / unexpected phase (water, heavies) causing hydrate, corrosion or "
        "carry-over",
        "phase / hydrate / separator screening (TPflash, hydrate equilibrium)",
        "API 521",
        "Composition monitoring, hydrate inhibitor injection, separator design margin",
    ),
    (
        "part_of",
        "composition",
        "Loss of a key component changing phase behaviour or downstream specification",
        "phase / separator screening (TPflash)",
        "API 521",
        "Composition analyser, specification alarm",
    ),
    (
        "other_than",
        "reaction",
        "Runaway / unintended exothermic reaction exceeding the maximum allowable temperature",
        "RunawayReactionAnalyzer",
        "DIERS / API 521",
        "Reaction quench, emergency relief (two-phase), reaction inhibitor",
    ),
)

#: The six equipment-specific flow mappings, in the class's declaration order:
#: ``(category, guide_word, parameter, consequence, calculator, standard, safeguard)``.
EQUIPMENT_MAPPINGS: tuple[tuple[str, str, str, str, str, str, str], ...] = (
    (
        "pump",
        "no",
        "flow",
        "Pump deadhead — blocked discharge causing temperature rise and shutoff overpressure",
        "PumpDeadheadAnalyzer",
        "API 610",
        "Minimum-flow recycle line, low-flow trip",
    ),
    (
        "compressor",
        "no",
        "flow",
        "Loss of forward flow can drive a running compressor into surge or recycle heating",
        "getUtilizationSnapshot (surge constraint) and compressor performance-chart review",
        "API 617",
        "Anti-surge recycle, low-flow trip and discharge check valve",
    ),
    (
        "compressor",
        "reverse",
        "flow",
        "Reverse flow can cause compressor backspin, surge and mechanical damage",
        "Compressor performance-chart and transient check-valve review",
        "API 617",
        "Discharge check valve, anti-surge recycle and reverse-rotation protection",
    ),
    (
        "pump",
        "reverse",
        "flow",
        "Reverse liquid flow can cause pump backspin and check-valve hydraulic surge",
        "WaterHammerStudy",
        "API 610 / API 521",
        "Discharge check valve, controlled closure and reverse-rotation protection",
    ),
    (
        "valve",
        "more",
        "flow",
        "Excess valve flow can overload downstream equipment or exceed line velocity and noise "
        "limits",
        "ThrottlingValve capacity review and getUtilizationSnapshot",
        "IEC 60534 / API 14E",
        "Flow control, high-flow alarm and downstream capacity constraint",
    ),
    (
        "stream",
        "more",
        "flow",
        "Excess stream flow can overload downstream equipment or exceed line velocity limits",
        "getUtilizationSnapshot (downstream capacity and velocity constraints)",
        "API 14E",
        "High-flow alarm, flow limitation and verified downstream capacity",
    ),
)


def equipment_category(equipment_type: str | None) -> str:
    """The class's equipment categorisation, by lower-case substring in its own order."""
    if equipment_type is None or not equipment_type.strip():
        return "generic"
    kind = equipment_type.lower()
    if "stream" in kind:
        return "stream"
    if "valve" in kind:
        return "valve"
    if "pump" in kind:
        return "pump"
    if "compressor" in kind or "expander" in kind:
        return "compressor"
    if "separator" in kind or "scrubber" in kind:
        return "separator"
    if (
        "cooler" in kind
        or "heater" in kind
        or "heatexchanger" in kind
        or "heat exchanger" in kind
        or "reboiler" in kind
        or "condenser" in kind
    ):
        return "heat_exchanger"
    if "pipe" in kind:
        return "pipeline"
    return "other"


def mapping_for(
    guide_word: str, parameter: str, equipment_type: str | None
) -> dict[str, object] | None:
    """The mapping for a deviation, the class's ``mappingFor(guideWord, parameter, type)``."""
    category = equipment_category(equipment_type)

    def generic() -> dict[str, object] | None:
        for row in GENERIC_MAPPINGS:
            if row[0] == guide_word and row[1] == parameter:
                return {
                    "category": "generic",
                    "consequence": row[2],
                    "calculator": row[3],
                    "standard": row[4],
                    "safeguard": row[5],
                    "facilitator": False,
                }
        return None

    if category == "generic":
        return generic()
    for row in EQUIPMENT_MAPPINGS:
        if row[0] == category and row[1] == guide_word and row[2] == parameter:
            return {
                "category": category,
                "consequence": row[3],
                "calculator": row[4],
                "standard": row[5],
                "safeguard": row[6],
                "facilitator": False,
            }
    if parameter == "flow":
        equipment = category.replace("_", " ")
        deviation = guide_word.upper().replace("_", " ")
        return {
            "category": category,
            "consequence": (
                f"{deviation} FLOW on {equipment} equipment has topology-dependent causes and "
                "consequences; facilitator confirmation required"
            ),
            "calculator": (
                "Facilitator review of process topology, inventory and transient response"
            ),
            "standard": "IEC 61882 / CCPS",
            "safeguard": (
                "Confirm applicable alarms, trips, isolation, relief and operating safeguards "
                "in the HAZOP workshop"
            ),
            "facilitator": True,
        }
    return generic()


def recommendation_for(chosen: dict[str, object]) -> str:
    """The class's recommendation citation for a chosen mapping."""
    if bool(chosen["facilitator"]):
        return (
            "Facilitator confirmation required: review causes, consequence pathways, safeguards "
            f"and calculation basis per {chosen['standard']}"
        )
    return f"Quantify with {chosen['calculator']} per {chosen['standard']}"


def is_placeholder(value: str | None) -> bool:
    """Whether a cell is the class's placeholder: null, blank or `TBD` after trimming."""
    if value is None:
        return True
    trimmed = value.strip()
    return trimmed == "" or trimmed.upper() == "TBD"
