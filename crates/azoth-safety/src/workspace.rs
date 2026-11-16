//! The safety study as a live document.
//!
//! **One envelope, as the flowsheet and the calculation book have.** A command edits the study,
//! a calculate runs the scenarios and bow-ties that are ready, and every answer is the whole
//! study: what a front end draws is the answer to the call it just made, not a document it
//! reassembled. Results are derived and never saved.

use std::collections::{BTreeMap, BTreeSet};

use azoth_core::{AzothError, Result};
use azoth_sheets::form::CalcForm;
use azoth_sheets::workspace::{CalcError, CalcOutcome, kind_of, stored_value};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::catalogue;
use crate::command::Command;
use crate::dispatch;
use crate::graph;
use crate::profile;
use crate::study::{Bowtie, SafetyLayout, SafetyStudy, Scenario, validate_bowtie};

/// Which item a calculate asks for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    Scenario(String),
    Bowtie(String),
}

/// One structural fact about a scenario or a bow-tie.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SafetyDiagnostic {
    pub code: &'static str,
    /// `error` or `warning`; only an error keeps an item from running.
    pub severity: &'static str,
    pub message: String,
    pub field: Option<String>,
}

/// What a scenario or a bow-tie's last calculate left.
#[derive(Debug, Clone, Default)]
struct Slot {
    result: Option<CalcOutcome>,
    error: Option<CalcError>,
}

/// The study, its results, and which results are older than their inputs.
#[derive(Debug, Clone)]
pub struct SafetyWorkspace {
    study: SafetyStudy,
    slots: BTreeMap<String, Slot>,
    /// The distance-to-threshold rows of each fire scenario, derived by its own kernel.
    profiles: BTreeMap<String, Vec<Value>>,
    stale: BTreeSet<String>,
}

impl SafetyWorkspace {
    /// Open a study from its TOML.
    ///
    /// # Errors
    /// A document the schema does not read, or an id that appears twice where it must be
    /// addressable. An empty document is a blank study.
    pub fn open(document: &str) -> Result<Self> {
        let study = SafetyStudy::parse(document)?;
        let mut stale = BTreeSet::new();
        for scenario in &study.scenarios {
            stale.insert(scenario_key(&scenario.id));
        }
        for bowtie in &study.bowties {
            stale.insert(bowtie_key(&bowtie.id));
        }
        Ok(Self {
            study,
            slots: BTreeMap::new(),
            profiles: BTreeMap::new(),
            stale,
        })
    }

    /// The study as TOML.
    ///
    /// # Errors
    /// A value the TOML writer cannot express.
    pub fn document(&self) -> Result<String> {
        self.study.to_document()
    }

    /// Whether any result is older than its inputs.
    #[must_use]
    pub fn dirty(&self) -> bool {
        !self.stale.is_empty()
    }

    /// Apply one edit.
    ///
    /// # Errors
    /// A command that cannot take effect: a duplicate id, an absent scenario or bow-tie, an
    /// input the calculation does not declare, a value of the wrong kind, or a bow-tie whose
    /// numbers the model would refuse.
    pub fn apply(&mut self, command: &Command) -> Result<()> {
        match command {
            Command::SetStudyName { name } => {
                let name = name.trim();
                if name.is_empty() {
                    return Err(AzothError::invalid_input("name", "a study needs a name"));
                }
                self.study.name = name.to_string();
            }
            Command::AddScenario {
                id,
                calculation,
                name,
            } => {
                editable_id("scenario", id)?;
                if self.study.scenario(id).is_some() {
                    return Err(AzothError::invalid_input(
                        "id",
                        format!("the study already has a scenario `{id}`"),
                    ));
                }
                let entry = catalogue::calculation_of(calculation).ok_or_else(|| {
                    AzothError::invalid_input(
                        "calculation",
                        format!("`{calculation}` is not a calculation in this build"),
                    )
                })?;
                if entry.form.is_none() {
                    return Err(AzothError::invalid_input(
                        "calculation",
                        entry
                            .refusal
                            .unwrap_or("this calculation is not a scenario calculation"),
                    ));
                }
                let name = name
                    .as_deref()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .unwrap_or(entry.name)
                    .to_string();
                self.study.scenarios.push(Scenario {
                    id: id.clone(),
                    name,
                    calculation: calculation.clone(),
                    inputs: BTreeMap::new(),
                });
                self.stale.insert(scenario_key(id));
            }
            Command::RemoveScenario { id } => {
                self.study.scenarios.retain(|scenario| scenario.id != *id);
                self.slots.remove(&scenario_key(id));
                self.profiles.remove(&scenario_key(id));
                self.stale.remove(&scenario_key(id));
            }
            Command::RenameScenario { id, name } => {
                let name = name.trim();
                if name.is_empty() {
                    return Err(AzothError::invalid_input("name", "a scenario needs a name"));
                }
                let scenario = self
                    .study
                    .scenarios
                    .iter_mut()
                    .find(|candidate| candidate.id == *id)
                    .ok_or_else(|| {
                        AzothError::invalid_input("scenario", format!("no scenario `{id}`"))
                    })?;
                scenario.name = name.to_string();
            }
            Command::SetScenarioInput {
                scenario,
                input,
                value,
            } => {
                let declaration = self.input_of(scenario, input)?;
                let stored = stored_value(declaration, input, value)?;
                let row = self
                    .study
                    .scenarios
                    .iter_mut()
                    .find(|candidate| candidate.id == *scenario)
                    .ok_or_else(|| {
                        AzothError::invalid_input("scenario", format!("no scenario `{scenario}`"))
                    })?;
                row.inputs.insert(input.clone(), stored);
                self.stale.insert(scenario_key(scenario));
            }
            Command::UnsetScenarioInput { scenario, input } => {
                self.input_of(scenario, input)?;
                let row = self
                    .study
                    .scenarios
                    .iter_mut()
                    .find(|candidate| candidate.id == *scenario)
                    .ok_or_else(|| {
                        AzothError::invalid_input("scenario", format!("no scenario `{scenario}`"))
                    })?;
                row.inputs.remove(input);
                self.stale.insert(scenario_key(scenario));
            }
            Command::SetBowtie { bowtie } => {
                validate_bowtie(bowtie)?;
                if let Some(existing) = self
                    .study
                    .bowties
                    .iter_mut()
                    .find(|candidate| candidate.id == bowtie.id)
                {
                    *existing = bowtie.clone();
                } else {
                    self.study.bowties.push(bowtie.clone());
                }
                self.prune_layout(bowtie);
                self.stale.insert(bowtie_key(&bowtie.id));
            }
            Command::RemoveBowtie { id } => {
                self.study.bowties.retain(|bowtie| bowtie.id != *id);
                if let Some(layout) = self.study.layout.as_mut() {
                    layout.bowties.remove(id);
                }
                self.slots.remove(&bowtie_key(id));
                self.stale.remove(&bowtie_key(id));
            }
            Command::SetBowtiePosition { bowtie, node, x, y } => {
                if !x.is_finite() || !y.is_finite() {
                    return Err(AzothError::invalid_input(
                        "position",
                        "a diagram position is a finite pair",
                    ));
                }
                let Some(row) = self.study.bowtie(bowtie) else {
                    return Err(AzothError::invalid_input(
                        "bowtie",
                        format!("no bow-tie `{bowtie}`"),
                    ));
                };
                if !node_in(row, node) {
                    return Err(AzothError::invalid_input(
                        "node",
                        format!("`{node}` is not a node of `{bowtie}`"),
                    ));
                }
                let layout = self
                    .study
                    .layout
                    .get_or_insert_with(SafetyLayout::default)
                    .bowties
                    .entry(bowtie.clone())
                    .or_default();
                if node == "top_event" {
                    layout.top_event = Some([*x, *y]);
                } else if let Some(id) = node.strip_prefix("threat:") {
                    layout.threats.insert(id.to_string(), [*x, *y]);
                } else if let Some(id) = node.strip_prefix("barrier:") {
                    layout.barriers.insert(id.to_string(), [*x, *y]);
                } else if let Some(id) = node.strip_prefix("consequence:") {
                    layout.consequences.insert(id.to_string(), [*x, *y]);
                }
            }
        }
        Ok(())
    }

    /// Calculate one scenario or bow-tie, or every stale item when `target` is `None`.
    ///
    /// An item with an **error** diagnostic is skipped and stays stale, as an incomplete sheet
    /// does; a warning does not block it.
    ///
    /// # Errors
    /// A scenario or bow-tie id the study does not carry.
    pub fn calculate(&mut self, target: Option<&Target>) -> Result<()> {
        let keys: Vec<String> = match target {
            Some(Target::Scenario(id)) => {
                if self.study.scenario(id).is_none() {
                    return Err(AzothError::invalid_input(
                        "scenario",
                        format!("no scenario `{id}`"),
                    ));
                }
                vec![scenario_key(id)]
            }
            Some(Target::Bowtie(id)) => {
                if self.study.bowtie(id).is_none() {
                    return Err(AzothError::invalid_input(
                        "bowtie",
                        format!("no bow-tie `{id}`"),
                    ));
                }
                vec![bowtie_key(id)]
            }
            None => self.stale.iter().cloned().collect(),
        };

        for key in keys {
            if let Some(id) = key.strip_prefix("scenario:") {
                let Some(scenario) = self.study.scenario(id).cloned() else {
                    continue;
                };
                if has_error(&self.scenario_diagnostics(&scenario)) {
                    continue;
                }
                let inputs = serde_json::to_value(&scenario.inputs).map_err(|error| {
                    AzothError::invalid_input(
                        "inputs",
                        format!("they could not be read for the calculation: {error}"),
                    )
                })?;
                let slot = match dispatch::run(&scenario.calculation, &inputs) {
                    dispatch::Outcome::Ok(outcome) => {
                        self.profiles.insert(
                            key.clone(),
                            profile::profile(&scenario.calculation, &inputs),
                        );
                        Slot {
                            result: Some(outcome),
                            error: None,
                        }
                    }
                    dispatch::Outcome::Error { message, field } => Slot {
                        result: None,
                        error: Some(CalcError { message, field }),
                    },
                };
                self.slots.insert(key.clone(), slot);
                self.stale.remove(&key);
            } else if let Some(id) = key.strip_prefix("bowtie:") {
                let Some(bowtie) = self.study.bowtie(id).cloned() else {
                    continue;
                };
                if has_error(&self.bowtie_diagnostics(&bowtie)) {
                    continue;
                }
                let slot = match dispatch::run("process.bow_tie", &bowtie_inputs(&bowtie)) {
                    dispatch::Outcome::Ok(outcome) => Slot {
                        result: Some(outcome),
                        error: None,
                    },
                    dispatch::Outcome::Error { message, field } => Slot {
                        result: None,
                        error: Some(CalcError { message, field }),
                    },
                };
                self.slots.insert(key.clone(), slot);
                self.stale.remove(&key);
            }
        }
        Ok(())
    }

    /// The answer a front end draws: the whole study, with per-item state.
    ///
    /// # Errors
    /// A study whose document cannot be written.
    pub fn to_json(&self) -> Result<String> {
        let document = self.document()?;
        let mut ok = true;
        let mut scenarios = Vec::with_capacity(self.study.scenarios.len());
        for scenario in &self.study.scenarios {
            let key = scenario_key(&scenario.id);
            let diagnostics = self.scenario_diagnostics(scenario);
            let ready = !has_error(&diagnostics);
            ok &= ready;
            let slot = self.slots.get(&key);
            let entry = catalogue::calculation_of(&scenario.calculation);
            scenarios.push(json!({
                "id": scenario.id,
                "name": scenario.name,
                "calculation": scenario.calculation,
                "family": entry.map(|entry| entry.family),
                "purpose": entry.map(|entry| entry.purpose),
                "inputs": serde_json::to_value(&scenario.inputs).unwrap_or_else(|_| json!({})),
                "ready": ready,
                "missing": self.missing(scenario),
                "stale": self.stale.contains(&key),
                "result": slot.and_then(|slot| slot.result.as_ref()),
                "error": slot.and_then(|slot| slot.error.as_ref()),
                "diagnostics": diagnostics,
                "profile": self.profiles.get(&key).cloned().unwrap_or_default(),
            }));
        }

        let mut bowties = Vec::with_capacity(self.study.bowties.len());
        for bowtie in &self.study.bowties {
            let key = bowtie_key(&bowtie.id);
            let diagnostics = self.bowtie_diagnostics(bowtie);
            let ready = !has_error(&diagnostics);
            ok &= ready;
            let slot = self.slots.get(&key);
            let result = slot.and_then(|slot| slot.result.as_ref());
            let derived = result.map(|result| {
                json!({
                    "unmitigated_frequency_per_year": result.outputs.get("unmitigated_frequency_per_year"),
                    "mitigated_frequency_per_year": result.outputs.get("mitigated_frequency_per_year"),
                    "total_rrf": result.outputs.get("total_rrf"),
                    "max_severity": result.outputs.get("max_severity"),
                })
            });
            let barriers: Vec<Value> = bowtie
                .barriers
                .iter()
                .map(|barrier| {
                    let side = if barrier.barrier_type == "mitigation" {
                        "mitigation"
                    } else {
                        "prevention"
                    };
                    json!({
                        "id": barrier.id,
                        "label": barrier.label,
                        "pfd": barrier.pfd,
                        "barrier_type": barrier.barrier_type,
                        "functional": barrier.functional,
                        "side": side,
                    })
                })
                .collect();
            let consequences: Vec<Value> = bowtie
                .consequences
                .iter()
                .map(|consequence| {
                    let scenario = consequence
                        .scenario
                        .as_deref()
                        .and_then(|id| self.study.scenario(id));
                    json!({
                        "id": consequence.id,
                        "label": consequence.label,
                        "severity": consequence.severity,
                        "scenario": consequence.scenario,
                        "scenario_name": scenario.map(|row| row.name.as_str()),
                        "scenario_ready": scenario.map(|row| !has_error(&self.scenario_diagnostics(row))),
                    })
                })
                .collect();
            let prevention = bowtie
                .barriers
                .iter()
                .filter(|barrier| barrier.barrier_type != "mitigation")
                .count();
            let mitigation = bowtie.barriers.len() - prevention;
            let layout = self
                .study
                .layout
                .as_ref()
                .and_then(|layout| layout.bowties.get(&bowtie.id));
            bowties.push(json!({
                "id": bowtie.id,
                "name": bowtie.name,
                "top_event": bowtie.top_event,
                "threats": bowtie.threats,
                "barriers": barriers,
                "consequences": consequences,
                "counts": {
                    "threats": bowtie.threats.len(),
                    "prevention_barriers": prevention,
                    "mitigation_barriers": mitigation,
                    "consequences": bowtie.consequences.len(),
                },
                "derived": derived,
                "graph": graph::graph(bowtie, layout),
                "ready": ready,
                "stale": self.stale.contains(&key),
                "result": result,
                "error": slot.and_then(|slot| slot.error.as_ref()),
                "diagnostics": diagnostics,
            }));
        }

        let envelope = json!({
            "ok": ok,
            "dirty": self.dirty(),
            "study": {
                "id": self.study.id,
                "name": self.study.name,
                "document": document,
                "scenarios": scenarios,
                "bowties": bowties,
                "layout": self.study.layout,
            },
        });
        serde_json::to_string(&envelope).map_err(|error| {
            AzothError::invalid_input(
                "safety study",
                format!("its envelope could not be written: {error}"),
            )
        })
    }

    /// Why a scenario cannot run yet, if it cannot.
    fn scenario_diagnostics(&self, scenario: &Scenario) -> Vec<SafetyDiagnostic> {
        let Some(entry) = catalogue::calculation_of(&scenario.calculation) else {
            return vec![error(
                "unknown_calculation",
                format!(
                    "`{}` is not a calculation in this build",
                    scenario.calculation
                ),
                None,
            )];
        };
        let Some(form) = entry.form else {
            return vec![error(
                "unsupported_calculation",
                entry
                    .refusal
                    .unwrap_or("this calculation is not a scenario calculation")
                    .to_string(),
                None,
            )];
        };
        let mut out = Vec::new();
        for name in scenario.inputs.keys() {
            if !form.inputs.iter().any(|input| input.name == name) {
                out.push(error(
                    "unknown_input",
                    format!("`{name}` is not an input of {}", form.name),
                    Some(name.clone()),
                ));
            }
        }
        for input in form.inputs {
            let Some(value) = scenario.inputs.get(input.name) else {
                if input.required {
                    out.push(error(
                        "missing_input",
                        format!("`{}` is required", input.name),
                        Some(input.name.to_string()),
                    ));
                }
                continue;
            };
            if let Err(reason) = kind_of(input, value) {
                out.push(error(
                    "input_kind",
                    format!("`{}` {reason}", input.name),
                    Some(input.name.to_string()),
                ));
            }
        }
        out
    }

    /// The required inputs a scenario has not set.
    fn missing(&self, scenario: &Scenario) -> Vec<&'static str> {
        let Some(form) = form_of(&scenario.calculation) else {
            return Vec::new();
        };
        form.inputs
            .iter()
            .filter(|input| input.required && !scenario.inputs.contains_key(input.name))
            .map(|input| input.name)
            .collect()
    }

    /// Why a bow-tie cannot run, and what its consequence references say.
    fn bowtie_diagnostics(&self, bowtie: &Bowtie) -> Vec<SafetyDiagnostic> {
        let mut out = Vec::new();
        for threat in &bowtie.threats {
            if !threat.frequency.is_finite() || threat.frequency < 0.0 {
                out.push(error(
                    "invalid_frequency",
                    format!(
                        "`{}` has a frequency that is not a finite number at least zero",
                        threat.id
                    ),
                    Some(format!("threats.{}", threat.id)),
                ));
            }
        }
        for barrier in &bowtie.barriers {
            if !barrier.pfd.is_finite() || !(0.0..=1.0).contains(&barrier.pfd) {
                out.push(error(
                    "invalid_pfd",
                    format!("`{}` has a PFD outside [0, 1]", barrier.id),
                    Some(format!("barriers.{}", barrier.id)),
                ));
            }
            if !matches!(
                barrier.barrier_type.as_str(),
                "prevention" | "mitigation" | "both"
            ) {
                out.push(error(
                    "unknown_barrier_type",
                    format!(
                        "`{}` names a barrier type the model does not carry",
                        barrier.id
                    ),
                    Some(format!("barriers.{}", barrier.id)),
                ));
            }
        }
        for consequence in &bowtie.consequences {
            if !consequence.severity.is_finite()
                || consequence.severity < 0.0
                || consequence.severity.fract() != 0.0
            {
                out.push(error(
                    "invalid_severity",
                    format!(
                        "`{}` has a severity that is not a whole number at least zero",
                        consequence.id
                    ),
                    Some(format!("consequences.{}", consequence.id)),
                ));
            }
            if let Some(target) = &consequence.scenario {
                if self.study.scenario(target).is_none() {
                    out.push(SafetyDiagnostic {
                        code: "unknown_scenario",
                        severity: "warning",
                        message: format!(
                            "`{}` names scenario `{target}`, which the study does not carry",
                            consequence.id
                        ),
                        field: Some(format!("consequences.{}.scenario", consequence.id)),
                    });
                }
            }
        }
        out
    }

    /// The declaration of one input of one scenario.
    fn input_of(
        &self,
        scenario: &str,
        input: &str,
    ) -> Result<&'static azoth_sheets::form::CalcInput> {
        let Some(row) = self.study.scenario(scenario) else {
            return Err(AzothError::invalid_input(
                "scenario",
                format!("no scenario `{scenario}`"),
            ));
        };
        let Some(form) = form_of(&row.calculation) else {
            return Err(AzothError::invalid_input(
                "calculation",
                format!("`{}` is not a scenario calculation", row.calculation),
            ));
        };
        form.inputs
            .iter()
            .find(|candidate| candidate.name == input)
            .ok_or_else(|| {
                AzothError::invalid_input(
                    "input",
                    format!("`{input}` is not an input of {}", form.name),
                )
            })
    }

    /// Drop layout entries for nodes the edited bow-tie no longer carries.
    fn prune_layout(&mut self, bowtie: &Bowtie) {
        let Some(layout) = self
            .study
            .layout
            .as_mut()
            .and_then(|layout| layout.bowties.get_mut(&bowtie.id))
        else {
            return;
        };
        layout
            .threats
            .retain(|id, _| bowtie.threats.iter().any(|threat| &threat.id == id));
        layout
            .barriers
            .retain(|id, _| bowtie.barriers.iter().any(|barrier| &barrier.id == id));
        layout.consequences.retain(|id, _| {
            bowtie
                .consequences
                .iter()
                .any(|consequence| &consequence.id == id)
        });
    }
}

fn scenario_key(id: &str) -> String {
    format!("scenario:{id}")
}

fn bowtie_key(id: &str) -> String {
    format!("bowtie:{id}")
}

fn form_of(calculation: &str) -> Option<&'static CalcForm> {
    catalogue::calculation_of(calculation).and_then(|entry| entry.form)
}

fn has_error(diagnostics: &[SafetyDiagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error")
}

fn error(code: &'static str, message: String, field: Option<String>) -> SafetyDiagnostic {
    SafetyDiagnostic {
        code,
        severity: "error",
        message,
        field,
    }
}

/// An id the schema and the document can both address.
fn editable_id(what: &str, id: &str) -> Result<()> {
    let mut characters = id.chars();
    let Some(first) = characters.next() else {
        return Err(AzothError::invalid_input(
            what,
            "an id is a lower-case name, at least one character",
        ));
    };
    let rest = characters.all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    });
    if !first.is_ascii_lowercase() || !rest {
        return Err(AzothError::invalid_input(
            what,
            "an id starts with a lower-case letter and carries only lower-case letters, digits and underscores",
        ));
    }
    Ok(())
}

/// Whether a node id names one of the bow-tie's nodes.
fn node_in(bowtie: &Bowtie, node: &str) -> bool {
    if node == "top_event" {
        return true;
    }
    if let Some(id) = node.strip_prefix("threat:") {
        return bowtie.threats.iter().any(|threat| threat.id == id);
    }
    if let Some(id) = node.strip_prefix("barrier:") {
        return bowtie.barriers.iter().any(|barrier| barrier.id == id);
    }
    if let Some(id) = node.strip_prefix("consequence:") {
        return bowtie
            .consequences
            .iter()
            .any(|consequence| consequence.id == id);
    }
    false
}

/// The numeric records `process.bow_tie` reads. **Labels never cross this boundary.**
fn bowtie_inputs(bowtie: &Bowtie) -> Value {
    json!({
        "threats": bowtie
            .threats
            .iter()
            .map(|threat| json!({"frequency": threat.frequency, "active": threat.active}))
            .collect::<Vec<Value>>(),
        "consequences": bowtie
            .consequences
            .iter()
            .map(|consequence| json!({"severity": consequence.severity}))
            .collect::<Vec<Value>>(),
        "barriers": bowtie
            .barriers
            .iter()
            .map(|barrier| json!({
                "pfd": barrier.pfd,
                "barrier_type": barrier.barrier_type,
                "functional": barrier.functional,
            }))
            .collect::<Vec<Value>>(),
    })
}
