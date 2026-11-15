//! `process.toxic_library` - the toxic-exposure threshold table and its conversion.
//!
//! Spec: `specs/models/process/toxic_library.toml`. The arithmetic is
//! `process/safety/dispersion/ToxicLibrary.java` at the pin: the static table, its
//! exact-then-lowercase lookup, and `ppmToKgPerM3` with the class's own `8.314`.

use azoth_core::units::{
    MassDensity, MolarMass, Pressure, ThermodynamicTemperature, kilograms_per_cubic_meter,
};
use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::{optional_scalar, warnings as wire_warnings};
use crate::model_gen;

/// The class's gas constant, J/(mol K); deliberately not the analyzer's `8.314462618`.
const R_GAS: f64 = 8.314;

/// The class's static table: `(key, name, IDLH ppm, ERPG-2 ppm, AEGL-2 ppm)`.
///
/// The keys are the map's own: the formula keys are exact-case only, and the five
/// spelled-out aliases and the two lowercase names are the entries that make those
/// lookups case-insensitive.
const TABLE: &[(&str, &str, f64, f64, f64)] = &[
    ("H2S", "Hydrogen sulfide", 100.0, 30.0, 27.0),
    ("hydrogen sulfide", "Hydrogen sulfide", 100.0, 30.0, 27.0),
    ("CO", "Carbon monoxide", 1200.0, 350.0, 83.0),
    ("carbon monoxide", "Carbon monoxide", 1200.0, 350.0, 83.0),
    ("Cl2", "Chlorine", 10.0, 3.0, 2.0),
    ("chlorine", "Chlorine", 10.0, 3.0, 2.0),
    ("NH3", "Ammonia", 300.0, 150.0, 160.0),
    ("ammonia", "Ammonia", 300.0, 150.0, 160.0),
    ("SO2", "Sulfur dioxide", 100.0, 3.0, 0.75),
    ("sulfur dioxide", "Sulfur dioxide", 100.0, 3.0, 0.75),
    ("HCN", "Hydrogen cyanide", 50.0, 10.0, 7.1),
    ("HF", "Hydrogen fluoride", 30.0, 20.0, 24.0),
    ("HCl", "Hydrogen chloride", 50.0, 20.0, 22.0),
    ("methanol", "Methanol", 6000.0, 1000.0, 530.0),
    ("benzene", "Benzene", 500.0, 150.0, 800.0),
];

/// Result of `process.toxic_library`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToxicLibraryResult {
    /// Whether the chemical is in the table; present iff a chemical is stated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub known: Option<bool>,
    /// The table's display name; present iff the chemical is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The NIOSH IDLH in ppm; present iff the chemical is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idlh_ppm: Option<f64>,
    /// The AIHA ERPG-2 in ppm; present iff the chemical is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub erpg2_ppm: Option<f64>,
    /// The EPA AEGL-2 (60 min) in ppm; present iff the chemical is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aegl2_ppm: Option<f64>,
    /// The converted concentration; present iff the conversion group is stated.
    #[serde(serialize_with = "optional_scalar")]
    pub mass_concentration: Option<MassDensity>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The class's exact hit, then its lowercased hit.
fn lookup(chemical: &str) -> Option<(&'static str, &'static str, f64, f64, f64)> {
    if let Some(row) = TABLE.iter().find(|row| row.0 == chemical) {
        return Some(*row);
    }
    let lower = chemical.to_lowercase();
    TABLE.iter().find(|row| row.0 == lower.as_str()).copied()
}

fn require_positive(name: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(AzothError::invalid_input(
            name,
            format!("a finite positive value is required, and {value} is not"),
        ));
    }
    Ok(())
}

/// The threshold row for a chemical, and the ppm-to-kg/m3 conversion.
///
/// # Errors
/// `InvalidInput` when neither arm is stated, when `concentration_ppm` is stated without
/// the molar mass, temperature and pressure, or when the conversion's values are
/// non-finite or out of range. An unknown chemical is not an error: `known` is false.
pub fn toxic_library(
    chemical: Option<&str>,
    concentration_ppm: Option<f64>,
    molar_mass: Option<MolarMass>,
    temperature: Option<ThermodynamicTemperature>,
    pressure: Option<Pressure>,
) -> Result<ToxicLibraryResult> {
    if chemical.is_none() && concentration_ppm.is_none() {
        return Err(AzothError::invalid_input(
            "chemical",
            "a chemical or the conversion group is required, and neither was stated",
        ));
    }
    let molar = molar_mass.map(|value| value.value);
    let hot = temperature.map(|value| value.value);
    let press = pressure.map(|value| value.value);
    if concentration_ppm.is_some() {
        for (name, value) in [
            ("molar_mass", molar),
            ("temperature", hot),
            ("pressure", press),
        ] {
            if value.is_none() {
                return Err(AzothError::invalid_input(
                    name,
                    "is required with concentration_ppm",
                ));
            }
        }
    }
    let spec = &model_gen::TOXIC_LIBRARY_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "concentration_ppm" => concentration_ppm,
            "molar_mass" => molar,
            "temperature" => hot,
            "pressure" => press,
            _ => None,
        },
        &mut warnings,
    )?;
    if let Some(ppm) = concentration_ppm {
        if !ppm.is_finite() || ppm < 0.0 {
            return Err(AzothError::invalid_input(
                "concentration_ppm",
                format!("a finite non-negative concentration is required, and {ppm} is not"),
            ));
        }
    }
    if let Some(value) = molar {
        require_positive("molar_mass", value)?;
    }
    if let Some(value) = hot {
        require_positive("temperature", value)?;
    }
    if let Some(value) = press {
        require_positive("pressure", value)?;
    }

    let record = chemical.and_then(lookup);
    let mass = match (concentration_ppm, molar, hot, press) {
        (Some(ppm), Some(molar), Some(hot), Some(press)) => {
            let bara = press / 1.0e5;
            Some((ppm / 1.0e6) * molar * (bara * 1.0e5) / (R_GAS * hot))
        }
        _ => None,
    };
    Ok(ToxicLibraryResult {
        known: chemical.map(|_| record.is_some()),
        name: record.map(|row| row.1.to_owned()),
        idlh_ppm: record.map(|row| row.2),
        erpg2_ppm: record.map(|row| row.3),
        aegl2_ppm: record.map(|row| row.4),
        mass_concentration: mass.map(kilograms_per_cubic_meter),
        warnings,
    })
}
