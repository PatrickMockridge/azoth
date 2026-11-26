//! `pvt.cvd_material_balance` - the constant-volume-depletion accounting.
//!
//! Spec: `specs/models/pvt/cvd_material_balance.toml`. The arithmetic is
//! `pvtsimulation/util/CvdMaterialBalance.java` at the pin: the vapor inventory from the
//! pressure and Z ratios, the named produced/remaining/liquid component amounts, the four
//! closures and the liquid-state resolution. The class's cell temperature and its
//! sample/experiment/data-use references are read by no formula and are not inputs.

use azoth_core::{AzothError, Result, Warning};

/// The composition tolerance the class applies to every vapor row.
const COMPOSITION_TOLERANCE: f64 = 1.0e-10;

/// One observation at the id boundary: SI magnitudes, in the declared component order.
#[derive(Debug, Clone, PartialEq)]
pub struct CvdObservation {
    /// The post-withdrawal cell pressure, in Pa.
    pub pressure: f64,
    /// The equilibrium vapor compressibility factor.
    pub gas_z: f64,
    /// The liquid volume per initial saturated cell volume, in `[0, 1)`.
    pub liquid_volume_fraction: f64,
    /// The cumulative withdrawn moles per initial mole, in `[0, 1)`.
    pub cumulative_produced: f64,
}

/// The liquid resolution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidState {
    /// Zero liquid volume and no resolved liquid inventory.
    Absent,
    /// Positive liquid volume and amount above the resolution floor.
    Resolved,
    /// Positive liquid volume but insufficient liquid moles to resolve composition.
    Unresolved,
    /// Contradictory phase amounts or component inventory.
    Inconsistent,
}

impl LiquidState {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Resolved => "resolved",
            Self::Unresolved => "unresolved",
            Self::Inconsistent => "inconsistent",
        }
    }
}

/// One reconstructed stage.
#[derive(Debug, Clone, PartialEq)]
pub struct CvdStage {
    /// The total moles remaining per initial mole.
    pub total_moles: f64,
    /// The vapor moles per initial mole.
    pub vapor_moles: f64,
    /// The liquid moles per initial mole.
    pub liquid_moles: f64,
    /// The moles withdrawn at this stage per initial mole.
    pub stage_withdrawn_moles: f64,
    /// The component sum minus the liquid amount.
    pub component_closure: f64,
    /// The remaining component sum minus the total moles.
    pub remaining_closure: f64,
    /// The produced component sum minus the cumulative production.
    pub produced_closure: f64,
    /// The vapor amount times the composition sum, minus the vapor amount.
    pub vapor_closure: f64,
    /// The liquid resolution state.
    pub liquid_state: LiquidState,
    /// Whether this stage's accounting closed and its composition resolved.
    pub compatible: bool,
}

/// Result of `pvt.cvd_material_balance`.
#[derive(Debug, Clone, PartialEq)]
pub struct CvdMaterialBalanceResult {
    /// The reconstructed stages, in observation order.
    pub stages: Vec<CvdStage>,
    /// Produced component moles, one row per observation.
    pub produced_component_moles: Vec<Vec<f64>>,
    /// Remaining component moles, one row per observation.
    pub remaining_component_moles: Vec<Vec<f64>>,
    /// Liquid component moles, one row per observation.
    pub liquid_component_moles: Vec<Vec<f64>>,
    /// Whether every stage closed and resolved.
    pub compatible: bool,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

fn invalid(field: &str, reason: &str) -> AzothError {
    AzothError::invalid_input(field, reason)
}

/// Reconstruct a constant-volume-depletion experiment.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an empty or duplicate component set, fewer than two
///   observations, a row whose values or composition are out of range, a composition that
///   does not already sum to one, a first row that is not the all-vapor reference, a
///   pressure that does not decrease, a cumulative production that decreases, invalid
///   limits, or an unrepresentable vapor inventory.
#[allow(clippy::too_many_arguments)]
pub fn cvd_material_balance(
    component_names: &[String],
    observations: &[CvdObservation],
    vapor_compositions: &[Vec<f64>],
    balance_tolerance: f64,
    minimum_liquid_moles: f64,
) -> Result<CvdMaterialBalanceResult> {
    if component_names.is_empty() {
        return Err(invalid(
            "component_names",
            "at least one component is required",
        ));
    }
    for (index, name) in component_names.iter().enumerate() {
        if component_names[..index].contains(name) {
            return Err(invalid(
                "component_names",
                "the component identities are not unique",
            ));
        }
    }
    if observations.len() < 2 {
        return Err(invalid(
            "observations",
            "a saturation reference and one depletion row are required",
        ));
    }
    if vapor_compositions.len() != observations.len() {
        return Err(invalid(
            "vapor_compositions",
            "one composition row is required per observation",
        ));
    }
    if !balance_tolerance.is_finite() || !(0.0..1.0).contains(&balance_tolerance) {
        return Err(invalid(
            "balance_tolerance",
            "the tolerance must be finite in [0, 1)",
        ));
    }
    if !minimum_liquid_moles.is_finite()
        || minimum_liquid_moles <= balance_tolerance
        || minimum_liquid_moles > 1.0
    {
        return Err(invalid(
            "minimum_liquid_moles",
            "the resolution floor must be finite, above the tolerance and at most one",
        ));
    }

    let count = observations.len();
    let components = component_names.len();
    for (index, row) in observations.iter().enumerate() {
        if !row.pressure.is_finite()
            || row.pressure <= 0.0
            || !row.gas_z.is_finite()
            || row.gas_z <= 0.0
            || !row.liquid_volume_fraction.is_finite()
            || !(0.0..1.0).contains(&row.liquid_volume_fraction)
            || !row.cumulative_produced.is_finite()
            || !(0.0..1.0).contains(&row.cumulative_produced)
        {
            return Err(invalid(
                "observations",
                "each row needs a finite positive pressure and Z and volume/production fractions in [0, 1)",
            ));
        }
        let composition = &vapor_compositions[index];
        if composition.len() != components {
            return Err(invalid(
                "vapor_compositions",
                "each row must carry one value per component",
            ));
        }
        let mut sum = 0.0;
        for value in composition {
            if !value.is_finite() || !(0.0..=1.0).contains(value) {
                return Err(invalid(
                    "vapor_compositions",
                    "a composition value is outside [0, 1]",
                ));
            }
            sum += value;
        }
        if (sum - 1.0).abs() > COMPOSITION_TOLERANCE {
            return Err(invalid(
                "vapor_compositions",
                "a composition must already sum to one; it is never normalised",
            ));
        }
    }
    if observations[0].liquid_volume_fraction != 0.0 || observations[0].cumulative_produced != 0.0 {
        return Err(invalid(
            "observations",
            "the first row must be the all-vapor saturation reference before withdrawal",
        ));
    }
    for index in 1..count {
        if observations[index].pressure >= observations[index - 1].pressure {
            return Err(invalid("observations", "pressures must strictly decrease"));
        }
        if observations[index].cumulative_produced < observations[index - 1].cumulative_produced {
            return Err(invalid(
                "observations",
                "cumulative production must not decrease",
            ));
        }
    }

    let reference = &observations[0];
    let mut produced = vec![0.0_f64; components];
    let mut stages = Vec::with_capacity(count);
    let mut produced_rows = Vec::with_capacity(count);
    let mut remaining_rows = Vec::with_capacity(count);
    let mut liquid_rows = Vec::with_capacity(count);

    for (index, row) in observations.iter().enumerate() {
        let withdrawal = if index == 0 {
            0.0
        } else {
            row.cumulative_produced - observations[index - 1].cumulative_produced
        };
        let vapor = (row.pressure / reference.pressure)
            * (reference.gas_z / row.gas_z)
            * (1.0 - row.liquid_volume_fraction);
        let total = 1.0 - row.cumulative_produced;
        let liquid = total - vapor;
        if !vapor.is_finite() || (vapor == 0.0 && row.liquid_volume_fraction < 1.0) {
            return Err(invalid(
                "observations",
                "the vapor inventory overflows or underflows at an observation",
            ));
        }
        let composition = &vapor_compositions[index];
        let mut remaining = vec![0.0_f64; components];
        let mut liquid_components = vec![0.0_f64; components];
        let mut compatible =
            vapor >= 0.0 && vapor <= total + balance_tolerance && liquid >= -balance_tolerance;
        let mut component_sum = 0.0;
        for component in 0..components {
            produced[component] += withdrawal * composition[component];
            remaining[component] = vapor_compositions[0][component] - produced[component];
            liquid_components[component] = remaining[component] - vapor * composition[component];
            component_sum += liquid_components[component];
            compatible &= remaining[component] >= -balance_tolerance
                && liquid_components[component] >= -balance_tolerance;
        }
        let closure = component_sum - liquid;
        let remaining_closure = remaining.iter().sum::<f64>() - total;
        let produced_closure = produced.iter().sum::<f64>() - row.cumulative_produced;
        let vapor_closure = vapor * composition.iter().sum::<f64>() - vapor;
        if !closure.is_finite() || !vapor_closure.is_finite() {
            return Err(invalid(
                "observations",
                "the component accounting is unrepresentable at an observation",
            ));
        }
        compatible &= closure.abs() <= balance_tolerance
            && remaining_closure.abs() <= balance_tolerance
            && produced_closure.abs() <= balance_tolerance
            && vapor_closure.abs() <= balance_tolerance;

        let state =
            if !compatible || (row.liquid_volume_fraction == 0.0 && liquid > balance_tolerance) {
                compatible = false;
                LiquidState::Inconsistent
            } else if row.liquid_volume_fraction == 0.0 && liquid.abs() <= balance_tolerance {
                let mut absent = true;
                for amount in &liquid_components {
                    absent &= amount.abs() <= balance_tolerance;
                }
                compatible &= absent;
                if compatible {
                    LiquidState::Absent
                } else {
                    LiquidState::Inconsistent
                }
            } else if liquid <= minimum_liquid_moles {
                compatible = false;
                LiquidState::Unresolved
            } else {
                let mut resolved = true;
                for amount in &liquid_components {
                    let fraction = amount / liquid;
                    if !(0.0..=1.0).contains(&fraction) {
                        resolved = false;
                    }
                }
                compatible &= resolved;
                if compatible {
                    LiquidState::Resolved
                } else {
                    LiquidState::Inconsistent
                }
            };

        stages.push(CvdStage {
            total_moles: total,
            vapor_moles: vapor,
            liquid_moles: liquid,
            stage_withdrawn_moles: withdrawal,
            component_closure: closure,
            remaining_closure,
            produced_closure,
            vapor_closure,
            liquid_state: state,
            compatible,
        });
        produced_rows.push(produced.clone());
        remaining_rows.push(remaining);
        liquid_rows.push(liquid_components);
    }

    let compatible = stages.iter().all(|stage| stage.compatible);
    Ok(CvdMaterialBalanceResult {
        stages,
        produced_component_moles: produced_rows,
        remaining_component_moles: remaining_rows,
        liquid_component_moles: liquid_rows,
        compatible,
        warnings: Vec::new(),
    })
}
