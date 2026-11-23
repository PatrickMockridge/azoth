//! The four CSP correction factors behind `eos.viscosity`'s boundary.

use azoth_core::units::{kelvins, pascals};
use azoth_eos::Cubic;
use azoth_eos::databank;
use azoth_eos::{viscosity, viscosity_with_csp};

const METHANE_TC: f64 = 190.56;
const METHANE_PC: f64 = 4.599e6;
const METHANE_M: f64 = 0.016043;

fn mixture() -> azoth_eos::mixture::Mixture {
    databank::mixture_of(&["n-heptane"], Cubic::Srk, None)
        .expect("n-heptane resolves")
        .0
}

#[test]
fn unit_factors_are_the_published_correlation() {
    let mixture = mixture();
    let z = [1.0];
    let published = viscosity(&mixture, kelvins(320.0), pascals(5.0e5), &z).expect("published");
    let unit = viscosity_with_csp(&mixture, kelvins(320.0), pascals(5.0e5), &z, [1.0; 4])
        .expect("unit factors");
    assert_eq!(published.mu.value, unit.mu.value);
}

#[test]
fn each_factor_scales_its_own_exponent() {
    let mixture = mixture();
    let z = [1.0];
    let unit = viscosity_with_csp(&mixture, kelvins(320.0), pascals(5.0e5), &z, [1.0; 4])
        .expect("unit factors");

    // One component makes the mixing rule the component's own constants: Tc, Pc and M.
    let component = &mixture.components()[0];
    let tc = component.tc.value;
    let pc = component.pc.value;
    let molar_mass = component.molar_mass.expect("n-heptane has a molar mass") * 1e3;
    for (index, factor) in [(0, 2.0), (1, 2.0), (2, 2.0)] {
        let mut correction = [1.0; 4];
        correction[index] = factor;
        let scaled = viscosity_with_csp(&mixture, kelvins(320.0), pascals(5.0e5), &z, correction)
            .expect("scaled factors");
        let expected = match index {
            0 => (tc / METHANE_TC).powf(-1.0 / 6.0 * (factor - 1.0)),
            1 => (pc / METHANE_PC).powf(2.0 / 3.0 * (factor - 1.0)),
            _ => (molar_mass / (METHANE_M * 1e3)).powf(0.5 * (factor - 1.0)),
        };
        assert!(
            (scaled.mu.value / unit.mu.value - expected).abs() < 1e-12,
            "factor {index}: got {}, expected {expected}",
            scaled.mu.value / unit.mu.value
        );
    }
}

#[test]
fn a_non_finite_factor_is_refused() {
    let mixture = mixture();
    let error = viscosity_with_csp(
        &mixture,
        kelvins(320.0),
        pascals(5.0e5),
        &[1.0],
        [1.0, f64::NAN, 1.0, 1.0],
    )
    .expect_err("a NaN factor has no exponent");
    assert!(matches!(error, azoth_core::AzothError::InvalidInput { .. }));
}
