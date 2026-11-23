#![allow(clippy::disallowed_methods)]
//! L6 for the offset: random star polygons (some with a round island), offset in and out, judged
//! point by point against the definition C = {p in R : d(p, ∂R) >= r} (or R ⊕ B_r outward),
//! away from a thin band around ∂C where f64 sampling cannot judge.
use forge_cam_geom::offset::{OLoop, offset};
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
fn random_regions_match_the_definition() {
    let mut rng = Rng(0x5eed);
    let mut judged = 0usize;
    for case in 0..60 {
        let mut loops = vec![star(&mut rng, 0.0, 0.0)];
        if case % 3 == 0 {
            let r = 1.0 + 2.0 * rng.next();
            let a = p2(r, 0.0);
            loops.push(Loop {
                segs: vec![Seg::Arc {
                    a,
                    b: a,
                    c: p2(0.0, 0.0),
                    ccw: false,
                }],
            });
        }
        let rg = Region { loops };
        // star polygons can still self-intersect (consecutive angles close): skip refusals
        let r = if case % 4 == 3 {
            -(0.25 + 3.0 * rng.next())
        } else {
            0.25 + 5.0 * rng.next()
        };
        let (out, _) = match offset(&rg, r) {
            Ok(o) => o,
            Err(e) => {
                assert!(
                    matches!(e, forge_cam_geom::offset::OffsetError::Region(_)),
                    "case {case}: {e:?}"
                );
                continue;
            }
        };
        let bnd: Vec<Vec<P2>> = rg.loops.iter().map(|l| polyline(&l.segs)).collect();
        let offs: Vec<Vec<P2>> = out
            .iter()
            .filter(|l: &&OLoop| l.segs.len() > 1 || matches!(l.segs[0], Seg::Arc { .. }))
            .map(|l| polyline(&l.segs))
            .collect();
        let band = 2e-3 * 20.0;
        for _ in 0..400 {
            let q = p2(-30.0 + 60.0 * rng.next(), -30.0 + 60.0 * rng.next());
            let inside_r: i32 = bnd.iter().map(|p| winding(p, q)).sum();
            let d = dist(&bnd, q);
            let want = if r > 0.0 {
                inside_r != 0 && d >= r
            } else {
                inside_r != 0 || d <= -r
            };
            let margin = (d - r.abs()).abs();
            if margin < band {
                continue;
            }
            let w: i32 = offs.iter().map(|p| winding(p, q)).sum();
            let got = if r > 0.0 {
                w != 0
            } else {
                w == 0 && !offs.is_empty() || w != 0
            };
            // outward: the loops run with the outside on their left (clockwise), so a point of
            // R ⊕ B_r has winding −1 inside the outer loop
            let got = if r > 0.0 { got } else { w != 0 };
            assert_eq!(
                got,
                want,
                "case {case} r {r} q {q:?} d {d} w {w} loops {}",
                out.len()
            );
            judged += 1;
        }
    }
    assert!(judged > 10_000, "{judged}");
}
