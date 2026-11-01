//! Does a warm-started solve reach a cold one's bits, and is it cheaper? warmbits PART NAME [swing] [delta]
//! Takes the traced curves (`IntCurve`) at two values of a number, matches them by edge name:
//! chain lengths, Newton steps from old node vs from the march predictor, and the limit bits of a
//! square solve (both surfaces + a station plane) from starts at several distances.
use forge_agent::bench;
use forge_agent::tools::{Dim, Part};
use forge_geom::intcurve::IntCurve;
use forge_geom::{Curve, Surface};
use forge_math::{Point3, Vec3};
use std::collections::HashMap;

fn curves(p: &Part) -> HashMap<forge_topo::Name, std::sync::Arc<IntCurve>> {
    let mut o = HashMap::new();
    let m = &p.model;
    for s in m.solids_after(m.built()) {
        for e in s.body.edge_ids() {
            let x = s.body.edge(e);
            if let Curve::Intersection(c) = &x.curve {
                o.insert(x.name, c.clone());
            }
        }
    }
    o
}

/// refine's own loop, counting steps; 40 = gave up.
fn steps(a: &Surface, b: &Surface, mut p: Point3) -> usize {
    for k in 0..40 {
        let ((ga, na), (gb, nb)) = (a.implicit_gradient(p), b.implicit_gradient(p));
        if ga.abs() <= 1e-10 && gb.abs() <= 1e-10 {
            return k;
        }
        let (aa, ab, bb) = (na.dot(na), na.dot(nb), nb.dot(nb));
        let det = aa * bb - ab * ab;
        if det <= 1e-12 * aa * bb {
            return 40;
        }
        p += na * ((-ga * bb + gb * ab) / det) + nb * ((-gb * aa + ga * ab) / det);
    }
    40
}

/// Square Newton: both surfaces and the plane through q normal to t. Runs `n` steps, returns the end.
fn square(a: &Surface, b: &Surface, q: Point3, t: Vec3, mut p: Point3, n: usize) -> Point3 {
    for _ in 0..n {
        let ((ga, na), (gb, nb)) = (a.implicit_gradient(p), b.implicit_gradient(p));
        let gc = t.dot(p - q);
        // solve [na; nb; t] d = -[ga; gb; gc] by Cramer
        let (c1, c2, c3) = (nb.cross(t), t.cross(na), na.cross(nb));
        let det = na.dot(c1);
        if det == 0.0 {
            return p;
        }
        p += (c1 * ga + c2 * gb + c3 * gc) * (-1.0 / det);
    }
    p
}

fn rng(s: &mut u64) -> f64 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*s >> 33) as f64 / (1u64 << 31) as f64) - 1.0
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let src = bench::src(&a[0]).expect("a part");
    let names: Vec<&str> = a[1].split('+').collect();
    let delta: f64 = a.get(2).map_or(0.002, |s| s.parse().unwrap());
    let mut p = Part::of(bench::model(src));
    let dims: Vec<Dim> = p.dims();
    let d0 = curves(&p);
    for n in &names {
        let d = dims.iter().find(|d| d.name == *n).expect("a number");
        p.model.edit(d.at.op(forge_agent::short(d.value * (1.0 + delta)))).unwrap();
    }
    let d1 = curves(&p);
    let (mut same, mut len_diff, mut nodes) = (0, 0, 0usize);
    let (mut warm, mut pred, mut warm_n) = (0usize, 0usize, 0usize);
    let (mut bits_same, mut bits_tot, mut spread_max, mut fixed2, mut one_limit) = (0usize, 0usize, 0f64, 0usize, 0usize);
    let mut rs = 7u64;
    let mut names_sorted: Vec<_> = d1.keys().copied().collect();
    names_sorted.sort();
    let mut surfaces_proc = 0usize;
    for n in &names_sorted {
        let c1 = &d1[n];
        let Some(c0) = d0.get(n) else { continue };
        if c0.pts == c1.pts {
            same += 1;
            continue;
        }
        if c0.pts.len() != c1.pts.len() {
            len_diff += 1;
        }
        nodes += c1.pts.len();
        if matches!(c1.a, Surface::Procedural(_)) || matches!(c1.b, Surface::Procedural(_)) {
            surfaces_proc += 1;
        }
        // old node i as the start (nearest index by fraction), against march's predictor
        for i in 1..c1.pts.len() {
            let f = i as f64 / (c1.pts.len() - 1).max(1) as f64;
            let j = ((f * (c0.pts.len() - 1) as f64).round() as usize).min(c0.pts.len() - 1);
            let h = c1.pts[i].distance(c1.pts[i - 1]);
            warm += steps(&c1.a, &c1.b, c0.pts[j]);
            pred += steps(&c1.a, &c1.b, c1.pts[i - 1] + c1.tans[i - 1] * h);
            warm_n += 1;
        }
        // limits of the square system from starts near a node
        for i in (0..c1.pts.len()).step_by((c1.pts.len() / 6).max(1)) {
            let (q, t) = (c1.pts[i], c1.tans[i]);
            let mut ends: Vec<Point3> = vec![];
            let mut settled = true;
            for scale in [1e-2, 1e-4, 1e-6, 1e-9, 0.0] {
                for _ in 0..3 {
                    let s = q + Vec3::new(rng(&mut rs), rng(&mut rs), rng(&mut rs)) * scale;
                    let e = square(&c1.a, &c1.b, q, t, s, 10);
                    let e2 = square(&c1.a, &c1.b, q, t, e, 1);
                    if e2 != e {
                        settled = false;
                    }
                    ends.push(e);
                }
            }
            let all_same = ends.iter().all(|e| *e == ends[0]);
            bits_tot += 1;
            bits_same += all_same as usize;
            fixed2 += settled as usize;
            one_limit += all_same as usize;
            let sp = ends.iter().map(|e| e.distance(ends[0])).fold(0.0, f64::max);
            spread_max = spread_max.max(sp);
        }
    }
    let _ = (bits_same,);
    println!(
        "{} {:?} delta {delta}: {} curves, {same} same bits, {len_diff} of the moved changed chain length ({surfaces_proc} on a procedural surface), {nodes} nodes; Newton steps/node warm {:.2} predictor {:.2}; square solve: {one_limit}/{bits_tot} nodes one limit from all starts, {fixed2} a true fixed point, spread up to {spread_max:.1e} mm",
        a[0], names, d1.len(), warm as f64 / warm_n.max(1) as f64, pred as f64 / warm_n.max(1) as f64
    );
}
