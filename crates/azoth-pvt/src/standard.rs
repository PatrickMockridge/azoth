//! The standard state and the conversion literals the PVT package states its inputs against.
//!
//! NeqSim's PVT classes are written in US oilfield units — psia, °F, scf/STB, cP and lb/ft³ —
//! and convert through `BlackOilUnits`' literals. Those literals are not exact SI definitions:
//! `14.5038` psi per bar and `0.178108` Sm³/Sm³ per scf/STB are the class's own numbers, and a
//! caller that recomputes them from the bar, the psi and the cubic metre disagrees in the fifth
//! significant figure. They are carried as stated.
//!
//! Gas volumes are referred to `288.15 K` and `1.01325 bar` — the class's standard state, not
//! the 273.15 K of `ThermodynamicConstantsInterface.referenceTemperature`. The saturation
//! searches' grids and bisection limits are the classes' own as well.

/// The temperature a gas volume is referred to, in K.
pub const STANDARD_TEMPERATURE_K: f64 = 288.15;

/// The pressure a gas volume is referred to, in bar.
pub const STANDARD_PRESSURE_BAR: f64 = 1.01325;

/// The same standard pressure in Pa, exact by the definition of the bar.
pub const STANDARD_PRESSURE_PA: f64 = 101_325.0;

/// `ThermodynamicConstantsInterface.referenceTemperature`, in K.
pub const NEQSIM_REFERENCE_TEMPERATURE_K: f64 = 273.15;

/// `ThermodynamicConstantsInterface.referencePressure`, in bar.
pub const NEQSIM_REFERENCE_PRESSURE_BAR: f64 = 1.01325;

/// `ThermodynamicConstantsInterface.R`, in J/(mol·K), as the class states it.
pub const NEQSIM_GAS_CONSTANT: f64 = 8.314_462_1;

/// `BlackOilUnits`' psia per bar.
pub const PSIA_PER_BAR: f64 = 14.5038;

/// `BlackOilUnits`' Sm³/Sm³ per scf/STB.
pub const SM3_PER_SCF_STB: f64 = 0.178_108;

/// `SaturationPressureCorrelation.psiaToBar`'s bar per psia.
pub const SATURATION_BAR_PER_PSIA: f64 = 0.068_947_6;

/// `SaturationPressureCorrelation.scfStbToSm3Sm3`'s Sm³/Sm³ per scf/STB.
pub const SATURATION_SM3_PER_SCF_STB: f64 = 0.178_107_606_679_03;

/// `BlackOilUnits`' kg/m³ per lb/ft³.
pub const KG_PER_M3_PER_LB_FT3: f64 = 16.018_46;

/// `BlackOilUnits`' centipoise per Pa·s.
pub const CENTIPOISE_PER_PA_S: f64 = 1000.0;

/// `BlackOilUnits`' Rankine offset above Fahrenheit.
pub const RANKINE_OFFSET: f64 = 459.67;

/// The Kelvin offset of the Celsius scale.
pub const CELSIUS_OFFSET_K: f64 = 273.15;

/// The bottom of `SaturationPressure`'s pressure grid, in bara.
pub const SATURATION_PRESSURE_MIN_BAR: f64 = 1.0;

/// The top of `SaturationPressure`'s pressure grid, in bara.
pub const SATURATION_PRESSURE_MAX_BAR: f64 = 1000.0;

/// `SaturationPressure`'s coarse pressure step, in bar.
pub const SATURATION_PRESSURE_STEP_BAR: f64 = 10.0;

/// `SaturationPressure`'s bisection tolerance, in bar.
pub const SATURATION_PRESSURE_TOLERANCE_BAR: f64 = 1.0e-5;

/// The bisection limit both saturation searches share.
pub const SATURATION_BISECTION_LIMIT: u32 = 500;

/// The bottom of `SaturationTemperature`'s temperature grid, in K.
pub const SATURATION_TEMPERATURE_MIN_K: f64 = 30.0;

/// The top of `SaturationTemperature`'s temperature grid, in K.
pub const SATURATION_TEMPERATURE_MAX_K: f64 = 1200.0;

/// `SaturationTemperature`'s coarse temperature step, in K.
pub const SATURATION_TEMPERATURE_STEP_K: f64 = 10.0;

/// `SaturationTemperature`'s bisection tolerance, in K.
pub const SATURATION_TEMPERATURE_TOLERANCE_K: f64 = 1.0e-5;

/// Bar to pascal, exact by the definition of the bar.
#[must_use]
pub fn bar_to_pa(bar: f64) -> f64 {
    bar * 1.0e5
}

/// Pascal to bar, exact by the definition of the bar.
#[must_use]
pub fn pa_to_bar(pa: f64) -> f64 {
    pa / 1.0e5
}

/// Bar to psia, with `BlackOilUnits`' literal.
#[must_use]
pub fn bar_to_psia(bar: f64) -> f64 {
    bar * PSIA_PER_BAR
}

/// Psia to bar, with `BlackOilUnits`' literal.
#[must_use]
pub fn psia_to_bar(psia: f64) -> f64 {
    psia / PSIA_PER_BAR
}

/// Pascal to psia: the exact bar definition first, then the class's literal.
#[must_use]
pub fn pa_to_psia(pa: f64) -> f64 {
    bar_to_psia(pa_to_bar(pa))
}

/// Psia to pascal: the class's literal first, then the exact bar definition.
#[must_use]
pub fn psia_to_pa(psia: f64) -> f64 {
    bar_to_pa(psia_to_bar(psia))
}

/// Kelvin to the Celsius magnitude.
#[must_use]
pub fn kelvin_to_celsius(kelvin: f64) -> f64 {
    kelvin - CELSIUS_OFFSET_K
}

/// The Celsius magnitude to Kelvin.
#[must_use]
pub fn celsius_to_kelvin(celsius: f64) -> f64 {
    celsius + CELSIUS_OFFSET_K
}

/// Celsius to Fahrenheit.
#[must_use]
pub fn celsius_to_fahrenheit(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}

/// Kelvin to Fahrenheit.
#[must_use]
pub fn kelvin_to_fahrenheit(kelvin: f64) -> f64 {
    celsius_to_fahrenheit(kelvin_to_celsius(kelvin))
}

/// Fahrenheit to Celsius.
#[must_use]
pub fn fahrenheit_to_celsius(fahrenheit: f64) -> f64 {
    (fahrenheit - 32.0) * 5.0 / 9.0
}

/// Fahrenheit to Kelvin.
#[must_use]
pub fn fahrenheit_to_kelvin(fahrenheit: f64) -> f64 {
    celsius_to_kelvin(fahrenheit_to_celsius(fahrenheit))
}

/// Celsius to Rankine.
#[must_use]
pub fn celsius_to_rankine(celsius: f64) -> f64 {
    celsius_to_fahrenheit(celsius) + RANKINE_OFFSET
}

/// Kelvin to Rankine.
#[must_use]
pub fn kelvin_to_rankine(kelvin: f64) -> f64 {
    kelvin_to_fahrenheit(kelvin) + RANKINE_OFFSET
}

/// scf/STB to Sm³/Sm³, with `BlackOilUnits`' literal.
#[must_use]
pub fn scf_stb_to_sm3_sm3(scf_per_stb: f64) -> f64 {
    scf_per_stb * SM3_PER_SCF_STB
}

/// Sm³/Sm³ to scf/STB, with `BlackOilUnits`' literal.
#[must_use]
pub fn sm3_sm3_to_scf_stb(sm3_per_sm3: f64) -> f64 {
    sm3_per_sm3 / SM3_PER_SCF_STB
}

/// Psia to bar, with `SaturationPressureCorrelation`'s literal.
#[must_use]
pub fn saturation_psia_to_bar(psia: f64) -> f64 {
    psia * SATURATION_BAR_PER_PSIA
}

/// Bar to psia, with `SaturationPressureCorrelation`'s literal.
#[must_use]
pub fn saturation_bar_to_psia(bar: f64) -> f64 {
    bar / SATURATION_BAR_PER_PSIA
}

/// scf/STB to Sm³/Sm³, with `SaturationPressureCorrelation`'s literal.
#[must_use]
pub fn saturation_scf_stb_to_sm3_sm3(scf_per_stb: f64) -> f64 {
    scf_per_stb * SATURATION_SM3_PER_SCF_STB
}

/// Sm³/Sm³ to scf/STB, with `SaturationPressureCorrelation`'s literal.
#[must_use]
pub fn saturation_sm3_sm3_to_scf_stb(sm3_per_sm3: f64) -> f64 {
    sm3_per_sm3 / SATURATION_SM3_PER_SCF_STB
}

/// Pa·s to centipoise.
#[must_use]
pub fn pa_s_to_centipoise(pa_s: f64) -> f64 {
    pa_s * CENTIPOISE_PER_PA_S
}

/// Centipoise to Pa·s.
#[must_use]
pub fn centipoise_to_pa_s(centipoise: f64) -> f64 {
    centipoise / CENTIPOISE_PER_PA_S
}

/// kg/m³ to lb/ft³, with `BlackOilUnits`' literal.
#[must_use]
pub fn kg_m3_to_lb_ft3(kg_per_m3: f64) -> f64 {
    kg_per_m3 / KG_PER_M3_PER_LB_FT3
}

/// lb/ft³ to kg/m³, with `BlackOilUnits`' literal.
#[must_use]
pub fn lb_ft3_to_kg_m3(lb_per_ft3: f64) -> f64 {
    lb_per_ft3 * KG_PER_M3_PER_LB_FT3
}

/// 1/bar to 1/psi.
#[must_use]
pub fn per_bar_to_per_psi(per_bar: f64) -> f64 {
    per_bar / PSIA_PER_BAR
}

/// 1/psi to 1/bar.
#[must_use]
pub fn per_psi_to_per_bar(per_psi: f64) -> f64 {
    per_psi * PSIA_PER_BAR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_conversion_literals_are_the_class_ones() {
        assert_eq!(bar_to_psia(1.0), 14.5038);
        assert_eq!(scf_stb_to_sm3_sm3(1.0), 0.178_108);
        assert_eq!(saturation_psia_to_bar(1.0), 0.068_947_6);
        assert_eq!(saturation_bar_to_psia(1.0), 1.0 / 0.068_947_6);
        assert_eq!(saturation_scf_stb_to_sm3_sm3(1.0), 0.178_107_606_679_03);
        assert_eq!(kg_m3_to_lb_ft3(16.018_46), 1.0);
        assert_eq!(pa_s_to_centipoise(0.001), 1.0);
        assert_eq!(celsius_to_rankine(0.0), 491.67);
    }

    #[test]
    fn the_two_class_literals_are_not_the_same_number() {
        // `BlackOilCorrelations` and `SaturationPressureCorrelation` each convert the same
        // quantities with their own literal; a port that picked one for both would miss the
        // capture in the fifth significant figure.
        assert_ne!(SM3_PER_SCF_STB, SATURATION_SM3_PER_SCF_STB);
        assert_ne!(psia_to_bar(1.0), saturation_psia_to_bar(1.0));
    }

    #[test]
    fn the_standard_state_is_the_class_standard_state() {
        assert_eq!(STANDARD_TEMPERATURE_K, 288.15);
        assert_eq!(STANDARD_PRESSURE_BAR, 1.01325);
        assert_eq!(STANDARD_PRESSURE_PA, 101_325.0);
    }

    #[test]
    fn pressure_round_trips_through_the_class_literal() {
        for bar in [1.0, 5.0, 50.0, 350.0, 1000.0] {
            let back = psia_to_bar(bar_to_psia(bar));
            assert!((back - bar).abs() <= 1.0e-12 * bar.abs().max(1.0));
        }
    }

    #[test]
    fn temperature_conversions_are_the_class_arithmetic() {
        assert_eq!(kelvin_to_celsius(288.15), 288.15 - 273.15);
        assert_eq!(celsius_to_kelvin(15.0), 288.15);
        assert_eq!(celsius_to_fahrenheit(100.0), 212.0);
        assert_eq!(fahrenheit_to_celsius(32.0), 0.0);
        assert_eq!(kelvin_to_rankine(273.15), 491.67);
    }

    #[test]
    fn the_saturation_grids_are_the_class_grids() {
        assert_eq!(SATURATION_PRESSURE_MIN_BAR, 1.0);
        assert_eq!(SATURATION_PRESSURE_MAX_BAR, 1000.0);
        assert_eq!(SATURATION_PRESSURE_STEP_BAR, 10.0);
        assert_eq!(SATURATION_PRESSURE_TOLERANCE_BAR, 1.0e-5);
        assert_eq!(SATURATION_TEMPERATURE_MIN_K, 30.0);
        assert_eq!(SATURATION_TEMPERATURE_MAX_K, 1200.0);
        assert_eq!(SATURATION_TEMPERATURE_STEP_K, 10.0);
        assert_eq!(SATURATION_TEMPERATURE_TOLERANCE_K, 1.0e-5);
        assert_eq!(SATURATION_BISECTION_LIMIT, 500);
    }
}
