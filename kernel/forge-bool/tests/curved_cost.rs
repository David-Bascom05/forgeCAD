//! Measurement only (curved-cost-estimate.md): the exact pieces' wall-clock on one plane–cylinder
//! and one cylinder–cylinder pair. `cargo test --release --features decide_once_curved --test
//! curved_cost -- --ignored --nocapture`.
#![cfg(feature = "decide_once_curved")]
use forge_alg::Q;
use forge_bool::decide_once_curved::*;

fn qi(n: i64) -> Q {
    Q::from_i64(n)
}

#[test]
#[ignore]
fn exact_piece_costs() {
    // the plane z = 3 against the drill x² + y² − 8x − 4y + 4 = 0 (r 4 at (4, 2))
    let pl = Quadric {
        lz: qi(1),
        c: qi(-3),
        ..Quadric::default()
    };
    let cyl = Quadric {
        xx: qi(1),
        yy: qi(1),
        lx: qi(-8),
        ly: qi(-4),
        c: qi(4),
        ..Quadric::default()
    };
    let t = std::time::Instant::now();
    let pc = pair_curve(&pl, &cyl).unwrap();
    let t1 = t.elapsed();
    let ns = curve_nodes(&pl, &cyl, &pc);
    eprintln!(
        "COST plane-cyl: curve {:?} (events {}, slabs {}), nodes {:?} ({})",
        t1,
        pc.events.len(),
        pc.slabs.len(),
        t.elapsed() - t1,
        ns.is_some()
    );
    // a cross hole: y² + z² − 9 = 0 against x² + y² − 16 = 0
    let c2 = Quadric {
        yy: qi(1),
        zz: qi(1),
        c: qi(-9),
        ..Quadric::default()
    };
    let c1 = Quadric {
        xx: qi(1),
        yy: qi(1),
        c: qi(-16),
        ..Quadric::default()
    };
    let t = std::time::Instant::now();
    let pc = pair_curve(&c1, &c2).unwrap();
    eprintln!(
        "COST cyl-cyl: curve {:?} (events {}, slabs {})",
        t.elapsed(),
        pc.events.len(),
        pc.slabs.len()
    );
    // three carriers at a vertex: plane, plane, cylinder; and plane, cylinder, cylinder
    let px = Quadric {
        lx: qi(1),
        c: qi(-6),
        ..Quadric::default()
    };
    let t = std::time::Instant::now();
    let v = propose_triple(
        &in_frame(&frames()[0], &pl),
        &in_frame(&frames()[0], &px),
        &in_frame(&frames()[0], &cyl),
        ROOT_FUEL,
    );
    eprintln!(
        "COST vertex plane-plane-cyl: {:?} ({:?})",
        t.elapsed(),
        v.map(|v| v.len())
    );
    let t = std::time::Instant::now();
    let v = propose_triple(
        &in_frame(&frames()[0], &pl),
        &in_frame(&frames()[0], &c1),
        &in_frame(&frames()[0], &c2),
        ROOT_FUEL,
    );
    eprintln!(
        "COST vertex plane-cyl-cyl: {:?} ({:?})",
        t.elapsed(),
        v.map(|v| v.len())
    );
}
