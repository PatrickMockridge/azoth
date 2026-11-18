//! `process.psv_valve` against `validation/neqsim/captures/process_psv_valve.tsv`.

#![allow(clippy::excessive_precision)]

use azoth_core::units::{kelvins, kilograms_per_mole, pascals, square_meters};
use azoth_process::models::psv_valve::{PsvValveResult, psv_valve};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-12,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

#[allow(clippy::too_many_arguments)]
fn valve(
    set: f64,
    blowdown: f64,
    coefficient: f64,
    area: f64,
    back: f64,
    upstream: f64,
    temperature: f64,
    molar_mass: f64,
    gamma: f64,
    z: f64,
    was_open: Option<bool>,
    prior: Option<f64>,
) -> PsvValveResult {
    psv_valve(
        pascals(set),
        blowdown,
        coefficient,
        square_meters(area),
        pascals(back),
        pascals(upstream),
        kelvins(temperature),
        kilograms_per_mole(molar_mass),
        gamma,
        z,
        was_open,
        prior,
    )
    .expect("the captured state is valid")
}

#[test]
fn the_latch_and_the_orifice_are_the_captured_states() {
    let closed = valve(
        5.0e6, 0.1, 0.975, 1.0e-3, 1.0e5, 4.8e6, 350.0, 0.018, 1.3, 0.95, None, None,
    );
    assert!(!closed.open);
    close(closed.cycle_count, 0.0, "the closed count");
    close(closed.mass_flow.value, 0.0, "the closed flow");

    let pops = valve(
        5.0e6, 0.1, 0.975, 1.0e-3, 1.0e5, 5.0e6, 350.0, 0.018, 1.3, 0.95, None, None,
    );
    assert!(pops.open);
    close(pops.cycle_count, 1.0, "the first pop");
    close(pops.mass_flow.value, 8.300545970532687, "the choked flow");

    let latched = valve(
        5.0e6,
        0.1,
        0.975,
        1.0e-3,
        1.0e5,
        4.6e6,
        350.0,
        0.018,
        1.3,
        0.95,
        Some(true),
        Some(1.0),
    );
    assert!(latched.open);
    close(latched.cycle_count, 1.0, "the latched count");
    close(
        latched.mass_flow.value,
        7.636502292890071,
        "the latched flow",
    );

    let reseats = valve(
        5.0e6,
        0.1,
        0.975,
        1.0e-3,
        1.0e5,
        4.4e6,
        350.0,
        0.018,
        1.3,
        0.95,
        Some(true),
        Some(1.0),
    );
    assert!(!reseats.open);
    close(reseats.cycle_count, 1.0, "the reseated count");
    close(reseats.mass_flow.value, 0.0, "the reseated flow");

    let second = valve(
        5.0e6,
        0.1,
        0.975,
        1.0e-3,
        1.0e5,
        5.1e6,
        350.0,
        0.018,
        1.3,
        0.95,
        None,
        Some(2.0),
    );
    assert!(second.open);
    close(second.cycle_count, 3.0, "the second pop");
    close(second.mass_flow.value, 8.466556889943341, "the second flow");

    let subcritical = valve(
        5.0e5,
        0.1,
        0.975,
        1.0e-3,
        4.0e5,
        5.2e5,
        350.0,
        0.018,
        1.3,
        0.95,
        Some(true),
        Some(1.0),
    );
    assert!(subcritical.open);
    close(subcritical.cycle_count, 1.0, "the subcritical count");
    close(
        subcritical.mass_flow.value,
        0.7544344197817343,
        "the subcritical flow",
    );
}

#[test]
fn the_refusals_are_the_class_guards() {
    assert!(
        psv_valve(
            pascals(5.0e6),
            0.1,
            0.975,
            square_meters(1.0e-3),
            pascals(-1.0),
            pascals(5.0e6),
            kelvins(350.0),
            kilograms_per_mole(0.018),
            1.3,
            0.95,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        psv_valve(
            pascals(5.0e6),
            0.1,
            0.975,
            square_meters(1.0e-3),
            pascals(6.0e6),
            pascals(5.0e6),
            kelvins(350.0),
            kilograms_per_mole(0.018),
            1.3,
            0.95,
            None,
            None,
        )
        .is_err()
    );
    for blowdown in [0.0, 1.0] {
        assert!(
            psv_valve(
                pascals(5.0e6),
                blowdown,
                0.975,
                square_meters(1.0e-3),
                pascals(1.0e5),
                pascals(5.0e6),
                kelvins(350.0),
                kilograms_per_mole(0.018),
                1.3,
                0.95,
                None,
                None,
            )
            .is_err()
        );
    }
    assert!(
        psv_valve(
            pascals(5.0e6),
            0.1,
            0.975,
            square_meters(1.0e-3),
            pascals(1.0e5),
            pascals(5.0e6),
            kelvins(350.0),
            kilograms_per_mole(0.018),
            1.0,
            0.95,
            None,
            None,
        )
        .is_err()
    );
    assert!(
        psv_valve(
            pascals(5.0e6),
            0.1,
            0.975,
            square_meters(1.0e-3),
            pascals(1.0e5),
            pascals(5.0e6),
            kelvins(350.0),
            kilograms_per_mole(0.018),
            1.3,
            0.95,
            None,
            Some(1.5),
        )
        .is_err()
    );
    assert!(
        psv_valve(
            pascals(5.0e6),
            0.1,
            0.975,
            square_meters(1.0e-3),
            pascals(1.0e5),
            pascals(f64::NAN),
            kelvins(350.0),
            kilograms_per_mole(0.018),
            1.3,
            0.95,
            None,
            None,
        )
        .is_err()
    );
}
