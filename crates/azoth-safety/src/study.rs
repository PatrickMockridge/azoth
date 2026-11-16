//! The saved safety study.
//!
//! **A document of its own.** A scenario names one registered `process.*` model and the
//! inputs it was given; a bow-tie carries labelled threats, barriers and consequences, of
//! which only the numbers reach `process.bow_tie`. Results are derived and never saved.
//!
//! The inputs are SI magnitudes for quantities, which is the same convention the
//! calculation book and the flowsheet's stream records cross with.

use std::collections::{BTreeMap, BTreeSet};

use azoth_core::{AzothError, Result};
use serde::{Deserialize, Serialize};

/// The study's id when a document does not give one.
const DEFAULT_ID: &str = "safety_study_1";

/// The study's name when a document does not give one.
const DEFAULT_NAME: &str = "Untitled safety study";

fn default_id() -> String {
    DEFAULT_ID.to_string()
}

fn default_name() -> String {
    DEFAULT_NAME.to_string()
}

const fn yes() -> bool {
    true
}

/// One fire scenario: a registered model and the inputs it was given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    /// One registered model id, e.g. `process.jet_fire`.
    pub calculation: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inputs: BTreeMap<String, toml::Value>,
}

/// One cause of the top event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Threat {
    pub id: String,
    pub label: String,
    /// 1/year; the vocabulary has no year, so the value is stored and shown unconverted.
    pub frequency: f64,
    /// Whether the threat contributes. **Absent is `true`.**
    #[serde(default = "yes")]
    pub active: bool,
}

/// One barrier. Only a functional `prevention` or `both` barrier multiplies the frequency;
/// a `mitigation` barrier is recorded and read by no formula.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Barrier {
    pub id: String,
    pub label: String,
    /// The probability of failure on demand, in `[0, 1]`.
    pub pfd: f64,
    /// `prevention`, `mitigation` or `both`.
    pub barrier_type: String,
    /// Whether the barrier is functional. **Absent is `true`.**
    #[serde(default = "yes")]
    pub functional: bool,
}

/// One outcome of the top event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consequence {
    pub id: String,
    pub label: String,
    /// A whole number at least zero; the model's own severity.
    pub severity: f64,
    /// The scenario that models this outcome, if one does. **Presentation only**: it does not
    /// change the severity the bow-tie computes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
}

/// One bow-tie diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bowtie {
    pub id: String,
    pub name: String,
    /// The top-event label. The numeric model carries no label; this is document data.
    pub top_event: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threats: Vec<Threat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub barriers: Vec<Barrier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consequences: Vec<Consequence>,
}

/// Where one bow-tie's nodes sit on the canvas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BowtieLayout {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_event: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub threats: BTreeMap<String, [f64; 2]>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub barriers: BTreeMap<String, [f64; 2]>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consequences: BTreeMap<String, [f64; 2]>,
}

/// The diagram positions of every bow-tie in a study.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyLayout {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bowties: BTreeMap<String, BowtieLayout>,
}

/// A saved safety study.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyStudy {
    #[serde(default = "default_id")]
    pub id: String,
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<Scenario>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bowties: Vec<Bowtie>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<SafetyLayout>,
}

impl Default for SafetyStudy {
    fn default() -> Self {
        Self {
            id: default_id(),
            name: default_name(),
            scenarios: Vec::new(),
            bowties: Vec::new(),
            layout: None,
        }
    }
}

impl SafetyStudy {
    /// Read a study from its TOML.
    ///
    /// # Errors
    /// [`AzothError::InvalidInput`] for a document the schema does not read, an empty id or
    /// name, or an id that appears twice where it must be addressable. An empty document is a
    /// blank study, which is what New starts from.
    pub fn parse(document: &str) -> Result<Self> {
        if document.trim().is_empty() {
            return Ok(Self::default());
        }
        let study: Self = toml::from_str(document).map_err(|error| {
            AzothError::invalid_input("safety study", format!("it could not be read: {error}"))
        })?;
        study.validate()?;
        Ok(study)
    }

    /// The study as TOML, which is what a save writes.
    ///
    /// # Errors
    /// A value the TOML writer cannot express, which this shape does not produce.
    pub fn to_document(&self) -> Result<String> {
        toml::to_string(self).map_err(|error| {
            AzothError::invalid_input("safety study", format!("it could not be written: {error}"))
        })
    }

    /// One scenario by id.
    #[must_use]
    pub fn scenario(&self, id: &str) -> Option<&Scenario> {
        self.scenarios.iter().find(|scenario| scenario.id == id)
    }

    /// One bow-tie by id.
    #[must_use]
    pub fn bowtie(&self, id: &str) -> Option<&Bowtie> {
        self.bowties.iter().find(|bowtie| bowtie.id == id)
    }

    /// The identity rules the schema cannot express: an empty label is not a name, and an id
    /// that appears twice cannot be edited.
    ///
    /// # Errors
    /// [`AzothError::InvalidInput`] naming the first violation.
    pub fn validate(&self) -> Result<()> {
        require_text("the study's id", &self.id)?;
        require_text("the study's name", &self.name)?;
        unique("scenario", self.scenarios.iter().map(|row| row.id.as_str()))?;
        for scenario in &self.scenarios {
            require_text("a scenario id", &scenario.id)?;
            require_text("a scenario name", &scenario.name)?;
            require_text("a scenario calculation", &scenario.calculation)?;
        }
        unique("bow-tie", self.bowties.iter().map(|row| row.id.as_str()))?;
        for bowtie in &self.bowties {
            require_text("a bow-tie id", &bowtie.id)?;
            require_text("a bow-tie name", &bowtie.name)?;
            require_text("a top-event label", &bowtie.top_event)?;
            unique("threat", bowtie.threats.iter().map(|row| row.id.as_str()))?;
            unique("barrier", bowtie.barriers.iter().map(|row| row.id.as_str()))?;
            unique(
                "consequence",
                bowtie.consequences.iter().map(|row| row.id.as_str()),
            )?;
            for threat in &bowtie.threats {
                require_text("a threat id", &threat.id)?;
                require_text("a threat label", &threat.label)?;
            }
            for barrier in &bowtie.barriers {
                require_text("a barrier id", &barrier.id)?;
                require_text("a barrier label", &barrier.label)?;
            }
            for consequence in &bowtie.consequences {
                require_text("a consequence id", &consequence.id)?;
                require_text("a consequence label", &consequence.label)?;
            }
        }
        Ok(())
    }
}

/// The rules a wholesale edit must satisfy before it reaches the document.
///
/// **A hand-written document is held to diagnostics instead.** A negative frequency in a saved
/// study opens and shows the model's refusal beside the field; a command that would write one
/// is refused, because the editor is the place that can still say why.
///
/// # Errors
/// [`AzothError::InvalidInput`] naming the first violation.
pub fn validate_bowtie(bowtie: &Bowtie) -> Result<()> {
    require_text("a bow-tie id", &bowtie.id)?;
    require_text("a bow-tie name", &bowtie.name)?;
    require_text("a top-event label", &bowtie.top_event)?;
    unique("threat", bowtie.threats.iter().map(|row| row.id.as_str()))?;
    unique("barrier", bowtie.barriers.iter().map(|row| row.id.as_str()))?;
    unique(
        "consequence",
        bowtie.consequences.iter().map(|row| row.id.as_str()),
    )?;
    for threat in &bowtie.threats {
        require_text("a threat id", &threat.id)?;
        require_text("a threat label", &threat.label)?;
        if !threat.frequency.is_finite() || threat.frequency < 0.0 {
            return Err(AzothError::invalid_input(
                format!("threats.{}", threat.id),
                "a frequency is a finite number at least zero",
            ));
        }
    }
    for barrier in &bowtie.barriers {
        require_text("a barrier id", &barrier.id)?;
        require_text("a barrier label", &barrier.label)?;
        if !barrier.pfd.is_finite() || !(0.0..=1.0).contains(&barrier.pfd) {
            return Err(AzothError::invalid_input(
                format!("barriers.{}", barrier.id),
                "a PFD is a finite number in [0, 1]",
            ));
        }
        if !matches!(
            barrier.barrier_type.as_str(),
            "prevention" | "mitigation" | "both"
        ) {
            return Err(AzothError::invalid_input(
                format!("barriers.{}", barrier.id),
                "a barrier type is `prevention`, `mitigation` or `both`",
            ));
        }
    }
    for consequence in &bowtie.consequences {
        require_text("a consequence id", &consequence.id)?;
        require_text("a consequence label", &consequence.label)?;
        if !consequence.severity.is_finite()
            || consequence.severity < 0.0
            || consequence.severity.fract() != 0.0
        {
            return Err(AzothError::invalid_input(
                format!("consequences.{}", consequence.id),
                "a severity is a whole number at least zero",
            ));
        }
    }
    Ok(())
}

fn require_text(what: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(AzothError::invalid_input(
            "safety study",
            format!("{what} is empty"),
        ));
    }
    Ok(())
}

fn unique<'a>(what: &str, ids: impl Iterator<Item = &'a str>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(AzothError::invalid_input(
                "safety study",
                format!("the {what} id `{id}` appears twice"),
            ));
        }
    }
    Ok(())
}
