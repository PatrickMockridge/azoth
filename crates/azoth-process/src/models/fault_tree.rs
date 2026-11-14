//! `process.fault_tree` - the fault-tree probability and cut-set enumeration.
//!
//! Spec: `specs/models/process/fault_tree.toml`. The arithmetic is
//! `process/safety/risk/fta/FaultTreeAnalyzer.java` at the pin: the recursive gate
//! probability, the beta correction on OR, the voting masks and the minimal cut sets.

use std::collections::HashSet;

use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;

/// One node of the flat fault tree, as the caller states it.
#[derive(Debug, Clone, PartialEq)]
pub struct FaultTreeNodeSpec {
    /// The node label; a basic event's name is its cut-set identity.
    pub name: String,
    /// The zero-based row index of the parent, or `-1` for the root.
    pub parent_index: f64,
    /// `basic`, `and`, `or` or `voting`.
    pub gate: String,
    /// A basic event's failure probability.
    pub probability: Option<f64>,
    /// A voting gate's k.
    pub k_of_n: Option<f64>,
    /// An or gate's beta factor.
    pub beta_ccf: Option<f64>,
}

/// One minimal cut set.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MinimalCutSet {
    /// The sorted basic-event names joined by `", "`.
    pub members: String,
    /// The number of members.
    pub cardinality: f64,
}

/// Result of `process.fault_tree`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FaultTreeResult {
    /// The recursive gate probability.
    pub top_event_probability: f64,
    /// The number of minimal cut sets.
    pub cut_set_count: f64,
    /// The minimal cut sets, ordered by cardinality and then member name.
    pub minimal_cut_sets: Vec<MinimalCutSet>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gate {
    Basic,
    And,
    Or,
    Voting,
}

struct Node {
    name: String,
    gate: Gate,
    probability: f64,
    k: usize,
    beta: f64,
    children: Vec<usize>,
}

fn require_whole(name: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err(AzothError::invalid_input(
            name.to_string(),
            format!("a whole number is required, and {value} is not"),
        ));
    }
    Ok(())
}

fn parse_gate(index: usize, gate: &str) -> Result<Gate> {
    match gate {
        "basic" => Ok(Gate::Basic),
        "and" => Ok(Gate::And),
        "or" => Ok(Gate::Or),
        "voting" => Ok(Gate::Voting),
        _ => Err(AzothError::invalid_input(
            format!("nodes[{index}].gate"),
            format!("a gate is basic, and, or or voting, and {gate:?} is not"),
        )),
    }
}

fn parse_nodes(spec_nodes: &[FaultTreeNodeSpec], cap: usize) -> Result<(Vec<Node>, usize)> {
    let count = spec_nodes.len();
    if count == 0 {
        return Err(AzothError::invalid_input(
            "nodes",
            "a fault tree has a root, and the node list is empty",
        ));
    }

    let mut nodes: Vec<Node> = Vec::with_capacity(count);
    let mut parents: Vec<Option<usize>> = Vec::with_capacity(count);
    let mut roots = 0usize;
    let mut basics = 0usize;
    for (index, row) in spec_nodes.iter().enumerate() {
        let label = row.name.clone();
        let name = format!("nodes[{index}]");
        if row.name.is_empty() {
            return Err(AzothError::invalid_input(
                format!("{name}.name"),
                "a node label is not empty",
            ));
        }
        require_whole(&format!("{name}.parent_index"), row.parent_index)?;
        let parent = if row.parent_index == -1.0 {
            roots += 1;
            None
        } else if row.parent_index >= 0.0 && (row.parent_index as usize) < count {
            Some(row.parent_index as usize)
        } else {
            return Err(AzothError::invalid_input(
                format!("{name}.parent_index"),
                format!(
                    "a parent index is -1 or a row index below {count}, and {} is not",
                    row.parent_index
                ),
            ));
        };
        let gate = parse_gate(index, &row.gate)?;

        let probability = match (gate, row.probability) {
            (Gate::Basic, Some(value)) => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(AzothError::invalid_input(
                        format!("{name}.probability"),
                        format!("a basic probability is in [0, 1], and {value} is not"),
                    ));
                }
                basics += 1;
                value
            }
            (Gate::Basic, None) => {
                return Err(AzothError::invalid_input(
                    format!("{name}.probability"),
                    "a basic event carries its probability",
                ));
            }
            (_, Some(_)) => {
                return Err(AzothError::invalid_input(
                    format!("{name}.probability"),
                    "only a basic event carries a probability",
                ));
            }
            (_, None) => 0.0,
        };

        let k = match (gate, row.k_of_n) {
            (Gate::Voting, Some(value)) => {
                require_whole(&format!("{name}.k_of_n"), value)?;
                if value < 1.0 {
                    return Err(AzothError::invalid_input(
                        format!("{name}.k_of_n"),
                        format!("a voting gate's k is at least one, and {value} is not"),
                    ));
                }
                value as usize
            }
            (Gate::Voting, None) => {
                return Err(AzothError::invalid_input(
                    format!("{name}.k_of_n"),
                    "a voting gate carries its k",
                ));
            }
            (_, Some(_)) => {
                return Err(AzothError::invalid_input(
                    format!("{name}.k_of_n"),
                    "only a voting gate carries a k",
                ));
            }
            (_, None) => 0,
        };

        let beta = match (gate, row.beta_ccf) {
            (Gate::Or, Some(value)) => {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(AzothError::invalid_input(
                        format!("{name}.beta_ccf"),
                        format!("a beta factor is in [0, 1], and {value} is not"),
                    ));
                }
                value
            }
            (Gate::Or, None) => 0.0,
            (_, Some(_)) => {
                return Err(AzothError::invalid_input(
                    format!("{name}.beta_ccf"),
                    "the beta factor is read on an or gate only, which is the class's own code",
                ));
            }
            (_, None) => 0.0,
        };

        nodes.push(Node {
            name: label,
            gate,
            probability,
            k,
            beta,
            children: Vec::new(),
        });
        parents.push(parent);
    }

    if roots != 1 {
        return Err(AzothError::invalid_input(
            "nodes",
            format!("a fault tree has one root, and the node list has {roots}"),
        ));
    }

    for (index, parent) in parents.iter().enumerate() {
        if let Some(parent) = parent {
            if parent == &index {
                return Err(AzothError::invalid_input(
                    format!("nodes[{index}].parent_index"),
                    "a node is not its own parent",
                ));
            }
            nodes[*parent].children.push(index);
        }
    }

    let root = parents
        .iter()
        .position(Option::is_none)
        .expect("one root was counted");
    let mut reached = vec![false; count];
    let mut stack = vec![root];
    while let Some(index) = stack.pop() {
        if reached[index] {
            return Err(AzothError::invalid_input(
                "nodes",
                "a node is reached twice, so the parent indices carry a cycle",
            ));
        }
        reached[index] = true;
        stack.extend(nodes[index].children.iter().copied());
    }
    if reached.iter().any(|seen| !seen) {
        return Err(AzothError::invalid_input(
            "nodes",
            "a node is unreachable from the root, so its parent chain does not end there",
        ));
    }

    for (index, node) in nodes.iter().enumerate() {
        if node.gate == Gate::Basic && !node.children.is_empty() {
            return Err(AzothError::invalid_input(
                format!("nodes[{index}]"),
                "a basic event is a leaf and carries no children",
            ));
        }
        if node.gate != Gate::Basic && node.children.is_empty() {
            return Err(AzothError::invalid_input(
                format!("nodes[{index}]"),
                "a gate carries at least one child; the class answers an empty gate but no tree states one",
            ));
        }
    }

    let mut names: HashSet<&str> = HashSet::new();
    for (index, node) in nodes.iter().enumerate() {
        if node.gate == Gate::Basic && !names.insert(node.name.as_str()) {
            return Err(AzothError::invalid_input(
                format!("nodes[{index}].name"),
                format!(
                    "a basic-event name is unique, and {:?} repeats",
                    node.name
                ),
            ));
        }
    }

    if basics > cap {
        return Err(AzothError::invalid_input(
            "nodes",
            format!(
                "the cut-set enumeration is 2^n, and {basics} basic events exceeds the {cap} \
                 the procedure carries"
            ),
        ));
    }
    for (index, node) in nodes.iter().enumerate() {
        if node.gate == Gate::Voting {
            let n = node.children.len();
            if n > cap {
                return Err(AzothError::invalid_input(
                    format!("nodes[{index}].k_of_n"),
                    format!(
                        "the voting masks are 2^n, and {n} children exceed the {cap} the \
                         procedure carries"
                    ),
                ));
            }
            if node.k > n {
                return Err(AzothError::invalid_input(
                    format!("nodes[{index}].k_of_n"),
                    format!("a voting gate's k is in [1, {n}], and {} is not", node.k),
                ));
            }
        }
    }

    Ok((nodes, root))
}

fn voting_probability(probabilities: &[f64], k: usize) -> f64 {
    let n = probabilities.len();
    let mut total = 0.0;
    for mask in 0..(1usize << n) {
        if (mask.count_ones() as usize) < k {
            continue;
        }
        let mut probability = 1.0;
        for (bit, p) in probabilities.iter().enumerate() {
            probability *= if (mask >> bit) & 1 == 1 {
                *p
            } else {
                1.0 - *p
            };
        }
        total += probability;
    }
    total
}

fn top_event_probability(index: usize, nodes: &[Node]) -> f64 {
    let node = &nodes[index];
    if node.gate == Gate::Basic {
        return node.probability;
    }
    let child_probs: Vec<f64> = node
        .children
        .iter()
        .map(|child| top_event_probability(*child, nodes))
        .collect();
    match node.gate {
        Gate::Basic => unreachable!("handled above"),
        Gate::And => {
            let mut p = 1.0;
            for child in &child_probs {
                p *= *child;
            }
            p
        }
        Gate::Or => {
            let mut complement = 1.0;
            for child in &child_probs {
                complement *= 1.0 - *child;
            }
            let independent = 1.0 - complement;
            if node.beta > 0.0 {
                let mut maximum = 0.0f64;
                for child in &child_probs {
                    maximum = maximum.max(*child);
                }
                (1.0 - node.beta) * independent + node.beta * maximum
            } else {
                independent
            }
        }
        Gate::Voting => voting_probability(&child_probs, node.k),
    }
}

fn evaluates_true(index: usize, nodes: &[Node], failed: &HashSet<&str>) -> bool {
    let node = &nodes[index];
    if node.gate == Gate::Basic {
        return failed.contains(node.name.as_str());
    }
    let mut true_count = 0;
    for child in &node.children {
        if evaluates_true(*child, nodes, failed) {
            true_count += 1;
        }
    }
    match node.gate {
        Gate::And => true_count == node.children.len(),
        Gate::Or => true_count >= 1,
        Gate::Voting => true_count >= node.k,
        Gate::Basic => unreachable!("handled above"),
    }
}

fn minimal_cut_sets(nodes: &[Node], root: usize, max_cardinality: usize) -> Vec<Vec<String>> {
    let mut basics: Vec<&str> = Vec::new();
    for node in nodes {
        if node.gate == Gate::Basic && !basics.contains(&node.name.as_str()) {
            basics.push(node.name.as_str());
        }
    }

    let mut cut_sets: Vec<Vec<String>> = Vec::new();
    for mask in 1usize..(1usize << basics.len()) {
        if (mask.count_ones() as usize) > max_cardinality {
            continue;
        }
        let selected: HashSet<&str> = basics
            .iter()
            .enumerate()
            .filter(|(bit, _)| (mask >> bit) & 1 == 1)
            .map(|(_, name)| *name)
            .collect();
        if !evaluates_true(root, nodes, &selected) {
            continue;
        }
        let mut sorted: Vec<String> = selected.iter().map(|name| (*name).to_string()).collect();
        sorted.sort();
        let is_superset = cut_sets
            .iter()
            .any(|existing| existing.iter().all(|name| sorted.contains(name)));
        if !is_superset {
            cut_sets.push(sorted);
        }
    }
    cut_sets.sort_by(|left, right| {
        left.len()
            .cmp(&right.len())
            .then_with(|| left.join(", ").cmp(&right.join(", ")))
    });
    cut_sets
}

/// The recursive gate probability and the minimal cut sets.
///
/// # Errors
/// `InvalidInput` for a malformed tree: no root or several, an unreachable node, a cycle,
/// a childless gate or a basic with children, a duplicate basic-event name, a probability
/// or beta outside `[0, 1]`, a `k` outside `[1, n]`, a field on a gate that does not read
/// it, or more than the procedure's 20 basics or voting children.
pub fn fault_tree(nodes: &[FaultTreeNodeSpec], max_cardinality: f64) -> Result<FaultTreeResult> {
    let spec = &model_gen::FAULT_TREE_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "max_cardinality" => Some(max_cardinality),
            _ => None,
        },
        &mut warnings,
    )?;
    require_whole("max_cardinality", max_cardinality)?;
    if max_cardinality < 1.0 {
        return Err(AzothError::invalid_input(
            "max_cardinality",
            format!("a cut set has at least one member, and {max_cardinality} is not"),
        ));
    }

    let cap = spec
        .algorithm
        .map_or(0, |algorithm| algorithm.max_iterations) as usize;
    let (parsed, root) = parse_nodes(nodes, cap)?;
    let top = top_event_probability(root, &parsed);
    let cut_sets = minimal_cut_sets(&parsed, root, max_cardinality as usize);
    let rows: Vec<MinimalCutSet> = cut_sets
        .iter()
        .map(|members| MinimalCutSet {
            members: members.join(", "),
            cardinality: members.len() as f64,
        })
        .collect();

    Ok(FaultTreeResult {
        top_event_probability: top,
        cut_set_count: rows.len() as f64,
        minimal_cut_sets: rows,
        warnings,
    })
}
