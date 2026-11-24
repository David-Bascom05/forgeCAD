//! Closed-form clearance: a proven lower bound on the cutter's signed distance to a sphere,
//! cylinder or plane face over a whole line move, with no branch and bound.
//!
//! The statement. Let K be a capsule (the points within radius ρ of the axis segment
//! {(0, 0, t) : zc ≤ t ≤ z1}) that contains the body B. Then sd_B ≥ sd_K everywhere:
//! outside K, dist(x, B) ≥ dist(x, K); inside B ⊆ K, a ball about x inside B is inside K, so
//! dist(x, ∂K) ≥ dist(x, ∂B). And sd_K(x) = dist(x, core) − ρ. So over a face F and a line move
//! g(s) = a + s(b − a), s ∈ [0, 1]:
//!
//!   min sd_B(y − g(s)) ≥ dist(F, P) − ρ,   P = {g(s) + (0, 0, t) : s ∈ [0, 1], t ∈ [zc, z1]},
//!
//! a parallelogram (a segment for a vertical move). A ball cutter's body is exactly such a
//! capsule's lower part (the ball about (0, 0, R), radius R, then the flutes' cylinder), so the
//! bound is tight where the ball touches. `capsule` accepts any body whose chain lies within ρ of
//! the core (lines checked at their ends, the distance to a segment being convex; arcs only with
//! their centre on the core and radius ≤ ρ); others get no closed form.
//!
//! dist(F, P), with F taken as its *full* surface (a lower bound for the trimmed patch, so it
//! can prove Clear and never a Hit; anything else falls back to the branch and bound):
//! - sphere about o: F's points lie at distances [r·k_lo, r·k_hi] from o (k from the frame's
//!   rounding, see `frame_k`); with D = |p − o| over P in [d_min, d_max]: the distance is at
//!   least d_min − r·k_hi, or r·k_lo − d_max, or 0;
//! - cylinder: the same with the distance to the axis line;
//! - plane through o spanned by u, v: the signed distances of P's corners to it.
//!
//! d_min² is the minimum over the (s, t) box of a convex quadratic Q. For any (s0, t0),
//! Q(x) ≥ Q(x0) + ∇Q(x0)·(x − x0) (convexity), and the right side's minimum over the box is at a
//! corner: so with x0 the f64 minimiser, a rigorous lower bound tight to the rounding, all in
//! outward-rounded intervals. d_max² is the maximum of Q at the corners (convex).
//!
//! Every bound is an outward-rounded interval evaluation: the result is a true lower bound for
//! the real (exact) inputs. The face's `mu` is held as the branch and bound holds it.

use crate::check::{Face, Move, Surf, V3, Verdict};
use crate::ival::Ival;
use crate::tool::{Body, PSeg};

/// The capsule containing a body: core from (0, 0, zc) to (0, 0, z1), radius rho.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub zc: f64,
    pub z1: f64,
    pub rho: f64,
}

/// Whether (r, z) in the (ρ, z) half-plane is proven within c.rho of the core segment: beside the
/// core (zc ≤ z ≤ z1) an exact comparison r ≤ ρ; beyond its ends, r² + dz² ≤ ρ² in intervals.
fn within(c: &Capsule, r: f64, z: f64) -> bool {
    if z >= c.zc && z <= c.z1 {
        return r.abs() <= c.rho;
    }
    let dz = if z < c.zc {
        Ival::pt(c.zc).sub(Ival::pt(z))
    } else {
        Ival::pt(z).sub(Ival::pt(c.z1))
    };
    Ival::pt(r).sqr().add(dz.sqr()).hi <= Ival::pt(c.rho).sqr().lo
}

/// The capsule of a body, when its whole chain lies within its largest radius of the axis
/// segment from z0 + rmax to z1 (a ball end mill's cutter body; not a flat or bull nose).
pub fn capsule(b: &Body) -> Option<Capsule> {
    let c = Capsule {
        // any zc is sound: containment is checked piece by piece below
        zc: b.z0 + b.rmax,
        z1: b.z1,
        rho: b.rmax,
    };
    if !(c.zc <= c.z1 && c.rho > 0.0) {
        return None;
    }
    let mut i = 0;
    while i < b.chain.len() {
        match b.chain[i] {
            PSeg::Line { a, b: e } => {
                if !within(&c, a[0], a[1]) || !within(&c, e[0], e[1]) {
                    return None;
                }
            }
            PSeg::Arc { c: ac, r, .. } => {
                if !(ac[0] == 0.0 && ac[1] >= c.zc && ac[1] <= c.z1 && r <= c.rho) {
                    return None;
                }
            }
        }
        i += 1;
    }
    Some(c)
}

fn iv(v: V3) -> [Ival; 3] {
    [Ival::pt(v[0]), Ival::pt(v[1]), Ival::pt(v[2])]
}
fn dot(a: [Ival; 3], b: [Ival; 3]) -> Ival {
    a[0].mul(b[0]).add(a[1].mul(b[1])).add(a[2].mul(b[2]))
}
fn sub(a: [Ival; 3], b: [Ival; 3]) -> [Ival; 3] {
    [a[0].sub(b[0]), a[1].sub(b[1]), a[2].sub(b[2])]
}
fn add(a: [Ival; 3], b: [Ival; 3]) -> [Ival; 3] {
    [a[0].add(b[0]), a[1].add(b[1]), a[2].add(b[2])]
}
fn scl(a: [Ival; 3], k: Ival) -> [Ival; 3] {
    [a[0].mul(k), a[1].mul(k), a[2].mul(k)]
}

pub(crate) const CORNERS: [(f64, f64); 4] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];

/// Q(s, t) = |w(s, t)|² − (w·ax)²/(ax·ax) (with ax: the squared distance to the line through
/// the origin along ax; without: the squared length), w = w0 + s d + t e. Convex in (s, t).
pub(crate) struct Quad {
    pub(crate) w0: [Ival; 3],
    pub(crate) d: [Ival; 3],
    pub(crate) e: [Ival; 3],
    pub(crate) ax: Option<([Ival; 3], Ival)>,
}

impl Quad {
    fn w(&self, s: Ival, t: Ival) -> [Ival; 3] {
        add(add(self.w0, scl(self.d, s)), scl(self.e, t))
    }
    fn q(&self, s: Ival, t: Ival) -> Ival {
        let w = self.w(s, t);
        let n = dot(w, w);
        match self.ax {
            None => n,
            Some((a, aa)) => {
                let k = dot(w, a);
                n.sub(k.sqr().div(aa))
            }
        }
    }
    /// ∇Q at (s, t).
    fn grad(&self, s: Ival, t: Ival) -> [Ival; 2] {
        let w = self.w(s, t);
        let (gs, gt) = (dot(w, self.d), dot(w, self.e));
        let (gs, gt) = match self.ax {
            None => (gs, gt),
            Some((a, aa)) => {
                let k = dot(w, a).div(aa);
                (gs.sub(k.mul(dot(self.d, a))), gt.sub(k.mul(dot(self.e, a))))
            }
        };
        [gs.scale(2.0), gt.scale(2.0)]
    }
    fn consider(&self, s: f64, t: f64, best: &mut (f64, f64), bq: &mut f64) {
        let s = s.clamp(0.0, 1.0);
        let t = t.clamp(0.0, 1.0);
        let v = self.qf(s, t);
        if v < *bq {
            *bq = v;
            *best = (s, t);
        }
    }
    fn qf(&self, s: f64, t: f64) -> f64 {
        self.q(Ival::pt(s), Ival::pt(t)).mid()
    }
    /// An f64 minimiser of Q over [0, 1]², by the interior stationary point and each edge's
    /// (a guess only: the bound below is valid for any guess).
    fn guess(&self) -> (f64, f64) {
        let mut best = (0.0, 0.0);
        let mut bq = f64::INFINITY;
        // Q's Hessian/2 and the linear part, in f64, from Q at a few points (Q is quadratic)
        let q00 = self.qf(0.0, 0.0);
        let q10 = self.qf(1.0, 0.0);
        let q01 = self.qf(0.0, 1.0);
        let q11 = self.qf(1.0, 1.0);
        let qh0 = self.qf(0.5, 0.0);
        let q0h = self.qf(0.0, 0.5);
        // Q = A s² + 2B st + C t² + D s + E t + F
        let f = q00;
        let a = 2.0 * (q10 + q00 - 2.0 * qh0);
        let c = 2.0 * (q01 + q00 - 2.0 * q0h);
        let dd = q10 - q00 - a;
        let ee = q01 - q00 - c;
        let b = 0.5 * (q11 - f - a - c - dd - ee);
        let det = a * c - b * b;
        if det.abs() > 1e-300 {
            let s = (-dd * c + ee * b) / (2.0 * det);
            let t = (-ee * a + dd * b) / (2.0 * det);
            self.consider(s, t, &mut best, &mut bq);
        }
        // edges: s = 0, 1 (minimise in t), t = 0, 1 (minimise in s)
        let mut k = 0;
        while k < 2 {
            let e = k as f64;
            let t = if c > 0.0 {
                -(2.0 * b * e + ee) / (2.0 * c)
            } else {
                0.0
            };
            self.consider(e, t, &mut best, &mut bq);
            let s = if a > 0.0 {
                -(2.0 * b * e + dd) / (2.0 * a)
            } else {
                0.0
            };
            self.consider(s, e, &mut best, &mut bq);
            k += 1;
        }
        let mut k = 0;
        while k < 4 {
            self.consider(CORNERS[k].0, CORNERS[k].1, &mut best, &mut bq);
            k += 1;
        }
        best
    }
    /// A proven lower bound on min Q over [0, 1]² (the tangent plane at the guess).
    pub(crate) fn min_lo(&self) -> Option<f64> {
        let (s0, t0) = self.guess();
        let (si, ti) = (Ival::pt(s0), Ival::pt(t0));
        let q0 = self.q(si, ti);
        let g = self.grad(si, ti);
        if !(fin(q0) && fin(g[0]) && fin(g[1])) {
            return None;
        }
        let mut lo = f64::INFINITY;
        let mut k = 0;
        while k < 4 {
            let (s, t) = CORNERS[k];
            k += 1;
            let l = q0
                .add(g[0].mul(Ival::pt(s).sub(si)))
                .add(g[1].mul(Ival::pt(t).sub(ti)));
            // a non-finite end (overflow, or NaN made the whole line) proves nothing: refuse
            if !fin(l) {
                return None;
            }
            if l.lo < lo {
                lo = l.lo;
            }
        }
        Some(lo.max(0.0))
    }
    /// A proven upper bound on max Q over [0, 1]² (a convex function's maximum is at a corner).
    pub(crate) fn max_hi(&self) -> Option<f64> {
        let mut hi = 0.0f64;
        let mut k = 0;
        while k < 4 {
            let (s, t) = CORNERS[k];
            k += 1;
            let v = self.q(Ival::pt(s), Ival::pt(t));
            if !fin(v) {
                return None;
            }
            if v.hi > hi {
                hi = v.hi;
            }
        }
        Some(hi)
    }
}

/// k_lo, k_hi with |c x + s y| and |cos φ (c x + s y) + sin φ ax| in [k_lo, k_hi] for every
/// c² + s² = 1, cos² φ + sin² φ = 1 (`ax` None: the first form): from the frame's deviation
/// from orthonormal, e = max of | |x|² − 1 |, .., 2|x·y|, ..: |v|² ∈ [1 − 3e, 1 + 3e].
fn frame_k(x: V3, y: V3, ax: Option<V3>) -> (f64, f64) {
    let (xi, yi) = (iv(x), iv(y));
    let one = Ival::pt(1.0);
    let mut e = dot(xi, xi).sub(one).abs().hi;
    e = e.max(dot(yi, yi).sub(one).abs().hi);
    e = e.max(dot(xi, yi).abs().scale(2.0).hi);
    if let Some(a) = ax {
        let ai = iv(a);
        e = e.max(dot(ai, ai).sub(one).abs().hi);
        e = e.max(dot(xi, ai).abs().scale(2.0).hi);
        e = e.max(dot(yi, ai).abs().scale(2.0).hi);
    }
    let e3 = Ival::pt(e).scale(3.0);
    let lo = one.sub(e3);
    let hi = one.add(e3);
    (if lo.lo > 0.0 { lo.sqrt().lo } else { 0.0 }, hi.sqrt().hi)
}

/// A lower bound on the distance from a point set whose distances to the reference lie in
/// [dmin, dmax] (squared: q_lo, q_hi) to a shell of radii [s_lo, s_hi].
pub(crate) fn shell(q_lo: f64, q_hi: f64, s_lo: f64, s_hi: f64) -> Option<f64> {
    if !(q_lo.is_finite() && q_hi.is_finite() && s_lo.is_finite() && s_hi.is_finite()) {
        return None;
    }
    let dmin = Ival::pt(q_lo).sqrt().lo;
    let dmax = Ival::pt(q_hi).sqrt().hi;
    Some(if dmin > s_hi {
        Ival::pt(dmin).sub(Ival::pt(s_hi)).lo.max(0.0)
    } else if dmax < s_lo {
        Ival::pt(s_lo).sub(Ival::pt(dmax)).lo.max(0.0)
    } else {
        0.0
    })
}

/// The proven lower bound on min sd_body over the face's full surface and the line a → b, or
/// None when no closed form applies.
pub fn lower(c: &Capsule, f: &Face, a: V3, b: V3) -> Option<f64> {
    // every input finite and at most 1e100 in size, so no square or sum of a few squares below
    // overflows; any non-finite value later is refused too (f64 min/max would drop a NaN and so
    // flip a comparison towards Clear)
    if !(inputs_ok(c, f, a, b)) {
        return None;
    }
    let base = [Ival::pt(a[0]), Ival::pt(a[1]), Ival::pt(a[2]).add(Ival::pt(c.zc))];
    let d = sub(iv(b), iv(a));
    let e = [Ival::pt(0.0), Ival::pt(0.0), Ival::pt(c.z1).sub(Ival::pt(c.zc))];
    let dist = match f.surf {
        Surf::Sphere { o, x, y, ax, r } => {
            let q = Quad {
                w0: sub(base, iv(o)),
                d,
                e,
                ax: None,
            };
            let (klo, khi) = frame_k(x, y, Some(ax));
            let ri = Ival::pt(r);
            shell(
                q.min_lo()?,
                q.max_hi()?,
                ri.mul(Ival::pt(klo)).lo,
                ri.mul(Ival::pt(khi)).hi,
            )?
        }
        Surf::Cyl { o, x, y, ax, r } => {
            // points o + r(c x + s y) + q ax: distance to the axis line at most r·k_hi; at least
            // r·k_lo less the part of the radial vector along ax (|x·â| + |y·â|)·r k_hi
            let ai = iv(ax);
            let aa = dot(ai, ai);
            if !(aa.lo > 0.0) {
                return None;
            }
            let q = Quad {
                w0: sub(base, iv(o)),
                d,
                e,
                ax: Some((ai, aa)),
            };
            let (klo, khi) = frame_k(x, y, None);
            let na = aa.sqrt();
            let along = dot(iv(x), ai).abs().add(dot(iv(y), ai).abs()).div(na);
            let ri = Ival::pt(r);
            let rhi = ri.mul(Ival::pt(khi));
            let rlo2 = ri.mul(Ival::pt(klo)).sqr().sub(rhi.mul(along).sqr());
            let rlo = if rlo2.lo > 0.0 { rlo2.sqrt().lo } else { 0.0 };
            shell(q.min_lo()?, q.max_hi()?, rlo, rhi.hi)?
        }
        Surf::Plane { o, u, v } => {
            let (ui, vi) = (iv(u), iv(v));
            let n = [
                ui[1].mul(vi[2]).sub(ui[2].mul(vi[1])),
                ui[2].mul(vi[0]).sub(ui[0].mul(vi[2])),
                ui[0].mul(vi[1]).sub(ui[1].mul(vi[0])),
            ];
            let nn = dot(n, n).sqrt();
            if !(nn.lo > 0.0) {
                return None;
            }
            let w0 = sub(base, iv(o));
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            let mut k = 0;
            while k < 4 {
                let (s, t) = CORNERS[k];
                k += 1;
                let w = add(add(w0, scl(d, Ival::pt(s))), scl(e, Ival::pt(t)));
                let h = dot(w, n).div(nn);
                if !fin(h) {
                    return None;
                }
                lo = lo.min(h.lo);
                hi = hi.max(h.hi);
            }
            if lo > 0.0 {
                lo
            } else if hi < 0.0 {
                -hi
            } else {
                0.0
            }
        }
        // tori and cones: contact_rev.rs
        Surf::Torus { o, .. } | Surf::Cone { o, .. } => crate::contact_rev::dist(f, sub(base, iv(o)), d, e)?,
    };
    Some(Ival::pt(dist).sub(Ival::pt(c.rho)).lo)
}

fn fin(i: Ival) -> bool {
    i.lo.is_finite() && i.hi.is_finite()
}

fn small(x: f64) -> bool {
    x.is_finite() && x.abs() <= 1e100
}

fn small3(v: V3) -> bool {
    small(v[0]) && small(v[1]) && small(v[2])
}

/// Every number the bound reads, finite and at most 1e100.
fn inputs_ok(c: &Capsule, f: &Face, a: V3, b: V3) -> bool {
    let surf = match f.surf {
        Surf::Plane { o, u, v } => small3(o) && small3(u) && small3(v),
        Surf::Cyl { o, x, y, ax, r } | Surf::Sphere { o, x, y, ax, r } => {
            small3(o) && small3(x) && small3(y) && small3(ax) && small(r)
        }
        Surf::Cone { o, x, y, ax, r0, k } => {
            small3(o) && small3(x) && small3(y) && small3(ax) && small(r0) && small(k)
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
            small3(o)
                && small3(x)
                && small3(y)
                && small3(ax)
                && small(big)
                && small(r)
                && small(cm)
                && small(sm)
        }
    };
    surf && small3(a)
        && small3(b)
        && small(c.zc)
        && small(c.z1)
        && small(c.rho)
        && small(f.mu)
        && small(f.p.lo)
        && small(f.p.hi)
        && small(f.q.lo)
        && small(f.q.hi)
}

/// Clear by the closed form, or None (the caller goes on to its other methods). Only a line
/// move, and only a body with a capsule.
pub fn clear(body: &Body, f: &Face, mv: &Move, limit: f64) -> Option<Verdict> {
    let Move::Line { a, b } = *mv else { return None };
    if !(limit.is_finite() && f.mu.is_finite()) {
        return None;
    }
    let c = capsule(body)?;
    let lo = lower(&c, f, a, b)?;
    let lim_clear = Ival::pt(limit).add(Ival::pt(f.mu)).hi;
    if lo >= lim_clear {
        Some(Verdict::Clear {
            lower: Ival::pt(lo).sub(Ival::pt(f.mu)).lo,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{Face, certify_bb};
    use crate::tool::{Cutter, assemble};

    fn ball(d: f64) -> Body {
        assemble(&Cutter::Ball { d }, 3.0 * d, d, 4.0 * d, None)
            .unwrap()
            .bodies[0]
            .clone()
    }

    #[test]
    fn a_ball_cutter_has_a_capsule_and_a_flat_does_not() {
        let b = ball(6.0);
        let c = capsule(&b).unwrap();
        assert_eq!((c.zc, c.rho), (3.0, 3.0));
        let f = assemble(&Cutter::Flat { d: 6.0 }, 18.0, 6.0, 24.0, None)
            .unwrap()
            .bodies[0]
            .clone();
        assert!(capsule(&f).is_none());
    }

    fn sphere(o: V3, r: f64) -> Face {
        Face::new(
            Surf::Sphere {
                o,
                x: [1.0, 0.0, 0.0],
                y: [0.0, 1.0, 0.0],
                ax: [0.0, 0.0, 1.0],
                r,
            },
            Ival::new(-1.0, 1.0),
            Ival::new(0.0, 1.0),
            None,
            0.0,
            0,
        )
    }

    #[test]
    fn exact_on_a_sphere_touching_from_above() {
        // ball tip at height 20 + gap over the pole of a sphere r 30 about (0, 0, −10)
        let b = ball(6.0);
        let c = capsule(&b).unwrap();
        let f = sphere([0.0, 0.0, -10.0], 30.0);
        for gap in [0.5, 0.01, 0.0] {
            let lo = lower(&c, &f, [0.0, 0.0, 20.0 + gap], [0.0, 0.0, 20.0 + gap]).unwrap();
            assert!(lo <= gap && lo > gap - 1e-12, "gap {gap} lo {lo}");
        }
    }

    /// The closed form never claims more than the branch and bound proves possible: on random
    /// (sphere, cylinder, plane) × line pairs, whenever the closed form says Clear at a limit,
    /// the branch and bound finds no Hit at that limit; and the bound never exceeds the true
    /// minimum sampled densely.
    #[test]
    fn never_above_the_true_minimum() {
        let b = ball(6.0);
        let c = capsule(&b).unwrap();
        let mut seed = 0x9e3779b97f4a7c15u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut agreed = 0;
        let mut k = 0;
        while k < 150 {
            let kind = k % 3;
            let f = match kind {
                0 => sphere(
                    [rnd() * 10.0 - 5.0, rnd() * 10.0 - 5.0, rnd() * 4.0 - 10.0],
                    5.0 + rnd() * 10.0,
                ),
                1 => Face::new(
                    Surf::Cyl {
                        o: [0.0, 0.0, -5.0],
                        x: [0.0, 1.0, 0.0],
                        y: [0.0, 0.0, 1.0],
                        ax: [1.0, 0.0, 0.0],
                        r: 3.0 + rnd() * 8.0,
                    },
                    Ival::new(-1.0, 1.0),
                    Ival::new(-20.0, 20.0),
                    None,
                    0.0,
                    1,
                ),
                _ => Face::new(
                    Surf::Plane {
                        o: [0.0, 0.0, rnd() * 4.0 - 2.0],
                        u: [1.0, 0.0, rnd() * 0.6 - 0.3],
                        v: [0.0, 1.0, rnd() * 0.6 - 0.3],
                    },
                    Ival::new(-30.0, 30.0),
                    Ival::new(-30.0, 30.0),
                    None,
                    0.0,
                    2,
                ),
            };
            let a = [rnd() * 20.0 - 10.0, rnd() * 20.0 - 10.0, 5.0 + rnd() * 15.0];
            let bb = [
                a[0] + rnd() * 6.0 - 3.0,
                a[1] + rnd() * 6.0 - 3.0,
                a[2] + rnd() * 4.0 - 2.0,
            ];
            let lo = lower(&c, &f, a, bb).unwrap();
            // dense sample of the true minimum over the face (its parameter box) and the move
            let mut tmin = f64::INFINITY;
            let n = 24;
            let mut i = 0;
            while i <= n {
                let mut j = 0;
                while j <= n {
                    let p = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / n as f64;
                    let q = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / n as f64;
                    let y = f.surf.eval(Ival::pt(p), Ival::pt(q));
                    let y = [y[0].mid(), y[1].mid(), y[2].mid()];
                    let mut m = 0;
                    while m <= 10 {
                        let s = m as f64 / 10.0;
                        let g = [
                            a[0] + (bb[0] - a[0]) * s,
                            a[1] + (bb[1] - a[1]) * s,
                            a[2] + (bb[2] - a[2]) * s,
                        ];
                        let sd = b
                            .sd(
                                Ival::pt(y[0] - g[0]),
                                Ival::pt(y[1] - g[1]),
                                Ival::pt(y[2] - g[2]),
                            )
                            .mid();
                        tmin = tmin.min(sd);
                        m += 1;
                    }
                    j += 1;
                }
                i += 1;
            }
            assert!(lo <= tmin + 1e-9, "k={k} kind={kind} lo={lo} sampled min={tmin}");
            // and the branch and bound never finds a Hit where the closed form clears
            if lo >= 0.0 {
                let mv = Move::Line { a, b: bb };
                let (v, _) = certify_bb(&b, &f, &mv, lo - 1e-9, 0.01, 20_000);
                assert!(
                    !matches!(v, Verdict::Hit { .. }),
                    "k={k}: closed form clears at {lo}, B&B hits: {v:?}"
                );
                agreed += 1;
            }
            k += 1;
        }
        assert!(agreed > 15, "{agreed}");
    }

    /// Non-finite or overflowing data anywhere (move, face, frame, radius, mu, limit) never
    /// gives Clear: the closed form refuses and the existing paths decide.
    #[test]
    fn nan_inf_and_overflow_are_refused() {
        let b = ball(6.0);
        let c = capsule(&b).unwrap();
        let bad = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -1e300];
        let base_sphere = |o: V3, x: V3, r: f64| {
            Face::new(
                Surf::Sphere {
                    o,
                    x,
                    y: [0.0, 1.0, 0.0],
                    ax: [0.0, 0.0, 1.0],
                    r,
                },
                Ival::new(-1.0, 1.0),
                Ival::new(0.0, 1.0),
                None,
                0.0,
                0,
            )
        };
        let (a, bb) = ([0.0, 0.0, 30.0], [5.0, 0.0, 30.0]);
        // sanity: the clean case clears
        let ok = base_sphere([0.0, 0.0, -10.0], [1.0, 0.0, 0.0], 30.0);
        assert!(clear(&b, &ok, &Move::Line { a, b: bb }, -0.01).is_some());
        for &v in &bad {
            let cases: Vec<(Face, V3, V3)> = vec![
                (base_sphere([v, 0.0, -10.0], [1.0, 0.0, 0.0], 30.0), a, bb),
                (base_sphere([0.0, 0.0, -10.0], [v, 0.0, 0.0], 30.0), a, bb),
                (base_sphere([0.0, 0.0, -10.0], [1.0, 0.0, 0.0], v), a, bb),
                (ok.clone(), [v, 0.0, 30.0], bb),
                (ok.clone(), a, [5.0, v, 30.0]),
                (
                    Face::new(
                        Surf::Plane {
                            o: [0.0, 0.0, v],
                            u: [1.0, 0.0, 0.0],
                            v: [0.0, 1.0, 0.0],
                        },
                        Ival::new(-1.0, 1.0),
                        Ival::new(-1.0, 1.0),
                        None,
                        0.0,
                        1,
                    ),
                    a,
                    bb,
                ),
                (
                    Face::new(
                        Surf::Cyl {
                            o: [0.0, 0.0, -5.0],
                            x: [0.0, 1.0, 0.0],
                            y: [0.0, 0.0, 1.0],
                            ax: [v, 0.0, 0.0],
                            r: 3.0,
                        },
                        Ival::new(-1.0, 1.0),
                        Ival::new(-1.0, 1.0),
                        None,
                        0.0,
                        2,
                    ),
                    a,
                    bb,
                ),
            ];
            for (k, (f, p, q)) in cases.iter().enumerate() {
                assert!(lower(&c, f, *p, *q).is_none(), "value {v} case {k}: lower");
                assert!(
                    clear(&b, f, &Move::Line { a: *p, b: *q }, -0.01).is_none(),
                    "value {v} case {k}"
                );
            }
            // a bad mu or limit
            let mut fm = ok.clone();
            fm.mu = v;
            assert!(
                clear(&b, &fm, &Move::Line { a, b: bb }, -0.01).is_none(),
                "mu {v}"
            );
            // a non-finite limit is refused (a finite one, however low, is a fair question)
            if !v.is_finite() {
                assert!(clear(&b, &ok, &Move::Line { a, b: bb }, v).is_none(), "limit {v}");
            }
        }
    }

    /// Degenerate moves (zero length, vertical, along z only between equal xy) and the cutter
    /// inside a large sphere near its wall: the bound stays below the sampled true minimum.
    #[test]
    fn degenerate_moves_and_the_inside_case() {
        let b = ball(6.0);
        let c = capsule(&b).unwrap();
        let dome = sphere([0.0, 0.0, -10.0], 30.0);
        let moves = [
            ([3.0, 2.0, 21.0], [3.0, 2.0, 21.0]),
            ([3.0, 2.0, 21.0], [3.0, 2.0, 35.0]),
            ([3.0, 2.0, 35.0], [3.0, 2.0, 21.0]),
            ([0.0, 0.0, 20.0], [0.0, 0.0, 20.0]),
        ];
        for (a, bb) in moves {
            let lo = lower(&c, &dome, a, bb).unwrap();
            let t = sampled_min(&b, &dome, a, bb);
            assert!(lo <= t + 1e-9, "{a:?}->{bb:?}: {lo} > {t}");
        }
        // inside a sphere of radius 50 about the origin (its lower half: a bowl), the tool near
        // the bottom: the inside-shell case proves the gap
        let bowl = Face::new(
            Surf::Sphere {
                o: [0.0, 0.0, 0.0],
                x: [1.0, 0.0, 0.0],
                y: [0.0, 1.0, 0.0],
                ax: [0.0, 0.0, 1.0],
                r: 50.0,
            },
            Ival::new(-1.0, 1.0),
            Ival::new(-1.0, 0.0),
            None,
            0.0,
            3,
        );
        let (a, bb) = ([0.0, 0.0, -49.0], [4.0, 0.0, -48.5]);
        let lo = lower(&c, &bowl, a, bb).unwrap();
        let t = sampled_min(&b, &bowl, a, bb);
        assert!(lo <= t + 1e-9, "{lo} > {t}");
        // and well inside, away from the wall, it is positive (the inside case decides)
        let lo = lower(&c, &bowl, [0.0, 0.0, -30.0], [1.0, 0.0, -30.0]).unwrap();
        assert!(lo > 5.0, "{lo}");
    }

    fn sampled_min(b: &Body, f: &Face, a: V3, bb: V3) -> f64 {
        let mut tmin = f64::INFINITY;
        let n = 60;
        let mut i = 0;
        while i <= n {
            let mut j = 0;
            while j <= n {
                let p = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / n as f64;
                let q = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / n as f64;
                let y = f.surf.eval(Ival::pt(p), Ival::pt(q));
                let y = [y[0].mid(), y[1].mid(), y[2].mid()];
                let mut m = 0;
                while m <= 10 {
                    let s = m as f64 / 10.0;
                    let g = [
                        a[0] + (bb[0] - a[0]) * s,
                        a[1] + (bb[1] - a[1]) * s,
                        a[2] + (bb[2] - a[2]) * s,
                    ];
                    tmin = tmin.min(
                        b.sd(
                            Ival::pt(y[0] - g[0]),
                            Ival::pt(y[1] - g[1]),
                            Ival::pt(y[2] - g[2]),
                        )
                        .mid(),
                    );
                    m += 1;
                }
                j += 1;
            }
            i += 1;
        }
        tmin
    }
}
