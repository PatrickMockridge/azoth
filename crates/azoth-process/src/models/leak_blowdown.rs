//! `process.leak_blowdown` - NeqSim's legacy vessel-blowdown time loop.
//!
//! Spec: `specs/models/process/leak_blowdown.toml`. The port source is
//! `process/safety/release/LeakModel.java`, through `calculateSourceTerm(duration, timeStep)`.
//!
//! The class integrates the inventory at fixed volume and internal energy: each substep
//! removes at most one percent of the mass, takes the removed stream's enthalpy with it and
//! solves the remaining state with a VU flash. The rate is the simplified orifice, the jet
//! velocity is capped and the momentum is not. Its scalar methods are this arithmetic, not
//! separate ids; the modern `calculateReleaseFlow` path is `process.hem_release`.

use std::f64::consts::PI;

use azoth_core::units::{
    Force, Length, Mass, MassRate, Pressure, ThermodynamicTemperature, Time, Velocity, Volume,
    kelvins, kilograms, kilograms_per_second, meters, meters_per_second, newtons, pascals, seconds,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use azoth_eos::cubic::Cubic;
use azoth_eos::databank;
use azoth_eos::entropy_state::molar_mass_of;
use azoth_eos::heat_capacity::phase_capacity;
use azoth_eos::mixture::{Mixture, ReducedParameters, RootSide};
use azoth_eos::molar_enthalpy_entropy::{IdealGasModel, molar_enthalpy_entropy};
use azoth_eos::pr_molar_volume::MOLAR_GAS_CONSTANT;
use azoth_eos::pt_flash::pt_flash;
use azoth_eos::results::Phase;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};
use crate::model_gen;
use crate::models::vu_flash_route::vu_at;

/// The class's fixed gas constant, J/(kmol K).
const R: f64 = 8314.0;
/// The largest mass one substep may remove, as a fraction of the current inventory.
const SUBSTEP_MASS_FRACTION: f64 = 0.01;
/// The class's substep cap; a run that reaches it is refused rather than continued.
const MAX_SUBSTEPS: u64 = 1_000_000;
/// The receiving-pressure event's absolute bracket, Pa.
const RECEIVING_PRESSURE_FLOOR: f64 = 1e-3;
/// The receiving-pressure event's relative bracket.
const RECEIVING_PRESSURE_RELATIVE: f64 = 1e-9;
/// The closure check's relative tolerance.
const CLOSURE_TOLERANCE: f64 = 1e-7;
/// The declared-point cap; a finer request is refused rather than run for hours.
const MAX_POINTS: u32 = 100_000;
/// The velocity cap, as a multiple of the ideal-gas sound speed.
const VELOCITY_CAP: f64 = 1.2;

/// One recorded step of the blowdown.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LeakBlowdownPoint {
    /// The step's time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
    /// The simplified orifice rate at the recorded state.
    #[serde(serialize_with = "scalar")]
    pub mass_flow_rate: MassRate,
    /// The state's temperature.
    #[serde(serialize_with = "scalar")]
    pub temperature: ThermodynamicTemperature,
    /// The state's pressure.
    #[serde(serialize_with = "scalar")]
    pub pressure: Pressure,
    /// The vapour mole fraction: one, because the model requires a single gas phase.
    pub vapour_mole_fraction: f64,
    /// `mdot/(rho A Cd)`, capped at 1.2 times the ideal-gas sound speed.
    #[serde(serialize_with = "scalar")]
    pub jet_velocity: Velocity,
    /// `mdot v + (P - back) A`, unclamped; zero where the rate is zero.
    #[serde(serialize_with = "scalar")]
    pub jet_momentum: Force,
    /// The liquid Sauter mean diameter, always zero for the required single gas phase.
    #[serde(serialize_with = "scalar")]
    pub droplet_smd: Length,
}

/// One diagnostic row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LeakBlowdownDiagnostic {
    /// The stable machine code.
    pub code: String,
    /// The engineering detail.
    pub message: String,
}

/// Result of `process.leak_blowdown`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LeakBlowdownResult {
    /// `valid`, `valid_with_warnings`, `invalid` or `unsupported`.
    pub status: String,
    /// The scenario label, echoed.
    pub scenario_name: String,
    /// The opening the calculation used.
    #[serde(serialize_with = "scalar")]
    pub hole_diameter: Length,
    /// The declared point count, `ceil(duration/time_step)+1`.
    pub number_of_points: u32,
    /// The visited rows.
    pub recorded_points: u32,
    /// The recorded series.
    pub points: Vec<LeakBlowdownPoint>,
    /// The summed `mdot dt`; absent on failure.
    #[serde(serialize_with = "optional_scalar")]
    pub total_mass_released: Option<Mass>,
    /// The largest recorded rate; absent on failure.
    #[serde(serialize_with = "optional_scalar")]
    pub peak_mass_flow_rate: Option<MassRate>,
    /// The class's `totalReleased/(0.5 peakRate)`; absent on failure.
    #[serde(serialize_with = "optional_scalar")]
    pub time_to_empty: Option<Time>,
    /// The model's records, in order.
    pub diagnostics: Vec<LeakBlowdownDiagnostic>,
    /// The diagnostics' machine codes alone.
    pub diagnostic_codes: Vec<String>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// A state failure the class would throw out of the loop; the port reports it.
#[derive(Debug, Clone)]
enum BlowdownFailure {
    Flash(String),
    Density(String),
    MolarMass(String),
    Gamma(String),
    PhaseBoundary(String),
    Vu(String),
    Closure(String),
    SubstepLimit(String),
    TimestepUnrepresentable(String),
    EventFailed(String),
}

impl BlowdownFailure {
    fn code(&self) -> &'static str {
        match self {
            Self::Flash(_) => "FLASH_FAILED",
            Self::Density(_) => "DENSITY_UNAVAILABLE",
            Self::MolarMass(_) => "MOLAR_MASS_UNAVAILABLE",
            Self::Gamma(_) => "GAMMA_OUT_OF_RANGE",
            Self::PhaseBoundary(_) => "PHASE_BOUNDARY",
            Self::Vu(_) => "VU_CLOSURE_FAILED",
            Self::Closure(_) => "CLOSURE_FAILED",
            Self::SubstepLimit(_) => "SUBSTEP_LIMIT",
            Self::TimestepUnrepresentable(_) => "TIMESTEP_UNREPRESENTABLE",
            Self::EventFailed(_) => "RECEIVING_PRESSURE_EVENT_FAILED",
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::Flash(message)
            | Self::Density(message)
            | Self::MolarMass(message)
            | Self::Gamma(message)
            | Self::PhaseBoundary(message)
            | Self::Vu(message)
            | Self::Closure(message)
            | Self::SubstepLimit(message)
            | Self::TimestepUnrepresentable(message)
            | Self::EventFailed(message) => message,
        }
    }
}

/// One flashed gas state and the quantities the class reads from it.
struct State {
    temperature: f64,
    pressure: f64,
    /// The corrected total density, kg/m3.
    density: f64,
    /// The class's `Cp/Cv`.
    gamma: f64,
    /// The molar enthalpy, J/mol.
    molar_enthalpy: f64,
    /// The cubic root at the state, so `U = H - Z R T` uses the solver's volume.
    compressibility: f64,
}

impl State {
    /// The molar internal energy on the same volume the VU flash inverts, J/mol.
    fn molar_internal_energy(&self) -> f64 {
        self.molar_enthalpy - self.compressibility * MOLAR_GAS_CONSTANT * self.temperature
    }
}

/// The inventory after one substep.
struct Advanced {
    state: State,
    moles: f64,
    mass: f64,
    energy: f64,
}

/// Blow a vessel down through a circular opening.
///
/// # Errors
/// `InvalidInput` for a state or opening the class refuses at construction, a composition
/// that is not one entry per component and sums to one, a non-positive duration or step, or a
/// declared point count over the port's cap. A state the class would throw on is reported as
/// an `invalid` result rather than an error.
#[allow(clippy::too_many_arguments)] // one parameter per declared input
pub fn leak_blowdown(
    components: &[String],
    initial_z: &[f64],
    initial_t: ThermodynamicTemperature,
    initial_p: Pressure,
    duration: Time,
    vessel_volume: Option<Volume>,
    hole_diameter: Option<Length>,
    discharge_coefficient: Option<f64>,
    back_pressure: Option<Pressure>,
    time_step: Option<Time>,
    scenario_name: Option<&str>,
) -> Result<LeakBlowdownResult> {
    let spec = &model_gen::LEAK_BLOWDOWN_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "initial_t" => Some(initial_t.value),
            "initial_p" => Some(initial_p.value),
            "vessel_volume" => vessel_volume.map(|v| v.value),
            "hole_diameter" => hole_diameter.map(|d| d.value),
            "discharge_coefficient" => discharge_coefficient,
            "back_pressure" => back_pressure.map(|p| p.value),
            "duration" => Some(duration.value),
            "time_step" => time_step.map(|d| d.value),
            _ => None,
        },
        &mut warnings,
    )?;

    let volume = vessel_volume.map(|v| v.value).unwrap_or(1.0);
    let diameter = hole_diameter.unwrap_or_else(|| meters(0.01));
    let coefficient = discharge_coefficient.unwrap_or(0.62);
    let back = back_pressure.unwrap_or_else(|| pascals(101_325.0)).value;
    let span = duration;
    let step = time_step.unwrap_or_else(|| seconds(1.0));
    let scenario = scenario_name.unwrap_or("Release").to_string();

    if !volume.is_finite() || volume <= 0.0 {
        return Err(AzothError::invalid_input(
            "vessel_volume",
            format!("a vessel has a positive volume, and {volume} m3 is not"),
        ));
    }
    if !step.value.is_finite() || step.value <= 0.0 || !span.value.is_finite() || span.value <= 0.0
    {
        return Err(AzothError::invalid_input(
            "duration",
            format!(
                "a positive finite duration and step are required, and they are {} s and {} s",
                span.value, step.value
            ),
        ));
    }
    if !coefficient.is_finite() || coefficient <= 0.0 || coefficient > 1.0 {
        return Err(AzothError::invalid_input(
            "discharge_coefficient",
            format!("the effective-area factor must be in (0, 1], and {coefficient} is not"),
        ));
    }
    let declared = (span.value / step.value).ceil();
    if !declared.is_finite() || declared < 0.0 || declared > f64::from(MAX_POINTS - 1) {
        return Err(AzothError::invalid_input(
            "duration",
            format!("the declared point count is {declared}+1, and the port's cap is {MAX_POINTS}"),
        ));
    }
    let number_of_points = declared as u32 + 1;

    let names: Vec<&str> = components.iter().map(String::as_str).collect();
    let (mixture, ideal_gas) = databank::mixture_of(&names, Cubic::Pr, None)?;
    if initial_z.len() != mixture.len() {
        return Err(AzothError::invalid_input(
            "initial_z",
            format!(
                "a mixture of {} components needs that many mole fractions, and {initial_z:?} \
                 has {}",
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

    let mut diagnostics = vec![LeakBlowdownDiagnostic {
        code: "MODEL_ASSUMPTIONS".to_string(),
        message: "Rigid adiabatic vessel; equilibrium flashes; simplified orifice; no slip \
                  or friction in the jet"
            .to_string(),
    }];
    if initial_p.value <= back {
        diagnostics.push(LeakBlowdownDiagnostic {
            code: "NO_FORWARD_FLOW".to_string(),
            message: "The initial pressure is at or below the receiving pressure".to_string(),
        });
    }

    let molar_mass = match molar_mass_of(&mixture, initial_z) {
        Ok(value) => value,
        Err(error) => {
            return Ok(failure_result(
                &scenario,
                diameter,
                number_of_points,
                BlowdownFailure::MolarMass(format!("{error}")),
                warnings,
            ));
        }
    };
    let mut state = match flash_state(
        &mixture,
        &ideal_gas,
        initial_t.value,
        initial_p.value,
        initial_z,
    ) {
        Ok(state) => state,
        Err(failure) => {
            return Ok(failure_result(
                &scenario,
                diameter,
                number_of_points,
                failure,
                warnings,
            ));
        }
    };
    // The class scales the inventory by the **EOS** volume, `V / (Z R T / P)`, not by the
    // physical-properties density the rate uses. The two differ by the volume shift.
    let initial_molar_volume =
        state.compressibility * MOLAR_GAS_CONSTANT * initial_t.value / initial_p.value;
    if !initial_molar_volume.is_finite() || initial_molar_volume <= 0.0 {
        return Ok(failure_result(
            &scenario,
            diameter,
            number_of_points,
            BlowdownFailure::Density(format!(
                "the initial molar volume is {initial_molar_volume} m3/mol"
            )),
            warnings,
        ));
    }
    let mut moles = volume / initial_molar_volume;
    let initial_mass = moles * molar_mass;
    if !initial_mass.is_finite() || initial_mass <= 0.0 {
        return Ok(failure_result(
            &scenario,
            diameter,
            number_of_points,
            BlowdownFailure::Density(format!(
                "the initial inventory is {initial_mass} kg for a {volume} m3 vessel"
            )),
            warnings,
        ));
    }

    let area = PI * diameter.value * diameter.value / 4.0;
    let molar_mass_kmol = 1000.0 * molar_mass;
    let mut mass = initial_mass;
    let mut energy = moles * state.molar_internal_energy();
    let initial_energy = energy;
    let mut total = 0.0;
    let mut released_energy = 0.0;
    let mut peak: f64 = 0.0;
    let mut points: Vec<LeakBlowdownPoint> = Vec::new();
    let mut substeps: u64 = 0;
    let mut stopped = state.pressure <= back;
    let mut elapsed = 0.0;

    for index in 0..number_of_points {
        let sample_time = if index + 1 == number_of_points {
            span.value
        } else {
            f64::from(index) * step.value
        };
        while !stopped && elapsed < sample_time {
            substeps += 1;
            if substeps > MAX_SUBSTEPS {
                return Ok(failure_result(
                    &scenario,
                    diameter,
                    number_of_points,
                    BlowdownFailure::SubstepLimit(format!(
                        "the run reached the class's {MAX_SUBSTEPS} substeps"
                    )),
                    warnings,
                ));
            }
            let rate = match screening_rate(&state, molar_mass_kmol, back, area, coefficient) {
                Ok(rate) => rate,
                Err(failure) => {
                    return Ok(failure_result(
                        &scenario,
                        diameter,
                        number_of_points,
                        failure,
                        warnings,
                    ));
                }
            };
            if rate == 0.0 {
                stopped = true;
                break;
            }
            let mut sub_step = (sample_time - elapsed).min(SUBSTEP_MASS_FRACTION * mass / rate);
            if !(sub_step > 0.0) || elapsed + sub_step <= elapsed {
                return Ok(failure_result(
                    &scenario,
                    diameter,
                    number_of_points,
                    BlowdownFailure::TimestepUnrepresentable(format!(
                        "a substep of {sub_step} s at {elapsed} s is not representable"
                    )),
                    warnings,
                ));
            }
            let mut next = match advance(
                &mixture,
                &ideal_gas,
                &state,
                initial_z,
                volume,
                moles,
                mass,
                energy,
                molar_mass,
                rate * sub_step,
                state.pressure,
            ) {
                Ok(next) => next,
                Err(failure) => {
                    return Ok(failure_result(
                        &scenario,
                        diameter,
                        number_of_points,
                        failure,
                        warnings,
                    ));
                }
            };
            let tolerance = RECEIVING_PRESSURE_FLOOR.max(back * RECEIVING_PRESSURE_RELATIVE);
            if next.state.pressure < back {
                // Locate the event by withdrawing less mass, never by clamping pressure.
                let mut lower = 0.0;
                let mut upper = sub_step;
                let mut located = false;
                for _ in 0..64 {
                    sub_step = 0.5 * (lower + upper);
                    next = match advance(
                        &mixture,
                        &ideal_gas,
                        &state,
                        initial_z,
                        volume,
                        moles,
                        mass,
                        energy,
                        molar_mass,
                        rate * sub_step,
                        state.pressure,
                    ) {
                        Ok(next) => next,
                        Err(failure) => {
                            return Ok(failure_result(
                                &scenario,
                                diameter,
                                number_of_points,
                                failure,
                                warnings,
                            ));
                        }
                    };
                    let difference = next.state.pressure - back;
                    if difference >= 0.0 && difference <= tolerance {
                        located = true;
                        break;
                    }
                    if difference >= 0.0 {
                        lower = sub_step;
                    } else {
                        upper = sub_step;
                    }
                }
                if !located {
                    return Ok(failure_result(
                        &scenario,
                        diameter,
                        number_of_points,
                        BlowdownFailure::EventFailed(
                            "the receiving-pressure crossing could not be bracketed".to_string(),
                        ),
                        warnings,
                    ));
                }
                stopped = true;
            }
            let removed = rate * sub_step;
            total += removed;
            released_energy += removed * state.molar_enthalpy / molar_mass;
            elapsed += sub_step;
            state = next.state;
            moles = next.moles;
            mass = next.mass;
            energy = next.energy;
            stopped |= state.pressure <= back + tolerance;
        }
        elapsed = sample_time;
        let rate = if stopped {
            0.0
        } else {
            match screening_rate(&state, molar_mass_kmol, back, area, coefficient) {
                Ok(rate) => rate,
                Err(failure) => {
                    return Ok(failure_result(
                        &scenario,
                        diameter,
                        number_of_points,
                        failure,
                        warnings,
                    ));
                }
            }
        };
        peak = peak.max(rate);
        points.push(record_point(
            &state,
            rate,
            sample_time,
            molar_mass_kmol,
            back,
            area,
            coefficient,
        ));
    }

    let scale = initial_mass;
    if !closes(mass + total, initial_mass, scale) {
        return Ok(failure_result(
            &scenario,
            diameter,
            number_of_points,
            BlowdownFailure::Closure(format!(
                "the final inventory {mass} kg plus {total} kg released does not close on \
                 {initial_mass} kg"
            )),
            warnings,
        ));
    }
    let energy_scale = initial_energy.abs().max(released_energy.abs());
    if !closes(energy + released_energy, initial_energy, energy_scale) {
        return Ok(failure_result(
            &scenario,
            diameter,
            number_of_points,
            BlowdownFailure::Closure(format!(
                "the final energy {} J plus {released_energy} J released does not close",
                energy
            )),
            warnings,
        ));
    }

    let recorded_points = points.len() as u32;
    let time_to_empty = if peak > 0.0 {
        total / (0.5 * peak)
    } else {
        0.0
    };
    Ok(LeakBlowdownResult {
        status: "valid".to_string(),
        scenario_name: scenario,
        hole_diameter: diameter,
        number_of_points,
        recorded_points,
        points,
        total_mass_released: Some(kilograms(total)),
        peak_mass_flow_rate: Some(kilograms_per_second(peak)),
        time_to_empty: Some(seconds(time_to_empty)),
        diagnostic_codes: diagnostics.iter().map(|row| row.code.clone()).collect(),
        diagnostics,
        warnings,
    })
}

/// The class's screening rate at a state, kg/s; zero when the state is at the receiver.
fn screening_rate(
    state: &State,
    molar_mass_kmol: f64,
    back: f64,
    area: f64,
    coefficient: f64,
) -> std::result::Result<f64, BlowdownFailure> {
    let mdot = if state.pressure <= back {
        0.0
    } else {
        let critical = (2.0 / (state.gamma + 1.0)).powf(state.gamma / (state.gamma - 1.0));
        let ratio = back / state.pressure;
        if ratio <= critical {
            let factor =
                (2.0 / (state.gamma + 1.0)).powf((state.gamma + 1.0) / (2.0 * (state.gamma - 1.0)));
            coefficient
                * area
                * state.pressure
                * (state.gamma * molar_mass_kmol / (R * state.temperature)).sqrt()
                * factor
        } else {
            let drop = state.pressure - back;
            coefficient * area * (2.0 * state.density * drop).sqrt()
        }
    };
    if !mdot.is_finite() || mdot < 0.0 {
        return Err(BlowdownFailure::Flash(format!(
            "the rate at {:.6} Pa is {mdot} kg/s",
            state.pressure
        )));
    }
    Ok(mdot)
}

/// One recorded row from the state at a sample time.
fn record_point(
    state: &State,
    rate: f64,
    time: f64,
    molar_mass_kmol: f64,
    back: f64,
    area: f64,
    coefficient: f64,
) -> LeakBlowdownPoint {
    let (velocity, momentum) = if rate == 0.0 {
        (0.0, 0.0)
    } else {
        let raw = rate / (state.density * area * coefficient);
        let sound = (state.gamma * R * state.temperature / molar_mass_kmol).sqrt();
        let velocity = raw.min(VELOCITY_CAP * sound);
        (
            velocity,
            rate * velocity + (state.pressure - back) * area,
        )
    };
    LeakBlowdownPoint {
        time: seconds(time),
        mass_flow_rate: kilograms_per_second(rate),
        temperature: kelvins(state.temperature),
        pressure: pascals(state.pressure),
        vapour_mole_fraction: 1.0,
        jet_velocity: meters_per_second(velocity),
        jet_momentum: newtons(momentum),
        droplet_smd: meters(0.0),
    }
}

/// One substep: remove `removed` kg of mass and its enthalpy, then solve the VU state.
#[allow(clippy::too_many_arguments)] // one argument per quantity the advance reads
fn advance(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    before: &State,
    z: &[f64],
    volume: f64,
    moles: f64,
    mass: f64,
    energy: f64,
    molar_mass: f64,
    removed: f64,
    previous_pressure: f64,
) -> std::result::Result<Advanced, BlowdownFailure> {
    let new_moles = moles * (1.0 - removed / mass);
    if !new_moles.is_finite() || new_moles <= 0.0 {
        return Err(BlowdownFailure::Vu(format!(
            "removing {removed} kg of a {mass} kg inventory leaves {new_moles} mol"
        )));
    }
    let target_energy = energy - removed * before.molar_enthalpy / molar_mass;
    let molar_volume = volume / new_moles;
    let molar_energy = target_energy / new_moles;
    if !molar_volume.is_finite()
        || molar_volume <= 0.0
        || !molar_energy.is_finite()
        || !target_energy.is_finite()
    {
        return Err(BlowdownFailure::Vu(format!(
            "the VU target is {molar_volume} m3/mol at {molar_energy} J/mol"
        )));
    }
    let flash = vu_at(
        mixture,
        ideal_gas,
        previous_pressure,
        molar_volume,
        molar_energy,
        z,
    )
    .map_err(|error| {
        BlowdownFailure::Vu(format!(
            "the VU flash at {molar_volume} m3/mol and {molar_energy} J/mol refused: {error}"
        ))
    })?;
    let state = flash_state(mixture, ideal_gas, flash.temperature, flash.pressure, z)?;
    Ok(Advanced {
        state,
        moles: new_moles,
        mass: new_moles * molar_mass,
        energy: target_energy,
    })
}

/// Whether `actual` closes on `expected` to the class's relative tolerance.
fn closes(actual: f64, expected: f64, scale: f64) -> bool {
    actual.is_finite()
        && expected.is_finite()
        && (actual - expected).abs() <= CLOSURE_TOLERANCE * scale.abs().max(1e-10)
}

/// Flash one state and require the single gas phase the class requires.
fn flash_state(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    temperature: f64,
    pressure: f64,
    z: &[f64],
) -> std::result::Result<State, BlowdownFailure> {
    let t = kelvins(temperature);
    let p = pascals(pressure);
    let reduced = mixture
        .reduced_parameters(t, p)
        .map_err(|error| BlowdownFailure::Flash(format!("{error}")))?;
    let flash =
        pt_flash(mixture, t, p, z).map_err(|error| BlowdownFailure::Flash(format!("{error}")))?;
    if flash.phase != Phase::AllVapour {
        return Err(BlowdownFailure::PhaseBoundary(format!(
            "the flash at {pressure:.6} Pa found a non-gas state, and the class requires a \
             single gas phase"
        )));
    }
    let root = mixture
        .phase_state(&reduced, z, RootSide::Vapour)
        .map_err(|error| BlowdownFailure::Flash(format!("{error}")))?
        .z;
    let phase = phase_of(mixture, ideal_gas, &reduced, t, p, z, root)?;
    if !phase.density.is_finite() || phase.density <= 0.0 {
        return Err(BlowdownFailure::Density(format!(
            "the density at {pressure} Pa is {} kg/m3",
            phase.density
        )));
    }
    let gamma = phase.cp / phase.cv;
    if !gamma.is_finite() || gamma <= 1.0 || gamma > 2.0 {
        return Err(BlowdownFailure::Gamma(format!(
            "the capacity ratio at {pressure} Pa is {gamma}, and the port refuses one outside \
             (1, 2]"
        )));
    }
    Ok(State {
        temperature,
        pressure,
        density: phase.density,
        gamma,
        molar_enthalpy: phase.molar_enthalpy,
        compressibility: root,
    })
}

/// One phase's molar mass, corrected molar volume, density, capacities and enthalpy.
struct PhaseProperties {
    density: f64,
    cp: f64,
    cv: f64,
    molar_enthalpy: f64,
}

#[allow(clippy::too_many_arguments)] // one argument per phase input
fn phase_of(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    reduced: &ReducedParameters,
    t: ThermodynamicTemperature,
    p: Pressure,
    composition: &[f64],
    root: f64,
) -> std::result::Result<PhaseProperties, BlowdownFailure> {
    let molar_mass = molar_mass_of(mixture, composition)
        .map_err(|error| BlowdownFailure::MolarMass(format!("{error}")))?;
    let shift = mixture.volume_shift(composition);
    let molar_volume = root * MOLAR_GAS_CONSTANT * t.value / p.value - shift;
    if !molar_volume.is_finite() || molar_volume <= 0.0 {
        return Err(BlowdownFailure::Density(format!(
            "the corrected molar volume at {:.6} Pa is {molar_volume} m3/mol",
            p.value
        )));
    }
    let capacity = phase_capacity(mixture, ideal_gas, reduced, t, p, composition, root)
        .map_err(|error| BlowdownFailure::Gamma(format!("{error}")))?;
    let enthalpy = molar_enthalpy_entropy(mixture, ideal_gas, t, p, composition, root)
        .map_err(|error| BlowdownFailure::Flash(format!("{error}")))?;
    Ok(PhaseProperties {
        density: molar_mass / molar_volume,
        cp: capacity.cp,
        cv: capacity.cv,
        molar_enthalpy: enthalpy.h.value,
    })
}

/// The class's failure result: no series, no totals, one diagnostic.
fn failure_result(
    scenario: &str,
    diameter: Length,
    number_of_points: u32,
    failure: BlowdownFailure,
    warnings: Vec<Warning>,
) -> LeakBlowdownResult {
    LeakBlowdownResult {
        status: "invalid".to_string(),
        scenario_name: scenario.to_string(),
        hole_diameter: diameter,
        number_of_points,
        recorded_points: 0,
        points: Vec::new(),
        total_mass_released: None,
        peak_mass_flow_rate: None,
        time_to_empty: None,
        diagnostics: vec![LeakBlowdownDiagnostic {
            code: failure.code().to_string(),
            message: failure.message().to_string(),
        }],
        diagnostic_codes: vec![failure.code().to_string()],
        warnings,
    }
}
