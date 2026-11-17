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
