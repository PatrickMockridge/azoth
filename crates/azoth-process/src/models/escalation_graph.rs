//! `process.escalation_graph` - the domino-effect propagation.
//!
//! Spec: `specs/models/process/escalation_graph.toml`. The arithmetic is
//! `process/safety/escalation/EscalationGraphAnalyzer.java` at the pin: cumulative loads over
//! every failed source, a threshold at or above which a target fails, and the worst case.

use std::collections::HashMap;

use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;

/// One process item.
#[derive(Debug, Clone, PartialEq)]
pub struct EscalationItem {
    /// The item's identifier.
    pub item_id: String,
    /// The load at which the item fails.
    pub failure_threshold: f64,
}

/// One directed exposure.
#[derive(Debug, Clone, PartialEq)]
pub struct EscalationExposure {
    /// The failed item delivering the load.
    pub source: String,
    /// The item exposed to the load.
    pub target: String,
    /// The load delivered to the target.
    pub load: f64,
}

/// One item in a published failure set.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EscalationItemResult {
    /// The item's identifier.
    pub item_id: String,
}

/// Result of `process.escalation_graph`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EscalationGraphResult {
    /// The items that ultimately fail, in the items order.
    pub failed_items: Vec<EscalationItemResult>,
    /// The number of failed items.
    pub failure_count: f64,
    /// Whether more than the initiator failed.
    pub any_escalation: bool,
    /// The largest propagation, in the items order.
    pub worst_case_items: Vec<EscalationItemResult>,
    /// The number of items in the worst case.
    pub worst_case_count: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// Refuse a value the arithmetic cannot carry to a published comparison.
fn require_finite(name: &'static str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AzothError::invalid_input(
            name,
            "a non-finite value is not a number this calculation compares",
        ))
    }
}

/// The items registered, in their first position, with a duplicate's last threshold.
struct Registry {
    ids: Vec<String>,
    thresholds: Vec<f64>,
    index: HashMap<String, usize>,
    outgoing: Vec<Vec<(usize, f64)>>,
}

impl Registry {
    fn new(items: &[EscalationItem]) -> Result<Self> {
        let mut registry = Self {
            ids: Vec::new(),
            thresholds: Vec::new(),
            index: HashMap::new(),
            outgoing: Vec::new(),
        };
        for item in items {
            require_finite("failure_threshold", item.failure_threshold)?;
            if let Some(at) = registry.index.get(&item.item_id).copied() {
                registry.thresholds[at] = item.failure_threshold;
            } else {
                registry
                    .index
                    .insert(item.item_id.clone(), registry.ids.len());
                registry.ids.push(item.item_id.clone());
                registry.thresholds.push(item.failure_threshold);
                registry.outgoing.push(Vec::new());
            }
        }
        Ok(registry)
    }

    fn add_exposure(&mut self, exposure: &EscalationExposure) -> Result<()> {
        require_finite("load", exposure.load)?;
        let source = *registry_index(&self.index, &exposure.source)?;
        let target = *registry_index(&self.index, &exposure.target)?;
        self.outgoing[source].push((target, exposure.load));
        Ok(())
    }

    fn propagate(&self, initiator: usize) -> Vec<bool> {
        let mut failed = vec![false; self.ids.len()];
        failed[initiator] = true;
        loop {
            let mut cumulative: HashMap<usize, f64> = HashMap::new();
            for (source, is_failed) in failed.iter().enumerate() {
                if !is_failed {
                    continue;
                }
                for &(target, load) in &self.outgoing[source] {
                    *cumulative.entry(target).or_insert(0.0) += load;
                }
            }
            let mut changed = false;
            for (target, load) in cumulative {
                if !failed[target] && load >= self.thresholds[target] {
                    failed[target] = true;
                    changed = true;
                }
            }
            if !changed {
                return failed;
            }
        }
    }

    fn items_at(&self, failed: &[bool]) -> Vec<EscalationItemResult> {
        failed
            .iter()
            .enumerate()
            .filter(|(_, failed)| **failed)
            .map(|(at, _)| EscalationItemResult {
                item_id: self.ids[at].clone(),
            })
            .collect()
    }
}

/// The index an id names, or the refusal the class throws.
fn registry_index<'a>(index: &'a HashMap<String, usize>, id: &str) -> Result<&'a usize> {
    index.get(id).ok_or_else(|| {
        AzothError::invalid_input("item_id", format!("`{id}` names no registered item"))
    })
}

/// The items that fail from an initiator, and the worst case over every item.
///
/// # Errors
/// `InvalidInput` for an initiator or exposure endpoint that names no item, or a non-finite
/// threshold or load.
pub fn escalation_graph(
    items: &[EscalationItem],
    exposures: &[EscalationExposure],
    initiator: &str,
) -> Result<EscalationGraphResult> {
    let mut registry = Registry::new(items)?;
    for exposure in exposures {
        registry.add_exposure(exposure)?;
    }
    let start = *registry_index(&registry.index, initiator)?;
    let failed = registry.propagate(start);
    let failure_count = failed.iter().filter(|failed| **failed).count() as f64;

    let mut worst: Option<(usize, Vec<bool>)> = None;
    for at in 0..registry.ids.len() {
        let candidate = registry.propagate(at);
        let count = candidate.iter().filter(|failed| **failed).count();
        if worst.as_ref().is_none_or(|(best, _)| count > *best) {
            worst = Some((count, candidate));
        }
    }
    let (worst_case_count, worst_failed) = worst.unwrap_or((0, vec![false; registry.ids.len()]));

    Ok(EscalationGraphResult {
        failed_items: registry.items_at(&failed),
        failure_count,
        any_escalation: failure_count > 1.0,
        worst_case_items: registry.items_at(&worst_failed),
        worst_case_count: worst_case_count as f64,
        warnings: Vec::new(),
    })
}
