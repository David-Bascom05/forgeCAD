//! The tolerance band through the decision log (forge-bool feature `decisions`, branch
//! claude/repo-review-mbdrlj-decisions): for each delta, every pattern with coincident ends moved
//! as box_band moves them, and the decision pairs that disagree, tallied separately over the runs
//! that fail (neither reading) and the runs that pass, plus the retry rungs each set reaches.
//! cargo run --release -p forge-exact --features decisions --example box_decisions -- [delta ...]
//! BAND_AXES as in box_band. ONE="x/y/z op" restricts to one pattern and prints each example.
use forge_bool::decisions;
use forge_exact::{Outcome, Pattern, block, exact, exact_boxes, kernel_on};
use forge_math::tol::LINEAR;
use std::collections::BTreeMap;

fn main() {
    let deltas: Vec<f64> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let axes: Vec<usize> = std::env::var("BAND_AXES")
        .map(|s| s.chars().filter_map(|c| c.to_digit(10).map(|d| d as usize)).collect())
        .unwrap_or_else(|_| vec![0, 1, 2]);
    let one = std::env::var("ONE").ok();
    let unit = 10.0;
    for &dl in &deltas {
        // (failing?, kind, site, site) -> runs
        let mut pairs: BTreeMap<(bool, String, String, String), (u64, String)> = BTreeMap::new();
        let mut rungs: BTreeMap<(bool, String), [u64; 4]> = BTreeMap::new();
        let (mut runs, mut bad, mut bad_clean, mut good_contra) = (0, 0, 0, 0);
        let _ = decisions::take();
        for p in Pattern::all() {
            if one.as_deref().is_some_and(|o| o != p.label()) {
                continue;
            }
            let (a, mut b) = p.boxes(unit);
            let mut moved = false;
            for &k in &axes {
                for s in 0..2 {
                    if a[k].contains(&b[k][s]) {
                        b[k][s] += if s == 0 { -dl * LINEAR } else { dl * LINEAR };
                        moved = true;
                    }
                }
            }
            if !moved {
                continue;
            }
            runs += 1;
            decisions::input("box_band", &format!("{} {dl}", p.label()));
            let got = kernel_on(block(a, 1), block(b, 2), p.op);
            let st = decisions::take();
            let coincident = exact(&p, unit);
            let apart = exact_boxes(a, b, p.op);
            let fine = got.agrees(&coincident)
                || got.agrees(&apart)
                || (matches!(got, Outcome::Ok(_)) && near_volume(&got, &[&coincident, &apart]));
            let failing = !fine;
            if failing {
                bad += 1;
                if st.pairs.is_empty() {
                    bad_clean += 1;
                    if one.is_some() || bad_clean <= 5 {
                        println!("NO-CONTRADICTION\t{dl}\t{}\t{}", p.label(), got.word());
                    }
                }
            } else if !st.pairs.is_empty() {
                good_contra += 1;
            }
            for ((kind, s1, s2), ps) in st.pairs {
                let e = pairs
                    .entry((failing, kind.name().to_string(), s1, s2))
                    .or_insert((0, String::new()));
                e.0 += 1;
                if e.1.is_empty() {
                    e.1 = format!("{} -> {} | {}", p.label(), got.word(), ps.example.replace('\n', " / "));
                }
            }
            for (r, rs) in st.rungs {
                let e = rungs.entry((failing, r.to_string())).or_default();
                for k in 0..4 {
                    e[k] += rs.outcomes[k];
                }
            }
        }
        println!(
            "== delta {dl} axes {axes:?}: {bad} of {runs} fail; {bad_clean} failing runs with no contradiction; {good_contra} passing runs with one"
        );
        for ((failing, kind, s1, s2), (n, ex)) in &pairs {
            println!("{}\t{n}\t{kind}\t{s1}  vs  {s2}", if *failing { "FAIL" } else { "pass" });
            println!("\t\te.g. {}", ex.chars().take(600).collect::<String>());
        }
        for ((failing, r), o) in &rungs {
            println!("{}\trung {r}: ok {} internal {} other {} unknown {}", if *failing { "FAIL" } else { "pass" }, o[0], o[1], o[2], o[3]);
        }
    }
}

fn near_volume(got: &Outcome, readings: &[&Outcome]) -> bool {
    let Outcome::Ok(v) = got else { return false };
    readings.iter().any(|r| match r {
        Outcome::Ok(w) => (v - w).abs() <= 1e-4 * w.max(1.0),
        _ => false,
    })
}
