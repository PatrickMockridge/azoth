//! `process.sts0131_gate` against `validation/neqsim/captures/process_sts0131_gate.tsv`.

#![allow(clippy::excessive_precision)]
#![allow(clippy::inconsistent_digit_grouping)]

use azoth_core::units::{kelvins, square_meters};
use azoth_process::models::sts0131_gate::{
    Sts0131CheckRow, Sts0131CheckType, Sts0131GateResult, Sts0131Severity, sts0131_gate,
};

fn row(check_type: &str) -> Sts0131CheckRow {
    Sts0131CheckRow {
        check_type: check_type.to_string(),
        ..Sts0131CheckRow::default()
    }
}

fn evaluate(rows: &[Sts0131CheckRow]) -> Sts0131GateResult {
    sts0131_gate(rows).expect("the captured state is valid")
}

#[test]
fn an_empty_gate_is_acceptable() {
    let result = evaluate(&[]);
    assert!(result.acceptable);
    assert_eq!(result.failure_count, 0.0);
    assert!(result.findings.is_empty());
}

#[test]
fn the_psv_margin_passes_at_equality_and_carries_its_value() {
    let equal = evaluate(&[Sts0131CheckRow {
        required_area: Some(square_meters(1.0)),
        selected_area: Some(square_meters(1.1)),
        required_margin: Some(0.1),
        ..row("psv_sizing_margin")
    }]);
    assert!(equal.acceptable);
    assert_eq!(equal.findings[0].severity, Sts0131Severity::Pass);
    let margin = equal.findings[0]
        .margin
        .expect("the PSV margin is published");
    assert!((margin - 0.1).abs() < 1.0e-12);

    let below = evaluate(&[Sts0131CheckRow {
        required_area: Some(square_meters(1.0)),
        selected_area: Some(square_meters(1.05)),
        required_margin: Some(0.1),
        ..row("psv_sizing_margin")
    }]);
    assert!(!below.acceptable);
    assert_eq!(below.findings[0].severity, Sts0131Severity::Fail);
}

#[test]
fn the_mdmt_comparison_is_inclusive() {
    let equal = evaluate(&[Sts0131CheckRow {
        minimum_wall_temperature: Some(kelvins(233.15)),
        design_mdmt: Some(kelvins(233.15)),
        ..row("mdmt")
    }]);
    assert!(equal.acceptable);
    assert_eq!(equal.findings[0].severity, Sts0131Severity::Pass);

    let below = evaluate(&[Sts0131CheckRow {
        minimum_wall_temperature: Some(kelvins(228.15)),
        design_mdmt: Some(kelvins(233.15)),
        ..row("mdmt")
    }]);
    assert!(!below.acceptable);
}

#[test]
fn the_four_pass_through_arms_read_their_own_flag() {
    let passing = evaluate(&[
        Sts0131CheckRow {
            acceptable: Some(true),
            ..row("depressurisation")
        },
        Sts0131CheckRow {
            rupture_predicted: Some(false),
            ..row("trapped_liquid_rupture")
        },
        Sts0131CheckRow {
            sil_compliant: Some(true),
            ..row("sil")
        },
        Sts0131CheckRow {
            name: Some("custom".to_string()),
            custom_ok: Some(true),
            ..row("custom")
        },
    ]);
    assert!(passing.acceptable);
    assert_eq!(passing.findings.len(), 4);
    assert!(
        passing
            .findings
            .iter()
            .all(|finding| finding.severity == Sts0131Severity::Pass)
    );
    assert_eq!(passing.findings[3].name.as_deref(), Some("custom"));

    let failing = evaluate(&[
        Sts0131CheckRow {
            acceptable: Some(false),
            ..row("depressurisation")
        },
        Sts0131CheckRow {
            rupture_predicted: Some(true),
            ..row("trapped_liquid_rupture")
        },
        Sts0131CheckRow {
            sil_compliant: Some(false),
            ..row("sil")
        },
        Sts0131CheckRow {
            name: Some("custom".to_string()),
            custom_ok: Some(false),
            ..row("custom")
        },
    ]);
    assert!(!failing.acceptable);
    assert_eq!(failing.failure_count, 4.0);
}

#[test]
fn a_mixed_gate_counts_only_the_failures() {
    let result = evaluate(&[
        Sts0131CheckRow {
            acceptable: Some(false),
            ..row("depressurisation")
        },
        Sts0131CheckRow {
            required_area: Some(square_meters(1.0)),
            selected_area: Some(square_meters(1.1)),
            required_margin: Some(0.1),
            ..row("psv_sizing_margin")
        },
        Sts0131CheckRow {
            name: Some("custom".to_string()),
            custom_ok: Some(true),
            ..row("custom")
        },
    ]);
    assert!(!result.acceptable);
    assert_eq!(result.failure_count, 1.0);
    assert_eq!(
        result.findings[0].check_type,
        Sts0131CheckType::Depressurisation
    );
    assert_eq!(result.findings[0].severity, Sts0131Severity::Fail);
    assert_eq!(result.findings[1].severity, Sts0131Severity::Pass);
    assert_eq!(result.findings[2].severity, Sts0131Severity::Pass);
}

#[test]
fn the_refusals_are_the_row_shape() {
    // An unknown check type.
    assert!(sts0131_gate(&[row("not_a_check")]).is_err());
    // The non-positive-area refusal.
    assert!(
        sts0131_gate(&[Sts0131CheckRow {
            required_area: Some(square_meters(0.0)),
            selected_area: Some(square_meters(1.0)),
            required_margin: Some(0.1),
            ..row("psv_sizing_margin")
        }])
        .is_err()
    );
    // A missing field and an extra one.
    assert!(sts0131_gate(&[row("mdmt")]).is_err());
    assert!(
        sts0131_gate(&[Sts0131CheckRow {
            acceptable: Some(true),
            sil_compliant: Some(true),
            ..row("depressurisation")
        }])
        .is_err()
    );
    // A name on a built-in check and a missing name on a custom one.
    assert!(
        sts0131_gate(&[Sts0131CheckRow {
            name: Some("named".to_string()),
            acceptable: Some(true),
            ..row("depressurisation")
        }])
        .is_err()
    );
    assert!(
        sts0131_gate(&[Sts0131CheckRow {
            custom_ok: Some(true),
            ..row("custom")
        }])
        .is_err()
    );
    // A non-finite temperature.
    assert!(
        sts0131_gate(&[Sts0131CheckRow {
            minimum_wall_temperature: Some(kelvins(f64::NAN)),
            design_mdmt: Some(kelvins(233.15)),
            ..row("mdmt")
        }])
        .is_err()
    );
}
