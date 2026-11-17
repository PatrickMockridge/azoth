//! The shared internals of the `process.*self_heating` family.
//!
//! The two criticality screens and the transient induction solver read one geometry table, one
//! verdict rule and one bisection. The arithmetic is NeqSim's
//! `process/safety/selfheating/SelfHeatingGeometry.java`, `SemenovSelfHeatingAnalyzer.java` and
//! `PorousMediaSelfHeatingAnalyzer.java`.

use azoth_core::{AzothError, Result};
use serde::Serialize;

/// The class's own gas constant, J/(mol K).
pub(crate) const R_GAS: f64 = 8.314_462_618;

/// The classical Semenov critical parameter, `1/e`.
pub(crate) const PSI_CRIT: f64 = 1.0 / std::f64::consts::E;

/// The lower bound of the critical-temperature bisection, K.
pub(crate) const T_SEARCH_MIN_K: f64 = 150.0;

/// The upper bound of the critical-temperature bisection, K.
pub(crate) const T_SEARCH_MAX_K: f64 = 2000.0;

/// The bisection's convergence tolerance, K.
pub(crate) const T_SEARCH_TOL_K: f64 = 1.0e-4;

/// The bisection's iteration cap.
pub(crate) const MAX_BISECTION_ITERATIONS: usize = 200;

/// The class's default marginal ratio.
pub(crate) const DEFAULT_MARGINAL_RATIO: f64 = 0.7;

/// The bracket-failure message, spelled as the class spells it.
pub(crate) const BRACKET_WARNING: &str = "Critical ambient temperature could not be bracketed \
between 150.0 K and 2000.0 K; check the kinetic parameters";

/// The Semenov model's always-present note.
pub(crate) const SEMENOV_WARNING: &str = "Semenov theory assumes a spatially uniform body \
temperature (small Biot number); use PorousMediaSelfHeatingAnalyzer when internal conduction \
controls";

/// The Frank-Kamenetskii model's always-present note.
pub(crate) const FK_WARNING: &str = "Frank-Kamenetskii theory assumes a uniform boundary \
temperature, conduction-controlled heat loss (large Biot number) and negligible reactant \
consumption; use SemenovSelfHeatingAnalyzer when surface cooling controls";

/// The pipe-insulation configuration's note.
pub(crate) const PIPE_INSULATION_WARNING: &str = "Pipe-insulation screening: slab half-dimension \
set to the full insulation thickness and boundary temperature set to the pipe wall temperature \
(conservative bounding assumption)";

/// Body shapes and their critical Frank-Kamenetskii parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SelfHeatingGeometry {
    /// Infinite slab; the characteristic dimension is the half-thickness.
    #[serde(rename = "slab")]
    Slab,
    /// Infinite cylinder; the characteristic dimension is the radius.
    #[serde(rename = "infinite_cylinder")]
    InfiniteCylinder,
    /// Sphere; the characteristic dimension is the radius.
    #[serde(rename = "sphere")]
    Sphere,
    /// Cube; the characteristic dimension is the half-side length.
    #[serde(rename = "cube")]
    Cube,
    /// Equicylinder; the characteristic dimension is the radius.
    #[serde(rename = "equicylinder")]
    Equicylinder,
}

impl SelfHeatingGeometry {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Slab => "slab",
            Self::InfiniteCylinder => "infinite_cylinder",
            Self::Sphere => "sphere",
            Self::Cube => "cube",
            Self::Equicylinder => "equicylinder",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "slab" => Self::Slab,
            "infinite_cylinder" => Self::InfiniteCylinder,
            "sphere" => Self::Sphere,
            "cube" => Self::Cube,
            "equicylinder" => Self::Equicylinder,
            _ => {
                return Err(AzothError::invalid_input(
                    "geometry",
                    format!(
                        "`{value}` is not `slab`, `infinite_cylinder`, `sphere`, `cube` or \
                         `equicylinder`"
                    ),
                ));
            }
        })
    }

    /// The shape's published critical Frank-Kamenetskii parameter.
    #[must_use]
    pub(crate) fn delta_crit(self) -> f64 {
        match self {
            Self::Slab => 0.878,
            Self::InfiniteCylinder => 2.000,
            Self::Sphere => 3.322,
            Self::Cube => 2.520,
            Self::Equicylinder => 2.760,
        }
    }

    /// The Laplacian shape factor: 0 slab, 1 cylinder, 2 sphere.
    #[must_use]
    pub(crate) fn shape_factor(self) -> f64 {
        match self {
            Self::Slab => 0.0,
            Self::InfiniteCylinder => 1.0,
            Self::Sphere => 2.0,
            Self::Cube => 0.0,
            Self::Equicylinder => 1.0,
        }
    }

    /// Whether the shape reduces to a one-dimensional conduction problem.
    #[must_use]
    pub(crate) fn one_dimensional(self) -> bool {
        matches!(self, Self::Slab | Self::InfiniteCylinder | Self::Sphere)
    }
}

/// The screening verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SelfHeatingVerdict {
    /// Below the marginal band; a stable profile exists with margin.
    #[serde(rename = "subcritical")]
    Subcritical,
    /// Below the critical value but inside the marginal band.
    #[serde(rename = "marginal")]
    Marginal,
    /// Above the critical value; the body is predicted to self-ignite.
    #[serde(rename = "self_ignition")]
    SelfIgnition,
}

impl SelfHeatingVerdict {
    /// The spec's spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Subcritical => "subcritical",
            Self::Marginal => "marginal",
            Self::SelfIgnition => "self_ignition",
        }
    }
}

/// The class's verdict rule: self-ignition strictly above one, marginal inclusively at the ratio.
#[must_use]
pub(crate) fn verdict_of(ratio: f64, marginal_ratio: f64) -> SelfHeatingVerdict {
    if ratio > 1.0 {
        SelfHeatingVerdict::SelfIgnition
    } else if ratio >= marginal_ratio {
        SelfHeatingVerdict::Marginal
    } else {
        SelfHeatingVerdict::Subcritical
    }
}

/// The Semenov criticality parameter, evaluated in logarithmic form.
#[must_use]
pub(crate) fn semenov_psi(
    activation_energy: f64,
    volume: f64,
    pre_factor: f64,
    heat_transfer_coefficient: f64,
    surface_area: f64,
    temperature_k: f64,
) -> f64 {
    let ln_psi = activation_energy.ln() + volume.ln() + pre_factor.ln()
        - heat_transfer_coefficient.ln()
        - surface_area.ln()
        - R_GAS.ln()
        - 2.0 * temperature_k.ln()
        - activation_energy / (R_GAS * temperature_k);
    ln_psi.exp()
}

/// The Frank-Kamenetskii criticality parameter, evaluated in logarithmic form.
#[must_use]
pub(crate) fn fk_delta(
    activation_energy: f64,
    pre_factor: f64,
    dimension_m: f64,
    conductivity: f64,
    temperature_k: f64,
) -> f64 {
    let ln_delta = activation_energy.ln() + pre_factor.ln() + 2.0 * dimension_m.ln()
        - conductivity.ln()
        - R_GAS.ln()
        - 2.0 * temperature_k.ln()
        - activation_energy / (R_GAS * temperature_k);
    ln_delta.exp()
}

/// The half-dimension at which `delta` reaches `delta_crit`.
#[must_use]
pub(crate) fn fk_critical_dimension(
    activation_energy: f64,
    pre_factor: f64,
    conductivity: f64,
    temperature_k: f64,
    delta_crit: f64,
) -> f64 {
    let ln_r_crit = 0.5
        * (delta_crit.ln()
            + conductivity.ln()
            + R_GAS.ln()
            + 2.0 * temperature_k.ln()
            + activation_energy / (R_GAS * temperature_k)
            - activation_energy.ln()
            - pre_factor.ln());
    ln_r_crit.exp()
}

/// The class's bisection on `[150, 2000]` K, answering absent when the bracket fails.
pub(crate) fn critical_temperature<F>(residual: F) -> Option<f64>
where
    F: Fn(f64) -> f64,
{
    let mut low = T_SEARCH_MIN_K;
    let mut high = T_SEARCH_MAX_K;
    if residual(low) > 0.0 || residual(high) < 0.0 {
        return None;
    }
    let mut iteration = 0;
    while iteration < MAX_BISECTION_ITERATIONS && (high - low) > T_SEARCH_TOL_K {
        let mid = 0.5 * (low + high);
        if residual(mid) < 0.0 {
            low = mid;
        } else {
            high = mid;
        }
        iteration += 1;
    }
    Some(0.5 * (low + high))
}

#[cfg(test)]
mod tests {
    use super::{PSI_CRIT, SelfHeatingGeometry, SelfHeatingVerdict, verdict_of};

    #[test]
    fn the_critical_parameter_is_inverse_e() {
        assert!((PSI_CRIT - 0.367_879_441_171_442_33).abs() < 1.0e-15);
    }

    #[test]
    fn the_verdict_boundaries_are_strict_above_one_and_inclusive_at_the_ratio() {
        assert_eq!(verdict_of(1.000_001, 0.7), SelfHeatingVerdict::SelfIgnition);
        assert_eq!(verdict_of(1.0, 0.7), SelfHeatingVerdict::Marginal);
        assert_eq!(verdict_of(0.7, 0.7), SelfHeatingVerdict::Marginal);
        assert_eq!(verdict_of(0.699_9, 0.7), SelfHeatingVerdict::Subcritical);
    }

    #[test]
    fn the_shape_table_carries_the_published_critical_values() {
        assert_eq!(SelfHeatingGeometry::Slab.delta_crit(), 0.878);
        assert_eq!(SelfHeatingGeometry::InfiniteCylinder.delta_crit(), 2.000);
        assert_eq!(SelfHeatingGeometry::Sphere.delta_crit(), 3.322);
        assert_eq!(SelfHeatingGeometry::Cube.delta_crit(), 2.520);
        assert_eq!(SelfHeatingGeometry::Equicylinder.delta_crit(), 2.760);
        assert!(!SelfHeatingGeometry::Cube.one_dimensional());
        assert!(!SelfHeatingGeometry::Equicylinder.one_dimensional());
        assert_eq!(SelfHeatingGeometry::Sphere.shape_factor(), 2.0);
    }
}
