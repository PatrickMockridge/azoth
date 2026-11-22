//! `pvt.separator_test` - NeqSim's three separator experiments.
//!
//! Spec: `specs/models/pvt/separator_test.toml`. The arithmetic is `GOR.java`,
//! `SeparatorTest.java` and `MultiStageSeparatorTest.java` at the pin.
//!
//! * `continuous` is `GOR`: every stage reflashes the full feed - the class's gas-removal line
//!   is commented out - and the stage GOR and Bo are against one stock-tank volume.
//! * `staged` is `SeparatorTest`: the gas leaves after each stage and the liquid cascades.
//! * `train` is `MultiStageSeparatorTest`: the multiphase cascade with stage densities, molar
//!   masses, Z, viscosity, API and an optional first-stage optimisation.
//!
//! The class's temperatures are Kelvin in `GOR` and `SeparatorTest` and Celsius in the train;
//! this id takes Kelvin everywhere and the probe converts. The stage oil volume is the
//! **untranslated** phase volume, as `getVolume()` is, while a density or viscosity is the
//! phase's physical property.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{Pressure, ThermodynamicTemperature};
use azoth_core::{AzothError, Result, Warning};

use crate::flash::{FlashMode, Phase};
use crate::fluid::{Fluid, cubic_named, kij_named};
use crate::standard::{
    SEPARATOR_API_DENSITY_MAX, SEPARATOR_API_DENSITY_MIN, SEPARATOR_API_NUMERATOR,
    SEPARATOR_API_OFFSET, SEPARATOR_API_WATER_DENSITY, STANDARD_PRESSURE_PA,
    STANDARD_TEMPERATURE_K,
};
use crate::transport::{TransportKind, phase_viscosity};

/// One separator stage as the id's input states it.
#[derive(Debug, Clone, PartialEq)]
pub struct SeparatorStageInput {
    /// The stage pressure, in Pa.
    pub pressure: f64,
    /// The stage temperature, in K.
    pub temperature: f64,
}

/// One separator stage as the experiment answers it.
#[derive(Debug, Clone, PartialEq)]
pub struct SeparatorStageResult {
    /// The stage's 1-based position.
    pub stage_number: usize,
    /// The pressure, in Pa.
    pub pressure: f64,
    /// The temperature, in K.
    pub temperature: f64,
    /// The gas volume at the standard state, in m3, in the train.
    pub gas_volume: Option<f64>,
    /// The stage gas-oil ratio.
    pub stage_gor: Option<f64>,
    /// The gas-oil ratio through this stage, in the train.
    pub cumulative_gor: Option<f64>,
    /// The stage oil formation factor, in the continuous and staged modes.
    pub oil_fvf: Option<f64>,
    /// The gas phase's density, in kg/m3, in the train.
    pub gas_density: Option<f64>,
    /// The oil phase's density, in kg/m3, in the train.
    pub oil_density: Option<f64>,
    /// The gas phase's molar mass, in kg/mol, in the train.
    pub gas_molar_mass: Option<f64>,
    /// The oil phase's molar mass, in kg/mol, in the train.
    pub oil_molar_mass: Option<f64>,
    /// The gas phase's compressibility, in the train.
    pub gas_z_factor: Option<f64>,
    /// The oil phase's viscosity, in Pa s, in the train.
    pub oil_viscosity: Option<f64>,
}

/// Result of `pvt.separator_test`.
#[derive(Debug, Clone, PartialEq)]
pub struct SeparatorTestResult {
    /// The total gas-oil ratio, in the train.
    pub total_gor: Option<f64>,
    /// The oil formation volume factor against the reservoir fluid, in the train.
    pub bo: Option<f64>,
    /// The train's `Rs`, which the class sets to the total GOR.
    pub rs: Option<f64>,
    /// The stock-tank oil density, in kg/m3, in the train.
    pub stock_tank_oil_density: Option<f64>,
    /// The stock-tank API gravity, in the train and inside the class's density band.
    pub stock_tank_api: Option<f64>,
    /// The optimal first-stage pressure, in Pa.
    pub optimal_stage_pressure: Option<f64>,
    /// The optimal first-stage temperature, in K.
    pub optimal_stage_temperature: Option<f64>,
    /// `1/Bo` at the optimum.
    pub maximum_oil_recovery: Option<f64>,
    /// The GOR at the optimum.
    pub gor_at_optimum: Option<f64>,
    /// The Bo at the optimum.
    pub bo_at_optimum: Option<f64>,
    /// The API at the optimum.
    pub api_at_optimum: Option<f64>,
    /// One row per stage.
    pub stages: Vec<SeparatorStageResult>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The separator experiment over the stated stages.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown `mode`, `eos` or `mixing_rule`, no stages, a
///   non-positive pressure or temperature, an optimisation outside the train or with a bad
///   grid - and propagates the flashes' refusals.
#[allow(clippy::too_many_arguments)]
pub fn separator_test(
    components: &[FluidComponent],
    mode: &str,
    stages: &[SeparatorStageInput],
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    volume_correction: Option<bool>,
    reservoir_pressure: Option<Pressure>,
    reservoir_temperature: Option<ThermodynamicTemperature>,
    optimize_first_stage: Option<bool>,
    first_stage_pressure_min: Option<Pressure>,
    first_stage_pressure_max: Option<Pressure>,
    first_stage_pressure_steps: Option<usize>,
    first_stage_temperature_min: Option<ThermodynamicTemperature>,
    first_stage_temperature_max: Option<ThermodynamicTemperature>,
    first_stage_temperature_steps: Option<usize>,
) -> Result<SeparatorTestResult> {
    let feed = Fluid::from_rows(
        components,
        cubic_named(eos)?,
        kij_named(mixing_rule)?,
        volume_correction.unwrap_or(true),
    )?;
    if stages.is_empty() {
        return Err(AzothError::invalid_input(
            "stages",
            "a separator test needs at least one stage",
        ));
    }
    for (index, stage) in stages.iter().enumerate() {
        positive(&format!("stages[{index}].pressure"), stage.pressure)?;
        positive(&format!("stages[{index}].temperature"), stage.temperature)?;
    }

    match mode {
        "continuous" => continuous(&feed, stages),
        "staged" => staged(&feed, stages),
        "train" => {
            let p = reservoir_pressure.ok_or_else(|| {
                AzothError::invalid_input(
                    "reservoir_pressure",
                    "the train needs a reservoir pressure",
                )
            })?;
            let t = reservoir_temperature.ok_or_else(|| {
                AzothError::invalid_input(
                    "reservoir_temperature",
                    "the train needs a reservoir temperature",
                )
            })?;
            positive("reservoir_pressure", p.value)?;
            positive("reservoir_temperature", t.value)?;
            let outcome = run_train(&feed, stages, t.value, p.value)?;
            let mut result = outcome.result;
            if optimize_first_stage.unwrap_or(false) {
                let optimization = optimisation(
                    &feed,
                    stages,
                    t.value,
                    p.value,
                    first_stage_pressure_min,
                    first_stage_pressure_max,
                    first_stage_pressure_steps,
                    first_stage_temperature_min,
                    first_stage_temperature_max,
                    first_stage_temperature_steps,
                )?;
                result.optimal_stage_pressure = Some(optimization.pressure);
                result.optimal_stage_temperature = Some(optimization.temperature);
                result.maximum_oil_recovery = Some(optimization.recovery);
                result.gor_at_optimum = optimization.gor;
                result.bo_at_optimum = optimization.bo;
                result.api_at_optimum = optimization.api;
            }
            Ok(result)
        }
        other => Err(AzothError::invalid_input(
            "mode",
            format!("{other} is not a separator mode; use continuous, staged or train"),
        )),
    }
}

fn continuous(feed: &Fluid, stages: &[SeparatorStageInput]) -> Result<SeparatorTestResult> {
    let mut rows = Vec::with_capacity(stages.len());
    let mut gas_volumes = Vec::with_capacity(stages.len());
    let mut oil_volumes = Vec::with_capacity(stages.len());
    for (index, stage) in stages.iter().enumerate() {
        let flash = feed.flash(stage.temperature, stage.pressure, FlashMode::TwoPhase)?;
        oil_volumes.push(stage_oil_volume(&flash));
        gas_volumes.push(stage_gas_volume(feed, &flash)?);
        rows.push(SeparatorStageResult {
            stage_number: index + 1,
            pressure: stage.pressure,
            temperature: stage.temperature,
            gas_volume: None,
            stage_gor: None,
            cumulative_gor: None,
            oil_fvf: None,
            gas_density: None,
            oil_density: None,
            gas_molar_mass: None,
            oil_molar_mass: None,
            gas_z_factor: None,
            oil_viscosity: None,
        });
    }
    let stock = stock_tank_oil(feed)?;
    for (index, row) in rows.iter_mut().enumerate() {
        if let Some(stock) = stock {
            let gas = gas_volumes[index];
            if gas > 1.0e-10 {
                row.stage_gor = Some(gas / stock);
                row.oil_fvf = Some(oil_volumes[index] / stock);
            }
        }
    }
    Ok(SeparatorTestResult {
        total_gor: None,
        bo: None,
        rs: None,
        stock_tank_oil_density: None,
        stock_tank_api: None,
        optimal_stage_pressure: None,
        optimal_stage_temperature: None,
        maximum_oil_recovery: None,
        gor_at_optimum: None,
        bo_at_optimum: None,
        api_at_optimum: None,
        stages: rows,
        warnings: Vec::new(),
    })
}

fn staged(feed: &Fluid, stages: &[SeparatorStageInput]) -> Result<SeparatorTestResult> {
    let mut fluid = feed.clone();
    let mut rows = Vec::with_capacity(stages.len());
    let mut gas_volumes = Vec::with_capacity(stages.len());
    let mut oil_volumes = Vec::with_capacity(stages.len());
    for (index, stage) in stages.iter().enumerate() {
        let flash = fluid.flash(stage.temperature, stage.pressure, FlashMode::TwoPhase)?;
        oil_volumes.push(stage_oil_volume(&flash));
        gas_volumes.push(stage_gas_volume(feed, &flash)?);
        if flash.phase_count > 1 {
            // The class strips the gas and carries phase 1 onward.
            if let Some(carried) = flash.phases.get(1) {
                fluid = fluid.with_moles(component_moles(carried, flash.total_moles));
            }
        }
        rows.push(SeparatorStageResult {
            stage_number: index + 1,
            pressure: stage.pressure,
            temperature: stage.temperature,
            gas_volume: None,
            stage_gor: None,
            cumulative_gor: None,
            oil_fvf: None,
            gas_density: None,
            oil_density: None,
            gas_molar_mass: None,
            oil_molar_mass: None,
            gas_z_factor: None,
            oil_viscosity: None,
        });
    }
    let stock = stock_tank_oil(&fluid)?;
    for (index, row) in rows.iter_mut().enumerate() {
        if let Some(stock) = stock {
            let gas = gas_volumes[index];
            if gas > 1.0e-10 {
                row.stage_gor = Some(gas / stock);
                row.oil_fvf = Some(oil_volumes[index] / stock);
            }
        }
    }
    Ok(SeparatorTestResult {
        total_gor: None,
        bo: None,
        rs: None,
        stock_tank_oil_density: None,
        stock_tank_api: None,
        optimal_stage_pressure: None,
        optimal_stage_temperature: None,
        maximum_oil_recovery: None,
        gor_at_optimum: None,
        bo_at_optimum: None,
        api_at_optimum: None,
        stages: rows,
        warnings: Vec::new(),
    })
}

struct TrainOutcome {
    result: SeparatorTestResult,
    bo: Option<f64>,
    gor: Option<f64>,
    api: Option<f64>,
}

fn run_train(
    feed: &Fluid,
    stages: &[SeparatorStageInput],
    reservoir_temperature: f64,
    reservoir_pressure: f64,
) -> Result<TrainOutcome> {
    let reservoir = feed.flash(
        reservoir_temperature,
        reservoir_pressure,
        FlashMode::Multiphase,
    )?;
    let reservoir_volume = match reservoir.oil() {
        Some(oil) => oil.volume_uncorrected(reservoir.total_moles),
        None => reservoir.volume_uncorrected(),
    };

    let mut current = feed.clone();
    let mut rows = Vec::with_capacity(stages.len());
    let mut gas_volumes = Vec::with_capacity(stages.len());
    let mut gas_densities = Vec::with_capacity(stages.len());
    let mut oil_densities = Vec::with_capacity(stages.len());
    let mut gas_masses = Vec::with_capacity(stages.len());
    let mut oil_masses = Vec::with_capacity(stages.len());
    let mut gas_z = Vec::with_capacity(stages.len());
    let mut oil_viscosities = Vec::with_capacity(stages.len());
    let mut reservoir_volume = Some(reservoir_volume);
    let mut stock_volume: Option<f64> = None;
    let mut stock_density: Option<f64> = None;

    for (index, stage) in stages.iter().enumerate() {
        let flash = current.flash(stage.temperature, stage.pressure, FlashMode::Multiphase)?;
        let gas = flash.gas();
        let liquid = flash.oil().or_else(|| {
            flash
                .phases
                .iter()
                .find(|phase| phase.label != azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Gas)
        });

        let mut gas_volume = 0.0;
        let mut gas_density = None;
        let mut gas_mass = None;
        let mut z = None;
        if let Some(gas) = gas {
            let standard = feed
                .with_moles(component_moles(gas, flash.total_moles))
                .flash(
                    STANDARD_TEMPERATURE_K,
                    STANDARD_PRESSURE_PA,
                    FlashMode::TwoPhase,
                )?;
            gas_volume = stage_standard_gas_volume(&standard);
            gas_density = Some(gas.density());
            gas_mass = Some(gas.molar_mass);
            z = Some(gas.z_factor);
        }
        let (oil_density, oil_mass, oil_viscosity) = match liquid {
            Some(liquid) => (
                Some(liquid.density()),
                Some(liquid.molar_mass),
                Some(phase_viscosity(
                    &current.mixture,
                    stage.temperature,
                    stage.pressure,
                    &liquid.x,
                    kind(liquid),
                )?),
            ),
            None => (None, None, None),
        };
        if let (Some(_), Some(liquid)) = (gas, liquid) {
            current = current.with_moles(component_moles(liquid, flash.total_moles));
        }
        gas_volumes.push(gas_volume);
        gas_densities.push(gas_density);
        oil_densities.push(oil_density);
        gas_masses.push(gas_mass);
        oil_masses.push(oil_mass);
        gas_z.push(z);
        oil_viscosities.push(oil_viscosity);

        if index + 1 == stages.len() {
            let standard = current.flash(
                STANDARD_TEMPERATURE_K,
                STANDARD_PRESSURE_PA,
                FlashMode::TwoPhase,
            )?;
            let oil = standard.oil().or_else(|| {
                standard
                    .phases
                    .iter()
                    .find(|phase| phase.label != azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Gas)
            });
            match oil {
                Some(oil) => {
                    stock_volume = Some(oil.volume_uncorrected(standard.total_moles));
                    stock_density = Some(oil.density());
                }
                None => stock_volume = Some(standard.volume_uncorrected()),
            }
        }
        rows.push(SeparatorStageResult {
            stage_number: index + 1,
            pressure: stage.pressure,
            temperature: stage.temperature,
            gas_volume: None,
            stage_gor: None,
            cumulative_gor: None,
            oil_fvf: None,
            gas_density: None,
            oil_density: None,
            gas_molar_mass: None,
            oil_molar_mass: None,
            gas_z_factor: None,
            oil_viscosity: None,
        });
    }

    let stock_volume = stock_volume.ok_or_else(|| {
        AzothError::invalid_input(
            "stages",
            "the train's last stage did not reach a stock tank",
        )
    })?;
    let stock_density = stock_density
        .ok_or_else(|| AzothError::invalid_input("stages", "the stock tank has no oil phase"))?;
    let api = if (SEPARATOR_API_DENSITY_MIN..=SEPARATOR_API_DENSITY_MAX).contains(&stock_density) {
        Some(
            SEPARATOR_API_NUMERATOR / (stock_density / SEPARATOR_API_WATER_DENSITY)
                - SEPARATOR_API_OFFSET,
        )
    } else {
        None
    };
    let total_gas: f64 = gas_volumes.iter().sum();
    let total_gor = total_gas / stock_volume;
    let bo = reservoir_volume.take().map(|volume| volume / stock_volume);
    let mut cumulative = 0.0;
    for (index, row) in rows.iter_mut().enumerate() {
        cumulative += gas_volumes[index];
        row.gas_volume = Some(gas_volumes[index]);
        row.stage_gor = Some(gas_volumes[index] / stock_volume);
        row.cumulative_gor = Some(cumulative / stock_volume);
        row.gas_density = gas_densities[index];
        row.oil_density = oil_densities[index];
        row.gas_molar_mass = gas_masses[index];
        row.oil_molar_mass = oil_masses[index];
        row.gas_z_factor = gas_z[index];
        row.oil_viscosity = oil_viscosities[index];
    }
    let result = SeparatorTestResult {
        total_gor: Some(total_gor),
        bo,
        rs: Some(total_gor),
        stock_tank_oil_density: Some(stock_density),
        stock_tank_api: api,
        optimal_stage_pressure: None,
        optimal_stage_temperature: None,
        maximum_oil_recovery: None,
        gor_at_optimum: None,
        bo_at_optimum: None,
        api_at_optimum: None,
        stages: rows,
        warnings: Vec::new(),
    };
    let bo = result.bo;
    let gor = result.total_gor;
    let api_at_optimum = result.stock_tank_api;
    Ok(TrainOutcome {
        result,
        bo,
        gor,
        api: api_at_optimum,
    })
}

struct Optimisation {
    pressure: f64,
    temperature: f64,
    recovery: f64,
    gor: Option<f64>,
    bo: Option<f64>,
    api: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
fn optimisation(
    feed: &Fluid,
    stages: &[SeparatorStageInput],
    reservoir_temperature: f64,
    reservoir_pressure: f64,
    pressure_min: Option<Pressure>,
    pressure_max: Option<Pressure>,
    pressure_steps: Option<usize>,
    temperature_min: Option<ThermodynamicTemperature>,
    temperature_max: Option<ThermodynamicTemperature>,
    temperature_steps: Option<usize>,
) -> Result<Optimisation> {
    let p_min = positive(
        "first_stage_pressure_min",
        pressure_min.map_or(f64::NAN, |p| p.value),
    )?;
    let p_max = positive(
        "first_stage_pressure_max",
        pressure_max.map_or(f64::NAN, |p| p.value),
    )?;
    let t_min = positive(
        "first_stage_temperature_min",
        temperature_min.map_or(f64::NAN, |t| t.value),
    )?;
    let t_max = positive(
        "first_stage_temperature_max",
        temperature_max.map_or(f64::NAN, |t| t.value),
    )?;
    let p_steps = pressure_steps.unwrap_or(0);
    let t_steps = temperature_steps.unwrap_or(0);
    if p_steps < 2 || t_steps < 2 {
        return Err(AzothError::invalid_input(
            "first_stage_pressure_steps",
            "the optimisation grid needs at least two points on each axis",
        ));
    }
    if p_min >= p_max || t_min >= t_max {
        return Err(AzothError::invalid_input(
            "first_stage",
            "the optimisation grid's bounds are not increasing",
        ));
    }

    let mut best: Option<(f64, f64, TrainOutcome)> = None;
    for p_index in 0..p_steps {
        let pressure = p_min + (p_max - p_min) * p_index as f64 / (p_steps - 1) as f64;
        for t_index in 0..t_steps {
            let temperature = t_min + (t_max - t_min) * t_index as f64 / (t_steps - 1) as f64;
            let mut trial = stages.to_vec();
            trial[0].pressure = pressure;
            trial[0].temperature = temperature;
            let outcome = run_train(feed, &trial, reservoir_temperature, reservoir_pressure)?;
            let Some(bo) = outcome.bo else {
                continue;
            };
            let better = match &best {
                None => true,
                Some((_, _, held)) => held.bo.is_none_or(|held_bo| bo < held_bo),
            };
            if bo > 0.0 && better {
                best = Some((pressure, temperature, outcome));
            }
        }
    }
    let (pressure, temperature, outcome) = best.ok_or_else(|| {
        AzothError::invalid_input(
            "first_stage",
            "no grid point answered a positive Bo, so there is no optimum",
        )
    })?;
    let bo = outcome.bo.unwrap_or(f64::NAN);
    Ok(Optimisation {
        pressure,
        temperature,
        recovery: 1.0 / bo,
        gor: outcome.gor,
        bo: outcome.bo,
        api: outcome.api,
    })
}

fn stage_oil_volume(flash: &crate::flash::Flash) -> f64 {
    if flash.phase_count > 1 {
        flash.phases.get(1).map_or_else(
            || flash.volume_uncorrected(),
            |phase| phase.volume_uncorrected(flash.total_moles),
        )
    } else {
        flash.volume_uncorrected()
    }
}

fn stage_gas_volume(feed: &Fluid, flash: &crate::flash::Flash) -> Result<f64> {
    if flash.phase_count > 1 {
        if let Some(phase) = flash.phases.first() {
            if phase.label == azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Gas {
                let standard = feed
                    .with_moles(component_moles(phase, flash.total_moles))
                    .flash(
                        STANDARD_TEMPERATURE_K,
                        STANDARD_PRESSURE_PA,
                        FlashMode::TwoPhase,
                    )?;
                return Ok(stage_standard_gas_volume(&standard));
            }
        }
    }
    Ok(0.0)
}

/// The gas phase's untranslated volume from a standard flash, as `getPhase(0).getVolume()`
/// is after the class re-initialises the gas at 288.15 K and 1.01325 bar.
fn stage_standard_gas_volume(flash: &crate::flash::Flash) -> f64 {
    flash.gas().map_or_else(
        || flash.volume_uncorrected(),
        |gas| gas.volume_uncorrected(flash.total_moles),
    )
}

fn stock_tank_oil(fluid: &Fluid) -> Result<Option<f64>> {
    let standard = fluid.flash(
        STANDARD_TEMPERATURE_K,
        STANDARD_PRESSURE_PA,
        FlashMode::TwoPhase,
    )?;
    Ok(standard
        .oil()
        .map(|oil| oil.volume_uncorrected(standard.total_moles)))
}

fn component_moles(phase: &Phase, total: f64) -> Vec<f64> {
    let moles = phase.moles(total);
    phase.x.iter().map(|fraction| fraction * moles).collect()
}

fn kind(phase: &Phase) -> TransportKind {
    match phase.label {
        azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Gas => TransportKind::Gas,
        azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Oil => TransportKind::Oil,
        azoth_eos::hydrate_inhibitor_wt::PhaseLabel::Aqueous => TransportKind::Aqueous,
    }
}

fn positive(name: &str, value: f64) -> Result<f64> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(AzothError::invalid_input(
            name,
            format!("the separator test needs a finite positive value; got {value}"),
        ))
    }
}
