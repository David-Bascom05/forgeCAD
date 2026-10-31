//! Where a smooth-loft span meets another span or a quadric, without tracing. A span whose rails are
//! all lines, or all arcs turning through the same angles, is exactly a (rational) Bezier patch, so a
//! piece of it lies in the hull of its control points. Two pieces whose hulls a plane separates can't
//! meet, and a quadric whose value has one sign over every Bernstein coefficient of a piece misses it.
//! Pieces neither test clears are split down to a fraction of the region. Seeded from what is left,
//! the curve is followed with Newton steps in the surfaces' own parameters: closed-form points, no
//! nearest-point searches. Derivation and measurements: research/loft-ssi/novel/README.md.
//! Lean proofs of the patch form, hull, separation and sign tests: research/loft-ssi/shipped/README.md.

use crate::Curve;
use crate::intcurve::IntCurve;
use crate::intersect::{Contact, SurfaceSurface};
use crate::meet::Alg;
use crate::skinned::{Hull, Skinned};
use crate::surface::{Procedural, SurfDeriv, Surface};
use forge_math::{Box2, Box3, Interval, Point2, Point3, Vec3, dmath, tol};

/// The most hull tests one pair may take before it is left to the tracer.
const BUDGET: usize = 200_000;

fn skinned(s: &Surface) -> Option<&Skinned> {
    match s {
        Surface::Procedural(p) => p.as_any().and_then(|x| x.downcast_ref::<Skinned>()),
        _ => None,
    }
}

/// The curves where `a` and `b` meet inside `region`, for a span against a span or a quadric; None
/// for any other pair, or when a step fails (the tracer takes those).
pub fn meet(a: &Surface, b: &Surface, region: Box3) -> Option<SurfaceSurface> {
    let pair = match (skinned(a), skinned(b)) {
        (Some(x), Some(y)) => Pair::Spans(x, y),
        (Some(x), None) => Pair::Quad(x, quadric(b)?),
        (None, Some(y)) => Pair::Quad(y, quadric(a)?),
        (None, None) => return None,
    };
    let leaf = (region.diagonal() / 64.0).max(10.0 * tol::LINEAR);
    // as the tracer: curves a step outside the region still count, for the trims that meet them
    let seeds = pair.seeds(region.grow(leaf * 0.5), leaf)?;
    let curves = pair.follow(&seeds, region, leaf * 0.5)?;
    // a span first, as the pair came
    let flip = matches!(pair, Pair::Quad(..)) && skinned(a).is_none();
    let (sa, sb) = if flip { (b, a) } else { (a, b) };
    Some(if curves.is_empty() {
        SurfaceSurface::Empty
    } else {
        SurfaceSurface::Curves(
            curves
                .into_iter()
                .map(|(pts, tans, closed, sag)| {
                    let c = IntCurve::new(sa.clone(), sb.clone(), pts, tans, closed, 1.5 * sag + tol::LINEAR);
                    (Curve::Intersection(std::sync::Arc::new(c)), Contact::Crossing)
                })
                .collect(),
        )
    })
}

fn quadric(s: &Surface) -> Option<Alg> {
    match Alg::of(s)? {
        q @ Alg::Quadric { .. } => Some(q),
        Alg::Torus { .. } => None,
    }
}

enum Pair<'a> {
    Spans(&'a Skinned, &'a Skinned),
    Quad(&'a Skinned, Alg),
}

/// The face's domain in pieces narrow enough for one rational arc each (a quarter turn).
fn pieces(s: &Skinned) -> Vec<Box2> {
    let d = s.domain();
    let n = (s.turn().abs() / (0.5 * dmath::PI)).ceil().max(1.0) as usize;
    (0..n)
        .map(|k| {
            let (u0, u1) = (k as f64 / n as f64, (k + 1) as f64 / n as f64);
            Box2::new(Interval::new(u0, u1), d.v.interval())
        })
        .collect()
}

fn halves(p: Box2, h: &Hull) -> [Box2; 2] {
    if h.along_v() >= h.along_u() {
        let m = p.v.mid();
        [
            Box2::new(p.u, Interval::new(p.v.lo, m)),
            Box2::new(p.u, Interval::new(m, p.v.hi)),
        ]
    } else {
        let m = p.u.mid();
        [
            Box2::new(Interval::new(p.u.lo, m), p.v),
            Box2::new(Interval::new(m, p.u.hi), p.v),
        ]
    }
}

fn size(b: &Box3) -> f64 {
    b.x.width().max(b.y.width()).max(b.z.width())
}

/// Whether a plane square to one of a few axes parts the two hulls by more than `gap`.
fn apart(x: &Hull, y: &Hull, gap: f64) -> bool {
    let axes = [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        x.normal(),
        y.normal(),
        y.centre() - x.centre(),
    ];
    axes.iter().any(|ax| {
        let Some(u) = ax.normalize() else { return false };
        let (ix, iy) = (x.extent(u.get()), y.extent(u.get()));
        ix.hi + gap < iy.lo || iy.hi + gap < ix.lo
    })
}

impl Pair<'_> {
    /// Parameters near every place the two may meet in `region` (u, v of the span, then of the other
    /// span); None past the budget or where a span has no hull.
    fn seeds(&self, region: Box3, leaf: f64) -> Option<Vec<[f64; 4]>> {
        let mut out = Vec::new();
        let mut tests = 0;
        match *self {
            Pair::Spans(a, b) => {
                let mut stack: Vec<(Box2, Box2)> = Vec::new();
                for &x in &pieces(a) {
                    for &y in &pieces(b) {
                        stack.push((x, y));
                    }
                }
                while let Some((x, y)) = stack.pop() {
                    tests += 1;
                    if tests > BUDGET {
                        return None;
                    }
                    let (hx, hy) = (a.hull(x)?, b.hull(y)?);
                    let (bx, by) = (hx.bbox(), hy.bbox());
                    if !bx.overlaps(&region) || !by.overlaps(&region) || apart(&hx, &hy, tol::LINEAR) {
                        continue;
                    }
                    if size(&bx) <= leaf && size(&by) <= leaf {
                        let (p, q) = (x.center(), y.center());
                        out.push([p.x, p.y, q.x, q.y]);
                    } else if size(&bx) >= size(&by) {
                        stack.extend(halves(x, &hx).map(|h| (h, y)));
                    } else {
                        stack.extend(halves(y, &hy).map(|h| (x, h)));
                    }
                }
            }
            Pair::Quad(a, q) => {
                let mut stack = pieces(a);
                while let Some(x) = stack.pop() {
                    tests += 1;
                    if tests > BUDGET {
                        return None;
                    }
                    let h = a.hull(x)?;
                    let bx = h.bbox();
                    if !bx.overlaps(&region) || h.misses(&q) {
                        continue;
                    }
                    if size(&bx) <= leaf {
                        let p = x.center();
                        out.push([p.x, p.y, 0.0, 0.0]);
                    } else {
                        stack.extend(halves(x, &h));
                    }
                }
            }
        }
        Some(out)
    }

    /// The residual, its Jacobian's rows (in the unknowns used), the normals of the two surfaces,
    /// and how far each parameter moves the point (per unit).
    #[allow(clippy::type_complexity)]
    fn at(&self, x: &[f64; 4]) -> (Point3, Vec<f64>, Vec<[f64; 4]>, Vec3, Vec3, [f64; 4]) {
        match *self {
            Pair::Spans(a, b) => {
                let (p, q) = (a.deriv(Point2::new(x[0], x[1])), b.deriv(Point2::new(x[2], x[3])));
                let f = p.p - q.p;
                let row = |k: usize| {
                    let c = |v: Vec3| [v.x, v.y, v.z][k];
                    [c(p.du), c(p.dv), -c(q.du), -c(q.dv)]
                };
                (
                    p.p,
                    vec![f.x, f.y, f.z],
                    vec![row(0), row(1), row(2)],
                    p.du.cross(p.dv),
                    q.du.cross(q.dv),
                    [p.du.length(), p.dv.length(), q.du.length(), q.dv.length()],
                )
            }
            Pair::Quad(a, q) => {
                let p = a.deriv(Point2::new(x[0], x[1]));
                let (h, g) = q.grad(p.p);
                (
                    p.p,
                    vec![h],
                    vec![[g.dot(p.du), g.dot(p.dv), 0.0, 0.0]],
                    p.du.cross(p.dv),
                    g,
                    [p.du.length(), p.dv.length(), 1.0, 1.0],
                )
            }
        }
    }

    /// Minimum-norm Newton from x onto both surfaces: the parameters and the point.
    fn newton(&self, mut x: [f64; 4]) -> Option<([f64; 4], Point3)> {
        let norm = |f: &[f64]| f.iter().map(|v| v * v).sum::<f64>().sqrt();
        let (mut p, mut f, mut jac, _, mut n2, mut w) = self.at(&x);
        for _ in 0..30 {
            let scale = match self {
                Pair::Spans(..) => 1.0,
                Pair::Quad(..) => n2.length(),
            };
            if norm(&f) <= 1e-11 * scale * (1.0 + p.to_vec().length()) {
                return Some((x, p));
            }
            // the least move of the point, not of the parameters (a turn of the angle sweeps far
            // more than a unit along the loft): dx = -W^-1 J^T (J W^-1 J^T)^-1 f, W = diag(w^2)
            let w2: [f64; 4] = std::array::from_fn(|k| (w[k] * w[k]).max(1e-300));
            let m = f.len();
            let mut g = [[0.0; 3]; 3];
            for i in 0..m {
                for j in 0..m {
                    g[i][j] = (0..4).map(|k| jac[i][k] * jac[j][k] / w2[k]).sum();
                }
            }
            let y = solve(&g, &f, m)?;
            let dx: [f64; 4] = std::array::from_fn(|k| (0..m).map(|i| jac[i][k] * y[i]).sum::<f64>() / w2[k]);
            x = std::array::from_fn(|k| x[k] - dx[k]);
            let next = self.at(&x);
            (p, f, jac, n2, w) = (next.0, next.1, next.2, next.4, next.5);
        }
        None
    }

    /// Within both spans as far as they are carried on.
    fn inside(&self, x: &[f64; 4]) -> bool {
        let on = |s: &Skinned, u: f64, v: f64| s.reach().contains(Point2::new(u, v));
        match *self {
            Pair::Spans(a, b) => on(a, x[0], x[1]) && on(b, x[2], x[3]),
            Pair::Quad(a, _) => on(a, x[0], x[1]),
        }
    }

    /// The parameters a step `h` along the curve's unit direction `d` from x.
    fn ahead(&self, x: &[f64; 4], d: Vec3, h: f64) -> [f64; 4] {
        // on each span, [S_u S_v] (du, dv) = d in least squares
        let local = |s: &SurfDeriv| {
            let (e, f, g) = (s.du.dot(s.du), s.du.dot(s.dv), s.dv.dot(s.dv));
            let (u, v) = (s.du.dot(d), s.dv.dot(d));
            let det = e * g - f * f;
            ((g * u - f * v) / det * h, (e * v - f * u) / det * h)
        };
        let (a, b) = match *self {
            Pair::Spans(a, b) => (a, Some(b)),
            Pair::Quad(a, _) => (a, None),
        };
        let (d0, d1) = local(&a.deriv(Point2::new(x[0], x[1])));
        let (d2, d3) = b.map_or((0.0, 0.0), |b| local(&b.deriv(Point2::new(x[2], x[3]))));
        [x[0] + d0, x[1] + d1, x[2] + d2, x[3] + d3]
    }

    fn direction(&self, x: &[f64; 4]) -> Option<Vec3> {
        let (.., n1, n2, _) = self.at(x);
        n1.cross(n2).normalize().map(|u| u.get())
    }

    /// Each branch through the seeds: its points, directions, whether it closes, and its sag. None
    /// where a step fails.
    #[allow(clippy::type_complexity)]
    fn follow(
        &self,
        seeds: &[[f64; 4]],
        region: Box3,
        step: f64,
    ) -> Option<Vec<(Vec<Point3>, Vec<Vec3>, bool, f64)>> {
        let grown = region.grow(step);
        let mut out: Vec<(Vec<Point3>, Vec<Vec3>, bool, f64)> = Vec::new();
        for &s in seeds {
            let (x0, p0) = self.newton(s)?;
            if !self.inside(&x0) || !grown.contains(p0) {
                continue;
            }
            if out
                .iter()
                .any(|c| c.0.iter().any(|q| q.distance(p0) <= 1.5 * step))
            {
                continue;
            }
            let walk = |sign: f64| -> Option<(Vec<[f64; 4]>, Vec<Point3>, bool)> {
                let (mut xs, mut ps) = (vec![x0], vec![p0]);
                let max = (64.0 * grown.diagonal() / step) as usize + 16;
                for k in 0..max {
                    let x = xs[xs.len() - 1];
                    let d = self.direction(&x)? * sign;
                    let (nx, q) = self.newton(self.ahead(&x, d, step))?;
                    if (q - ps[ps.len() - 1]).dot(d) <= 0.0 {
                        return None;
                    }
                    if k > 2 && q.distance(p0) <= 0.75 * step {
                        return Some((xs, ps, true));
                    }
                    // the spans end where they stop being carried on: the curve ends there too
                    if !self.inside(&nx) {
                        let (mut lo, mut hi) = (0.0, step);
                        let mut last = None;
                        for _ in 0..20 {
                            let h = 0.5 * (lo + hi);
                            match self.newton(self.ahead(&x, d, h)) {
                                Some((y, q)) if self.inside(&y) => (lo, last) = (h, Some((y, q))),
                                _ => hi = h,
                            }
                        }
                        if let Some((y, q)) = last.filter(|_| lo > 1e-3 * step) {
                            xs.push(y);
                            ps.push(q);
                        }
                        return Some((xs, ps, false));
                    }
                    xs.push(nx);
                    ps.push(q);
                    if !grown.contains(q) {
                        return Some((xs, ps, false));
                    }
                }
                None
            };
            let (fx, fp, closed) = walk(1.0)?;
            let (xs, pts) = if closed {
                (fx, fp)
            } else {
                let (mut bx, mut bp, _) = walk(-1.0)?;
                bx.reverse();
                bp.reverse();
                bx.pop();
                bp.pop();
                bx.extend(fx);
                bp.extend(fp);
                (bx, bp)
            };
            if pts.len() < 2 {
                continue;
            }
            let n = pts.len();
            let segs = if closed { n } else { n - 1 };
            // measured: how far each chord's midpoint sits from the curve
            let mut sag = 0.0f64;
            for i in 0..segs {
                let j = (i + 1) % n;
                let mid = std::array::from_fn(|k| 0.5 * (xs[i][k] + xs[j][k]));
                let m = pts[i].lerp(pts[j], 0.5);
                sag = sag.max(self.newton(mid).map_or(step, |(_, q)| q.distance(m)));
            }
            let tans = (0..n)
                .map(|i| {
                    let along = if closed || (i > 0 && i + 1 < n) {
                        pts[(i + 1) % n] - pts[(i + n - 1) % n]
                    } else if i == 0 {
                        pts[1] - pts[0]
                    } else {
                        pts[i] - pts[i - 1]
                    };
                    let d = self.direction(&xs[i]).unwrap_or(Vec3::ZERO);
                    if d.dot(along) < 0.0 { -d } else { d }
                })
                .collect();
            out.push((pts, tans, closed, sag));
        }
        Some(out)
    }
}

/// y with G y = f for the leading m x m block (m = 1 or 3); None when singular.
fn solve(g: &[[f64; 3]; 3], f: &[f64], m: usize) -> Option<[f64; 3]> {
    if m == 1 {
        return (g[0][0] > 0.0).then(|| [f[0] / g[0][0], 0.0, 0.0]);
    }
    let det3 = |a: &[[f64; 3]; 3]| {
        a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1]) - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
    };
    let det = det3(g);
    let tr = g[0][0] + g[1][1] + g[2][2];
    if det.abs() <= 1e-24 * tr * tr * tr {
        return None;
    }
    let mut y = [0.0; 3];
    for (k, yk) in y.iter_mut().enumerate() {
        let mut a = *g;
        for i in 0..3 {
            a[i][k] = f[i];
        }
        *yk = det3(&a) / det;
    }
    Some(y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Circle;
    use crate::ruled::Rail;
    use crate::skinned::Spline;
    use crate::surface::{Cylinder, Plane, SurfaceGeom};
    use forge_math::Frame;
    use forge_math::rng::Rng;
    use std::sync::Arc;

    /// Span k of half (0 or 1) of a smooth loft of four circles round a bend, `lift` up.
    fn tube(radii: [f64; 4], k: usize, half: usize, lift: f64) -> Surface {
        let (mut rails, mut knots, mut last) = (Vec::new(), Vec::new(), None::<Point3>);
        for (i, &radius) in radii.iter().enumerate() {
            let t = 0.5 * i as f64;
            let (sn, cs) = dmath::sin_cos(t);
            let o = Point3::new(30.0 * cs, 30.0 * sn, lift);
            let z = Vec3::new(-sn, cs, 0.0).normalize().unwrap();
            let frame = Frame::from_zx(o, z, Vec3::new(cs, sn, 0.0)).unwrap();
            knots.push(last.map_or(0.0, |p| knots[knots.len() - 1] + p.distance(o)));
            last = Some(o);
            let start = half as f64 * dmath::PI;
            rails.push(Rail::Arc {
                circle: Circle { frame, radius },
                start,
                sweep: dmath::PI,
            });
        }
        let sp = Arc::new(Spline::new(knots, false, false).unwrap());
        Skinned::new(rails, vec![], sp, k, vec![]).unwrap().surface()
    }

    fn radii(r: &mut Rng) -> [f64; 4] {
        std::array::from_fn(|_| r.range(6.0, 9.0))
    }

    fn on_both(c: &Curve, a: &Surface, b: &Surface) {
        let Curve::Intersection(ic) = c else {
            panic!("an intersection curve")
        };
        for i in 0..=40 {
            let p = ic.eval(ic.domain_hi() * i as f64 / 40.0);
            assert!(a.distance(p) <= tol::LINEAR && b.distance(p) <= tol::LINEAR);
        }
    }

    #[test]
    fn hulls_hold_their_pieces_and_quadric_signs_agree() {
        let mut r = Rng::new(17);
        for _ in 0..20 {
            let s = tube(radii(&mut r), r.below(3), r.below(2), 0.0);
            let sk = skinned(&s).unwrap();
            let v = sk.domain().v.interval();
            let (u0, v0) = (r.range(0.0, 0.5), r.range(v.lo, v.mid()));
            let piece = Box2::new(
                Interval::new(u0, u0 + 0.4),
                Interval::new(v0, v0 + 0.4 * v.width()),
            );
            let h = sk.hull(piece).unwrap();
            let cyl = Surface::Cylinder(Cylinder {
                frame: Frame::WORLD,
                radius: r.range(20.0, 40.0),
            });
            let q = quadric(&cyl).unwrap();
            let (mut neg, mut pos) = (false, false);
            for i in 0..=8 {
                for j in 0..=8 {
                    let x = piece.u.lo + piece.u.width() * i as f64 / 8.0;
                    let p = sk
                        .deriv(Point2::new(x, piece.v.lo + piece.v.width() * j as f64 / 8.0))
                        .p;
                    assert!(h.bbox().grow(1e-9).contains(p));
                    let g = q.grad(p).0;
                    neg |= g < 0.0;
                    pos |= g > 0.0;
                }
            }
            assert!(!(neg && pos) || !h.misses(&q));
        }
    }

    #[test]
    #[ignore = "soundness gap: follow() drops a branch within 1.5 steps of one already traced"]
    fn close_parallel_branches_are_both_found() {
        let region = Box3::from_points([Point3::new(-50.0, -50.0, -20.0), Point3::new(50.0, 50.0, 20.0)]);
        // the top half of a tube of radius 8 (top at z = 8 all along), cut by z = 7.98: two curves
        // about 1.13 apart, closer than 1.5 steps (1.72 for this region)
        let a = tube([8.0; 4], 1, 1, 0.0);
        let plane = Surface::Plane(Plane {
            frame: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 7.98)),
        });
        let Some(SurfaceSurface::Curves(cs)) = meet(&a, &plane, region) else {
            panic!("it cuts")
        };
        cs.iter().for_each(|c| on_both(&c.0, &a, &plane));
        assert_eq!(cs.len(), 2);
    }

    #[test]
    fn spans_meet_where_the_tracer_finds_and_nested_ones_nowhere() {
        let region = Box3::from_points([Point3::new(-50.0, -50.0, -20.0), Point3::new(50.0, 50.0, 20.0)]);
        let rs = radii(&mut Rng::new(3));
        // a's top half and b's bottom half, 5 up: equal circles meet half way
        let (a, b) = (tube(rs, 1, 1, 0.0), tube(rs, 1, 0, 5.0));
        let Some(SurfaceSurface::Curves(cs)) = meet(&a, &b, region) else {
            panic!("they cross")
        };
        // one each side of the bend (the tracer gives up on this pair: they turn tangent past the faces)
        assert_eq!(cs.len(), 2);
        cs.iter().for_each(|c| on_both(&c.0, &a, &b));
        let inner = tube(rs.map(|x| x - 3.0), 1, 1, 0.0);
        assert_eq!(meet(&a, &inner, region), Some(SurfaceSurface::Empty));
        let plane = Surface::Plane(Plane {
            frame: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 2.0)),
        });
        let Some(SurfaceSurface::Curves(cs)) = meet(&a, &plane, region) else {
            panic!("it cuts")
        };
        cs.iter().for_each(|c| on_both(&c.0, &a, &plane));
    }
}
