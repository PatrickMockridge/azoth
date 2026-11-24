//! `pvt.relative_permeability` at its boundary.
//!
//! The states are the pin's, from `validation/neqsim/captures/pvt_relative_permeability.tsv`.

use azoth_pvt::{RelativePermeabilityResult, relative_permeability};

#[allow(clippy::too_many_arguments)]
fn call(
    table_type: Option<&str>,
    model_family: Option<&str>,
    rows: Option<u32>,
    swc: Option<f64>,
    critical_water_saturation: Option<f64>,
    sorw: Option<f64>,
    sorg: Option<f64>,
    sgcr: Option<f64>,
    kro_max: Option<f64>,
    krw_max: Option<f64>,
    krg_max: Option<f64>,
    no: Option<f64>,
    nw: Option<f64>,
    ng: Option<f64>,
    nog: Option<f64>,
    lo: Option<f64>,
    eo: Option<f64>,
    to: Option<f64>,
    lw: Option<f64>,
    ew: Option<f64>,
    tw: Option<f64>,
    lg: Option<f64>,
    eg: Option<f64>,
    tg: Option<f64>,
    log: Option<f64>,
    eog: Option<f64>,
    tog: Option<f64>,
) -> RelativePermeabilityResult {
    relative_permeability(
        table_type,
        model_family,
        rows,
        swc,
        critical_water_saturation,
        sorw,
        sorg,
        sgcr,
        kro_max,
        krw_max,
        krg_max,
        no,
        nw,
        ng,
        nog,
        lo,
        eo,
        to,
        lw,
        ew,
        tw,
        lg,
        eg,
        tg,
        log,
        eog,
        tog,
    )
    .unwrap()
}

#[test]
fn the_default_table_is_swof_at_twenty_rows_with_the_endpoint_overwrites() {
    let result = call(
        None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
        None, None, None, None, None, None, None, None, None, None, None, None,
    );
    assert_eq!(result.table_type, "SWOF");
    assert_eq!(result.saturation_name, "Sw");
    assert_eq!(result.table.len(), 20);
    assert_eq!(result.table[0].sw, Some(0.0));
    assert_eq!(result.table[0].krw, Some(0.0));
    assert_eq!(result.table[0].krow, Some(1.0));
    assert_eq!(result.table[19].sw, Some(1.0));
    assert_eq!(result.table[19].krw, Some(1.0));
    assert_eq!(result.table[19].krow, Some(0.0));
}

#[test]
fn the_sgof_table_is_the_capture() {
    let result = call(
        Some("sgof"),
        None,
        Some(5),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert_eq!(result.table_type, "SGOF");
    assert_eq!(result.saturation_name, "Sg");
    assert_eq!(result.table[0].sg, Some(0.0));
    assert_eq!(result.table[0].krg, Some(0.0));
    assert_eq!(result.table[0].krog, Some(1.0));
    assert_eq!(result.table[4].sg, Some(1.0));
    assert_eq!(result.table[4].krg, Some(1.0));
    assert_eq!(result.table[4].krog, Some(0.0));
    assert_eq!(result.table[1].krg, Some(0.0625));
}

#[test]
fn the_let_table_is_the_capture() {
    let result = call(
        Some("swof"),
        Some("let"),
        Some(5),
        Some(0.15),
        None,
        Some(0.2),
        None,
        None,
        None,
        Some(0.3),
        None,
        None,
        None,
        None,
        None,
        Some(2.5),
        Some(1.25),
        Some(1.75),
        Some(1.2),
        Some(1.5),
        Some(2.0),
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert_eq!(result.table[0].sw, Some(0.15));
    assert_eq!(result.table[1].krw, Some(0.05501216576859834));
    assert_eq!(result.table[1].krow, Some(0.8151257300970868));
    assert_eq!(result.table[4].krw, Some(0.3));
    assert_eq!(result.table[4].krow, Some(0.0));
}

#[test]
fn a_descending_grid_is_published() {
    let result = call(
        None,
        None,
        Some(5),
        Some(0.9),
        None,
        Some(0.2),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    assert!(result.table[0].sw.unwrap() > result.table[4].sw.unwrap());
    assert_eq!(result.table[0].krw, Some(0.0));
    assert_eq!(result.table[4].krow, Some(0.0));
}

#[test]
fn a_refused_state_is_named() {
    let short = relative_permeability(
        None,
        None,
        Some(2),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(short.to_string().contains("three rows"));

    let unknown = relative_permeability(
        Some("not_a_table"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(unknown.to_string().contains("not_a_table"));
}
