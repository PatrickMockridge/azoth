//! The process layer: unit operations and flowsheets, on typed, directional
//! channels.
//!
//! A unit operation is a process with inlets and outlets; a channel carries a
//! field record (names → dimensions) with a polarity; conservation is linearity.
//! This crate states the schema — the palette of unit operations and the
//! flowsheet that wires them — and checks that a flowsheet honours the calculus's
//! rules. **The executor that turns a flowsheet into something that runs is
//! [`executor`]**, and a flowsheet is self-contained: its `[[inputs]]` declare the
//! fluid and the state, so it runs with no argument but the document.

pub mod channel;
pub mod check;
pub mod column;
pub mod compressor_chart;
pub mod executor;
pub mod expander_chart;
pub mod flowsheet;
pub mod kernels;
pub mod load;
pub mod middleware;
pub mod model_gen;
pub mod model_inputs_gen;
pub mod model_outputs_gen;
pub mod model_records_gen;
pub mod models;
pub mod order;
pub mod palette_gen;
pub mod pump_chart;
pub mod reactor;
pub mod recycle;
pub mod rotating_design;
pub mod segment;
pub mod separator_design;
pub mod stream;
pub mod surge_curve;
pub mod unit_op;
pub mod unported;
pub mod unported_gen;
pub mod valve;

pub use channel::{Direction, FieldType, Multiplicity, Port, Shape};
pub use check::{
    Diagnostic, Location, NodeRole, Severity, Target, split_node_id, validate, validate_palette,
};
pub use flowsheet::{Connection, Flowsheet, Input, Instance, Layout, Recycle};
pub use load::{load_palette, load_palette_text, parse_flowsheet};
pub use order::{ExecutionOrder, execution_order};
pub use recycle::{Acceleration, RecycleSettings, Residuals};
// **The kernels stay under `kernels` and the ids take the flat names**, which is the split
// the Python package makes too. Two call shapes under one name is not a naming problem to
// work around: a kernel takes and returns a `Stream`, which is what a flowsheet's
// connection carries, while a model takes the record field by field so that a case, a
// cross-impl test and a NeqSim capture can address it.
pub use models::{
    AbsorptionColumnResult, ActiveFireProtectionScreeningResult, AntiSurgeRecycleResult,
    Api14cSafeChartResult, Api537FlareFlameModelResult, BasketTestRegressionResult, BleveResult,
    BlockedOutletReliefResult, BlowdownPressureProfileResult, BowTieResult,
    CheckValveLeakReliefResult, ComponentSplitterResult, CompressorResult,
    ConsequenceAnalysisResult, ControlValveFailureReliefResult, CoolerResult,
    DelugeNozzleLayoutResult, DistillationColumnResult, EjectorResult, EventTreeResult,
    ExpanderResult, FaultTreeResult, FilterResult, FireCaseReliefResult, FireMonitorCoverageResult,
    FireWaterCoverageAssessmentResult, FireWaterDemandResult, FlareResult, FmeaWorksheetResult,
    GasDispersionResult, GasScrubberResult, GaussianPlumeResult, GibbsReactorResult,
    HazardousAreaResult, HazopConsequencePopulationResult, HazopTemplateResult,
    HeatExchangerResult, HeaterResult, HeavyGasDispersionResult, HemReleaseResult, JetFireResult,
    LeakBlowdownResult, LopaResult, MahBowTieSkeletonResult, ManifoldResult,
    MapTurboExpanderResult, MixerResult, Nog070SilDeterminationResult,
    OverpressureAcceptanceResult, OverpressureProtectionStudyResult, PackedColumnResult,
    PfpDemandResult, PipeFireRuptureMaterialResult, PipeFireRuptureResult, PipeResult,
    PipelinePressureProtectionResult, PlugFlowReactorResult, PoolFireResult,
    PorousMediaSelfHeatingResult, ProbitResult, PsvFireSizingResult, PsvSizingResult, PumpResult,
    RateBasedPackedColumnResult, ReliefDisposalNetworkResult, RiskMatrixResult,
    SafetyInstrumentedFunctionResult, SelfHeatingInductionResult, SemenovSelfHeatingResult,
    SeparatorPerformanceResult, SeparatorResult, ShortcutDistillationColumnResult,
    SilVerificationResult, SplitterResult, StirredTankReactorResult, StrippingColumnResult,
    TankResult, ThreePhaseSeparatorResult, ThrottlingValveResult, ToxicLibraryResult,
    Tr3001ComplianceResult, TubeRuptureReliefResult, TurboExpanderResult, VceResult,
    VesselRuptureResult, absorption_column, active_fire_protection_screening, anti_surge_recycle,
    api14c_safe_chart, api537_flare_flame_model, basket_test_regression, bleve,
    blocked_outlet_relief, blowdown_pressure_profile, bow_tie, check_valve_leak_relief,
    component_splitter, compressor, consequence_analysis, control_valve_failure_relief, cooler,
    deluge_nozzle_layout, distillation_column, ejector, event_tree, expander, fault_tree, filter,
    fire_case_relief, fire_monitor_coverage, fire_water_coverage_assessment, fire_water_demand,
    flare, fmea_worksheet, gas_dispersion, gas_scrubber, gaussian_plume, gibbs_reactor,
    hazardous_area, hazop_consequence_population, hazop_template, heat_exchanger, heater,
    heavy_gas_dispersion, hem_release, jet_fire, leak_blowdown, lopa, mah_bow_tie_skeleton,
    manifold, map_turbo_expander_compressor, mixer, nog070_sil_determination,
    overpressure_acceptance, overpressure_protection_study, packed_column, pfp_demand, pipe,
    pipe_fire_rupture, pipe_fire_rupture_material, pipeline_pressure_protection, plug_flow_reactor,
    pool_fire, porous_media_self_heating, probit, psv_fire_sizing, psv_sizing, pump,
    rate_based_packed_column, relief_disposal_network, risk_matrix, safety_instrumented_function,
    self_heating_induction, semenov_self_heating, separator, separator_capacity_assessment,
    separator_performance, shortcut_distillation_column, sil_verification, splitter,
    stirred_tank_reactor, stripping_column, tank, three_phase_separator, throttling_valve,
    toxic_library, tr3001_compliance, tube_rupture_relief, turbo_expander_compressor, vce,
    vessel_rupture,
};
pub use stream::Stream;
pub use unit_op::{Param, Source, UnitOpSpec};
