//! The 2.5D checker in closed form: the faces that are prisms along z.
//!
//! For a planar move (the tip at one height z_t, its xy path a segment or an arc) and a body
//! {ρ ≤ w(z)}, the signed distance is non-decreasing in ρ at each height (tool.rs). So for a face
//! whose points are (section point, height):
//! - a horizontal plane at z_f with region T: min = sd(D(path, T), z_f − z_t);
//! - a vertical wall over a horizontal segment, or a vertical cylinder over an arc, for heights
//!   in [z0, z1]: min = min over Δ ∈ [z0 − z_t, z1 − z_t] of sd(D(path, section), Δ);
//! with D the least 2D distance between the path and the section, a closed form: the least of the
//! distances between each curve's ends and the other curve, the feet of the normals common to
//! both (a centre's foot on a line; the line of centres for two circles), and 0 where they meet.
//! Each candidate is an interval; one whose feasibility (the foot inside both pieces) is not
//! decided counts for the lower bound only. The Δ search is a one-dimensional interval branch
//! and bound. The same verdicts as `check::certify`.
use crate::check::{Face, Planar, Surf, TSeg, Verdict};
use crate::ival::Ival;
use crate::tool::Body;

/// A face that is a prism along z, in the part's xy.
#[derive(Clone, Debug, PartialEq)]
pub enum Prism {
    /// A horizontal face at height z, its region in xy.
    Flat {
        z: f64,
        region: Vec<TSeg>,
        boxes: Vec<[f64; 4]>,
        /// Each piece's end gap (arc_gap; 0 for a line), or −1 for an arc outside Piece.ok:
        /// worked out once with the face.
        gaps: Vec<f64>,
    },
    /// A vertical face over a section curve, for heights z0..z1; `gap` its arc's end gap.
    Wall { sec: Sec, z0: f64, z1: f64, gap: f64 },
}

/// A section or path curve in xy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sec {
    Seg {
        a: [f64; 2],
        b: [f64; 2],
    },
    /// Of at most a half turn.
    Arc {
        c: [f64; 2],
        r: f64,
        a: [f64; 2],
        b: [f64; 2],
        ccw: bool,
    },
}

fn p(x: f64) -> Ival {
    Ival::pt(x)
}
fn sub2(a: [f64; 2], b: [f64; 2]) -> [Ival; 2] {
    [p(a[0]).sub(p(b[0])), p(a[1]).sub(p(b[1]))]
}
fn len(v: [Ival; 2]) -> Ival {
    v[0].sqr().add(v[1].sqr()).sqrt()
}
fn cross(u: [Ival; 2], v: [Ival; 2]) -> Ival {
    u[0].mul(v[1]).sub(u[1].mul(v[0]))
}
fn dot(u: [Ival; 2], v: [Ival; 2]) -> Ival {
    u[0].mul(v[0]).add(u[1].mul(v[1]))
}

/// A candidate distance: its interval, and whether it is surely realised (both feet inside).
#[derive(Clone, Copy, Debug)]
struct Cand {
    d: Ival,
    sure: bool,
}

/// The candidates of one distance, on the stack (at most 4 ends, then a curve pair's own).
struct Cands {
    v: [Cand; 12],
    n: usize,
}

impl Cands {
    fn new() -> Cands {
        Cands {
            v: [Cand {
                d: Ival::pt(0.0),
                sure: false,
            }; 12],
            n: 0,
        }
    }
    fn push(&mut self, c: Cand) {
        self.v[self.n] = c;
        self.n += 1;
    }
}

impl std::ops::Deref for Cands {
    type Target = [Cand];
    fn deref(&self) -> &[Cand] {
        &self.v[..self.n]
    }
}

/// The least of the candidates: [min of all lows, min of the sure ones' highs].
fn least(cs: &[Cand]) -> Ival {
    let mut lo = f64::INFINITY;
    let mut hi = f64::INFINITY;
    let mut i = 0;
    while i < cs.len() {
        if cs[i].d.lo < lo {
            lo = cs[i].d.lo;
        }
        if cs[i].sure && cs[i].d.hi < hi {
            hi = cs[i].d.hi;
        }
        i += 1;
    }
    Ival { lo: lo.max(0.0), hi }
}

/// 1 surely in the arc's sector (closed), -1 surely out, 0 unknown, for the direction v from its
/// centre.
fn in_sector(c: [f64; 2], a: [f64; 2], b: [f64; 2], ccw: bool, v: [Ival; 2]) -> i32 {
    let (ua, ub) = (sub2(a, c), sub2(b, c));
    let (mut c1, mut c2) = (cross(ua, v), cross(v, ub));
    if !ccw {
        c1 = c1.neg();
        c2 = c2.neg();
    }
    if c1.lo >= 0.0 && c2.lo >= 0.0 {
        1
    } else if c1.hi < 0.0 || c2.hi < 0.0 {
        -1
    } else {
        0
    }
}

/// The distance from a point to a section curve (always realised).
pub fn pt_sec(q: [f64; 2], s: &Sec) -> Ival {
    pt_pre(q, &pre(s))
}

/// A section curve with what every point's distance to it shares, worked out once: a segment's
/// direction and its square length (the same arithmetic as before, done once per curve).
enum Pre {
    Seg { a: [f64; 2], b: [f64; 2], d: [Ival; 2], dd: Ival },
    Other(Sec),
}

fn pre(s: &Sec) -> Pre {
    match *s {
        Sec::Seg { a, b } => {
            let d = sub2(b, a);
            Pre::Seg { a, b, d, dd: dot(d, d) }
        }
        _ => Pre::Other(*s),
    }
}

fn pt_pre(q: [f64; 2], s: &Pre) -> Ival {
    let s = match s {
        Pre::Seg { a, b, d, dd } => return pt_seg(q, *a, *b, *d, *dd),
        Pre::Other(s) => s,
    };
    match *s {
        Sec::Seg { .. } => unreachable!("segments are Pre::Seg"),
        Sec::Arc { c, r, a, b, ccw } => {
            let v = sub2(q, c);
            let side = in_sector(c, a, b, ccw, v);
            if side == 1 {
                return len(v).sub(p(r)).abs();
            }
            let (da, db) = (len(sub2(q, a)), len(sub2(q, b)));
            let ends = Ival {
                lo: da.lo.min(db.lo),
                hi: da.hi.min(db.hi),
            };
            if side == -1 {
                return ends;
            }
            let circ = len(v).sub(p(r)).abs();
            Ival {
                lo: circ.lo.min(ends.lo),
                hi: ends.hi,
            }
        }
    }
}

fn pt_seg(q: [f64; 2], a: [f64; 2], b: [f64; 2], d: [Ival; 2], dd: Ival) -> Ival {
    let w = sub2(q, a);
    let t = dot(w, d).div(dd);
    // only what the answer needs
    if t.hi <= 0.0 {
        return len(w);
    }
    if t.lo >= 1.0 {
        return len(sub2(q, b));
    }
    let perp = cross(d, w).abs().div(dd.sqrt());
    if t.lo >= 0.0 && t.hi <= 1.0 {
        return perp;
    }
    let (da, db) = (len(w), len(sub2(q, b)));
    Ival {
        lo: perp.lo,
        hi: da.hi.min(db.hi),
    }
}

fn ends(s: &Sec) -> ([f64; 2], [f64; 2]) {
    match *s {
        Sec::Seg { a, b } | Sec::Arc { a, b, .. } => (a, b),
    }
}

#[allow(dead_code)]
/// 1 if the point (as an interval box) is surely on the segment's span (by its foot parameter),
/// -1 surely off, 0 unknown.
fn on_seg_span(a: [f64; 2], b: [f64; 2], q: [Ival; 2]) -> i32 {
    let d = sub2(b, a);
    let w = [q[0].sub(p(a[0])), q[1].sub(p(a[1]))];
    let t = dot(w, d).div(dot(d, d));
    if t.lo >= 0.0 && t.hi <= 1.0 {
        1
    } else if t.hi < 0.0 || t.lo > 1.0 {
        -1
    } else {
        0
    }
}

fn and3(a: i32, b: i32) -> i32 {
    if a < 0 || b < 0 {
        -1
    } else if a > 0 && b > 0 {
        1
    } else {
        0
    }
}

fn degenerate(s: &Sec) -> Option<[f64; 2]> {
    match *s {
        Sec::Seg { a, b } if a == b => Some(a),
        _ => None,
    }
}

/// The least distance between two section curves.
pub fn dist(s: &Sec, t: &Sec) -> Ival {
    // arcs reach here as pieces of at most a quarter turn (faces are lowered so, trims split so,
    // moves are cut so): strictly under a half turn, as the candidates are proven for
    // (cross(a − c, b − c) > 0 in the arc's sense: DistArc.lean)
    debug_assert!(
        [s, t].iter().all(|x| match **x {
            Sec::Arc { c, a, b, ccw, .. } => {
                let cr = cross(sub2(a, c), sub2(b, c));
                if ccw { cr.lo > 0.0 } else { cr.hi < 0.0 }
            }
            Sec::Seg { .. } => true,
        }),
        "slab::dist takes arcs strictly under a half turn"
    );
    if let Some(q) = degenerate(s) {
        return pt_sec(q, t);
    }
    if let Some(q) = degenerate(t) {
        return pt_sec(q, s);
    }
    let mut cs = Cands::new();
    let (sa, sb) = ends(s);
    let (ta, tb) = ends(t);
    let (ps, pt) = (pre(s), pre(t));
    cs.push(Cand {
        d: pt_pre(sa, &pt),
        sure: true,
    });
    cs.push(Cand {
        d: pt_pre(sb, &pt),
        sure: true,
    });
    cs.push(Cand {
        d: pt_pre(ta, &ps),
        sure: true,
    });
    cs.push(Cand {
        d: pt_pre(tb, &ps),
        sure: true,
    });
    match (*s, *t) {
        (Sec::Seg { a, b }, Sec::Seg { a: c, b: d }) => {
            // they cross: 0 (the segments' directions as worked out above)
            let (u, v) = match (&ps, &pt) {
                (Pre::Seg { d: u, .. }, Pre::Seg { d: v, .. }) => (*u, *v),
                _ => (sub2(b, a), sub2(d, c)),
            };
            let o1 = cross(u, sub2(c, a));
            let o2 = cross(u, sub2(d, a));
            let o3 = cross(v, sub2(a, c));
            let o4 = cross(v, sub2(b, c));
            let apart = (o1.lo > 0.0 && o2.lo > 0.0)
                || (o1.hi < 0.0 && o2.hi < 0.0)
                || (o3.lo > 0.0 && o4.lo > 0.0)
                || (o3.hi < 0.0 && o4.hi < 0.0);
            let cross_sure = ((o1.lo > 0.0 && o2.hi < 0.0) || (o1.hi < 0.0 && o2.lo > 0.0))
                && ((o3.lo > 0.0 && o4.hi < 0.0) || (o3.hi < 0.0 && o4.lo > 0.0));
            if !apart {
                cs.push(Cand {
                    d: Ival::pt(0.0),
                    sure: cross_sure,
                });
            }
        }
        (
            Sec::Seg { a, b },
            Sec::Arc {
                c,
                r,
                a: pa,
                b: pb,
                ccw,
            },
        )
        | (
            Sec::Arc {
                c,
                r,
                a: pa,
                b: pb,
                ccw,
            },
            Sec::Seg { a, b },
        ) => {
            let d = sub2(b, a);
            let dd = dot(d, d);
            let w = sub2(c, a);
            let t = dot(w, d).div(dd);
            let foot = [p(a[0]).add(t.mul(d[0])), p(a[1]).add(t.mul(d[1]))];
            let h = cross(d, w).abs().div(dd.sqrt());
            // the common normal through the centre: distance |h − r| if the foot is on the
            // segment and its direction from the centre in the arc
            let fv = [foot[0].sub(p(c[0])), foot[1].sub(p(c[1]))];
            let on = if t.lo >= 0.0 && t.hi <= 1.0 {
                1
            } else if t.hi < 0.0 || t.lo > 1.0 {
                -1
            } else {
                0
            };
            let f = and3(on, in_sector(c, pa, pb, ccw, fv));
            if f >= 0 {
                cs.push(Cand {
                    d: h.sub(p(r)).abs(),
                    sure: f > 0,
                });
            }
            // crossings: the line meets the circle where the foot ± k along it
            if h.lo <= r {
                let k2 = p(r).sqr().sub(h.sqr());
                let k = Ival {
                    lo: k2.lo.max(0.0),
                    hi: k2.hi.max(0.0),
                }
                .sqrt()
                .div(dd.sqrt());
                let mut sgn = 0;
                while sgn < 2 {
                    let tt = if sgn == 0 { t.add(k) } else { t.sub(k) };
                    let x = [p(a[0]).add(tt.mul(d[0])), p(a[1]).add(tt.mul(d[1]))];
                    let xv = [x[0].sub(p(c[0])), x[1].sub(p(c[1]))];
                    let on = if tt.lo >= 0.0 && tt.hi <= 1.0 {
                        1
                    } else if tt.hi < 0.0 || tt.lo > 1.0 {
                        -1
                    } else {
                        0
                    };
                    let f = and3(on, in_sector(c, pa, pb, ccw, xv));
                    if f >= 0 {
                        cs.push(Cand {
                            d: Ival::pt(0.0),
                            sure: f > 0 && h.hi < r,
                        });
                    }
                    sgn += 1;
                }
            }
        }
        (
            Sec::Arc {
                c: c1,
                r: r1,
                a: a1,
                b: b1,
                ccw: w1,
            },
            Sec::Arc {
                c: c2,
                r: r2,
                a: a2,
                b: b2,
                ccw: w2,
            },
        ) => {
            let dv = sub2(c2, c1);
            let dl = len(dv);
            if c1 == c2 {
                // concentric: |r1 − r2| wherever the sectors overlap
                let v1 = sub2(a1, c1);
                let v2 = sub2(a2, c2);
                let o = if in_sector(c1, a1, b1, w1, v2) > 0 || in_sector(c2, a2, b2, w2, v1) > 0 {
                    1
                } else if in_sector(c1, a1, b1, w1, sub2(b2, c2)) > 0
                    || in_sector(c2, a2, b2, w2, sub2(b1, c1)) > 0
                {
                    1
                } else {
                    0
                };
                cs.push(Cand {
                    d: p(r1).sub(p(r2)).abs(),
                    sure: o > 0,
                });
            } else {
                // the line of centres: the points c1 ± r1 u and c2 ± r2 u
                let mut i = 0;
                while i < 4 {
                    let (s1, s2) = (
                        if i & 1 == 0 { 1.0 } else { -1.0 },
                        if i & 2 == 0 { 1.0 } else { -1.0 },
                    );
                    let u1 = [dv[0].scale(s1), dv[1].scale(s1)];
                    let u2 = [dv[0].scale(s2), dv[1].scale(s2)];
                    let f = and3(in_sector(c1, a1, b1, w1, u1), in_sector(c2, a2, b2, w2, u2));
                    if f >= 0 {
                        // |d + s2 r2 − s1 r1|
                        let dd = dl.add(p(r2).scale(s2)).sub(p(r1).scale(s1)).abs();
                        cs.push(Cand { d: dd, sure: f > 0 });
                    }
                    i += 1;
                }
                // crossings
                let rsum = p(r1).add(p(r2));
                let rdif = p(r1).sub(p(r2)).abs();
                if dl.lo <= rsum.hi && dl.hi >= rdif.lo {
                    let dd2 = dot(dv, dv);
                    let kk = dd2.add(p(r1).sqr()).sub(p(r2).sqr()).div(dd2.scale(2.0));
                    let h2 = p(r1).sqr().div(dd2).sub(kk.sqr());
                    let h = Ival {
                        lo: h2.lo.max(0.0),
                        hi: h2.hi.max(0.0),
                    }
                    .sqrt();
                    let base = [p(c1[0]).add(kk.mul(dv[0])), p(c1[1]).add(kk.mul(dv[1]))];
                    let pv = [dv[1].neg(), dv[0]];
                    let mut sg = 0;
                    while sg < 2 {
                        let hs = if sg == 0 { h } else { h.neg() };
                        let x = [base[0].add(hs.mul(pv[0])), base[1].add(hs.mul(pv[1]))];
                        let f = and3(
                            in_sector(c1, a1, b1, w1, [x[0].sub(p(c1[0])), x[1].sub(p(c1[1]))]),
                            in_sector(c2, a2, b2, w2, [x[0].sub(p(c2[0])), x[1].sub(p(c2[1]))]),
                        );
                        if f >= 0 {
                            cs.push(Cand {
                                d: Ival::pt(0.0),
                                sure: f > 0 && h2.lo > 0.0,
                            });
                        }
                        sg += 1;
                    }
                }
            }
        }
    }
    least(&cs)
}

/// A box (xmin, ymin, xmax, ymax) holding a trim piece (an arc by its whole circle).
pub fn tseg_box(s: &TSeg) -> [f64; 4] {
    match *s {
        TSeg::Line { a, b } => [a[0].min(b[0]), a[1].min(b[1]), a[0].max(b[0]), a[1].max(b[1])],
        // the circle's box, and the ends (which may lie off the circle by its gap)
        TSeg::Arc { a, b, c, r, .. } => [
            Ival::pt(c[0]).sub(Ival::pt(r)).lo.min(a[0]).min(b[0]),
            Ival::pt(c[1]).sub(Ival::pt(r)).lo.min(a[1]).min(b[1]),
            Ival::pt(c[0]).add(Ival::pt(r)).hi.max(a[0]).max(b[0]),
            Ival::pt(c[1]).add(Ival::pt(r)).hi.max(a[1]).max(b[1]),
        ],
    }
}

fn sec_box(s: &Sec) -> [f64; 4] {
    match *s {
        Sec::Seg { a, b } => tseg_box(&TSeg::Line { a, b }),
        Sec::Arc { c, r, a, b, ccw } => tseg_box(&TSeg::Arc { a, b, c, r, ccw }),
    }
}

/// Trim.lean's Piece.ok, proven in outward-rounded arithmetic: r > 0, neither end at the centre,
/// and the sweep strictly under a half turn in the arc's sense.
fn arc_ok(a: [f64; 2], b: [f64; 2], c: [f64; 2], r: f64, ccw: bool) -> bool {
    let (ua, ub) = (sub2(a, c), sub2(b, c));
    let cr = cross(ua, ub);
    let off = |u: [Ival; 2]| dot(u, u).lo > 0.0;
    r > 0.0 && off(ua) && off(ub) && if ccw { cr.lo > 0.0 } else { cr.hi < 0.0 }
}

/// An arc piece's stored ends may lie off its circle by `gap` (the trim is the circle's arc plus
/// the radial pieces to them): the end candidates may then exceed the truth by gap, and the
/// radial pieces lie up to gap nearer, so the low end comes down by twice the gap (never below
/// 0: the closed form needs ρ ≥ 0). The high end stays: it is realised at real points.
fn less_gap(d: Ival, gap: f64) -> Ival {
    if gap == 0.0 {
        return d;
    }
    Ival {
        lo: Ival::pt(d.lo).sub(Ival::pt(2.0 * gap)).lo.max(0.0),
        hi: d.hi,
    }
}

/// A lower bound on the distance between two boxes.
fn box_gap(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let gx = Ival::pt(b[0])
        .sub(Ival::pt(a[2]))
        .lo
        .max(Ival::pt(a[0]).sub(Ival::pt(b[2])).lo)
        .max(0.0);
    let gy = Ival::pt(b[1])
        .sub(Ival::pt(a[3]))
        .lo
        .max(Ival::pt(a[1]).sub(Ival::pt(b[3])).lo)
        .max(0.0);
    Ival::pt(gx).sqr().add(Ival::pt(gy).sqr()).sqrt().lo
}

/// The least distance from a path curve to a flat region (0 if they meet). Pieces whose box is
/// farther than `cutoff` count by that box bound alone (a lower bound; the distance only
/// matters below the tool's reach).
fn dist_region(path: &Sec, region: &[TSeg], boxes: &[[f64; 4]], gaps: &[f64], cutoff: f64) -> Ival {
    let mut lo = f64::INFINITY;
    let mut hi = f64::INFINITY;
    let pb = sec_box(path);
    // the region's box (its pieces' boxes hold it)
    let mut rb = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let mut i = 0;
    while i < region.len() {
        let b = &boxes[i];
        rb = [rb[0].min(b[0]), rb[1].min(b[1]), rb[2].max(b[2]), rb[3].max(b[3])];
        let g = box_gap(&pb, &boxes[i]);
        if g > cutoff {
            if g < lo {
                lo = g;
            }
            i += 1;
            continue;
        }
        // its distance is at least g ≥ hi ≥ lo: it changes neither end
        if g >= hi {
            i += 1;
            continue;
        }
        let s = match region[i] {
            TSeg::Line { a, b } => Sec::Seg { a, b },
            TSeg::Arc { a, b, c, r, ccw } => Sec::Arc { c, r, a, b, ccw },
        };
        let (gap, ok) = (gaps[i].max(0.0), gaps[i] >= 0.0);
        // an arc piece outside the proven candidates' hypothesis (Trim.lean's Piece.ok) gives
        // no lower bound: 0
        let d = if ok {
            less_gap(dist(path, &s), gap)
        } else {
            Ival { lo: 0.0, hi: f64::INFINITY }
        };
        if d.lo < lo {
            lo = d.lo;
        }
        if d.hi < hi {
            hi = d.hi;
        }
        i += 1;
    }
    // inside: the path's start is a face point at distance 0 (outside the region's box it is
    // outside the region, and the ray cast is not needed)
    let (a, _) = ends(path);
    if a[0] < rb[0] || a[0] > rb[2] || a[1] < rb[1] || a[1] > rb[3] {
        return Ival { lo, hi };
    }
    let tr = crate::check::TrimRef {
        segs: region,
        gaps: Some(gaps),
    };
    let side = tr.classify(Ival::pt(a[0]), Ival::pt(a[1]));
    if side > 0 {
        return Ival::pt(0.0);
    }
    if side == 0 {
        lo = 0.0;
    }
    Ival { lo, hi }
}

impl Prism {
    /// The prism form of a lowered face, if it has one.
    pub fn of(f: &Face) -> Option<Prism> {
        match f.surf {
            Surf::Plane { o, u, v } => {
                if u[2] == 0.0 && v[2] == 0.0 {
                    // horizontal: the trim mapped to xy
                    let t = f.trim.as_ref()?;
                    let map =
                        |q: [f64; 2]| [o[0] + q[0] * u[0] + q[1] * v[0], o[1] + q[0] * u[1] + q[1] * v[1]];
                    let flip = u[0] * v[1] - u[1] * v[0] < 0.0;
                    let mut region = Vec::with_capacity(t.segs.len());
                    for s in &t.segs {
                        region.push(match *s {
                            TSeg::Line { a, b } => TSeg::Line { a: map(a), b: map(b) },
                            TSeg::Arc { a, b, c, r, ccw } => TSeg::Arc {
                                a: map(a),
                                b: map(b),
                                c: map(c),
                                r,
                                ccw: ccw != flip,
                            },
                        });
                    }
                    let boxes = region.iter().map(tseg_box).collect();
                    let gaps = region
                        .iter()
                        .map(|t| match *t {
                            TSeg::Line { .. } => 0.0,
                            TSeg::Arc { a, b, c, r, ccw } => {
                                if arc_ok(a, b, c, r, ccw) {
                                    crate::check::arc_gap(a, b, c, r)
                                } else {
                                    -1.0
                                }
                            }
                        })
                        .collect();
                    return Some(Prism::Flat {
                        z: o[2],
                        region,
                        boxes,
                        gaps,
                    });
                }
                // vertical, u horizontal and v along ±z, over a rectangular trim
                if u[2] == 0.0
                    && v[0] == 0.0
                    && v[1] == 0.0
                    && (v[2] == 1.0 || v[2] == -1.0)
                    && rectangular(f)
                {
                    // the trim's own corners (the vertices), not the padded box
                    let (mut p0, mut p1, mut q0, mut q1) = (f.p.lo, f.p.hi, f.q.lo, f.q.hi);
                    if let Some(t) = &f.trim {
                        let (mut a0, mut a1, mut b0, mut b1) =
                            (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
                        for sg in &t.segs {
                            if let TSeg::Line { a, b } = *sg {
                                for pt in [a, b] {
                                    a0 = a0.min(pt[0]);
                                    a1 = a1.max(pt[0]);
                                    b0 = b0.min(pt[1]);
                                    b1 = b1.max(pt[1]);
                                }
                            }
                        }
                        (p0, p1, q0, q1) = (a0, a1, b0, b1);
                    }
                    let (q0, q1) = (o[2] + v[2] * q0, o[2] + v[2] * q1);
                    let a = [o[0] + p0 * u[0], o[1] + p0 * u[1]];
                    let b = [o[0] + p1 * u[0], o[1] + p1 * u[1]];
                    return Some(Prism::Wall {
                        sec: Sec::Seg { a, b },
                        z0: q0.min(q1),
                        z1: q0.max(q1),
                        gap: 0.0,
                    });
                }
                None
            }
            Surf::Cyl { o, x, y, ax, r } => {
                if !(ax[0] == 0.0
                    && ax[1] == 0.0
                    && (ax[2] == 1.0 || ax[2] == -1.0)
                    && x[2] == 0.0
                    && y[2] == 0.0)
                {
                    return None;
                }
                let at = |t: f64| {
                    let (c, s) = ((1.0 - t * t) / (1.0 + t * t), 2.0 * t / (1.0 + t * t));
                    [o[0] + r * (c * x[0] + s * y[0]), o[1] + r * (c * x[1] + s * y[1])]
                };
                let ccw = x[0] * y[1] - x[1] * y[0] > 0.0;
                let (q0, q1) = (o[2] + ax[2] * f.q.lo, o[2] + ax[2] * f.q.hi);
                let (a, b) = (at(f.p.lo), at(f.p.hi));
                // the closed form's arc candidates hold for arcs strictly under a half turn
                // (DistArc.lean); a wider wall is left to the general branch and bound
                // the sign proven (outward-rounded), else not a prism: the guard never errs
                let cr = cross(sub2(a, [o[0], o[1]]), sub2(b, [o[0], o[1]]));
                if !(if ccw { cr.lo > 0.0 } else { cr.hi < 0.0 }) {
                    return None;
                }
                Some(Prism::Wall {
                    sec: Sec::Arc {
                        c: [o[0], o[1]],
                        r,
                        a,
                        b,
                        ccw,
                    },
                    z0: q0.min(q1),
                    z1: q0.max(q1),
                    gap: crate::check::arc_gap(a, b, [o[0], o[1]], r),
                })
            }
            _ => None,
        }
    }
}

/// Whether a planar face's trim is exactly its parameter box (four lines along the axes, or none).
fn rectangular(f: &Face) -> bool {
    match &f.trim {
        None => true,
        Some(t) => {
            if t.segs.len() != 4 {
                return false;
            }
            let mut i = 0;
            while i < 4 {
                match t.segs[i] {
                    TSeg::Line { a, b } => {
                        if a[0] != b[0] && a[1] != b[1] {
                            return false;
                        }
                    }
                    _ => return false,
                }
                i += 1;
            }
            true
        }
    }
}

/// The path's xy curve and the tip's height range.
fn path_sec(pl: &Planar) -> (Sec, Ival) {
    match *pl {
        Planar::Line { a, b, z } => (Sec::Seg { a, b }, Ival::pt(z)),
        Planar::Arc { c, r, a, b, ccw, z } => (Sec::Arc { c, r, a, b, ccw }, Ival::pt(z)),
        Planar::Helix {
            c,
            r,
            a,
            b,
            ccw,
            z0,
            z1,
        } => (Sec::Arc { c, r, a, b, ccw }, Ival::new(z0, z1)),
        Planar::Vertical { p, z0, z1 } => (Sec::Seg { a: p, b: p }, Ival::new(z0, z1)),
    }
}

/// Where along the move a reported violation is (sampled: a report, nothing proven by it), and
/// the tip there.
pub fn locate(mv: &crate::check::Move, pr: &Prism) -> (f64, [f64; 3]) {
    let mut best = (f64::INFINITY, 0.0, [0.0; 3]);
    let mut k = 0;
    while k <= 64 {
        let s = k as f64 / 64.0;
        let g = mv.at(Ival::pt(s));
        let q = [g[0].mid(), g[1].mid()];
        let d = match pr {
            Prism::Wall { sec, .. } => pt_sec(q, sec).mid(),
            Prism::Flat { region, .. } => {
                let mut m = f64::INFINITY;
                for t in region {
                    let sc = match *t {
                        TSeg::Line { a, b } => Sec::Seg { a, b },
                        TSeg::Arc { a, b, c, r, ccw } => Sec::Arc { c, r, a, b, ccw },
                    };
                    m = m.min(pt_sec(q, &sc).mid());
                }
                m
            }
        };
        if d < best.0 {
            best = (d, s, [g[0].mid(), g[1].mid(), g[2].mid()]);
        }
        k += 1;
    }
    (best.1, best.2)
}

/// The verdict for a planar move against a prism face (module doc).
pub fn certify(
    body: &Body,
    f: &Face,
    pr: &Prism,
    pl: &Planar,
    limit: f64,
    res: f64,
    budget: usize,
) -> (Verdict, usize) {
    certify_shared(body, f, pr, pl, limit, res, budget, &mut None)
}

/// `certify`, the move's distance to the face's section shared between calls for the same move
/// and face (the bodies of one tool): `shared` holds it once computed, with the cutoff it was
/// computed to (segments beyond it count only by their box gap); a body that needs a larger
/// cutoff computes it again.
#[allow(clippy::too_many_arguments)]
pub fn certify_shared(
    body: &Body,
    f: &Face,
    pr: &Prism,
    pl: &Planar,
    limit: f64,
    res: f64,
    budget: usize,
    shared: &mut Option<(Ival, f64)>,
) -> (Verdict, usize) {
    let (path, zt) = path_sec(pl);
    let (lim_clear, lim_hit) = (limit + f.mu, limit - f.mu);
    let (d, dz) = match pr {
        Prism::Flat { z, region, boxes, gaps } => {
            let dz = Ival::pt(*z).sub(zt);
            // the face is wholly beyond the body's heights by more than the limit: no distance
            // needed (sd at ρ = 0 is already a lower bound for every ρ)
            let lb0 = body.sd_rz(Ival::pt(0.0), dz).lo;
            if lb0 >= lim_clear {
                return (Verdict::Clear { lower: lb0 - f.mu }, 1);
            }
            let cutoff = Ival::pt(body.rmax)
                .add(Ival::pt(limit.abs()))
                .add(Ival::pt(f.mu))
                .hi;
            let d = match *shared {
                Some((d, cut)) if cut >= cutoff => d,
                _ => {
                    let d = dist_region(&path, region, boxes, gaps, cutoff);
                    *shared = Some((d, cutoff));
                    d
                }
            };
            (d, dz)
        }
        Prism::Wall { sec, z0, z1, gap } => {
            let d = match *shared {
                Some((d, _)) => d,
                None => {
                    // a wall's arc ends are its parametrisation's, evaluated: off the circle by
                    // their rounding, accounted as for trims
                    let d = less_gap(dist(&path, sec), *gap);
                    *shared = Some((d, f64::INFINITY));
                    d
                }
            };
            (d, Ival::new(Ival::pt(*z0).sub(zt).lo, Ival::pt(*z1).sub(zt).hi))
        }
    };
    // the one-dimensional search over the height offset Δ
    let lo_rho = Ival::pt(d.lo);
    let hi_rho = Ival::pt(d.hi);
    let mut stack = vec![dz];
    let mut cells = 0usize;
    let mut lower = f64::INFINITY;
    let mut undecided = false;
    while let Some(c) = stack.pop() {
        cells += 1;
        if cells > budget {
            return (
                Verdict::Undecided {
                    lower: f64::NEG_INFINITY,
                    cells,
                },
                cells,
            );
        }
        let lb = body.sd_rz(lo_rho, c).lo;
        if lb >= lim_clear {
            lower = lower.min(lb);
            continue;
        }
        if d.hi.is_finite() {
            let m = Ival::pt(c.mid());
            let w = body.sd_rz(hi_rho, m).hi;
            if w < lim_hit {
                return (
                    Verdict::Hit {
                        d: w + f.mu,
                        p: d.hi,
                        q: c.mid(),
                        s: 0.5,
                        at: [f64::NAN; 3],
                    },
                    cells,
                );
            }
        }
        if c.width() <= res {
            undecided = true;
            lower = lower.min(lb);
            continue;
        }
        let (a, b) = c.split();
        stack.push(a);
        stack.push(b);
    }
    if undecided {
        return (Verdict::Undecided { lower, cells }, cells);
    }
    (Verdict::Clear { lower: lower - f.mu }, cells)
}

#[cfg(test)]
mod gap_tests {
    use super::*;

    /// A trim arc whose stored ends lie off its circle (by an exaggerated 1e-3 here, ~1e-11 in
    /// practice): the trim is the circle's arc plus the radial pieces to the ends. From points
    /// all around, dist_region's low end must not exceed the distance to that trim (sampled
    /// densely: the samples' least is at least the truth).
    #[test]
    fn an_arc_with_ends_off_its_circle_never_overstates_the_distance() {
        let (c, r, g) = ([0.0, 0.0], 1.0, 1e-3);
        // ends pushed in and out along their radii, a quarter turn ccw
        let a = [r - g, 0.0];
        let b = [0.0, r + g];
        let region = vec![
            TSeg::Arc { a, b, c, r, ccw: true },
            TSeg::Line { a: b, b: [-2.0, 2.0] },
            TSeg::Line { a: [-2.0, 2.0], b: [2.0, -2.0] },
            TSeg::Line { a: [2.0, -2.0], b: a },
        ];
        let boxes: Vec<[f64; 4]> = region.iter().map(tseg_box).collect();
        let gaps: Vec<f64> = region
            .iter()
            .map(|t| match *t {
                TSeg::Line { .. } => 0.0,
                TSeg::Arc { a, b, c, r, .. } => crate::check::arc_gap(a, b, c, r),
            })
            .collect();
        // the trim's points: the arc on its circle, the radial pieces, the lines
        let mut pts: Vec<[f64; 2]> = vec![];
        for k in 0..=4000 {
            let t = std::f64::consts::FRAC_PI_2 * k as f64 / 4000.0;
            pts.push([r * t.cos(), r * t.sin()]);
        }
        for k in 0..=100 {
            let u = k as f64 / 100.0;
            pts.push([r - g * u, 0.0]);
            pts.push([0.0, r + g * u]);
        }
        let mut seed = 0x2545f4914f6cdd1du64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        for _ in 0..2000 {
            // points near the arc's ends, where the end candidates matter
            let e = if rnd() < 0.5 { a } else { b };
            let q = [e[0] + (rnd() - 0.5) * 0.02, e[1] + (rnd() - 0.5) * 0.02];
            let d = dist_region(&Sec::Seg { a: q, b: q }, &region, &boxes, &gaps, 10.0);
            // the truth, from the samples (each a real trim point: the true least is ≤ theirs,
            // so compare against their least less the samples' spacing)
            let mut m = f64::INFINITY;
            for p in &pts {
                m = m.min(((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt());
            }
            let spacing = std::f64::consts::FRAC_PI_2 / 4000.0 + g / 100.0;
            assert!(d.lo <= m + spacing, "q {q:?}: lo {} vs trim {m}", d.lo);
        }
    }
}
