//! The capacity correlations and labour rates of `CostEstimationCalculator`.
//!
//! Every function returns USD on the stated CEPCI ratio. The clamps and the
//! parallel-unit splits are the class's own: a duty above a correlation's
//! validity range is several equal machines, not one extrapolated machine.

use crate::costing::records::MtoItem;

/// The validity bounds of the Turton compressor correlation, kW.
pub const COMPRESSOR_MIN_KW: f64 = 450.0;
/// The validity upper bound of the Turton compressor correlation, kW.
pub const COMPRESSOR_MAX_KW: f64 = 3000.0;
/// The validity lower bound of the centrifugal-pump correlation, kW.
pub const PUMP_MIN_KW: f64 = 1.0;
/// The validity upper bound of the centrifugal-pump correlation, kW.
pub const PUMP_MAX_KW: f64 = 300.0;

/// One Turton capacity correlation's coefficients and validity range.
#[derive(Clone, Copy, Debug)]
pub struct Turton {
    /// The lower validity bound.
    pub min_capacity: f64,
    /// The upper validity bound; a larger duty is several equal units.
    pub max_capacity: f64,
    /// The constant term.
    pub k1: f64,
    /// The first-order coefficient.
    pub k2: f64,
    /// The second-order coefficient.
    pub k3: f64,
    /// A multiplier on the unit cost, for example 1.3 for a reciprocating machine.
    pub type_factor: f64,
}

/// A Turton capacity correlation evaluated as equal parallel units.
#[must_use]
pub fn turton_parallel(capacity: f64, correlation: &Turton, cepci_ratio: f64) -> f64 {
    let units = (capacity / correlation.max_capacity).ceil().max(1.0);
    let per_unit = (capacity / units).max(correlation.min_capacity);
    let log_s = per_unit.log10();
    let log_cp = correlation.k1 + correlation.k2 * log_s + correlation.k3 * log_s * log_s;
    units * 10.0_f64.powf(log_cp) * correlation.type_factor * cepci_ratio
}

const PUMP_CORRELATION: Turton = Turton {
    min_capacity: PUMP_MIN_KW,
    max_capacity: PUMP_MAX_KW,
    k1: 3.3892,
    k2: 0.0536,
    k3: 0.1538,
    type_factor: 1.0,
};

const COMPRESSOR_CORRELATION: Turton = Turton {
    min_capacity: COMPRESSOR_MIN_KW,
    max_capacity: COMPRESSOR_MAX_KW,
    k1: 2.2897,
    k2: 1.3604,
    k3: -0.1027,
    type_factor: 1.0,
};

const RECIPROCATING_CORRELATION: Turton = Turton {
    min_capacity: COMPRESSOR_MIN_KW,
    max_capacity: COMPRESSOR_MAX_KW,
    k1: 2.2897,
    k2: 1.3604,
    k3: -0.1027,
    type_factor: 1.3,
};

/// A vertical pressure vessel priced from its internal volume, clamped 0.3–520 m3.
#[must_use]
pub fn vertical_vessel_by_volume(volume_m3: f64, cepci_ratio: f64) -> f64 {
    if volume_m3 <= 0.0 {
        return 0.0;
    }
    let volume = volume_m3.clamp(0.3, 520.0);
    let log_v = volume.log10();
    let log_cp = 3.4974 + 0.4485 * log_v + 0.1074 * log_v * log_v;
    10.0_f64.powf(log_cp) * cepci_ratio
}

/// The deprecated weight-based vertical-vessel route, carried as the class answers it.
#[must_use]
pub fn vertical_vessel_by_weight(weight_kg: f64, cepci_ratio: f64) -> f64 {
    if weight_kg <= 0.0 {
        return 0.0;
    }
    let log_w = weight_kg.max(250.0).log10();
    let log_cp = 3.4974 + 0.4485 * log_w + 0.1074 * log_w * log_w;
    10.0_f64.powf(log_cp) * cepci_ratio
}

/// A horizontal pressure vessel priced from its internal volume, clamped 0.1–628 m3.
#[must_use]
pub fn horizontal_vessel_by_volume(volume_m3: f64, cepci_ratio: f64) -> f64 {
    if volume_m3 <= 0.0 {
        return 0.0;
    }
    let volume = volume_m3.clamp(0.1, 628.0);
    let log_v = volume.log10();
    let log_cp = 3.5565 + 0.3776 * log_v + 0.0905 * log_v * log_v;
    10.0_f64.powf(log_cp) * cepci_ratio
}

/// The deprecated weight-based horizontal-vessel route.
#[must_use]
pub fn horizontal_vessel_by_weight(weight_kg: f64, cepci_ratio: f64) -> f64 {
    if weight_kg <= 0.0 {
        return 0.0;
    }
    let log_w = weight_kg.max(250.0).log10();
    let log_cp = 3.5565 + 0.3776 * log_w + 0.0905 * log_w * log_w;
    10.0_f64.powf(log_cp) * cepci_ratio
}

/// A fixed-tubesheet shell-and-tube exchanger priced from area, floored at 10 m2.
#[must_use]
pub fn shell_tube_heat_exchanger(area_m2: f64, cepci_ratio: f64) -> f64 {
    if area_m2 <= 0.0 {
        return 0.0;
    }
    let log_a = area_m2.max(10.0).log10();
    let log_cp = 4.3247 - 0.3030 * log_a + 0.1634 * log_a * log_a;
    10.0_f64.powf(log_cp) * cepci_ratio
}

/// A plate heat exchanger priced from area.
#[must_use]
pub fn plate_heat_exchanger(area_m2: f64, cepci_ratio: f64) -> f64 {
    if area_m2 <= 0.0 {
        return 0.0;
    }
    1000.0 * area_m2.powf(0.68) * cepci_ratio
}

/// An air cooler priced from area, floored at 10 m2.
#[must_use]
pub fn air_cooler(area_m2: f64, cepci_ratio: f64) -> f64 {
    if area_m2 <= 0.0 {
        return 0.0;
    }
    let log_a = area_m2.max(10.0).log10();
    let log_cp = 4.0336 + 0.2341 * log_a + 0.0497 * log_a * log_a;
    10.0_f64.powf(log_cp) * cepci_ratio
}

/// Sieve trays priced from diameter (clamped 0.3–5 m) and a tray count.
#[must_use]
pub fn sieve_trays(diameter_m: f64, number_of_trays: f64, cepci_ratio: f64) -> f64 {
    if diameter_m <= 0.0 || number_of_trays <= 0.0 {
        return 0.0;
    }
    let d = diameter_m.clamp(0.3, 5.0);
    468.0 * (0.1739 * d).exp() * number_of_trays * cepci_ratio
}

/// Valve trays, 1.2 times sieve.
#[must_use]
pub fn valve_trays(diameter_m: f64, number_of_trays: f64, cepci_ratio: f64) -> f64 {
    sieve_trays(diameter_m, number_of_trays, cepci_ratio) * 1.2
}

/// Bubble-cap trays, 1.5 times sieve.
#[must_use]
pub fn bubble_cap_trays(diameter_m: f64, number_of_trays: f64, cepci_ratio: f64) -> f64 {
    sieve_trays(diameter_m, number_of_trays, cepci_ratio) * 1.5
}

/// Packing priced by volume; metal 8000, ceramic 3000, everything else 2000 per m3.
#[must_use]
pub fn packing(volume_m3: f64, packing_type: &str, cepci_ratio: f64) -> f64 {
    if volume_m3 <= 0.0 {
        return 0.0;
    }
    let per_m3 = if packing_type.eq_ignore_ascii_case("metal") {
        8000.0
    } else if packing_type.eq_ignore_ascii_case("ceramic") {
        3000.0
    } else {
        2000.0
    };
    per_m3 * volume_m3 * cepci_ratio
}

/// A centrifugal pump, split into parallel units above 300 kW.
#[must_use]
pub fn centrifugal_pump(power_kw: f64, cepci_ratio: f64) -> f64 {
    if power_kw <= 0.0 {
        return 0.0;
    }
    turton_parallel(power_kw, &PUMP_CORRELATION, cepci_ratio)
}

/// A centrifugal compressor, split into parallel units above 3000 kW.
#[must_use]
pub fn centrifugal_compressor(power_kw: f64, cepci_ratio: f64) -> f64 {
    if power_kw <= 0.0 {
        return 0.0;
    }
    turton_parallel(power_kw, &COMPRESSOR_CORRELATION, cepci_ratio)
}

/// A reciprocating compressor, the centrifugal correlation with a 1.3 type factor.
#[must_use]
pub fn reciprocating_compressor(power_kw: f64, cepci_ratio: f64) -> f64 {
    if power_kw <= 0.0 {
        return 0.0;
    }
    turton_parallel(power_kw, &RECIPROCATING_CORRELATION, cepci_ratio)
}

/// Piping priced per metre, the material factor applied inside.
#[must_use]
pub fn piping_cost(
    diameter_m: f64,
    length_m: f64,
    schedule: u32,
    material_factor: f64,
    cepci_ratio: f64,
) -> f64 {
    if diameter_m <= 0.0 || length_m <= 0.0 {
        return 0.0;
    }
    let diameter_in = diameter_m / 0.0254;
    let base_per_m = 50.0 * diameter_in.powf(1.5);
    let schedule_factor = match schedule {
        80 => 1.5,
        160 => 2.5,
        _ => 1.0,
    };
    base_per_m * length_m * schedule_factor * material_factor * cepci_ratio
}

/// A control valve priced from Cv, the material factor applied inside.
#[must_use]
pub fn control_valve(cv: f64, material_factor: f64, cepci_ratio: f64) -> f64 {
    if cv <= 0.0 {
        return 0.0;
    }
    2500.0 * cv.powf(0.55) * material_factor * cepci_ratio
}

/// Installation labour hours from equipment weight and type.
#[must_use]
pub fn installation_man_hours(weight_kg: f64, equipment_type: &str) -> f64 {
    if weight_kg <= 0.0 {
        return 0.0;
    }
    let per_tonne = if equipment_type.eq_ignore_ascii_case("vessel") {
        30.0
    } else if equipment_type.eq_ignore_ascii_case("exchanger") {
        40.0
    } else if equipment_type.eq_ignore_ascii_case("pump") {
        50.0
    } else if equipment_type.eq_ignore_ascii_case("compressor") {
        60.0
    } else {
        35.0
    };
    per_tonne * weight_kg / 1000.0
}

/// Piping installation labour, 100 hours per tonne.
#[must_use]
pub fn piping_man_hours(weight_kg: f64) -> f64 {
    100.0 * weight_kg / 1000.0
}

/// The calculator's vessel bill of materials.
#[must_use]
pub fn vessel_bom(
    shell_weight_kg: f64,
    heads_weight_kg: f64,
    nozzle_count: f64,
    internals_weight_kg: f64,
    material: &str,
    material_factor: f64,
    cepci_ratio: f64,
) -> Vec<MtoItem> {
    let mut rows = Vec::new();
    let material = material.to_string();
    if shell_weight_kg > 0.0 {
        rows.push(MtoItem {
            item: "Vessel Shell".to_string(),
            material: Some(material.clone()),
            quantity: None,
            unit: None,
            weight_kg: Some(shell_weight_kg),
            cost_usd: vertical_vessel_by_weight(shell_weight_kg, cepci_ratio) * 0.6,
            source: None,
        });
    }
    if heads_weight_kg > 0.0 {
        rows.push(MtoItem {
            item: "Vessel Heads (2)".to_string(),
            material: Some(material.clone()),
            quantity: None,
            unit: None,
            weight_kg: Some(heads_weight_kg),
            cost_usd: heads_weight_kg * 8.0 * material_factor,
            source: None,
        });
    }
    if nozzle_count > 0.0 {
        rows.push(MtoItem {
            item: "Nozzles".to_string(),
            material: None,
            quantity: Some(nozzle_count),
            unit: None,
            weight_kg: None,
            cost_usd: 500.0 * nozzle_count * material_factor,
            source: None,
        });
    }
    if internals_weight_kg > 0.0 {
        rows.push(MtoItem {
            item: "Internals".to_string(),
            material: Some(material),
            quantity: None,
            unit: None,
            weight_kg: Some(internals_weight_kg),
            cost_usd: internals_weight_kg * 15.0 * material_factor,
            source: None,
        });
    }
    rows
}
