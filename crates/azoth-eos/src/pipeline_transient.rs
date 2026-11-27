//! `eos.pipeline_transient` - a segmented, compressible HEM pipeline stage.
//!
//! Spec: `specs/models/eos/pipeline_transient.toml`. The conserved state per cell is the
//! component densities `c_i`, the momentum `rho u` and the total energy `rho E`. Each
//! cell's `(v, u, z)` is inverted by [`crate::vu_flash::vu_flash`]; the faces carry a
//! Rusanov flux with the frozen homogeneous wave speed; the march is explicit Euler with
//! Darcy friction and the caller's wall flux.
//!
//! This is azoth's own line, not a NeqSim `TransientPipe` port: NeqSim's class carries
//! one composition for the whole line and cannot close per component.

use azoth_core::units::{
    HeatFluxDensity, Length, Mass, MassFlux, MolarEnergy, MolarVolume, Pressure,
    ThermodynamicTemperature, Time, Volume, kelvins, pascals,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use uom::si::molar_energy::joule_per_mole;
use uom::si::molar_volume::cubic_meter_per_mole;
use uom::si::volume::cubic_meter;

use crate::entropy_state::molar_mass_of;
use crate::heat_capacity::phase_capacity;
use crate::mixture::Mixture;
use crate::molar_enthalpy_entropy::IdealGasModel;
use crate::results::{Phase, PipelineTransientPoint, PipelineTransientResult};
use crate::state_at_tp::state_at_tp;
use crate::vu_flash::vu_flash;

/// The largest number of times one step may be halved.
const MAX_HALVINGS: u32 = 40;
/// The smallest step, in seconds, the halving loop will accept before refusing.
const MIN_STEP: f64 = 1.0e-9;

/// One cell's derived state and the quantities a face flux reads.
#[derive(Debug, Clone, PartialEq)]
struct Cell {
    c: Vec<f64>,
    momentum: f64,
    energy: f64,
    pressure: f64,
    temperature: f64,
    velocity: f64,
    density: f64,
    beta: f64,
    sound_speed: f64,
    liquid_fraction: f64,
}

/// One face's flux in the +x direction.
#[derive(Debug, Clone, PartialEq)]
struct Flux {
    c: Vec<f64>,
    momentum: f64,
    energy: f64,
}

fn normalized(c: &[f64]) -> Result<Vec<f64>> {
    let total: f64 = c.iter().sum();
    if !total.is_finite() || total <= 0.0 {
        return Err(AzothError::invalid_input(
            "initial_component_density",
            "a cell holds no component",
        ));
    }
    Ok(c.iter().map(|value| value / total).collect())
}

fn derive(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    c: &[f64],
    momentum: f64,
    energy: f64,
) -> Result<Cell> {
    let total: f64 = c.iter().sum();
    if !total.is_finite() || total <= 0.0 {
        return Err(AzothError::invalid_input(
            "initial_component_density",
            "a cell holds no component",
        ));
    }
    let z = normalized(c)?;
    let molar_mass = molar_mass_of(mixture, &z)?;
    let density = total * molar_mass;
    if !density.is_finite() || density <= 0.0 {
        return Err(AzothError::invalid_input(
            "initial_component_density",
            "a cell's mass density is not positive",
        ));
    }
    let velocity = momentum / density;
    let volume = 1.0 / total;
    let specific_internal = energy / density - 0.5 * velocity * velocity;
    let molar_internal = specific_internal * molar_mass;
    if !molar_internal.is_finite() {
        return Err(AzothError::invalid_input(
            "initial_total_energy",
            "a cell's internal energy is not finite",
        ));
    }

    let flash = vu_flash(
        mixture,
        ideal_gas,
        MolarVolume::new::<cubic_meter_per_mole>(volume),
        MolarEnergy::new::<joule_per_mole>(molar_internal),
        &z,
    )?;
    let pressure = flash.pressure.value;
    let temperature = flash.temperature.value;
    let reduced = mixture.reduced_parameters(flash.temperature, flash.pressure)?;

    let (beta, cp, cv, liquid_fraction) = match flash.phase {
        Phase::TwoPhase => {
            let beta = flash.vapour_fraction.ok_or_else(|| {
                AzothError::invalid_input(
                    "vapour_fraction",
                    "a split reported no vapour fraction",
                )
            })?;
            let liquid = phase_capacity(
                mixture,
                ideal_gas,
                &reduced,
                flash.temperature,
                flash.pressure,
                &flash.x,
                flash.z_liquid,
            )?;
            let vapour = phase_capacity(
                mixture,
                ideal_gas,
                &reduced,
                flash.temperature,
                flash.pressure,
                &flash.y,
                flash.z_vapour,
            )?;
            let liquid_volume = flash.z_liquid * crate::pr_molar_volume::MOLAR_GAS_CONSTANT
                * temperature
                / pressure;
            let vapour_volume = flash.z_vapour * crate::pr_molar_volume::MOLAR_GAS_CONSTANT
                * temperature
                / pressure;
            let mixture_volume = (1.0 - beta) * liquid_volume + beta * vapour_volume;
            (
                beta,
                (1.0 - beta) * liquid.cp + beta * vapour.cp,
                (1.0 - beta) * liquid.cv + beta * vapour.cv,
                (1.0 - beta) * liquid_volume / mixture_volume,
            )
        }
        phase => {
            let root = if phase == Phase::AllLiquid {
                flash.z_liquid
            } else {
                flash.z_vapour
            };
            let capacity = phase_capacity(
                mixture,
                ideal_gas,
                &reduced,
                flash.temperature,
                flash.pressure,
                &z,
                root,
            )?;
            let beta = if phase == Phase::AllLiquid { 0.0 } else { 1.0 };
            (beta, capacity.cp, capacity.cv, 1.0 - beta)
        }
    };

    let gamma = cp / cv;
    let sound_speed = (gamma * pressure / density).max(1.0e-12).sqrt();
    Ok(Cell {
        c: c.to_vec(),
        momentum,
        energy,
        pressure,
        temperature,
        velocity,
        density,
        beta,
        sound_speed,
        liquid_fraction,
    })
}

fn ghost(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    c: &[f64],
    cell: &Cell,
    pressure: Pressure,
    temperature: Option<ThermodynamicTemperature>,
) -> Result<Cell> {
    let z = normalized(c)?;
    let t = temperature.unwrap_or_else(|| kelvins(cell.temperature));
    let state = state_at_tp(mixture, ideal_gas, t, pressure, &z, None, None)?;
    if !state.converged {
        return Err(AzothError::SolverNotConverged {
            iterations: 0,
            residual: 0.0,
            tolerance: 1.0,
        });
    }
    if !state.molar_volume.is_finite() || state.molar_volume <= 0.0 {
        return Err(AzothError::invalid_input(
            "right_boundary_pressure",
            "the ghost volume is not positive",
        ));
    }
    let velocity = cell.velocity;
    let specific_internal = state.internal_energy / state.molar_mass;
    let gamma = state.cp / state.cv;
    let beta = match state.phase {
        Phase::AllLiquid => 0.0,
        Phase::AllVapour => 1.0,
        _ => state.vapour_fraction.unwrap_or(0.0),
    };
    Ok(Cell {
        c: z.iter().map(|value| value / state.molar_volume).collect(),
        momentum: state.density * velocity,
        energy: state.density * (specific_internal + 0.5 * velocity * velocity),
        pressure: pressure.value,
        temperature: t.value,
        velocity,
        density: state.density,
        beta,
        sound_speed: (gamma * pressure.value / state.density).max(1.0e-12).sqrt(),
        liquid_fraction: 1.0 - beta,
    })
}

fn face(left: &Cell, right: &Cell) -> Flux {
    let wave =
        (left.velocity.abs() + left.sound_speed).max(right.velocity.abs() + right.sound_speed);
    let c = (0..left.c.len())
        .map(|j| {
            0.5 * (left.c[j] * left.velocity + right.c[j] * right.velocity)
                - 0.5 * wave * (right.c[j] - left.c[j])
        })
        .collect();
    let momentum = 0.5
        * ((left.momentum * left.velocity + left.pressure)
            + (right.momentum * right.velocity + right.pressure))
        - 0.5 * wave * (right.momentum - left.momentum);
    let energy = 0.5
        * (((left.energy + left.pressure) * left.velocity)
            + ((right.energy + right.pressure) * right.velocity))
        - 0.5 * wave * (right.energy - left.energy);
    Flux {
        c,
        momentum,
        energy,
    }
}

type Stepped = (Vec<Vec<f64>>, Vec<f64>, Vec<f64>, Flux, Flux);

#[allow(clippy::too_many_arguments)] // one parameter per boundary the spec declares
fn step(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    states: &[Cell],
    c: &[Vec<f64>],
    momentum: &[f64],
    energy: &[f64],
    dx: f64,
    diameter: f64,
    friction: f64,
    heat_flux: f64,
    left_boundary: &str,
    right_boundary: &str,
    left_pressure: Option<Pressure>,
    right_pressure: Option<Pressure>,
    left_temperature: Option<ThermodynamicTemperature>,
    right_temperature: Option<ThermodynamicTemperature>,
    dt: f64,
) -> Result<Stepped> {
    let count = states.len();
    let n = states[0].c.len();
    let left_flux = if left_boundary == "closed" {
        Flux {
            c: vec![0.0; n],
            momentum: states[0].pressure,
            energy: 0.0,
        }
    } else {
        let pressure = left_pressure
            .ok_or_else(|| AzothError::invalid_input("left_boundary_pressure", "missing"))?;
        let ghost = ghost(
            mixture,
            ideal_gas,
            &c[0],
            &states[0],
            pressure,
            left_temperature,
        )?;
        face(&ghost, &states[0])
    };
    let right_flux = if right_boundary == "closed" {
        Flux {
            c: vec![0.0; n],
            momentum: states[count - 1].pressure,
            energy: 0.0,
        }
    } else {
        let pressure = right_pressure
            .ok_or_else(|| AzothError::invalid_input("right_boundary_pressure", "missing"))?;
        let ghost = ghost(
            mixture,
            ideal_gas,
            &c[count - 1],
            &states[count - 1],
            pressure,
            right_temperature,
        )?;
        face(&states[count - 1], &ghost)
    };

    let mut faces = Vec::with_capacity(count + 1);
    faces.push(left_flux.clone());
    for i in 1..count {
        faces.push(face(&states[i - 1], &states[i]));
    }
    faces.push(right_flux.clone());

    let mut new_c = Vec::with_capacity(count);
    let mut new_momentum = Vec::with_capacity(count);
    let mut new_energy = Vec::with_capacity(count);
    for i in 0..count {
        let left = &faces[i];
        let right = &faces[i + 1];
        let row: Vec<f64> = (0..n)
            .map(|j| c[i][j] - dt / dx * (right.c[j] - left.c[j]))
            .collect();
        if row.iter().any(|value| *value < -1.0e-12 || !value.is_finite()) {
            return Err(AzothError::invalid_input(
                "initial_component_density",
                "a component density went negative in the step",
            ));
        }
        new_c.push(row.into_iter().map(|value| value.max(0.0)).collect());
        let cell = &states[i];
        let friction_source =
            -friction * cell.density * cell.velocity.abs() * cell.velocity / (2.0 * diameter);
        new_momentum.push(momentum[i] - dt / dx * (right.momentum - left.momentum) + dt * friction_source);
        let heat_source = 4.0 * heat_flux / diameter;
        let updated = energy[i] - dt / dx * (right.energy - left.energy) + dt * heat_source;
        if !updated.is_finite() {
            return Err(AzothError::invalid_input(
                "initial_total_energy",
                "the total energy is not finite",
            ));
        }
        new_energy.push(updated);
    }
    Ok((new_c, new_momentum, new_energy, left_flux, right_flux))
}

fn row(
    time: f64,
    mass: f64,
    first: &Cell,
    last: &Cell,
    states: &[Cell],
    area: f64,
    dx: f64,
    left_boundary: &str,
    right_boundary: &str,
    left_pressure: Option<Pressure>,
    right_pressure: Option<Pressure>,
) -> PipelineTransientPoint {
    let p_in = if left_boundary == "constant_pressure" {
        left_pressure.map_or(first.pressure, |p| p.value)
    } else {
        first.pressure
    };
    let p_out = if right_boundary == "constant_pressure" {
        right_pressure.map_or(last.pressure, |p| p.value)
    } else {
        last.pressure
    };
    let liquid = states
        .iter()
        .map(|cell| cell.liquid_fraction)
        .sum::<f64>()
        * area
        * dx;
    PipelineTransientPoint {
        time: Time::new::<uom::si::time::second>(time),
        mass: Mass::new::<uom::si::mass::kilogram>(mass),
        p_in: pascals(p_in),
        p_out: pascals(p_out),
        t_out: kelvins(last.temperature),
        beta_out: last.beta,
        liquid_inventory: Volume::new::<cubic_meter>(liquid),
    }
}

/// Advance one stage of a segmented compressible line.
///
/// # Errors
/// Propagates the per-cell flash and state failures; refuses a requested step above the
/// initial CFL limit, a malformed profile, and a stage whose step cannot be halved far
/// enough to advance.
#[allow(clippy::too_many_arguments)] // one parameter per input the spec declares
pub fn pipeline_transient(
    mixture: &Mixture,
    ideal_gas: &IdealGasModel,
    length: Length,
    diameter: Length,
    cells: f64,
    friction_factor: f64,
    wall_heat_flux: Option<HeatFluxDensity>,
    left_boundary: &str,
    right_boundary: &str,
    left_boundary_pressure: Option<Pressure>,
    right_boundary_pressure: Option<Pressure>,
    left_boundary_temperature: Option<ThermodynamicTemperature>,
    right_boundary_temperature: Option<ThermodynamicTemperature>,
    initial_component_density: &[Vec<f64>],
    initial_momentum: &[MassFlux],
    initial_total_energy: &[Pressure],
    time_step: Time,
    duration: Time,
    cfl_number: Option<f64>,
) -> Result<PipelineTransientResult> {
    let spec = &crate::model_gen::PIPELINE_TRANSIENT_SPEC;
    let mut warnings: Vec<Warning> = Vec::new();
    let requested = cells;
    let count = requested.round();
    if (requested - count).abs() > 1.0e-9 {
        return Err(AzothError::invalid_input(
            "cells",
            format!("`cells` must be a whole number, got {requested}"),
        ));
    }
    let count = count as usize;
    let cfl = cfl_number.unwrap_or(0.5);
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "length" => Some(length.value),
            "diameter" => Some(diameter.value),
            "cells" => Some(requested),
            "time_step" => Some(time_step.value),
            "duration" => Some(duration.value),
            "cfl_number" => Some(cfl),
            _ => None,
        },
        &mut warnings,
    )?;
    if count < 2 {
        return Err(AzothError::out_of_range(
            "cells",
            requested,
            "a finite-volume line needs at least two cells",
        ));
    }
    for (name, boundary) in [
        ("left_boundary", left_boundary),
        ("right_boundary", right_boundary),
    ] {
        if boundary != "closed" && boundary != "constant_pressure" {
            return Err(AzothError::invalid_input(
                name,
                format!("unknown boundary `{boundary}`"),
            ));
        }
    }
    if left_boundary == "constant_pressure" && left_boundary_pressure.is_none() {
        return Err(AzothError::invalid_input(
            "left_boundary_pressure",
            "a constant-pressure inlet needs a pressure",
        ));
    }
    if right_boundary == "constant_pressure" && right_boundary_pressure.is_none() {
        return Err(AzothError::invalid_input(
            "right_boundary_pressure",
            "a constant-pressure outlet needs a pressure",
        ));
    }
    let n = mixture.len();
    if initial_component_density.len() != count {
        return Err(AzothError::invalid_input(
            "initial_component_density",
            format!(
                "{count} cells need {count} rows, got {}",
                initial_component_density.len()
            ),
        ));
    }
    for row in initial_component_density {
        if row.len() != n {
            return Err(AzothError::invalid_input(
                "initial_component_density",
                format!("a row needs one entry per component ({n}), got {}", row.len()),
            ));
        }
    }
    if initial_momentum.len() != count || initial_total_energy.len() != count {
        return Err(AzothError::invalid_input(
            "initial_momentum",
            format!("{count} cells need {count} values"),
        ));
    }

    let dx = length.value / count as f64;
    let area = std::f64::consts::PI * diameter.value * diameter.value / 4.0;
    let heat_flux = wall_heat_flux.map_or(0.0, |value| value.value);
    let mut c: Vec<Vec<f64>> = initial_component_density.to_vec();
    let mut momentum: Vec<f64> = initial_momentum.iter().map(|value| value.value).collect();
    let mut energy: Vec<f64> = initial_total_energy.iter().map(|value| value.value).collect();
    let mut states: Vec<Cell> = (0..count)
        .map(|i| derive(mixture, ideal_gas, &c[i], momentum[i], energy[i]))
        .collect::<Result<Vec<_>>>()?;
    let fastest = states
        .iter()
        .map(|cell| cell.velocity.abs() + cell.sound_speed)
        .fold(0.0_f64, f64::max);
    if time_step.value > cfl * dx / fastest {
        return Err(AzothError::out_of_range(
            "time_step",
            time_step.value,
            format!("exceeds the CFL limit {:.6} s for the initial state", cfl * dx / fastest),
        ));
    }

    let holdup = |c: &[Vec<f64>]| -> (Vec<f64>, f64) {
        let component: Vec<f64> = (0..n)
            .map(|j| c.iter().map(|row| row[j]).sum::<f64>() * area * dx)
            .collect();
        let masses: Vec<f64> = mixture
            .components()
            .iter()
            .map(|component| component.molar_mass.unwrap_or(0.0))
            .collect();
        let mass = (0..n).map(|j| component[j] * masses[j]).sum();
        (component, mass)
    };

    let (initial_component, initial_mass) = holdup(&c);
    let mut boundary_component = vec![0.0; n];
    let mut boundary_mass = 0.0;
    let mut history = vec![row(
        0.0,
        initial_mass,
        &states[0],
        &states[count - 1],
        &states,
        area,
        dx,
        left_boundary,
        right_boundary,
        left_boundary_pressure,
        right_boundary_pressure,
    )];
    let mut elapsed = 0.0;
    let mut steps = 0u32;
    let mut rejections = 0u32;
    let mut smallest = time_step.value;

    while elapsed < duration.value - 1.0e-12 {
        let mut dt = time_step.value.min(duration.value - elapsed);
        let mut accepted = false;
        for _attempt in 0..=MAX_HALVINGS {
            let stepped = step(
                mixture,
                ideal_gas,
                &states,
                &c,
                &momentum,
                &energy,
                dx,
                diameter.value,
                friction_factor,
                heat_flux,
                left_boundary,
                right_boundary,
                left_boundary_pressure,
                right_boundary_pressure,
                left_boundary_temperature,
                right_boundary_temperature,
                dt,
            );
            let Ok((new_c, new_momentum, new_energy, left_flux, right_flux)) = stepped else {
                dt *= 0.5;
                rejections += 1;
                smallest = smallest.min(dt);
                if dt < MIN_STEP {
                    return Err(AzothError::SolverNotConverged {
                        iterations: steps,
                        residual: dt,
                        tolerance: MIN_STEP,
                    });
                }
                continue;
            };
            let derived = (0..count)
                .map(|i| derive(mixture, ideal_gas, &new_c[i], new_momentum[i], new_energy[i]))
                .collect::<Result<Vec<_>>>();
            let Ok(new_states) = derived else {
                dt *= 0.5;
                rejections += 1;
                smallest = smallest.min(dt);
                if dt < MIN_STEP {
                    return Err(AzothError::SolverNotConverged {
                        iterations: steps,
                        residual: dt,
                        tolerance: MIN_STEP,
                    });
                }
                continue;
            };

            elapsed += dt;
            smallest = smallest.min(dt);
            c = new_c;
            momentum = new_momentum;
            energy = new_energy;
            states = new_states;
            steps += 1;
            for j in 0..n {
                boundary_component[j] += (right_flux.c[j] - left_flux.c[j]) * area * dt;
            }
            let masses: Vec<f64> = mixture
                .components()
                .iter()
                .map(|component| component.molar_mass.unwrap_or(0.0))
                .collect();
            let right_mass_flux: f64 = (0..n)
                .map(|j| right_flux.c[j] * masses[j])
                .sum();
            let left_mass_flux: f64 = (0..n).map(|j| left_flux.c[j] * masses[j]).sum();
            boundary_mass += (right_mass_flux - left_mass_flux) * area * dt;
            let (_, current_mass) = holdup(&c);
            history.push(row(
                elapsed,
                current_mass,
                &states[0],
                &states[count - 1],
                &states,
                area,
                dx,
                left_boundary,
                right_boundary,
                left_boundary_pressure,
                right_boundary_pressure,
            ));
            accepted = true;
            break;
        }
        if !accepted {
            return Err(AzothError::SolverNotConverged {
                iterations: steps,
                residual: dt,
                tolerance: MIN_STEP,
            });
        }
    }

    let (final_component, final_mass) = holdup(&c);
    Ok(PipelineTransientResult {
        final_component_density: c,
        final_velocity: states.iter().map(|cell| cell.velocity).collect(),
        final_pressure: states.iter().map(|cell| cell.pressure).collect(),
        final_temperature: states.iter().map(|cell| cell.temperature).collect(),
        final_vapour_fraction: states.iter().map(|cell| cell.beta).collect(),
        final_density: states.iter().map(|cell| cell.density).collect(),
        final_liquid_volume_fraction: states.iter().map(|cell| cell.liquid_fraction).collect(),
        boundary_component_outflow: boundary_component,
        initial_component_holdup: initial_component,
        final_component_holdup: final_component,
        boundary_mass_outflow: boundary_mass,
        initial_mass,
        final_mass,
        steps,
        rejections,
        min_time_step: smallest,
        history,
        warnings,
    })
}
