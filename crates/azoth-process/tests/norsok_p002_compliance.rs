//! `process.norsok_p002_compliance` against
//! `validation/neqsim/captures/process_norsok_p002_compliance.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{meters_per_second, pascals};
use azoth_process::models::norsok_p002_compliance::{
    NorsokP002ComplianceResult, P002CheckRow, P002Criterion, norsok_p002_compliance,
};

fn row(name: &str, criterion: &str) -> P002CheckRow {
    P002CheckRow {
        name: name.to_string(),
        criterion: criterion.to_string(),
        ..P002CheckRow::default()
    }
}

fn evaluate(rows: &[P002CheckRow]) -> NorsokP002ComplianceResult {
    norsok_p002_compliance(rows).expect("the captured state is valid")
}

fn all_pass() -> Vec<P002CheckRow> {
    vec![
        P002CheckRow {
            mach_number: Some(0.5),
            ..row("flare", "flare_line_mach_07")
        },
        P002CheckRow {
            momentum_flux: Some(pascals(150000.0)),
            ..row("blowdown", "blowdown_rho_v2")
        },
        P002CheckRow {
            momentum_flux: Some(pascals(90000.0)),
            ..row("line", "erosional_velocity")
        },
        P002CheckRow {
            vent_velocity: Some(meters_per_second(50.0)),
            ..row("vent", "vent_gas_velocity")
        },
        P002CheckRow {
            carry_over_fraction: Some(5.0e-4),
            ..row("separator", "liquid_carry_over")
        },
        P002CheckRow {
            recorded_ok: Some(true),
            ..row("bdv", "depressurisation_valve_size")
        },
        P002CheckRow {
            recorded_ok: Some(true),
            ..row("drain", "drain_slope_capacity")
        },
    ]
}

#[test]
fn the_captured_findings_preserve_the_callers_order() {
    let result = evaluate(&all_pass());
    assert!(result.compliant);
    assert_eq!(result.non_compliant_count, 0.0);
    assert_eq!(result.findings.len(), 7);
    let criteria: Vec<P002Criterion> = result.findings.iter().map(|row| row.criterion).collect();
    assert_eq!(
        criteria,
        vec![
            P002Criterion::FlareLineMach07,
            P002Criterion::BlowdownRhoV2,
            P002Criterion::ErosionalVelocity,
            P002Criterion::VentGasVelocity,
            P002Criterion::LiquidCarryOver,
            P002Criterion::DepressurisationValveSize,
            P002Criterion::DrainSlopeCapacity,
        ]
    );
    assert!(result.findings.iter().all(|row| row.compliant));
}

#[test]
fn every_criterion_can_fail() {
    let rows = vec![
        P002CheckRow {
            mach_number: Some(0.8),
            ..row("flare", "flare_line_mach_07")
        },
        P002CheckRow {
            momentum_flux: Some(pascals(250000.0)),
            ..row("blowdown", "blowdown_rho_v2")
        },
        P002CheckRow {
            momentum_flux: Some(pascals(150000.0)),
            ..row("line", "erosional_velocity")
        },
        P002CheckRow {
            vent_velocity: Some(meters_per_second(70.0)),
            ..row("vent", "vent_gas_velocity")
        },
        P002CheckRow {
            carry_over_fraction: Some(2.0e-3),
            ..row("separator", "liquid_carry_over")
        },
        P002CheckRow {
            recorded_ok: Some(false),
            ..row("bdv", "depressurisation_valve_size")
        },
        P002CheckRow {
            recorded_ok: Some(false),
            ..row("drain", "drain_slope_capacity")
        },
    ];
    let result = evaluate(&rows);
    assert!(!result.compliant);
    assert_eq!(result.non_compliant_count, 7.0);
}

#[test]
fn the_comparison_is_inclusive_at_every_default_limit() {
    let rows = vec![
        P002CheckRow {
            mach_number: Some(0.7),
            ..row("flare", "flare_line_mach_07")
        },
        P002CheckRow {
            momentum_flux: Some(pascals(200000.0)),
            ..row("blowdown", "blowdown_rho_v2")
        },
        P002CheckRow {
            momentum_flux: Some(pascals(100000.0)),
            ..row("line", "erosional_velocity")
        },
        P002CheckRow {
            vent_velocity: Some(meters_per_second(60.0)),
            ..row("vent", "vent_gas_velocity")
        },
        P002CheckRow {
            carry_over_fraction: Some(1.0e-3),
            ..row("separator", "liquid_carry_over")
        },
    ];
    assert!(evaluate(&rows).compliant);
}

#[test]
fn custom_limits_and_recorded_checks_are_the_callers() {
    let rows = vec![
        P002CheckRow {
            mach_number: Some(0.6),
            mach_limit: Some(0.5),
            ..row("flare", "flare_line_mach_07")
        },
        P002CheckRow {
            vent_velocity: Some(meters_per_second(25.0)),
            vent_velocity_limit: Some(meters_per_second(30.0)),
            ..row("vent", "vent_gas_velocity")
        },
    ];
    let result = evaluate(&rows);
    assert!(!result.compliant);
    assert!(!result.findings[0].compliant);
    assert!(result.findings[1].compliant);

    let recorded = vec![
        P002CheckRow {
            recorded_ok: Some(true),
            ..row("bdv", "depressurisation_valve_size")
        },
        P002CheckRow {
            recorded_ok: Some(false),
            ..row("drain", "drain_slope_capacity")
        },
    ];
    let result = evaluate(&recorded);
    assert!(!result.compliant);
    assert_eq!(result.non_compliant_count, 1.0);
}

#[test]
fn an_empty_check_list_is_compliant() {
    let result = evaluate(&[]);
    assert!(result.compliant);
    assert_eq!(result.non_compliant_count, 0.0);
    assert!(result.findings.is_empty());
}

#[test]
fn the_refusals_are_the_row_shape() {
    // An unknown criterion.
    assert!(norsok_p002_compliance(&[row("x", "not_a_criterion")]).is_err());
    // A numeric criterion missing its value.
    assert!(norsok_p002_compliance(&[row("flare", "flare_line_mach_07")]).is_err());
    // A row carrying another criterion's value or a pass flag.
    assert!(
        norsok_p002_compliance(&[P002CheckRow {
            mach_number: Some(0.5),
            momentum_flux: Some(pascals(1000.0)),
            ..row("flare", "flare_line_mach_07")
        }])
        .is_err()
    );
    assert!(
        norsok_p002_compliance(&[P002CheckRow {
            mach_number: Some(0.5),
            recorded_ok: Some(true),
            ..row("flare", "flare_line_mach_07")
        }])
        .is_err()
    );
    // A recorded criterion missing its flag and carrying a numeric value.
    assert!(norsok_p002_compliance(&[row("bdv", "depressurisation_valve_size")]).is_err());
    assert!(
        norsok_p002_compliance(&[P002CheckRow {
            recorded_ok: Some(true),
            vent_velocity: Some(meters_per_second(1.0)),
            ..row("bdv", "depressurisation_valve_size")
        }])
        .is_err()
    );
    // A non-finite value or limit.
    assert!(
        norsok_p002_compliance(&[P002CheckRow {
            mach_number: Some(f64::NAN),
            ..row("flare", "flare_line_mach_07")
        }])
        .is_err()
    );
    assert!(
        norsok_p002_compliance(&[P002CheckRow {
            mach_number: Some(0.5),
            mach_limit: Some(f64::INFINITY),
            ..row("flare", "flare_line_mach_07")
        }])
        .is_err()
    );
}
