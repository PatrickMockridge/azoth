//! `process.mah_bow_tie_skeleton` - the ISO 17776 MAH catalogue skeleton.
//!
//! Spec: `specs/models/process/mah_bow_tie_skeleton.toml`. The arithmetic is NeqSim's
//! `process/safety/hazid/MahBowTieBuilder.java` and `MahCatalogue.java`: the default
//! threats, consequences and barrier classes at the class's placeholder constants.

use azoth_core::{Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::hazid::MahType;

/// The class's placeholder threat frequency, events per year.
const DEFAULT_THREAT_FREQUENCY: f64 = 1.0e-3;

/// The class's placeholder consequence severity, 1 to 5.
const DEFAULT_CONSEQUENCE_SEVERITY: f64 = 4.0;

/// The class's placeholder barrier PFD.
const DEFAULT_BARRIER_PFD: f64 = 1.0e-2;

/// One threat row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MahThreat {
    /// The row id, `T1` and upwards.
    pub id: String,
    /// The threat.
    pub description: String,
    /// The placeholder frequency in 1/year.
    pub frequency_per_year: f64,
}

/// One consequence row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MahConsequence {
    /// The row id, `C1` and upwards.
    pub id: String,
    /// The consequence.
    pub description: String,
    /// The placeholder severity.
    pub severity: f64,
}

/// One barrier row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MahBarrier {
    /// The row id, `B1` and upwards.
    pub id: String,
    /// The barrier class.
    pub description: String,
    /// The placeholder PFD.
    pub pfd: f64,
}

/// Result of `process.mah_bow_tie_skeleton`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MahBowTieSkeletonResult {
    /// The default threats.
    pub threats: Vec<MahThreat>,
    /// The default consequences.
    pub consequences: Vec<MahConsequence>,
    /// The default barrier classes.
    pub barriers: Vec<MahBarrier>,
    /// The threat count.
    pub threat_count: f64,
    /// The consequence count.
    pub consequence_count: f64,
    /// The barrier count.
    pub barrier_count: f64,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The catalogue skeleton for one major accident hazard.
///
/// # Errors
/// `InvalidInput` for a type that is not one of the eleven ISO 17776 categories.
pub fn mah_bow_tie_skeleton(mah_type: &str) -> Result<MahBowTieSkeletonResult> {
    let spec = &model_gen::MAH_BOW_TIE_SKELETON_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    let mah = MahType::parse(mah_type)?;
    let entry = mah.entry();

    let threats: Vec<MahThreat> = entry
        .threats
        .iter()
        .enumerate()
        .map(|(index, description)| MahThreat {
            id: format!("T{}", index + 1),
            description: (*description).to_string(),
            frequency_per_year: DEFAULT_THREAT_FREQUENCY,
        })
        .collect();
    let consequences: Vec<MahConsequence> = entry
        .consequences
        .iter()
        .enumerate()
        .map(|(index, description)| MahConsequence {
            id: format!("C{}", index + 1),
            description: (*description).to_string(),
            severity: DEFAULT_CONSEQUENCE_SEVERITY,
        })
        .collect();
    let barriers: Vec<MahBarrier> = entry
        .barriers
        .iter()
        .enumerate()
        .map(|(index, description)| MahBarrier {
            id: format!("B{}", index + 1),
            description: (*description).to_string(),
            pfd: DEFAULT_BARRIER_PFD,
        })
        .collect();

    Ok(MahBowTieSkeletonResult {
        threat_count: threats.len() as f64,
        consequence_count: consequences.len() as f64,
        barrier_count: barriers.len() as f64,
        threats,
        consequences,
        barriers,
        warnings,
    })
}
