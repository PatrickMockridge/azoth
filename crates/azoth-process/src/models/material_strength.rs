//! The rupture studies' material strength and flange ratings.
//!
//! `MaterialStrengthCurve`'s API 5L fallback table, the generic retained-strength grid,
//! the thin-wall von Mises stress and the B16.5 flange tables. `process.vessel_rupture`
//! imports the first three unchanged; `process.trapped_liquid_fire_rupture` adds the
//! flange ratings.

use azoth_core::{AzothError, Result};

/// The class's psi-to-pascal factor.
pub(crate) const PSI_TO_PA: f64 = 6894.76;

/// The class's retained-strength temperature grid, K.
const RETAINED_TEMPERATURES: [f64; 9] = [
    293.15, 373.15, 473.15, 573.15, 673.15, 773.15, 873.15, 973.15, 1073.15,
];

/// The class's generic carbon-steel retained-strength factors.
const RETAINED_FACTORS: [f64; 9] = [1.00, 0.95, 0.85, 0.72, 0.52, 0.32, 0.16, 0.07, 0.03];

/// The class's `Api5LGrade.normalize`.
pub(crate) fn normalize_grade(grade: &str) -> Result<String> {
    if grade.trim().is_empty() {
        return Err(AzothError::invalid_input(
            "grade",
            "Pipe material grade must not be empty",
        ));
    }
    let mut key = grade.trim().to_uppercase();
    if let Some(rest) = key.strip_prefix("GRADE ") {
        key = rest.trim().to_string();
    }
    let metric = [
        "L175", "L210", "L245", "L290", "L320", "L360", "L390", "L415", "L450", "L485", "L555",
        "L625", "L690", "L830",
    ];
    let imperial = [
        "A25", "A", "B", "X42", "X46", "X52", "X56", "X60", "X65", "X70", "X80", "X90", "X100",
        "X120",
    ];
    if let Some(index) = metric.iter().position(|value| *value == key) {
        return Ok(imperial[index].to_string());
    }
    Ok(key)
}

/// The class's built-in API 5L fallback strengths, Pa.
pub(crate) fn api5l_strengths(grade: &str) -> Result<(f64, f64)> {
    let psi = |yield_psi: f64, tensile_psi: f64| (yield_psi * PSI_TO_PA, tensile_psi * PSI_TO_PA);
    match grade {
        "B" | "GRADE B" => Ok(psi(35000.0, 60000.0)),
        "X42" => Ok(psi(42000.0, 60000.0)),
        "X52" => Ok(psi(52000.0, 66000.0)),
        "X60" => Ok(psi(60000.0, 75000.0)),
        "X65" => Ok(psi(65000.0, 77000.0)),
        "X70" => Ok(psi(70000.0, 82000.0)),
        "X80" => Ok(psi(80000.0, 90000.0)),
        "A25" => Ok(psi(25000.0, 45000.0)),
        "A" => Ok(psi(30000.0, 48000.0)),
        "X46" => Ok(psi(46000.0, 63000.0)),
        "X56" => Ok(psi(56000.0, 71000.0)),
        "X90" => Ok((620.0e6, 695.0e6)),
        "X100" => Ok(psi(100000.0, 110000.0)),
        "X120" => Ok(psi(120000.0, 130000.0)),
        other => Err(AzothError::invalid_input(
            "grade",
            format!("No API 5L material strength data for grade {other}"),
        )),
    }
}

/// The class's endpoint-clamped retained-strength interpolation.
pub(crate) fn retained_factor(metal_temperature_k: f64) -> f64 {
    if metal_temperature_k <= RETAINED_TEMPERATURES[0] {
        return RETAINED_FACTORS[0];
    }
    for index in 1..RETAINED_TEMPERATURES.len() {
        if metal_temperature_k <= RETAINED_TEMPERATURES[index] {
            let fraction = (metal_temperature_k - RETAINED_TEMPERATURES[index - 1])
                / (RETAINED_TEMPERATURES[index] - RETAINED_TEMPERATURES[index - 1]);
            return RETAINED_FACTORS[index - 1]
                + fraction * (RETAINED_FACTORS[index] - RETAINED_FACTORS[index - 1]);
        }
    }
    RETAINED_FACTORS[RETAINED_FACTORS.len() - 1]
}

/// The class's thin-wall von Mises stress, Pa.
pub(crate) fn von_mises_pa(pressure_pa: f64, radius_m: f64, wall_m: f64) -> f64 {
    let hoop = pressure_pa * radius_m / wall_m;
    let axial = pressure_pa * radius_m / (2.0 * wall_m);
    (hoop * hoop + axial * axial - hoop * axial).sqrt()
}

/// The class's `allowableRuptureStressAt`, Pa.
pub(crate) fn allowable_rupture_stress_pa(
    ambient_tensile_pa: f64,
    metal_temperature_k: f64,
    tensile_strength_factor: f64,
) -> f64 {
    ambient_tensile_pa * retained_factor(metal_temperature_k) * tensile_strength_factor
}

/// The class's ambient B16.5 flange rating, Pa; an unconfigured class is `+∞`.
pub(crate) fn flange_class_rating_pa(flange_class: i64) -> f64 {
    match flange_class {
        150 => 19.6 * 1.0e5,
        300 => 51.0 * 1.0e5,
        600 => 102.0 * 1.0e5,
        900 => 153.0 * 1.0e5,
        1500 => 255.0 * 1.0e5,
        2500 => 425.0 * 1.0e5,
        _ => f64::INFINITY,
    }
}

/// The class's temperature-reduced flange rating, Pa.
pub(crate) fn temperature_reduced_flange_rating_pa(
    ambient_rating_pa: f64,
    metal_temperature_k: f64,
) -> f64 {
    if !ambient_rating_pa.is_finite() {
        return ambient_rating_pa;
    }
    let temperature_c = metal_temperature_k - 273.15;
    let factor = if temperature_c <= 100.0 {
        1.0
    } else if temperature_c <= 200.0 {
        0.90
    } else if temperature_c <= 300.0 {
        0.75
    } else if temperature_c <= 400.0 {
        0.55
    } else if temperature_c <= 500.0 {
        0.35
    } else if temperature_c <= 600.0 {
        0.20
    } else {
        0.10
    };
    ambient_rating_pa * factor
}
