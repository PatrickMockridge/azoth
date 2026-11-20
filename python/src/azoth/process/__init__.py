"""The process layer: unit operations and flowsheets.

The ``process.*`` ids, one per palette entry under ``specs/unit_ops/``, and the check
that holds a flowsheet to the calculus's rules. Each id is a registered model: a spec
under ``specs/models/process/``, a kernel in Rust and a reference here, compared by
``test_cross_impl`` against a committed NeqSim capture.

The Stream-level arithmetic those models wrap is :mod:`azoth.process.kernels`, which is what
a flowsheet's executor calls and what ``azoth check`` validates the wiring of. To *run* one,
:func:`run_flowsheet` in :mod:`azoth.process.flowsheet`; to hold one and edit it,
:class:`Session` in the same module — the middleware's live document, which is also what
:func:`forms` and :func:`tools` describe.
"""

from __future__ import annotations

import pathlib

from azoth import _core
from azoth._dispatch import resolve
from azoth.core.result import (
    AbsorptionColumnResult,
    AcousticInducedVibrationResult,
    ActiveFireProtectionScreeningResult,
    AlarpAuditResult,
    AntiSurgeRecycleResult,
    Api14cSafeChartResult,
    Api537FlareFlameModelResult,
    BarrierRegisterValidationResult,
    BasketTestRegressionResult,
    BleveResult,
    BlockedOutletOverpressureResult,
    BlockedOutletReliefResult,
    BlowdownPressureProfileResult,
    BowTieResult,
    BubbleTransferZoneReleaseResult,
    CheckValveLeakReliefResult,
    ComponentSelectiveReleaseResult,
    ComponentSplitterResult,
    CompressorResult,
    ConsequenceAnalysisResult,
    ControlValveFailureReliefResult,
    CoolerResult,
    DelugeNozzleLayoutResult,
    DepressurizationAcceptanceResult,
    DepressurizationResult,
    DistillationColumnResult,
    DriftFluxReleaseResult,
    DynamicPsvSizingResult,
    EjectorResult,
    EscalationGraphResult,
    EsdResponseTimeResult,
    EventTreeResult,
    ExpanderResult,
    FaultTreeResult,
    FilterResult,
    FiniteRateReleaseResult,
    FireCaseReliefResult,
    FireMonitorCoverageResult,
    FireWaterCoverageAssessmentResult,
    FireWaterDemandResult,
    FlareResult,
    FlowInducedPulsationResult,
    FmeaWorksheetResult,
    GasBlowbyResult,
    GasDispersionResult,
    GasScrubberResult,
    GaussianPlumeResult,
    GibbsReactorResult,
    HazardousAreaResult,
    HazopConsequencePopulationResult,
    HazopTemplateResult,
    HeaterResult,
    HeatExchangerResult,
    HeavyGasDispersionResult,
    HemReleaseResult,
    IdealGasFannoReleaseResult,
    IdealGasReleaseResult,
    InherentSafetyResult,
    JetFireResult,
    LeakBlowdownResult,
    LegacyScreeningReleaseResult,
    LopaResult,
    MahBowTieSkeletonResult,
    ManifoldResult,
    MapTurboExpanderResult,
    MassBalanceLeakDetectionResult,
    MdmtResult,
    MixerResult,
    MultiVesselBlowdownStudyResult,
    Nog070SilDeterminationResult,
    NonEquilibriumBlowdownResult,
    NorsokP002ComplianceResult,
    OverpressureAcceptanceResult,
    OverpressureProtectionStudyResult,
    PackedColumnResult,
    PfpDemandResult,
    PipeFireRuptureMaterialResult,
    PipeFireRuptureResult,
    PipelinePressureProtectionResult,
    PipeResult,
    PipingFivGasResult,
    PipingFivLiquidResult,
    PlugFlowReactorResult,
    PoolFireResult,
    PorousMediaSelfHeatingResult,
    PrescribedSlipReleaseResult,
    ProbitResult,
    PsvFireSizingResult,
    PsvSizingResult,
    PsvValveResult,
    PumpDeadheadResult,
    PumpResult,
    RanzMarshallReleaseResult,
    RateBasedPackedColumnResult,
    RealGasFannoReleaseResult,
    ReliefDisposalNetworkResult,
    RiskMatrixResult,
    RunawayReactionResult,
    S001Clause10ReviewResult,
    S001SecondaryPressureProtectionResult,
    SafetyInstrumentedFunctionResult,
    SelfHeatingInductionResult,
    SemenovSelfHeatingResult,
    SeparatorCapacityAssessmentResult,
    SeparatorPerformanceResult,
    SeparatorResult,
    SettleOutPressureResult,
    ShortcutDistillationColumnResult,
    SilVerificationResult,
    SplitterResult,
    StirredTankReactorResult,
    StrippingColumnResult,
    Sts0131GateResult,
    TankResult,
    ThreePhaseSeparatorResult,
    ThrottlingValveResult,
    ToxicLibraryResult,
    Tr3001ComplianceResult,
    TrappedInventoryResult,
    TubeRuptureReliefResult,
    TurboExpanderResult,
    VacuumCollapseResult,
    VceResult,
    VesselFillingResult,
    VesselRuptureResult,
)
from azoth.core.units import Q
from azoth.process.flowsheet import (
    FlowsheetResult,
    FlowsheetStream,
    Residuals,
    Session,
    TearResult,
    catalogue,
    forms,
    run_flowsheet,
    tools,
)
from azoth.process.kernels import Stream

__all__ = [
    "FlowsheetResult",
    "FlowsheetStream",
    "Residuals",
    "Session",
    "Stream",
    "TearResult",
    "absorption_column",
    "acoustic_induced_vibration_screening",
    "active_fire_protection_screening",
    "alarp_audit",
    "anti_surge_recycle",
    "api14c_safe_chart",
    "api537_flare_flame_model",
    "barrier_register_validation",
    "bleve",
    "blowdown_pressure_profile",
    "bow_tie",
    "catalogue",
    "component_splitter",
    "compressor",
    "consequence_analysis",
    "cooler",
    "deluge_nozzle_layout",
    "distillation_column",
    "ejector",
    "escalation_graph",
    "event_tree",
    "expander",
    "fault_tree",
    "filter",
    "fire_case_relief",
    "fire_monitor_coverage",
    "fire_water_coverage_assessment",
    "fire_water_demand",
    "flare",
    "flow_induced_pulsation_screening",
    "forms",
    "gas_blowby",
    "gas_dispersion",
    "gas_scrubber",
    "gaussian_plume",
    "gibbs_reactor",
    "hazardous_area",
    "heat_exchanger",
    "heater",
    "heavy_gas_dispersion",
    "hem_release",
    "inherent_safety",
    "jet_fire",
    "leak_blowdown",
    "load_flowsheet",
    "lopa",
    "manifold",
    "map_turbo_expander_compressor",
    "mdmt",
    "mixer",
    "nog070_sil_determination",
    "norsok_p002_compliance",
    "overpressure_acceptance",
    "packed_column",
    "pfp_demand",
    "pipe",
    "pipe_fire_rupture",
    "pipe_fire_rupture_material",
    "pipeline_pressure_protection",
    "piping_fiv_gas",
    "piping_fiv_liquid",
    "plug_flow_reactor",
    "pool_fire",
    "probit",
    "psv_fire_sizing",
    "psv_sizing",
    "pump",
    "pump_deadhead",
    "rate_based_packed_column",
    "relief_disposal_network",
    "risk_matrix",
    "run_flowsheet",
    "runaway_reaction",
    "s001_clause10_review",
    "s001_secondary_pressure_protection",
    "safety_instrumented_function",
    "separator",
    "separator_capacity_assessment",
    "settle_out_pressure",
    "shortcut_distillation_column",
    "sil_verification",
    "splitter",
    "stirred_tank_reactor",
    "stripping_column",
    "sts0131_gate",
    "tank",
    "three_phase_separator",
    "throttling_valve",
    "tools",
    "toxic_library",
    "tr3001_compliance",
    "vacuum_collapse",
    "validate",
    "vce",
    "vessel_rupture",
]

_FIRE_MONITOR_COVERAGE = "process.fire_monitor_coverage"
_FIRE_WATER_COVERAGE_ASSESSMENT = "process.fire_water_coverage_assessment"
_FIRE_WATER_DEMAND = "process.fire_water_demand"
_MANIFOLD = "process.manifold"
_MIXER = "process.mixer"
_NOG070_SIL_DETERMINATION = "process.nog070_sil_determination"
_OVERPRESSURE_ACCEPTANCE = "process.overpressure_acceptance"
_OVERPRESSURE_PROTECTION_STUDY = "process.overpressure_protection_study"
_PACKED_COLUMN = "process.packed_column"
_PFP_DEMAND = "process.pfp_demand"
_PIPELINE_PRESSURE_PROTECTION = "process.pipeline_pressure_protection"
_RATE_BASED_PACKED_COLUMN = "process.rate_based_packed_column"
_RELIEF_DISPOSAL_NETWORK = "process.relief_disposal_network"
_RISK_MATRIX = "process.risk_matrix"
_RUNAWAY_REACTION = "process.runaway_reaction"
_SAFETY_INSTRUMENTED_FUNCTION = "process.safety_instrumented_function"
_PIPE = "process.pipe"
_GAS_SCRUBBER = "process.gas_scrubber"
_HEAT_EXCHANGER = "process.heat_exchanger"
_COMPONENT_SPLITTER = "process.component_splitter"
_COMPRESSOR = "process.compressor"
_CONSEQUENCE_ANALYSIS = "process.consequence_analysis"
_CONTROL_VALVE_FAILURE_RELIEF = "process.control_valve_failure_relief"
_ABSORPTION_COLUMN = "process.absorption_column"
_ACOUSTIC_INDUCED_VIBRATION_SCREENING = "process.acoustic_induced_vibration_screening"
_ACTIVE_FIRE_PROTECTION_SCREENING = "process.active_fire_protection_screening"
_STRIPPING_COLUMN = "process.stripping_column"
_DISTILLATION_COLUMN = "process.distillation_column"
_COOLER = "process.cooler"
_DELUGE_NOZZLE_LAYOUT = "process.deluge_nozzle_layout"
_EJECTOR = "process.ejector"
_EVENT_TREE = "process.event_tree"
_FAULT_TREE = "process.fault_tree"
_BOW_TIE = "process.bow_tie"
_EXPANDER = "process.expander"
_TURBO_EXPANDER_COMPRESSOR = "process.turbo_expander_compressor"
_MAP_TURBO_EXPANDER_COMPRESSOR = "process.map_turbo_expander_compressor"
_FLOW_INDUCED_PULSATION_SCREENING = "process.flow_induced_pulsation_screening"
_ALARP_AUDIT = "process.alarp_audit"
_ESCALATION_GRAPH = "process.escalation_graph"
_ESD_RESPONSE_TIME = "process.esd_response_time"
_INHERENT_SAFETY = "process.inherent_safety"
_NORSOK_P002_COMPLIANCE = "process.norsok_p002_compliance"
_STS0131_GATE = "process.sts0131_gate"
_MDMT = "process.mdmt"
_MASS_BALANCE_LEAK_DETECTION = "process.mass_balance_leak_detection"
_PIPING_FIV_GAS = "process.piping_fiv_gas"
_PIPING_FIV_LIQUID = "process.piping_fiv_liquid"
_VACUUM_COLLAPSE = "process.vacuum_collapse"
_VCE = "process.vce"
_FILTER = "process.filter"
_FIRE_CASE_RELIEF = "process.fire_case_relief"
_FLARE = "process.flare"
_HEATER = "process.heater"
_HEM_RELEASE = "process.hem_release"
_IDEAL_GAS_RELEASE = "process.ideal_gas_release"
_LEGACY_SCREENING_RELEASE = "process.legacy_screening_release"
_PRESCRIBED_SLIP_RELEASE = "process.prescribed_slip_release"
_DRIFT_FLUX_RELEASE = "process.drift_flux_release"
_FINITE_RATE_RELEASE = "process.finite_rate_release"
_COMPONENT_SELECTIVE_RELEASE = "process.component_selective_release"
_RANZ_MARSHALL_RELEASE = "process.ranz_marshall_release"
_BUBBLE_TRANSFER_ZONE_RELEASE = "process.bubble_transfer_zone_release"
_IDEAL_GAS_FANNO_RELEASE = "process.ideal_gas_fanno_release"
_REAL_GAS_FANNO_RELEASE = "process.real_gas_fanno_release"
_JET_FIRE = "process.jet_fire"
_LEAK_BLOWDOWN = "process.leak_blowdown"
_LOPA = "process.lopa"
_GAS_BLOWBY = "process.gas_blowby"
_GAS_DISPERSION = "process.gas_dispersion"
_GAUSSIAN_PLUME = "process.gaussian_plume"
_HAZARDOUS_AREA = "process.hazardous_area"
_HEAVY_GAS_DISPERSION = "process.heavy_gas_dispersion"
_PROBIT = "process.probit"
_PSV_FIRE_SIZING = "process.psv_fire_sizing"
_PSV_SIZING = "process.psv_sizing"
_PSV_VALVE = "process.psv_valve"
_DYNAMIC_PSV_SIZING = "process.dynamic_psv_sizing"
_VESSEL_FILLING = "process.vessel_filling"
_DEPRESSURIZATION = "process.depressurization"
_DEPRESSURIZATION_ACCEPTANCE = "process.depressurization_acceptance"
_BLOCKED_OUTLET_OVERPRESSURE = "process.blocked_outlet_overpressure"
_MULTI_VESSEL_BLOWDOWN_STUDY = "process.multi_vessel_blowdown_study"
_NON_EQUILIBRIUM_BLOWDOWN = "process.non_equilibrium_blowdown"
_SEPARATOR = "process.separator"
_SEPARATOR_CAPACITY_ASSESSMENT = "process.separator_capacity_assessment"
_SEPARATOR_PERFORMANCE = "process.separator_performance"
_SETTLE_OUT_PRESSURE = "process.settle_out_pressure"
_SHORTCUT_DISTILLATION_COLUMN = "process.shortcut_distillation_column"
_SIL_VERIFICATION = "process.sil_verification"
_THROTTLING_VALVE = "process.throttling_valve"
_TOXIC_LIBRARY = "process.toxic_library"
_TR3001_COMPLIANCE = "process.tr3001_compliance"
_TRAPPED_INVENTORY = "process.trapped_inventory"
_TUBE_RUPTURE_RELIEF = "process.tube_rupture_relief"
_GIBBS_REACTOR = "process.gibbs_reactor"
_PLUG_FLOW_REACTOR = "process.plug_flow_reactor"
_POOL_FIRE = "process.pool_fire"
_POROUS_MEDIA_SELF_HEATING = "process.porous_media_self_heating"
_PUMP_DEADHEAD = "process.pump_deadhead"
_PUMP = "process.pump"
_SPLITTER = "process.splitter"
_STIRRED_TANK_REACTOR = "process.stirred_tank_reactor"
_TANK = "process.tank"
_THREE_PHASE_SEPARATOR = "process.three_phase_separator"
_ANTI_SURGE_RECYCLE = "process.anti_surge_recycle"
_API537_FLARE_FLAME_MODEL = "process.api537_flare_flame_model"
_BLEVE = "process.bleve"
_BLOCKED_OUTLET_RELIEF = "process.blocked_outlet_relief"
_CHECK_VALVE_LEAK_RELIEF = "process.check_valve_leak_relief"
_BLOWDOWN_PRESSURE_PROFILE = "process.blowdown_pressure_profile"
_PIPE_FIRE_RUPTURE = "process.pipe_fire_rupture"
_PIPE_FIRE_RUPTURE_MATERIAL = "process.pipe_fire_rupture_material"
_VESSEL_RUPTURE = "process.vessel_rupture"
_SEMENOV_SELF_HEATING = "process.semenov_self_heating"
_SELF_HEATING_INDUCTION = "process.self_heating_induction"
_BASKET_TEST_REGRESSION = "process.basket_test_regression"
_API14C_SAFE_CHART = "process.api14c_safe_chart"
_BARRIER_REGISTER_VALIDATION = "process.barrier_register_validation"
_S001_CLAUSE10_REVIEW = "process.s001_clause10_review"
_S001_SECONDARY_PRESSURE_PROTECTION = "process.s001_secondary_pressure_protection"
_HAZOP_TEMPLATE = "process.hazop_template"
_FMEA_WORKSHEET = "process.fmea_worksheet"
_HAZOP_CONSEQUENCE_POPULATION = "process.hazop_consequence_population"
_MAH_BOW_TIE_SKELETON = "process.mah_bow_tie_skeleton"


def validate(flowsheet: str, palette_dir: str = "specs/unit_ops") -> list[str]:
    """Validate a flowsheet's TOML text against a palette directory.

    Returns the diagnostic lines; empty when the flowsheet is clean.
    """
    return _core.validate_flowsheet(flowsheet, palette_dir)


def load_flowsheet(path: str, palette_dir: str = "specs/unit_ops") -> list[str]:
    """Read a flowsheet file and validate it; returns the diagnostic lines."""
    return validate(pathlib.Path(path).read_text(), palette_dir)


def pump_deadhead(
    suction_pressure: Q,
    normal_discharge_pressure: Q,
    shutoff_head_ratio: float | None = None,
    protected_pressure_rating: Q | None = None,
    liquid_density: Q | None = None,
    specific_heat: Q | None = None,
    minimum_flow_efficiency: float | None = None,
    max_allowable_temperature_rise: Q | None = None,
) -> PumpDeadheadResult:
    """The deadhead pressure, the rating verdict and the minimum-flow rise.

    **Equality with the protected rating is within it** and the temperature-rise test is
    strict. Without density or specific heat the shut-off head and rise are absent. The
    defaults are a 1.20 shut-off ratio and a 0.30 minimum-flow efficiency.

    See :func:`azoth.process.reference.pump_deadhead`.
    """
    return resolve(_PUMP_DEADHEAD)(  # type: ignore[no-any-return]
        suction_pressure=suction_pressure,
        normal_discharge_pressure=normal_discharge_pressure,
        shutoff_head_ratio=shutoff_head_ratio,
        protected_pressure_rating=protected_pressure_rating,
        liquid_density=liquid_density,
        specific_heat=specific_heat,
        minimum_flow_efficiency=minimum_flow_efficiency,
        max_allowable_temperature_rise=max_allowable_temperature_rise,
    )


def pump(
    components: list[str],
    inlet_n: Q,
    inlet_z: list[float],
    inlet_p: Q,
    inlet_t: Q,
    isentropic_efficiency: float,
    outlet_pressure: Q | None = None,
    curve: list[dict[str, Q]] | None = None,
    speed: float | None = None,
    reference_density: Q | None = None,
    viscosity: Q | None = None,
    use_viscosity_correction: bool | None = None,
    check_npsh: bool | None = None,
    npsh_margin: float | None = None,
    mechanical_design: bool | None = None,
    api610_pump_type: str | None = None,
    seal_type: str | None = None,
    maximum_suction_pressure: Q | None = None,
    furnished_casing_mawp: Q | None = None,
    shutoff_head: Q | None = None,
    npsh_margin_factor: float | None = None,
    minimum_npsh_margin: Q | None = None,
    por_low_fraction: float | None = None,
    por_high_fraction: float | None = None,
    aor_low_fraction: float | None = None,
    aor_high_fraction: float | None = None,
) -> PumpResult:
    """Raise a stream's pressure, adding the pump's work as enthalpy.

    ``components``, ``inlet_n``, ``inlet_z``, ``inlet_p`` and ``inlet_t`` are the inlet's
    record - the fluid, its flow, composition, pressure and temperature. The inlet's molar
    enthalpy is *not* an input: it is a state function of ``(T, P, z)``, so accepting one
    would let a caller hand over a state that does not exist.

    Without a curve the work is the isentropic head - ``(H(P_out, s_in) - H(P_in)) / eta``,
    from a flash - and ``outlet_pressure`` is the pressure it reaches. A non-empty ``curve``
    selects ``PumpChart``'s reduced quadratic instead: the head sets the pressure rise, the
    chart's efficiency sets the shaft power, and ``outlet_pressure`` is ignored. ``speed``,
    ``reference_density``, ``viscosity``, ``use_viscosity_correction``, ``check_npsh`` and
    ``npsh_margin`` are the chart's own surface.

    Raises:
        InvalidInputError: where the shapes disagree, the efficiency is outside ``(0, 1]``, a
            pump with no curve has no outlet pressure, or the curve is not a curve.

    See :func:`azoth.process.reference.pump`.
    """
    return resolve(_PUMP)(  # type: ignore[no-any-return]
        components=components,
        inlet_n=inlet_n,
        inlet_z=inlet_z,
        inlet_p=inlet_p,
        inlet_t=inlet_t,
        outlet_pressure=outlet_pressure,
        isentropic_efficiency=isentropic_efficiency,
        curve=curve,
        speed=speed,
        reference_density=reference_density,
        viscosity=viscosity,
        use_viscosity_correction=use_viscosity_correction,
        check_npsh=check_npsh,
        npsh_margin=npsh_margin,
        mechanical_design=mechanical_design,
        api610_pump_type=api610_pump_type,
        seal_type=seal_type,
        maximum_suction_pressure=maximum_suction_pressure,
        furnished_casing_mawp=furnished_casing_mawp,
        shutoff_head=shutoff_head,
        npsh_margin_factor=npsh_margin_factor,
        minimum_npsh_margin=minimum_npsh_margin,
        por_low_fraction=por_low_fraction,
        por_high_fraction=por_high_fraction,
        aor_low_fraction=aor_low_fraction,
        aor_high_fraction=aor_high_fraction,
    )


def distillation_column(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    number_of_stages: int,
    feed_stage: int,
    has_reboiler: bool,
    has_condenser: bool,
    top_pressure: Q,
    bottom_pressure: Q,
    temperature_tolerance: float = 1.0e-6,
    max_iterations: int = 200,
    reboiler_temperature: Q | None = None,
    condenser_temperature: Q | None = None,
    murphree_efficiency: float | None = None,
    tray_murphree_efficiency: list[float] | None = None,
    reactive: bool | None = None,
    reactive_start_tray: int | None = None,
    reactive_end_tray: int | None = None,
    solver_type: str | None = None,
    top_specification_type: str | None = None,
    top_specification_target: float | None = None,
    top_specification_component: str | None = None,
    bottom_specification_type: str | None = None,
    bottom_specification_target: float | None = None,
    bottom_specification_component: str | None = None,
    gas_side_draw_fractions: list[float] | None = None,
    liquid_side_draw_fractions: list[float] | None = None,
    pumparound_fractions: list[float] | None = None,
    side_draw_flow_tray: int | None = None,
    side_draw_flow_phase: str | None = None,
    side_draw_flow_target: Q | None = None,
    side_draw_flow_tolerance: float | None = None,
    side_draw_flow_max_iterations: int | None = None,
    pumparound_return_tray: int | None = None,
    pumparound_draw_tray: int | None = None,
    pumparound_draw_fraction: float | None = None,
    pumparound_temperature_drop: Q | None = None,
    pumparound_tolerance: float | None = None,
    pumparound_max_iterations: int | None = None,
    column_diameter: Q | None = None,
    max_allowable_fs_factor: float | None = None,
    internals_type: str | None = None,
    tray_spacing: Q | None = None,
    weir_height: Q | None = None,
    hole_diameter: Q | None = None,
    hole_area_fraction: float | None = None,
    downcommer_area_fraction: float | None = None,
    design_flood_fraction: float | None = None,
    column_diameter_override: Q | None = None,
    hydraulic_pressure_drop_coupling: bool | None = None,
    hydraulic_pressure_drop_internals_type: str | None = None,
    tray_efficiency: float | None = None,
    max_flooding_factor: float | None = None,
    tray_type: str | None = None,
    contactor_internals_type: str | None = None,
    material_grade: str | None = None,
    max_operation_pressure: float | None = None,
) -> DistillationColumnResult:
    """Solve a distillation column by sequential substitution.

    ``number_of_stages`` counts the trays **between the ends**, so a reboiler or a condenser
    adds one stage each; ``feed_stage`` is 0-based over the trays *including* them, which is
    why stage 0 is the reboiler when there is one. The ends are pinned by temperature -
    ``condenser_temperature`` and ``reboiler_temperature`` reach the trays' own outlet
    specifications, exactly as ``setCondenserTemperature`` does - and a middle tray flashes at
    its own enthalpy.

    **The answer is a profile**, not a scalar: the tray temperatures, pressures and both
    traffic rates, then the two products and the two duties. **Four of the twenty-two declared
    parameters are refused by name** - the Murphree efficiency and the two product
    specifications, whose arithmetic is not ported - as are the nine other solving strategies
    the class carries. A refusal names the class that would close it, so what is owed is
    readable rather than absent.

    Raises:
        InvalidInputError: for a stage or a feed stage outside the column, and for any
            declared parameter whose arithmetic is not ported.
        SolverNotConvergedError: when the solve misses its gate; the message carries the
            residuals.

    See :func:`azoth.process.reference.distillation_column`.
    """
    return resolve(_DISTILLATION_COLUMN)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        number_of_stages=number_of_stages,
        feed_stage=feed_stage,
        has_reboiler=has_reboiler,
        has_condenser=has_condenser,
        top_pressure=top_pressure,
        bottom_pressure=bottom_pressure,
        reboiler_temperature=reboiler_temperature,
        condenser_temperature=condenser_temperature,
        temperature_tolerance=temperature_tolerance,
        max_iterations=max_iterations,
        murphree_efficiency=murphree_efficiency,
        tray_murphree_efficiency=tray_murphree_efficiency,
        reactive=reactive,
        reactive_start_tray=reactive_start_tray,
        reactive_end_tray=reactive_end_tray,
        solver_type=solver_type,
        top_specification_type=top_specification_type,
        top_specification_target=top_specification_target,
        top_specification_component=top_specification_component,
        bottom_specification_type=bottom_specification_type,
        bottom_specification_target=bottom_specification_target,
        bottom_specification_component=bottom_specification_component,
        gas_side_draw_fractions=gas_side_draw_fractions,
        liquid_side_draw_fractions=liquid_side_draw_fractions,
        pumparound_fractions=pumparound_fractions,
        side_draw_flow_tray=side_draw_flow_tray,
        side_draw_flow_phase=side_draw_flow_phase,
        side_draw_flow_target=side_draw_flow_target,
        side_draw_flow_tolerance=side_draw_flow_tolerance,
        side_draw_flow_max_iterations=side_draw_flow_max_iterations,
        pumparound_return_tray=pumparound_return_tray,
        pumparound_draw_tray=pumparound_draw_tray,
        pumparound_draw_fraction=pumparound_draw_fraction,
        pumparound_temperature_drop=pumparound_temperature_drop,
        pumparound_tolerance=pumparound_tolerance,
        pumparound_max_iterations=pumparound_max_iterations,
        column_diameter=column_diameter,
        max_allowable_fs_factor=max_allowable_fs_factor,
        internals_type=internals_type,
        tray_spacing=tray_spacing,
        weir_height=weir_height,
        hole_diameter=hole_diameter,
        hole_area_fraction=hole_area_fraction,
        downcommer_area_fraction=downcommer_area_fraction,
        design_flood_fraction=design_flood_fraction,
        column_diameter_override=column_diameter_override,
        hydraulic_pressure_drop_coupling=hydraulic_pressure_drop_coupling,
        hydraulic_pressure_drop_internals_type=hydraulic_pressure_drop_internals_type,
        tray_efficiency=tray_efficiency,
        max_flooding_factor=max_flooding_factor,
        tray_type=tray_type,
        contactor_internals_type=contactor_internals_type,
        material_grade=material_grade,
        max_operation_pressure=max_operation_pressure,
    )


def absorption_column(
    gas_components: list[str],
    gas_n: Q,
    gas_z: list[float],
    gas_p: Q,
    gas_t: Q,
    solvent_components: list[str],
    solvent_n: Q,
    solvent_z: list[float],
    solvent_p: Q,
    solvent_t: Q,
    number_of_stages: int,
    top_pressure: Q,
    bottom_pressure: Q,
    temperature_tolerance: float,
    max_iterations: int,
    tray_temperatures: list[float] | None = None,
    murphree_efficiency: float | None = None,
    component_murphree_efficiency: list[float] | None = None,
    max_allowable_gas_load_factor: float | None = None,
    reactive: bool | None = None,
    reactive_start_tray: int | None = None,
    reactive_end_tray: int | None = None,
    solver_type: str | None = None,
    tray_murphree_efficiency: list[float] | None = None,
    gas_side_draw_fractions: list[float] | None = None,
    liquid_side_draw_fractions: list[float] | None = None,
    pumparound_fractions: list[float] | None = None,
    side_draw_flow_tray: int | None = None,
    side_draw_flow_phase: str | None = None,
    side_draw_flow_target: Q | None = None,
    side_draw_flow_tolerance: float | None = None,
    side_draw_flow_max_iterations: int | None = None,
    pumparound_return_tray: int | None = None,
    pumparound_draw_tray: int | None = None,
    pumparound_draw_fraction: float | None = None,
    pumparound_temperature_drop: Q | None = None,
    pumparound_tolerance: float | None = None,
    pumparound_max_iterations: int | None = None,
    column_diameter: Q | None = None,
    max_allowable_fs_factor: float | None = None,
    tray_efficiency: float | None = None,
    max_flooding_factor: float | None = None,
    tray_type: str | None = None,
    contactor_internals_type: str | None = None,
    material_grade: str | None = None,
    max_operation_pressure: float | None = None,
) -> AbsorptionColumnResult:
    """Solve a tray absorber, or a stripper.

    ``AbsorptionColumn extends DistillationColumn`` and overrides no ``run``, so this is the
    column's counter-current stage solve with **no condenser, no reboiler and two inlets**: the
    gas enters stage 0 and the solvent the top stage, which is what ``addGasInStream`` and
    ``addSolventInStream`` call. A stripper is the same machine with its inlets named for what
    they carry.

    The answer is the profile - each tray's temperature, pressure and both traffic rates - and
    the class's own two products: ``getGasOutStream`` is the treated gas and
    ``getLiquidOutStream`` the loaded solvent.

    **The class's own tests pin every tray, and a pinned column stops after one sweep**:
    ``SimpleTray.setOutletTemperature`` makes the base's own gate - the mean tray-temperature
    change - exactly zero. ``tray_temperatures`` is that mechanism, and the capture's pinned
    rows are kept as evidence rather than as the oracle.

    **Declared and refused by name**: the Murphree efficiency of either kind, and every solving
    strategy this port does not carry. ``max_allowable_gas_load_factor`` is accepted and does
    not enter the solve - ``isGasLoadFactorWithinDesignLimit`` reads it and no part of ``run``
    does.

    See :func:`azoth.process.reference.absorption_column`.
    """
    return resolve(_ABSORPTION_COLUMN)(  # type: ignore[no-any-return]
        gas_components=gas_components,
        gas_n=gas_n,
        gas_z=gas_z,
        gas_p=gas_p,
        gas_t=gas_t,
        solvent_components=solvent_components,
        solvent_n=solvent_n,
        solvent_z=solvent_z,
        solvent_p=solvent_p,
        solvent_t=solvent_t,
        number_of_stages=number_of_stages,
        top_pressure=top_pressure,
        bottom_pressure=bottom_pressure,
        temperature_tolerance=temperature_tolerance,
        max_iterations=max_iterations,
        tray_temperatures=tray_temperatures,
        murphree_efficiency=murphree_efficiency,
        component_murphree_efficiency=component_murphree_efficiency,
        max_allowable_gas_load_factor=max_allowable_gas_load_factor,
        reactive=reactive,
        reactive_start_tray=reactive_start_tray,
        reactive_end_tray=reactive_end_tray,
        solver_type=solver_type,
        tray_murphree_efficiency=tray_murphree_efficiency,
        gas_side_draw_fractions=gas_side_draw_fractions,
        liquid_side_draw_fractions=liquid_side_draw_fractions,
        pumparound_fractions=pumparound_fractions,
        side_draw_flow_tray=side_draw_flow_tray,
        side_draw_flow_phase=side_draw_flow_phase,
        side_draw_flow_target=side_draw_flow_target,
        side_draw_flow_tolerance=side_draw_flow_tolerance,
        side_draw_flow_max_iterations=side_draw_flow_max_iterations,
        pumparound_return_tray=pumparound_return_tray,
        pumparound_draw_tray=pumparound_draw_tray,
        pumparound_draw_fraction=pumparound_draw_fraction,
        pumparound_temperature_drop=pumparound_temperature_drop,
        pumparound_tolerance=pumparound_tolerance,
        pumparound_max_iterations=pumparound_max_iterations,
        column_diameter=column_diameter,
        max_allowable_fs_factor=max_allowable_fs_factor,
        tray_efficiency=tray_efficiency,
        max_flooding_factor=max_flooding_factor,
        tray_type=tray_type,
        contactor_internals_type=contactor_internals_type,
        material_grade=material_grade,
        max_operation_pressure=max_operation_pressure,
    )


def packed_column(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    packed_height: Q,
    feed_stage: int,
    has_reboiler: bool,
    has_condenser: bool,
    top_pressure: Q,
    bottom_pressure: Q,
    temperature_tolerance: float = 1.0e-6,
    max_iterations: int = 200,
    reboiler_temperature: Q | None = None,
    condenser_temperature: Q | None = None,
    packing_type: str | None = None,
    structured_packing: bool | None = None,
    design_flood_fraction: float | None = None,
    packing_hydraulic_capacity_factor: float | None = None,
    column_diameter: Q | None = None,
    murphree_efficiency: float | None = None,
    tray_murphree_efficiency: list[float] | None = None,
    reactive: bool | None = None,
    reactive_start_tray: int | None = None,
    reactive_end_tray: int | None = None,
    solver_type: str | None = None,
    top_specification_type: str | None = None,
    top_specification_target: float | None = None,
    top_specification_component: str | None = None,
    bottom_specification_type: str | None = None,
    bottom_specification_target: float | None = None,
    bottom_specification_component: str | None = None,
    gas_side_draw_fractions: list[float] | None = None,
    liquid_side_draw_fractions: list[float] | None = None,
    pumparound_fractions: list[float] | None = None,
    side_draw_flow_tray: int | None = None,
    side_draw_flow_phase: str | None = None,
    side_draw_flow_target: Q | None = None,
    side_draw_flow_tolerance: float | None = None,
    side_draw_flow_max_iterations: int | None = None,
    pumparound_return_tray: int | None = None,
    pumparound_draw_tray: int | None = None,
    pumparound_draw_fraction: float | None = None,
    pumparound_temperature_drop: Q | None = None,
    pumparound_tolerance: float | None = None,
    pumparound_max_iterations: int | None = None,
    max_allowable_fs_factor: float | None = None,
) -> PackedColumnResult:
    """Solve a packed column.

    ``PackedColumn extends DistillationColumn`` and its ``run`` is ``super.run(id)`` followed by
    ``calcPackingHydraulics()``, so **the separation is the base column's and the packing does
    not change it**. What the packing does change is the stage count, at construction:
    ``estimateStages(packed_height, 0.5)`` is ``ceil(packed_height / 0.5)`` floored at two, so
    ``packed_height`` replaces the base's ``number_of_stages`` here.

    The five packing parameters are read only by the hydraulics report on the far side of the
    solve, and that report is mechanical design and is not ported, so they are declarations this
    solve is indifferent to - except ``packing_hydraulic_capacity_factor``, which the class's own
    setter refuses where it is not positive and finite.

    See :func:`azoth.process.reference.packed_column`.
    """
    return resolve(_PACKED_COLUMN)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        packed_height=packed_height,
        feed_stage=feed_stage,
        has_reboiler=has_reboiler,
        has_condenser=has_condenser,
        top_pressure=top_pressure,
        bottom_pressure=bottom_pressure,
        temperature_tolerance=temperature_tolerance,
        max_iterations=max_iterations,
        reboiler_temperature=reboiler_temperature,
        condenser_temperature=condenser_temperature,
        packing_type=packing_type,
        structured_packing=structured_packing,
        design_flood_fraction=design_flood_fraction,
        packing_hydraulic_capacity_factor=packing_hydraulic_capacity_factor,
        column_diameter=column_diameter,
        murphree_efficiency=murphree_efficiency,
        tray_murphree_efficiency=tray_murphree_efficiency,
        reactive=reactive,
        reactive_start_tray=reactive_start_tray,
        reactive_end_tray=reactive_end_tray,
        solver_type=solver_type,
        top_specification_type=top_specification_type,
        top_specification_target=top_specification_target,
        top_specification_component=top_specification_component,
        bottom_specification_type=bottom_specification_type,
        bottom_specification_target=bottom_specification_target,
        bottom_specification_component=bottom_specification_component,
        gas_side_draw_fractions=gas_side_draw_fractions,
        liquid_side_draw_fractions=liquid_side_draw_fractions,
        pumparound_fractions=pumparound_fractions,
        side_draw_flow_tray=side_draw_flow_tray,
        side_draw_flow_phase=side_draw_flow_phase,
        side_draw_flow_target=side_draw_flow_target,
        side_draw_flow_tolerance=side_draw_flow_tolerance,
        side_draw_flow_max_iterations=side_draw_flow_max_iterations,
        pumparound_return_tray=pumparound_return_tray,
        pumparound_draw_tray=pumparound_draw_tray,
        pumparound_draw_fraction=pumparound_draw_fraction,
        pumparound_temperature_drop=pumparound_temperature_drop,
        pumparound_tolerance=pumparound_tolerance,
        pumparound_max_iterations=pumparound_max_iterations,
        max_allowable_fs_factor=max_allowable_fs_factor,
    )


def rate_based_packed_column(
    gas_components: list[str],
    gas_n: Q,
    gas_z: list[float],
    gas_p: Q,
    gas_t: Q,
    liquid_components: list[str],
    liquid_n: Q,
    liquid_z: list[float],
    liquid_p: Q,
    liquid_t: Q,
    transfer_components: list[str] | None = None,
    column_diameter: Q | None = None,
    packed_height: Q | None = None,
    number_of_segments: int | None = None,
    packing_type: str | None = None,
    max_iterations: int | None = None,
    convergence_tolerance: Q | None = None,
    mass_transfer_correction: float | None = None,
    mass_transfer_correlation: str | None = None,
    film_model: str | None = None,
    heat_transfer_model: str | None = None,
    segment_solver: str | None = None,
    column_solver: str | None = None,
) -> RateBasedPackedColumnResult:
    """Solve a rate-based packed column.

    **A second physics rather than a packed column**: ``RateBasedPackedColumn`` extends
    ``ProcessEquipmentBaseClass`` and its 4,081 lines are a segment model - axial slices through
    the packing, each with gas and liquid film coefficients, an interface equilibrium and an
    interphase heat balance, coupled counter-currently by a profile solver. There is no stage, no
    condenser, no reboiler and no MESH residual; the four streams are the whole interface.

    **The packing is inside the equations here**, unlike :func:`packed_column`, whose packing is
    read by a report after the solve: the wetted area and the two film coefficients come from
    ``hydraulics.packing_hydraulics`` every segment.

    A profile that reaches its iteration cap is *published* with its last iterate, and
    ``converged`` says so - which is what the class does. A bed of no height is a converged
    solve on its first pass and transfers nothing.

    See :func:`azoth.process.reference.rate_based_packed_column`.
    """
    return resolve(_RATE_BASED_PACKED_COLUMN)(  # type: ignore[no-any-return]
        gas_components=gas_components,
        gas_n=gas_n,
        gas_z=gas_z,
        gas_p=gas_p,
        gas_t=gas_t,
        liquid_components=liquid_components,
        liquid_n=liquid_n,
        liquid_z=liquid_z,
        liquid_p=liquid_p,
        liquid_t=liquid_t,
        transfer_components=transfer_components,
        column_diameter=column_diameter,
        packed_height=packed_height,
        number_of_segments=number_of_segments,
        packing_type=packing_type,
        max_iterations=max_iterations,
        convergence_tolerance=convergence_tolerance,
        mass_transfer_correction=mass_transfer_correction,
        mass_transfer_correlation=mass_transfer_correlation,
        film_model=film_model,
        heat_transfer_model=heat_transfer_model,
        segment_solver=segment_solver,
        column_solver=column_solver,
    )


def stripping_column(
    stripping_gas_components: list[str],
    rich_liquid_components: list[str],
    stripping_gas_n: Q,
    stripping_gas_z: list[float],
    stripping_gas_p: Q,
    stripping_gas_t: Q,
    rich_liquid_n: Q,
    rich_liquid_z: list[float],
    rich_liquid_p: Q,
    rich_liquid_t: Q,
    number_of_stages: int,
    top_pressure: Q,
    bottom_pressure: Q,
    temperature_tolerance: float,
    max_iterations: int,
    tray_temperatures: list[float] | None = None,
    murphree_efficiency: float | None = None,
    component_murphree_efficiency: list[float] | None = None,
    max_allowable_gas_load_factor: float | None = None,
    reactive: bool | None = None,
    reactive_start_tray: int | None = None,
    reactive_end_tray: int | None = None,
    solver_type: str | None = None,
    tray_murphree_efficiency: list[float] | None = None,
    gas_side_draw_fractions: list[float] | None = None,
    liquid_side_draw_fractions: list[float] | None = None,
    pumparound_fractions: list[float] | None = None,
    side_draw_flow_tray: int | None = None,
    side_draw_flow_phase: str | None = None,
    side_draw_flow_target: Q | None = None,
    side_draw_flow_tolerance: float | None = None,
    side_draw_flow_max_iterations: int | None = None,
    pumparound_return_tray: int | None = None,
    pumparound_draw_tray: int | None = None,
    pumparound_draw_fraction: float | None = None,
    pumparound_temperature_drop: Q | None = None,
    pumparound_tolerance: float | None = None,
    pumparound_max_iterations: int | None = None,
    column_diameter: Q | None = None,
    max_allowable_fs_factor: float | None = None,
    tray_efficiency: float | None = None,
    max_flooding_factor: float | None = None,
    tray_type: str | None = None,
    contactor_internals_type: str | None = None,
    material_grade: str | None = None,
    max_operation_pressure: float | None = None,
) -> StrippingColumnResult:
    """Strip a rich liquid with a counter-current gas.

    ``StrippingColumn extends AbsorptionColumn`` and its ninety lines rename the two inlets and
    the two products: ``addStrippingGasStream`` is ``addGasInStream`` and
    ``addRichLiquidStream`` is ``addSolventInStream``, so the stripping gas enters stage 0 and
    the rich liquid the top stage. **The class adds no equations** - absorption and stripping
    are one set of counter-current equilibrium-stage equations, and the driving force sets the
    direction of transfer - so this is :func:`absorption_column` under the class's own names, and
    the two products are ``getOverheadGasStream`` and ``getLeanLiquidStream``.

    See :func:`azoth.process.reference.stripping_column`.
    """
    return resolve(_STRIPPING_COLUMN)(  # type: ignore[no-any-return]
        stripping_gas_components=stripping_gas_components,
        rich_liquid_components=rich_liquid_components,
        stripping_gas_n=stripping_gas_n,
        stripping_gas_z=stripping_gas_z,
        stripping_gas_p=stripping_gas_p,
        stripping_gas_t=stripping_gas_t,
        rich_liquid_n=rich_liquid_n,
        rich_liquid_z=rich_liquid_z,
        rich_liquid_p=rich_liquid_p,
        rich_liquid_t=rich_liquid_t,
        number_of_stages=number_of_stages,
        top_pressure=top_pressure,
        bottom_pressure=bottom_pressure,
        temperature_tolerance=temperature_tolerance,
        max_iterations=max_iterations,
        tray_temperatures=tray_temperatures,
        murphree_efficiency=murphree_efficiency,
        component_murphree_efficiency=component_murphree_efficiency,
        max_allowable_gas_load_factor=max_allowable_gas_load_factor,
        reactive=reactive,
        reactive_start_tray=reactive_start_tray,
        reactive_end_tray=reactive_end_tray,
        solver_type=solver_type,
        tray_murphree_efficiency=tray_murphree_efficiency,
        gas_side_draw_fractions=gas_side_draw_fractions,
        liquid_side_draw_fractions=liquid_side_draw_fractions,
        pumparound_fractions=pumparound_fractions,
        side_draw_flow_tray=side_draw_flow_tray,
        side_draw_flow_phase=side_draw_flow_phase,
        side_draw_flow_target=side_draw_flow_target,
        side_draw_flow_tolerance=side_draw_flow_tolerance,
        side_draw_flow_max_iterations=side_draw_flow_max_iterations,
        pumparound_return_tray=pumparound_return_tray,
        pumparound_draw_tray=pumparound_draw_tray,
        pumparound_draw_fraction=pumparound_draw_fraction,
        pumparound_temperature_drop=pumparound_temperature_drop,
        pumparound_tolerance=pumparound_tolerance,
        pumparound_max_iterations=pumparound_max_iterations,
        column_diameter=column_diameter,
        max_allowable_fs_factor=max_allowable_fs_factor,
        tray_efficiency=tray_efficiency,
        max_flooding_factor=max_flooding_factor,
        tray_type=tray_type,
        contactor_internals_type=contactor_internals_type,
        material_grade=material_grade,
        max_operation_pressure=max_operation_pressure,
    )


def stirred_tank_reactor(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    reaction: str,
    limiting_reactant: str,
    conversion: float,
    isothermal: bool,
    reactor_temperature: Q | None = None,
    reactor_pressure: Q | None = None,
    pressure_drop: Q | None = None,
    vessel_volume: Q | None = None,
    residence_time: Q | None = None,
    agitator_power_per_volume: Q | None = None,
) -> StirredTankReactorResult:
    """React a feed to a conversion of one reactant, then flash the result.

    ``StirredTankReactor.run`` applies its reaction, sets the pressure and holds the
    temperature when it is isothermal, then flashes: a ``TPflash`` at the held temperature
    or a ``PHflash`` at the **feed's own enthalpy**.

    **The reaction is a movement of moles**, not an energy balance: ``react`` takes the
    limiting reactant's moles times ``conversion`` and scales every stoichiometric
    coefficient by the limiting one's, and the heat the reaction releases shows up as the
    temperature the adiabatic flash lands on rather than as a duty. The isothermal branch
    reports the duty it had to supply, which is the outlet's total enthalpy less the feed's.

    **A stoichiometry row naming a substance the feed does not carry is skipped**, which is
    the class's own behaviour, so a reaction whose products are not in the feed moves less
    material than its stoichiometry says.

    See :func:`azoth.process.reference.stirred_tank_reactor`.
    """
    return resolve(_STIRRED_TANK_REACTOR)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        reaction=reaction,
        limiting_reactant=limiting_reactant,
        conversion=conversion,
        isothermal=isothermal,
        reactor_temperature=reactor_temperature,
        reactor_pressure=reactor_pressure,
        pressure_drop=pressure_drop,
        vessel_volume=vessel_volume,
        residence_time=residence_time,
        agitator_power_per_volume=agitator_power_per_volume,
    )


def plug_flow_reactor(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    length: float,
    diameter: float,
    number_of_tubes: float,
    energy_mode: str,
    coolant_temperature: Q,
    overall_heat_transfer_coefficient: Q,
    number_of_steps: float,
    integration_method: str,
    property_update_frequency: float,
    thermodynamic_coupling: str,
    reaction: str,
    reaction_orders: list[float],
    rate_type: str,
    pre_exponential_factor: float,
    activation_energy: float,
    temperature_exponent: float,
    heat_of_reaction: float,
    catalyst_bulk_density: Q | None = None,
    catalyst_activity_factor: float | None = None,
    catalyst_particle_diameter: float | None = None,
    catalyst_void_fraction: float | None = None,
    catalyst_molecular_diffusivity: Q | None = None,
    catalyst_effectiveness_enabled: bool | None = None,
    key_component: str | None = None,
) -> PlugFlowReactorResult:
    """March a feed along a tube, reacting as it goes.

    ``PlugFlowReactor.run`` integrates ``[F1..Fn, T, P]`` in a fixed number of steps with a
    scheme it writes out itself - RK4 by default, Euler on request, ``dz = length / steps`` and
    no step-size control - and the answer is the whole profile rather than an outlet row.

    **The properties are frozen by default.** ``thermodynamic_coupling`` is
    ``"frozen_properties"``, so the rate law reads the last re-flash and never one of the RK4
    sub-states; at the default frequency of ten that is why the class's two schemes agree to
    thirteen digits on its own default, and why the step count moves the answer through *where*
    the properties are re-read rather than through the integration order.

    **The pressure row is the empty-tube branch unless a bed is declared.** Supplying
    ``catalyst_bulk_density`` is what sets one, and it is what puts the Ergun equation in the
    pressure row - measured, four orders of magnitude apart on the same gas.

    **The stoichiometry is the reaction data's and the orders are yours.** ``KineticReaction`` is
    a configured object in NeqSim and a declaration carries no coefficient map, so ``reaction``
    resolves against the reaction data exactly as `azoth.process.stirred_tank_reactor`'s does,
    and ``reaction_orders`` supplies the one argument that data does not carry - one entry per
    reactant, in the table's order.

    See :func:`azoth.process.reference.plug_flow_reactor`.
    """
    return resolve(_PLUG_FLOW_REACTOR)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        length=length,
        diameter=diameter,
        number_of_tubes=number_of_tubes,
        energy_mode=energy_mode,
        coolant_temperature=coolant_temperature,
        overall_heat_transfer_coefficient=overall_heat_transfer_coefficient,
        number_of_steps=number_of_steps,
        integration_method=integration_method,
        property_update_frequency=property_update_frequency,
        thermodynamic_coupling=thermodynamic_coupling,
        reaction=reaction,
        reaction_orders=reaction_orders,
        rate_type=rate_type,
        pre_exponential_factor=pre_exponential_factor,
        activation_energy=activation_energy,
        temperature_exponent=temperature_exponent,
        heat_of_reaction=heat_of_reaction,
        catalyst_bulk_density=catalyst_bulk_density,
        catalyst_activity_factor=catalyst_activity_factor,
        catalyst_particle_diameter=catalyst_particle_diameter,
        catalyst_void_fraction=catalyst_void_fraction,
        catalyst_molecular_diffusivity=catalyst_molecular_diffusivity,
        catalyst_effectiveness_enabled=catalyst_effectiveness_enabled,
        key_component=key_component,
    )


def pool_fire(
    pool_diameter: Q,
    burn_rate: Q,
    air_density: float | None = None,
    distance: Q | None = None,
    target_flux: Q | None = None,
) -> PoolFireResult:
    """The pool-fire radiation at one distance, or the distance to a target flux.

    Thomas's flame height, Mudan's smoke-corrected SEP and vertical-cylinder view factor.
    **A target the fire cannot produce is absent**: the class answers its near bound,
    ``R + 0.1``, where the flux is already below the target.

    See :func:`azoth.process.reference.pool_fire`.
    """
    return resolve(_POOL_FIRE)(  # type: ignore[no-any-return]
        pool_diameter=pool_diameter,
        burn_rate=burn_rate,
        air_density=air_density,
        distance=distance,
        target_flux=target_flux,
    )


def porous_media_self_heating(
    geometry: str,
    characteristic_dimension: Q,
    effective_conductivity: Q,
    activation_energy: Q,
    volumetric_pre_factor: float,
    boundary_temperature: Q,
    marginal_ratio: float | None = None,
    pipe_insulation_screening: bool | None = None,
) -> PorousMediaSelfHeatingResult:
    """The Frank-Kamenetskii criticality screen for a porous body.

    **`delta` scales with the square of the characteristic dimension**, so a thick layer
    ignites where a laboratory dish does not. The critical boundary temperature is the
    same bisection as the Semenov screen; the critical half-dimension is closed form.
    ``pipe_insulation_screening`` forces the slab and records the conservative bounding.

    See :func:`azoth.process.reference.porous_media_self_heating`.
    """
    return resolve(_POROUS_MEDIA_SELF_HEATING)(  # type: ignore[no-any-return]
        geometry=geometry,
        characteristic_dimension=characteristic_dimension,
        effective_conductivity=effective_conductivity,
        activation_energy=activation_energy,
        volumetric_pre_factor=volumetric_pre_factor,
        boundary_temperature=boundary_temperature,
        marginal_ratio=marginal_ratio,
        pipe_insulation_screening=pipe_insulation_screening,
    )


def self_heating_induction(
    geometry: str,
    characteristic_dimension: Q,
    effective_conductivity: Q,
    volumetric_heat_capacity: float,
    activation_energy: Q,
    volumetric_pre_factor: float,
    boundary_temperature: Q,
    initial_temperature: Q | None = None,
    ignition_rise: Q | None = None,
    max_time: Q | None = None,
    node_count: float | None = None,
) -> SelfHeatingInductionResult:
    """The one-dimensional induction-time march for a self-heating body.

    **Explicit Euler with an adaptive step**, ending on ignition, on the steady heuristic,
    or at the time cap. The retained history is decimated; the three warnings are the
    class's own.

    See :func:`azoth.process.reference.self_heating_induction`.
    """
    return resolve(_SELF_HEATING_INDUCTION)(  # type: ignore[no-any-return]
        geometry=geometry,
        characteristic_dimension=characteristic_dimension,
        effective_conductivity=effective_conductivity,
        volumetric_heat_capacity=volumetric_heat_capacity,
        activation_energy=activation_energy,
        volumetric_pre_factor=volumetric_pre_factor,
        boundary_temperature=boundary_temperature,
        initial_temperature=initial_temperature,
        ignition_rise=ignition_rise,
        max_time=max_time,
        node_count=node_count,
    )


def basket_test_regression(
    points: list[dict[str, object]],
    effective_conductivity: Q | None = None,
) -> BasketTestRegressionResult:
    """The least-squares basket-test fit for the activation energy and pre-factor.

    **The slope alone gives E; the conductivity splits the intercept into P.** A fit with
    fewer than three sizes carries the class's recommendation, and a fit without a
    conductivity yields no pre-factor.

    See :func:`azoth.process.reference.basket_test_regression`.
    """
    return resolve(_BASKET_TEST_REGRESSION)(  # type: ignore[no-any-return]
        points=points,
        effective_conductivity=effective_conductivity,
    )


def filter(
    components: list[str],
    inlet_n: Q,
    inlet_z: list[float],
    inlet_p: Q,
    inlet_t: Q,
    pressure_drop: Q,
    pressure_drop_model: str | None = None,
    reference_flow: Q | None = None,
    flow_exponent: float | None = None,
    pressure_drop_curve: list[dict[str, Q]] | None = None,
    media_area: Q | None = None,
    media_bed_depth: Q | None = None,
    media_particle_diameter: Q | None = None,
    media_void_fraction: float | None = None,
    viscosity: Q | None = None,
    performance_curve: list[dict[str, Q]] | None = None,
    particle_size: Q | None = None,
    nominal_removal_efficiency: float | None = None,
    inlet_particle_concentration: float | None = None,
    solids_loading: Q | None = None,
    loading_capacity: Q | None = None,
    pressure_drop_increase_at_capacity: Q | None = None,
    breakthrough_start_fraction: float | None = None,
    bypass_cracking_pressure: Q | None = None,
    terminal_pressure_drop: Q | None = None,
    time_step: Q | None = None,
    holdup_volume: Q | None = None,
    solids_loading_rate: Q | None = None,
    backwash_removal_rate: Q | None = None,
    regeneration_removal_rate: Q | None = None,
    backwash_active: bool | None = None,
    regeneration_active: bool | None = None,
    mechanical_design: bool | None = None,
    filter_type: str | None = None,
    number_of_elements: float | None = None,
    max_face_velocity: Q | None = None,
    element_area: Q | None = None,
    max_nozzle_velocity: Q | None = None,
    min_ld_ratio: float | None = None,
    max_ld_ratio: float | None = None,
    design_pressure_margin: float | None = None,
    minimum_design_pressure_margin: Q | None = None,
    design_temperature_margin: Q | None = None,
    allowable_stress: Q | None = None,
    joint_efficiency: float | None = None,
    corrosion_allowance: Q | None = None,
    element_collapse_pressure: Q | None = None,
    element_integrity_verified: bool | None = None,
) -> FilterResult:
    """Drop a stream's pressure by a fixed amount at a constant temperature.

    **It holds the temperature**, which is what separates the entry from
    :func:`throttling_valve`: ``Filter.run`` sets the reduced pressure and flashes, so the
    outlet is at the feed's temperature and its enthalpy moves with the pressure - about
    ``24.8`` J/mol per bar on the fluid these cases share.

    A drop larger than the inlet pressure is **clamped rather than refused**, which is what
    ``Filter.run`` does: the outlet lands a millionth of a bar above vacuum and
    ``applied_drop`` reports what was applied, with a warning when it differs from the
    request.

    ``pressure_drop_model`` selects the class's clean-drop model - ``fixed`` by default, then
    ``flow_scaled``, ``tabulated`` and ``ergun`` - and the inputs after it are that model's
    own settings. The beta-ratio ``performance_curve`` (or a scalar
    ``nominal_removal_efficiency``) with an ``inlet_particle_concentration`` enables the
    particle capture.

    ``solids_loading`` with a ``loading_capacity`` adds the loading's pressure drop, the
    breakthrough retention and the optional ``bypass_cracking_pressure`` to the same run;
    ``terminal_pressure_drop`` is the replacement threshold. A positive ``time_step`` runs one
    ``runTransient`` step, accreting ``solids_loading_rate`` (or the concentration capture) and
    subtracting the active backwash and regeneration removals.

    See :func:`azoth.process.reference.filter`.
    """
    return resolve(_FILTER)(  # type: ignore[no-any-return]
        components=components,
        inlet_n=inlet_n,
        inlet_z=inlet_z,
        inlet_p=inlet_p,
        inlet_t=inlet_t,
        pressure_drop=pressure_drop,
        pressure_drop_model=pressure_drop_model,
        reference_flow=reference_flow,
        flow_exponent=flow_exponent,
        pressure_drop_curve=pressure_drop_curve,
        media_area=media_area,
        media_bed_depth=media_bed_depth,
        media_particle_diameter=media_particle_diameter,
        media_void_fraction=media_void_fraction,
        viscosity=viscosity,
        performance_curve=performance_curve,
        particle_size=particle_size,
        nominal_removal_efficiency=nominal_removal_efficiency,
        inlet_particle_concentration=inlet_particle_concentration,
        solids_loading=solids_loading,
        loading_capacity=loading_capacity,
        pressure_drop_increase_at_capacity=pressure_drop_increase_at_capacity,
        breakthrough_start_fraction=breakthrough_start_fraction,
        bypass_cracking_pressure=bypass_cracking_pressure,
        terminal_pressure_drop=terminal_pressure_drop,
        time_step=time_step,
        holdup_volume=holdup_volume,
        solids_loading_rate=solids_loading_rate,
        backwash_removal_rate=backwash_removal_rate,
        regeneration_removal_rate=regeneration_removal_rate,
        backwash_active=backwash_active,
        regeneration_active=regeneration_active,
        mechanical_design=mechanical_design,
        filter_type=filter_type,
        number_of_elements=number_of_elements,
        max_face_velocity=max_face_velocity,
        element_area=element_area,
        max_nozzle_velocity=max_nozzle_velocity,
        min_ld_ratio=min_ld_ratio,
        max_ld_ratio=max_ld_ratio,
        design_pressure_margin=design_pressure_margin,
        minimum_design_pressure_margin=minimum_design_pressure_margin,
        design_temperature_margin=design_temperature_margin,
        allowable_stress=allowable_stress,
        joint_efficiency=joint_efficiency,
        corrosion_allowance=corrosion_allowance,
        element_collapse_pressure=element_collapse_pressure,
        element_integrity_verified=element_integrity_verified,
    )


def active_fire_protection_screening(
    scenario: str,
    blowdown_time_to_target: Q,
    bare_steel_time_to_critical: Q,
    dedicated_object_demand: Q,
    general_area_demand: Q,
) -> ActiveFireProtectionScreeningResult:
    """The active-versus-passive fire protection screen and its machine-readable facts.

    The class's arithmetic only: water extinguishes a pool or an ordinary combustible fire
    and not a jet; **blowdown is the governing control when it is faster than the steel**,
    with `bare - blowdown` as the margin; and selective protection's saving is the fraction of
    the area requirement it does not discharge. The verdict, the ranked measures and the
    substitution argument are prose and are not carried.

    Raises:
        InvalidInputError: for a non-positive blowdown or time to critical, a negative
            dedicated demand, a non-positive general demand, or an unknown scenario.

    See :func:`azoth.process.reference.active_fire_protection_screening`.
    """
    return resolve(_ACTIVE_FIRE_PROTECTION_SCREENING)(  # type: ignore[no-any-return]
        scenario=scenario,
        blowdown_time_to_target=blowdown_time_to_target,
        bare_steel_time_to_critical=bare_steel_time_to_critical,
        dedicated_object_demand=dedicated_object_demand,
        general_area_demand=general_area_demand,
    )


def api537_flare_flame_model(
    mass_flow: Q,
    heat_of_combustion: float,
    radiative_fraction: float,
    exit_velocity: Q,
    ground_distance: Q,
    sound_distance: Q,
    target_fluxes: list[dict[str, object]],
    stack_height: Q | None = None,
    wind_speed: Q | None = None,
    transmissivity: float | None = None,
    acoustic_efficiency: float | None = None,
) -> Api537FlareFlameModelResult:
    """The API 537 flare flame: Kent's length, the tilt, the flux, sterile zones and noise.

    The point-source chain the class publishes: ``Q = m hc``, ``L = 0.00326 Q^0.478``, the
    tilt ``atan2(wind, exit)``, the ground flux ``tau qr / (4 pi r^2)`` from the flame
    centre and one sterile radius per target flux. The noise is ``10 log10(eta Q / 1e-12)``
    dB and the pressure level subtracts ``20 log10(max(d, 1)) + 11``. **The class's clamps
    are mirrored and unreachable**: negative wind and efficiency are refused rather than
    clamped, and so is a negative stack height the class accepts.

    Raises:
        InvalidInputError: for a negative mass flow, a non-positive heat of combustion or
            exit velocity, a radiative fraction outside ``[0, 1]``, a negative distance,
            stack height or efficiency, a transmissivity outside ``[0, 1]``, or a
            non-positive target flux.

    See :func:`azoth.process.reference.api537_flare_flame_model`.
    """
    return resolve(_API537_FLARE_FLAME_MODEL)(  # type: ignore[no-any-return]
        mass_flow=mass_flow,
        heat_of_combustion=heat_of_combustion,
        radiative_fraction=radiative_fraction,
        exit_velocity=exit_velocity,
        ground_distance=ground_distance,
        sound_distance=sound_distance,
        target_fluxes=target_fluxes,
        stack_height=stack_height,
        wind_speed=wind_speed,
        transmissivity=transmissivity,
        acoustic_efficiency=acoustic_efficiency,
    )


def anti_surge_recycle(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    curve: list[dict[str, Q]],
    surge_curve: list[dict[str, Q]],
    speed: float | None = None,
    surge_control_margin: float | None = None,
    recycle_cooler_temperature: Q | None = None,
    damping: float | None = None,
    tolerance: float | None = None,
    max_iterations: int | None = None,
) -> AntiSurgeRecycleResult:
    """Converge the cooled recycle that holds a charted compressor on its control line.

    **The object is a loop around :func:`compressor`'s chart branch, not a machine.** It runs
    the natural suction, compares its inlet volume flow with ``surge * (1 + margin)``, and -
    where the point is short - wires a cooled clone of the suction through a mixer and converges
    the recycle mass with a damped fixed point. The run-time recycle block is off throughout,
    because this loop owns the recycle.

    **A converged pass breaks before updating**, so the reported recycle mass is the one the
    machine last ran at; an exhausted ``max_iterations`` reports the last update instead. A
    natural inlet at or above the control line opens nothing and reports ``converged`` true
    with zero passes.

    See :func:`azoth.process.reference.anti_surge_recycle`.
    """
    return resolve(_ANTI_SURGE_RECYCLE)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        curve=curve,
        surge_curve=surge_curve,
        speed=speed,
        surge_control_margin=surge_control_margin,
        recycle_cooler_temperature=recycle_cooler_temperature,
        damping=damping,
        tolerance=tolerance,
        max_iterations=max_iterations,
    )


def bleve(
    fuel_mass: Q,
    heat_of_combustion: float,
    radiative_fraction: float,
    relative_humidity: float | None = None,
    distance: Q | None = None,
    target_flux: Q | None = None,
) -> BleveResult:
    """The fireball radiation at one distance, or the distance to a target flux.

    The CCPS/TNO fireball's diameter, duration and surface emissive power, the capped
    point-source view factor and the jet fire's shared transmissivity. **A target above
    the near-field flux is absent**: the class answers ``D/2 + 0.1``.

    See :func:`azoth.process.reference.bleve`.
    """
    return resolve(_BLEVE)(  # type: ignore[no-any-return]
        fuel_mass=fuel_mass,
        heat_of_combustion=heat_of_combustion,
        radiative_fraction=radiative_fraction,
        relative_humidity=relative_humidity,
        distance=distance,
        target_flux=target_flux,
    )


def blocked_outlet_relief(
    components: list[str],
    mole_fractions: list[float],
    inflow_rate: Q,
    relief_pressure: Q,
    relief_temperature: Q,
    scenario_name: str | None = None,
) -> BlockedOutletReliefResult:
    """The full-inflow relief rate and the relieving-fluid state at it.

    **The relief rate is the maximum credible inflow**, TR3001 SR-26460's basis, and the
    fluid is flashed at the relieving pressure and temperature for its ``getKappa``, molar
    mass, ``Z`` and phase. ``specific_heat_ratio`` is that isentropic exponent, not the
    plain ``Cp/Cv``.

    See :func:`azoth.process.reference.blocked_outlet_relief`.
    """
    return resolve(_BLOCKED_OUTLET_RELIEF)(  # type: ignore[no-any-return]
        components=components,
        mole_fractions=mole_fractions,
        inflow_rate=inflow_rate,
        relief_pressure=relief_pressure,
        relief_temperature=relief_temperature,
        scenario_name=scenario_name,
    )


def check_valve_leak_relief(
    components: list[str],
    mole_fractions: list[float],
    nominal_diameter: Q,
    upstream_pressure: Q,
    leak_area_fraction: float | None = None,
    upstream_temperature: Q | None = None,
    downstream_pressure: Q | None = None,
    discharge_coefficient: float | None = None,
    scenario_name: str | None = None,
) -> CheckValveLeakReliefResult:
    """The leakage rate through a fraction of the nominal check-valve area.

    **The fraction defaults to 1% and is clamped up to 0.1%** with the class's warning,
    and the flow is the isentropic nozzle through the leak area. A zero diameter answers
    zero.

    See :func:`azoth.process.reference.check_valve_leak_relief`.
    """
    return resolve(_CHECK_VALVE_LEAK_RELIEF)(  # type: ignore[no-any-return]
        components=components,
        mole_fractions=mole_fractions,
        nominal_diameter=nominal_diameter,
        upstream_pressure=upstream_pressure,
        leak_area_fraction=leak_area_fraction,
        upstream_temperature=upstream_temperature,
        downstream_pressure=downstream_pressure,
        discharge_coefficient=discharge_coefficient,
        scenario_name=scenario_name,
    )


def component_splitter(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    split_factors: list[float],
) -> ComponentSplitterResult:
    """Divide a stream between two outlets component by component, flashing each.

    **The factor is per component and the outlet count is two.** ``ComponentSplitter.run``
    loops ``for i in 0..2`` and reads ``splitFactor[k]`` as the fraction of component *k* to
    the overhead, so the outlets carry different compositions - which is what separates this
    from :func:`splitter`.

    **The outlet enthalpies are the state's and not the class's**: a fresh NeqSim fluid at
    the first captured row's bottoms gives ``-20309.24832914077`` where the class reports
    ``-14431.31737097011``, the same defect :func:`splitter` records.

    Raises:
        InvalidInputError: where the factors are not one per component, or any is outside
            ``[0, 1]``.

    See :func:`azoth.process.reference.component_splitter`.
    """
    return resolve(_COMPONENT_SPLITTER)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        split_factors=split_factors,
    )


def compressor(
    components: list[str],
    inlet_n: Q,
    inlet_z: list[float],
    inlet_p: Q,
    inlet_t: Q,
    isentropic_efficiency: float,
    use_polytropic_calc: bool | None = None,
    polytropic_efficiency: float | None = None,
    polytropic_method: str | None = None,
    compression_ratio: float | None = None,
    max_outlet_pressure: Q | None = None,
    power: Q | None = None,
    deposit: list[dict[str, Q]] | None = None,
    wetted_area: Q | None = None,
    passage_half_height: Q | None = None,
    outlet_pressure: Q | None = None,
    curve: list[dict[str, Q]] | None = None,
    speed: float | None = None,
    solve_speed: bool | None = None,
    min_speed: float | None = None,
    max_speed: float | None = None,
    limit_speed: bool | None = None,
    chart_type: str | None = None,
    chart_conditions: list[float] | None = None,
    impeller_diameter: Q | None = None,
    surge_curve: list[dict[str, Q]] | None = None,
    surge_control_margin: float | None = None,
    anti_surge_active: bool | None = None,
    surge_control_factor: float | None = None,
    mechanical_design: bool | None = None,
    max_stages_per_casing: float | None = None,
) -> CompressorResult:
    """Raise a stream's pressure along an isentrope or the machine's chart.

    **The entropy is derived, not carried.** The record has five fields and ``s`` is not one
    of them: it is a function of ``(T, P, z)``, and the flash at the inlet decides it. At an
    efficiency of one the outlet's entropy equals the inlet's, which is what the captured
    reversible row is for.

    ``h_out = h_in + (h_isentropic - h_in) / eta``. :func:`expander` is the same route with a
    multiplication, because an expansion's isentropic difference is negative.

    ``curve`` selects ``CompressorChart``'s fixed-speed branch instead: the pressure is solved
    from the head, ``speed`` reads the map, and ``chart_type`` names the map class (only the
    default ``fan_law`` is carried). With ``solve_speed`` the speed is the unknown: a Newton
    drives the chart's pressure ratio to ``outlet_pressure``, and ``min_speed``/``max_speed``
    bound it, with ``limit_speed`` pegging an out-of-range answer at the bound.

    ``surge_curve`` is the anti-surge boundary in the same (flow, head) unit as ``curve``.
    With it the result carries the machine's surge read-outs; with ``anti_surge_active`` set,
    the run-time recycle block re-runs the machine above the surge flow and reports the
    fraction and the boosted power.

    Raises:
        InvalidInputError: where the shapes disagree, the efficiency is outside ``(0, 1]``, or
            the chart type is refused.

    See :func:`azoth.process.reference.compressor`.
    """
    return resolve(_COMPRESSOR)(  # type: ignore[no-any-return]
        components=components,
        inlet_n=inlet_n,
        inlet_z=inlet_z,
        inlet_p=inlet_p,
        inlet_t=inlet_t,
        isentropic_efficiency=isentropic_efficiency,
        use_polytropic_calc=use_polytropic_calc,
        polytropic_efficiency=polytropic_efficiency,
        polytropic_method=polytropic_method,
        compression_ratio=compression_ratio,
        max_outlet_pressure=max_outlet_pressure,
        power=power,
        deposit=deposit,
        wetted_area=wetted_area,
        passage_half_height=passage_half_height,
        outlet_pressure=outlet_pressure,
        curve=curve,
        speed=speed,
        solve_speed=solve_speed,
        min_speed=min_speed,
        max_speed=max_speed,
        limit_speed=limit_speed,
        chart_type=chart_type,
        chart_conditions=chart_conditions,
        impeller_diameter=impeller_diameter,
        surge_curve=surge_curve,
        surge_control_margin=surge_control_margin,
        anti_surge_active=anti_surge_active,
        surge_control_factor=surge_control_factor,
        mechanical_design=mechanical_design,
        max_stages_per_casing=max_stages_per_casing,
    )


def expander(
    components: list[str],
    inlet_n: Q,
    inlet_z: list[float],
    inlet_p: Q,
    inlet_t: Q,
    outlet_pressure: Q,
    isentropic_efficiency: float,
    use_polytropic_calc: bool | None = None,
    polytropic_efficiency: float | None = None,
    number_of_steps: int | None = None,
    rated_recovered_power: Q | None = None,
    shaft_power: Q | None = None,
    outlet_temperature: Q | None = None,
    mechanical_design: bool | None = None,
) -> ExpanderResult:
    """Drop a stream's pressure along an isentrope, multiplying the step by the efficiency.

    ``h_out = h_in + (h_isentropic - h_in) * eta`` - the compressor's rule with the efficiency
    on the other side of the division, because an expansion's isentropic difference is
    negative and dividing would make the machine beat the reversible one at any efficiency
    below one. The captured pair of rows is what holds that: `0.75` of the step is
    ``-618.933`` J/mol and the step divided would be ``-1478.8``.

    ``rated_recovered_power`` is ``setRatedRecoveredPower``: a positive rating adds the HARD
    ``recoveredPower`` constraint, whose utilization is ``|power| / rating`` and whose hard
    band is ``1.1 x`` the rating. Absent or non-positive creates no constraint.

    ``shaft_power`` states the total power instead of an efficiency and reports the
    back-computed one; ``outlet_temperature`` solves the efficiency for that outlet. Both are
    the isentropic branch's, and one at a time.

    Raises:
        InvalidInputError: where the shapes disagree or the efficiency is outside ``(0, 1]``.

    See :func:`azoth.process.reference.expander`.
    """
    return resolve(_EXPANDER)(  # type: ignore[no-any-return]
        components=components,
        inlet_n=inlet_n,
        inlet_z=inlet_z,
        inlet_p=inlet_p,
        inlet_t=inlet_t,
        outlet_pressure=outlet_pressure,
        isentropic_efficiency=isentropic_efficiency,
        use_polytropic_calc=use_polytropic_calc,
        polytropic_efficiency=polytropic_efficiency,
        number_of_steps=number_of_steps,
        rated_recovered_power=rated_recovered_power,
        shaft_power=shaft_power,
        outlet_temperature=outlet_temperature,
        mechanical_design=mechanical_design,
    )


def mass_balance_leak_detection(
    components: list[str],
    z: list[float],
    inlet_pressure: float,
    inlet_temperature: float,
    inlet_flow: float,
    outlet_flow: float,
    eos: str | None = None,
    inlet_density: float | None = None,
    linepack_volume: float | None = None,
    flow_measurement_uncertainty_fraction: float | None = None,
    pressure_uncertainty: float | None = None,
    temperature_uncertainty: float | None = None,
    detection_window: float | None = None,
    confidence_multiplier: float | None = None,
    inlet_stream_name: str | None = None,
    outlet_stream_name: str | None = None,
) -> MassBalanceLeakDetectionResult:
    """The mass-balance leak sensitivity.

    The measured imbalance, the two meters' uncertainty, the linepack's pressure and
    temperature uncertainty, and the minimum detectable rate. **The setters' clamps are
    carried**: volume, fraction and the uncertainties floor at zero and the window at one.

    See :func:`azoth.process.reference.mass_balance_leak_detection`.
    """
    return resolve(_MASS_BALANCE_LEAK_DETECTION)(  # type: ignore[no-any-return]
        components=components,
        z=z,
        inlet_pressure=inlet_pressure,
        inlet_temperature=inlet_temperature,
        inlet_flow=inlet_flow,
        outlet_flow=outlet_flow,
        eos=eos,
        inlet_density=inlet_density,
        linepack_volume=linepack_volume,
        flow_measurement_uncertainty_fraction=flow_measurement_uncertainty_fraction,
        pressure_uncertainty=pressure_uncertainty,
        temperature_uncertainty=temperature_uncertainty,
        detection_window=detection_window,
        confidence_multiplier=confidence_multiplier,
        inlet_stream_name=inlet_stream_name,
        outlet_stream_name=outlet_stream_name,
    )


def mdmt(
    material_curve: str,
    governing_thickness: Q,
    min_operating_temperature: Q,
    stress_ratio: float | None = None,
) -> MdmtResult:
    """The UCS-66 curve's MDMT and the cold-service verdict.

    The thickness is clamped to 6-100 mm. The four curves are analytic approximations of
    ASME Fig. UCS-66, **not the figure's table**, and the verdict is
    ``min_operating_temperature >= MDMT``.

    See :func:`azoth.process.reference.mdmt`.
    """
    return resolve(_MDMT)(  # type: ignore[no-any-return]
        material_curve=material_curve,
        governing_thickness=governing_thickness,
        stress_ratio=stress_ratio,
        min_operating_temperature=min_operating_temperature,
    )


def acoustic_induced_vibration_screening(
    mass_flow: Q,
    upstream_pressure: Q,
    downstream_pressure: Q,
    temperature: Q,
    molecular_weight: Q,
    outside_diameter: Q,
    wall_thickness: Q,
) -> AcousticInducedVibrationResult:
    """The sound power level downstream of a pressure-reducing device and its band.

    The bands are exclusive upward, so -10 dB is medium, 0 high and +10 very high; the
    downstream pressure must be strictly below the upstream one.

    See :func:`azoth.process.reference.acoustic_induced_vibration_screening`.
    """
    return resolve(_ACOUSTIC_INDUCED_VIBRATION_SCREENING)(  # type: ignore[no-any-return]
        mass_flow=mass_flow,
        upstream_pressure=upstream_pressure,
        downstream_pressure=downstream_pressure,
        temperature=temperature,
        molecular_weight=molecular_weight,
        outside_diameter=outside_diameter,
        wall_thickness=wall_thickness,
    )


def flow_induced_pulsation_screening(
    acoustic_length: Q,
    branch_internal_diameter: Q,
    run_internal_diameter: Q,
    main_line_velocity: Q,
    main_line_density: Q,
    speed_of_sound: Q,
    edge_radius: Q | None = None,
    termination: str | None = None,
    strouhal_number: float | None = None,
    mode_count: float | None = None,
) -> FlowInducedPulsationResult:
    """The side-branch pulsation likelihood and every evaluated acoustic mode.

    A closed branch is a quarter-wave tube and an open one a half-wave tube; a mode locks in
    inside ``[0.8 f_n, 1.2 f_n]``. Only the strongest locked mode's weight enters the
    severity, and a branch with no lock-in or a severity below 0.05 is low.

    See :func:`azoth.process.reference.flow_induced_pulsation_screening`.
    """
    return resolve(_FLOW_INDUCED_PULSATION_SCREENING)(  # type: ignore[no-any-return]
        acoustic_length=acoustic_length,
        branch_internal_diameter=branch_internal_diameter,
        run_internal_diameter=run_internal_diameter,
        main_line_velocity=main_line_velocity,
        main_line_density=main_line_density,
        speed_of_sound=speed_of_sound,
        edge_radius=edge_radius,
        termination=termination,
        strouhal_number=strouhal_number,
        mode_count=mode_count,
    )


def norsok_p002_compliance(
    checks: list[dict[str, object]],
) -> NorsokP002ComplianceResult:
    """The NORSOK P-002 point checks and their findings.

    Each numeric check is ``value <= limit``, so equality passes; the defaults are the
    class's. The two recorded criteria read only the caller's pass flag, and a row states
    only the field its criterion reads. **A passing report covers the supplied checks
    alone**, not conformity with the whole standard.

    See :func:`azoth.process.reference.norsok_p002_compliance`.
    """
    return resolve(_NORSOK_P002_COMPLIANCE)(  # type: ignore[no-any-return]
        checks=checks,
    )


def sts0131_gate(
    checks: list[dict[str, object]],
) -> Sts0131GateResult:
    """The STS-0131 gate's findings and its overall acceptability.

    The PSV margin passes at or above its required value; the MDMT check passes at or above
    the design MDMT; the trapped-liquid check passes when no rupture is predicted. A row
    states only the field its check reads.

    See :func:`azoth.process.reference.sts0131_gate`.
    """
    return resolve(_STS0131_GATE)(  # type: ignore[no-any-return]
        checks=checks,
    )


def inherent_safety(
    substitute_score: float | None = None,
    minimize_score: float | None = None,
    moderate_score: float | None = None,
    simplify_score: float | None = None,
) -> InherentSafetyResult:
    """The mean of the four Kletz inherent-safety pillars.

    Each score is in ``[0, 10]`` and 10 is the best; an absent pillar is zero, the class's
    zero-initialised array.

    See :func:`azoth.process.reference.inherent_safety`.
    """
    return resolve(_INHERENT_SAFETY)(  # type: ignore[no-any-return]
        substitute_score=substitute_score,
        minimize_score=minimize_score,
        moderate_score=moderate_score,
        simplify_score=simplify_score,
    )


def alarp_audit(
    measures: list[dict[str, object]],
    value_of_statistical_life: float | None = None,
    disproportion_factor: float | None = None,
) -> AlarpAuditResult:
    """Each measure's ICAF and its ALARP verdict.

    A measure is implemented at or below ``VSL x disproportion_factor``; a zero-risk measure
    has an infinite ICAF, published absent. The amounts are in NOK and declared
    dimensionless.

    See :func:`azoth.process.reference.alarp_audit`.
    """
    return resolve(_ALARP_AUDIT)(  # type: ignore[no-any-return]
        measures=measures,
        value_of_statistical_life=value_of_statistical_life,
        disproportion_factor=disproportion_factor,
    )


def escalation_graph(
    items: list[dict[str, object]],
    exposures: list[dict[str, object]],
    initiator: str,
) -> EscalationGraphResult:
    """The items that fail from an initiator, and the worst case over every item.

    A target fails when the cumulative load from every failed source is at or above its
    threshold. The sets are published in the items order.

    See :func:`azoth.process.reference.escalation_graph`.
    """
    return resolve(_ESCALATION_GRAPH)(  # type: ignore[no-any-return]
        items=items,
        exposures=exposures,
        initiator=initiator,
    )


def piping_fiv_gas(
    density: Q,
    velocity: Q,
    outside_diameter: Q,
    wall_thickness: Q,
    branch_count: float,
    pulsation_factor: float,
    support_quality_factor: float,
) -> PipingFivGasResult:
    """The gas-line flow-induced-vibration likelihood of failure.

    The pulsation factor is clamped to ``[1, 4]`` and the support-quality factor to
    ``[1, 3]``; the bands are exclusive upward, so 0.3 is medium, 0.5 high and 1.0 very high.

    See :func:`azoth.process.reference.piping_fiv_gas`.
    """
    return resolve(_PIPING_FIV_GAS)(  # type: ignore[no-any-return]
        density=density,
        velocity=velocity,
        outside_diameter=outside_diameter,
        wall_thickness=wall_thickness,
        branch_count=branch_count,
        pulsation_factor=pulsation_factor,
        support_quality_factor=support_quality_factor,
    )


def piping_fiv_liquid(
    velocity: Q,
    outside_diameter: Q,
    wall_thickness: Q,
    branch_count: float,
    support_quality_factor: float,
) -> PipingFivLiquidResult:
    """The liquid-line flow-induced-vibration likelihood of failure.

    The support-quality factor is clamped to ``[1, 3]``; the bands are exclusive upward, so
    0.3 is medium, 0.5 high and 1.0 very high.

    See :func:`azoth.process.reference.piping_fiv_liquid`.
    """
    return resolve(_PIPING_FIV_LIQUID)(  # type: ignore[no-any-return]
        velocity=velocity,
        outside_diameter=outside_diameter,
        wall_thickness=wall_thickness,
        branch_count=branch_count,
        support_quality_factor=support_quality_factor,
    )


def map_turbo_expander_compressor(
    expander_components: list[str],
    compressor_components: list[str],
    expander_n: Q,
    expander_z: list[float],
    expander_p: Q,
    expander_t: Q,
    compressor_n: Q,
    compressor_z: list[float],
    compressor_p: Q,
    compressor_t: Q,
    expander_out_pressure: Q,
    expander_isentropic_efficiency: float,
    shaft_mode: str | None = None,
    mechanical_efficiency: float | None = None,
    bearing_loss: Q | None = None,
    compressor_outlet_pressure: Q | None = None,
    compressor_isentropic_efficiency: float | None = None,
    compressor_speed: float | None = None,
    min_shaft_speed: float | None = None,
    max_shaft_speed: float | None = None,
    curve: list[dict[str, Q]] | None = None,
    chart_type: str | None = None,
    chart_conditions: list[float] | None = None,
    impeller_diameter: Q | None = None,
) -> MapTurboExpanderResult:
    """Balance a map-driven brake compressor against an expander on one shaft.

    The expander's power, less the mechanical and bearing losses, is what the compressor may
    consume. ``shaft_mode="balanced_speed"`` bisects the shaft speed over the compressor
    map's own measured range until the compressor's power meets the available power - pegged
    at a bound with ``under_power_surge`` or ``over_power_max_speed`` when it cannot.
    ``specified_pressures`` runs the compressor at its own setting and reports the residual.

    See :func:`azoth.process.reference.map_turbo_expander_compressor`.
    """
    return resolve(_MAP_TURBO_EXPANDER_COMPRESSOR)(  # type: ignore[no-any-return]
        expander_components=expander_components,
        compressor_components=compressor_components,
        expander_n=expander_n,
        expander_z=expander_z,
        expander_p=expander_p,
        expander_t=expander_t,
        compressor_n=compressor_n,
        compressor_z=compressor_z,
        compressor_p=compressor_p,
        compressor_t=compressor_t,
        expander_out_pressure=expander_out_pressure,
        expander_isentropic_efficiency=expander_isentropic_efficiency,
        shaft_mode=shaft_mode,
        mechanical_efficiency=mechanical_efficiency,
        bearing_loss=bearing_loss,
        compressor_outlet_pressure=compressor_outlet_pressure,
        compressor_isentropic_efficiency=compressor_isentropic_efficiency,
        compressor_speed=compressor_speed,
        min_shaft_speed=min_shaft_speed,
        max_shaft_speed=max_shaft_speed,
        curve=curve,
        chart_type=chart_type,
        chart_conditions=chart_conditions,
        impeller_diameter=impeller_diameter,
    )


def tube_rupture_relief(
    components: list[str],
    mole_fractions: list[float],
    tube_inner_diameter: Q,
    high_pressure: Q,
    number_of_open_ends: float | None = None,
    high_temperature: Q | None = None,
    low_pressure: Q | None = None,
    discharge_coefficient: float | None = None,
    scenario_name: str | None = None,
) -> TubeRuptureReliefResult:
    """The full-bore tube-rupture rate through one or two open ends.

    **A full-bore single-tube rupture presents two open ends**, the class's default, and
    the flow is the isentropic nozzle through their combined bore. A count below one is
    refused rather than reset. Vaporization on the low-pressure side is not modelled.

    See :func:`azoth.process.reference.tube_rupture_relief`.
    """
    return resolve(_TUBE_RUPTURE_RELIEF)(  # type: ignore[no-any-return]
        components=components,
        mole_fractions=mole_fractions,
        tube_inner_diameter=tube_inner_diameter,
        high_pressure=high_pressure,
        number_of_open_ends=number_of_open_ends,
        high_temperature=high_temperature,
        low_pressure=low_pressure,
        discharge_coefficient=discharge_coefficient,
        scenario_name=scenario_name,
    )


def turbo_expander_compressor(
    expander_components: list[str],
    compressor_components: list[str],
    expander_n: Q,
    expander_z: list[float],
    expander_p: Q,
    expander_t: Q,
    compressor_n: Q,
    compressor_z: list[float],
    compressor_p: Q,
    compressor_t: Q,
    expander_out_pressure: Q,
    impeller_diameter: Q,
    design_speed: float,
    design_uc: float,
    design_qn: float,
    igv_opening: float | None = None,
    igv_control_mode: bool | None = None,
    maximum_igv_area: Q | None = None,
    igv_area_increase_factor: float | None = None,
    bearing_loss: Q | None = None,
    expander_design_isentropic_efficiency: float | None = None,
    compressor_design_polytropic_efficiency: float | None = None,
    compressor_design_polytropic_head: Q | None = None,
    expander_out_temperature: Q | None = None,
    uc_curve: list[dict[str, float]] | None = None,
    qn_efficiency_curve: list[dict[str, float]] | None = None,
    qn_head_curve: list[dict[str, float]] | None = None,
    chart: list[dict[str, float]] | None = None,
    igv_penalty_curve: list[dict[str, float]] | None = None,
    mechanical_design: bool | None = None,
) -> TurboExpanderResult:
    """Balance a turbo-expander and a boost compressor on one shaft.

    **The speed is the only unknown.** The expander's isentropic head comes from one
    ``PSflash`` at ``expander_out_pressure`` and its efficiency from the velocity-ratio
    parabola or the 2-D map; the compressor's absorbed power is its design polytropic head
    scaled by ``(N/N_design)²`` and a Q/N head factor over a Q/N efficiency factor times the
    design efficiency. A damped Newton on ``N`` drives the expander's power to the
    compressor's plus the bearing loss ``10 (N/N_design)²``.

    **``chart`` selects the map branch**, whose rows carry ``(igv, uc, efficiency)``; its
    stage-head grid is not read by the machine. Stating ``expander_out_temperature`` runs
    the inverse: the required efficiency is solved from the target state and the design
    efficiency is back-calculated from the converged ratio.

    Raises:
        InvalidInputError: for a non-positive design or geometry value, an efficiency outside
            ``(0, 1]``, a non-positive isentropic drop or a map that is not a map.

    See :func:`azoth.process.reference.turbo_expander_compressor`.
    """
    return resolve(_TURBO_EXPANDER_COMPRESSOR)(  # type: ignore[no-any-return]
        expander_components=expander_components,
        compressor_components=compressor_components,
        expander_n=expander_n,
        expander_z=expander_z,
        expander_p=expander_p,
        expander_t=expander_t,
        compressor_n=compressor_n,
        compressor_z=compressor_z,
        compressor_p=compressor_p,
        compressor_t=compressor_t,
        expander_out_pressure=expander_out_pressure,
        impeller_diameter=impeller_diameter,
        design_speed=design_speed,
        design_uc=design_uc,
        design_qn=design_qn,
        igv_opening=igv_opening,
        igv_control_mode=igv_control_mode,
        maximum_igv_area=maximum_igv_area,
        igv_area_increase_factor=igv_area_increase_factor,
        bearing_loss=bearing_loss,
        expander_design_isentropic_efficiency=expander_design_isentropic_efficiency,
        compressor_design_polytropic_efficiency=compressor_design_polytropic_efficiency,
        compressor_design_polytropic_head=compressor_design_polytropic_head,
        expander_out_temperature=expander_out_temperature,
        uc_curve=uc_curve,
        qn_efficiency_curve=qn_efficiency_curve,
        qn_head_curve=qn_head_curve,
        chart=chart,
        igv_penalty_curve=igv_penalty_curve,
        mechanical_design=mechanical_design,
    )


def consequence_analysis(
    release_frequency_per_year: float,
    receiver_distance: Q,
    outcomes: list[dict[str, object]],
) -> ConsequenceAnalysisResult:
    """The class's four consequence branches over one receiver, and the summed risk.

    The jet, pool, VCE and toxic branches in caller order, over the ported consequence
    kernels. **A zero dose is a zero probability**, and a field belonging to another
    branch type is refused.

    See :func:`azoth.process.reference.consequence_analysis`.
    """
    return resolve(_CONSEQUENCE_ANALYSIS)(  # type: ignore[no-any-return]
        release_frequency_per_year=release_frequency_per_year,
        receiver_distance=receiver_distance,
        outcomes=outcomes,
    )


def control_valve_failure_relief(
    components: list[str],
    mole_fractions: list[float],
    upstream_pressure: Q,
    upstream_temperature: Q | None = None,
    downstream_pressure: Q | None = None,
    effective_area: Q | None = None,
    cv: float | None = None,
    cv_erosion_margin: float | None = None,
    discharge_coefficient: float | None = None,
    scenario_name: str | None = None,
) -> ControlValveFailureReliefResult:
    """The fail-open rate from an explicit area or a Cv with an erosion margin.

    **The area is the explicit one when it is positive, else the Cv through the class's
    screening relation.** The fluid is flashed at the upstream pressure and temperature for
    its ``getKappa``, molar mass and ``Z``, and the downstream pressure defaults to
    1.01325 bara.

    See :func:`azoth.process.reference.control_valve_failure_relief`.
    """
    return resolve(_CONTROL_VALVE_FAILURE_RELIEF)(  # type: ignore[no-any-return]
        components=components,
        mole_fractions=mole_fractions,
        upstream_pressure=upstream_pressure,
        upstream_temperature=upstream_temperature,
        downstream_pressure=downstream_pressure,
        effective_area=effective_area,
        cv=cv,
        cv_erosion_margin=cv_erosion_margin,
        discharge_coefficient=discharge_coefficient,
        scenario_name=scenario_name,
    )


def cooler(
    components: list[str],
    inlet_n: Q,
    inlet_z: list[float],
    inlet_p: Q,
    inlet_t: Q,
    outlet_temperature: Q | None = None,
    duty: Q | None = None,
    pressure_drop: Q | None = None,
    time_step: Q | None = None,
    dynamic_temperature_control_enabled: bool | None = None,
    design_mass_flow: Q | None = None,
    design_inlet_temperature: Q | None = None,
    design_outlet_temperature: Q | None = None,
    cooling_medium_temperature: Q | None = None,
    design_valve_opening: float | None = None,
    actuator_time_constant: Q | None = None,
    thermal_time_constant: Q | None = None,
    cooling_valve_opening: float | None = None,
    requested_valve_opening: float | None = None,
    dynamic_outlet_temperature: Q | None = None,
) -> CoolerResult:
    """Cool a stream to a stated temperature, or by a stated duty.

    ``unit_ops.cooler``'s three parameters are ``Heater``'s three setters, reached through
    ``Cooler`` - and ``Cooler`` adds no steady-state arithmetic to ``Heater``, so this is the
    same call as :func:`heater` under the entry a palette shows for a machine that removes
    heat. ``duty`` is signed and unchecked: ``-5000`` W is the usual case here, and a
    positive value heats.

    Raises:
        InvalidInputError: for both specifications at once, a pressure drop that leaves a
            non-positive pressure, or a duty on a stream carrying no flow.

    See :func:`azoth.process.reference.cooler`.
    """
    return resolve(_COOLER)(  # type: ignore[no-any-return]
        components=components,
        inlet_n=inlet_n,
        inlet_z=inlet_z,
        inlet_p=inlet_p,
        inlet_t=inlet_t,
        outlet_temperature=outlet_temperature,
        duty=duty,
        pressure_drop=pressure_drop,
        time_step=time_step,
        dynamic_temperature_control_enabled=dynamic_temperature_control_enabled,
        design_mass_flow=design_mass_flow,
        design_inlet_temperature=design_inlet_temperature,
        design_outlet_temperature=design_outlet_temperature,
        cooling_medium_temperature=cooling_medium_temperature,
        design_valve_opening=design_valve_opening,
        actuator_time_constant=actuator_time_constant,
        thermal_time_constant=thermal_time_constant,
        cooling_valve_opening=cooling_valve_opening,
        requested_valve_opening=requested_valve_opening,
        dynamic_outlet_temperature=dynamic_outlet_temperature,
    )


def heater(
    components: list[str],
    inlet_n: Q,
    inlet_z: list[float],
    inlet_p: Q,
    inlet_t: Q,
    outlet_temperature: Q | None = None,
    duty: Q | None = None,
    pressure_drop: Q | None = None,
    max_outlet_temperature: Q | None = None,
    min_outlet_temperature: Q | None = None,
    instrument_design: bool | None = None,
    include_safety_instruments: bool | None = None,
    default_sil_level: float | None = None,
    heat_exchanger_type: str | None = None,
    electrical_design: bool | None = None,
    motor_sizing_margin: float | None = None,
    motor_poles: float | None = None,
    efficiency_class: str | None = None,
    frequency_hz: float | None = None,
    phases: float | None = None,
    number_of_fans: float | None = None,
    instrumentation_power: Q | None = None,
    cooling_water_pump_power: Q | None = None,
    cable_length: Q | None = None,
    use_vfd: bool | None = None,
    utility_specification: bool | None = None,
    utility_supply_temperature: Q | None = None,
    utility_return_temperature: Q | None = None,
    utility_approach_temperature: Q | None = None,
    utility_heat_capacity_rate: Q | None = None,
    utility_overall_heat_transfer_coefficient: Q | None = None,
    mechanical_design: bool | None = None,
    mechanical_type: str | None = None,
    mechanical_selection: str | None = None,
) -> HeaterResult:
    """Heat or cool a stream to a stated temperature, or by a stated duty.

    ``components``, ``inlet_n``, ``inlet_z``, ``inlet_p`` and ``inlet_t`` are the inlet's
    record, and ``outlet_temperature``, ``duty`` and ``pressure_drop`` are
    ``unit_ops.heater``'s own three parameters - all optional, and the pressure drop applied
    to the inlet before the flash.

    **A temperature and a duty together are refused**, because ``Heater.setOutletTemperature``
    and ``setDuty`` clear each other's flags: which one the class runs is the order they were
    set in, which these arguments cannot express. With neither, the drop is isothermal,
    because ``run``'s else branch is ``T_in + dT`` with ``dT`` zero.

    ``max_outlet_temperature`` and ``min_outlet_temperature`` are the class's two HARD
    capacity limits. Supplying either initializes the map; the maximum's utilization is
    ``outlet_t / limit`` and the minimum's is the inverse, both capped at 9.99. Only the
    minimum can trip ``hard_limit_exceeded``.

    Raises:
        InvalidInputError: for both specifications at once, a pressure drop that leaves a
            non-positive pressure, or a duty on a stream carrying no flow.

    See :func:`azoth.process.reference.heater`.
    """
    return resolve(_HEATER)(  # type: ignore[no-any-return]
        components=components,
        inlet_n=inlet_n,
        inlet_z=inlet_z,
        inlet_p=inlet_p,
        inlet_t=inlet_t,
        outlet_temperature=outlet_temperature,
        duty=duty,
        pressure_drop=pressure_drop,
        max_outlet_temperature=max_outlet_temperature,
        min_outlet_temperature=min_outlet_temperature,
        instrument_design=instrument_design,
        include_safety_instruments=include_safety_instruments,
        default_sil_level=default_sil_level,
        heat_exchanger_type=heat_exchanger_type,
        electrical_design=electrical_design,
        motor_sizing_margin=motor_sizing_margin,
        motor_poles=motor_poles,
        efficiency_class=efficiency_class,
        frequency_hz=frequency_hz,
        phases=phases,
        number_of_fans=number_of_fans,
        instrumentation_power=instrumentation_power,
        cooling_water_pump_power=cooling_water_pump_power,
        cable_length=cable_length,
        use_vfd=use_vfd,
        utility_specification=utility_specification,
        utility_supply_temperature=utility_supply_temperature,
        utility_return_temperature=utility_return_temperature,
        utility_approach_temperature=utility_approach_temperature,
        utility_heat_capacity_rate=utility_heat_capacity_rate,
        utility_overall_heat_transfer_coefficient=utility_overall_heat_transfer_coefficient,
        mechanical_design=mechanical_design,
        mechanical_type=mechanical_type,
        mechanical_selection=mechanical_selection,
    )


def pipe(
    components: list[str],
    inlet_n: Q,
    inlet_z: list[float],
    inlet_p: Q,
    inlet_t: Q,
    length: Q,
    diameter: Q,
    roughness: Q,
    fittings: list[dict[str, Q]] | None = None,
    outlet_pressure: Q | None = None,
    correlation: str | None = None,
    number_of_increments: int | None = None,
    elevation: Q | None = None,
) -> PipeResult:
    """Drop a stream's pressure along a line, solving the outlet pressure it implies.

    ``length``, ``diameter`` and ``roughness`` are ``unit_ops.pipe``'s three parameters, and
    the outlet pressure is *not* one of them: a line's drop is a function of its geometry and
    its fluid, which is what makes a pipe a different shape of unit operation from the
    two-port machines around it.

    **Two equations, one per phase**, chosen by the phase label: a gas takes the compressible
    ``P1^2 - P2^2`` form and anything else Darcy-Weisbach. ``fittings`` adds their ``sum(L/D) D``
    to the physical length; ``outlet_pressure`` replaces the solved drop with a solved *flow*,
    ``AdiabaticPipe.calcFlow``'s bisection; ``correlation = "beggs_and_brills"`` runs the ported
    Beggs-and-Brills march over the same geometry instead.

    Raises:
        InvalidInputError: where the geometry is not positive.

    See :func:`azoth.process.reference.pipe`.
    """
    return resolve(_PIPE)(  # type: ignore[no-any-return]
        components=components,
        inlet_n=inlet_n,
        inlet_z=inlet_z,
        inlet_p=inlet_p,
        inlet_t=inlet_t,
        length=length,
        diameter=diameter,
        roughness=roughness,
        fittings=fittings,
        outlet_pressure=outlet_pressure,
        correlation=correlation,
        number_of_increments=number_of_increments,
        elevation=elevation,
    )


def splitter(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    split_factors: list[float],
    design_pressure_drop: Q | None = None,
    max_design_velocity: Q | None = None,
    header_diameter: Q | None = None,
    capacity_analysis_enabled: bool | None = None,
    mechanical_design: bool | None = None,
    design_velocity: Q | None = None,
    max_operation_pressure: Q | None = None,
    tensile_strength: Q | None = None,
    joint_efficiency: float | None = None,
    corrosion_allowance: Q | None = None,
) -> SplitterResult:
    """Split a stream into several with the same state, in proportion to ``split_factors``.

    ``feed_n``, ``feed_z``, ``feed_p`` and ``feed_t`` are the inlet's record and
    ``split_factors`` is ``unit_ops.splitter``'s own parameter, whose length is how many
    outlets there are. A splitter changes no state: every outlet carries the inlet's
    composition, pressure, temperature and molar enthalpy, and the factors divide the
    flow between them. They are normalised, so only their ratios mean anything.

    ``design_pressure_drop``, ``max_design_velocity``, ``header_diameter`` and
    ``capacity_analysis_enabled`` are the capacity map's inputs. The first two are the
    class's setters, so supplying either initializes the map; the third is the mechanical
    design's header diameter, which the velocity constraint reads; the fourth initializes
    an otherwise empty map at the class defaults. The map is absent until one of them is
    used, exactly as ``Splitter``'s is.

    ``mechanical_design`` runs ``SplitterMechanicalDesign.calcDesign`` and publishes the
    sized header, branch, wall, manifold volume, pressure drop and weights; the five
    parameters after it are the design's own and are refused without the flag. The design
    sizes the header, so ``header_diameter`` is refused with it. With both the design and
    the capacity map asked for, the map's velocity reads the sized diameter.

    NeqSim's ``Splitter.run`` returns outlet enthalpies that do not conserve energy, so
    this diverges deliberately rather than reproducing them.

    Raises:
        InvalidInputError: where there is no outlet, the factors do not sum to a positive
            value, a design input is stated without ``mechanical_design``, a stated
            ``header_diameter`` meets the flag, or a design division has no answer.

    See :func:`azoth.process.reference.splitter`.
    """
    return resolve(_SPLITTER)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        split_factors=split_factors,
        design_pressure_drop=design_pressure_drop,
        max_design_velocity=max_design_velocity,
        header_diameter=header_diameter,
        capacity_analysis_enabled=capacity_analysis_enabled,
        mechanical_design=mechanical_design,
        design_velocity=design_velocity,
        max_operation_pressure=max_operation_pressure,
        tensile_strength=tensile_strength,
        joint_efficiency=joint_efficiency,
        corrosion_allowance=corrosion_allowance,
    )


def mixer(
    components: list[str],
    feed_n: list[Q],
    feed_z: list[list[float]],
    feed_p: list[Q],
    feed_t: list[Q],
    outlet_pressure: Q | None = None,
) -> MixerResult:
    """Join several streams into one, conserving molar flow and enthalpy.

    ``feed_n``, ``feed_z``, ``feed_p`` and ``feed_t`` are the feeds' records - one entry
    per feed, the fluid's ``components`` named once - and ``outlet_pressure`` is
    ``unit_ops.mixer``'s own parameter, optional because a mixer joins at the feeds'
    lowest pressure unless a header's pressure is stated.

    Mixing is isenthalpic, so the outlet's enthalpy is the feeds' weighted by their flows
    and its temperature is the one at which the joined mixture carries it at the outlet
    pressure. That is why the outlet is at neither feed's temperature in general.

    Raises:
        InvalidInputError: where the feeds' shapes disagree, no feed carries anything, or
            the outlet pressure is not positive.

    See :func:`azoth.process.reference.mixer`.
    """
    return resolve(_MIXER)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        outlet_pressure=outlet_pressure,
    )


def gas_scrubber(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    pressure_drop: Q,
    gas_in_liquid: float,
    internal_diameter: Q | None = None,
    design_gas_load_factor: Q | None = None,
    heat_input: Q | None = None,
    mechanical_design: bool | None = None,
    orientation: str | None = None,
    volume_safety_factor: float | None = None,
    design_liquid_level_fraction: float | None = None,
    max_operation_pressure: Q | None = None,
    allowable_stress: Q | None = None,
    joint_efficiency: float | None = None,
    corrosion_allowance: Q | None = None,
    safety_factor: float | None = None,
    mesh_pad_area: float | None = None,
    mesh_pad_thickness: Q | None = None,
    mesh_pad_elevation: Q | None = None,
) -> GasScrubberResult:
    """Flash a feed into vapour and liquid, as a scrubber does.

    ``GasScrubber extends Separator`` and does not override ``run``, so this is
    :func:`separator`'s arithmetic under the other entry, and its three parameters are the
    separator's. **The Souders-Brown capacity metric is the class's own addition**, computed
    when ``internal_diameter`` and ``design_gas_load_factor`` are both given: stated one at a
    time it is refused, because a vessel with a size and no design basis has nothing to check.
    ``mechanical_design`` runs ``GasScrubberMechanicalDesign.calcDesign`` and publishes the
    vessel; its basis is the scrubber standard's ``1.1`` volume factor and the default
    pressure-vessel values.

    Raises:
        InvalidInputError: where the shapes disagree, the pressure drop leaves a
            non-positive pressure, the entrainment fraction is outside ``[0, 1]``, only one
            of the two mechanical inputs is given, or the state makes the metric undefined.

    See :func:`azoth.process.reference.gas_scrubber`.
    """
    return resolve(_GAS_SCRUBBER)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        pressure_drop=pressure_drop,
        gas_in_liquid=gas_in_liquid,
        internal_diameter=internal_diameter,
        design_gas_load_factor=design_gas_load_factor,
        heat_input=heat_input,
        mechanical_design=mechanical_design,
        orientation=orientation,
        volume_safety_factor=volume_safety_factor,
        design_liquid_level_fraction=design_liquid_level_fraction,
        max_operation_pressure=max_operation_pressure,
        allowable_stress=allowable_stress,
        joint_efficiency=joint_efficiency,
        corrosion_allowance=corrosion_allowance,
        safety_factor=safety_factor,
        mesh_pad_area=mesh_pad_area,
        mesh_pad_thickness=mesh_pad_thickness,
        mesh_pad_elevation=mesh_pad_elevation,
    )


def flare(
    components: list[str],
    inlet_n: Q,
    inlet_z: list[float],
    inlet_p: Q,
    inlet_t: Q,
    flame_height: Q | None = None,
    radiant_fraction: float | None = None,
    tip_diameter: Q | None = None,
    ground_distance: Q | None = None,
    radiation_flux_threshold: Q | None = None,
    design_heat_duty_capacity: Q | None = None,
    design_mass_flow_capacity: Q | None = None,
    design_molar_flow_capacity: Q | None = None,
) -> FlareResult:
    """A flare's steady state: the record through, and the numbers beside it.

    ``Flare.run`` clones the inlet into the outlet and changes nothing in it, so the record is
    a pass-through. What the machine computes is its own report: ``heatDuty``, the gas's
    inferior calorific value per normal cubic metre at 0 C times its flow in standard cubic
    metres per second at 15 C, and ``co2Emission``, the carbon the gas carries times
    ``44.01e-3`` kg/mol.

    **The duty multiplies two reference states**: the calorific value is per *normal* cubic
    metre at 0 C and the flow is at 15 C, which the class's own arithmetic does and this
    reproduces - the case records the ratio.

    ``flame_height``, ``radiant_fraction`` and ``tip_diameter`` are the class's setters, and
    they clamp rather than refuse. A stated ``ground_distance`` reports the point-source
    radiation flux at it; a stated ``radiation_flux_threshold`` reports the horizontal distance
    the flux falls to. **The three design capacities are the class's NaN when unstated**: a
    utilisation exists only where its design is finite and positive, and ``overloaded`` only
    where one does.

    Raises:
        InvalidInputError: where a component has no row in ISO 6976's table or none in the
            element table.

    See :func:`azoth.process.reference.flare`.
    """
    return resolve(_FLARE)(  # type: ignore[no-any-return]
        components=components,
        inlet_n=inlet_n,
        inlet_z=inlet_z,
        inlet_p=inlet_p,
        inlet_t=inlet_t,
        flame_height=flame_height,
        radiant_fraction=radiant_fraction,
        tip_diameter=tip_diameter,
        ground_distance=ground_distance,
        radiation_flux_threshold=radiation_flux_threshold,
        design_heat_duty_capacity=design_heat_duty_capacity,
        design_mass_flow_capacity=design_mass_flow_capacity,
        design_molar_flow_capacity=design_molar_flow_capacity,
    )


def tank(
    components: list[str],
    feed_n: list[Q],
    feed_z: list[list[float]],
    feed_p: list[Q],
    feed_t: list[Q],
) -> TankResult:
    """Join a tank's inlets and split the result into a gas and a liquid outlet.

    **It declares no parameters, and the one it looked like it needed is why.**
    ``Tank.run`` flashes at the *fluid's* own volume and internal energy, so the steady
    state is the flash the feed already carries - measured, the captured two-phase row is
    :func:`separator`'s first row to the last digit. ``setVolume`` is read by the mechanical
    design, the capacity report and the JSON dump and by nothing in ``run``, so a declared
    ``volume`` would be a parameter no steady-state path reads.

    **A tank is not a three-phase machine**: ``run`` never enables ``multiPhaseCheck``, so a
    feed a three-phase separator splits three ways comes back in two, with the water in the
    liquid.

    Raises:
        InvalidInputError: where the feeds' shapes disagree or carry no flow in total.

    See :func:`azoth.process.reference.tank`.
    """
    return resolve(_TANK)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
    )


def ejector(
    motive_components: list[str],
    motive_n: Q,
    motive_z: list[float],
    motive_p: Q,
    motive_t: Q,
    suction_components: list[str],
    suction_n: Q,
    suction_z: list[float],
    suction_p: Q,
    suction_t: Q,
    discharge_pressure: Q,
    motive_nozzle_efficiency: float,
    suction_nozzle_efficiency: float,
    mixing_efficiency: float,
    diffuser_efficiency: float,
) -> EjectorResult:
    """Expand a motive stream, entrain a suction stream with it, and diffuse the mixture.

    **A gas ejector on NeqSim's quasi one-dimensional route**: each stream expands
    isentropically to the mixing pressure, drops to the enthalpy its own nozzle efficiency
    leaves, is given the velocity ``sqrt(2 dh)`` that implies, and the two momenta mix; a
    diffuser then recovers the mixing velocity's kinetic energy up to ``discharge_pressure``.

    **The mixing pressure is the class's own estimate**, not an input - ``setMixingPressure``
    exists and the palette does not declare it - and so are the two design velocities the
    outlet's final ``v^2/2`` is taken at.

    Raises:
        InvalidInputError: where an efficiency is outside ``(0, 1]`` or a stream's shapes
            disagree.

    See :func:`azoth.process.reference.ejector`.
    """
    return resolve(_EJECTOR)(  # type: ignore[no-any-return]
        motive_components=motive_components,
        motive_n=motive_n,
        motive_z=motive_z,
        motive_p=motive_p,
        motive_t=motive_t,
        suction_components=suction_components,
        suction_n=suction_n,
        suction_z=suction_z,
        suction_p=suction_p,
        suction_t=suction_t,
        discharge_pressure=discharge_pressure,
        motive_nozzle_efficiency=motive_nozzle_efficiency,
        suction_nozzle_efficiency=suction_nozzle_efficiency,
        mixing_efficiency=mixing_efficiency,
        diffuser_efficiency=diffuser_efficiency,
    )


def esd_response_time(
    sif_tag: str | None = None,
    contributions: list[dict[str, object]] | None = None,
    allowable_response_time: float | None = None,
) -> EsdResponseTimeResult:
    """The ESD response-time budget.

    The three stage sums, their total and the inclusive budget test. **An absent budget is
    within budget and has no margin**, and the contributions are echoed in caller order.

    See :func:`azoth.process.reference.esd_response_time`.
    """
    return resolve(_ESD_RESPONSE_TIME)(  # type: ignore[no-any-return]
        sif_tag=sif_tag,
        contributions=contributions,
        allowable_response_time=allowable_response_time,
    )


def event_tree(
    initiating_frequency_per_year: float,
    branches: list[dict[str, object]],
) -> EventTreeResult:
    """The full 2^n event tree in bit order.

    The leaf paths, probabilities and frequencies of the class's enumeration. **The branch
    count is capped at 20** and a probability outside ``[0, 1]`` is refused.

    See :func:`azoth.process.reference.event_tree`.
    """
    return resolve(_EVENT_TREE)(  # type: ignore[no-any-return]
        initiating_frequency_per_year=initiating_frequency_per_year,
        branches=branches,
    )


def fault_tree(
    nodes: list[dict[str, object]],
    max_cardinality: float,
) -> FaultTreeResult:
    """The recursive gate probability and the minimal cut sets.

    The flat node list is the tree: a row's position is its index, ``parent_index`` names
    its parent and ``-1`` is the single root. AND is the product, OR the exact complement
    union, VOTING the Poisson-binomial sum, and the beta factor is read on an OR gate only.
    **The cut sets are sorted by cardinality and then member name**, where the class's
    ``HashSet`` order is unspecified.

    See :func:`azoth.process.reference.fault_tree`.
    """
    return resolve(_FAULT_TREE)(  # type: ignore[no-any-return]
        nodes=nodes,
        max_cardinality=max_cardinality,
    )


def bow_tie(
    threats: list[dict[str, object]],
    consequences: list[dict[str, object]],
    barriers: list[dict[str, object]],
) -> BowTieResult:
    """The bow-tie risk: the threat sum, the prevention barriers and the RRF.

    The unmitigated frequency is the active threats' sum; the mitigated frequency
    multiplies the functional ``prevention`` and ``both`` barriers' PFDs in list order.
    **A ``mitigation`` barrier is read by no formula**, and neither is a threat-to-barrier
    link. ``total_rrf`` is zero at a zero mitigated frequency, the class's own answer.

    See :func:`azoth.process.reference.bow_tie`.
    """
    return resolve(_BOW_TIE)(  # type: ignore[no-any-return]
        threats=threats,
        consequences=consequences,
        barriers=barriers,
    )


def s001_clause10_review(
    items: list[dict[str, object]] | None = None,
    evidence: list[dict[str, object]] | None = None,
    references: list[dict[str, object]] | None = None,
) -> S001Clause10ReviewResult:
    """The NORSOK S-001 Clause 10 review.

    **Fifteen checks over normalized evidence, a coverage result and a verdict cascade.**
    The rows carry the requirement id, clause, status and severity; the messages and
    recommendation prose are presentation and are not part of the result.

    See :func:`azoth.process.reference.s001_clause10_review`.
    """
    return resolve(_S001_CLAUSE10_REVIEW)(  # type: ignore[no-any-return]
        items=items,
        evidence=evidence,
        references=references,
    )


def s001_secondary_pressure_protection(
    maximum_event_pressure: float | None = None,
    design_pressure: float | None = None,
    test_pressure: float | None = None,
    demand_frequency_per_year: float | None = None,
    target_frequency_per_year: float | None = None,
    relief_leakage_assessed: bool | None = None,
    relief_leakage_to_safe_location: bool | None = None,
    proof_test_interval_months: float | None = None,
) -> S001SecondaryPressureProtectionResult:
    """The NORSOK S-001 secondary-pressure screen.

    **The pressure band's default target, the test and event pressure checks, the
    demand-versus-target test and the leakage and proof-test criteria.** The two leakage
    flags are true-only; an absent proof-test interval is not a failure of its criterion.

    See :func:`azoth.process.reference.s001_secondary_pressure_protection`.
    """
    return resolve(_S001_SECONDARY_PRESSURE_PROTECTION)(  # type: ignore[no-any-return]
        maximum_event_pressure=maximum_event_pressure,
        design_pressure=design_pressure,
        test_pressure=test_pressure,
        demand_frequency_per_year=demand_frequency_per_year,
        target_frequency_per_year=target_frequency_per_year,
        relief_leakage_assessed=relief_leakage_assessed,
        relief_leakage_to_safe_location=relief_leakage_to_safe_location,
        proof_test_interval_months=proof_test_interval_months,
    )


def safety_instrumented_function(
    pfd_average: float,
    claimed_sil: float,
    unmitigated_frequency_per_year: float,
    target_frequency_per_year: float,
    test_interval_hours: float,
    lambda_du_per_hour: float | None = None,
    spurious_trip_rate_per_year: float | None = None,
    mttr_hours: float | None = None,
    base_availability: float | None = None,
) -> SafetyInstrumentedFunctionResult:
    """The SIF's RRF, required PFD and SIL, architecture PFDs and availability.

    ``risk_reduction_factor`` is absent at a PFD of zero, where the class answers
    ``+Infinity``. **A PFD outside the claimed SIL's band is published, not refused**:
    the class prints a warning, so the result carries ``pfd_within_claimed_sil`` and an
    ``OUT_OF_VALID_RANGE`` warning.

    See :func:`azoth.process.reference.safety_instrumented_function`.
    """
    return resolve(_SAFETY_INSTRUMENTED_FUNCTION)(  # type: ignore[no-any-return]
        pfd_average=pfd_average,
        claimed_sil=claimed_sil,
        unmitigated_frequency_per_year=unmitigated_frequency_per_year,
        target_frequency_per_year=target_frequency_per_year,
        test_interval_hours=test_interval_hours,
        lambda_du_per_hour=lambda_du_per_hour,
        spurious_trip_rate_per_year=spurious_trip_rate_per_year,
        mttr_hours=mttr_hours,
        base_availability=base_availability,
    )


def lopa(
    initiating_event_frequency_per_year: float,
    target_frequency_per_year: float,
    mitigated_frequency_per_year: float,
    layers: list[dict[str, object]],
    event_pressure: Q | None = None,
    design_pressure: Q | None = None,
    test_pressure: Q | None = None,
) -> LopaResult:
    """The LOPA result: the target status, the RRFs, the layers and the STS0131 band.

    ``gap_to_target`` is ``mitigated - target``, positive on a shortfall. **The layers
    are recorded, not multiplied**: the mitigated frequency is an input. The required
    additional RRF is absent when the target is zero and not met, where the class answers
    ``+Infinity``, and the required SIL then reads 4.

    See :func:`azoth.process.reference.lopa`.
    """
    return resolve(_LOPA)(  # type: ignore[no-any-return]
        initiating_event_frequency_per_year=initiating_event_frequency_per_year,
        target_frequency_per_year=target_frequency_per_year,
        mitigated_frequency_per_year=mitigated_frequency_per_year,
        layers=layers,
        event_pressure=event_pressure,
        design_pressure=design_pressure,
        test_pressure=test_pressure,
    )


def sil_verification(
    pfd_average: float,
    claimed_sil: float,
    architecture: str,
    test_interval_hours: float,
) -> SilVerificationResult:
    """The SIL verification: the achieved SIL, the architecture and the issues.

    The fault tolerance is the class's exact-match table and the diagnostic coverage its
    substring rule; any other architecture answers 0 and 0.5. **Warnings alone do not
    fail the verification**; an error does. A NaN or negative PFD, which the class lets
    through to achieved SIL 4, is refused.

    See :func:`azoth.process.reference.sil_verification`.
    """
    return resolve(_SIL_VERIFICATION)(  # type: ignore[no-any-return]
        pfd_average=pfd_average,
        claimed_sil=claimed_sil,
        architecture=architecture,
        test_interval_hours=test_interval_hours,
    )


def nog070_sil_determination(
    sif_type: str,
    pfd_average: float,
    minimum_sil: float | None = None,
) -> Nog070SilDeterminationResult:
    """A SIF against the NOG 070 catalogue: the catalogue minimum and compliance.

    ``pfdToSil`` reads ``[0.1, 1)`` as SIL 0, where ``process.safety_instrumented_function``'s
    required-SIL map reads 1: the two are different functions. **``custom`` carries a
    catalogue minimum of 0**, so an omitted project minimum is compliant by construction.
    The required PFD reduction factor is published only when the SIF is not compliant.

    See :func:`azoth.process.reference.nog070_sil_determination`.
    """
    return resolve(_NOG070_SIL_DETERMINATION)(  # type: ignore[no-any-return]
        sif_type=sif_type,
        pfd_average=pfd_average,
        minimum_sil=minimum_sil,
    )


def overpressure_protection_study(
    mawp: Q,
    scenarios: list[dict[str, object]],
    relief_set_pressure: Q | None = None,
    back_pressure: Q | None = None,
    multiple_devices: bool | None = None,
) -> OverpressureProtectionStudyResult:
    """The governing case, its sizing and the accumulated-pressure verdict.

    **The governing case is the first credible row with the largest rate**, the fire basis
    is 21 % and every other cause 10 %, and the sizing is the ported API 520 arithmetic.
    A liquid case with no density falls back to 700 kg/m3; a two-phase case missing an
    omega input is not sized and still runs acceptance.

    See :func:`azoth.process.reference.overpressure_protection_study`.
    """
    return resolve(_OVERPRESSURE_PROTECTION_STUDY)(  # type: ignore[no-any-return]
        mawp=mawp,
        scenarios=scenarios,
        relief_set_pressure=relief_set_pressure,
        back_pressure=back_pressure,
        multiple_devices=multiple_devices,
    )


def overpressure_acceptance(
    peak_pressure: Q,
    mawp: Q,
    cause: str,
    multiple_devices: bool,
) -> OverpressureAcceptanceResult:
    """The accumulation factor, the allowable pressure and the acceptance verdict.

    See :func:`azoth.process.reference.overpressure_acceptance`.
    """
    return resolve(_OVERPRESSURE_ACCEPTANCE)(  # type: ignore[no-any-return]
        peak_pressure=peak_pressure,
        mawp=mawp,
        cause=cause,
        multiple_devices=multiple_devices,
    )


def pipeline_pressure_protection(
    max_source_pressure: Q,
    design_pressure: Q,
    incidental_factor: float,
    barrier1_setpoint: Q,
    barrier2_setpoint: Q,
) -> PipelinePressureProtectionResult:
    """The maximum incidental pressure, the design margin and the three verdicts.

    See :func:`azoth.process.reference.pipeline_pressure_protection`.
    """
    return resolve(_PIPELINE_PRESSURE_PROTECTION)(  # type: ignore[no-any-return]
        max_source_pressure=max_source_pressure,
        design_pressure=design_pressure,
        incidental_factor=incidental_factor,
        barrier1_setpoint=barrier1_setpoint,
        barrier2_setpoint=barrier2_setpoint,
    )


def relief_disposal_network(
    contributions: list[dict[str, object]],
) -> ReliefDisposalNetworkResult:
    """The simultaneous total, the peak single load and the echoed rows.

    See :func:`azoth.process.reference.relief_disposal_network`.
    """
    return resolve(_RELIEF_DISPOSAL_NETWORK)(  # type: ignore[no-any-return]
        contributions=contributions,
    )


def tr3001_compliance(
    scenarios: list[dict[str, object]],
    capacity_adequate: bool,
    required_area_in2: float | None = None,
    acceptance_accepted: bool | None = None,
    accumulation_fraction: float | None = None,
) -> Tr3001ComplianceResult:
    """The six TR3001 checks over a study's scenario rows and sizing verdict.

    See :func:`azoth.process.reference.tr3001_compliance`.
    """
    return resolve(_TR3001_COMPLIANCE)(  # type: ignore[no-any-return]
        scenarios=scenarios,
        capacity_adequate=capacity_adequate,
        required_area_in2=required_area_in2,
        acceptance_accepted=acceptance_accepted,
        accumulation_fraction=accumulation_fraction,
    )


def trapped_inventory(
    components: list[str],
    z: list[float],
    pressure: float,
    temperature: float,
    segments: list[dict[str, object]],
    eos: str | None = None,
    multiphase_check: bool | None = None,
    fallback_liquid_density: float | None = None,
    segment_evidence: list[dict[str, object]] | None = None,
) -> TrappedInventoryResult:
    """The trapped gas and liquid inventory.

    The flash's gas and liquid densities over the documented equipment and pipe volumes,
    with the gas and liquid shares. **A segment with no evidence row warns**, and the
    class's 800 kg/m3 fallback is used when the flash finds no liquid.

    See :func:`azoth.process.reference.trapped_inventory`.
    """
    return resolve(_TRAPPED_INVENTORY)(  # type: ignore[no-any-return]
        components=components,
        z=z,
        pressure=pressure,
        temperature=temperature,
        segments=segments,
        eos=eos,
        multiphase_check=multiphase_check,
        fallback_liquid_density=fallback_liquid_density,
        segment_evidence=segment_evidence,
    )


def runaway_reaction(
    process_temperature: Q,
    reaction_enthalpy: Q,
    limiting_reactant_moles: Q,
    total_mass: Q,
    specific_heat: Q,
    max_allowable_temperature: Q | None = None,
    activation_energy: Q | None = None,
    initial_heat_release_rate: float | None = None,
    acceptable_margin: Q | None = None,
    gassy_system: bool | None = None,
) -> RunawayReactionResult:
    """The adiabatic rise, MTSR, margin, time to maximum rate and runaway verdict.

    The enthalpy sign is ignored. Without a MAT the verdict reads the adiabatic rise alone;
    with one the exceeded test is strict. The TMR needs a positive activation energy and a
    positive initial rate, the latter **in W/kg**.

    See :func:`azoth.process.reference.runaway_reaction`.
    """
    return resolve(_RUNAWAY_REACTION)(  # type: ignore[no-any-return]
        process_temperature=process_temperature,
        reaction_enthalpy=reaction_enthalpy,
        limiting_reactant_moles=limiting_reactant_moles,
        total_mass=total_mass,
        specific_heat=specific_heat,
        max_allowable_temperature=max_allowable_temperature,
        activation_energy=activation_energy,
        initial_heat_release_rate=initial_heat_release_rate,
        acceptable_margin=acceptable_margin,
        gassy_system=gassy_system,
    )


def risk_matrix(
    equipment: list[dict[str, object]],
    product_price_per_kg: float | None = None,
    downtime_cost_per_hour: float | None = None,
) -> RiskMatrixResult:
    """The risk matrix's bands, score, MTBF, downtime and costs.

    The production loss and the failure frequency are inputs: the class derives them from
    a process system and a reliability database, which this id does not carry. The price
    defaults to 0.5 per kg and the downtime cost to 5000 per hour; the MTBF is **1e6 hours
    at zero failures**, the class's sentinel.

    See :func:`azoth.process.reference.risk_matrix`.
    """
    return resolve(_RISK_MATRIX)(  # type: ignore[no-any-return]
        equipment=equipment,
        product_price_per_kg=product_price_per_kg,
        downtime_cost_per_hour=downtime_cost_per_hour,
    )


def fire_case_relief(
    has_drainage: bool,
    has_fire_fighting: bool,
    wetted_area: Q | None = None,
    vessel_diameter: Q | None = None,
    wetted_height: Q | None = None,
    latent_heat: float | None = None,
    relieving_temperature: Q | None = None,
) -> FireCaseReliefResult:
    """The API 521 fire-case heat input and the vapour relief rate.

    See :func:`azoth.process.reference.fire_case_relief`.
    """
    return resolve(_FIRE_CASE_RELIEF)(  # type: ignore[no-any-return]
        has_drainage=has_drainage,
        has_fire_fighting=has_fire_fighting,
        wetted_area=wetted_area,
        vessel_diameter=vessel_diameter,
        wetted_height=wetted_height,
        latent_heat=latent_heat,
        relieving_temperature=relieving_temperature,
    )


def fire_monitor_coverage(
    target_area: Q,
    required_density: float,
    monitor_count: float | None = None,
    monitor_flow: Q | None = None,
    effective_throw: Q | None = None,
    sweep_angle: float | None = None,
    wind_speed: Q | None = None,
    fall_height: Q | None = None,
    droplet_terminal_velocity: Q | None = None,
    characteristic_dimension: Q | None = None,
    line_of_sight_obstructed: bool | None = None,
) -> FireMonitorCoverageResult:
    """The fire-monitor coverage: density, wind drift and the class's verdict.

    The monitor-substitution screen: the nominal density ``count flow / area``, the drift
    ``fall wind / terminal``, the coverage fraction ``clamp(1 - drift / dimension, 0, 1)``
    and the effective density. **The verdict is the class's classification** in its priority
    order, obstructed sight first. **No finite flow suffices when the drift removes all
    coverage**, so `flow_needed_for_compliance` is absent there, where the class returns
    `+Infinity`.

    Raises:
        InvalidInputError: for a non-positive area, density, flow, throw, fall height or
            terminal velocity, a count below one or not whole, an angle outside
            ``(0, 360]``, or a negative wind speed or dimension.

    See :func:`azoth.process.reference.fire_monitor_coverage`.
    """
    return resolve(_FIRE_MONITOR_COVERAGE)(  # type: ignore[no-any-return]
        target_area=target_area,
        required_density=required_density,
        monitor_count=monitor_count,
        monitor_flow=monitor_flow,
        effective_throw=effective_throw,
        sweep_angle=sweep_angle,
        wind_speed=wind_speed,
        fall_height=fall_height,
        droplet_terminal_velocity=droplet_terminal_velocity,
        characteristic_dimension=characteristic_dimension,
        line_of_sight_obstructed=line_of_sight_obstructed,
    )


def fire_water_coverage_assessment(
    protected_area: Q,
    required_flow: Q,
    spare_supply_flow: Q,
    worst_case_pressure_margin: Q,
    covered_area: Q | None = None,
    existing_delivered_flow: Q | None = None,
    required_pressure_margin: Q | None = None,
) -> FireWaterCoverageAssessmentResult:
    """The fire-water coverage, flow and pressure gate and the class's verdict.

    The class asks coverage, then flow, then pressure, because the routes to a fix differ in
    cost. The verdict distinguishes a coverage gap with supply in hand from a flow deficit
    and from a pressure deficit, and answers **dedicated supply** when neither margin exists.
    **An unset supply is not a verdict**: the class's NaN defaults read as feasible, and this
    port requires the spare flow and the pressure margin.

    Raises:
        InvalidInputError: for a non-positive area or required flow, a negative covered
            area, delivered flow or required margin, or a non-finite input.

    See :func:`azoth.process.reference.fire_water_coverage_assessment`.
    """
    return resolve(_FIRE_WATER_COVERAGE_ASSESSMENT)(  # type: ignore[no-any-return]
        protected_area=protected_area,
        required_flow=required_flow,
        spare_supply_flow=spare_supply_flow,
        worst_case_pressure_margin=worst_case_pressure_margin,
        covered_area=covered_area,
        existing_delivered_flow=existing_delivered_flow,
        required_pressure_margin=required_pressure_margin,
    )


def fire_water_demand(
    protected_area: Q,
    application_rate: float | None = None,
    duration: Q | None = None,
    foam_concentrate_percent: float | None = None,
    simultaneous_area_factor: float | None = None,
    protected_objects: list[dict[str, object]] | None = None,
) -> FireWaterDemandResult:
    """The fire-water demand: area and object rates, simultaneous factor and volumes.

    The NORSOK S-001 / NFPA 15 application-rate screen: ``area = protected_area rate``, the
    object demand summed over the wetted surfaces, the simultaneous factor applied to the
    total, and the water and foam volumes over the duration. **A zero area demand has no
    ratio**, the class's NaN.

    Raises:
        InvalidInputError: for a negative area or rate, a non-positive duration, a foam
            percent outside ``[0, 10]``, a factor below one, or a negative row area or rate.

    See :func:`azoth.process.reference.fire_water_demand`.
    """
    return resolve(_FIRE_WATER_DEMAND)(  # type: ignore[no-any-return]
        protected_area=protected_area,
        application_rate=application_rate,
        duration=duration,
        foam_concentrate_percent=foam_concentrate_percent,
        simultaneous_area_factor=simultaneous_area_factor,
        protected_objects=protected_objects,
    )


def deluge_nozzle_layout(
    protected_area: Q,
    required_density: float,
    nozzle_k: float,
    nozzle_min_pressure: Q,
    nozzle_max_spacing: Q,
    nozzle_clearance: Q,
    available_clearance: Q,
    operating_pressure: Q | None = None,
    coverage_efficiency: float | None = None,
) -> DelugeNozzleLayoutResult:
    """The deluge nozzle net: the flow and coverage criteria and the larger count.

    The NFPA 15 screen: ``flow_per_nozzle = k sqrt(p)``, ``coverage_per_nozzle =
    spacing^2 efficiency``, both counts ceilinged and the larger governing, with **a tie set
    by the flow criterion**. The grid spacing and delivered density follow the count.

    Raises:
        InvalidInputError: for a non-positive area, density, coefficient, minimum pressure,
            spacing or operating pressure, an efficiency outside ``(0, 1]``, or a negative
            clearance.

    See :func:`azoth.process.reference.deluge_nozzle_layout`.
    """
    return resolve(_DELUGE_NOZZLE_LAYOUT)(  # type: ignore[no-any-return]
        protected_area=protected_area,
        required_density=required_density,
        nozzle_k=nozzle_k,
        nozzle_min_pressure=nozzle_min_pressure,
        nozzle_max_spacing=nozzle_max_spacing,
        nozzle_clearance=nozzle_clearance,
        available_clearance=available_clearance,
        operating_pressure=operating_pressure,
        coverage_efficiency=coverage_efficiency,
    )


def three_phase_separator(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    pressure_drop: Q,
    gas_in_aqueous: float = 0.0,
    gas_in_oil: float = 0.0,
    oil_in_aqueous: float = 0.0,
    oil_in_gas: float = 0.0,
    aqueous_in_gas: float = 0.0,
    aqueous_in_oil: float = 0.0,
    heat_input: Q | None = None,
    internal_diameter: Q | None = None,
    separator_length: Q | None = None,
    nll_fraction: float | None = None,
    nil_fraction: float | None = None,
    water_level: Q | None = None,
    oil_level: Q | None = None,
    gas_outlet_flow_fraction: float | None = None,
    oil_outlet_flow_fraction: float | None = None,
    water_outlet_flow_fraction: float | None = None,
) -> ThreePhaseSeparatorResult:
    """Flash a stream into vapour, oil and aqueous outlets.

    **The three-phase vessel turns `multiPhaseCheck` on for its flash**, so a feed that
    separates into a gas, an oil and an aqueous phase is split three ways - a two-phase feed
    into whichever two it has. The vessel holds the feed's temperature, so a pressure drop is
    not a throttling; ``heat_input`` is the one thing that moves the flash.

    **The six entrainment fractions are declared, and their order is part of the answer**:
    each moves a share of the *current* from-phase's moles, component by component, in
    ``run``'s own order, so a later pair reads what an earlier one left.

    ``internal_diameter`` and ``separator_length`` are both or neither and add the two
    retention times: the class's mechanical-design NLL/NIL (``nll_fraction``, ``nil_fraction``,
    defaults ``0.5`` and ``0.2``) turn the vessel into the oil and water volumes, over each
    phase's own flow. ``water_level``/``oil_level`` are the class's stored, zero-clamped level
    state and do not move the retention times.

    Raises:
        InvalidInputError: where the pressure drop takes the outlet below zero, a fraction
            is outside ``[0, 1]``, or half a vessel is stated.

    See :func:`azoth.process.reference.three_phase_separator`.
    """
    return resolve(_THREE_PHASE_SEPARATOR)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        pressure_drop=pressure_drop,
        gas_in_aqueous=gas_in_aqueous,
        gas_in_oil=gas_in_oil,
        oil_in_aqueous=oil_in_aqueous,
        oil_in_gas=oil_in_gas,
        aqueous_in_gas=aqueous_in_gas,
        aqueous_in_oil=aqueous_in_oil,
        heat_input=heat_input,
        internal_diameter=internal_diameter,
        separator_length=separator_length,
        nll_fraction=nll_fraction,
        nil_fraction=nil_fraction,
        water_level=water_level,
        oil_level=oil_level,
        gas_outlet_flow_fraction=gas_outlet_flow_fraction,
        oil_outlet_flow_fraction=oil_outlet_flow_fraction,
        water_outlet_flow_fraction=water_outlet_flow_fraction,
    )


def hem_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
) -> HemReleaseResult:
    """The release rate of a homogeneous-equilibrium short-opening orifice.

    The upstream stagnation state is expanded isentropically to a 49-pressure grid, and the
    maximum of ``rho sqrt(2 (h0 - h))`` over that grid is the throat. The answer is **four
    stations** - the stagnation state, the throat, the aliased exit and the receiving-pressure
    state - with the rate, the choked flag and, when the flow is choked, the throat's
    equilibrium sound speed.

    A failure is the class's own result and not an exception: ``status`` is ``invalid`` or
    ``unsupported``, the rate and stations are absent, and one diagnostic says why. **No NaN
    crosses the boundary.**

    Raises:
        InvalidInputError: for a non-positive state or opening, a discharge coefficient
            outside ``(0, 1]``, or a composition that is not one entry per component and sums
            to one.

    See :func:`azoth.process.reference.hem_release`.
    """
    return resolve(_HEM_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
    )


def ideal_gas_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
) -> IdealGasReleaseResult:
    """The release rate of an ideal-gas isentropic short-opening orifice.

    One gas phase at a constant heat-capacity ratio. The upstream density is ``P M/(R T)``,
    the stagnation enthalpy and entropy are the EOS flash's, and the throat is the critical
    ratio or the receiving-pressure state. The answer is **four stations** with the rate,
    the choked flag and the throat's ideal-gas sound speed.

    A failure is the class's own result and not an exception: ``status`` is ``invalid`` or
    ``unsupported``, the rate and stations are absent, and one diagnostic says why. **No
    NaN crosses the boundary.**

    Raises:
        InvalidInputError: for a non-positive state or opening, a coefficient outside
            ``(0, 1]``, or a composition that is not one entry per component and sums to one.

    See :func:`azoth.process.reference.ideal_gas_release`.
    """
    return resolve(_IDEAL_GAS_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
    )


def legacy_screening_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
) -> LegacyScreeningReleaseResult:
    """The historical screening release rate through a circular opening.

    The compatibility equation: a choked branch from ``P``, ``gamma``, ``T`` and ``MW``, and
    a subcritical branch from ``2 rho (P - P_back)``. The stations alias the flashed upstream
    state because no throat is solved, and the result is always ``valid_with_warnings`` on a
    usable state.

    Raises:
        InvalidInputError: for a non-positive state or opening, a coefficient outside
            ``(0, 1]``, or a composition that is not one entry per component and sums to one.

    See :func:`azoth.process.reference.legacy_screening_release`.
    """
    return resolve(_LEGACY_SCREENING_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
    )


def prescribed_slip_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    gas_to_liquid_slip_ratio: float,
) -> PrescribedSlipReleaseResult:
    """The release rate of a prescribed-slip short-opening orifice.

    The homogeneous-equilibrium scan supplies the stations; the opening's phase velocities
    are partitioned by the caller's gas-to-liquid ratio, and the rate is the partition's
    mass flux times the effective area. A single-phase opening is ``unsupported``.

    Raises:
        InvalidInputError: for a non-positive state or opening, a slip ratio below one, a
            coefficient outside ``(0, 1]``, or a composition that is not one entry per
            component and sums to one.

    See :func:`azoth.process.reference.prescribed_slip_release`.
    """
    return resolve(_PRESCRIBED_SLIP_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        gas_to_liquid_slip_ratio=gas_to_liquid_slip_ratio,
    )


def drift_flux_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    surface_tension: Q,
) -> DriftFluxReleaseResult:
    """The release rate of a vertical drift-flux short-opening orifice.

    The homogeneous-equilibrium scan supplies the stations; the shared Zuber-Findlay/Harmathy
    closure solves the opening's slip ratio and the rate is the closure's mass flux times the
    effective area. A single-phase opening or a gas area fraction above 0.80 is
    ``unsupported``.

    See :func:`azoth.process.reference.drift_flux_release`.
    """
    return resolve(_DRIFT_FLUX_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        surface_tension=surface_tension,
    )


def finite_rate_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    surface_tension: Q,
    phase_transfer_relaxation_time: Q,
    residence_time: Q,
    parameter_provenance: str,
) -> FiniteRateReleaseResult:
    """The release rate of a finite-rate short-opening orifice.

    The opening's phase split relaxes from the upstream value toward equilibrium by
    ``1 - exp(-residence/relaxation)``, then the shared drift-flux closure solves the station.
    An upstream without a liquid is ``unsupported``.

    See :func:`azoth.process.reference.finite_rate_release`.
    """
    return resolve(_FINITE_RATE_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        surface_tension=surface_tension,
        phase_transfer_relaxation_time=phase_transfer_relaxation_time,
        residence_time=residence_time,
        parameter_provenance=parameter_provenance,
    )


def component_selective_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    surface_tension: Q,
    component_relaxation_times: list[Q],
    residence_time: Q,
    parameter_provenance: str,
) -> ComponentSelectiveReleaseResult:
    """The release rate of a component-selective short-opening orifice.

    Each component gas-held mass fraction relaxes by ``1 - exp(-residence/tau_i)``, then the
    shared drift-flux closure solves the station. A relaxation-time list that does not match
    the component set is ``unsupported``.

    See :func:`azoth.process.reference.component_selective_release`.
    """
    return resolve(_COMPONENT_SELECTIVE_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        surface_tension=surface_tension,
        component_relaxation_times=component_relaxation_times,
        residence_time=residence_time,
        parameter_provenance=parameter_provenance,
    )


def ranz_marshall_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    surface_tension: Q,
    residence_time: Q,
    morphology: str,
    sauter_mean_diameter: Q,
    continuous_phase_density: Q,
    continuous_phase_dynamic_viscosity: Q,
    relative_velocity: Q,
    component_diffusivities: list[Q],
    parameter_provenance: str,
) -> RanzMarshallReleaseResult:
    """The release rate of a Ranz-Marshall short-opening orifice.

    The external-film correlation supplies one relaxation time per component and the
    component-selective model solves the station. Out-of-range Reynolds or Schmidt numbers
    and the droplet morphology are ``unsupported``; a diffusivity list that does not match
    the fluid is ``invalid``.

    See :func:`azoth.process.reference.ranz_marshall_release`.
    """
    return resolve(_RANZ_MARSHALL_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        surface_tension=surface_tension,
        residence_time=residence_time,
        morphology=morphology,
        sauter_mean_diameter=sauter_mean_diameter,
        continuous_phase_density=continuous_phase_density,
        continuous_phase_dynamic_viscosity=continuous_phase_dynamic_viscosity,
        relative_velocity=relative_velocity,
        component_diffusivities=component_diffusivities,
        parameter_provenance=parameter_provenance,
    )


def bubble_transfer_zone_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    surface_tension: Q,
    transfer_zone_hydraulic_diameter: Q,
    transfer_zone_vertical_length: Q,
    continuous_phase_dynamic_viscosity: Q,
    component_diffusivities: list[Q],
    parameter_provenance: str,
) -> BubbleTransferZoneReleaseResult:
    """The release rate of a predictive bubble transfer-zone orifice.

    The drift-flux base supplies the phase velocities; a fixed point predicts the bubble
    diameter (the minimum of the Eotvos, Weber, quarter-hydraulic and opening limits), the
    relative velocity and the residence time, and the Ranz-Marshall adapter solves each
    iterate.

    See :func:`azoth.process.reference.bubble_transfer_zone_release`.
    """
    return resolve(_BUBBLE_TRANSFER_ZONE_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        surface_tension=surface_tension,
        transfer_zone_hydraulic_diameter=transfer_zone_hydraulic_diameter,
        transfer_zone_vertical_length=transfer_zone_vertical_length,
        continuous_phase_dynamic_viscosity=continuous_phase_dynamic_viscosity,
        component_diffusivities=component_diffusivities,
        parameter_provenance=parameter_provenance,
    )


def ideal_gas_fanno_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    flow_path_length: Q,
    darcy_friction_factor: float,
) -> IdealGasFannoReleaseResult:
    """The release rate of an ideal-gas Fanno pipe.

    A quasi-steady adiabatic constant-area pipe with a specified Darcy friction factor. The
    exit Mach is one when choked, otherwise the back-pressure bisection; the Fanno function
    and the class's ``1/M`` stagnation-pressure ratio are the class's own.

    See :func:`azoth.process.reference.ideal_gas_fanno_release`.
    """
    return resolve(_IDEAL_GAS_FANNO_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        flow_path_length=flow_path_length,
        darcy_friction_factor=darcy_friction_factor,
    )


def real_gas_fanno_release(
    components: list[str],
    upstream_z: list[float],
    upstream_t: Q,
    upstream_p: Q,
    diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    flow_path_length: Q,
    darcy_friction_factor: float,
) -> RealGasFannoReleaseResult:
    """The release rate of a real-gas Fanno pipe.

    A one-sided adiabatic constant-area pipe of a single equilibrium gas with EOS
    properties: ten axial predictor-corrector steps, a PH flash at every energy state, the
    entropy inlet root and nested flux bisections. A two-phase state is ``unsupported``.
    **The pure-Python mirror is slow**; its cross-implementation cases run in the slow sweep.

    See :func:`azoth.process.reference.real_gas_fanno_release`.
    """
    return resolve(_REAL_GAS_FANNO_RELEASE)(  # type: ignore[no-any-return]
        components=components,
        upstream_z=upstream_z,
        upstream_t=upstream_t,
        upstream_p=upstream_p,
        diameter=diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        flow_path_length=flow_path_length,
        darcy_friction_factor=darcy_friction_factor,
    )


def jet_fire(
    mass_flow: Q,
    heat_of_combustion: float,
    radiative_fraction: float,
    relative_humidity: float | None = None,
    distance: Q | None = None,
    target_flux: Q | None = None,
) -> JetFireResult:
    """The point-source jet-fire radiation at one distance, or the distance to a target flux.

    ``Q = eta m_dot dHc`` and the incident flux ``Q tau / (4 pi d^2)`` over Wayne's
    transmissivity. **A non-positive distance, target or humidity is refused**: the class
    answers ``Infinity`` at zero distance, throws on a zero target, and answers ``NaN`` at
    zero humidity.

    See :func:`azoth.process.reference.jet_fire`.
    """
    return resolve(_JET_FIRE)(  # type: ignore[no-any-return]
        mass_flow=mass_flow,
        heat_of_combustion=heat_of_combustion,
        radiative_fraction=radiative_fraction,
        relative_humidity=relative_humidity,
        distance=distance,
        target_flux=target_flux,
    )


def leak_blowdown(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    duration: Q,
    vessel_volume: Q | None = None,
    hole_diameter: Q | None = None,
    discharge_coefficient: float | None = None,
    back_pressure: Q | None = None,
    time_step: Q | None = None,
    scenario_name: str | None = None,
) -> LeakBlowdownResult:
    """Blow a vessel down through a circular opening.

    The legacy ``LeakModel.calculateSourceTerm`` path: the inventory is the flashed
    corrected density times ``vessel_volume``; each step records the simplified orifice
    rate, the capped jet velocity, the unclamped momentum and the droplet SMD, then depletes
    the mass and updates the pressure and temperature isentropically. The modern
    ``calculateReleaseFlow`` path is :func:`hem_release`.

    **The class's silent fallbacks are refused** - an ideal-gas density, ``gamma = 1.3``,
    methane's molar mass, ``rho = 1``, a NaN rate as zero - so a state needing one is an
    ``invalid`` result with the value in its diagnostic. Only visited rows are published;
    ``number_of_points`` and ``recorded_points`` carry the class's zero-filled tail.

    Raises:
        InvalidInputError: for a non-positive state, volume, opening, duration or step, a
            discharge coefficient outside ``(0, 1]``, a composition that is not one entry per
            component and sums to one, or a declared point count over the port's cap.

    See :func:`azoth.process.reference.leak_blowdown`.
    """
    return resolve(_LEAK_BLOWDOWN)(  # type: ignore[no-any-return]
        components=components,
        initial_z=initial_z,
        initial_t=initial_t,
        initial_p=initial_p,
        duration=duration,
        vessel_volume=vessel_volume,
        hole_diameter=hole_diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        time_step=time_step,
        scenario_name=scenario_name,
    )


def gaussian_plume(
    emission_rate: Q,
    effective_stack_height: Q,
    wind_speed: Q,
    stability: str,
    terrain: str,
    distance: Q,
    crosswind: Q,
    height: Q,
    threshold: Q,
) -> GaussianPlumeResult:
    """The concentration of a continuous Gaussian plume at one point.

    The Briggs sigma curves for the six Pasquill classes over rural or urban terrain, the
    two-image concentration, and the class's log scan and bisection for the ground
    centreline distance to ``threshold``.

    **The distance is absent when the scan never reaches the threshold**, and ``100000.0`` m
    when it never falls below it, which is the scan's end rather than a physical distance.

    Raises:
        InvalidInputError: for a negative emission rate, a non-positive wind speed or
            threshold, a negative distance, or an unknown stability or terrain word.

    See :func:`azoth.process.reference.gaussian_plume`.
    """
    return resolve(_GAUSSIAN_PLUME)(  # type: ignore[no-any-return]
        emission_rate=emission_rate,
        effective_stack_height=effective_stack_height,
        wind_speed=wind_speed,
        stability=stability,
        terrain=terrain,
        distance=distance,
        crosswind=crosswind,
        height=height,
        threshold=threshold,
    )


def heavy_gas_dispersion(
    volumetric_rate: Q,
    source_density: Q,
    air_density: Q,
    wind_speed: Q,
    concentration_ratio: float | None = None,
) -> HeavyGasDispersionResult:
    """The Britter-McQuaid dense-gas distance to a concentration ratio.

    The signed reduced gravity, the unfloored characteristic parameter and the fitted
    downwind distance; ``reduced_gravity`` is in m/s2, and the distance is absent when no
    ratio is stated.

    Raises:
        InvalidInputError: for a non-positive rate, density or wind speed, a ratio
            outside ``(0, 1)``, or a non-finite value.

    See :func:`azoth.process.reference.heavy_gas_dispersion`.
    """
    return resolve(_HEAVY_GAS_DISPERSION)(  # type: ignore[no-any-return]
        volumetric_rate=volumetric_rate,
        source_density=source_density,
        air_density=air_density,
        wind_speed=wind_speed,
        concentration_ratio=concentration_ratio,
    )


def hazardous_area(
    mass_flow: Q,
    exit_density: Q,
    exit_velocity: Q,
    lfl_volume_fraction: float,
    gas_molar_mass: Q,
    ambient_density: Q | None = None,
    safety_factor: float | None = None,
    release_grade: str | None = None,
) -> HazardousAreaResult:
    """The hazardous distance and IEC 60079-10-1 zone for a continuous gas jet.

    The notional source diameter, the class's ideal mole-to-mass LFL, the de-rated
    downwind distance and the zone the grade of release selects.

    Raises:
        InvalidInputError: for a non-positive flow, density, velocity, molar mass or
            ambient density, an LFL outside ``(0, 1)``, a safety factor outside
            ``(0, 1]``, or an unknown grade.

    See :func:`azoth.process.reference.hazardous_area`.
    """
    return resolve(_HAZARDOUS_AREA)(  # type: ignore[no-any-return]
        mass_flow=mass_flow,
        exit_density=exit_density,
        exit_velocity=exit_velocity,
        lfl_volume_fraction=lfl_volume_fraction,
        gas_molar_mass=gas_molar_mass,
        ambient_density=ambient_density,
        safety_factor=safety_factor,
        release_grade=release_grade,
    )


def api14c_safe_chart(
    equipment: list[dict[str, object]] | None = None,
    present_devices: list[dict[str, object]] | None = None,
    assume_complete: bool | None = None,
) -> Api14cSafeChartResult:
    """The API 14C SAFE chart: the required devices against the declared ones.

    **The class-name classifier, the eight-category table and the missing set.** An
    unclassified row is omitted, an undeclared equipment has an empty present set, and the
    assuming-complete mode ignores declarations.

    See :func:`azoth.process.reference.api14c_safe_chart`.
    """
    return resolve(_API14C_SAFE_CHART)(  # type: ignore[no-any-return]
        equipment=equipment,
        present_devices=present_devices,
        assume_complete=assume_complete,
    )


def barrier_register_validation(
    register_id: str,
    elements: list[dict[str, object]] | None = None,
    barriers: list[dict[str, object]] | None = None,
    standards: list[dict[str, object]] | None = None,
    links: list[dict[str, object]] | None = None,
    evidence: list[dict[str, object]] | None = None,
) -> BarrierRegisterValidationResult:
    """The barrier register's consistency findings.

    **The register, element, barrier and performance-standard rules**, with the class's
    iteration quirks: a linked barrier is validated once per element, an unlinked barrier
    gets only the linkage finding, and a shared standard is validated once per barrier.

    See :func:`azoth.process.reference.barrier_register_validation`.
    """
    return resolve(_BARRIER_REGISTER_VALIDATION)(  # type: ignore[no-any-return]
        register_id=register_id,
        elements=elements,
        barriers=barriers,
        standards=standards,
        links=links,
        evidence=evidence,
    )


def hazop_template(
    grid_parameters: list[dict[str, object]],
    equipment_type: str | None = None,
) -> HazopTemplateResult:
    """The deviation grid a HAZOP node starts from.

    **Seven guide words per parameter**, in the class's order, then the equipment-type seed
    set when a type is stated. Every generated cell is ``TBD`` and the recommendation is
    absent; an empty parameter list is the class's no-argument grid.

    See :func:`azoth.process.reference.hazop_template`.
    """
    return resolve(_HAZOP_TEMPLATE)(  # type: ignore[no-any-return]
        grid_parameters=grid_parameters,
        equipment_type=equipment_type,
    )


def fmea_worksheet(
    entries: list[dict[str, object]],
    rpn_threshold: float,
) -> FmeaWorksheetResult:
    """The RPN ranking and the entries at or above a threshold.

    **`RPN = S O D`**, ranked descending and stable for a tie. The threshold filter is
    inclusive and keeps caller order; S, O and D are whole numbers in ``[1, 10]``.

    See :func:`azoth.process.reference.fmea_worksheet`.
    """
    return resolve(_FMEA_WORKSHEET)(  # type: ignore[no-any-return]
        entries=entries,
        rpn_threshold=rpn_threshold,
    )


def hazop_consequence_population(
    deviations: list[dict[str, object]],
    equipment_type: str | None = None,
) -> HazopConsequencePopulationResult:
    """The consequence, safeguard and calculator citation that fill a node's TBD cells.

    **Eleven generic mappings and six equipment-specific flow mappings.** A flow deviation
    with no equipment entry gets the neutral fallback, which requires facilitator
    confirmation; only null, blank or ``TBD`` cells are filled.

    See :func:`azoth.process.reference.hazop_consequence_population`.
    """
    return resolve(_HAZOP_CONSEQUENCE_POPULATION)(  # type: ignore[no-any-return]
        deviations=deviations,
        equipment_type=equipment_type,
    )


def mah_bow_tie_skeleton(mah_type: str) -> MahBowTieSkeletonResult:
    """The ISO 17776 threats, consequences and barrier classes for one MAH type.

    **The placeholders are `0.001` a year, a severity of `4` and a PFD of `0.01`.** The
    skeleton is the input to :func:`bow_tie`, which computes the frequency; this id does not.

    See :func:`azoth.process.reference.mah_bow_tie_skeleton`.
    """
    return resolve(_MAH_BOW_TIE_SKELETON)(  # type: ignore[no-any-return]
        mah_type=mah_type,
    )


def toxic_library(
    chemical: str | None = None,
    concentration_ppm: float | None = None,
    molar_mass: Q | None = None,
    temperature: Q | None = None,
    pressure: Q | None = None,
) -> ToxicLibraryResult:
    """The toxic-exposure thresholds for a chemical, and the ppm-to-kg/m3 conversion.

    The class's static table of IDLH, ERPG-2 and AEGL-2 values, its exact-then-lowercase
    lookup, and ``(ppm/1e6) M (P_bara 1e5) / (8.314 T)``. **An unknown or blank chemical is
    a stated false**, not a refusal; the measured case-sensitivity asymmetry is reproduced.

    Raises:
        InvalidInputError: when neither a chemical nor the conversion group is stated,
            when the group is incomplete, or when its values are out of range.

    See :func:`azoth.process.reference.toxic_library`.
    """
    return resolve(_TOXIC_LIBRARY)(  # type: ignore[no-any-return]
        chemical=chemical,
        concentration_ppm=concentration_ppm,
        molar_mass=molar_mass,
        temperature=temperature,
        pressure=pressure,
    )


def gas_blowby(
    upstream_pressure: Q,
    upstream_temperature: Q,
    downstream_pressure: Q,
    restriction_area: Q,
    specific_heat_ratio: float,
    molar_mass: Q,
    discharge_coefficient: float | None = None,
    downstream_relief_capacity: Q | None = None,
) -> GasBlowbyResult:
    """The blowby rate through an open restriction and the relief verdict.

    The isentropic nozzle forms: the flow is choked at or below the critical ratio and the
    slower subcritical form applies above it. The standard volume is at **288.15 K and
    101325 Pa - 15 C, not 0 C**. A stated capacity at least the blowby rate is adequate; a
    missing capacity publishes no relief data rather than a zero capacity.

    See :func:`azoth.process.reference.gas_blowby`.
    """
    return resolve(_GAS_BLOWBY)(  # type: ignore[no-any-return]
        upstream_pressure=upstream_pressure,
        upstream_temperature=upstream_temperature,
        downstream_pressure=downstream_pressure,
        restriction_area=restriction_area,
        specific_heat_ratio=specific_heat_ratio,
        molar_mass=molar_mass,
        discharge_coefficient=discharge_coefficient,
        downstream_relief_capacity=downstream_relief_capacity,
    )


def gas_dispersion(
    components: list[str],
    composition: list[float],
    mass_release_rate: Q,
    ambient_temperature: Q | None = None,
    atmospheric_pressure: Q | None = None,
    wind_speed: Q | None = None,
    pasquill_stability_class: str | None = None,
    surface_roughness: Q | None = None,
    offshore: bool | None = None,
    release_height: Q | None = None,
    model_selection: str | None = None,
    lower_flammable_limit: float | None = None,
    flammable_endpoint_fraction_of_lfl: float | None = None,
    toxic_component: str | None = None,
    toxic_threshold: float | None = None,
) -> GasDispersionResult:
    """Screen a continuous gas release for its flammable and toxic endpoints.

    The ambient TP flash of the released composition, the gas phase's Le Chatelier LFL and
    fuel data, and the passive Gaussian plume or dense-gas Britter-McQuaid arm selected by
    the source-to-air density ratio (or stated). **A distance the arm does not reach is
    absent**, never a NaN.

    ``toxic_threshold`` is a **volume fraction**, so 100 ppm is ``1.0e-4``.

    Raises:
        InvalidInputError: for a non-positive release rate, wind speed or stated manual LFL,
            an unknown stability or model word, an endpoint fraction outside ``(0, 1]``, a
            composition that is not one entry per component and sums to one, or whatever the
            databank or the ambient flash refuses.

    See :func:`azoth.process.reference.gas_dispersion`.
    """
    return resolve(_GAS_DISPERSION)(  # type: ignore[no-any-return]
        components=components,
        composition=composition,
        mass_release_rate=mass_release_rate,
        ambient_temperature=ambient_temperature,
        atmospheric_pressure=atmospheric_pressure,
        wind_speed=wind_speed,
        pasquill_stability_class=pasquill_stability_class,
        surface_roughness=surface_roughness,
        offshore=offshore,
        release_height=release_height,
        model_selection=model_selection,
        lower_flammable_limit=lower_flammable_limit,
        flammable_endpoint_fraction_of_lfl=flammable_endpoint_fraction_of_lfl,
        toxic_component=toxic_component,
        toxic_threshold=toxic_threshold,
    )


def probit(
    a: float,
    b: float,
    n: float,
    dose: Q | None = None,
    probit_value: Q | None = None,
) -> ProbitResult:
    """The probit of a hazard dose, and the probability a probit value implies.

    ``Y = a + b n ln(dose)`` and ``P = 0.5 (1 + erf((Y - 5) / sqrt(2)))`` over the class's
    Abramowitz-Stegun ``erf``. **A non-positive dose is refused**: the class answers it with
    ``-Infinity``, which the port does not publish.

    Raises:
        InvalidInputError: when neither a dose nor a probit value is stated, or when the
            stated dose is not positive.

    See :func:`azoth.process.reference.probit`.
    """
    return resolve(_PROBIT)(  # type: ignore[no-any-return]
        a=a,
        b=b,
        n=n,
        dose=dose,
        probit_value=probit_value,
    )


def psv_fire_sizing(
    initial_mass: Q,
    initial_pressure: Q,
    set_pressure: Q,
    initial_temperature: Q,
    fire_heat_input: Q,
    vessel_volume: Q,
    molar_mass: Q,
    specific_heat_ratio: float,
    compressibility: float,
    heat_capacity: Q,
    blowdown_time: Q,
    blowdown_percent: float | None = None,
    orifice_area: Q | None = None,
    cv_discharge_coefficient: float | None = None,
) -> PsvFireSizingResult:
    """The API 521 dynamic fire-case area, blowdown pressure, Cv and next orifice.

    See :func:`azoth.process.reference.psv_fire_sizing`.
    """
    return resolve(_PSV_FIRE_SIZING)(  # type: ignore[no-any-return]
        initial_mass=initial_mass,
        initial_pressure=initial_pressure,
        set_pressure=set_pressure,
        initial_temperature=initial_temperature,
        fire_heat_input=fire_heat_input,
        vessel_volume=vessel_volume,
        molar_mass=molar_mass,
        specific_heat_ratio=specific_heat_ratio,
        compressibility=compressibility,
        heat_capacity=heat_capacity,
        blowdown_time=blowdown_time,
        blowdown_percent=blowdown_percent,
        orifice_area=orifice_area,
        cv_discharge_coefficient=cv_discharge_coefficient,
    )


def psv_sizing(
    service: str,
    set_pressure: Q,
    overpressure_fraction: float,
    back_pressure: Q,
    is_balanced_bellows: bool | None = None,
    has_rupture_disk: bool | None = None,
    mass_flow: Q | None = None,
    temperature: Q | None = None,
    molar_mass: Q | None = None,
    compressibility: float | None = None,
    specific_heat_ratio: float | None = None,
    volume_flow: Q | None = None,
    liquid_density: Q | None = None,
    viscosity: Q | None = None,
    gas_fraction: float | None = None,
    gas_density: Q | None = None,
    latent_heat: float | None = None,
    liquid_heat_capacity: Q | None = None,
    orifice_area: Q | None = None,
    capacity_discharge_coefficient: float | None = None,
) -> PsvSizingResult:
    """The API 520 relief-orifice area, the D to T ladder and the capacity helper.

    See :func:`azoth.process.reference.psv_sizing`.
    """
    return resolve(_PSV_SIZING)(  # type: ignore[no-any-return]
        service=service,
        set_pressure=set_pressure,
        overpressure_fraction=overpressure_fraction,
        back_pressure=back_pressure,
        is_balanced_bellows=is_balanced_bellows,
        has_rupture_disk=has_rupture_disk,
        mass_flow=mass_flow,
        temperature=temperature,
        molar_mass=molar_mass,
        compressibility=compressibility,
        specific_heat_ratio=specific_heat_ratio,
        volume_flow=volume_flow,
        liquid_density=liquid_density,
        viscosity=viscosity,
        gas_fraction=gas_fraction,
        gas_density=gas_density,
        latent_heat=latent_heat,
        liquid_heat_capacity=liquid_heat_capacity,
        orifice_area=orifice_area,
        capacity_discharge_coefficient=capacity_discharge_coefficient,
    )


def psv_valve(
    set_pressure: Q,
    blowdown_fraction: float,
    discharge_coefficient: float,
    orifice_area: Q,
    back_pressure: Q,
    upstream_pressure: Q,
    upstream_temperature: Q,
    molar_mass: Q,
    heat_capacity_ratio: float,
    compressibility: float,
    was_open: bool | None = None,
    prior_cycle_count: float | None = None,
) -> PsvValveResult:
    """One step of the PSV latch and its relieving flow.

    The valve opens at the set pressure, counts the pop once, and closes only at the
    reseat pressure ``set (1 - blowdown)``; the flow is the API 520 compressible-orifice
    relation and is zero when closed.

    See :func:`azoth.process.reference.psv_valve`.
    """
    return resolve(_PSV_VALVE)(  # type: ignore[no-any-return]
        set_pressure=set_pressure,
        blowdown_fraction=blowdown_fraction,
        discharge_coefficient=discharge_coefficient,
        orifice_area=orifice_area,
        back_pressure=back_pressure,
        upstream_pressure=upstream_pressure,
        upstream_temperature=upstream_temperature,
        molar_mass=molar_mass,
        heat_capacity_ratio=heat_capacity_ratio,
        compressibility=compressibility,
        was_open=was_open,
        prior_cycle_count=prior_cycle_count,
    )


def dynamic_psv_sizing(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    fire_heat_input: Q,
    set_pressure: Q,
    overpressure_fraction: float,
    back_pressure: Q,
    blowdown_fraction: float | None = None,
    discharge_coefficient: float | None = None,
    latent_heat: float | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    initial_pressure_fraction: float | None = None,
) -> DynamicPsvSizingResult:
    """The steady API 521 area and the dynamically required area for a cycling PSV.

    The dynamic march is an ideal-gas lumped vessel seeded from one TP flash; its start
    pressure is ``set_pressure x initial_pressure_fraction``, so the fluid's stated
    pressure enters only through the flash and the compressibility.

    See :func:`azoth.process.reference.dynamic_psv_sizing`.
    """
    return resolve(_DYNAMIC_PSV_SIZING)(  # type: ignore[no-any-return]
        components=components,
        initial_z=initial_z,
        initial_t=initial_t,
        initial_p=initial_p,
        vessel_volume=vessel_volume,
        fire_heat_input=fire_heat_input,
        set_pressure=set_pressure,
        overpressure_fraction=overpressure_fraction,
        back_pressure=back_pressure,
        blowdown_fraction=blowdown_fraction,
        discharge_coefficient=discharge_coefficient,
        latent_heat=latent_heat,
        time_step=time_step,
        max_time=max_time,
        initial_pressure_fraction=initial_pressure_fraction,
    )


def vessel_filling(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    inlet_temperature: Q,
    supply_pressure: Q,
    inlet_mass_flow: Q,
    target_pressure: Q | None = None,
    wall_mass: Q | None = None,
    wall_area: Q | None = None,
    wall_specific_heat: Q | None = None,
    wall_heat_transfer_coefficient: Q | None = None,
    woodfield_nozzle_diameter: Q | None = None,
    vessel_height: Q | None = None,
    liner_min_temperature: Q | None = None,
    liner_max_temperature: Q | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
) -> VesselFillingResult:
    """The gas-charging transient of a vessel.

    The inlet specific enthalpy is flashed once; each step adds the charge, exchanges heat
    with the lumped wall and VU-flashes at the vessel volume and the new internal energy.
    The Woodfield coefficient is used when both the nozzle diameter and the height are given.

    See :func:`azoth.process.reference.vessel_filling`.
    """
    return resolve(_VESSEL_FILLING)(  # type: ignore[no-any-return]
        components=components,
        initial_z=initial_z,
        initial_t=initial_t,
        initial_p=initial_p,
        vessel_volume=vessel_volume,
        inlet_temperature=inlet_temperature,
        supply_pressure=supply_pressure,
        inlet_mass_flow=inlet_mass_flow,
        target_pressure=target_pressure,
        wall_mass=wall_mass,
        wall_area=wall_area,
        wall_specific_heat=wall_specific_heat,
        wall_heat_transfer_coefficient=wall_heat_transfer_coefficient,
        woodfield_nozzle_diameter=woodfield_nozzle_diameter,
        vessel_height=vessel_height,
        liner_min_temperature=liner_min_temperature,
        liner_max_temperature=liner_max_temperature,
        time_step=time_step,
        max_time=max_time,
    )


def depressurization(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    orifice_diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    fire_heat_input: Q | None = None,
    fire_heat_input_to_wall: bool | None = None,
    fire_exposed_area: Q | None = None,
    fire_preset: str | None = None,
    fire_emissivity: float | None = None,
    fire_flame_temperature: Q | None = None,
    fire_convective_coefficient: Q | None = None,
    wall_mass: Q | None = None,
    wall_area: Q | None = None,
    wall_specific_heat: Q | None = None,
    wall_heat_transfer_coefficient: Q | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    stop_pressure: Q | None = None,
    withdrawal_mode: str | None = None,
) -> DepressurizationResult:
    """The equilibrium vessel blowdown and its API 521 summary.

    AUTO and VAPOUR take the lightest phase of a multiphase inventory when its density is at
    most 300 kg/m3 and remove at most a quarter of it per step; the fire boundary is the
    constant duty or a temperature-dependent flux over the exposed area.

    See :func:`azoth.process.reference.depressurization`.
    """
    return resolve(_DEPRESSURIZATION)(  # type: ignore[no-any-return]
        components=components,
        initial_z=initial_z,
        initial_t=initial_t,
        initial_p=initial_p,
        vessel_volume=vessel_volume,
        orifice_diameter=orifice_diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        fire_heat_input=fire_heat_input,
        fire_heat_input_to_wall=fire_heat_input_to_wall,
        fire_exposed_area=fire_exposed_area,
        fire_preset=fire_preset,
        fire_emissivity=fire_emissivity,
        fire_flame_temperature=fire_flame_temperature,
        fire_convective_coefficient=fire_convective_coefficient,
        wall_mass=wall_mass,
        wall_area=wall_area,
        wall_specific_heat=wall_specific_heat,
        wall_heat_transfer_coefficient=wall_heat_transfer_coefficient,
        time_step=time_step,
        max_time=max_time,
        stop_pressure=stop_pressure,
        withdrawal_mode=withdrawal_mode,
    )


def depressurization_acceptance(
    points: list[dict[str, object]],
    time_to_escape: Q | None = None,
    estimated_time_to_rupture: Q | None = None,
    maximum_pressure_at_rupture: Q | None = None,
    maximum_remaining_mass: Q | None = None,
    maximum_escalated_fire_rate: Q | None = None,
) -> DepressurizationAcceptanceResult:
    """The STS-0131 acceptance of a stored blowdown trajectory.

    The limiting time is the last sample's time reduced by the positive escape and rupture
    times; pressure and mass are linearly interpolated and the peak rate is the maximum at
    or before that time.

    See :func:`azoth.process.reference.depressurization_acceptance`.
    """
    return resolve(_DEPRESSURIZATION_ACCEPTANCE)(  # type: ignore[no-any-return]
        points=points,
        time_to_escape=time_to_escape,
        estimated_time_to_rupture=estimated_time_to_rupture,
        maximum_pressure_at_rupture=maximum_pressure_at_rupture,
        maximum_remaining_mass=maximum_remaining_mass,
        maximum_escalated_fire_rate=maximum_escalated_fire_rate,
    )


def blocked_outlet_overpressure(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    inlet_temperature: Q,
    supply_pressure: Q,
    inlet_mass_flow: Q,
    relief_set_pressure: Q,
    time_step: Q | None = None,
    max_time: Q | None = None,
) -> BlockedOutletOverpressureResult:
    """The blocked-outlet pressure build-up to the relief set pressure.

    A vessel filling run whose target is the relief set pressure; the time to demand is the
    first recorded sample at or above it.

    See :func:`azoth.process.reference.blocked_outlet_overpressure`.
    """
    return resolve(_BLOCKED_OUTLET_OVERPRESSURE)(  # type: ignore[no-any-return]
        components=components,
        initial_z=initial_z,
        initial_t=initial_t,
        initial_p=initial_p,
        vessel_volume=vessel_volume,
        inlet_temperature=inlet_temperature,
        supply_pressure=supply_pressure,
        inlet_mass_flow=inlet_mass_flow,
        relief_set_pressure=relief_set_pressure,
        time_step=time_step,
        max_time=max_time,
    )


def multi_vessel_blowdown_study(
    samples: list[dict[str, object]],
    header_diameter: Q | None = None,
    header_pressure: Q | None = None,
    header_temperature: Q | None = None,
    header_molar_mass: Q | None = None,
    header_heat_capacity_ratio: float | None = None,
    max_allowable_mach: float | None = None,
    grid_step: Q | None = None,
) -> MultiVesselBlowdownStudyResult:
    """The simultaneous blowdown of sources into a shared header.

    Each source's profile is linearly interpolated onto a common grid; the peak total and
    its per-source contributions are published. The header Mach check runs only when the
    header diameter is given.

    See :func:`azoth.process.reference.multi_vessel_blowdown_study`.
    """
    return resolve(_MULTI_VESSEL_BLOWDOWN_STUDY)(  # type: ignore[no-any-return]
        samples=samples,
        header_diameter=header_diameter,
        header_pressure=header_pressure,
        header_temperature=header_temperature,
        header_molar_mass=header_molar_mass,
        header_heat_capacity_ratio=header_heat_capacity_ratio,
        max_allowable_mach=max_allowable_mach,
        grid_step=grid_step,
    )


def non_equilibrium_blowdown(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    orifice_diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    fire_emissivity: float | None = None,
    fire_flame_temperature: Q | None = None,
    fire_convective_coefficient: Q | None = None,
    fire_exposed_area: Q | None = None,
    wall_mass: Q | None = None,
    wall_specific_heat: Q | None = None,
    inside_gas_coefficient: Q | None = None,
    interfacial_coefficient: Q | None = None,
    interfacial_area: Q | None = None,
    latent_heat: float | None = None,
    surface_tension: float | None = None,
    surface_fluid_constant: float | None = None,
    critical_heat_flux: float | None = None,
    effective_boiling_coefficient: Q | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    stop_pressure: Q | None = None,
) -> NonEquilibriumBlowdownResult:
    """The two-zone fire-exposed blowdown.

    The vapour and liquid zones are flashed at their own temperatures and the shared
    pressure; the wetted wall drives Rohsenow boiling and the boil-off moves liquid into
    the vapour zone.

    See :func:`azoth.process.reference.non_equilibrium_blowdown`.
    """
    return resolve(_NON_EQUILIBRIUM_BLOWDOWN)(  # type: ignore[no-any-return]
        components=components,
        initial_z=initial_z,
        initial_t=initial_t,
        initial_p=initial_p,
        vessel_volume=vessel_volume,
        orifice_diameter=orifice_diameter,
        discharge_coefficient=discharge_coefficient,
        back_pressure=back_pressure,
        fire_emissivity=fire_emissivity,
        fire_flame_temperature=fire_flame_temperature,
        fire_convective_coefficient=fire_convective_coefficient,
        fire_exposed_area=fire_exposed_area,
        wall_mass=wall_mass,
        wall_specific_heat=wall_specific_heat,
        inside_gas_coefficient=inside_gas_coefficient,
        interfacial_coefficient=interfacial_coefficient,
        interfacial_area=interfacial_area,
        latent_heat=latent_heat,
        surface_tension=surface_tension,
        surface_fluid_constant=surface_fluid_constant,
        critical_heat_flux=critical_heat_flux,
        effective_boiling_coefficient=effective_boiling_coefficient,
        time_step=time_step,
        max_time=max_time,
        stop_pressure=stop_pressure,
    )


def pfp_demand(
    incident_heat_flux: Q,
    steel_thickness: Q,
    required_survival_time: Q,
    absorptivity: float | None = None,
    initial_temperature: Q | None = None,
    critical_temperature: Q | None = None,
    fire_temperature: Q | None = None,
    pfp_conductivity: Q | None = None,
    fire_type: str | None = None,
) -> PfpDemandResult:
    """The bare-steel time to critical, the strict PFP verdict, the thickness and the rating.

    The API 521 / NORSOK S-001 demand chain: ``t_bare = m'' c (T_crit - T_0) / (alpha q)``,
    ``pfp_required = t_bare < t_req``, a conduction-limited thickness and an H or J rating.
    The driving temperature is floored at 1 K, the class's own. **A critical temperature at
    or below the initial one is refused**: the class publishes a zero or negative time with
    an infinite or negative thickness there.

    Raises:
        InvalidInputError: for a non-positive flux, thickness or survival time, an
            absorptivity outside ``(0, 1]``, a non-positive conductivity or temperature, a
            critical temperature at or below the initial one, or an unknown fire type.

    See :func:`azoth.process.reference.pfp_demand`.
    """
    return resolve(_PFP_DEMAND)(  # type: ignore[no-any-return]
        incident_heat_flux=incident_heat_flux,
        steel_thickness=steel_thickness,
        required_survival_time=required_survival_time,
        absorptivity=absorptivity,
        initial_temperature=initial_temperature,
        critical_temperature=critical_temperature,
        fire_temperature=fire_temperature,
        pfp_conductivity=pfp_conductivity,
        fire_type=fire_type,
    )


def manifold(
    components: list[str],
    feed_n: list[Q],
    feed_z: list[list[float]],
    feed_p: list[Q],
    feed_t: list[Q],
    split_factors: list[float],
) -> ManifoldResult:
    """Join a manifold's feeds and divide the mixture between its outlets.

    ``Manifold.run`` is a mixer and a splitter composed - ``localmixer.run()``, then
    ``localsplitter.run()`` over the mixture - so every rule :func:`mixer` and
    :func:`splitter` state holds here. The one rule that is the manifold's own is the
    low-flow filter: a feed whose mass flow is at or below ``1e-20`` kg/hr is not mixed, and
    the palette declares no way to change that.

    The outlet count is ``split_factors``' length, which is why ``unit_ops.manifold``'s
    ``outlet`` is a ``many`` one: an entry with a single outlet could not describe the
    machine.

    Raises:
        InvalidInputError: where the feeds' shapes disagree, a factor is negative, or no
            feed is above the low-flow threshold.

    See :func:`azoth.process.reference.manifold`.
    """
    return resolve(_MANIFOLD)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        split_factors=split_factors,
    )


def semenov_self_heating(
    volume: Q,
    surface_area: Q,
    heat_transfer_coefficient: Q,
    activation_energy: Q,
    volumetric_pre_factor: float,
    ambient_temperature: Q,
    marginal_ratio: float | None = None,
) -> SemenovSelfHeatingResult:
    """The Semenov criticality screen for a thermally thin body.

    **`psi` is compared with `1/e`**: below the marginal band the body is subcritical, inside
    it marginal, and above it the body is predicted to self-ignite. The critical ambient
    temperature is a bisection and is absent when it cannot be bracketed; the critical rise
    is `R Ta^2/E`.

    See :func:`azoth.process.reference.semenov_self_heating`.
    """
    return resolve(_SEMENOV_SELF_HEATING)(  # type: ignore[no-any-return]
        volume=volume,
        surface_area=surface_area,
        heat_transfer_coefficient=heat_transfer_coefficient,
        activation_energy=activation_energy,
        volumetric_pre_factor=volumetric_pre_factor,
        ambient_temperature=ambient_temperature,
        marginal_ratio=marginal_ratio,
    )


def separator(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    pressure_drop: Q,
    gas_in_liquid: float,
    heat_input: Q | None = None,
    internal_diameter: Q | None = None,
    design_gas_load_factor: Q | None = None,
    orientation: str | None = None,
    design_liquid_level_fraction: float | None = None,
    separator_length: Q | None = None,
    liquid_level_fraction: float | None = None,
    safety_factor: float | None = None,
    mechanical_design: bool | None = None,
    auto_size: bool | None = None,
    fg: float | None = None,
    volume_safety_factor: float | None = None,
    retention_time: Q | None = None,
    max_operation_pressure: Q | None = None,
    allowable_stress: Q | None = None,
    joint_efficiency: float | None = None,
    corrosion_allowance: Q | None = None,
) -> SeparatorResult:
    """Flash a stream into vapour and liquid outlets.

    ``feed_n``, ``feed_z``, ``feed_p`` and ``feed_t`` are the inlet's record and
    ``pressure_drop``, ``heat_input`` and ``gas_in_liquid`` are ``unit_ops.separator``'s
    own parameters. The flash is at the **feed's** temperature - the vessel holds it - so a
    pressure drop is not a throttling and the outlets do not carry the inlet's enthalpy; a
    ``heat_input`` is the one thing that moves it.

    ``internal_diameter`` and ``design_gas_load_factor`` are the vessel's size and design
    basis, **both or neither**, and add the five capacity getters: utilisation, the maximum
    allowable and superficial velocities, the actual Souders-Brown load factor, and the
    overload flag. ``orientation`` and ``design_liquid_level_fraction`` shape the gas area and
    default to the class's ``horizontal`` and ``0.8``. ``separator_length`` and
    ``liquid_level_fraction`` are the class's ``5.0`` m and ``0.5`` when absent, and add the
    operating gas area fraction and the phase-indexed load factors; ``safety_factor`` adds
    ``sizeFromFlow``'s required diameter. ``mechanical_design`` runs
    ``SeparatorMechanicalDesign.calcDesign`` and publishes the vessel; ``fg``,
    ``volume_safety_factor``, ``retention_time``, ``max_operation_pressure``,
    ``allowable_stress``, ``joint_efficiency`` and ``corrosion_allowance`` are its design basis,
    at the class's default standards' values when absent.

    Raises:
        InvalidInputError: where the pressure drop takes the outlet below zero, the
            entrainment fraction is outside ``[0, 1]``, or the vessel is half stated.

    See :func:`azoth.process.reference.separator`.
    """
    return resolve(_SEPARATOR)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        pressure_drop=pressure_drop,
        gas_in_liquid=gas_in_liquid,
        heat_input=heat_input,
        internal_diameter=internal_diameter,
        design_gas_load_factor=design_gas_load_factor,
        orientation=orientation,
        design_liquid_level_fraction=design_liquid_level_fraction,
        separator_length=separator_length,
        liquid_level_fraction=liquid_level_fraction,
        safety_factor=safety_factor,
        mechanical_design=mechanical_design,
        auto_size=auto_size,
        fg=fg,
        volume_safety_factor=volume_safety_factor,
        retention_time=retention_time,
        max_operation_pressure=max_operation_pressure,
        allowable_stress=allowable_stress,
        joint_efficiency=joint_efficiency,
        corrosion_allowance=corrosion_allowance,
    )


def separator_capacity_assessment(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    pressure_drop: Q,
    gas_in_liquid: float,
    internal_diameter: Q,
    design_gas_load_factor: Q,
    orientation: str,
    design_liquid_level_fraction: float,
    design_basis_provenance: str,
    heat_input: Q | None = None,
    mechanical_design_max_gas_flow: Q | None = None,
) -> SeparatorCapacityAssessmentResult:
    """Assess a separator's gas capacity, fail-closed.

    The pinned ``SeparatorCapacityAssessment.from``: a detached snapshot of the vessel, the
    flashed state, the two limits and the utilisation. **A blank provenance, an unknown
    orientation, non-positive geometry or K, no gas phase, a level outside ``[0, 1)`` and
    non-finite evidence answer ``"unassessed"``** - statuses, not exceptions - and the numeric
    evidence is absent there. ``mechanical_design_max_gas_flow`` is the class's
    ``maxDesignGassVolumeFlow / 3600``; a positive limit at or below the Souders-Brown flow
    governs.

    **The run comes first**, as it does in the class: a feed that cannot flash is an error,
    and the statuses are about the vessel's evidence rather than the feed. The checks then run
    in the class's order, so a state failing two carries the earlier sentence.

    Raises:
        InvalidInputError: where the feed cannot flash or a stated mechanical limit is not
            finite.
        OutOfRangeError: for a non-finite quantity at the units boundary.

    See :func:`azoth.process.reference.separator_capacity_assessment`.
    """
    return resolve(_SEPARATOR_CAPACITY_ASSESSMENT)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        pressure_drop=pressure_drop,
        gas_in_liquid=gas_in_liquid,
        internal_diameter=internal_diameter,
        design_gas_load_factor=design_gas_load_factor,
        orientation=orientation,
        design_liquid_level_fraction=design_liquid_level_fraction,
        design_basis_provenance=design_basis_provenance,
        heat_input=heat_input,
        mechanical_design_max_gas_flow=mechanical_design_max_gas_flow,
    )


def settle_out_pressure(
    compartments: list[dict[str, object]],
    settle_out_temperature: Q | None = None,
    protected_pressure_rating: Q | None = None,
) -> SettleOutPressureResult:
    """The settle-out pressure of connected gas volumes and the rating verdict.

    Each compartment's inventory is ``p V/(R T)`` with ``Z = 1``, the class's no-gas basis;
    the settle temperature is the mole-weighted mean unless stated. **At least two
    compartments are required.**

    See :func:`azoth.process.reference.settle_out_pressure`.
    """
    return resolve(_SETTLE_OUT_PRESSURE)(  # type: ignore[no-any-return]
        compartments=compartments,
        settle_out_temperature=settle_out_temperature,
        protected_pressure_rating=protected_pressure_rating,
    )


def separator_performance(
    gas_density: Q,
    oil_density: Q,
    water_density: Q,
    gas_viscosity: Q,
    oil_viscosity: Q,
    water_viscosity: Q,
    gas_velocity: Q,
    vessel_diameter: Q,
    vessel_length: Q,
    orientation: str,
    liquid_level_fraction: float,
    mist_position: float | None = None,
    weir_position: float | None = None,
    gas_liquid_distribution: str | None = None,
    gas_liquid_characteristic_diameter: Q | None = None,
    gas_liquid_spread: float | None = None,
    gas_bubble_distribution: str | None = None,
    gas_bubble_characteristic_diameter: Q | None = None,
    gas_bubble_spread: float | None = None,
    oil_in_water_distribution: str | None = None,
    oil_in_water_characteristic_diameter: Q | None = None,
    oil_in_water_spread: float | None = None,
    water_in_oil_distribution: str | None = None,
    water_in_oil_characteristic_diameter: Q | None = None,
    water_in_oil_spread: float | None = None,
    mist_eliminator: str | None = None,
    mist_eliminator_cut_diameter: Q | None = None,
    mist_eliminator_sharpness: float | None = None,
    mist_eliminator_max_efficiency: float | None = None,
    mist_eliminator_points: list[dict[str, Q]] | None = None,
    oil_water_coalescer: str | None = None,
    oil_water_coalescer_cut_diameter: Q | None = None,
    oil_water_coalescer_max_efficiency: float | None = None,
    oil_water_coalescer_points: list[dict[str, Q]] | None = None,
    include_gravity_section: bool | None = None,
    liquid_in_gas_calibration_factor: float | None = None,
    gas_carry_under_calibration_factor: float | None = None,
    liquid_liquid_calibration_factor: float | None = None,
    enhanced: bool | None = None,
    inlet_pipe_diameter: Q | None = None,
    surface_tension: Q | None = None,
    oil_water_interfacial_tension: Q | None = None,
    oil_volume_fraction: float | None = None,
    apply_turbulence_correction: bool | None = None,
    inlet_device: str | None = None,
    override_bulk_efficiency: float | None = None,
    override_dsd_multiplier: float | None = None,
    design_k_factor: Q | None = None,
    liquid_liquid_residence_time: Q | None = None,
) -> SeparatorPerformanceResult:
    """Compute a separator's droplet-based entrainment and efficiencies.

    ``SeparatorPerformanceCalculator``'s standard path: the gas-liquid gravity stage, a mist
    eliminator when one is named, the gas bubble carry-under, and the two liquid-liquid stages,
    each integrated over a droplet size distribution and a grade efficiency curve. **The
    fractions are the separator ids' own entrainment inputs**, so the calculator composes with
    a vessel rather than being a flag on it.

    **The three-phase gas entrainment is the whole ``1 - overall`` on both liquid phases**, and
    the gas carry-under is one fraction written to both; the liquid-liquid stage uses the
    class's fixed 300 s. Those are the class's answers and are reproduced.

    Args:
        gas_density: the gas phase's mass density; zero is absent.
        oil_density: the oil phase's mass density; zero is absent.
        water_density: the water phase's mass density; zero is absent.
        gas_viscosity: the gas phase's dynamic viscosity.
        oil_viscosity: the oil phase's dynamic viscosity.
        water_viscosity: the water phase's dynamic viscosity.
        gas_velocity: the superficial gas velocity in the gas section.
        vessel_diameter: the vessel's internal diameter.
        vessel_length: the vessel's tangent-to-tangent length.
        orientation: ``horizontal`` or ``vertical``.
        liquid_level_fraction: the liquid's share of the cross-section or height.
        mist_position: the mist eliminator's position as a fraction of the length; absent is
            ``0.90``.
        weir_position: the weir's position as a fraction of the length; absent is ``0.60``.
        gas_liquid_distribution: the entrained liquid's distribution, ``rosin_rammler`` or
            ``log_normal``; absent leaves the gas-liquid branch unrun.
        gas_liquid_characteristic_diameter: its ``d_63.2`` or ``d_50``.
        gas_liquid_spread: its spread.
        gas_bubble_distribution: the entrained gas's distribution.
        gas_bubble_characteristic_diameter: its characteristic diameter.
        gas_bubble_spread: its spread.
        oil_in_water_distribution: the dispersed oil's distribution in water.
        oil_in_water_characteristic_diameter: its characteristic diameter.
        oil_in_water_spread: its spread.
        water_in_oil_distribution: the dispersed water's distribution in oil.
        water_in_oil_characteristic_diameter: its characteristic diameter.
        water_in_oil_spread: its spread.
        mist_eliminator: ``wire_mesh``, ``vane_pack``, ``axial_cyclone``, ``plate_pack`` or
            ``custom``; absent is no eliminator.
        mist_eliminator_cut_diameter: its ``d_50``; absent takes the type's default.
        mist_eliminator_sharpness: its sigmoid sharpness; absent takes the type's default.
        mist_eliminator_max_efficiency: its ``eta_max``; absent takes the type's default.
        mist_eliminator_points: a custom curve's lookup table.
        oil_water_coalescer: ``plate_pack`` or ``custom``; absent is no coalescer.
        oil_water_coalescer_cut_diameter: the plate pack's ``d_50``.
        oil_water_coalescer_max_efficiency: the plate pack's maximum efficiency.
        oil_water_coalescer_points: a custom coalescer's lookup table.
        include_gravity_section: whether the gravity stage is included; absent is true.
        liquid_in_gas_calibration_factor: the multiplier on the two gas entrainments.
        gas_carry_under_calibration_factor: the multiplier on the two carry-unders.
        liquid_liquid_calibration_factor: the multiplier on the two liquid-liquid fractions.

    Raises:
        InvalidInputError: for a value outside its declared range, an undeclared name, a
            distribution whose fields are not stated together, a curve missing what its type
            requires, or a lookup table on a type that does not read one.

    See :func:`azoth.process.reference.separator_performance`.
    """
    return resolve(_SEPARATOR_PERFORMANCE)(  # type: ignore[no-any-return]
        gas_density=gas_density,
        oil_density=oil_density,
        water_density=water_density,
        gas_viscosity=gas_viscosity,
        oil_viscosity=oil_viscosity,
        water_viscosity=water_viscosity,
        gas_velocity=gas_velocity,
        vessel_diameter=vessel_diameter,
        vessel_length=vessel_length,
        orientation=orientation,
        liquid_level_fraction=liquid_level_fraction,
        mist_position=mist_position,
        weir_position=weir_position,
        gas_liquid_distribution=gas_liquid_distribution,
        gas_liquid_characteristic_diameter=gas_liquid_characteristic_diameter,
        gas_liquid_spread=gas_liquid_spread,
        gas_bubble_distribution=gas_bubble_distribution,
        gas_bubble_characteristic_diameter=gas_bubble_characteristic_diameter,
        gas_bubble_spread=gas_bubble_spread,
        oil_in_water_distribution=oil_in_water_distribution,
        oil_in_water_characteristic_diameter=oil_in_water_characteristic_diameter,
        oil_in_water_spread=oil_in_water_spread,
        water_in_oil_distribution=water_in_oil_distribution,
        water_in_oil_characteristic_diameter=water_in_oil_characteristic_diameter,
        water_in_oil_spread=water_in_oil_spread,
        mist_eliminator=mist_eliminator,
        mist_eliminator_cut_diameter=mist_eliminator_cut_diameter,
        mist_eliminator_sharpness=mist_eliminator_sharpness,
        mist_eliminator_max_efficiency=mist_eliminator_max_efficiency,
        mist_eliminator_points=mist_eliminator_points,
        oil_water_coalescer=oil_water_coalescer,
        oil_water_coalescer_cut_diameter=oil_water_coalescer_cut_diameter,
        oil_water_coalescer_max_efficiency=oil_water_coalescer_max_efficiency,
        oil_water_coalescer_points=oil_water_coalescer_points,
        include_gravity_section=include_gravity_section,
        liquid_in_gas_calibration_factor=liquid_in_gas_calibration_factor,
        gas_carry_under_calibration_factor=gas_carry_under_calibration_factor,
        liquid_liquid_calibration_factor=liquid_liquid_calibration_factor,
        enhanced=enhanced,
        inlet_pipe_diameter=inlet_pipe_diameter,
        surface_tension=surface_tension,
        oil_water_interfacial_tension=oil_water_interfacial_tension,
        oil_volume_fraction=oil_volume_fraction,
        apply_turbulence_correction=apply_turbulence_correction,
        inlet_device=inlet_device,
        override_bulk_efficiency=override_bulk_efficiency,
        override_dsd_multiplier=override_dsd_multiplier,
        design_k_factor=design_k_factor,
        liquid_liquid_residence_time=liquid_liquid_residence_time,
    )


def shortcut_distillation_column(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    light_key: str,
    heavy_key: str,
    light_key_recovery_distillate: float,
    heavy_key_recovery_bottoms: float,
    reflux_ratio_multiplier: float,
    condenser_pressure: Q | None = None,
    reboiler_pressure: Q | None = None,
) -> ShortcutDistillationColumnResult:
    """Solve a Fenske-Underwood-Gilliland shortcut column.

    ``light_key`` and ``heavy_key`` name the two components the separation is written
    between, and their recoveries plus ``reflux_ratio_multiplier`` are the four numbers the
    class takes. The answer is **not a profile but a stage count, a feed tray and two
    duties**: Fenske's minimum stages, Underwood's minimum reflux, Molokanov's fit to
    Gilliland for the actual stages, Kirkbride for the feed tray.

    Three things the class does that the numbers alone do not show, and all three are
    reproduced rather than corrected. The **duties are estimates**: a hard-coded 30000 J/mol
    average latent heat for the condenser and one per cent of the feed's enthalpy for the
    reboiler, so the condenser's figure does not depend on what the fluid is. The
    **Kirkbride argument is the class's own**, with the square on the bottoms-to-distillate
    ratio, where the correlation as usually quoted squares the composition ratio. And a
    **reflux multiplier at or below one is refused**, because the class returns an infinite
    stage count at exactly one and a silent fallback below it.

    Raises:
        InvalidInputError: for a key that is not a component, a relative volatility at or
            below one, a reflux multiplier at or below one, a split that empties a product,
            or a feed whose flash is a trivial solution.

    See :func:`azoth.process.reference.shortcut_distillation_column`.
    """
    return resolve(_SHORTCUT_DISTILLATION_COLUMN)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        light_key=light_key,
        heavy_key=heavy_key,
        light_key_recovery_distillate=light_key_recovery_distillate,
        heavy_key_recovery_bottoms=heavy_key_recovery_bottoms,
        reflux_ratio_multiplier=reflux_ratio_multiplier,
        condenser_pressure=condenser_pressure,
        reboiler_pressure=reboiler_pressure,
    )


def throttling_valve(
    components: list[str],
    inlet_n: Q,
    inlet_z: list[float],
    inlet_p: Q,
    inlet_t: Q,
    outlet_pressure: Q,
    valve_opening: float | None = None,
    kv: float | None = None,
    fouling_fraction: float | None = None,
    valve_characterization: str | None = None,
    xt: float | None = None,
    allow_choked: bool | None = None,
    is_calc_pressure: bool | None = None,
    is_isothermal: bool | None = None,
    sizing_standard: str | None = None,
) -> ThrottlingValveResult:
    """Drop a stream to a lower pressure without heat or work.

    ``inlet_n``, ``inlet_z``, ``inlet_p`` and ``inlet_t`` are the inlet's record and
    ``outlet_pressure`` is ``unit_ops.throttling_valve``'s own parameter. The drop is
    isenthalpic, so the outlet's temperature is the one the mixture reaches at the outlet
    pressure and its enthalpy is the inlet's.

    ``outlet_pressure`` is taken at face value even when it is above the inlet: NeqSim's
    ``acceptNegativeDP`` defaults to ``true`` and the capture's third case records it.

    See :func:`azoth.process.reference.throttling_valve`.
    """
    return resolve(_THROTTLING_VALVE)(  # type: ignore[no-any-return]
        components=components,
        inlet_n=inlet_n,
        inlet_z=inlet_z,
        inlet_p=inlet_p,
        inlet_t=inlet_t,
        outlet_pressure=outlet_pressure,
        valve_opening=valve_opening,
        kv=kv,
        fouling_fraction=fouling_fraction,
        valve_characterization=valve_characterization,
        xt=xt,
        allow_choked=allow_choked,
        is_calc_pressure=is_calc_pressure,
        is_isothermal=is_isothermal,
        sizing_standard=sizing_standard,
    )


def heat_exchanger(
    hot_components: list[str],
    hot_in_n: Q,
    hot_in_z: list[float],
    hot_in_p: Q,
    hot_in_t: Q,
    cold_components: list[str],
    cold_in_n: Q,
    cold_in_z: list[float],
    cold_in_p: Q,
    cold_in_t: Q,
    ua: Q | None = None,
    flow_arrangement: str = "counterflow",
    hot_outlet_temperature: Q | None = None,
    cold_outlet_temperature: Q | None = None,
    use_delta_t: bool | None = None,
    delta_t: Q | None = None,
    guess_out_temperature: Q | None = None,
    mechanical_design: bool | None = None,
    mechanical_type: str | None = None,
    mechanical_selection: str | None = None,
    use_rating_pressure_drop: bool | None = None,
    rating_area: Q | None = None,
    rating_tube_side: str | None = None,
    rating_tube_outside_diameter: Q | None = None,
    rating_tube_inside_diameter: Q | None = None,
    rating_tube_length: Q | None = None,
    rating_tube_count: float | None = None,
    rating_tube_passes: float | None = None,
    rating_tube_pitch: Q | None = None,
    rating_triangular_pitch: bool | None = None,
    rating_shell_inside_diameter: Q | None = None,
    rating_baffle_spacing: Q | None = None,
    rating_baffle_count: float | None = None,
    rating_baffle_cut: float | None = None,
    rating_tube_wall_conductivity: Q | None = None,
    rating_fouling_tube: float | None = None,
    rating_fouling_shell: float | None = None,
    rating_tube_fouling_layer_thickness: Q | None = None,
    rating_tube_fouling_layer_conductivity: Q | None = None,
    rating_shell_side_method: str | None = None,
    rating_shell_viscosity_wall: Q | None = None,
    rating_has_sealing: bool | None = None,
    rating_sealing_pairs: float | None = None,
    rating_tube_to_baffle_clearance: Q | None = None,
    rating_shell_to_baffle_clearance: Q | None = None,
    rating_bypass_area: Q | None = None,
    time_step: Q | None = None,
    dynamic_model_enabled: bool | None = None,
    transient_tube_side: str | None = None,
    wall_mass: Q | None = None,
    wall_cp: Q | None = None,
    wall_temperature: Q | None = None,
    shell_holdup_volume: Q | None = None,
    tube_holdup_volume: Q | None = None,
    shell_side_htc: Q | None = None,
    tube_side_htc: Q | None = None,
    heat_transfer_area: Q | None = None,
    shell_fluid_temperature: Q | None = None,
    tube_fluid_temperature: Q | None = None,
) -> HeatExchangerResult:
    """Exchange heat between two streams.

    **The two sides carry different fluids**, so this is the one unit operation with two
    component lists rather than one. ``hot_in_*`` and ``cold_in_*`` are the two inlets'
    records and ``ua``, ``flow_arrangement``, ``hot_outlet_temperature`` and
    ``cold_outlet_temperature`` are ``unit_ops.heat_exchanger``'s own parameters, of which
    the last three describe two modes.

    The default mode is the **effectiveness-NTU rating** its NeqSim class runs: ``ua`` and
    ``flow_arrangement`` size the exchanger and the duty is what comes out. Pinning one
    outlet temperature is the other mode, and it energy-balances the opposite side. A stated
    ``time_step`` is one ``runTransient`` step; the wall and holdup temperatures are carried
    back in as inputs.

    Raises:
        InvalidInputError: where neither or both outlet temperatures are given, where the
            rating is asked for without a ``ua``, or where the arrangement is not one of
            ``counterflow``, ``parallelflow`` and ``shell_and_tube``.

    See :func:`azoth.process.reference.heat_exchanger`.
    """
    return resolve(_HEAT_EXCHANGER)(  # type: ignore[no-any-return]
        hot_components=hot_components,
        hot_in_n=hot_in_n,
        hot_in_z=hot_in_z,
        hot_in_p=hot_in_p,
        hot_in_t=hot_in_t,
        cold_components=cold_components,
        cold_in_n=cold_in_n,
        cold_in_z=cold_in_z,
        cold_in_p=cold_in_p,
        cold_in_t=cold_in_t,
        ua=ua,
        flow_arrangement=flow_arrangement,
        hot_outlet_temperature=hot_outlet_temperature,
        cold_outlet_temperature=cold_outlet_temperature,
        use_delta_t=use_delta_t,
        delta_t=delta_t,
        guess_out_temperature=guess_out_temperature,
        mechanical_design=mechanical_design,
        mechanical_type=mechanical_type,
        mechanical_selection=mechanical_selection,
        use_rating_pressure_drop=use_rating_pressure_drop,
        rating_area=rating_area,
        rating_tube_side=rating_tube_side,
        rating_tube_outside_diameter=rating_tube_outside_diameter,
        rating_tube_inside_diameter=rating_tube_inside_diameter,
        rating_tube_length=rating_tube_length,
        rating_tube_count=rating_tube_count,
        rating_tube_passes=rating_tube_passes,
        rating_tube_pitch=rating_tube_pitch,
        rating_triangular_pitch=rating_triangular_pitch,
        rating_shell_inside_diameter=rating_shell_inside_diameter,
        rating_baffle_spacing=rating_baffle_spacing,
        rating_baffle_count=rating_baffle_count,
        rating_baffle_cut=rating_baffle_cut,
        rating_tube_wall_conductivity=rating_tube_wall_conductivity,
        rating_fouling_tube=rating_fouling_tube,
        rating_fouling_shell=rating_fouling_shell,
        rating_tube_fouling_layer_thickness=rating_tube_fouling_layer_thickness,
        rating_tube_fouling_layer_conductivity=rating_tube_fouling_layer_conductivity,
        rating_shell_side_method=rating_shell_side_method,
        rating_shell_viscosity_wall=rating_shell_viscosity_wall,
        rating_has_sealing=rating_has_sealing,
        rating_sealing_pairs=rating_sealing_pairs,
        rating_tube_to_baffle_clearance=rating_tube_to_baffle_clearance,
        rating_shell_to_baffle_clearance=rating_shell_to_baffle_clearance,
        rating_bypass_area=rating_bypass_area,
        time_step=time_step,
        dynamic_model_enabled=dynamic_model_enabled,
        transient_tube_side=transient_tube_side,
        wall_mass=wall_mass,
        wall_cp=wall_cp,
        wall_temperature=wall_temperature,
        shell_holdup_volume=shell_holdup_volume,
        tube_holdup_volume=tube_holdup_volume,
        shell_side_htc=shell_side_htc,
        tube_side_htc=tube_side_htc,
        heat_transfer_area=heat_transfer_area,
        shell_fluid_temperature=shell_fluid_temperature,
        tube_fluid_temperature=tube_fluid_temperature,
    )


def gibbs_reactor(
    components: list[str],
    feed_n: Q,
    feed_z: list[float],
    feed_p: Q,
    feed_t: Q,
    energy_mode: str,
    damping_composition: float,
    max_iterations: float,
    convergence_tolerance: float,
    min_iterations: float,
    reactor_type: str | None = None,
    catalyst_bulk_density: float | None = None,
    catalyst_particle_diameter: float | None = None,
    bed_void_fraction: float | None = None,
    ghsv: float | None = None,
    lhsv: float | None = None,
    number_of_beds: int | None = None,
    max_operation_pressure: float | None = None,
    max_operation_temperature: float | None = None,
    joint_efficiency: float | None = None,
    corrosion_allowance: float | None = None,
    armijo_line_search: bool | None = None,
    armijo_c1: float | None = None,
    armijo_rho: float | None = None,
    armijo_max_backtracks: int | None = None,
    adaptive_step_size: bool | None = None,
    regularization: bool | None = None,
    regularization_threshold: float | None = None,
    regularization_tau: float | None = None,
) -> GibbsReactorResult:
    """Bring a feed to its Gibbs equilibrium at its own temperature and pressure.

    ``GibbsReactor.run`` carries its **own** Lagrange-multiplier Newton iteration over the
    element balances and its **own** species database, and neither is ``ChemicalEquilibrium``'s -
    so this is not a composition of P10's minimiser and the two would answer differently.

    **The equilibrium temperature is the feed's, and there is no setter.** The class reads
    ``system.getTemperature()`` from the fluid it is handed.

    **The convergence test is on the undamped step and the floor bites.** ``min_iterations`` is
    the pass below which the tolerance is not consulted, and three of the class's own capture
    rows stop there rather than on the tolerance. A run that reaches ``max_iterations`` still
    succeeds, with ``converged`` false.

    **The vessel is ``ReactorMechanicalDesign``, and it is read from the feed.** The eleven
    mechanical inputs default to the class's own field initialisers, so omitting them gives the
    class's design rather than no design; ``ghsv`` and ``lhsv`` are reciprocal hours and cross
    unconverted, the unit vocabulary having no reciprocal-time unit.

    See :func:`azoth.process.reference.gibbs_reactor`.
    """
    return resolve(_GIBBS_REACTOR)(  # type: ignore[no-any-return]
        components=components,
        feed_n=feed_n,
        feed_z=feed_z,
        feed_p=feed_p,
        feed_t=feed_t,
        energy_mode=energy_mode,
        damping_composition=damping_composition,
        max_iterations=max_iterations,
        convergence_tolerance=convergence_tolerance,
        min_iterations=min_iterations,
        reactor_type=reactor_type,
        catalyst_bulk_density=catalyst_bulk_density,
        catalyst_particle_diameter=catalyst_particle_diameter,
        bed_void_fraction=bed_void_fraction,
        ghsv=ghsv,
        lhsv=lhsv,
        number_of_beds=number_of_beds,
        max_operation_pressure=max_operation_pressure,
        max_operation_temperature=max_operation_temperature,
        joint_efficiency=joint_efficiency,
        corrosion_allowance=corrosion_allowance,
        armijo_line_search=armijo_line_search,
        armijo_c1=armijo_c1,
        armijo_rho=armijo_rho,
        armijo_max_backtracks=armijo_max_backtracks,
        adaptive_step_size=adaptive_step_size,
        regularization=regularization,
        regularization_threshold=regularization_threshold,
        regularization_tau=regularization_tau,
    )


def vacuum_collapse(
    components: list[str],
    mole_fractions: list[float],
    vessel_volume: Q,
    initial_pressure: Q,
    initial_temperature: Q,
    cold_end_temperature: Q,
    atmospheric_pressure: Q | None = None,
    external_pressure_rating: Q | None = None,
    makeup_setpoint: Q | None = None,
    makeup_gas_molar_mass: Q | None = None,
    cooling_steps: float | None = None,
) -> VacuumCollapseResult:
    """The constant-volume cooling sweep's minimum pressure and the make-up gas.

    The inventory is the amount that fills the rigid vessel at the stated state, and the
    sweep holds its molar volume while a constant-volume flash runs at every cooling step.
    **An absent external rating is 0 bara**, the class's full-vacuum assumption.

    See :func:`azoth.process.reference.vacuum_collapse`.
    """
    return resolve(_VACUUM_COLLAPSE)(  # type: ignore[no-any-return]
        components=components,
        mole_fractions=mole_fractions,
        vessel_volume=vessel_volume,
        initial_pressure=initial_pressure,
        initial_temperature=initial_temperature,
        cold_end_temperature=cold_end_temperature,
        atmospheric_pressure=atmospheric_pressure,
        external_pressure_rating=external_pressure_rating,
        makeup_setpoint=makeup_setpoint,
        makeup_gas_molar_mass=makeup_gas_molar_mass,
        cooling_steps=cooling_steps,
    )


def vce(
    cloud_fuel_mass: Q,
    heat_of_combustion: float,
    strength_class: float,
    ambient_pressure: Q | None = None,
    distance: Q | None = None,
    target_overpressure: Q | None = None,
) -> VceResult:
    """The TNO Multi-Energy overpressure at one distance, or its inverse.

    The Sachs-scaled distance, the strength-class plateau and the class's power-law fit.
    **A target above the near-field overpressure is absent**: the class answers 0.1 m with
    a lower overpressure.

    See :func:`azoth.process.reference.vce`.
    """
    return resolve(_VCE)(  # type: ignore[no-any-return]
        cloud_fuel_mass=cloud_fuel_mass,
        heat_of_combustion=heat_of_combustion,
        strength_class=strength_class,
        ambient_pressure=ambient_pressure,
        distance=distance,
        target_overpressure=target_overpressure,
    )


def blowdown_pressure_profile(
    points: list[dict[str, object]],
    time: Q,
    interpolation_mode: str | None = None,
    gauge_offset: Q | None = None,
) -> BlowdownPressureProfileResult:
    """The blowdown profile's pressure at one query time.

    The class's step or linear lookup, its absolute ``1e-10`` second window, both endpoint
    clamps, and the absolute pressure less the gauge offset.

    Raises:
        InvalidInputError: for fewer than two points, a negative or non-finite time, a
            non-positive pressure, times that are not strictly increasing, a negative
            gauge offset, or an unknown mode.

    See :func:`azoth.process.reference.blowdown_pressure_profile`.
    """
    return resolve(_BLOWDOWN_PRESSURE_PROFILE)(  # type: ignore[no-any-return]
        points=points,
        time=time,
        interpolation_mode=interpolation_mode,
        gauge_offset=gauge_offset,
    )


def pipe_fire_rupture_material(
    material_name: str,
    evaluation_temperature: Q,
    evaluation_stress: Q,
    curve: list[dict[str, object]] | None = None,
    density: Q | None = None,
    sellars_tegart_a: float | None = None,
    sellars_tegart_alpha: float | None = None,
    sellars_tegart_n: float | None = None,
    activation_temperature: Q | None = None,
    correction_strain_rate: Q | None = None,
    correction_limit_temperature: Q | None = None,
) -> PipeFireRuptureMaterialResult:
    """A pipe-fire material's properties and Sellars-Tegart strain rate.

    The six workbook tables or the caller's curve, their clamped interpolation, the
    class's temperature-correction factor and the strain rate at one state.

    Raises:
        InvalidInputError: for an unknown name, a curve and constants that are not
            all-or-none, an invalid grid or constant, or a non-positive temperature.

    See :func:`azoth.process.reference.pipe_fire_rupture_material`.
    """
    return resolve(_PIPE_FIRE_RUPTURE_MATERIAL)(  # type: ignore[no-any-return]
        material_name=material_name,
        evaluation_temperature=evaluation_temperature,
        evaluation_stress=evaluation_stress,
        curve=curve,
        density=density,
        sellars_tegart_a=sellars_tegart_a,
        sellars_tegart_alpha=sellars_tegart_alpha,
        sellars_tegart_n=sellars_tegart_n,
        activation_temperature=activation_temperature,
        correction_strain_rate=correction_strain_rate,
        correction_limit_temperature=correction_limit_temperature,
    )


def pipe_fire_rupture(
    outside_diameter: Q,
    nominal_wall_thickness: Q,
    material_name: str,
    fire_scenario: str,
    pressure_profile: list[dict[str, object]],
    corrosion_allowance: Q | None = None,
    wall_thickness_undertolerance: float | None = None,
    exposed_length: Q | None = None,
    weight_stress: Q | None = None,
    fluid_density: Q | None = None,
    fluid_heat_capacity: Q | None = None,
    gas_molecular_weight: Q | None = None,
    initial_temperature: Q | None = None,
    fire_temperature: Q | None = None,
    gas_temperature: Q | None = None,
    convective_heat_transfer_coefficient: Q | None = None,
    fire_emissivity: float | None = None,
    metal_emissivity: float | None = None,
    metal_absorptivity: float | None = None,
    passive_protection_heat_flux_factor: float | None = None,
    gauge_offset: Q | None = None,
    interpolation_mode: str | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    spreadsheet_gas_thermal_mass: bool | None = None,
) -> PipeFireRuptureResult:
    """The pipe fire-rupture strain-rate march.

    The thick-wall von Mises stress, the Sellars-Tegart strain rate, the accumulated-strain
    limit and the class's spreadsheet-style release, with the step history.

    Raises:
        InvalidInputError: for a non-positive geometry, fluid or temperature, an unknown
            material or scenario, a malformed profile, a step or time out of range, or a
            non-finite value.

    See :func:`azoth.process.reference.pipe_fire_rupture`.
    """
    return resolve(_PIPE_FIRE_RUPTURE)(  # type: ignore[no-any-return]
        outside_diameter=outside_diameter,
        nominal_wall_thickness=nominal_wall_thickness,
        material_name=material_name,
        fire_scenario=fire_scenario,
        pressure_profile=pressure_profile,
        corrosion_allowance=corrosion_allowance,
        wall_thickness_undertolerance=wall_thickness_undertolerance,
        exposed_length=exposed_length,
        weight_stress=weight_stress,
        fluid_density=fluid_density,
        fluid_heat_capacity=fluid_heat_capacity,
        gas_molecular_weight=gas_molecular_weight,
        initial_temperature=initial_temperature,
        fire_temperature=fire_temperature,
        gas_temperature=gas_temperature,
        convective_heat_transfer_coefficient=convective_heat_transfer_coefficient,
        fire_emissivity=fire_emissivity,
        metal_emissivity=metal_emissivity,
        metal_absorptivity=metal_absorptivity,
        passive_protection_heat_flux_factor=passive_protection_heat_flux_factor,
        gauge_offset=gauge_offset,
        interpolation_mode=interpolation_mode,
        time_step=time_step,
        max_time=max_time,
        spreadsheet_gas_thermal_mass=spreadsheet_gas_thermal_mass,
    )


def vessel_rupture(
    inner_radius: Q,
    wall_thickness: Q,
    material: str,
    history: list[dict[str, object]],
    yield_strength: Q | None = None,
    tensile_strength: Q | None = None,
    grade: str | None = None,
    tensile_strength_factor: float | None = None,
) -> VesselRuptureResult:
    """The thin-wall vessel rupture crossover.

    The von Mises wall stress against the temperature-derated allowable strength, the first
    margin at or below zero and the interpolated crossover time.

    Raises:
        InvalidInputError: for a non-positive radius, wall or strength, a missing strength
            or grade, an unknown grade, an empty or non-increasing history, or a
            non-finite value.

    See :func:`azoth.process.reference.vessel_rupture`.
    """
    return resolve(_VESSEL_RUPTURE)(  # type: ignore[no-any-return]
        inner_radius=inner_radius,
        wall_thickness=wall_thickness,
        material=material,
        history=history,
        yield_strength=yield_strength,
        tensile_strength=tensile_strength,
        grade=grade,
        tensile_strength_factor=tensile_strength_factor,
    )
