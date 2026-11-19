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
    /// The gas phase's velocity, m/s.
    pub gas_velocity: f64,
    /// The liquid phase's velocity, m/s.
    pub liquid_velocity: f64,
    /// The gas phase's area fraction.
    pub gas_area_fraction: f64,
    /// The phase area fractions' sum.
    pub phase_area_sum: f64,
    /// The phase kinetic energy minus the homogeneous kinetic energy, J/kg.
    pub kinetic_energy_error: f64,
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
        gas_velocity,
        liquid_velocity,
        gas_area_fraction,
        phase_area_sum,
        kinetic_energy_error,
    })
}
