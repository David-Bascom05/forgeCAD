//! M.ct's oracle in CI: every torus × cylinder event search on the gate parts, mixed and fuzz
//! 1–300 is certified by forge-geom's meet_cert (interval cover, Krawczyk); a search it does not
//! certify (an event missed, or undecided) fails here. Run with the decisions feature:
//! `cargo test --release -p forge-model --features decisions --test poly4_oracle`.
#![cfg(feature = "decisions")]

use forge_model::suite;

#[test]
fn every_torus_cylinder_event_search_is_certified() {
    forge_geom::meet::oracle::start();
    let mut parts: Vec<(String, forge_model::Part)> = [
        "torture",
        "plate",
        "shaft",
        "housing",
        "coplanar",
        "patterns",
        "mixed",
        "hole-grid",
        "tour",
        "upto",
        "upto-flat",
    ]
    .iter()
    .filter_map(|n| suite::part(n).map(|p| (n.to_string(), p)))
    .collect();
    parts.extend((1..=300).map(|s| (format!("fuzz{s}"), suite::fuzz(s, 80))));
    for (name, part) in &parts {
        let _ = part.regenerate();
        let (_, _, failed) = forge_geom::meet::oracle::counts();
        assert_eq!(
            failed, 0,
            "{name}: a torus × cylinder event search not certified (see stderr)"
        );
    }
    let (certified, _, _) = forge_geom::meet::oracle::counts();
    assert!(
        certified > 1000,
        "only {certified} searches certified: is the oracle reached?"
    );
}
