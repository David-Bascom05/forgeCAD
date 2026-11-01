//! B-splines for what no STEP entity states exactly: cubic Hermite pieces joined C1, so each knot
//! has multiplicity 2 and a piece's error falls with its width to the fourth.

use crate::write::FaceMask;
use forge_geom::surface::SurfDeriv;
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Box2, Interval, Point2, Point3, Vec3};

/// How far a fitted curve or surface may stray from the exact one, in mm.
pub const DEFAULT_TOL: f64 = 1e-4;

/// Most spans a curve grows to, and a surface's grid cells; past that the error is what it is,
/// and stated.
const MAX_SPANS: usize = 8192;
const MAX_CELLS: usize = 1 << 16;

/// A curve as a clamped cubic B-spline.
pub struct CurveFit {
    pub ctrl: Vec<Point3>,
    pub knots: Vec<f64>,
    pub mults: Vec<u32>,
    pub error: f64,
}

/// A surface over a parameter box as a clamped bicubic B-spline.
pub struct SurfaceFit {
    /// By rows of constant u.
    pub ctrl: Vec<Vec<Point3>>,
    /// Whether the net closes on itself in u and in v (a seam on the face).
    pub closed: (bool, bool),
    pub u: (Vec<f64>, Vec<u32>),
    pub v: (Vec<f64>, Vec<u32>),
    pub error: f64,
}

fn mults(n: usize) -> Vec<u32> {
    let mut m = vec![2; n];
    m[0] = 4;
    m[n - 1] = 4;
    m
}

fn hermite(p0: Point3, d0: Vec3, p1: Point3, d1: Vec3, h: f64, s: f64) -> Point3 {
    let (s2, s3) = (s * s, s * s * s);
    let (a, b, c, d) = (
        2.0 * s3 - 3.0 * s2 + 1.0,
        s3 - 2.0 * s2 + s,
        -2.0 * s3 + 3.0 * s2,
        s3 - s2,
    );
    Point3::ORIGIN + (p0 - Point3::ORIGIN) * a + d0 * (b * h) + (p1 - Point3::ORIGIN) * c + d1 * (d * h)
}

/// A curve's place on a surface, as a clamped cubic B-spline in the surface's parameters.
pub struct PcurveFit {
    pub ctrl: Vec<Point2>,
    pub knots: Vec<f64>,
    pub mults: Vec<u32>,
}

/// `c` on `s` through `anchors`, its parameter and where the face's loop drew it, in order (they
/// fix the period and which turn of a thread), the curve's own parameter kept: a reader then has
/// the edge on the surface and need not search for it.
pub fn fit_pcurve(c: &Curve, anchors: &[(f64, Point2)], s: &Surface, tol: f64) -> PcurveFit {
    let dom = s.domain();
    let near = |uv: Point2, by: Point2| {
        let fix = |x: f64, r: f64, p: Option<f64>| p.map_or(x, |p| x + p * ((r - x) / p).round());
        Point2::new(fix(uv.x, by.x, dom.u.period), fix(uv.y, by.y, dom.v.period))
    };
    // the parameters' rate along the curve, from its tangent in the surface's tangent plane
    let rate = |t: f64, uv: Point2| -> (f64, f64) {
        let (d, q) = (c.deriv(t), s.deriv(uv));
        let (e, f, g) = (q.du.dot(q.du), q.du.dot(q.dv), q.dv.dot(q.dv));
        let det = e * g - f * f;
        if det <= 1e-14 * e * g || det == 0.0 {
            return (0.0, 0.0);
        }
        let (a, b) = (d.d1.dot(q.du), d.d1.dot(q.dv));
        ((g * a - f * b) / det, (e * b - f * a) / det)
    };
    let locate = |t: f64, by: Point2| near(s.project_near(c.eval(t), by), by);
    let closed = match (c, s) {
        (Curve::Procedural(pc), Surface::Procedural(ps)) => ps.0.curve_uv(pc),
        _ => None,
    };
    let mut pts: Vec<(f64, Point2)> = anchors.to_vec();
    if let Some((base, dir)) = closed {
        for p in &mut pts {
            p.1 = base + dir * p.0;
        }
    }
    // the drawing's samples are many where few spans do: refinement adds back what is needed
    if pts.len() > 9 {
        let step = (pts.len() - 1) / 8;
        let last = *pts.last().unwrap();
        pts = pts.iter().step_by(step.max(1)).copied().collect();
        if pts.last() != Some(&last) {
            pts.push(last);
        }
    }
    let herm2 = |a: (f64, Point2), b: (f64, Point2), x: f64| -> Point2 {
        let h = b.0 - a.0;
        let (ra, rb) = (rate(a.0, a.1), rate(b.0, b.1));
        let (s2, s3) = (x * x, x * x * x);
        let (c0, c1, c2, c3) = (
            2.0 * s3 - 3.0 * s2 + 1.0,
            s3 - 2.0 * s2 + x,
            -2.0 * s3 + 3.0 * s2,
            s3 - s2,
        );
        Point2::new(
            c0 * a.1.x + c1 * h * ra.0 + c2 * b.1.x + c3 * h * rb.0,
            c0 * a.1.y + c1 * h * ra.1 + c2 * b.1.y + c3 * h * rb.1,
        )
    };
    for _ in 0..20 {
        let mut next = Vec::with_capacity(pts.len() * 2);
        for k in 0..pts.len() - 1 {
            let (a, b) = (pts[k], pts[k + 1]);
            next.push(a);
            let tm = 0.5 * (a.0 + b.0);
            let m = herm2(a, b, 0.5);
            let miss = s.eval(m).distance(c.eval(tm));
            if miss > tol && pts.len() < MAX_SPANS {
                next.push((tm, locate(tm, a.1)));
            }
        }
        next.push(*pts.last().unwrap());
        let same = next.len() == pts.len();
        pts = next;
        if same {
            break;
        }
    }
    let mut ctrl = Vec::with_capacity(2 * pts.len());
    for (i, &(t, uv)) in pts.iter().enumerate() {
        let r = rate(t, uv);
        let step = |h: f64| Point2::new(uv.x + r.0 * h / 3.0, uv.y + r.1 * h / 3.0);
        ctrl.push(if i > 0 { step(pts[i - 1].0 - t) } else { uv });
        ctrl.push(if i + 1 < pts.len() {
            step(pts[i + 1].0 - t)
        } else {
            uv
        });
    }
    PcurveFit {
        ctrl,
        mults: mults(pts.len()),
        knots: pts.iter().map(|x| x.0).collect(),
    }
}

/// `c` over `range`, within `tol` at samples of each piece, its parameter kept.
pub fn fit_curve(c: &Curve, range: Interval, tol: f64) -> CurveFit {
    let mut ts = vec![range.lo, range.hi];
    // pieces at the curve's own smooth stretches would be sharper, but a kink only costs spans
    let at = |t: f64| {
        let d = c.deriv(t);
        (d.p, d.d1)
    };
    let mut error = 0.0_f64;
    // start from a few pieces: a curve can close on itself and look straight at its ends
    for _ in 0..2 {
        let mid: Vec<f64> = ts.windows(2).map(|w| 0.5 * (w[0] + w[1])).collect();
        ts.extend(mid);
        ts.sort_by(f64::total_cmp);
    }
    loop {
        let mut next = Vec::with_capacity(ts.len());
        let mut worst = 0.0_f64;
        for w in ts.windows(2) {
            let (a, b) = (at(w[0]), at(w[1]));
            let h = w[1] - w[0];
            let e = [0.25, 0.5, 0.75]
                .iter()
                .map(|&s| hermite(a.0, a.1, b.0, b.1, h, s).distance(c.eval(w[0] + h * s)))
                .fold(0.0, f64::max);
            worst = worst.max(e);
            next.push(w[0]);
            if e > tol && ts.len() < MAX_SPANS {
                next.push(w[0] + 0.5 * h);
            }
        }
        next.push(*ts.last().unwrap());
        error = error.max(worst);
        if next.len() == ts.len() {
            error = worst;
            break;
        }
        ts = next;
    }
    let mut ctrl = Vec::with_capacity(2 * ts.len());
    for (i, &t) in ts.iter().enumerate() {
        let (p, d) = at(t);
        if i > 0 {
            ctrl.push(p - d * ((t - ts[i - 1]) / 3.0));
        } else {
            ctrl.push(p);
        }
        if i + 1 < ts.len() {
            ctrl.push(p + d * ((ts[i + 1] - t) / 3.0));
        } else {
            ctrl.push(p);
        }
    }
    CurveFit {
        ctrl,
        mults: mults(ts.len()),
        knots: ts,
        error,
    }
}

fn bounded_domain(s: &Surface) -> bool {
    matches!(s, Surface::Procedural(p) if p.as_any().is_some_and(|a| a.is::<forge_geom::corner::HolePatch>()))
}

/// `s` over the parameter box, within `tol` where `keep` says the face is: what the box holds
/// beyond the face never shows, and a surface may be wild there. u and v are refined apart: a
/// span of either is halved while its middle misses by more than half of `tol`.
pub fn fit_surface(s: &Surface, uv: Box2, tol: f64, keep: &FaceMask) -> SurfaceFit {
    let mut us = vec![uv.u.lo, uv.u.mid(), uv.u.hi];
    let mut vs = vec![uv.v.lo, uv.v.mid(), uv.v.hi];
    if bounded_domain(s) {
        return fit_inside(s, uv, tol, keep);
    }
    let node = |u: f64, v: f64| s.deriv(Point2::new(u, v));
    let grid = |us: &[f64], vs: &[f64]| -> Vec<Vec<SurfDeriv>> {
        us.iter()
            .map(|&u| vs.iter().map(|&v| node(u, v)).collect())
            .collect()
    };
    let mut net = grid(&us, &vs);
    let mut error = 0.0;
    // the distance from the surface: parameters missing by more can still lie on it, as near a
    // patch's corner where its parametrisation is only continuous
    let off = |q: Point3, at: Point2| {
        if !keep.any(at.x, at.x, at.y, at.y) {
            return 0.0;
        }
        let e = q.distance(s.eval(at));
        if e > 0.25 * tol { e.min(s.distance(q)) } else { e }
    };
    for _ in 0..16 {
        let (mut eu, mut ev) = (vec![0.0_f64; us.len() - 1], vec![0.0_f64; vs.len() - 1]);
        let mut worst = 0.0_f64;
        for i in 0..us.len() - 1 {
            let (u0, u1) = (us[i], us[i + 1]);
            for j in 0..vs.len() - 1 {
                let (v0, v1) = (vs[j], vs[j + 1]);
                if !keep.any(u0, u1, v0, v1) {
                    continue;
                }
                let (a, b, c, d) = (&net[i][j], &net[i + 1][j], &net[i][j + 1], &net[i + 1][j + 1]);
                // along each direction, the mids of the cell's two lines
                let (mut lu, mut lv) = (0.0_f64, 0.0_f64);
                for (p, q, v) in [(a, b, v0), (c, d, v1)] {
                    lu = lu.max(off(
                        hermite(p.p, p.du, q.p, q.du, u1 - u0, 0.5),
                        Point2::new(0.5 * (u0 + u1), v),
                    ));
                }
                for (p, q, u) in [(a, c, u0), (b, d, u1)] {
                    lv = lv.max(off(
                        hermite(p.p, p.dv, q.p, q.dv, v1 - v0, 0.5),
                        Point2::new(u, 0.5 * (v0 + v1)),
                    ));
                }
                eu[i] = eu[i].max(lu);
                ev[j] = ev[j].max(lv);
                worst = worst.max(lu).max(lv);
                let (m, k) = (u1 - u0, v1 - v0);
                for (st, ct) in [(0.5, 0.5), (0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                    let e = off(
                        patch(a, b, c, d, m, k, st, ct),
                        Point2::new(u0 + m * st, v0 + k * ct),
                    );
                    worst = worst.max(e);
                    if e > 0.5 * tol {
                        // the inside's miss goes to the direction its edges miss most by; a
                        // twist alone (both edges right) to both
                        if lu >= 0.25 * tol && lu >= 2.0 * lv {
                            eu[i] = eu[i].max(e);
                        } else if lv >= 0.25 * tol && lv >= 2.0 * lu {
                            ev[j] = ev[j].max(e);
                        } else {
                            eu[i] = eu[i].max(e);
                            ev[j] = ev[j].max(e);
                        }
                    }
                }
            }
        }
        error = worst;
        if worst <= tol {
            break;
        }
        let split = |g: &[f64], e: &[f64]| -> Vec<f64> {
            let mut o = Vec::with_capacity(g.len() * 2);
            for k in 0..e.len() {
                o.push(g[k]);
                if e[k] > 0.5 * tol {
                    o.push(0.5 * (g[k] + g[k + 1]));
                }
            }
            o.push(*g.last().unwrap());
            o
        };
        let (nu, nv) = (split(&us, &eu), split(&vs, &ev));
        if nu.len() * nv.len() > MAX_CELLS || (nu.len() == us.len() && nv.len() == vs.len()) {
            break;
        }
        us = nu;
        vs = nv;
        net = grid(&us, &vs);
    }
    // the control net: at each grid point the four Bezier points the corners give
    let mut ctrl: Vec<Vec<Point3>> = Vec::with_capacity(2 * us.len());
    for i in 0..us.len() {
        let mut rows = [Vec::with_capacity(2 * vs.len()), Vec::with_capacity(2 * vs.len())];
        let hu = [
            if i > 0 { us[i - 1] - us[i] } else { 0.0 },
            if i + 1 < us.len() { us[i + 1] - us[i] } else { 0.0 },
        ];
        for j in 0..vs.len() {
            let d = &net[i][j];
            let hv = [
                if j > 0 { vs[j - 1] - vs[j] } else { 0.0 },
                if j + 1 < vs.len() { vs[j + 1] - vs[j] } else { 0.0 },
            ];
            for (r, &h) in rows.iter_mut().zip(&hu) {
                let a = h / 3.0;
                for &k in &hv {
                    let b = k / 3.0;
                    r.push(d.p + d.du * a + d.dv * b + d.duv * (a * b));
                }
            }
        }
        let [lo, hi] = rows;
        ctrl.push(lo);
        ctrl.push(hi);
    }
    SurfaceFit {
        ctrl,
        closed: (false, false),
        u: (us.clone(), mults(us.len())),
        v: (vs.clone(), mults(vs.len())),
        error,
    }
}

/// The bicubic Hermite patch over a cell, its twist from the corners.
#[allow(clippy::too_many_arguments)]
fn patch(
    a: &SurfDeriv,
    b: &SurfDeriv,
    c: &SurfDeriv,
    d: &SurfDeriv,
    m: f64,
    k: f64,
    s: f64,
    t: f64,
) -> Point3 {
    let f = |x: f64| {
        [
            2.0 * x * x * x - 3.0 * x * x + 1.0,
            x * x * x - 2.0 * x * x + x,
            -2.0 * x * x * x + 3.0 * x * x,
            x * x * x - x * x,
        ]
    };
    let (fu, fv) = (f(s), f(t));
    let term = |q: &SurfDeriv, su: f64, sv: f64, scale_u: f64, scale_v: f64| -> Vec3 {
        (q.p - Point3::ORIGIN) * (su * sv)
            + q.du * (scale_u * sv)
            + q.dv * (su * scale_v)
            + q.duv * (scale_u * scale_v)
    };
    let v = term(a, fu[0], fv[0], fu[1] * m, fv[1] * k)
        + term(b, fu[2], fv[0], fu[3] * m, fv[1] * k)
        + term(c, fu[0], fv[2], fu[1] * m, fv[3] * k)
        + term(d, fu[2], fv[2], fu[3] * m, fv[3] * k);
    Point3::ORIGIN + v
}

/// Bicubic B-spline with even knots, least squares over points on the face only: for surfaces
/// whose formula is wild beyond the face (a corner patch's blend past its sides), where a fit
/// through derivatives at grid points outside it would chase the wildness. Spans double until
/// points between the fitted ones are within `tol`. A little smoothing settles control points no
/// face point constrains.
fn fit_inside(s: &Surface, uv: Box2, tol: f64, keep: &FaceMask) -> SurfaceFit {
    let mut best: Option<SurfaceFit> = None;
    for &n in &[8usize, 16, 32, 64] {
        let fit = lsq(s, uv, n, keep);
        let done = fit.error <= tol;
        if best.as_ref().is_none_or(|b| fit.error < b.error) {
            best = Some(fit);
        }
        if done {
            break;
        }
    }
    best.expect("at least one size is tried")
}

/// The four cubic B-splines live at x in [0, 1] of the even clamped knot vector with `n` spans:
/// the first control point's index, and the values.
fn basis(x: f64, n: usize) -> (usize, [f64; 4]) {
    let t = (x.clamp(0.0, 1.0) * n as f64).min(n as f64 - 1e-12);
    let span = t as usize;
    let knot = |i: isize| -> f64 { (i.clamp(0, n as isize)) as f64 };
    let k = span as isize;
    // the triangle of Cox-de Boor over knots clamped at both ends
    let mut b = [1.0, 0.0, 0.0, 0.0];
    for d in 1..4isize {
        let mut saved = 0.0;
        for r in 0..d {
            let (right, left) = (knot(k + 1 + r) - t, t - knot(k + 1 + r - d));
            let w = b[r as usize] / (right + left);
            b[r as usize] = saved + right * w;
            saved = left * w;
        }
        b[d as usize] = saved;
    }
    (span, b)
}

#[allow(clippy::needless_range_loop)]
fn lsq(s: &Surface, uv: Box2, n: usize, keep: &FaceMask) -> SurfaceFit {
    let m = n + 3;
    let total = m * m;
    let band = 3 * m + 4;
    // the normal equations, lower band by columns, one right side per coordinate
    let mut a = vec![0.0_f64; total * (band + 1)];
    let mut rhs = vec![[0.0_f64; 3]; total];
    let at = |i: usize, j: usize| i * m + j;
    let param = |x: f64, y: f64| Point2::new(uv.u.lo + uv.u.width() * x, uv.v.lo + uv.v.width() * y);
    let mut add = |x: f64, y: f64, q: Point3, w: f64| {
        let (i0, bu) = basis(x, n);
        let (j0, bv) = basis(y, n);
        let mut idx = [0usize; 16];
        let mut val = [0.0_f64; 16];
        for i in 0..4 {
            for j in 0..4 {
                idx[i * 4 + j] = at(i0 + i, j0 + j);
                val[i * 4 + j] = bu[i] * bv[j];
            }
        }
        for k in 0..16 {
            for l in 0..16 {
                if idx[l] <= idx[k] {
                    a[idx[l] * (band + 1) + (idx[k] - idx[l])] += w * val[k] * val[l];
                }
            }
            rhs[idx[k]][0] += w * val[k] * q.x;
            rhs[idx[k]][1] += w * val[k] * q.y;
            rhs[idx[k]][2] += w * val[k] * q.z;
        }
    };
    let dense = 4 * n;
    for i in 0..=dense {
        for j in 0..=dense {
            let (x, y) = (i as f64 / dense as f64, j as f64 / dense as f64);
            let p = param(x, y);
            if keep.inside(p) {
                add(x, y, s.eval(p), 1.0);
            }
        }
    }
    for p in keep.boundary(dense) {
        let (x, y) = ((p.x - uv.u.lo) / uv.u.width(), (p.y - uv.v.lo) / uv.v.width());
        add(x, y, s.eval(p), 4.0);
    }
    // second differences along each direction, weighted lightly
    let lambda = 1e-6;
    for i in 0..m {
        for j in 0..m {
            for (a0, a1, a2) in [
                (
                    i.checked_sub(1).map(|k| at(k, j)),
                    at(i, j),
                    (i + 1 < m).then(|| at(i + 1, j)),
                ),
                (
                    j.checked_sub(1).map(|k| at(i, k)),
                    at(i, j),
                    (j + 1 < m).then(|| at(i, j + 1)),
                ),
            ] {
                if let (Some(p), Some(q)) = (a0, a2) {
                    let t = [(p, 1.0), (a1, -2.0), (q, 1.0)];
                    for &(k, u) in &t {
                        for &(l, v) in &t {
                            if l <= k {
                                a[l * (band + 1) + (k - l)] += lambda * u * v;
                            }
                        }
                    }
                }
            }
        }
    }
    // band Cholesky, LDL^T free
    for c in 0..total {
        let d = a[c * (band + 1)].max(1e-300).sqrt();
        a[c * (band + 1)] = d;
        for r in 1..=band.min(total - 1 - c) {
            a[c * (band + 1) + r] /= d;
        }
        for r in 1..=band.min(total - 1 - c) {
            let v = a[c * (band + 1) + r];
            if v == 0.0 {
                continue;
            }
            for q in r..=band.min(total - 1 - c) {
                a[(c + r) * (band + 1) + (q - r)] -= v * a[c * (band + 1) + q];
            }
        }
    }
    for k in 0..3 {
        for c in 0..total {
            let d = a[c * (band + 1)];
            let v = rhs[c][k] / d;
            rhs[c][k] = v;
            for r in 1..=band.min(total - 1 - c) {
                rhs[c + r][k] -= a[c * (band + 1) + r] * v;
            }
        }
        for c in (0..total).rev() {
            let mut v = rhs[c][k];
            for r in 1..=band.min(total - 1 - c) {
                v -= a[c * (band + 1) + r] * rhs[c + r][k];
            }
            rhs[c][k] = v / a[c * (band + 1)];
        }
    }
    let ctrl: Vec<Vec<Point3>> = (0..m)
        .map(|i| {
            (0..m)
                .map(|j| {
                    let r = rhs[at(i, j)];
                    Point3::new(r[0], r[1], r[2])
                })
                .collect()
        })
        .collect();
    let eval = |x: f64, y: f64| {
        let (i0, bu) = basis(x, n);
        let (j0, bv) = basis(y, n);
        let mut q = Vec3::ZERO;
        for i in 0..4 {
            for j in 0..4 {
                q += (ctrl[i0 + i][j0 + j] - Point3::ORIGIN) * (bu[i] * bv[j]);
            }
        }
        Point3::ORIGIN + q
    };
    // between the fitted points, and along the loops
    let mut error = 0.0_f64;
    let probe = 2 * dense;
    for i in 0..probe {
        for j in 0..probe {
            if (i + j) % 2 == 0 {
                continue;
            }
            let (x, y) = ((i as f64 + 0.5) / probe as f64, (j as f64 + 0.5) / probe as f64);
            let p = param(x, y);
            if keep.inside(p) {
                error = error.max(eval(x, y).distance(s.eval(p)));
            }
        }
    }
    for p in keep.boundary(probe) {
        let (x, y) = ((p.x - uv.u.lo) / uv.u.width(), (p.y - uv.v.lo) / uv.v.width());
        error = error.max(eval(x, y).distance(s.eval(p)));
    }
    let knots = |lo: f64, w: f64| -> (Vec<f64>, Vec<u32>) {
        let k: Vec<f64> = (0..=n).map(|i| lo + w * i as f64 / n as f64).collect();
        let mut mu = vec![1; n + 1];
        mu[0] = 4;
        mu[n] = 4;
        (k, mu)
    };
    SurfaceFit {
        ctrl,
        closed: (false, false),
        u: knots(uv.u.lo, uv.u.width()),
        v: knots(uv.v.lo, uv.v.width()),
        error,
    }
}

/// Makes the net's ends one row of points, where a seam joins them.
pub fn close_net(fit: &mut SurfaceFit, seam: (bool, bool)) {
    if seam.0 {
        let n = fit.ctrl.len() - 1;
        for j in 0..fit.ctrl[0].len() {
            let m = fit.ctrl[0][j] + (fit.ctrl[n][j] - fit.ctrl[0][j]) * 0.5;
            fit.ctrl[0][j] = m;
            fit.ctrl[n][j] = m;
        }
        fit.closed.0 = true;
    }
    if seam.1 {
        for r in &mut fit.ctrl {
            let n = r.len() - 1;
            let m = r[0] + (r[n] - r[0]) * 0.5;
            r[0] = m;
            r[n] = m;
        }
        fit.closed.1 = true;
    }
}

/// What a fit left, for the export's report.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fit {
    pub count: usize,
    pub error: f64,
}
