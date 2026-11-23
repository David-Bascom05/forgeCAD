#![allow(clippy::disallowed_methods)]
//! L6 for the offset: random star polygons (some with a round island), offset in and out, judged
//! point by point against the definition C = {p in R : d(p, ∂R) >= r} (or R ⊕ B_r outward),
//! away from a thin band around ∂C where f64 sampling cannot judge.
use forge_cam_geom::region::{Loop, P2, Region, Seg, p2};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// A segment chain as a polyline (arcs finely subdivided).
fn polyline(segs: &[Seg]) -> Vec<P2> {
    let mut out = Vec::new();
    for s in segs {
        match *s {
            Seg::Line { a, .. } => out.push(a),
            Seg::Arc { a, b, c, ccw } => {
                let r = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
                let t0 = (a.y - c.y).atan2(a.x - c.x);
                let mut t1 = (b.y - c.y).atan2(b.x - c.x);
                if ccw {
                    while t1 <= t0 {
                        t1 += std::f64::consts::TAU;
                    }
                } else {
                    while t1 >= t0 {
                        t1 -= std::f64::consts::TAU;
                    }
                }
                let n = 512;
                for k in 0..n {
                    let t = t0 + (t1 - t0) * k as f64 / n as f64;
                    out.push(p2(c.x + r * t.cos(), c.y + r * t.sin()));
                }
            }
        }
    }
    out
}

fn winding(poly: &[P2], q: P2) -> i32 {
    let mut w = 0;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let c = (b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x);
        if a.y <= q.y {
            if b.y > q.y && c > 0.0 {
                w += 1;
            }
        } else if b.y <= q.y && c < 0.0 {
            w -= 1;
        }
    }
    w
}

fn seg_dist(a: P2, b: P2, q: P2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let t = (((q.x - a.x) * dx + (q.y - a.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    ((a.x + t * dx - q.x).powi(2) + (a.y + t * dy - q.y).powi(2)).sqrt()
}

fn dist(polys: &[Vec<P2>], q: P2) -> f64 {
    let mut d = f64::INFINITY;
    for p in polys {
        for i in 0..p.len() {
            d = d.min(seg_dist(p[i], p[(i + 1) % p.len()], q));
        }
    }
    d
}

fn star(rng: &mut Rng, cx: f64, cy: f64) -> Loop {
    let n = 4 + (rng.next() * 9.0) as usize;
    let mut ang: Vec<f64> = (0..n).map(|_| rng.next() * std::f64::consts::TAU).collect();
    ang.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pts: Vec<P2> = ang
        .iter()
        .map(|&t| {
            let r = 8.0 + 12.0 * rng.next();
            // round to a 1/64 grid: exact, and makes ties (parallel edges, equal lengths) likely
            p2(
                ((cx + r * t.cos()) * 64.0).round() / 64.0,
                ((cy + r * t.sin()) * 64.0).round() / 64.0,
            )
        })
        .collect();
    let segs = (0..n)
        .map(|i| Seg::Line {
            a: pts[i],
            b: pts[(i + 1) % n],
        })
        .collect();
    Loop { segs }
}

#[test]
fn random_booleans_match_the_definition() {
    use forge_cam_geom::boolean::{Op, boolean};
    let mut rng = Rng(0xb001);
    let mut judged = 0usize;
    for case in 0..60 {
        let a = Region {
            loops: vec![star(&mut rng, 0.0, 0.0)],
        };
        let b = if case % 2 == 0 {
            {
                let (ox, oy) = (6.0 * rng.next() - 3.0, 6.0 * rng.next() - 3.0);
                Region {
                    loops: vec![star(&mut rng, ox, oy)],
                }
            }
        } else {
            let r = 4.0 + 8.0 * rng.next();
            let c = p2(10.0 * rng.next() - 5.0, 10.0 * rng.next() - 5.0);
            Region {
                loops: vec![Loop {
                    segs: vec![Seg::Arc {
                        a: p2(c.x + r, c.y),
                        b: p2(c.x + r, c.y),
                        c,
                        ccw: true,
                    }],
                }],
            }
        };
        let op = [Op::Union, Op::Inter, Op::Diff][case % 3];
        let out = match boolean(&a, &b, op) {
            Ok(o) => o,
            Err(_) => continue,
        };
        let (pa, pb): (Vec<Vec<P2>>, Vec<Vec<P2>>) = (
            a.loops.iter().map(|l| polyline(&l.segs)).collect(),
            b.loops.iter().map(|l| polyline(&l.segs)).collect(),
        );
        let po: Vec<Vec<P2>> = out.iter().map(|l| polyline(&l.segs)).collect();
        for _ in 0..400 {
            let q = p2(-30.0 + 60.0 * rng.next(), -30.0 + 60.0 * rng.next());
            let (ia, ib) = (
                pa.iter().map(|p| winding(p, q)).sum::<i32>() != 0,
                pb.iter().map(|p| winding(p, q)).sum::<i32>() != 0,
            );
            let near = dist(&pa, q).min(dist(&pb, q));
            if near < 0.05 {
                continue;
            }
            let want = match op {
                Op::Union => ia || ib,
                Op::Inter => ia && ib,
                Op::Diff => ia && !ib,
            };
            let got = po.iter().map(|p| winding(p, q)).sum::<i32>() != 0;
            assert_eq!(got, want, "case {case} {op:?} q {q:?}");
            judged += 1;
        }
    }
    assert!(judged > 10_000, "{judged}");
}
