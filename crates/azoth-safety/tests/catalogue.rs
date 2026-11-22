//! The safety catalogue and the family registry a front end draws.
//!
//! **The registry is the one place a family can disappear.** `tools/gen_safety.py` refuses a
//! family the specs name and the registry omits; these tests hold the generated answer — every
//! calculation has a family, every family has its copy, and a deck is exactly the families whose
//! entries name that deck as their sub-interface.

use std::collections::BTreeSet;

use azoth_safety::catalogue::{calculations, families, families_to_value};

#[test]
fn every_calculation_names_a_served_family() {
    let declared: BTreeSet<&str> = families().iter().map(|family| family.id).collect();
    assert_eq!(calculations().len(), 96);
    assert_eq!(declared.len(), 23);
    for entry in calculations() {
        assert!(
            declared.contains(entry.family),
            "{} is in {}, which the registry does not serve",
            entry.id,
            entry.family
        );
    }
}

#[test]
fn every_family_has_at_least_one_calculation() {
    for family in families() {
        assert!(
            calculations().iter().any(|entry| entry.family == family.id),
            "{} has no calculation",
            family.id
        );
    }
}

#[test]
fn the_registry_is_ten_decks_and_thirteen_listed_families() {
    let decks: Vec<&str> = families()
        .iter()
        .filter(|family| family.deck.is_some())
        .map(|family| family.id)
        .collect();
    assert_eq!(
        decks,
        [
            "depressurization",
            "dispersion",
            "equipmentsafety",
            "fire",
            "firewater",
            "overpressure",
            "processsafetysystem",
            "risk",
            "safetyassessment",
            "vibration",
        ]
    );
    assert_eq!(families().len() - decks.len(), 13);
}

#[test]
fn a_deck_family_sub_interfaces_its_own_entries() {
    for entry in calculations() {
        let family = families()
            .iter()
            .find(|family| family.id == entry.family)
            .unwrap_or_else(|| panic!("{} has no family row", entry.id));
        if entry.id == "process.bow_tie" {
            // The bow-tie is the one native id that is a sub-interface rather than a scenario.
            assert_eq!(entry.sub_interface, Some("bowtie"));
            assert!(family.deck.is_some());
        } else if family.deck.is_some() {
            assert_eq!(
                entry.sub_interface,
                Some(family.id),
                "{} left the {} deck",
                entry.id,
                family.id
            );
        } else {
            assert_eq!(
                entry.sub_interface, None,
                "{} names a deck its family does not have",
                entry.id
            );
        }
    }
}

#[test]
fn the_family_copy_is_showable() {
    let mut seen = BTreeSet::new();
    for family in families() {
        assert!(seen.insert(family.id), "{} appears twice", family.id);
        assert!(!family.label.trim().is_empty(), "{}", family.id);
        assert!(!family.blurb.trim().is_empty(), "{}", family.id);
        let copy = match &family.deck {
            None => family.blurb.to_string(),
            Some(deck) => {
                for field in [deck.tab, deck.title, deck.heading, deck.search, deck.step] {
                    assert!(!field.trim().is_empty(), "{}", family.id);
                }
                format!(
                    "{} {} {} {} {} {}",
                    family.label, family.blurb, deck.tab, deck.title, deck.heading, deck.step
                )
            }
        };
        assert!(
            !copy.to_lowercase().contains("neqsim"),
            "{} names the upstream",
            family.id
        );
    }
}

#[test]
fn the_served_family_list_is_the_registry() {
    let value = families_to_value().expect("the registry writes");
    let rows = value.as_array().expect("the registry is an array");
    assert_eq!(rows.len(), families().len());
    let fire = rows
        .iter()
        .find(|row| row["id"] == "fire")
        .expect("fire is served");
    assert!(fire["deck"].is_object());
    let esd = rows
        .iter()
        .find(|row| row["id"] == "esd")
        .expect("esd is served");
    assert!(esd["deck"].is_null());
}
