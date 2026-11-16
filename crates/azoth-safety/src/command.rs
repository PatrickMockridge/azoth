//! One edit to a safety study.
//!
//! **A bow-tie is one record, not a command per row.** The inspector edits a field and sends the
//! whole record; add, remove and reorder are the same call. Scenarios stay granular because a
//! scenario is a form with an input map, and an input edit should not rewrite the document's
//! other scenarios to say so.

use serde::{Deserialize, Serialize};

use crate::study::Bowtie;

/// One edit to a safety study.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    SetStudyName {
        name: String,
    },
    AddScenario {
        id: String,
        calculation: String,
        #[serde(default)]
        name: Option<String>,
    },
    RemoveScenario {
        id: String,
    },
    RenameScenario {
        id: String,
        name: String,
    },
    SetScenarioInput {
        scenario: String,
        input: String,
        value: serde_json::Value,
    },
    UnsetScenarioInput {
        scenario: String,
        input: String,
    },
    /// Create or replace one bow-tie wholesale.
    SetBowtie {
        bowtie: Bowtie,
    },
    RemoveBowtie {
        id: String,
    },
    /// Move one diagram node. `node` is `top_event`, `threat:<id>`, `barrier:<id>` or
    /// `consequence:<id>`; a layout edit does not make a result stale.
    SetBowtiePosition {
        bowtie: String,
        node: String,
        x: f64,
        y: f64,
    },
}
