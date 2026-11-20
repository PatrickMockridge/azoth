//! `process.trapped_inventory` against
//! `validation/neqsim/captures/process_trapped_inventory.tsv`.

use azoth_core::WarningCode;
use azoth_core::units::{cubic_meters, kelvins, kilograms_per_cubic_meter, meters, pascals};
use azoth_process::models::trapped_inventory::{
    SegmentKind, TrappedInventoryEvidence, TrappedInventoryResult, TrappedInventorySegment,
    trapped_inventory,
};

fn equipment(id: &str, volume: f64, fill: f64) -> TrappedInventorySegment {
    TrappedInventorySegment {
        segment_id: id.to_string(),
        kind: SegmentKind::Equipment,
        volume: Some(cubic_meters(volume)),
        liquid_fill_fraction: fill,
        internal_diameter: None,
        length: None,
    }
}

fn pipe(id: &str, diameter: f64, length: f64, fill: f64) -> TrappedInventorySegment {
    TrappedInventorySegment {
        segment_id: id.to_string(),
        kind: SegmentKind::Pipe,
        volume: None,
        liquid_fill_fraction: fill,
        internal_diameter: Some(meters(diameter)),
        length: Some(meters(length)),
    }
}

fn evidence(ids: &[&str]) -> Vec<TrappedInventoryEvidence> {
    ids.iter()
        .map(|id| TrappedInventoryEvidence {
            segment_id: (*id).to_string(),
            document_id: Some(format!("DOC-{id}")),
            source_reference: Some(format!("STID-{id}")),
            excerpt: Some("documented volume".to_string()),
        })
        .collect()
}

fn close_with(actual: f64, expected: f64, what: &str, tolerance: f64) {
    let error = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        error <= tolerance,
        "{what}: {actual} against {expected}, relative error {error:.3e} over {tolerance:.3e}"
    );
}

fn components() -> Vec<String> {
    vec![
        "methane".to_string(),
        "ethane".to_string(),
        "n-butane".to_string(),
    ]
}

fn two_phase_state() -> TrappedInventoryResult {
    trapped_inventory(
        &components(),
        &[0.5, 0.3, 0.2],
        pascals(5.0e6),
        kelvins(300.0),
        &[
            equipment("V-101", 10.0, 0.5),
            pipe("P-101", 0.2, 100.0, 0.2),
        ],
        None,
        None,
        None,
        Some(&evidence(&["V-101", "P-101"])),
    )
    .expect("the state evaluates")
}

#[test]
fn the_two_phase_equipment_and_pipe_state() {
    let result = two_phase_state();
    close_with(
        result.gas_density.value,
        62.13948916875829,
        "gas density",
        1.0e-6,
    );
    close_with(
        result.liquid_density.value,
        449.21182603388814,
        "liquid density",
        1.0e-6,
    );
    close_with(
        result.total_volume.value,
        13.141592653589793,
        "volume",
        1.0e-12,
    );
    close_with(
        result.total_gas_mass.value,
        466.87101598010634,
        "gas mass",
        1.0e-6,
    );
    close_with(
        result.total_liquid_mass.value,
        2528.3072446841848,
        "liquid mass",
        1.0e-6,
    );
    assert_eq!(result.segments.len(), 2);
    assert_eq!(result.segments[0].kind, SegmentKind::Equipment);
    assert_eq!(result.segments[1].kind, SegmentKind::Pipe);
    assert!(
        !result
            .warnings
            .iter()
            .any(|warning| warning.code == WarningCode::FallbackToInput)
    );
}

#[test]
fn the_gas_only_state_uses_the_fallback_and_warns() {
    let result = trapped_inventory(
        &["methane".to_string()],
        &[1.0],
        pascals(5.0e6),
        kelvins(300.0),
        &[equipment("V-GAS", 5.0, 0.5)],
        None,
        None,
        None,
        Some(&evidence(&["V-GAS"])),
    )
    .expect("the state evaluates");
    close_with(result.liquid_density.value, 800.0, "fallback", 1.0e-12);
    close_with(
        result.total_gas_mass.value,
        88.1385351403471,
        "gas mass",
        1.0e-6,
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.code == WarningCode::FallbackToInput)
    );
}

#[test]
fn a_segment_with_no_evidence_warns() {
    let result = trapped_inventory(
        &components(),
        &[0.5, 0.3, 0.2],
        pascals(5.0e6),
        kelvins(300.0),
        &[equipment("V-NOEV", 5.0, 0.5)],
        None,
        None,
        None,
        None,
    )
    .expect("the state evaluates");
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.code == WarningCode::RangeCheckSkipped)
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.message == "Segment V-NOEV has no traceable document evidence.")
    );
}

#[test]
fn the_water_bearing_multiphase_state_uses_the_aqueous_density() {
    let result = trapped_inventory(
        &["methane".to_string(), "water".to_string()],
        &[0.9, 0.1],
        pascals(5.0e6),
        kelvins(300.0),
        &[equipment("V-WET", 5.0, 0.5)],
        None,
        Some(true),
        None,
        Some(&evidence(&["V-WET"])),
    )
    .expect("the state evaluates");
    close_with(
        result.liquid_density.value,
        984.6166289023153,
        "aqueous density",
        1.0e-5,
    );
    close_with(
        result.total_liquid_mass.value,
        2461.541572255788,
        "liquid mass",
        1.0e-5,
    );
}

#[test]
fn the_srk_state_and_the_refusals() {
    let result = trapped_inventory(
        &components(),
        &[0.5, 0.3, 0.2],
        pascals(5.0e6),
        kelvins(300.0),
        &[
            equipment("V-SRK", 10.0, 0.5),
            pipe("P-SRK", 0.2, 100.0, 0.2),
        ],
        Some("srk"),
        None,
        None,
        Some(&evidence(&["V-SRK", "P-SRK"])),
    )
    .expect("the state evaluates");
    // The SRK arm is `1.84e-2` relative below the class on this state; the case records it.
    close_with(
        result.gas_density.value,
        60.61952618325027,
        "gas density",
        2.0e-2,
    );
    assert!(
        trapped_inventory(
            &components(),
            &[0.5, 0.3, 0.2],
            pascals(5.0e6),
            kelvins(300.0),
            &[],
            None,
            None,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        trapped_inventory(
            &components(),
            &[0.5, 0.3, 0.2],
            pascals(5.0e6),
            kelvins(300.0),
            &[equipment("V-101", 10.0, 1.5)],
            None,
            None,
            Some(kilograms_per_cubic_meter(800.0)),
            None,
        )
        .is_err()
    );
}
