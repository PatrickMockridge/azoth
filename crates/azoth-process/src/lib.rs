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
    AbsorptionColumnResult, AcousticInducedVibrationResult, ActiveFireProtectionScreeningResult,
    AlarpAuditResult, AntiSurgeRecycleResult, Api14cSafeChartResult, Api537FlareFlameModelResult,
    BarrierRegisterValidationResult, BasketTestRegressionResult, BleveResult,
    BlockedOutletOverpressureResult, BlockedOutletReliefResult, BlowdownPressureProfileResult,
    BowTieResult, CheckValveLeakReliefResult, ComponentSelectiveReleaseResult,
    ComponentSplitterResult, CompressorResult, ConsequenceAnalysisResult,
    ControlValveFailureReliefResult, CoolerResult, DelugeNozzleLayoutResult,
    DepressurizationAcceptancePoint, DepressurizationAcceptanceResult, DepressurizationPoint,
    DepressurizationResult, DistillationColumnResult, DriftFluxReleaseResult,
    DynamicPsvSizingResult, EjectorResult, EscalationGraphResult, EventTreeResult, ExpanderResult,
    FaultTreeResult, FilterResult, FiniteRateReleaseResult, FireCaseReliefResult,
    FireMonitorCoverageResult, FireWaterCoverageAssessmentResult, FireWaterDemandResult,
    FlareResult, FlowInducedPulsationResult, FmeaWorksheetResult, GasDispersionResult,
    GasScrubberResult, GaussianPlumeResult, GibbsReactorResult, HazardousAreaResult,
    HazopConsequencePopulationResult, HazopTemplateResult, HeatExchangerResult, HeaterResult,
    HeavyGasDispersionResult, HemReleaseResult, IdealGasReleaseResult, InherentSafetyResult,
    JetFireResult, LeakBlowdownResult, LegacyScreeningReleaseResult, LopaResult,
    MahBowTieSkeletonResult, ManifoldResult, MapTurboExpanderResult, MdmtResult, MixerResult,
    Nog070SilDeterminationResult, NorsokP002ComplianceResult, OverpressureAcceptanceResult,
    OverpressureProtectionStudyResult, PackedColumnResult, PfpDemandResult,
    PipeFireRuptureMaterialResult, PipeFireRuptureResult, PipeResult,
    PipelinePressureProtectionResult, PipingFivGasResult, PipingFivLiquidResult,
    PlugFlowReactorResult, PoolFireResult, PorousMediaSelfHeatingResult,
    PrescribedSlipReleaseResult, ProbitResult, PsvFireSizingResult, PsvSizingResult,
    PsvValveResult, PumpDeadheadResult, PumpResult, RateBasedPackedColumnResult,
    ReliefDisposalNetworkResult, RiskMatrixResult, RunawayReactionResult, S001Clause10ReviewResult,
    S001SecondaryPressureProtectionResult, SafetyInstrumentedFunctionResult,
    SelfHeatingInductionResult, SemenovSelfHeatingResult, SeparatorPerformanceResult,
    SeparatorResult, SettleOutPressureResult, ShortcutDistillationColumnResult,
    SilVerificationResult, SplitterResult, StirredTankReactorResult, StrippingColumnResult,
    Sts0131GateResult, TankResult, ThreePhaseSeparatorResult, ThrottlingValveResult,
    ToxicLibraryResult, Tr3001ComplianceResult, TubeRuptureReliefResult, TurboExpanderResult,
    VacuumCollapseResult, VceResult, VesselFillingPoint, VesselFillingResult, VesselRuptureResult,
    absorption_column, acoustic_induced_vibration_screening, active_fire_protection_screening,
    alarp_audit, anti_surge_recycle, api14c_safe_chart, api537_flare_flame_model,
    barrier_register_validation, basket_test_regression, bleve, blocked_outlet_relief,
    blowdown_pressure_profile, bow_tie, check_valve_leak_relief, component_selective_release,
    component_splitter, compressor, consequence_analysis, control_valve_failure_relief, cooler,
    deluge_nozzle_layout, depressurization, depressurization_acceptance, distillation_column,
    drift_flux_release, dynamic_psv_sizing, ejector, escalation_graph, event_tree, expander,
    fault_tree, filter, finite_rate_release, fire_case_relief, fire_monitor_coverage,
    fire_water_coverage_assessment, fire_water_demand, flare, flow_induced_pulsation_screening,
    fmea_worksheet, gas_blowby, gas_dispersion, gas_scrubber, gaussian_plume, gibbs_reactor,
    hazardous_area, hazop_consequence_population, hazop_template, heat_exchanger, heater,
    heavy_gas_dispersion, hem_release, ideal_gas_release, inherent_safety, jet_fire, leak_blowdown,
    legacy_screening_release, lopa, mah_bow_tie_skeleton, manifold, map_turbo_expander_compressor,
    mdmt, mixer, nog070_sil_determination, norsok_p002_compliance, overpressure_acceptance,
    overpressure_protection_study, packed_column, pfp_demand, pipe, pipe_fire_rupture,
    pipe_fire_rupture_material, pipeline_pressure_protection, piping_fiv_gas, piping_fiv_liquid,
    plug_flow_reactor, pool_fire, porous_media_self_heating, prescribed_slip_release, probit,
    psv_fire_sizing, psv_sizing, psv_valve, pump, pump_deadhead, rate_based_packed_column,
    relief_disposal_network, risk_matrix, runaway_reaction, s001_clause10_review,
    s001_secondary_pressure_protection, safety_instrumented_function, self_heating_induction,
    semenov_self_heating, separator, separator_capacity_assessment, separator_performance,
    settle_out_pressure, shortcut_distillation_column, sil_verification, splitter,
    stirred_tank_reactor, stripping_column, sts0131_gate, tank, three_phase_separator,
    throttling_valve, toxic_library, tr3001_compliance, tube_rupture_relief,
    turbo_expander_compressor, vacuum_collapse, vce, vessel_filling, vessel_rupture,
};
pub use stream::Stream;
pub use unit_op::{Param, Source, UnitOpSpec};
