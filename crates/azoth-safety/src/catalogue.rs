//! The safety calculations a front end reads.
//!
//! **The family is the source path, not a second table.** Every entry is a model whose
//! `[source] standard` is under `NeqSim process/safety/<family>/`, and `tools/gen_safety.py`
//! takes that segment as the family. The five scenarios this tranche can run carry the same
//! `CalcForm` the calculation book reads, joined the same way; the rest are listed so a reader
//! can find their manual page.

use azoth_core::{AzothError, Result};
use azoth_sheets::form::CalcForm;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::forms_gen::CALCULATIONS;

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
