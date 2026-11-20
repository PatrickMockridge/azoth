//! `process.safety_system_performance` - the safety-system performance assessment.
//!
//! Spec: `specs/models/process/safety_system_performance.toml`. The arithmetic is NeqSim's
//! `process/safety/barrier/SafetySystemPerformanceAnalyzer.java`.

use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::sil_verification::{IssueSeverity, sil_verification};

/// A safety-critical element.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceElement {
    /// The element id.
    pub id: String,
    /// The element type.
    pub element_type: ElementKind,
}

/// The class's element type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// Process equipment.
    ProcessEquipment,
    /// Instrumented function.
    InstrumentedFunction,
    /// Fire protection.
    FireProtection,
    /// Structural.
    Structural,
    /// Utility.
    Utility,
    /// Other.
    Other,
}

impl ElementKind {
    /// Parse the spec's spelling.
    ///
    /// # Errors
    /// `InvalidInput` for an unknown spelling.
    pub fn parse(text: &str) -> Result<Self> {
        match text {
            "process_equipment" => Ok(Self::ProcessEquipment),
            "instrumented_function" => Ok(Self::InstrumentedFunction),
            "fire_protection" => Ok(Self::FireProtection),
            "structural" => Ok(Self::Structural),
            "utility" => Ok(Self::Utility),
            "other" => Ok(Self::Other),
            other => Err(AzothError::invalid_input(
                "elements.fields.element_type",
                format!("`{other}` is not an element type"),
            )),
        }
    }
}

/// An element-barrier link.
#[derive(Debug, Clone, PartialEq)]
pub struct SafetySystemLink {
    /// The element id.
    pub element_id: String,
    /// The barrier id.
    pub barrier_id: String,
}

/// A barrier row.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceBarrier {
    /// The barrier id.
    pub id: String,
    /// The barrier status.
    pub status: PerformanceBarrierStatus,
    /// Whether the barrier or its standard carries traceable evidence.
    pub has_traceable_evidence: bool,
    /// The barrier name.
    pub name: Option<String>,
    /// The barrier description.
    pub description: Option<String>,
    /// The safety function.
    pub safety_function: Option<String>,
    /// The barrier PFD.
    pub pfd: Option<f64>,
    /// The barrier effectiveness; absent uses the class's `1 - pfd` when a PFD is set.
    pub effectiveness: Option<f64>,
    /// The linked performance standard's id.
    pub performance_standard_id: Option<String>,
}

/// The barrier status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PerformanceBarrierStatus {
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

impl PerformanceBarrierStatus {
    /// Parse the spec's spelling.
    ///
    /// # Errors
    /// `InvalidInput` for an unknown spelling.
    pub fn parse(text: &str) -> Result<Self> {
        match text {
            "available" => Ok(Self::Available),
            "impaired" => Ok(Self::Impaired),
            "bypassed" => Ok(Self::Bypassed),
            "out_of_service" => Ok(Self::OutOfService),
            "unknown" => Ok(Self::Unknown),
            other => Err(AzothError::invalid_input(
                "barriers.fields.status",
                format!("`{other}` is not a barrier status"),
            )),
        }
    }

    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Impaired => "impaired",
            Self::Bypassed => "bypassed",
            Self::OutOfService => "out_of_service",
            Self::Unknown => "unknown",
        }
    }
}

/// A barrier equipment tag.
#[derive(Debug, Clone, PartialEq)]
pub struct BarrierEquipmentTag {
    /// The barrier id.
    pub barrier_id: String,
    /// The equipment tag.
    pub tag: String,
}

/// A performance standard row.
#[derive(Debug, Clone, PartialEq)]
pub struct SafetyPerformanceStandard {
    /// The standard id.
    pub id: String,
    /// The standard title.
    pub title: Option<String>,
    /// The standard's safety function.
    pub safety_function: Option<String>,
    /// The target PFD.
    pub target_pfd: Option<f64>,
    /// The required availability.
    pub required_availability: Option<f64>,
    /// The required response time.
    pub response_time_seconds: Option<f64>,
}

/// An acceptance criterion.
#[derive(Debug, Clone, PartialEq)]
pub struct StandardCriterion {
    /// The standard id.
    pub standard_id: String,
    /// The criterion text.
    pub criterion: String,
}

/// A demand case.
#[derive(Debug, Clone, PartialEq)]
pub struct SafetySystemDemandRow {
    /// The demand id.
    pub demand_id: String,
    /// The barrier id.
    pub barrier_id: Option<String>,
    /// The equipment tag.
    pub equipment_tag: Option<String>,
    /// The scenario text.
    pub scenario: Option<String>,
    /// The safety-system category.
    pub category: PerformanceCategory,
    /// The demand value.
    pub demand_value: Option<f64>,
    /// The capacity value.
    pub capacity_value: Option<f64>,
    /// The unit label.
    pub demand_unit: Option<String>,
    /// The required PFD.
    pub required_pfd: Option<f64>,
    /// The actual PFD.
    pub actual_pfd: Option<f64>,
    /// The required availability.
    pub required_availability: Option<f64>,
    /// The actual availability.
    pub actual_availability: Option<f64>,
    /// The required response time.
    pub required_response_time_seconds: Option<f64>,
    /// The actual response time.
    pub actual_response_time_seconds: Option<f64>,
    /// The required effectiveness.
    pub required_effectiveness: Option<f64>,
    /// The actual effectiveness.
    pub actual_effectiveness: Option<f64>,
}

/// The safety-system category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PerformanceCategory {
    /// Firewater and deluge.
    #[serde(rename = "firewater_deluge")]
    FirewaterDeluge,
    /// Fire and gas detection.
    #[serde(rename = "fire_gas_detection")]
    FireGasDetection,
    /// Passive fire protection.
    #[serde(rename = "passive_fire_protection")]
    PassiveFireProtection,
    /// Process shutdown.
    #[serde(rename = "psd")]
    Psd,
    /// Emergency shutdown and blowdown.
    #[serde(rename = "esd_blowdown")]
    EsdBlowdown,
    /// Relief and flare.
    #[serde(rename = "relief_flare")]
    ReliefFlare,
    /// Structural firewall.
    #[serde(rename = "structural_firewall")]
    StructuralFirewall,
    /// High-integrity pressure protection.
    #[serde(rename = "hipps")]
    Hipps,
    /// Unknown.
    #[serde(rename = "unknown")]
    Unknown,
}

impl PerformanceCategory {
    /// Parse the spec's spelling.
    ///
    /// # Errors
    /// `InvalidInput` for an unknown spelling.
    pub fn parse(text: &str) -> Result<Self> {
        match text {
            "firewater_deluge" => Ok(Self::FirewaterDeluge),
            "fire_gas_detection" => Ok(Self::FireGasDetection),
            "passive_fire_protection" => Ok(Self::PassiveFireProtection),
            "psd" => Ok(Self::Psd),
            "esd_blowdown" => Ok(Self::EsdBlowdown),
            "relief_flare" => Ok(Self::ReliefFlare),
            "structural_firewall" => Ok(Self::StructuralFirewall),
            "hipps" => Ok(Self::Hipps),
            "unknown" => Ok(Self::Unknown),
            other => Err(AzothError::invalid_input(
                "demands.fields.category",
                format!("`{other}` is not a safety-system category"),
            )),
        }
    }

    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FirewaterDeluge => "firewater_deluge",
            Self::FireGasDetection => "fire_gas_detection",
            Self::PassiveFireProtection => "passive_fire_protection",
            Self::Psd => "psd",
            Self::EsdBlowdown => "esd_blowdown",
            Self::ReliefFlare => "relief_flare",
            Self::StructuralFirewall => "structural_firewall",
            Self::Hipps => "hipps",
            Self::Unknown => "unknown",
        }
    }
}

/// A measurement device.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementDeviceRow {
    /// The device name.
    pub name: String,
    /// The device kind.
    pub kind: DeviceKind,
    /// The device tag.
    pub tag: Option<String>,
    /// The device location.
    pub location: Option<String>,
    /// The response time; absent uses the class's constructor default.
    pub response_time_seconds: Option<f64>,
}

/// The measurement-device kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// A fire detector.
    FireDetector,
    /// A gas detector.
    GasDetector,
    /// Any other device.
    Other,
}

impl DeviceKind {
    /// Parse the spec's spelling.
    ///
    /// # Errors
    /// `InvalidInput` for an unknown spelling.
    pub fn parse(text: &str) -> Result<Self> {
        match text {
            "fire_detector" => Ok(Self::FireDetector),
            "gas_detector" => Ok(Self::GasDetector),
            "other" => Ok(Self::Other),
            other => Err(AzothError::invalid_input(
                "measurement_devices.fields.kind",
                format!("`{other}` is not a device kind"),
            )),
        }
    }
}

/// An event and voting SIF.
#[derive(Debug, Clone, PartialEq)]
pub struct EventSifRow {
    /// The SIF name.
    pub name: String,
    /// The voting logic.
    pub voting_notation: VotingNotation,
    /// Whether the SIF is overridden.
    pub overridden: bool,
}

/// The voting logic and its required trips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VotingNotation {
    /// 1oo1.
    OneOutOfOne,
    /// 1oo2.
    OneOutOfTwo,
    /// 2oo2.
    TwoOutOfTwo,
    /// 2oo3.
    TwoOutOfThree,
    /// 2oo4.
    TwoOutOfFour,
    /// 3oo4.
    ThreeOutOfFour,
}

impl VotingNotation {
    /// Parse the class's notation.
    ///
    /// # Errors
    /// `InvalidInput` for an unknown notation.
    pub fn parse(text: &str) -> Result<Self> {
        match text {
            "1oo1" => Ok(Self::OneOutOfOne),
            "1oo2" => Ok(Self::OneOutOfTwo),
            "2oo2" => Ok(Self::TwoOutOfTwo),
            "2oo3" => Ok(Self::TwoOutOfThree),
            "2oo4" => Ok(Self::TwoOutOfFour),
            "3oo4" => Ok(Self::ThreeOutOfFour),
            other => Err(AzothError::invalid_input(
                "event_sifs.fields.voting_notation",
                format!("`{other}` is not a voting notation"),
            )),
        }
    }

    /// The class's notation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OneOutOfOne => "1oo1",
            Self::OneOutOfTwo => "1oo2",
            Self::TwoOutOfTwo => "2oo2",
            Self::TwoOutOfThree => "2oo3",
            Self::TwoOutOfFour => "2oo4",
            Self::ThreeOutOfFour => "3oo4",
        }
    }

    /// The trips the logic requires.
    pub fn required_trips(self) -> f64 {
        match self {
            Self::OneOutOfOne | Self::OneOutOfTwo => 1.0,
            Self::TwoOutOfTwo | Self::TwoOutOfThree | Self::TwoOutOfFour => 2.0,
            Self::ThreeOutOfFour => 3.0,
        }
    }

    /// The sensors the logic configures.
    pub fn total_sensors(self) -> f64 {
        match self {
            Self::OneOutOfOne => 1.0,
            Self::OneOutOfTwo | Self::TwoOutOfTwo => 2.0,
            Self::TwoOutOfThree | Self::ThreeOutOfFour => 3.0,
            Self::TwoOutOfFour => 4.0,
        }
    }
}

/// One of an event SIF's detectors.
#[derive(Debug, Clone, PartialEq)]
pub struct SifDetectorRow {
    /// The SIF name.
    pub sif_name: String,
    /// The detector name.
    pub name: String,
    /// Whether the detector is bypassed.
    pub bypassed: bool,
    /// Whether the detector is faulty.
    pub faulty: bool,
    /// Whether the detector is tripped.
    pub tripped: bool,
}

/// A quantitative SIL/PFD SIF.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantitativeSifRow {
    /// The SIF id.
    pub id: String,
    /// The SIF name.
    pub name: Option<String>,
    /// The description.
    pub description: Option<String>,
    /// The architecture.
    pub architecture: String,
    /// The claimed SIL.
    pub claimed_sil: f64,
    /// The average PFD.
    pub pfd_average: f64,
    /// The proof-test interval; absent is the class's 0.
    pub test_interval_hours: Option<f64>,
}

/// One protected-equipment tag.
#[derive(Debug, Clone, PartialEq)]
pub struct ProtectedEquipmentRow {
    /// The SIF id.
    pub sif_id: String,
    /// The equipment tag.
    pub equipment: String,
}

/// The report's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PerformanceVerdict {
    /// Pass.
    #[serde(rename = "pass")]
    Pass,
    /// Pass with warnings.
    #[serde(rename = "pass_with_warnings")]
    PassWithWarnings,
    /// Fail.
    #[serde(rename = "fail")]
    Fail,
}

impl PerformanceVerdict {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::PassWithWarnings => "pass_with_warnings",
            Self::Fail => "fail",
        }
    }
}

/// A finding's severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PerformanceFindingSeverity {
    /// Info.
    #[serde(rename = "info")]
    Info,
    /// Warning.
    #[serde(rename = "warning")]
    Warning,
    /// Fail.
    #[serde(rename = "fail")]
    Fail,
}

impl PerformanceFindingSeverity {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Fail => "fail",
        }
    }
}

/// A finding's stable rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PerformanceRule {
    /// The barrier is not available.
    #[serde(rename = "barrier_not_available")]
    BarrierNotAvailable,
    /// The barrier has no traceable evidence.
    #[serde(rename = "barrier_untraceable")]
    BarrierUntraceable,
    /// No performance standard is linked.
    #[serde(rename = "no_performance_standard")]
    NoPerformanceStandard,
    /// The target PFD has no actual value.
    #[serde(rename = "target_pfd_without_actual")]
    TargetPfdWithoutActual,
    /// The actual PFD exceeds the target.
    #[serde(rename = "pfd_exceeded")]
    PfdExceeded,
    /// The required availability has no actual value.
    #[serde(rename = "required_availability_without_actual")]
    RequiredAvailabilityWithoutActual,
    /// The actual availability is below the required.
    #[serde(rename = "availability_below_required")]
    AvailabilityBelowRequired,
    /// The required response time has no actual value.
    #[serde(rename = "required_response_time_without_actual")]
    RequiredResponseTimeWithoutActual,
    /// The actual response time exceeds the required.
    #[serde(rename = "response_time_exceeded")]
    ResponseTimeExceeded,
    /// The required effectiveness has no actual value.
    #[serde(rename = "required_effectiveness_without_actual")]
    RequiredEffectivenessWithoutActual,
    /// The actual effectiveness is below the required.
    #[serde(rename = "effectiveness_below_required")]
    EffectivenessBelowRequired,
    /// The demand exceeds the documented capacity.
    #[serde(rename = "demand_exceeds_capacity")]
    DemandExceedsCapacity,
    /// No fire or gas detector is linked.
    #[serde(rename = "no_fire_gas_detector")]
    NoFireGasDetector,
    /// The detector response time exceeds the requirement.
    #[serde(rename = "detector_response_time_exceeded")]
    DetectorResponseTimeExceeded,
    /// The SIF is overridden.
    #[serde(rename = "sif_overridden")]
    SifOverridden,
    /// The detector count and the voting logic disagree.
    #[serde(rename = "sif_detector_count_mismatch")]
    SifDetectorCountMismatch,
    /// The healthy channels are insufficient.
    #[serde(rename = "sif_healthy_channels_insufficient")]
    SifHealthyChannelsInsufficient,
    /// The SIF has bypassed or faulty channels.
    #[serde(rename = "sif_channels_impaired")]
    SifChannelsImpaired,
    /// The quantitative SIL is not achieved.
    #[serde(rename = "quantitative_sil_not_achieved")]
    QuantitativeSilNotAchieved,
    /// The quantitative verification raised a warning.
    #[serde(rename = "quantitative_sif_verification_warning")]
    QuantitativeSifVerificationWarning,
    /// A fire-protection barrier has no demand case.
    #[serde(rename = "fire_protection_demand_missing")]
    FireProtectionDemandMissing,
}

impl PerformanceRule {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BarrierNotAvailable => "barrier_not_available",
            Self::BarrierUntraceable => "barrier_untraceable",
            Self::NoPerformanceStandard => "no_performance_standard",
            Self::TargetPfdWithoutActual => "target_pfd_without_actual",
            Self::PfdExceeded => "pfd_exceeded",
            Self::RequiredAvailabilityWithoutActual => "required_availability_without_actual",
            Self::AvailabilityBelowRequired => "availability_below_required",
            Self::RequiredResponseTimeWithoutActual => "required_response_time_without_actual",
            Self::ResponseTimeExceeded => "response_time_exceeded",
            Self::RequiredEffectivenessWithoutActual => "required_effectiveness_without_actual",
            Self::EffectivenessBelowRequired => "effectiveness_below_required",
            Self::DemandExceedsCapacity => "demand_exceeds_capacity",
            Self::NoFireGasDetector => "no_fire_gas_detector",
            Self::DetectorResponseTimeExceeded => "detector_response_time_exceeded",
            Self::SifOverridden => "sif_overridden",
            Self::SifDetectorCountMismatch => "sif_detector_count_mismatch",
            Self::SifHealthyChannelsInsufficient => "sif_healthy_channels_insufficient",
            Self::SifChannelsImpaired => "sif_channels_impaired",
            Self::QuantitativeSilNotAchieved => "quantitative_sil_not_achieved",
            Self::QuantitativeSifVerificationWarning => "quantitative_sif_verification_warning",
            Self::FireProtectionDemandMissing => "fire_protection_demand_missing",
        }
    }
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerformanceFinding {
    /// The barrier id.
    pub barrier_id: String,
    /// The demand id.
    pub demand_id: Option<String>,
    /// The stable rule.
    pub rule: PerformanceRule,
    /// The severity.
    pub severity: PerformanceFindingSeverity,
}

/// One related instrument.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerformanceInstrument {
    /// The barrier id.
    pub barrier_id: String,
    /// The demand id.
    pub demand_id: Option<String>,
    /// The tag, or the name when the tag is absent.
    pub tag: String,
}

/// The SIF family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PerformanceSifKind {
    /// An event and voting SIF.
    #[serde(rename = "event")]
    Event,
    /// A quantitative SIL/PFD SIF.
    #[serde(rename = "quantitative")]
    Quantitative,
}

impl PerformanceSifKind {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Quantitative => "quantitative",
        }
    }
}

/// One related SIF.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerformanceSif {
    /// The barrier id.
    pub barrier_id: String,
    /// The demand id.
    pub demand_id: Option<String>,
    /// The identifier.
    pub identifier: String,
    /// The family.
    pub kind: PerformanceSifKind,
    /// The event SIF's voting notation.
    pub voting_notation: Option<String>,
    /// The event SIF's detector count.
    pub detector_count: Option<f64>,
    /// The event SIF's bypassed detector count.
    pub bypassed_detector_count: Option<f64>,
    /// The event SIF's faulty detector count.
    pub faulty_detector_count: Option<f64>,
    /// The event SIF's tripped detector count.
    pub tripped_detector_count: Option<f64>,
    /// The quantitative SIF's claimed SIL.
    pub claimed_sil: Option<f64>,
    /// The quantitative SIF's achieved SIL.
    pub achieved_sil: Option<f64>,
    /// Whether the achieved SIL meets the claim.
    pub sil_achieved: Option<bool>,
    /// The quantitative SIF's average PFD.
    pub pfd_average: Option<f64>,
    /// `1.5 PFD`.
    pub pfd_upper: Option<f64>,
    /// `1 / PFD`; absent at a zero PFD.
    pub risk_reduction_factor: Option<f64>,
    /// The quantitative SIF's architecture.
    pub architecture: Option<String>,
    /// The quantitative SIF's proof-test interval.
    pub proof_test_interval_hours: Option<f64>,
    /// The number of protected equipment tags.
    pub protected_equipment_count: Option<f64>,
}

/// One assessment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerformanceAssessment {
    /// The barrier id.
    pub barrier_id: String,
    /// The barrier name.
    pub barrier_name: String,
    /// The demand id.
    pub demand_id: Option<String>,
    /// The category.
    pub category: PerformanceCategory,
    /// The verdict.
    pub verdict: PerformanceVerdict,
    /// The barrier status.
    pub barrier_status: PerformanceBarrierStatus,
    /// The barrier PFD.
    pub barrier_pfd: Option<f64>,
    /// The barrier effectiveness.
    pub barrier_effectiveness: Option<f64>,
    /// The evidence flag.
    pub has_traceable_evidence: bool,
    /// The linked standard's id.
    pub performance_standard_id: Option<String>,
    /// The selected target PFD.
    pub target_pfd: Option<f64>,
    /// The selected actual PFD.
    pub actual_pfd: Option<f64>,
    /// The selected required availability.
    pub required_availability: Option<f64>,
    /// The demand's actual availability.
    pub actual_availability: Option<f64>,
    /// The selected required response time.
    pub required_response_time_seconds: Option<f64>,
    /// The demand's actual response time.
    pub actual_response_time_seconds: Option<f64>,
    /// The demand's required effectiveness.
    pub required_effectiveness: Option<f64>,
    /// The selected actual effectiveness.
    pub actual_effectiveness: Option<f64>,
    /// The demand's scenario.
    pub scenario: Option<String>,
    /// The demand value.
    pub demand_value: Option<f64>,
    /// The capacity value.
    pub capacity_value: Option<f64>,
    /// The unit label.
    pub demand_unit: Option<String>,
    /// `capacity - demand`.
    pub capacity_margin: Option<f64>,
    /// The related instrument count.
    pub related_instrument_count: f64,
    /// The largest detector response time.
    pub max_instrument_response_time_seconds: Option<f64>,
    /// The related event SIF count.
    pub related_sif_count: f64,
    /// The related quantitative SIF count.
    pub related_quantitative_sif_count: f64,
    /// The finding count.
    pub finding_count: f64,
}

/// Result of `process.safety_system_performance`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SafetySystemPerformanceResult {
    /// The register id.
    pub register_id: String,
    /// The register name.
    pub register_name: String,
    /// The report's verdict.
    pub overall_verdict: PerformanceVerdict,
    /// The assessment count.
    pub assessment_count: f64,
    /// The pass count.
    pub pass_count: f64,
    /// The pass-with-warnings count.
    pub warning_count: f64,
    /// The fail count.
    pub fail_count: f64,
    /// One row per barrier and matching demand.
    pub assessments: Vec<PerformanceAssessment>,
    /// Every finding.
    pub findings: Vec<PerformanceFinding>,
    /// Every related instrument.
    pub instruments: Vec<PerformanceInstrument>,
    /// Every related SIF.
    pub sifs: Vec<PerformanceSif>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One element with its linked barriers.
struct Element {
    row: PerformanceElement,
    barriers: Vec<String>,
}

/// The register, indexed.
struct Register {
    elements: Vec<Element>,
    barriers: Vec<PerformanceBarrier>,
    standards: Vec<SafetyPerformanceStandard>,
    criteria: Vec<StandardCriterion>,
    tags: Vec<BarrierEquipmentTag>,
    protected: Vec<ProtectedEquipmentRow>,
    devices: Vec<MeasurementDeviceRow>,
    event_sifs: Vec<EventSifRow>,
    detectors: Vec<SifDetectorRow>,
    quantitative_sifs: Vec<QuantitativeSifRow>,
}

fn normalise(text: &str) -> String {
    text.trim().to_lowercase()
}

fn matches_text(value: Option<&str>, pattern: Option<&str>) -> bool {
    let left = normalise(value.unwrap_or(""));
    let right = normalise(pattern.unwrap_or(""));
    if left.is_empty() || right.is_empty() {
        return false;
    }
    left == right || left.contains(&right) || right.contains(&left)
}

fn select_first_finite(first: Option<f64>, second: Option<f64>) -> Option<f64> {
    first
        .filter(|value| value.is_finite())
        .or(second.filter(|value| value.is_finite()))
}

fn push_finding(
    findings: &mut Vec<PerformanceFinding>,
    barrier_id: &str,
    demand_id: Option<&str>,
    rule: PerformanceRule,
    severity: PerformanceFindingSeverity,
) {
    findings.push(PerformanceFinding {
        barrier_id: barrier_id.to_string(),
        demand_id: demand_id.map(str::to_string),
        rule,
        severity,
    });
}

/// The safety-system performance assessment.
///
/// # Errors
/// `InvalidInput` for a row naming an object that is not in the register, or a
/// quantitative SIF `process.sil_verification` refuses.
#[allow(clippy::too_many_arguments)]
pub fn safety_system_performance(
    register_id: &str,
    register_name: Option<&str>,
    elements: Option<&[PerformanceElement]>,
    links: Option<&[SafetySystemLink]>,
    barriers: Option<&[PerformanceBarrier]>,
    barrier_tags: Option<&[BarrierEquipmentTag]>,
    standards: Option<&[SafetyPerformanceStandard]>,
    standard_criteria: Option<&[StandardCriterion]>,
    demands: Option<&[SafetySystemDemandRow]>,
    measurement_devices: Option<&[MeasurementDeviceRow]>,
    event_sifs: Option<&[EventSifRow]>,
    sif_detectors: Option<&[SifDetectorRow]>,
    quantitative_sifs: Option<&[QuantitativeSifRow]>,
    protected_equipment: Option<&[ProtectedEquipmentRow]>,
) -> Result<SafetySystemPerformanceResult> {
    let _spec = &model_gen::SAFETY_SYSTEM_PERFORMANCE_SPEC;
    let register = index(
        elements,
        links,
        barriers,
        barrier_tags,
        standards,
        standard_criteria,
        measurement_devices,
        event_sifs,
        sif_detectors,
        quantitative_sifs,
        protected_equipment,
    )?;

    let mut assessments = Vec::new();
    let mut findings = Vec::new();
    let mut instruments = Vec::new();
    let mut sifs = Vec::new();
    for barrier in &register.barriers {
        let matching: Vec<&SafetySystemDemandRow> = demands
            .unwrap_or(&[])
            .iter()
            .filter(|demand| matches_demand(demand, barrier, &register.tags))
            .collect();
        if matching.is_empty() {
            let assessment = assess(
                barrier,
                None,
                &register,
                &mut findings,
                &mut instruments,
                &mut sifs,
            )?;
            assessments.push(assessment);
        } else {
            for demand in matching {
                let assessment = assess(
                    barrier,
                    Some(demand),
                    &register,
                    &mut findings,
                    &mut instruments,
                    &mut sifs,
                )?;
                assessments.push(assessment);
            }
        }
    }

    let mut overall = PerformanceVerdict::Pass;
    let mut pass_count = 0.0;
    let mut warning_count = 0.0;
    let mut fail_count = 0.0;
    for assessment in &assessments {
        match assessment.verdict {
            PerformanceVerdict::Pass => pass_count += 1.0,
            PerformanceVerdict::PassWithWarnings => {
                warning_count += 1.0;
                if overall == PerformanceVerdict::Pass {
                    overall = PerformanceVerdict::PassWithWarnings;
                }
            }
            PerformanceVerdict::Fail => {
                fail_count += 1.0;
                overall = PerformanceVerdict::Fail;
            }
        }
    }
    Ok(SafetySystemPerformanceResult {
        register_id: register_id.to_string(),
        register_name: register_name.unwrap_or("").to_string(),
        overall_verdict: overall,
        assessment_count: assessments.len() as f64,
        pass_count,
        warning_count,
        fail_count,
        assessments,
        findings,
        instruments,
        sifs,
        warnings: Vec::new(),
    })
}

#[allow(clippy::too_many_arguments)]
fn index(
    elements: Option<&[PerformanceElement]>,
    links: Option<&[SafetySystemLink]>,
    barriers: Option<&[PerformanceBarrier]>,
    barrier_tags: Option<&[BarrierEquipmentTag]>,
    standards: Option<&[SafetyPerformanceStandard]>,
    standard_criteria: Option<&[StandardCriterion]>,
    measurement_devices: Option<&[MeasurementDeviceRow]>,
    event_sifs: Option<&[EventSifRow]>,
    sif_detectors: Option<&[SifDetectorRow]>,
    quantitative_sifs: Option<&[QuantitativeSifRow]>,
    protected_equipment: Option<&[ProtectedEquipmentRow]>,
) -> Result<Register> {
    let mut barrier_rows: Vec<PerformanceBarrier> = Vec::new();
    for row in barriers.unwrap_or(&[]) {
        if let Some(existing) = barrier_rows.iter_mut().find(|item| item.id == row.id) {
            *existing = row.clone();
        } else {
            barrier_rows.push(row.clone());
        }
    }
    let mut standard_rows: Vec<SafetyPerformanceStandard> = Vec::new();
    for row in standards.unwrap_or(&[]) {
        if let Some(existing) = standard_rows.iter_mut().find(|item| item.id == row.id) {
            *existing = row.clone();
        } else {
            standard_rows.push(row.clone());
        }
    }
    let event_rows: Vec<EventSifRow> = event_sifs.unwrap_or(&[]).to_vec();
    let mut element_rows: Vec<Element> = Vec::new();
    for row in elements.unwrap_or(&[]) {
        if let Some(existing) = element_rows.iter_mut().find(|item| item.row.id == row.id) {
            existing.row = row.clone();
        } else {
            element_rows.push(Element {
                row: row.clone(),
                barriers: Vec::new(),
            });
        }
    }
    for link in links.unwrap_or(&[]) {
        if !barrier_rows
            .iter()
            .any(|barrier| barrier.id == link.barrier_id)
        {
            return Err(AzothError::invalid_input(
                "links.barrier_id",
                format!("`{}` names no barrier row", link.barrier_id),
            ));
        }
        let element = element_rows
            .iter_mut()
            .find(|element| element.row.id == link.element_id)
            .ok_or_else(|| {
                AzothError::invalid_input(
                    "links.element_id",
                    format!("`{}` names no element row", link.element_id),
                )
            })?;
        element.barriers.push(link.barrier_id.clone());
    }
    let tags = barrier_tags.unwrap_or(&[]).to_vec();
    for tag in &tags {
        if !barrier_rows
            .iter()
            .any(|barrier| barrier.id == tag.barrier_id)
        {
            return Err(AzothError::invalid_input(
                "barrier_tags.barrier_id",
                format!("`{}` names no barrier row", tag.barrier_id),
            ));
        }
    }
    let criteria = standard_criteria.unwrap_or(&[]).to_vec();
    for criterion in &criteria {
        if !standard_rows
            .iter()
            .any(|standard| standard.id == criterion.standard_id)
        {
            return Err(AzothError::invalid_input(
                "standard_criteria.standard_id",
                format!("`{}` names no standard row", criterion.standard_id),
            ));
        }
    }
    let detectors = sif_detectors.unwrap_or(&[]).to_vec();
    for detector in &detectors {
        if !event_rows.iter().any(|sif| sif.name == detector.sif_name) {
            return Err(AzothError::invalid_input(
                "sif_detectors.sif_name",
                format!("`{}` names no event SIF row", detector.sif_name),
            ));
        }
    }
    let quantitative = quantitative_sifs.unwrap_or(&[]).to_vec();
    let protected = protected_equipment.unwrap_or(&[]).to_vec();
    for row in &protected {
        if !quantitative.iter().any(|sif| sif.id == row.sif_id) {
            return Err(AzothError::invalid_input(
                "protected_equipment.sif_id",
                format!("`{}` names no quantitative SIF row", row.sif_id),
            ));
        }
    }
    Ok(Register {
        elements: element_rows,
        barriers: barrier_rows,
        standards: standard_rows,
        criteria,
        tags,
        protected,
        devices: measurement_devices.unwrap_or(&[]).to_vec(),
        event_sifs: event_rows,
        detectors,
        quantitative_sifs: quantitative,
    })
}

fn barrier_tags<'a>(register: &'a Register, barrier_id: &str) -> Vec<&'a str> {
    register
        .tags
        .iter()
        .filter(|tag| tag.barrier_id == barrier_id)
        .map(|tag| tag.tag.as_str())
        .collect()
}

fn matches_demand(
    demand: &SafetySystemDemandRow,
    barrier: &PerformanceBarrier,
    tags: &[BarrierEquipmentTag],
) -> bool {
    if let Some(id) = &demand.barrier_id {
        if !id.is_empty() && id == &barrier.id {
            return true;
        }
    }
    match &demand.equipment_tag {
        Some(tag) if !tag.is_empty() => tags
            .iter()
            .any(|row| row.barrier_id == barrier.id && &row.tag == tag),
        _ => false,
    }
}

fn standard<'a>(register: &'a Register, id: Option<&str>) -> Option<&'a SafetyPerformanceStandard> {
    let id = id?;
    register.standards.iter().find(|standard| standard.id == id)
}

fn effective_effectiveness(barrier: &PerformanceBarrier) -> Option<f64> {
    barrier.effectiveness.or(barrier.pfd.map(|pfd| 1.0 - pfd))
}

fn category_text(
    register: &Register,
    barrier: &PerformanceBarrier,
    standard: Option<&SafetyPerformanceStandard>,
) -> String {
    let mut text = String::new();
    for part in [
        Some(barrier.id.as_str()),
        barrier.name.as_deref(),
        barrier.description.as_deref(),
        barrier.safety_function.as_deref(),
    ] {
        if let Some(part) = part {
            text.push_str(part);
            text.push(' ');
        }
    }
    if let Some(standard) = standard {
        for part in [
            standard.title.as_deref(),
            standard.safety_function.as_deref(),
        ] {
            if let Some(part) = part {
                text.push_str(part);
                text.push(' ');
            }
        }
        for criterion in &register.criteria {
            if criterion.standard_id == standard.id {
                text.push_str(&criterion.criterion);
                text.push(' ');
            }
        }
    }
    text.to_lowercase()
}

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| text.contains(keyword))
}

fn determine_category(
    register: &Register,
    barrier: &PerformanceBarrier,
    demand: Option<&SafetySystemDemandRow>,
) -> PerformanceCategory {
    if let Some(demand) = demand {
        if demand.category != PerformanceCategory::Unknown {
            return demand.category;
        }
    }
    let standard = standard(register, barrier.performance_standard_id.as_deref());
    let text = category_text(register, barrier, standard);
    if contains_any(
        &text,
        &[
            "deluge",
            "firewater",
            "fire water",
            "sprinkler",
            "water spray",
        ],
    ) {
        return PerformanceCategory::FirewaterDeluge;
    }
    if contains_any(
        &text,
        &[
            "fire detector",
            "gas detector",
            "f&g",
            "fgs",
            "fire and gas",
            "detection",
        ],
    ) {
        return PerformanceCategory::FireGasDetection;
    }
    if contains_any(
        &text,
        &["pfp", "passive fire", "fire proof", "fireproof", "coating"],
    ) {
        return PerformanceCategory::PassiveFireProtection;
    }
    if contains_any(
        &text,
        &["psd", "process shutdown", "shutdown valve", "process trip"],
    ) {
        return PerformanceCategory::Psd;
    }
    if contains_any(&text, &["blowdown", "esd", "emergency shutdown"]) {
        return PerformanceCategory::EsdBlowdown;
    }
    if contains_any(&text, &["psv", "relief", "flare", "vent"]) {
        return PerformanceCategory::ReliefFlare;
    }
    if contains_any(
        &text,
        &["firewall", "fire wall", "blast wall", "structural"],
    ) {
        return PerformanceCategory::StructuralFirewall;
    }
    if contains_any(&text, &["hipps", "high integrity pressure"]) {
        return PerformanceCategory::Hipps;
    }
    for element in &register.elements {
        if !element.barriers.iter().any(|id| id == &barrier.id) {
            continue;
        }
        match element.row.element_type {
            ElementKind::InstrumentedFunction => return PerformanceCategory::EsdBlowdown,
            ElementKind::FireProtection => return PerformanceCategory::FirewaterDeluge,
            ElementKind::Structural => return PerformanceCategory::StructuralFirewall,
            _ => {}
        }
    }
    PerformanceCategory::Unknown
}

fn assess(
    barrier: &PerformanceBarrier,
    demand: Option<&SafetySystemDemandRow>,
    register: &Register,
    findings: &mut Vec<PerformanceFinding>,
    instruments: &mut Vec<PerformanceInstrument>,
    sifs: &mut Vec<PerformanceSif>,
) -> Result<PerformanceAssessment> {
    let demand_id = demand.map(|demand| demand.demand_id.as_str());
    let category = determine_category(register, barrier, demand);
    let tags = barrier_tags(register, &barrier.id);
    let standard = standard(register, barrier.performance_standard_id.as_deref());
    let effectiveness = effective_effectiveness(barrier);

    let mut local: Vec<PerformanceFinding> = Vec::new();
    let mut tags_out: Vec<String> = Vec::new();
    let mut sifs_out: Vec<PerformanceSif> = Vec::new();

    // 1. Credit.
    if barrier.status != PerformanceBarrierStatus::Available {
        push_finding(
            &mut local,
            &barrier.id,
            demand_id,
            PerformanceRule::BarrierNotAvailable,
            PerformanceFindingSeverity::Fail,
        );
    }
    if !barrier.has_traceable_evidence {
        push_finding(
            &mut local,
            &barrier.id,
            demand_id,
            PerformanceRule::BarrierUntraceable,
            PerformanceFindingSeverity::Warning,
        );
    }

    // 2. Performance standard.
    let mut standard_id = None;
    let mut target_pfd = None;
    let mut actual_pfd = None;
    let mut required_availability = None;
    let mut actual_availability = None;
    let mut required_response = None;
    let mut actual_response = None;
    let mut required_effectiveness = None;
    let mut actual_effectiveness = None;
    if let Some(standard) = standard {
        standard_id = Some(standard.id.clone());
        let target = select_first_finite(
            demand.and_then(|demand| demand.required_pfd),
            standard.target_pfd,
        );
        let actual = select_first_finite(demand.and_then(|demand| demand.actual_pfd), barrier.pfd);
        if let Some(target) = target {
            target_pfd = Some(target);
            actual_pfd = actual;
            match actual {
                None => push_finding(
                    &mut local,
                    &barrier.id,
                    demand_id,
                    PerformanceRule::TargetPfdWithoutActual,
                    PerformanceFindingSeverity::Warning,
                ),
                Some(actual) if actual > target => push_finding(
                    &mut local,
                    &barrier.id,
                    demand_id,
                    PerformanceRule::PfdExceeded,
                    PerformanceFindingSeverity::Fail,
                ),
                Some(_) => {}
            }
        }
        let required = select_first_finite(
            demand.and_then(|demand| demand.required_availability),
            standard.required_availability,
        );
        if let Some(required) = required {
            required_availability = Some(required);
            actual_availability = demand.and_then(|demand| demand.actual_availability);
            match actual_availability {
                None => push_finding(
                    &mut local,
                    &barrier.id,
                    demand_id,
                    PerformanceRule::RequiredAvailabilityWithoutActual,
                    PerformanceFindingSeverity::Warning,
                ),
                Some(actual) if actual < required => push_finding(
                    &mut local,
                    &barrier.id,
                    demand_id,
                    PerformanceRule::AvailabilityBelowRequired,
                    PerformanceFindingSeverity::Fail,
                ),
                Some(_) => {}
            }
        }
        let required = select_first_finite(
            demand.and_then(|demand| demand.required_response_time_seconds),
            standard.response_time_seconds,
        );
        if let Some(required) = required {
            required_response = Some(required);
            actual_response = demand.and_then(|demand| demand.actual_response_time_seconds);
            match actual_response {
                None => push_finding(
                    &mut local,
                    &barrier.id,
                    demand_id,
                    PerformanceRule::RequiredResponseTimeWithoutActual,
                    PerformanceFindingSeverity::Warning,
                ),
                Some(actual) if actual > required => push_finding(
                    &mut local,
                    &barrier.id,
                    demand_id,
                    PerformanceRule::ResponseTimeExceeded,
                    PerformanceFindingSeverity::Fail,
                ),
                Some(_) => {}
            }
        }
        if let Some(demand) = demand {
            if let Some(required) = demand.required_effectiveness {
                required_effectiveness = Some(required);
                actual_effectiveness =
                    select_first_finite(demand.actual_effectiveness, effectiveness);
                match actual_effectiveness {
                    None => push_finding(
                        &mut local,
                        &barrier.id,
                        demand_id,
                        PerformanceRule::RequiredEffectivenessWithoutActual,
                        PerformanceFindingSeverity::Warning,
                    ),
                    Some(actual) if actual < required => push_finding(
                        &mut local,
                        &barrier.id,
                        demand_id,
                        PerformanceRule::EffectivenessBelowRequired,
                        PerformanceFindingSeverity::Fail,
                    ),
                    Some(_) => {}
                }
            }
        }
    } else {
        push_finding(
            &mut local,
            &barrier.id,
            demand_id,
            PerformanceRule::NoPerformanceStandard,
            PerformanceFindingSeverity::Warning,
        );
    }

    // 3. Demand capacity.
    let mut scenario = None;
    let mut demand_value = None;
    let mut capacity_value = None;
    let mut demand_unit = None;
    let mut capacity_margin = None;
    if let Some(demand) = demand {
        scenario = demand.scenario.clone().filter(|text| !text.is_empty());
        demand_value = demand.demand_value;
        capacity_value = demand.capacity_value;
        demand_unit = demand.demand_unit.clone().filter(|text| !text.is_empty());
        if let (Some(demand_value), Some(capacity_value)) = (demand_value, capacity_value) {
            let margin = capacity_value - demand_value;
            capacity_margin = Some(margin);
            if margin < 0.0 {
                push_finding(
                    &mut local,
                    &barrier.id,
                    demand_id,
                    PerformanceRule::DemandExceedsCapacity,
                    PerformanceFindingSeverity::Fail,
                );
            }
        }
    }

    // 4. Instrumentation.
    let related: Vec<&MeasurementDeviceRow> = register
        .devices
        .iter()
        .filter(|device| {
            if category == PerformanceCategory::FireGasDetection && device.kind == DeviceKind::Other
            {
                return false;
            }
            tags.iter().any(|tag| {
                matches_text(Some(&device.name), Some(tag))
                    || matches_text(device.tag.as_deref(), Some(tag))
                    || matches_text(device.location.as_deref(), Some(tag))
            })
        })
        .collect();
    let mut max_response: Option<f64> = None;
    for device in &related {
        let identifier = match &device.tag {
            Some(tag) if !tag.trim().is_empty() => tag.clone(),
            _ => device.name.clone(),
        };
        if !tags_out.contains(&identifier) {
            tags_out.push(identifier.clone());
        }
        let response = match device.kind {
            DeviceKind::FireDetector => Some(device.response_time_seconds.unwrap_or(0.0)),
            DeviceKind::GasDetector => Some(device.response_time_seconds.unwrap_or(10.0)),
            DeviceKind::Other => None,
        };
        if let Some(response) = response {
            max_response = Some(match max_response {
                Some(current) => current.max(response),
                None => response,
            });
        }
    }
    if category == PerformanceCategory::FireGasDetection && related.is_empty() {
        push_finding(
            &mut local,
            &barrier.id,
            demand_id,
            PerformanceRule::NoFireGasDetector,
            PerformanceFindingSeverity::Warning,
        );
    }
    if let Some(demand) = demand {
        if demand.actual_response_time_seconds.is_none() {
            if let (Some(required), Some(max)) =
                (demand.required_response_time_seconds, max_response)
            {
                if max > required {
                    push_finding(
                        &mut local,
                        &barrier.id,
                        demand_id,
                        PerformanceRule::DetectorResponseTimeExceeded,
                        PerformanceFindingSeverity::Fail,
                    );
                }
            }
        }
    }
    for id in &tags_out {
        instruments.push(PerformanceInstrument {
            barrier_id: barrier.id.clone(),
            demand_id: demand_id.map(str::to_string),
            tag: id.clone(),
        });
    }

    let event_related: Vec<&EventSifRow> = register
        .event_sifs
        .iter()
        .filter(|sif| {
            matches_text(Some(&sif.name), Some(&barrier.id))
                || matches_text(Some(&sif.name), barrier.name.as_deref())
                || register
                    .detectors
                    .iter()
                    .filter(|detector| detector.sif_name == sif.name)
                    .any(|detector| {
                        tags.iter()
                            .any(|tag| matches_text(Some(&detector.name), Some(tag)))
                    })
        })
        .collect();
    for sif in &event_related {
        let detectors: Vec<&SifDetectorRow> = register
            .detectors
            .iter()
            .filter(|detector| detector.sif_name == sif.name)
            .collect();
        let count = detectors.len() as f64;
        let bypassed = detectors.iter().filter(|row| row.bypassed).count() as f64;
        let faulty = detectors.iter().filter(|row| row.faulty).count() as f64;
        let tripped = detectors.iter().filter(|row| row.tripped).count() as f64;
        if !sifs_out.iter().any(|row| row.identifier == sif.name) {
            sifs_out.push(PerformanceSif {
                barrier_id: barrier.id.clone(),
                demand_id: demand_id.map(str::to_string),
                identifier: sif.name.clone(),
                kind: PerformanceSifKind::Event,
                voting_notation: Some(sif.voting_notation.as_str().to_string()),
                detector_count: Some(count),
                bypassed_detector_count: Some(bypassed),
                faulty_detector_count: Some(faulty),
                tripped_detector_count: Some(tripped),
                claimed_sil: None,
                achieved_sil: None,
                sil_achieved: None,
                pfd_average: None,
                pfd_upper: None,
                risk_reduction_factor: None,
                architecture: None,
                proof_test_interval_hours: None,
                protected_equipment_count: None,
            });
        }
        if sif.overridden {
            push_finding(
                &mut local,
                &barrier.id,
                demand_id,
                PerformanceRule::SifOverridden,
                PerformanceFindingSeverity::Fail,
            );
        }
        if count != sif.voting_notation.total_sensors() {
            push_finding(
                &mut local,
                &barrier.id,
                demand_id,
                PerformanceRule::SifDetectorCountMismatch,
                PerformanceFindingSeverity::Warning,
            );
        }
        let healthy = count - bypassed - faulty;
        if healthy < sif.voting_notation.required_trips() {
            push_finding(
                &mut local,
                &barrier.id,
                demand_id,
                PerformanceRule::SifHealthyChannelsInsufficient,
                PerformanceFindingSeverity::Fail,
            );
        } else if bypassed > 0.0 || faulty > 0.0 {
            push_finding(
                &mut local,
                &barrier.id,
                demand_id,
                PerformanceRule::SifChannelsImpaired,
                PerformanceFindingSeverity::Warning,
            );
        }
    }

    let quantitative_related: Vec<&QuantitativeSifRow> = register
        .quantitative_sifs
        .iter()
        .filter(|sif| {
            matches_text(Some(&sif.id), Some(&barrier.id))
                || matches_text(sif.name.as_deref(), Some(&barrier.id))
                || matches_text(sif.name.as_deref(), barrier.name.as_deref())
                || matches_text(
                    sif.description.as_deref(),
                    barrier.safety_function.as_deref(),
                )
                || register
                    .protected
                    .iter()
                    .filter(|row| row.sif_id == sif.id)
                    .any(|row| {
                        tags.iter()
                            .any(|tag| matches_text(Some(&row.equipment), Some(tag)))
                    })
        })
        .collect();
    for sif in &quantitative_related {
        let identifier = match &sif.name {
            Some(name) if !name.trim().is_empty() => name.trim().to_string(),
            _ => {
                if !sif.id.trim().is_empty() {
                    sif.id.trim().to_string()
                } else {
                    "quantitativeSIF".to_string()
                }
            }
        };
        let interval = sif.test_interval_hours.unwrap_or(0.0);
        let verification = sil_verification(
            sif.pfd_average,
            sif.claimed_sil,
            &sif.architecture,
            interval,
        )?;
        let protected_count = register
            .protected
            .iter()
            .filter(|row| row.sif_id == sif.id)
            .count() as f64;
        if !sifs_out.iter().any(|row| row.identifier == identifier) {
            sifs_out.push(PerformanceSif {
                barrier_id: barrier.id.clone(),
                demand_id: demand_id.map(str::to_string),
                identifier: identifier.clone(),
                kind: PerformanceSifKind::Quantitative,
                voting_notation: None,
                detector_count: None,
                bypassed_detector_count: None,
                faulty_detector_count: None,
                tripped_detector_count: None,
                claimed_sil: Some(verification.claimed_sil),
                achieved_sil: Some(verification.achieved_sil),
                sil_achieved: Some(verification.sil_achieved),
                pfd_average: Some(verification.pfd_average),
                pfd_upper: Some(verification.pfd_upper),
                risk_reduction_factor: if sif.pfd_average > 0.0 {
                    Some(1.0 / sif.pfd_average)
                } else {
                    None
                },
                architecture: Some(sif.architecture.clone()),
                proof_test_interval_hours: Some(interval),
                protected_equipment_count: Some(protected_count),
            });
        }
        if !verification.sil_achieved || verification.error_count > 0.0 {
            push_finding(
                &mut local,
                &barrier.id,
                demand_id,
                PerformanceRule::QuantitativeSilNotAchieved,
                PerformanceFindingSeverity::Fail,
            );
        }
        for issue in &verification.issues {
            if issue.severity == IssueSeverity::Warning {
                push_finding(
                    &mut local,
                    &barrier.id,
                    demand_id,
                    PerformanceRule::QuantitativeSifVerificationWarning,
                    PerformanceFindingSeverity::Warning,
                );
            }
        }
    }
    let related_sif_count = event_related.len() as f64;
    let related_quantitative_count = quantitative_related.len() as f64;
    for row in &sifs_out {
        sifs.push(row.clone());
    }

    // 5. Demand completeness.
    if demand.is_none()
        && matches!(
            category,
            PerformanceCategory::FirewaterDeluge | PerformanceCategory::PassiveFireProtection
        )
    {
        push_finding(
            &mut local,
            &barrier.id,
            demand_id,
            PerformanceRule::FireProtectionDemandMissing,
            PerformanceFindingSeverity::Warning,
        );
    }

    let verdict = if local
        .iter()
        .any(|finding| finding.severity == PerformanceFindingSeverity::Fail)
    {
        PerformanceVerdict::Fail
    } else if local
        .iter()
        .any(|finding| finding.severity == PerformanceFindingSeverity::Warning)
    {
        PerformanceVerdict::PassWithWarnings
    } else {
        PerformanceVerdict::Pass
    };
    let finding_count = local.len() as f64;
    findings.extend(local);
    Ok(PerformanceAssessment {
        barrier_id: barrier.id.clone(),
        barrier_name: barrier.name.clone().unwrap_or_default(),
        demand_id: demand_id.map(str::to_string),
        category,
        verdict,
        barrier_status: barrier.status,
        barrier_pfd: barrier.pfd,
        barrier_effectiveness: effectiveness,
        has_traceable_evidence: barrier.has_traceable_evidence,
        performance_standard_id: standard_id,
        target_pfd,
        actual_pfd,
        required_availability,
        actual_availability,
        required_response_time_seconds: required_response,
        actual_response_time_seconds: actual_response,
        required_effectiveness,
        actual_effectiveness,
        scenario,
        demand_value,
        capacity_value,
        demand_unit,
        capacity_margin,
        related_instrument_count: related.len() as f64,
        max_instrument_response_time_seconds: max_response,
        related_sif_count,
        related_quantitative_sif_count: related_quantitative_count,
        finding_count,
    })
}
