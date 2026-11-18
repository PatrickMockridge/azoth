//! The registry surface: one module per unit operation, wrapping its kernel.
//!
//! A kernel ([`crate::kernels`]) is the flowsheet's arithmetic — it reads and writes
//! [`crate::stream::Stream`] values, which is what a connection carries. A model is the
//! same unit operation as an id the rest of the library can address: a declared input
//! list, a [`CalcResult`](azoth_core::CalcResult) impl, a cross-implementation test and a
//! NeqSim oracle.
//!
//! **Two surfaces, one arithmetic**, and the split is deliberate: the executor P12 will
//! build calls kernels, while a case, a cross-impl test and an oracle capture call models.
//! Neither wraps the other's types, so a change to the flowsheet's stream record cannot
//! silently move a published id.
//!
//! # How a port crosses
//!
//! A port carries the record `n, z, P, T, h`. The fluid's `components` is shared by every
//! port of a unit operation, so it is declared once; each port then contributes
//! `<port>_n`, `<port>_z`, `<port>_p`, `<port>_t`, and — on an outlet — `<port>_h`.
//!
//! **An inlet takes four of the record's five fields, and that is stated rather than
//! assumed.** `h` is a state function of `(T, P, z)`: the same flash that `Stream::from_pt`
//! runs decides it. Accepting an `h` as well would let a case hand over a state that does
//! not exist, so the inlet's enthalpy is computed and an outlet's is reported.
//!
//! **A `many` port crosses as a matrix and vectors.** `z` becomes one row per stream and
//! each scalar field one entry per stream, which is the port's multiplicity written out —
//! the calculus makes the multiplicity part of the declaration an implementation must
//! agree with, so folding a mixer to a fixed two inlets would answer a smaller question.
//! [`splitter`] is the first id with such a port, and its result is the shape in full:
//! `products_n`, `products_p`, `products_t` and `products_h` are vectors with one entry
//! per outlet and `products_z` a matrix with one row per outlet.

pub mod absorption_column;
pub mod acoustic_induced_vibration_screening;
pub mod active_fire_protection_screening;
pub mod alarp_audit;
pub mod anti_surge_recycle;
pub mod api14c_safe_chart;
pub mod api537_flare_flame_model;
pub mod barrier_register_validation;
pub mod basket_test_regression;
pub mod bleve;
pub mod blocked_outlet_relief;
pub mod blowdown_pressure_profile;
pub mod bow_tie;
pub mod check_valve_leak_relief;
pub mod component_splitter;
pub mod compressor;
pub mod consequence_analysis;
pub mod control_valve_failure_relief;
pub mod cooler;
pub mod deluge_nozzle_layout;
pub mod distillation_column;
pub mod dynamic_psv_sizing;
pub mod ejector;
pub mod escalation_graph;
pub mod event_tree;
pub mod expander;
pub mod fault_tree;
pub mod filter;
pub mod fire_case_relief;
pub mod fire_monitor_coverage;
pub mod fire_water_coverage_assessment;
pub mod fire_water_demand;
pub mod flare;
pub mod flow_induced_pulsation_screening;
pub mod fmea_worksheet;
pub mod gas_blowby;
pub mod gas_dispersion;
pub mod gas_scrubber;
pub mod gaussian_plume;
pub mod gibbs_reactor;
pub mod hazardous_area;
pub mod hazid;
pub mod hazop_consequence_population;
pub mod hazop_template;
pub mod heat_exchanger;
pub mod heater;
pub mod heavy_gas_dispersion;
pub mod hem_release;
pub mod inherent_safety;
pub mod jet_fire;
pub mod leak_blowdown;
pub mod lopa;
pub mod mah_bow_tie_skeleton;
pub mod manifold;
pub mod map_turbo_expander_compressor;
pub mod mdmt;
pub mod mixer;
pub mod nog070_sil_determination;
pub mod norsok_p002_compliance;
pub mod overpressure;
pub mod overpressure_acceptance;
pub mod overpressure_protection_study;
pub mod packed_column;
pub mod pfp_demand;
pub mod pipe;
pub mod pipe_fire_rupture;
pub mod pipe_fire_rupture_material;
pub mod pipeline_pressure_protection;
pub mod piping_fiv_gas;
pub mod piping_fiv_liquid;
pub mod plug_flow_reactor;
pub mod pool_fire;
pub mod porous_media_self_heating;
pub mod probit;
pub mod psv_fire_sizing;
pub mod psv_sizing;
pub mod psv_valve;
pub mod pump;
pub mod pump_deadhead;
pub mod rate_based_packed_column;
pub mod relief_disposal_network;
pub mod risk_matrix;
pub mod runaway_reaction;
pub mod s001_clause10_review;
pub mod s001_secondary_pressure_protection;
pub mod safety_instrumented_function;
pub mod safety_review;
pub mod self_heating_induction;
pub mod selfheating;
pub mod semenov_self_heating;
pub mod separator;
pub mod separator_capacity_assessment;
pub mod separator_performance;
pub mod settle_out_pressure;
pub mod shortcut_distillation_column;
pub mod sil_verification;
pub mod splitter;
pub mod stirred_tank_reactor;
pub mod stripping_column;
pub mod sts0131_gate;
pub mod tank;
pub mod three_phase_separator;
pub mod throttling_valve;
pub mod toxic_library;
pub mod tr3001_compliance;
pub mod tube_rupture_relief;
pub mod turbo_expander_compressor;
pub mod vacuum_collapse;
pub mod vce;
pub mod vessel_rupture;

pub use absorption_column::{AbsorptionColumnResult, absorption_column};
pub use acoustic_induced_vibration_screening::{
    AcousticInducedVibrationResult, acoustic_induced_vibration_screening,
};
pub use active_fire_protection_screening::{
    ActiveFireProtectionScreeningResult, active_fire_protection_screening,
};
pub use alarp_audit::{
    AlarpAuditResult, AlarpEvaluationResult, AlarpMeasure, AlarpVerdict, alarp_audit,
};
pub use anti_surge_recycle::{AntiSurgeRecycleResult, anti_surge_recycle};
pub use api14c_safe_chart::{
    Api14cSafeChartResult, SafeChartEquipment, SafeChartItem, SafeChartMissingDevice,
    SafeChartPresentDevice, api14c_safe_chart,
};
pub use api537_flare_flame_model::{
    Api537FlareFlameModelResult, FlareTargetFlux, SterileRadius, api537_flare_flame_model,
};
pub use barrier_register_validation::{
    BarrierElement, BarrierRegisterValidationResult, BarrierRow, ElementBarrierLink,
    PerformanceStandardRow, RegisterEvidence, RegisterFinding, barrier_register_validation,
};
pub use basket_test_regression::{
    BasketTestPoint, BasketTestRegressionResult, basket_test_regression,
};
pub use bleve::{BleveResult, bleve};
pub use blocked_outlet_relief::{BlockedOutletReliefResult, blocked_outlet_relief};
pub use blowdown_pressure_profile::{
    BlowdownPressureProfileResult, BlowdownProfilePoint, blowdown_pressure_profile,
};
pub use bow_tie::{BowTieBarrier, BowTieConsequence, BowTieResult, BowTieThreat, bow_tie};
pub use check_valve_leak_relief::{CheckValveLeakReliefResult, check_valve_leak_relief};
pub use component_splitter::{ComponentSplitterResult, component_splitter};
pub use compressor::{CompressorResult, compressor};
pub use consequence_analysis::{
    ConsequenceAnalysisResult, ConsequenceOutcomeResult, ConsequenceOutcomeSpec,
    consequence_analysis,
};
pub use control_valve_failure_relief::{
    ControlValveFailureReliefResult, control_valve_failure_relief,
};
pub use cooler::{CoolerResult, cooler};
pub use deluge_nozzle_layout::{
    DelugeNozzleLayoutResult, GoverningCriterion, deluge_nozzle_layout,
};
pub use distillation_column::{DistillationColumnResult, distillation_column};
pub use dynamic_psv_sizing::{DynamicPsvSizingResult, dynamic_psv_sizing};
pub use ejector::{EjectorResult, ejector};
pub use escalation_graph::{
    EscalationExposure, EscalationGraphResult, EscalationItem, EscalationItemResult,
    escalation_graph,
};
pub use event_tree::{EventTreeBranch, EventTreeOutcome, EventTreeResult, event_tree};
pub use expander::{ExpanderResult, expander};
pub use fault_tree::{FaultTreeNodeSpec, FaultTreeResult, MinimalCutSet, fault_tree};
pub use filter::{FilterResult, filter};
pub use fire_case_relief::{FireCaseReliefResult, fire_case_relief};
pub use fire_monitor_coverage::{FireMonitorCoverageResult, fire_monitor_coverage};
pub use fire_water_coverage_assessment::{
    FireWaterCoverageAssessmentResult, SupplyVerdict, fire_water_coverage_assessment,
};
pub use fire_water_demand::{FireWaterDemandResult, ProtectedObject, fire_water_demand};
pub use flare::{FlareResult, flare};
pub use flow_induced_pulsation_screening::{
    FlowInducedPulsationResult, PulsationModeResult, flow_induced_pulsation_screening,
};
pub use fmea_worksheet::{FmeaEntry, FmeaRankedEntry, FmeaWorksheetResult, fmea_worksheet};
pub use gas_blowby::{BlowbyVerdict, GasBlowbyResult, gas_blowby};
pub use gas_dispersion::{GasDispersionResult, gas_dispersion};
pub use gas_scrubber::{GasScrubberResult, gas_scrubber};
pub use gaussian_plume::{GaussianPlumeResult, gaussian_plume};
pub use gibbs_reactor::{GibbsReactorResult, gibbs_reactor};
pub use hazardous_area::{HazardousAreaResult, hazardous_area};
pub use hazid::{HazopGuideWord, HazopParameter};
pub use hazop_consequence_population::{
    HazopConsequencePopulationResult, HazopDeviationInput, HazopPopulationRow,
    hazop_consequence_population,
};
pub use hazop_template::{
    HazopGridParameter, HazopTemplateDeviation, HazopTemplateResult, hazop_template,
};
pub use heat_exchanger::{HeatExchangerResult, heat_exchanger};
pub use heater::{HeaterResult, heater};
pub use heavy_gas_dispersion::{HeavyGasDispersionResult, heavy_gas_dispersion};
pub use hem_release::{HemReleaseResult, hem_release};
pub use inherent_safety::{InherentSafetyResult, inherent_safety};
pub use jet_fire::{JetFireResult, jet_fire};
pub use leak_blowdown::{LeakBlowdownResult, leak_blowdown};
pub use lopa::{
    LopaLayerContribution, LopaProtectionLayer, LopaResult, Sts0131PressureCategory, lopa,
};
pub use mah_bow_tie_skeleton::{
    MahBarrier, MahBowTieSkeletonResult, MahConsequence, MahThreat, mah_bow_tie_skeleton,
};
pub use manifold::{ManifoldResult, manifold};
pub use map_turbo_expander_compressor::{MapTurboExpanderResult, map_turbo_expander_compressor};
pub use mdmt::{MdmtResult, mdmt};
pub use mixer::{MixerResult, mixer};
pub use nog070_sil_determination::{Nog070SilDeterminationResult, nog070_sil_determination};
pub use norsok_p002_compliance::{
    NorsokP002ComplianceResult, P002CheckRow, P002Criterion, P002FindingResult,
    norsok_p002_compliance,
};
pub use overpressure_acceptance::{OverpressureAcceptanceResult, overpressure_acceptance};
pub use overpressure_protection_study::{
    OverpressureProtectionStudyResult, OverpressureScenario, overpressure_protection_study,
};
pub use packed_column::{PackedColumnResult, packed_column};
pub use pfp_demand::{PfpDemandResult, pfp_demand};
pub use pipe::{PipeResult, pipe};
pub use pipe_fire_rupture::{
    FluidBasis, PipeFireRupturePoint, PipeFireRuptureResult, RuptureStatus, pipe_fire_rupture,
};
pub use pipe_fire_rupture_material::{
    PipeFireRuptureMaterialPoint, PipeFireRuptureMaterialResult, pipe_fire_rupture_material,
};
pub use pipeline_pressure_protection::{
    PipelinePressureProtectionResult, pipeline_pressure_protection,
};
pub use piping_fiv_gas::{FivLikelihood, PipingFivGasResult, piping_fiv_gas};
pub use piping_fiv_liquid::{PipingFivLiquidResult, piping_fiv_liquid};
pub use plug_flow_reactor::{PlugFlowReactorResult, plug_flow_reactor};
pub use pool_fire::{PoolFireResult, pool_fire};
pub use porous_media_self_heating::{PorousMediaSelfHeatingResult, porous_media_self_heating};
pub use probit::{ProbitResult, probit};
pub use psv_fire_sizing::{PsvFireSizingResult, psv_fire_sizing};
pub use psv_sizing::{PsvSizingResult, psv_sizing};
pub use psv_valve::{PsvValveResult, psv_valve};
pub use pump::{PumpResult, pump};
pub use pump_deadhead::{DeadheadVerdict, PumpDeadheadResult, pump_deadhead};
pub use rate_based_packed_column::{RateBasedPackedColumnResult, rate_based_packed_column};
pub use relief_disposal_network::{
    ReliefCause, ReliefContribution, ReliefDisposalNetworkResult, ReliefDisposalRow, ReliefPhase,
    relief_disposal_network,
};
pub use risk_matrix::{
    RiskAssessmentRow, RiskConsequenceCategory, RiskEquipment, RiskLevel, RiskMatrixResult,
    RiskProbabilityCategory, risk_matrix,
};
pub use runaway_reaction::{RunawayReactionResult, RunawayVerdict, runaway_reaction};
pub use s001_clause10_review::{
    OverallVerdict, ReviewAssessment, ReviewCoverage, ReviewEvidence, ReviewItem, ReviewReference,
    ReviewVerdict, S001Clause10ReviewResult, s001_clause10_review,
};
pub use s001_secondary_pressure_protection::{
    S001SecondaryPressureProtectionResult, s001_secondary_pressure_protection,
};
pub use safety_instrumented_function::{
    SafetyInstrumentedFunctionResult, safety_instrumented_function,
};
pub use safety_review::{Api14cCategory, Api14cDevice};
pub use self_heating_induction::{
    SelfHeatingInductionResult, SelfHeatingTimePoint, self_heating_induction,
};
pub use selfheating::{SelfHeatingGeometry, SelfHeatingVerdict};
pub use semenov_self_heating::{SemenovSelfHeatingResult, semenov_self_heating};
pub use separator::{SeparatorResult, separator};
pub use separator_capacity_assessment::{
    SeparatorCapacityAssessmentResult, separator_capacity_assessment,
};
pub use separator_performance::{SeparatorPerformanceResult, separator_performance};
pub use settle_out_pressure::{
    SettleOutCompartment, SettleOutCompartmentResult, SettleOutPressureResult, SettleOutVerdict,
    settle_out_pressure,
};
pub use shortcut_distillation_column::{
    ShortcutDistillationColumnResult, shortcut_distillation_column,
};
pub use sil_verification::{
    IssueCategory, IssueSeverity, SilIssue, SilVerificationResult, sil_verification,
};
pub use splitter::{SplitterResult, splitter};
pub use stirred_tank_reactor::{StirredTankReactorResult, stirred_tank_reactor};
pub use stripping_column::{StrippingColumnResult, stripping_column};
pub use sts0131_gate::{
    Sts0131CheckRow, Sts0131CheckType, Sts0131FindingResult, Sts0131GateResult, Sts0131Severity,
    sts0131_gate,
};
pub use tank::{TankResult, tank};
pub use three_phase_separator::{ThreePhaseSeparatorResult, three_phase_separator};
pub use throttling_valve::{ThrottlingValveResult, throttling_valve};
pub use toxic_library::{ToxicLibraryResult, toxic_library};
pub use tr3001_compliance::{
    Tr3001Check, Tr3001ComplianceResult, Tr3001Finding, Tr3001Scenario, Tr3001Status,
    tr3001_compliance,
};
pub use tube_rupture_relief::{TubeRuptureReliefResult, tube_rupture_relief};
pub use turbo_expander_compressor::{TurboExpanderResult, turbo_expander_compressor};
pub use vacuum_collapse::{VacuumCollapseResult, VacuumVerdict, vacuum_collapse};
pub use vce::{VceResult, vce};
pub use vessel_rupture::{
    VesselRuptureResult, VesselRuptureState, VesselRuptureStep, vessel_rupture,
};
