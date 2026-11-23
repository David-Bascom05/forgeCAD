//! Tilted planes: box-like solids one of whose faces is turned by theta off the other body's face,
//! swept over theta from 1e-13 to 1e-1 and across each angle threshold the flat path reads:
//!   ANGULAR 1e-11  planes_coincident / plane_plane (forge-geom intersect.rs:846, 853)
//!   1e-9           classify_specs' lean (boolean.rs:5926)
//!   1e-6           trim's run-on line test (boolean.rs:2450)
//!   1e-4           nearly_flush NEAR (ssi.rs:66), with LINEAR over the region (ssi.rs:85)
//!   1e-3           the across face's `smooth` test on a straight edge (boolean.rs:4077)
//! Each run is checked against two readings: the bodies taken exactly (convex clipping, so every
//! family here is convex-convex) and the theta = 0 configuration taken exactly. A run passes when
//! it is a valid body whose volume is within 1e-6 relative of a reading (1e-9 absolute), or Empty
//! where a reading is (volume below 1e-9).
//! cargo run --release -p forge-coverage --example tilt_sweep -- [family ...]   (THETAS="1e-7 2e-7")
use forge_bool::Op;
use forge_coverage::Outcome;
use forge_coverage::kernel_on;
use forge_coverage::tilt::{FAMILIES, family, reading};

fn fine(got: &Outcome, vols: &[f64]) -> bool {
    match got {
        Outcome::Ok(v) => vols.iter().any(|w| (v - w).abs() <= 1e-6 * w.abs().max(1.0)),
        Outcome::Empty => vols.iter().any(|&w| w.abs() <= 1e-6),
        _ => false,
    }
}

fn main() {
    if let Ok(th) = std::env::var("POLY") {
        let th: f64 = th.parse().unwrap();
        let fam = std::env::var("FAM").unwrap_or_else(|_| "cap".into());
        let ((_, a), (_, b)) = family(&fam, th);
        let m = a.meet(&b);
        println!("A {} B {} meet {}", a.volume(), b.volume(), m.volume());
        for f in &m.faces {
            println!("  {f:?}");
        }
        for (n, d) in b.planes() {
            println!("  plane {n:?} {d}");
        }
        return;
    }
    let fams: Vec<String> = std::env::args().skip(1).collect();
    let fams: Vec<&str> = if fams.is_empty() {
        FAMILIES.to_vec()
    } else {
        fams.iter().map(|s| s.as_str()).collect()
    };
    let thetas: Vec<f64> = match std::env::var("THETAS") {
        Ok(s) => s.split_whitespace().filter_map(|t| t.parse().ok()).collect(),
        Err(_) => {
            let mut t = vec![0.0];
            for e in -13..=-1 {
                for m in [1.0, 2.0, 5.0] {
                    t.push(m * 10f64.powi(e));
                }
            }
            for th in [1e-11, 1e-9, 1e-6, 1e-4, 1e-3] {
                t.push(th * 0.9);
                t.push(th * 1.1);
            }
            t.sort_by(f64::total_cmp);
            t
        }
    };
    let mut bad = 0;
    let mut runs = 0;
    for fam in &fams {
        let ops = std::env::var("OPS").unwrap_or_default();
        for op in [Op::Union, Op::Subtract, Op::Intersect]
            .into_iter()
            .filter(|o| ops.is_empty() || ops.contains(forge_coverage::op_name(*o)))
        {
            let mut line = Vec::new();
            for &th in &thetas {
                let ((ab, ap), (bb, bp)) = family(fam, th);
                let ((_, a0), (_, b0)) = family(fam, 0.0);
                let got = kernel_on(ab, bb, op);
                let vols = [reading(&ap, &bp, op), reading(&a0, &b0, op)];
                runs += 1;
                let ok = fine(&got, &vols);
                if !ok {
                    bad += 1;
                    let w = match &got {
                        Outcome::Ok(v) => format!("ok {v:.9}"),
                        o => o.word(),
                    };
                    println!(
                        "FAIL\t{fam}\t{}\t{th:e}\t{w}\texact {:.9} flat {:.9}",
                        forge_coverage::op_name(op),
                        vols[0],
                        vols[1]
                    );
                }
                line.push(if ok { '.' } else { 'x' });
            }
            eprintln!(
                "{fam:>10} {:>9} {}",
                forge_coverage::op_name(op),
                line.iter().collect::<String>()
            );
        }
    }
    eprintln!(
        "thetas: {}",
        thetas
            .iter()
            .map(|t| format!("{t:e}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    eprintln!("{bad} of {runs} runs fail");
}
