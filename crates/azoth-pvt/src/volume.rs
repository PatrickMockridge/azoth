//! Molar volume, density and the Peneloux translation, in SI.
//!
//! NeqSim holds pressure in bar and volume in units of `1e-5 m3`, which is why its density
//! multiplies by `1e5`: `M/(v_internal - c_internal) * 1e5` is `M/(v_SI - c_SI)`. This module
//! is the same arithmetic in SI, so the `1e5` never appears except where a class's published
//! number carries it (CCE's `Bg`).
//!
//! **The translation is chosen by the cubic.** `Mixture::volume_shift` sums the
//! Peng-Robinson components' shifts; a Soave-Redlich-Kwong fluid must sum
//! `Component::volume_shift_srk` instead. The two differ by their constants
//! (`0.50033 (0.25969 - Z_RA)` against `0.40768 (0.29441 - Z_RA)`) and mistaking one for the
//! other moves every density.

use azoth_core::{AzothError, Result};
use azoth_eos::{Cubic, Mixture, ReducedParameters};

use crate::standard::NEQSIM_GAS_CONSTANT;

/// The composition-weighted volume translation of a mixture, in m3/mol.
#[must_use]
pub fn volume_shift(mixture: &Mixture, x: &[f64]) -> f64 {
    match mixture.cubic() {
        Cubic::Srk => mixture
            .components()
            .iter()
            .enumerate()
            .map(|(index, component)| x[index] * component.volume_shift_srk)
            .sum(),
        // Peng-Robinson is the translation `Mixture::volume_shift` documents; the other
        // cubics that carry a translation share its geometry.
        _ => mixture.volume_shift(x),
    }
}

/// The untranslated molar volume `v = Z R T / P`, in m3/mol.
#[must_use]
pub fn molar_volume(z: f64, t: f64, p: f64) -> f64 {
    z * NEQSIM_GAS_CONSTANT * t / p
}

/// The molar volume after the Peneloux translation, or before it when correction is off.
#[must_use]
pub fn corrected_molar_volume(
    mixture: &Mixture,
    x: &[f64],
    z: f64,
    t: f64,
    p: f64,
    corrected: bool,
) -> f64 {
    let volume = molar_volume(z, t, p);
    if corrected {
        volume - volume_shift(mixture, x)
    } else {
        volume
    }
}

/// A phase's molar mass, in kg/mol.
///
/// # Errors
/// * [`AzothError::InvalidInput`] if a component carries no molar mass.
pub fn phase_molar_mass(mixture: &Mixture, x: &[f64]) -> Result<f64> {
    let mut molar_mass = 0.0;
    for (index, component) in mixture.components().iter().enumerate() {
        let mass = component.molar_mass.ok_or_else(|| {
            AzothError::invalid_input(
                "components",
                format!("component {index} carries no molar mass, so the phase has none"),
            )
        })?;
        molar_mass += x[index] * mass;
    }
    Ok(molar_mass)
}

/// A phase's mass density, in kg/m3.
///
/// # Errors
/// * The same as [`phase_molar_mass`], and a non-positive translated volume.
pub fn phase_density(
    mixture: &Mixture,
    x: &[f64],
    z: f64,
    t: f64,
    p: f64,
    corrected: bool,
) -> Result<f64> {
    let volume = corrected_molar_volume(mixture, x, z, t, p, corrected);
    if !(volume.is_finite() && volume > 0.0) {
        return Err(AzothError::invalid_input(
            "volume",
            format!("the translated molar volume is {volume} m3/mol"),
        ));
    }
    Ok(phase_molar_mass(mixture, x)? / volume)
}

/// `P V_corr/(n R T)`, the volume-corrected compressibility.
#[must_use]
pub fn zvolcorr(z: f64, t: f64, p: f64, corrected: bool, shift: f64) -> f64 {
    let corrected_volume = molar_volume(z, t, p) - if corrected { shift } else { 0.0 };
    corrected_volume / molar_volume(z, t, p) * z
}

/// `dP/dv|T` at a composition, in Pa·mol/m3.
///
/// The cubic is `P = R T/(v - b) - a/((v + d1 b)(v + d2 b))`, so
///
/// ```text
/// dP/dv = -R T/(v - b)^2 + a (2v + (d1 + d2) b) / ((v + d1 b)^2 (v + d2 b)^2)
/// ```
///
/// with the reduced `A` and `B` turned back into `a` and `b` at the stated state. A caller
/// wanting `dP/dV|Tn` divides by the amount.
#[must_use]
pub fn dp_dv_molar(
    mixture: &Mixture,
    reduced: &ReducedParameters,
    x: &[f64],
    z: f64,
    t: f64,
    p: f64,
) -> f64 {
    let (a_reduced, b_reduced) = mixture.mixture_parameters(reduced, x);
    let r = NEQSIM_GAS_CONSTANT;
    let a = a_reduced * r * r * t * t / p;
    let b = b_reduced * r * t / p;
    let v = molar_volume(z, t, p);
    let delta1 = mixture.cubic().delta1();
    let delta2 = mixture.cubic().delta2();
    let left = (v + delta1 * b) * (v + delta1 * b);
    let right = (v + delta2 * b) * (v + delta2 * b);
    -r * t / ((v - b) * (v - b)) + a * (2.0 * v + (delta1 + delta2) * b) / (left * right)
}

/// The isothermal compressibility `-1/(V dP/dV|Tn)`, in 1/Pa.
///
/// The amount cancels: `V dP/dV = v dP/dv` at one composition, so the answer is
/// `-1/(v dP/dv)`.
#[must_use]
pub fn isothermal_compressibility(
    mixture: &Mixture,
    reduced: &ReducedParameters,
    x: &[f64],
    z: f64,
    t: f64,
    p: f64,
) -> f64 {
    let volume = molar_volume(z, t, p);
    -1.0 / (volume * dp_dv_molar(mixture, reduced, x, z, t, p))
}
