//! `process.fault_tree` against `validation/neqsim/captures/process_fault_tree.tsv`.

use azoth_process::models::fault_tree::{FaultTreeNodeSpec, fault_tree};

fn close(actual: f64, expected: f64, what: &str) {
    let error = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        error <= 1.0e-9,
        "{what}: {actual} against {expected}, relative error {error:.3e}"
    );
}

fn node(
    name: &str,
    parent_index: f64,
    gate: &str,
    probability: Option<f64>,
    k_of_n: Option<f64>,
    beta_ccf: Option<f64>,
) -> FaultTreeNodeSpec {
    FaultTreeNodeSpec {
        name: name.to_string(),
        parent_index,
        gate: gate.to_string(),
        probability,
        k_of_n,
        beta_ccf,
    }
}

fn basic(name: &str, parent_index: f64, probability: f64) -> FaultTreeNodeSpec {
    node(name, parent_index, "basic", Some(probability), None, None)
}

fn gate(name: &str, parent_index: f64, kind: &str) -> FaultTreeNodeSpec {
    node(name, parent_index, kind, None, None, None)
}

fn voting(name: &str, parent_index: f64, k: f64) -> FaultTreeNodeSpec {
    node(name, parent_index, "voting", None, Some(k), None)
}

#[test]
fn the_and_gate_is_the_product_of_its_basics() {
    let result = fault_tree(
        &[gate("Both fail", -1.0, "and"), basic("A", 0.0, 0.1), basic("B", 0.0, 0.2)],
        3.0,
    )
    .expect("the captured state is valid");
    close(result.top_event_probability, 0.020_000_000_000_000_004, "AND");
    close(result.cut_set_count, 1.0, "one cut set");
    assert_eq!(result.minimal_cut_sets[0].members, "A, B");
    close(result.minimal_cut_sets[0].cardinality, 2.0, "two members");
}

#[test]
fn the_or_gate_is_the_exact_complement_union() {
    let result = fault_tree(
        &[
            gate("Either fail", -1.0, "or"),
            basic("A", 0.0, 0.1),
            basic("B", 0.0, 0.2),
        ],
        3.0,
    )
    .expect("the captured state is valid");
    close(result.top_event_probability, 0.279_999_999_999_999_9, "OR");
    close(result.cut_set_count, 2.0, "two single-event cut sets");
    assert_eq!(result.minimal_cut_sets[0].members, "A");
    assert_eq!(result.minimal_cut_sets[1].members, "B");
}

#[test]
fn the_voting_gate_is_the_poisson_binomial_sum() {
    let result = fault_tree(
        &[
            voting("2oo3", -1.0, 2.0),
            basic("A", 0.0, 0.1),
            basic("B", 0.0, 0.1),
            basic("C", 0.0, 0.1),
        ],
        3.0,
    )
    .expect("the captured state is valid");
    close(result.top_event_probability, 0.028_000_000_000_000_004, "2oo3");
    close(result.cut_set_count, 3.0, "three pairs");
    assert_eq!(result.minimal_cut_sets[0].members, "A, B");
    assert_eq!(result.minimal_cut_sets[1].members, "A, C");
    assert_eq!(result.minimal_cut_sets[2].members, "B, C");
}

#[test]
fn the_beta_factor_combines_the_independent_and_common_cause_terms() {
    let result = fault_tree(
        &[
            node("any", -1.0, "or", None, None, Some(0.10)),
            basic("A", 0.0, 0.01),
            basic("B", 0.0, 0.01),
        ],
        3.0,
    )
    .expect("the captured state is valid");
    close(
        result.top_event_probability,
        0.018_910_000_000_000_027,
        "the beta combination",
    );
    close(result.cut_set_count, 2.0, "the cut sets ignore beta");
}

#[test]
fn the_cut_sets_are_sorted_by_cardinality_and_then_name() {
    let result = fault_tree(
        &[
            gate("top", -1.0, "or"),
            gate("ab", 0.0, "and"),
            basic("A", 1.0, 0.1),
            basic("B", 1.0, 0.1),
            basic("C", 0.0, 0.1),
        ],
        3.0,
    )
    .expect("the captured state is valid");
    close(result.top_event_probability, 0.108_999_999_999_999_99, "the top");
    close(result.cut_set_count, 2.0, "two cut sets");
    assert_eq!(result.minimal_cut_sets[0].members, "C");
    assert_eq!(result.minimal_cut_sets[1].members, "A, B");
}

#[test]
fn the_class_states_the_port_refuses_are_refused() {
    // Beta on a VOTING gate is documented and ignored by the class; the port refuses it.
    let beta_on_voting = [
        node("2oo3", -1.0, "voting", None, Some(2.0), Some(0.10)),
        basic("A", 0.0, 0.1),
        basic("B", 0.0, 0.1),
    ];
    assert!(fault_tree(&beta_on_voting, 3.0).is_err(), "beta on VOTING");

    // k is unvalidated by the class: 0 answers 1.0 and above n answers 0.0.
    let k_zero = [voting("k0", -1.0, 0.0), basic("A", 0.0, 0.1)];
    assert!(fault_tree(&k_zero, 2.0).is_err(), "k below one");
    let k_above = [voting("k2", -1.0, 2.0), basic("A", 0.0, 0.1)];
    assert!(fault_tree(&k_above, 2.0).is_err(), "k above n");

    // Duplicate basic names: independent in the probability, aliased in the cut sets.
    let duplicate = [
        gate("or", -1.0, "or"),
        basic("A", 0.0, 0.1),
        basic("A", 0.0, 0.2),
    ];
    assert!(fault_tree(&duplicate, 2.0).is_err(), "duplicate names");

    // A NaN probability passes the class's guard.
    let nan = [basic("A", -1.0, f64::NAN)];
    assert!(fault_tree(&nan, 1.0).is_err(), "a NaN probability");

    // The class's mask enumeration overflows at 31 basics.
    let mut many = vec![gate("many", -1.0, "or")];
    for index in 0..21 {
        many.push(basic(&format!("B{index}"), 0.0, 0.5));
    }
    assert!(fault_tree(&many, 5.0).is_err(), "21 basics exceed the cap");

    // A childless gate is answered by the class and refused by the port.
    let childless = [gate("empty", -1.0, "and")];
    assert!(fault_tree(&childless, 1.0).is_err(), "a childless gate");

    // max_cardinality zero enumerates nothing in the class.
    assert!(
        fault_tree(&[basic("A", -1.0, 0.1)], 0.0).is_err(),
        "a zero cardinality"
    );
}

#[test]
fn a_malformed_structure_is_refused() {
    // Two roots.
    let two_roots = [basic("A", -1.0, 0.1), basic("B", -1.0, 0.1)];
    assert!(fault_tree(&two_roots, 1.0).is_err(), "two roots");

    // A cycle among non-root nodes leaves both unreachable.
    let cycle = [
        gate("root", -1.0, "or"),
        gate("a", 2.0, "or"),
        gate("b", 1.0, "or"),
        basic("leaf", 0.0, 0.1),
    ];
    assert!(fault_tree(&cycle, 1.0).is_err(), "an unreachable cycle");

    // A basic event with a child.
    let child_of_basic = [basic("A", -1.0, 0.1), basic("B", 0.0, 0.1)];
    assert!(fault_tree(&child_of_basic, 1.0).is_err(), "a child of a basic");

    // An empty node list has no root.
    assert!(fault_tree(&[], 1.0).is_err(), "an empty node list");
}
