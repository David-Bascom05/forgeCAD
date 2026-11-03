//! Reproduces a missed crossing in forge_geom::spline::meet::cross_sections: two closed planar
//! splines from `case(Overlap, 20, 0)`, cuts 16 and 18, against a fine polyline count.
use forge_geom::spline::meet::cross_sections;
use forge_math::Point2;
use forge_ops::Segment;
use sheet_proto::cases::{Kind, case};
fn main() {
    let c = case(Kind::Overlap, 20, 0);
    let spl = |i: usize| match &c.cuts[i].segments[0].0 {
        Segment::Spline(s) => s.c.clone(),
        _ => panic!("a spline"),
    };
    let (a, b) = (spl(16), spl(18));
    let (da, db) = (a.domain(), b.domain());
    let poly = |s: &forge_geom::spline::Spline, n: usize| -> Vec<Point2> {
        let d = s.domain();
        (0..=n)
            .map(|k| {
                let p = s.point(d.lo + d.width() * k as f64 / n as f64);
                Point2::new(p.x, p.y)
            })
            .collect()
    };
    let (pa, pb) = (poly(&a, 8000), poly(&b, 8000));
    let mut found = Vec::new();
    for i in 0..pa.len() - 1 {
        for j in 0..pb.len() - 1 {
            let (p, r, q, s) = (pa[i], pa[i + 1] - pa[i], pb[j], pb[j + 1] - pb[j]);
            let d = r.cross(s);
            if d == 0.0 {
                continue;
            }
            let (t, u) = ((q - p).cross(s) / d, (q - p).cross(r) / d);
            if (0.0..1.0).contains(&t) && (0.0..1.0).contains(&u) {
                found.push((da.lo + da.width() * (i as f64 + t) / 8000.0, p + r * t));
            }
        }
    }
    println!("polyline crossings {}", found.len());
    for f in &found {
        println!("  t {:.6} at {:?}", f.0, f.1);
    }
    let r = cross_sections(&a, da, &b, db);
    println!("cross_sections a,b {}: {:?}", r.len(), r);
    let r = cross_sections(&b, db, &a, da);
    println!("cross_sections b,a {}: {:?}", r.len(), r);
    // the same curves moved near the origin: is the missing crossing a matter of coordinate size?
    let shift =
        |s: &forge_geom::spline::Spline| s.mapped(|p| forge_math::Point3::new(p.x - 270.0, p.y - 131.0, p.z));
    let (ma, mb) = (shift(&a), shift(&b));
    let r = cross_sections(&ma, da, &mb, db);
    println!("moved to the origin {}: {:?}", r.len(), r);
    // and the leaf's Newton at the missing crossing, step by step
    let (mut s, mut t) = (found[1].0, {
        let q = found[1].1;
        b.closest(forge_math::Point3::new(q.x, q.y, 0.0), db)
    });
    for i in 0..12 {
        let (p, q) = (a.deriv(s), b.deriv(t));
        let e = q.p - p.p;
        let det = p.d1.x * (-q.d1.y) + q.d1.x * p.d1.y;
        let ds = (e.x * (-q.d1.y) + q.d1.x * e.y) / det;
        let dt = (p.d1.x * e.y - p.d1.y * e.x) / det;
        s += ds;
        t += dt;
        println!(
            "  newton {i}: step measure {:.3e} (limit 1e-13) residual {:.2e}",
            (ds * p.d1.length()).abs() + (dt * q.d1.length()).abs(),
            e.length()
        );
    }
}
