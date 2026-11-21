//! `pvt.oil_material_balance` against its capture.

use azoth_pvt::{OilMaterialBalanceResult, oil_material_balance};

#[derive(Default)]
struct Inputs {
    calculation: String,
    np: Option<f64>,
    rp: Option<f64>,
    bo: Option<f64>,
    rs: Option<f64>,
    bg: Option<f64>,
    wp: Option<f64>,
    bw: Option<f64>,
    boi: Option<f64>,
    rsi: Option<f64>,
    bgi: Option<f64>,
    cw: Option<f64>,
    swi: Option<f64>,
    cf: Option<f64>,
    delta_p: Option<f64>,
    m: Option<f64>,
    n: Option<f64>,
    eo_term: Option<f64>,
    eg_term: Option<f64>,
    efw_term: Option<f64>,
    we_value: Option<f64>,
    f_value: Option<f64>,
    f_history: Option<Vec<f64>>,
    eo_history: Option<Vec<f64>>,
    eg_history: Option<Vec<f64>>,
    we_history: Option<Vec<f64>>,
}

fn blank(calculation: &str) -> Inputs {
    Inputs {
        calculation: calculation.to_owned(),
        ..Inputs::default()
    }
}

fn call(input: Inputs) -> azoth_core::Result<OilMaterialBalanceResult> {
    oil_material_balance(
        &input.calculation,
        input.np,
        input.rp,
        input.bo,
        input.rs,
        input.bg,
        input.wp,
        input.bw,
        input.boi,
        input.rsi,
        input.bgi,
        input.cw,
        input.swi,
        input.cf,
        input.delta_p,
        input.m,
        input.n,
        input.eo_term,
        input.eg_term,
        input.efw_term,
        input.we_value,
        input.f_value,
        input.f_history.as_deref(),
        input.eo_history.as_deref(),
        input.eg_history.as_deref(),
        input.we_history.as_deref(),
    )
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1.0e-9 * expected.abs().max(1.0)
}

#[test]
fn the_terms_match_the_capture() {
    let mut input = blank("withdrawal_f");
    input.np = Some(1.0e6);
    input.rp = Some(500.0);
    input.bo = Some(1.2);
    input.rs = Some(300.0);
    input.bg = Some(0.005);
    input.wp = Some(1.0e5);
    input.bw = Some(1.01);
    assert!(close(call(input).unwrap().f.unwrap(), 2_301_000.0));

    let mut input = blank("eo");
    input.bo = Some(1.2);
    input.boi = Some(1.3);
    input.rsi = Some(400.0);
    input.rs = Some(300.0);
    input.bg = Some(0.005);
    assert!(close(
        call(input).unwrap().eo_value.unwrap(),
        0.399_999_999_999_999_9
    ));

    let mut input = blank("eg");
    input.boi = Some(1.3);
    input.bg = Some(0.005);
    input.bgi = Some(0.004);
    assert!(close(call(input).unwrap().eg_value.unwrap(), 0.325));

    let mut input = blank("efw");
    input.boi = Some(1.3);
    input.m = Some(0.2);
    input.cw = Some(4.0e-5);
    input.swi = Some(0.2);
    input.cf = Some(3.0e-5);
    input.delta_p = Some(50.0);
    assert!(close(
        call(input).unwrap().efw_value.unwrap(),
        0.003_705_000_000_000_001
    ));
}

#[test]
fn the_fits_match_the_capture() {
    let eo = vec![0.1, 0.2, 0.3];

    let mut input = blank("fit_depletion_drive");
    input.f_history = Some(vec![1.0e6, 2.0e6, 3.0e6]);
    input.eo_history = Some(eo.clone());
    let row = call(input).unwrap();
    assert!(close(row.ooip.unwrap(), 9_999_999.999_999_998));
    assert!(close(row.m_fit.unwrap(), 0.0));
    assert!(close(row.r_squared.unwrap(), 1.0));

    let mut input = blank("fit_gas_cap_drive");
    input.f_history = Some(vec![1.03e6, 2.036e6, 3.06e6]);
    input.eo_history = Some(eo.clone());
    input.eg_history = Some(vec![0.015, 0.018, 0.03]);
    let row = call(input).unwrap();
    assert!(close(row.ooip.unwrap(), 9_999_999.999_999_957));
    assert!(close(row.m_fit.unwrap(), 0.200_000_000_000_053_88));
    assert!(close(row.r_squared.unwrap(), 1.0));

    let mut input = blank("fit_water_drive");
    input.f_history = Some(vec![1.0e6, 2.101e6, 3.202e6]);
    input.eo_history = Some(eo);
    input.we_history = Some(vec![0.0, 1.0e5, 2.0e5]);
    input.bw = Some(1.01);
    let row = call(input).unwrap();
    assert!(close(row.ooip.unwrap(), 1.0e7));
    assert!(close(row.m_fit.unwrap(), 0.0));
    assert!(close(row.r_squared.unwrap(), 1.0));
}

#[test]
fn the_drive_indices_match_the_capture() {
    let mut input = blank("drive_indices");
    input.n = Some(1.0e7);
    input.m = Some(0.2);
    input.eo_term = Some(0.2);
    input.eg_term = Some(0.02);
    input.efw_term = Some(0.001);
    input.we_value = Some(1.0e5);
    input.bw = Some(1.01);
    input.f_value = Some(3.0e6);
    let row = call(input).unwrap();
    assert!(close(row.ddi.unwrap(), 0.666_666_666_666_666_6));
    assert!(close(row.sdi.unwrap(), 0.013_333_333_333_333_334));
    assert!(close(row.wdi.unwrap(), 0.033_666_666_666_666_664));
    assert!(close(row.edi.unwrap(), 0.003_333_333_333_333_333_5));
}

#[test]
fn a_zero_withdrawal_answers_four_zero_indices() {
    let mut input = blank("drive_indices");
    input.n = Some(1.0e7);
    input.m = Some(0.2);
    input.eo_term = Some(0.2);
    input.eg_term = Some(0.02);
    input.efw_term = Some(0.001);
    input.we_value = Some(1.0e5);
    input.bw = Some(1.01);
    input.f_value = Some(0.0);
    let row = call(input).unwrap();
    assert_eq!(row.ddi, Some(0.0));
    assert_eq!(row.sdi, Some(0.0));
    assert_eq!(row.wdi, Some(0.0));
    assert_eq!(row.edi, Some(0.0));
}

#[test]
fn a_zero_eo_in_the_water_drive_plot_is_refused() {
    let mut input = blank("fit_water_drive");
    input.f_history = Some(vec![1.0e6, 2.0e6]);
    input.eo_history = Some(vec![0.1, 0.0]);
    input.we_history = Some(vec![0.0, 1.0e5]);
    input.bw = Some(1.01);
    let err = call(input).expect_err("a zero Eo is not a plot");
    assert!(err.to_string().contains("eo_history[1]"));
}
