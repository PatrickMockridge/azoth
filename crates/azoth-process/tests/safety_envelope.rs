//! `process.safety_envelope` against
//! `validation/neqsim/captures/process_safety_envelope.tsv`.

use azoth_core::units::{kelvins, pascals};
use azoth_process::models::safety_envelope::{
    EnvelopeKind, EnvelopeTracePoint, SafetyEnvelopeResult, safety_envelope,
};

fn close(actual: f64, expected: f64, what: &str) {
    close_with(actual, expected, what, 1.0e-6);
}

fn close_with(actual: f64, expected: f64, what: &str, tolerance: f64) {
    let error = (actual - expected).abs() / expected.abs().max(1.0);
    assert!(
        error <= tolerance,
        "{what}: {actual} against {expected}, relative error {error:.3e} over {tolerance:.3e}"
    );
}

fn point(
    result: &SafetyEnvelopeResult,
    envelope: EnvelopeKind,
    index: usize,
) -> &azoth_process::models::safety_envelope::EnvelopePoint {
    result
        .points
        .iter()
        .find(|row| row.envelope == envelope && row.index == index)
        .expect("the point exists")
}

fn query(
    result: &SafetyEnvelopeResult,
    envelope: EnvelopeKind,
) -> &azoth_process::models::safety_envelope::EnvelopeQuery {
    result
        .queries
        .iter()
        .find(|row| row.envelope == envelope)
        .expect("the query exists")
}

fn hydrate_state() -> SafetyEnvelopeResult {
    safety_envelope(
        &["methane".to_string(), "water".to_string()],
        &[0.9, 0.1],
        pascals(1.0e5),
        pascals(15.0e6),
        5,
        Some("srk"),
        None,
        None,
        None,
        None,
        None,
        Some(pascals(5.0e6)),
        Some(kelvins(280.0)),
        None,
    )
    .expect("the state evaluates")
}

#[test]
fn the_hydrate_state_runs_four_arms() {
    let result = hydrate_state();
    assert_eq!(result.envelopes.len(), 4);
    assert_eq!(result.envelopes[0].envelope, EnvelopeKind::Hydrate);
    assert!((result.envelopes[0].reference_water_content - 0.1).abs() < 1.0e-12);
    assert_eq!(result.envelopes[1].reference_water_content, 0.0);
    close(
        point(&result, EnvelopeKind::Hydrate, 2)
            .temperature
            .expect("the hydrate point is present")
            .value,
        283.75975614855724,
        "hydrate",
    );
    assert!(point(&result, EnvelopeKind::Wax, 2).temperature.is_none());
    close(
        point(&result, EnvelopeKind::Co2Freezing, 4)
            .temperature
            .expect("the CO2 point is present")
            .value,
        217.12928,
        "CO2",
    );
    // The MDMT gamma is `6.4e-2` relative below the class's on this two-phase state.
    close_with(
        point(&result, EnvelopeKind::Mdmt, 1)
            .temperature
            .expect("the MDMT point is present")
            .value,
        110.84959166754226,
        "MDMT",
        1.0e-1,
    );
    let hydrate = query(&result, EnvelopeKind::Hydrate);
    close(
        hydrate
            .temperature
            .expect("the hydrate query is present")
            .value,
        279.2637590766215,
        "hydrate limit",
    );
    close(
        hydrate.margin.expect("the hydrate margin is present").value,
        0.7362409233784888,
        "hydrate margin",
    );
    assert!(hydrate.safe);
    assert!(query(&result, EnvelopeKind::Wax).safe);
    assert_eq!(result.all_safe, Some(true));
    assert_eq!(
        result.most_limiting_envelope.as_deref(),
        Some("Hydrate Formation")
    );
    assert_eq!(
        result
            .warnings
            .iter()
            .filter(|warning| warning.field.as_deref() == Some("wax"))
            .count(),
        5
    );
}

#[test]
fn the_wax_state_is_limited_by_mdmt() {
    let result = safety_envelope(
        &[
            "methane".to_string(),
            "n-heptane".to_string(),
            "nc14".to_string(),
            "nc20".to_string(),
            "nc24".to_string(),
        ],
        &[0.4, 0.1, 0.1, 0.2, 0.2],
        pascals(1.0e5),
        pascals(2.0e6),
        5,
        Some("srk"),
        None,
        None,
        None,
        None,
        None,
        Some(pascals(5.0e5)),
        Some(kelvins(270.0)),
        None,
    )
    .expect("the state evaluates");
    close(
        point(&result, EnvelopeKind::Wax, 0)
            .temperature
            .expect("the wax point is present")
            .value,
        271.30714416503906,
        "wax",
    );
    assert!(!query(&result, EnvelopeKind::Mdmt).safe);
    assert_eq!(result.all_safe, Some(false));
    assert_eq!(result.most_limiting_envelope.as_deref(), Some("MDMT"));
    assert!(
        point(&result, EnvelopeKind::Hydrate, 0)
            .temperature
            .is_none()
    );
}

#[test]
fn the_phase_trace_is_the_container() {
    let trace = [
        (110517.09180756477, 166.71388189460214),
        (122140.27581601699, 168.18265461320092),
        (134985.88075760032, 169.67969457404368),
        (149182.46976412703, 171.2057230517833),
        (166529.11949458864, 172.91871031834745),
    ]
    .map(|(pressure, temperature)| EnvelopeTracePoint {
        pressure: pascals(pressure),
        temperature: kelvins(temperature),
    });
    let result = safety_envelope(
        &["methane".to_string(), "ethane".to_string()],
        &[0.7, 0.3],
        pascals(1.0e5),
        pascals(5.0e6),
        5,
        Some("srk"),
        None,
        None,
        None,
        None,
        None,
        Some(pascals(2.0e6)),
        Some(kelvins(250.0)),
        Some(&trace),
    )
    .expect("the state evaluates");
    assert_eq!(result.envelopes.len(), 5);
    assert_eq!(result.envelopes[4].envelope, EnvelopeKind::PhaseEnvelope);
    close(
        point(&result, EnvelopeKind::PhaseEnvelope, 1)
            .temperature
            .expect("the trace point is present")
            .value,
        168.18265461320092,
        "trace",
    );
    let phase = query(&result, EnvelopeKind::PhaseEnvelope);
    assert!(phase.temperature.is_none());
    assert!(phase.safe);
    assert_eq!(result.all_safe, Some(true));
    assert_eq!(
        result.most_limiting_envelope.as_deref(),
        Some("CO2 Freezing")
    );
}
