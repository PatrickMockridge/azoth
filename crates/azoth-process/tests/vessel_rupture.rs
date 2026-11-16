//! `process.vessel_rupture` against
//! `validation/neqsim/captures/process_vessel_rupture.tsv`.

use azoth_core::units::{kelvins, megapascals, meters, pascals, seconds};
use azoth_process::models::vessel_rupture::{
    VesselRuptureResult, VesselRuptureState, vessel_rupture,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn state(time_s: f64, pressure_pa: f64, temperature_k: f64) -> VesselRuptureState {
    VesselRuptureState {
        time: seconds(time_s),
        internal_pressure: pascals(pressure_pa),
        metal_temperature: kelvins(temperature_k),
    }
}

fn run(
    wall: f64,
    material: &str,
    history: &[VesselRuptureState],
    yield_strength: Option<f64>,
    tensile_strength: Option<f64>,
    grade: Option<&str>,
) -> azoth_core::Result<VesselRuptureResult> {
    vessel_rupture(
        meters(0.5),
        meters(wall),
        material,
        history,
        yield_strength.map(megapascals),
        tensile_strength.map(megapascals),
        grade,
        None,
    )
}

#[test]
fn the_severe_x65_state_ruptures_at_zero() {
    let history: Vec<_> = (0..11)
        .map(|index| {
            state(
                index as f64 * 30.0,
                (10.0 + index as f64 * 9.0) * 1.0e5,
                973.15,
            )
        })
        .collect();
    let result =
        run(0.01, "api_5l", &history, None, None, Some("X65")).expect("the capture state is valid");
    assert!(result.ruptured);
    close(
        result.rupture_time.expect("a rupture has a time").value,
        0.0,
        "the rupture time",
    );
    close(
        result.rupture_stress.expect("a rupture has a stress").value,
        4.330_127_018_922_193e7,
        "the rupture stress",
    );
    close(
        result
            .rupture_allowable_stress
            .expect("a rupture has an allowable")
            .value,
        3.716_275_640_000_000_6e7,
        "the rupture allowable",
    );
    close(
        result
            .rupture_metal_temperature
            .expect("a rupture has a temperature")
            .value,
        973.15,
        "the rupture metal temperature",
    );
    close(
        result.min_margin.value,
        -3.958_499_454_922_193e8,
        "the minimum margin",
    );
    close(
        result.min_margin_time.value,
        300.0,
        "the minimum-margin time",
    );
    assert_eq!(result.material_name, "API 5L X65");
    close(
        result.ambient_yield_strength.value / 1.0e6,
        448.1594,
        "the yield strength",
    );
    close(
        result.ambient_tensile_strength.value / 1.0e6,
        530.89652,
        "the tensile strength",
    );
    assert_eq!(result.history.len(), 11);
}

#[test]
fn the_mild_x65_state_survives() {
    let history: Vec<_> = (0..6)
        .map(|index| state(index as f64 * 30.0, 20.0e5, 300.0))
        .collect();
    let result =
        run(0.02, "api_5l", &history, None, None, Some("X65")).expect("the capture state is valid");
    assert!(!result.ruptured);
    assert!(result.rupture_time.is_none());
    close(
        result.min_margin.value,
        4.853_223_490_845_281e8,
        "the minimum margin",
    );
    close(result.min_margin_time.value, 0.0, "the minimum-margin time");
    assert_eq!(result.history.len(), 6);
}

#[test]
fn the_bare_lpg_vessel_ruptures_and_the_insulated_one_does_not() {
    let bare: Vec<_> = (0..21)
        .map(|index| {
            let fraction = index as f64 / 20.0;
            state(
                index as f64 * 30.0,
                (18.0 + 8.0 * fraction) * 1.0e5,
                300.0 + 600.0 * fraction,
            )
        })
        .collect();
    let bare_result = run(0.012, "carbon_steel", &bare, Some(245.0), Some(415.0), None)
        .expect("the capture state is valid");
    assert!(bare_result.ruptured);
    close(
        bare_result
            .rupture_time
            .expect("a rupture has a time")
            .value,
        536.459_734_497_078_6,
        "the bare rupture time",
    );
    close(
        bare_result
            .rupture_stress
            .expect("a rupture has a stress")
            .value,
        9.093_266_739_736_606e7,
        "the bare rupture stress",
    );
    close(
        bare_result
            .rupture_allowable_stress
            .expect("a rupture has an allowable")
            .value,
        8.841_159_999_999_999e7,
        "the bare rupture allowable",
    );
    close(
        bare_result
            .rupture_metal_temperature
            .expect("a rupture has a temperature")
            .value,
        840.0,
        "the bare rupture metal temperature",
    );

    let insulated: Vec<_> = (0..21)
        .map(|index| {
            let fraction = index as f64 / 20.0;
            state(
                index as f64 * 30.0,
                (18.0 + 5.0 * fraction) * 1.0e5,
                300.0 + 110.0 * fraction,
            )
        })
        .collect();
    let insulated_result = run(
        0.012,
        "carbon_steel",
        &insulated,
        Some(245.0),
        Some(415.0),
        None,
    )
    .expect("the capture state is valid");
    assert!(!insulated_result.ruptured);
    close(
        insulated_result.min_margin.value,
        2.959_631_488_039_912_6e8,
        "the insulated minimum margin",
    );
}

#[test]
fn the_x90_literal_strengths_and_the_l450_alias() {
    let one = [state(0.0, 20.0e5, 973.15)];
    let x90 =
        run(0.01, "api_5l", &one, None, None, Some("X90")).expect("the capture state is valid");
    assert_eq!(x90.material_name, "API 5L X90");
    close(x90.ambient_yield_strength.value / 1.0e6, 620.0, "X90 yield");
    close(
        x90.ambient_tensile_strength.value / 1.0e6,
        695.0,
        "X90 tensile",
    );
    assert!(x90.ruptured);
    close(
        x90.rupture_allowable_stress.expect("a rupture").value,
        4.865_000_000_000_001e7,
        "the X90 allowable",
    );

    let l450 =
        run(0.01, "api_5l", &one, None, None, Some("L450")).expect("the capture state is valid");
    assert_eq!(l450.material_name, "API 5L X65");
    close(
        l450.ambient_yield_strength.value / 1.0e6,
        448.1594,
        "the alias yield",
    );
    close(
        l450.ambient_tensile_strength.value / 1.0e6,
        530.89652,
        "the alias tensile",
    );
}

#[test]
fn the_class_refusals_are_the_ports() {
    let one = [state(0.0, 20.0e5, 973.15)];
    assert!(
        run(
            0.01,
            "carbon_steel",
            &one,
            Some(245.0),
            Some(415.0),
            Some("X65")
        )
        .is_err(),
        "a carbon-steel grade is refused"
    );
    assert!(
        run(0.01, "api_5l", &one, Some(245.0), Some(415.0), Some("X65")).is_err(),
        "API strengths are refused"
    );
    assert!(
        run(0.01, "api_5l", &one, None, None, None).is_err(),
        "a missing grade is refused"
    );
    assert!(
        run(0.01, "api_5l", &one, None, None, Some("X999")).is_err(),
        "an unknown grade is refused"
    );
    assert!(
        run(0.01, "api_5l", &one, None, None, Some("")).is_err(),
        "an empty grade is refused"
    );
    assert!(
        run(0.01, "carbon_steel", &[], Some(245.0), Some(415.0), None).is_err(),
        "an empty history is refused"
    );

    let non_monotonic = [
        state(0.0, 20.0e5, 300.0),
        state(10.0, 20.0e5, 300.0),
        state(5.0, 20.0e5, 300.0),
    ];
    assert!(
        run(
            0.01,
            "carbon_steel",
            &non_monotonic,
            Some(245.0),
            Some(415.0),
            None
        )
        .is_err(),
        "a non-monotonic history is refused"
    );

    let negative = [state(0.0, -1.0, 300.0)];
    assert!(
        run(
            0.01,
            "carbon_steel",
            &negative,
            Some(245.0),
            Some(415.0),
            None
        )
        .is_err(),
        "a negative pressure is refused"
    );

    let factor_zero = vessel_rupture(
        meters(0.5),
        meters(0.01),
        "carbon_steel",
        &one,
        Some(megapascals(245.0)),
        Some(megapascals(415.0)),
        None,
        Some(0.0),
    );
    assert!(factor_zero.is_err(), "a zero factor is refused");
    let factor_high = vessel_rupture(
        meters(0.5),
        meters(0.01),
        "carbon_steel",
        &one,
        Some(megapascals(245.0)),
        Some(megapascals(415.0)),
        None,
        Some(1.5),
    );
    assert!(factor_high.is_err(), "a factor above one is refused");
}
