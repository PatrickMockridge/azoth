//! `process.dynamic_psv_sizing` - steady API 521 area against a cycling-PSV transient.
//!
//! Spec: `specs/models/process/dynamic_psv_sizing.toml`. The arithmetic is
//! `process/safety/depressurization/DynamicPsvSizingStudy.java` at the pin: one TP flash for
//! the gas properties, the steady gas area, and a bisection over the orifice area whose
//! inner march is the class's ideal-gas vessel with the `PsvValveModel` latch.

use azoth_core::units::{
    Area, MassRate, Power, Pressure, ThermodynamicTemperature, Time, Volume, kilograms_per_second,
    pascals, square_meters,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use azoth_eos::cubic::Cubic;
use azoth_eos::databank;
use azoth_eos::entropy_state::molar_mass_of;
use azoth_eos::heat_capacity::phase_capacity;
use azoth_eos::mixture::{Mixture, ReducedParameters, RootSide};
use azoth_eos::molar_enthalpy_entropy::IdealGasModel;
use azoth_eos::pr_molar_volume::MOLAR_GAS_CONSTANT;
use azoth_eos::pt_flash::pt_flash;
use azoth_eos::results::{Phase, PtFlashResult};

use crate::executor::json::{scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::psv_sizing::gas_area;
use crate::models::psv_valve::step;

/// The class's gas constant, J/(mol K).
const GAS_CONSTANT: f64 = 8.314;
/// The class's fallback capacity ratio when cp/cv degenerates.
const FALLBACK_GAMMA: f64 = 1.3;
/// The class's bisection iterations.
const BISECTION_STEPS: usize = 40;

/// Result of `process.dynamic_psv_sizing`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DynamicPsvSizingResult {
    /// The steady API 521 required area.
    #[serde(serialize_with = "scalar")]
    pub steady_required_area: Area,
    /// The dynamically required area.
    #[serde(serialize_with = "scalar")]
    pub dynamic_required_area: Area,
    /// Steady-to-dynamic oversizing; absent when the dynamic area is not positive.
    pub oversizing_ratio: Option<f64>,
    /// `set_pressure (1 + overpressure_fraction)`.
    #[serde(serialize_with = "scalar")]
    pub allowable_pressure: Pressure,
    /// The peak vessel pressure at the dynamically required area.
    #[serde(serialize_with = "scalar")]
    pub peak_dynamic_pressure: Pressure,
    /// The steady relief rate, `fire_heat_input / latent_heat`.
    #[serde(serialize_with = "scalar")]
    pub steady_relief_rate: MassRate,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// One phase's properties, at its own composition and root.
struct PhaseProperties {
    molar_mass: f64,
    molar_volume: f64,
    cp: f64,
    cv: f64,
}

/// The class's bulk density and capacity ratio after one TP flash.
fn bulk_properties(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    temperature: ThermodynamicTemperature,
    pressure: Pressure,
    z: &[f64],
) -> Result<(f64, f64)> {
    let reduced = mixture.reduced_parameters(temperature, pressure)?;
    let flash: PtFlashResult = pt_flash(mixture, temperature, pressure, z)?;
    match flash.phase {
        Phase::TwoPhase => {
            let beta = flash.vapour_fraction.ok_or_else(|| {
                AzothError::invalid_input(
                    "initial_p",
                    "the flash split the feed and reported no vapour fraction",
                )
            })?;
            let vapour = phase_properties(
                mixture,
                ideal_gas,
                &reduced,
                temperature,
                pressure,
                &flash.y,
                flash.z_vapour,
            )?;
            let liquid = phase_properties(
                mixture,
                ideal_gas,
                &reduced,
                temperature,
                pressure,
                &flash.x,
                flash.z_liquid,
            )?;
            let mass = beta * vapour.molar_mass + (1.0 - beta) * liquid.molar_mass;
            let volume = beta * vapour.molar_volume + (1.0 - beta) * liquid.molar_volume;
            let cp = beta * vapour.cp + (1.0 - beta) * liquid.cp;
            let cv = beta * vapour.cv + (1.0 - beta) * liquid.cv;
            Ok((mass / volume, cp / cv))
        }
        Phase::AllVapour | Phase::AllLiquid => {
            let side = if matches!(flash.phase, Phase::AllVapour) {
                RootSide::Vapour
            } else {
                RootSide::Liquid
            };
            let root = mixture.phase_state(&reduced, z, side)?.z;
            let phase =
                phase_properties(mixture, ideal_gas, &reduced, temperature, pressure, z, root)?;
            Ok((phase.molar_mass / phase.molar_volume, phase.cp / phase.cv))
        }
        Phase::Trivial => Err(AzothError::invalid_input(
            "initial_p",
            "the flash found a trivial solution at the initial state",
        )),
    }
}

/// One phase's molar mass, corrected molar volume and capacities.
#[allow(clippy::too_many_arguments)] // one parameter per property the flash exposes
fn phase_properties(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    reduced: &ReducedParameters,
    temperature: ThermodynamicTemperature,
    pressure: Pressure,
    composition: &[f64],
    root: f64,
) -> Result<PhaseProperties> {
    let molar_mass = molar_mass_of(mixture, composition)?;
    let shift = mixture.volume_shift(composition);
    let molar_volume = root * MOLAR_GAS_CONSTANT * temperature.value / pressure.value - shift;
    if !molar_volume.is_finite() || molar_volume <= 0.0 {
        return Err(AzothError::invalid_input(
            "initial_p",
            format!("the corrected molar volume is {molar_volume} m3/mol"),
        ));
    }
    let capacity = phase_capacity(
        mixture,
        ideal_gas,
        reduced,
        temperature,
        pressure,
        composition,
        root,
    )?;
    Ok(PhaseProperties {
        molar_mass,
        molar_volume,
        cp: capacity.cp,
        cv: capacity.cv,
    })
}

/// The peak vessel pressure over the class's ideal-gas PSV-cycling march.
#[allow(clippy::too_many_arguments)] // one parameter per study dimension the march reads
fn peak_pressure(
    area: f64,
    set_pressure: f64,
    blowdown_fraction: f64,
    discharge_coefficient: f64,
    back_pressure: f64,
    fire_heat_input: f64,
    vessel_volume: f64,
    initial_pressure_fraction: f64,
    initial_temperature: f64,
    molar_mass: f64,
    gamma: f64,
    compressibility: f64,
    time_step: f64,
    max_time: f64,
) -> f64 {
    let cv_mass = (GAS_CONSTANT / molar_mass) / (gamma - 1.0);
    let cp_mass = gamma * cv_mass;
    let mut temperature = initial_temperature;
    let start = set_pressure * initial_pressure_fraction;
    let mut moles = start * vessel_volume / (compressibility * GAS_CONSTANT * temperature);
    let mut mass = moles * molar_mass;
    let mut peak = start;
    let mut time = 0.0;
    let mut open = false;
    let mut cycles = 0.0;
    while time < max_time && mass > 1.0e-6 {
        let pressure = moles * compressibility * GAS_CONSTANT * temperature / vessel_volume;
        if pressure > peak {
            peak = pressure;
        }
        let (new_open, new_cycles, mdot) = step(
            set_pressure,
            blowdown_fraction,
            discharge_coefficient,
            area,
            back_pressure,
            pressure,
            temperature,
            molar_mass,
            gamma,
            compressibility,
            open,
            cycles,
        );
        open = new_open;
        cycles = new_cycles;
        let dm = (mdot * time_step).min(mass);
        let u_old = mass * cv_mass * temperature;
        let u_new = u_old + fire_heat_input * time_step - cp_mass * temperature * dm;
        let mass_new = mass - dm;
        if mass_new <= 1.0e-9 {
            break;
        }
        temperature = u_new / (mass_new * cv_mass);
        if temperature <= 0.0 || !temperature.is_finite() {
            break;
        }
        mass = mass_new;
        moles = mass / molar_mass;
        time += time_step;
    }
    peak
}

/// The steady API 521 area and the dynamically required area for a cycling PSV.
///
/// # Errors
/// `InvalidInput` for a non-positive dimension, an invalid blowdown fraction or coefficient,
/// a non-finite input, or a feed state the flash cannot resolve.
#[allow(clippy::too_many_arguments)] // one parameter per quantity the class reads
pub fn dynamic_psv_sizing(
    components: &[String],
    initial_z: &[f64],
    initial_t: ThermodynamicTemperature,
    initial_p: Pressure,
    vessel_volume: Volume,
    fire_heat_input: Power,
    set_pressure: Pressure,
    overpressure_fraction: f64,
    back_pressure: Pressure,
    blowdown_fraction: Option<f64>,
    discharge_coefficient: Option<f64>,
    latent_heat: Option<f64>,
    time_step: Option<Time>,
    max_time: Option<Time>,
    initial_pressure_fraction: Option<f64>,
) -> Result<DynamicPsvSizingResult> {
    let spec = &model_gen::DYNAMIC_PSV_SIZING_SPEC;
    let mut warnings = Vec::new();
    let blowdown = blowdown_fraction.unwrap_or(0.1);
    let coefficient = discharge_coefficient.unwrap_or(0.975);
    let latent = latent_heat.unwrap_or(350_000.0);
    let step_seconds = time_step.map_or(0.5, |value| value.value);
    let span = max_time.map_or(1200.0, |value| value.value);
    let start_fraction = initial_pressure_fraction.unwrap_or(0.9);

    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "initial_t" => Some(initial_t.value),
            "initial_p" => Some(initial_p.value),
            "vessel_volume" => Some(vessel_volume.value),
            "fire_heat_input" => Some(fire_heat_input.value),
            "set_pressure" => Some(set_pressure.value),
            "overpressure_fraction" => Some(overpressure_fraction),
            "back_pressure" => Some(back_pressure.value),
            "blowdown_fraction" => Some(blowdown),
            "discharge_coefficient" => Some(coefficient),
            "latent_heat" => Some(latent),
            "time_step" => Some(step_seconds),
            "max_time" => Some(span),
            "initial_pressure_fraction" => Some(start_fraction),
            _ => None,
        },
        &mut warnings,
    )?;

    for (name, value) in [
        ("initial_t", initial_t.value),
        ("initial_p", initial_p.value),
        ("vessel_volume", vessel_volume.value),
        ("fire_heat_input", fire_heat_input.value),
        ("set_pressure", set_pressure.value),
        ("overpressure_fraction", overpressure_fraction),
        ("back_pressure", back_pressure.value),
        ("blowdown_fraction", blowdown),
        ("discharge_coefficient", coefficient),
        ("latent_heat", latent),
        ("time_step", step_seconds),
        ("max_time", span),
        ("initial_pressure_fraction", start_fraction),
    ] {
        if !value.is_finite() {
            return Err(AzothError::invalid_input(
                name,
                "a non-finite value is not a number this calculation publishes",
            ));
        }
    }
    if vessel_volume.value <= 0.0
        || fire_heat_input.value <= 0.0
        || set_pressure.value <= 0.0
        || overpressure_fraction <= 0.0
        || back_pressure.value <= 0.0
    {
        return Err(AzothError::invalid_input(
            "vessel_volume",
            "the class requires the volume, fire heat, pressures and overpressure to be positive",
        ));
    }
    if blowdown <= 0.0 || blowdown >= 1.0 {
        return Err(AzothError::invalid_input(
            "blowdown_fraction",
            "the class refuses a fraction outside (0, 1)",
        ));
    }
    if coefficient <= 0.0 || latent <= 0.0 || step_seconds <= 0.0 || span <= 0.0 {
        return Err(AzothError::invalid_input(
            "discharge_coefficient",
            "the coefficient, latent heat, step and horizon must be positive",
        ));
    }
    if initial_t.value <= 0.0 || initial_p.value <= 0.0 {
        return Err(AzothError::invalid_input(
            "initial_t",
            "the initial temperature and pressure must be positive",
        ));
    }

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, Cubic::Pr, None)?;
    if initial_z.len() != mixture.len() {
        return Err(AzothError::invalid_input(
            "initial_z",
            format!(
                "a mixture of {} components needs that many mole fractions, and {initial_z:?} has {}",
                mixture.len(),
                initial_z.len()
            ),
        ));
    }
    azoth_eos::phase_boundary::check_composition(initial_z, mixture.len(), "initial_z")?;
    if initial_z.iter().any(|value| !value.is_finite()) {
        return Err(AzothError::invalid_input(
            "initial_z",
            "a mole fraction must be finite",
        ));
    }

    let (density, ratio) = bulk_properties(&mixture, &ideal_gas, initial_t, initial_p, initial_z)?;
    let molar_mass = molar_mass_of(&mixture, initial_z)?;
    let gamma = if ratio <= 1.0 || !ratio.is_finite() {
        FALLBACK_GAMMA
    } else {
        ratio
    };
    let mut compressibility = if density > 0.0 && molar_mass > 0.0 {
        set_pressure.value * molar_mass / (density * GAS_CONSTANT * initial_t.value)
    } else {
        1.0
    };
    if compressibility <= 0.0 || !compressibility.is_finite() {
        compressibility = 1.0;
    }

    let allowable = set_pressure.value * (1.0 + overpressure_fraction);
    let relief_rate = fire_heat_input.value / latent;
    let steady = gas_area(
        relief_rate,
        set_pressure.value,
        overpressure_fraction,
        back_pressure.value,
        initial_t.value,
        molar_mass,
        compressibility,
        gamma,
        false,
        false,
    );
    let steady_area = steady.required_area;

    let mut low = steady_area * 1.0e-3;
    let mut high = steady_area * 3.0;
    let mut peak_high = peak_pressure(
        high,
        set_pressure.value,
        blowdown,
        coefficient,
        back_pressure.value,
        fire_heat_input.value,
        vessel_volume.value,
        start_fraction,
        initial_t.value,
        molar_mass,
        gamma,
        compressibility,
        step_seconds,
        span,
    );
    if peak_high > allowable {
        high = steady_area * 10.0;
        peak_high = peak_pressure(
            high,
            set_pressure.value,
            blowdown,
            coefficient,
            back_pressure.value,
            fire_heat_input.value,
            vessel_volume.value,
            start_fraction,
            initial_t.value,
            molar_mass,
            gamma,
            compressibility,
            step_seconds,
            span,
        );
    }
    let mut dynamic_area = high;
    let mut peak_dynamic = peak_high;
    for _ in 0..BISECTION_STEPS {
        let mid = 0.5 * (low + high);
        let peak = peak_pressure(
            mid,
            set_pressure.value,
            blowdown,
            coefficient,
            back_pressure.value,
            fire_heat_input.value,
            vessel_volume.value,
            start_fraction,
            initial_t.value,
            molar_mass,
            gamma,
            compressibility,
            step_seconds,
            span,
        );
        if peak <= allowable {
            high = mid;
            dynamic_area = mid;
            peak_dynamic = peak;
        } else {
            low = mid;
        }
        if (high - low) <= 1.0e-9 * steady_area {
            break;
        }
    }

    Ok(DynamicPsvSizingResult {
        steady_required_area: square_meters(steady_area),
        dynamic_required_area: square_meters(dynamic_area),
        oversizing_ratio: (dynamic_area > 0.0).then(|| steady_area / dynamic_area),
        allowable_pressure: pascals(allowable),
        peak_dynamic_pressure: pascals(peak_dynamic),
        steady_relief_rate: kilograms_per_second(relief_rate),
        warnings,
    })
}
