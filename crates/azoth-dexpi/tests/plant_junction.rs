//! Mixer and splitter junctions: a `PipeTee`, not a tagged item, and its streams.

mod common;

use azoth_dexpi::plant;
use azoth_dexpi::plant::export::{PlantExportOptions, export_plant};
use azoth_dexpi::plant::import::{PlantImportOptions, import_plant};
use azoth_dexpi::view::{CoverageStatus, inspect};

fn pid() -> azoth_dexpi::PlantExportReport {
    export_plant(
        &common::mix_split(),
        &common::palette(),
        &PlantExportOptions {
            allow_lossy: true,
            azoth_extensions: true,
            drawing: true,
            ..PlantExportOptions::default()
        },
    )
    .expect("the junction fixture exports a P&ID")
}

#[test]
fn the_junctions_are_pipe_tees_and_not_tagged_items() {
    let report = pid();
    assert!(report.semantic_valid);
    assert_eq!(report.items, 1, "only the cooler is a tagged item");
    assert_eq!(report.segments, 6, "one segment per incident stream");
    assert_eq!(report.xml.matches("Plant/Piping.PipeTee").count(), 2);
    assert!(report.xml.contains("<String>M-101</String>"));
    assert!(report.xml.contains("<String>S-101</String>"));
    let path = common::temp_path("mix_split.xml");
    std::fs::write(&path, &report.xml).expect("the P&ID writes");
    common::xmllint(&path);
}

#[test]
fn the_junctions_survive_the_round_trip() {
    let imported = import_plant(
        &pid().xml,
        &common::palette(),
        &PlantImportOptions::default(),
    )
    .expect("the P&ID imports");
    let units: Vec<(&str, &str)> = imported
        .flowsheet
        .instances
        .iter()
        .map(|instance| (instance.id.as_str(), instance.unit.as_str()))
        .collect();
    assert!(units.contains(&("M-101", "unit_ops.mixer")), "{units:?}");
    assert!(units.contains(&("S-101", "unit_ops.splitter")), "{units:?}");
    assert_eq!(imported.flowsheet.connections.len(), 6);
    let to_mixer = imported
        .flowsheet
        .connections
        .iter()
        .filter(|connection| connection.to == "M-101.feed")
        .count();
    assert_eq!(to_mixer, 2, "both feeds reach the mixer's many inlet");
    let from_splitter: Vec<&str> = imported
        .flowsheet
        .connections
        .iter()
        .filter(|connection| connection.from.starts_with("S-101.products["))
        .map(|connection| connection.from.as_str())
        .collect();
    assert_eq!(
        from_splitter.len(),
        2,
        "{:?}",
        imported.flowsheet.connections
    );
    assert!(from_splitter.contains(&"S-101.products[0]"));
    assert!(from_splitter.contains(&"S-101.products[1]"));
}

#[test]
fn a_tee_is_an_ambiguous_junction_in_the_view() {
    let view = inspect(&pid().xml, &common::palette()).expect("the P&ID is viewable");
    let tees: Vec<_> = view
        .objects
        .iter()
        .filter(|object| object.ty.ends_with(".PipeTee"))
        .collect();
    assert_eq!(tees.len(), 2, "{:?}", view.objects);
    for tee in tees {
        assert_eq!(tee.status, CoverageStatus::Ambiguous);
        assert_eq!(tee.alternatives.len(), 2);
    }
}

#[test]
fn the_mapping_counts_the_junctions_as_writable() {
    let writable = plant::units()
        .iter()
        .filter(|row| row.write != azoth_dexpi::Disposition::Refused)
        .count();
    assert_eq!(writable, 12);
    assert!(plant::is_junction_unit("unit_ops.mixer"));
    assert!(plant::is_junction_unit("unit_ops.splitter"));
    assert!(!plant::is_junction_unit("unit_ops.pump"));
}
