//! `pvt.van_everdingen_hurst_aquifer` - the radial aquifer influence functions.
//!
//! Spec: `specs/models/pvt/van_everdingen_hurst_aquifer.toml`. The arithmetic is
//! `pvtsimulation/reservoirproperties/materialbalance/VanEverdingenHurstAquifer.java` at the
//! pin, with the spectral kernel in `RadialInfiniteInfluence.java` carried inside this module:
//! that helper is package-private and its only main caller is the aquifer, so it is the
//! calculation's arithmetic rather than a separate id.

use std::sync::OnceLock;

use azoth_core::units::{Area, DynamicViscosity, Length, Time};
use azoth_core::{AzothError, Result, Warning};

/// Euler's constant in the small-frequency Bessel expansions.
const EULER: f64 = 0.5772156649015329;
/// Lower logarithmic frequency of the resolved band.
const LOWER_LOG: f64 = -40.0;
/// Upper frequency of the resolved band.
const UPPER: f64 = 64.0;
/// Taylor boundary of the exponential integral.
const SMALL_ARGUMENT: f64 = 1.0e-3;
/// Exponential boundary where the pressure omission is below 1.16e-33.
const NEGLIGIBLE_ARGUMENT: f64 = 80.0;
/// Tail endpoint; the omitted pressure is below 2.6e-183 at the smallest positive time.
const TAIL_UPPER_LOG: f64 = 420.0;
/// Reciprocal Bessel-modulus asymptotic coefficients, in increasing even powers.
const TAIL: [f64; 5] = [
    1.0,
    -3.0 / 8.0,
    63.0 / 128.0,
    -1899.0 / 1024.0,
    543483.0 / 32768.0,
];
/// Prefix panel count; the fallback retains every original node.
const PREFIX_PANELS: usize = 96;
/// First-call tables.
static TABLES: OnceLock<Tables> = OnceLock::new();
/// The complete tail, built only when a query needs it.
static FULL_TAIL: OnceLock<Tail> = OnceLock::new();

/// Result of `pvt.van_everdingen_hurst_aquifer`.
#[derive(Debug, Clone, PartialEq)]
pub struct VanEverdingenHurstAquiferResult {
    /// The dimensionless pressure at the stated time and radius.
    pub dimensionless_pressure: Option<f64>,
    /// The dimensionless pressure derivative at the stated time and radius.
    pub dimensionless_pressure_derivative: Option<f64>,
    /// The dimensionless time from the physical inputs.
    pub dimensionless_time: Option<f64>,
    /// The aquifer constant, in reservoir m3 per Pa.
    pub aquifer_constant: Option<f64>,
    /// The influence table's dimensionless pressures.
    pub dimensionless_pressures: Option<Vec<f64>>,
    /// The Carter-Tracy cumulative water influx history, m3.
    pub cumulative_influx: Option<Vec<f64>>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// One positive quadrature grid.
struct Tail {
    /// Frequencies, positive and increasing.
    nodes: Vec<f64>,
    /// Positive pressure weights.
    weights: Vec<f64>,
    /// Conservative maximum of every weight from this node to the end.
    suffix_max: Vec<f64>,
}

/// The kernel's fixed grids.
struct Tables {
    /// Low-frequency regular panel on zero to one.
    low_quadrature: (Vec<f64>, Vec<f64>),
    /// The small-frequency integral at fifty, the monotone continuation anchor.
    ein_at_fifty: f64,
    /// Squared frequencies and pressure weights of the resolved band.
    quadrature: (Vec<f64>, Vec<f64>),
    /// Initial tail segment.
    tail_prefix: Tail,
}

/// The radial influence kernel's pressure and its analytic derivative.
fn evaluate(time: f64) -> Result<(f64, f64)> {
    if !time.is_finite() || time < 0.0 {
        return Err(AzothError::invalid_input(
            "dimensionless_time",
            "the dimensionless aquifer time must be finite and nonnegative",
        ));
    }
    if time == 0.0 {
        return Ok((0.0, 0.0));
    }
    let tables = tables();
    let x = time * lower_squared();
    let mut low = if x > 50.0 {
        (0.5 * (tables.ein_at_fifty + (x / 50.0).ln()), 0.5 / time)
    } else {
        small_frequency_integral_with(&tables.low_quadrature, x)
    };
    if x <= 50.0 {
        low.0 *= 0.5;
        low.1 *= 0.5 * lower_squared();
    }
    let mut pressure = low.0;
    let mut derivative = low.1;
    for index in 0..tables.quadrature.0.len() {
        let frequency_squared = tables.quadrature.0[index];
        let argument = time * frequency_squared;
        let (loss, slope) = loss_and_slope(argument);
        pressure += tables.quadrature.1[index] * loss;
        derivative += tables.quadrature.1[index] * frequency_squared * slope;
    }
    let root_time = time.sqrt();
    let mut tail: &'static Tail = &tables.tail_prefix;
    for index in 0..tail_panels() * 8 {
        if index == tail.nodes.len() {
            tail = full_tail();
        }
        let scaled_frequency = root_time * tail.nodes[index];
        let argument = scaled_frequency * scaled_frequency;
        if argument >= NEGLIGIBLE_ARGUMENT && tail.suffix_max[index] < 0.25 * ulp(pressure) {
            break;
        }
        let weight = tail.weights[index];
        if argument >= NEGLIGIBLE_ARGUMENT {
            pressure += weight;
        } else {
            let (loss, slope) = loss_and_slope(argument);
            pressure += weight * loss;
            derivative += weight * argument * slope / time;
        }
    }
    Ok((pressure, derivative))
}

/// The loss and its slope for one kernel argument.
fn loss_and_slope(argument: f64) -> (f64, f64) {
    if argument <= SMALL_ARGUMENT {
        (
            argument * (1.0 - argument * (0.5 - argument * (1.0 / 6.0 - argument / 24.0))),
            1.0 - argument * (1.0 - argument * (0.5 - argument / 6.0)),
        )
    } else if argument >= NEGLIGIBLE_ARGUMENT {
        (1.0, 0.0)
    } else {
        (-((-argument).exp_m1()), (-argument).exp())
    }
}

/// Ein(x) and its derivative, by Taylor series or the regular low-frequency panel.
fn small_frequency_integral_with(low: &(Vec<f64>, Vec<f64>), x: f64) -> (f64, f64) {
    if x <= SMALL_ARGUMENT {
        return (
            x * (1.0 - x * (0.25 - x * (1.0 / 18.0 - x / 96.0))),
            1.0 - x * (0.5 - x * (1.0 / 6.0 - x / 24.0)),
        );
    }
    let mut value = 0.0;
    let mut derivative = 0.0;
    for index in 0..low.0.len() {
        let node = low.0[index];
        let weight = low.1[index];
        value += weight * -((-x * node).exp_m1()) / node;
        derivative += weight * (-(x * node)).exp();
    }
    (value, derivative)
}

/// The fixed grids, built once.
fn tables() -> &'static Tables {
    TABLES.get_or_init(|| {
        let low_quadrature = gauss_panels(0.0, 1.0, 16, 16);
        let ein_at_fifty = small_frequency_integral_with(&low_quadrature, 50.0).0;
        let quadrature = build_quadrature();
        let tail_prefix = tail_quadrature(PREFIX_PANELS);
        Tables {
            low_quadrature,
            ein_at_fifty,
            quadrature,
            tail_prefix,
        }
    })
}

/// The complete tail grid, built on first need.
fn full_tail() -> &'static Tail {
    FULL_TAIL.get_or_init(|| tail_quadrature(tail_panels()))
}

/// The number of tail panels, from the interval and the half-log width.
fn tail_panels() -> usize {
    ((TAIL_UPPER_LOG - UPPER.ln()) / 0.5).ceil() as usize
}

/// Squared frequencies and pressure weights of the resolved band.
fn build_quadrature() -> (Vec<f64>, Vec<f64>) {
    let panels = ((UPPER.ln() - LOWER_LOG) / 0.5).ceil() as usize;
    let (mut nodes, mut weights) = gauss_panels(LOWER_LOG, UPPER.ln(), panels, panels);
    for index in 0..nodes.len() {
        let u = nodes[index].exp();
        nodes[index] = u * u;
        weights[index] *=
            4.0 / (std::f64::consts::PI * std::f64::consts::PI * u * u * modulus_squared(u));
    }
    (nodes, weights)
}

/// One tail segment with its conservative suffix bound.
fn tail_quadrature(panels_to_build: usize) -> Tail {
    let (nodes, ordinary) =
        gauss_panels(UPPER.ln(), TAIL_UPPER_LOG, tail_panels(), panels_to_build);
    let mut frequencies = vec![0.0; nodes.len()];
    let mut weights = vec![0.0; nodes.len()];
    let mut suffix_max = vec![0.0; nodes.len()];
    for index in 0..nodes.len() {
        let u = nodes[index].exp();
        let inverse_squared = (1.0 / u) / u;
        let mut polynomial = TAIL[TAIL.len() - 1];
        for k in (0..TAIL.len() - 1).rev() {
            polynomial = TAIL[k] + inverse_squared * polynomial;
        }
        frequencies[index] = u;
        weights[index] = (2.0 / std::f64::consts::PI) * ordinary[index] / u * polynomial;
    }
    let boundary =
        UPPER.ln() + (TAIL_UPPER_LOG - UPPER.ln()) / tail_panels() as f64 * panels_to_build as f64;
    let mut maximum = if panels_to_build == tail_panels() {
        0.0
    } else {
        (-boundary).exp()
    };
    for index in (0..weights.len()).rev() {
        maximum = maximum.max(weights[index]);
        suffix_max[index] = maximum;
    }
    Tail {
        nodes: frequencies,
        weights,
        suffix_max,
    }
}

/// Eight-point Gauss-Legendre panels on an interval, building a prefix of the global grid.
fn gauss_panels(
    lower: f64,
    upper: f64,
    panels: usize,
    panels_to_build: usize,
) -> (Vec<f64>, Vec<f64>) {
    const NODES: [f64; 8] = [
        -0.9602898564975363,
        -0.7966664774136267,
        -0.525532409916329,
        -0.1834346424956498,
        0.1834346424956498,
        0.525532409916329,
        0.7966664774136267,
        0.9602898564975363,
    ];
    const WEIGHTS: [f64; 8] = [
        0.1012285362903763,
        0.2223810344533745,
        0.3137066458778873,
        0.362683783378362,
        0.362683783378362,
        0.3137066458778873,
        0.2223810344533745,
        0.1012285362903763,
    ];
    let width = (upper - lower) / panels as f64;
    let mut nodes = Vec::with_capacity(panels_to_build * 8);
    let mut weights = Vec::with_capacity(panels_to_build * 8);
    for panel in 0..panels_to_build {
        for (node, weight) in NODES.iter().zip(WEIGHTS.iter()) {
            nodes.push(lower + width * (panel as f64 + 0.5 * (node + 1.0)));
            weights.push(weight * width * 0.5);
        }
    }
    (nodes, weights)
}

/// J1(u)^2 + Y1(u)^2, by its small-argument series or the nonoscillatory modulus.
fn modulus_squared(u: f64) -> f64 {
    if u < 12.0 {
        let mut term = 0.5 * u;
        let mut j = term;
        let mut harmonic = 0.0;
        let mut sum = term * (1.0 - 2.0 * EULER);
        for k in 1..100 {
            term *= -0.25 * u * u / (k as f64 * (k as f64 + 1.0));
            harmonic += 1.0 / k as f64;
            j += term;
            sum += term * (2.0 * harmonic + 1.0 / (k as f64 + 1.0) - 2.0 * EULER);
            if term.abs() < 1.0e-20 {
                let y = (2.0 * (0.5 * u).ln() * j - 2.0 / u - sum) / std::f64::consts::PI;
                return j * j + y * y;
            }
        }
        panic!("aquifer Bessel modulus series did not converge");
    }
    let mut p = 1.0;
    let mut q = 0.0;
    let mut term = 1.0;
    let mut previous = 1.0;
    for k in 1..=64 {
        let odd = 2.0 * k as f64 - 1.0;
        term *= (4.0 - odd * odd) / (8.0 * k as f64 * u);
        if term.abs() > previous {
            break;
        }
        let signed = if (k / 2) % 2 == 0 { term } else { -term };
        if k % 2 == 0 {
            p += signed;
        } else {
            q += signed;
        }
        previous = term.abs();
    }
    2.0 / (std::f64::consts::PI * u) * (p * p + q * q)
}

/// The dimensionless time, with the class's exponent-scaled fallback.
///
/// # Errors
/// [`AzothError::InvalidInput`] for a physical input outside its stated domain, or a
/// strictly positive result this arithmetic cannot represent.
pub fn dimensionless_time(
    permeability: f64,
    time: f64,
    porosity: f64,
    viscosity: f64,
    total_compressibility: f64,
    reservoir_radius: f64,
) -> Result<f64> {
    if !permeability.is_finite()
        || permeability < 0.0
        || !time.is_finite()
        || time < 0.0
        || !porosity.is_finite()
        || porosity <= 0.0
        || porosity > 1.0
        || !viscosity.is_finite()
        || viscosity <= 0.0
        || !total_compressibility.is_finite()
        || total_compressibility <= 0.0
        || !reservoir_radius.is_finite()
        || reservoir_radius <= 0.0
    {
        return Err(AzothError::invalid_input(
            "dimensionless_time",
            "the aquifer time needs finite nonnegative permeability and time, porosity in (0,1], \
             and positive viscosity, compressibility (1/Pa) and inner radius",
        ));
    }
    if permeability == 0.0 || time == 0.0 {
        return Ok(0.0);
    }
    let numerator = permeability * time;
    let first = porosity * viscosity;
    let second = first * total_compressibility;
    let third = second * reservoir_radius;
    let denominator = third * reservoir_radius;
    let result = numerator / denominator;
    if normal_positive(numerator)
        && normal_positive(first)
        && normal_positive(second)
        && normal_positive(third)
        && normal_positive(denominator)
        && result > 2.0 * f64::MIN_POSITIVE
        && result < 0.5 * f64::MAX
    {
        return Ok(result);
    }
    scaled_physical_ratio(
        &[permeability, time],
        &[
            porosity,
            viscosity,
            total_compressibility,
            reservoir_radius,
            reservoir_radius,
        ],
    )
}

/// The aquifer constant `U = 2 pi f phi c_t h r_i^2`, in reservoir m3 per Pa.
///
/// # Errors
/// [`AzothError::InvalidInput`] for a physical input outside its stated domain, or a
/// strictly positive result this arithmetic cannot represent.
pub fn aquifer_constant(
    porosity: f64,
    total_compressibility: f64,
    thickness: f64,
    reservoir_radius: f64,
    encroachment_angle: f64,
) -> Result<f64> {
    if !porosity.is_finite()
        || !(0.0..=1.0).contains(&porosity)
        || !total_compressibility.is_finite()
        || total_compressibility < 0.0
        || !thickness.is_finite()
        || thickness < 0.0
        || !reservoir_radius.is_finite()
        || reservoir_radius <= 0.0
        || !encroachment_angle.is_finite()
        || !(0.0..=360.0).contains(&encroachment_angle)
    {
        return Err(AzothError::invalid_input(
            "aquifer_constant",
            "the aquifer storage needs finite porosity in [0,1], nonnegative compressibility \
             (1/Pa) and thickness, positive inner radius and an angle in [0,360] degrees",
        ));
    }
    if porosity == 0.0
        || total_compressibility == 0.0
        || thickness == 0.0
        || encroachment_angle == 0.0
    {
        return Ok(0.0);
    }
    let fraction = encroachment_angle / 360.0;
    let first = 2.0 * std::f64::consts::PI * fraction;
    let second = first * porosity;
    let third = second * total_compressibility;
    let fourth = third * thickness;
    let fifth = fourth * reservoir_radius;
    let result = fifth * reservoir_radius;
    if normal_positive(fraction)
        && normal_positive(first)
        && normal_positive(second)
        && normal_positive(third)
        && normal_positive(fourth)
        && normal_positive(fifth)
        && result > 2.0 * f64::MIN_POSITIVE
        && result < 0.5 * f64::MAX
    {
        return Ok(result);
    }
    scaled_physical_ratio(
        &[
            2.0 * std::f64::consts::PI,
            encroachment_angle,
            porosity,
            total_compressibility,
            thickness,
            reservoir_radius,
            reservoir_radius,
        ],
        &[360.0],
    )
}

/// The infinite-acting pressure, or the class's legacy bounded-outer-radius maximum.
///
/// # Errors
/// [`AzothError::InvalidInput`] for a negative or non-finite time, or a NaN radius.
pub fn dimensionless_pressure(time: f64, outer_radius: Option<f64>) -> Result<f64> {
    Ok(influence(time, outer_radius)?.0)
}

/// The derivative of the selected dimensionless-pressure branch.
///
/// # Errors
/// The same as [`dimensionless_pressure`].
pub fn dimensionless_pressure_derivative(time: f64, outer_radius: Option<f64>) -> Result<f64> {
    Ok(influence(time, outer_radius)?.1)
}

/// The selected pressure and derivative pair.
fn influence(time: f64, outer_radius: Option<f64>) -> Result<(f64, f64)> {
    let radius = outer_radius.unwrap_or(f64::INFINITY);
    if radius.is_nan() {
        return Err(AzothError::invalid_input(
            "dimensionless_outer_radius",
            "the aquifer outer-radius argument must not be NaN",
        ));
    }
    let mut result = evaluate(time)?;
    if time == 0.0 || radius <= 0.0 || radius.is_infinite() {
        return Ok(result);
    }
    let denominator = radius * radius - 1.0;
    if denominator > 1.0e-30 {
        let bounded = 2.0 * time / denominator + radius.ln() - 0.75;
        if bounded > result.0 {
            result = (bounded, 2.0 / denominator);
        }
    }
    Ok(result)
}

/// The Carter-Tracy cumulative water influx history.
///
/// # Errors
/// [`AzothError::InvalidInput`] for an empty or unequal history, a non-increasing time, a
/// negative or non-finite aquifer constant, a NaN radius, or an influence or transition the
/// recurrence cannot represent.
pub fn cumulative_influx_carter_tracy(
    dimensionless_times: &[f64],
    pressure_drops: &[f64],
    aquifer_constant: f64,
    outer_radius: Option<f64>,
) -> Result<Vec<f64>> {
    if dimensionless_times.is_empty() || dimensionless_times.len() != pressure_drops.len() {
        return Err(AzothError::invalid_input(
            "dimensionless_times",
            "the Carter-Tracy history needs nonempty times and drops of equal length",
        ));
    }
    if !aquifer_constant.is_finite() || aquifer_constant < 0.0 {
        return Err(AzothError::invalid_input(
            "aquifer_constant_value",
            "the aquifer constant must be finite and nonnegative",
        ));
    }
    if outer_radius.is_some_and(f64::is_nan) {
        return Err(AzothError::invalid_input(
            "dimensionless_outer_radius",
            "the aquifer outer-radius argument must not be NaN",
        ));
    }
    for index in 0..dimensionless_times.len() {
        if !dimensionless_times[index].is_finite()
            || dimensionless_times[index] < 0.0
            || (index > 0 && dimensionless_times[index] <= dimensionless_times[index - 1])
            || !pressure_drops[index].is_finite()
        {
            return Err(AzothError::invalid_input(
                format!("dimensionless_times[{index}]"),
                "times must be finite, nonnegative and strictly increasing, and drops finite",
            ));
        }
    }
    let mut influx = vec![0.0; dimensionless_times.len()];
    if aquifer_constant == 0.0 {
        return Ok(influx);
    }
    for index in 1..dimensionless_times.len() {
        let (pd, pd_prime) = influence(dimensionless_times[index], outer_radius)?;
        if !pd.is_finite() || pd <= 0.0 || !pd_prime.is_finite() {
            return Err(AzothError::invalid_input(
                format!("dimensionless_times[{index}]"),
                "the Carter-Tracy influence is not finite and positive at this index",
            ));
        }
        let elasticity = (dimensionless_times[index] * pd_prime) / pd;
        let denominator =
            1.0 - (dimensionless_times[index - 1] / dimensionless_times[index]) * elasticity;
        let retained = (1.0 - elasticity) / denominator;
        if denominator == 0.0 || !retained.is_finite() {
            return Err(AzothError::invalid_input(
                format!("dimensionless_times[{index}]"),
                "the Carter-Tracy transition is singular or unrepresentable at this index",
            ));
        }
        let next = retained * influx[index - 1]
            + scaled_product_ratio(
                dimensionless_times[index] - dimensionless_times[index - 1],
                aquifer_constant,
                pressure_drops[index],
                pd,
                denominator,
            );
        influx[index] = if next < 0.0 { 0.0 } else { next };
        if !influx[index].is_finite() {
            return Err(AzothError::invalid_input(
                format!("dimensionless_times[{index}]"),
                "the Carter-Tracy cumulative influx is unrepresentable at this index",
            ));
        }
    }
    Ok(influx)
}

/// The dimensionless pressures of an influence table.
///
/// # Errors
/// The same refusals as [`dimensionless_pressure`], at the first refused time.
pub fn influence_table(dimensionless_times: &[f64], outer_radius: Option<f64>) -> Result<Vec<f64>> {
    dimensionless_times
        .iter()
        .map(|time| dimensionless_pressure(*time, outer_radius))
        .collect()
}

/// The three numerator and two denominator factors, without intermediate exponent loss.
fn scaled_product_ratio(
    first: f64,
    second: f64,
    third: f64,
    pressure: f64,
    normalized_denominator: f64,
) -> f64 {
    if first == 0.0 || second == 0.0 || third == 0.0 {
        return 0.0;
    }
    let first_exponent = get_exponent(first);
    let second_exponent = get_exponent(second);
    let third_exponent = get_exponent(third);
    let pressure_exponent = get_exponent(pressure);
    let denominator_exponent = get_exponent(normalized_denominator);
    let mantissa = scalb(first, -first_exponent)
        * scalb(second, -second_exponent)
        * scalb(third, -third_exponent)
        / scalb(pressure, -pressure_exponent)
        / scalb(normalized_denominator, -denominator_exponent);
    scalb(
        mantissa,
        first_exponent + second_exponent + third_exponent
            - pressure_exponent
            - denominator_exponent,
    )
}

/// The class's exponent-scaled positive conversion.
fn scaled_physical_ratio(numerator: &[f64], denominator: &[f64]) -> Result<f64> {
    let mut mantissa = 1.0;
    let mut exponent = 0i32;
    for &factor in numerator {
        let power = get_exponent(factor);
        mantissa *= scalb(factor, -power);
        exponent += power;
    }
    for &factor in denominator {
        let power = get_exponent(factor);
        mantissa /= scalb(factor, -power);
        exponent -= power;
    }
    let result = scalb(mantissa, exponent);
    if result <= 2.0 * f64::MIN_POSITIVE || result >= 0.5 * f64::MAX || !result.is_finite() {
        return Err(AzothError::invalid_input(
            "aquifer_conversion",
            "the positive conversion sits at a double precision boundary the class resolves with \
             exact decimal arithmetic, which this port does not carry",
        ));
    }
    if result == 0.0 || !result.is_finite() {
        return Err(AzothError::invalid_input(
            "aquifer_conversion",
            "the positive conversion is not representable",
        ));
    }
    Ok(result)
}

/// Whether a value is finite and at least the smallest normal.
fn normal_positive(value: f64) -> bool {
    value.is_finite() && value >= f64::MIN_POSITIVE
}

/// The squared lower frequency of the resolved band.
fn lower_squared() -> f64 {
    (2.0 * LOWER_LOG).exp()
}

/// `Math.getExponent`: the unbiased binary exponent, -1023 for subnormal or zero, 1024 for
/// infinity or NaN.
fn get_exponent(value: f64) -> i32 {
    let bits = value.to_bits() & 0x7fff_ffff_ffff_ffff;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0 {
        -1023
    } else {
        exponent - 1023
    }
}

/// `Math.scalb`: `value * 2^power`, in steps that do not overflow an intermediate.
fn scalb(value: f64, power: i32) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    let mut scaled = value;
    let mut exponent = power;
    while exponent > 1023 {
        scaled *= f64::from_bits(0x7fe0_0000_0000_0000);
        exponent -= 1023;
    }
    while exponent < -1022 {
        scaled *= f64::from_bits(0x0010_0000_0000_0000);
        exponent += 1022;
    }
    scaled * f64::from_bits(((1023 + exponent) as u64) << 52)
}

/// `Math.ulp`: the spacing from the value to the next double.
fn ulp(value: f64) -> f64 {
    if value.is_nan() {
        return f64::NAN;
    }
    if value.is_infinite() {
        return f64::INFINITY;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    let next = if value > 0.0 {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    };
    (next - value).abs()
}

/// The aquifer calculation over the stated inputs.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown calculation, a missing required input, a
///   physical input outside its domain, a refused Carter-Tracy history or an unrepresentable
///   answer.
#[allow(clippy::too_many_arguments)]
pub fn van_everdingen_hurst_aquifer(
    calculation: &str,
    dimensionless_time_value: Option<f64>,
    dimensionless_times: Option<&[f64]>,
    dimensionless_outer_radius: Option<f64>,
    permeability: Option<Area>,
    time: Option<Time>,
    porosity: Option<f64>,
    viscosity: Option<DynamicViscosity>,
    total_compressibility: Option<f64>,
    reservoir_radius: Option<Length>,
    thickness: Option<Length>,
    encroachment_angle: Option<f64>,
    aquifer_constant_value: Option<f64>,
    pressure_drop: Option<&[f64]>,
) -> Result<VanEverdingenHurstAquiferResult> {
    let mut row = empty();
    match calculation {
        "dimensionless_pressure" => {
            let td = required("dimensionless_time_value", dimensionless_time_value)?;
            row.dimensionless_pressure =
                Some(dimensionless_pressure(td, dimensionless_outer_radius)?);
        }
        "dimensionless_pressure_derivative" => {
            let td = required("dimensionless_time_value", dimensionless_time_value)?;
            row.dimensionless_pressure_derivative = Some(dimensionless_pressure_derivative(
                td,
                dimensionless_outer_radius,
            )?);
        }
        "dimensionless_time" => {
            let k = required_area(permeability)?;
            let t = required_time(time)?;
            let phi = required("porosity", porosity)?;
            let mu = required_viscosity(viscosity)?;
            let ct = required("total_compressibility", total_compressibility)?;
            let radius = required_length(reservoir_radius, "reservoir_radius")?;
            row.dimensionless_time = Some(dimensionless_time(k, t, phi, mu, ct, radius)?);
        }
        "aquifer_constant" => {
            let phi = required("porosity", porosity)?;
            let ct = required("total_compressibility", total_compressibility)?;
            let h = required_length(thickness, "thickness")?;
            let radius = required_length(reservoir_radius, "reservoir_radius")?;
            let angle = required("encroachment_angle", encroachment_angle)?;
            row.aquifer_constant = Some(aquifer_constant(phi, ct, h, radius, angle)?);
        }
        "carter_tracy" => {
            let times = dimensionless_times.ok_or_else(|| {
                AzothError::invalid_input(
                    "dimensionless_times",
                    "the Carter-Tracy recurrence needs the dimensionless time history",
                )
            })?;
            let drops = pressure_drop.ok_or_else(|| {
                AzothError::invalid_input(
                    "pressure_drop",
                    "the Carter-Tracy recurrence needs the pressure-drop history",
                )
            })?;
            let constant = required("aquifer_constant_value", aquifer_constant_value)?;
            row.cumulative_influx = Some(cumulative_influx_carter_tracy(
                times,
                drops,
                constant,
                dimensionless_outer_radius,
            )?);
        }
        "influence_table" => {
            let times = dimensionless_times.ok_or_else(|| {
                AzothError::invalid_input(
                    "dimensionless_times",
                    "the influence table needs the dimensionless time history",
                )
            })?;
            row.dimensionless_pressures = Some(influence_table(times, dimensionless_outer_radius)?);
        }
        other => {
            return Err(AzothError::invalid_input(
                "calculation",
                format!("{other:?} is not one of the aquifer calculations"),
            ));
        }
    }
    Ok(row)
}

fn empty() -> VanEverdingenHurstAquiferResult {
    VanEverdingenHurstAquiferResult {
        dimensionless_pressure: None,
        dimensionless_pressure_derivative: None,
        dimensionless_time: None,
        aquifer_constant: None,
        dimensionless_pressures: None,
        cumulative_influx: None,
        warnings: Vec::new(),
    }
}

fn required(name: &str, value: Option<f64>) -> Result<f64> {
    value.ok_or_else(|| AzothError::invalid_input(name, "the calculation needs this field"))
}

fn required_area(value: Option<Area>) -> Result<f64> {
    value.map(|quantity| quantity.value).ok_or_else(|| {
        AzothError::invalid_input("permeability", "the calculation needs this field")
    })
}

fn required_time(value: Option<Time>) -> Result<f64> {
    value
        .map(|quantity| quantity.value)
        .ok_or_else(|| AzothError::invalid_input("time", "the calculation needs this field"))
}

fn required_viscosity(value: Option<DynamicViscosity>) -> Result<f64> {
    value
        .map(|quantity| quantity.value)
        .ok_or_else(|| AzothError::invalid_input("viscosity", "the calculation needs this field"))
}

fn required_length(value: Option<Length>, name: &str) -> Result<f64> {
    value
        .map(|quantity| quantity.value)
        .ok_or_else(|| AzothError::invalid_input(name, "the calculation needs this field"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kernel_is_zero_at_the_bookkeeping_origin() {
        assert_eq!(dimensionless_pressure(0.0, None).unwrap(), 0.0);
        assert_eq!(dimensionless_pressure_derivative(0.0, None).unwrap(), 0.0);
    }

    #[test]
    fn the_pressure_is_monotone_at_the_reference_times() {
        let times = [
            1e-12, 1e-8, 1e-6, 3e-5, 1e-4, 0.003, 0.009999, 0.01, 0.010001, 0.3, 1.0, 7.5, 100.0,
            199.999, 200.0, 200.001, 8000.0, 10000.0, 1e8, 1e12,
        ];
        let mut previous = 0.0;
        for time in times {
            let pressure = dimensionless_pressure(time, None).unwrap();
            assert!(pressure >= previous, "pressure decreased at {time}");
            previous = pressure;
        }
    }

    #[test]
    fn the_class_reference_values_are_reproduced() {
        assert_eq!(
            dimensionless_pressure(0.005, None).unwrap(),
            0.0773837496680494
        );
        assert_eq!(
            dimensionless_pressure_derivative(0.005, None).unwrap(),
            7.507012527254403
        );
        assert_eq!(
            dimensionless_pressure(1.0e5, None).unwrap(),
            6.161035384162979
        );
        assert_eq!(
            dimensionless_pressure(100.0, Some(5.0)).unwrap(),
            9.192771245767434
        );
        assert_eq!(
            dimensionless_pressure_derivative(1.0e-4, Some(10.0)).unwrap(),
            0.020202020202020204
        );
    }

    #[test]
    fn the_dimensionless_time_and_constant_are_si() {
        let td = dimensionless_time(1.0e-13, 8.64e7, 0.2, 5.0e-4, 1.0e-9, 3000.0).unwrap();
        assert_eq!(td, 9.599999999999998);
        let constant = aquifer_constant(0.20, 1.0e-9, 30.0, 3000.0, 180.0).unwrap();
        assert_eq!(constant, 0.16964600329384885);
    }

    #[test]
    fn carter_tracy_reproduces_the_first_history() {
        let times = [0.0, 10.0, 20.0, 40.0, 80.0, 160.0];
        let drops = [0.0, 5.0e5, 1.2e6, 2.2e6, 3.5e6, 5.0e6];
        let influx =
            cumulative_influx_carter_tracy(&times, &drops, 0.16964600329384885, None).unwrap();
        assert_eq!(influx[0], 0.0);
        assert!((influx[1] / 513800.1920958346 - 1.0).abs() < 1.0e-12);
        assert!((influx[5] / 3.831957738347466e7 - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn a_negative_drop_is_projected_to_a_zero_increment() {
        let influx =
            cumulative_influx_carter_tracy(&[0.0, 0.1, 0.2], &[0.0, 1.0, -10.0], 1.0, None)
                .unwrap();
        assert!((influx[1] / 0.31823407285179967 - 1.0).abs() < 1.0e-12);
        assert_eq!(influx[2], 0.0);
    }
}
