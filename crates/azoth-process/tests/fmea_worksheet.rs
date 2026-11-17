//! `process.fmea_worksheet` against
//! `validation/neqsim/captures/process_fmea_worksheet.tsv`.

use azoth_process::models::fmea_worksheet::{FmeaEntry, FmeaWorksheetResult, fmea_worksheet};

fn entry(
    component: &str,
    severity: f64,
    occurrence: f64,
    detection: f64,
    mitigation: Option<&str>,
) -> FmeaEntry {
    FmeaEntry {
        component: component.to_string(),
        failure_mode: "mode".to_string(),
        effect: "effect".to_string(),
        cause: "cause".to_string(),
        severity,
        occurrence,
        detection,
        mitigation: mitigation.map(str::to_string),
    }
}

fn components(result: &FmeaWorksheetResult) -> Vec<&str> {
    result
        .ranked_entries
        .iter()
        .map(|row| row.component.as_str())
        .collect()
}

#[test]
fn the_class_test_ranks_120_ahead_of_108() {
    let entries = vec![
        FmeaEntry {
            component: "Bearing".to_string(),
            failure_mode: "Wear".to_string(),
            effect: "Vibration".to_string(),
            cause: "Lub failure".to_string(),
            severity: 5.0,
            occurrence: 4.0,
            detection: 6.0,
            mitigation: Some("VIB monitor".to_string()),
        },
        FmeaEntry {
            component: "Seal".to_string(),
            failure_mode: "Leak".to_string(),
            effect: "HC release".to_string(),
            cause: "Aging".to_string(),
            severity: 9.0,
            occurrence: 3.0,
            detection: 4.0,
            mitigation: Some("Online seal monitor".to_string()),
        },
    ];
    let result = fmea_worksheet(&entries, 110.0).expect("the captured state is valid");
    assert_eq!(components(&result), vec!["Bearing", "Seal"]);
    assert_eq!(result.ranked_entries[0].rpn, 120.0);
    assert_eq!(result.ranked_entries[1].rpn, 108.0);
    assert_eq!(
        result
            .entries_above_threshold
            .iter()
            .map(|row| row.component.as_str())
            .collect::<Vec<_>>(),
        vec!["Bearing"]
    );
    assert_eq!(result.highest_rpn, Some(120.0));
    assert_eq!(result.entry_count, 2.0);
}

#[test]
fn a_tie_keeps_caller_order_and_the_threshold_is_inclusive() {
    let entries = vec![
        entry("A", 5.0, 4.0, 6.0, Some("monitor A")),
        entry("B", 4.0, 5.0, 6.0, None),
        entry("C", 2.0, 3.0, 4.0, Some("strainer")),
    ];
    let result = fmea_worksheet(&entries, 120.0).expect("the captured state is valid");
    assert_eq!(components(&result), vec!["A", "B", "C"]);
    assert_eq!(result.ranked_entries[0].rpn, result.ranked_entries[1].rpn);
    assert_eq!(
        result
            .entries_above_threshold
            .iter()
            .map(|row| row.component.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "B"]
    );
    assert_eq!(result.ranked_entries[1].mitigation, None);
    assert_eq!(result.highest_rpn, Some(120.0));
}

#[test]
fn the_threshold_filter_keeps_caller_order_not_rank_order() {
    let entries = vec![
        entry("low", 2.0, 3.0, 4.0, None),
        entry("high", 10.0, 10.0, 10.0, None),
    ];
    let result = fmea_worksheet(&entries, 1.0).expect("the captured state is valid");
    assert_eq!(components(&result), vec!["high", "low"]);
    assert_eq!(
        result
            .entries_above_threshold
            .iter()
            .map(|row| row.component.as_str())
            .collect::<Vec<_>>(),
        vec!["low", "high"]
    );
}

#[test]
fn an_empty_worksheet_has_no_highest_rpn() {
    let result = fmea_worksheet(&[], 100.0).expect("an empty worksheet is a state");
    assert!(result.ranked_entries.is_empty());
    assert!(result.entries_above_threshold.is_empty());
    assert_eq!(result.highest_rpn, None);
    assert_eq!(result.entry_count, 0.0);
}

#[test]
fn scores_outside_one_to_ten_are_refused() {
    for (severity, occurrence, detection) in [
        (0.0, 5.0, 5.0),
        (11.0, 5.0, 5.0),
        (5.0, 0.0, 5.0),
        (5.0, 11.0, 5.0),
        (5.0, 5.0, 0.0),
        (5.0, 5.0, 11.0),
        (5.5, 5.0, 5.0),
    ] {
        assert!(
            fmea_worksheet(&[entry("X", severity, occurrence, detection, None)], 100.0).is_err(),
            "{severity}, {occurrence}, {detection} is not a legal score"
        );
    }
}

#[test]
fn a_non_whole_threshold_is_refused() {
    assert!(fmea_worksheet(&[entry("X", 5.0, 5.0, 5.0, None)], 100.5).is_err());
}
