//! PVT simulation.
//!
//! NeqSim's `pvtsimulation/` without its `flowassurance/`: the black-oil and gas-water
//! correlation tables, the decline and material-balance arithmetic, and the laboratory
//! experiments a reservoir fluid is described by. It is a namespace of its own rather than a
//! family under `eos` or `process`, mirroring NeqSim's package split: a PVT experiment is a
//! sequence of flashes and property reads, not an equation of state and not a unit operation.
//!
//! * [`standard`] — the standard state and the class conversion literals every id here states
//!   its inputs against.

pub mod black_oil_correlations;
pub mod constant_mass_expansion;
pub mod constant_volume_depletion;
pub mod decline_curve;
pub mod differential_liberation;
pub mod fetkovich_aquifer;
pub mod flash;
pub mod fluid;
pub mod gas_correlations;
pub mod gas_material_balance;
pub mod generalized_material_balance;
pub mod ledger;
pub mod mmp;
pub mod model_gen;
pub mod model_records_gen;
pub mod oil_material_balance;
pub mod property_sweep;
pub mod regression;
pub mod relative_permeability;
pub mod saturation;
pub mod saturation_test;
pub mod separator_test;
pub mod standard;
pub mod swelling_test;
pub mod transport;
pub mod tune_to_saturation;
pub mod van_everdingen_hurst_aquifer;
pub mod volume;
pub mod water_correlations;

pub use black_oil_correlations::{
    BlackOilCorrelationsResult, BlackOilQuery, BlackOilResultRow, black_oil_correlations,
};
pub use constant_mass_expansion::{
    CcePoint, ConstantMassExpansionResult, constant_mass_expansion, constant_mass_expansion_of,
};
pub use constant_volume_depletion::{
    ConstantVolumeDepletionResult, CvdPoint, constant_volume_depletion,
    constant_volume_depletion_of,
};
pub use decline_curve::{DeclineCurveResult, decline_curve};
pub use differential_liberation::{
    DifferentialLiberationResult, DlePoint, differential_liberation, differential_liberation_of,
};
pub use fetkovich_aquifer::{
    FetkovichAquiferResult, FetkovichInterval, FetkovichState, fetkovich_aquifer,
};
pub use flash::{Flash, FlashMode, Phase};
pub use fluid::{Fluid, GasTemplate};
pub use gas_correlations::{GasCorrelationsResult, GasQuery, GasResultRow, gas_correlations};
pub use gas_material_balance::{GasMaterialBalanceResult, gas_material_balance};
pub use generalized_material_balance::{
    GeneralizedMaterialBalanceResult, MaterialBalanceObservation, generalized_material_balance,
};
pub use mmp::{MmpPoint, MmpResult, mmp};
pub use oil_material_balance::{OilMaterialBalanceResult, oil_material_balance};
pub use property_sweep::{PropertyPoint, PropertySweepResult, property_sweep};
pub use regression::{
    CceExperiment, CceRow, ConvergenceReason, CvdExperiment, CvdProperty, CvdRow, DleExperiment,
    DleProperty, DleRow, ExperimentType, ExperimentWeights, FittedParameterRow,
    LM_CHI_SQUARE_TOLERANCE, LM_GRADIENT_TOLERANCE, LM_MAXIMUM_ITERATIONS, LM_MINIMUM_ITERATIONS,
    LmResult, Objectives, RegressionData, RegressionFit, RegressionFluid, RegressionObjectiveRow,
    RegressionParameter, RegressionParameterConfig, RegressionParameterRow, RegressionResult,
    Sample, SampleKind, ViscosityPhase, ViscosityPoint, ViscosityRow, build_fluid, chi_square, fit,
    levenberg_marquardt, objectives, regression, sample_value, sample_values,
};
pub use relative_permeability::{
    RelativePermeabilityResult, RelativePermeabilityRow, relative_permeability,
};
pub use saturation_test::{SaturationTestResult, saturation_test};
pub use separator_test::{
    SeparatorStageInput, SeparatorStageResult, SeparatorTestResult, separator_test,
};
pub use swelling_test::{SwellingPoint, SwellingTestResult, swelling_test};
pub use tune_to_saturation::{TuneToSaturationResult, tune_to_saturation};
pub use van_everdingen_hurst_aquifer::{
    VanEverdingenHurstAquiferResult, van_everdingen_hurst_aquifer,
};
pub use water_correlations::{
    WaterCorrelationsResult, WaterQuery, WaterResultRow, water_correlations,
};
