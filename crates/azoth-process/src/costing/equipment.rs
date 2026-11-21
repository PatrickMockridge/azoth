//! The twenty-one equipment estimators and their dispatcher.
//!
//! Each estimator answers the same question with a different purchased-cost
//! correlation; the stack above the purchased cost is shared. The class's own
//! quirks are carried: the material factor is applied inside several estimators
//! and again in the module stack, the deprecated weight-based vessel routes
//! remain, and a zero or negative capacity answers zero.

use crate::costing::correlations::{
    air_cooler, bubble_cap_trays, centrifugal_compressor, centrifugal_pump, control_valve,
    horizontal_vessel_by_volume, horizontal_vessel_by_weight, packing, plate_heat_exchanger,
    reciprocating_compressor, shell_tube_heat_exchanger, sieve_trays, valve_trays,
    vertical_vessel_by_volume,
};
use crate::costing::factors::{COST_PER_WEIGHT_UNIT, cepci_ratio as ratio_of, pressure_factor};
use crate::costing::module_stack::{self, ModuleStack};
use crate::costing::records::MtoItem;

/// The twenty-one equipment kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A separator vessel.
    Separator,
    /// A reactor vessel with catalyst.
    Reactor,
    /// A distillation column.
    Column,
    /// An absorber.
    Absorber,
    /// A compressor.
    Compressor,
    /// A pump.
    Pump,
    /// An expander.
    Expander,
    /// A shell-and-tube, plate or air-cooled exchanger.
    HeatExchanger,
    /// A brazed aluminium heat exchanger.
    Bahx,
    /// A storage tank.
    Tank,
    /// A filter vessel.
    Filter,
    /// A production manifold.
    Manifold,
    /// A mercury-removal bed.
    MercuryRemoval,
    /// An electrolyzer.
    Electrolyzer,
    /// A pressure-swing adsorption cascade.
    Psa,
    /// An ejector.
    Ejector,
    /// A pipe.
    Pipe,
    /// A valve.
    Valve,
    /// A mixer.
    Mixer,
    /// A splitter.
    Splitter,
    /// A well.
    Well,
}

impl Kind {
    /// The spec spelling of the kind.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Separator => "separator",
            Self::Reactor => "reactor",
            Self::Column => "column",
            Self::Absorber => "absorber",
            Self::Compressor => "compressor",
            Self::Pump => "pump",
            Self::Expander => "expander",
            Self::HeatExchanger => "heat_exchanger",
            Self::Bahx => "bahx",
            Self::Tank => "tank",
            Self::Filter => "filter",
            Self::Manifold => "manifold",
            Self::MercuryRemoval => "mercury_removal",
            Self::Electrolyzer => "electrolyzer",
            Self::Psa => "psa",
            Self::Ejector => "ejector",
            Self::Pipe => "pipe",
            Self::Valve => "valve",
            Self::Mixer => "mixer",
            Self::Splitter => "splitter",
            Self::Well => "well",
        }
    }

    /// The kind for a spec spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "separator" => Self::Separator,
            "reactor" => Self::Reactor,
            "column" => Self::Column,
            "absorber" => Self::Absorber,
            "compressor" => Self::Compressor,
            "pump" => Self::Pump,
            "expander" => Self::Expander,
            "heat_exchanger" => Self::HeatExchanger,
            "bahx" => Self::Bahx,
            "tank" => Self::Tank,
            "filter" => Self::Filter,
            "manifold" => Self::Manifold,
            "mercury_removal" => Self::MercuryRemoval,
            "electrolyzer" => Self::Electrolyzer,
            "psa" => Self::Psa,
            "ejector" => Self::Ejector,
            "pipe" => Self::Pipe,
            "valve" => Self::Valve,
            "mixer" => Self::Mixer,
            "splitter" => Self::Splitter,
            "well" => Self::Well,
            _ => return None,
        })
    }

    /// The class's `equipmentType` string, which selects the labour rate.
    #[must_use]
    pub fn equipment_type(self) -> &'static str {
        match self {
            Self::Separator | Self::MercuryRemoval => "vessel",
            Self::Reactor => "reactor",
            Self::Column => "column",
            Self::Absorber => "absorber",
            Self::Compressor => "compressor",
            Self::Pump => "pump",
            Self::Expander => "expander",
            Self::HeatExchanger => "exchanger",
            Self::Bahx => "BAHX",
            Self::Tank => "tank",
            Self::Filter => "filter",
            Self::Manifold => "manifold",
            Self::Electrolyzer => "electrolyzer",
            Self::Psa => "psa-cascade",
            Self::Ejector => "ejector",
            Self::Pipe => "piping",
            Self::Valve => "valve",
            Self::Mixer => "mixer",
            Self::Splitter => "splitter",
            Self::Well => "well",
        }
    }

    /// Whether the class's `getTotalCost` returns the total module cost alone.
    #[must_use]
    pub fn total_cost_is_module_only(self) -> bool {
        matches!(
            self,
            Self::Separator
                | Self::Column
                | Self::Compressor
                | Self::Pump
                | Self::HeatExchanger
                | Self::MercuryRemoval
                | Self::Pipe
                | Self::Valve
        )
    }
}

/// The design-independent basis every estimator shares.
#[derive(Clone, Copy, Debug)]
pub struct Common {
    /// The calculator's material factor.
    pub material_factor: f64,
    /// The current CEPCI.
    pub cepci: f64,
    /// Whether a mechanical design is present; without one the class computes
    /// the purchased cost and no module stack.
    pub design_present: bool,
    /// The mechanical design's maximum design pressure, barg, as the class reads it.
    pub max_design_pressure_barg: f64,
    /// The equipment's total weight, kg.
    pub weight_total_kg: f64,
    /// The contingency factor.
    pub contingency: f64,
    /// The engineering factor.
    pub engineering: f64,
}

impl Common {
    fn ratio(&self) -> f64 {
        ratio_of(self.cepci)
    }
}

/// Per-kind outputs beside the cost stack.
#[derive(Clone, Debug, Default)]
pub struct Extras {
    /// A published equipment cost that differs from the stack's PEC.
    pub equipment_cost: Option<f64>,
    /// A published installed cost.
    pub installed_cost: Option<f64>,
    /// A published specific cost per square metre.
    pub specific_cost_per_m2: Option<f64>,
    /// A published specific capex per kilowatt.
    pub specific_capex_per_kw: Option<f64>,
    /// A published annual maintenance cost.
    pub annual_maintenance_cost: Option<f64>,
    /// A published sorbent replacement cost.
    pub sorbent_replacement_cost: Option<f64>,
    /// A published annual operating cost.
    pub annual_operating_cost: Option<f64>,
    /// A weight the class computes rather than carries.
    pub calculated_weight_kg: Option<f64>,
}

/// One evaluated equipment row.
#[derive(Clone, Debug)]
pub struct EquipmentResult {
    /// The kind.
    pub kind: Kind,
    /// The class's equipment-type string.
    pub equipment_type: &'static str,
    /// The cost stack.
    pub stack: ModuleStack,
    /// The class's `getTotalCost` answer.
    pub total_cost: f64,
    /// The material factor used.
    pub material_factor: f64,
    /// Per-kind extras.
    pub extras: Extras,
    /// Material-take-off rows.
    pub mto: Vec<MtoItem>,
    /// Quality flags.
    pub quality_flags: Vec<String>,
}

/// The operating-cost inputs.
#[derive(Clone, Copy, Debug)]
pub struct OpexInputs {
    /// Operating hours per year.
    pub hours_per_year: f64,
    /// Electricity cost, USD/kWh.
    pub electricity_cost_per_kwh: f64,
    /// Steam cost, USD/tonne.
    pub steam_cost_per_tonne: f64,
    /// Cooling-water cost, USD/m3.
    pub cooling_water_cost_per_m3: f64,
    /// Fuel cost, USD/GJ.
    pub fuel_cost_per_gj: f64,
}

impl Default for OpexInputs {
    fn default() -> Self {
        Self {
            hours_per_year: 8000.0,
            electricity_cost_per_kwh: 0.08,
            steam_cost_per_tonne: 15.0,
            cooling_water_cost_per_m3: 0.05,
            fuel_cost_per_gj: 5.0,
        }
    }
}

macro_rules! spec_struct {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        /// A kind's required inputs.
        #[derive(Clone, Debug)]
        pub struct $name { $(pub $field: $ty,)* }
    };
}

spec_struct!(SeparatorSpec {
    inner_diameter_m: f64,
    tangent_length_m: f64,
    vessel_volume_m3: Option<f64>,
});

spec_struct!(ReactorSpec {
    vessel_diameter_m: f64,
    vessel_length_m: f64,
    design_pressure_bara: f64,
    catalyst_mass_kg: f64,
    number_of_beds: f64,
    total_equipped_weight_kg: f64,
    catalyst_cost_per_kg: f64,
    internals_fraction: f64,
    bed_allowance_usd: f64,
});

spec_struct!(ColumnSpec {
    column_type: String,
    tray_type: String,
    packing_type: String,
    diameter_m: f64,
    height_m: f64,
    number_of_trays: f64,
    packing_height_m: f64,
    design_pressure_barg: f64,
    include_reboiler: bool,
    include_condenser: bool,
    reboiler_duty_kw: f64,
    condenser_duty_kw: f64,
});

spec_struct!(AbsorberSpec {
    absorber_type: String,
    tray_type: String,
    packing_type: String,
    diameter_m: f64,
    height_m: f64,
    number_of_stages: f64,
    packing_height_m: f64,
    include_liquid_distributor: bool,
    include_mist_eliminator: bool,
    include_reboiler: bool,
    include_reflux_system: bool,
    reboiler_duty_kw: f64,
});

spec_struct!(CompressorSpec {
    power_kw: f64,
    compressor_type: String,
    driver_type: String,
    include_driver: bool,
    number_of_stages: f64,
    include_intercoolers: bool,
});

spec_struct!(PumpSpec {
    power_kw: f64,
    pump_type: String,
    include_motor: bool,
    api_rated: bool,
    seal_type: String,
});

spec_struct!(ExpanderSpec {
    shaft_power_kw: f64,
    expander_type: String,
    load_type: String,
    include_load: bool,
    include_gearbox: bool,
    include_lube_oil_system: bool,
    include_control_system: bool,
    cryogenic_service: bool,
});

spec_struct!(HeatExchangerSpec {
    area_m2: f64,
    duty_kw: f64,
    exchanger_type: String,
    tema_type: String,
    shell_passes: f64,
});

spec_struct!(BahxSpec {
    core_weight_kg: f64,
    total_weight_kg: f64,
    area_m2: f64,
});

spec_struct!(TankSpec {
    tank_type: String,
    volume_m3: f64,
    diameter_m: f64,
    height_m: f64,
    design_pressure_barg: f64,
    floating_roof: bool,
    include_foundation: bool,
    include_heating_coils: bool,
    include_insulation: bool,
    insulation_thickness_mm: f64,
});

spec_struct!(FilterSpec {
    inner_diameter_m: f64,
    vessel_length_m: f64,
    required_elements: f64,
    design_pressure_bara: f64,
    material_grade: String,
    empty_vessel_weight_kg: f64,
    element_cost_usd: f64,
    auxiliaries_fraction: f64,
    fallback_steel_rate_usd_per_kg: f64,
    fallback_element_cost_usd: f64,
});

spec_struct!(ManifoldSpec {
    inlets: f64,
    outlets: f64,
    weight_kg: f64,
    location: String,
    material_grade: String,
    manifold_type: String,
    header_diameter_m: f64,
    branch_diameter_m: f64,
    water_depth_m: f64,
    base_cost_per_kg: f64,
    branch_allowance_usd: f64,
});

spec_struct!(MercuryRemovalSpec {
    vessel_shell_weight_kg: f64,
    internals_weight_kg: f64,
    nozzle_weight_kg: f64,
    sorbent_charge_weight_kg: f64,
    sorbent_unit_price_usd_per_kg: f64,
    steel_cost_per_kg: f64,
    bed_lifetime_years: f64,
});

spec_struct!(ElectrolyzerSpec {
    power_kw: f64,
    technology: String,
    include_balance_of_plant: bool,
});

spec_struct!(PsaSpec {
    number_of_beds: f64,
    sorbent: String,
    sorbent_mass_per_bed_kg: f64,
    include_balance_of_plant: bool,
    bed_diameter_m: Option<f64>,
    bed_length_m: Option<f64>,
});

spec_struct!(EjectorSpec {
    ejector_type: String,
    number_of_stages: f64,
    suction_pressure_mbar: f64,
    discharge_pressure_bara: f64,
    suction_capacity_kg_per_h: f64,
    include_intercondensers: bool,
    include_aftercondenser: bool,
});

spec_struct!(PipeSpec {
    nominal_diameter_in: f64,
    length_m: f64,
    schedule: String,
    installation_type: String,
    include_fittings: bool,
    fittings_per_hundred_m: f64,
    number_of_flanges: f64,
    include_insulation: bool,
    insulation_thickness_mm: f64,
});

spec_struct!(ValveSpec {
    valve_type: String,
    nominal_size_in: f64,
    valve_cv: f64,
    pressure_class: f64,
    include_actuator: bool,
    actuator_type: String,
});

spec_struct!(MixerSpec {
    mixer_type: String,
    pipe_diameter_in: f64,
    number_of_elements: f64,
    pressure_class: f64,
    flanged_connections: bool,
    header_cost_usd: Option<f64>,
});

spec_struct!(SplitterSpec {
    splitter_type: String,
    number_of_outlets: f64,
    inlet_diameter_in: f64,
    outlet_diameter_in: f64,
    pressure_class: f64,
    include_control_valves: bool,
    include_flow_meters: bool,
    header_cost_usd: Option<f64>,
});

spec_struct!(WellSpec {
    well_capex_usd: f64,
});

/// The selected equipment and its inputs.
#[derive(Clone, Debug)]
pub enum EquipmentSpec {
    /// Separator.
    Separator(SeparatorSpec),
    /// Reactor.
    Reactor(ReactorSpec),
    /// Column.
    Column(ColumnSpec),
    /// Absorber.
    Absorber(AbsorberSpec),
    /// Compressor.
    Compressor(CompressorSpec),
    /// Pump.
    Pump(PumpSpec),
    /// Expander.
    Expander(ExpanderSpec),
    /// Heat exchanger.
    HeatExchanger(HeatExchangerSpec),
    /// BAHX.
    Bahx(BahxSpec),
    /// Tank.
    Tank(TankSpec),
    /// Filter.
    Filter(FilterSpec),
    /// Manifold.
    Manifold(ManifoldSpec),
    /// Mercury removal.
    MercuryRemoval(MercuryRemovalSpec),
    /// Electrolyzer.
    Electrolyzer(ElectrolyzerSpec),
    /// PSA.
    Psa(PsaSpec),
    /// Ejector.
    Ejector(EjectorSpec),
    /// Pipe.
    Pipe(PipeSpec),
    /// Valve.
    Valve(ValveSpec),
    /// Mixer.
    Mixer(MixerSpec),
    /// Splitter.
    Splitter(SplitterSpec),
    /// Well.
    Well(WellSpec),
}

impl EquipmentSpec {
    /// The kind.
    #[must_use]
    pub fn kind(&self) -> Kind {
        match self {
            Self::Separator(_) => Kind::Separator,
            Self::Reactor(_) => Kind::Reactor,
            Self::Column(_) => Kind::Column,
            Self::Absorber(_) => Kind::Absorber,
            Self::Compressor(_) => Kind::Compressor,
            Self::Pump(_) => Kind::Pump,
            Self::Expander(_) => Kind::Expander,
            Self::HeatExchanger(_) => Kind::HeatExchanger,
            Self::Bahx(_) => Kind::Bahx,
            Self::Tank(_) => Kind::Tank,
            Self::Filter(_) => Kind::Filter,
            Self::Manifold(_) => Kind::Manifold,
            Self::MercuryRemoval(_) => Kind::MercuryRemoval,
            Self::Electrolyzer(_) => Kind::Electrolyzer,
            Self::Psa(_) => Kind::Psa,
            Self::Ejector(_) => Kind::Ejector,
            Self::Pipe(_) => Kind::Pipe,
            Self::Valve(_) => Kind::Valve,
            Self::Mixer(_) => Kind::Mixer,
            Self::Splitter(_) => Kind::Splitter,
            Self::Well(_) => Kind::Well,
        }
    }
}

/// Evaluate one equipment row.
#[must_use]
pub fn evaluate(spec: &EquipmentSpec, common: &Common) -> EquipmentResult {
    let mut extras = Extras::default();
    let mto = Vec::new();
    let mut flags = Vec::new();
    let pec = match spec {
        EquipmentSpec::Separator(s) => separator_pec(s, common),
        EquipmentSpec::Reactor(s) => reactor_pec(s, common),
        EquipmentSpec::Column(s) => column_pec(s, common),
        EquipmentSpec::Absorber(s) => absorber_pec(s, common),
        EquipmentSpec::Compressor(s) => compressor_pec(s, common),
        EquipmentSpec::Pump(s) => pump_pec(s, common),
        EquipmentSpec::Expander(s) => expander_pec(s, common),
        EquipmentSpec::HeatExchanger(s) => heat_exchanger_pec(s, common),
        EquipmentSpec::Bahx(s) => bahx_pec(s, common, &mut extras),
        EquipmentSpec::Tank(s) => tank_pec(s, common),
        EquipmentSpec::Filter(s) => filter_pec(s, common),
        EquipmentSpec::Manifold(s) => manifold_pec(s, common),
        EquipmentSpec::MercuryRemoval(s) => mercury_pec(s, common, &mut extras),
        EquipmentSpec::Electrolyzer(s) => electrolyzer_pec(s, common, &mut extras),
        EquipmentSpec::Psa(s) => psa_pec(s, common),
        EquipmentSpec::Ejector(s) => ejector_pec(s, common),
        EquipmentSpec::Pipe(s) => pipe_pec(s, common, &mut extras),
        EquipmentSpec::Valve(s) => valve_pec(s, common),
        EquipmentSpec::Mixer(s) => mixer_pec(s, common),
        EquipmentSpec::Splitter(s) => splitter_pec(s, common),
        EquipmentSpec::Well(s) => s.well_capex_usd.max(0.0),
    };
    let kind = spec.kind();
    let equipment_type = kind.equipment_type();

    if matches!(spec, EquipmentSpec::Well(_)) {
        let stack = ModuleStack {
            purchased_equipment_cost: pec,
            bare_module_cost: pec,
            total_module_cost: pec,
            grass_roots_cost: pec,
            installation_man_hours: 0.0,
        };
        return EquipmentResult {
            kind,
            equipment_type,
            stack,
            total_cost: pec,
            material_factor: common.material_factor,
            extras,
            mto,
            quality_flags: flags,
        };
    }

    let stack = module_stack::equipment_stack(
        pec,
        &module_stack::StackInput {
            design_present: common.design_present,
            max_design_pressure_barg: common.max_design_pressure_barg,
            weight_kg: common.weight_total_kg,
            equipment_type,
            material_factor: common.material_factor,
            contingency: common.contingency,
            engineering: common.engineering,
        },
    );

    let total_cost = if kind.total_cost_is_module_only() || stack.total_module_cost > 0.0 {
        stack.total_module_cost
    } else if common.design_present {
        common.weight_total_kg * COST_PER_WEIGHT_UNIT
    } else {
        0.0
    };

    if pec <= 0.0 {
        flags.push("The purchased cost is zero; check the capacity basis.".to_string());
    }

    EquipmentResult {
        kind,
        equipment_type,
        stack,
        total_cost,
        material_factor: common.material_factor,
        extras,
        mto,
        quality_flags: flags,
    }
}

/// The base class's `generateBillOfMaterials` over a stated weight breakdown.
#[must_use]
pub fn weight_bom(breakdown: &WeightBreakdown) -> Vec<MtoItem> {
    let material = breakdown.material.as_str();
    let cost_per_weight_unit = breakdown.cost_per_weight_unit;
    let mut rows = Vec::new();
    let mut push = |item: &str, material: Option<&str>, weight: f64, multiplier: f64| {
        if weight > 0.0 {
            rows.push(MtoItem {
                item: item.to_string(),
                material: material.map(str::to_string),
                quantity: None,
                unit: Some("kg".to_string()),
                weight_kg: Some(weight),
                cost_usd: weight * cost_per_weight_unit * multiplier,
                source: None,
            });
        }
    };
    push("Vessel Shell", Some(material), breakdown.shell_kg, 1.0);
    push("Internals", Some(material), breakdown.internals_kg, 1.5);
    push("Nozzles", Some(material), breakdown.nozzle_kg, 2.0);
    push("Piping", Some(material), breakdown.piping_kg, 1.2);
    push(
        "Electrical & Instrumentation",
        None,
        breakdown.electro_instrument_kg,
        3.0,
    );
    push(
        "Structural Steel",
        Some("Carbon Steel"),
        breakdown.structural_kg,
        0.8,
    );
    rows
}

/// The base class's weight components, for its bill of materials.
#[derive(Clone, Debug)]
pub struct WeightBreakdown {
    /// The vessel shell weight, kg.
    pub shell_kg: f64,
    /// The internals weight, kg.
    pub internals_kg: f64,
    /// The nozzle weight, kg.
    pub nozzle_kg: f64,
    /// The piping weight, kg.
    pub piping_kg: f64,
    /// The electrical and instrumentation weight, kg.
    pub electro_instrument_kg: f64,
    /// The structural steel weight, kg.
    pub structural_kg: f64,
    /// The material of construction.
    pub material: String,
    /// The fallback cost per kilogram.
    pub cost_per_weight_unit: f64,
}

// ---------------------------------------------------------------------------
// The per-kind purchased-cost correlations.
// ---------------------------------------------------------------------------

fn separator_pec(spec: &SeparatorSpec, common: &Common) -> f64 {
    let mut volume = 0.0;
    if spec.inner_diameter_m > 0.0 && spec.tangent_length_m > 0.0 {
        volume = std::f64::consts::FRAC_PI_4
            * spec.inner_diameter_m
            * spec.inner_diameter_m
            * spec.tangent_length_m;
    }
    if volume <= 0.0 {
        volume = spec.vessel_volume_m3.unwrap_or(0.0);
    }
    if volume <= 0.0 {
        return 0.0;
    }
    vertical_vessel_by_volume(volume, common.ratio())
}

fn reactor_pec(spec: &ReactorSpec, common: &Common) -> f64 {
    let volume = if spec.vessel_diameter_m > 0.0 && spec.vessel_length_m > 0.0 {
        std::f64::consts::FRAC_PI_4
            * spec.vessel_diameter_m
            * spec.vessel_diameter_m
            * spec.vessel_length_m
    } else {
        0.0
    };
    let vessel_cost = vertical_vessel_by_volume(volume, common.ratio());
    let pressure = pressure_factor((spec.design_pressure_bara - 1.01325).max(0.0));
    let catalyst = spec.catalyst_mass_kg * spec.catalyst_cost_per_kg;
    let internals = vessel_cost * spec.internals_fraction
        + spec.number_of_beds.max(1.0) * spec.bed_allowance_usd;
    let fallback = spec.total_equipped_weight_kg * 20.0;
    (vessel_cost * pressure + catalyst + internals).max(fallback)
}

fn column_pressure_factor(design_pressure_barg: f64) -> f64 {
    if design_pressure_barg <= 5.0 {
        1.0
    } else if design_pressure_barg <= 20.0 {
        1.2
    } else if design_pressure_barg <= 50.0 {
        1.5
    } else if design_pressure_barg <= 100.0 {
        2.0
    } else {
        2.5
    }
}

fn column_pec(spec: &ColumnSpec, common: &Common) -> f64 {
    let ratio = common.ratio();
    let area = std::f64::consts::FRAC_PI_4 * spec.diameter_m * spec.diameter_m;
    let volume = area * spec.height_m;
    let shell = vertical_vessel_by_volume(volume, ratio)
        * column_pressure_factor(spec.design_pressure_barg)
        * common.material_factor;
    let internals = if spec.column_type.eq_ignore_ascii_case("packed") {
        let packing_volume = area * spec.packing_height_m;
        packing(packing_volume, &spec.packing_type, ratio) * common.material_factor
            + 1500.0 * area.powf(0.6) * 2.0 * ratio * common.material_factor
    } else {
        trays(
            &spec.tray_type,
            spec.diameter_m,
            spec.number_of_trays,
            ratio,
        ) * common.material_factor
    };
    let reboiler = if spec.include_reboiler && spec.reboiler_duty_kw > 0.0 {
        let area = spec.reboiler_duty_kw * 1000.0 / (1000.0 * 30.0);
        shell_tube_heat_exchanger(area, ratio) * 1.3 * common.material_factor
    } else {
        0.0
    };
    let condenser = if spec.include_condenser && spec.condenser_duty_kw > 0.0 {
        let area = spec.condenser_duty_kw * 1000.0 / (800.0 * 20.0);
        shell_tube_heat_exchanger(area, ratio) * common.material_factor
    } else {
        0.0
    };
    shell + internals + reboiler + condenser
}

fn trays(tray_type: &str, diameter_m: f64, count: f64, ratio: f64) -> f64 {
    if tray_type.eq_ignore_ascii_case("valve") {
        valve_trays(diameter_m, count, ratio)
    } else if tray_type.eq_ignore_ascii_case("bubble-cap") {
        bubble_cap_trays(diameter_m, count, ratio)
    } else {
        sieve_trays(diameter_m, count, ratio)
    }
}

fn absorber_pec(spec: &AbsorberSpec, common: &Common) -> f64 {
    let ratio = common.ratio();
    let area = std::f64::consts::FRAC_PI_4 * spec.diameter_m * spec.diameter_m;
    let volume = area * spec.height_m;
    let shell = vertical_vessel_by_volume(volume, ratio);
    let internals = if spec.absorber_type.eq_ignore_ascii_case("trayed") {
        trays(
            &spec.tray_type,
            spec.diameter_m,
            spec.number_of_stages,
            ratio,
        )
    } else if spec.absorber_type.eq_ignore_ascii_case("spray") {
        (5000.0 + 2000.0 * area) * ratio
    } else {
        packing(area * spec.packing_height_m, &spec.packing_type, ratio)
    };
    let mut total = (shell + internals) * common.material_factor;
    if spec.include_liquid_distributor {
        total += 3000.0 * area * common.material_factor * ratio;
    }
    if spec.include_mist_eliminator {
        total += 1500.0 * area * common.material_factor * ratio;
    }
    if spec.include_reboiler {
        let reboiler_area = spec.reboiler_duty_kw * 1000.0 / (800.0 * 30.0);
        total += shell_tube_heat_exchanger(reboiler_area, ratio) * common.material_factor;
    }
    if spec.include_reflux_system {
        let drum_volume = 0.1 * volume;
        let drum_weight = drum_volume * 100.0;
        let drum = horizontal_vessel_by_weight(drum_weight, ratio);
        let pump = centrifugal_pump(10.0, ratio);
        total += (drum + pump) * common.material_factor;
    }
    total
}

fn compressor_pec(spec: &CompressorSpec, common: &Common) -> f64 {
    if spec.power_kw <= 0.0 {
        return common.weight_total_kg * 20.0;
    }
    let ratio = common.ratio();
    let mut cost = if spec.compressor_type.eq_ignore_ascii_case("reciprocating") {
        reciprocating_compressor(spec.power_kw, ratio)
    } else if spec.compressor_type.eq_ignore_ascii_case("screw") {
        centrifugal_compressor(spec.power_kw, ratio) * 0.8
    } else if spec.compressor_type.eq_ignore_ascii_case("axial") {
        centrifugal_compressor(spec.power_kw, ratio) * 1.3
    } else {
        centrifugal_compressor(spec.power_kw, ratio)
    };
    if spec.number_of_stages > 1.0 {
        cost *= 1.0 + (spec.number_of_stages - 1.0) * 0.25;
    }
    if spec.include_driver {
        cost += driver_cost(&spec.driver_type, spec.power_kw, ratio);
    }
    if spec.include_intercoolers && spec.number_of_stages > 1.0 {
        let heat_duty = spec.power_kw * 0.7 * 1000.0;
        let area = heat_duty / (500.0 * 30.0);
        cost += shell_tube_heat_exchanger(area, ratio) * (spec.number_of_stages - 1.0);
    }
    cost
}

fn driver_cost(driver_type: &str, power_kw: f64, ratio: f64) -> f64 {
    if power_kw <= 0.0 {
        return 0.0;
    }
    if driver_type.eq_ignore_ascii_case("gas-turbine") {
        4000.0 * power_kw.powf(0.8) * ratio
    } else if driver_type.eq_ignore_ascii_case("steam-turbine") {
        2000.0 * power_kw.powf(0.75) * ratio
    } else if driver_type.eq_ignore_ascii_case("gas-engine") {
        500.0 * power_kw.powf(0.85) * ratio
    } else {
        300.0 * (power_kw / 0.95).powf(0.75) * ratio
    }
}

fn pump_pec(spec: &PumpSpec, common: &Common) -> f64 {
    if spec.power_kw <= 0.0 {
        return common.weight_total_kg * 15.0;
    }
    let ratio = common.ratio();
    let mut cost = if spec.pump_type.eq_ignore_ascii_case("reciprocating") {
        centrifugal_pump(spec.power_kw, ratio) * 1.5
    } else if spec.pump_type.eq_ignore_ascii_case("gear") {
        400.0 * spec.power_kw.max(1.0).powf(0.7) * ratio
    } else if spec.pump_type.eq_ignore_ascii_case("screw") {
        centrifugal_pump(spec.power_kw, ratio) * 2.0
    } else if spec.pump_type.eq_ignore_ascii_case("diaphragm") {
        600.0 * spec.power_kw.max(1.0).powf(0.6) * ratio
    } else {
        centrifugal_pump(spec.power_kw, ratio)
    };
    if spec.api_rated {
        cost *= 1.5;
    }
    cost *= if spec.seal_type.eq_ignore_ascii_case("double-mechanical") {
        1.3
    } else if spec.seal_type.eq_ignore_ascii_case("packed") {
        0.8
    } else if spec.seal_type.eq_ignore_ascii_case("mag-drive")
        || spec.seal_type.eq_ignore_ascii_case("magnetic-drive")
    {
        2.0
    } else if spec.seal_type.eq_ignore_ascii_case("canned") {
        2.5
    } else {
        1.0
    };
    if spec.include_motor {
        cost += 200.0 * (spec.power_kw / 0.9).powf(0.75) * ratio;
    }
    cost
}

fn expander_pec(spec: &ExpanderSpec, common: &Common) -> f64 {
    let power = spec.shaft_power_kw;
    if power <= 0.0 {
        return if common.weight_total_kg > 0.0 {
            common.weight_total_kg * 30.0
        } else {
            0.0
        };
    }
    let ratio = common.ratio();
    let radial = || 3000.0 * power.max(50.0).powf(0.75) * ratio;
    let mut cost = if spec.expander_type.eq_ignore_ascii_case("axial") {
        radial() * 1.2
    } else if spec.expander_type.eq_ignore_ascii_case("mixed-flow") {
        radial() * 1.1
    } else {
        radial()
    };
    if spec.cryogenic_service {
        cost *= 1.4;
    }
    cost *= common.material_factor;
    if spec.include_load {
        cost += if spec.load_type.eq_ignore_ascii_case("generator") {
            400.0 * power.powf(0.7) * ratio
        } else if spec.load_type.eq_ignore_ascii_case("compressor") {
            centrifugal_compressor(power, ratio)
        } else {
            150.0 * power.powf(0.6) * ratio
        };
    }
    if spec.include_gearbox {
        cost += 200.0 * power.max(10.0).powf(0.65) * ratio;
    }
    if spec.include_lube_oil_system {
        cost += radial() * 0.05;
    }
    if spec.include_control_system {
        cost += (15000.0 + 10.0 * power) * ratio;
    }
    cost
}

fn heat_exchanger_pec(spec: &HeatExchangerSpec, common: &Common) -> f64 {
    let ratio = common.ratio();
    let mut area = spec.area_m2;
    if area <= 0.0 {
        area = spec.duty_kw.abs() * 1000.0 / (500.0 * 30.0);
    }
    if area <= 0.0 {
        return common.weight_total_kg * 8.0;
    }
    let mut cost = if spec.exchanger_type.eq_ignore_ascii_case("plate") {
        plate_heat_exchanger(area, ratio)
    } else if spec.exchanger_type.eq_ignore_ascii_case("air-cooler")
        || spec.exchanger_type.eq_ignore_ascii_case("aircooler")
    {
        air_cooler(area, ratio)
    } else if spec.exchanger_type.eq_ignore_ascii_case("double-pipe") {
        shell_tube_heat_exchanger(area, ratio) * 0.6
    } else {
        shell_tube_heat_exchanger(area, ratio)
    };
    cost *= tema_factor(&spec.tema_type);
    if spec.shell_passes > 1.0 {
        cost *= 1.0 + 0.1 * (spec.shell_passes - 1.0);
    }
    cost
}

fn tema_factor(tema_type: &str) -> f64 {
    let bytes = tema_type.as_bytes();
    let front = bytes.first().copied().unwrap_or(b'A') as char;
    let rear = bytes.get(2).copied().unwrap_or(b'S') as char;
    let mut factor = if front == 'B' {
        1.1
    } else if front == 'C' {
        1.2
    } else if front == 'N' {
        1.3
    } else {
        1.0
    };
    factor *= match rear {
        'P' => 1.2,
        'S' => 1.15,
        'T' => 1.25,
        'U' => 0.9,
        'W' => 1.1,
        _ => 1.0,
    };
    factor
}

fn bahx_pec(spec: &BahxSpec, _common: &Common, extras: &mut Extras) -> f64 {
    let header_weight = spec.total_weight_kg - spec.core_weight_kg;
    let core_cost = spec.core_weight_kg * (8.0 + 25.0);
    let header_cost = header_weight * (8.0 + 15.0);
    let brazing = spec.area_m2 * 120.0;
    let testing = (core_cost + header_cost) * 0.12;
    let equipment = (core_cost + header_cost + brazing + testing) * 1.15;
    extras.equipment_cost = Some(equipment);
    extras.installed_cost = Some(equipment * 3.5 * 1.10);
    extras.specific_cost_per_m2 = (spec.area_m2 > 0.0).then(|| equipment / spec.area_m2);
    extras.annual_maintenance_cost = Some(equipment * 0.02);
    equipment
}

fn tank_pec(spec: &TankSpec, common: &Common) -> f64 {
    let ratio = common.ratio();
    let diameter = spec.diameter_m;
    let height = spec.height_m;
    let volume = if common.design_present {
        std::f64::consts::FRAC_PI_4 * diameter * diameter * height
    } else {
        spec.volume_m3
    };
    let floating = spec.floating_roof || spec.tank_type.eq_ignore_ascii_case("floating-roof");
    let mut base = if spec.tank_type.eq_ignore_ascii_case("spherical") {
        let v = volume.max(50.0);
        let mut cost = 2500.0 * v.powf(0.65);
        if spec.design_pressure_barg > 1.0 {
            cost *= 1.0 + 0.05 * spec.design_pressure_barg;
        }
        cost * ratio
    } else if spec.tank_type.eq_ignore_ascii_case("horizontal") {
        let v = volume.max(10.0);
        let mut cost = 600.0 * v.powf(0.55);
        if spec.design_pressure_barg > 0.0 {
            cost *= pressure_factor(spec.design_pressure_barg);
        }
        cost * ratio
    } else if floating {
        700.0 * volume.max(500.0).powf(0.52) * ratio
    } else {
        let mut cost = 475.0 * volume.max(100.0).powf(0.507);
        if spec.tank_type.eq_ignore_ascii_case("fixed-dome-roof") {
            cost *= 1.15;
        }
        cost * ratio
    };
    base *= common.material_factor;
    if spec.design_pressure_barg > 0.07 && spec.design_pressure_barg < 1.0 {
        base *= 1.0 + spec.design_pressure_barg * 0.3;
    }
    if spec.include_foundation {
        let footprint = std::f64::consts::FRAC_PI_4 * diameter * diameter;
        let mut foundation = footprint * 300.0;
        if volume > 10000.0 {
            foundation *= 1.2;
        }
        base += foundation * ratio;
    }
    if spec.include_heating_coils {
        base += volume * 50.0 * common.material_factor * ratio;
    }
    if spec.include_insulation {
        let surface = std::f64::consts::PI * diameter * height
            + 2.0 * std::f64::consts::FRAC_PI_4 * diameter * diameter;
        let mut insulation = surface * 80.0;
        insulation *= 1.0 + (spec.insulation_thickness_mm - 50.0) / 100.0;
        base += insulation * ratio;
    }
    base
}

fn filter_material_factor(grade: &str) -> f64 {
    let grade = grade.to_ascii_uppercase();
    if grade.contains("DUPLEX") || grade.contains("22CR") || grade.contains("25CR") {
        2.2
    } else if grade.contains("316") || grade.contains("SS") || grade.contains("STAINLESS") {
        1.8
    } else {
        1.0
    }
}

fn filter_pec(spec: &FilterSpec, common: &Common) -> f64 {
    let volume = if spec.inner_diameter_m > 0.0 && spec.vessel_length_m > 0.0 {
        std::f64::consts::FRAC_PI_4
            * spec.inner_diameter_m
            * spec.inner_diameter_m
            * spec.vessel_length_m
    } else {
        0.0
    };
    let vessel = horizontal_vessel_by_volume(volume, common.ratio());
    let elements = spec.required_elements.max(1.0) * spec.element_cost_usd;
    let correlation = (vessel + elements) * (1.0 + spec.auxiliaries_fraction);
    let fallback = spec.empty_vessel_weight_kg * spec.fallback_steel_rate_usd_per_kg
        + spec.required_elements * spec.fallback_element_cost_usd;
    let mut cost = correlation.max(fallback);
    cost *= filter_material_factor(&spec.material_grade);
    cost *= pressure_factor((spec.design_pressure_bara - 1.01325).max(0.0));
    cost
}

fn manifold_location_factor(location: &str) -> f64 {
    if location.eq_ignore_ascii_case("subsea") {
        2.6
    } else if location.eq_ignore_ascii_case("onshore") {
        0.75
    } else {
        1.0
    }
}

fn manifold_type_factor(manifold_type: &str) -> f64 {
    if manifold_type.eq_ignore_ascii_case("pigging") || manifold_type.eq_ignore_ascii_case("test") {
        1.2
    } else if manifold_type.eq_ignore_ascii_case("injection") {
        1.1
    } else {
        1.0
    }
}

fn manifold_pressure_factor(header_diameter_m: f64) -> f64 {
    if header_diameter_m >= 0.4 {
        1.8
    } else if header_diameter_m >= 0.25 {
        1.4
    } else {
        1.0
    }
}

fn manifold_pec(spec: &ManifoldSpec, _common: &Common) -> f64 {
    let branches = (spec.inlets + spec.outlets).max(1.0);
    let mut weight = spec.weight_kg;
    if weight <= 0.0 {
        let header_diameter = spec.header_diameter_m.max(0.1);
        let branch_diameter = spec.branch_diameter_m.max(0.05);
        let header_length = (branches * 1.2).max(4.0);
        let header_thickness = (header_diameter * 0.04).max(0.008);
        let branch_thickness = (branch_diameter * 0.04).max(0.006);
        let header_weight =
            std::f64::consts::PI * header_diameter * header_thickness * header_length * 7850.0;
        let branch_weight =
            std::f64::consts::PI * branch_diameter * branch_thickness * 1.5 * 7850.0 * branches;
        weight = (header_weight + branch_weight) * 2.2;
    }
    let fabrication = weight
        * spec.base_cost_per_kg
        * manifold_location_factor(&spec.location)
        * filter_material_factor(&spec.material_grade)
        * manifold_type_factor(&spec.manifold_type);
    let branch =
        branches * spec.branch_allowance_usd * manifold_pressure_factor(spec.header_diameter_m);
    let subsea = if spec.location.eq_ignore_ascii_case("subsea") {
        250000.0 + spec.water_depth_m.max(0.0) * 350.0
    } else {
        0.0
    };
    fabrication + branch + subsea
}

fn mercury_pec(spec: &MercuryRemovalSpec, common: &Common, extras: &mut Extras) -> f64 {
    let steel = spec.vessel_shell_weight_kg + spec.internals_weight_kg + spec.nozzle_weight_kg;
    let vessel = steel * spec.steel_cost_per_kg * common.material_factor;
    let sorbent = spec.sorbent_charge_weight_kg * spec.sorbent_unit_price_usd_per_kg;
    extras.sorbent_replacement_cost = Some(sorbent * 1.25);
    vessel + sorbent
}

fn electrolyzer_specific_capex(technology: &str) -> f64 {
    if technology.eq_ignore_ascii_case("ALKALINE") {
        800.0
    } else if technology.eq_ignore_ascii_case("SOEC") {
        2500.0
    } else if technology.eq_ignore_ascii_case("AEM") {
        1500.0
    } else {
        1250.0
    }
}

fn electrolyzer_pec(spec: &ElectrolyzerSpec, common: &Common, extras: &mut Extras) -> f64 {
    if spec.power_kw <= 0.0 {
        return 0.0;
    }
    let specific = electrolyzer_specific_capex(&spec.technology);
    extras.specific_capex_per_kw = Some(specific);
    let mut cost = spec.power_kw * specific * (spec.power_kw / 1000.0).powf(-0.15);
    if !spec.include_balance_of_plant {
        cost *= 0.65;
    }
    cost * (common.cepci / 800.0)
}

fn psa_sorbent_price(sorbent: &str) -> f64 {
    if sorbent.eq_ignore_ascii_case("ZEOLITE_13X") {
        10.0
    } else {
        4.0
    }
}

fn psa_pec(spec: &PsaSpec, common: &Common) -> f64 {
    let mut bed_volume = 12.566;
    if let (Some(diameter), Some(length)) = (spec.bed_diameter_m, spec.bed_length_m) {
        if diameter > 0.0 && length > 0.0 {
            bed_volume = std::f64::consts::FRAC_PI_4 * diameter * diameter * length;
        }
    }
    let scale = (bed_volume / 12.566).powf(0.6);
    let per_bed_vessel = 250000.0 * scale;
    let per_bed_sorbent = spec.sorbent_mass_per_bed_kg * psa_sorbent_price(&spec.sorbent);
    let per_bed = per_bed_vessel + 60000.0 + per_bed_sorbent;
    let mut cost = spec.number_of_beds * per_bed;
    if !spec.include_balance_of_plant {
        cost *= 0.75;
    }
    cost * (common.cepci / 800.0)
}

fn ejector_pec(spec: &EjectorSpec, common: &Common) -> f64 {
    let ratio = common.ratio();
    let compression = if spec.suction_pressure_mbar > 0.0 {
        (spec.discharge_pressure_bara * 1000.0) / spec.suction_pressure_mbar
    } else {
        0.0
    };
    let stage_factor = 1.0 + (spec.number_of_stages - 1.0) * 0.7;
    let steam = || {
        500.0
            * spec.suction_capacity_kg_per_h.max(10.0).powf(0.6)
            * compression.powf(0.3)
            * stage_factor
            * ratio
    };
    let mut cost = if spec.ejector_type.eq_ignore_ascii_case("gas") {
        steam() * 1.3
    } else if spec.ejector_type.eq_ignore_ascii_case("liquid") {
        300.0 * spec.suction_capacity_kg_per_h.max(10.0).powf(0.5) * ratio
    } else if spec.ejector_type.eq_ignore_ascii_case("hybrid") {
        steam() + 800.0 * spec.suction_capacity_kg_per_h.max(10.0).powf(0.55) * ratio
    } else {
        steam()
    };
    cost *= common.material_factor;
    if spec.include_intercondensers && spec.number_of_stages > 1.0 {
        let area = spec.suction_capacity_kg_per_h * 0.1;
        cost += (spec.number_of_stages - 1.0)
            * shell_tube_heat_exchanger(area, ratio)
            * common.material_factor;
    }
    if spec.include_aftercondenser {
        let area = spec.suction_capacity_kg_per_h * 0.15;
        cost += shell_tube_heat_exchanger(area, ratio) * common.material_factor;
    }
    cost
}

fn pipe_schedule_for_cost(schedule: &str) -> u32 {
    if schedule == "80" || schedule == "80S" {
        80
    } else if schedule == "160" {
        160
    } else {
        40
    }
}

fn pipe_schedule_weight_factor(schedule: &str) -> f64 {
    if schedule == "10" || schedule == "10S" {
        0.7
    } else if schedule == "80" || schedule == "80S" {
        1.4
    } else if schedule == "160" {
        2.0
    } else if schedule == "XXS" {
        2.5
    } else {
        1.0
    }
}

fn pipe_wall_thickness(nominal_diameter_in: f64, schedule: &str) -> f64 {
    let base = if nominal_diameter_in <= 2.0 {
        0.003
    } else if nominal_diameter_in <= 6.0 {
        0.0055
    } else if nominal_diameter_in <= 12.0 {
        0.0095
    } else {
        0.0125
    };
    base * pipe_schedule_weight_factor(schedule)
}

fn pipe_weight(spec: &PipeSpec) -> f64 {
    let thickness = pipe_wall_thickness(spec.nominal_diameter_in, &spec.schedule);
    let outer = spec.nominal_diameter_in * 0.0254;
    let inner = outer - 2.0 * thickness;
    std::f64::consts::FRAC_PI_4 * (outer * outer - inner * inner) * spec.length_m * 7850.0
}

fn pipe_pec(spec: &PipeSpec, common: &Common, extras: &mut Extras) -> f64 {
    let ratio = common.ratio();
    let diameter_m = spec.nominal_diameter_in * 0.0254;
    let mut cost = crate::costing::correlations::piping_cost(
        diameter_m,
        spec.length_m,
        pipe_schedule_for_cost(&spec.schedule),
        common.material_factor,
        ratio,
    );
    cost *= common.material_factor;
    if spec.include_fittings {
        let total = (spec.fittings_per_hundred_m * spec.length_m / 100.0).trunc();
        let average = 30.0 + 15.0 * spec.nominal_diameter_in.powf(0.8);
        cost += total * average * common.material_factor * ratio;
    }
    if spec.number_of_flanges > 0.0 {
        let price = 50.0 + 30.0 * spec.nominal_diameter_in.powf(1.2);
        cost += spec.number_of_flanges * price * common.material_factor * ratio;
    }
    cost *= if spec.installation_type.eq_ignore_ascii_case("buried") {
        1.5
    } else if spec.installation_type.eq_ignore_ascii_case("subsea") {
        3.0
    } else {
        1.0
    };
    if spec.include_insulation {
        let surface = std::f64::consts::PI * diameter_m * spec.length_m;
        let mut insulation = surface * 30.0;
        insulation *= 1.0 + spec.insulation_thickness_mm / 100.0;
        cost += insulation * ratio;
    }
    extras.calculated_weight_kg = Some(pipe_weight(spec));
    cost
}

fn valve_pressure_class_factor(pressure_class: f64) -> f64 {
    match pressure_class as i64 {
        150 => 1.0,
        300 => 1.2,
        600 => 1.5,
        900 => 2.0,
        1500 => 2.5,
        2500 => 3.5,
        _ => 1.0,
    }
}

fn valve_pec(spec: &ValveSpec, common: &Common) -> f64 {
    let ratio = common.ratio();
    let mut cost = if spec.valve_type.eq_ignore_ascii_case("control") {
        control_valve(spec.valve_cv, common.material_factor, ratio)
    } else {
        let gate = || (40.0 + 20.0 * spec.nominal_size_in.powf(1.5)) * ratio;
        if spec.valve_type.eq_ignore_ascii_case("gate") {
            gate()
        } else if spec.valve_type.eq_ignore_ascii_case("ball") {
            gate() * 1.5
        } else if spec.valve_type.eq_ignore_ascii_case("globe") {
            gate() * 1.3
        } else if spec.valve_type.eq_ignore_ascii_case("check") {
            gate() * 0.8
        } else if spec.valve_type.eq_ignore_ascii_case("butterfly") {
            gate() * 0.6
        } else {
            control_valve(spec.valve_cv, common.material_factor, ratio)
        }
    };
    cost *= valve_pressure_class_factor(spec.pressure_class);
    cost *= common.material_factor;
    if spec.include_actuator && spec.valve_type.eq_ignore_ascii_case("control") {
        let base = if spec.actuator_type.eq_ignore_ascii_case("electric") {
            500.0 + 100.0 * spec.nominal_size_in.powf(0.8)
        } else if spec.actuator_type.eq_ignore_ascii_case("hydraulic") {
            800.0 + 150.0 * spec.nominal_size_in.powf(0.8)
        } else if spec.actuator_type.eq_ignore_ascii_case("manual") {
            50.0 + 10.0 * spec.nominal_size_in
        } else {
            300.0 + 80.0 * spec.nominal_size_in.powf(0.8)
        };
        cost += base * ratio;
    }
    cost
}

fn mixer_pressure_class_factor(pressure_class: f64) -> f64 {
    if pressure_class <= 150.0 {
        1.0
    } else if pressure_class <= 300.0 {
        1.1
    } else if pressure_class <= 600.0 {
        1.3
    } else if pressure_class <= 900.0 {
        1.5
    } else if pressure_class <= 1500.0 {
        1.8
    } else {
        2.2
    }
}

fn mixer_pec(spec: &MixerSpec, common: &Common) -> f64 {
    let pressure = mixer_pressure_class_factor(spec.pressure_class);
    if let Some(header_cost) = spec.header_cost_usd {
        if header_cost > 0.0 {
            return header_cost * common.material_factor * pressure;
        }
    }
    let ratio = common.ratio();
    let mut cost = if spec.mixer_type.eq_ignore_ascii_case("inline") {
        100.0 * spec.pipe_diameter_in.powf(1.5) * spec.number_of_elements.powf(0.5) * ratio * 2.5
    } else if spec.mixer_type.eq_ignore_ascii_case("tee") {
        (50.0 + 20.0 * spec.pipe_diameter_in.powf(1.2)) * ratio
    } else if spec.mixer_type.eq_ignore_ascii_case("vessel") {
        let diameter_m = spec.pipe_diameter_in * 0.0254;
        let height_m = diameter_m * 2.0;
        let volume = std::f64::consts::FRAC_PI_4 * diameter_m * diameter_m * height_m;
        500.0 * (volume * 1000.0).max(100.0).powf(0.55) * ratio
    } else {
        100.0 * spec.pipe_diameter_in.powf(1.5) * spec.number_of_elements.powf(0.5) * ratio
    };
    cost *= common.material_factor;
    cost *= pressure;
    if spec.flanged_connections {
        let price = 50.0 + 30.0 * spec.pipe_diameter_in.powf(1.2);
        cost += 2.0 * price * common.material_factor * pressure * ratio;
    }
    cost
}

fn splitter_pec(spec: &SplitterSpec, common: &Common) -> f64 {
    let pressure = mixer_pressure_class_factor(spec.pressure_class);
    if let Some(header_cost) = spec.header_cost_usd {
        if header_cost > 0.0 {
            return header_cost * common.material_factor * pressure;
        }
    }
    let ratio = common.ratio();
    let manifold = || {
        let base = 200.0 + 50.0 * spec.inlet_diameter_in.powf(1.3);
        let outlets = spec.number_of_outlets * 30.0 * spec.outlet_diameter_in.powf(1.2);
        (base + outlets) * ratio
    };
    let mut cost = if spec.splitter_type.eq_ignore_ascii_case("header") {
        manifold() * 1.3
    } else if spec.splitter_type.eq_ignore_ascii_case("tee") {
        (50.0 + 25.0 * spec.inlet_diameter_in.powf(1.2)) * ratio
    } else if spec.splitter_type.eq_ignore_ascii_case("vessel") {
        let diameter_m = spec.inlet_diameter_in * 0.0254 * 2.0;
        let height_m = diameter_m * 1.5;
        let volume = std::f64::consts::FRAC_PI_4 * diameter_m * diameter_m * height_m;
        let vessel = 400.0 * (volume * 1000.0).max(50.0).powf(0.55);
        let nozzles = (1.0 + spec.number_of_outlets) * 100.0 * spec.outlet_diameter_in.powf(0.8);
        (vessel + nozzles) * ratio
    } else {
        manifold()
    };
    cost *= common.material_factor;
    cost *= pressure;
    if spec.include_control_valves {
        let valve = control_valve(
            100.0 * spec.outlet_diameter_in,
            common.material_factor,
            ratio,
        );
        cost += spec.number_of_outlets * valve * common.material_factor * pressure;
    }
    if spec.include_flow_meters {
        let meter = 500.0 + 100.0 * spec.outlet_diameter_in;
        cost += spec.number_of_outlets * meter * ratio;
    }
    cost
}

/// The ported `SplitterMechanicalDesign.calculateSplitterCost` /
/// `MixerMechanicalDesign.calculateMixerCost`: header and branch piping plus a
/// 50 % fitting allowance.
#[must_use]
pub fn header_cost(
    header_diameter_m: f64,
    header_length_m: f64,
    branch_diameter_m: f64,
    branch_count: f64,
    material_factor: f64,
    cepci: f64,
) -> f64 {
    if header_diameter_m <= 0.0 {
        return 0.0;
    }
    let ratio = ratio_of(cepci);
    let header_per_m = crate::costing::correlations::piping_cost(
        header_diameter_m,
        1.0,
        40,
        material_factor,
        ratio,
    );
    let branch_per_m = crate::costing::correlations::piping_cost(
        branch_diameter_m,
        1.0,
        40,
        material_factor,
        ratio,
    );
    let header = header_per_m * header_length_m;
    let branch = branch_per_m * 0.5 * branch_count;
    header + branch + (header + branch) * 0.5
}

// ---------------------------------------------------------------------------
// Operating cost.
// ---------------------------------------------------------------------------

/// The annual operating cost, for the kinds that override the base 5 % of PEC.
#[must_use]
pub fn annual_operating_cost(spec: &EquipmentSpec, stack: &ModuleStack, opex: &OpexInputs) -> f64 {
    let pec = stack.purchased_equipment_cost;
    let hours = opex.hours_per_year;
    match spec {
        EquipmentSpec::Compressor(s) => {
            if s.power_kw <= 0.0 {
                return 0.0;
            }
            if s.driver_type.eq_ignore_ascii_case("gas-turbine") {
                let fuel_power = s.power_kw / 0.30;
                let energy_gj = fuel_power * hours * 3600.0 / 1e9;
                energy_gj * opex.fuel_cost_per_gj
            } else if s.driver_type.eq_ignore_ascii_case("gas-engine") {
                let fuel_power = s.power_kw / 0.38;
                let energy_gj = fuel_power * hours * 3600.0 / 1e9;
                energy_gj * opex.fuel_cost_per_gj
            } else if s.driver_type.eq_ignore_ascii_case("steam-turbine") {
                let steam_kg = s.power_kw * hours * 3.0;
                steam_kg * 10.0 / 1000.0
            } else {
                (s.power_kw / 0.95) * hours * opex.electricity_cost_per_kwh
            }
        }
        EquipmentSpec::Pump(s) => {
            if s.power_kw <= 0.0 {
                0.0
            } else {
                (s.power_kw / 0.9) * hours * opex.electricity_cost_per_kwh
            }
        }
        EquipmentSpec::Expander(s) => {
            let mut cost = pec * 0.03;
            if s.include_lube_oil_system {
                cost += s.shaft_power_kw / 1000.0 * 0.5 * hours * 3.0;
            }
            cost
        }
        EquipmentSpec::Tank(s) => {
            let mut cost = pec * 0.02;
            if s.include_heating_coils {
                let duty = s.volume_m3 * 0.01;
                cost += duty * hours * 0.08;
            }
            cost
        }
        EquipmentSpec::Column(s) => {
            let mut cost = 0.0;
            if s.include_reboiler && s.reboiler_duty_kw > 0.0 {
                let steam_flow = s.reboiler_duty_kw * 3600.0 / 2760.0;
                let tonnes = steam_flow * hours / 1000.0;
                cost += tonnes * opex.steam_cost_per_tonne;
            }
            if s.include_condenser && s.condenser_duty_kw > 0.0 {
                let flow = s.condenser_duty_kw * 3600.0 / (4.18 * 10.0);
                let m3 = flow * hours / 1000.0;
                cost += m3 * opex.cooling_water_cost_per_m3;
            }
            cost
        }
        EquipmentSpec::Absorber(s) => {
            let mut cost = 0.0;
            if s.include_reboiler {
                let steam = s.reboiler_duty_kw * 3600.0 / 2200.0;
                cost += steam * hours * opex.steam_cost_per_tonne / 1000.0;
            }
            if s.include_reflux_system {
                cost += 10.0 * hours * opex.electricity_cost_per_kwh;
            }
            cost + pec * 0.03
        }
        EquipmentSpec::Ejector(s) => {
            let mut cost = 0.0;
            if s.ejector_type.eq_ignore_ascii_case("steam") {
                let compression = if s.suction_pressure_mbar > 0.0 {
                    (s.discharge_pressure_bara * 1000.0) / s.suction_pressure_mbar
                } else {
                    0.0
                };
                let factor = 3.0 + 2.0 * compression.ln();
                let steam = s.suction_capacity_kg_per_h * factor;
                cost += steam * hours * opex.steam_cost_per_tonne / 1000.0;
            }
            if s.include_intercondensers || s.include_aftercondenser {
                let duty = s.suction_capacity_kg_per_h * 2.5;
                let flow = duty * 3600.0 / (4.18 * 10.0) / 1000.0;
                cost += flow * hours * opex.cooling_water_cost_per_m3;
            }
            cost + pec * 0.02
        }
        EquipmentSpec::MercuryRemoval(s) => {
            let replacement = s.sorbent_charge_weight_kg * s.sorbent_unit_price_usd_per_kg * 1.25;
            stack.total_module_cost * 0.03 + replacement / 5.0
        }
        _ => pec * 0.05,
    }
}
