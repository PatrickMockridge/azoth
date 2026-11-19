//! Shared arithmetic for the derived release source-term models.
//!
//! The slip, drift-flux and finite-rate models all post-process the equilibrium stations of
//! `process.hem_release`. This module holds the basis they share: the two-phase check and
//! the prescribed-slip partition. **The equilibrium scan itself stays in `hem_release`.**

use crate::models::hem_release::TraceStation;

/// A failure a derived release model reports inside its result.
#[derive(Debug, Clone)]
pub(crate) enum ReleaseRefusal {
    /// The requested regime is outside the model.
    Unsupported(String),
    /// A required numerical or closure check failed.
    Invalid(String),
}

/// The resolved gas and liquid basis of one station.
pub(crate) struct PhaseBasis {
    /// The gas phase's share of the mass.
    pub gas_fraction: f64,
    /// The liquid phase's share of the mass.
    pub liquid_fraction: f64,
    /// The gas phase's density, kg/m3.
    pub gas_density: f64,
    /// The liquid phase's density, kg/m3.
    pub liquid_density: f64,
}

/// The gas and liquid basis, or the class's refusal.
pub(crate) fn phase_basis(state: &TraceStation) -> Result<PhaseBasis, ReleaseRefusal> {
    let (gas, liquid) = match (state.gas_phase.as_ref(), state.liquid_phase.as_ref()) {
        (Some(gas), Some(liquid)) => (gas, liquid),
        _ => {
            return Err(ReleaseRefusal::Unsupported(
                "Exactly one gas and one liquid phase are required at the opening".to_string(),
            ));
        }
    };
    let gas_fraction = state.gas_mass_fraction;
    let liquid_fraction = 1.0 - gas_fraction;
    if gas_fraction <= 1.0e-10 || liquid_fraction <= 1.0e-10 {
        return Err(ReleaseRefusal::Unsupported(
            "Both phases must have positive resolved mass fraction".to_string(),
        ));
    }
    if !gas.density.is_finite()
        || gas.density <= 0.0
        || !liquid.density.is_finite()
        || liquid.density <= 0.0
    {
        return Err(ReleaseRefusal::Invalid(
            "Phase density and mass-fraction bases do not match".to_string(),
        ));
    }
    Ok(PhaseBasis {
        gas_fraction,
        liquid_fraction,
        gas_density: gas.density,
        liquid_density: liquid.density,
    })
}

/// One station's slip hydrodynamics.
pub(crate) struct SlipOutcome {
    /// The station with its bulk and phase velocities replaced.
    pub state: TraceStation,
    /// The closure's mass flux, kg/(m2 s).
    pub mass_flux: f64,
}

/// The prescribed-slip partition of one station.
///
/// # Errors
/// The class's unsupported phase basis and its phase-area or kinetic-energy closure failures.
pub(crate) fn prescribed_slip(
    state: &TraceStation,
    gas_to_liquid_slip_ratio: f64,
) -> Result<SlipOutcome, ReleaseRefusal> {
    let basis = phase_basis(state)?;
    let homogeneous_velocity = state.velocity;
    if !homogeneous_velocity.is_finite() || homogeneous_velocity <= 0.0 {
        return Err(ReleaseRefusal::Invalid(
            "homogeneous throat velocity must be positive".to_string(),
        ));
    }
    let ratio_squared = gas_to_liquid_slip_ratio * gas_to_liquid_slip_ratio;
    let denominator = basis.gas_fraction * ratio_squared + basis.liquid_fraction;
    let liquid_velocity = homogeneous_velocity / denominator.sqrt();
    let gas_velocity = gas_to_liquid_slip_ratio * liquid_velocity;
    let specific_area = basis.gas_fraction / (basis.gas_density * gas_velocity)
        + basis.liquid_fraction / (basis.liquid_density * liquid_velocity);
    let mass_flux = 1.0 / specific_area;
    let gas_area_fraction = mass_flux * basis.gas_fraction / (basis.gas_density * gas_velocity);
    let liquid_area_fraction =
        mass_flux * basis.liquid_fraction / (basis.liquid_density * liquid_velocity);
    let phase_area_sum = gas_area_fraction + liquid_area_fraction;
    let phase_kinetic_energy = 0.5
        * (basis.gas_fraction * gas_velocity * gas_velocity
            + basis.liquid_fraction * liquid_velocity * liquid_velocity);
    let homogeneous_kinetic_energy = 0.5 * homogeneous_velocity * homogeneous_velocity;
    let kinetic_energy_error = phase_kinetic_energy - homogeneous_kinetic_energy;
    if !mass_flux.is_finite()
        || mass_flux <= 0.0
        || (phase_area_sum - 1.0).abs() > 1.0e-10
        || kinetic_energy_error.abs() > 1.0e-9 * 1.0_f64.max(homogeneous_kinetic_energy)
    {
        return Err(ReleaseRefusal::Invalid(
            "Phase area or kinetic-energy closure failed".to_string(),
        ));
    }
    let bulk_velocity = mass_flux / state.density;
    let mut partitioned = state.clone();
    partitioned.velocity = bulk_velocity;
    partitioned.mass_flux = state.density * bulk_velocity;
    partitioned.gas_velocity = Some(gas_velocity);
    partitioned.liquid_velocity = Some(liquid_velocity);
    Ok(SlipOutcome {
        state: partitioned,
        mass_flux,
    })
}

/// The class's distribution parameter.
pub(crate) const DRIFT_DISTRIBUTION_PARAMETER: f64 = 1.2;
/// The class's Harmathy coefficient.
pub(crate) const HARMATHY_COEFFICIENT: f64 = 1.53;
/// The class's bounded bubbly/dispersed gas area fraction.
pub(crate) const MAX_GAS_AREA_FRACTION: f64 = 0.80;
/// The class's largest bracketed slip ratio.
pub(crate) const MAX_SLIP_RATIO: f64 = 128.0;
/// The closure's bisection cap.
const DRIFT_BISECTIONS: usize = 80;
/// The closure's gravity.
const GRAVITY: f64 = 9.80665;

/// One station's drift-flux hydrodynamics.
pub(crate) struct DriftOutcome {
    /// The station with its bulk and phase velocities replaced.
    pub state: TraceStation,
    /// The closure's mass flux, kg/(m2 s).
    pub mass_flux: f64,
    /// The gas phase's velocity, m/s.
    pub gas_velocity: f64,
    /// The liquid phase's velocity, m/s.
    pub liquid_velocity: f64,
}

/// One closure evaluation.
struct DriftRaw {
    mass_flux: f64,
    gas_velocity: f64,
    liquid_velocity: f64,
    gas_area_fraction: f64,
    drift_residual: f64,
}

/// The drift-flux closure at one slip ratio.
fn drift_raw(
    state: &TraceStation,
    basis: &PhaseBasis,
    slip_ratio: f64,
    drift_velocity: f64,
) -> Result<DriftRaw, ReleaseRefusal> {
    let homogeneous_velocity = state.velocity;
    if !homogeneous_velocity.is_finite() || homogeneous_velocity <= 0.0 {
        return Err(ReleaseRefusal::Invalid(
            "homogeneous throat velocity must be positive".to_string(),
        ));
    }
    let denominator = basis.gas_fraction * slip_ratio * slip_ratio + basis.liquid_fraction;
    let liquid_velocity = homogeneous_velocity / denominator.sqrt();
    let gas_velocity = slip_ratio * liquid_velocity;
    let specific_area = basis.gas_fraction / (basis.gas_density * gas_velocity)
        + basis.liquid_fraction / (basis.liquid_density * liquid_velocity);
    let mass_flux = 1.0 / specific_area;
    let gas_area_fraction = mass_flux * basis.gas_fraction / (basis.gas_density * gas_velocity);
    let liquid_area_fraction =
        mass_flux * basis.liquid_fraction / (basis.liquid_density * liquid_velocity);
    let phase_area_sum = gas_area_fraction + liquid_area_fraction;
    let volumetric_flux = mass_flux
        * (basis.gas_fraction / basis.gas_density + basis.liquid_fraction / basis.liquid_density);
    let drift_residual =
        gas_velocity - DRIFT_DISTRIBUTION_PARAMETER * volumetric_flux - drift_velocity;
    let phase_kinetic_energy = 0.5
        * (basis.gas_fraction * gas_velocity * gas_velocity
            + basis.liquid_fraction * liquid_velocity * liquid_velocity);
    let homogeneous_kinetic_energy = 0.5 * homogeneous_velocity * homogeneous_velocity;
    let kinetic_energy_error = phase_kinetic_energy - homogeneous_kinetic_energy;
    if !mass_flux.is_finite()
        || mass_flux <= 0.0
        || !drift_residual.is_finite()
        || (phase_area_sum - 1.0).abs() > 1.0e-10
        || kinetic_energy_error.abs() > 1.0e-9 * 1.0_f64.max(homogeneous_kinetic_energy)
    {
        return Err(ReleaseRefusal::Invalid(
            "Phase area, drift-flux or kinetic-energy closure failed".to_string(),
        ));
    }
    Ok(DriftRaw {
        mass_flux,
        gas_velocity,
        liquid_velocity,
        gas_area_fraction,
        drift_residual,
    })
}

/// The drift-flux partition of one station, with the shared closure solved.
///
/// # Errors
/// The class's unsupported phase basis, its bracket and closure failures, and its gas-area
/// cap.
pub(crate) fn drift_flux(
    state: &TraceStation,
    surface_tension: f64,
) -> Result<DriftOutcome, ReleaseRefusal> {
    let basis = phase_basis(state)?;
    if basis.liquid_density <= basis.gas_density {
        return Err(ReleaseRefusal::Unsupported(
            "Harmathy closure requires liquid density greater than gas density".to_string(),
        ));
    }
    let drift_velocity = HARMATHY_COEFFICIENT
        * (GRAVITY * surface_tension * (basis.liquid_density - basis.gas_density)
            / (basis.liquid_density * basis.liquid_density))
            .powf(0.25);
    if !drift_velocity.is_finite() || drift_velocity <= 0.0 {
        return Err(ReleaseRefusal::Invalid(
            "Harmathy drift velocity must be positive".to_string(),
        ));
    }
    let mut lower = 1.0_f64;
    let lower_residual = drift_raw(state, &basis, lower, drift_velocity)?.drift_residual;
    let mut upper = 2.0_f64;
    let mut upper_residual = drift_raw(state, &basis, upper, drift_velocity)?.drift_residual;
    while upper_residual <= 0.0 && upper < MAX_SLIP_RATIO {
        upper *= 2.0;
        upper_residual = drift_raw(state, &basis, upper, drift_velocity)?.drift_residual;
    }
    if !(lower_residual < 0.0) || !(upper_residual > 0.0) {
        return Err(ReleaseRefusal::Invalid(
            "Unable to bracket positive drift-flux slip solution".to_string(),
        ));
    }
    for _ in 0..DRIFT_BISECTIONS {
        let middle = 0.5 * (lower + upper);
        let middle_residual = drift_raw(state, &basis, middle, drift_velocity)?.drift_residual;
        if middle_residual > 0.0 {
            upper = middle;
        } else {
            lower = middle;
        }
    }
    let slip_ratio = 0.5 * (lower + upper);
    let raw = drift_raw(state, &basis, slip_ratio, drift_velocity)?;
    if raw.gas_area_fraction > MAX_GAS_AREA_FRACTION {
        return Err(ReleaseRefusal::Unsupported(format!(
            "Predicted gas area fraction {:.6} exceeds the bounded bubbly/dispersed screening \
             limit {}",
            raw.gas_area_fraction, MAX_GAS_AREA_FRACTION
        )));
    }
    let bulk_velocity = raw.mass_flux / state.density;
    let mut partitioned = state.clone();
    partitioned.velocity = bulk_velocity;
    partitioned.mass_flux = state.density * bulk_velocity;
    partitioned.gas_velocity = Some(raw.gas_velocity);
    partitioned.liquid_velocity = Some(raw.liquid_velocity);
    Ok(DriftOutcome {
        state: partitioned,
        mass_flux: raw.mass_flux,
        gas_velocity: raw.gas_velocity,
        liquid_velocity: raw.liquid_velocity,
    })
}

/// The uniform first-order relaxation of the equilibrium station toward equilibrium.
///
/// # Errors
/// The class's exactly-two-phase requirement and its relaxed-fraction bounds.
pub(crate) fn relax_uniform(
    upstream_gas_fraction: f64,
    equilibrium: &TraceStation,
    progress: f64,
) -> Result<TraceStation, ReleaseRefusal> {
    let basis = phase_basis(equilibrium)?;
    let relaxed = upstream_gas_fraction + progress * (basis.gas_fraction - upstream_gas_fraction);
    if relaxed <= 1.0e-10 || relaxed >= 1.0 - 1.0e-10 {
        return Err(ReleaseRefusal::Unsupported(
            "Relaxed state must retain positive gas and liquid mass fractions".to_string(),
        ));
    }
    let density = 1.0 / (relaxed / basis.gas_density + (1.0 - relaxed) / basis.liquid_density);
    let mut out = equilibrium.clone();
    out.density = density;
    out.gas_mass_fraction = relaxed;
    if let Some(gas) = out.gas_phase.as_mut() {
        gas.mass_fraction = relaxed;
    }
    if let Some(liquid) = out.liquid_phase.as_mut() {
        liquid.mass_fraction = 1.0 - relaxed;
    }
    Ok(out)
}

/// The component-selective first-order relaxation of the equilibrium station.
///
/// `times` is one relaxation time per component, in mixture order; `overall` is the upstream
/// overall component mass fractions.
///
/// # Errors
/// The class's exactly-two-phase requirement, its per-component inventory bounds and its
/// relaxed-fraction bounds.
pub(crate) fn relax_components(
    upstream: &TraceStation,
    equilibrium: &TraceStation,
    times: &[f64],
    residence: f64,
    overall: &[f64],
) -> Result<TraceStation, ReleaseRefusal> {
    let basis = phase_basis(equilibrium)?;
    let (Some(gas_phase), Some(liquid_phase)) = (
        equilibrium.gas_phase.as_ref(),
        equilibrium.liquid_phase.as_ref(),
    ) else {
        return Err(ReleaseRefusal::Unsupported(
            "Exactly one gas and one liquid phase are required at the opening".to_string(),
        ));
    };
    let upstream_gas = upstream.gas_phase.as_ref();
    if times.len() != overall.len()
        || gas_phase.component_mass_fractions.len() != overall.len()
        || liquid_phase.component_mass_fractions.len() != overall.len()
    {
        return Err(ReleaseRefusal::Unsupported(
            "Relaxation-time components must exactly match the fluid components".to_string(),
        ));
    }
    let mut held = Vec::with_capacity(overall.len());
    let mut gas_mass_fraction = 0.0;
    for index in 0..overall.len() {
        let initial = upstream_gas.map_or(0.0, |gas| {
            gas.mass_fraction * gas.component_mass_fractions[index]
        });
        let target = basis.gas_fraction * gas_phase.component_mass_fractions[index];
        let progress = 1.0 - (-residence / times[index]).exp();
        let relaxed = initial + progress * (target - initial);
        if !relaxed.is_finite() || relaxed < -1.0e-14 || relaxed > overall[index] + 1.0e-14 {
            return Err(ReleaseRefusal::Invalid(
                "Component phase allocation is outside its conserved inventory".to_string(),
            ));
        }
        let relaxed = relaxed.clamp(0.0, overall[index]);
        gas_mass_fraction += relaxed;
        held.push(relaxed);
    }
    if gas_mass_fraction <= 1.0e-10 || gas_mass_fraction >= 1.0 - 1.0e-10 {
        return Err(ReleaseRefusal::Unsupported(
            "Relaxed state must retain positive gas and liquid mass fractions".to_string(),
        ));
    }
    let liquid_mass_fraction = 1.0 - gas_mass_fraction;
    let mut gas_composition = Vec::with_capacity(overall.len());
    let mut liquid_composition = Vec::with_capacity(overall.len());
    for index in 0..overall.len() {
        gas_composition.push(held[index] / gas_mass_fraction);
        liquid_composition.push((overall[index] - held[index]) / liquid_mass_fraction);
    }
    let density =
        1.0 / (gas_mass_fraction / basis.gas_density + liquid_mass_fraction / basis.liquid_density);
    let mut out = equilibrium.clone();
    out.density = density;
    out.gas_mass_fraction = gas_mass_fraction;
    if let Some(gas) = out.gas_phase.as_mut() {
        gas.mass_fraction = gas_mass_fraction;
        gas.component_mass_fractions = gas_composition;
    }
    if let Some(liquid) = out.liquid_phase.as_mut() {
        liquid.mass_fraction = liquid_mass_fraction;
        liquid.component_mass_fractions = liquid_composition;
    }
    Ok(out)
}

/// The reconstructed overall component mass fractions of a station.
pub(crate) fn reconstructed_components(state: &TraceStation) -> Option<Vec<f64>> {
    let (gas, liquid) = (state.gas_phase.as_ref()?, state.liquid_phase.as_ref()?);
    let mut out = Vec::with_capacity(gas.component_mass_fractions.len());
    for index in 0..gas.component_mass_fractions.len() {
        out.push(
            gas.mass_fraction * gas.component_mass_fractions[index]
                + liquid.mass_fraction * liquid.component_mass_fractions[index],
        );
    }
    Some(out)
}
