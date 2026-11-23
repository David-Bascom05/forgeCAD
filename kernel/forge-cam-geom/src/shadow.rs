//! Closed forms by symmetry, for two kinds of face that are not prisms along z: a plane with a
//! horizontal direction e in it (a 45° chamfer, a draft), and a cone with a vertical axis (a
//! chamfer round an arc, a countersink, a drill point).
//!
//! The checker's quantity is the least, over tool positions A on the move and points P of the
//! face, of the body's signed distance sd_rz(|P − A| horizontally, P.z − A.z), which is
//! non-decreasing in its first argument (tool.rs). So any lower bound h on |P − A| over a cell
//! of P and a piece of the move gives sd_rz(h, Δz) as a lower bound there. The bounds:
//!
//! - the plane: in the horizontal frame (n across e, e along it), the face's points over a cell
//!   of its sloped parameter t lie in a box (all of its extent along e, the cell's across), the
//!   move's in another; h = √(gap across² + gap along²) between the two. Across e the face
//!   depends on t alone, so when the move runs along e (an edge break's line), the bound is the
//!   exact distance in the section across the edge;
//! - the cone: ||P − o| − |A − o|| ≤ |P − A|, o its axis; the face's radius over a cell of t
//!   against the move's distances from the axis. When the move is an arc about the axis (an
//!   edge break's corner) that is again the exact distance in the section.
//!
//! The whole move is taken exactly (its line or arc), a piece of it by the box `Move::at`
//! encloses it in. The search splits the face's t to the resolution against the whole move,
//! and splits the move only under a face cell at the resolution. It only ever proves a
//! clearance: whatever it cannot prove goes to the branch and bound (check::certify_bb), which
//! decides as before. It is tight when the move follows the face's symmetry; elsewhere it is a
//! cheap first try.
use crate::check::{Face, Move, Planar, Surf};
use crate::ival::Ival;
use crate::slab::{Sec, pt_sec};
use crate::tool::Body;

fn p(x: f64) -> Ival {
    Ival::pt(x)
}

/// The face's form for this bound, if it has one: (offset of a cell's points, given the cell of
/// the sloped parameter) is computed per kind.
enum Kind {
    /// o + P e + t g, P over its box, e horizontal; n the unit horizontal normal to e.
    Plane {
        o: [f64; 3],
        e: [f64; 3],
        g: [f64; 3],
        pbox: Ival,
        n: [Ival; 2],
    },
    /// Radius |r0 + t k| · |c x + s y| (`rf` encloses the second factor), height o.z + t az.
    Cone {
        axis: [f64; 2],
        oz: f64,
        az: f64,
        r0: f64,
        k: f64,
        rf: Ival,
    },
}

fn kind(f: &Face) -> Option<(Kind, Ival)> {
    match f.surf {
        Surf::Plane { o, u, v } => {
            // the sloped parameter is the one along the direction that is not horizontal
            let (e, g, pbox, t) = if u[2] == 0.0 && v[2] != 0.0 {
                (u, v, f.p, f.q)
            } else if v[2] == 0.0 && u[2] != 0.0 {
                (v, u, f.q, f.p)
            } else {
                return None;
            };
            let ne = p(e[0]).sqr().add(p(e[1]).sqr()).sqrt();
            if !(ne.lo > 0.0) {
                return None;
            }
            let n = [p(-e[1]).div(ne), p(e[0]).div(ne)];
            Some((Kind::Plane { o, e, g, pbox, n }, t))
        }
        Surf::Cone { o, x, y, ax, r0, k } => {
            if !(ax[0] == 0.0
                && ax[1] == 0.0
                && (ax[2] == 1.0 || ax[2] == -1.0)
                && x[2] == 0.0
                && y[2] == 0.0)
            {
                return None;
            }
            // |c x + s y|² = c²|x|² + s²|y|² + 2cs x·y with c² + s² = 1, 2|cs| ≤ 1
            let xx = p(x[0]).sqr().add(p(x[1]).sqr());
            let yy = p(y[0]).sqr().add(p(y[1]).sqr());
            let xy = p(x[0]).mul(p(y[0])).add(p(x[1]).mul(p(y[1]))).abs();
            let lo = Ival::pt(xx.lo.min(yy.lo)).sub(Ival::pt(xy.hi)).lo;
            let hi = Ival::pt(xx.hi.max(yy.hi)).add(Ival::pt(xy.hi)).hi;
            if !(lo > 0.0) {
                return None;
            }
            let rf = Ival::new(lo, hi).sqrt();
            Some((
                Kind::Cone {
                    axis: [o[0], o[1]],
                    oz: o[2],
                    az: ax[2],
                    r0,
                    k,
                    rf,
                },
                f.q,
            ))
        }
        _ => None,
    }
}

/// The path's xy curve and the tip's heights.
fn path(pl: &Planar) -> (Sec, Ival) {
    match *pl {
        Planar::Line { a, b, z } => (Sec::Seg { a, b }, p(z)),
        Planar::Arc { c, r, a, b, ccw, z } => (Sec::Arc { c, r, a, b, ccw }, p(z)),
        Planar::Helix {
            c,
            r,
            a,
            b,
            ccw,
            z0,
            z1,
        } => (Sec::Arc { c, r, a, b, ccw }, Ival::new(z0.min(z1), z0.max(z1))),
        Planar::Vertical { p: q, z0, z1 } => (Sec::Seg { a: q, b: q }, Ival::new(z0.min(z1), z0.max(z1))),
    }
}

/// What a piece of the path is, as the bounds need it: the intervals of its w and of its
/// coordinate along e (a plane), or of its distance from the axis (a cone), and the tip's heights.
struct Piece {
    w: Ival,
    along: Ival,
    zt: Ival,
}

fn dot2(q: [Ival; 2], n: [Ival; 2]) -> Ival {
    q[0].mul(n[0]).add(q[1].mul(n[1]))
}

/// The least distance between two intervals, or None where they meet.
fn gap(a: Ival, b: Ival) -> Option<f64> {
    if a.lo > b.hi {
        Some(a.sub(b).lo.max(0.0))
    } else if b.lo > a.hi {
        Some(b.sub(a).lo.max(0.0))
    } else {
        None
    }
}

/// A clearance proven by symmetry: the lower bound (less the face's mu) and the cells spent, or
/// None with the cells spent when it cannot be proven this way (the caller goes on to the
/// branch and bound). The search splits the face's sloped parameter and the move's: the whole
/// move is taken exactly (its curve), a part of it by the box `Move::at` encloses it in.
pub fn clear(
    body: &Body,
    f: &Face,
    mv: &Move,
    pl: &Planar,
    limit: f64,
    res: f64,
    budget: usize,
) -> (Option<f64>, usize) {
    let Some((kd, t0)) = kind(f) else {
        return (None, 0);
    };
    let lim_clear = limit + f.mu;
    // a plane's unit direction along e
    let ev = match &kd {
        // n = (−e.y, e.x)/|e|, so e/|e| = (n.y, −n.x)
        Kind::Plane { n, .. } => [n[1], n[0].neg()],
        Kind::Cone { .. } => [p(0.0), p(0.0)],
    };
    let whole = |pl: &Planar| -> Option<Piece> {
        let (sec, zt) = path(pl);
        let pc = match &kd {
            Kind::Plane { n, .. } => {
                let pr = |v: [Ival; 2]| match sec {
                    Sec::Seg { a, b } => dot2([p(a[0]), p(a[1])], v).hull(dot2([p(b[0]), p(b[1])], v)),
                    Sec::Arc { c, r, .. } => {
                        let h = dot2([p(c[0]), p(c[1])], v);
                        Ival::new(h.sub(p(r)).lo, h.add(p(r)).hi)
                    }
                };
                Piece {
                    w: pr(*n),
                    along: pr(ev),
                    zt,
                }
            }
            Kind::Cone { axis, .. } => {
                let dmin = pt_sec(*axis, &sec).lo.max(0.0);
                let far = |q: [f64; 2]| {
                    p(q[0])
                        .sub(p(axis[0]))
                        .sqr()
                        .add(p(q[1]).sub(p(axis[1])).sqr())
                        .sqrt()
                };
                let dmax = match sec {
                    Sec::Seg { a, b } => far(a).hull(far(b)).hi,
                    Sec::Arc { c, r, .. } => far(c).add(p(r)).hi,
                };
                if !(dmin <= dmax) {
                    return None;
                }
                Piece {
                    w: Ival::new(dmin, dmax),
                    along: p(0.0),
                    zt,
                }
            }
        };
        Some(pc)
    };
    let part = |s: Ival| -> Piece {
        let b = mv.at(s);
        match &kd {
            Kind::Plane { n, .. } => Piece {
                w: dot2([b[0], b[1]], *n),
                along: dot2([b[0], b[1]], ev),
                zt: b[2],
            },
            Kind::Cone { axis, .. } => Piece {
                w: b[0].sub(p(axis[0])).sqr().add(b[1].sub(p(axis[1])).sqr()).sqrt(),
                along: p(0.0),
                zt: b[2],
            },
        }
    };
    let Some(root) = whole(pl) else {
        return (None, 0);
    };
    if !(root.w.lo.is_finite() && root.w.hi.is_finite()) {
        return (None, 0);
    }
    // a lower bound for a cell of the face against a piece of the path: the least horizontal
    // distance between them (0 where they may meet), then sd_rz, non-decreasing in it
    let bound = |t: Ival, pc: &Piece| -> f64 {
        let (h, dz) = match &kd {
            Kind::Plane { o, e, g, pbox, n } => {
                let dz = p(o[2]).add(t.mul(p(g[2]))).sub(pc.zt);
                // the face cell's coordinates across e (w) and along it, against the path's
                let en = dot2([p(e[0]), p(e[1])], *n);
                let gn = dot2([p(g[0]), p(g[1])], *n);
                let w = dot2([p(o[0]), p(o[1])], *n).add(pbox.mul(en)).add(t.mul(gn));
                let ee = dot2([p(e[0]), p(e[1])], ev);
                let ge = dot2([p(g[0]), p(g[1])], ev);
                let a = dot2([p(o[0]), p(o[1])], ev).add(pbox.mul(ee)).add(t.mul(ge));
                let (gw, ga) = (gap(w, pc.w).unwrap_or(0.0), gap(a, pc.along).unwrap_or(0.0));
                (p(gw).sqr().add(p(ga).sqr()).sqrt().lo, dz)
            }
            Kind::Cone {
                oz, az, r0, k, rf, ..
            } => {
                // | |P − o| − |A − o| | ≤ |P − A| horizontally
                let r = p(*r0).add(t.mul(p(*k))).abs().mul(*rf);
                let dz = p(*oz).add(t.mul(p(*az))).sub(pc.zt);
                (gap(r, pc.w).unwrap_or(0.0), dz)
            }
        };
        body.sd_rz(Ival::pt(h.max(0.0)), dz).lo
    };
    // the image of a unit of t: the cell's size in length
    let scale = match &kd {
        Kind::Plane { g, .. } => p(g[0])
            .sqr()
            .add(p(g[1]).sqr())
            .add(p(g[2]).sqr())
            .sqrt()
            .hi
            .max(1.0),
        Kind::Cone { k, rf, .. } => (k.abs() * rf.hi).max(1.0) * 2.0,
    };
    let size = |s: Ival| -> f64 {
        let b = mv.at(s);
        b[0].width().max(b[1].width()).max(b[2].width())
    };
    let s_all = Ival::new(0.0, 1.0);
    let mut stack = vec![(s_all, t0)];
    let mut cells = 0usize;
    let mut lower = f64::INFINITY;
    while let Some((s, t)) = stack.pop() {
        cells += 1;
        if cells > budget {
            return (None, cells);
        }
        let lb = if s == s_all {
            bound(t, &root)
        } else {
            bound(t, &part(s))
        };
        if lb >= lim_clear {
            lower = lower.min(lb);
            continue;
        }
        let (ts, ss) = (t.width() * scale, size(s));
        if ts <= res && ss <= res {
            return (None, cells);
        }
        // the face first, against the whole move taken exactly; the move is split only under
        // a face cell already at the resolution
        if ts > res {
            let (a, b) = t.split();
            stack.push((s, a));
            stack.push((s, b));
        } else {
            let (a, b) = s.split();
            stack.push((a, t));
            stack.push((b, t));
        }
    }
    (Some(lower - f.mu), cells)
}
