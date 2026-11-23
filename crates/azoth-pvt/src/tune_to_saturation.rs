//! `pvt.tune_to_saturation` - the plus-molar-mass search.
//!
//! Spec: `specs/models/pvt/tuning.toml`. `TuneToSaturation` sets the plus fraction's molar
//! mass, re-characterises the fluid, runs the saturation-pressure simulation and steps the
//! mass by `dp/1000` (bar), flipping the sign when the error grows. The class's loop reads
//! the error at the top, so its exit criterion lags one run behind; this reproduces that
//! rather than tidying it.
//!
//! **The plus row must be the fluid's only pseudo row.** `characterisePlusFraction` lumps
//! every TBP and plus row together, while `characterization.characterise_plus_fraction` is
//! the plus end alone; a fluid with both would be a different characterisation and is
//! refused here instead of answered with the wrong one.

use azoth_characterization::component_row::FluidComponent;
use azoth_characterization::{
    KijSource, PlusModel, characterise_plus_fraction, tbp_fraction_component,
};
use azoth_core::units::{
    Pressure, ThermodynamicTemperature, kilograms_per_cubic_meter, kilograms_per_mole,
};
use azoth_core::{AzothError, Result, Warning};

use crate::fluid::Fluid;
use crate::saturation::upper_saturation_pressure;

/// The class's early return, in Pa: `0.01` bar.
const EARLY_RETURN_PA: f64 = 1000.0;
/// The class's loop tolerance, in Pa: `1e-3` bar.
const LOOP_TOLERANCE_PA: f64 = 100.0;
/// The class's iteration cap.
const DEFAULT_MAX_ITERATIONS: u32 = 50;

/// The id's answer.
#[derive(Debug, Clone, PartialEq)]
pub struct TuneToSaturationResult {
    /// The tuned plus-fraction molar mass, in kg/mol.
    pub tuned_molar_mass: f64,
    /// The saturation pressure of the last run, in Pa.
    pub final_saturation_pressure: f64,
    /// The steps taken.
    pub iterations: f64,
    /// Whether the loop exited on its tolerance rather than the cap.
    pub converged: bool,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The first carbon number the class reads off a plus row's name (`C11` gives 11).
fn first_carbon_number(name: &str) -> Result<usize> {
    let digits: String = name
        .trim_start_matches(['C', 'c'])
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse::<usize>().map_err(|_| {
        AzothError::invalid_input(
            "plus_component",
            format!("{name:?} has no leading carbon number, which the Pedersen split needs"),
        )
    })
}

/// The fluid re-characterised at one plus molar mass.
#[allow(clippy::too_many_arguments)] // One call per loop iteration, each argument a stated input.
fn rebuild(
    base: &[FluidComponent],
    plus: &FluidComponent,
    molar_mass: f64,
    plus_model: PlusModel,
    lumps: Option<usize>,
    first_carbon: usize,
    cubic: azoth_eos::Cubic,
    kij: KijSource,
    corrected: bool,
) -> Result<Fluid> {
    let total: f64 = base.iter().map(|row| row.moles).sum::<f64>() + plus.moles;
    let fraction = plus.moles / total;
    let density = plus.density.ok_or_else(|| {
        AzothError::invalid_input("plus_component", "the plus row states no density")
    })?;
    let characterised = characterise_plus_fraction(
        kilograms_per_mole(molar_mass),
        kilograms_per_cubic_meter(density),
        fraction,
        first_carbon,
        Some(plus_model),
        lumps,
    )?;
    let mut rows = base.to_vec();
    for (index, (&share, (&lump_mass, &lump_density))) in characterised
        .fraction_of_heavy_end
        .iter()
        .zip(
            characterised
                .lump_molar_mass
                .iter()
                .zip(&characterised.lump_density),
        )
        .enumerate()
    {
        let card = tbp_fraction_component(kilograms_per_mole(lump_mass.value), lump_density)?;
        rows.push(FluidComponent {
            fluid: 0.0,
            name: format!("PC{}", index + 1),
            pseudo_component: true,
            moles: plus.moles * share,
            molar_mass: Some(lump_mass.value),
            density: Some(lump_density.value),
            normal_boiling_point: Some(card.boiling_temperature.value),
            critical_temperature: Some(card.critical_temperature.value),
            critical_pressure: Some(card.critical_pressure.value),
            acentric_factor: Some(card.acentric_factor),
            critical_volume: Some(card.critical_volume.value),
            racket_z: Some(card.racket_z),
            racket_z_cpa: Some(card.racket_z_cpa),
            parachor: Some(card.parachor),
            critical_viscosity: Some(card.critical_viscosity.value),
            triple_point_temperature: Some(card.triple_point_temperature.value),
            heat_of_fusion: Some(card.heat_of_fusion.value),
            ideal_gas_enthalpy_of_formation: Some(card.ideal_gas_enthalpy_of_formation.value),
            cp_a: Some(card.cp_a),
            cp_b: Some(card.cp_b),
            cp_c: Some(card.cp_c),
            cp_d: Some(card.cp_d),
            attractive_m: Some(card.attraction_exponent),
        });
    }
    Fluid::from_rows(&rows, cubic, kij, corrected)
}

fn saturation(fluid: &Fluid, temperature: f64) -> Result<f64> {
    upper_saturation_pressure(fluid, temperature)?
        .pressure
        .ok_or_else(|| {
            AzothError::invalid_input(
                "saturation_pressure",
                "the saturation scan found no two-phase region at this temperature",
            )
        })
}

/// `TuneToSaturation` as the id exposes it.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for a plus name the row list does not carry, a plus row
///   that is not a pseudo row, another pseudo row present, or a name without a carbon
///   number.
/// * Propagates the characterisation, the flash and the saturation scan.
#[allow(clippy::too_many_arguments)] // The signature is the spec's declared inputs.
pub fn tune_to_saturation(
    components: &[FluidComponent],
    plus_component: &str,
    saturation_temperature: ThermodynamicTemperature,
    saturation_pressure: Pressure,
    plus_model: Option<&str>,
    number_of_lumps: Option<usize>,
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    volume_correction: Option<bool>,
    max_iterations: Option<u32>,
) -> Result<TuneToSaturationResult> {
    let temperature = saturation_temperature.value;
    if !(temperature.is_finite() && temperature > 0.0) {
        return Err(AzothError::invalid_input(
            "saturation_temperature",
            "a saturation temperature must be finite and above zero",
        ));
    }
    if !(saturation_pressure.value.is_finite() && saturation_pressure.value > 0.0) {
        return Err(AzothError::invalid_input(
            "saturation_pressure",
            "a saturation pressure must be finite and above zero",
        ));
    }
    let plus_index = components
        .iter()
        .position(|row| row.name == plus_component)
        .ok_or_else(|| {
            AzothError::invalid_input(
                "plus_component",
                format!("no row is named {plus_component:?}"),
            )
        })?;
    let plus = &components[plus_index];
    if !plus.pseudo_component {
        return Err(AzothError::invalid_input(
            "plus_component",
            "the plus fraction is a pseudo row, and this one is not",
        ));
    }
    if let Some((other, row)) = components
        .iter()
        .enumerate()
        .find(|(index, row)| *index != plus_index && row.pseudo_component)
    {
        return Err(AzothError::invalid_input(
            "components",
            format!(
                "row {other} ({}) is another pseudo row; the class lumps every pseudo row \
                 together and this id re-characterises the plus end alone",
                row.name
            ),
        ));
    }
    let initial = plus.molar_mass.ok_or_else(|| {
        AzothError::invalid_input("plus_component", "the plus row states no molar mass")
    })?;
    let first_carbon = first_carbon_number(&plus.name)?;
    let base: Vec<FluidComponent> = components
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != plus_index)
        .map(|(_, row)| row.clone())
        .collect();
    let model = plus_model
        .map(|name| name.parse::<PlusModel>().unwrap_or_default())
        .unwrap_or_default();
    let cubic = crate::fluid::cubic_named(eos)?;
    let kij = crate::fluid::kij_named(mixing_rule)?;
    let corrected = volume_correction.unwrap_or(true);
    let max = max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS).max(1);
    let target = saturation_pressure.value;

    let mut molar_mass = initial;
    let mut fluid = rebuild(
        &base,
        plus,
        molar_mass,
        model,
        number_of_lumps,
        first_carbon,
        cubic,
        kij,
        corrected,
    )?;
    let mut pressure = saturation(&fluid, temperature)?;
    let mut error = pressure - target;
    let warnings = Vec::new();
    if error.abs() < EARLY_RETURN_PA {
        return Ok(TuneToSaturationResult {
            tuned_molar_mass: molar_mass,
            final_saturation_pressure: pressure,
            iterations: 0.0,
            converged: true,
            warnings,
        });
    }
    let mut previous: f64 = 0.0;
    let mut sign: f64 = 1.0;
    let mut iterations = 0u32;
    while error.abs() > LOOP_TOLERANCE_PA && iterations < max {
        iterations += 1;
        // The class steps on the error it read at the top of this pass.
        molar_mass -= sign * (error / 1.0e5) / 1000.0;
        fluid = rebuild(
            &base,
            plus,
            molar_mass,
            model,
            number_of_lumps,
            first_carbon,
            cubic,
            kij,
            corrected,
        )?;
        pressure = saturation(&fluid, temperature)?;
        if previous.abs() < error.abs() {
            sign = -sign;
        }
        previous = error;
        // The class reads the error again at the top of the next pass.
        error = pressure - target;
    }
    Ok(TuneToSaturationResult {
        tuned_molar_mass: molar_mass,
        final_saturation_pressure: pressure,
        iterations: f64::from(iterations),
        converged: iterations < max,
        warnings,
    })
}
