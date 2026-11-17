//! `process.api14c_safe_chart` against
//! `validation/neqsim/captures/process_api14c_safe_chart.tsv`.

use azoth_process::models::api14c_safe_chart::{
    SafeChartEquipment, SafeChartPresentDevice, api14c_safe_chart,
};
use azoth_process::models::safety_review::{Api14cCategory, Api14cDevice};

fn equipment(name: &str, class: &str) -> SafeChartEquipment {
    SafeChartEquipment {
        equipment: name.to_string(),
        unit_class: class.to_string(),
    }
}

fn present(name: &str, device: &str) -> SafeChartPresentDevice {
    SafeChartPresentDevice {
        equipment: name.to_string(),
        device: device.to_string(),
    }
}

fn chart(
    equipment: &[SafeChartEquipment],
    present_devices: Option<&[SafeChartPresentDevice]>,
    assume_complete: Option<bool>,
) -> azoth_process::models::api14c_safe_chart::Api14cSafeChartResult {
    api14c_safe_chart(Some(equipment), present_devices, assume_complete)
        .expect("the captured state is valid")
}

#[test]
fn the_table_is_the_captured_requirement() {
    // One unit per category; with nothing declared, every required device is missing, in
    // the device declaration order.
    let cases: [(Api14cCategory, &str, &[Api14cDevice]); 8] = [
        (
            Api14cCategory::PressureVessel,
            "Separator",
            &[
                Api14cDevice::Psh,
                Api14cDevice::Psl,
                Api14cDevice::Lsh,
                Api14cDevice::Lsl,
                Api14cDevice::Sdv,
                Api14cDevice::Psv,
            ],
        ),
        (
            Api14cCategory::AtmosphericVessel,
            "Tank",
            &[Api14cDevice::Lsh, Api14cDevice::Psv, Api14cDevice::Fsv],
        ),
        (
            Api14cCategory::FiredVessel,
            "FiredHeater",
            &[
                Api14cDevice::Psh,
                Api14cDevice::Lsl,
                Api14cDevice::Tsh,
                Api14cDevice::Sdv,
                Api14cDevice::Bdv,
                Api14cDevice::Psv,
                Api14cDevice::Fire,
            ],
        ),
        (
            Api14cCategory::PipelineSegment,
            "PipeBeggsAndBrills",
            &[
                Api14cDevice::Psh,
                Api14cDevice::Psl,
                Api14cDevice::Sdv,
                Api14cDevice::Fsv,
            ],
        ),
        (
            Api14cCategory::Compressor,
            "Compressor",
            &[
                Api14cDevice::Psh,
                Api14cDevice::Psl,
                Api14cDevice::Tsh,
                Api14cDevice::Sdv,
                Api14cDevice::Bdv,
                Api14cDevice::Psv,
            ],
        ),
        (
            Api14cCategory::Pump,
            "Pump",
            &[
                Api14cDevice::Psh,
                Api14cDevice::Sdv,
                Api14cDevice::Psv,
                Api14cDevice::Fsv,
            ],
        ),
        (
            Api14cCategory::HeatExchanger,
            "Cooler",
            &[Api14cDevice::Psh, Api14cDevice::Tsh, Api14cDevice::Psv],
        ),
        (
            Api14cCategory::Wellhead,
            "WellFlow",
            &[
                Api14cDevice::Psh,
                Api14cDevice::Psl,
                Api14cDevice::Usv,
                Api14cDevice::Sdv,
                Api14cDevice::Psv,
            ],
        ),
    ];

    for (category, class, required) in cases {
        let result = chart(&[equipment("E-1", class)], None, None);
        assert_eq!(result.items.len(), 1, "{category:?}");
        assert_eq!(result.items[0].category, category);
        assert_eq!(result.items[0].required_count, required.len() as f64);
        let missing: Vec<Api14cDevice> = result
            .missing_devices
            .iter()
            .map(|row| row.device)
            .collect();
        assert_eq!(missing, required, "{category:?}");
    }
}

#[test]
fn the_classifier_is_the_captured_substring_order() {
    let rows = [
        equipment("a", "GasScrubber"),
        equipment("b", "FlashDrum"),
        equipment("c", "Flash"),
        equipment("d", "Drum"),
        equipment("e", "KnockOutDrum"),
        equipment("f", "LNGTank"),
        equipment("g", "Furnace"),
        equipment("h", "Riser"),
        equipment("i", "Expander"),
        equipment("j", "AirCooler"),
        equipment("k", "Stream"),
    ];
    let result = chart(&rows, None, None);
    let categories: Vec<Api14cCategory> = result.items.iter().map(|item| item.category).collect();
    // The skipped rows do not appear; the eight classified ones do, in caller order.
    assert_eq!(
        categories,
        vec![
            Api14cCategory::PressureVessel,
            Api14cCategory::PressureVessel,
            Api14cCategory::PressureVessel,
            Api14cCategory::AtmosphericVessel,
            Api14cCategory::FiredVessel,
            Api14cCategory::PipelineSegment,
            Api14cCategory::Compressor,
            Api14cCategory::HeatExchanger,
        ]
    );
    let names: Vec<&str> = result
        .items
        .iter()
        .map(|item| item.equipment.as_str())
        .collect();
    assert_eq!(names, vec!["a", "b", "e", "f", "g", "h", "i", "j"]);
}

#[test]
fn a_declared_equipment_keeps_its_extras_and_reports_the_gap() {
    let result = chart(
        &[equipment("V-101", "Separator")],
        Some(&[
            present("V-101", "psh"),
            present("V-101", "psv"),
            present("V-101", "gas"),
        ]),
        None,
    );
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].present_count, 3.0);
    assert_eq!(result.items[0].missing_count, 4.0);
    assert!(!result.items[0].complete);
    let missing: Vec<Api14cDevice> = result
        .missing_devices
        .iter()
        .map(|row| row.device)
        .collect();
    assert_eq!(
        missing,
        vec![
            Api14cDevice::Psl,
            Api14cDevice::Lsh,
            Api14cDevice::Lsl,
            Api14cDevice::Sdv,
        ]
    );
    assert_eq!(result.equipment_count, 1.0);
    assert_eq!(result.missing_count, 4.0);
    assert!(!result.complete);
}

#[test]
fn an_undeclared_equipment_misses_everything() {
    let result = chart(&[equipment("T-101", "Tank")], None, None);
    assert_eq!(result.items[0].present_count, 0.0);
    assert_eq!(result.items[0].missing_count, 3.0);
    assert_eq!(result.missing_count, 3.0);
}

#[test]
fn assuming_complete_ignores_the_declarations() {
    let result = chart(
        &[equipment("V-101", "Separator")],
        Some(&[present("V-101", "psh")]),
        Some(true),
    );
    assert_eq!(result.items[0].present_count, 6.0);
    assert_eq!(result.items[0].missing_count, 0.0);
    assert!(result.items[0].complete);
    assert!(result.complete);
    assert!(result.missing_devices.is_empty());
}

#[test]
fn an_empty_chart_is_complete() {
    let result = api14c_safe_chart(None, None, None).expect("the captured state is valid");
    assert!(result.items.is_empty());
    assert_eq!(result.equipment_count, 0.0);
    assert_eq!(result.missing_count, 0.0);
    assert!(result.complete);
}

#[test]
fn an_unknown_device_is_refused() {
    let error = api14c_safe_chart(
        Some(&[equipment("V-101", "Separator")]),
        Some(&[present("V-101", "xyz")]),
        None,
    );
    assert!(error.is_err());
}
