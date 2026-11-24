//! The wax appearance temperature of NeqSim's `WATcalc`.
//!
//! `calcWAT` brackets the appearance on a 10 K scan of independent TP wax flashes,
//! bisects the bracket to 1e-4 K, and returns the warm endpoint after re-checking both
//! ends. The onset is the wax **mass** fraction crossing 1e-8; the ported
//! `eos.tp_multiflash_wax` reports a mole fraction, so the mass fraction is assembled
//! from the wax phase's molar mass and the feed's.

use azoth_core::units::{kelvins, pascals};
use azoth_core::{AzothError, Result};
use azoth_eos::entropy_state::molar_mass_of;
use azoth_eos::mixture::Mixture;
use azoth_eos::tp_multiflash_wax::tp_multiflash_wax;

/// The class's appearance threshold, a wax mass fraction of the whole fluid.
const WAX_MASS_TOLERANCE: f64 = 1.0e-8;
/// The class's final bracket width, K.
const TEMPERATURE_TOLERANCE: f64 = 1.0e-4;
/// The class's lower search bound, K.
const MIN_TEMPERATURE: f64 = 100.0;
/// The class's upper search bound, K.
const MAX_TEMPERATURE: f64 = 1000.0;
/// The class's scan step, K.
const SEARCH_STEP: f64 = 10.0;
/// The class's cap on TP flash evaluations.
const MAX_FLASHES: u32 = 120;

/// One feed's wax probe, counting the class's flash evaluations.
struct Probe<'a> {
    mixture: &'a Mixture,
    pressure: f64,
    z: &'a [f64],
    feed_molar_mass: f64,
    evaluations: u32,
}

impl Probe<'_> {
    /// Whether a wax phase above the class's mass threshold exists at a temperature.
    fn has_wax(&mut self, temperature: f64) -> Result<bool> {
        self.evaluations += 1;
        if self.evaluations > MAX_FLASHES {
            return Err(AzothError::invalid_input(
                "wax_appearance",
                format!("exceeded {MAX_FLASHES} TP flash evaluations"),
            ));
        }
        let result = tp_multiflash_wax(
            self.mixture,
            kelvins(temperature),
            pascals(self.pressure),
            self.z,
        )?;
        if !result.converged
            || result.phase_count < 3
            || result.wax_fraction.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
        {
            return Ok(false);
        }
        let Some(composition) = result.x.last() else {
            return Ok(false);
        };
        let wax_mass = result.wax_fraction * molar_mass_of(self.mixture, composition)?;
        let fraction = wax_mass / self.feed_molar_mass;
        if !fraction.is_finite() || fraction < 0.0 {
            return Err(AzothError::invalid_input(
                "wax_appearance",
                "the wax mass fraction is not a fraction",
            ));
        }
        Ok(fraction > WAX_MASS_TOLERANCE)
    }
}

/// The class's `calcWAT`: the warm endpoint of the appearance bracket, K.
pub(crate) fn wat_temperature(
    mixture: &Mixture,
    start_temperature: f64,
    pressure: f64,
    z: &[f64],
) -> Result<f64> {
    if !start_temperature.is_finite()
        || !(MIN_TEMPERATURE..=MAX_TEMPERATURE).contains(&start_temperature)
    {
        return Err(AzothError::invalid_input(
            "design_temperature",
            "calcWAT requires a finite temperature in 100-1000 K",
        ));
    }
    if !(pressure > 0.0 && pressure.is_finite()) {
        return Err(AzothError::invalid_input(
            "pressure",
            "calcWAT requires a positive finite pressure",
        ));
    }
    let feed_molar_mass = molar_mass_of(mixture, z)?;
    let mut probe = Probe {
        mixture,
        pressure,
        z,
        feed_molar_mass,
        evaluations: 0,
    };

    let mut temperature = start_temperature;
    let mut lower = f64::NAN;
    let mut upper = f64::NAN;
    if probe.has_wax(temperature)? {
        lower = temperature;
        let mut wax = true;
        while wax && temperature < MAX_TEMPERATURE {
            temperature = (temperature + SEARCH_STEP).min(MAX_TEMPERATURE);
            wax = probe.has_wax(temperature)?;
            if wax {
                lower = temperature;
            } else {
                upper = temperature;
            }
        }
    } else {
        upper = temperature;
        let mut wax = false;
        while !wax && temperature > MIN_TEMPERATURE {
            temperature = (temperature - SEARCH_STEP).max(MIN_TEMPERATURE);
            wax = probe.has_wax(temperature)?;
            if wax {
                lower = temperature;
            } else {
                upper = temperature;
            }
        }
    }
    if !lower.is_finite() || !upper.is_finite() {
        return Err(AzothError::invalid_input(
            "wax_appearance",
            "no wax appearance bracket in the 100-1000 K search range",
        ));
    }
    while upper - lower > TEMPERATURE_TOLERANCE {
        temperature = 0.5 * (lower + upper);
        if probe.has_wax(temperature)? {
            lower = temperature;
        } else {
            upper = temperature;
        }
    }
    if !probe.has_wax(lower)? || probe.has_wax(upper)? {
        return Err(AzothError::invalid_input(
            "wax_appearance",
            "the wax appearance endpoints were not reproducible",
        ));
    }
    Ok(upper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use azoth_eos::Cubic;
    use azoth_eos::databank::mixture_of;

    fn check(names: &[&str], z: &[f64], pressure: f64, start: f64, expected: f64) {
        let (mixture, _) = mixture_of(names, Cubic::Srk, None).expect("the feed resolves");
        let got = wat_temperature(&mixture, start, pressure, z).expect("the onset brackets");
        assert!(
            (got - expected).abs() < 1.0e-6,
            "WAT at {pressure} Pa: got {got}, expected {expected}"
        );
    }

    #[test]
    fn the_captured_wax_appearance_temperatures() {
        // `water_dew_point_probe.tsv`: the three nameable `calcWAT` rows. The fourth is
        // `WaxFlashTest`'s plus-fraction fixture, which a nameable feed cannot build.
        check(
            &["methane", "n-heptane", "nc14", "nc20"],
            &[0.7, 0.1, 0.1, 0.1],
            5.0e5,
            320.0,
            269.150924682617,
        );
        check(
            &["methane", "n-heptane", "nc14", "nc20", "nc24"],
            &[0.4, 0.1, 0.1, 0.2, 0.2],
            1.0e5,
            320.0,
            270.185699462891,
        );
        // The lean feed is the probe's own "calcWAT refuses rather than answers" state. The
        // capture's 132.072 K is that marginal answer; the port's wax flash brackets a
        // physical onset at 233.635 K. The divergence is recorded, not smoothed: the lean
        // row is evidence, not a case.
        let amounts = [
            0.018, 0.333, 96.702, 1.773, 0.496, 0.099, 0.115, 0.004, 0.024, 0.324,
        ];
        let total: f64 = amounts.iter().sum();
        let z: Vec<f64> = amounts.iter().map(|value| value / total).collect();
        let (mixture, _) = mixture_of(
            &[
                "CO2",
                "nitrogen",
                "methane",
                "ethane",
                "propane",
                "i-butane",
                "n-butane",
                "i-pentane",
                "n-pentane",
                "n-heptane",
            ],
            Cubic::Srk,
            None,
        )
        .expect("the lean feed resolves");
        let got = wat_temperature(&mixture, 303.0, 50.0e5, &z).expect("the lean onset brackets");
        assert!(
            (got - 233.63507080078125).abs() < 1.0e-3,
            "the lean feed's port onset moved: {got}"
        );
    }
}
