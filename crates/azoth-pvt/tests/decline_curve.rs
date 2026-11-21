//! `pvt.decline_curve` against its capture.

use azoth_pvt::{DeclineCurveResult, decline_curve};

#[derive(Default)]
struct Inputs {
    calculation: String,
    qi: Option<f64>,
    di: Option<f64>,
    b: Option<f64>,
    t: Option<f64>,
    q_limit: Option<f64>,
    d_nominal: Option<f64>,
    d_effective: Option<f64>,
    q1: Option<f64>,
    t1: Option<f64>,
    q2: Option<f64>,
    t2: Option<f64>,
    q3: Option<f64>,
    t3: Option<f64>,
    t_start: Option<f64>,
    t_end: Option<f64>,
    t_step: Option<f64>,
    t_current: Option<f64>,
    a: Option<f64>,
    m: Option<f64>,
    start_index: Option<f64>,
    end_index: Option<f64>,
    history_time: Option<Vec<f64>>,
    history_rate: Option<Vec<f64>>,
}

fn blank(calculation: &str) -> Inputs {
    Inputs {
        calculation: calculation.to_owned(),
        ..Inputs::default()
    }
}

fn call(input: Inputs) -> azoth_core::Result<DeclineCurveResult> {
    decline_curve(
        &input.calculation,
        input.qi,
        input.di,
        input.b,
        input.t,
        input.q_limit,
        input.d_nominal,
        input.d_effective,
        input.q1,
        input.t1,
        input.q2,
        input.t2,
        input.q3,
        input.t3,
        input.t_start,
        input.t_end,
        input.t_step,
        input.t_current,
        input.a,
        input.m,
        input.start_index,
        input.end_index,
        input.history_time.as_deref(),
        input.history_rate.as_deref(),
    )
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-9 * expected.abs().max(1.0)
}

fn history() -> (Option<Vec<f64>>, Option<Vec<f64>>) {
    let times: Vec<f64> = (0..11).map(|i| i as f64 * 365.0).collect();
    let rates: Vec<f64> = times.iter().map(|t| 1000.0 * (-0.001 * t).exp()).collect();
    (Some(times), Some(rates))
}

#[test]
fn the_arps_forms_match_the_capture() {
    for (b, rate, cumulative) in [
        (0.0, 694.023_123_407_096_9, 974_008.871_221_244_7),
        (0.5, 714.999_748_465_322_8, 1_292_035.398_230_088_4),
        (1.0, 732.466_581_212_232_2, 1_536_867.219_599_265),
    ] {
        let mut input = blank("rate");
        input.qi = Some(1000.0);
        input.di = Some(0.001);
        input.b = Some(b);
        input.t = Some(365.25);
        assert!(close(call(input).unwrap().rate_value.unwrap(), rate));

        let mut input = blank("cumulative_production");
        input.qi = Some(1000.0);
        input.di = Some(0.001);
        input.b = Some(b);
        input.t = Some(3650.0);
        assert!(close(
            call(input).unwrap().cumulative_value.unwrap(),
            cumulative
        ));
    }

    let mut input = blank("rate_exponential");
    input.qi = Some(1000.0);
    input.di = Some(0.001);
    input.t = Some(365.25);
    assert!(close(
        call(input).unwrap().rate_value.unwrap(),
        694.023_123_407_096_9
    ));

    let mut input = blank("rate_harmonic");
    input.qi = Some(1000.0);
    input.di = Some(0.001);
    input.t = Some(365.25);
    assert!(close(
        call(input).unwrap().rate_value.unwrap(),
        732.466_581_212_232_2
    ));

    let mut input = blank("cumulative_exponential");
    input.qi = Some(1000.0);
    input.di = Some(0.001);
    input.t = Some(3650.0);
    assert!(close(
        call(input).unwrap().cumulative_value.unwrap(),
        974_008.871_221_244_7
    ));
}

#[test]
fn the_conversions_and_the_summary_match_the_capture() {
    let mut input = blank("nominal_to_effective_annual");
    input.d_nominal = Some(0.001);
    assert!(close(
        call(input).unwrap().effective.unwrap(),
        0.305_976_876_592_903_07
    ));

    let mut input = blank("effective_to_nominal_annual");
    input.d_effective = Some(0.305_976_876_592_903_07);
    assert!(close(
        call(input).unwrap().nominal.unwrap(),
        0.001_000_000_000_000_000_2
    ));

    let mut input = blank("instantaneous_decline_rate");
    input.di = Some(0.001);
    input.b = Some(0.5);
    input.t = Some(1000.0);
    assert!(close(
        call(input).unwrap().instantaneous_decline_rate.unwrap(),
        6.666_666_666_666_666e-4
    ));

    let mut input = blank("time_to_rate");
    input.qi = Some(1000.0);
    input.di = Some(0.001);
    input.b = Some(0.5);
    input.q_limit = Some(50.0);
    assert!(close(
        call(input).unwrap().time_to_rate.unwrap(),
        6_944.271_909_999_159
    ));

    let mut input = blank("eur");
    input.qi = Some(1000.0);
    input.di = Some(0.001);
    input.b = Some(0.5);
    input.q_limit = Some(50.0);
    assert!(close(
        call(input).unwrap().eur.unwrap(),
        1_552_786.404_500_042
    ));

    let mut input = blank("remaining_reserves");
    input.qi = Some(1000.0);
    input.di = Some(0.001);
    input.b = Some(0.5);
    input.t_current = Some(1000.0);
    input.q_limit = Some(50.0);
    assert!(close(
        call(input).unwrap().remaining.unwrap(),
        886_119.737_833_375_3
    ));

    let mut input = blank("summary");
    input.qi = Some(1000.0);
    input.di = Some(0.001);
    input.b = Some(0.5);
    input.q_limit = Some(50.0);
    let row = call(input).unwrap();
    assert!(close(
        row.summary_effective.unwrap(),
        0.305_976_876_592_903_07
    ));
    assert!(close(row.summary_rate_1yr.unwrap(), 714.999_748_465_322_8));
    assert!(close(row.summary_rate_5yr.unwrap(), 273.220_515_317_371_8));
    assert!(close(
        row.summary_rate_10yr.unwrap(),
        125.192_654_868_693_44
    ));
    assert!(close(row.summary_cum_1yr.unwrap(), 308_846.844_942_395_1));
    assert!(close(row.summary_cum_5yr.unwrap(), 954_590.003_266_906_1));
    assert!(close(
        row.summary_cum_10yr.unwrap(),
        1_292_348.518_354_710_4
    ));
    assert!(close(
        row.summary_time_to_limit.unwrap(),
        6_944.271_909_999_159
    ));
    assert!(close(
        row.summary_time_to_limit_years.unwrap(),
        19.012_380_314_850_54
    ));
    assert!(close(row.summary_eur.unwrap(), 1_552_786.404_500_042));
}

#[test]
fn the_forecast_matches_the_capture() {
    let mut input = blank("forecast");
    input.qi = Some(1000.0);
    input.di = Some(0.001);
    input.b = Some(0.5);
    input.t_start = Some(0.0);
    input.t_end = Some(3650.0);
    input.t_step = Some(365.0);
    let row = call(input).unwrap();
    let times = row.time.unwrap();
    let rates = row.rate.unwrap();
    let cumulative = row.cumulative.unwrap();
    assert_eq!(times.len(), 11);
    assert!(close(times[10], 3650.0));
    assert!(close(rates[1], 715.150_919_192_415_7));
    assert!(close(rates[10], 125.303_469_339_807_32));
    assert!(close(cumulative[1], 308_668.076_109_936_75));
    assert!(close(cumulative[10], 1_292_035.398_230_088_4));
}

#[test]
fn the_fits_match_the_capture() {
    let mut input = blank("estimate_exponential_decline");
    input.q1 = Some(1000.0);
    input.t1 = Some(0.0);
    input.q2 = Some(800.0);
    input.t2 = Some(365.0);
    assert!(close(
        call(input).unwrap().exponential_decline.unwrap(),
        6.113_521_953_813_966e-4
    ));

    let mut input = blank("estimate_hyperbolic_parameters");
    input.q1 = Some(1000.0);
    input.t1 = Some(0.0);
    input.q2 = Some(700.0);
    input.t2 = Some(365.0);
    input.q3 = Some(500.0);
    input.t3 = Some(730.0);
    let row = call(input).unwrap();
    assert!(close(row.fit_qi.unwrap(), 1000.0));
    assert!(close(row.fit_di.unwrap(), 0.001_007_723_486_614_096_8));
    assert!(close(row.fit_b.unwrap(), 0.17));

    let (times, rates) = history();
    let mut input = blank("fit_arps");
    input.history_time = times.clone();
    input.history_rate = rates.clone();
    let row = call(input).unwrap();
    assert!(close(row.fit_qi.unwrap(), 1_000.000_000_000_002_4));
    assert!(close(row.fit_di.unwrap(), 0.001_000_000_000_000_000_7));
    assert!(close(row.fit_b.unwrap(), 0.0));
    assert!(close(row.fit_r_squared.unwrap(), 1.0));

    let mut input = blank("eur_from_fit");
    input.qi = Some(1_000.000_000_000_002_4);
    input.di = Some(0.001_000_000_000_000_000_7);
    input.b = Some(0.0);
    input.q_limit = Some(50.0);
    assert!(close(
        call(input).unwrap().eur.unwrap(),
        950_000.000_000_001_9
    ));

    let mut input = blank("fit_arps_window");
    input.history_time = times;
    input.history_rate = rates;
    input.start_index = Some(2.0);
    input.end_index = Some(10.0);
    let row = call(input).unwrap();
    assert!(close(row.fit_qi.unwrap(), 481.908_990_090_202_5));
    assert!(close(row.fit_di.unwrap(), 0.001));
    assert!(close(row.fit_b.unwrap(), 0.0));
}

#[test]
fn the_duong_model_matches_the_capture() {
    let mut input = blank("duong_time_function");
    input.a = Some(0.5);
    input.m = Some(1.2);
    input.t = Some(365.0);
    assert!(close(
        call(input).unwrap().duong_time.unwrap(),
        0.004_757_231_291_140_787
    ));

    let mut input = blank("rate_duong");
    input.q1 = Some(1000.0);
    input.a = Some(0.5);
    input.m = Some(1.2);
    input.t = Some(365.0);
    assert!(close(
        call(input).unwrap().rate_value.unwrap(),
        4.757_231_291_140_787
    ));

    let mut input = blank("cumulative_duong");
    input.q1 = Some(1000.0);
    input.a = Some(0.5);
    input.m = Some(1.2);
    input.t = Some(365.0);
    assert!(close(
        call(input).unwrap().cumulative_value.unwrap(),
        11_301.489_863_608_776
    ));

    let times: Vec<f64> = (1..=10).map(|i| i as f64 * 365.0).collect();
    let rates: Vec<f64> = times
        .iter()
        .map(|t| 1000.0 * decline_time(0.5, 1.2, *t))
        .collect();
    let mut input = blank("fit_duong");
    input.history_time = Some(times);
    input.history_rate = Some(rates);
    let row = call(input).unwrap();
    assert!(close(row.duong_q1.unwrap(), 1.427_758_425_538_196_9e-13));
    assert!(close(row.duong_a.unwrap(), 23.253_693_140_077_13));
    assert!(close(row.duong_m.unwrap(), 1.555_859_918_330_947_6));
    assert!(close(row.fit_r_squared.unwrap(), 0.988_342_858_158_993_3));
}

fn decline_time(a: f64, m: f64, t: f64) -> f64 {
    let exponent = a / (1.0 - m) * (t.powf(1.0 - m) - 1.0);
    t.powf(-m) * exponent.exp()
}

#[test]
fn a_zero_decline_answers_an_absent_limit_and_a_warning() {
    let mut input = blank("time_to_rate");
    input.qi = Some(1000.0);
    input.di = Some(0.0);
    input.b = Some(0.5);
    input.q_limit = Some(50.0);
    let row = call(input).unwrap();
    assert!(row.time_to_rate.is_none());
    assert_eq!(row.warnings.len(), 1);
}

#[test]
fn a_missing_scalar_and_a_short_history_are_refused() {
    let input = blank("rate");
    let err = call(input).expect_err("qi is missing");
    assert!(err.to_string().contains("qi"));

    let mut input = blank("fit_arps");
    input.history_time = Some(vec![0.0, 1.0]);
    input.history_rate = Some(vec![100.0, 90.0]);
    let err = call(input).expect_err("two points are not a fit");
    assert!(err.to_string().contains("at least three"));
}
