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
//! **The vertical gap of a move.** For a move g(s) and a face, v = min over s and y of
//! g(s).z + b(ρ) − y.z (over y in the column's footprint). v ≥ L with L ≤ 0 proves the signed
//! distance of every body inside the column to F is ≥ L (a point of F at depth d > −L inside a
//! body would sit with the ball of radius d about it inside the column, so d below it would still
//! be in the column: v ≤ −d < L). This is how a 3D finishing move is certified against curved
//! faces without the Euclidean branch and bound, whose tangent contacts are slow.
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
    Arc { a: [f64; 2], b: [f64; 2], c: [f64; 2], r: f64 },
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
}

/// √ and an interval constant on top of `Num`, for the profile's formulas.
pub trait NumS: Num {
    fn sqrt(self) -> Self;
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
        Some(Column { r: rho, segs })
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
                    (a, na, b, nb, CSeg::Arc { a: ga, b: gb, c, r: rr })
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
        Some(Column { r, segs: out })
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
            let k = Ival::pt(b[1]).sub(Ival::pt(a[1])).div(Ival::pt(b[0]).sub(Ival::pt(a[0])));
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
            let k = Ival::pt(b[1]).sub(Ival::pt(a[1])).div(Ival::pt(b[0]).sub(Ival::pt(a[0])));
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
    // natural enclosures and the monotone bound
    let y = f.surf.eval(c.p, c.q);
    let g = mv.at(c.s);
    let dx = y[0].sub(g[0]);
    let dy = y[1].sub(g[1]);
    let r2 = dx.sqr().add(dy.sqr());
    let rho = r2.sqrt();
    let bb = col.b(rho)?;
    let mut best = y[2].sub(bb).sub(g[2]).hi;
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
        if self.v.is_empty() { f64::NEG_INFINITY } else { self.v[0].0 }
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
pub fn drop_above(col: &Column, faces: &[Face], idx: &[u32], x: f64, y: f64, floor: f64, res: f64, budget: usize) -> Cl {
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
        if let Some((ub, at)) = closed(col, &f.surf, x, y) {
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
        if hi - lo <= res || cells >= budget || hi <= floor {
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
fn disk_drop<T: NumS>(col: &Column, sr: f64, d: T) -> Option<(T, f64)> {
    let s2 = Ival::pt(sr).sqr();
    let mut best: Option<T> = None;
    let mut arg = 0.0f64;
    let mut bestf = f64::NEG_INFINITY;
    let dv = d.val();
    let mut i = 0;
    while i < col.segs.len() {
        let sg = col.segs[i];
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
                    let k = Ival::pt(b[1]).sub(Ival::pt(a[1])).div(Ival::pt(b[0]).sub(Ival::pt(a[0])));
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
pub fn closed(col: &Column, s: &Surf, x: f64, y: f64) -> Option<(f64, Option<(f64, f64)>)> {
    match *s {
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
        Surf::Sphere { o, x: fx, y: fy, ax, r } => {
            let dx = Ival::pt(x).sub(Ival::pt(o[0]));
            let dy = Ival::pt(y).sub(Ival::pt(o[1]));
            let d = dx.sqr().add(dy.sqr()).sqrt();
            let Some((v, rho)) = disk_drop(col, r, d) else {
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
        Surf::Cyl { o, x: fx, y: fy, ax, r } => {
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
            let Some((v, rho)) = disk_drop(col, r, d) else {
                return Some((f64::NEG_INFINITY, None));
            };
            let rho = inside(col, rho);
            let ub = Ival::pt(o[2]).add(v).hi;
            // the contact: the foot on the axis, ρ toward it
            let t = (rx.mid() * ax[0] + ry.mid() * ax[1]) / (al * al);
            let foot = [o[0] + t * ax[0], o[1] + t * ax[1]];
            let (fxv, fyv) = (foot[0] - x, foot[1] - y);
            let dm = (fxv * fxv + fyv * fyv).sqrt();
            let (ux, uy) = if dm > 1e-12 { (fxv / dm, fyv / dm) } else { (1.0, 0.0) };
            let off = rho - dm;
            let h = (r * r - off * off).max(0.0).sqrt();
            let w = [x + rho * ux - foot[0], y + rho * uy - foot[1], h];
            // across the axis: (c, s) in the frame (x, y)
            let cx = (w[0] * fx[0] + w[1] * fx[1] + w[2] * fx[2]) / r;
            let cy = (w[0] * fy[0] + w[1] * fy[1] + w[2] * fy[2]) / r;
            if !(cx > -0.999) {
                return Some((ub, None));
            }
            Some((ub, Some((cy / (1.0 + cx), t))))
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
fn plane_drop(col: &Column, o: V3, u: V3, v: V3, n: [Ival; 3], x: f64, y: f64) -> Option<(f64, Option<(f64, f64)>)> {
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
                let g = Ival::pt(c[1]).sub(tau.mul(Ival::pt(c[0]))).sub(Ival::pt(r).mul(w));
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
    if p.is_finite() && q.is_finite() { Some((p, q)) } else { None }
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
    if p.is_finite() && q.is_finite() { Some((p, q)) } else { None }
}

/// What the vertical-gap check proved for a move against a face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gap {
    /// v ≥ lower ≥ limit, lower ≤ 0 (the face narrowed by mu).
    Clear { lower: f64 },
    /// A real face point at a real instant s sits more than −limit above the column's bottom
    /// (the face widened by mu): the column, not necessarily the cutter, meets the face.
    Over { s: f64, by: f64 },
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
            match closed_u(col, &f.surf, mv, si) {
                Some(None) => continue,
                Some(Some(u)) if u <= need => {
                    if u > worst {
                        worst = u;
                    }
                    continue;
                }
                _ => {}
            }
            // at the piece's middle: the contact, if it is in the face, is a witness or shows the
            // closed form is the face's own there (then split s); else the face's edge decides
            let sm = si.mid();
            let g = mv.at(Ival::pt(sm));
            let mut own = false;
            if let Some((_, Some((p, q)))) = closed(col, &f.surf, g[0].mid(), g[1].mid())
                && f.holds(Ival::pt(p), Ival::pt(q)) == 1
            {
                own = true;
                let at = Cell {
                    p: Ival::pt(p),
                    q: Ival::pt(q),
                    s: Ival::pt(sm),
                };
                let w = Ival::pt(cell_lower(col, f, mv, &at)).sub(Ival::pt(f.mu)).lo;
                if w > -limit {
                    return (Gap::Over { s: sm, by: w }, cells);
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
        if let Some((u, _)) = cell_upper(col, f, mv, &c) {
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
        let w = Ival::pt(cell_lower(col, f, mv, &at)).sub(Ival::pt(f.mu)).lo;
        if w > -limit {
            return (Gap::Over { s: sm, by: w }, cells);
        }
        let img = diam2(&f.surf.eval(c.p, c.q)).max(diam2(&mv.at(c.s)));
        if img <= res || (c.p.width() == 0.0 && c.q.width() == 0.0 && c.s.width() == 0.0) {
            unknown = true;
            continue;
        }
        let k = match cell_upper(col, f, mv, &c) {
            Some((_, k)) => k,
            None => continue,
        };
        let (a, b) = split(&c, k);
        for h in [a, b] {
            cells += 1;
            if f.holds(h.p, h.q) == -1 {
                continue;
            }
            if let Some((u2, _)) = cell_upper(col, f, mv, &h) {
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
    (Gap::Clear { lower: if v < 0.0 { v } else { 0.0 } }, cells)
}

fn has_closed(s: &Surf) -> bool {
    match *s {
        Surf::Plane { .. } | Surf::Sphere { .. } => true,
        Surf::Cyl { ax, .. } => ax[2] == 0.0,
        _ => false,
    }
}

/// An upper bound of u = CL(g(s)) − g(s).z over s ∈ si for the untrimmed surface (Some(None):
/// out of reach), by the mean-value form in s intersected with the natural one; None: no closed
/// form.
fn closed_u(col: &Column, sf: &Surf, mv: &Move, si: Ival) -> Option<Option<f64>> {
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
fn closed_t<T: NumS>(col: &Column, s: &Surf, x: T, y: T) -> Option<Option<T>> {
    match *s {
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
            Some(disk_drop(col, r, d).map(|(v, _)| T::c(o[2]).add(v)))
        }
        Surf::Cyl { o, ax, r, .. } => {
            if ax[2] != 0.0 {
                return None;
            }
            let len = Ival::pt(ax[0]).sqr().add(Ival::pt(ax[1]).sqr()).sqrt();
            let rx = x.sub(T::c(o[0]));
            let ry = y.sub(T::c(o[1]));
            let cr = rx.mul(T::c(ax[1])).sub(ry.mul(T::c(ax[0])));
            let d = cr.sqr().sqrt().div(T::iv(len));
            Some(disk_drop(col, r, d).map(|(v, _)| T::c(o[2]).add(v)))
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
                TSeg::Line { a: [0.0, 0.0], b: [w, 0.0] },
                TSeg::Line { a: [w, 0.0], b: [w, h] },
                TSeg::Line { a: [w, h], b: [0.0, h] },
                TSeg::Line { a: [0.0, h], b: [0.0, 0.0] },
            ],
        };
        Face::new(Surf::Plane { o, u, v }, Ival::new(0.0, w), Ival::new(0.0, h), Some(trim), 0.0, 0)
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
        let f = plane_face([-50.0, -50.0, -50.0 * t], [1.0 / n, 0.0, t / n], [0.0, 1.0, 0.0], 100.0 * n, 100.0);
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
            let cl = drop(&col, &[f.clone()], &[0], d, 0.0, 1e-5, 200_000);
            let want = (169.0 - d * d).sqrt() - 3.0;
            assert!(cl.lo <= want + 1e-9 && cl.hi >= want - 1e-9, "{d} {cl:?} {want}");
            assert!(cl.hi - cl.lo <= 1e-5, "{d} {cl:?}");
        }
    }

    #[test]
    fn flat_on_ridge_and_gap() {
        // two planes meeting at a ridge along y at x = 0, z = 0, each falling 1:1 away
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let l = plane_face([-20.0, -20.0, -20.0], [s, 0.0, s], [0.0, 1.0, 0.0], 20.0 / s, 40.0);
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
