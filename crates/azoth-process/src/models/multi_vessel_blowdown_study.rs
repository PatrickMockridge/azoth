//! `process.multi_vessel_blowdown_study` - the shared-header blowdown superposition.
//!
//! Spec: `specs/models/process/multi_vessel_blowdown_study.toml`. The arithmetic is
//! `process/safety/depressurization/MultiVesselBlowdownStudy.java` at the pin: the common
//! grid, the per-source interpolation, the peak total and the header Mach check.

use azoth_core::units::{
    Length, MassRate, MolarMass, Pressure, ThermodynamicTemperature, Time, Velocity,
    kilograms_per_second, meters_per_second, seconds,
};
use azoth_core::{AzothError, Result, Warning};
use serde::Serialize;

use crate::executor::json::{optional_scalar, scalar, warnings as wire_warnings};

/// The class's gas constant, J/(mol K).
const GAS_CONSTANT: f64 = 8.314;
/// The largest grid this port builds, a guard the class does not carry.
const MAX_GRID_ROWS: f64 = 1_000_000.0;

/// One precomputed sample of a source's discharge profile.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiVesselSample {
    /// The source's name.
    pub source: String,
    /// The sample's time.
    pub time: Time,
    /// The source's discharge rate.
    pub mass_flow: MassRate,
}

/// One grid row of the combined profile.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MultiVesselProfileRow {
    /// The grid time.
    #[serde(serialize_with = "scalar")]
    pub time: Time,
    /// The summed discharge rate.
    #[serde(serialize_with = "scalar")]
    pub total_mass_flow: MassRate,
}

/// One source's contribution at the peak.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MultiVesselContributionRow {
    /// The source's name.
    pub source: String,
    /// The source's rate at the peak time.
    #[serde(serialize_with = "scalar")]
    pub peak_contribution: MassRate,
}

/// Result of `process.multi_vessel_blowdown_study`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MultiVesselBlowdownStudyResult {
    /// The combined header load at each grid time.
    pub profile: Vec<MultiVesselProfileRow>,
    /// Each source's contribution at the peak.
    pub contributions: Vec<MultiVesselContributionRow>,
    /// The controlling combined load.
    #[serde(serialize_with = "scalar")]
    pub peak_total_mass_flow: MassRate,
    /// The time of the controlling load.
    #[serde(serialize_with = "scalar")]
    pub peak_time: Time,
    /// The header gas velocity at the peak.
    #[serde(serialize_with = "optional_scalar")]
    pub header_velocity: Option<Velocity>,
    /// The header Mach number at the peak.
    pub header_mach: Option<f64>,
    /// The Mach limit the run used.
    pub max_allowable_mach: f64,
    /// Whether the Mach number is within the limit.
    pub header_mach_acceptable: bool,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// A source's samples, in the order the input first named it.
struct SourceProfile {
    name: String,
    times: Vec<f64>,
    flows: Vec<f64>,
}

impl SourceProfile {
    fn interpolate(&self, target: f64) -> f64 {
        if target <= self.times[0] {
            return self.flows[0];
        }
        if target >= *self.times.last().expect("a source has at least one sample") {
            return 0.0;
        }
        for index in 1..self.times.len() {
            let upper = self.times[index];
            if target <= upper {
                let lower = self.times[index - 1];
                let fraction = if upper > lower {
                    (target - lower) / (upper - lower)
                } else {
                    0.0
                };
                return self.flows[index - 1]
                    + fraction * (self.flows[index] - self.flows[index - 1]);
            }
        }
        0.0
    }
}

/// The simultaneous blowdown of sources into a shared header.
///
/// # Errors
/// `InvalidInput` for no samples, a non-increasing source time, a non-finite sample, a
/// non-positive header diameter or grid step, or a grid larger than this port builds.
#[allow(clippy::too_many_arguments)] // one parameter per quantity the class reads
pub fn multi_vessel_blowdown_study(
    samples: &[MultiVesselSample],
    header_diameter: Option<Length>,
    header_pressure: Option<Pressure>,
    header_temperature: Option<ThermodynamicTemperature>,
    header_molar_mass: Option<MolarMass>,
    header_heat_capacity_ratio: Option<f64>,
    max_allowable_mach: Option<f64>,
    grid_step: Option<Time>,
) -> Result<MultiVesselBlowdownStudyResult> {
    let step = grid_step.map_or(1.0, |value| value.value);
    let limit = max_allowable_mach.unwrap_or(0.70);
    if samples.is_empty() {
        return Err(AzothError::invalid_input(
            "samples",
            "the class requires at least one blowdown source",
        ));
    }
    if step <= 0.0 || !step.is_finite() {
        return Err(AzothError::invalid_input(
            "grid_step",
            "the class refuses a non-positive grid step",
        ));
    }
    if header_diameter.is_some_and(|value| value.value <= 0.0) {
        return Err(AzothError::invalid_input(
            "header_diameter",
            "the class refuses a non-positive header diameter",
        ));
    }
    if !limit.is_finite() {
        return Err(AzothError::invalid_input(
            "max_allowable_mach",
            "a non-finite Mach limit is not a comparison",
        ));
    }

    let mut sources: Vec<SourceProfile> = Vec::new();
    for sample in samples {
        if !sample.time.value.is_finite() || !sample.mass_flow.value.is_finite() {
            return Err(AzothError::invalid_input(
                "samples",
                "a non-finite sample is not a state this calculation uses",
            ));
        }
        match sources
            .iter_mut()
            .find(|profile| profile.name == sample.source)
        {
            Some(profile) => {
                if profile
                    .times
                    .last()
                    .is_some_and(|last| sample.time.value <= *last)
                {
                    return Err(AzothError::invalid_input(
                        "samples",
                        "a source's times must be strictly increasing",
                    ));
                }
                profile.times.push(sample.time.value);
                profile.flows.push(sample.mass_flow.value);
            }
            None => sources.push(SourceProfile {
                name: sample.source.clone(),
                times: vec![sample.time.value],
                flows: vec![sample.mass_flow.value],
            }),
        }
    }

    let end = sources
        .iter()
        .filter_map(|profile| profile.times.last().copied())
        .fold(0.0_f64, f64::max);
    let declared = end / step;
    if declared > MAX_GRID_ROWS {
        return Err(AzothError::invalid_input(
            "grid_step",
            format!(
                "the grid would have {declared} rows, and this port builds at most {MAX_GRID_ROWS}"
            ),
        ));
    }

    let mut profile = Vec::new();
    let mut peak_total = 0.0;
    let mut peak_time = 0.0;
    let mut peak_index = 0;
    let mut per_source: Vec<Vec<f64>> = sources.iter().map(|_| Vec::new()).collect();
    let mut time = 0.0;
    let mut index = 0;
    while time <= end + 1.0e-9 {
        let mut total = 0.0;
        for (at, source) in sources.iter().enumerate() {
            let flow = source.interpolate(time);
            per_source[at].push(flow);
            total += flow;
        }
        if total > peak_total {
            peak_total = total;
            peak_time = time;
            peak_index = index;
        }
        profile.push(MultiVesselProfileRow {
            time: seconds(time),
            total_mass_flow: kilograms_per_second(total),
        });
        time += step;
        index += 1;
    }
    let contributions: Vec<MultiVesselContributionRow> = sources
        .iter()
        .enumerate()
        .map(|(at, source)| MultiVesselContributionRow {
            source: source.name.clone(),
            peak_contribution: kilograms_per_second(per_source[at][peak_index]),
        })
        .collect();

    let (velocity, mach) = match header_diameter {
        Some(diameter) => {
            let pressure = header_pressure.map_or(1.5e5, |value| value.value);
            let temperature = header_temperature.map_or(288.15, |value| value.value);
            let molar_mass = header_molar_mass.map_or(0.020, |value| value.value);
            let gamma = header_heat_capacity_ratio.unwrap_or(1.30);
            let area = std::f64::consts::PI * 0.25 * diameter.value * diameter.value;
            let density = pressure * molar_mass / (GAS_CONSTANT * temperature);
            let velocity = peak_total / (density * area);
            let sonic = (gamma * GAS_CONSTANT * temperature / molar_mass).sqrt();
            (Some(velocity), Some(velocity / sonic))
        }
        None => (None, None),
    };
    let acceptable = mach.is_none_or(|value| value <= limit);

    Ok(MultiVesselBlowdownStudyResult {
        profile,
        contributions,
        peak_total_mass_flow: kilograms_per_second(peak_total),
        peak_time: seconds(peak_time),
        header_velocity: velocity.map(meters_per_second),
        header_mach: mach,
        max_allowable_mach: limit,
        header_mach_acceptable: acceptable,
        warnings: Vec::new(),
    })
}
