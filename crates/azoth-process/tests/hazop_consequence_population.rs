//! `process.hazop_consequence_population` against
//! `validation/neqsim/captures/process_hazop_consequence_population.tsv`.

use azoth_process::models::hazop_consequence_population::{
    HazopConsequencePopulationResult, HazopDeviationInput, hazop_consequence_population,
};

const OVERPRESSURE: &str =
    "Overpressure of equipment beyond MAWP from blocked outlet, gas blow-by or settle-out";
const DEADHEAD: &str =
    "Pump deadhead — blocked discharge causing temperature rise and shutoff overpressure";

fn row(
    guide_word: &str,
    parameter: &str,
    consequence: &str,
    safeguard: &str,
    recommendation: Option<&str>,
) -> HazopDeviationInput {
    HazopDeviationInput {
        guide_word: guide_word.to_string(),
        parameter: parameter.to_string(),
        cause: "TBD".to_string(),
        consequence: consequence.to_string(),
        safeguard: safeguard.to_string(),
        recommendation: recommendation.map(str::to_string),
    }
}

fn populate(
    deviations: &[HazopDeviationInput],
    equipment_type: Option<&str>,
) -> HazopConsequencePopulationResult {
    hazop_consequence_population(deviations, equipment_type).expect("the captured state is valid")
}

#[test]
fn the_manual_node_fills_two_of_three_rows() {
    let result = populate(
        &[
            row("more", "pressure", "TBD", "TBD", None),
            row("no", "flow", "TBD", "TBD", None),
            row("as_well_as", "level", "TBD", "TBD", None),
        ],
        None,
    );
    assert_eq!(result.mapped_count, 2.0);
    assert_eq!(result.populated[0].consequence, OVERPRESSURE);
    assert_eq!(
        result.populated[0].safeguard,
        "Pressure safety valve (PSV) and/or high-integrity pressure protection system (HIPPS)"
    );
    assert_eq!(
        result.populated[0].recommendation.as_deref(),
        Some(
            "Quantify with ReliefValveSizing, SettleOutPressureAnalyzer, GasBlowbyAnalyzer per \
             API 520 / API 521"
        )
    );
    assert_eq!(result.populated[1].consequence, DEADHEAD);
    assert_eq!(
        result.populated[1].recommendation.as_deref(),
        Some("Quantify with PumpDeadheadAnalyzer per API 610")
    );
    assert_eq!(result.populated[2].consequence, "TBD");
    assert_eq!(result.populated[2].safeguard, "TBD");
    assert_eq!(result.populated[2].recommendation, None);
}

#[test]
fn the_compressor_takes_its_equipment_entries_and_the_neutral_fallback() {
    let result = populate(
        &[
            row("no", "flow", "TBD", "TBD", None),
            row("reverse", "flow", "TBD", "TBD", None),
            row("more", "flow", "TBD", "TBD", None),
            row("less", "pressure", "TBD", "TBD", None),
        ],
        Some("Compressor"),
    );
    assert_eq!(result.mapped_count, 4.0);
    assert_eq!(
        result.populated[0].consequence,
        "Loss of forward flow can drive a running compressor into surge or recycle heating"
    );
    assert_eq!(
        result.populated[1].consequence,
        "Reverse flow can cause compressor backspin, surge and mechanical damage"
    );
    assert_eq!(
        result.populated[2].consequence,
        "MORE FLOW on compressor equipment has topology-dependent causes and consequences; \
         facilitator confirmation required"
    );
    assert_eq!(
        result.populated[2].recommendation.as_deref(),
        Some(
            "Facilitator confirmation required: review causes, consequence pathways, safeguards \
             and calculation basis per IEC 61882 / CCPS"
        )
    );
    assert_eq!(
        result.populated[3].consequence,
        "Vacuum collapse of a vessel from cooling, draining or blocked make-up gas"
    );
    assert_eq!(
        result.populated[3].recommendation.as_deref(),
        Some("Quantify with VacuumCollapseAnalyzer per API 521 / ASME BPVC")
    );
}

#[test]
fn a_separator_no_flow_is_the_neutral_fallback() {
    let result = populate(&[row("no", "flow", "TBD", "TBD", None)], Some("Separator"));
    assert_eq!(
        result.populated[0].consequence,
        "NO FLOW on separator equipment has topology-dependent causes and consequences; \
         facilitator confirmation required"
    );
    assert_eq!(
        result.populated[0].safeguard,
        "Confirm applicable alarms, trips, isolation, relief and operating safeguards in the \
         HAZOP workshop"
    );
}

#[test]
fn every_category_names_itself_in_the_neutral_fallback() {
    for (equipment_type, guide_word, deviation, word) in [
        ("Pump", "more", "MORE", "pump"),
        ("ThrottlingValve", "reverse", "REVERSE", "valve"),
        ("Stream", "reverse", "REVERSE", "stream"),
        ("Separator", "reverse", "REVERSE", "separator"),
        ("Cooler", "reverse", "REVERSE", "heat exchanger"),
        ("PipeBeggsAndBrills", "reverse", "REVERSE", "pipeline"),
        ("Reactor", "reverse", "REVERSE", "other"),
    ] {
        let result = populate(
            &[row(guide_word, "flow", "TBD", "TBD", None)],
            Some(equipment_type),
        );
        assert_eq!(
            result.populated[0].consequence,
            format!(
                "{deviation} FLOW on {word} equipment has topology-dependent causes and \
                 consequences; facilitator confirmation required"
            ),
            "{equipment_type}"
        );
    }
}

#[test]
fn a_filled_cell_is_preserved_and_the_cause_is_always_copied() {
    let result = populate(
        &[HazopDeviationInput {
            guide_word: "no".to_string(),
            parameter: "flow".to_string(),
            cause: "Closed discharge valve".to_string(),
            consequence: "Custom consequence text".to_string(),
            safeguard: " TBD ".to_string(),
            recommendation: Some("Existing recommendation".to_string()),
        }],
        Some("Pump"),
    );
    assert_eq!(result.populated[0].cause, "Closed discharge valve");
    assert_eq!(result.populated[0].consequence, "Custom consequence text");
    assert_eq!(
        result.populated[0].safeguard,
        "Minimum-flow recycle line, low-flow trip"
    );
    assert_eq!(
        result.populated[0].recommendation.as_deref(),
        Some("Existing recommendation")
    );
}

#[test]
fn an_unmapped_row_is_copied_unchanged() {
    let result = populate(
        &[HazopDeviationInput {
            guide_word: "as_well_as".to_string(),
            parameter: "level".to_string(),
            cause: "Cause".to_string(),
            consequence: "Consequence".to_string(),
            safeguard: "Safeguard".to_string(),
            recommendation: Some("Recommendation".to_string()),
        }],
        None,
    );
    assert_eq!(result.mapped_count, 0.0);
    assert_eq!(result.populated[0].cause, "Cause");
    assert_eq!(result.populated[0].consequence, "Consequence");
    assert_eq!(result.populated[0].safeguard, "Safeguard");
    assert_eq!(
        result.populated[0].recommendation.as_deref(),
        Some("Recommendation")
    );
}

#[test]
fn a_blank_equipment_type_is_generic() {
    let result = populate(&[row("no", "flow", "TBD", "TBD", None)], Some("  "));
    assert_eq!(result.populated[0].consequence, DEADHEAD);
}

#[test]
fn an_empty_list_is_no_rows() {
    let result = populate(&[], Some("Pump"));
    assert!(result.populated.is_empty());
    assert_eq!(result.mapped_count, 0.0);
}

#[test]
fn an_unknown_guide_word_is_refused() {
    assert!(
        hazop_consequence_population(&[row("perhaps", "flow", "TBD", "TBD", None)], None).is_err()
    );
}
