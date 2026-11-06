//! Where two curves meet: points (crossing or touching) and overlaps, on the curves and, when
//! asked, on their extensions (apparent intersections).
//!
//! Lines, circles and ellipses meet in closed form: a line and a conic by a quadratic, two circles
//! directly, a conic and an ellipse by a quartic in the half-angle tangent, polished by Newton.
//! A spline or an offset curve meets an analytic curve where the analytic curve's implicit
//! equation along it changes sign (or touches zero), and another numeric curve by subdivision of
//! both into chords followed by Newton on the pair.

use super::curve::{Arc, Curve, Ellipse, Seg, scan_roots};
use super::geom::{Affine, Bbox, PI, TAU, Tol, V2, solve2, v2, wrap};
use forge_math::dmath;

/// A point where two curves meet: the parameters on each, whether the curves touch there
/// (tangent) rather than cross, and whether it is on each curve itself (false: on its extension).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub p: V2,
    pub ta: f64,
    pub tb: f64,
    pub tangent: bool,
    pub on_a: bool,
    pub on_b: bool,
}

impl Hit {
    pub fn on_both(&self) -> bool {
        self.on_a && self.on_b
    }
}

/// A stretch the curves share: parameter ranges on each (`a.0 < a.1`; `b` in the order matching
/// a's ends).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overlap {
    pub a: (f64, f64),
    pub b: (f64, f64),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hits {
    pub points: Vec<Hit>,
    pub overlaps: Vec<Overlap>,
}

/// Which curves to take past their ends: lines along their direction, arcs round their circle,
/// elliptical arcs round their ellipse, an open polyline's first and last segments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ext {
    None,
    A,
    B,
    Both,
}

impl Ext {
    fn a(self) -> bool {
        matches!(self, Ext::A | Ext::Both)
    }
    fn b(self) -> bool {
        matches!(self, Ext::B | Ext::Both)
    }
}

/// The geometry a piece lies on.
#[derive(Clone, Copy, Debug)]
enum Shape<'a> {
    /// p + u d.
    Lin { p: V2, d: V2 },
    /// Angle start + dir u.
    Circ { c: V2, r: f64, start: f64, dir: f64 },
    /// Eccentric angle start + dir u.
    Ell(Ellipse),
    /// A spline or offset curve over its own parameter.
    Num(&'a Curve),
}

/// A piece of a curve: its local parameter `u` runs over [lo, hi]; the curve's is off + k u.
#[derive(Clone, Copy, Debug)]
struct Piece<'a> {
    shape: Shape<'a>,
    lo: f64,
    hi: f64,
    off: f64,
    k: f64,
    ext_lo: bool,
    ext_hi: bool,
    /// A whole circle or ellipse: u wraps.
    full: bool,
}

fn circ_piece<'a>(a: &Arc, off: f64, k: f64, ext_lo: bool, ext_hi: bool) -> Piece<'a> {
    Piece {
        shape: Shape::Circ {
            c: a.c,
            r: a.r,
            start: a.start,
            dir: a.dir(),
        },
        lo: 0.0,
        hi: a.len(),
        off,
        k,
        ext_lo,
        ext_hi,
        full: false,
    }
}

fn pieces(c: &Curve, ext: bool) -> Vec<Piece<'_>> {
    let lin = |p: V2, d: V2, lo: f64, hi: f64, off: f64, el: bool, eh: bool| Piece {
        shape: Shape::Lin { p, d },
        lo,
        hi,
        off,
        k: 1.0,
        ext_lo: el,
        ext_hi: eh,
        full: false,
    };
    match c {
        Curve::Line { a, b } => vec![lin(*a, *b - *a, 0.0, 1.0, 0.0, ext, ext)],
        Curve::Ray { at, dir } => vec![lin(*at, *dir, 0.0, f64::INFINITY, 0.0, false, false)],
        Curve::Xline { at, dir } => vec![lin(
            *at,
            *dir,
            f64::NEG_INFINITY,
            f64::INFINITY,
            0.0,
            false,
            false,
        )],
        Curve::Circle { c, r } => vec![Piece {
            shape: Shape::Circ {
                c: *c,
                r: *r,
                start: 0.0,
                dir: 1.0,
            },
            lo: 0.0,
            hi: TAU,
            off: 0.0,
            k: 1.0,
            ext_lo: false,
            ext_hi: false,
            full: true,
        }],
        Curve::Arc(a) => vec![circ_piece(a, 0.0, 1.0, ext, ext)],
        Curve::Ellipse(e) => vec![Piece {
            shape: Shape::Ell(*e),
            lo: 0.0,
            hi: e.sweep.abs().min(TAU),
            off: 0.0,
            k: 1.0,
            ext_lo: ext && !e.is_full(),
            ext_hi: ext && !e.is_full(),
            full: e.is_full(),
        }],
        Curve::Poly(p) => {
            let n = p.nseg();
            let open_ext = ext && !p.closed;
            (0..n)
                .map(|k| {
                    let (el, eh) = (open_ext && k == 0, open_ext && k + 1 == n);
                    match p.seg(k) {
                        Seg::Line(a, b) => lin(a, b - a, 0.0, 1.0, k as f64, el, eh),
                        Seg::Arc(a) => circ_piece(&a, k as f64, 1.0 / a.len(), el, eh),
                    }
                })
                .collect()
        }
        Curve::Spline(_) | Curve::Offset(_) => {
            let (lo, hi) = c.domain();
            vec![Piece {
                shape: Shape::Num(c),
                lo,
                hi,
                off: 0.0,
                k: 1.0,
                ext_lo: false,
                ext_hi: false,
                full: c.is_closed(),
            }]
        }
    }
}

impl Piece<'_> {
    /// The local parameter of a point on the piece's carrier, mapped past the ends the way the
    /// extension runs.
    fn local(&self, p: V2) -> f64 {
        match self.shape {
            Shape::Lin { p: o, d } => (p - o).dot(d) / d.len2(),
            Shape::Circ { c, start, dir, .. } => self.wrap_u(wrap(dir * ((p - c).angle() - start))),
            Shape::Ell(e) => self.wrap_u(e.param_of_angle(e.angle_of(p))),
            Shape::Num(c) => c.param_of(p),
        }
    }
    fn wrap_u(&self, u: f64) -> f64 {
        if self.full || u <= self.hi {
            return u;
        }
        match (self.ext_lo, self.ext_hi) {
            (true, false) => u - TAU,
            (true, true) if u - self.hi > TAU - u => u - TAU,
            _ => u,
        }
    }
    /// How far a unit of u moves along the piece (for parameter tolerances).
    fn speed(&self) -> f64 {
        match self.shape {
            Shape::Lin { d, .. } => d.len(),
            Shape::Circ { r, .. } => r,
            Shape::Ell(e) => e.b().max(1e-300),
            Shape::Num(_) => 1.0,
        }
    }
    fn speed_at(&self, u: f64) -> f64 {
        match self.shape {
            Shape::Num(c) => c.deriv(u.clamp(self.lo, self.hi)).len().max(1e-300),
            _ => self.speed(),
        }
    }
    /// Whether u is on the piece (Some(true)), on its extension (Some(false)), or neither.
    fn classify(&self, u: f64, tol: f64) -> Option<bool> {
        let e = tol / self.speed_at(u);
        if u >= self.lo - e && u <= self.hi + e {
            Some(true)
        } else if (u < self.lo && self.ext_lo) || (u > self.hi && self.ext_hi) {
            Some(false)
        } else {
            None
        }
    }
    fn curve_t(&self, u: f64) -> f64 {
        let (lo, hi) = (self.lo, self.hi);
        // snap to the ends a whisker away
        let u = if (u - lo).abs() <= 1e-12 * (1.0 + hi.abs()) {
            lo
        } else if (u - hi).abs() <= 1e-12 * (1.0 + hi.abs()) {
            hi
        } else {
            u
        };
        let u = if !self.full { u } else { u.clamp(lo, hi) };
        self.off + self.k * u
    }
    fn tangent_at(&self, p: V2) -> V2 {
        match self.shape {
            Shape::Lin { d, .. } => d.unit_or_zero(),
            Shape::Circ { c, dir, .. } => (p - c).perp().unit_or_zero() * dir,
            Shape::Ell(e) => {
                let th = e.angle_of(p);
                let (s, co) = dmath::sin_cos(th);
                ((e.minor() * co - e.major * s) * e.dir()).unit_or_zero()
            }
            Shape::Num(c) => c.tangent(c.param_of(p)),
        }
    }
}

/// The full ellipse an ellipse or circle lies on, as centre and conjugate axes.
fn conic(s: &Shape) -> Option<(V2, V2, V2)> {
    match *s {
        Shape::Circ { c, r, .. } => Some((c, v2(r, 0.0), v2(0.0, r))),
        Shape::Ell(e) => Some((e.c, e.major, e.minor())),
        _ => None,
    }
}

/// Points where two carriers meet (lines infinite, conics whole); tangent flags; and, for numeric
/// curves, their own parameters.
struct Raw {
    p: V2,
    tangent: bool,
    ua: Option<f64>,
    ub: Option<f64>,
}

fn raw(p: V2, tangent: bool) -> Raw {
    Raw {
        p,
        tangent,
        ua: None,
        ub: None,
    }
}

fn line_circle(p: V2, d: V2, c: V2, r: f64, tol: f64) -> Vec<Raw> {
    let u = d.unit_or_zero();
    let s0 = (c - p).dot(u);
    let foot = p + u * s0;
    let h = foot.dist(c);
    if h > r + tol {
        return vec![];
    }
    if (h - r).abs() <= tol {
        return vec![raw(foot, true)];
    }
    let w = ((r - h) * (r + h)).max(0.0).sqrt();
    vec![raw(foot - u * w, false), raw(foot + u * w, false)]
}

fn circle_circle(c1: V2, r1: f64, c2: V2, r2: f64, tol: f64) -> Vec<Raw> {
    let dv = c2 - c1;
    let d = dv.len();
    if d <= tol {
        return vec![];
    }
    if d > r1 + r2 + tol || d < (r1 - r2).abs() - tol {
        return vec![];
    }
    let u = dv / d;
    // distance from c1 along the centre line
    let a = (d * d + r1 * r1 - r2 * r2) / (2.0 * d);
    let foot = c1 + u * a;
    if (d - (r1 + r2)).abs() <= tol || (d - (r1 - r2).abs()).abs() <= tol {
        // the touching point
        let s = if (d - (r1 + r2)).abs() <= tol || r1 >= r2 {
            1.0
        } else {
            -1.0
        };
        return vec![raw(c1 + u * (r1 * s), true)];
    }
    let h = (r1 * r1 - a * a).max(0.0).sqrt();
    vec![raw(foot - u.perp() * h, false), raw(foot + u.perp() * h, false)]
}

fn line_ellipse(p: V2, d: V2, (c, u, v): (V2, V2, V2), tol: f64) -> Vec<Raw> {
    // to the frame where the ellipse is the unit circle
    let m = Affine {
        m: [[u.x, v.x], [u.y, v.y]],
        t: c,
    };
    let Some(inv) = m.inverse() else { return vec![] };
    let b = u.len().min(v.len()).max(1e-300);
    let hits = line_circle(inv.apply(p), inv.vec(d), V2::ZERO, 1.0, tol / b);
    hits.into_iter().map(|h| raw(m.apply(h.p), h.tangent)).collect()
}

/// Ellipse (or circle) A and ellipse B by the quartic in B's half-angle tangent.
fn conic_conic(a: (V2, V2, V2), b: (V2, V2, V2), tol: f64) -> Result<Vec<Raw>, ()> {
    let ma = Affine {
        m: [[a.1.x, a.2.x], [a.1.y, a.2.y]],
        t: a.0,
    };
    let Some(inv) = ma.inverse() else {
        return Ok(vec![]);
    };
    let c = inv.apply(b.0);
    let (uu, vv) = (inv.vec(b.1), inv.vec(b.2));
    let f = |th: f64| {
        let (s, co) = dmath::sin_cos(th);
        let q = c + uu * co + vv * s;
        let dq = vv * co - uu * s;
        (q.len2() - 1.0, 2.0 * q.dot(dq))
    };
    let c0 = c.len2() - 1.0;
    let (ul, vl, cu, cv, uv) = (uu.len2(), vv.len2(), c.dot(uu), c.dot(vv), uu.dot(vv));
    let coef = [
        c0 + ul + 2.0 * cu,
        4.0 * cv + 4.0 * uv,
        2.0 * c0 - 2.0 * ul + 4.0 * vl,
        4.0 * cv - 4.0 * uv,
        c0 + ul - 2.0 * cu,
    ];
    let scale = 1.0 + c.len2() + ul + vl;
    if coef.iter().all(|x| x.abs() <= 1e-12 * scale) {
        // the same ellipse
        return Err(());
    }
    let mut ths: Vec<(f64, bool)> = forge_math::poly::all_real_roots(&coef)
        .iter()
        .map(|r| (2.0 * dmath::atan(r.x), r.touching))
        .collect();
    if f(PI).0.abs() <= 1e-9 * scale {
        ths.push((PI, false));
    }
    let bmin = a.1.len().min(a.2.len()).max(1e-300);
    let mut out: Vec<Raw> = vec![];
    for (th0, touching) in ths {
        let mut th = th0;
        for _ in 0..20 {
            let (v, dv) = f(th);
            if dv.abs() < 1e-300 {
                break;
            }
            let step = (v / dv).clamp(-0.1, 0.1);
            th -= step;
            if step.abs() < 1e-16 {
                break;
            }
        }
        // a double root polishes poorly: keep the better of the two
        if f(th0).0.abs() < f(th).0.abs() {
            th = th0;
        }
        let (s, co) = dmath::sin_cos(th);
        let q = c + uu * co + vv * s;
        if (q.len() - 1.0).abs() * bmin > tol * 1e3 {
            continue;
        }
        let p = b.0 + b.1 * co + b.2 * s;
        // touching: the tangents agree
        let tb = (b.2 * co - b.1 * s).unit_or_zero();
        let qa = inv.apply(p);
        let ta = ma.vec(qa.perp()).unit_or_zero();
        let tangent = touching || ta.cross(tb).abs() < 1e-7;
        out.push(raw(p, tangent));
    }
    Ok(out)
}

/// The implicit function of an analytic carrier: zero on it, about the signed distance near it.
fn implicit<'a>(s: &Shape<'a>) -> Box<dyn Fn(V2) -> f64 + 'a> {
    match *s {
        Shape::Lin { p, d } => {
            let u = d.unit_or_zero();
            Box::new(move |q| (q - p).cross(u))
        }
        Shape::Circ { c, r, .. } => Box::new(move |q| q.dist(c) - r),
        Shape::Ell(e) => {
            let m = Affine {
                m: [[e.major.x, e.minor().x], [e.major.y, e.minor().y]],
                t: e.c,
            };
            let inv = m.inverse().unwrap_or(Affine::IDENTITY);
            let b = e.b();
            Box::new(move |q| (inv.apply(q).len() - 1.0) * b)
        }
        Shape::Num(_) => Box::new(|_| 0.0),
    }
}

fn numeric_analytic(n: &Curve, other: &Shape, tol: f64) -> Vec<Raw> {
    let f = implicit(other);
    let g = |t: f64| f(n.point(t));
    let ts = n.samples();
    let mut out = vec![];
    for t in scan_roots(&g, &ts, 1e-15) {
        let p = n.point(t);
        if g(t).abs() > tol * 10.0 {
            continue;
        }
        // tangent when the curve runs along the carrier: |dg/dt| small for its speed
        let h = 1e-7 * (ts[ts.len() - 1] - ts[0]).abs().max(1e-12);
        let dg = (g(t + h) - g(t - h)) / (2.0 * h);
        let sp = n.deriv(t).len().max(1e-300);
        let tangent = (dg / sp).abs() < 1e-6;
        out.push(Raw {
            p,
            tangent,
            ua: Some(t),
            ub: None,
        });
    }
    out
}

/// Chords of a numeric curve between densified samples, with how far the curve strays from them.
fn chords(c: &Curve) -> (Vec<(f64, V2)>, f64) {
    let ts = c.samples();
    let mut pts = vec![];
    for w in ts.windows(2) {
        for k in 0..4 {
            let t = w[0] + (w[1] - w[0]) * k as f64 / 4.0;
            pts.push((t, c.point(t)));
        }
    }
    let last = *ts.last().unwrap_or(&0.0);
    pts.push((last, c.point(last)));
    let mut dev = 0.0f64;
    for w in pts.windows(2) {
        let tm = 0.5 * (w[0].0 + w[1].0);
        let pm = c.point(tm);
        dev = dev.max(pm.dist(w[0].1.mid(w[1].1)));
    }
    (pts, dev)
}

/// Newton on a(s) = b(t) from (s, t).
fn newton2(a: &Curve, b: &Curve, mut s: f64, mut t: f64) -> Option<(f64, f64)> {
    let (alo, ahi) = a.domain();
    let (blo, bhi) = b.domain();
    for _ in 0..50 {
        let r = a.point(s) - b.point(t);
        let (da, db) = (a.deriv(s), b.deriv(t));
        let (ds, dt) = solve2(da.x, -db.x, da.y, -db.y, -r.x, -r.y, 1e-14)?;
        let (ns, nt) = ((s + ds).clamp(alo, ahi), (t + dt).clamp(blo, bhi));
        let done = (ns - s).abs() <= 1e-15 * (1.0 + s.abs()) && (nt - t).abs() <= 1e-15 * (1.0 + t.abs());
        s = ns;
        t = nt;
        if done {
            break;
        }
    }
    Some((s, t))
}

fn numeric_numeric(a: &Curve, b: &Curve, tol: f64) -> Vec<Raw> {
    let (pa, da) = chords(a);
    let (pb, db) = chords(b);
    let grow = da + db + tol;
    let mut out: Vec<Raw> = vec![];
    // a bound on the work where the curves run together (partly coincident)
    let mut budget = 4 * (pa.len() + pb.len());
    for i in 0..pa.len().saturating_sub(1) {
        let ba = Bbox::of([pa[i].1, pa[i + 1].1]).grow(grow);
        for j in 0..pb.len().saturating_sub(1) {
            let bb = Bbox::of([pb[j].1, pb[j + 1].1]);
            if !ba.overlaps(&bb) {
                continue;
            }
            if budget == 0 {
                return out;
            }
            budget -= 1;
            // start at the chords' crossing (or their middles)
            let (s0, t0) =
                match super::curve::line_line(pa[i].1, pa[i + 1].1 - pa[i].1, pb[j].1, pb[j + 1].1 - pb[j].1)
                {
                    Some((x, y)) if (-0.5..=1.5).contains(&x) && (-0.5..=1.5).contains(&y) => {
                        (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))
                    }
                    _ => (0.5, 0.5),
                };
            let s = pa[i].0 + (pa[i + 1].0 - pa[i].0) * s0;
            let t = pb[j].0 + (pb[j + 1].0 - pb[j].0) * t0;
            let Some((s, t)) = newton2(a, b, s, t).or_else(|| touch_min(a, b, s, t)) else {
                continue;
            };
            let (p, q) = (a.point(s), b.point(t));
            if p.dist(q) > tol * 10.0 {
                // a near tangency where Newton stalls: minimise the distance instead
                if let Some((s2, t2)) = touch_min(a, b, s, t)
                    && a.point(s2).dist(b.point(t2)) <= tol * 10.0
                {
                    push_num(&mut out, a, b, s2, t2, tol);
                }
                continue;
            }
            push_num(&mut out, a, b, s, t, tol);
        }
    }
    out
}

fn push_num(out: &mut Vec<Raw>, a: &Curve, b: &Curve, s: f64, t: f64, tol: f64) {
    let p = a.point(s);
    if out.iter().any(|r| r.p.dist(p) <= tol * 10.0) {
        return;
    }
    let tangent = a.tangent(s).cross(b.tangent(t)).abs() < 1e-6;
    out.push(Raw {
        p,
        tangent,
        ua: Some(s),
        ub: Some(t),
    });
}

/// Alternating closest points from (s, t): for touching curves.
fn touch_min(a: &Curve, b: &Curve, mut s: f64, mut t: f64) -> Option<(f64, f64)> {
    for _ in 0..30 {
        t = b.closest(a.point(s)).0;
        s = a.closest(b.point(t)).0;
    }
    Some((s, t))
}

fn shape_hits(a: &Piece, b: &Piece, tol: f64) -> Result<Vec<Raw>, ()> {
    use Shape::*;
    Ok(match (&a.shape, &b.shape) {
        (Lin { p, d }, Lin { p: q, d: e }) => {
            let cr = d.unit_or_zero().cross(e.unit_or_zero());
            if cr.abs() <= 1e-13 {
                // parallel: the same line overlaps
                let h = (*q - *p).cross(d.unit_or_zero()).abs();
                return if h <= tol { Err(()) } else { Ok(vec![]) };
            }
            match super::curve::line_line(*p, *d, *q, *e) {
                Some((s, _)) => vec![raw(*p + *d * s, false)],
                None => vec![],
            }
        }
        (Lin { p, d }, Circ { c, r, .. }) | (Circ { c, r, .. }, Lin { p, d }) => {
            line_circle(*p, *d, *c, *r, tol)
        }
        (Circ { c, r, .. }, Circ { c: c2, r: r2, .. }) => {
            if c.dist(*c2) <= tol && (r - r2).abs() <= tol {
                return Err(());
            }
            circle_circle(*c, *r, *c2, *r2, tol)
        }
        (Lin { p, d }, Ell(_)) => line_ellipse(*p, *d, conic(&b.shape).ok_or(())?, tol),
        (Ell(_), Lin { p, d }) => line_ellipse(*p, *d, conic(&a.shape).ok_or(())?, tol),
        (Circ { .. } | Ell(_), Circ { .. } | Ell(_)) => {
            conic_conic(conic(&a.shape).ok_or(())?, conic(&b.shape).ok_or(())?, tol)?
        }
        (Num(n), Num(m)) => numeric_numeric(n, m, tol),
        (Num(n), _) => numeric_analytic(n, &b.shape, tol),
        (_, Num(m)) => numeric_analytic(m, &a.shape, tol)
            .into_iter()
            .map(|r| Raw {
                ub: r.ua,
                ua: None,
                ..r
            })
            .collect(),
    })
}

/// The overlap of two pieces on the same carrier, in local parameters (on the pieces only).
fn piece_overlap(a: &Piece, b: &Piece, tol: f64) -> Option<((f64, f64), (f64, f64))> {
    match (a.shape, b.shape) {
        (Shape::Lin { .. }, Shape::Lin { .. }) => {
            let pb0 = point_of(b, b.lo.max(-1e300));
            let pb1 = point_of(b, b.hi.min(1e300));
            let (u0, u1) = (a.local(pb0), a.local(pb1));
            let (lo, hi) = (u0.min(u1).max(a.lo), u0.max(u1).min(a.hi));
            let e = tol / a.speed();
            if hi - lo <= e {
                return None;
            }
            let (v0, v1) = (b.local(point_of(a, lo)), b.local(point_of(a, hi)));
            Some(((lo, hi), (v0, v1)))
        }
        _ if a.full || b.full || conic(&a.shape).is_some() => {
            // angular stretches on the same circle or ellipse, in a's angle from its start
            let bs = point_of(b, b.lo);
            let ta = a.tangent_at(bs);
            let tb = b.tangent_at(bs);
            let same = ta.dot(tb) >= 0.0;
            let blen = b.hi - b.lo;
            let start_u = wrap(a.local_angle(bs));
            let (bu0, bu1) = if same {
                (start_u, start_u + blen)
            } else {
                (start_u - blen, start_u)
            };
            let alen = a.hi - a.lo;
            let e = tol / a.speed();
            let mut best: Option<(f64, f64)> = None;
            for shift in [-TAU, 0.0, TAU] {
                let (lo, hi) = ((bu0 + shift).max(0.0), (bu1 + shift).min(alen));
                if hi - lo > e && best.is_none_or(|(l, h)| hi - lo > h - l) {
                    best = Some((lo, hi));
                }
            }
            let (lo, hi) = best?;
            let (v0, v1) = (b.local_plain(point_of(a, lo)), b.local_plain(point_of(a, hi)));
            Some(((lo, hi), (v0, v1)))
        }
        _ => None,
    }
}

impl Piece<'_> {
    /// The angle travelled from the piece's start to a point, in [0, 2pi).
    fn local_angle(&self, p: V2) -> f64 {
        match self.shape {
            Shape::Circ { c, start, dir, .. } => wrap(dir * ((p - c).angle() - start)),
            Shape::Ell(e) => e.param_of_angle(e.angle_of(p)),
            _ => self.local(p),
        }
    }
    /// The local parameter of a point on the piece, clamped into it.
    fn local_plain(&self, p: V2) -> f64 {
        let u = self.local_angle(p);
        if u > self.hi {
            if u - self.hi < TAU - u { self.hi } else { self.lo }
        } else {
            u
        }
    }
}

fn point_of(p: &Piece, u: f64) -> V2 {
    match p.shape {
        Shape::Lin { p: o, d } => o + d * u,
        Shape::Circ { c, r, start, dir } => c + V2::polar(start + dir * u, r),
        Shape::Ell(e) => e.point(u),
        Shape::Num(c) => c.point(u),
    }
}

/// Where `a` and `b` meet. With `ext`, also where their extensions do (those hits have `on_a` or
/// `on_b` false). Sorted by `ta`.
pub fn intersect(a: &Curve, b: &Curve, ext: Ext) -> Hits {
    let tol = Tol::of_box(&a.rough_bbox().union(b.rough_bbox())).lin;
    intersect_tol(a, b, ext, tol)
}

/// `intersect` with an explicit length tolerance.
pub fn intersect_tol(a: &Curve, b: &Curve, ext: Ext, tol: f64) -> Hits {
    if a == b && matches!(a, Curve::Spline(_) | Curve::Offset(_)) {
        // the same numeric curve: all of it is shared
        let (lo, hi) = a.domain();
        return Hits {
            points: vec![],
            overlaps: vec![Overlap {
                a: (lo, hi),
                b: (lo, hi),
            }],
        };
    }
    let pa = pieces(a, ext.a());
    let pb = pieces(b, ext.b());
    let mut out = Hits::default();
    let (alo, ahi) = a.domain();
    let (blo, bhi) = b.domain();
    for x in &pa {
        for y in &pb {
            let raws = match shape_hits(x, y, tol) {
                Ok(r) => r,
                Err(()) => {
                    if let Some(((u0, u1), (v0, v1))) = piece_overlap(x, y, tol) {
                        out.overlaps.push(Overlap {
                            a: (x.curve_t(u0), x.curve_t(u1)),
                            b: (y.curve_t(v0), y.curve_t(v1)),
                        });
                    }
                    continue;
                }
            };
            for r in raws {
                let ua = r.ua.unwrap_or_else(|| x.local(r.p));
                let ub = r.ub.unwrap_or_else(|| y.local(r.p));
                let (Some(on_a), Some(on_b)) = (x.classify(ua, tol), y.classify(ub, tol)) else {
                    continue;
                };
                let mut ta = x.curve_t(ua);
                let mut tb = y.curve_t(ub);
                if on_a {
                    ta = ta.clamp(alo, ahi);
                }
                if on_b {
                    tb = tb.clamp(blo, bhi);
                }
                if a.is_closed() && ta >= ahi {
                    ta = alo;
                }
                if b.is_closed() && tb >= bhi {
                    tb = blo;
                }
                let h = Hit {
                    p: r.p,
                    ta,
                    tb,
                    tangent: r.tangent,
                    on_a,
                    on_b,
                };
                // a polyline vertex shows up on both its segments
                if let Some(k) = out.points.iter().position(|o| {
                    o.p.dist(h.p) <= tol * 10.0
                        && (o.on_a, o.on_b) == (h.on_a, h.on_b)
                        && near_t(o.ta, h.ta, a)
                        && near_t(o.tb, h.tb, b)
                }) {
                    out.points[k].tangent |= h.tangent;
                    continue;
                }
                out.points.push(h);
            }
        }
    }
    // hits inside an overlap are its ends
    out.points.retain(|h| {
        !out.overlaps
            .iter()
            .any(|o| h.ta > o.a.0 && h.ta < o.a.1 && h.on_a)
    });
    out.points
        .sort_by(|x, y| x.ta.total_cmp(&y.ta).then(x.tb.total_cmp(&y.tb)));
    out
}

fn near_t(x: f64, y: f64, c: &Curve) -> bool {
    let (lo, hi) = c.domain();
    let span = if lo.is_finite() && hi.is_finite() {
        hi - lo
    } else {
        1.0
    };
    let e = 1e-7 * span.max(1e-300);
    (x - y).abs() <= e || (c.is_closed() && ((x - y).abs() - span).abs() <= e)
}

/// The points where curves meet, for many curves at once (each pair once, on the curves).
pub fn intersect_all(curves: &[Curve]) -> Vec<(usize, usize, Hit)> {
    let boxes: Vec<Bbox> = curves.iter().map(|c| c.rough_bbox()).collect();
    let mut out = vec![];
    for i in 0..curves.len() {
        for j in i + 1..curves.len() {
            let tol = Tol::of_box(&boxes[i].union(boxes[j])).lin;
            if !boxes[i].grow(tol).overlaps(&boxes[j]) {
                continue;
            }
            for h in intersect_tol(&curves[i], &curves[j], Ext::None, tol).points {
                out.push((i, j, h));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::curve::{Ellipse, Offset};
    use crate::draft::nurbs::Nurbs;

    fn close(a: V2, b: V2) -> bool {
        a.dist(b) < 1e-9
    }

    #[test]
    fn lines() {
        let a = Curve::line(v2(0.0, 0.0), v2(10.0, 0.0));
        let b = Curve::line(v2(5.0, -5.0), v2(5.0, 5.0));
        let h = intersect(&a, &b, Ext::None);
        assert_eq!(h.points.len(), 1);
        assert!(close(h.points[0].p, v2(5.0, 0.0)));
        assert!((h.points[0].ta - 0.5).abs() < 1e-12 && (h.points[0].tb - 0.5).abs() < 1e-12);
        let c = Curve::line(v2(12.0, -1.0), v2(12.0, 1.0));
        assert!(intersect(&a, &c, Ext::None).points.is_empty());
        let h = intersect(&a, &c, Ext::A);
        assert_eq!(h.points.len(), 1);
        assert!(!h.points[0].on_a && h.points[0].on_b && (h.points[0].ta - 1.2).abs() < 1e-12);
        // collinear
        let d = Curve::line(v2(8.0, 0.0), v2(15.0, 0.0));
        let h = intersect(&a, &d, Ext::None);
        assert_eq!(h.overlaps.len(), 1);
        let o = h.overlaps[0];
        assert!((o.a.0 - 0.8).abs() < 1e-12 && (o.a.1 - 1.0).abs() < 1e-12);
        assert!(o.b.0.abs() < 1e-12 && (o.b.1 - 2.0 / 7.0).abs() < 1e-12);
    }

    #[test]
    fn line_circle_arcs() {
        let c = Curve::circle(v2(0.0, 0.0), 5.0);
        let l = Curve::line(v2(-10.0, 3.0), v2(10.0, 3.0));
        let h = intersect(&l, &c, Ext::None);
        assert_eq!(h.points.len(), 2);
        assert!(close(h.points[0].p, v2(-4.0, 3.0)) && close(h.points[1].p, v2(4.0, 3.0)));
        // tangent
        let t = Curve::line(v2(-10.0, 5.0), v2(10.0, 5.0));
        let h = intersect(&t, &c, Ext::None);
        assert_eq!(h.points.len(), 1);
        assert!(h.points[0].tangent && close(h.points[0].p, v2(0.0, 5.0)));
        // an arc covering only the right half: one hit, the other apparent
        let a = Curve::arc(V2::ZERO, 5.0, -PI / 2.0, PI / 2.0);
        let h = intersect(&l, &a, Ext::None);
        assert_eq!(h.points.len(), 1);
        let h = intersect(&l, &a, Ext::B);
        assert_eq!(h.points.len(), 2);
        assert!(h.points.iter().any(|x| !x.on_b));
    }

    #[test]
    fn circles() {
        let a = Curve::circle(v2(0.0, 0.0), 5.0);
        let b = Curve::circle(v2(8.0, 0.0), 5.0);
        let h = intersect(&a, &b, Ext::None);
        assert_eq!(h.points.len(), 2);
        assert!(
            h.points
                .iter()
                .all(|x| (x.p.x - 4.0).abs() < 1e-12 && (x.p.y.abs() - 3.0).abs() < 1e-12)
        );
        let c = Curve::circle(v2(10.0, 0.0), 5.0);
        let h = intersect(&a, &c, Ext::None);
        assert!(h.points.len() == 1 && h.points[0].tangent);
        let inner = Curve::circle(v2(2.0, 0.0), 3.0);
        let h = intersect(&a, &inner, Ext::None);
        assert!(h.points.len() == 1 && h.points[0].tangent && close(h.points[0].p, v2(5.0, 0.0)));
        // coincident arcs overlap
        let x = Curve::arc(V2::ZERO, 5.0, 0.0, PI);
        let y = Curve::arc(V2::ZERO, 5.0, PI / 2.0, 3.0 * PI / 2.0);
        let h = intersect(&x, &y, Ext::None);
        assert_eq!(h.overlaps.len(), 1, "{h:?}");
        let o = h.overlaps[0];
        assert!((o.a.0 - PI / 2.0).abs() < 1e-12 && (o.a.1 - PI).abs() < 1e-12);
        assert!(o.b.0.abs() < 1e-12 && (o.b.1 - PI / 2.0).abs() < 1e-12);
    }

    #[test]
    fn ellipses() {
        let e = Curve::Ellipse(Ellipse {
            c: V2::ZERO,
            major: v2(4.0, 0.0),
            ratio: 0.5,
            start: 0.0,
            sweep: TAU,
        });
        let l = Curve::line(v2(-10.0, 1.0), v2(10.0, 1.0));
        let h = intersect(&l, &e, Ext::None);
        assert_eq!(h.points.len(), 2);
        let x = 4.0 * (0.75f64).sqrt();
        assert!(close(h.points[0].p, v2(-x, 1.0)) && close(h.points[1].p, v2(x, 1.0)));
        let c = Curve::circle(V2::ZERO, 3.0);
        let h = intersect(&c, &e, Ext::None);
        assert_eq!(h.points.len(), 4);
        for p in &h.points {
            assert!((p.p.len() - 3.0).abs() < 1e-9);
            assert!((p.p.x * p.p.x / 16.0 + p.p.y * p.p.y / 4.0 - 1.0).abs() < 1e-9);
        }
        // two ellipses crossing at right angles, and touching ones
        let f = Curve::Ellipse(Ellipse {
            c: V2::ZERO,
            major: v2(0.0, 4.0),
            ratio: 0.5,
            start: 0.0,
            sweep: TAU,
        });
        assert_eq!(intersect(&e, &f, Ext::None).points.len(), 4);
        let g = Curve::Ellipse(Ellipse {
            c: v2(8.0, 0.0),
            major: v2(4.0, 0.0),
            ratio: 0.5,
            start: 0.0,
            sweep: TAU,
        });
        let h = intersect(&e, &g, Ext::None);
        assert_eq!(h.points.len(), 1, "{h:?}");
        assert!(h.points[0].tangent && close(h.points[0].p, v2(4.0, 0.0)));
        // the same ellipse
        assert_eq!(intersect(&e, &e.clone(), Ext::None).overlaps.len(), 1);
    }

    #[test]
    fn polylines_and_splines() {
        let p = Curve::poly(
            &[v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0)],
            &[0.0, 0.0, 0.0],
            false,
        );
        let l = Curve::line(v2(2.0, -2.0), v2(14.0, 6.0));
        let h = intersect(&p, &l, Ext::None);
        assert_eq!(h.points.len(), 2);
        assert!((h.points[0].ta - 0.5).abs() < 1e-12 && (h.points[1].ta - 4.0 / 3.0).abs() < 1e-12);
        // through the vertex: once
        let v = Curve::line(v2(5.0, 5.0), v2(15.0, -5.0));
        assert_eq!(intersect(&p, &v, Ext::None).points.len(), 1);
        let n = Nurbs {
            degree: 3,
            knots: vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
            ctrl: vec![v2(0.0, -5.0), v2(3.0, 10.0), v2(6.0, -10.0), v2(9.0, 5.0)],
            weights: None,
            closed: false,
        };
        let s = Curve::Spline(n);
        let x = Curve::line(v2(-1.0, 0.0), v2(10.0, 0.0));
        let h = intersect(&s, &x, Ext::None);
        assert_eq!(h.points.len(), 3);
        for hit in &h.points {
            assert!(hit.p.y.abs() < 1e-9 && s.point(hit.ta).dist(hit.p) < 1e-9);
        }
        let c = Curve::circle(v2(4.5, 0.0), 2.0);
        for hit in intersect(&s, &c, Ext::None).points {
            assert!((hit.p.dist(v2(4.5, 0.0)) - 2.0).abs() < 1e-9);
        }
        // spline against spline
        let s2 = s.transformed(&Affine::rotate(v2(4.5, 0.0), PI / 2.0)).unwrap();
        let h = intersect(&s, &s2, Ext::None);
        assert!(!h.points.is_empty());
        for hit in &h.points {
            assert!(s.point(hit.ta).dist(s2.point(hit.tb)) < 1e-9);
        }
        // an offset curve against a line
        let o = Curve::Offset(Offset {
            base: Box::new(Curve::Ellipse(Ellipse {
                c: V2::ZERO,
                major: v2(4.0, 0.0),
                ratio: 0.5,
                start: 0.0,
                sweep: TAU,
            })),
            d: 0.5,
        });
        let h = intersect(&o, &Curve::line(v2(-10.0, 0.0), v2(10.0, 0.0)), Ext::None);
        assert_eq!(h.points.len(), 2);
        assert!(h.points.iter().all(|x| (x.p.x.abs() - 3.5).abs() < 1e-9));
    }
}
