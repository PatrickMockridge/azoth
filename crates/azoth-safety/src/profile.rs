//! The distance-to-threshold table a fire scenario shows.
//!
//! **Derived by the same kernel, not by the browser.** Each row is one call to the scenario's
//! own model with `distance` removed and the threshold set, so the table cannot disagree with
//! the model it is drawn beside. The thresholds are screening points the specs and cases
//! already use, not a second correlation.

use azoth_sheets::dispatch::Outcome;
use serde_json::{Value, json};

use crate::dispatch;

/// The incident-flux thresholds, W/m²: pain at 1.58, equipment at 4.73, ignition of wood at
/// 12.5, and the three commonly screened distances beyond it.
pub const FLUX_TARGETS: &[f64] = &[1_580.0, 4_730.0, 6_300.0, 9_460.0, 12_500.0, 37_500.0];

/// The overpressure thresholds, Pa: minor, moderate and severe structural damage.
pub const OVERPRESSURE_TARGETS: &[f64] = &[5_000.0, 10_000.0, 20_000.0];

/// The distance-to-threshold rows for one scenario, empty for an id with no such inverse.
#[must_use]
pub fn profile(id: &str, inputs: &Value) -> Vec<Value> {
    let (input, targets, output, unit) = match id {
        "process.bleve" | "process.jet_fire" | "process.pool_fire" => {
            ("target_flux", FLUX_TARGETS, "distance_to_flux", "W/m**2")
        }
        "process.vce" => (
            "target_overpressure",
            OVERPRESSURE_TARGETS,
            "distance_to_overpressure",
            "Pa",
        ),
        _ => return Vec::new(),
    };
    let Some(base) = inputs.as_object() else {
        return Vec::new();
    };
    targets
        .iter()
        .map(|target| {
            let mut call = base.clone();
            call.remove("distance");
            call.insert(input.to_string(), json!(target));
            let distance = match dispatch::run(id, &Value::Object(call)) {
                Outcome::Ok(result) => result
                    .outputs
                    .get(output)
                    .and_then(|value| value.get("magnitude_si"))
                    .and_then(Value::as_f64),
                Outcome::Error { .. } => None,
            };
            json!({
                "target": { "magnitude_si": target, "unit": unit },
                "distance": distance.map(|value| json!({ "magnitude_si": value, "unit": "m" })),
                "reached": distance.is_some(),
            })
        })
        .collect()
}
