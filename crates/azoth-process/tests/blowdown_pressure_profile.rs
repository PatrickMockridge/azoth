//! `process.blowdown_pressure_profile` against
//! `validation/neqsim/captures/process_blowdown_pressure_profile.tsv`.

use azoth_core::units::{bars, seconds};
use azoth_process::models::blowdown_pressure_profile::{
    BlowdownProfilePoint, blowdown_pressure_profile,
};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn point(time: f64, pressure: f64) -> BlowdownProfilePoint {
    BlowdownProfilePoint {
        time: seconds(time),
        pressure: bars(pressure),
    }
}

fn workbook() -> Vec<BlowdownProfilePoint> {
    // The class's `fromMinutesAndBara` grid: the nine-decimal minutes times 60.
    let minutes = [
        0.0,
        0.083_333_333,
        0.166_666_667,
        0.25,
        0.333_333_333,
        0.416_666_667,
        0.5,
        0.583_333_333,
        0.666_666_667,
        0.75,
        0.833_333_333,
        0.916_666_667,
        1.0,
        1.083_333_333,
        1.166_666_667,
        1.25,
        1.333_333_333,
        1.416_666_667,
        1.5,
        1.583_333_333,
        1.666_666_667,
        1.75,
        1.833_333_333,
        1.916_666_667,
        2.0,
    ];
    let bara = [
        61.3, 59.7053, 58.6349, 57.3483, 55.9731, 54.5583, 53.1249, 51.6842, 50.2472, 48.8145,
        47.3977, 46.004, 44.6383, 43.3046, 42.0058, 40.7442, 39.5206, 38.3367, 37.1941, 36.0922,
        35.0302, 34.0082, 33.0255, 32.0817, 31.1758,
    ];
    minutes
        .iter()
        .zip(bara.iter())
        .map(|(minute, pressure)| point(minute * 60.0, *pressure))
        .collect()
}

fn exact() -> Vec<BlowdownProfilePoint> {
    vec![point(0.0, 87.0), point(5.0, 80.0), point(10.0, 74.0)]
}

#[test]
fn the_workbook_exact_and_between_states() {
    let profile = workbook();
    let exact = blowdown_pressure_profile(
        &profile,
        seconds(110.0),
        Some("step_previous"),
        Some(bars(1.0)),
    )
    .expect("the capture state is valid");
    close(exact.pressure_absolute.value / 1.0e5, 33.0255, "110 s bara");
    close(exact.pressure_gauge.value / 1.0e5, 32.0255, "110 s barg");

    let between = blowdown_pressure_profile(
        &profile,
        seconds(107.0),
        Some("step_previous"),
        Some(bars(1.0)),
    )
    .expect("the capture state is valid");
    close(
        between.pressure_absolute.value / 1.0e5,
        34.0082,
        "107 s bara",
    );

    let linear =
        blowdown_pressure_profile(&profile, seconds(107.5), Some("linear"), Some(bars(1.0)))
            .expect("the capture state is valid");
    close(
        linear.pressure_absolute.value / 1.0e5,
        33.516_849_998_034_6,
        "107.5 s linear bara",
    );
}

#[test]
fn the_absolute_near_equal_window_is_both_sided() {
    let profile = exact();
    let upper = blowdown_pressure_profile(
        &profile,
        seconds(5.0 - 5.0e-11),
        Some("step_previous"),
        Some(bars(1.0)),
    )
    .expect("the capture state is valid");
    close(
        upper.pressure_absolute.value / 1.0e5,
        80.0,
        "inside the window",
    );

    let lower = blowdown_pressure_profile(
        &profile,
        seconds(5.0 - 2.0e-10),
        Some("step_previous"),
        Some(bars(1.0)),
    )
    .expect("the capture state is valid");
    close(
        lower.pressure_absolute.value / 1.0e5,
        87.0,
        "outside the window",
    );

    let interpolated = blowdown_pressure_profile(
        &profile,
        seconds(5.0 - 2.0e-10),
        Some("linear"),
        Some(bars(1.0)),
    )
    .expect("the capture state is valid");
    close(
        interpolated.pressure_absolute.value / 1.0e5,
        80.000_000_000_28,
        "linear outside the window",
    );
}

#[test]
fn the_endpoints_clamp_and_the_offset_is_subtracted() {
    let profile = workbook();
    let before = blowdown_pressure_profile(
        &profile,
        seconds(0.0),
        Some("step_previous"),
        Some(bars(1.0)),
    )
    .expect("the capture state is valid");
    close(
        before.pressure_absolute.value / 1.0e5,
        61.3,
        "before the first",
    );
    let after = blowdown_pressure_profile(
        &profile,
        seconds(130.0),
        Some("step_previous"),
        Some(bars(1.0)),
    )
    .expect("the capture state is valid");
    close(
        after.pressure_absolute.value / 1.0e5,
        31.1758,
        "after the last",
    );

    let two = blowdown_pressure_profile(
        &exact(),
        seconds(5.0),
        Some("step_previous"),
        Some(bars(2.0)),
    )
    .expect("the capture state is valid");
    close(two.pressure_gauge.value / 1.0e5, 78.0, "a 2 bar offset");
    let zero = blowdown_pressure_profile(
        &exact(),
        seconds(5.0),
        Some("step_previous"),
        Some(bars(0.0)),
    )
    .expect("the capture state is valid");
    close(zero.pressure_gauge.value / 1.0e5, 80.0, "a zero offset");
}

#[test]
fn the_class_refusals_are_the_ports() {
    let one = blowdown_pressure_profile(&[point(0.0, 87.0)], seconds(0.0), None, None);
    assert!(one.is_err(), "one point is refused");

    let repeated = blowdown_pressure_profile(
        &[point(0.0, 87.0), point(5.0, 80.0), point(5.0, 74.0)],
        seconds(0.0),
        None,
        None,
    );
    assert!(repeated.is_err(), "repeated times are refused");

    let negative = blowdown_pressure_profile(
        &[point(-1.0, 87.0), point(5.0, 80.0)],
        seconds(0.0),
        None,
        None,
    );
    assert!(negative.is_err(), "a negative point time is refused");

    let zero_pressure = blowdown_pressure_profile(
        &[point(0.0, 87.0), point(5.0, 0.0)],
        seconds(0.0),
        None,
        None,
    );
    assert!(zero_pressure.is_err(), "a zero pressure is refused");

    let negative_query = blowdown_pressure_profile(&exact(), seconds(-1.0), None, None);
    assert!(negative_query.is_err(), "a negative query is refused");

    let negative_offset = blowdown_pressure_profile(&exact(), seconds(0.0), None, Some(bars(-1.0)));
    assert!(negative_offset.is_err(), "a negative offset is refused");

    let mode = blowdown_pressure_profile(&exact(), seconds(0.0), Some("cubic"), None);
    assert!(mode.is_err(), "an unknown mode is refused");
}
