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
//!   edge break's corner) that is again the exact distance in the section;
//! - the torus with a vertical axis (a fillet round a boss or a hole) by the same inequality:
//!   its radius from the axis and its height over a cell of the tube's parameter. An arc about
//!   the axis is again exact in the section, and an arc whose centre is ε off the axis (a
//!   posted arc, its centre from rounded words) loses only ε: its distances from the axis are
//!   within r ± ε.
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
    /// Radius |big + r cos φ| · |c x + s y| (`rf` the second factor), height o.z + az r sin φ,
    /// φ = φm + 2 atan q over a cell of q.
    Torus {
        axis: [f64; 2],
        oz: f64,
        az: f64,
        big: f64,
        r: f64,
        cm: f64,
        sm: f64,
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
        Surf::Torus {
            o,
            x,
            y,
            ax,
            big,
            r,
            cm,
            sm,
        } => {
            if !(ax[0] == 0.0
                && ax[1] == 0.0
                && (ax[2] == 1.0 || ax[2] == -1.0)
                && x[2] == 0.0
                && y[2] == 0.0
                && big.is_finite()
                && r.is_finite()
                && cm.is_finite()
                && sm.is_finite())
            {
                return None;
            }
            let rf = horizontal_factor(x, y)?;
            Some((
                Kind::Torus {
                    axis: [o[0], o[1]],
                    oz: o[2],
                    az: ax[2],
                    big,
                    r,
                    cm,
                    sm,
                    rf,
                },
                f.q,
            ))
        }
        _ => None,
    }
}

/// An enclosure of |c x + s y| over every angle (c² + s² = 1), x and y horizontal: from
/// c²|x|² + s²|y|² + 2cs x·y with 2|cs| ≤ 1. None unless it is bounded away from 0.
fn horizontal_factor(x: [f64; 3], y: [f64; 3]) -> Option<Ival> {
    let xx = p(x[0]).sqr().add(p(x[1]).sqr());
    let yy = p(y[0]).sqr().add(p(y[1]).sqr());
    let xy = p(x[0]).mul(p(y[0])).add(p(x[1]).mul(p(y[1]))).abs();
    let lo = Ival::pt(xx.lo.min(yy.lo)).sub(Ival::pt(xy.hi)).lo;
    let hi = Ival::pt(xx.hi.max(yy.hi)).add(Ival::pt(xy.hi)).hi;
    if !(lo > 0.0 && hi.is_finite()) {
        return None;
    }
    Some(Ival::new(lo, hi).sqrt())
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
    // a plane face lies in its plane: a tool that stays in {N·x − c ≥ D} through the whole move
    // has signed distance ≥ D from every point of the face (Halfspace.lean, either side), in
    // one cell
    if let Kind::Plane { o, e, g, .. } = &kd
        && let Some(d) = halfspace_bound(body, mv, Ival::new(0.0, 1.0), *o, *e, *g)
        && d >= lim_clear
    {
        return (Some(d - f.mu), 1);
    }
    // a plane's unit direction along e
    let ev = match &kd {
        // n = (−e.y, e.x)/|e|, so e/|e| = (n.y, −n.x)
        Kind::Plane { n, .. } => [n[1], n[0].neg()],
        Kind::Cone { .. } | Kind::Torus { .. } => [p(0.0), p(0.0)],
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
            Kind::Cone { axis, .. } | Kind::Torus { axis, .. } => {
                // the path arc's stored ends: lowered by Move::path_gap
                let dmin = Ival::pt(pt_sec(*axis, &sec).lo).sub(p(mv.path_gap())).lo.max(0.0);
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
            Kind::Cone { axis, .. } | Kind::Torus { axis, .. } => Piece {
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
            Kind::Torus {
                oz,
                az,
                big,
                r,
                cm,
                sm,
                rf,
                ..
            } => {
                // cos φ, sin φ over the cell of q (φ = φm + ψ, q = tan(ψ/2))
                let (cp, sp) = crate::check::circ(t);
                let cf = cp.mul(p(*cm)).sub(sp.mul(p(*sm)));
                let sf = cp.mul(p(*sm)).add(sp.mul(p(*cm)));
                let rad = p(*big).add(p(*r).mul(cf)).abs().mul(*rf);
                let dz = p(*oz).add(p(*az).mul(p(*r)).mul(sf)).sub(pc.zt);
                (gap(rad, pc.w).unwrap_or(0.0), dz)
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
        // a point moves at most 2 r per unit of q (dψ/dq = 2 / (1 + q²))
        Kind::Torus { r, rf, .. } => (2.0 * r.abs() * rf.hi).max(1.0) * 2.0,
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
        // a plane: the move's piece on one side of it, for the whole face at once
        if let Kind::Plane { o, e, g, .. } = &kd
            && let Some(d) = halfspace_bound(body, mv, s, *o, *e, *g)
            && d >= lim_clear
        {
            lower = lower.min(d);
            continue;
        }
        let (ts, ss) = (t.width() * scale, size(s));
        if ts <= res && ss <= res {
            return (None, cells);
        }
        // the face first, against the whole move taken exactly; the move is split only under
        // a face cell already at the resolution (a plane: the larger first, so the bound for a
        // piece of the move on one side of the plane gets its chance)
        let plane = matches!(kd, Kind::Plane { .. });
        if ts > res && !(plane && ss > ts) {
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

/// The least of N·x − c over the moved tool (the move over `sr` of its parameter), N the plane's unit normal (either sense: the
/// larger bound), c = N·o: a lower bound, outward-rounded. Over the move by the box of its tip
/// positions; over the body by its (ρ, z) profile (a point at radius ρ and height z above the
/// tip reaches N·(ρ cos φ, ρ sin φ, z) ≥ −ρ|N_xy| + z N_z; an arc piece by its centre less its
/// radius). None where the plane's normal cannot be formed.
fn halfspace_bound(body: &Body, mv: &Move, sr: Ival, o: [f64; 3], e: [f64; 3], g: [f64; 3]) -> Option<f64> {
    let n = [
        p(e[1]).mul(p(g[2])).sub(p(e[2]).mul(p(g[1]))),
        p(e[2]).mul(p(g[0])).sub(p(e[0]).mul(p(g[2]))),
        p(e[0]).mul(p(g[1])).sub(p(e[1]).mul(p(g[0]))),
    ];
    let len = n[0].sqr().add(n[1].sqr()).add(n[2].sqr()).sqrt();
    if !(len.lo > 0.0) {
        return None;
    }
    let u = [n[0].div(len), n[1].div(len), n[2].div(len)];
    let nh = u[0].sqr().add(u[1].sqr()).sqrt();
    let unit = nh.sqr().add(u[2].sqr()).sqrt();
    let c = u[0].mul(p(o[0])).add(u[1].mul(p(o[1]))).add(u[2].mul(p(o[2])));
    // N·A over the move: a line's at its ends (linear in s); an arc's or helix's by its centre
    // less the radius times N's horizontal part, over its heights
    let dotn = |b: [Ival; 3]| u[0].mul(b[0]).add(u[1].mul(b[1])).add(u[2].mul(b[2]));
    let na = match *mv {
        Move::Line { .. } => dotn(mv.at(Ival::pt(sr.lo))).hull(dotn(mv.at(Ival::pt(sr.hi)))),
        Move::Arc { c, r, .. } | Move::Helix { c, r, .. } => {
            // the horizontal part: its least is at an end unless the radial direction against
            // N's horizontal part, −N_xy, lies within the arc's turn (pieces are under a half
            // turn, counter-clockwise or not); then the circle's least, centre − r|N_xy|
            let (a0, a1) = (mv.at(Ival::pt(sr.lo)), mv.at(Ival::pt(sr.hi)));
            let dxy = |b: [Ival; 3]| u[0].mul(b[0]).add(u[1].mul(b[1]));
            let ends = dxy(a0).hull(dxy(a1));
            let (r0, r1) = (
                [a0[0].sub(p(c[0])), a0[1].sub(p(c[1]))],
                [a1[0].sub(p(c[0])), a1[1].sub(p(c[1]))],
            );
            let cr = |a: [Ival; 2], b: [Ival; 2]| a[0].mul(b[1]).sub(a[1].mul(b[0]));
            // the turn's sense from its ends (under a half turn: cross(r0, r1)'s sign)
            let turn = cr(r0, r1);
            // whether the radial direction m may lie within the arc's turn
            let within = |m: [Ival; 2]| {
                if turn.lo > 0.0 {
                    !(cr(r0, m).hi < 0.0 || cr(m, r1).hi < 0.0)
                } else if turn.hi < 0.0 {
                    !(cr(r0, m).lo > 0.0 || cr(m, r1).lo > 0.0)
                } else {
                    true
                }
            };
            // the least at −N_xy's direction, the greatest at N_xy's, if within the turn
            let h = u[0].mul(p(c[0])).add(u[1].mul(p(c[1])));
            let rr = p(r).mul(nh);
            let lo = if within([u[0].neg(), u[1].neg()]) {
                h.sub(rr).lo.min(ends.lo)
            } else {
                ends.lo
            };
            let hi = if within([u[0], u[1]]) {
                h.add(rr).hi.max(ends.hi)
            } else {
                ends.hi
            };
            let hx = Ival::new(lo, hi);
            let zs = mv.at(sr)[2];
            hx.add(u[2].mul(zs))
        }
    };
    let mut best = f64::NEG_INFINITY;
    for sense in [1.0, -1.0] {
        let nz = u[2].scale(sense);
        // the profile's least −ρ nh + z nz
        let lin = |q: [f64; 2]| p(q[0]).mul(nh).neg().add(p(q[1]).mul(nz));
        let mut m = lin([0.0, body.z0]).lo.min(lin([0.0, body.z1]).lo);
        for sg in &body.chain {
            match *sg {
                crate::tool::PSeg::Line { a, b } => {
                    m = m.min(lin(a).lo).min(lin(b).lo);
                }
                crate::tool::PSeg::Arc { c, r, .. } => {
                    m = m.min(lin(c).sub(p(r).mul(unit)).lo);
                }
            }
        }
        let d = na.scale(sense).add(p(m)).sub(c.scale(sense));
        if d.lo.is_finite() {
            best = best.max(d.lo);
        }
    }
    if best.is_finite() { Some(best) } else { None }
}

#[cfg(test)]
mod halfspace_tests {
    use super::*;
    use crate::tool::{Cutter, PSeg, assemble};

    /// A unit vector from a parameter (the rational map: no libm, as on every target).
    fn cs(t: f64) -> (f64, f64) {
        let w = 1.0 + t * t;
        ((1.0 - t * t) / w, 2.0 * t / w)
    }

    struct Rng(u64);
    impl Rng {
        fn f(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
        fn r(&mut self, a: f64, b: f64) -> f64 {
            a + (b - a) * self.f()
        }
    }

    /// The bound is at most the sampled least of N·x − c over the moved tool, for the better of
    /// the two senses (that sense's points all satisfy it, which is what the clearance rests on):
    /// random tilted planes, line and arc moves (whole and in pieces), flat, ball and chamfer
    /// tools, sampled densely over the profile, the turn and the move.
    #[test]
    fn the_halfspace_bound_is_below_the_samples() {
        let mut g = Rng(0x2545f4914f6cdd1d);
        let tools = [
            assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
            assemble(&Cutter::Ball { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
            assemble(
                &Cutter::Chamfer {
                    d_tip: 1.0,
                    angle: 90.0,
                    d: 10.0,
                },
                8.0,
                10.0,
                25.0,
                None,
            )
            .unwrap(),
        ];
        let mut checked = 0;
        for case in 0..300 {
            let body = &tools[case % 3].bodies[0];
            // a plane through o, spanned by a horizontal e and a sloped g
            let (tc, ts) = cs(g.r(-4.0, 4.0));
            let e = [tc, ts, 0.0];
            let (sc, ss) = cs(g.r(0.1, 0.9));
            let gg = [-ts * sc, tc * sc, -ss];
            let o = [g.r(-5.0, 5.0), g.r(-5.0, 5.0), g.r(-3.0, 3.0)];
            let z = g.r(-4.0, 4.0);
            let mv = if case % 2 == 0 {
                Move::Line {
                    a: [g.r(-8.0, 8.0), g.r(-8.0, 8.0), z],
                    b: [g.r(-8.0, 8.0), g.r(-8.0, 8.0), z + g.r(-1.0, 1.0)],
                }
            } else {
                let (pc, ps) = cs(g.r(-4.0, 4.0));
                Move::Arc {
                    c: [g.r(-6.0, 6.0), g.r(-6.0, 6.0), z],
                    x: [pc, ps, 0.0],
                    y: [-ps, pc, 0.0],
                    r: g.r(1.0, 6.0),
                    t0: -g.r(0.05, 0.4),
                    t1: g.r(0.05, 0.4),
                }
            };
            for sr in [Ival::new(0.0, 1.0), Ival::new(0.0, 0.5), Ival::new(0.25, 0.75)] {
                let Some(d) = halfspace_bound(body, &mv, sr, o, e, gg) else {
                    continue;
                };
                // the unit normal, both senses' sampled least
                let n = [
                    e[1] * gg[2] - e[2] * gg[1],
                    e[2] * gg[0] - e[0] * gg[2],
                    e[0] * gg[1] - e[1] * gg[0],
                ];
                let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                let n = [n[0] / l, n[1] / l, n[2] / l];
                let c = n[0] * o[0] + n[1] * o[1] + n[2] * o[2];
                let (mut lo_p, mut lo_m) = (f64::INFINITY, f64::INFINITY);
                for k in 0..=40 {
                    let s = sr.lo + (sr.hi - sr.lo) * k as f64 / 40.0;
                    let a = mv.at(Ival::pt(s));
                    let a = [a[0].mid(), a[1].mid(), a[2].mid()];
                    for sg in &body.chain {
                        let (pa, pb) = match *sg {
                            PSeg::Line { a, b } | PSeg::Arc { a, b, .. } => (a, b),
                        };
                        for q in [pa, pb, [0.5 * (pa[0] + pb[0]), 0.5 * (pa[1] + pb[1])]] {
                            for j in 0..36 {
                                // round the axis: the rational map over t in [-4, 4], and its mirror
                                let (pc, ps) = cs(-4.0 + 8.0 * (j / 2) as f64 / 17.0);
                                let pc = if j % 2 == 0 { pc } else { -pc };
                                let x = [a[0] + q[0] * pc, a[1] + q[0] * ps, a[2] + q[1]];
                                let v = n[0] * x[0] + n[1] * x[1] + n[2] * x[2] - c;
                                lo_p = lo_p.min(v);
                                lo_m = lo_m.min(-v);
                            }
                        }
                    }
                }
                assert!(d <= lo_p.max(lo_m) + 1e-9, "case {case}: {d} vs {lo_p} / {lo_m}");
                checked += 1;
            }
        }
        assert!(checked > 600, "{checked}");
    }
}
