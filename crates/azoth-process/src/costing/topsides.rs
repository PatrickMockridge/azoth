//! The topsides facility stack over process cost rows.
//!
//! `TopsidesFacilityCostEstimator` consumes a `ProcessCostEstimate`; the port
//! consumes the same per-unit rows. The module classification is the class's
//! ordered `contains` chain over the process type.

use crate::costing::records::MtoItem;

/// The facility-type factor set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FacilityType {
    /// Fixed offshore platform.
    FixedPlatform,
    /// Floating production, storage and offloading.
    Fpso,
    /// Semi-submersible or tension-leg platform.
    SemiSubmersible,
    /// Onshore processing facility.
    Onshore,
    /// Brownfield host tie-in.
    BrownfieldTieIn,
}

impl FacilityType {
    /// The display name.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::FixedPlatform => "Fixed platform",
            Self::Fpso => "FPSO",
            Self::SemiSubmersible => "Semi-submersible",
            Self::Onshore => "Onshore",
            Self::BrownfieldTieIn => "Brownfield tie-in",
        }
    }

    /// Module integration, installation, hook-up, structural-weight and utility factors.
    #[must_use]
    pub fn factors(self) -> [f64; 5] {
        match self {
            Self::FixedPlatform => [0.18, 0.20, 0.12, 0.90, 0.18],
            Self::Fpso => [0.22, 0.14, 0.10, 0.70, 0.22],
            Self::SemiSubmersible => [0.24, 0.18, 0.12, 0.80, 0.22],
            Self::Onshore => [0.08, 0.02, 0.04, 0.35, 0.25],
            Self::BrownfieldTieIn => [0.26, 0.12, 0.20, 0.45, 0.12],
        }
    }

    /// Parse the spec spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "fixed_platform" => Self::FixedPlatform,
            "fpso" => Self::Fpso,
            "semi_submersible" => Self::SemiSubmersible,
            "onshore" => Self::Onshore,
            "brownfield_tie_in" => Self::BrownfieldTieIn,
            _ => return None,
        })
    }

    /// The spec spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FixedPlatform => "fixed_platform",
            Self::Fpso => "fpso",
            Self::SemiSubmersible => "semi_submersible",
            Self::Onshore => "onshore",
            Self::BrownfieldTieIn => "brownfield_tie_in",
        }
    }
}

/// The project execution context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectContext {
    /// New facility.
    Greenfield,
    /// Brownfield modification.
    BrownfieldModification,
    /// Host tie-in.
    HostTieIn,
}

impl ProjectContext {
    /// The display name.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Greenfield => "Greenfield",
            Self::BrownfieldModification => "Brownfield modification",
            Self::HostTieIn => "Host tie-in",
        }
    }

    /// The congestion factor.
    #[must_use]
    pub fn congestion_factor(self) -> f64 {
        match self {
            Self::Greenfield => 1.0,
            Self::BrownfieldModification => 1.35,
            Self::HostTieIn => 1.50,
        }
    }

    /// Parse the spec spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "greenfield" => Self::Greenfield,
            "brownfield_modification" => Self::BrownfieldModification,
            "host_tie_in" => Self::HostTieIn,
            _ => return None,
        })
    }

    /// The spec spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Greenfield => "greenfield",
            Self::BrownfieldModification => "brownfield_modification",
            Self::HostTieIn => "host_tie_in",
        }
    }
}

/// The factor inputs, the class's nine defaults.
#[derive(Clone, Copy, Debug)]
pub struct TopsidesFactors {
    /// Piping bulk factor.
    pub piping_bulk_factor: f64,
    /// Electrical and instrumentation factor.
    pub electrical_instrumentation_factor: f64,
    /// Insulation, paint and fireproofing factor.
    pub insulation_paint_fireproofing_factor: f64,
    /// HVAC, telecom and safety factor.
    pub hvac_telecom_safety_factor: f64,
    /// Engineering and procurement factor.
    pub engineering_procurement_factor: f64,
    /// Construction management factor.
    pub construction_management_factor: f64,
    /// Project contingency factor.
    pub project_contingency_factor: f64,
    /// Owner cost factor.
    pub owner_cost_factor: f64,
    /// Whether utilities and offsites are included.
    pub include_utilities_and_offsites: bool,
}

impl Default for TopsidesFactors {
    fn default() -> Self {
        Self {
            piping_bulk_factor: 0.25,
            electrical_instrumentation_factor: 0.16,
            insulation_paint_fireproofing_factor: 0.06,
            hvac_telecom_safety_factor: 0.08,
            engineering_procurement_factor: 0.14,
            construction_management_factor: 0.06,
            project_contingency_factor: 0.15,
            owner_cost_factor: 0.10,
            include_utilities_and_offsites: true,
        }
    }
}

/// One equipment row for the topsides stack.
#[derive(Clone, Debug)]
pub struct TopsidesEquipmentRow {
    /// The unit name.
    pub name: String,
    /// The process type the roll-up classified, e.g. "Vessels".
    pub equipment_type: String,
    /// The unit's total module cost, unlocated USD.
    pub total_module_cost: f64,
    /// The unit's dry weight, kg.
    pub weight: f64,
}

/// The topsides inputs.
#[derive(Clone, Debug)]
pub struct TopsidesInput {
    /// The equipment rows.
    pub equipment: Vec<TopsidesEquipmentRow>,
    /// The facility type.
    pub facility_type: FacilityType,
    /// The project context.
    pub project_context: ProjectContext,
    /// The location factor.
    pub location_factor: f64,
    /// The factor inputs.
    pub factors: TopsidesFactors,
}

/// The topsides result.
#[derive(Clone, Debug, Default)]
pub struct TopsidesResult {
    /// The ten capital-cost lines.
    pub capital_costs: Vec<(String, f64)>,
    /// The summary lines.
    pub capital_cost_summary: Vec<(String, f64)>,
    /// The module breakdown lines, keyed `module.<camelCase>`.
    pub capital_cost_breakdown: Vec<(String, f64)>,
    /// The project lines.
    pub project_costs: Vec<(String, f64)>,
    /// The project summary.
    pub project_cost_summary: Vec<(String, f64)>,
    /// The weights.
    pub weight_basis: Vec<(String, f64)>,
    /// The material-take-off rows.
    pub material_take_off: Vec<MtoItem>,
    /// Quality flags.
    pub quality_flags: Vec<String>,
}

fn classify_module(equipment_type: &str) -> &'static str {
    if equipment_type.contains("Compressor") || equipment_type.contains("Expander") {
        "Compression and Rotating Equipment"
    } else if equipment_type.contains("Pump") {
        "Pumping and Utility Packages"
    } else if equipment_type.contains("Heat Exchanger") {
        "Heat Transfer and Cooling"
    } else if equipment_type.contains("Valve")
        || equipment_type.contains("Piping")
        || equipment_type.contains("Pipeline")
    {
        "Piping and Flow Control"
    } else if equipment_type.contains("Column")
        || equipment_type.contains("Vessel")
        || equipment_type.contains("Tank")
    {
        "Separation and Treatment Modules"
    } else {
        "Miscellaneous Process Packages"
    }
}

fn excluded_from_topsides(equipment_type: &str) -> bool {
    equipment_type.contains("Well") || equipment_type.contains("Reservoir")
}

fn normalize_key(label: &str) -> String {
    let parts: Vec<&str> = label
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return "unknown".to_string();
    }
    let mut key = {
        let mut chars = parts[0].chars();
        let first = chars.next().map(|c| c.to_ascii_lowercase()).unwrap_or('u');
        std::iter::once(first).chain(chars).collect::<String>()
    };
    for part in &parts[1..] {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            key.push(first.to_ascii_uppercase());
        }
        key.push_str(chars.as_str());
    }
    key
}

fn add_to_map(rows: &mut Vec<(String, f64)>, key: &str, value: f64) {
    match rows.iter_mut().find(|(name, _)| name == key) {
        Some((_, current)) => *current += value,
        None => rows.push((key.to_string(), value)),
    }
}

/// The topsides estimate over the stated process rows.
#[must_use]
pub fn estimate(input: &TopsidesInput) -> TopsidesResult {
    let mut result = TopsidesResult::default();
    if input.equipment.is_empty() {
        result.quality_flags.push(
            "No equipment cost summaries were available; state at least one unit.".to_string(),
        );
    }
    let [
        module_integration,
        installation,
        hook_up,
        structural_weight_factor,
        utility_offsite,
    ] = input.facility_type.factors();
    let congestion = input.project_context.congestion_factor();
    let factors = input.factors;

    let mut module_costs: Vec<(String, f64)> = Vec::new();
    let mut equipment_weight = 0.0;
    let mut process_module_cost = 0.0;
    let mut excluded_scope_cost = 0.0;

    for row in &input.equipment {
        if excluded_from_topsides(&row.equipment_type) {
            excluded_scope_cost += row.total_module_cost;
            result.quality_flags.push(format!(
                "Excluded non-topsides item from topsides module build-up: {}",
                row.name
            ));
            continue;
        }
        let located = row.total_module_cost * input.location_factor;
        let module = classify_module(&row.equipment_type);
        add_to_map(&mut module_costs, module, located);
        process_module_cost += located;
        equipment_weight += row.weight.max(0.0);
        result.material_take_off.push(MtoItem {
            item: row.name.clone(),
            material: Some(row.equipment_type.clone()),
            quantity: Some(row.weight),
            unit: Some("kg".to_string()),
            weight_kg: Some(row.weight),
            cost_usd: located,
            source: Some("process-cost-estimate".to_string()),
        });
        if row.weight <= 0.0 && row.total_module_cost > 0.0 {
            result.quality_flags.push(format!(
                "Equipment cost has no positive weight basis: {}",
                row.name
            ));
        }
    }

    for (module, value) in &module_costs {
        result
            .capital_cost_breakdown
            .push((format!("module.{}", normalize_key(module)), *value));
    }

    let structural_weight = equipment_weight * structural_weight_factor;
    let piping_weight = equipment_weight * 0.35;
    let structural_steel = structural_weight * 10.0 * input.location_factor;
    let piping_bulk = process_module_cost * factors.piping_bulk_factor;
    let electrical_instrumentation =
        process_module_cost * factors.electrical_instrumentation_factor;
    let insulation = process_module_cost * factors.insulation_paint_fireproofing_factor;
    let hvac = process_module_cost * factors.hvac_telecom_safety_factor;
    let utility = if factors.include_utilities_and_offsites {
        process_module_cost * utility_offsite
    } else {
        0.0
    };
    let module_integration_cost = process_module_cost * module_integration;
    let bulk =
        structural_steel + piping_bulk + electrical_instrumentation + insulation + hvac + utility;
    let direct_fabricated = process_module_cost + bulk + module_integration_cost;
    let offshore_installation = direct_fabricated * installation * congestion;
    let hook_up_cost = direct_fabricated * hook_up * congestion;
    let direct_field = direct_fabricated + offshore_installation + hook_up_cost;
    let engineering = direct_field * factors.engineering_procurement_factor;
    let construction = direct_field * factors.construction_management_factor;
    let contingency = direct_field * factors.project_contingency_factor;
    let owner = direct_field * factors.owner_cost_factor;
    let total = direct_field + engineering + construction + contingency + owner;

    result.capital_costs = vec![
        ("processEquipmentModules".to_string(), process_module_cost),
        ("structuralSteel".to_string(), structural_steel),
        ("pipingBulk".to_string(), piping_bulk),
        (
            "electricalAndInstrumentation".to_string(),
            electrical_instrumentation,
        ),
        ("insulationPaintFireproofing".to_string(), insulation),
        ("hvacTelecomAndSafetySystems".to_string(), hvac),
        ("utilitiesAndOffsites".to_string(), utility),
        (
            "moduleFabricationAndIntegration".to_string(),
            module_integration_cost,
        ),
        ("offshoreInstallation".to_string(), offshore_installation),
        ("hookUpAndCommissioning".to_string(), hook_up_cost),
    ];
    result.capital_cost_summary = vec![
        ("directFieldCost".to_string(), direct_field),
        (
            "excludedNonTopsidesScope".to_string(),
            excluded_scope_cost * input.location_factor,
        ),
    ];
    result.project_costs = vec![
        (
            "engineeringProcurementProjectManagement".to_string(),
            engineering,
        ),
        ("constructionManagement".to_string(), construction),
        ("projectContingency".to_string(), contingency),
        ("ownerCost".to_string(), owner),
    ];
    result
        .project_cost_summary
        .push(("totalTopsidesCapex".to_string(), total));

    result.weight_basis = vec![
        ("equipmentDryWeight".to_string(), equipment_weight),
        ("structuralSteelWeight".to_string(), structural_weight),
        ("pipingBulkWeight".to_string(), piping_weight),
        (
            "totalEstimatedDryWeight".to_string(),
            equipment_weight + structural_weight + piping_weight,
        ),
    ];

    result.material_take_off.push(MtoItem {
        item: "Structural steel allowance".to_string(),
        material: Some("Carbon Steel".to_string()),
        quantity: Some(structural_weight),
        unit: Some("kg".to_string()),
        weight_kg: Some(structural_weight),
        cost_usd: structural_steel,
        source: Some("topsides-allowance".to_string()),
    });
    result.material_take_off.push(MtoItem {
        item: "Piping bulk allowance".to_string(),
        material: Some("Mixed piping classes".to_string()),
        quantity: Some(piping_weight),
        unit: Some("kg".to_string()),
        weight_kg: Some(piping_weight),
        cost_usd: piping_bulk,
        source: Some("topsides-allowance".to_string()),
    });
    result.material_take_off.push(MtoItem {
        item: "Electrical and instrumentation allowance".to_string(),
        material: Some("Cable, trays, instruments".to_string()),
        quantity: Some(1.0),
        unit: Some("allowance".to_string()),
        weight_kg: None,
        cost_usd: electrical_instrumentation,
        source: Some("topsides-allowance".to_string()),
    });
    result.material_take_off.push(MtoItem {
        item: "HVAC, telecom, fire and gas allowance".to_string(),
        material: Some("Package allowance".to_string()),
        quantity: Some(1.0),
        unit: Some("allowance".to_string()),
        weight_kg: None,
        cost_usd: hvac,
        source: Some("topsides-allowance".to_string()),
    });

    result.quality_flags.push(format!(
        "Facility type factor set: {}",
        input.facility_type.display_name()
    ));
    result.quality_flags.push(format!(
        "Project context factor set: {}",
        input.project_context.display_name()
    ));
    if process_module_cost <= 0.0 {
        result
            .quality_flags
            .push("No positive topsides process module cost was calculated.".to_string());
    }
    if equipment_weight <= 0.0 {
        result.quality_flags.push(
            "No positive equipment dry weight was available; bulk MTO weights are allowance based."
                .to_string(),
        );
    }
    if factors.include_utilities_and_offsites {
        result.quality_flags.push(
            "Utilities and offsites are factor allowances unless explicit utility modules are included."
                .to_string(),
        );
    }
    if input.project_context != ProjectContext::Greenfield {
        result.quality_flags.push(
            "Brownfield or host tie-in congestion factor applied; verify with site-specific survey data."
                .to_string(),
        );
    }
    result
}
