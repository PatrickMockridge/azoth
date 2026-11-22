//! The safety calculations a front end reads.
//!
//! **The family is the source path, not a second table.** Every entry is a model whose
//! `[source] standard` is under `NeqSim process/safety/<family>/`, and `tools/gen_safety.py`
//! takes that segment as the family. The scenario models this workspace can run carry the same
//! `CalcForm` the calculation book reads, joined the same way; the rest are listed so a reader
//! can find their manual page.

use azoth_core::{AzothError, Result};
use azoth_sheets::form::CalcForm;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::forms_gen::{CALCULATIONS, FAMILIES};

/// The copy a scenario deck needs: its tab, picker title, starter heading, search label and
/// the first step it suggests.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SafetyDeck {
    pub tab: &'static str,
    pub title: &'static str,
    pub heading: &'static str,
    pub search: &'static str,
    pub step: &'static str,
}

/// One safety family: its display copy and, where it has one, its deck.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SafetyFamily {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    /// `None` for a family the catalogue lists without a scenario deck.
    pub deck: Option<SafetyDeck>,
}

/// One safety calculation.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SafetyCalculation {
    pub id: &'static str,
    pub name: &'static str,
    /// The `process/safety/<family>/` segment of the spec's source path.
    pub family: &'static str,
    /// `"bowtie"` or `"fire"` where the first tranche gives it a native view.
    pub sub_interface: Option<&'static str>,
    /// What the calculation is for. Authored in the generator because a model spec has no
    /// summary field; empty for an entry the overview only lists.
    pub purpose: &'static str,
    /// Whether a scenario can run it in this workspace.
    pub native: bool,
    /// Why a native-looking entry cannot run, where it has its own sentence.
    pub refusal: Option<&'static str>,
    /// The form a scenario draws. Absent for the bow-tie and for anything not run natively.
    #[serde(skip)]
    pub form: Option<&'static CalcForm>,
}

/// Every safety calculation, in id order.
#[must_use]
pub fn calculations() -> &'static [SafetyCalculation] {
    CALCULATIONS
}

/// One entry by id.
#[must_use]
pub fn calculation_of(id: &str) -> Option<&'static SafetyCalculation> {
    CALCULATIONS.iter().find(|entry| entry.id == id)
}

/// Every safety family, in registry order.
#[must_use]
pub fn families() -> &'static [SafetyFamily] {
    FAMILIES
}

/// One family by id.
#[must_use]
pub fn family_of(id: &str) -> Option<&'static SafetyFamily> {
    FAMILIES.iter().find(|entry| entry.id == id)
}

/// The family registry as a front end reads it.
///
/// # Errors
/// A projection that cannot be written, which for this shape it cannot.
pub fn families_to_value() -> Result<Value> {
    serde_json::to_value(FAMILIES).map_err(|error| {
        AzothError::invalid_input(
            "safety-families",
            format!("the family registry could not be written: {error}"),
        )
    })
}

/// The catalogue as a front end reads it.
///
/// # Errors
/// A projection that cannot be written, which for this shape it cannot.
pub fn to_value() -> Result<Value> {
    let mut out = Vec::with_capacity(CALCULATIONS.len());
    for entry in CALCULATIONS {
        let mut object = serde_json::to_value(entry)
            .map_err(|error| {
                AzothError::invalid_input(
                    entry.id,
                    format!("the safety calculation could not be written: {error}"),
                )
            })?
            .as_object()
            .cloned()
            .unwrap_or_else(Map::new);
        if let Some(form) = entry.form {
            object.insert(
                "form".to_string(),
                azoth_sheets::catalogue::form_value(form)?,
            );
        }
        out.push(Value::Object(object));
    }
    Ok(Value::Array(out))
}
