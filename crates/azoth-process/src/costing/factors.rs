//! The factor tables: cost indices, materials, locations, currencies and the
//! pressure-factor step function.

/// CEPCI 2019, the reference year of the Turton correlations.
pub const CEPCI_2019: f64 = 607.5;
/// CEPCI 2020.
pub const CEPCI_2020: f64 = 596.2;
/// CEPCI 2021.
pub const CEPCI_2021: f64 = 708.0;
/// CEPCI 2022.
pub const CEPCI_2022: f64 = 816.0;
/// CEPCI 2023.
pub const CEPCI_2023: f64 = 800.0;
/// CEPCI 2024 (estimated in the class).
pub const CEPCI_2024: f64 = 820.0;
/// CEPCI 2025 (estimated in the class), the default current index.
pub const CEPCI_2025: f64 = 840.0;

/// The bare-module constant `B1`.
pub const BARE_MODULE_B1: f64 = 1.89;
/// The bare-module constant `B2`.
pub const BARE_MODULE_B2: f64 = 1.35;
/// The default contingency factor.
pub const CONTINGENCY_DEFAULT: f64 = 0.15;
/// The default engineering factor.
pub const ENGINEERING_DEFAULT: f64 = 0.10;
/// Grass roots as a multiple of the total module cost.
pub const GRASS_ROOTS_FACTOR: f64 = 1.5;
/// The Lang factor the class stores and no formula reads.
pub const INSTALLATION_FACTOR_STORED: f64 = 3.0;
/// The fallback cost per kilogram of equipment weight.
pub const COST_PER_WEIGHT_UNIT: f64 = 8.0;

/// The CEPCI for a stated year, as the class's constants give it.
#[must_use]
pub fn cepci_for_year(year: i32) -> Option<f64> {
    match year {
        2019 => Some(CEPCI_2019),
        2020 => Some(CEPCI_2020),
        2021 => Some(CEPCI_2021),
        2022 => Some(CEPCI_2022),
        2023 => Some(CEPCI_2023),
        2024 => Some(CEPCI_2024),
        2025 => Some(CEPCI_2025),
        _ => None,
    }
}

/// The vessel pressure factor, the class's step table.
#[must_use]
pub fn pressure_factor(design_pressure_barg: f64) -> f64 {
    if design_pressure_barg <= 5.0 {
        1.0
    } else if design_pressure_barg <= 10.0 {
        1.05
    } else if design_pressure_barg <= 20.0 {
        1.15
    } else if design_pressure_barg <= 50.0 {
        1.30
    } else if design_pressure_barg <= 100.0 {
        1.50
    } else if design_pressure_barg <= 150.0 {
        1.65
    } else if design_pressure_barg <= 200.0 {
        1.90
    } else {
        2.5
    }
}

/// The material factor for a named material, or `None` where the class would
/// silently keep its previous factor.
#[must_use]
pub fn material_factor(material: &str) -> Option<f64> {
    let name = material.trim().to_ascii_lowercase();
    match name.as_str() {
        "carbon steel" | "cs" => Some(1.0),
        "ss304" | "304ss" => Some(1.8),
        "ss316" | "316ss" => Some(2.1),
        "ss316l" | "316lss" => Some(2.3),
        "monel" => Some(3.2),
        "hastelloy" | "hastelloy c" => Some(3.8),
        "inconel" => Some(3.5),
        "titanium" => Some(4.5),
        "nickel" => Some(3.0),
        _ => None,
    }
}

/// The canonical material name the class stores for a factor-bearing name.
#[must_use]
pub fn material_name(material: &str) -> Option<&'static str> {
    let name = material.trim().to_ascii_lowercase();
    match name.as_str() {
        "carbon steel" | "cs" => Some("Carbon Steel"),
        "ss304" | "304ss" => Some("SS304"),
        "ss316" | "316ss" => Some("SS316"),
        "ss316l" | "316lss" => Some("SS316L"),
        "monel" => Some("Monel"),
        "hastelloy" | "hastelloy c" => Some("Hastelloy C"),
        "inconel" => Some("Inconel"),
        "titanium" => Some("Titanium"),
        "nickel" => Some("Nickel"),
        _ => None,
    }
}

/// The location factor for a region name, the class's ordered `contains` chain.
#[must_use]
pub fn location_factor(region: &str) -> f64 {
    let name = region.to_ascii_lowercase();
    if name.contains("gulf coast") || name.contains("usgc") {
        1.0
    } else if name.contains("midwest") {
        1.05
    } else if name.contains("west coast") || name.contains("california") {
        1.15
    } else if name.contains("north sea") || name.contains("norway") || name.contains("norwegian") {
        1.35
    } else if name.contains("western europe")
        || name.contains("uk")
        || name.contains("netherlands")
        || name.contains("germany")
    {
        1.20
    } else if name.contains("middle east")
        || name.contains("qatar")
        || name.contains("saudi")
        || name.contains("uae")
        || name.contains("abu dhabi")
    {
        1.10
    } else if name.contains("southeast asia")
        || name.contains("singapore")
        || name.contains("malaysia")
        || name.contains("indonesia")
    {
        0.90
    } else if name.contains("china") {
        0.75
    } else if name.contains("australia") {
        1.30
    } else if name.contains("brazil") {
        1.15
    } else if name.contains("west africa") || name.contains("nigeria") || name.contains("angola") {
        1.40
    } else {
        1.0
    }
}

/// The default exchange rate from USD for a currency code.
#[must_use]
pub fn exchange_rate(currency: &str) -> Option<f64> {
    match currency.trim().to_ascii_uppercase().as_str() {
        "USD" => Some(1.0),
        "EUR" => Some(0.92),
        "NOK" => Some(11.0),
        "GBP" => Some(0.79),
        "CNY" => Some(7.25),
        "JPY" => Some(155.0),
        _ => None,
    }
}

/// The ratio of a current CEPCI to the correlation reference, 607.5.
#[must_use]
pub fn cepci_ratio(current: f64) -> f64 {
    current / CEPCI_2019
}
