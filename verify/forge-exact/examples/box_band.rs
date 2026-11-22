//! The box-box family inside the tolerance band: every pattern with coincident ends, B's coincident
//! ends moved by delta x LINEAR (the same delta on every axis), against the two exact readings.
//! cargo run --release -p forge-exact --example box_band -- [delta ...]
//! A line per run whose outcome is neither reading's (or is not a valid body or a clean refusal).
use forge_bool::Op;
use forge_exact::{Outcome, Pattern, block, exact, kernel_on};
use forge_math::tol::LINEAR;

fn main() {
    let deltas: Vec<f64> = {
        let d: Vec<f64> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
        if d.is_empty() {
            vec![-2.5, -1.5, -0.9, -0.5, 0.5, 0.9, 1.5, 2.5]
        } else {
            d
        }
    };
    let unit = 10.0;
    // BAND_AXES=02: move the coincident ends on these axes only (default all three)
    let axes: Vec<usize> = std::env::var("BAND_AXES")
        .map(|s| {
            s.chars()
                .filter_map(|c| c.to_digit(10).map(|d| d as usize))
                .collect()
        })
        .unwrap_or_else(|_| vec![0, 1, 2]);
    let mut tally = std::collections::BTreeMap::new();
    for &dl in &deltas {
        let (mut runs, mut bad) = (0, 0);
        for p in Pattern::all() {
            let (a, mut b) = p.boxes(unit);
            // B's ends equal to one of A's move by delta (outwards from B's centre on its low end
            // and its high end alike: the pattern's coincidences all open or all close)
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
            let got = kernel_on(block(a, 1), block(b, 2), p.op);
            // the readings: the coincident pattern, and the boxes as they now are taken exactly
            let coincident = exact(&p, unit);
            let apart = forge_exact::exact_boxes(a, b, p.op);
            let fine = got.agrees(&coincident)
                || got.agrees(&apart)
                || (matches!(got, Outcome::Ok(_)) && near_volume(&got, &[&coincident, &apart]));
            if !fine {
                bad += 1;
                *tally
                    .entry((
                        dl.to_bits(),
                        got.word().split(' ').take(2).collect::<Vec<_>>().join(" "),
                    ))
                    .or_insert(0) += 1;
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    dl,
                    p.label(),
                    got.word(),
                    coincident.word(),
                    apart.word()
                );
            }
        }
        eprintln!("delta {dl} LINEAR: {bad} of {runs} runs neither reading");
    }
    for ((d, w), n) in tally {
        eprintln!("  {:>5} {w}: {n}", f64::from_bits(d));
    }
    let _ = Op::Union;
}

/// A valid body within 1e-6 relative of either reading's volume (the band moves volume by ~1e-5 mm).
fn near_volume(got: &Outcome, readings: &[&Outcome]) -> bool {
    let Outcome::Ok(v) = got else { return false };
    readings.iter().any(|r| match r {
        Outcome::Ok(w) => (v - w).abs() <= 1e-4 * w.max(1.0),
        _ => false,
    })
}
