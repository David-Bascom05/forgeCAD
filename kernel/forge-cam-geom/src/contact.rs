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

/// The capsule containing a body: the points within rho of its core, the solid cylinder of
/// radius `a` about the axis from (0, 0, zc) to (0, 0, z1). `a` = 0: the core is the axis
/// segment (a ball cutter's capsule); `a` > 0, a disc capsule: a bull-nose cutter (radius R,
/// corner r) is exactly the points within r of the cylinder of radius R − r from z0 + r up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub zc: f64,
    pub z1: f64,
    pub rho: f64,
    pub a: f64,
}

/// Whether (r, z) in the (ρ, z) half-plane is proven within c.rho of the core segment: beside the
/// core (zc ≤ z ≤ z1) an exact comparison r ≤ ρ; beyond its ends, r² + dz² ≤ ρ² in intervals.
fn within(c: &Capsule, r: f64, z: f64) -> bool {
    if c.a > 0.0 {
        return within_disc(c, r, z);
    }
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

/// x − y when it is exact (Knuth's two-sum: its rounding error is computed exactly and is 0),
/// else None.
fn exact_diff(x: f64, y: f64) -> Option<f64> {
    let s = x - y;
    let ny = -y;
    let bv = s - x;
    let av = s - bv;
    let e = (x - av) + (ny - bv);
    (s.is_finite() && e == 0.0).then_some(s)
}

/// `within` for a disc capsule: (r, z)'s distance to the core's half-section, the rectangle
/// [0, a] × [zc, z1], proven at most rho. Beside the rectangle (zc ≤ z ≤ z1, or |r| ≤ a) the
/// distance is one difference, compared exactly where it is exact (a bull-nose's profile lies
/// on its capsule's boundary: an interval's widening would refuse it); at its corners
/// (max(|r| − a, 0))² + dz² ≤ rho² in intervals.
fn within_disc(c: &Capsule, r: f64, z: f64) -> bool {
    let r = r.abs();
    let beside_z = z >= c.zc && z <= c.z1;
    if beside_z && r <= c.a {
        return true;
    }
    if beside_z {
        return match exact_diff(r, c.a) {
            Some(d) => d <= c.rho,
            None => Ival::pt(r).sub(Ival::pt(c.a)).hi <= c.rho,
        };
    }
    let dz = if z < c.zc { (c.zc, z) } else { (z, c.z1) };
    if r <= c.a {
        return match exact_diff(dz.0, dz.1) {
            Some(d) => d <= c.rho,
            None => Ival::pt(dz.0).sub(Ival::pt(dz.1)).hi <= c.rho,
        };
    }
    let dr = Ival::pt(r).sub(Ival::pt(c.a));
    let dzi = Ival::pt(dz.0).sub(Ival::pt(dz.1));
    dr.sqr().add(dzi.sqr()).hi <= Ival::pt(c.rho).sqr().lo
}

/// The capsule of a body, when its whole chain lies within its largest radius of the axis
/// segment from z0 + rmax to z1 (a ball end mill's cutter body; not a flat or bull nose).
pub fn capsule(b: &Body) -> Option<Capsule> {
    let c = Capsule {
        // any zc is sound: containment is checked piece by piece below
        zc: b.z0 + b.rmax,
        z1: b.z1,
        rho: b.rmax,
        a: 0.0,
    };
    // (the `old-closed-form` feature: a ball's capsule only, as before)
    axis_capsule(b, c).or_else(|| {
        if cfg!(feature = "old-closed-form") {
            None
        } else {
            disc_capsule(b)
        }
    })
}

/// A body within the axis capsule `c` (every piece checked), or None.
fn axis_capsule(b: &Body, c: Capsule) -> Option<Capsule> {
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

/// A disc capsule for a body with a corner arc off the axis (a bull-nose): radius rho the
/// arc's, core radius a its centre's distance from the axis, from the centre's height up; every
/// piece proven within it (a line by its ends: the distance to the core, a convex set, is convex
/// along it; an arc by its centre in the core and its radius at most rho). None otherwise.
fn disc_capsule(b: &Body) -> Option<Capsule> {
    let (ac, r) = b.chain.iter().find_map(|s| match *s {
        PSeg::Arc { c, r, .. } if c[0] > 0.0 => Some((c, r)),
        _ => None,
    })?;
    let c = Capsule {
        zc: ac[1],
        z1: b.z1,
        rho: r,
        a: ac[0],
    };
    if !(c.zc <= c.z1 && c.rho > 0.0 && c.a > 0.0 && c.a.is_finite()) {
        return None;
    }
    for s in &b.chain {
        match *s {
            PSeg::Line { a, b: e } => {
                if !within_disc(&c, a[0], a[1]) || !within_disc(&c, e[0], e[1]) {
                    return None;
                }
            }
            PSeg::Arc { c: k, r, .. } => {
                if !(k[0] >= 0.0 && k[0] <= c.a && k[1] >= c.zc && k[1] <= c.z1 && r <= c.rho) {
                    return None;
                }
            }
        }
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

/// `shell` for a disc capsule against a cylinder about the line through the origin along `ax`:
/// P is P0 (the parallelogram `q` is of) swept by the horizontal disc of radius a. Each point of
/// P is within a of P0, so its distance to the axis lies in [d_min(P0) − a, d_max(P0) + a]; and
/// for the unit k = â × h (h = ẑ × â, horizontal, ⊥ â), k ⊥ â, so a point's distance to the axis
/// is at least |w·k|, linear over P: its range is P0's corners' (a parallelogram's extremes are
/// at corners) widened by the disc's reach along k, a·|k_xy|. The larger of the two lower
/// bounds is one. (A horizontal axis: k is vertical, the disc adds nothing, and the bound is
/// the core's height over or under the axis: tight for a cutter riding over the cylinder.)
fn disc_shell(q: &Quad, ai: [Ival; 3], a: f64, s_lo: f64, s_hi: f64) -> Option<f64> {
    let (q_lo, q_hi) = (q.min_lo()?, q.max_hi()?);
    if !(q_lo.is_finite() && q_hi.is_finite() && s_lo.is_finite() && s_hi.is_finite()) {
        return None;
    }
    let ai_ = Ival::pt(a);
    let mut dmin = Ival::pt(q_lo).sqrt().sub(ai_).lo;
    let dmax = Ival::pt(q_hi).sqrt().add(ai_).hi;
    // h = ẑ × ax, k = ax × h
    let h = [ai[1].neg(), ai[0], Ival::pt(0.0)];
    let k = [
        ai[1].mul(h[2]).sub(ai[2].mul(h[1])),
        ai[2].mul(h[0]).sub(ai[0].mul(h[2])),
        ai[0].mul(h[1]).sub(ai[1].mul(h[0])),
    ];
    let kk = dot(k, k).sqrt();
    if kk.lo > 0.0 {
        let kxy = k[0].sqr().add(k[1].sqr()).sqrt();
        let reach = ai_.mul(kxy).div(kk);
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut j = 0;
        while j < 4 {
            let (s, t) = CORNERS[j];
            j += 1;
            let w = add(add(q.w0, scl(q.d, Ival::pt(s))), scl(q.e, Ival::pt(t)));
            let v = dot(w, k).div(kk);
            if !(fin(v) && fin(reach)) {
                return None;
            }
            lo = lo.min(v.lo);
            hi = hi.max(v.hi);
        }
        let lo = Ival::pt(lo).sub(Ival::pt(reach.hi)).lo;
        let hi = Ival::pt(hi).add(Ival::pt(reach.hi)).hi;
        let vmin = if lo > 0.0 {
            lo
        } else if hi < 0.0 {
            -hi
        } else {
            0.0
        };
        if vmin > dmin {
            dmin = vmin;
        }
    }
    let dmin = if dmin > 0.0 { dmin } else { 0.0 };
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
            if c.a > 0.0 {
                disc_shell(&q, ai, c.a, rlo, rhi.hi)?
            } else {
                shell(q.min_lo()?, q.max_hi()?, rlo, rhi.hi)?
            }
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
            // a disc capsule: P is the parallelogram swept by the horizontal disc of radius a,
            // whose points reach a·|n_xy|/|n| either way along the normal
            if c.a > 0.0 {
                let nxy = n[0].sqr().add(n[1].sqr()).sqrt();
                let w = Ival::pt(c.a).mul(nxy).div(nn);
                if !fin(w) {
                    return None;
                }
                lo = Ival::pt(lo).sub(Ival::pt(w.hi)).lo;
                hi = Ival::pt(hi).add(Ival::pt(w.hi)).hi;
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
    // a disc capsule's P is the parallelogram P0 above swept by the horizontal disc of radius
    // a: every point of P is within a of P0, so dist(F, P) ≥ dist(F, P0) − a (planes take the
    // disc exactly, above)
    let dist = if c.a > 0.0 && !matches!(f.surf, Surf::Plane { .. } | Surf::Cyl { .. }) {
        Ival::pt(dist).sub(Ival::pt(c.a)).lo
    } else {
        dist
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
        && small(c.a)
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

/// A level arc (`Move::Arc`) clear by its chords, or None. The arc is cut into n pieces evenly in
/// its parameter t; a piece spans an angle of at most 2h (h the step in t: dθ/dt = 2/(1+t²) ≤ 2),
/// so every point of it is within its sagitta r(1 − cos h) ≤ r h²/2 of its chord. The body's
/// signed distance to a face is 1-Lipschitz in the body's position, so each chord proven clear
/// by `clear` at `limit + s` (s that sagitta, widened past the chords' ends' rounding) proves the
/// arc at `limit`, with a lower bound s under the chords' least. None (the caller goes on to
/// its other methods) for any other move, non-finite or overflowing data, too fine a cut, or a
/// chord `clear` does not prove.
pub fn clear_arc(body: &Body, f: &Face, mv: &Move, limit: f64) -> Option<Verdict> {
    let Move::Arc { c, x, y, r, t0, t1 } = *mv else {
        return None;
    };
    let small = |v: f64| v.is_finite() && v.abs() <= 1e100;
    if !(c.iter().chain(x.iter()).chain(y.iter()).all(|v| small(*v))
        && small(r)
        && r > 0.0
        && small(t0)
        && small(t1)
        && t0 <= t1
        && t0.abs() <= 1.5
        && t1.abs() <= 1.5
        && small(limit)
        && arc_frame_ok(x, y))
    {
        return None;
    }
    // the chords' sagitta at most ARC_SAG (a step h = √(2 s / r)), in at most ARC_CHORDS pieces
    let h_max = (2.0 * ARC_SAG / r).sqrt();
    let n = ((t1 - t0) / h_max).ceil().max(1.0);
    if !(n <= ARC_CHORDS as f64) {
        return None;
    }
    let n = n as usize;
    let h = (t1 - t0) / n as f64;
    let at = |t: f64| -> V3 {
        let d = 1.0 + t * t;
        let u = (1.0 - t * t) / d;
        let v = 2.0 * t / d;
        [
            c[0] + r * (x[0] * u + y[0] * v),
            c[1] + r * (x[1] * u + y[1] * v),
            c[2] + r * (x[2] * u + y[2] * v),
        ]
    };
    // the sagitta, and each computed end's distance from the true arc (a few roundings of terms
    // of size |c| + r), both widened
    let scale = 1.0 + r + c.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let s = Ival::pt(r)
        .mul(Ival::pt(h))
        .mul(Ival::pt(h))
        .mul(Ival::pt(0.5))
        .add(Ival::pt(1e-12 * scale))
        .hi;
    let lim = Ival::pt(limit).add(Ival::pt(s)).hi;
    let mut least = f64::INFINITY;
    let mut a = at(t0);
    for k in 1..=n {
        let b = if k == n { at(t1) } else { at(t0 + h * k as f64) };
        match clear(body, f, &Move::Line { a, b }, lim)? {
            Verdict::Clear { lower } => least = least.min(lower),
            _ => return None,
        }
        a = b;
    }
    Some(Verdict::Clear {
        lower: Ival::pt(least).sub(Ival::pt(s)).lo,
    })
}

/// Whether an arc's frame (x, y) is orthonormal within 1e-13: then every point c + r (u x + v y)
/// of `Move::Arc`'s rational map (u² + v² = 1) lies within r·4e-13 of the circle of radius r about
/// c in the plane x, y spans, which the chord bounds' 1e-12·(1 + r + |c|) widening covers. The
/// checker's arcs (forge-cam ops.rs checker_moves: x = (cos, sin, 0), y = (∓sin, ±cos, 0) of one
/// angle) pass with room; a frame from anywhere else is checked, not assumed.
pub fn arc_frame_ok(x: [f64; 3], y: [f64; 3]) -> bool {
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    (dot(x, x) - 1.0).abs() <= 1e-13 && (dot(y, y) - 1.0).abs() <= 1e-13 && dot(x, y).abs() <= 1e-13
}

/// `clear_arc`'s largest chord sagitta (mm): small beside any tolerance, so the chords' margin is
/// barely more than the arc's.
const ARC_SAG: f64 = 1e-4;

/// `clear_arc`'s most chords: a finer cut than this is left to the caller's other methods.
const ARC_CHORDS: usize = 4096;

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
        never_above(&ball(6.0), 0x9e3779b97f4a7c15u64);
    }

    /// The same for a bull-nose cutter (its disc capsule): the bound never exceeds the sampled
    /// true minimum, and never clears where the branch and bound finds a Hit.
    #[test]
    #[cfg(not(feature = "old-closed-form"))]
    fn a_bull_noses_bound_is_never_above_the_true_minimum() {
        let b = assemble(&Cutter::Bull { d: 10.0, re: 1.0 }, 30.0, 10.0, 40.0, None)
            .unwrap()
            .bodies[0]
            .clone();
        never_above(&b, 0x2545f4914f6cdd1du64);
    }

    /// A bull-nose's body has a disc capsule: its corner's radius, its corner centre's distance
    /// from the axis as the core's radius, from the corner centre's height up.
    #[test]
    #[cfg(not(feature = "old-closed-form"))]
    fn a_bull_nose_has_a_disc_capsule() {
        let b = assemble(&Cutter::Bull { d: 10.0, re: 1.0 }, 30.0, 10.0, 40.0, None)
            .unwrap()
            .bodies[0]
            .clone();
        let c = capsule(&b).unwrap();
        assert_eq!((c.zc, c.rho, c.a), (b.z0 + 1.0, 1.0, 4.0));
    }

    fn never_above(b: &Body, seed0: u64) {
        let b = b.clone();
        let c = capsule(&b).unwrap();
        let mut seed = seed0;
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

    /// A level arc cleared by its chords is never cleared above its true minimum (dense
    /// sampling of the arc against the face), and the branch and bound never finds a Hit where
    /// it clears.
    #[test]
    fn arcs_by_chords_never_above_the_true_minimum() {
        let b = ball(6.0);
        let mut seed = 0x2545f4914f6cdd1du64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let (mut cleared, mut k) = (0, 0);
        while k < 120 {
            let f = if k % 2 == 0 {
                sphere(
                    [rnd() * 10.0 - 5.0, rnd() * 10.0 - 5.0, rnd() * 4.0 - 10.0],
                    5.0 + rnd() * 10.0,
                )
            } else {
                Face::new(
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
                )
            };
            let (ux, uy) = (rnd() - 0.5, rnd() - 0.5);
            let l = (ux * ux + uy * uy).sqrt().max(1e-3);
            let (cs, sn) = (ux / l, uy / l);
            let x = [cs, sn, 0.0];
            let y = [-sn, cs, 0.0];
            let c = [rnd() * 16.0 - 8.0, rnd() * 16.0 - 8.0, 4.0 + rnd() * 12.0];
            let r = 0.5 + rnd() * 10.0;
            let t0 = -rnd() * 0.9;
            let t1 = rnd() * 0.9;
            let mv = Move::Arc { c, x, y, r, t0, t1 };
            if let Some(Verdict::Clear { lower }) = clear_arc(&b, &f, &mv, -1.0) {
                // dense sample of the true minimum over the face and the arc
                let mut tmin = f64::INFINITY;
                let n = 24;
                for i in 0..=n {
                    for j in 0..=n {
                        let p = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / n as f64;
                        let q = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / n as f64;
                        let w = f.surf.eval(Ival::pt(p), Ival::pt(q));
                        let w = [w[0].mid(), w[1].mid(), w[2].mid()];
                        for m in 0..=40 {
                            let t = t0 + (t1 - t0) * m as f64 / 40.0;
                            let d = 1.0 + t * t;
                            let (u, v) = ((1.0 - t * t) / d, 2.0 * t / d);
                            let g = [
                                c[0] + r * (x[0] * u + y[0] * v),
                                c[1] + r * (x[1] * u + y[1] * v),
                                c[2],
                            ];
                            let sd = b
                                .sd(
                                    Ival::pt(w[0] - g[0]),
                                    Ival::pt(w[1] - g[1]),
                                    Ival::pt(w[2] - g[2]),
                                )
                                .mid();
                            tmin = tmin.min(sd);
                        }
                    }
                }
                assert!(lower <= tmin + 1e-9, "k={k}: lower {lower} sampled {tmin}");
                if lower >= 0.0 {
                    let (v, _) = certify_bb(&b, &f, &mv, lower - 1e-9, 0.01, 20_000);
                    assert!(
                        !matches!(v, Verdict::Hit { .. }),
                        "k={k}: chords clear at {lower}, B&B hits: {v:?}"
                    );
                }
                cleared += 1;
            }
            k += 1;
        }
        assert!(cleared > 20, "{cleared}");
    }

    /// `clear_arc` keeps the chords' sagitta: over a sphere (where the closed form is exact), a
    /// level arc chorded at exactly the sagitta bound's step, whose chords are clear while the arc
    /// dips in, is not cleared; with the sagitta dropped from the chords' limit it would be.
    #[test]
    fn clear_arc_keeps_the_sagitta() {
        let b = ball(6.0);
        let f = sphere([0.0, 0.0, -10.0], 30.0);
        // the arc: radius 20 about (0, 40), nearest the sphere's axis at ρ = 20 (t = 0); three
        // chords, each exactly the step the sagitta bound allows
        let r = 20.0_f64;
        let step = (2.0 * ARC_SAG / r).sqrt();
        let t1 = 1.5 * step;
        assert_eq!(((2.0 * t1) / step).ceil(), 3.0);
        // the middle chord (t in ±step/2, t the half-angle tangent) is nearest the axis at its
        // middle
        let rho_chord = 40.0 - r / (1.0 + 0.25 * step * step).sqrt(); // r cos(atan(step/2))
        // the ball's centre where the clearance is 0 halfway between the arc and its chord
        let rm = 0.5 * (20.0 + rho_chord);
        let hz = (33.0_f64 * 33.0 - rm * rm).sqrt();
        let clear_at = |rho: f64| (rho * rho + hz * hz).sqrt() - 33.0;
        assert!(
            clear_at(20.0) < -5e-6 && clear_at(rho_chord) > 5e-6,
            "{} {}",
            clear_at(20.0),
            clear_at(rho_chord)
        );
        let mv = Move::Arc {
            c: [0.0, 40.0, hz - 10.0 - 3.0],
            x: [0.0, -1.0, 0.0],
            y: [1.0, 0.0, 0.0],
            r,
            t0: -t1,
            t1,
        };
        // the chord alone is clear
        let p = |t: f64| {
            let d = 1.0 + t * t;
            [r * 2.0 * t / d, 40.0 - r * (1.0 - t * t) / d, hz - 13.0]
        };
        let chord = Move::Line {
            a: p(-0.5 * step),
            b: p(0.5 * step),
        };
        assert!(matches!(clear(&b, &f, &chord, 0.0), Some(Verdict::Clear { .. })));
        assert!(clear_arc(&b, &f, &mv, 0.0).is_none());
    }

    /// `clear_arc` refuses non-finite or overflowing data and anything but a level arc.
    /// An arc whose frame is not orthonormal is refused (its map is no circle of radius r); the
    /// checker's frames, x = (cos, sin, 0) and y = (∓sin, ±cos, 0) of one angle, are taken.
    #[test]
    fn an_arc_frame_must_be_orthonormal() {
        let b = ball(6.0);
        let f = sphere([0.0, 0.0, -10.0], 5.0);
        let arc = |x: [f64; 3], y: [f64; 3]| Move::Arc {
            c: [0.0, 0.0, 20.0],
            x,
            y,
            r: 3.0,
            t0: -0.5,
            t1: 0.5,
        };
        for (x, y) in [
            ([1.001, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([1.0, 0.0, 0.0], [0.0, 0.999, 0.0]),
            ([1.0, 0.0, 0.0], [0.001, 1.0, 0.0]),
            ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ] {
            assert!(!arc_frame_ok(x, y), "{x:?} {y:?}");
            assert!(clear_arc(&b, &f, &arc(x, y), -0.01).is_none(), "{x:?} {y:?}");
        }
        // unit vectors rounded as a computed cosine and sine are (each within an ulp or two)
        for k in 0..1000 {
            let t = -3.0 + 6.0 * k as f64 / 999.0;
            let (cs, sn) = ((1.0 - t * t) / (1.0 + t * t), 2.0 * t / (1.0 + t * t));
            for y in [[-sn, cs, 0.0], [sn, -cs, 0.0]] {
                let x = [cs, sn, 0.0];
                assert!(arc_frame_ok(x, y), "t {t}");
                assert!(clear_arc(&b, &f, &arc(x, y), -0.01).is_some(), "t {t}");
            }
        }
    }

    #[test]
    fn arcs_with_bad_data_are_refused() {
        let b = ball(6.0);
        let f = sphere([0.0, 0.0, -10.0], 5.0);
        let ok = Move::Arc {
            c: [0.0, 0.0, 20.0],
            x: [1.0, 0.0, 0.0],
            y: [0.0, 1.0, 0.0],
            r: 3.0,
            t0: -0.5,
            t1: 0.5,
        };
        assert!(matches!(
            clear_arc(&b, &f, &ok, -0.01),
            Some(Verdict::Clear { .. })
        ));
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300] {
            let Move::Arc { c, x, y, r, t0, t1 } = ok else {
                unreachable!()
            };
            let tries = [
                Move::Arc {
                    c: [bad, c[1], c[2]],
                    x,
                    y,
                    r,
                    t0,
                    t1,
                },
                Move::Arc {
                    c,
                    x: [bad, x[1], x[2]],
                    y,
                    r,
                    t0,
                    t1,
                },
                Move::Arc {
                    c,
                    x,
                    y: [y[0], bad, y[2]],
                    r,
                    t0,
                    t1,
                },
                Move::Arc {
                    c,
                    x,
                    y,
                    r: bad,
                    t0,
                    t1,
                },
                Move::Arc {
                    c,
                    x,
                    y,
                    r,
                    t0: bad,
                    t1,
                },
                Move::Arc {
                    c,
                    x,
                    y,
                    r,
                    t0,
                    t1: bad,
                },
            ];
            for mv in tries {
                assert!(clear_arc(&b, &f, &mv, -0.01).is_none(), "{mv:?}");
            }
            assert!(clear_arc(&b, &f, &ok, bad).is_none());
        }
        // a line, a reversed or vanishing arc, a too-fine cut
        assert!(
            clear_arc(
                &b,
                &f,
                &Move::Line {
                    a: [0.0; 3],
                    b: [1.0, 0.0, 20.0]
                },
                -0.01
            )
            .is_none()
        );
        let Move::Arc { c, x, y, .. } = ok else {
            unreachable!()
        };
        assert!(
            clear_arc(
                &b,
                &f,
                &Move::Arc {
                    c,
                    x,
                    y,
                    r: 3.0,
                    t0: 0.5,
                    t1: -0.5
                },
                -0.01
            )
            .is_none()
        );
        assert!(
            clear_arc(
                &b,
                &f,
                &Move::Arc {
                    c,
                    x,
                    y,
                    r: 1e9,
                    t0: -0.5,
                    t1: 0.5
                },
                -0.01
            )
            .is_none()
        );
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
