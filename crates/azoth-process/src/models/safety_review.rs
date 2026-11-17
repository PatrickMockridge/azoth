//! The shared internals of the `process.*` safety-review worksheets.
//!
//! The API 14C SAFE chart's device and category vocabularies, its required-device table and
//! the classifier. The arithmetic is NeqSim's
//! `process/safety/api14c/Api14cSafeChartBuilder.java`.

use azoth_core::{AzothError, Result};
use serde::Serialize;

/// An API 14C protective device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Api14cDevice {
    /// Pressure safety high.
    #[serde(rename = "psh")]
    Psh,
    /// Pressure safety low.
    #[serde(rename = "psl")]
    Psl,
    /// Level safety high.
    #[serde(rename = "lsh")]
    Lsh,
    /// Level safety low.
    #[serde(rename = "lsl")]
    Lsl,
    /// Temperature safety high.
    #[serde(rename = "tsh")]
    Tsh,
    /// Temperature safety low.
    #[serde(rename = "tsl")]
    Tsl,
    /// Underwater or subsurface safety valve.
    #[serde(rename = "usv")]
    Usv,
    /// Shutdown valve.
    #[serde(rename = "sdv")]
    Sdv,
    /// Blowdown valve.
    #[serde(rename = "bdv")]
    Bdv,
    /// Pressure safety valve.
    #[serde(rename = "psv")]
    Psv,
    /// Flow safety valve.
    #[serde(rename = "fsv")]
    Fsv,
    /// Fire detection.
    #[serde(rename = "fire")]
    Fire,
    /// Combustible gas detection.
    #[serde(rename = "gas")]
    Gas,
}

impl Api14cDevice {
    /// The class's enum order, which is `Api14cSafetyAnalysisTable.deviceOrder()`.
    pub(crate) const ALL: [Self; 13] = [
        Self::Psh,
        Self::Psl,
        Self::Lsh,
        Self::Lsl,
        Self::Tsh,
        Self::Tsl,
        Self::Usv,
        Self::Sdv,
        Self::Bdv,
        Self::Psv,
        Self::Fsv,
        Self::Fire,
        Self::Gas,
    ];

    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Psh => "psh",
            Self::Psl => "psl",
            Self::Lsh => "lsh",
            Self::Lsl => "lsl",
            Self::Tsh => "tsh",
            Self::Tsl => "tsl",
            Self::Usv => "usv",
            Self::Sdv => "sdv",
            Self::Bdv => "bdv",
            Self::Psv => "psv",
            Self::Fsv => "fsv",
            Self::Fire => "fire",
            Self::Gas => "gas",
        }
    }

    /// Parses one of the spec's device values.
    pub(crate) fn parse(value: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|device| device.as_str() == value)
            .ok_or_else(|| {
                AzothError::invalid_input(
                    "device",
                    format!("`{value}` is not one of the API 14C device types"),
                )
            })
    }
}

/// An API 14C equipment category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Api14cCategory {
    /// Pressure vessel: separator, scrubber or knockout drum.
    #[serde(rename = "pressure_vessel")]
    PressureVessel,
    /// Atmospheric or low-pressure tank.
    #[serde(rename = "atmospheric_vessel")]
    AtmosphericVessel,
    /// Fired vessel: heater or boiler.
    #[serde(rename = "fired_vessel")]
    FiredVessel,
    /// Pipeline segment.
    #[serde(rename = "pipeline_segment")]
    PipelineSegment,
    /// Compressor.
    #[serde(rename = "compressor")]
    Compressor,
    /// Pump.
    #[serde(rename = "pump")]
    Pump,
    /// Heat exchanger.
    #[serde(rename = "heat_exchanger")]
    HeatExchanger,
    /// Wellhead or Christmas tree.
    #[serde(rename = "wellhead")]
    Wellhead,
}

impl Api14cCategory {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PressureVessel => "pressure_vessel",
            Self::AtmosphericVessel => "atmospheric_vessel",
            Self::FiredVessel => "fired_vessel",
            Self::PipelineSegment => "pipeline_segment",
            Self::Compressor => "compressor",
            Self::Pump => "pump",
            Self::HeatExchanger => "heat_exchanger",
            Self::Wellhead => "wellhead",
        }
    }
}

/// The table's required devices for a category, in the device declaration order.
pub(crate) fn api14c_required(category: Api14cCategory) -> &'static [Api14cDevice] {
    match category {
        Api14cCategory::PressureVessel => &[
            Api14cDevice::Psh,
            Api14cDevice::Psl,
            Api14cDevice::Lsh,
            Api14cDevice::Lsl,
            Api14cDevice::Sdv,
            Api14cDevice::Psv,
        ],
        Api14cCategory::AtmosphericVessel => {
            &[Api14cDevice::Lsh, Api14cDevice::Psv, Api14cDevice::Fsv]
        }
        Api14cCategory::FiredVessel => &[
            Api14cDevice::Psh,
            Api14cDevice::Lsl,
            Api14cDevice::Tsh,
            Api14cDevice::Sdv,
            Api14cDevice::Bdv,
            Api14cDevice::Psv,
            Api14cDevice::Fire,
        ],
        Api14cCategory::PipelineSegment => &[
            Api14cDevice::Psh,
            Api14cDevice::Psl,
            Api14cDevice::Sdv,
            Api14cDevice::Fsv,
        ],
        Api14cCategory::Compressor => &[
            Api14cDevice::Psh,
            Api14cDevice::Psl,
            Api14cDevice::Tsh,
            Api14cDevice::Sdv,
            Api14cDevice::Bdv,
            Api14cDevice::Psv,
        ],
        Api14cCategory::Pump => &[
            Api14cDevice::Psh,
            Api14cDevice::Sdv,
            Api14cDevice::Psv,
            Api14cDevice::Fsv,
        ],
        Api14cCategory::HeatExchanger => &[Api14cDevice::Psh, Api14cDevice::Tsh, Api14cDevice::Psv],
        Api14cCategory::Wellhead => &[
            Api14cDevice::Psh,
            Api14cDevice::Psl,
            Api14cDevice::Usv,
            Api14cDevice::Sdv,
            Api14cDevice::Psv,
        ],
    }
}

/// The class's equipment classifier: a lower-case substring test on the simple class name.
pub(crate) fn api14c_classify(unit_class: &str) -> Option<Api14cCategory> {
    let name = unit_class.to_lowercase();
    let has = |word: &str| name.contains(word);
    if has("scrubber")
        || has("separator")
        || has("flashdrum")
        || has("knockout")
        || (has("flash") && has("drum"))
    {
        Some(Api14cCategory::PressureVessel)
    } else if has("tank") {
        Some(Api14cCategory::AtmosphericVessel)
    } else if has("heater") || has("furnace") || has("reformer") {
        Some(Api14cCategory::FiredVessel)
    } else if has("pipe") || has("flowline") || has("riser") {
        Some(Api14cCategory::PipelineSegment)
    } else if has("compressor") || has("expander") {
        Some(Api14cCategory::Compressor)
    } else if has("pump") {
        Some(Api14cCategory::Pump)
    } else if has("heatexchanger")
        || has("cooler")
        || has("condenser")
        || has("reboiler")
        || has("aircooler")
    {
        Some(Api14cCategory::HeatExchanger)
    } else if has("well") {
        Some(Api14cCategory::Wellhead)
    } else {
        None
    }
}
