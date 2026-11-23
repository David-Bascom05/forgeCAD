//! The certified tool–part clearance checker.
//!
//! The statement, for one move g : s ∈ [0, 1] ↦ tool tip position, one tool body B (cutter or
//! not) and one face F of the part (or of the stock, a fixture):
//!
//!   min over y ∈ F, s ∈ [0, 1] of sd_B(y − g(s)) ≥ limit
//!
//! (sd_B positive outside B). With limit = δ − ε for the cutter against the target (δ the stock
//! to leave, ε the inner tolerance) this is G1, no gouge; with a positive clearance for the
//! holder and shank against the in-process stock it is G2; for the cutter on a rapid, G3.
//!
//! The method is forge-brake's: interval branch and bound over a cell of the face's parameters
//! and of s. A cell is *cleared* when the interval signed distance over its image is at least
//! the limit; a cell gives a *witness* when a real point of the face (proven inside its trim) at a
//! real instant has a signed distance proven below the limit. The verdicts:
//! - `Clear { lower }`: every cell cleared, so the minimum is proven ≥ lower ≥ limit;
//! - `Hit { .. }`: a witness, so the minimum is proven < limit;
//! - `Undecided { .. }`: neither within the budget or the resolution. The caller refuses it.
//!
//! Faces are lowered by the caller to `Face`, a parametrised surface with a domain, plus `mu`, a
//! bound on how far the lowered face may be from the true one (rounding in the lowering): a
//! clearance must hold by mu more, and a witness must fail by mu more.
use crate::dual::{D, Num, mean_value};
use crate::ival::Ival;
use crate::tool::Body;

pub type V3 = [f64; 3];

/// A surface by its parametrisation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Surf {
    /// o + p u + q v.
    Plane { o: V3, u: V3, v: V3 },
    /// About the axis through o along `ax`, radius r: o + r (c(t) x + s(t) y) + q ax, with
    /// c = (1 − t²)/(1 + t²), s = 2t/(1 + t²) (t = tan of half the angle from x).
    Cyl { o: V3, x: V3, y: V3, ax: V3, r: f64 },
    /// A cone: o + (r0 + q k) (c(t) x + s(t) y) + q ax (k: the radius's change per unit height).
    Cone { o: V3, x: V3, y: V3, ax: V3, r0: f64, k: f64 },
    /// A sphere about o, radius r: point = o + r (cos φ (c x + s y) + sin φ ax) with the same
    /// rational map for both angles: p = t (azimuth), q = tan(φ/2) (elevation).
    Sphere { o: V3, x: V3, y: V3, ax: V3, r: f64 },
    /// A torus: o + (R + r cos φ)(c x + s y) + r sin φ ax; p = t, q = tan(φ/2).
    Torus { o: V3, x: V3, y: V3, ax: V3, big: f64, r: f64 },
}

/// The domain: a box of (p, q), and for planes optionally a region of (p, q) in line/arc loops
/// that trims it (the box is then its bounds).
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub surf: Surf,
    pub p: Ival,
    pub q: Ival,
    pub trim: Option<Trim>,
    pub mu: f64,
    /// An enclosure of the face (computed by `Face::new`).
    pub bbox: [Ival; 3],
    /// The caller's name for it.
    pub id: u32,
}

/// A planar face's trim: closed polygons and arcs in (p, q), as f64 polylines whose arcs are
/// given exactly (centre and radius), with the inside on the left.
#[derive(Clone, Debug, PartialEq)]
pub struct Trim {
    pub segs: Vec<TSeg>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TSeg {
    Line { a: [f64; 2], b: [f64; 2] },
    /// An arc of less than a half turn.
    Arc { a: [f64; 2], b: [f64; 2], c: [f64; 2], r: f64, ccw: bool },
}

fn add3<T: Num>(a: [T; 3], b: [T; 3]) -> [T; 3] {
    [a[0].add(b[0]), a[1].add(b[1]), a[2].add(b[2])]
}
fn scale3<T: Num>(v: V3, k: T) -> [T; 3] {
    [k.mul(T::c(v[0])), k.mul(T::c(v[1])), k.mul(T::c(v[2]))]
}
fn pt3<T: Num>(v: V3) -> [T; 3] {
    [T::c(v[0]), T::c(v[1]), T::c(v[2])]
}
fn mul3<T: Num>(a: [T; 3], k: T) -> [T; 3] {
    [a[0].mul(k), a[1].mul(k), a[2].mul(k)]
}

/// (c(t), s(t)): the rational unit circle.
fn circ<T: Num>(t: T) -> (T, T) {
    let t2 = t.sqr();
    let den = T::c(1.0).add(t2);
    let c = T::c(1.0).sub(t2).div(den);
    let s = t.mul(T::c(2.0)).div(den);
    (c, s)
}

impl Surf {
    /// An enclosure of the surface's points over a box of parameters.
    pub fn eval(&self, p: Ival, q: Ival) -> [Ival; 3] {
        self.eval_n(p, q)
    }

    pub fn eval_n<T: Num>(&self, p: T, q: T) -> [T; 3] {
        match *self {
            Surf::Plane { o, u, v } => add3(add3(pt3(o), scale3(u, p)), scale3(v, q)),
            Surf::Cyl { o, x, y, ax, r } => {
                let (c, s) = circ(p);
                let rad = add3(scale3(x, c), scale3(y, s));
                add3(add3(pt3(o), mul3(rad, T::c(r))), scale3(ax, q))
            }
            Surf::Cone { o, x, y, ax, r0, k } => {
                let (c, s) = circ(p);
                let rad = add3(scale3(x, c), scale3(y, s));
                let rr = T::c(r0).add(q.mul(T::c(k)));
                add3(add3(pt3(o), mul3(rad, rr)), scale3(ax, q))
            }
            Surf::Sphere { o, x, y, ax, r } => {
                let (c, s) = circ(p);
                let (cf, sf) = circ(q);
                let rad = add3(scale3(x, c), scale3(y, s));
                add3(add3(pt3(o), mul3(rad, cf.mul(T::c(r)))), scale3(ax, sf.mul(T::c(r))))
            }
            Surf::Torus { o, x, y, ax, big, r } => {
                let (c, s) = circ(p);
                let (cf, sf) = circ(q);
                let rad = add3(scale3(x, c), scale3(y, s));
                let kr = T::c(big).add(cf.mul(T::c(r)));
                add3(add3(pt3(o), mul3(rad, kr)), scale3(ax, sf.mul(T::c(r))))
            }
        }
    }
}

impl Face {
    pub fn new(surf: Surf, p: Ival, q: Ival, trim: Option<Trim>, mu: f64, id: u32) -> Face {
        let bbox = surf.eval(p, q);
        Face { surf, p, q, trim, mu, bbox, id }
    }

    /// 1: every point of the parameter box is in the face; -1: none is; 0: unknown.
    pub fn holds(&self, p: Ival, q: Ival) -> i32 {
        if p.hi < self.p.lo || p.lo > self.p.hi || q.hi < self.q.lo || q.lo > self.q.hi {
            return -1;
        }
        let in_box = p.lo >= self.p.lo && p.hi <= self.p.hi && q.lo >= self.q.lo && q.hi <= self.q.hi;
        match &self.trim {
            None => {
                if in_box {
                    1
                } else {
                    0
                }
            }
            Some(t) => {
                let s = t.classify(p, q);
                if s == 1 && !in_box { 0 } else { s }
            }
        }
    }
}

impl Trim {
    /// 1: the box is inside the trim region; -1: outside; 0: unknown.
    pub fn classify(&self, p: Ival, q: Ival) -> i32 {
        // the box misses the boundary: then all of it is on the side of its centre
        let mut lo = f64::INFINITY;
        let mut i = 0;
        while i < self.segs.len() {
            let d = tseg_dist(&self.segs[i], p, q);
            if d < lo {
                lo = d;
            }
            i += 1;
        }
        if !(lo > 0.0) {
            return 0;
        }
        let (cp, cq) = (p.mid(), q.mid());
        self.point_side(cp, cq)
    }

    /// Ray casting from an f64 point to +p, every decision in intervals: 1 in, -1 out, 0 unknown.
    fn point_side(&self, x: f64, y: f64) -> i32 {
        let mut odd = false;
        let mut i = 0;
        while i < self.segs.len() {
            match self.segs[i] {
                TSeg::Line { a, b } => {
                    let ua = a[1] > y;
                    let ub = b[1] > y;
                    if ua != ub {
                        // cross((b − a), (q − a)) sign, in intervals
                        let c = Ival::pt(b[0])
                            .sub(Ival::pt(a[0]))
                            .mul(Ival::pt(y).sub(Ival::pt(a[1])))
                            .sub(Ival::pt(b[1]).sub(Ival::pt(a[1])).mul(Ival::pt(x).sub(Ival::pt(a[0]))));
                        let s = c.sign();
                        if s == 0 {
                            return 0;
                        }
                        if (ub && s > 0) || (!ub && s < 0) {
                            odd = !odd;
                        }
                    }
                }
                TSeg::Arc { a, b, c, r, ccw } => {
                    // crossings of the horizontal line y with the circle, inside the arc's sector
                    let dy = Ival::pt(y).sub(Ival::pt(c[1]));
                    let h2 = Ival::pt(r).sqr().sub(dy.sqr());
                    if h2.hi >= 0.0 {
                        if h2.lo <= 0.0 {
                            return 0;
                        }
                        let h = h2.sqrt();
                        let xs = [Ival::pt(c[0]).add(h), Ival::pt(c[0]).sub(h)];
                        let mut k = 0;
                        while k < 2 {
                            let xk = xs[k];
                            // in the sector from a to b
                            let (vx, vy) = (xk.sub(Ival::pt(c[0])), dy);
                            let c1 = Ival::pt(a[0] - c[0]).mul(vy).sub(Ival::pt(a[1] - c[1]).mul(vx));
                            let c2 = vx.mul(Ival::pt(b[1] - c[1])).sub(vy.mul(Ival::pt(b[0] - c[0])));
                            let (c1, c2) = if ccw { (c1, c2) } else { (c1.neg(), c2.neg()) };
                            let ins = c1.lo > 0.0 && c2.lo > 0.0;
                            let out = c1.hi < 0.0 || c2.hi < 0.0;
                            if !ins && !out {
                                return 0;
                            }
                            if ins {
                                let right = xk.sub(Ival::pt(x)).sign();
                                if right == 0 {
                                    return 0;
                                }
                                if right > 0 {
                                    odd = !odd;
                                }
                            }
                            k += 1;
                        }
                    }
                }
            }
            i += 1;
        }
        if odd { 1 } else { -1 }
    }
}

/// A lower bound on the distance from the box to the trim segment.
fn tseg_dist(s: &TSeg, p: Ival, q: Ival) -> f64 {
    let seg = match *s {
        TSeg::Line { a, b } => crate::tool::PSeg::Line { a, b },
        TSeg::Arc { a, b, c, r, ccw } => crate::tool::PSeg::Arc { a, b, c, r, ccw },
    };
    crate::tool::seg_dist(&seg, p, q).lo
}

/// A move of the tool tip (3-axis: the tool is translated, its axis +z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Move {
    Line { a: V3, b: V3 },
    /// A planar arc in z = const: centre c, radius r, from the direction x (unit) by the
    /// rational map over t in [t0, t1] (each |t| < 1 + small, so less than a half turn either
    /// side), in the frame (x, y).
    Arc { c: V3, x: V3, y: V3, r: f64, t0: f64, t1: f64 },
}

impl Move {
    pub fn at(&self, s: Ival) -> [Ival; 3] {
        self.at_n(s)
    }

    pub fn at_n<T: Num>(&self, s: T) -> [T; 3] {
        match *self {
            Move::Line { a, b } => {
                let d = [T::c(b[0]).sub(T::c(a[0])), T::c(b[1]).sub(T::c(a[1])), T::c(b[2]).sub(T::c(a[2]))];
                [T::c(a[0]).add(s.mul(d[0])), T::c(a[1]).add(s.mul(d[1])), T::c(a[2]).add(s.mul(d[2]))]
            }
            Move::Arc { c, x, y, r, t0, t1 } => {
                let t = T::c(t0).add(s.mul(T::c(t1).sub(T::c(t0))));
                let (cc, ss) = circ(t);
                let rad = add3(scale3(x, cc), scale3(y, ss));
                add3(pt3(c), mul3(rad, T::c(r)))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    Clear { lower: f64 },
    /// A real face point (params p, q) at a real instant s whose signed distance is at most d
    /// (< limit), at `at` (part frame).
    Hit { d: f64, p: f64, q: f64, s: f64, at: V3 },
    Undecided { lower: f64, cells: usize },
}

#[derive(Clone, Copy, Debug)]
struct Cell {
    p: Ival,
    q: Ival,
    s: Ival,
}

/// The interval signed distance over a cell: ρ² and z of the face point relative to the tool
/// tip by the mean-value form (second order at a tangency) intersected with the natural
/// extension, then the body's (ρ, z) signed distance.
fn cell_sd(body: &Body, f: &Face, mv: &Move, c: &Cell) -> Ival {
    cell_eval(body, f, mv, c).0
}

/// The signed distance over the cell, and each parameter's smear (how much of the enclosure's
/// width it accounts for: |∂(ρ², z)| times its width), which picks what to split.
fn cell_eval(body: &Body, f: &Face, mv: &Move, c: &Cell) -> (Ival, [f64; 3]) {
    if let Some(pl) = mv.planar() {
        return cell_eval_planar(body, f, &pl, c);
    }
    let x = [c.p, c.q, c.s];
    // at the centre
    let (pc, qc, sc) = (Ival::pt(c.p.mid()), Ival::pt(c.q.mid()), Ival::pt(c.s.mid()));
    let yc = f.surf.eval(pc, qc);
    let gc = mv.at(sc);
    let rc = [yc[0].sub(gc[0]), yc[1].sub(gc[1]), yc[2].sub(gc[2])];
    let r2c = rc[0].sqr().add(rc[1].sqr());
    if c.p.width() == 0.0 && c.q.width() == 0.0 && c.s.width() == 0.0 {
        return (body.sd_rz(r2c.sqrt(), rc[2]), [0.0; 3]);
    }
    // over the box, with derivatives
    let y: [D; 3] = f.surf.eval_n(D::var(c.p, 0), D::var(c.q, 1));
    let g: [D; 3] = mv.at_n(D::var(c.s, 2));
    let rel = [y[0].sub(g[0]), y[1].sub(g[1]), y[2].sub(g[2])];
    let r2 = rel[0].sqr().add(rel[1].sqr());
    let r2b = mean_value(&r2, r2c, x);
    let zb = mean_value(&rel[2], rc[2], x);
    let r2b = Ival { lo: r2b.lo.max(0.0), hi: r2b.hi };
    // ρ's width per unit of ρ²'s near ρ: 1 / (2ρ), at least 1 / (2 ρmax); z counts as is
    let inv = if r2c.lo > 1e-12 { 0.5 / r2c.lo.sqrt() } else { 1.0 };
    // how much the signed distance moves with ρ and with z at the centre (a heuristic for the
    // split only: soundness never rests on it)
    let (rm, zm) = (r2c.mid().max(0.0).sqrt(), rc[2].mid());
    let h = 1e-6 * (1.0 + rm.abs() + zm.abs());
    let s0 = body.sd_rz(Ival::pt(rm), Ival::pt(zm)).mid();
    let nr = ((body.sd_rz(Ival::pt(rm + h), Ival::pt(zm)).mid() - s0) / h).abs().min(1.0);
    let nz = ((body.sd_rz(Ival::pt(rm), Ival::pt(zm + h)).mid() - s0) / h).abs().min(1.0);
    let mut sm = [0.0; 3];
    let mut k = 0;
    while k < 3 {
        let w = x[k].width();
        let gr = r2.g[k].abs().hi * inv;
        let gz = rel[2].g[k].abs().hi;
        sm[k] = (0.05 + nr) * gr * w + (0.05 + nz) * gz * w;
        k += 1;
    }
    (body.sd_rz(r2b.sqrt(), zb), sm)
}

/// A move at one height, in the xy plane: its 2D path and the tip's z.
#[derive(Clone, Copy, Debug)]
pub enum Planar {
    Line { a: [f64; 2], b: [f64; 2], z: f64 },
    Arc { c: [f64; 2], r: f64, a: [f64; 2], b: [f64; 2], ccw: bool, z: f64 },
}

impl Move {
    /// The move as a planar path, when the tip keeps one height. A planar arc's ends are taken
    /// from the rational map at t0 and t1 in intervals, so the sector test uses enclosures of
    /// them: the path is the exact arc between the exact ends.
    pub fn planar(&self) -> Option<Planar> {
        match *self {
            Move::Line { a, b } => {
                if a[2] == b[2] {
                    Some(Planar::Line { a: [a[0], a[1]], b: [b[0], b[1]], z: a[2] })
                } else {
                    None
                }
            }
            Move::Arc { c, x, y, r, t0, t1 } => {
                // the frame must be the xy frame, turned: x, y horizontal and unit (exact
                // requirement checked here: their z parts are 0)
                if x[2] != 0.0 || y[2] != 0.0 {
                    return None;
                }
                let e0 = self.at(Ival::pt(0.0));
                let e1 = self.at(Ival::pt(1.0));
                // ccw in xy iff the frame is right-handed (x × y = +z) and t increases
                let up = x[0] * y[1] - x[1] * y[0] > 0.0;
                let ccw = up == (t1 > t0);
                let _ = r;
                Some(Planar::Arc {
                    c: [c[0], c[1]],
                    r,
                    a: [e0[0].mid(), e0[1].mid()],
                    b: [e1[0].mid(), e1[1].mid()],
                    ccw,
                    z: c[2],
                })
            }
        }
    }
}

impl Move {
    /// For a planar move: the instant whose tip is horizontally nearest the point (reported
    /// with a witness; nothing is proven by it).
    pub fn nearest_s(&self, p: [f64; 2]) -> f64 {
        match *self {
            Move::Line { a, b } => {
                let d = [b[0] - a[0], b[1] - a[1]];
                let dd = d[0] * d[0] + d[1] * d[1];
                if dd == 0.0 {
                    return 0.0;
                }
                (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / dd).clamp(0.0, 1.0)
            }
            Move::Arc { c, x, y, t0, t1, .. } => {
                let (vx, vy) = (p[0] - c[0], p[1] - c[1]);
                let (u, w) = (vx * x[0] + vy * x[1], vx * y[0] + vy * y[1]);
                let n = (u * u + w * w).sqrt();
                let t = if n + u > 0.0 { w / (n + u) } else { f64::INFINITY };
                if t1 == t0 {
                    return 0.0;
                }
                ((t - t0) / (t1 - t0)).clamp(0.0, 1.0)
            }
        }
    }
}

/// min over the motion of sd(y − g(s)) for a planar move, over a cell of face parameters.
///
/// sd of a body {ρ ≤ w(z)} is non-decreasing in ρ at fixed z (shrinking a point toward the axis
/// at its height never moves it away from such a body: research/cam/build.md §4), and every
/// instant of a planar move shares the face point's z offset; so the minimum over s is sd at
/// (ρ_min, z), ρ_min the horizontal distance from the face point to the path. That is closed
/// form: the motion needs no cells.
fn cell_eval_planar(body: &Body, f: &Face, pl: &Planar, c: &Cell) -> (Ival, [f64; 3]) {
    let x = [c.p, c.q, Ival::pt(0.0)];
    let (pc, qc) = (Ival::pt(c.p.mid()), Ival::pt(c.q.mid()));
    let yc = f.surf.eval(pc, qc);
    let point = c.p.width() == 0.0 && c.q.width() == 0.0;
    let y: [D; 3] = if point {
        [D::c(0.0); 3]
    } else {
        f.surf.eval_n(D::var(c.p, 0), D::var(c.q, 1))
    };
    // a quantity f(y) by the mean-value form from its value at the centre
    let mv = |d: D, at: Ival| if point { at } else { mean_value(&d, at, x) };
    let (zp, z_at) = match *pl {
        Planar::Line { z, .. } | Planar::Arc { z, .. } => (z, z),
    };
    let _ = zp;
    let zb = mv(y[2].sub(D::c(z_at)), yc[2].sub(Ival::pt(z_at)));
    let rho = match *pl {
        Planar::Line { a, b, .. } => {
            let d = [b[0] - a[0], b[1] - a[1]];
            let dd = Ival::pt(d[0]).sqr().add(Ival::pt(d[1]).sqr());
            let (ux, uy) = (y[0].sub(D::c(a[0])), y[1].sub(D::c(a[1])));
            let (uxc, uyc) = (yc[0].sub(Ival::pt(a[0])), yc[1].sub(Ival::pt(a[1])));
            let tn = mv(ux.mul(D::c(d[0])).add(uy.mul(D::c(d[1]))), uxc.scale(d[0]).add(uyc.scale(d[1])));
            let da2 = mv(ux.sqr().add(uy.sqr()), uxc.sqr().add(uyc.sqr()));
            let (vx, vy) = (y[0].sub(D::c(b[0])), y[1].sub(D::c(b[1])));
            let (vxc, vyc) = (yc[0].sub(Ival::pt(b[0])), yc[1].sub(Ival::pt(b[1])));
            let db2 = mv(vx.sqr().add(vy.sqr()), vxc.sqr().add(vyc.sqr()));
            let (da, db) = (nonneg(da2).sqrt(), nonneg(db2).sqrt());
            if dd.hi == 0.0 {
                da
            } else {
                let cr = mv(ux.mul(D::c(d[1])).sub(uy.mul(D::c(d[0]))), uxc.scale(d[1]).sub(uyc.scale(d[0])));
                let perp = cr.abs().div(dd.sqrt());
                if tn.hi <= 0.0 {
                    da
                } else if tn.lo >= dd.hi {
                    db
                } else if tn.lo >= 0.0 && tn.hi <= dd.lo {
                    perp
                } else {
                    Ival { lo: perp.lo, hi: da.hi.min(db.hi) }
                }
            }
        }
        Planar::Arc { c: cc, r, a, b, ccw, .. } => {
            let (vx, vy) = (y[0].sub(D::c(cc[0])), y[1].sub(D::c(cc[1])));
            let (vxc, vyc) = (yc[0].sub(Ival::pt(cc[0])), yc[1].sub(Ival::pt(cc[1])));
            let n2 = mv(vx.sqr().add(vy.sqr()), vxc.sqr().add(vyc.sqr()));
            let circ = nonneg(n2).sqrt().sub(Ival::pt(r)).abs();
            let (ax, ay) = (a[0] - cc[0], a[1] - cc[1]);
            let (bx, by) = (b[0] - cc[0], b[1] - cc[1]);
            let c1 = mv(vy.mul(D::c(ax)).sub(vx.mul(D::c(ay))), vyc.scale(ax).sub(vxc.scale(ay)));
            let c2 = mv(vx.mul(D::c(by)).sub(vy.mul(D::c(bx))), vxc.scale(by).sub(vyc.scale(bx)));
            let (c1, c2) = if ccw { (c1, c2) } else { (c1.neg(), c2.neg()) };
            let ea = mv(y[0].sub(D::c(a[0])).sqr().add(y[1].sub(D::c(a[1])).sqr()), yc[0].sub(Ival::pt(a[0])).sqr().add(yc[1].sub(Ival::pt(a[1])).sqr()));
            let eb = mv(y[0].sub(D::c(b[0])).sqr().add(y[1].sub(D::c(b[1])).sqr()), yc[0].sub(Ival::pt(b[0])).sqr().add(yc[1].sub(Ival::pt(b[1])).sqr()));
            let (da, db) = (nonneg(ea).sqrt(), nonneg(eb).sqrt());
            let ends = Ival { lo: da.lo.min(db.lo), hi: da.hi.min(db.hi) };
            // the sector of an arc of at most a half turn: both crosses non-negative
            if c1.lo >= 0.0 && c2.lo >= 0.0 {
                circ
            } else if c1.hi < 0.0 || c2.hi < 0.0 {
                ends
            } else {
                Ival { lo: circ.lo, hi: ends.hi }
            }
        }
    };
    let sd = body.sd_rz(rho, zb);
    if point {
        return (sd, [0.0; 3]);
    }
    // the split: each parameter's share of the image's width (a heuristic only)
    let sm = [diam(&f.surf.eval(c.p, qc)), diam(&f.surf.eval(pc, c.q)), 0.0];
    (sd, sm)
}

fn nonneg(x: Ival) -> Ival {
    Ival { lo: x.lo.max(0.0), hi: x.hi.max(0.0) }
}

/// A binary min-heap of cells by their lower bound: the least bound is searched first, so a
/// violation is met early and a pass is proven where the bound is tightest last.
struct Heap {
    v: Vec<(f64, Cell)>,
}

impl Heap {
    fn push(&mut self, k: f64, c: Cell) {
        self.v.push((k, c));
        let mut i = self.v.len() - 1;
        while i > 0 {
            let p = (i - 1) / 2;
            if self.v[p].0 <= self.v[i].0 {
                break;
            }
            self.v.swap(p, i);
            i = p;
        }
    }
    fn pop(&mut self) -> Option<(f64, Cell)> {
        if self.v.is_empty() {
            return None;
        }
        let n = self.v.len() - 1;
        self.v.swap(0, n);
        let top = self.v.pop();
        let len = self.v.len();
        let mut i = 0;
        loop {
            let (l, r) = (2 * i + 1, 2 * i + 2);
            let mut m = i;
            if l < len && self.v[l].0 < self.v[m].0 {
                m = l;
            }
            if r < len && self.v[r].0 < self.v[m].0 {
                m = r;
            }
            if m == i {
                break;
            }
            self.v.swap(i, m);
            i = m;
        }
        top
    }
}

fn diam(b: &[Ival; 3]) -> f64 {
    let (a, c, d) = (b[0].width(), b[1].width(), b[2].width());
    (a * a + c * c + d * d).sqrt().next_up()
}

/// Certifies one (body, face, move) against `limit`, to resolution `res` (cells whose image is
/// smaller are leaves), spending at most `budget` cells.
pub fn certify(body: &Body, f: &Face, mv: &Move, limit: f64, res: f64, budget: usize) -> (Verdict, usize) {
    let lim_clear = limit + f.mu;
    let lim_hit = limit - f.mu;
    let s0 = if mv.planar().is_some() { Ival::pt(0.5) } else { Ival::new(0.0, 1.0) };
    let root = Cell { p: f.p, q: f.q, s: s0 };
    let mut heap = Heap { v: Vec::new() };
    heap.push(cell_sd(body, f, mv, &root).lo, root);
    let mut lower = f64::INFINITY;
    let mut spent = 0usize;
    let mut undecided = false;
    while let Some((key, c)) = heap.pop() {
        if key >= lim_clear {
            // the least bound left clears: so does every other cell
            if key < lower {
                lower = key;
            }
            break;
        }
        spent += 1;
        if spent > budget {
            return (Verdict::Undecided { lower: f64::NEG_INFINITY, cells: spent }, spent);
        }
        let holds = f.holds(c.p, c.q);
        if holds < 0 {
            continue;
        }
        let (sd, smear) = cell_eval(body, f, mv, &c);
        if sd.lo >= lim_clear {
            if sd.lo < lower {
                lower = sd.lo;
            }
            continue;
        }
        // a witness: the cell's centre, if it is surely on the face
        let (pm, qm, sm) = (c.p.mid(), c.q.mid(), c.s.mid());
        if f.holds(Ival::pt(pm), Ival::pt(qm)) == 1 {
            let w = cell_sd(body, f, mv, &Cell { p: Ival::pt(pm), q: Ival::pt(qm), s: Ival::pt(sm) });
            if w.hi < lim_hit {
                let y = f.surf.eval(Ival::pt(pm), Ival::pt(qm));
                let sm = if mv.planar().is_some() { mv.nearest_s([y[0].mid(), y[1].mid()]) } else { sm };
                return (Verdict::Hit { d: w.hi + f.mu, p: pm, q: qm, s: sm, at: [y[0].mid(), y[1].mid(), y[2].mid()] }, spent);
            }
        }
        // split the parameter that moves the image most
        let img = f.surf.eval(c.p, c.q);
        let mvb = mv.at(c.s);
        let size = diam(&[img[0].add(mvb[0]), img[1].add(mvb[1]), img[2].add(mvb[2])]);
        if size <= res {
            undecided = true;
            if sd.lo < lower {
                lower = sd.lo;
            }
            continue;
        }
        let (a, b) = split_by(&c, &smear);
        heap.push(cell_sd(body, f, mv, &a).lo, a);
        heap.push(cell_sd(body, f, mv, &b).lo, b);
    }
    if undecided {
        return (Verdict::Undecided { lower, cells: spent }, spent);
    }
    (Verdict::Clear { lower: lower - f.mu }, spent)
}

fn split_by(c: &Cell, sm: &[f64; 3]) -> (Cell, Cell) {
    let mut k = 0usize;
    let mut best = -1.0f64;
    let ws = [c.p.width(), c.q.width(), c.s.width()];
    let mut i = 0;
    while i < 3 {
        // a parameter with width but no smear still curves a little: count its width slightly
        let sc = if ws[i] > 0.0 { sm[i].max(1e-9 * ws[i]) } else { -1.0 };
        if sc > best {
            best = sc;
            k = i;
        }
        i += 1;
    }
    match k {
        0 => {
            let (x, y) = c.p.split();
            (Cell { p: x, ..*c }, Cell { p: y, ..*c })
        }
        1 => {
            let (x, y) = c.q.split();
            (Cell { q: x, ..*c }, Cell { q: y, ..*c })
        }
        _ => {
            let (x, y) = c.s.split();
            (Cell { s: x, ..*c }, Cell { s: y, ..*c })
        }
    }
}

/// The halving that raises the lower bound most (the least of the two halves'), among the
/// parameters with width: the search spends its cells where the bound is loose, not where the
/// image is merely wide.
#[allow(dead_code)]
fn best_split(body: &Body, f: &Face, mv: &Move, c: &Cell) -> (Cell, Cell) {
    let mut best: Option<(f64, Cell, Cell)> = None;
    let mut k = 0;
    while k < 3 {
        let w = match k {
            0 => c.p.width(),
            1 => c.q.width(),
            _ => c.s.width(),
        };
        if w > 0.0 {
            let (a, b) = match k {
                0 => {
                    let (x, y) = c.p.split();
                    (Cell { p: x, ..*c }, Cell { p: y, ..*c })
                }
                1 => {
                    let (x, y) = c.q.split();
                    (Cell { q: x, ..*c }, Cell { q: y, ..*c })
                }
                _ => {
                    let (x, y) = c.s.split();
                    (Cell { s: x, ..*c }, Cell { s: y, ..*c })
                }
            };
            let la = cell_sd(body, f, mv, &a).lo;
            let lb = cell_sd(body, f, mv, &b).lo;
            let m = if la < lb { la } else { lb };
            let better = match &best {
                None => true,
                Some((bm, _, _)) => m > *bm,
            };
            if better {
                best = Some((m, a, b));
            }
        }
        k += 1;
    }
    match best {
        Some((_, a, b)) => (a, b),
        None => (*c, *c),
    }
}

#[allow(dead_code)]
fn split_dim(f: &Face, mv: &Move, c: &Cell) -> usize {
    let (pm, qm, sm) = (Ival::pt(c.p.mid()), Ival::pt(c.q.mid()), Ival::pt(c.s.mid()));
    let wp = if c.p.width() > 0.0 { diam(&f.surf.eval(c.p, qm)) } else { 0.0 };
    let wq = if c.q.width() > 0.0 { diam(&f.surf.eval(pm, c.q)) } else { 0.0 };
    let ws = if c.s.width() > 0.0 { diam(&mv.at(c.s)) } else { 0.0 };
    let _ = sm;
    if wp >= wq && wp >= ws {
        0
    } else if wq >= ws {
        1
    } else {
        2
    }
}

/// The swept box of the tool's bounding cylinder along a move.
pub fn swept_box(mv: &Move, rmax: f64, top: f64) -> [Ival; 3] {
    let g = mv.at(Ival::new(0.0, 1.0));
    [
        g[0].add(Ival::new(-rmax, rmax)),
        g[1].add(Ival::new(-rmax, rmax)),
        g[2].add(Ival::new(0.0, top)),
    ]
}

/// Whether two boxes are farther apart than `gap` along some axis (then every pair of points
/// is): a certified cheap clear.
pub fn apart(a: &[Ival; 3], b: &[Ival; 3], gap: f64) -> bool {
    let mut k = 0;
    while k < 3 {
        if Ival::pt(b[k].lo).sub(Ival::pt(a[k].hi)).lo > gap || Ival::pt(a[k].lo).sub(Ival::pt(b[k].hi)).lo > gap {
            return true;
        }
        k += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{Cutter, assemble};

    fn wall(x: f64) -> Face {
        // the plane x = const, facing −x, over y in [−50, 50], z in [−20, 0]
        Face::new(
            Surf::Plane { o: [x, 0.0, 0.0], u: [0.0, 1.0, 0.0], v: [0.0, 0.0, 1.0] },
            Ival::new(-50.0, 50.0),
            Ival::new(-20.0, 0.0),
            None,
            0.0,
            0,
        )
    }

    #[test]
    fn tangent_pass_clears_by_the_tolerance_and_a_gouge_is_found() {
        let t = assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap();
        let cut = &t.bodies[0];
        let f = wall(10.0);
        // the tool's side runs along the wall, exactly touching it (x = 10 − 3)
        let mv = Move::Line { a: [7.0, -20.0, -5.0], b: [7.0, 20.0, -5.0] };
        let (v, n) = certify(cut, &f, &mv, -0.001, 1e-4, 1_000);
        assert!(matches!(v, Verdict::Clear { lower } if lower >= -0.001), "{v:?} {n}");
        // 0.01 mm into the wall: a gouge, with a witness
        let mv = Move::Line { a: [7.01, -20.0, -5.0], b: [7.01, 20.0, -5.0] };
        let (v, _) = certify(cut, &f, &mv, -0.001, 1e-4, 1_000_000);
        assert!(matches!(v, Verdict::Hit { d, .. } if d < -0.001), "{v:?}");
        // a diagonal move that only touches at its end
        let mv = Move::Line { a: [0.0, 0.0, -5.0], b: [7.02, 3.0, -5.0] };
        let (v, _) = certify(cut, &f, &mv, -0.001, 1e-4, 1_000_000);
        assert!(matches!(v, Verdict::Hit { s, .. } if s > 0.9), "{v:?}");
    }

    #[test]
    fn around_a_boss_on_an_arc() {
        // a cylindrical boss of radius 10 about the z axis, z in [−10, 0]; the tool's centre goes
        // round it on radius 13 (tangent), then 12.99 (a gouge)
        let t = assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap();
        let cut = &t.bodies[0];
        let boss = Face::new(
            Surf::Cyl { o: [0.0, 0.0, -10.0], x: [1.0, 0.0, 0.0], y: [0.0, 1.0, 0.0], ax: [0.0, 0.0, 1.0], r: 10.0 },
            Ival::new(-1.0, 1.0),
            Ival::new(0.0, 10.0),
            None,
            0.0,
            1,
        );
        let mv = |r: f64| Move::Arc { c: [0.0, 0.0, -5.0], x: [1.0, 0.0, 0.0], y: [0.0, 1.0, 0.0], r, t0: -0.5, t1: 0.5 };
        let (v, n) = certify(cut, &boss, &mv(13.0), -0.01, 2e-3, 20_000);
        assert!(matches!(v, Verdict::Clear { .. }), "{v:?} {n}");
        let (v, _) = certify(cut, &boss, &mv(12.98), -0.01, 2e-3, 2_000_000);
        assert!(matches!(v, Verdict::Hit { .. }), "{v:?}");
    }
}
