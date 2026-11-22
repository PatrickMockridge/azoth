//! The `pvt` namespace's PyO3 bindings.
//!
//! Each `#[pyfunction]` is the extension's door for one registered `pvt.*` id; the wrapper,
//! the stub and the bridge are generated from the spec and this module's signature. The
//! module carries no arithmetic: the kernels are `azoth_pvt`'s, and a binding that restated
//! one would be a third implementation.
//!
//! **The swelling experiment's injection gas is an optional `record_list`**, which the wrapper
//! generator does not derive, so that binding is hand-written here.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::kelvins;
use pyo3::prelude::*;

use crate::errors::to_pyerr;

fn component(row: crate::records_gen::FluidComponent) -> FluidComponent {
    FluidComponent {
        fluid: row.fluid,
        name: row.name,
        pseudo_component: row.pseudo_component,
        moles: row.moles,
        molar_mass: row.molar_mass,
        density: row.density,
        normal_boiling_point: row.normal_boiling_point,
        critical_temperature: row.critical_temperature,
        critical_pressure: row.critical_pressure,
        acentric_factor: row.acentric_factor,
        critical_volume: row.critical_volume,
        racket_z: row.racket_z,
        racket_z_cpa: row.racket_z_cpa,
        parachor: row.parachor,
        critical_viscosity: row.critical_viscosity,
        triple_point_temperature: row.triple_point_temperature,
        heat_of_fusion: row.heat_of_fusion,
        ideal_gas_enthalpy_of_formation: row.ideal_gas_enthalpy_of_formation,
        cp_a: row.cp_a,
        cp_b: row.cp_b,
        cp_c: row.cp_c,
        cp_d: row.cp_d,
        attractive_m: row.attractive_m,
    }
}

/// The swelling experiment.
///
/// All arguments are SI magnitudes. See the module documentation for why the injection gas's
/// optional `record_list` makes this binding hand-written.
#[pyfunction]
#[pyo3(signature = (components, injection_gas, eos, mixing_rule, temperature, cumulative_gas_injected, volume_correction))]
#[pyo3(
    text_signature = "(components, injection_gas, eos, mixing_rule, temperature, cumulative_gas_injected, volume_correction)"
)]
#[allow(clippy::too_many_arguments)] // The signature is the spec's declared inputs.
pub fn swelling_test(
    py: Python<'_>,
    components: Vec<crate::records_gen::FluidComponent>,
    injection_gas: Option<Vec<crate::records_gen::FluidComponent>>,
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    temperature: Option<f64>,
    cumulative_gas_injected: Vec<f64>,
    volume_correction: Option<bool>,
) -> PyResult<crate::transport_gen::PySwellingTestResult> {
    let oil: Vec<FluidComponent> = components.into_iter().map(component).collect();
    let gas: Option<Vec<FluidComponent>> =
        injection_gas.map(|rows| rows.into_iter().map(component).collect());
    azoth_pvt::swelling_test::swelling_test(
        &oil,
        gas.as_deref(),
        eos,
        mixing_rule,
        temperature.map(kelvins),
        &cumulative_gas_injected,
        volume_correction,
    )
    .map(|r| crate::transport_gen::PySwellingTestResult::from(&r))
    .map_err(|e| to_pyerr(py, e))
}
