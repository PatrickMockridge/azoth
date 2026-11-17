//! `process.mah_bow_tie_skeleton` against
//! `validation/neqsim/captures/process_mah_bow_tie_skeleton.tsv`.

use azoth_process::models::mah_bow_tie_skeleton::{MahBowTieSkeletonResult, mah_bow_tie_skeleton};

fn skeleton(mah_type: &str) -> MahBowTieSkeletonResult {
    mah_bow_tie_skeleton(mah_type).expect("the captured state is valid")
}

#[test]
fn the_fire_explosion_category_reproduces_the_capture() {
    let result = skeleton("fire_explosion");
    assert_eq!(result.threat_count, 4.0);
    assert_eq!(result.consequence_count, 3.0);
    assert_eq!(result.barrier_count, 6.0);

    assert_eq!(
        result
            .threats
            .iter()
            .map(|row| (row.id.as_str(), row.description.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("T1", "Hydrocarbon ignition"),
            ("T2", "Hot work"),
            ("T3", "Electrical fault"),
            ("T4", "Lightning strike"),
        ]
    );
    assert_eq!(
        result
            .consequences
            .iter()
            .map(|row| (row.id.as_str(), row.description.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("C1", "Personnel injury"),
            ("C2", "Equipment damage"),
            ("C3", "Escalation"),
        ]
    );
    assert_eq!(
        result
            .barriers
            .iter()
            .map(|row| (row.id.as_str(), row.description.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("B1", "Hazardous area classification"),
            ("B2", "Ignition control"),
            ("B3", "Gas detection"),
            ("B4", "Fire detection"),
            ("B5", "PFP"),
            ("B6", "Blast walls"),
        ]
    );
    for threat in &result.threats {
        assert_eq!(threat.frequency_per_year, 0.001);
    }
    for consequence in &result.consequences {
        assert_eq!(consequence.severity, 4.0);
    }
    for barrier in &result.barriers {
        assert_eq!(barrier.pfd, 0.01);
    }
}

#[test]
fn every_category_carries_its_own_counts() {
    let expected = [
        ("topside_hydrocarbon_release", 5, 4, 8),
        ("riser_leak", 5, 3, 5),
        ("well_blowout", 3, 3, 7),
        ("structural_collapse", 4, 2, 5),
        ("dropped_object", 3, 3, 4),
        ("helicopter_loss", 4, 2, 4),
        ("ship_collision", 3, 3, 4),
        ("fire_explosion", 4, 3, 6),
        ("toxic_release", 3, 2, 5),
        ("loss_of_buoyancy", 4, 2, 4),
        ("extreme_weather", 3, 3, 3),
    ];
    for (mah_type, threats, consequences, barriers) in expected {
        let result = skeleton(mah_type);
        assert_eq!(result.threat_count, threats as f64, "{mah_type} threats");
        assert_eq!(
            result.consequence_count, consequences as f64,
            "{mah_type} consequences"
        );
        assert_eq!(result.barrier_count, barriers as f64, "{mah_type} barriers");
        assert_eq!(result.threats.len(), threats);
        assert_eq!(result.consequences.len(), consequences);
        assert_eq!(result.barriers.len(), barriers);
        assert_eq!(result.threats[0].id, "T1");
        assert_eq!(result.consequences[0].id, "C1");
        assert_eq!(result.barriers[0].id, "B1");
    }
}

#[test]
fn the_topside_catalogue_reproduces_the_capture() {
    let result = skeleton("topside_hydrocarbon_release");
    assert_eq!(
        result
            .barriers
            .iter()
            .map(|row| row.description.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Process containment",
            "ESD / SDV isolation",
            "Blowdown",
            "Gas detection",
            "Fire detection",
            "Deluge",
            "PFP",
            "Emergency response",
        ]
    );
    assert_eq!(result.consequences[0].description, "Jet fire");
    assert_eq!(result.threats[0].description, "Corrosion failure");
}

#[test]
fn an_unknown_type_is_refused() {
    assert!(mah_bow_tie_skeleton("tsunami").is_err());
}
