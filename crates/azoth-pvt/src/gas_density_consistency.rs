//! `pvt.gas_density_consistency` - the gas-density identity over observed intervals.
//!
//! Spec: `specs/models/pvt/gas_density_consistency.toml`. The arithmetic is
//! `pvtsimulation/util/PvtGasDensityConsistency.java` at the pin: `rho = p M / (Z R T)` with
//! outward-rounded closed intervals and the class's exact gas constant.

use azoth_core::{AzothError, Result, Warning};

/// The class's gas constant, in J/(mol K); exactly this literal.
const GAS_CONSTANT: f64 = 8.31446261815324;

/// One gas-phase quantity and its nonnegative absolute half-width, in the field's SI unit.
#[derive(Debug, Clone, PartialEq)]
pub struct GasDensityObservation {
    /// The pressure's nominal value and absolute bound, in Pa.
    pub pressure: f64,
    /// The pressure's absolute bound, in Pa.
    pub pressure_bound: f64,
    /// The temperature's nominal value and absolute bound, in K.
    pub temperature: f64,
    /// The temperature's absolute bound, in K.
    pub temperature_bound: f64,
    /// The compressibility factor's nominal value and absolute bound.
    pub compressibility: f64,
    /// The compressibility factor's absolute bound.
    pub compressibility_bound: f64,
    /// The molar mass's nominal value and absolute bound, in kg/mol.
    pub molar_mass: f64,
    /// The molar mass's absolute bound, in kg/mol.
    pub molar_mass_bound: f64,
    /// The observed density's nominal value and absolute bound, in kg/m3.
    pub density: f64,
    /// The observed density's absolute bound, in kg/m3.
    pub density_bound: f64,
}

/// One row's implied density and verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct GasDensityRow {
    /// The density implied by the nominal values, in kg/m3.
    pub implied_density: f64,
    /// The conservative lower end of the implied interval, in kg/m3.
    pub implied_lower: f64,
    /// The conservative upper end of the implied interval, in kg/m3.
    pub implied_upper: f64,
    /// The observed nominal density minus the implied nominal, in kg/m3.
    pub observed_minus_implied: f64,
    /// Whether the observed interval overlaps the implied one, endpoints included.
    pub compatible: bool,
}

/// Result of `pvt.gas_density_consistency`.
#[derive(Debug, Clone, PartialEq)]
pub struct GasDensityConsistencyResult {
    /// One row per observation, in caller order.
    pub rows: Vec<GasDensityRow>,
    /// Whether every row is compatible.
    pub all_compatible: bool,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The next representable `f64` above `value`: `Math.nextUp`.
fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

/// The next representable `f64` below `value`: `Math.nextDown`.
fn next_down(value: f64) -> f64 {
    -next_up(-value)
}

/// One closed interval `[value - bound, value + bound]` with the class's outward rounding.
#[derive(Clone, Copy)]
struct Interval {
    value: f64,
    bound: f64,
}

impl Interval {
    fn new(value: f64, bound: f64, field: &str) -> Result<Self> {
        if !value.is_finite()
            || !bound.is_finite()
            || bound < 0.0
            || value - bound <= 0.0
            || !(value + bound).is_finite()
            || (bound > 0.0 && !next_up(value + bound).is_finite())
        {
            return Err(AzothError::invalid_input(
                field,
                "a quantity needs finite positive endpoints and a nonnegative bound",
            ));
        }
        Ok(Self { value, bound })
    }

    /// The lower endpoint, rounded outward when the bound is nonzero.
    fn lower(self) -> f64 {
        if self.bound == 0.0 {
            self.value
        } else {
            next_down(self.value - self.bound).max(f64::from_bits(1))
        }
    }

    /// The upper endpoint, rounded outward when the bound is nonzero.
    fn upper(self) -> f64 {
        if self.bound == 0.0 {
            self.value
        } else {
            next_up(self.value + self.bound)
        }
    }
}

fn round(value: f64, upper: bool) -> f64 {
    if upper {
        next_up(value)
    } else {
        next_down(value)
    }
}

fn endpoint(
    pressure: Interval,
    temperature: Interval,
    compressibility: Interval,
    molar_mass: Interval,
    upper: bool,
) -> f64 {
    let p = if upper {
        pressure.upper()
    } else {
        pressure.lower()
    };
    let t = if upper {
        temperature.lower()
    } else {
        temperature.upper()
    };
    let z = if upper {
        compressibility.lower()
    } else {
        compressibility.upper()
    };
    let m = if upper {
        molar_mass.upper()
    } else {
        molar_mass.lower()
    };
    let value = round(p * m, upper);
    let value = round(value / z, upper);
    let value = round(value / GAS_CONSTANT, upper);
    round(value / t, upper)
}

/// Propagate each observation's gas-density interval and test the observed one against it.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for a non-finite or non-positive nominal value, a negative
///   or unrepresentable bound, or an implied interval that is not a positive finite pair.
pub fn gas_density_consistency(
    observations: &[GasDensityObservation],
) -> Result<GasDensityConsistencyResult> {
    if observations.is_empty() {
        return Err(AzothError::invalid_input(
            "observations",
            "at least one observation is required",
        ));
    }
    let mut rows = Vec::with_capacity(observations.len());
    for observation in observations {
        let pressure = Interval::new(observation.pressure, observation.pressure_bound, "pressure")?;
        let temperature = Interval::new(
            observation.temperature,
            observation.temperature_bound,
            "temperature",
        )?;
        let compressibility = Interval::new(
            observation.compressibility,
            observation.compressibility_bound,
            "compressibility",
        )?;
        let molar_mass = Interval::new(
            observation.molar_mass,
            observation.molar_mass_bound,
            "molar_mass",
        )?;
        let density = Interval::new(observation.density, observation.density_bound, "density")?;

        let implied_density = pressure.value * molar_mass.value
            / compressibility.value
            / GAS_CONSTANT
            / temperature.value;
        let implied_lower = endpoint(pressure, temperature, compressibility, molar_mass, false);
        let implied_upper = endpoint(pressure, temperature, compressibility, molar_mass, true);
        if !implied_density.is_finite()
            || implied_density <= 0.0
            || implied_lower <= 0.0
            || !implied_upper.is_finite()
        {
            return Err(AzothError::invalid_input(
                "density",
                "the gas-density interval cannot be represented as positive finite doubles",
            ));
        }
        rows.push(GasDensityRow {
            implied_density,
            implied_lower,
            implied_upper,
            observed_minus_implied: density.value - implied_density,
            compatible: density.lower() <= implied_upper && density.upper() >= implied_lower,
        });
    }
    let all_compatible = rows.iter().all(|row| row.compatible);
    Ok(GasDensityConsistencyResult {
        rows,
        all_compatible,
        warnings: Vec::new(),
    })
}
