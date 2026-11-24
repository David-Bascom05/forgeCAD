//! Drop-cutter for 3-axis surfacing, certified.
//!
//! A 3-axis cutter is held axis +z. Below its widest radius R its surface is a *lower profile*
//! z = b(ρ), ρ ∈ [0, R], non-decreasing in ρ (a flat end: b = 0; a ball of radius r:
//! b = r − √(r² − ρ²); a bull nose; a chamfer or drill cone). The cutter, and every body of the
//! tool no wider than R that sits above the profile, lies inside the tool's *column*
//!
//!   C(X) = { (x, y, z) : ρ = |(x, y) − X.xy| ≤ R, z ≥ X.z + b(ρ) }      (X the tip).
//!
//! **Drop-cutter.** For a face F and a tool position (x, y), the cutter-location height is
//!
//!   CL_F(x, y) = sup { y.z − b(ρ) : y ∈ F, ρ = |y.xy − (x, y)| ≤ R }     (−∞ if no such y),
//!
//! the least tip height at which the column misses F's interior (at CL the column touches F,
//! above it the column is clear). Over a part, CL = max over its faces.
//!
//! **The gap of a move.** For a move g(s) and a face F, v = min over s and y ∈ F of the signed
//! distance of y to the column at g(s). v ≥ L with L ≤ 0 proves the signed distance of every
//! body inside the column to F is ≥ L (a point at depth d inside a body is at least that deep
//! inside the column). Its upper bounds on the depth over a cell: the column's own interval
//! signed distance (the column as a revolved body up to a height past every face: well
//! conditioned at side contact, a tool beside a wall), capped by the vertical depth
//! y.z − g.z − b(ρ) and the radial one R − ρ (each a way out of the column, so each bounds the
//! depth). This is how a 3D finishing move is certified against curved faces without the
//! branch and bound over the tool's own bodies, whose tangent contacts are slow.
//!
//! **A dome cylinder's move, exactly.** A horizontal cylinder above its axis, against a convex
//! column, along a line move: across the axis its untrimmed CL is concave (the sup-convolution of
//! the disk's top and the profile upside down), the tip affine, so the move's worst point is the
//! unconstrained maximum (the two conjugates) unless its maximiser lies beyond an end, when that
//! end decides (`cyl_chord`, Concave.lean). Past the face's end along the axis, the column's
//! section there (`section`) or b(√·)'s superadditivity (`Column::sq_convex`).
//!
//! Both are found by interval branch and bound over the face's parameters (and the move's s):
//! - an upper bound per cell from the mean-value form of y.z − b_k(ρ) for each profile piece k
//!   whose ρ-range the cell meets (each piece is smooth in the parameters), intersected with the
//!   monotone bound y.z.hi − b(ρ.lo);
//! - a lower bound (drop-cutter only) from a real point of the face proven inside its trim.
//!
//! Every float operation rounds outward (ival.rs). The face is the lowered one, within `mu` of
//! the true face: CL is widened by mu each way, a gap narrowed by mu.
use crate::check::{Face, Move, Surf, V3};
use crate::dual::{D, Num};
use crate::ival::Ival;
use crate::tool::{Body, PSeg};

/// One piece of a lower profile, ρ increasing from `a` to `b`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CSeg {
    /// A straight piece: z = a.z + k (ρ − a.ρ) (k ≥ 0; k = 0 a flat ring).
    Line { a: [f64; 2], b: [f64; 2] },
    /// The lower-right quarter of a circle about c of radius r: z = c.z − √(r² − (ρ − c.ρ)²),
    /// with a.ρ ≥ c.ρ and b.z ≤ c.z.
    Arc {
        a: [f64; 2],
        b: [f64; 2],
        c: [f64; 2],
        r: f64,
    },
}

impl CSeg {
    fn a(&self) -> [f64; 2] {
        match *self {
            CSeg::Line { a, .. } | CSeg::Arc { a, .. } => a,
        }
    }
    fn b(&self) -> [f64; 2] {
        match *self {
            CSeg::Line { b, .. } | CSeg::Arc { b, .. } => b,
        }
    }
}

/// A tool's column: its lower profile up to its widest radius R.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub r: f64,
    pub segs: Vec<CSeg>,
    /// The column as a solid of revolution up to `TALL` above its profile (for its signed
    /// distance).
    pub solid: Body,
}

/// How far above its profile the column's solid reaches: past every part this kernel takes.
pub const TALL: f64 = 1e5;

fn solid_of(segs: &[CSeg], r: f64) -> Option<Body> {
    let mut ch = Vec::new();
    let mut i = 0;
    while i < segs.len() {
        ch.push(match segs[i] {
            CSeg::Line { a, b } => PSeg::Line { a, b },
            CSeg::Arc { a, b, c, r } => PSeg::Arc {
                a,
                b,
                c,
                r,
                ccw: true,
            },
        });
        i += 1;
    }
    let top = segs[segs.len() - 1].b();
    ch.push(PSeg::Line {
        a: top,
        b: [r, top[1] + TALL],
    });
    ch.push(PSeg::Line {
        a: [r, top[1] + TALL],
        b: [0.0, top[1] + TALL],
    });
    Body::new(ch, crate::tool::Part::Cutter).ok()
}

/// √ and an interval constant on top of `Num`, for the profile's formulas.
pub trait NumS: Num {
    fn sqrt(self) -> Self;
    fn neg_t(self) -> Self {
        Self::c(0.0).sub(self)
    }
    fn iv(x: Ival) -> Self;
    fn val(self) -> Ival;
}

impl NumS for Ival {
    #[inline]
    fn sqrt(self) -> Ival {
        Ival::sqrt(self)
    }
    #[inline]
    fn iv(x: Ival) -> Ival {
        x
    }
    #[inline]
    fn val(self) -> Ival {
        self
    }
}

const Z0: Ival = Ival { lo: 0.0, hi: 0.0 };

impl NumS for D {
    #[inline]
    fn sqrt(self) -> D {
        let v = self.v.sqrt();
        // (√a)' = a' / (2 √a): unbounded where a may be 0, and then the whole line
        let half = Ival::pt(0.5).div(v);
        D {
            v,
            g: [self.g[0].mul(half), self.g[1].mul(half), self.g[2].mul(half)],
        }
    }
    #[inline]
    fn iv(x: Ival) -> D {
        D { v: x, g: [Z0; 3] }
    }
    #[inline]
    fn val(self) -> Ival {
        self.v
    }
}

impl Column {
    /// The column of a cutter body: the chain from the tip outward while ρ grows, which must
    /// reach the body's widest radius, with every later point no wider (so the body lies in the
    /// column). None for a body that is not so shaped.
    pub fn of(body: &Body) -> Option<Column> {
        let ch = &body.chain;
        if ch.is_empty() {
            return None;
        }
        let first = match ch[0] {
            PSeg::Line { a, .. } | PSeg::Arc { a, .. } => a,
        };
        if first != [0.0, body.z0] {
            return None;
        }
        let mut segs = Vec::new();
        let mut i = 0;
        let mut rho = 0.0f64;
        while i < ch.len() {
            match ch[i] {
                PSeg::Line { a, b } => {
                    // the profile ends where it stops widening
                    if !(b[0] > a[0]) || b[1] < a[1] {
                        break;
                    }
                    segs.push(CSeg::Line { a, b });
                    rho = b[0];
                }
                PSeg::Arc { a, b, c, r, ccw } => {
                    if !(ccw && b[0] > a[0] && b[1] >= a[1] && a[0] >= c[0] && b[1] <= c[1]) {
                        break;
                    }
                    segs.push(CSeg::Arc { a, b, c, r });
                    rho = b[0];
                }
            }
            i += 1;
        }
        if segs.is_empty() || rho != body.rmax {
            return None;
        }
        let solid = solid_of(&segs, rho)?;
        Some(Column { r: rho, segs, solid })
    }

    /// The profile's height at one radius ρ ∈ [0, R], enclosed.
    pub fn b_at(&self, rho: f64) -> Ival {
        let mut i = 0;
        while i + 1 < self.segs.len() && rho > self.segs[i].b()[0] {
            i += 1;
        }
        let s = self.segs[i];
        let (a, b) = (s.a(), s.b());
        let lim = Ival { lo: a[1], hi: b[1] };
        let v = seg_b(&s, Ival::pt(rho));
        let lo = if v.lo > lim.lo { v.lo } else { lim.lo };
        let hi = if v.hi < lim.hi { v.hi } else { lim.hi };
        if lo <= hi { Ival { lo, hi } } else { lim }
    }

    /// The profile over an interval of ρ (clipped to [0, R]): monotone, so its ends' values.
    /// None if the interval misses [0, R].
    pub fn b(&self, rho: Ival) -> Option<Ival> {
        if rho.lo > self.r || rho.hi < 0.0 {
            return None;
        }
        let lo = if rho.lo < 0.0 { 0.0 } else { rho.lo };
        let hi = if rho.hi > self.r { self.r } else { rho.hi };
        Some(Ival {
            lo: self.b_at(lo).lo,
            hi: self.b_at(hi).hi,
        })
    }

    /// The column of the cutter grown by δ ≥ 0 (the Minkowski sum with a ball of radius δ), in the
    /// same tip frame: each line moved δ along its downward-outward normal, each arc's radius
    /// grown by δ, an arc of radius δ about each joint (and about the last end, turning to
    /// horizontal), so the grown profile ends at R + δ. Drop-cutter with it places the tool δ off
    /// the part (a leave). None if δ is not ≥ 0.
    pub fn grow(&self, d: f64) -> Option<Column> {
        if !(d >= 0.0) {
            return None;
        }
        if d == 0.0 {
            return Some(self.clone());
        }
        // each piece's unit normal (ρ, z) at its start and end, pointing out of the tool (down
        // and out); in f64: the grown profile is a new tool shape, exact as given
        let mut out: Vec<CSeg> = Vec::new();
        let mut prev_end: Option<([f64; 2], [f64; 2])> = None; // (point on the original, normal)
        let mut i = 0;
        while i < self.segs.len() {
            let (pa, na, pb, nb, grown) = match self.segs[i] {
                CSeg::Line { a, b } => {
                    let (dr, dz) = (b[0] - a[0], b[1] - a[1]);
                    let l = (dr * dr + dz * dz).sqrt();
                    let n = [dz / l, -dr / l];
                    let ga = [a[0] + d * n[0], a[1] + d * n[1]];
                    let gb = [b[0] + d * n[0], b[1] + d * n[1]];
                    (a, n, b, n, CSeg::Line { a: ga, b: gb })
                }
                CSeg::Arc { a, b, c, r } => {
                    let na = [(a[0] - c[0]) / r, (a[1] - c[1]) / r];
                    let nb = [(b[0] - c[0]) / r, (b[1] - c[1]) / r];
                    let rr = r + d;
                    let ga = [c[0] + rr * na[0], c[1] + rr * na[1]];
                    let gb = [c[0] + rr * nb[0], c[1] + rr * nb[1]];
                    (
                        a,
                        na,
                        b,
                        nb,
                        CSeg::Arc {
                            a: ga,
                            b: gb,
                            c,
                            r: rr,
                        },
                    )
                }
            };
            if let Some((q, nq)) = prev_end {
                // the joint: an arc of radius δ about it from the last normal to this one
                if nq != na {
                    out.push(CSeg::Arc {
                        a: [q[0] + d * nq[0], q[1] + d * nq[1]],
                        b: [pa[0] + d * na[0], pa[1] + d * na[1]],
                        c: q,
                        r: d,
                    });
                }
            } else if pa[0] != 0.0 || na[0] != 0.0 {
                // the profile must start on the axis facing straight down
                return None;
            }
            out.push(grown);
            prev_end = Some((pb, nb));
            i += 1;
        }
        let (q, nq) = prev_end?;
        if nq != [1.0, 0.0] {
            out.push(CSeg::Arc {
                a: [q[0] + d * nq[0], q[1] + d * nq[1]],
                b: [q[0] + d, q[1]],
                c: q,
                r: d,
            });
        }
        // a profile piece of a sharp tip (a drill) whose joint turns more than a quarter is not
        // a lower-right quarter arc: refuse
        let mut k = 0;
        while k < out.len() {
            if let CSeg::Arc { a, b, c, .. } = out[k]
                && !(b[0] >= a[0] && a[0] >= c[0] - 1e-12 && b[1] <= c[1] + 1e-12)
            {
                return None;
            }
            k += 1;
        }
        let r = out[out.len() - 1].b()[0];
        let solid = solid_of(&out, r)?;
        Some(Column { r, segs: out, solid })
    }

    /// The column cut back to radius r (0 < r ≤ R): the profile up to r, its last piece shortened
    /// (a line's end interpolated, an arc's end on its circle). For making paths only: a tool
    /// placed by it may reach R − r further sideways than its column says, which the path's own
    /// check (with the full column) bounds. None if r is out of range.
    pub fn trimmed(&self, r: f64) -> Option<Column> {
        if !(r > 0.0 && r <= self.r) {
            return None;
        }
        if r == self.r {
            return Some(self.clone());
        }
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.segs.len() {
            let sg = self.segs[i];
            let (a, b) = (sg.a(), sg.b());
            if b[0] < r {
                out.push(sg);
                i += 1;
                continue;
            }
            if a[0] < r {
                let e = match sg {
                    CSeg::Line { a, b } => {
                        let t = (r - a[0]) / (b[0] - a[0]);
                        [r, a[1] + t * (b[1] - a[1])]
                    }
                    CSeg::Arc { c, r: rr, .. } => {
                        let d = r - c[0];
                        [r, c[1] - (rr * rr - d * d).max(0.0).sqrt()]
                    }
                };
                out.push(match sg {
                    CSeg::Line { a, .. } => CSeg::Line { a, b: e },
                    CSeg::Arc { a, c, r: rr, .. } => CSeg::Arc { a, b: e, c, r: rr },
                });
            }
            break;
        }
        if out.is_empty() {
            return None;
        }
        let rr = out[out.len() - 1].b()[0];
        let solid = solid_of(&out, rr)?;
        Some(Column {
            r: rr,
            segs: out,
            solid,
        })
    }

    /// The column widened past its rim by ε > 0 with a 45° lip (from (R, b(R)) to
    /// (R + ε, b(R) + ε)): for making paths only, so a tool is not placed exactly tangent to a
    /// wall (where the footprint's closed rim makes the drop jump, and a move grazing the wall
    /// cannot be proven by intervals); it keeps about ε off walls instead. (Any face point inside
    /// the widened footprint blocks; the lip's slope only sets how high, and a gentle one keeps
    /// the search well conditioned.)
    pub fn widened(&self, eps: f64) -> Option<Column> {
        if !(eps > 0.0) {
            return None;
        }
        let top = self.segs[self.segs.len() - 1].b();
        let mut segs = self.segs.clone();
        segs.push(CSeg::Line {
            a: top,
            b: [top[0] + eps, top[1] + eps],
        });
        let r = top[0] + eps;
        let solid = solid_of(&segs, r)?;
        Some(Column { r, segs, solid })
    }

    /// Whether the column is convex: its profile's slope never falls along ρ (each line's slope,
    /// each arc's from its start to its end, no larger than the next piece's at the joint). The
    /// sphere bound in `gap_move` needs it.
    pub fn convex(&self) -> bool {
        // slopes as (dρ, dz) directions, compared by cross product (no division)
        let dir_start = |sg: &CSeg| -> [f64; 2] {
            match *sg {
                CSeg::Line { a, b } => [b[0] - a[0], b[1] - a[1]],
                CSeg::Arc { a, c, .. } => [c[1] - a[1], a[0] - c[0]],
            }
        };
        let dir_end = |sg: &CSeg| -> [f64; 2] {
            match *sg {
                CSeg::Line { a, b } => [b[0] - a[0], b[1] - a[1]],
                CSeg::Arc { b, c, .. } => [c[1] - b[1], b[0] - c[0]],
            }
        };
        let mut i = 0;
        while i + 1 < self.segs.len() {
            let (u, v) = (dir_end(&self.segs[i]), dir_start(&self.segs[i + 1]));
            // turning left (counter-clockwise) or straight in (ρ, z): the slope does not fall
            if u[0] * v[1] - u[1] * v[0] < 0.0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Whether h(x) = b(√x) is convex on [0, R²]: the profile starts on the axis, is convex, and
    /// b'(ρ)/ρ never falls (flat rings and arcs centred on or right of the axis; not a cone).
    /// Then b(√(u² + a²)) ≥ b(|u|) + b(a) − b(0) (a convex h with h(0) is superadditive): a tool
    /// a beyond a horizontal cylinder's end sees it at least b(a) − b(0) lower (`gap_move`).
    pub fn sq_convex(&self) -> bool {
        if self.segs[0].a()[0] != 0.0 || !self.convex() {
            return false;
        }
        self.segs.iter().all(|sg| match *sg {
            CSeg::Line { a, b } => a[1] == b[1],
            CSeg::Arc { c, .. } => c[0] >= 0.0,
        })
    }

    /// The profile's top: b(R).
    pub fn top(&self) -> f64 {
        self.segs[self.segs.len() - 1].b()[1]
    }

    /// Whether a body lies inside the column (no wider than R, its bottom at or above b(R)).
    pub fn holds(&self, body: &Body) -> bool {
        body.rmax <= self.r && body.z0 >= self.top()
    }

    /// Whether a body lies inside the column: one above the profile (`holds`), or the cutter
    /// the column was made from (its profile is the column's, and every later point of its chain
    /// is no wider than R and, the side rising, no lower than the profile there).
    pub fn holds_or_is(&self, body: &Body) -> bool {
        if self.holds(body) {
            return true;
        }
        match Column::of(body) {
            Some(c) => c == *self,
            None => false,
        }
    }
}

/// A piece's formula at ρ (any interval, the formula extended past the piece's ends).
fn seg_b(s: &CSeg, rho: Ival) -> Ival {
    match *s {
        CSeg::Line { a, b } => {
            if a[1] == b[1] {
                return Ival::pt(a[1]);
            }
            let k = Ival::pt(b[1])
                .sub(Ival::pt(a[1]))
                .div(Ival::pt(b[0]).sub(Ival::pt(a[0])));
            Ival::pt(a[1]).add(k.mul(rho.sub(Ival::pt(a[0]))))
        }
        CSeg::Arc { c, r, .. } => {
            let d = rho.sub(Ival::pt(c[0]));
            Ival::pt(c[1]).sub(Ival::pt(r).sqr().sub(d.sqr()).sqrt())
        }
    }
}

/// y.z − b_k(ρ) − tip.z for one piece, from ρ² (the piece's formula, smooth in the parameters
/// where ρ² is).
fn seg_u<T: NumS>(s: &CSeg, r2: T, yz: T, tz: T) -> T {
    match *s {
        CSeg::Line { a, b } => {
            if a[1] == b[1] {
                return yz.sub(T::c(a[1])).sub(tz);
            }
            let k = Ival::pt(b[1])
                .sub(Ival::pt(a[1]))
                .div(Ival::pt(b[0]).sub(Ival::pt(a[0])));
            let rho = r2.sqrt();
            let bz = T::c(a[1]).add(T::iv(k).mul(rho.sub(T::c(a[0]))));
            yz.sub(bz).sub(tz)
        }
        CSeg::Arc { c, r, .. } => {
            let d2 = if c[0] == 0.0 {
                r2
            } else {
                r2.sqrt().sub(T::c(c[0])).sqr()
            };
            let root = T::iv(Ival::pt(r).sqr()).sub(d2).sqrt();
            yz.sub(T::c(c[1])).add(root).sub(tz)
        }
    }
}

/// A cell of the search: face parameters and the move's s.
#[derive(Clone, Copy, Debug)]
struct Cell {
    p: Ival,
    q: Ival,
    s: Ival,
}

/// The upper bound over a cell of u = y.z − b(ρ) − g(s).z (over the cell's points within the
/// column's footprint), or None when no point of the cell is within R. Also which parameter
/// to split (the one whose derivative times width is largest).
fn cell_upper(col: &Column, f: &Face, mv: &Move, c: &Cell) -> Option<(f64, usize)> {
    cell_upper_side(col, f, mv, c, false)
}

/// `cell_upper`; with `side`, of min(u, R − ρ) (the depth into the column is at most the
/// radial gap to its side as well as the vertical one to its bottom).
fn cell_upper_side(col: &Column, f: &Face, mv: &Move, c: &Cell, side: bool) -> Option<(f64, usize)> {
    // natural enclosures and the monotone bound
    let y = f.surf.eval(c.p, c.q);
    let g = mv.at(c.s);
    let dx = y[0].sub(g[0]);
    let dy = y[1].sub(g[1]);
    let r2 = dx.sqr().add(dy.sqr());
    let rho = r2.sqrt();
    let bb = col.b(rho)?;
    let mut best = y[2].sub(bb).sub(g[2]).hi;
    if side {
        let rad = Ival::pt(col.r)
            .sub(Ival {
                lo: rho.lo,
                hi: rho.lo,
            })
            .hi;
        if rad < best {
            best = rad;
        }
        let sd = col.solid.sd_rz(rho, y[2].sub(g[2]));
        if -sd.lo < best {
            best = -sd.lo;
        }
    }
    let point = c.p.width() == 0.0 && c.q.width() == 0.0 && c.s.width() == 0.0;
    if point {
        return Some((best, 0));
    }
    // per piece, the mean-value form about the cell's centre
    let x = [c.p, c.q, c.s];
    let (pc, qc, sc) = (Ival::pt(c.p.mid()), Ival::pt(c.q.mid()), Ival::pt(c.s.mid()));
    let yc = f.surf.eval(pc, qc);
    let gc = mv.at(sc);
    let r2c = yc[0].sub(gc[0]).sqr().add(yc[1].sub(gc[1]).sqr());
    let yd: [D; 3] = f.surf.eval_n(D::var(c.p, 0), D::var(c.q, 1));
    let gd: [D; 3] = mv.at_n(D::var(c.s, 2));
    let r2d = yd[0].sub(gd[0]).sqr().add(yd[1].sub(gd[1]).sqr());
    let zd = yd[2];
    let mut pieces = f64::NEG_INFINITY;
    let mut grad = [0.0f64; 3];
    let mut i = 0;
    while i < col.segs.len() {
        let s = &col.segs[i];
        let (a, b) = (s.a(), s.b());
        // the piece's ρ-range against the cell's (the first piece from 0, the last to R)
        let lo = if i == 0 { f64::NEG_INFINITY } else { a[0] };
        if rho.hi < lo || rho.lo > b[0] {
            i += 1;
            continue;
        }
        let uc = seg_u(s, r2c, yc[2], gc[2]);
        let ud = seg_u(s, r2d, zd, gd[2]);
        let mut acc = uc;
        let mut k = 0;
        while k < 3 {
            let m = x[k].mid();
            acc = acc.add(ud.g[k].mul(x[k].sub(Ival::pt(m))));
            let gk = ud.g[k].abs().hi * x[k].width();
            if gk.is_finite() && gk > grad[k] {
                grad[k] = gk;
            }
            k += 1;
        }
        let hi = if acc.hi < ud.v.hi { acc.hi } else { ud.v.hi };
        if hi > pieces {
            pieces = hi;
        }
        i += 1;
    }
    if pieces < best {
        best = pieces;
    }
    // split where the bound is smeared most; with no finite derivative, the widest image
    let mut k = 0;
    let mut kmax = 0;
    let mut m = -1.0;
    let w = [
        diam2(&f.surf.eval(c.p, qc)),
        diam2(&f.surf.eval(pc, c.q)),
        diam2(&mv.at(c.s)),
    ];
    while k < 3 {
        let sm = grad[k] + 1e-3 * w[k];
        if x[k].width() > 0.0 && sm > m {
            m = sm;
            kmax = k;
        }
        k += 1;
    }
    Some((best, kmax))
}

fn diam2(b: &[Ival; 3]) -> f64 {
    let (a, c, d) = (b[0].width(), b[1].width(), b[2].width());
    (a * a + c * c + d * d).sqrt()
}

/// A lower bound on u at the cell's centre, from a real point of the face proven inside its
/// trim and within the column's footprint (drop-cutter only: s is a point).
fn cell_lower(col: &Column, f: &Face, mv: &Move, c: &Cell) -> f64 {
    let (pm, qm) = (c.p.mid(), c.q.mid());
    let (p, q) = (Ival::pt(pm), Ival::pt(qm));
    if f.holds(p, q) != 1 {
        return f64::NEG_INFINITY;
    }
    let y = f.surf.eval(p, q);
    let g = mv.at(c.s);
    let rho = y[0].sub(g[0]).sqr().add(y[1].sub(g[1]).sqr()).sqrt();
    if !(rho.hi <= col.r) {
        return f64::NEG_INFINITY;
    }
    let b = col.b_at(rho.hi);
    y[2].sub(b).sub(g[2]).lo
}

/// A lower bound on the depth into the column of a real point of the face (the cell's centre,
/// proven in the face) at the cell's instant: −(its signed distance's upper bound); −∞ if none.
fn side_lower(col: &Column, f: &Face, mv: &Move, c: &Cell) -> f64 {
    let (p, q) = (Ival::pt(c.p.mid()), Ival::pt(c.q.mid()));
    if f.holds(p, q) != 1 {
        return f64::NEG_INFINITY;
    }
    let y = f.surf.eval(p, q);
    let g = mv.at(c.s);
    let rho = y[0].sub(g[0]).sqr().add(y[1].sub(g[1]).sqr()).sqrt();
    -col.solid.sd_rz(rho, y[2].sub(g[2])).hi
}

/// A binary max-heap of (bound, face index, cell).
struct Heap {
    v: Vec<(f64, usize, Cell)>,
}

impl Heap {
    fn push(&mut self, k: f64, fi: usize, c: Cell) {
        self.v.push((k, fi, c));
        let mut i = self.v.len() - 1;
        while i > 0 {
            let p = (i - 1) / 2;
            if self.v[p].0 >= self.v[i].0 {
                break;
            }
            self.v.swap(p, i);
            i = p;
        }
    }
    fn pop(&mut self) -> Option<(f64, usize, Cell)> {
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
            if l < len && self.v[l].0 > self.v[m].0 {
                m = l;
            }
            if r < len && self.v[r].0 > self.v[m].0 {
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
    fn top(&self) -> f64 {
        if self.v.is_empty() {
            f64::NEG_INFINITY
        } else {
            self.v[0].0
        }
    }
}

/// The certified drop-cutter height at one tool position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cl {
    /// CL ≥ lo (a real face point proven to need the tip at least this high); −∞ if none found.
    pub lo: f64,
    /// CL ≤ hi (−∞: no face point within the column's footprint).
    pub hi: f64,
    /// Cells searched.
    pub cells: usize,
}

fn split(c: &Cell, k: usize) -> (Cell, Cell) {
    let mut a = *c;
    let mut b = *c;
    if k == 0 {
        let (l, r) = c.p.split();
        a.p = l;
        b.p = r;
    } else if k == 1 {
        let (l, r) = c.q.split();
        a.q = l;
        b.q = r;
    } else {
        let (l, r) = c.s.split();
        a.s = l;
        b.s = r;
    }
    (a, b)
}

/// The faces whose box comes within the column's footprint of (x, y).
fn near(faces: &[Face], idx: &[u32], x: f64, y: f64, r: f64) -> Vec<usize> {
    let mut out = Vec::new();
    let mut k = 0;
    while k < idx.len() {
        let i = idx[k] as usize;
        let b = &faces[i].bbox;
        let gx = gap(b[0], x);
        let gy = gap(b[1], y);
        if Ival::pt(gx).sqr().add(Ival::pt(gy).sqr()).lo <= Ival::pt(r + faces[i].mu).sqr().hi {
            out.push(i);
        }
        k += 1;
    }
    out
}

fn gap(b: Ival, x: f64) -> f64 {
    if x < b.lo {
        Ival::pt(b.lo).sub(Ival::pt(x)).lo
    } else if x > b.hi {
        Ival::pt(x).sub(Ival::pt(b.hi)).lo
    } else {
        0.0
    }
}

/// Drop-cutter: the certified CL height at (x, y) over the faces listed in `idx` (indices into
/// `faces`), to `res` (hi − lo ≤ res unless the budget runs out), each face widened by its mu.
///
/// Per face, a closed form (plane, sphere, cylinder with a horizontal axis: `closed`) gives an
/// upper bound for the untrimmed surface and a contact point; the contact point, when it is
/// proven inside the face, gives a lower bound by evaluation. A face whose bounds already meet
/// is done; the rest are searched by branch and bound, each cell's bound capped by the face's
/// closed form.
pub fn drop(col: &Column, faces: &[Face], idx: &[u32], x: f64, y: f64, res: f64, budget: usize) -> Cl {
    drop_above(col, faces, idx, x, y, f64::NEG_INFINITY, res, budget)
}

/// `drop`, resolving CL only above `floor`: a result with hi ≤ floor says only that CL ≤ hi
/// (the caller lifts the tool to the floor anyway), so the search stops as soon as that is
/// proven.
#[allow(clippy::too_many_arguments)]
pub fn drop_above(
    col: &Column,
    faces: &[Face],
    idx: &[u32],
    x: f64,
    y: f64,
    floor: f64,
    res: f64,
    budget: usize,
) -> Cl {
    search(col, faces, idx, x, y, floor, f64::INFINITY, res, budget, true)
}

/// Whether CL at (x, y) is above z: `drop`'s search, stopped as soon as the answer is proven
/// either way (hi ≤ z: the column at z misses the faces; lo > z: it meets one), else run to
/// `res`. Its bounds hold CL as `drop`'s do, so a decided answer is true: `drop` can only agree
/// with it or leave it within `res` of z.
#[allow(clippy::too_many_arguments)]
pub fn drop_decide(
    col: &Column,
    faces: &[Face],
    idx: &[u32],
    x: f64,
    y: f64,
    z: f64,
    floor: f64,
    res: f64,
    budget: usize,
) -> Cl {
    let f = if floor > z { floor } else { z };
    search(col, faces, idx, x, y, f, z, res, budget, true)
}

/// The branch and bound alone, without the closed forms: the reference the closed forms are
/// tested against (L6).
pub fn drop_general(
    col: &Column,
    faces: &[Face],
    idx: &[u32],
    x: f64,
    y: f64,
    res: f64,
    budget: usize,
) -> Cl {
    search(
        col,
        faces,
        idx,
        x,
        y,
        f64::NEG_INFINITY,
        f64::INFINITY,
        res,
        budget,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn search(
    col: &Column,
    faces: &[Face],
    idx: &[u32],
    x: f64,
    y: f64,
    floor: f64,
    stop: f64,
    res: f64,
    budget: usize,
    closed_forms: bool,
) -> Cl {
    let mv = Move::Line {
        a: [x, y, 0.0],
        b: [x, y, 0.0],
    };
    let cand = near(faces, idx, x, y, col.r);
    let mut heap = Heap { v: Vec::new() };
    let mut lo = f64::NEG_INFINITY;
    // the faces' closed-form caps (+∞: none), and the largest cap of a face already done
    let mut cap = vec![f64::INFINITY; cand.len()];
    let mut done = f64::NEG_INFINITY;
    let mut cells = 0usize;
    let mut k = 0;
    while k < cand.len() {
        let f = &faces[cand[k]];
        let cf = if closed_forms { closed(col, f, x, y) } else { None };
        if let Some((ub, at)) = cf {
            if ub == f64::NEG_INFINITY {
                // the untrimmed surface is out of reach
                cap[k] = ub;
                k += 1;
                continue;
            }
            cap[k] = Ival::pt(ub).add(Ival::pt(f.mu)).hi;
            if let Some((p, q)) = at {
                let w = Ival::pt(point_u(col, f, &mv, p, q)).sub(Ival::pt(f.mu)).lo;
                if w > lo {
                    lo = w;
                }
            }
        }
        k += 1;
    }
    k = 0;
    while k < cand.len() {
        let f = &faces[cand[k]];
        if cap[k] == f64::NEG_INFINITY {
            k += 1;
            continue;
        }
        if cap[k] <= lo + res || cap[k] <= floor {
            // nothing of this face can lift the tool past what is known
            if cap[k] > done {
                done = cap[k];
            }
            k += 1;
            continue;
        }
        let c = Cell {
            p: f.p,
            q: f.q,
            s: Ival::pt(0.0),
        };
        cells += 1;
        if let Some((u, _)) = cell_upper(col, f, &mv, &c) {
            let u = Ival::pt(u).add(Ival::pt(f.mu)).hi;
            let u = if u < cap[k] { u } else { cap[k] };
            heap.push(u, k, c);
        }
        k += 1;
    }
    // the largest bound left over all faces
    let mut hi;
    loop {
        hi = heap.top();
        if hi < done {
            hi = done;
        }
        if hi - lo <= res || cells >= budget || hi <= floor || lo > stop {
            break;
        }
        let Some((u, ki, c)) = heap.pop() else {
            break;
        };
        let f = &faces[cand[ki]];
        if u <= lo {
            continue;
        }
        let w = Ival::pt(cell_lower(col, f, &mv, &c)).sub(Ival::pt(f.mu)).lo;
        if w > lo {
            lo = w;
        }
        if c.p.width() == 0.0 && c.q.width() == 0.0 {
            // a point cell cannot be split: its bound stands
            heap.push(u, ki, c);
            hi = heap.top();
            if hi < done {
                hi = done;
            }
            break;
        }
        let kk = match cell_upper(col, f, &mv, &c) {
            Some((_, kk)) => kk,
            None => continue,
        };
        let (a, b) = split(&c, kk);
        for h in [a, b] {
            cells += 1;
            if f.holds(h.p, h.q) == -1 {
                continue;
            }
            // each child's centre is a witness candidate now (bounds often tie, and the heap's
            // order among ties must not decide whether a witness is ever seen)
            let w = Ival::pt(cell_lower(col, f, &mv, &h)).sub(Ival::pt(f.mu)).lo;
            if w > lo {
                lo = w;
            }
            if let Some((u2, _)) = cell_upper(col, f, &mv, &h) {
                let u2 = Ival::pt(u2).add(Ival::pt(f.mu)).hi;
                let u2 = if u2 < cap[ki] { u2 } else { cap[ki] };
                if u2 > lo && u2 > floor {
                    heap.push(u2, ki, h);
                }
            }
        }
    }
    if hi < lo {
        hi = lo;
    }
    Cl { lo, hi, cells }
}

/// u at one face point (p, q) for the tool at the move's start: a lower bound on CL when the
/// point is proven in the face and within the footprint, else −∞.
fn point_u(col: &Column, f: &Face, mv: &Move, p: f64, q: f64) -> f64 {
    let c = Cell {
        p: Ival::pt(p),
        q: Ival::pt(q),
        s: Ival::pt(0.0),
    };
    cell_lower(col, f, mv, &c)
}

/// The column's drop onto a disk of radius `sr` whose centre is `d` away horizontally (in the
/// vertical plane through the tool's axis) and at height 0: the largest of √(sr² − (ρ − d)²) − b(ρ)
/// over ρ ∈ [0, R], each profile piece's maximum of a concave function (the critical value
/// where the piece's critical point may lie in it, else its ends). Also, in f64, the ρ where it
/// is attained. None when the disk is out of reach.
///
/// A sphere is such a disk in the plane through its centre; so is a cylinder with a horizontal
/// axis, in the plane across it (a line along the axis at the column's nearest point is the
/// highest, b being non-decreasing).
fn disk_drop<T: NumS>(segs: &[CSeg], sr: f64, d: T) -> Option<(T, f64)> {
    let s2 = Ival::pt(sr).sqr();
    let mut best: Option<T> = None;
    let mut arg = 0.0f64;
    let mut bestf = f64::NEG_INFINITY;
    let dv = d.val();
    let mut i = 0;
    while i < segs.len() {
        let sg = segs[i];
        let (a, b) = (sg.a(), sg.b());
        // the ends: on the circle exactly at their own heights
        let mut e = 0;
        while e < 2 {
            let pnt = if e == 0 { a } else { b };
            let off = T::c(pnt[0]).sub(d);
            let under = T::iv(s2).sub(off.sqr());
            if under.val().hi >= 0.0 {
                let v = under.sqrt().sub(T::c(pnt[1]));
                keep(&mut best, &mut bestf, &mut arg, v, pnt[0]);
            }
            e += 1;
        }
        // the piece's critical point
        match sg {
            CSeg::Line { a, b } => {
                if a[1] == b[1] {
                    // flat: the disk's top over the ring
                    if dv.hi >= a[0] && dv.lo <= b[0] {
                        let v = T::c(sr).sub(T::c(a[1]));
                        let at = dv.mid().max(a[0]).min(b[0]);
                        keep(&mut best, &mut bestf, &mut arg, v, at);
                    }
                } else {
                    let k = Ival::pt(b[1])
                        .sub(Ival::pt(a[1]))
                        .div(Ival::pt(b[0]).sub(Ival::pt(a[0])));
                    let w = Ival::pt(1.0).add(k.sqr()).sqrt();
                    // ρ* = d − k sr / √(1 + k²); value sr √(1 + k²) − (a.z − k a.ρ) − k d
                    let shift = k.mul(Ival::pt(sr)).div(w);
                    let rs = dv.sub(shift);
                    if rs.hi >= a[0] && rs.lo <= b[0] {
                        let c0 = Ival::pt(a[1]).sub(k.mul(Ival::pt(a[0])));
                        let v = T::iv(Ival::pt(sr).mul(w).sub(c0)).sub(T::iv(k).mul(d));
                        keep(&mut best, &mut bestf, &mut arg, v, rs.mid().max(a[0]).min(b[0]));
                    }
                }
            }
            CSeg::Arc { a, b, c, r } => {
                // ρ* = c.ρ + r (d − c.ρ) / (sr + r); value √((sr + r)² − (d − c.ρ)²) − c.z
                let tot = Ival::pt(sr).add(Ival::pt(r));
                let dc = dv.sub(Ival::pt(c[0]));
                let rs = Ival::pt(c[0]).add(Ival::pt(r).mul(dc).div(tot));
                if rs.hi >= a[0] && rs.lo <= b[0] && dc.hi >= 0.0 {
                    let off = d.sub(T::c(c[0]));
                    let under = T::iv(tot.sqr()).sub(off.sqr());
                    if under.val().hi >= 0.0 {
                        let v = under.sqrt().sub(T::c(c[1]));
                        keep(&mut best, &mut bestf, &mut arg, v, rs.mid().max(a[0]).min(b[0]));
                    }
                }
            }
        }
        i += 1;
    }
    match best {
        Some(v) => Some((v, arg)),
        None => None,
    }
}

/// The column's drop into a bowl: the lower half of a circle of radius `sr` (centre at height 0,
/// the tool's axis at lateral place `d`, signed), in the vertical plane through the axis and the
/// circle's centre, over the face's lateral range [xa, xb] ⊆ [−sr, sr]: the largest of
/// −√(sr² − x²) − b(|ρ|) over ρ ∈ [−R, R], x = d + ρ ∈ [xa, xb].
/// Its candidates: each piece's ends on both sides (rings), the range's ends (x = xa, xb: the
/// arc's end lines), and each piece's critical points on both sides: an arc's internal tangency
/// (x = (d ± c.ρ) sr / (sr − r)), a line's slope point (x = k sr / √(1 + k²)), a flat ring's
/// bottom (x = 0). The largest is the maximum (every stationary point and every end is a
/// candidate; a candidate whose place is only possibly in range adds an upper bound, never less).
/// Also, in f64, the signed ρ attaining it.
fn bowl_drop<T: NumS>(col: &Column, sr: f64, d: T, xa: f64, xb: f64) -> Option<(T, f64)> {
    let s2 = Ival::pt(sr).sqr();
    let mut best: Option<T> = None;
    let mut arg = 0.0f64;
    let mut bestf = f64::NEG_INFINITY;
    let dv = d.val();
    // the bowl's height at lateral x (relative to its centre): −√(sr² − x²), if x may be in range
    let inr = |x: Ival| x.hi >= xa && x.lo <= xb;
    let low = |x: T| -> Option<T> {
        if !inr(x.val()) {
            return None;
        }
        let under = T::iv(s2).sub(x.sqr());
        if under.val().hi >= 0.0 {
            Some(under.sqrt().neg_t())
        } else {
            None
        }
    };
    let mut side = 0;
    while side < 2 {
        let sg = if side == 0 { 1.0 } else { -1.0 };
        let mut i = 0;
        while i < col.segs.len() {
            let piece = col.segs[i];
            let (a, b) = (piece.a(), piece.b());
            // ends: the profile point at signed ρ = ±end
            for pnt in [a, b] {
                let x = d.add(T::c(sg * pnt[0]));
                if let Some(h) = low(x) {
                    keep(&mut best, &mut bestf, &mut arg, h.sub(T::c(pnt[1])), sg * pnt[0]);
                }
            }
            match piece {
                CSeg::Line { a, b } => {
                    if a[1] == b[1] {
                        // the bowl's bottom (x = 0) under the ring: ρ = −d·sg
                        let rho = dv.neg().mul(Ival::pt(sg));
                        if rho.hi >= a[0] && rho.lo <= b[0] && inr(Ival::pt(0.0)) {
                            let v = T::c(-sr).sub(T::c(a[1]));
                            keep(
                                &mut best,
                                &mut bestf,
                                &mut arg,
                                v,
                                (-dv.mid()).clamp(sg.min(0.0) * b[0], sg.max(0.0) * b[0]),
                            );
                        }
                    } else {
                        let k = Ival::pt(b[1])
                            .sub(Ival::pt(a[1]))
                            .div(Ival::pt(b[0]).sub(Ival::pt(a[0])));
                        let w = Ival::pt(1.0).add(k.sqr()).sqrt();
                        // on side sg, b'(|ρ|) = k: x/√(sr² − x²) = sg k, x = sg k sr / w
                        let x = Ival::pt(sg).mul(k).mul(Ival::pt(sr)).div(w);
                        let rho = x.sub(dv).mul(Ival::pt(sg));
                        if rho.hi >= a[0] && rho.lo <= b[0] && inr(x) {
                            // value: −sr / w − (a.z + k (|ρ| − a.ρ)), |ρ| = sg (x − d)
                            let xr = T::iv(x);
                            let absr = xr.sub(d).mul(T::c(sg));
                            let bz = T::c(a[1]).add(T::iv(k).mul(absr.sub(T::c(a[0]))));
                            let v = T::iv(Ival::pt(-sr).div(w)).sub(bz);
                            keep(
                                &mut best,
                                &mut bestf,
                                &mut arg,
                                v,
                                sg * rho.mid().max(a[0]).min(b[0]),
                            );
                        }
                    }
                }
                CSeg::Arc { a, b, c, r } => {
                    // internal tangency: the tube centre at x_c = d + sg c.ρ is sr − r from the
                    // bowl's centre; the contact x = x_c sr / (sr − r), on the arc where its
                    // offset from the tube centre, sg (x − x_c), lies in [a.ρ − c.ρ, b.ρ − c.ρ]
                    if r < sr {
                        let inner = Ival::pt(sr).sub(Ival::pt(r));
                        let xc = dv.add(Ival::pt(sg * c[0]));
                        let x = xc.mul(Ival::pt(sr)).div(inner);
                        let off = x.sub(xc).mul(Ival::pt(sg));
                        if off.hi >= a[0] - c[0] && off.lo <= b[0] - c[0] && inr(x) {
                            let xct = d.add(T::c(sg * c[0]));
                            let under = T::iv(inner.sqr()).sub(xct.sqr());
                            if under.val().hi >= 0.0 {
                                let v = under.sqrt().neg_t().sub(T::c(c[1]));
                                let rho = sg * (c[0] + off.mid()).max(a[0]).min(b[0]);
                                keep(&mut best, &mut bestf, &mut arg, v, rho);
                            }
                        }
                    }
                }
            }
            i += 1;
        }
        // the range's end line: x = xa or xb, ρ = x − d, if within reach
        let xe = if side == 0 { xb } else { xa };
        let rho = Ival::pt(xe).sub(dv);
        if rho.abs().lo <= col.r
            && let Some(bz) = col.b(rho.abs())
        {
            let h = Ival::pt(sr).sqr().sub(Ival::pt(xe).sqr());
            let h = Ival {
                lo: h.lo.max(0.0),
                hi: h.hi.max(0.0),
            }
            .sqrt()
            .neg();
            keep(
                &mut best,
                &mut bestf,
                &mut arg,
                T::iv(h.sub(bz)),
                rho.mid().clamp(-col.r, col.r),
            );
        }
        side += 1;
    }
    match best {
        Some(v) => Some((v, arg)),
        None => None,
    }
}

/// Keeps the candidate with the larger upper bound as the bound (an upper bound of a max is
/// the max of upper bounds; ties in value keep the hull), and the f64-best argument.
fn keep<T: NumS>(best: &mut Option<T>, bestf: &mut f64, arg: &mut f64, v: T, at: f64) {
    let vv = v.val();
    match best {
        None => *best = Some(v),
        Some(b) => {
            let bv = b.val();
            if vv.lo >= bv.hi {
                *best = Some(v);
            } else if !(bv.lo >= vv.hi) {
                // overlapping: an interval holding both (derivatives dropped)
                *best = Some(T::iv(bv.hull(vv)));
            }
        }
    }
    let m = vv.mid();
    if m > *bestf {
        *bestf = m;
        *arg = at;
    }
}

/// The closed-form drop for the untrimmed surface at (x, y): an upper bound on its CL (−∞ if
/// out of reach) and a face parameter where the column touches it (in f64, for the evaluated
/// lower bound), or None for a surface without one here.
pub fn closed(col: &Column, f: &Face, x: f64, y: f64) -> Option<(f64, Option<(f64, f64)>)> {
    if let Some(crate::slab::Prism::Wall { sec, z1, gap, .. }) = &f.prism {
        return Some(wall_drop(col, f, sec, *z1, *gap, x, y));
    }
    let eta = bowl_eta(f);
    let bowl = eta.is_some();
    let lift = Ival::pt(eta.unwrap_or(0.0)).scale(2.0).hi;
    match f.surf {
        Surf::Plane { o, u, v } => {
            let n = cross(u, v);
            let nz = n[2];
            if nz.hi < 0.0 {
                // the same plane, the normal turned up
                return plane_drop(col, o, u, v, [n[0].neg(), n[1].neg(), nz.neg()], x, y);
            }
            if nz.lo > 0.0 {
                return plane_drop(col, o, u, v, n, x, y);
            }
            None
        }
        Surf::Sphere {
            o,
            x: fx,
            y: fy,
            ax,
            r,
        } => {
            let dx = Ival::pt(x).sub(Ival::pt(o[0]));
            let dy = Ival::pt(y).sub(Ival::pt(o[1]));
            let d = dx.sqr().add(dy.sqr()).sqrt();
            if bowl {
                let Some((v, rho)) = bowl_drop(col, r, d, -r, r) else {
                    return Some((f64::NEG_INFINITY, None));
                };
                let rho = inside_signed(col, rho);
                let ub = Ival::pt(o[2]).add(v).add(Ival::pt(lift)).hi;
                // the contact: lateral d + ρ along the centre-to-tool direction, on the lower half
                let dm = d.mid();
                let (ex, ey) = if dm > 1e-12 {
                    (dx.mid() / dm, dy.mid() / dm)
                } else {
                    (1.0, 0.0)
                };
                let xl = dm + rho;
                let h = (r * r - xl * xl).max(0.0).sqrt();
                let w = [xl * ex, xl * ey, -h];
                return Some((ub, sphere_param(w, fx, fy, ax, r)));
            }
            let Some((v, rho)) = disk_drop(&col.segs, r, d) else {
                return Some((f64::NEG_INFINITY, None));
            };
            let rho = inside(col, rho);
            let ub = Ival::pt(o[2]).add(v).hi;
            // the contact: ρ toward the centre, at the disk's height there
            let dm = d.mid();
            let (ux, uy) = if dm > 1e-12 {
                (-dx.mid() / dm, -dy.mid() / dm)
            } else {
                (1.0, 0.0)
            };
            let off = rho - dm;
            let h = (r * r - off * off).max(0.0).sqrt();
            let w = [x + rho * ux - o[0], y + rho * uy - o[1], h];
            let at = sphere_param(w, fx, fy, ax, r);
            Some((ub, at))
        }
        Surf::Cyl {
            o,
            x: fx,
            y: fy,
            ax,
            r,
        } => {
            if ax[2] != 0.0 {
                return None;
            }
            // the axis in xy, unit
            let al = (ax[0] * ax[0] + ax[1] * ax[1]).sqrt();
            let a = [Ival::pt(ax[0]), Ival::pt(ax[1])];
            let len = a[0].sqr().add(a[1].sqr()).sqrt();
            let rx = Ival::pt(x).sub(Ival::pt(o[0]));
            let ry = Ival::pt(y).sub(Ival::pt(o[1]));
            // the distance across the axis: |r × a| / |a|
            let d = rx.mul(a[1]).sub(ry.mul(a[0])).abs().div(len);
            let t = (rx.mid() * ax[0] + ry.mid() * ax[1]) / (al * al);
            let foot = [o[0] + t * ax[0], o[1] + t * ax[1]];
            let (fxv, fyv) = (foot[0] - x, foot[1] - y);
            let dm = (fxv * fxv + fyv * fyv).sqrt();
            let (ux, uy) = if dm > 1e-12 {
                (fxv / dm, fyv / dm)
            } else {
                (1.0, 0.0)
            };
            let (ub, w) = if bowl {
                // lateral places along n = (a.y, −a.x) / |a|: the tool's, signed, and the arc's
                let ds = rx.mul(a[1]).sub(ry.mul(a[0])).div(len);
                let (xa, xb) = cyl_lateral(&f.surf, f.p);
                let Some((v, rho)) = bowl_drop(col, r, ds, xa, xb) else {
                    return Some((f64::NEG_INFINITY, None));
                };
                let rho = inside_signed(col, rho);
                let (nx, ny) = (ax[1] / al, -ax[0] / al);
                let xl = ds.mid() + rho;
                let h = (r * r - xl * xl).max(0.0).sqrt();
                (
                    Ival::pt(o[2]).add(v).add(Ival::pt(lift)).hi,
                    [xl * nx, xl * ny, -h],
                )
            } else {
                let Some((v, rho)) = disk_drop(&col.segs, r, d) else {
                    return Some((f64::NEG_INFINITY, None));
                };
                let rho = inside(col, rho);
                // the contact: the foot on the axis, ρ toward it
                let off = rho - dm;
                let h = (r * r - off * off).max(0.0).sqrt();
                (
                    Ival::pt(o[2]).add(v).hi,
                    [x + rho * ux - foot[0], y + rho * uy - foot[1], h],
                )
            };
            // across the axis: (c, s) in the frame (x, y)
            let cx = (w[0] * fx[0] + w[1] * fx[1] + w[2] * fx[2]) / r;
            let cy = (w[0] * fy[0] + w[1] * fy[1] + w[2] * fy[2]) / r;
            if !(cx > -0.999) {
                return Some((ub, None));
            }
            Some((ub, Some((cy / (1.0 + cx), t))))
        }
        Surf::Torus {
            o,
            x: fx,
            y: fy,
            ax,
            big,
            r,
            cm,
            sm,
        } if level_torus(&f.surf) => {
            let ub = closed_t::<Ival>(col, f, Ival::pt(x), Ival::pt(y))?;
            let Some(ub) = ub else {
                return Some((f64::NEG_INFINITY, None));
            };
            let ub = ub.hi;
            // the contact, from the 2D problem's argument
            let (ex0, ey0) = (x - o[0], y - o[1]);
            let dd = (ex0 * ex0 + ey0 * ey0).sqrt();
            let (ex, ey) = if dd > 1e-12 {
                (ex0 / dd, ey0 / dd)
            } else {
                (1.0, 0.0)
            };
            let d = dd - big;
            let (xr, hw) = if bowl {
                let (xa, xb) = tube_lateral(f);
                let Some((_, rho)) = bowl_drop(col, r, Ival::pt(d), xa, xb) else {
                    return Some((ub, None));
                };
                let xr = d + inside_signed(col, rho);
                (xr, -(r * r - xr * xr).max(0.0).sqrt())
            } else {
                let Some((_, rho)) = disk_drop(&col.segs, r, Ival::pt(d.abs())) else {
                    return Some((ub, None));
                };
                let rho = inside(col, rho);
                let xr = if d >= 0.0 { d - rho } else { d + rho };
                (xr, (r * r - xr * xr).max(0.0).sqrt())
            };
            // φ: cos φ = xr / r, sin φ = height / r along ax
            let (cf, sf) = (xr / r, hw * ax[2] / r);
            let (cp, sp) = (cf * cm + sf * sm, sf * cm - cf * sm);
            if !(1.0 + cp > 1e-12) {
                return Some((ub, None));
            }
            let q = sp / (1.0 + cp);
            let (cx, cy) = (ex * fx[0] + ey * fx[1], ex * fy[0] + ey * fy[1]);
            if !(1.0 + cx > 1e-12) {
                return Some((ub, None));
            }
            Some((ub, Some((cy / (1.0 + cx), q))))
        }
        _ => None,
    }
}

/// A contact radius moved a hair inside the column (the rim, ρ = R, is where a flat end touches
/// a convex face, and a point there cannot be proven within reach): the lower bound loses only
/// the profile's slope times the hair.
fn inside(col: &Column, rho: f64) -> f64 {
    let hair = 1e-9 * (1.0 + col.r);
    if rho > col.r - hair { col.r - hair } else { rho }
}

/// A vertical wall over a section curve up to z1: every point is at least D (slab.rs's distance
/// from (x, y) to the section, an arc's stored ends lowered by twice their gap) across from the
/// tool's axis and at most z1 high, and b is non-decreasing, so z1 − b(D) bounds its CL (−∞ when
/// D > R). The witness: the section's nearest point, a hair under the top.
fn wall_drop(
    col: &Column,
    f: &Face,
    sec: &crate::slab::Sec,
    z1: f64,
    gap: f64,
    x: f64,
    y: f64,
) -> (f64, Option<(f64, f64)>) {
    let d = crate::slab::pt_sec([x, y], sec);
    let dl = Ival::pt(d.lo).sub(Ival::pt(gap).scale(2.0)).lo;
    let dl = if dl > 0.0 { dl } else { 0.0 };
    let Some(bz) = col.b(Ival::pt(dl)) else {
        return (f64::NEG_INFINITY, None);
    };
    let ub = Ival::pt(z1).sub(Ival::pt(bz.lo)).hi;
    // the nearest point of the section, in f64
    let near = match *sec {
        crate::slab::Sec::Seg { a, b } => {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let dd = dx * dx + dy * dy;
            let t = if dd > 0.0 {
                (((x - a[0]) * dx + (y - a[1]) * dy) / dd).clamp(0.0, 1.0)
            } else {
                0.0
            };
            [a[0] + t * dx, a[1] + t * dy]
        }
        crate::slab::Sec::Arc { c, r, a, b, ccw } => {
            let (vx, vy) = (x - c[0], y - c[1]);
            let l = (vx * vx + vy * vy).sqrt();
            let p = if l > 0.0 {
                [c[0] + r * vx / l, c[1] + r * vy / l]
            } else {
                a
            };
            // on the arc if between its ends in its sense, else the nearer end
            let cr = |u: [f64; 2], w: [f64; 2]| (u[0] - c[0]) * (w[1] - c[1]) - (u[1] - c[1]) * (w[0] - c[0]);
            let within = if ccw {
                cr(a, p) >= 0.0 && cr(p, b) >= 0.0
            } else {
                cr(a, p) <= 0.0 && cr(p, b) <= 0.0
            };
            if within {
                p
            } else {
                let da = (a[0] - x).powi(2) + (a[1] - y).powi(2);
                let db = (b[0] - x).powi(2) + (b[1] - y).powi(2);
                if da <= db { a } else { b }
            }
        }
    };
    let zt = z1 - 4.0 * f.mu.max(1e-12) * (1.0 + z1.abs());
    let at = match f.surf {
        Surf::Plane { o, u, v } => {
            // least squares for o + p u + q v = (near, zt)
            let w = [near[0] - o[0], near[1] - o[1], zt - o[2]];
            let (uu, uv, vv) = (dot3(u, u), dot3(u, v), dot3(v, v));
            let (wu, wv) = (dot3(w, u), dot3(w, v));
            let det = uu * vv - uv * uv;
            if det.abs() > 0.0 {
                Some(((wu * vv - wv * uv) / det, (uu * wv - uv * wu) / det))
            } else {
                None
            }
        }
        Surf::Cyl {
            o,
            x: fx,
            y: fy,
            ax,
            r,
        } => {
            let (vx, vy) = ((near[0] - o[0]) / r, (near[1] - o[1]) / r);
            let (cx, cy) = (vx * fx[0] + vy * fx[1], vx * fy[0] + vy * fy[1]);
            if 1.0 + cx > 1e-12 {
                Some((cy / (1.0 + cx), (zt - o[2]) / ax[2]))
            } else {
                None
            }
        }
        _ => None,
    };
    (ub, at)
}

fn dot3(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `inside` for a signed radius.
fn inside_signed(col: &Column, rho: f64) -> f64 {
    if rho < 0.0 {
        -inside(col, -rho)
    } else {
        inside(col, rho)
    }
}

/// A sphere, a cylinder with a horizontal axis or a torus about a vertical axis whose face lies at
/// or below its centre (axis, tube centre) but for η ≤ 2e-4 (the lowering pads a face's parameter
/// box by its mu, which lifts a quarter's end that much): a bowl, whose drop is
/// the lower half's (`bowl_drop`) plus 2η (a point of the face above the centre by δ ≤ η is
/// within 2δ above the lower half's point at the same lateral place). None: not a bowl.
fn bowl_eta(f: &Face) -> Option<f64> {
    // the most a point of the face rises above the centre / axis, bounded from the
    // parametrisation (the face's box is too loose for this)
    let rise = match f.surf {
        Surf::Sphere { ax, x, y, .. } => {
            // only with the axis vertical: then z − o.z = ±r sin(elevation), elevation's sign q's
            if x[2] != 0.0 || y[2] != 0.0 {
                return None;
            }
            if ax[2] > 0.0 && f.q.hi <= 0.0 || ax[2] < 0.0 && f.q.lo >= 0.0 {
                0.0
            } else {
                return None;
            }
        }
        Surf::Cyl { ax, x, y, r, .. } if ax[2] == 0.0 => {
            // z − o.z = r (c x.z + s y.z) over the arc t ∈ p: its ends', unless the arc holds the
            // direction (x.z, y.z) where it peaks
            let (a, b) = (Ival::pt(x[2]), Ival::pt(y[2]));
            let end = |t: f64| {
                let (c, s) = circ_iv(Ival::pt(t));
                c.mul(a).add(s.mul(b))
            };
            let (e0, e1) = (end(f.p.lo), end(f.p.hi));
            // the peak direction (a, b) inside the arc (from angle of p.lo to p.hi, under a
            // half turn): cross(e_lo, d) ≥ 0 and cross(d, e_hi) ≥ 0, possibly
            let (c0, s0) = circ_iv(Ival::pt(f.p.lo));
            let (c1, s1) = circ_iv(Ival::pt(f.p.hi));
            let k0 = c0.mul(b).sub(s0.mul(a));
            let k1 = a.mul(s1).sub(b.mul(c1));
            if k0.hi >= 0.0 && k1.hi >= 0.0 && (a.hi > 0.0 || b.hi > 0.0 || a.lo < 0.0 || b.lo < 0.0) {
                return None;
            }
            let m = if e0.hi > e1.hi { e0.hi } else { e1.hi };
            Ival::pt(r).mul(Ival::pt(m)).hi
        }
        Surf::Torus { ax, x, y, r, .. } if x[2] == 0.0 && y[2] == 0.0 && (ax[2] == 1.0 || ax[2] == -1.0) => {
            // in the tube's plane (outward, up): the direction (cos φ, sin φ), height r sin φ ax.z;
            // the arc's ends, unless it holds the peak direction (0, ax.z)
            let (e0, e1) = tube_ends(&f.surf, f.q)?;
            let az = Ival::pt(ax[2]);
            let peak = [Ival::pt(0.0), az];
            let k0 = e0[0].mul(peak[1]).sub(e0[1].mul(peak[0]));
            let k1 = peak[0].mul(e1[1]).sub(peak[1].mul(e1[0]));
            if k0.hi >= 0.0 && k1.hi >= 0.0 {
                return None;
            }
            let h0 = e0[1].mul(az).hi;
            let h1 = e1[1].mul(az).hi;
            Ival::pt(r).mul(Ival::pt(if h0 > h1 { h0 } else { h1 })).hi
        }
        _ => return None,
    };
    if rise <= 0.0 {
        Some(0.0)
    } else if rise <= 2e-4 {
        Some(rise)
    } else {
        None
    }
}

/// A horizontal cylinder face's lateral range across its axis, along n = (a.y, −a.x) / |a|:
/// r e(t)·n at the arc's ends, outward (on a bowl's arc, within the lower half, the lateral
/// place is monotone along the arc, so its ends bound it).
fn cyl_lateral(s: &Surf, p: Ival) -> (f64, f64) {
    match *s {
        Surf::Cyl { x, y, ax, r, .. } => {
            let len = Ival::pt(ax[0]).sqr().add(Ival::pt(ax[1]).sqr()).sqrt();
            let (nx, ny) = (Ival::pt(ax[1]).div(len), Ival::pt(-ax[0]).div(len));
            let lat = |t: f64| {
                let (c, sn) = circ_iv(Ival::pt(t));
                let ex = c.mul(Ival::pt(x[0])).add(sn.mul(Ival::pt(y[0])));
                let ey = c.mul(Ival::pt(x[1])).add(sn.mul(Ival::pt(y[1])));
                ex.mul(nx).add(ey.mul(ny)).mul(Ival::pt(r))
            };
            let (a, b) = (lat(p.lo), lat(p.hi));
            let lo = if a.lo < b.lo { a.lo } else { b.lo };
            let hi = if a.hi > b.hi { a.hi } else { b.hi };
            (lo.max(-r), hi.min(r))
        }
        _ => (f64::NEG_INFINITY, f64::INFINITY),
    }
}

/// A torus face's tube directions (cos φ, sin φ) at its two tube ends, φ = φm + ψ, ψ = 2 atan q.
fn tube_ends(s: &Surf, q: Ival) -> Option<([Ival; 2], [Ival; 2])> {
    match *s {
        Surf::Torus { cm, sm, .. } => {
            let at = |t: f64| {
                let (cp, sp) = circ_iv(Ival::pt(t));
                let (c, sn) = (Ival::pt(cm), Ival::pt(sm));
                [cp.mul(c).sub(sp.mul(sn)), cp.mul(sn).add(sp.mul(c))]
            };
            Some((at(q.lo), at(q.hi)))
        }
        _ => None,
    }
}

/// A vertical-axis torus face's lateral range r cos φ over its tube arc, outward.
fn tube_lateral(f: &Face) -> (f64, f64) {
    match (f.surf, tube_ends(&f.surf, f.q)) {
        (Surf::Torus { r, .. }, Some((e0, e1))) => {
            let (a, b) = (e0[0].mul(Ival::pt(r)), e1[0].mul(Ival::pt(r)));
            let lo = if a.lo < b.lo { a.lo } else { b.lo };
            let hi = if a.hi > b.hi { a.hi } else { b.hi };
            (lo.max(-r), hi.min(r))
        }
        _ => (f64::NEG_INFINITY, f64::INFINITY),
    }
}

/// A torus about a vertical axis (its frame level).
fn level_torus(s: &Surf) -> bool {
    matches!(*s, Surf::Torus { ax, x, y, .. } if x[2] == 0.0 && y[2] == 0.0 && (ax[2] == 1.0 || ax[2] == -1.0))
}

fn circ_iv(t: Ival) -> (Ival, Ival) {
    let t2 = t.sqr();
    let den = Ival::pt(1.0).add(t2);
    (Ival::pt(1.0).sub(t2).div(den), t.scale(2.0).div(den))
}

fn cross(u: V3, v: V3) -> [Ival; 3] {
    let (u0, u1, u2) = (Ival::pt(u[0]), Ival::pt(u[1]), Ival::pt(u[2]));
    let (v0, v1, v2) = (Ival::pt(v[0]), Ival::pt(v[1]), Ival::pt(v[2]));
    [
        u1.mul(v2).sub(u2.mul(v1)),
        u2.mul(v0).sub(u0.mul(v2)),
        u0.mul(v1).sub(u1.mul(v0)),
    ]
}

/// A plane with normal n (n.z > 0) through o: CL = z_plane(x, y) − min over ρ of b(ρ) − τ ρ,
/// τ = |n_xy| / n_z, the column's lowest support against the slope (it touches uphill).
fn plane_drop(
    col: &Column,
    o: V3,
    u: V3,
    v: V3,
    n: [Ival; 3],
    x: f64,
    y: f64,
) -> Option<(f64, Option<(f64, f64)>)> {
    let dx = Ival::pt(x).sub(Ival::pt(o[0]));
    let dy = Ival::pt(y).sub(Ival::pt(o[1]));
    let zp = Ival::pt(o[2]).sub(n[0].mul(dx).add(n[1].mul(dy)).div(n[2]));
    let nh = n[0].sqr().add(n[1].sqr()).sqrt();
    let (m, arg) = plane_support(col, n);
    let ub = zp.sub(m).hi;
    let arg = inside(col, arg);
    // the contact: arg uphill of the tip (against n's horizontal part), on the plane
    let nhm = nh.mid();
    let (ex, ey) = if nhm > 0.0 {
        (n[0].mid() / nhm, n[1].mid() / nhm)
    } else {
        (0.0, 0.0)
    };
    let cx = x - arg * ex;
    let cy = y - arg * ey;
    let at = plane_param(o, u, v, cx, cy);
    Some((ub, at))
}

/// The least over ρ ∈ [0, R] of b(ρ) − τ ρ, τ = |n_xy| / n_z (n.z > 0), enclosed, and (in f64)
/// the smallest ρ attaining it: per piece its ends, and an arc's critical point where it may lie
/// in the arc (b − τρ is convex on an arc, so that critical value is below the arc's ends).
fn plane_support(col: &Column, n: [Ival; 3]) -> (Ival, f64) {
    let nh = n[0].sqr().add(n[1].sqr()).sqrt();
    let tau = nh.div(n[2]);
    let mut m = Ival::pt(f64::INFINITY);
    let mut arg = 0.0f64;
    let mut mf = f64::INFINITY;
    let mut i = 0;
    while i < col.segs.len() {
        let sg = col.segs[i];
        let (a, b) = (sg.a(), sg.b());
        for pnt in [a, b] {
            let g = Ival::pt(pnt[1]).sub(tau.mul(Ival::pt(pnt[0])));
            m = m.min(g);
            // the smallest ρ among (near) ties: a flat bottom on a level plane touches at its axis
            if g.mid() < mf - 1e-15 {
                mf = g.mid();
                arg = pnt[0];
            }
        }
        if let CSeg::Arc { a, b, c, r } = sg {
            // d* = r τ / √(1 + τ²); value c.z − τ c.ρ − r √(1 + τ²)
            let w = Ival::pt(1.0).add(tau.sqr()).sqrt();
            let ds = Ival::pt(r).mul(tau).div(w);
            let rs = Ival::pt(c[0]).add(ds);
            if rs.hi >= a[0] && rs.lo <= b[0] {
                let g = Ival::pt(c[1])
                    .sub(tau.mul(Ival::pt(c[0])))
                    .sub(Ival::pt(r).mul(w));
                m = m.min(g);
                if g.mid() < mf - 1e-15 {
                    mf = g.mid();
                    arg = rs.mid().max(a[0]).min(b[0]);
                }
            }
        }
        i += 1;
    }
    (m, arg)
}

/// The plane's parameters (p, q) of the point above (x, y): solves o.xy + p u.xy + q v.xy = (x, y).
fn plane_param(o: V3, u: V3, v: V3, x: f64, y: f64) -> Option<(f64, f64)> {
    let det = u[0] * v[1] - u[1] * v[0];
    if !(det.abs() > 1e-300) {
        return None;
    }
    let (rx, ry) = (x - o[0], y - o[1]);
    let p = (rx * v[1] - ry * v[0]) / det;
    let q = (u[0] * ry - u[1] * rx) / det;
    if p.is_finite() && q.is_finite() {
        Some((p, q))
    } else {
        None
    }
}

/// A sphere's parameters for a point w (relative to the centre): p = tan(azimuth / 2) in the
/// frame (x, y), q = tan(elevation / 2) toward ax.
fn sphere_param(w: V3, fx: V3, fy: V3, ax: V3, r: f64) -> Option<(f64, f64)> {
    let cx = w[0] * fx[0] + w[1] * fx[1] + w[2] * fx[2];
    let cy = w[0] * fy[0] + w[1] * fy[1] + w[2] * fy[2];
    let sz = (w[0] * ax[0] + w[1] * ax[1] + w[2] * ax[2]) / r;
    let ch = (cx * cx + cy * cy).sqrt();
    let p = if ch <= 1e-12 * r {
        // the pole: any azimuth
        0.0
    } else if ch + cx > 1e-12 * r {
        cy / (ch + cx)
    } else {
        return None;
    };
    let cz = ch / r;
    if !(1.0 + cz > 1e-12) {
        return None;
    }
    let q = sz / (1.0 + cz);
    if p.is_finite() && q.is_finite() {
        Some((p, q))
    } else {
        None
    }
}

/// What the vertical-gap check proved for a move against a face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gap {
    /// v ≥ lower ≥ limit, lower ≤ 0 (the face narrowed by mu).
    Clear { lower: f64 },
    /// A real face point (parameters p, q, proven in the face) at a real instant s lies more
    /// than −limit inside the column (the face widened by mu): the column, not necessarily the
    /// cutter, meets the face (the caller checks the cutter's own distance there).
    Over { s: f64, by: f64, p: f64, q: f64 },
    /// Neither within the budget or the resolution.
    Unknown,
}

/// The vertical gap of a move against one face (see the module's statement). `limit` must be
/// ≤ 0. Moves of every kind; a helix is enclosed by its arc over its heights.
///
/// First by the face's closed form, if it has one, over pieces of s (an upper bound of the
/// untrimmed surface's CL along the piece, by the mean-value form in s); the pieces it cannot
/// clear go to the branch and bound over the face's parameters and s.
pub fn gap_move(col: &Column, f: &Face, mv: &Move, limit: f64, res: f64, budget: usize) -> (Gap, usize) {
    if limit > 0.0 {
        return (Gap::Unknown, 0);
    }
    // the move's box against the face's: out of the footprint, nothing to prove
    let sb = mv.at(Ival::new(0.0, 1.0));
    let r = Ival::pt(col.r).add(Ival::pt(f.mu)).hi;
    let gx = box_gap(sb[0], f.bbox[0]);
    let gy = box_gap(sb[1], f.bbox[1]);
    if Ival::pt(gx).sqr().add(Ival::pt(gy).sqr()).lo > Ival::pt(r).sqr().hi {
        return (Gap::Clear { lower: 0.0 }, 0);
    }
    // proven: u ≤ -limit - mu
    let need = Ival::pt(-limit).sub(Ival::pt(f.mu)).lo;
    // a vertical wall: every point of it is at least D across from the move's xy path (slab.rs's
    // 2D distance, its arc's stored ends lowered by twice their gap), so no point of it is more
    // than R − D inside the column (the radial way out)
    if let Some(crate::slab::Prism::Wall { sec, gap, .. }) = &f.prism
        && let Some(path) = xy_path(mv)
    {
        let d = crate::slab::dist(&path, sec).lo;
        let d = Ival::pt(d).sub(Ival::pt(*gap).scale(2.0)).lo;
        let d = if d > 0.0 { d } else { 0.0 };
        let mut depth = Ival::pt(col.r).sub(Ival::pt(d)).hi;
        // and the vertical way: no point of the wall is above its top z1, nor nearer than D, so
        // none is more than z1 − b(D) − tip above the profile (the tip's lowest over the move)
        if let (Some(bz), Some(crate::slab::Prism::Wall { z1, .. })) = (col.b(Ival::pt(d)), &f.prism) {
            let tip = mv.at(Ival::new(0.0, 1.0))[2].lo;
            let v = Ival::pt(*z1).sub(Ival::pt(bz.lo)).sub(Ival::pt(tip)).hi;
            if v < depth {
                depth = v;
            }
        }
        if depth <= need {
            let v = Ival::pt(depth).add(Ival::pt(f.mu)).neg().lo;
            return (
                Gap::Clear {
                    lower: if v < 0.0 { v } else { 0.0 },
                },
                1,
            );
        }
    }
    let convex = col.convex();
    let dome_cyl = convex && matches!(f.surf, Surf::Cyl { ax, .. } if ax[2] == 0.0) && bowl_eta(f).is_none();
    let sq_convex = dome_cyl && col.sq_convex();
    let mut cells = 0usize;
    let mut worst = f64::NEG_INFINITY;
    let mut rest: Vec<Ival> = Vec::new();
    if has_closed(&f.surf) {
        let mut st: Vec<Ival> = vec![Ival::new(0.0, 1.0)];
        while let Some(si) = st.pop() {
            cells += 1;
            if cells > budget {
                return (Gap::Unknown, cells);
            }
            // a sphere against a convex column: no point of it is deeper than S − sd(centre)
            // (the column's supporting half-space at the centre's nearest point)
            let mut sphere_may = false;
            if let Surf::Sphere { o, r, .. } = f.surf
                && convex
            {
                let g = mv.at(si);
                let rel = [
                    Ival::pt(o[0]).sub(g[0]),
                    Ival::pt(o[1]).sub(g[1]),
                    Ival::pt(o[2]).sub(g[2]),
                ];
                let rho = rel[0].sqr().add(rel[1].sqr()).sqrt();
                let sd = col.solid.sd_rz(rho, rel[2]);
                let depth = Ival::pt(r).sub(sd).hi;
                if depth <= need {
                    if depth > worst {
                        worst = depth;
                    }
                    continue;
                }
                // a smaller piece of s may clear it only if the bound's other end would
                sphere_may = Ival::pt(r).sub(sd).lo <= need;
            }
            // a dome cylinder (a horizontal axis) against a convex column along a line: exact
            // over the piece (its untrimmed CL is concave across the axis, so the piece's worst
            // point is the unconstrained maximum or its nearer end: `cyl_chord`, Concave.lean)
            let chord = if dome_cyl {
                match (*mv, si.lo == si.hi) {
                    (Move::Line { .. }, false) => {
                        // past the face's end along the axis by a: the column's section there
                        // (ball and flat profiles), else the whole profile
                        let ga = mv.at(Ival::pt(si.lo));
                        let gb = mv.at(Ival::pt(si.hi));
                        // (the face widened by mu: its span too)
                        let a = Ival::pt(beyond_end(f, mv.at(si))).sub(Ival::pt(f.mu)).lo;
                        if a > 0.0
                            && let Some(sec) = section(&col.segs, a)
                        {
                            if sec.is_empty() {
                                Some(f64::NEG_INFINITY)
                            } else {
                                cyl_chord_iv(&sec, f, ga, gb)
                            }
                        } else {
                            cyl_chord_iv(&col.segs, f, ga, gb).map(|u| {
                                // h = b(√·) convex: lower by b(a) − b(0)
                                if a > 0.0 && sq_convex && u != f64::NEG_INFINITY {
                                    match col.b(Ival::pt(a)) {
                                        None => f64::NEG_INFINITY,
                                        Some(ba) => Ival::pt(u).sub(ba).add(Ival::pt(col.segs[0].a()[1])).hi,
                                    }
                                } else {
                                    u
                                }
                            })
                        }
                    }
                    _ => None,
                }
            } else {
                None
            };
            if let Some(u) = chord {
                if u <= need {
                    if u > worst {
                        worst = u;
                    }
                    continue;
                }
            } else {
                match closed_u(col, f, mv, si) {
                    Some(None) => continue,
                    Some(Some(u)) if u <= need => {
                        if u > worst {
                            worst = u;
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            // at the piece's middle: the contact, if it is in the face, is a witness or shows the
            // closed form is the face's own there (then split s); else the face's edge decides
            let sm = si.mid();
            let g = mv.at(Ival::pt(sm));
            // a sphere's bound against a convex column tightens as s shrinks: split for it too
            // a dome cylinder's exact bound tightens as s shrinks too (a piece across the face's
            // end splits into one within the span and one past it)
            let mut own = sphere_may || chord.is_some();
            if let Some((_, Some((p, q)))) = closed(col, f, g[0].mid(), g[1].mid())
                && f.holds(Ival::pt(p), Ival::pt(q)) == 1
            {
                own = true;
                let at = Cell {
                    p: Ival::pt(p),
                    q: Ival::pt(q),
                    s: Ival::pt(sm),
                };
                let w = Ival::pt(side_lower(col, f, mv, &at)).sub(Ival::pt(f.mu)).lo;
                if w > -limit {
                    return (Gap::Over { s: sm, by: w, p, q }, cells);
                }
            }
            let img = diam2(&mv.at(si));
            if !own || img <= res.max(64.0 * f64::EPSILON) || si.width() == 0.0 {
                rest.push(si);
            } else {
                let (a, b) = si.split();
                st.push(a);
                st.push(b);
            }
        }
    } else {
        rest.push(Ival::new(0.0, 1.0));
    }
    // best first (largest bound), so a violation is met before the search spends itself on
    // cells near the limit; a cell too small to split leaves the verdict unknown, but the search
    // goes on for a witness
    let mut heap = Heap { v: Vec::new() };
    let mut i = 0;
    while i < rest.len() {
        let c = Cell {
            p: f.p,
            q: f.q,
            s: rest[i],
        };
        cells += 1;
        if let Some((u, _)) = cell_upper_side(col, f, mv, &c, true) {
            heap.push(u, 0, c);
        }
        i += 1;
    }
    let mut unknown = false;
    while let Some((u, _, c)) = heap.pop() {
        if u <= need {
            if u > worst {
                worst = u;
            }
            continue;
        }
        if cells > budget {
            return (Gap::Unknown, cells);
        }
        // a real point over the limit: proven over, stop
        let sm = c.s.mid();
        let at = Cell {
            p: c.p,
            q: c.q,
            s: Ival::pt(sm),
        };
        let w = Ival::pt(side_lower(col, f, mv, &at)).sub(Ival::pt(f.mu)).lo;
        if w > -limit {
            return (
                Gap::Over {
                    s: sm,
                    by: w,
                    p: c.p.mid(),
                    q: c.q.mid(),
                },
                cells,
            );
        }
        let img = diam2(&f.surf.eval(c.p, c.q)).max(diam2(&mv.at(c.s)));
        if img <= res || (c.p.width() == 0.0 && c.q.width() == 0.0 && c.s.width() == 0.0) {
            unknown = true;
            continue;
        }
        let k = match cell_upper_side(col, f, mv, &c, true) {
            Some((_, k)) => k,
            None => continue,
        };
        let (a, b) = split(&c, k);
        for h in [a, b] {
            cells += 1;
            if f.holds(h.p, h.q) == -1 {
                continue;
            }
            if let Some((u2, _)) = cell_upper_side(col, f, mv, &h, true) {
                if u2 <= need {
                    if u2 > worst {
                        worst = u2;
                    }
                } else {
                    heap.push(u2, 0, h);
                }
            }
        }
    }
    if unknown {
        return (Gap::Unknown, cells);
    }
    let v = if worst == f64::NEG_INFINITY {
        0.0
    } else {
        Ival::pt(worst).add(Ival::pt(f.mu)).neg().lo
    };
    (
        Gap::Clear {
            lower: if v < 0.0 { v } else { 0.0 },
        },
        cells,
    )
}

/// A convex profile's conjugate at slope k ≥ 0 (an interval): an upper bound of
/// B(k) = max over ρ ∈ [0, R] of k ρ − b(ρ), and an interval holding a maximiser ρ* (for every
/// slope in k: the maximisers move outward with k, b being convex). None: the profile does not
/// start on the axis.
fn conj(segs: &[CSeg], cr: f64, k: Ival) -> Option<(f64, Ival)> {
    if segs[0].a()[0] != 0.0 || k.lo < 0.0 {
        return None;
    }
    let one = Ival::pt(1.0);
    let mut best = f64::NEG_INFINITY;
    let mut up = |v: Ival| {
        if v.hi > best {
            best = v.hi;
        }
    };
    // an arc's tangency place for slope k: c.ρ + r k / √(1 + k²)
    let tang =
        |c: [f64; 2], r: f64, k: Ival| Ival::pt(c[0]).add(Ival::pt(r).mul(k).div(one.add(k.sqr()).sqrt()));
    for sg in segs {
        let (a, b) = (sg.a(), sg.b());
        // the ends (a line's maximum is at one of them)
        up(k.mul(Ival::pt(a[0])).sub(Ival::pt(a[1])));
        up(k.mul(Ival::pt(b[0])).sub(Ival::pt(b[1])));
        if let CSeg::Arc { a, b, c, r } = *sg {
            // k ρ − c.z + √(r² − (ρ − c.ρ)²) is concave: its stationary value bounds the piece
            // (taken when the place may be inside it; else the ends hold the piece's maximum)
            let t = tang(c, r, k);
            if t.hi >= a[0] && t.lo <= b[0] {
                up(k.mul(Ival::pt(c[0]))
                    .sub(Ival::pt(c[1]))
                    .add(Ival::pt(r).mul(one.add(k.sqr()).sqrt())));
            }
        }
    }
    // ρ*: from below, the first place whose right slope may reach k.lo; from above, the last
    // whose left slope may be at most k.hi
    let mut lo = cr;
    for sg in segs {
        let (a, b) = (sg.a(), sg.b());
        let below = match *sg {
            CSeg::Line { .. } => {
                let sl = Ival::pt(b[1])
                    .sub(Ival::pt(a[1]))
                    .div(Ival::pt(b[0]).sub(Ival::pt(a[0])));
                sl.hi < k.lo
            }
            CSeg::Arc { c, r, .. } => tang(c, r, Ival::pt(k.lo)).lo > b[0],
        };
        if !below {
            lo = match *sg {
                CSeg::Line { .. } => a[0],
                CSeg::Arc { c, r, .. } => tang(c, r, Ival::pt(k.lo)).lo.max(a[0]),
            };
            break;
        }
    }
    let mut hi = 0.0f64;
    for sg in segs.iter().rev() {
        let (a, b) = (sg.a(), sg.b());
        let above = match *sg {
            CSeg::Line { .. } => {
                let sl = Ival::pt(b[1])
                    .sub(Ival::pt(a[1]))
                    .div(Ival::pt(b[0]).sub(Ival::pt(a[0])));
                sl.lo > k.hi
            }
            CSeg::Arc { c, r, .. } => tang(c, r, Ival::pt(k.hi)).hi < a[0],
        };
        if !above {
            hi = match *sg {
                CSeg::Line { .. } => b[0],
                CSeg::Arc { c, r, .. } => tang(c, r, Ival::pt(k.hi)).hi.min(b[0]),
            };
            break;
        }
    }
    if !(lo <= hi) {
        // rounding at a joint: the whole range holds it
        return Some((best, Ival::new(0.0, cr)));
    }
    Some((best, Ival::new(lo, hi)))
}

/// A line move against a dome cylinder (horizontal axis, the face above it) and a convex column:
/// an upper bound of u(s) = CL(g(s)) − g(s).z over the whole move for the untrimmed cylinder
/// (−∞: out of reach throughout). In the plane across the axis, with d the tool's signed
/// lateral place, CL = o.z + φ(d), φ(d) = max over e − u = d of √(r² − e²) − b(|u|): the sum of
/// two concave functions' sup-convolution, so concave. Along the move d and z are affine, so
/// with λ = Δz/Δd, u = φ(d) − λ d + const: its maximum over all d is
/// r √(1 + λ²) + B(|λ|) (the disk's and the profile's conjugates), attained at
/// d* = e* − u*, e* = −λ r / √(1 + λ²), u* = sign(λ) ρ*. If d* is outside the move's range of
/// d, the maximum over the move is at its nearer end (concavity); else it is the unconstrained
/// one. None: not this case (or Δd not proven non-zero: the existing route handles it).
pub fn cyl_chord(col: &Column, f: &Face, mv: &Move) -> Option<f64> {
    let Move::Line { a, b } = *mv else {
        return None;
    };
    if !col.convex() {
        return None;
    }
    let iv = |p: [f64; 3]| [Ival::pt(p[0]), Ival::pt(p[1]), Ival::pt(p[2])];
    cyl_chord_iv(&col.segs, f, iv(a), iv(b))
}

/// `cyl_chord` over a straight piece given by its ends as intervals (each holds the true end);
/// the column is convex (the caller's).
fn cyl_chord_iv(segs: &[CSeg], f: &Face, a: [Ival; 3], b: [Ival; 3]) -> Option<f64> {
    let Surf::Cyl { o, ax, r, .. } = f.surf else {
        return None;
    };
    if ax[2] != 0.0 || bowl_eta(f).is_some() {
        return None;
    }
    let len = Ival::pt(ax[0]).sqr().add(Ival::pt(ax[1]).sqr()).sqrt();
    let lat = |p: [Ival; 3]| {
        let rx = p[0].sub(Ival::pt(o[0]));
        let ry = p[1].sub(Ival::pt(o[1]));
        rx.mul(Ival::pt(ax[1])).sub(ry.mul(Ival::pt(ax[0]))).div(len)
    };
    let (d0, d1) = (lat(a), lat(b));
    let dd = d1.sub(d0);
    if dd.sign() == 0 {
        // (nearly) along the axis: d over the piece is within the ends' hull (affine), z at
        // least the lower end's
        let zl = a[2].min(b[2]);
        return Some(match disk_drop(segs, r, d0.hull(d1).abs()) {
            Some((v, _)) => Ival::pt(o[2]).add(v).sub(zl).hi,
            None => f64::NEG_INFINITY,
        });
    }
    let lam = b[2].sub(a[2]).div(dd);
    let k = lam.abs();
    let cr = segs[segs.len() - 1].b()[0];
    let (bk, rho) = conj(segs, cr, k)?;
    let w = Ival::pt(1.0).add(lam.sqr()).sqrt();
    let es = lam.mul(Ival::pt(r)).div(w).neg();
    let us = match lam.sign() {
        1 => rho,
        -1 => rho.neg(),
        _ => Ival::new(-rho.hi, rho.hi),
    };
    let ds = es.sub(us);
    // the end nearer d* (the lower d first)
    let (lo_end, hi_end, dl, dh) = if dd.lo > 0.0 {
        (a, b, d0, d1)
    } else {
        (b, a, d1, d0)
    };
    // the untrimmed CL at an end (closed_t's dome cylinder: o.z + φ(|d|))
    let at_end = |p: [Ival; 3]| -> f64 {
        match disk_drop(segs, r, lat(p).abs()) {
            Some((v, _)) => Ival::pt(o[2]).add(v).sub(p[2]).hi,
            None => f64::NEG_INFINITY,
        }
    };
    if ds.hi < dl.lo {
        return Some(at_end(lo_end));
    }
    if ds.lo > dh.hi {
        return Some(at_end(hi_end));
    }
    // the unconstrained maximum: o.z + r √(1 + λ²) + B − (z − λ d) at either end
    let top = Ival::pt(o[2]).add(Ival::pt(r).mul(w)).add(Ival::pt(bk));
    let ua = top.sub(a[2].sub(lam.mul(d0))).hi;
    let ub = top.sub(b[2].sub(lam.mul(d1))).hi;
    Some(if ua < ub { ua } else { ub })
}

/// The column's section by a vertical plane a ≥ 0 from its axis, as a profile in the plane's
/// lateral place u: b_a(u) = b(√(u² + a²)), |u| ≤ √(R² − a²), for a one-piece profile: a flat
/// end, or a ball (b = c.z − √(r² − ρ²) gives c.z − √((r² − a²) − u²)); rounded to a profile
/// below it and as wide or wider. None for other profiles. Empty: a ≥ R (no part of the column
/// there). Convex if the column is (b convex and non-decreasing, √(u² + a²) convex).
fn section(segs: &[CSeg], a: f64) -> Option<Vec<CSeg>> {
    // one piece (a flat end, a ball): each bound rounded the column's way (wider, lower)
    let [sg] = segs else {
        return None;
    };
    let (pa, pb) = (sg.a(), sg.b());
    if pa[0] != 0.0 {
        return None;
    }
    if !(a < pb[0]) {
        return Some(Vec::new());
    }
    let a2 = Ival::pt(a).sqr();
    // the section's half-width √(R² − a²), up
    let ub = Ival::pt(pb[0]).sqr().sub(a2).sqrt().hi;
    match *sg {
        CSeg::Line { a: la, b: lb } if la[1] == lb[1] => Some(vec![CSeg::Line {
            a: [0.0, la[1]],
            b: [ub, la[1]],
        }]),
        CSeg::Arc { c, r, b: ab, .. } if c[0] == 0.0 => {
            // c.z − √((r² − a²) − u²): the radius up (the arc lower), its start b(a) down
            let rr = Ival::pt(r).sqr().sub(a2).sqrt().hi;
            let za = Ival::pt(c[1]).sub(Ival::pt(rr)).lo;
            Some(vec![CSeg::Arc {
                a: [0.0, za],
                b: [if ub > rr { ub } else { rr }, ab[1]],
                c,
                r: rr,
            }])
        }
        _ => None,
    }
}

/// How far (a lower bound, ≥ 0) the tool's axis is beyond a horizontal cylinder face's span
/// along its axis, over the tool positions g (0 where it may be within the span).
fn beyond_end(f: &Face, g: [Ival; 3]) -> f64 {
    let Surf::Cyl { o, ax, .. } = f.surf else {
        return 0.0;
    };
    let len = Ival::pt(ax[0]).sqr().add(Ival::pt(ax[1]).sqr()).sqrt();
    // the axial place: (g − o)·ax / |ax|; the face's span: q |ax|
    let t = g[0]
        .sub(Ival::pt(o[0]))
        .mul(Ival::pt(ax[0]))
        .add(g[1].sub(Ival::pt(o[1])).mul(Ival::pt(ax[1])))
        .div(len);
    let (q0, q1) = (Ival::pt(f.q.lo).mul(len), Ival::pt(f.q.hi).mul(len));
    let past = t.sub(q1).lo;
    let before = q0.sub(t).lo;
    let a = if past > before { past } else { before };
    if a > 0.0 { a } else { 0.0 }
}

/// A move's path in xy as a section curve: a line's (a point for a vertical move), or a planar
/// or helical arc's (each under a half turn, as the checker cuts them).
fn xy_path(mv: &Move) -> Option<crate::slab::Sec> {
    match *mv {
        Move::Line { a, b } => Some(crate::slab::Sec::Seg {
            a: [a[0], a[1]],
            b: [b[0], b[1]],
        }),
        _ => None,
    }
}

fn has_closed(s: &Surf) -> bool {
    match *s {
        Surf::Plane { .. } | Surf::Sphere { .. } => true,
        Surf::Cyl { ax, .. } => ax[2] == 0.0,
        Surf::Torus { .. } => level_torus(s),
        _ => false,
    }
}

/// An upper bound of u = CL(g(s)) − g(s).z over s ∈ si for the untrimmed surface (Some(None):
/// out of reach), by the mean-value form in s intersected with the natural one; None: no closed
/// form.
fn closed_u(col: &Column, sf: &Face, mv: &Move, si: Ival) -> Option<Option<f64>> {
    let gn: [Ival; 3] = mv.at(si);
    let nat = match closed_t::<Ival>(col, sf, gn[0], gn[1])? {
        None => return Some(None),
        Some(cl) => cl.sub(gn[2]).hi,
    };
    if si.width() == 0.0 {
        return Some(Some(nat));
    }
    let sc = Ival::pt(si.mid());
    let gc: [Ival; 3] = mv.at(sc);
    let gd: [D; 3] = mv.at_n(D::var(si, 2));
    let uc = match closed_t::<Ival>(col, sf, gc[0], gc[1])? {
        None => return Some(Some(nat)),
        Some(cl) => cl.sub(gc[2]),
    };
    let ud = match closed_t::<D>(col, sf, gd[0], gd[1])? {
        None => return Some(Some(nat)),
        Some(cl) => cl.sub(gd[2]),
    };
    let mv_hi = uc.add(ud.g[2].mul(si.sub(sc))).hi;
    Some(Some(if mv_hi < nat { mv_hi } else { nat }))
}

/// The untrimmed surface's CL at a tool position (x, y) given as numbers of any kind (points,
/// intervals, dual numbers): None, no closed form; Some(None), out of reach.
fn closed_t<T: NumS>(col: &Column, f: &Face, x: T, y: T) -> Option<Option<T>> {
    let eta = bowl_eta(f);
    let bowl = eta.is_some();
    let lift = Ival::pt(eta.unwrap_or(0.0)).scale(2.0).hi;
    match f.surf {
        Surf::Plane { o, u, v } => {
            let mut n = cross(u, v);
            if n[2].hi < 0.0 {
                n = [n[0].neg(), n[1].neg(), n[2].neg()];
            } else if !(n[2].lo > 0.0) {
                return None;
            }
            let m = plane_support(col, n).0;
            let dx = x.sub(T::c(o[0]));
            let dy = y.sub(T::c(o[1]));
            let k0 = n[0].div(n[2]);
            let k1 = n[1].div(n[2]);
            let zp = T::c(o[2]).sub(T::iv(k0).mul(dx)).sub(T::iv(k1).mul(dy));
            Some(Some(zp.sub(T::iv(m))))
        }
        Surf::Sphere { o, r, .. } => {
            let dx = x.sub(T::c(o[0]));
            let dy = y.sub(T::c(o[1]));
            let d = dx.sqr().add(dy.sqr()).sqrt();
            let v = if bowl {
                bowl_drop(col, r, d, -r, r)
            } else {
                disk_drop(&col.segs, r, d)
            };
            Some(v.map(|(v, _)| T::c(o[2]).add(v).add(T::c(lift))))
        }
        Surf::Cyl { o, ax, r, .. } => {
            if ax[2] != 0.0 {
                return None;
            }
            let len = Ival::pt(ax[0]).sqr().add(Ival::pt(ax[1]).sqr()).sqrt();
            let rx = x.sub(T::c(o[0]));
            let ry = y.sub(T::c(o[1]));
            let cr = rx.mul(T::c(ax[1])).sub(ry.mul(T::c(ax[0])));
            let v = if bowl {
                let (xa, xb) = cyl_lateral(&f.surf, f.p);
                bowl_drop(col, r, cr.div(T::iv(len)), xa, xb)
            } else {
                disk_drop(&col.segs, r, cr.sqr().sqrt().div(T::iv(len)))
            };
            Some(v.map(|(v, _)| T::c(o[2]).add(v).add(T::c(lift))))
        }
        Surf::Torus { o, big, r, .. } if level_torus(&f.surf) => {
            // in the vertical plane through both axes: the tube's circle, `big` out from the
            // torus's axis; each ring's nearest point to the tool is in that plane
            let dx = x.sub(T::c(o[0]));
            let dy = y.sub(T::c(o[1]));
            let d = dx.sqr().add(dy.sqr()).sqrt().sub(T::c(big));
            let v = if bowl {
                let (xa, xb) = tube_lateral(f);
                bowl_drop(col, r, d, xa, xb)
            } else {
                disk_drop(&col.segs, r, d.sqr().sqrt())
            };
            Some(v.map(|(v, _)| T::c(o[2]).add(v).add(T::c(lift))))
        }
        _ => None,
    }
}

fn box_gap(a: Ival, b: Ival) -> f64 {
    if a.hi < b.lo {
        Ival::pt(b.lo).sub(Ival::pt(a.hi)).lo
    } else if b.hi < a.lo {
        Ival::pt(a.lo).sub(Ival::pt(b.hi)).lo
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{Surf, TSeg, Trim};
    use crate::tool::{Cutter, Part};

    fn cutter(c: Cutter) -> Body {
        let sh = crate::tool::assemble(&c, 15.0, 2.0 * c.radius(), 30.0, None).unwrap();
        assert_eq!(sh.bodies[0].part, Part::Cutter);
        sh.bodies[0].clone()
    }

    fn plane_face(o: [f64; 3], u: [f64; 3], v: [f64; 3], w: f64, h: f64) -> Face {
        let trim = Trim {
            segs: vec![
                TSeg::Line {
                    a: [0.0, 0.0],
                    b: [w, 0.0],
                },
                TSeg::Line {
                    a: [w, 0.0],
                    b: [w, h],
                },
                TSeg::Line {
                    a: [w, h],
                    b: [0.0, h],
                },
                TSeg::Line {
                    a: [0.0, h],
                    b: [0.0, 0.0],
                },
            ],
        };
        Face::new(
            Surf::Plane { o, u, v },
            Ival::new(0.0, w),
            Ival::new(0.0, h),
            Some(trim),
            0.0,
            0,
        )
    }

    #[test]
    fn profiles() {
        let ball = Column::of(&cutter(Cutter::Ball { d: 6.0 })).unwrap();
        assert_eq!(ball.r, 3.0);
        let b = ball.b_at(3.0);
        assert!(b.lo <= 3.0 && b.hi >= 3.0);
        let b = ball.b_at(0.0);
        assert!(b.lo <= 0.0 && b.hi >= 0.0);
        let flat = Column::of(&cutter(Cutter::Flat { d: 6.0 })).unwrap();
        assert_eq!(flat.b_at(2.0), Ival::pt(0.0));
        let bull = Column::of(&cutter(Cutter::Bull { d: 10.0, re: 2.0 })).unwrap();
        let b = bull.b_at(5.0);
        assert!(b.lo <= 2.0 && b.hi >= 2.0);
        assert_eq!(bull.b_at(3.0).hi, 0.0);
    }

    #[test]
    fn ball_on_tilted_plane() {
        // a 30° slope rising along x: z = x tan 30°; ball r = 3: CL = x t + r (sec − 1)
        let t = 0.5773502691896257f64;
        let n = (1.0 + t * t).sqrt();
        let f = plane_face(
            [-50.0, -50.0, -50.0 * t],
            [1.0 / n, 0.0, t / n],
            [0.0, 1.0, 0.0],
            100.0 * n,
            100.0,
        );
        let col = Column::of(&cutter(Cutter::Ball { d: 6.0 })).unwrap();
        let cl = drop(&col, &[f], &[0], 2.0, 1.0, 1e-6, 100_000);
        let want = 2.0 * t + 3.0 * (n - 1.0);
        assert!(cl.lo <= want + 1e-12 && cl.hi >= want - 1e-12, "{cl:?} {want}");
        assert!(cl.hi - cl.lo <= 1e-6, "{cl:?}");
    }

    #[test]
    fn ball_on_sphere() {
        // sphere radius 10 at the origin, ball r = 3 at horizontal distance d: CL = √(13² − d²) − 3
        let f = Face::new(
            Surf::Sphere {
                o: [0.0, 0.0, 0.0],
                x: [1.0, 0.0, 0.0],
                y: [0.0, 1.0, 0.0],
                ax: [0.0, 0.0, 1.0],
                r: 10.0,
            },
            Ival::new(-1.0, 1.0),
            Ival::new(0.0, 1.0),
            None,
            0.0,
            0,
        );
        let col = Column::of(&cutter(Cutter::Ball { d: 6.0 })).unwrap();
        for d in [0.5f64, 3.0, 6.0] {
            let cl = drop(&col, std::slice::from_ref(&f), &[0], d, 0.0, 1e-5, 200_000);
            let want = (169.0 - d * d).sqrt() - 3.0;
            assert!(cl.lo <= want + 1e-9 && cl.hi >= want - 1e-9, "{d} {cl:?} {want}");
            assert!(cl.hi - cl.lo <= 1e-5, "{d} {cl:?}");
        }
    }

    #[test]
    fn flat_on_ridge_and_gap() {
        // two planes meeting at a ridge along y at x = 0, z = 0, each falling 1:1 away
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let l = plane_face(
            [-20.0, -20.0, -20.0],
            [s, 0.0, s],
            [0.0, 1.0, 0.0],
            20.0 / s,
            40.0,
        );
        let r = plane_face([0.0, -20.0, 0.0], [s, 0.0, -s], [0.0, 1.0, 0.0], 20.0 / s, 40.0);
        let faces = vec![l, r];
        let col = Column::of(&cutter(Cutter::Flat { d: 6.0 })).unwrap();
        // a flat tool over the ridge sits on it: CL = 0 for |x| ≤ 3, else 3 − |x|
        for x in [0.0f64, 1.5, 4.0, -5.0] {
            let cl = drop(&col, &faces, &[0, 1], x, 0.0, 1e-6, 200_000);
            let want = if x.abs() <= 3.0 { 0.0 } else { 3.0 - x.abs() };
            assert!(cl.lo <= want + 1e-9 && cl.hi >= want - 1e-9, "{x} {cl:?} {want}");
            assert!(cl.hi - cl.lo <= 1e-6, "{x} {cl:?}");
        }
        // a move along the ridge at CL is clear to the gap; 0.01 lower is not
        let along = Move::Line {
            a: [0.0, -10.0, 0.0],
            b: [0.0, 10.0, 0.0],
        };
        for f in &faces {
            let (g, _) = gap_move(&col, f, &along, -1e-3, 1e-5, 100_000);
            assert!(matches!(g, Gap::Clear { .. }), "{g:?}");
        }
        let low = Move::Line {
            a: [0.0, -10.0, -0.01],
            b: [0.0, 10.0, -0.01],
        };
        let (g, _) = gap_move(&col, &faces[0], &low, -1e-3, 1e-5, 100_000);
        assert!(matches!(g, Gap::Over { .. }), "{g:?}");
    }
}
