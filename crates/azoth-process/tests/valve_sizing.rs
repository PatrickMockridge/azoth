//! `process.throttling_valve`'s sizing-standard routes, held to
//! `validation/neqsim/captures/process_valve_standards.tsv`.
//!
//! The gas `Kv` carries the measured `3e-5` heat-capacity divergence the default route's own
//! tests already bound; the liquid branch reads only the density and agrees to rounding.

use azoth_core::units::{kelvins, meters, pascals};
use azoth_process::models::throttling_valve::{ThrottlingValveResult, throttling_valve};

const RICH: [&str; 4] = ["methane", "ethane", "propane", "n-butane"];
const RICH_Z: [f64; 4] = [0.1, 0.2, 0.4, 0.3];
const RICH_N: f64 = 97.219_619_722_781_74;
const RICH_P: f64 = 182_925.0;
const RICH_T: f64 = 296.25;
const RICH_OUT: f64 = 141_325.0;

const WATER_N: f64 = 26_100.471_829_031_365;
const WATER_P: f64 = 1_480_000.0;
const WATER_T: f64 = 289.15;
const WATER_OUT: f64 = 446_000.0;

#[allow(clippy::too_many_arguments)] // the model's own boundary
fn valve(
    components: &[&str],
    n: f64,
    z: &[f64],
    p: f64,
    t: f64,
    outlet: f64,
    allow_choked: Option<bool>,
    standard: Option<&str>,
) -> ThrottlingValveResult {
    throttling_valve(
        &components
            .iter()
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>(),
        n,
        z,
        pascals(p),
        kelvins(t),
        pascals(outlet),
        None,
        None,
        None,
        None,
        None,
        allow_choked,
        None,
        None,
        standard,
        None,
        None,
    )
    .expect("the captured state sizes")
}

fn close(actual: f64, expected: f64, tolerance: f64, context: &str) {
    let error = (actual - expected).abs() / expected.abs().max(1.0e-30);
    assert!(
        error < tolerance,
        "{context}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn rich_gas(standard: &str, allow_choked: Option<bool>) -> ThrottlingValveResult {
    valve(
        &RICH,
        RICH_N,
        &RICH_Z,
        RICH_P,
        RICH_T,
        RICH_OUT,
        allow_choked,
        Some(standard),
    )
}

fn water(standard: &str) -> ThrottlingValveResult {
    valve(
        &["water"],
        WATER_N,
        &[1.0],
        WATER_P,
        WATER_T,
        WATER_OUT,
        None,
        Some(standard),
    )
}

/// The IEC gas row: capped by its constructor's `allowChoked = true`.
#[test]
fn the_iec_gas_row_is_the_capped_sizing() {
    let result = rich_gas("iec_60534", None);
    // `standard_gas_iec_60534`'s `kv=873.109295270072`.
    close(result.kv, 873.109_295_270_072, 5.0e-5, "IEC gas Kv");
    close(result.cv, 1_009.314_345_332_203, 5.0e-5, "IEC gas Cv");
    // `calculated_molar_flow=97.21961972278176`.
    close(
        result.calculated_molar_flow,
        97.219_619_722_781_76,
        1.0e-6,
        "IEC gas flow",
    );
    assert!(result.choked);
    assert!(result.gas_valve);
    close(
        result.outlet_t.value,
        295.635_024_827_099_9,
        2.0e-4,
        "IEC gas outlet T",
    );
}

/// A stated `allowChoked = false` is the uncapped form: the factory's `true` is overridden.
#[test]
fn the_iec_gas_row_honours_a_stated_allow_choked() {
    let result = rich_gas("iec_60534", Some(false));
    // `standard_gas_iec_60534_allow_false`'s `kv=607.9540338130714`.
    close(
        result.kv,
        607.954_033_813_071_4,
        5.0e-5,
        "IEC gas uncapped Kv",
    );
    assert!(result.choked);
}

/// The IEC liquid row reproduces the class's `Z`-as-`Psat` call: `FF = -inf`, never choked.
#[test]
fn the_iec_liquid_row_is_the_simplified_liquid_branch() {
    let result = water("iec_60534");
    // `standard_water_iec_60534`'s `kv=528.7333498347227`.
    close(result.kv, 528.733_349_834_722_7, 1.0e-6, "IEC liquid Kv");
    // The inverse passes `Psat = P1` and does choke: `9425.264109024572` against the inlet's
    // `26100`.
    close(
        result.calculated_molar_flow,
        9_425.264_109_024_572,
        1.0e-6,
        "IEC liquid flow",
    );
    assert!(!result.choked);
    assert!(!result.gas_valve);
}

/// The full variant's `D = 0` early return is the simplified sizing, in both branches.
#[test]
fn the_full_variant_is_the_simplified_sizing_on_its_reachable_branch() {
    let gas = rich_gas("iec_60534_full", None);
    close(gas.kv, 873.109_295_270_072, 5.0e-5, "full gas Kv");
    close(
        gas.calculated_molar_flow,
        97.219_405_102_291_86,
        1.0e-5,
        "full gas flow",
    );

    let liquid = water("iec_60534_full");
    close(liquid.kv, 528.733_349_834_722_7, 1.0e-6, "full liquid Kv");
    close(
        liquid.calculated_molar_flow,
        9_425.308_667_981_49,
        1.0e-6,
        "full liquid flow",
    );
}

/// The production choke divides the gas `Kv` by `Cd` and leaves the liquid branch alone.
#[test]
fn the_production_choke_divides_the_gas_branch_by_cd() {
    let gas = rich_gas("prod_choke", None);
    // `standard_gas_prod_choke`'s `kv=1027.1874062000845`.
    close(gas.kv, 1_027.187_406_200_084_5, 5.0e-5, "prod choke gas Kv");
    close(
        gas.calculated_molar_flow,
        97.219_619_722_781_73,
        1.0e-6,
        "prod choke gas flow",
    );
    assert!(gas.choked);

    let liquid = water("prod_choke");
    // `standard_water_prod_choke`'s `kv=528.4961597255068`, the default's own.
    close(
        liquid.kv,
        528.496_159_725_506_8,
        1.0e-6,
        "prod choke liquid Kv",
    );
    close(
        liquid.calculated_molar_flow,
        26_100.471_829_031_37,
        1.0e-6,
        "prod choke liquid flow",
    );
    assert!(!liquid.choked);
}

/// `allow_choked` absent is each constructor's state: `false` on the default, `true` on the IEC.
#[test]
fn the_absent_allow_choked_follows_the_selected_standard() {
    let default = rich_gas("default", None);
    let iec = rich_gas("iec_60534", None);
    assert!(default.kv < iec.kv, "the default starts uncapped");
    // The default's construction, not a stated flag.
    let default_stated = rich_gas("default", Some(true));
    close(default_stated.kv, iec.kv, 5.0e-5, "the default capped");
}

/// A two-phase inlet is refused by the single-phase standards, naming the class.
#[test]
fn the_single_phase_standards_refuse_a_two_phase_inlet() {
    for standard in ["default", "iec_60534", "iec_60534_full", "prod_choke"] {
        let error = throttling_valve(
            &["methane".to_string(), "n-butane".to_string()],
            1.0,
            &[0.7, 0.3],
            pascals(2_000_000.0),
            kelvins(300.0),
            pascals(1_500_000.0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(standard),
            None,
            None,
        )
        .expect_err("two phases cannot be read as one root");
        let message = format!("{error}");
        assert!(message.contains("single phase"), "{standard}: {message}");
    }
}

/// The separator capture's two-phase feed: methane/n-butane 0.7/0.3 at 20 bara and 300 K.
#[allow(clippy::too_many_arguments)]
fn well(
    standard: &str,
    outlet_bara: f64,
    diameter: Option<f64>,
    discharge: Option<f64>,
    calc_pressure: bool,
) -> ThrottlingValveResult {
    throttling_valve(
        &["methane".to_string(), "n-butane".to_string()],
        1.0,
        &[0.7, 0.3],
        pascals(2_000_000.0),
        kelvins(300.0),
        pascals(outlet_bara * 1.0e5),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(calc_pressure),
        None,
        Some(standard),
        diameter.map(meters),
        discharge,
    )
    .expect("the captured two-phase state sizes")
}

/// Sachdeva's two regimes, on the capture's subcritical and critical rows.
#[test]
fn the_sachdeva_rows_match_the_capture() {
    let subcritical = well("sachdeva", 15.0, Some(0.0127), None, false);
    // `standard_subcritical_sachdeva`.
    close(
        subcritical.kv,
        76.176_297_525_844_67,
        1.0e-4,
        "Sachdeva subcritical Kv",
    );
    close(
        subcritical.calculated_molar_flow,
        286.262_795_105_894_84,
        1.0e-4,
        "Sachdeva subcritical flow",
    );
    assert!(!subcritical.choked);
    assert!(subcritical.gas_valve);

    let critical = well("sachdeva", 5.0, Some(0.0127), None, false);
    // `standard_critical_sachdeva`.
    close(
        critical.kv,
        31.871_585_378_095_933,
        1.0e-4,
        "Sachdeva critical Kv",
    );
    close(
        critical.calculated_molar_flow,
        207.448_042_639_319_85,
        1.0e-4,
        "Sachdeva critical flow",
    );
    assert!(critical.choked);
}

/// A stated `Cd` scales the choke's mass flow.
#[test]
fn a_stated_discharge_coefficient_scales_the_sachdeva_flow() {
    let result = well("sachdeva", 5.0, Some(0.0127), Some(0.9), false);
    // `standard_sachdeva_cd`.
    close(result.kv, 34.148_127_190_817_07, 1.0e-4, "Sachdeva Cd Kv");
    close(
        result.calculated_molar_flow,
        222.265_759_970_699_86,
        1.0e-4,
        "Sachdeva Cd flow",
    );
}

/// The five choke standards all accept a two-phase inlet and answer a mass flow.
#[test]
fn the_choke_standards_accept_a_two_phase_inlet() {
    for standard in ["sachdeva", "gilbert", "baxendell", "ros", "achong"] {
        let result = well(standard, 5.0, Some(0.0127), None, false);
        assert!(result.kv > 0.0, "{standard}: no sizing");
        assert!(
            result.calculated_molar_flow.is_finite(),
            "{standard}: no flow"
        );
    }
}

/// The four Gilbert-type correlations are one formula with four constant sets.
#[test]
fn the_gilbert_family_matches_the_capture() {
    let rows = [
        ("gilbert", 24.575_359_998_706_734, 159.957_851_748_161_83),
        ("baxendell", 29.528_949_376_542_23, 192.200_126_748_925_87),
        ("ros", 23.342_792_324_132_382, 151.935_227_568_108_44),
        ("achong", 47.247_914_623_691_294, 307.530_588_491_234_2),
    ];
    for (standard, kv, flow) in rows {
        let result = well(standard, 5.0, Some(0.0127), None, false);
        close(result.kv, kv, 1.0e-4, &format!("{standard} Kv"));
        close(
            result.calculated_molar_flow,
            flow,
            1.0e-4,
            &format!("{standard} flow"),
        );
        assert!(result.choked, "{standard}: the row is critical");
    }
}

/// `findOutletPressureForFixedKv` answers `P1 * y_c`, not an inverse of the sizing.
#[test]
fn the_sachdeva_pressure_mode_is_the_critical_pressure() {
    let result = well("sachdeva", 5.0, Some(0.0127), None, true);
    // `standard_sachdeva_pressure_mode`'s `solved_outlet_p_bara=11.884456598849502`.
    close(
        result.outlet_p.value,
        1_188_445.659_884_950_2,
        1.0e-5,
        "Sachdeva solved pressure",
    );
    close(
        result.outlet_t.value,
        289.773_588_374_696_15,
        2.0e-4,
        "Sachdeva solved outlet T",
    );
}
