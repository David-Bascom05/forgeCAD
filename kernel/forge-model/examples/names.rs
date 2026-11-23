//! Every regenerated part's names by where they lie: each face, edge and vertex as a line of its
//! place (rounded to 1e-4) and name, sorted, for comparing names between builds (forge-bool's
//! decide_once switch on and off, say). The parts of `bits` (the suite, torture) and the fuzz
//! seeds 1..=N (NAMES_FUZZ=N, default 10), seq and n-ary: `names [PART]`.
use forge_geom::{CurveGeom, Surface, SurfaceGeom};
use forge_model::{suite, torture};
use forge_topo::Body;

fn lines(b: &Body) -> Vec<String> {
    let r = |x: f64| (x * 1e4).round() / 1e4 + 0.0;
    let mut out: Vec<String> = Vec::new();
    let key = |p: forge_math::Point3| (r(p.x), r(p.y), r(p.z));
    let lt = |a: &(f64, f64, f64), b: &(f64, f64, f64)| {
        a.0.total_cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.total_cmp(&b.2))
    };
    for f in b.face_ids() {
        let face = b.face(f);
        // a face by its least vertex (the loops may start anywhere), with its surface's kind and
        // its normal there
        let p = face
            .loops
            .iter()
            .flat_map(|&l| b.lp(l).coedges().map(|c| b.vertex(b.coedge_start(c)).point))
            .min_by(|x, y| lt(&key(*x), &key(*y)))
            .unwrap_or(forge_math::Point3::ORIGIN);
        let kind = match &face.surface {
            Surface::Plane(_) => "plane",
            _ => "curved",
        };
        let n = face
            .surface
            .normal(face.surface.project(p))
            .map_or(forge_math::Vec3::new(0.0, 0.0, 0.0), |n| n.get());
        let s = if face.same_sense { 1.0 } else { -1.0 };
        out.push(format!(
            "face {kind} {:?} n ({}, {}, {}) {:?}",
            key(p),
            r(s * n.x),
            r(s * n.y),
            r(s * n.z),
            face.name
        ));
    }
    for e in b.edge_ids() {
        let ed = b.edge(e);
        // an edge by its ends, sorted (its parameter may run either way); a closed one by the
        // box of its samples too (its seam may lie anywhere)
        let (a, z) = (key(b.vertex(ed.start).point), key(b.vertex(ed.end).point));
        let (a, z) = if lt(&a, &z).is_le() { (a, z) } else { (z, a) };
        let mid = if ed.start == ed.end {
            let ps: Vec<_> = (0..16)
                .map(|k| {
                    ed.curve
                        .eval(ed.range.lo + (ed.range.hi - ed.range.lo) * k as f64 / 16.0)
                })
                .collect();
            let lo = ps.iter().fold((f64::MAX, f64::MAX, f64::MAX), |m, p| {
                (m.0.min(r(p.x)), m.1.min(r(p.y)), m.2.min(r(p.z)))
            });
            let hi = ps.iter().fold((f64::MIN, f64::MIN, f64::MIN), |m, p| {
                (m.0.max(r(p.x)), m.1.max(r(p.y)), m.2.max(r(p.z)))
            });
            format!(" closed {lo:?}..{hi:?}")
        } else {
            let m = ed.curve.eval(ed.range.mid());
            format!(" via {:?}", key(m))
        };
        out.push(format!("edge {a:?} {z:?}{mid} {:?}", ed.name));
    }
    for v in b.vertex_ids() {
        let p = b.vertex(v).point;
        out.push(format!("vertex {:?} {:?}", key(p), b.vertex(v).name));
    }
    out.sort();
    out
}

fn main() {
    let only = std::env::args().nth(1);
    let mut parts: Vec<(String, forge_model::Part)> = suite::parts()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    parts.push(("torture".to_string(), torture::part(221)));
    let seeds: u64 = std::env::var("NAMES_FUZZ")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(10);
    parts.extend((1..=seeds).map(|s| (format!("fuzz{s}"), suite::fuzz(s, 80))));
    for (name, part) in &parts {
        if only.as_ref().is_some_and(|o| o != name) {
            continue;
        }
        for (path, r) in [("seq", part.regenerate()), ("nary", part.regenerate_nary(1))] {
            println!("== {name} {path}");
            match r {
                Ok(b) => {
                    for l in lines(&b) {
                        println!("{l}");
                    }
                }
                Err(e) => println!("error {e:?}"),
            }
        }
    }
}
