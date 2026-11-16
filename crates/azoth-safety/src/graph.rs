//! The bow-tie as the canvas reads it.
//!
//! **The drawing is a projection of the document, not a second copy.** Nodes and edges are
//! derived on every envelope; a drag is a document edit like any other. The prevention barriers
//! are drawn against the aggregate because that is the arithmetic: `process.bow_tie` multiplies
//! every functional prevention barrier once and reads no threat-to-barrier link.

use serde_json::{Value, json};

use crate::study::{Barrier, Bowtie, BowtieLayout};

/// The x of each rank: threats, prevention barriers, top event, mitigation barriers,
/// consequences.
const COLUMNS: [f64; 5] = [0.0, 240.0, 480.0, 720.0, 960.0];

/// The y every rank is centred on, and the spacing between rows.
const CENTRE_Y: f64 = 120.0;
const ROW: f64 = 90.0;

/// One bow-tie, as nodes and edges.
#[must_use]
pub fn graph(bowtie: &Bowtie, layout: Option<&BowtieLayout>) -> Value {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for (index, threat) in bowtie.threats.iter().enumerate() {
        let id = format!("threat:{}", threat.id);
        nodes.push(json!({
            "id": id,
            "type": "threat",
            "role": "threat",
            "position": point(layout.and_then(|entry| entry.threats.get(&threat.id)), index, bowtie.threats.len(), COLUMNS[0]),
            "data": {
                "label": threat.label,
                "frequency": threat.frequency,
                "active": threat.active,
            },
        }));
    }

    for (index, barrier) in bowtie.barriers.iter().enumerate() {
        let side = side_of(barrier);
        let column = if side == "prevention" {
            COLUMNS[1]
        } else {
            COLUMNS[3]
        };
        let id = format!("barrier:{}", barrier.id);
        nodes.push(json!({
            "id": id,
            "type": "barrier",
            "role": side,
            "position": point(layout.and_then(|entry| entry.barriers.get(&barrier.id)), index, bowtie.barriers.len(), column),
            "data": {
                "label": barrier.label,
                "pfd": barrier.pfd,
                "barrier_type": barrier.barrier_type,
                "functional": barrier.functional,
                "side": side,
            },
        }));
    }

    nodes.push(json!({
        "id": "top_event",
        "type": "top_event",
        "role": "top_event",
        "position": point(layout.and_then(|entry| entry.top_event).as_ref(), 0, 1, COLUMNS[2]),
        "data": { "label": bowtie.top_event },
    }));

    for (index, consequence) in bowtie.consequences.iter().enumerate() {
        let id = format!("consequence:{}", consequence.id);
        nodes.push(json!({
            "id": id,
            "type": "consequence",
            "role": "consequence",
            "position": point(layout.and_then(|entry| entry.consequences.get(&consequence.id)), index, bowtie.consequences.len(), COLUMNS[4]),
            "data": {
                "label": consequence.label,
                "severity": consequence.severity,
                "scenario": consequence.scenario,
            },
        }));
    }

    let threats: Vec<String> = bowtie
        .threats
        .iter()
        .map(|threat| format!("threat:{}", threat.id))
        .collect();
    let prevention: Vec<String> = bowtie
        .barriers
        .iter()
        .filter(|barrier| side_of(barrier) == "prevention")
        .map(|barrier| format!("barrier:{}", barrier.id))
        .collect();
    let mitigation: Vec<String> = bowtie
        .barriers
        .iter()
        .filter(|barrier| side_of(barrier) == "mitigation")
        .map(|barrier| format!("barrier:{}", barrier.id))
        .collect();
    let consequences: Vec<String> = bowtie
        .consequences
        .iter()
        .map(|consequence| format!("consequence:{}", consequence.id))
        .collect();

    connect(&mut edges, &threats, &prevention);
    if prevention.is_empty() {
        connect(&mut edges, &threats, &["top_event".to_string()]);
    } else {
        connect(&mut edges, &prevention, &["top_event".to_string()]);
    }
    connect(&mut edges, &["top_event".to_string()], &mitigation);
    let right: Vec<String> = if mitigation.is_empty() {
        vec!["top_event".to_string()]
    } else {
        mitigation.clone()
    };
    connect(&mut edges, &right, &consequences);

    json!({
        "id": format!("bowtie:{}", bowtie.id),
        "nodes": nodes,
        "edges": edges,
    })
}

/// `prevention`, `mitigation`, or the side a `both` barrier is drawn on.
fn side_of(barrier: &Barrier) -> &'static str {
    if barrier.barrier_type == "mitigation" {
        "mitigation"
    } else {
        "prevention"
    }
}

fn point(stated: Option<&[f64; 2]>, index: usize, count: usize, x: f64) -> Value {
    let (x, y) = stated.map_or_else(
        || {
            let offset = index as f64 - (count as f64 - 1.0) / 2.0;
            (x, CENTRE_Y + offset * ROW)
        },
        |position| (position[0], position[1]),
    );
    json!({ "x": x, "y": y })
}

fn connect(edges: &mut Vec<Value>, sources: &[String], targets: &[String]) {
    for source in sources {
        for target in targets {
            edges.push(json!({
                "id": format!("edge:{source}->{target}"),
                "source": source,
                "target": target,
            }));
        }
    }
}
