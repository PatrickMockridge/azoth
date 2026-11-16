//! `azoth safety` — one edit and a calculate, printed as the study envelope.
//!
//! **A command line, not a second editor.** The document, the commands and the envelope are the
//! safety workspace's; this reads a file, applies what it was given and prints the answer. It is
//! what the fixture generator calls, and what a headless study uses.

use std::path::Path;

use azoth_core::{AzothError, Result};
use azoth_safety::command::Command;
use azoth_safety::workspace::{SafetyWorkspace, Target};

/// Open a study, apply one command if given, calculate, and return the envelope as JSON.
///
/// # Errors
/// A study that does not read, a command that cannot take effect, a calculate target the study
/// does not carry, or an envelope that cannot be written.
pub fn run(study: &Path, command: Option<&str>, calculate: Option<&str>) -> Result<String> {
    let document = std::fs::read_to_string(study).map_err(|error| {
        AzothError::invalid_input("study", format!("{}: {error}", study.display()))
    })?;
    let mut workspace = SafetyWorkspace::open(&document)?;
    if let Some(command) = command {
        let command: Command = serde_json::from_str(command)
            .map_err(|error| AzothError::invalid_input("command", error.to_string()))?;
        workspace.apply(&command)?;
    }
    if let Some(calculate) = calculate {
        let target = target_of(calculate)?;
        workspace.calculate(target.as_ref())?;
    }
    workspace.to_json()
}

/// The calculate target a `--calculate` value names.
///
/// # Errors
/// A value that is not `all`, `scenario:<id>` or `bowtie:<id>`.
pub fn target_of(text: &str) -> Result<Option<Target>> {
    if text == "all" {
        return Ok(None);
    }
    if let Some(id) = text.strip_prefix("scenario:") {
        return Ok(Some(Target::Scenario(id.to_string())));
    }
    if let Some(id) = text.strip_prefix("bowtie:") {
        return Ok(Some(Target::Bowtie(id.to_string())));
    }
    Err(AzothError::invalid_input(
        "calculate",
        "`calculate` takes `all`, `scenario:<id>` or `bowtie:<id>`",
    ))
}
