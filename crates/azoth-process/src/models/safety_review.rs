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

/// A safety-critical element's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BarrierElementType {
    /// Process equipment.
    #[serde(rename = "process_equipment")]
    ProcessEquipment,
    /// Instrumented function.
    #[serde(rename = "instrumented_function")]
    InstrumentedFunction,
    /// Fire protection.
    #[serde(rename = "fire_protection")]
    FireProtection,
    /// Structural.
    #[serde(rename = "structural")]
    Structural,
    /// Utility.
    #[serde(rename = "utility")]
    Utility,
    /// Other.
    #[serde(rename = "other")]
    Other,
}

impl BarrierElementType {
    /// Parses one of the spec's element types.
    pub(crate) fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "process_equipment" => Self::ProcessEquipment,
            "instrumented_function" => Self::InstrumentedFunction,
            "fire_protection" => Self::FireProtection,
            "structural" => Self::Structural,
            "utility" => Self::Utility,
            "other" => Self::Other,
            _ => {
                return Err(AzothError::invalid_input(
                    "type",
                    format!("`{value}` is not one of the element types"),
                ));
            }
        })
    }
}

/// A barrier's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BarrierKind {
    /// Prevention.
    #[serde(rename = "prevention")]
    Prevention,
    /// Mitigation.
    #[serde(rename = "mitigation")]
    Mitigation,
    /// Both.
    #[serde(rename = "both")]
    Both,
}

impl BarrierKind {
    /// Parses one of the spec's barrier types.
    pub(crate) fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "prevention" => Self::Prevention,
            "mitigation" => Self::Mitigation,
            "both" => Self::Both,
            _ => {
                return Err(AzothError::invalid_input(
                    "type",
                    format!("`{value}` is not one of the barrier types"),
                ));
            }
        })
    }
}

/// A barrier's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BarrierStatus {
    /// Available.
    #[serde(rename = "available")]
    Available,
    /// Impaired.
    #[serde(rename = "impaired")]
    Impaired,
    /// Bypassed.
    #[serde(rename = "bypassed")]
    Bypassed,
    /// Out of service.
    #[serde(rename = "out_of_service")]
    OutOfService,
    /// Unknown.
    #[serde(rename = "unknown")]
    Unknown,
}

impl BarrierStatus {
    /// Parses one of the spec's statuses.
    pub(crate) fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "available" => Self::Available,
            "impaired" => Self::Impaired,
            "bypassed" => Self::Bypassed,
            "out_of_service" => Self::OutOfService,
            "unknown" => Self::Unknown,
            _ => {
                return Err(AzothError::invalid_input(
                    "status",
                    format!("`{value}` is not one of the barrier statuses"),
                ));
            }
        })
    }
}

/// Which record a piece of evidence is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EvidenceOwner {
    /// A safety-critical element.
    #[serde(rename = "element")]
    Element,
    /// A barrier.
    #[serde(rename = "barrier")]
    Barrier,
    /// A performance standard.
    #[serde(rename = "standard")]
    Standard,
}

impl EvidenceOwner {
    /// Parses one of the spec's owner kinds.
    pub(crate) fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "element" => Self::Element,
            "barrier" => Self::Barrier,
            "standard" => Self::Standard,
            _ => {
                return Err(AzothError::invalid_input(
                    "owner_kind",
                    format!("`{value}` is not one of the evidence owner kinds"),
                ));
            }
        })
    }
}

/// Which record a finding is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FindingScope {
    /// The register.
    #[serde(rename = "register")]
    Register,
    /// A safety-critical element.
    #[serde(rename = "element")]
    Element,
    /// A barrier.
    #[serde(rename = "barrier")]
    Barrier,
    /// A performance standard.
    #[serde(rename = "standard")]
    Standard,
}

impl FindingScope {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::Element => "element",
            Self::Barrier => "barrier",
            Self::Standard => "standard",
        }
    }
}

/// A register rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RegisterRule {
    /// The register id is missing.
    #[serde(rename = "register_id_missing")]
    RegisterIdMissing,
    /// The register has no elements and no barriers.
    #[serde(rename = "register_empty")]
    RegisterEmpty,
    /// An element id is missing.
    #[serde(rename = "element_id_missing")]
    ElementIdMissing,
    /// An element has neither tag nor name.
    #[serde(rename = "element_tag_or_name_missing")]
    ElementTagOrNameMissing,
    /// An element has no barriers.
    #[serde(rename = "element_no_barriers")]
    ElementNoBarriers,
    /// An element has no traceable evidence.
    #[serde(rename = "element_no_traceable_evidence")]
    ElementNoTraceableEvidence,
    /// A barrier id is missing.
    #[serde(rename = "barrier_id_missing")]
    BarrierIdMissing,
    /// A barrier has neither name nor description.
    #[serde(rename = "barrier_name_or_description_missing")]
    BarrierNameOrDescriptionMissing,
    /// A barrier PFD is outside `[0, 1]`.
    #[serde(rename = "barrier_pfd_out_of_range")]
    BarrierPfdOutOfRange,
    /// A barrier effectiveness is outside `[0, 1]`.
    #[serde(rename = "barrier_effectiveness_out_of_range")]
    BarrierEffectivenessOutOfRange,
    /// A barrier status is unknown.
    #[serde(rename = "barrier_status_unknown")]
    BarrierStatusUnknown,
    /// A barrier has no linked equipment tag.
    #[serde(rename = "barrier_no_equipment_tag")]
    BarrierNoEquipmentTag,
    /// A barrier has no performance standard.
    #[serde(rename = "barrier_no_performance_standard")]
    BarrierNoPerformanceStandard,
    /// A barrier has no traceable evidence.
    #[serde(rename = "barrier_no_traceable_evidence")]
    BarrierNoTraceableEvidence,
    /// A standard id is missing.
    #[serde(rename = "standard_id_missing")]
    StandardIdMissing,
    /// A standard has no safety function.
    #[serde(rename = "standard_safety_function_missing")]
    StandardSafetyFunctionMissing,
    /// A standard target PFD is outside `(0, 1]`.
    #[serde(rename = "standard_target_pfd_out_of_range")]
    StandardTargetPfdOutOfRange,
    /// A standard required availability is outside `[0, 1]`.
    #[serde(rename = "standard_required_availability_out_of_range")]
    StandardRequiredAvailabilityOutOfRange,
    /// A standard proof-test interval is not positive.
    #[serde(rename = "standard_proof_test_interval_not_positive")]
    StandardProofTestIntervalNotPositive,
    /// A standard has no acceptance criterion.
    #[serde(rename = "standard_no_acceptance_criteria")]
    StandardNoAcceptanceCriteria,
    /// A standard has no traceable evidence.
    #[serde(rename = "standard_no_traceable_evidence")]
    StandardNoTraceableEvidence,
    /// A barrier is not linked to an element.
    #[serde(rename = "barrier_not_linked_to_element")]
    BarrierNotLinkedToElement,
}

impl RegisterRule {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RegisterIdMissing => "register_id_missing",
            Self::RegisterEmpty => "register_empty",
            Self::ElementIdMissing => "element_id_missing",
            Self::ElementTagOrNameMissing => "element_tag_or_name_missing",
            Self::ElementNoBarriers => "element_no_barriers",
            Self::ElementNoTraceableEvidence => "element_no_traceable_evidence",
            Self::BarrierIdMissing => "barrier_id_missing",
            Self::BarrierNameOrDescriptionMissing => "barrier_name_or_description_missing",
            Self::BarrierPfdOutOfRange => "barrier_pfd_out_of_range",
            Self::BarrierEffectivenessOutOfRange => "barrier_effectiveness_out_of_range",
            Self::BarrierStatusUnknown => "barrier_status_unknown",
            Self::BarrierNoEquipmentTag => "barrier_no_equipment_tag",
            Self::BarrierNoPerformanceStandard => "barrier_no_performance_standard",
            Self::BarrierNoTraceableEvidence => "barrier_no_traceable_evidence",
            Self::StandardIdMissing => "standard_id_missing",
            Self::StandardSafetyFunctionMissing => "standard_safety_function_missing",
            Self::StandardTargetPfdOutOfRange => "standard_target_pfd_out_of_range",
            Self::StandardRequiredAvailabilityOutOfRange => {
                "standard_required_availability_out_of_range"
            }
            Self::StandardProofTestIntervalNotPositive => {
                "standard_proof_test_interval_not_positive"
            }
            Self::StandardNoAcceptanceCriteria => "standard_no_acceptance_criteria",
            Self::StandardNoTraceableEvidence => "standard_no_traceable_evidence",
            Self::BarrierNotLinkedToElement => "barrier_not_linked_to_element",
        }
    }
}
