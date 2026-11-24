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

/// The agreed bands on the medians of ours/theirs (the 10th–90th spread is the makers' own,
/// not ours, so only medians are banded).
#[test]
fn medians_stay_in_band() {
    use feeds_vs_makers::stats;
    let rows = rows();
    let mill: Vec<_> = rows.iter().filter(|r| r.kind != "drill").collect();
    let med = |v: Vec<f64>| stats(v).unwrap().0;
    let all_rpm = med(mill.iter().map(|r| r.r_rpm()).collect());
    let all_f = med(mill.iter().filter_map(|r| r.r_f()).collect());
    for (what, m) in [("milling rpm", all_rpm), ("milling fz", all_f)] {
        assert!((0.8..=1.25).contains(&m), "{what} median {m}");
    }
    for g in ["P", "M", "K", "N", "S", "H"] {
        let sel: Vec<_> = mill.iter().filter(|r| r.group == g).collect();
        let r = med(sel.iter().map(|r| r.r_rpm()).collect());
        // ours never well over the makers' speed for a group
        assert!((0.6..=1.5).contains(&r), "group {g} rpm median {r}");
        if let Some((f, ..)) = stats(sel.iter().filter_map(|r| r.r_f()).collect()) {
            assert!((0.5..=1.5).contains(&f), "group {g} fz median {f}");
        }
    }
    // drills (calibration rows: their medians are checked for drift by the pins)
    let drill: Vec<_> = rows.iter().filter(|r| r.kind == "drill").collect();
    let r = med(drill.iter().map(|r| r.r_rpm()).collect());
    assert!((0.6..=1.6).contains(&r), "drill rpm median {r}");
}

/// Each maker against a table derived without it: no maker's held-out median is more than 3x
/// from the others' consensus (the makers' own spread reaches ~2.5x: Harvey's slot chips,
/// Guhring's drill speeds; the per-row pins catch any drift of ours).
#[test]
fn held_out_makers_stay_in_band() {
    for (m, n, rpm, chip) in feeds_vs_makers::held_out_by_maker(&rows()) {
        if n == 0 {
            continue;
        }
        let r = rpm.unwrap().0;
        assert!((0.33..=3.0).contains(&r), "{m}: held-out rpm median {r}");
        if let Some((c, ..)) = chip {
            assert!((0.33..=3.0).contains(&c), "{m}: held-out chip median {c}");
        }
    }
}

/// Presets ON: the preset written for a maker's row gives that row's numbers back (inside its
/// printed range). The only exceptions are Lakeshore's tools at a diameter that ends one
/// column of its table and starts the next (1/8, 1/4, 3/8, 1/2"): the preset holds the first
/// column's feed, the next column's row prints a higher one; the speed is in range.
#[test]
fn own_presets_reproduce_the_makers_rows() {
    let rows = rows();
    let own: Vec<_> = rows.iter().filter(|r| r.own.is_some()).collect();
    assert!(own.len() >= 2500, "{} rows with their own preset", own.len());
    for r in own.iter().filter(|r| r.own_in_range() == Some(false)) {
        let (lo, hi) = r.theirs_rpm_span;
        let n = r.own.unwrap().0;
        let boundary = [3.175, 6.35, 9.525, 12.7].iter().any(|d| (r.d - d).abs() < 1e-6);
        assert!(
            r.vendor == "lakeshore" && boundary && n >= lo * 0.999 && n <= hi * 1.001,
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
