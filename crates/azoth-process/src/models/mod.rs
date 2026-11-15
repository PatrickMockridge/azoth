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
pub mod active_fire_protection_screening;
pub mod anti_surge_recycle;
pub mod api537_flare_flame_model;
pub mod bleve;
pub mod bow_tie;
pub mod component_splitter;
pub mod compressor;
pub mod consequence_analysis;
pub mod cooler;
pub mod deluge_nozzle_layout;
pub mod distillation_column;
pub mod ejector;
pub mod event_tree;
pub mod expander;
pub mod fault_tree;
pub mod filter;
pub mod fire_case_relief;
pub mod fire_monitor_coverage;
pub mod fire_water_coverage_assessment;
pub mod fire_water_demand;
pub mod flare;
pub mod gas_dispersion;
pub mod gas_scrubber;
pub mod gaussian_plume;
pub mod gibbs_reactor;
pub mod heat_exchanger;
pub mod heater;
pub mod heavy_gas_dispersion;
pub mod hem_release;
pub mod jet_fire;
pub mod leak_blowdown;
pub mod lopa;
pub mod manifold;
pub mod map_turbo_expander_compressor;
pub mod mixer;
pub mod nog070_sil_determination;
pub mod overpressure_acceptance;
pub mod packed_column;
pub mod pfp_demand;
pub mod pipe;
pub mod pipeline_pressure_protection;
pub mod plug_flow_reactor;
pub mod pool_fire;
pub mod probit;
pub mod psv_fire_sizing;
pub mod psv_sizing;
pub mod pump;
pub mod rate_based_packed_column;
pub mod relief_disposal_network;
pub mod risk_matrix;
pub mod safety_instrumented_function;
pub mod separator;
pub mod separator_capacity_assessment;
pub mod separator_performance;
pub mod shortcut_distillation_column;
pub mod sil_verification;
pub mod splitter;
pub mod stirred_tank_reactor;
pub mod stripping_column;
pub mod tank;
pub mod three_phase_separator;
pub mod throttling_valve;
pub mod tr3001_compliance;
pub mod turbo_expander_compressor;
pub mod vce;

pub use absorption_column::{AbsorptionColumnResult, absorption_column};
pub use active_fire_protection_screening::{
    ActiveFireProtectionScreeningResult, active_fire_protection_screening,
};
pub use anti_surge_recycle::{AntiSurgeRecycleResult, anti_surge_recycle};
pub use api537_flare_flame_model::{
    Api537FlareFlameModelResult, FlareTargetFlux, SterileRadius, api537_flare_flame_model,
};
pub use bleve::{BleveResult, bleve};
pub use bow_tie::{BowTieBarrier, BowTieConsequence, BowTieResult, BowTieThreat, bow_tie};
pub use component_splitter::{ComponentSplitterResult, component_splitter};
pub use compressor::{CompressorResult, compressor};
pub use consequence_analysis::{
    ConsequenceAnalysisResult, ConsequenceOutcomeResult, ConsequenceOutcomeSpec,
    consequence_analysis,
};
pub use cooler::{CoolerResult, cooler};
pub use deluge_nozzle_layout::{
    DelugeNozzleLayoutResult, GoverningCriterion, deluge_nozzle_layout,
};
pub use distillation_column::{DistillationColumnResult, distillation_column};
pub use ejector::{EjectorResult, ejector};
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
pub use gas_dispersion::{GasDispersionResult, gas_dispersion};
pub use gas_scrubber::{GasScrubberResult, gas_scrubber};
pub use gaussian_plume::{GaussianPlumeResult, gaussian_plume};
pub use gibbs_reactor::{GibbsReactorResult, gibbs_reactor};
pub use heat_exchanger::{HeatExchangerResult, heat_exchanger};
pub use heater::{HeaterResult, heater};
pub use heavy_gas_dispersion::{HeavyGasDispersionResult, heavy_gas_dispersion};
pub use hem_release::{HemReleaseResult, hem_release};
pub use jet_fire::{JetFireResult, jet_fire};
pub use leak_blowdown::{LeakBlowdownResult, leak_blowdown};
pub use lopa::{
    LopaLayerContribution, LopaProtectionLayer, LopaResult, Sts0131PressureCategory, lopa,
};
pub use manifold::{ManifoldResult, manifold};
pub use map_turbo_expander_compressor::{MapTurboExpanderResult, map_turbo_expander_compressor};
pub use mixer::{MixerResult, mixer};
pub use nog070_sil_determination::{Nog070SilDeterminationResult, nog070_sil_determination};
pub use overpressure_acceptance::{OverpressureAcceptanceResult, overpressure_acceptance};
pub use packed_column::{PackedColumnResult, packed_column};
pub use pfp_demand::{PfpDemandResult, pfp_demand};
pub use pipe::{PipeResult, pipe};
pub use pipeline_pressure_protection::{
    PipelinePressureProtectionResult, pipeline_pressure_protection,
};
pub use plug_flow_reactor::{PlugFlowReactorResult, plug_flow_reactor};
pub use pool_fire::{PoolFireResult, pool_fire};
pub use probit::{ProbitResult, probit};
pub use psv_fire_sizing::{PsvFireSizingResult, psv_fire_sizing};
pub use psv_sizing::{PsvSizingResult, psv_sizing};
pub use pump::{PumpResult, pump};
pub use rate_based_packed_column::{RateBasedPackedColumnResult, rate_based_packed_column};
pub use relief_disposal_network::{
    ReliefCause, ReliefContribution, ReliefDisposalNetworkResult, ReliefDisposalRow, ReliefPhase,
    relief_disposal_network,
};
pub use risk_matrix::{
    RiskAssessmentRow, RiskConsequenceCategory, RiskEquipment, RiskLevel, RiskMatrixResult,
    RiskProbabilityCategory, risk_matrix,
};
pub use safety_instrumented_function::{
    SafetyInstrumentedFunctionResult, safety_instrumented_function,
};
pub use separator::{SeparatorResult, separator};
pub use separator_capacity_assessment::{
    SeparatorCapacityAssessmentResult, separator_capacity_assessment,
};
pub use separator_performance::{SeparatorPerformanceResult, separator_performance};
pub use shortcut_distillation_column::{
    ShortcutDistillationColumnResult, shortcut_distillation_column,
};
pub use sil_verification::{
    IssueCategory, IssueSeverity, SilIssue, SilVerificationResult, sil_verification,
};
pub use splitter::{SplitterResult, splitter};
pub use stirred_tank_reactor::{StirredTankReactorResult, stirred_tank_reactor};
pub use stripping_column::{StrippingColumnResult, stripping_column};
pub use tank::{TankResult, tank};
pub use three_phase_separator::{ThreePhaseSeparatorResult, three_phase_separator};
pub use throttling_valve::{ThrottlingValveResult, throttling_valve};
pub use tr3001_compliance::{
    Tr3001Check, Tr3001ComplianceResult, Tr3001Finding, Tr3001Scenario, Tr3001Status,
    tr3001_compliance,
};
pub use turbo_expander_compressor::{TurboExpanderResult, turbo_expander_compressor};
pub use vce::{VceResult, vce};
