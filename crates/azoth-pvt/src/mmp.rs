//! `pvt.mmp` - NeqSim's minimum miscibility pressure.
//!
//! Spec: `specs/models/pvt/mmp.toml`. The arithmetic is
//! `pvtsimulation/simulation/MMPCalculator.java` at the pin:
//!
//! * `slim_tube`: a recovery curve over the pressure grid, by a chain of mixing cells that
//!   inject the gas phase at the pressure's equilibrium composition, forward half of each
//!   cell's gas to the next and produce a tenth of the last cell's oil per step;
//! * `key_tie_line`: a 0.5-bar bisection on the tie-line length of the oil + full gas mixture;
//! * `rising_bubble`: the class delegates to the slim tube, and so does this.
//!
//! **`setTemperature` is Celsius in the class** despite its comment; the id takes K and the
//! probe converts.

use azoth_characterization::component_row::FluidComponent;
use azoth_core::units::{Pressure, ThermodynamicTemperature};
use azoth_core::{AzothError, Result, Warning};

use crate::flash::FlashMode;
use crate::fluid::{Fluid, cubic_named, kij_named};
use crate::standard::{
    MMP_DEFAULT_MAX_PRESSURE_BAR, MMP_DEFAULT_MIN_PRESSURE_BAR, MMP_DEFAULT_NODES,
    MMP_DEFAULT_PRESSURE_POINTS, MMP_DEFAULT_PV_INJECTED, MMP_DEFAULT_THRESHOLD,
    MMP_KEY_TIE_LINE_LENGTH_TOLERANCE, MMP_KEY_TIE_LINE_PRESSURE_TOLERANCE_BAR, bar_to_pa,
};

/// One pressure of a slim-tube recovery curve.
#[derive(Debug, Clone, PartialEq)]
pub struct MmpPoint {
    /// The pressure, in Pa.
    pub pressure: f64,
    /// The recovery, 0 to 1.
    pub recovery: f64,
}

/// Result of `pvt.mmp`.
#[derive(Debug, Clone, PartialEq)]
pub struct MmpResult {
    /// The minimum miscibility pressure, in Pa; absent where the range does not reach it.
    pub mmp: Option<f64>,
    /// `first_contact`, `vaporizing`, `condensing`, `combined`, `immiscible` or `unknown`.
    pub mechanism: String,
    /// The slim tube's curve; empty for the key tie line.
    pub points: Vec<MmpPoint>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The MMP of an oil and an injection gas.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown `eos`, `mixing_rule` or `method`, fewer than
///   three pressure points, no slim-tube nodes, a non-positive PV injected, a decreasing
///   pressure range, a threshold outside `(0, 1]`, a non-positive temperature, a gas with a
///   component the oil does not carry, or a gas sharing no component with the oil.
#[allow(clippy::too_many_arguments)]
pub fn mmp(
    oil_components: &[FluidComponent],
    gas_components: &[FluidComponent],
    eos: Option<&str>,
    mixing_rule: Option<&str>,
    temperature: ThermodynamicTemperature,
    method: Option<&str>,
    recovery_threshold: Option<f64>,
    number_of_pressure_points: Option<usize>,
    min_pressure: Option<Pressure>,
    max_pressure: Option<Pressure>,
    slim_tube_nodes: Option<usize>,
    pv_injected: Option<f64>,
) -> Result<MmpResult> {
    let cubic = cubic_named(eos)?;
    let kij = kij_named(mixing_rule)?;
    let oil = Fluid::from_rows(oil_components, cubic, kij, true)?;
    let gas = Fluid::from_rows(gas_components, cubic, kij, true)?;
    let t = temperature.value;
    if !(t.is_finite() && t > 0.0) {
        return Err(AzothError::invalid_input(
            "temperature",
            format!("an MMP needs a positive temperature; got {t}"),
        ));
    }
    let names = oil.mixture.names().unwrap_or(&[]);
    let gas_names = gas.mixture.names().ok_or_else(|| {
        AzothError::invalid_input(
            "injection_gas",
            "the gas mixture carries no component names",
        )
    })?;
    let mut gas_index = Vec::with_capacity(gas.mixture.len());
    let mut shared = 0usize;
    for (position, _component) in gas.mixture.components().iter().enumerate() {
        let name = &gas_names[position];
        match names.iter().position(|held| held == name) {
            Some(index) => {
                gas_index.push(index);
                shared += 1;
            }
            None => {
                return Err(AzothError::invalid_input(
                    "injection_gas",
                    format!(
                        "{name} is not a component of the oil, and the class injects only the \
                         components a cell already carries"
                    ),
                ));
            }
        }
    }
    if shared == 0 {
        return Err(AzothError::invalid_input(
            "injection_gas",
            "the injection gas shares no component with the oil",
        ));
    }

    let threshold = recovery_threshold.unwrap_or(MMP_DEFAULT_THRESHOLD);
    if !(threshold > 0.0 && threshold <= 1.0) {
        return Err(AzothError::invalid_input(
            "recovery_threshold",
            format!("the threshold is a recovery fraction in (0, 1]; got {threshold}"),
        ));
    }
    let points_count = number_of_pressure_points.unwrap_or(MMP_DEFAULT_PRESSURE_POINTS);
    if points_count < 3 {
        return Err(AzothError::invalid_input(
            "number_of_pressure_points",
            "the mechanism heuristic reads the second and the penultimate recovery, so there \
             must be at least three pressures",
        ));
    }
    let min_bar = min_pressure.map_or(MMP_DEFAULT_MIN_PRESSURE_BAR, |value| value.value) / 1.0e5;
    let max_bar = max_pressure.map_or(MMP_DEFAULT_MAX_PRESSURE_BAR, |value| value.value) / 1.0e5;
    if !(min_bar.is_finite() && min_bar > 0.0 && max_bar > min_bar) {
        return Err(AzothError::invalid_input(
            "first_stage",
            format!(
                "the pressure range must be positive and increasing; got {min_bar} to {max_bar} bar"
            ),
        ));
    }
    let nodes = slim_tube_nodes.unwrap_or(MMP_DEFAULT_NODES);
    if nodes < 1 {
        return Err(AzothError::invalid_input(
            "slim_tube_nodes",
            "the slim tube needs at least one cell",
        ));
    }
    let pv = pv_injected.unwrap_or(MMP_DEFAULT_PV_INJECTED);
    if !(pv.is_finite() && pv > 0.0) {
        return Err(AzothError::invalid_input(
            "pv_injected",
            format!("the injected volume is {pv} pore volumes"),
        ));
    }

    let chosen = method.unwrap_or("slim_tube");
    match chosen {
        "key_tie_line" => {
            let value = key_tie_line(&oil, &gas, &gas_index, t, min_bar, max_bar)?;
            Ok(MmpResult {
                mmp: Some(bar_to_pa(value)),
                mechanism: "combined".to_owned(),
                points: Vec::new(),
                warnings: Vec::new(),
            })
        }
        "slim_tube" | "rising_bubble" => {
            let mut warnings = Vec::new();
            let (pressures, recoveries) = slim_tube(
                &oil,
                &gas,
                &gas_index,
                t,
                min_bar,
                max_bar,
                points_count,
                nodes,
                pv,
                &mut warnings,
            )?;
            let interpolated = interpolate(&pressures, &recoveries, threshold);
            let mechanism = mechanism(&pressures, &recoveries, interpolated);
            Ok(MmpResult {
                mmp: interpolated.map(bar_to_pa),
                mechanism,
                points: pressures
                    .iter()
                    .zip(&recoveries)
                    .map(|(pressure, recovery)| MmpPoint {
                        pressure: bar_to_pa(*pressure),
                        recovery: *recovery,
                    })
                    .collect(),
                warnings,
            })
        }
        other => Err(AzothError::invalid_input(
            "method",
            format!("{other} is not one of slim_tube, key_tie_line and rising_bubble"),
        )),
    }
}

/// The gas phase's composition at a pressure, mapped into the oil's component order.
fn gas_composition(
    gas: &Fluid,
    gas_index: &[usize],
    t: f64,
    pressure_bar: f64,
    oil_len: usize,
) -> Result<Vec<f64>> {
    let flash = gas.flash(t, bar_to_pa(pressure_bar), FlashMode::TwoPhase)?;
    let composition = flash.gas().map_or_else(|| gas.z(), |phase| phase.x.clone());
    let mut mapped = vec![0.0; oil_len];
    for (component, index) in composition.iter().zip(gas_index) {
        mapped[*index] = *component;
    }
    Ok(mapped)
}

#[allow(clippy::too_many_arguments)]
fn slim_tube(
    oil: &Fluid,
    gas: &Fluid,
    gas_index: &[usize],
    t: f64,
    min_bar: f64,
    max_bar: f64,
    points: usize,
    nodes: usize,
    pv: f64,
    warnings: &mut Vec<Warning>,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let dp = (max_bar - min_bar) / (points - 1) as f64;
    let mut pressures = Vec::with_capacity(points);
    let mut recoveries = Vec::with_capacity(points);
    for index in 0..points {
        let pressure = min_bar + index as f64 * dp;
        pressures.push(pressure);
        recoveries.push(recovery_at(
            oil, gas, gas_index, t, pressure, nodes, pv, warnings,
        )?);
    }
    Ok((pressures, recoveries))
}

#[allow(clippy::too_many_arguments)]
fn recovery_at(
    oil: &Fluid,
    gas: &Fluid,
    gas_index: &[usize],
    t: f64,
    pressure_bar: f64,
    nodes: usize,
    pv: f64,
    warnings: &mut Vec<Warning>,
) -> Result<f64> {
    let pressure = bar_to_pa(pressure_bar);
    let oil_flash = oil.flash(t, pressure, FlashMode::TwoPhase)?;
    let gas_flash = gas.flash(t, pressure, FlashMode::TwoPhase)?;
    let injection = gas_composition(gas, gas_index, t, pressure_bar, oil.moles().len())?;

    // First-contact miscibility: the oil and the full gas amounts mix to one phase.
    if oil_flash.phase_count == 1 && gas_flash.phase_count == 1 {
        let mixture = mix(oil, &injection, gas.total_moles());
        if mixture.flash(t, pressure, FlashMode::TwoPhase)?.phase_count == 1 {
            return Ok(1.0);
        }
    }

    let oil_in_place = oil.total_moles();
    let mut cells = vec![oil.moles().to_vec(); nodes];
    let steps = (pv * nodes as f64) as usize;
    let gas_per_step = gas.total_moles() / nodes as f64;
    let mut recovered = 0.0;
    for _ in 0..steps {
        for (index, value) in injection.iter().enumerate() {
            cells[0][index] += gas_per_step * value;
        }
        for cell in 0..nodes {
            let fluid = oil.with_moles(cells[cell].clone());
            let flash = match fluid.flash(t, pressure, FlashMode::TwoPhase) {
                Ok(flash) => flash,
                Err(_) => continue,
            };
            if flash.phase_count > 1 && cell + 1 < nodes {
                if let Some(gas_phase) = flash.gas() {
                    let forward = 0.5 * gas_phase.fraction * flash.total_moles;
                    for (index, value) in gas_phase.x.iter().enumerate() {
                        cells[cell + 1][index] += forward * value;
                    }
                    if let Some(oil_phase) = flash.oil() {
                        let retained = oil_phase.fraction * flash.total_moles;
                        cells[cell] = oil_phase
                            .x
                            .iter()
                            .map(|fraction| fraction * retained)
                            .collect();
                    }
                }
            }
        }
        let last = oil.with_moles(cells[nodes - 1].clone());
        if let Ok(flash) = last.flash(t, pressure, FlashMode::TwoPhase) {
            if let Some(oil_phase) = flash.oil() {
                recovered += oil_phase.fraction * flash.total_moles * 0.1;
            }
        }
    }
    let recovery = (recovered / oil_in_place).clamp(0.0, 1.0);
    if !recovery.is_finite() {
        warnings.push(Warning::new(
            azoth_core::warning::WarningCode::OutOfValidRange,
            "the slim tube answered a non-finite recovery; the point is reported as zero",
        ));
        return Ok(0.0);
    }
    Ok(recovery)
}

fn mix(oil: &Fluid, gas_composition: &[f64], gas_moles: f64) -> Fluid {
    let mut moles = oil.moles().to_vec();
    for (index, value) in gas_composition.iter().enumerate() {
        moles[index] += gas_moles * value;
    }
    oil.with_moles(moles)
}

fn interpolate(pressures: &[f64], recoveries: &[f64], threshold: f64) -> Option<f64> {
    for index in 1..pressures.len() {
        if recoveries[index] >= threshold && recoveries[index - 1] < threshold {
            let slope = (pressures[index] - pressures[index - 1])
                / (recoveries[index] - recoveries[index - 1]);
            return Some(pressures[index - 1] + slope * (threshold - recoveries[index - 1]));
        }
    }
    if recoveries[0] >= threshold {
        return Some(pressures[0]);
    }
    if recoveries[recoveries.len() - 1] < threshold {
        return None;
    }
    Some(pressures[pressures.len() - 1])
}

fn mechanism(pressures: &[f64], recoveries: &[f64], mmp: Option<f64>) -> String {
    let Some(mmp) = mmp else {
        return "immiscible".to_owned();
    };
    let mut at_mmp = 0.0;
    for index in 0..pressures.len() - 1 {
        if pressures[index] <= mmp && pressures[index + 1] >= mmp {
            let fraction = (mmp - pressures[index]) / (pressures[index + 1] - pressures[index]);
            at_mmp = recoveries[index] + fraction * (recoveries[index + 1] - recoveries[index]);
            break;
        }
    }
    if at_mmp > 0.99 {
        return "first_contact".to_owned();
    }
    let low = recoveries[1];
    let mid = recoveries[recoveries.len() / 2];
    if low < 0.3 && mid < 0.6 {
        "vaporizing".to_owned()
    } else if low > 0.5 {
        "condensing".to_owned()
    } else {
        "combined".to_owned()
    }
}

fn key_tie_line(
    oil: &Fluid,
    gas: &Fluid,
    gas_index: &[usize],
    t: f64,
    min_bar: f64,
    max_bar: f64,
) -> Result<f64> {
    let injection = gas.z();
    let full = mix(
        oil,
        &{
            let mut mapped = vec![0.0; oil.moles().len()];
            for (value, index) in injection.iter().zip(gas_index) {
                mapped[*index] = *value;
            }
            mapped
        },
        gas.total_moles(),
    );
    let mut low = min_bar;
    let mut high = max_bar;
    while high - low > MMP_KEY_TIE_LINE_PRESSURE_TOLERANCE_BAR {
        let mid = (low + high) / 2.0;
        let length = tie_line_length(&full, t, mid)?;
        if length < MMP_KEY_TIE_LINE_LENGTH_TOLERANCE {
            high = mid;
        } else {
            low = mid;
        }
    }
    Ok((low + high) / 2.0)
}

fn tie_line_length(fluid: &Fluid, t: f64, pressure_bar: f64) -> Result<f64> {
    let flash = match fluid.flash(t, bar_to_pa(pressure_bar), FlashMode::TwoPhase) {
        Ok(flash) => flash,
        Err(_) => return Ok(1.0),
    };
    if flash.phase_count == 1 {
        return Ok(0.0);
    }
    let first = flash.phases.first().ok_or_else(|| {
        AzothError::invalid_input("flash", "the tie-line flash answered no phase")
    })?;
    let second = flash.phases.get(1).ok_or_else(|| {
        AzothError::invalid_input("flash", "the tie-line flash answered one phase")
    })?;
    Ok(first
        .x
        .iter()
        .zip(&second.x)
        .map(|(vapour, liquid)| (vapour - liquid) * (vapour - liquid))
        .sum::<f64>()
        .sqrt())
}
