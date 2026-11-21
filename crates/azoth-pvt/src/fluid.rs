//! A fluid the laboratory experiments flash.
//!
//! NeqSim's experiments take a `SystemInterface` and mutate it: add the injection gas, remove
//! the gas phase, remove the moles that restore a cell volume. The port keeps the same
//! sequence as an explicit composition ledger - the amounts are a vector, and every step is a
//! fresh [`Fluid`] rather than a mutation of a shared object. What the classes reach through
//! phase properties is computed from the flash's own phases in [`crate::flash`].
//!
//! The fluid is built from the characterization component rows; the two fluids of a swelling
//! or MMP experiment are one union mixture with the injection gas's amounts carried separately,
//! because NeqSim's `addFluid` merges by component name.

use azoth_characterization::component_row::FluidComponent;
use azoth_characterization::{KijSource, mixture_from_characterization_with};
use azoth_core::units::kelvins;
use azoth_core::units::pascals;
use azoth_core::{AzothError, Result};
use azoth_eos::{Cubic, IdealGasModel, Mixture};

use crate::flash::{Flash, FlashMode};

/// One fluid, its amounts, and the two models a flash and an enthalpy take.
#[derive(Debug, Clone, PartialEq)]
pub struct Fluid {
    /// The mixture, frozen at construction.
    pub mixture: Mixture,
    /// The ideal-gas heat-capacity model, carried for a caller that asks for one.
    pub ideal_gas: IdealGasModel,
    /// The cubic the mixture was built for.
    pub cubic: Cubic,
    /// The component amounts, in the mixture's own order, in mol.
    moles: Vec<f64>,
    /// Whether the Peneloux translation is applied, NeqSim's `useVolumeCorrection`.
    corrected: bool,
}

/// The injection gas's composition in a union fluid's component order.
///
/// Amounts are unnormalised; [`Fluid::with_added`] divides by [`Self::total`]. A name the oil
/// already carries merges into the oil's row, and a name it does not is appended with a zero
/// starting amount, which is what `addFluid` does.
#[derive(Debug, Clone, PartialEq)]
pub struct GasTemplate {
    /// The gas's amount of each union component, in mol.
    pub amounts: Vec<f64>,
    /// The gas's total amount, in mol.
    pub total: f64,
}

impl Fluid {
    /// A fluid from its component rows.
    ///
    /// # Errors
    /// * [`AzothError::InvalidInput`] if the rows are empty, belong to different fluids, carry
    ///   a non-finite or negative amount, or sum to no amount - and propagates
    ///   [`mixture_from_characterization_with`]'s refusals.
    pub fn from_rows(
        rows: &[FluidComponent],
        cubic: Cubic,
        kij: KijSource,
        corrected: bool,
    ) -> Result<Self> {
        check_rows(rows)?;
        let moles = rows.iter().map(|row| row.moles).collect::<Vec<_>>();
        let (mixture, ideal_gas) = mixture_from_characterization_with(rows, cubic, kij)?;
        Ok(Self {
            mixture,
            ideal_gas,
            cubic,
            moles,
            corrected,
        })
    }

    /// An oil and an injection gas as one union fluid, with the gas's template.
    ///
    /// # Errors
    /// The same as [`Self::from_rows`], plus a gas row that shares a name with an oil row of
    /// the other kind (a base row against a pseudo row).
    pub fn from_rows_pair(
        oil: &[FluidComponent],
        gas: &[FluidComponent],
        cubic: Cubic,
        kij: KijSource,
        corrected: bool,
    ) -> Result<(Self, GasTemplate)> {
        check_rows(oil)?;
        check_rows(gas)?;
        let mut union = oil.to_vec();
        let mut amounts = vec![0.0; union.len()];
        let mut total = 0.0;
        for row in gas {
            let position = union.iter().position(|held| held.name == row.name);
            let index = match position {
                Some(index) => {
                    if union[index].pseudo_component != row.pseudo_component {
                        return Err(AzothError::invalid_input(
                            "injection_gas",
                            format!(
                                "{} is a base row in one fluid and a pseudo row in the other, so \
                                 the union has no one component for it",
                                row.name
                            ),
                        ));
                    }
                    index
                }
                None => {
                    let mut appended = row.clone();
                    appended.fluid = 0.0;
                    appended.moles = 0.0;
                    union.push(appended);
                    amounts.push(0.0);
                    union.len() - 1
                }
            };
            amounts[index] += row.moles;
            total += row.moles;
        }
        let fluid = Self::from_rows(&union, cubic, kij, corrected)?;
        Ok((fluid, GasTemplate { amounts, total }))
    }

    /// The component amounts, in mol.
    #[must_use]
    pub fn moles(&self) -> &[f64] {
        &self.moles
    }

    /// Whether the Peneloux translation is applied.
    #[must_use]
    pub fn corrected(&self) -> bool {
        self.corrected
    }

    /// The feed's total amount, in mol.
    #[must_use]
    pub fn total_moles(&self) -> f64 {
        self.moles.iter().sum()
    }

    /// The feed's mole fractions, normalised.
    #[must_use]
    pub fn z(&self) -> Vec<f64> {
        let total = self.total_moles();
        self.moles.iter().map(|moles| moles / total).collect()
    }

    /// The feed's mass, in kg.
    ///
    /// # Errors
    /// * [`AzothError::InvalidInput`] if a component carries no molar mass.
    pub fn mass(&self) -> Result<f64> {
        let mut mass = 0.0;
        for (index, component) in self.mixture.components().iter().enumerate() {
            let molar_mass = component.molar_mass.ok_or_else(|| {
                AzothError::invalid_input(
                    "components",
                    format!(
                        "component {} carries no molar mass, so the fluid has no mass",
                        index
                    ),
                )
            })?;
            mass += self.moles[index] * molar_mass;
        }
        Ok(mass)
    }

    /// The same fluid at different amounts.
    #[must_use]
    pub fn with_moles(&self, moles: Vec<f64>) -> Self {
        let mut next = self.clone();
        next.moles = moles;
        next
    }

    /// The fluid after adding `total` moles of a gas with the given template.
    #[must_use]
    pub fn with_added(&self, gas: &GasTemplate, total: f64) -> Self {
        let mut moles = self.moles.clone();
        for (index, amount) in gas.amounts.iter().enumerate() {
            moles[index] += total * amount / gas.total;
        }
        self.with_moles(moles)
    }

    /// Flash the fluid at a temperature and a pressure, both SI.
    ///
    /// # Errors
    /// * [`AzothError::InvalidInput`] if the temperature or pressure is not positive - and
    ///   propagates the flash's own refusals.
    pub fn flash(&self, t: f64, p: f64, mode: FlashMode) -> Result<Flash> {
        if !(t.is_finite() && t > 0.0) {
            return Err(AzothError::invalid_input(
                "temperature",
                format!("a flash needs a positive temperature; got {t}"),
            ));
        }
        if !(p.is_finite() && p > 0.0) {
            return Err(AzothError::invalid_input(
                "pressure",
                format!("a flash needs a positive pressure; got {p}"),
            ));
        }
        crate::flash::run(
            &self.mixture,
            self.corrected,
            self.total_moles(),
            &self.z(),
            kelvins(t),
            pascals(p),
            mode,
        )
    }
}

/// The checks every row list faces before a mixture is built.
fn check_rows(rows: &[FluidComponent]) -> Result<()> {
    if rows.is_empty() {
        return Err(AzothError::invalid_input(
            "components",
            "a fluid needs at least one component row",
        ));
    }
    let fluid = rows[0].fluid;
    if rows.iter().any(|row| row.fluid != fluid) {
        return Err(AzothError::invalid_input(
            "components",
            "the rows belong to more than one fluid, and one id describes one fluid",
        ));
    }
    for (index, row) in rows.iter().enumerate() {
        if !row.moles.is_finite() || row.moles < 0.0 {
            return Err(AzothError::invalid_input(
                format!("components[{index}].moles"),
                format!("a component's amount is {} mol", row.moles),
            ));
        }
    }
    if rows.iter().map(|row| row.moles).sum::<f64>() <= 0.0 {
        return Err(AzothError::invalid_input(
            "components",
            "the fluid's components sum to no amount",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use azoth_characterization::KijSource;

    fn pair() -> (Vec<FluidComponent>, Vec<FluidComponent>) {
        (
            vec![
                FluidComponent::base("methane", 0.2),
                FluidComponent::base("nC10", 0.8),
            ],
            vec![FluidComponent::base("methane", 1.0)],
        )
    }

    #[test]
    fn the_union_merges_a_gas_component_into_the_oils_row() {
        let (oil, gas) = pair();
        let (fluid, template) =
            Fluid::from_rows_pair(&oil, &gas, Cubic::Srk, KijSource::Database, true).unwrap();
        assert_eq!(fluid.moles(), &[0.2, 0.8]);
        assert_eq!(template.amounts, vec![1.0, 0.0]);
        assert_eq!(template.total, 1.0);
        let swelled = fluid.with_added(&template, 0.1);
        assert!((swelled.moles()[0] - 0.3).abs() < 1e-15);
        assert!((swelled.moles()[1] - 0.8).abs() < 1e-15);
    }

    #[test]
    fn the_union_appends_a_gas_component_the_oil_does_not_have() {
        let oil = vec![
            FluidComponent::base("methane", 0.5),
            FluidComponent::base("nC10", 0.5),
        ];
        let gas = vec![
            FluidComponent::base("methane", 0.5),
            FluidComponent::base("ethane", 0.5),
        ];
        let (fluid, template) =
            Fluid::from_rows_pair(&oil, &gas, Cubic::Srk, KijSource::Database, true).unwrap();
        assert_eq!(fluid.moles(), &[0.5, 0.5, 0.0]);
        assert_eq!(template.amounts, vec![0.5, 0.0, 0.5]);
    }

    #[test]
    fn two_fluids_in_one_row_list_are_refused() {
        let mut a = FluidComponent::base("methane", 1.0);
        let mut b = FluidComponent::base("nC10", 1.0);
        b.fluid = 1.0;
        a.fluid = 0.0;
        let err = Fluid::from_rows(&[a, b], Cubic::Srk, KijSource::Database, true)
            .expect_err("two fluids are not one id's input");
        assert!(err.to_string().contains("more than one fluid"));
    }
}
