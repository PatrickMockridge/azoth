//! `process.open_drain_review` - the NORSOK S-001 open-drain review.
//!
//! Spec: `specs/models/process/open_drain_review.toml`. The arithmetic is NeqSim's
//! `process/safety/opendrain/OpenDrainReviewEngine.java`.

use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;
use crate::models::review_evidence::{
    Evidence, EvidenceValue, contains_ignore_case, get_bool, get_bool_default, get_double,
    get_string_list, has_any,
};

/// One review item.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenDrainItem {
    /// The area identifier.
    pub area_id: String,
    /// The free-form area type.
    pub area_type: Option<String>,
    /// The free-form drain system type.
    pub drain_system_type: Option<String>,
}

/// One evidence row.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenDrainEvidence {
    /// The area the evidence belongs to.
    pub area_id: String,
    /// The evidence key.
    pub key: String,
    /// The value, absent for the class's null.
    pub value: Option<String>,
}

/// One design-standard row.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenDrainStandard {
    /// The area the standard belongs to.
    pub area_id: String,
    /// The standard text.
    pub standard: String,
}

/// One reference row.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenDrainReference {
    /// The area the reference belongs to.
    pub area_id: String,
    /// The reference text.
    pub reference: String,
}

/// An assessment's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OpenDrainStatus {
    /// Pass.
    #[serde(rename = "pass")]
    Pass,
    /// Warning.
    #[serde(rename = "warning")]
    Warning,
    /// Fail.
    #[serde(rename = "fail")]
    Fail,
    /// Info.
    #[serde(rename = "info")]
    Info,
    /// Not applicable.
    #[serde(rename = "not_applicable")]
    NotApplicable,
}

impl OpenDrainStatus {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warning => "warning",
            Self::Fail => "fail",
            Self::Info => "info",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// An assessment's severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OpenDrainSeverity {
    /// Low.
    #[serde(rename = "low")]
    Low,
    /// Medium.
    #[serde(rename = "medium")]
    Medium,
    /// High.
    #[serde(rename = "high")]
    High,
    /// Critical.
    #[serde(rename = "critical")]
    Critical,
}

impl OpenDrainSeverity {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// An item's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OpenDrainVerdict {
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

impl OpenDrainVerdict {
    /// The spec's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::PassWithWarnings => "pass_with_warnings",
            Self::Fail => "fail",
        }
    }
}

/// One assessment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpenDrainAssessment {
    /// The item the assessment belongs to.
    pub area_id: String,
    /// The stable requirement identifier.
    pub requirement_id: String,
    /// The clause.
    pub clause: String,
    /// The status.
    pub status: OpenDrainStatus,
    /// The severity.
    pub severity: OpenDrainSeverity,
}

/// One item's result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpenDrainResult {
    /// The item's area id.
    pub area_id: String,
    /// The item's verdict.
    pub verdict: OpenDrainVerdict,
    /// The item's evidence confidence.
    pub confidence: f64,
}

/// Result of `process.open_drain_review`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpenDrainReviewResult {
    /// The report's verdict.
    pub overall_verdict: OpenDrainVerdict,
    /// The number of reviewed items.
    pub item_count: f64,
    /// The number of failing items.
    pub failed_item_count: f64,
    /// The number of warning items.
    pub warning_item_count: f64,
    /// The total number of assessments.
    pub assessment_count: f64,
    /// The number of failing assessments.
    pub fail_assessment_count: f64,
    /// The number of warning assessments.
    pub warning_assessment_count: f64,
    /// One row per item.
    pub results: Vec<OpenDrainResult>,
    /// Every assessment.
    pub assessments: Vec<OpenDrainAssessment>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One item's evidence and references.
struct Item {
    row: OpenDrainItem,
    evidence: Evidence,
    references: Vec<String>,
}

/// The open-drain review.
///
/// # Errors
/// `InvalidInput` for a duplicate area id, a duplicate evidence key, or an evidence,
/// standard or reference row naming no item.
pub fn open_drain_review(
    project_name: Option<&str>,
    default_liquid_leak_rate: Option<f64>,
    items: Option<&[OpenDrainItem]>,
    evidence: Option<&[OpenDrainEvidence]>,
    design_standards: Option<&[OpenDrainStandard]>,
    references: Option<&[OpenDrainReference]>,
) -> Result<OpenDrainReviewResult> {
    let _spec = &model_gen::OPEN_DRAIN_REVIEW_SPEC;
    let _project = project_name.unwrap_or("open-drain-review");
    let default_leak = default_liquid_leak_rate.unwrap_or(5.0);
    let mut reviewed: Vec<Item> = Vec::new();
    for row in items.unwrap_or(&[]) {
        if reviewed.iter().any(|item| item.row.area_id == row.area_id) {
            return Err(AzothError::invalid_input(
                "items.area_id",
                format!("`{}` appears more than once", row.area_id),
            ));
        }
        reviewed.push(Item {
            row: row.clone(),
            evidence: Evidence::new(),
            references: Vec::new(),
        });
    }
    for row in evidence.unwrap_or(&[]) {
        let item = reviewed
            .iter_mut()
            .find(|item| item.row.area_id == row.area_id)
            .ok_or_else(|| {
                AzothError::invalid_input(
                    "evidence.area_id",
                    format!("`{}` names no item row", row.area_id),
                )
            })?;
        if item.evidence.contains_key(&row.key) {
            return Err(AzothError::invalid_input(
                "evidence.key",
                format!("`{}` appears more than once for `{}`", row.key, row.area_id),
            ));
        }
        item.evidence.insert(
            row.key.clone(),
            match &row.value {
                Some(value) => EvidenceValue::Text(value.clone()),
                None => EvidenceValue::Null,
            },
        );
    }
    for row in design_standards.unwrap_or(&[]) {
        let item = reviewed
            .iter_mut()
            .find(|item| item.row.area_id == row.area_id)
            .ok_or_else(|| {
                AzothError::invalid_input(
                    "design_standards.area_id",
                    format!("`{}` names no item row", row.area_id),
                )
            })?;
        match item
            .evidence
            .entry("designStandards".to_string())
            .or_insert_with(|| EvidenceValue::List(Vec::new()))
        {
            EvidenceValue::List(values) => values.push(row.standard.clone()),
            other => {
                *other = EvidenceValue::List(vec![row.standard.clone()]);
            }
        }
    }
    for row in references.unwrap_or(&[]) {
        let item = reviewed
            .iter_mut()
            .find(|item| item.row.area_id == row.area_id)
            .ok_or_else(|| {
                AzothError::invalid_input(
                    "references.area_id",
                    format!("`{}` names no item row", row.area_id),
                )
            })?;
        item.references.push(row.reference.clone());
    }

    let mut results = Vec::with_capacity(reviewed.len());
    let mut assessments = Vec::new();
    for item in &reviewed {
        let mut rows = Vec::new();
        evaluate_item(item, default_leak, &mut rows);
        let confidence = confidence(item);
        let verdict = verdict(&rows);
        results.push(OpenDrainResult {
            area_id: item.row.area_id.clone(),
            verdict,
            confidence,
        });
        assessments.extend(rows);
    }
    let overall_verdict = if results
        .iter()
        .any(|row| row.verdict == OpenDrainVerdict::Fail)
    {
        OpenDrainVerdict::Fail
    } else if results
        .iter()
        .any(|row| row.verdict == OpenDrainVerdict::PassWithWarnings)
    {
        OpenDrainVerdict::PassWithWarnings
    } else {
        OpenDrainVerdict::Pass
    };
    Ok(OpenDrainReviewResult {
        overall_verdict,
        item_count: results.len() as f64,
        failed_item_count: results
            .iter()
            .filter(|row| row.verdict == OpenDrainVerdict::Fail)
            .count() as f64,
        warning_item_count: results
            .iter()
            .filter(|row| row.verdict == OpenDrainVerdict::PassWithWarnings)
            .count() as f64,
        assessment_count: assessments.len() as f64,
        fail_assessment_count: assessments
            .iter()
            .filter(|row| row.status == OpenDrainStatus::Fail)
            .count() as f64,
        warning_assessment_count: assessments
            .iter()
            .filter(|row| row.status == OpenDrainStatus::Warning)
            .count() as f64,
        results,
        assessments,
        warnings: Vec::new(),
    })
}

fn push(
    rows: &mut Vec<OpenDrainAssessment>,
    item: &Item,
    requirement_id: &str,
    clause: &str,
    status: OpenDrainStatus,
    severity: OpenDrainSeverity,
) {
    rows.push(OpenDrainAssessment {
        area_id: item.row.area_id.clone(),
        requirement_id: requirement_id.to_string(),
        clause: clause.to_string(),
        status,
        severity,
    });
}

fn required_bool(
    rows: &mut Vec<OpenDrainAssessment>,
    item: &Item,
    requirement_id: &str,
    clause: &str,
    key: &str,
) {
    let severity = match get_bool(&item.evidence, &[key]) {
        None => (OpenDrainStatus::Warning, OpenDrainSeverity::High),
        Some(false) => (OpenDrainStatus::Fail, OpenDrainSeverity::Critical),
        Some(true) => (OpenDrainStatus::Pass, OpenDrainSeverity::Low),
    };
    push(rows, item, requirement_id, clause, severity.0, severity.1);
}

fn evaluate_item(item: &Item, default_leak: f64, rows: &mut Vec<OpenDrainAssessment>) {
    // 1. Design-standard evidence.
    let standards = get_string_list(
        &item.evidence,
        &["designStandards", "standards", "drainageSystemStandards"],
    );
    let combined = standards.join(" ").to_lowercase();
    if standards.is_empty() {
        push(
            rows,
            item,
            "OD-S001-9.4.1-STANDARDS",
            "9.4.1",
            OpenDrainStatus::Warning,
            OpenDrainSeverity::Medium,
        );
    } else if combined.contains("p-002") && combined.contains("13702") {
        push(
            rows,
            item,
            "OD-S001-9.4.1-STANDARDS",
            "9.4.1",
            OpenDrainStatus::Pass,
            OpenDrainSeverity::Low,
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.1-STANDARDS",
            "9.4.1",
            OpenDrainStatus::Warning,
            OpenDrainSeverity::Medium,
        );
    }

    // 2. Spill collection.
    if !requires_drainage_review(item) {
        push(
            rows,
            item,
            "OD-S001-9.4.1-COLLECTION",
            "9.4.1",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    } else {
        required_bool(
            rows,
            item,
            "OD-S001-9.4.1-COLLECTION",
            "9.4.1",
            "hasOpenDrainMeasures",
        );
    }

    // 3. Capacity.
    let fire_water = get_double(
        &item.evidence,
        0.0,
        &[
            "fireWaterCapacityKgPerS",
            "fireWaterRateKgPerS",
            "delugeLoadKgPerS",
        ],
    );
    let leak_rate = get_double(
        &item.evidence,
        default_leak,
        &[
            "liquidLeakRateKgPerS",
            "worstCredibleLeakKgPerS",
            "processFireLeakKgPerS",
        ],
    );
    let required = fire_water.max(0.0) + leak_rate.max(0.0);
    let capacity_keys = [
        "drainageCapacityKgPerS",
        "openDrainCapacityKgPerS",
        "capacityKgPerS",
    ];
    if !has_any(&item.evidence, &capacity_keys) {
        push(
            rows,
            item,
            "OD-S001-9.4.1-CAPACITY",
            "9.4.1",
            OpenDrainStatus::Warning,
            OpenDrainSeverity::High,
        );
    } else {
        let capacity = get_double(&item.evidence, f64::NAN, &capacity_keys);
        if capacity + 1.0e-12 < required {
            push(
                rows,
                item,
                "OD-S001-9.4.1-CAPACITY",
                "9.4.1",
                OpenDrainStatus::Fail,
                OpenDrainSeverity::Critical,
            );
        } else {
            push(
                rows,
                item,
                "OD-S001-9.4.1-CAPACITY",
                "9.4.1",
                OpenDrainStatus::Pass,
                OpenDrainSeverity::Low,
            );
        }
    }

    // 4. Backflow.
    required_bool(
        rows,
        item,
        "OD-S001-9.4.1-BACKFLOW",
        "9.4.1",
        "backflowPrevented",
    );

    // 5. Bunding.
    if get_bool_default(
        &item.evidence,
        false,
        &[
            "hasFlammableLiquidTanks",
            "tankArea",
            "flammableLiquidVesselsPresent",
        ],
    ) {
        required_bool(
            rows,
            item,
            "OD-S001-9.4.1-BUNDING",
            "9.4.1",
            "bundingAndDrainageAdequate",
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.1-BUNDING",
            "9.4.1",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    }

    // 6. Floater.
    if get_bool_default(
        &item.evidence,
        false,
        &["isFloater", "floatingFacility", "floater"],
    ) {
        required_bool(
            rows,
            item,
            "OD-S001-9.4.1-FLOATER",
            "9.4.1",
            "preventsCrossAreaSpreading",
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.1-FLOATER",
            "9.4.1",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    }

    // 7. Hazardous drain tank inerting.
    let tank = get_bool_default(
        &item.evidence,
        false,
        &[
            "hazardousAreaCollectionTank",
            "hazardousDrainTank",
            "collectionTankInHazardousService",
        ],
    ) || (get_bool_default(&item.evidence, false, &["hazardousArea"])
        && get_bool_default(&item.evidence, false, &["collectionTankPresent"]));
    if tank {
        required_bool(
            rows,
            item,
            "OD-S001-9.4.2-INERTING",
            "9.4.2",
            "purgedOrInerted",
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.2-INERTING",
            "9.4.2",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    }

    // 8. Closed/open interaction.
    required_bool(
        rows,
        item,
        "OD-S001-9.4.2-CLOSED-OPEN",
        "9.4.2",
        "closedOpenDrainInteractionPrevented",
    );

    // 9. Hazardous/non-hazardous separation.
    let separated = get_bool(
        &item.evidence,
        &["hazardousNonHazardousPhysicallySeparated"],
    );
    let combined = get_bool(
        &item.evidence,
        &[
            "hazardousNonHazardousCombined",
            "hazardousNonHazardousShareSump",
        ],
    );
    if combined == Some(true) && separated != Some(true) {
        push(
            rows,
            item,
            "OD-S001-9.4.2-HAZ-NONHAZ",
            "9.4.2",
            OpenDrainStatus::Fail,
            OpenDrainSeverity::Critical,
        );
    } else if separated.is_none() && combined.is_none() {
        push(
            rows,
            item,
            "OD-S001-9.4.2-HAZ-NONHAZ",
            "9.4.2",
            OpenDrainStatus::Warning,
            OpenDrainSeverity::High,
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.2-HAZ-NONHAZ",
            "9.4.2",
            OpenDrainStatus::Pass,
            OpenDrainSeverity::Low,
        );
    }

    // 10. Common sump.
    if get_bool_default(
        &item.evidence,
        false,
        &["commonCaissonOrSump", "sharedCaisson", "sharedSump"],
    ) {
        required_bool(
            rows,
            item,
            "OD-S001-9.4.2-COMMON-SUMP",
            "9.4.2",
            "nonHazardousBackflowPrevented",
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.2-COMMON-SUMP",
            "9.4.2",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    }

    // 11-13. Seals, vent and fire seals.
    required_bool(
        rows,
        item,
        "OD-S001-9.4.2-SEAL-BACKPRESSURE",
        "9.4.2",
        "sealDesignedForMaxBackpressure",
    );
    required_bool(
        rows,
        item,
        "OD-S001-9.4.2-VENT",
        "9.4.2",
        "ventTerminatedSafe",
    );
    if get_bool_default(
        &item.evidence,
        false,
        &["multipleFireAreas", "crossesFireAreas", "fireAreaInterface"],
    ) {
        required_bool(
            rows,
            item,
            "OD-S001-9.4.2-FIRE-SEALS",
            "9.4.2",
            "liquidFireSealsBetweenFireAreas",
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.2-FIRE-SEALS",
            "9.4.2",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    }

    // 14. Helideck.
    if is_helideck(item) {
        required_bool(
            rows,
            item,
            "OD-S001-9.4.2-HELIDECK",
            "9.4.2",
            "dedicatedPipeDrainage",
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.2-HELIDECK",
            "9.4.2",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    }

    // 15. Drilling/production separation.
    if get_bool_default(&item.evidence, false, &["drillingArea", "drillingDrain"])
        || contains_ignore_case(item.row.area_type.as_deref().unwrap_or(""), "drilling")
    {
        match get_bool(
            &item.evidence,
            &[
                "drillingProductionOpenDrainConnected",
                "connectedToProductionOpenDrain",
            ],
        ) {
            None => push(
                rows,
                item,
                "OD-S001-9.4.2-DRILLING-PROD",
                "9.4.2",
                OpenDrainStatus::Warning,
                OpenDrainSeverity::High,
            ),
            Some(true) => push(
                rows,
                item,
                "OD-S001-9.4.2-DRILLING-PROD",
                "9.4.2",
                OpenDrainStatus::Fail,
                OpenDrainSeverity::Critical,
            ),
            Some(false) => push(
                rows,
                item,
                "OD-S001-9.4.2-DRILLING-PROD",
                "9.4.2",
                OpenDrainStatus::Pass,
                OpenDrainSeverity::Low,
            ),
        }
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.2-DRILLING-PROD",
            "9.4.2",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    }

    // 16. Temporary storage.
    if get_bool_default(
        &item.evidence,
        false,
        &["temporaryFlammableLiquidStorage", "toteTankStorage"],
    ) {
        required_bool(
            rows,
            item,
            "OD-S001-9.4.2-TOTE",
            "9.4.2",
            "adequateCollectionDrainForTote",
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.2-TOTE",
            "9.4.2",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    }

    // 17. Fire division.
    if get_bool_default(
        &item.evidence,
        false,
        &[
            "drainBoxInFireDivision",
            "drainPipePenetratesFireDivision",
            "fireDivisionPenetration",
        ],
    ) {
        required_bool(
            rows,
            item,
            "OD-S001-9.4.3-FIRE-DIVISION",
            "9.4.3",
            "fireDivisionIntegrityMaintained",
        );
    } else {
        push(
            rows,
            item,
            "OD-S001-9.4.3-FIRE-DIVISION",
            "9.4.3",
            OpenDrainStatus::NotApplicable,
            OpenDrainSeverity::Low,
        );
    }

    // 18. Utility independence.
    match get_bool(
        &item.evidence,
        &[
            "openDrainDependsOnUtility",
            "dependsOnUtility",
            "requiresUtilityForDrainage",
        ],
    ) {
        None => push(
            rows,
            item,
            "OD-S001-9.4.1-UTILITY",
            "9.4.1",
            OpenDrainStatus::Info,
            OpenDrainSeverity::Low,
        ),
        Some(true) => push(
            rows,
            item,
            "OD-S001-9.4.1-UTILITY",
            "9.4.1",
            OpenDrainStatus::Fail,
            OpenDrainSeverity::High,
        ),
        Some(false) => push(
            rows,
            item,
            "OD-S001-9.4.1-UTILITY",
            "9.4.1",
            OpenDrainStatus::Pass,
            OpenDrainSeverity::Low,
        ),
    }

    // 19. Operational evidence.
    let operational_keys = [
        "observedBackflowEvents",
        "sumpHighLevelEvents",
        "observedBackpressureBara",
        "maximumAllowedBackpressureBara",
        "tagreaderSource",
    ];
    if !has_any(&item.evidence, &operational_keys) {
        push(
            rows,
            item,
            "OD-OPS-TAGREADER",
            "Operational evidence",
            OpenDrainStatus::Info,
            OpenDrainSeverity::Low,
        );
    } else {
        let backflow = get_double(&item.evidence, 0.0, &["observedBackflowEvents"]);
        let high_level = get_double(&item.evidence, 0.0, &["sumpHighLevelEvents"]);
        let observed = get_double(&item.evidence, f64::NAN, &["observedBackpressureBara"]);
        let maximum = get_double(
            &item.evidence,
            f64::NAN,
            &["maximumAllowedBackpressureBara"],
        );
        if backflow > 0.0 {
            push(
                rows,
                item,
                "OD-OPS-TAGREADER",
                "Operational evidence",
                OpenDrainStatus::Fail,
                OpenDrainSeverity::High,
            );
        } else if !observed.is_nan() && !maximum.is_nan() && observed > maximum {
            push(
                rows,
                item,
                "OD-OPS-TAGREADER",
                "Operational evidence",
                OpenDrainStatus::Fail,
                OpenDrainSeverity::High,
            );
        } else if high_level > 0.0 {
            push(
                rows,
                item,
                "OD-OPS-TAGREADER",
                "Operational evidence",
                OpenDrainStatus::Warning,
                OpenDrainSeverity::Medium,
            );
        } else {
            push(
                rows,
                item,
                "OD-OPS-TAGREADER",
                "Operational evidence",
                OpenDrainStatus::Pass,
                OpenDrainSeverity::Low,
            );
        }
    }
}

fn requires_drainage_review(item: &Item) -> bool {
    if let Some(source) = get_bool(
        &item.evidence,
        &[
            "sourceHasFlammableOrHazardousLiquid",
            "flammableOrHazardousLiquidSource",
        ],
    ) {
        return source;
    }
    let searchable = format!(
        "{} {}",
        item.row.area_type.as_deref().unwrap_or(""),
        item.row.drain_system_type.as_deref().unwrap_or("")
    )
    .to_lowercase();
    [
        "process",
        "hazard",
        "hydrocarbon",
        "tank",
        "well",
        "drilling",
        "helideck",
        "storage",
    ]
    .iter()
    .any(|token| searchable.contains(token))
        || searchable.trim().is_empty()
}

fn is_helideck(item: &Item) -> bool {
    get_bool_default(&item.evidence, false, &["helideckDrain", "helideckArea"])
        || contains_ignore_case(item.row.area_type.as_deref().unwrap_or(""), "helideck")
        || contains_ignore_case(&item.row.area_id, "helideck")
}

fn confidence(item: &Item) -> f64 {
    let mut score = 0.20_f64;
    if !item.references.is_empty() {
        score += 0.20;
    }
    if has_any(
        &item.evidence,
        &[
            "drainageCapacityKgPerS",
            "openDrainCapacityKgPerS",
            "capacityKgPerS",
        ],
    ) {
        score += 0.15;
    }
    if has_any(
        &item.evidence,
        &[
            "hasOpenDrainMeasures",
            "backflowPrevented",
            "closedOpenDrainInteractionPrevented",
        ],
    ) {
        score += 0.20;
    }
    if has_any(
        &item.evidence,
        &["designStandards", "standards", "drainageSystemStandards"],
    ) {
        score += 0.10;
    }
    if has_any(
        &item.evidence,
        &[
            "observedBackflowEvents",
            "sumpHighLevelEvents",
            "tagreaderSource",
        ],
    ) {
        score += 0.15;
    }
    score.min(1.0)
}

fn verdict(rows: &[OpenDrainAssessment]) -> OpenDrainVerdict {
    if rows.iter().any(|row| row.status == OpenDrainStatus::Fail) {
        OpenDrainVerdict::Fail
    } else if rows
        .iter()
        .any(|row| row.status == OpenDrainStatus::Warning)
    {
        OpenDrainVerdict::PassWithWarnings
    } else {
        OpenDrainVerdict::Pass
    }
}
