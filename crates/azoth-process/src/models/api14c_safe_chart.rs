//! `process.api14c_safe_chart` - the API 14C SAFE chart.
//!
//! Spec: `specs/models/process/api14c_safe_chart.toml`. The arithmetic is NeqSim's
//! `process/safety/api14c/Api14cSafeChartBuilder.java`: the class-name classifier, the
//! required-device table, and the present-versus-required comparison.

use azoth_core::{Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::safety_review::{
    Api14cCategory, Api14cDevice, api14c_classify, api14c_required,
};

/// One equipment row a caller states.
#[derive(Debug, Clone, PartialEq)]
pub struct SafeChartEquipment {
    /// The equipment name or tag.
    pub equipment: String,
    /// The unit's simple class name or category.
    pub unit_class: String,
}

/// One declared-present device.
#[derive(Debug, Clone, PartialEq)]
pub struct SafeChartPresentDevice {
    /// The equipment name.
    pub equipment: String,
    /// The device, the spec's spelling.
    pub device: String,
}

/// One chart row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SafeChartItem {
    /// The equipment name.
    pub equipment: String,
    /// The SAFE-table category.
    pub category: Api14cCategory,
    /// The number of required devices.
    pub required_count: f64,
    /// The number of declared-present devices.
    pub present_count: f64,
    /// The number of required devices that are not present.
    pub missing_count: f64,
    /// Whether the missing set is empty.
    pub complete: bool,
}

/// One missing device.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SafeChartMissingDevice {
    /// The equipment name.
    pub equipment: String,
    /// The missing device.
    pub device: Api14cDevice,
}

/// Result of `process.api14c_safe_chart`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Api14cSafeChartResult {
    /// One row per classified equipment.
    pub items: Vec<SafeChartItem>,
    /// One row per missing device.
    pub missing_devices: Vec<SafeChartMissingDevice>,
    /// The number of classified equipment rows.
    pub equipment_count: f64,
    /// The total number of missing devices.
    pub missing_count: f64,
    /// Whether every row is complete.
    pub complete: bool,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The SAFE chart over the caller's equipment rows.
///
/// # Errors
/// `InvalidInput` for a declared device that is not one of the spec's device values.
pub fn api14c_safe_chart(
    equipment: Option<&[SafeChartEquipment]>,
    present_devices: Option<&[SafeChartPresentDevice]>,
    assume_complete: Option<bool>,
) -> Result<Api14cSafeChartResult> {
    let spec = &model_gen::API14C_SAFE_CHART_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    // The declarations are a flattened set per equipment name.
    let mut declared: Vec<(String, Vec<Api14cDevice>)> = Vec::new();
    for row in present_devices.unwrap_or(&[]) {
        let device = Api14cDevice::parse(&row.device)?;
        match declared.iter_mut().find(|(name, _)| name == &row.equipment) {
            Some((_, devices)) => {
                if !devices.contains(&device) {
                    devices.push(device);
                }
            }
            None => declared.push((row.equipment.clone(), vec![device])),
        }
    }

    let complete_mode = assume_complete.unwrap_or(false);
    let mut items = Vec::new();
    let mut missing_devices = Vec::new();
    for row in equipment.unwrap_or(&[]) {
        let Some(category) = api14c_classify(&row.unit_class) else {
            continue;
        };
        let required = api14c_required(category);
        let present: Vec<Api14cDevice> = if complete_mode {
            required.to_vec()
        } else {
            declared
                .iter()
                .find(|(name, _)| name == &row.equipment)
                .map(|(_, devices)| devices.clone())
                .unwrap_or_default()
        };
        let missing: Vec<Api14cDevice> = required
            .iter()
            .copied()
            .filter(|device| !present.contains(device))
            .collect();
        let complete = missing.is_empty();
        items.push(SafeChartItem {
            equipment: row.equipment.clone(),
            category,
            required_count: required.len() as f64,
            present_count: present.len() as f64,
            missing_count: missing.len() as f64,
            complete,
        });
        for device in missing {
            missing_devices.push(SafeChartMissingDevice {
                equipment: row.equipment.clone(),
                device,
            });
        }
    }

    Ok(Api14cSafeChartResult {
        equipment_count: items.len() as f64,
        missing_count: missing_devices.len() as f64,
        complete: items.iter().all(|item| item.complete),
        items,
        missing_devices,
        warnings,
    })
}
