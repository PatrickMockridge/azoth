//! The shared internals of the `process.*` HAZID worksheets.
//!
//! The template grid, the equipment-type seed table and (from the population id) the
//! deviation-to-calculation catalogue read one pair of HAZOP enums. The arithmetic is
//! NeqSim's `process/safety/hazid/HAZOPTemplate.java`.

use azoth_core::{AzothError, Result};
use serde::Serialize;

/// A HAZOP guide word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HazopGuideWord {
    /// Negation of the design intent.
    #[serde(rename = "no")]
    No,
    /// Quantitative increase.
    #[serde(rename = "more")]
    More,
    /// Quantitative decrease.
    #[serde(rename = "less")]
    Less,
    /// The logical opposite.
    #[serde(rename = "reverse")]
    Reverse,
    /// Qualitative increase.
    #[serde(rename = "as_well_as")]
    AsWellAs,
    /// Qualitative decrease.
    #[serde(rename = "part_of")]
    PartOf,
    /// Complete substitution.
    #[serde(rename = "other_than")]
    OtherThan,
}

impl HazopGuideWord {
    /// The class's enum order, which is the grid's row order.
    pub(crate) const ALL: [Self; 7] = [
        Self::No,
        Self::More,
        Self::Less,
        Self::Reverse,
        Self::AsWellAs,
        Self::PartOf,
        Self::OtherThan,
    ];

    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::No => "no",
            Self::More => "more",
            Self::Less => "less",
            Self::Reverse => "reverse",
            Self::AsWellAs => "as_well_as",
            Self::PartOf => "part_of",
            Self::OtherThan => "other_than",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "no" => Self::No,
            "more" => Self::More,
            "less" => Self::Less,
            "reverse" => Self::Reverse,
            "as_well_as" => Self::AsWellAs,
            "part_of" => Self::PartOf,
            "other_than" => Self::OtherThan,
            _ => {
                return Err(AzothError::invalid_input(
                    "guide_word",
                    format!(
                        "`{value}` is not `no`, `more`, `less`, `reverse`, `as_well_as`, \
                         `part_of` or `other_than`"
                    ),
                ));
            }
        })
    }
}

/// A HAZOP process parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HazopParameter {
    /// Volumetric or mass flow.
    #[serde(rename = "flow")]
    Flow,
    /// Pressure.
    #[serde(rename = "pressure")]
    Pressure,
    /// Temperature.
    #[serde(rename = "temperature")]
    Temperature,
    /// Level.
    #[serde(rename = "level")]
    Level,
    /// Composition.
    #[serde(rename = "composition")]
    Composition,
    /// Reaction rate or completion.
    #[serde(rename = "reaction")]
    Reaction,
}

impl HazopParameter {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flow => "flow",
            Self::Pressure => "pressure",
            Self::Temperature => "temperature",
            Self::Level => "level",
            Self::Composition => "composition",
            Self::Reaction => "reaction",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "flow" => Self::Flow,
            "pressure" => Self::Pressure,
            "temperature" => Self::Temperature,
            "level" => Self::Level,
            "composition" => Self::Composition,
            "reaction" => Self::Reaction,
            _ => {
                return Err(AzothError::invalid_input(
                    "parameter",
                    format!(
                        "`{value}` is not `flow`, `pressure`, `temperature`, `level`, \
                         `composition` or `reaction`"
                    ),
                ));
            }
        })
    }
}

/// One seeded deviation pair.
pub(crate) type SeedRow = (HazopGuideWord, HazopParameter);

const SEPARATOR_SEEDS: &[SeedRow] = &[
    (HazopGuideWord::No, HazopParameter::Flow),
    (HazopGuideWord::More, HazopParameter::Level),
    (HazopGuideWord::Less, HazopParameter::Level),
    (HazopGuideWord::More, HazopParameter::Pressure),
    (HazopGuideWord::Less, HazopParameter::Pressure),
];

const COMPRESSOR_SEEDS: &[SeedRow] = &[
    (HazopGuideWord::No, HazopParameter::Flow),
    (HazopGuideWord::Reverse, HazopParameter::Flow),
    (HazopGuideWord::More, HazopParameter::Pressure),
    (HazopGuideWord::More, HazopParameter::Temperature),
];

const PUMP_SEEDS: &[SeedRow] = &[
    (HazopGuideWord::No, HazopParameter::Flow),
    (HazopGuideWord::Reverse, HazopParameter::Flow),
    (HazopGuideWord::More, HazopParameter::Pressure),
];

const VALVE_SEEDS: &[SeedRow] = &[
    (HazopGuideWord::No, HazopParameter::Flow),
    (HazopGuideWord::More, HazopParameter::Flow),
    (HazopGuideWord::Less, HazopParameter::Flow),
];

const HEAT_EXCHANGER_SEEDS: &[SeedRow] = &[
    (HazopGuideWord::More, HazopParameter::Temperature),
    (HazopGuideWord::Less, HazopParameter::Temperature),
    (HazopGuideWord::No, HazopParameter::Flow),
];

const PIPE_SEEDS: &[SeedRow] = &[
    (HazopGuideWord::No, HazopParameter::Flow),
    (HazopGuideWord::Less, HazopParameter::Flow),
    (HazopGuideWord::Less, HazopParameter::Pressure),
];

const DEFAULT_SEEDS: &[SeedRow] = &[
    (HazopGuideWord::No, HazopParameter::Flow),
    (HazopGuideWord::More, HazopParameter::Flow),
    (HazopGuideWord::More, HazopParameter::Pressure),
    (HazopGuideWord::Less, HazopParameter::Pressure),
];

/// The class's equipment-type seed set, by lower-case substring in the class's order.
#[must_use]
pub(crate) fn seed_rows(equipment_type: &str) -> &'static [SeedRow] {
    let kind = equipment_type.to_ascii_lowercase();
    if kind.contains("separator") || kind.contains("scrubber") {
        SEPARATOR_SEEDS
    } else if kind.contains("compressor") || kind.contains("expander") {
        COMPRESSOR_SEEDS
    } else if kind.contains("pump") {
        PUMP_SEEDS
    } else if kind.contains("valve") {
        VALVE_SEEDS
    } else if kind.contains("cooler")
        || kind.contains("heater")
        || kind.contains("heatexchanger")
        || kind.contains("reboiler")
        || kind.contains("condenser")
    {
        HEAT_EXCHANGER_SEEDS
    } else if kind.contains("pipe") {
        PIPE_SEEDS
    } else {
        DEFAULT_SEEDS
    }
}

/// The equipment category a consequence mapping is constrained to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HazopEquipmentCategory {
    /// Equipment-neutral mapping.
    Generic,
    /// Material stream.
    Stream,
    /// Flow-control or throttling valve.
    Valve,
    /// Liquid pump.
    Pump,
    /// Compressor or expander.
    Compressor,
    /// Separator or scrubber.
    Separator,
    /// Heater, cooler, heat exchanger, reboiler or condenser.
    HeatExchanger,
    /// Pipe or pipeline.
    Pipeline,
    /// Recognised process equipment outside the categories above.
    Other,
}

impl HazopEquipmentCategory {
    /// The spec's spelling.
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Stream => "stream",
            Self::Valve => "valve",
            Self::Pump => "pump",
            Self::Compressor => "compressor",
            Self::Separator => "separator",
            Self::HeatExchanger => "heat_exchanger",
            Self::Pipeline => "pipeline",
            Self::Other => "other",
        }
    }
}

/// The class's equipment categorisation, by lower-case substring in its own order.
#[must_use]
pub(crate) fn equipment_category(equipment_type: Option<&str>) -> HazopEquipmentCategory {
    let Some(equipment_type) = equipment_type else {
        return HazopEquipmentCategory::Generic;
    };
    if equipment_type.trim().is_empty() {
        return HazopEquipmentCategory::Generic;
    }
    let kind = equipment_type.to_ascii_lowercase();
    if kind.contains("stream") {
        HazopEquipmentCategory::Stream
    } else if kind.contains("valve") {
        HazopEquipmentCategory::Valve
    } else if kind.contains("pump") {
        HazopEquipmentCategory::Pump
    } else if kind.contains("compressor") || kind.contains("expander") {
        HazopEquipmentCategory::Compressor
    } else if kind.contains("separator") || kind.contains("scrubber") {
        HazopEquipmentCategory::Separator
    } else if kind.contains("cooler")
        || kind.contains("heater")
        || kind.contains("heatexchanger")
        || kind.contains("heat exchanger")
        || kind.contains("reboiler")
        || kind.contains("condenser")
    {
        HazopEquipmentCategory::HeatExchanger
    } else if kind.contains("pipe") {
        HazopEquipmentCategory::Pipeline
    } else {
        HazopEquipmentCategory::Other
    }
}

/// One catalogue entry.
pub(crate) struct ConsequenceMapping {
    pub guide_word: HazopGuideWord,
    pub parameter: HazopParameter,
    pub category: HazopEquipmentCategory,
    pub consequence: &'static str,
    pub calculator: &'static str,
    pub standard: &'static str,
    pub safeguard: &'static str,
    pub facilitator: bool,
}

const fn mapping(
    guide_word: HazopGuideWord,
    parameter: HazopParameter,
    category: HazopEquipmentCategory,
    consequence: &'static str,
    calculator: &'static str,
    standard: &'static str,
    safeguard: &'static str,
) -> ConsequenceMapping {
    ConsequenceMapping {
        guide_word,
        parameter,
        category,
        consequence,
        calculator,
        standard,
        safeguard,
        facilitator: false,
    }
}

/// The eleven generic mappings, in the class's declaration order.
pub(crate) const GENERIC_MAPPINGS: &[ConsequenceMapping] = &[
    mapping(
        HazopGuideWord::More,
        HazopParameter::Pressure,
        HazopEquipmentCategory::Generic,
        "Overpressure of equipment beyond MAWP from blocked outlet, gas blow-by or settle-out",
        "ReliefValveSizing, SettleOutPressureAnalyzer, GasBlowbyAnalyzer",
        "API 520 / API 521",
        "Pressure safety valve (PSV) and/or high-integrity pressure protection system (HIPPS)",
    ),
    mapping(
        HazopGuideWord::Less,
        HazopParameter::Pressure,
        HazopEquipmentCategory::Generic,
        "Vacuum collapse of a vessel from cooling, draining or blocked make-up gas",
        "VacuumCollapseAnalyzer",
        "API 521 / ASME BPVC",
        "Vacuum breaker / vacuum relief valve",
    ),
    mapping(
        HazopGuideWord::Less,
        HazopParameter::Temperature,
        HazopEquipmentCategory::Generic,
        "Autorefrigeration / low-temperature brittle fracture from depressurisation or J-T cooling",
        "MDMTCalculator, DepressurizationSimulator",
        "ASME UCS-66 / API 521",
        "Low-temperature trip, material selection for MDMT",
    ),
    mapping(
        HazopGuideWord::More,
        HazopParameter::Temperature,
        HazopEquipmentCategory::Generic,
        "External fire heat input causing thermal expansion, overpressure or wall rupture",
        "TrappedLiquidFireRuptureStudy, PipeFireRuptureStudy, PfpDemandCalculator",
        "API 521",
        "Thermal safety valve (TSV), passive fire protection (PFP), depressurisation",
    ),
    mapping(
        HazopGuideWord::No,
        HazopParameter::Flow,
        HazopEquipmentCategory::Generic,
        "Pump deadhead — blocked discharge causing temperature rise and shutoff overpressure",
        "PumpDeadheadAnalyzer",
        "API 610",
        "Minimum-flow recycle line, low-flow trip",
    ),
    mapping(
        HazopGuideWord::More,
        HazopParameter::Flow,
        HazopEquipmentCategory::Generic,
        "Compressor surge or high pipe velocity causing erosion and mechanical damage",
        "getUtilizationSnapshot (surge / velocity constraints)",
        "API 14E / API 617",
        "Anti-surge controller, flow restriction / velocity limit",
    ),
    mapping(
        HazopGuideWord::Less,
        HazopParameter::Flow,
        HazopEquipmentCategory::Generic,
        "Low flow causing deadhead, overheating or loss of cooling duty",
        "PumpDeadheadAnalyzer",
        "API 610",
        "Minimum-flow recycle, low-flow alarm",
    ),
    mapping(
        HazopGuideWord::Reverse,
        HazopParameter::Flow,
        HazopEquipmentCategory::Generic,
        "Backflow / check-valve slam generating hydraulic surge (water hammer)",
        "WaterHammerStudy",
        "API 521",
        "Check valve, slow-closing actuator, surge analysis",
    ),
    mapping(
        HazopGuideWord::AsWellAs,
        HazopParameter::Composition,
        HazopEquipmentCategory::Generic,
        "Contamination / unexpected phase (water, heavies) causing hydrate, corrosion or carry-over",
        "phase / hydrate / separator screening (TPflash, hydrate equilibrium)",
        "API 521",
        "Composition monitoring, hydrate inhibitor injection, separator design margin",
    ),
    mapping(
        HazopGuideWord::PartOf,
        HazopParameter::Composition,
        HazopEquipmentCategory::Generic,
        "Loss of a key component changing phase behaviour or downstream specification",
        "phase / separator screening (TPflash)",
        "API 521",
        "Composition analyser, specification alarm",
    ),
    mapping(
        HazopGuideWord::OtherThan,
        HazopParameter::Reaction,
        HazopEquipmentCategory::Generic,
        "Runaway / unintended exothermic reaction exceeding the maximum allowable temperature",
        "RunawayReactionAnalyzer",
        "DIERS / API 521",
        "Reaction quench, emergency relief (two-phase), reaction inhibitor",
    ),
];

/// The six equipment-specific flow mappings, in the class's declaration order.
pub(crate) const EQUIPMENT_MAPPINGS: &[ConsequenceMapping] = &[
    mapping(
        HazopGuideWord::No,
        HazopParameter::Flow,
        HazopEquipmentCategory::Pump,
        "Pump deadhead — blocked discharge causing temperature rise and shutoff overpressure",
        "PumpDeadheadAnalyzer",
        "API 610",
        "Minimum-flow recycle line, low-flow trip",
    ),
    mapping(
        HazopGuideWord::No,
        HazopParameter::Flow,
        HazopEquipmentCategory::Compressor,
        "Loss of forward flow can drive a running compressor into surge or recycle heating",
        "getUtilizationSnapshot (surge constraint) and compressor performance-chart review",
        "API 617",
        "Anti-surge recycle, low-flow trip and discharge check valve",
    ),
    mapping(
        HazopGuideWord::Reverse,
        HazopParameter::Flow,
        HazopEquipmentCategory::Compressor,
        "Reverse flow can cause compressor backspin, surge and mechanical damage",
        "Compressor performance-chart and transient check-valve review",
        "API 617",
        "Discharge check valve, anti-surge recycle and reverse-rotation protection",
    ),
    mapping(
        HazopGuideWord::Reverse,
        HazopParameter::Flow,
        HazopEquipmentCategory::Pump,
        "Reverse liquid flow can cause pump backspin and check-valve hydraulic surge",
        "WaterHammerStudy",
        "API 610 / API 521",
        "Discharge check valve, controlled closure and reverse-rotation protection",
    ),
    mapping(
        HazopGuideWord::More,
        HazopParameter::Flow,
        HazopEquipmentCategory::Valve,
        "Excess valve flow can overload downstream equipment or exceed line velocity and noise limits",
        "ThrottlingValve capacity review and getUtilizationSnapshot",
        "IEC 60534 / API 14E",
        "Flow control, high-flow alarm and downstream capacity constraint",
    ),
    mapping(
        HazopGuideWord::More,
        HazopParameter::Flow,
        HazopEquipmentCategory::Stream,
        "Excess stream flow can overload downstream equipment or exceed line velocity limits",
        "getUtilizationSnapshot (downstream capacity and velocity constraints)",
        "API 14E",
        "High-flow alarm, flow limitation and verified downstream capacity",
    ),
];

/// The mapping a deviation resolves to, borrowed from the table or generated as a fallback.
pub(crate) struct ChosenMapping {
    pub consequence: String,
    pub calculator: &'static str,
    pub standard: &'static str,
    pub safeguard: &'static str,
    pub facilitator: bool,
}

fn chosen(entry: &ConsequenceMapping) -> ChosenMapping {
    ChosenMapping {
        consequence: entry.consequence.to_string(),
        calculator: entry.calculator,
        standard: entry.standard,
        safeguard: entry.safeguard,
        facilitator: entry.facilitator,
    }
}

fn generic_mapping(guide_word: HazopGuideWord, parameter: HazopParameter) -> Option<ChosenMapping> {
    GENERIC_MAPPINGS
        .iter()
        .find(|entry| entry.guide_word == guide_word && entry.parameter == parameter)
        .map(chosen)
}

fn equipment_mapping(
    category: HazopEquipmentCategory,
    guide_word: HazopGuideWord,
    parameter: HazopParameter,
) -> Option<ChosenMapping> {
    EQUIPMENT_MAPPINGS
        .iter()
        .find(|entry| {
            entry.category == category
                && entry.guide_word == guide_word
                && entry.parameter == parameter
        })
        .map(chosen)
}

fn neutral_flow_mapping(
    guide_word: HazopGuideWord,
    category: HazopEquipmentCategory,
) -> ChosenMapping {
    let equipment = category.as_str().replace('_', " ");
    let deviation = guide_word.as_str().to_uppercase().replace('_', " ");
    ChosenMapping {
        consequence: format!(
            "{deviation} FLOW on {equipment} equipment has topology-dependent causes and \
             consequences; facilitator confirmation required"
        ),
        calculator: "Facilitator review of process topology, inventory and transient response",
        standard: "IEC 61882 / CCPS",
        safeguard: "Confirm applicable alarms, trips, isolation, relief and operating safeguards \
                    in the HAZOP workshop",
        facilitator: true,
    }
}

/// The mapping for a deviation, the class's `mappingFor(guideWord, parameter, equipmentType)`.
#[must_use]
pub(crate) fn mapping_for(
    guide_word: HazopGuideWord,
    parameter: HazopParameter,
    equipment_type: Option<&str>,
) -> Option<ChosenMapping> {
    let category = equipment_category(equipment_type);
    if category == HazopEquipmentCategory::Generic {
        return generic_mapping(guide_word, parameter);
    }
    if let Some(entry) = equipment_mapping(category, guide_word, parameter) {
        return Some(entry);
    }
    if parameter == HazopParameter::Flow {
        return Some(neutral_flow_mapping(guide_word, category));
    }
    generic_mapping(guide_word, parameter)
}

/// Whether a cell is the class's placeholder: null, blank or `TBD` after trimming.
#[must_use]
pub(crate) fn is_placeholder(value: Option<&str>) -> bool {
    match value {
        None => true,
        Some(value) => {
            let trimmed = value.trim();
            trimmed.is_empty() || trimmed.eq_ignore_ascii_case("TBD")
        }
    }
}

/// The class's recommendation citation for a chosen mapping.
#[must_use]
pub(crate) fn recommendation_for(chosen: &ChosenMapping) -> String {
    if chosen.facilitator {
        format!(
            "Facilitator confirmation required: review causes, consequence pathways, safeguards \
             and calculation basis per {}",
            chosen.standard
        )
    } else {
        format!("Quantify with {} per {}", chosen.calculator, chosen.standard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_table_takes_the_first_match() {
        assert_eq!(seed_rows("Separator").len(), 5);
        assert_eq!(seed_rows("Scrubber").len(), 5);
        assert_eq!(seed_rows("Compressor").len(), 4);
        assert_eq!(seed_rows("Expander").len(), 4);
        assert_eq!(seed_rows("Pump").len(), 3);
        assert_eq!(seed_rows("ThrottlingValve").len(), 3);
        assert_eq!(seed_rows("Cooler").len(), 3);
        assert_eq!(seed_rows("PipeBeggsAndBrills").len(), 3);
        assert_eq!(seed_rows("Stream").len(), 4);
        assert_eq!(seed_rows("").len(), 4);
    }
}
