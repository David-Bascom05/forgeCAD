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
    Cone {
        o: V3,
        x: V3,
        y: V3,
        ax: V3,
        r0: f64,
        k: f64,
    },
    /// A sphere about o, radius r: point = o + r (cos φ (c x + s y) + sin φ ax) with the same
    /// rational map for both angles: p = t (azimuth), q = tan(φ/2) (elevation).
    Sphere { o: V3, x: V3, y: V3, ax: V3, r: f64 },
    /// A torus: o + (R + r cos φ)(c x + s y) + r sin φ ax; p = t, q = tan(ψ/2) with
    /// φ = φm + ψ, (cm, sm) = (cos φm, sin φm): the tube angle about the face's middle.
    Torus {
        o: V3,
        x: V3,
        y: V3,
        ax: V3,
        big: f64,
        r: f64,
        cm: f64,
        sm: f64,
    },
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
    /// Its form as a prism along z, when it has one (slab.rs).
    pub prism: Option<crate::slab::Prism>,
    /// The trim's pieces' end gaps (`arc_gap`; 0 for a line), worked out once by `Face::new`
    /// for `holds`: the same values `TrimRef` would work out on each call; with the pieces they
    /// were worked out for (the fields are public: a trim changed since is not taken as these).
    tgaps: (Vec<TSeg>, Vec<f64>),
}

/// A planar face's trim: closed polygons and arcs in (p, q), as f64 polylines whose arcs are
/// given exactly (centre and radius), with the inside on the left.
#[derive(Clone, Debug, PartialEq)]
pub struct Trim {
    pub segs: Vec<TSeg>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TSeg {
    Line {
        a: [f64; 2],
        b: [f64; 2],
    },
    /// An arc of less than a half turn.
    Arc {
        a: [f64; 2],
        b: [f64; 2],
        c: [f64; 2],
        r: f64,
        ccw: bool,
    },
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
                add3(
                    add3(pt3(o), mul3(rad, cf.mul(T::c(r)))),
                    scale3(ax, sf.mul(T::c(r))),
                )
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
                let (c, s) = circ(p);
                let (cp, sp) = circ(q);
                // cos(φm + ψ), sin(φm + ψ)
                let cf = cp.mul(T::c(cm)).sub(sp.mul(T::c(sm)));
                let sf = cp.mul(T::c(sm)).add(sp.mul(T::c(cm)));
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
        let mut f = Face {
            surf,
            p,
            q,
            trim,
            mu,
            bbox,
            id,
            prism: None,
            tgaps: (vec![], vec![]),
        };
        f.prism = crate::slab::Prism::of(&f);
        if let Some(t) = &f.trim {
            f.tgaps.0 = t.segs.clone();
            f.tgaps.1 = t
                .segs
                .iter()
                .map(|sg| match *sg {
                    TSeg::Line { .. } => 0.0,
                    TSeg::Arc { a, b, c, r, .. } => arc_gap(a, b, c, r),
                })
                .collect();
        }
        f
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
                // the gaps worked out once (if the trim was changed since: worked out here)
                let gaps = if self.tgaps.0 == t.segs {
                    Some(&self.tgaps.1[..])
                } else {
                    None
                };
                let s = TrimRef { segs: &t.segs, gaps }.classify(p, q);
                if s == 1 && !in_box { 0 } else { s }
            }
        }
    }
}

impl Trim {
    /// 1: the box is inside the trim region; -1: outside; 0: unknown.
    pub fn classify(&self, p: Ival, q: Ival) -> i32 {
        TrimRef {
            segs: &self.segs,
            gaps: None,
        }
        .classify(p, q)
    }

    #[cfg(test)]
    fn ray_side(&self, x: f64, y: f64, d: [f64; 2]) -> i32 {
        TrimRef {
            segs: &self.segs,
            gaps: None,
        }
        .ray_side(x, y, d)
    }
}

/// A trim's pieces borrowed, with their end gaps when the caller has them worked out (a gap
/// below 0 or none: worked out here).
pub struct TrimRef<'a> {
    pub segs: &'a [TSeg],
    pub gaps: Option<&'a [f64]>,
}

impl TrimRef<'_> {
    fn gap(&self, i: usize) -> f64 {
        if let Some(g) = self.gaps
            && g[i] >= 0.0
        {
            return g[i];
        }
        match self.segs[i] {
            TSeg::Line { .. } => 0.0,
            TSeg::Arc { a, b, c, r, .. } => arc_gap(a, b, c, r),
        }
    }

    /// 1: the box is inside the trim region; -1: outside; 0: unknown.
    pub fn classify(&self, p: Ival, q: Ival) -> i32 {
        // the box misses the boundary: then all of it is on the side of its centre
        let mut lo = f64::INFINITY;
        let mut i = 0;
        while i < self.segs.len() {
            let d = tseg_dist(&self.segs[i], p, q, self.gap(i));
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

    /// Ray casting from an f64 point, every decision in intervals: 1 in, -1 out, 0 unknown. The
    /// ray goes along each of a fixed list of directions (exact Pythagorean unit vectors) until
    /// one decides: a ray through a vertex leaves its crossing undecided, never miscounted.
    fn point_side(&self, x: f64, y: f64) -> i32 {
        const DIRS: [[f64; 2]; 5] = [[1.0, 0.0], [0.6, 0.8], [-0.8, 0.6], [0.28, -0.96], [-0.96, -0.28]];
        let mut k = 0;
        while k < DIRS.len() {
            let s = self.ray_side(x, y, DIRS[k]);
            if s != 0 {
                return s;
            }
            k += 1;
        }
        0
    }

    /// One ray: coordinates turned so the ray is +x' from the point (x' = d·v, y' = d × v).
    fn ray_side(&self, x: f64, y: f64, d: [f64; 2]) -> i32 {
        let (dx, dy) = (Ival::pt(d[0]), Ival::pt(d[1]));
        let rot = |p: [f64; 2]| -> [Ival; 2] {
            let (vx, vy) = (Ival::pt(p[0]).sub(Ival::pt(x)), Ival::pt(p[1]).sub(Ival::pt(y)));
            [dx.mul(vx).add(dy.mul(vy)), dx.mul(vy).sub(dy.mul(vx))]
        };
        let mut odd = false;
        let axis = d[0] == 1.0 && d[1] == 0.0;
        let mut i = 0;
        while i < self.segs.len() {
            // along +x: a piece wholly above, below or behind the ray cannot cross it
            if axis {
                let mut bx = crate::slab::tseg_box(&self.segs[i]);
                if let TSeg::Arc { a, b, .. } = self.segs[i] {
                    // the ends, which may lie off the circle by the gap
                    bx = [
                        bx[0].min(a[0]).min(b[0]),
                        bx[1].min(a[1]).min(b[1]),
                        bx[2].max(a[0]).max(b[0]),
                        bx[3].max(a[1]).max(b[1]),
                    ];
                }
                if bx[1] > y || bx[3] < y || bx[2] < x {
                    i += 1;
                    continue;
                }
            }
            match self.segs[i] {
                TSeg::Line { a, b } => {
                    let (ra, rb) = (rot(a), rot(b));
                    let sa = ra[1].sign();
                    let sb = rb[1].sign();
                    if sa == 0 || sb == 0 {
                        // an end on the ray's line: this direction cannot say
                        if (ra[1].lo <= 0.0 && ra[1].hi >= 0.0 && ra[0].hi >= 0.0)
                            || (rb[1].lo <= 0.0 && rb[1].hi >= 0.0 && rb[0].hi >= 0.0)
                        {
                            return 0;
                        }
                    } else if sa != sb {
                        // where it crosses y' = 0: x' = xa − ya (xb − xa)/(yb − ya)
                        let xc = ra[0].sub(ra[1].mul(rb[0].sub(ra[0])).div(rb[1].sub(ra[1])));
                        let s = xc.sign();
                        if s == 0 {
                            return 0;
                        }
                        if s > 0 {
                            odd = !odd;
                        }
                    }
                }
                TSeg::Arc { a, b, c, r, ccw } => {
                    let (ra, rb, rc) = (rot(a), rot(b), rot(c));
                    // an end, or the radial piece joining it to the circle, on the ray's line
                    // (within the gap of the end): undecided here
                    let _ = (c, r);
                    let g = self.gap(i);
                    let near = |e: [Ival; 2]| e[1].lo <= g && e[1].hi >= -g && e[0].hi >= -g;
                    if near(ra) || near(rb) {
                        return 0;
                    }
                    let h2 = Ival::pt(r).sqr().sub(rc[1].sqr());
                    if h2.hi >= 0.0 {
                        if h2.lo <= 0.0 {
                            return 0;
                        }
                        let h = h2.sqrt();
                        let xs = [rc[0].add(h), rc[0].sub(h)];
                        let mut k = 0;
                        while k < 2 {
                            let xk = xs[k];
                            let (vx, vy) = (xk.sub(rc[0]), rc[1].neg());
                            let ua = [ra[0].sub(rc[0]), ra[1].sub(rc[1])];
                            let ub = [rb[0].sub(rc[0]), rb[1].sub(rc[1])];
                            let c1 = ua[0].mul(vy).sub(ua[1].mul(vx));
                            let c2 = vx.mul(ub[1]).sub(vy.mul(ub[0]));
                            let (c1, c2) = if ccw { (c1, c2) } else { (c1.neg(), c2.neg()) };
                            let ins = c1.lo > 0.0 && c2.lo > 0.0;
                            let out = c1.hi < 0.0 || c2.hi < 0.0;
                            if !ins && !out {
                                return 0;
                            }
                            if ins {
                                let right = xk.sign();
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

/// How far a trim arc's stored ends may lie off its circle (|r − |a − c||, |r − |b − c||, an
/// upper bound): the trim is the circle's arc plus the radial pieces from it to those ends, so
/// that it closes exactly with its neighbours (which share the ends bit for bit).
pub(crate) fn arc_gap(a: [f64; 2], b: [f64; 2], c: [f64; 2], r: f64) -> f64 {
    let d = |p: [f64; 2]| {
        Ival::pt(p[0])
            .sub(Ival::pt(c[0]))
            .sqr()
            .add(Ival::pt(p[1]).sub(Ival::pt(c[1])).sqr())
            .sqrt()
            .sub(Ival::pt(r))
            .abs()
            .hi
    };
    d(a).max(d(b))
}

/// A lower bound on the distance from the box to the trim segment (a trim arc's radial end
/// pieces included).
fn tseg_dist(s: &TSeg, p: Ival, q: Ival, gap: f64) -> f64 {
    let seg = match *s {
        TSeg::Line { a, b } => crate::tool::PSeg::Line { a, b },
        TSeg::Arc { a, b, c, r, ccw } => crate::tool::PSeg::Arc { a, b, c, r, ccw },
    };
    Ival::pt(crate::tool::seg_dist(&seg, p, q).lo)
        .sub(Ival::pt(gap))
        .lo
}

/// A move of the tool tip (3-axis: the tool is translated, its axis +z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Move {
    Line {
        a: V3,
        b: V3,
    },
    /// A planar arc in z = const: centre c, radius r, from the direction x (unit) by the
    /// rational map over t in [t0, t1] (each |t| < 1 + small, so less than a half turn either
    /// side), in the frame (x, y).
    Arc {
        c: V3,
        x: V3,
        y: V3,
        r: f64,
        t0: f64,
        t1: f64,
    },
    /// A helical piece (an arc of less than a half turn whose height goes from z0 to z1, as a
    /// control interpolates it, linearly in the angle): the arc as `Arc` (c's height unused).
    /// The checker takes the arc swept over every height between z0 and z1, a superset of the
    /// helix; `at` encloses heights by that range (its z is linear in s, which is the
    /// machine's height only at the ends: only enclosures over all of [0, 1] are taken). The
    /// general branch and bound takes the arc at each instant at every height of the range
    /// (`Move::enclose`).
    Helix {
        c: V3,
        x: V3,
        y: V3,
        r: f64,
        t0: f64,
        t1: f64,
        z0: f64,
        z1: f64,
    },
}

impl Move {
    pub fn at(&self, s: Ival) -> [Ival; 3] {
        self.at_n(s)
    }

    /// An enclosure of the move's positions over the instants s: `at`, except that a helix's
    /// height is its whole range (its z is linear in s only at the ends).
    pub fn enclose(&self, s: Ival) -> [Ival; 3] {
        let mut g = self.at(s);
        if let Some(z) = self.helix_heights() {
            g[2] = z;
        }
        g
    }

    /// A helix's range of heights, None for any other move.
    fn helix_heights(&self) -> Option<Ival> {
        match *self {
            Move::Helix { z0, z1, .. } => Some(Ival::new(z0.min(z1), z0.max(z1))),
            _ => None,
        }
    }

    pub fn at_n<T: Num>(&self, s: T) -> [T; 3] {
        match *self {
            Move::Line { a, b } => {
                let d = [
                    T::c(b[0]).sub(T::c(a[0])),
                    T::c(b[1]).sub(T::c(a[1])),
                    T::c(b[2]).sub(T::c(a[2])),
                ];
                [
                    T::c(a[0]).add(s.mul(d[0])),
                    T::c(a[1]).add(s.mul(d[1])),
                    T::c(a[2]).add(s.mul(d[2])),
                ]
            }
            Move::Arc { c, x, y, r, t0, t1 } => {
                let t = T::c(t0).add(s.mul(T::c(t1).sub(T::c(t0))));
                let (cc, ss) = circ(t);
                let rad = add3(scale3(x, cc), scale3(y, ss));
                add3(pt3(c), mul3(rad, T::c(r)))
            }
            Move::Helix {
                c,
                x,
                y,
                r,
                t0,
                t1,
                z0,
                z1,
            } => {
                let t = T::c(t0).add(s.mul(T::c(t1).sub(T::c(t0))));
                let (cc, ss) = circ(t);
                let rad = add3(scale3(x, cc), scale3(y, ss));
                let p = add3(pt3([c[0], c[1], 0.0]), mul3(rad, T::c(r)));
                [p[0], p[1], T::c(z0).add(s.mul(T::c(z1).sub(T::c(z0))))]
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    Clear {
        lower: f64,
    },
    /// A real face point (params p, q) at a real instant s whose signed distance is at most d
    /// (< limit), at `at` (part frame).
    Hit {
        d: f64,
        p: f64,
        q: f64,
        s: f64,
        at: V3,
    },
    Undecided {
        lower: f64,
        cells: usize,
    },
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
fn cell_sd(body: &Body, f: &Face, mv: &Move, c: &Cell, pg: f64) -> Ival {
    cell_eval(body, f, mv, c, pg).0
}

/// The signed distance over the cell, and each parameter's smear (how much of the enclosure's
/// width it accounts for: |∂(ρ², z)| times its width), which picks what to split.
fn cell_eval(body: &Body, f: &Face, mv: &Move, c: &Cell, pg: f64) -> (Ival, [f64; 3]) {
    if let Some(pl) = mv.planar()
        && !matches!(pl, Planar::Vertical { .. } | Planar::Helix { .. })
    {
        return cell_eval_planar(body, f, &pl, c, pg);
    }
    let x = [c.p, c.q, c.s];
    // at the centre
    let (pc, qc, sc) = (Ival::pt(c.p.mid()), Ival::pt(c.q.mid()), Ival::pt(c.s.mid()));
    let yc = f.surf.eval(pc, qc);
    // a helix: its arc at the instant, at every height of its range (a superset of the helix,
    // whose height at an instant is somewhere in that range); the height is a constant of the
    // cell, so the mean-value form below holds for each height and so over all of them
    let gc = mv.enclose(sc);
    let rc = [yc[0].sub(gc[0]), yc[1].sub(gc[1]), yc[2].sub(gc[2])];
    let r2c = rc[0].sqr().add(rc[1].sqr());
    if c.p.width() == 0.0 && c.q.width() == 0.0 && c.s.width() == 0.0 {
        return (body.sd_rz(r2c.sqrt(), rc[2]), [0.0; 3]);
    }
    // over the box, with derivatives
    let y: [D; 3] = f.surf.eval_n(D::var(c.p, 0), D::var(c.q, 1));
    let mut g: [D; 3] = mv.at_n(D::var(c.s, 2));
    if let Some(z) = mv.helix_heights() {
        g[2] = D {
            v: z,
            g: [Ival::pt(0.0); 3],
        };
    }
    let rel = [y[0].sub(g[0]), y[1].sub(g[1]), y[2].sub(g[2])];
    let r2 = rel[0].sqr().add(rel[1].sqr());
    let r2b = mean_value(&r2, r2c, x);
    let zb = mean_value(&rel[2], rc[2], x);
    let r2b = Ival {
        lo: r2b.lo.max(0.0),
        hi: r2b.hi,
    };
    // ρ's width per unit of ρ²'s near ρ: 1 / (2ρ), at least 1 / (2 ρmax); z counts as is
    let inv = if r2c.lo > 1e-12 { 0.5 / r2c.lo.sqrt() } else { 1.0 };
    // how much the signed distance moves with ρ and with z at the centre (a heuristic for the
    // split only: soundness never rests on it)
    let (rm, zm) = (r2c.mid().max(0.0).sqrt(), rc[2].mid());
    let h = 1e-6 * (1.0 + rm.abs() + zm.abs());
    let s0 = body.sd_rz(Ival::pt(rm), Ival::pt(zm)).mid();
    let nr = ((body.sd_rz(Ival::pt(rm + h), Ival::pt(zm)).mid() - s0) / h)
        .abs()
        .min(1.0);
    let nz = ((body.sd_rz(Ival::pt(rm), Ival::pt(zm + h)).mid() - s0) / h)
        .abs()
        .min(1.0);
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
    Line {
        a: [f64; 2],
        b: [f64; 2],
        z: f64,
    },
    /// A vertical move: the tip at one xy, its height from z0 to z1 (either order).
    Vertical {
        p: [f64; 2],
        z0: f64,
        z1: f64,
    },
    Arc {
        c: [f64; 2],
        r: f64,
        a: [f64; 2],
        b: [f64; 2],
        ccw: bool,
        z: f64,
    },
    /// The arc swept over every height from z0 to z1 (a helical piece's superset).
    Helix {
        c: [f64; 2],
        r: f64,
        a: [f64; 2],
        b: [f64; 2],
        ccw: bool,
        z0: f64,
        z1: f64,
    },
}

impl Move {
    /// The move as a planar path, when the tip keeps one height. A planar arc's ends are taken
    /// from the rational map at t0 and t1 in intervals, so the sector test uses enclosures of
    /// them: the path is the exact arc between the exact ends.
    pub fn planar(&self) -> Option<Planar> {
        match *self {
            Move::Line { a, b } => {
                if a[2] == b[2] {
                    Some(Planar::Line {
                        a: [a[0], a[1]],
                        b: [b[0], b[1]],
                        z: a[2],
                    })
                } else if a[0] == b[0] && a[1] == b[1] {
                    Some(Planar::Vertical {
                        p: [a[0], a[1]],
                        z0: a[2],
                        z1: b[2],
                    })
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
                let (a, b) = ([e0[0].mid(), e0[1].mid()], [e1[0].mid(), e1[1].mid()]);
                // the closed forms take arcs proven strictly under a half turn in their sense
                // (cross(a − c, b − c) of the sign, in intervals: DistArc.lean); a vanishing arc
                // (a near-tangent corner's, ~1e-17 rad) is not provably so: no planar form, the
                // branch and bound takes it
                let (ua, ub) = (
                    [
                        Ival::pt(a[0]).sub(Ival::pt(c[0])),
                        Ival::pt(a[1]).sub(Ival::pt(c[1])),
                    ],
                    [
                        Ival::pt(b[0]).sub(Ival::pt(c[0])),
                        Ival::pt(b[1]).sub(Ival::pt(c[1])),
                    ],
                );
                let cr = ua[0].mul(ub[1]).sub(ua[1].mul(ub[0]));
                if !(if ccw { cr.lo > 0.0 } else { cr.hi < 0.0 }) {
                    return None;
                }
                Some(Planar::Arc {
                    c: [c[0], c[1]],
                    r,
                    a,
                    b,
                    ccw,
                    z: c[2],
                })
            }
            Move::Helix {
                c,
                x,
                y,
                r,
                t0,
                t1,
                z0,
                z1,
            } => {
                let arc = Move::Arc { c, x, y, r, t0, t1 };
                match arc.planar()? {
                    Planar::Arc { c, r, a, b, ccw, .. } => Some(Planar::Helix {
                        c,
                        r,
                        a,
                        b,
                        ccw,
                        z0,
                        z1,
                    }),
                    _ => None,
                }
            }
        }
    }
}

impl Move {
    /// How far the planar path's arc (`planar()`'s, its ends the f64 midpoints of their
    /// enclosures) may misstate distances to the move's true arc, for an arc or helix (0 for a
    /// line, whose ends are exact). The candidates (DistArc, Trim) bound distances to the model
    /// M: the circle's arc between the rays through the stored ends a, b, plus the radial pieces
    /// to them, once lowered by 2·g, g = arc_gap(a, b, c, r). A true end A₀ is within ε of a (ε
    /// from the enclosure's half-widths), so the point a′ = c + r·(a − c)/|a − c| of M is within
    /// ε + g ≤ 2ε of A₀; a point of the true arc beyond a's ray lies on the circle between A₀
    /// and a′, so within |A₀ − a′| ≤ 2ε of a′; one inside the rays is on M. So every point of the
    /// true arc is within 2ε of M, and every point of M within 2ε of the true arc (the same
    /// argument from a′'s side), and distances to either differ by at most 2ε: lowering by
    /// 2g + 2ε (raising by 2ε) bounds the true arc's. Outward-rounded; +∞ if not finite.
    pub fn path_gap(&self) -> f64 {
        let arc = match *self {
            Move::Line { .. } => return 0.0,
            Move::Arc { c, x, y, r, t0, t1 }
            | Move::Helix {
                c, x, y, r, t0, t1, ..
            } => Move::Arc { c, x, y, r, t0, t1 },
        };
        let Some(Planar::Arc { c, r, a, b, .. }) = arc.planar() else {
            return f64::INFINITY;
        };
        let mut eps = 0.0f64;
        for s in [0.0, 1.0] {
            let e = arc.at(Ival::pt(s));
            let hw = |v: Ival| {
                let m = Ival::pt(v.mid());
                Ival::pt(v.hi).sub(m).hi.max(m.sub(Ival::pt(v.lo)).hi).max(0.0)
            };
            let (hx, hy) = (hw(e[0]), hw(e[1]));
            eps = eps.max(Ival::pt(hx).sqr().add(Ival::pt(hy).sqr()).sqrt().hi);
        }
        let g = arc_gap(a, b, c, r);
        let pg = Ival::pt(2.0 * g).add(Ival::pt(2.0 * eps)).hi;
        if pg.is_finite() { pg } else { f64::INFINITY }
    }

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
            Move::Arc { c, x, y, t0, t1, .. } | Move::Helix { c, x, y, t0, t1, .. } => {
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
fn cell_eval_planar(body: &Body, f: &Face, pl: &Planar, c: &Cell, pg: f64) -> (Ival, [f64; 3]) {
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
        Planar::Vertical { .. } | Planar::Helix { .. } => {
            unreachable!("vertical moves take the general cells; helices are not taken there")
        }
    };
    let _ = zp;
    let zb = mv(y[2].sub(D::c(z_at)), yc[2].sub(Ival::pt(z_at)));
    let rho = match *pl {
        Planar::Line { a, b, .. } => {
            let d = [b[0] - a[0], b[1] - a[1]];
            let dd = Ival::pt(d[0]).sqr().add(Ival::pt(d[1]).sqr());
            let (ux, uy) = (y[0].sub(D::c(a[0])), y[1].sub(D::c(a[1])));
            let (uxc, uyc) = (yc[0].sub(Ival::pt(a[0])), yc[1].sub(Ival::pt(a[1])));
            let tn = mv(
                ux.mul(D::c(d[0])).add(uy.mul(D::c(d[1]))),
                uxc.scale(d[0]).add(uyc.scale(d[1])),
            );
            let da2 = mv(ux.sqr().add(uy.sqr()), uxc.sqr().add(uyc.sqr()));
            let (vx, vy) = (y[0].sub(D::c(b[0])), y[1].sub(D::c(b[1])));
            let (vxc, vyc) = (yc[0].sub(Ival::pt(b[0])), yc[1].sub(Ival::pt(b[1])));
            let db2 = mv(vx.sqr().add(vy.sqr()), vxc.sqr().add(vyc.sqr()));
            let (da, db) = (nonneg(da2).sqrt(), nonneg(db2).sqrt());
            if dd.hi == 0.0 {
                da
            } else {
                let cr = mv(
                    ux.mul(D::c(d[1])).sub(uy.mul(D::c(d[0]))),
                    uxc.scale(d[1]).sub(uyc.scale(d[0])),
                );
                let perp = cr.abs().div(dd.sqrt());
                if tn.hi <= 0.0 {
                    da
                } else if tn.lo >= dd.hi {
                    db
                } else if tn.lo >= 0.0 && tn.hi <= dd.lo {
                    perp
                } else {
                    Ival {
                        lo: perp.lo,
                        hi: da.hi.min(db.hi),
                    }
                }
            }
        }
        Planar::Vertical { .. } | Planar::Helix { .. } => unreachable!(),
        Planar::Arc {
            c: cc, r, a, b, ccw, ..
        } => {
            let (vx, vy) = (y[0].sub(D::c(cc[0])), y[1].sub(D::c(cc[1])));
            let (vxc, vyc) = (yc[0].sub(Ival::pt(cc[0])), yc[1].sub(Ival::pt(cc[1])));
            let n2 = mv(vx.sqr().add(vy.sqr()), vxc.sqr().add(vyc.sqr()));
            let circ = nonneg(n2).sqrt().sub(Ival::pt(r)).abs();
            let (ax, ay) = (a[0] - cc[0], a[1] - cc[1]);
            let (bx, by) = (b[0] - cc[0], b[1] - cc[1]);
            let c1 = mv(
                vy.mul(D::c(ax)).sub(vx.mul(D::c(ay))),
                vyc.scale(ax).sub(vxc.scale(ay)),
            );
            let c2 = mv(
                vx.mul(D::c(by)).sub(vy.mul(D::c(bx))),
                vxc.scale(by).sub(vyc.scale(bx)),
            );
            let (c1, c2) = if ccw { (c1, c2) } else { (c1.neg(), c2.neg()) };
            let ea = mv(
                y[0].sub(D::c(a[0])).sqr().add(y[1].sub(D::c(a[1])).sqr()),
                yc[0]
                    .sub(Ival::pt(a[0]))
                    .sqr()
                    .add(yc[1].sub(Ival::pt(a[1])).sqr()),
            );
            let eb = mv(
                y[0].sub(D::c(b[0])).sqr().add(y[1].sub(D::c(b[1])).sqr()),
                yc[0]
                    .sub(Ival::pt(b[0]))
                    .sqr()
                    .add(yc[1].sub(Ival::pt(b[1])).sqr()),
            );
            let (da, db) = (nonneg(ea).sqrt(), nonneg(eb).sqrt());
            let ends = Ival {
                lo: da.lo.min(db.lo),
                hi: da.hi.min(db.hi),
            };
            // the sector of an arc of at most a half turn: both crosses non-negative
            if c1.lo >= 0.0 && c2.lo >= 0.0 {
                circ
            } else if c1.hi < 0.0 || c2.hi < 0.0 {
                ends
            } else {
                Ival {
                    lo: circ.lo,
                    hi: ends.hi,
                }
            }
        }
    };
    // the path arc's stored ends: its distances widened by `pg` (Move::path_gap)
    let rho = if pg > 0.0 {
        Ival {
            lo: Ival::pt(rho.lo).sub(Ival::pt(pg)).lo.max(0.0),
            hi: Ival::pt(rho.hi).add(Ival::pt(pg)).hi,
        }
    } else {
        rho
    };
    let sd = body.sd_rz(rho, zb);
    if point {
        return (sd, [0.0; 3]);
    }
    // the split: how much the signed distance changes across the cell along each parameter,
    // at the centre, plus a little of the image's width (a heuristic only)
    let at = |p: f64, q: f64| {
        cell_eval_planar(
            body,
            f,
            pl,
            &Cell {
                p: Ival::pt(p),
                q: Ival::pt(q),
                s: c.s,
            },
            pg,
        )
        .0
        .mid()
    };
    let (pm, qm) = (c.p.mid(), c.q.mid());
    let (hp, hq) = (0.5 * c.p.width(), 0.5 * c.q.width());
    let dp = (at(pm + hp, qm) - at(pm - hp, qm)).abs()
        + (at(pm + hp, qm) + at(pm - hp, qm) - 2.0 * at(pm, qm)).abs();
    let dq = (at(pm, qm + hq) - at(pm, qm - hq)).abs()
        + (at(pm, qm + hq) + at(pm, qm - hq) - 2.0 * at(pm, qm)).abs();
    let sm = [
        dp + 1e-4 * diam(&f.surf.eval(c.p, qc)),
        dq + 1e-4 * diam(&f.surf.eval(pc, c.q)),
        0.0,
    ];
    (sd, sm)
}

fn nonneg(x: Ival) -> Ival {
    Ival {
        lo: x.lo.max(0.0),
        hi: x.hi.max(0.0),
    }
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
    certify_shared(body, f, mv, limit, res, budget, &mut None)
}

/// `certify` for several bodies of one tool on the same move and face: `shared` carries the
/// move's distance to the face's section from one body's call to the next (start it at None
/// for each move and face).
#[allow(clippy::too_many_arguments)]
pub fn certify_shared(
    body: &Body,
    f: &Face,
    mv: &Move,
    limit: f64,
    res: f64,
    budget: usize,
    shared: &mut Option<(Ival, f64)>,
) -> (Verdict, usize) {
    // a line move of a capsule-shaped body (a ball end mill's cutter) against a sphere, a
    // cylinder that is not a prism, or a plane that is not a prism on a move that is not
    // planar: closed form (contact.rs), which proves Clear or passes on. Those pairs are 3D
    // finishing's; the 2.5D ops' pairs (prisms, planar moves on planes) keep their paths
    let fast = match f.surf {
        Surf::Sphere { .. } => true,
        Surf::Cyl { .. } => f.prism.is_none(),
        Surf::Plane { .. } => f.prism.is_none() && mv.planar().is_none(),
        Surf::Torus { .. } | Surf::Cone { .. } => f.prism.is_none(),
    };
    // (an early Clear leaves `shared` as it was: the next body's call recomputes what it needs)
    if fast && let Some(v) = crate::contact::clear(body, f, mv, limit) {
        return (v, 0);
    }
    // a planar move against a face that is a prism along z: closed form (slab.rs)
    if let (Some(pl), Some(pr)) = (mv.planar(), f.prism.as_ref()) {
        let (v, n) = crate::slab::certify_shared(body, f, pr, &pl, limit, res, budget, shared, mv.path_gap());
        // a helix is taken as its arc swept over every height it passes, a superset of the
        // move: a Hit there need not be a point the helix reaches, so it is no witness (no
        // lower bound either): undecided, refused without a witness
        if matches!(mv, Move::Helix { .. }) && matches!(v, Verdict::Hit { .. }) {
            return (
                Verdict::Undecided {
                    lower: f64::NEG_INFINITY,
                    cells: n,
                },
                n,
            );
        }
        if let Verdict::Hit { d, p, q, .. } = v {
            let (s, at) = crate::slab::locate(mv, pr);
            return (Verdict::Hit { d, p, q, s, at }, n);
        }
        return (v, n);
    }
    // a ramp (a line that moves in z and in xy) against a prism face: the closed form for its xy
    // segment swept over every height it passes (a superset of the move). Only `Clear` proves
    // anything about the move itself (certified_clear, the tip's heights in that range); any
    // other verdict goes on to the general path below, as if this were not tried
    if let (Move::Line { a, b }, Some(pr)) = (mv, f.prism.as_ref())
        && a[2] != b[2]
        && (a[0] != b[0] || a[1] != b[1])
    {
        let sec = crate::slab::Sec::Seg {
            a: [a[0], a[1]],
            b: [b[0], b[1]],
        };
        let zt = Ival::new(a[2].min(b[2]), a[2].max(b[2]));
        let (v, n) = crate::slab::certify_path(body, f, pr, &sec, zt, limit, res, budget, &mut None, 0.0);
        if let Verdict::Clear { .. } = v {
            return (v, n);
        }
        let (v, m) = certify_bb(body, f, mv, limit, res, budget);
        return (v, n + m);
    }
    // a plane with a horizontal direction, or a cone with a vertical axis: a clearance proven by
    // symmetry (shadow.rs) where it can be, else the branch and bound decides
    if let Some(pl) = mv.planar() {
        let (lower, n) = crate::shadow::clear(body, f, mv, &pl, limit, res, budget);
        if let Some(lower) = lower {
            return (Verdict::Clear { lower }, n);
        }
        let (v, m) = certify_bb(body, f, mv, limit, res, budget);
        return (v, n + m);
    }
    certify_bb(body, f, mv, limit, res, budget)
}

/// The branch and bound itself (any face, any move).
pub fn certify_bb(body: &Body, f: &Face, mv: &Move, limit: f64, res: f64, budget: usize) -> (Verdict, usize) {
    // a helical piece is taken as its arc swept over every height it passes (cell_eval): a
    // Clear holds for the helix; a Hit holds too, its bound taken over all those heights
    let lim_clear = limit + f.mu;
    let lim_hit = limit - f.mu;
    let pg = mv.path_gap();
    let s0 = if matches!(mv.planar(), Some(Planar::Line { .. } | Planar::Arc { .. })) {
        Ival::pt(0.5)
    } else {
        Ival::new(0.0, 1.0)
    };
    let root = Cell {
        p: f.p,
        q: f.q,
        s: s0,
    };
    let mut heap = Heap { v: Vec::new() };
    heap.push(cell_sd(body, f, mv, &root, pg).lo, root);
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
            return (
                Verdict::Undecided {
                    lower: f64::NEG_INFINITY,
                    cells: spent,
                },
                spent,
            );
        }
        let holds = f.holds(c.p, c.q);
        if holds < 0 {
            continue;
        }
        let (mut sd, smear) = cell_eval(body, f, mv, &c, pg);
        if std::env::var("FORGE_CAM_CELLS").is_ok() {
            eprintln!("cell p {:?} q {:?} sd {:?} smear {:?}", c.p, c.q, sd, smear);
        }
        if holds == 0 && sd.lo < lim_clear {
            // the cell crosses the face's trim: its off-face points mean nothing. Every face
            // point of the cell is within the image's diameter of a face point of the cell, and
            // the minimum over the motion of the signed distance is 1-Lipschitz
            let (pm, qm) = (c.p.mid(), c.q.mid());
            if f.holds(Ival::pt(pm), Ival::pt(qm)) == 1 {
                let w = cell_sd(
                    body,
                    f,
                    mv,
                    &Cell {
                        p: Ival::pt(pm),
                        q: Ival::pt(qm),
                        s: c.s,
                    },
                    pg,
                );
                let img = f.surf.eval(c.p, c.q);
                let mvd = if matches!(mv.planar(), Some(Planar::Line { .. } | Planar::Arc { .. })) {
                    0.0
                } else {
                    diam(&mv.enclose(c.s))
                };
                let lip = Ival::pt(w.lo).sub(Ival::pt(diam(&img))).sub(Ival::pt(mvd)).lo;
                if lip > sd.lo {
                    sd = Ival { lo: lip, hi: sd.hi };
                }
            }
        }
        if sd.lo >= lim_clear {
            if sd.lo < lower {
                lower = sd.lo;
            }
            continue;
        }
        // a witness: the cell's centre, if it is surely on the face
        let (pm, qm, sm) = (c.p.mid(), c.q.mid(), c.s.mid());
        if f.holds(Ival::pt(pm), Ival::pt(qm)) == 1 {
            let w = cell_sd(
                body,
                f,
                mv,
                &Cell {
                    p: Ival::pt(pm),
                    q: Ival::pt(qm),
                    s: Ival::pt(sm),
                },
                pg,
            );
            if w.hi < lim_hit {
                let y = f.surf.eval(Ival::pt(pm), Ival::pt(qm));
                let sm = if matches!(mv.planar(), Some(Planar::Line { .. } | Planar::Arc { .. })) {
                    mv.nearest_s([y[0].mid(), y[1].mid()])
                } else {
                    sm
                };
                return (
                    Verdict::Hit {
                        d: w.hi + f.mu,
                        p: pm,
                        q: qm,
                        s: sm,
                        at: [y[0].mid(), y[1].mid(), y[2].mid()],
                    },
                    spent,
                );
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
        heap.push(cell_sd(body, f, mv, &a, pg).lo, a);
        heap.push(cell_sd(body, f, mv, &b, pg).lo, b);
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
        let sc = if ws[i] > 0.0 {
            sm[i].max(1e-9 * ws[i])
        } else {
            -1.0
        };
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
            let la = cell_sd(body, f, mv, &a, mv.path_gap()).lo;
            let lb = cell_sd(body, f, mv, &b, mv.path_gap()).lo;
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
    let wp = if c.p.width() > 0.0 {
        diam(&f.surf.eval(c.p, qm))
    } else {
        0.0
    };
    let wq = if c.q.width() > 0.0 {
        diam(&f.surf.eval(pm, c.q))
    } else {
        0.0
    };
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

/// The swept box of one body of the tool along a move.
pub fn swept_box_body(mv: &Move, b: &Body) -> [Ival; 3] {
    let g = mv.at(Ival::new(0.0, 1.0));
    [
        g[0].add(Ival::new(-b.rmax, b.rmax)),
        g[1].add(Ival::new(-b.rmax, b.rmax)),
        g[2].add(Ival::new(b.z0, b.z1)),
    ]
}

/// Whether two boxes are farther apart than `gap` along some axis (then every pair of points
/// is): a certified cheap clear.
pub fn apart(a: &[Ival; 3], b: &[Ival; 3], gap: f64) -> bool {
    let mut k = 0;
    while k < 3 {
        if Ival::pt(b[k].lo).sub(Ival::pt(a[k].hi)).lo > gap
            || Ival::pt(a[k].lo).sub(Ival::pt(b[k].hi)).lo > gap
        {
            return true;
        }
        k += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trim of a jaw block's top face (the block less its pocket, from stock.rs): a trim arc's
    /// stored radius and its end differ by 2e-11, and the +x ray from this point (inside the
    /// pocket, outside the face) passes through that gap. It once counted a crossing there and
    /// called the point surely inside.
    #[test]
    fn a_ray_through_an_arcs_end_gap_decides_nothing() {
        let tr = Trim {
            segs: vec![
                TSeg::Line {
                    a: [-40.0, -62.0],
                    b: [110.0, -62.0],
                },
                TSeg::Line {
                    a: [110.0, -62.0],
                    b: [110.0, 14.0],
                },
                TSeg::Line {
                    a: [110.0, 14.0],
                    b: [-40.0, 14.0],
                },
                TSeg::Line {
                    a: [-40.0, 14.0],
                    b: [-40.0, -62.0],
                },
                TSeg::Line {
                    a: [17.85044221393764, -42.878726485185325],
                    b: [2.121273514814675, -27.14955778606236],
                },
                TSeg::Arc {
                    a: [2.121273514814675, -27.14955778606236],
                    b: [-1.515769860994955, -27.612056991869597],
                    c: [0.0, -25.0],
                    r: 3.0200000000456155,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [-1.515769860994955, -27.612056991869597],
                    b: [-2.9188143592292217, -24.22480793564896],
                    c: [0.0, -25.0],
                    r: 3.0200000000456155,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [-2.9188143592292217, -24.22480793564896],
                    b: [-0.02000000001862645, -21.980066225863993],
                    c: [0.0, -25.0],
                    r: 3.0200000000456155,
                    ccw: false,
                },
                TSeg::Line {
                    a: [-0.02000000001862645, -21.980066225863993],
                    b: [-0.02000000001862645, -3.019933774136007],
                },
                TSeg::Arc {
                    a: [-0.02000000001862645, -3.019933774136007],
                    b: [-3.0199926415714993, 0.006666709989991189],
                    c: [0.0, 0.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [-3.0199926415714993, 0.006666709989991189],
                    b: [-0.006666709989994795, 3.0199926415714993],
                    c: [0.0, 0.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [-0.006666709989994795, 3.0199926415714993],
                    b: [3.019933774136007, 0.02000000001862645],
                    c: [0.0, 0.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Line {
                    a: [3.019933774136007, 0.02000000001862645],
                    b: [66.980066225864, 0.02000000001862645],
                },
                TSeg::Arc {
                    a: [66.980066225864, 0.02000000001862645],
                    b: [70.00666670998999, 3.0199926415714993],
                    c: [70.0, 0.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [70.00666670998999, 3.0199926415714993],
                    b: [73.0199926415715, 0.006666709989991557],
                    c: [70.0, 0.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [73.0199926415715, 0.006666709989991557],
                    b: [70.02000000001863, -3.019933774136007],
                    c: [70.0, 0.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Line {
                    a: [70.02000000001863, -3.019933774136007],
                    b: [70.02000000001863, -26.980066225863993],
                },
                TSeg::Arc {
                    a: [70.02000000001863, -26.980066225863993],
                    b: [72.95220213997041, -29.363680485194575],
                    c: [70.0, -30.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [72.95220213997041, -29.363680485194575],
                    b: [71.26226567209616, -32.74355342087879],
                    c: [70.0, -30.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [71.26226567209616, -32.74355342087879],
                    b: [67.59605298074894, -31.827960264403373],
                    c: [70.0, -30.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Line {
                    a: [67.59605298074894, -31.827960264403373],
                    b: [52.42794701922685, -43.20403973548673],
                },
                TSeg::Arc {
                    a: [52.42794701922685, -43.20403973548673],
                    b: [52.45720324338611, -46.75571985822792],
                    c: [50.0, -45.0],
                    r: 3.0199999999806475,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [52.45720324338611, -46.75571985822792],
                    b: [49.087669320316344, -47.878897832642046],
                    c: [50.0, -45.0],
                    r: 3.0199999999806475,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [49.087669320316344, -47.878897832642046],
                    b: [46.98006622586399, -45.02000000001863],
                    c: [50.0, -45.0],
                    r: 3.0199999999806475,
                    ccw: false,
                },
                TSeg::Line {
                    a: [46.98006622586399, -45.02000000001863],
                    b: [45.01993377413601, -45.02000000001863],
                },
                TSeg::Arc {
                    a: [45.01993377413601, -45.02000000001863],
                    b: [42.02835667080615, -48.019866867825286],
                    c: [42.0, -45.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [42.02835667080615, -48.019866867825286],
                    b: [38.98097429974572, -45.076706071286694],
                    c: [42.0, -45.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [38.98097429974572, -45.076706071286694],
                    b: [41.87496419483796, -41.9825895128306],
                    c: [42.0, -45.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [41.87496419483796, -41.9825895128306],
                    b: [37.0, -38.02000000001863],
                    c: [37.0, -43.0],
                    r: 4.98000000003558,
                    ccw: true,
                },
                TSeg::Line {
                    a: [37.0, -38.02000000001863],
                    b: [33.0, -38.02000000001863],
                },
                TSeg::Arc {
                    a: [33.0, -38.02000000001863],
                    b: [28.125035805162042, -41.9825895128306],
                    c: [33.0, -43.0],
                    r: 4.9799999999813735,
                    ccw: true,
                },
                TSeg::Arc {
                    a: [28.125035805162042, -41.9825895128306],
                    b: [31.01902570033444, -45.076706071288726],
                    c: [28.0, -45.0],
                    r: 3.0200000001080123,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [31.01902570033444, -45.076706071288726],
                    b: [27.9716433291931, -48.01986686790546],
                    c: [28.0, -45.0],
                    r: 3.0200000001080123,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [27.9716433291931, -48.01986686790546],
                    b: [24.980066225863993, -45.02000000001863],
                    c: [28.0, -45.0],
                    r: 3.0200000001080123,
                    ccw: false,
                },
                TSeg::Line {
                    a: [24.980066225863993, -45.02000000001863],
                    b: [23.019933774136007, -45.02000000001863],
                },
                TSeg::Arc {
                    a: [23.019933774136007, -45.02000000001863],
                    b: [20.775192064346474, -47.918814359212035],
                    c: [20.0, -45.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [20.775192064346474, -47.918814359212035],
                    b: [17.387943008145786, -46.515769860986026],
                    c: [20.0, -45.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
                TSeg::Arc {
                    a: [17.387943008145786, -46.515769860986026],
                    b: [17.85044221393764, -42.878726485185325],
                    c: [20.0, -45.0],
                    r: 3.0200000000278298,
                    ccw: false,
                },
            ],
        };
        const DIRS: [[f64; 2]; 5] = [[1.0, 0.0], [0.6, 0.8], [-0.8, 0.6], [0.28, -0.96], [-0.96, -0.28]];
        assert_eq!(tr.ray_side(23.02, -38.019999999999996, DIRS[0]), 0);
        assert_eq!(tr.classify(Ival::pt(23.02), Ival::pt(-38.019999999999996)), -1);
    }

    use crate::tool::{Cutter, assemble};

    fn wall(x: f64) -> Face {
        // the plane x = const, facing −x, over y in [−50, 50], z in [−20, 0]
        Face::new(
            Surf::Plane {
                o: [x, 0.0, 0.0],
                u: [0.0, 1.0, 0.0],
                v: [0.0, 0.0, 1.0],
            },
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
        let mv = Move::Line {
            a: [7.0, -20.0, -5.0],
            b: [7.0, 20.0, -5.0],
        };
        let (v, n) = certify(cut, &f, &mv, -0.001, 1e-4, 1_000);
        assert!(
            matches!(v, Verdict::Clear { lower } if lower >= -0.001),
            "{v:?} {n}"
        );
        // 0.01 mm into the wall: a gouge, with a witness
        let mv = Move::Line {
            a: [7.01, -20.0, -5.0],
            b: [7.01, 20.0, -5.0],
        };
        let (v, _) = certify(cut, &f, &mv, -0.001, 1e-4, 1_000_000);
        assert!(matches!(v, Verdict::Hit { d, .. } if d < -0.001), "{v:?}");
        // a diagonal move that only touches at its end
        let mv = Move::Line {
            a: [0.0, 0.0, -5.0],
            b: [7.02, 3.0, -5.0],
        };
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
            Surf::Cyl {
                o: [0.0, 0.0, -10.0],
                x: [1.0, 0.0, 0.0],
                y: [0.0, 1.0, 0.0],
                ax: [0.0, 0.0, 1.0],
                r: 10.0,
            },
            Ival::new(-1.0, 1.0),
            Ival::new(0.0, 10.0),
            None,
            0.0,
            1,
        );
        let mv = |r: f64| Move::Arc {
            c: [0.0, 0.0, -5.0],
            x: [1.0, 0.0, 0.0],
            y: [0.0, 1.0, 0.0],
            r,
            t0: -0.5,
            t1: 0.5,
        };
        let (v, n) = certify(cut, &boss, &mv(13.0), -0.01, 2e-3, 20_000);
        assert!(matches!(v, Verdict::Clear { .. }), "{v:?} {n}");
        let (v, _) = certify(cut, &boss, &mv(12.98), -0.01, 2e-3, 2_000_000);
        assert!(matches!(v, Verdict::Hit { .. }), "{v:?}");
    }
}

/// A uniform grid over faces' xy boxes: which faces a box may touch, without a loop over all.
/// A face is listed in every cell its box overlaps (grown by `pad`), so a query misses none of
/// those near the box; the certified clear is still `apart` on each candidate.
pub struct Grid {
    x0: f64,
    y0: f64,
    cw: f64,
    ch: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
    n: usize,
}

/// A caller's scratch for `Grid::near` (one per thread): which faces a query has listed.
#[derive(Clone, Debug, Default)]
pub struct Seen {
    tag: u32,
    mark: Vec<u32>,
}

impl Grid {
    pub fn new(faces: &[Face], pad: f64) -> Grid {
        let (mut x0, mut y0, mut x1, mut y1) =
            (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for f in faces {
            x0 = x0.min(f.bbox[0].lo - pad);
            y0 = y0.min(f.bbox[1].lo - pad);
            x1 = x1.max(f.bbox[0].hi + pad);
            y1 = y1.max(f.bbox[1].hi + pad);
        }
        if !(x0 < x1 && y0 < y1) {
            return Grid {
                x0: 0.0,
                y0: 0.0,
                cw: 1.0,
                ch: 1.0,
                nx: 1,
                ny: 1,
                cells: vec![(0..faces.len() as u32).collect()],
                n: faces.len(),
            };
        }
        let n = ((faces.len() as f64).sqrt() * 2.0).clamp(1.0, 64.0) as usize;
        let (cw, ch) = ((x1 - x0) / n as f64, (y1 - y0) / n as f64);
        let mut cells = vec![Vec::new(); n * n];
        for (i, f) in faces.iter().enumerate() {
            let (a, b) = Self::span(x0, cw, n, f.bbox[0].lo - pad, f.bbox[0].hi + pad);
            let (c, d) = Self::span(y0, ch, n, f.bbox[1].lo - pad, f.bbox[1].hi + pad);
            for gy in c..=d {
                for gx in a..=b {
                    cells[gy * n + gx].push(i as u32);
                }
            }
        }
        Grid {
            x0,
            y0,
            cw,
            ch,
            nx: n,
            ny: n,
            cells,
            n: faces.len(),
        }
    }

    fn span(o: f64, w: f64, n: usize, lo: f64, hi: f64) -> (usize, usize) {
        let a = ((lo - o) / w).floor().max(0.0) as usize;
        let b = ((hi - o) / w).floor().max(0.0) as usize;
        (a.min(n - 1), b.min(n - 1))
    }

    /// The faces whose (grown) boxes may meet the xy of `b`.
    pub fn near(&self, b: &[Ival; 3], out: &mut Vec<u32>, seen: &mut Seen) {
        out.clear();
        if seen.mark.len() < self.n {
            seen.mark.resize(self.n, 0);
        }
        seen.tag = seen.tag.wrapping_add(1);
        if seen.tag == 0 {
            seen.mark.iter_mut().for_each(|m| *m = 0);
            seen.tag = 1;
        }
        let tag = seen.tag;
        let (a, bb) = Self::span(self.x0, self.cw, self.nx, b[0].lo, b[0].hi);
        let (c, d) = Self::span(self.y0, self.ch, self.ny, b[1].lo, b[1].hi);
        // a box wholly outside the grid's range still lands in the border cells: those list
        // every face near the border, so nothing is missed
        for gy in c..=d {
            for gx in a..=bb {
                for &i in &self.cells[gy * self.nx + gx] {
                    if seen.mark[i as usize] != tag {
                        seen.mark[i as usize] = tag;
                        out.push(i);
                    }
                }
            }
        }
    }
}
