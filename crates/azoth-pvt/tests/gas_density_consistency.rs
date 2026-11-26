//! `pvt.gas_density_consistency` at its boundary.
//!
//! The states are the pin's, from `validation/neqsim/captures/pvt_gas_density_consistency.tsv`.

use azoth_pvt::{GasDensityConsistencyResult, GasDensityObservation, gas_density_consistency};

const PSI: f64 = 6894.757293168;

#[allow(clippy::too_many_arguments)] // one parameter per field of the observation record
fn row(
    pressure: f64,
    pressure_bound: f64,
    temperature: f64,
    temperature_bound: f64,
    compressibility: f64,
    compressibility_bound: f64,
    molar_mass: f64,
    molar_mass_bound: f64,
    density: f64,
    density_bound: f64,
) -> GasDensityObservation {
    GasDensityObservation {
        pressure,
        pressure_bound,
        temperature,
        temperature_bound,
        compressibility,
        compressibility_bound,
        molar_mass,
        molar_mass_bound,
        density,
        density_bound,
    }
}

fn duvernay() -> GasDensityConsistencyResult {
    let rows = [
        row(
            4661.0 * PSI,
            0.5 * PSI,
            379.15,
            0.5,
            0.922,
            0.0005,
            0.03018,
            0.000005,
            339.0,
            0.5,
        ),
        row(
            4451.0 * PSI,
            0.5 * PSI,
            379.15,
            0.5,
            0.897,
            0.0005,
            0.02987,
            0.000005,
            324.0,
            0.5,
        ),
        row(
            4100.0 * PSI,
            0.5 * PSI,
            379.15,
            0.5,
            0.861,
            0.0005,
            0.02893,
            0.000005,
            302.0,
            0.5,
        ),
        row(
            3701.0 * PSI,
            0.5 * PSI,
            379.15,
            0.5,
            0.828,
            0.0005,
            0.02755,
            0.000005,
            270.0,
            0.5,
        ),
        row(
            3201.0 * PSI,
            0.5 * PSI,
            379.15,
            0.5,
            0.813,
            0.0005,
            0.02595,
            0.000005,
            224.0,
            0.5,
        ),
        row(
            2600.0 * PSI,
            0.5 * PSI,
            379.15,
            0.5,
            0.813,
            0.0005,
            0.02456,
            0.000005,
            172.0,
            0.5,
        ),
        row(
            2100.0 * PSI,
            0.5 * PSI,
            379.15,
            0.5,
            0.826,
            0.0005,
            0.02384,
            0.000005,
            133.0,
            0.5,
        ),
        row(
            1650.0 * PSI,
            0.5 * PSI,
            379.15,
            0.5,
            0.848,
            0.0005,
            0.02311,
            0.000005,
            98.0,
            0.5,
        ),
        row(
            1300.0 * PSI,
            0.5 * PSI,
            379.15,
            0.5,
            0.874,
            0.0005,
            0.02314,
            0.000005,
            75.0,
            0.5,
        ),
    ];
    gas_density_consistency(&rows).unwrap()
}

#[test]
fn the_duvernay_table_is_the_capture_and_psat_is_incompatible() {
    let result = duvernay();
    assert!(!result.all_compatible);
    assert_eq!(result.rows.len(), 9);
    let psat = &result.rows[0];
    assert!((psat.implied_density - 333.6884347909451).abs() < 1.0e-12);
    assert!((psat.implied_lower - 332.9774398725011).abs() < 1.0e-9);
    assert!((psat.implied_upper - 334.40161473750067).abs() < 1.0e-9);
    assert!(!psat.compatible);
    let last = &result.rows[8];
    assert!((last.implied_density - 75.27816287269633).abs() < 1.0e-12);
    assert!(last.compatible);
    assert_eq!(result.rows.iter().filter(|row| row.compatible).count(), 8);
}

#[test]
fn the_point_touching_gap_and_dilute_limits_are_the_capture() {
    let rows = [
        row(
            831446.261815324,
            0.0,
            100.0,
            0.0,
            1.0,
            0.0,
            0.02,
            0.0,
            20.0,
            0.0,
        ),
        row(
            831446.261815324,
            0.0,
            100.0,
            0.0,
            1.0,
            0.0,
            0.02,
            0.0,
            21.0,
            1.0,
        ),
        row(
            831446.261815324,
            0.0,
            100.0,
            0.0,
            1.0,
            0.0,
            0.02,
            0.0,
            21.0,
            0.99,
        ),
        row(
            1.0e-6,
            0.0,
            100.0,
            0.0,
            1.0,
            0.0,
            0.02,
            0.0,
            2.4054471008545208e-11,
            1.0e-25,
        ),
    ];
    let result = gas_density_consistency(&rows).unwrap();
    assert!((result.rows[0].implied_density - 20.000000000000004).abs() < 1.0e-14);
    assert!(result.rows[0].compatible);
    assert!(result.rows[1].compatible);
    assert!(!result.rows[2].compatible);
    assert!(result.rows[3].implied_lower > 0.0);
    assert!(result.rows[3].compatible);
}

#[test]
fn a_nonphysical_or_unrepresentable_quantity_is_refused() {
    let negative =
        gas_density_consistency(&[row(100.0, -1.0, 300.0, 0.0, 1.0, 0.0, 0.02, 0.0, 20.0, 0.0)])
            .unwrap_err();
    assert!(negative.to_string().contains("nonnegative bound"));

    let zero =
        gas_density_consistency(&[row(0.0, 0.0, 300.0, 0.0, 1.0, 0.0, 0.02, 0.0, 20.0, 0.0)])
            .unwrap_err();
    assert!(zero.to_string().contains("positive endpoints"));

    let nonpositive =
        gas_density_consistency(&[row(1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0)]).unwrap();
    assert!(!nonpositive.rows[0].compatible);
}
