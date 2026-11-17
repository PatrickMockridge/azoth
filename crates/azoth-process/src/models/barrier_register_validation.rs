//! `process.barrier_register_validation` - the barrier register's rule review.
//!
//! Spec: `specs/models/process/barrier_register_validation.toml`. The rules are NeqSim's
//! `process/safety/barrier/BarrierRegister.java` (`validate`) and its
//! `SafetyBarrier`, `PerformanceStandard` and `SafetyCriticalElement` `validate` methods.

use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::safety_review::{
    BarrierElementType, BarrierKind, BarrierStatus, EvidenceOwner, FindingScope, RegisterRule,
};

/// One safety-critical element a caller states.
#[derive(Debug, Clone, PartialEq)]
pub struct BarrierElement {
    /// The element id.
    pub id: String,
    /// The equipment tag.
    pub tag: Option<String>,
    /// The element name.
    pub name: Option<String>,
    /// The element type, the spec's spelling.
    pub element_type: String,
}

/// One barrier a caller states.
#[derive(Debug, Clone, PartialEq)]
pub struct BarrierRow {
    /// The barrier id.
    pub id: String,
    /// The barrier name.
    pub name: Option<String>,
    /// The barrier description.
    pub description: Option<String>,
    /// The barrier type, the spec's spelling.
    pub barrier_type: String,
    /// The status, the spec's spelling.
    pub status: String,
    /// The PFD, absent for the class's NaN.
    pub pfd: Option<f64>,
    /// The effectiveness, absent for the class's NaN.
    pub effectiveness: Option<f64>,
    /// The linked performance standard's id; absent is the class's null.
    pub performance_standard_id: Option<String>,
    /// The number of linked equipment tags.
    pub equipment_tag_count: f64,
}

/// One performance standard a caller states.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceStandardRow {
    /// The standard id.
    pub id: String,
    /// The safety-function description.
    pub safety_function: Option<String>,
    /// The target PFD.
    pub target_pfd: Option<f64>,
    /// The required availability.
    pub required_availability: Option<f64>,
    /// The proof-test interval in hours.
    pub proof_test_interval_hours: Option<f64>,
    /// The number of acceptance criteria.
    pub acceptance_criteria_count: f64,
}

/// One element-to-barrier link.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementBarrierLink {
    /// The element id.
    pub element_id: String,
    /// The barrier id.
    pub barrier_id: String,
}

/// One document-evidence row.
#[derive(Debug, Clone, PartialEq)]
pub struct RegisterEvidence {
    /// The owner kind, the spec's spelling.
    pub owner_kind: String,
    /// The owner's id.
    pub owner_id: String,
    /// The source document identifier.
    pub document_id: Option<String>,
    /// The source reference.
    pub source_reference: Option<String>,
    /// The supporting excerpt.
    pub excerpt: Option<String>,
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegisterFinding {
    /// Which record the finding is about.
    pub scope: FindingScope,
    /// The record's id.
    pub subject_id: String,
    /// The rule the row violates.
    pub rule: RegisterRule,
}

/// Result of `process.barrier_register_validation`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BarrierRegisterValidationResult {
    /// One row per finding, in the class's order.
    pub findings: Vec<RegisterFinding>,
    /// The number of findings.
    pub finding_count: f64,
    /// Whether the register has no findings.
    pub valid: bool,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

fn trimmed(value: &str) -> &str {
    value.trim()
}

fn trimmed_opt(value: Option<&String>) -> &str {
    value.map_or("", |text| text.trim())
}

fn non_empty(value: &str) -> bool {
    !trimmed(value).is_empty()
}

fn traceable(rows: &[(EvidenceOwner, String, bool)], owner: EvidenceOwner, id: &str) -> bool {
    rows.iter()
        .filter(|(kind, owner_id, _)| *kind == owner && owner_id == id)
        .any(|(_, _, traceable)| *traceable)
}

struct Registry {
    elements: Vec<(String, BarrierElement)>,
    barriers: Vec<(String, BarrierRow)>,
    standards: Vec<(String, PerformanceStandardRow)>,
}

impl Registry {
    fn element(&self, id: &str) -> Option<&BarrierElement> {
        self.elements
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, value)| value)
    }

    fn barrier(&self, id: &str) -> Option<&BarrierRow> {
        self.barriers
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, value)| value)
    }

    fn standard(&self, id: &str) -> Option<&PerformanceStandardRow> {
        self.standards
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, value)| value)
    }
}

fn upsert<T>(items: &mut Vec<(String, T)>, id: String, value: T) {
    match items.iter_mut().find(|(key, _)| key == &id) {
        Some((_, slot)) => *slot = value,
        None => items.push((id, value)),
    }
}

fn push_finding(
    findings: &mut Vec<RegisterFinding>,
    scope: FindingScope,
    subject_id: &str,
    rule: RegisterRule,
) {
    findings.push(RegisterFinding {
        scope,
        subject_id: subject_id.to_string(),
        rule,
    });
}

fn validate_standard(
    standard_id: &str,
    standard: &PerformanceStandardRow,
    standard_traceable: bool,
    findings: &mut Vec<RegisterFinding>,
) {
    if standard_id.is_empty() {
        push_finding(
            findings,
            FindingScope::Standard,
            standard_id,
            RegisterRule::StandardIdMissing,
        );
    }
    if trimmed_opt(standard.safety_function.as_ref()).is_empty() {
        push_finding(
            findings,
            FindingScope::Standard,
            standard_id,
            RegisterRule::StandardSafetyFunctionMissing,
        );
    }
    if let Some(target) = standard.target_pfd {
        if !target.is_nan() && (target <= 0.0 || target > 1.0) {
            push_finding(
                findings,
                FindingScope::Standard,
                standard_id,
                RegisterRule::StandardTargetPfdOutOfRange,
            );
        }
    }
    if let Some(availability) = standard.required_availability {
        if !availability.is_nan() && !(0.0..=1.0).contains(&availability) {
            push_finding(
                findings,
                FindingScope::Standard,
                standard_id,
                RegisterRule::StandardRequiredAvailabilityOutOfRange,
            );
        }
    }
    if let Some(interval) = standard.proof_test_interval_hours {
        if !interval.is_nan() && interval <= 0.0 {
            push_finding(
                findings,
                FindingScope::Standard,
                standard_id,
                RegisterRule::StandardProofTestIntervalNotPositive,
            );
        }
    }
    if standard.acceptance_criteria_count < 1.0 {
        push_finding(
            findings,
            FindingScope::Standard,
            standard_id,
            RegisterRule::StandardNoAcceptanceCriteria,
        );
    }
    if !standard_traceable {
        push_finding(
            findings,
            FindingScope::Standard,
            standard_id,
            RegisterRule::StandardNoTraceableEvidence,
        );
    }
}

fn validate_barrier(
    registry: &Registry,
    barrier_id: &str,
    barrier: &BarrierRow,
    evidence: &[(EvidenceOwner, String, bool)],
    findings: &mut Vec<RegisterFinding>,
) {
    if barrier_id.is_empty() {
        push_finding(
            findings,
            FindingScope::Barrier,
            barrier_id,
            RegisterRule::BarrierIdMissing,
        );
    }
    if trimmed_opt(barrier.name.as_ref()).is_empty()
        && trimmed_opt(barrier.description.as_ref()).is_empty()
    {
        push_finding(
            findings,
            FindingScope::Barrier,
            barrier_id,
            RegisterRule::BarrierNameOrDescriptionMissing,
        );
    }
    if let Some(pfd) = barrier.pfd {
        if !pfd.is_nan() && !(0.0..=1.0).contains(&pfd) {
            push_finding(
                findings,
                FindingScope::Barrier,
                barrier_id,
                RegisterRule::BarrierPfdOutOfRange,
            );
        }
    }
    if let Some(effectiveness) = barrier.effectiveness {
        if !effectiveness.is_nan() && !(0.0..=1.0).contains(&effectiveness) {
            push_finding(
                findings,
                FindingScope::Barrier,
                barrier_id,
                RegisterRule::BarrierEffectivenessOutOfRange,
            );
        }
    }
    if barrier.status == "unknown" {
        push_finding(
            findings,
            FindingScope::Barrier,
            barrier_id,
            RegisterRule::BarrierStatusUnknown,
        );
    }
    if barrier.equipment_tag_count < 1.0 {
        push_finding(
            findings,
            FindingScope::Barrier,
            barrier_id,
            RegisterRule::BarrierNoEquipmentTag,
        );
    }
    let standard_id = trimmed_opt(barrier.performance_standard_id.as_ref());
    let standard = registry.standard(standard_id);
    match standard {
        None => push_finding(
            findings,
            FindingScope::Barrier,
            barrier_id,
            RegisterRule::BarrierNoPerformanceStandard,
        ),
        Some(standard) => validate_standard(
            standard_id,
            standard,
            traceable(evidence, EvidenceOwner::Standard, standard_id),
            findings,
        ),
    }
    let own = traceable(evidence, EvidenceOwner::Barrier, barrier_id);
    let through_standard =
        standard.is_some_and(|_| traceable(evidence, EvidenceOwner::Standard, standard_id));
    if !(own || through_standard) {
        push_finding(
            findings,
            FindingScope::Barrier,
            barrier_id,
            RegisterRule::BarrierNoTraceableEvidence,
        );
    }
}

/// The register's findings.
///
/// # Errors
/// `InvalidInput` for an unknown enumeration value, a link naming an unknown element or
/// barrier, or a negative count.
pub fn barrier_register_validation(
    register_id: &str,
    elements: Option<&[BarrierElement]>,
    barriers: Option<&[BarrierRow]>,
    standards: Option<&[PerformanceStandardRow]>,
    links: Option<&[ElementBarrierLink]>,
    evidence: Option<&[RegisterEvidence]>,
) -> Result<BarrierRegisterValidationResult> {
    let spec = &model_gen::BARRIER_REGISTER_VALIDATION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(spec.input_checks(), |_| None, &mut warnings)?;

    let mut registry = Registry {
        elements: Vec::new(),
        barriers: Vec::new(),
        standards: Vec::new(),
    };
    for row in elements.unwrap_or(&[]) {
        BarrierElementType::parse(&row.element_type)?;
        let id = trimmed(&row.id).to_string();
        upsert(
            &mut registry.elements,
            id,
            BarrierElement {
                id: row.id.clone(),
                tag: row.tag.clone(),
                name: row.name.clone(),
                element_type: row.element_type.clone(),
            },
        );
    }
    for row in barriers.unwrap_or(&[]) {
        BarrierKind::parse(&row.barrier_type)?;
        BarrierStatus::parse(&row.status)?;
        if !row.equipment_tag_count.is_finite() || row.equipment_tag_count < 0.0 {
            return Err(AzothError::invalid_input(
                "equipment_tag_count",
                "the count must be finite and at least zero",
            ));
        }
        upsert(
            &mut registry.barriers,
            trimmed(&row.id).to_string(),
            row.clone(),
        );
    }
    for row in standards.unwrap_or(&[]) {
        if !row.acceptance_criteria_count.is_finite() || row.acceptance_criteria_count < 0.0 {
            return Err(AzothError::invalid_input(
                "acceptance_criteria_count",
                "the count must be finite and at least zero",
            ));
        }
        upsert(
            &mut registry.standards,
            trimmed(&row.id).to_string(),
            row.clone(),
        );
    }

    // The links, grouped by element in caller order and deduplicated per element.
    let mut element_links: Vec<(String, Vec<String>)> = Vec::new();
    for (index, link) in links.unwrap_or(&[]).iter().enumerate() {
        let element_id = trimmed(&link.element_id);
        let barrier_id = trimmed(&link.barrier_id);
        if registry.element(element_id).is_none() {
            return Err(AzothError::invalid_input(
                format!("links[{index}].element_id"),
                format!("`{element_id}` names no element row"),
            ));
        }
        if registry.barrier(barrier_id).is_none() {
            return Err(AzothError::invalid_input(
                format!("links[{index}].barrier_id"),
                format!("`{barrier_id}` names no barrier row"),
            ));
        }
        match element_links.iter_mut().find(|(key, _)| key == element_id) {
            Some((_, barrier_ids)) => {
                if !barrier_ids.iter().any(|id| id == barrier_id) {
                    barrier_ids.push(barrier_id.to_string());
                }
            }
            None => element_links.push((element_id.to_string(), vec![barrier_id.to_string()])),
        }
    }

    // Evidence traceability, by owner.
    let mut evidence_rows: Vec<(EvidenceOwner, String, bool)> = Vec::new();
    for row in evidence.unwrap_or(&[]) {
        let owner = EvidenceOwner::parse(&row.owner_kind)?;
        let is_traceable = (non_empty(row.document_id.as_deref().unwrap_or(""))
            || non_empty(row.source_reference.as_deref().unwrap_or("")))
            && non_empty(row.excerpt.as_deref().unwrap_or(""));
        evidence_rows.push((owner, trimmed(&row.owner_id).to_string(), is_traceable));
    }

    let mut findings = Vec::new();
    let register_id = trimmed(register_id).to_string();
    if register_id.is_empty() {
        push_finding(
            &mut findings,
            FindingScope::Register,
            &register_id,
            RegisterRule::RegisterIdMissing,
        );
    }
    if registry.elements.is_empty() && registry.barriers.is_empty() {
        push_finding(
            &mut findings,
            FindingScope::Register,
            &register_id,
            RegisterRule::RegisterEmpty,
        );
    }

    for (element_id, element) in &registry.elements {
        if element_id.is_empty() {
            push_finding(
                &mut findings,
                FindingScope::Element,
                element_id,
                RegisterRule::ElementIdMissing,
            );
        }
        if trimmed_opt(element.tag.as_ref()).is_empty()
            && trimmed_opt(element.name.as_ref()).is_empty()
        {
            push_finding(
                &mut findings,
                FindingScope::Element,
                element_id,
                RegisterRule::ElementTagOrNameMissing,
            );
        }
        let linked: &[String] = element_links
            .iter()
            .find(|(key, _)| key == element_id)
            .map(|(_, barrier_ids)| barrier_ids.as_slice())
            .unwrap_or(&[]);
        if linked.is_empty() {
            push_finding(
                &mut findings,
                FindingScope::Element,
                element_id,
                RegisterRule::ElementNoBarriers,
            );
        }
        let own = traceable(&evidence_rows, EvidenceOwner::Element, element_id);
        let through_barrier = linked.iter().any(|barrier_id| {
            let own_barrier = traceable(&evidence_rows, EvidenceOwner::Barrier, barrier_id);
            let standard_id = registry
                .barrier(barrier_id)
                .map(|barrier| trimmed_opt(barrier.performance_standard_id.as_ref()))
                .unwrap_or("");
            own_barrier || traceable(&evidence_rows, EvidenceOwner::Standard, standard_id)
        });
        if !(own || through_barrier) {
            push_finding(
                &mut findings,
                FindingScope::Element,
                element_id,
                RegisterRule::ElementNoTraceableEvidence,
            );
        }
        for barrier_id in linked {
            if let Some(barrier) = registry.barrier(barrier_id) {
                validate_barrier(
                    &registry,
                    barrier_id,
                    barrier,
                    &evidence_rows,
                    &mut findings,
                );
            }
        }
    }

    for (barrier_id, _) in &registry.barriers {
        let linked = element_links
            .iter()
            .any(|(_, barrier_ids)| barrier_ids.contains(barrier_id));
        if !linked {
            push_finding(
                &mut findings,
                FindingScope::Barrier,
                barrier_id,
                RegisterRule::BarrierNotLinkedToElement,
            );
        }
    }

    Ok(BarrierRegisterValidationResult {
        finding_count: findings.len() as f64,
        valid: findings.is_empty(),
        findings,
        warnings,
    })
}
