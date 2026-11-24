//! Our defaults against the makers' published rows: no row's ratio moves without a re-pin
//! (FORGE_BLESS=1 cargo test -p feeds-vs-makers, numbers in the commit), and the medians stay in
//! the agreed bands (verify/feeds-vs-makers/REPORT.md).
use std::path::Path;

fn rows() -> Vec<feeds_vs_makers::Row> {
    let raw = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/cam/tooling/raw");
    feeds_vs_makers::rows(&raw).0
}

#[test]
fn every_row_is_pinned() {
    let rows = rows();
    assert!(
        rows.len() >= 2000,
        "{} rows: the raw files are missing?",
        rows.len()
    );
    let now: Vec<String> = rows.iter().map(feeds_vs_makers::pin_line).collect();
    let pin = Path::new(env!("CARGO_MANIFEST_DIR")).join("pinned.tsv");
    if std::env::var("FORGE_BLESS").is_ok() {
        std::fs::write(&pin, now.join("\n") + "\n").unwrap();
        return;
    }
    let was = std::fs::read_to_string(&pin).expect("pinned.tsv (FORGE_BLESS=1 to write it)");
    let was: Vec<&str> = was.lines().collect();
    let moved: Vec<String> = now
        .iter()
        .zip(&was)
        .filter(|(a, b)| a.as_str() != **b)
        .take(20)
        .map(|(a, b)| format!("now {a}\nwas {b}"))
        .collect();
    assert!(
        now.len() == was.len() && moved.is_empty(),
        "{} rows now, {} pinned; moved (first 20; re-pin only for a judged change):\n{}",
        now.len(),
        was.len(),
        moved.join("\n")
    );
}

/// The generic default is the lower quartile of its class: over the makers' rows ours sits
/// under the makers at the median (speed and chip), and no group's median is far over.
#[test]
fn generic_defaults_sit_under_the_makers() {
    use feeds_vs_makers::stats;
    let rows = rows();
    let mill: Vec<_> = rows.iter().filter(|r| r.kind != "drill").collect();
    let med = |v: Vec<f64>| stats(v).unwrap().0;
    let all_rpm = med(mill.iter().map(|r| r.r_rpm()).collect());
    let all_f = med(mill.iter().filter_map(|r| r.r_f()).collect());
    for (what, m) in [("milling rpm", all_rpm), ("milling fz", all_f)] {
        assert!((0.6..=1.0).contains(&m), "{what} median {m}");
    }
    for g in ["P", "M", "K", "N", "S", "H"] {
        let sel: Vec<_> = mill.iter().filter(|r| r.group == g).collect();
        let r = med(sel.iter().map(|r| r.r_rpm()).collect());
        // H has one maker: no class, the group table (Helical's two hardness classes straddle it)
        let hi = if g == "H" { 1.5 } else { 1.0 };
        assert!((0.5..=hi).contains(&r), "group {g} rpm median {r}");
        if let Some((f, ..)) = stats(sel.iter().filter_map(|r| r.r_f()).collect()) {
            assert!((0.4..=1.1).contains(&f), "group {g} fz median {f}");
        }
    }
    // drills (unchanged: the drill rows are the drill model's own calibration)
    let drill: Vec<_> = rows.iter().filter(|r| r.kind == "drill").collect();
    let r = med(drill.iter().map(|r| r.r_rpm()).collect());
    assert!((0.6..=1.6).contains(&r), "drill rpm median {r}");
}

/// Each maker against P25-by-class built from the other makers only: the held-out speed
/// median is at most the maker's own for every maker (Harvey's sits right at the others' P25:
/// 1.00); the chip, within the makers' own spread
/// (Helical prints chips ~0.4-0.75 of the others', so the others' P25 is over its median).
/// The shares above each maker's own numbers are in REPORT.md.
#[test]
fn generic_held_out_by_maker() {
    let rows = rows();
    let ho = feeds_vs_makers::generic_held_out(&rows);
    for m in ["harvey", "helical", "lakeshore"] {
        let sel: Vec<_> = ho.iter().filter(|x| rows[x.0].vendor == m).collect();
        assert!(sel.len() >= 100, "{m}: {} rows held out", sel.len());
        let v = feeds_vs_makers::stats(sel.iter().map(|x| x.1).collect())
            .unwrap()
            .0;
        assert!(v <= 1.01, "{m}: held-out P25 speed median {v}");
        let c = feeds_vs_makers::stats(sel.iter().filter_map(|x| x.2).collect())
            .unwrap()
            .0;
        assert!((0.25..=1.5).contains(&c), "{m}: held-out P25 chip median {c}");
    }
}

/// data/generic.toml is what the makers' rows give now (FORGE_BLESS=1, or
/// `cargo run -p feeds-vs-makers -- --generic`, to write it).
#[test]
fn generic_table_is_current() {
    let t = feeds_vs_makers::generic_table(&rows(), None);
    let now = feeds_vs_makers::generic_toml(&t);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/forge-tooling/data/generic.toml");
    if std::env::var("FORGE_BLESS").is_ok() {
        std::fs::write(&path, &now).unwrap();
        return;
    }
    assert!(
        std::fs::read_to_string(&path).unwrap() == now,
        "data/generic.toml is stale: cargo run -p feeds-vs-makers -- --generic"
    );
}

/// Presets ON: the preset written for a maker's row gives that row's numbers back, inside its
/// printed range, for every row (a Lakeshore diameter on a column boundary takes the column it
/// starts, CAM-HAAS ac551a88, and its feed lies in both columns' ranges).
#[test]
fn own_presets_reproduce_the_makers_rows() {
    let rows = rows();
    let own: Vec<_> = rows.iter().filter(|r| r.own.is_some()).collect();
    assert!(own.len() >= 2900, "{} rows with their own preset", own.len());
    for r in &own {
        assert!(
            r.own_in_range() == Some(true),
            "{} {} {} {}: own preset {:?} outside {:?} {:?}",
            r.vendor,
            r.tool,
            r.op,
            r.material,
            r.own,
            r.theirs_rpm_span,
            r.theirs_f_span
        );
    }
}
