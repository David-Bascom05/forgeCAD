//! A tensor-product rational B-spline surface, as STEP files state them: a `Procedural` surface
//! private to import. The kernel has no surface splines otherwise (research/step/README.md).

use forge_geom::curve::Domain1;
use forge_geom::surface::{Proc, Procedural, SurfDeriv};
use forge_geom::{Domain2, Surface};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec3};
use std::sync::{Arc, OnceLock};

/// The highest degree taken: as the kernel's curves'. STEP has no limit; none surveyed passes 8.
pub const MAX_DEGREE: usize = forge_geom::spline::MAX_DEGREE;
/// The most control points taken.
pub const MAX_CTRL: usize = 4 << 20;

#[derive(Debug, Clone)]
pub struct Nurbs {
    p: usize,
    q: usize,
    /// Expanded knots.
    u: Vec<f64>,
    v: Vec<f64>,
    /// Homogeneous points by rows of constant u index, `nv` to a row.
    net: Arc<Vec<[f64; 4]>>,
    nu: usize,
    nv: usize,
    closed: (bool, bool),
    seeds: Arc<OnceLock<Seeds>>,
    tree: Arc<OnceLock<Tree>>,
}

/// A tree of boxes over the surface's knot cells, for the nearest point anywhere on it: a net can
/// have thousands of cells on a part that winds near itself, like a thread.
#[derive(Debug)]
struct Tree {
    /// Node i's box, and its children (0: a leaf, whose cell is `leaf`).
    nodes: Vec<(Box3, u32, u32, (u32, u32))>,
    us: Vec<(f64, f64, usize)>,
    vs: Vec<(f64, f64, usize)>,
}

#[derive(Debug)]
struct Seeds {
    /// What a grid of surface points shows: the radius of curvature and lengths per parameter.
    radius: f64,
    scale: (f64, f64),
}

/// The live basis functions at `t` and their first two derivatives, from Cox and de Boor's
/// recurrence: `out[d][j]` is derivative d of function `span - p + j`.
fn basis(knots: &[f64], p: usize, span: usize, t: f64, out: &mut [[f64; MAX_DEGREE + 1]; 3]) {
    // a triangle of values for each degree, kept for the derivative formulas
    let mut ndu = [[0.0; MAX_DEGREE + 1]; MAX_DEGREE + 1];
    let (mut left, mut right) = ([0.0; MAX_DEGREE + 1], [0.0; MAX_DEGREE + 1]);
    ndu[0][0] = 1.0;
    for j in 1..=p {
        left[j] = t - knots[span + 1 - j];
        right[j] = knots[span + j] - t;
        let mut saved = 0.0;
        for r in 0..j {
            ndu[j][r] = right[r + 1] + left[j - r];
            let w = if ndu[j][r] != 0.0 {
                ndu[r][j - 1] / ndu[j][r]
            } else {
                0.0
            };
            ndu[r][j] = saved + right[r + 1] * w;
            saved = left[j - r] * w;
        }
        ndu[j][j] = saved;
    }
    for j in 0..=p {
        out[0][j] = ndu[j][p];
    }
    // N'[i, k] = k (N[i, k - 1] / (u[i + k] - u[i]) - N[i + 1, k - 1] / (u[i + k + 1] - u[i + 1]));
    // the second derivative is the same over the degree below's first derivatives
    let ratio = |a: f64, d: f64| if d != 0.0 { a / d } else { 0.0 };
    let first = |k: usize, j: usize| -> f64 {
        if k == 0 {
            return 0.0;
        }
        let i = span - k + j;
        let a = if j >= 1 {
            ratio(ndu[j - 1][k - 1], knots[i + k] - knots[i])
        } else {
            0.0
        };
        let b = if j < k {
            ratio(ndu[j][k - 1], knots[i + k + 1] - knots[i + 1])
        } else {
            0.0
        };
        k as f64 * (a - b)
    };
    for j in 0..=p {
        out[1][j] = first(p, j);
        let i = span - p + j;
        let a = if j >= 1 && p >= 1 {
            ratio(first(p - 1, j - 1), knots[i + p] - knots[i])
        } else {
            0.0
        };
        let b = if j < p {
            ratio(first(p - 1, j), knots[i + p + 1] - knots[i + 1])
        } else {
            0.0
        };
        out[2][j] = p as f64 * (a - b);
    }
}

/// The span holding `t`: `knots[s] <= t < knots[s + 1]`, the last span taking the domain's end.
fn span_of(knots: &[f64], p: usize, n: usize, t: f64) -> usize {
    if t >= knots[n] {
        let mut s = n - 1;
        while s > p && knots[s] >= knots[s + 1] {
            s -= 1;
        }
        return s;
    }
    if t <= knots[p] {
        let mut s = p;
        while s + 1 < n && knots[s + 1] <= knots[s] {
            s += 1;
        }
        return s;
    }
    let (mut lo, mut hi) = (p, n);
    while hi - lo > 1 {
        let m = (lo + hi) / 2;
        if t < knots[m] {
            hi = m;
        } else {
            lo = m;
        }
    }
    lo
}

impl Nurbs {
    /// None when the data does not make a surface: knots out of order or too few, weights not
    /// positive, a number not finite, or over the limits.
    pub fn new(
        (p, q): (usize, usize),
        (u, v): (Vec<f64>, Vec<f64>),
        (nu, nv): (usize, usize),
        pts: &[Point3],
        weights: Option<&[f64]>,
    ) -> Option<Nurbs> {
        if p == 0 || q == 0 || p > MAX_DEGREE || q > MAX_DEGREE {
            return None;
        }
        if nu <= p || nv <= q || nu.checked_mul(nv)? > MAX_CTRL || pts.len() != nu * nv {
            return None;
        }
        if u.len() != nu + p + 1 || v.len() != nv + q + 1 {
            return None;
        }
        let good = |k: &[f64], d: usize, n: usize| {
            k.iter().all(|x| x.is_finite()) && k.windows(2).all(|w| w[1] >= w[0]) && k[n] > k[d]
        };
        if !good(&u, p, nu) || !good(&v, q, nv) {
            return None;
        }
        if weights.is_some_and(|w| w.len() != pts.len() || w.iter().any(|&x| !(x > 0.0 && x.is_finite()))) {
            return None;
        }
        let mut net = Vec::with_capacity(pts.len());
        for (i, c) in pts.iter().enumerate() {
            if !(c.x.is_finite() && c.y.is_finite() && c.z.is_finite()) {
                return None;
            }
            let w = weights.map_or(1.0, |w| w[i]);
            net.push([c.x * w, c.y * w, c.z * w, w]);
        }
        let mut n = Nurbs {
            p,
            q,
            u,
            v,
            net: Arc::new(net),
            nu,
            nv,
            closed: (false, false),
            seeds: Arc::new(OnceLock::new()),
            tree: Arc::new(OnceLock::new()),
        };
        n.closed = n.find_closed();
        Some(n)
    }

    pub fn surface(self) -> Surface {
        Surface::Procedural(Proc(Arc::new(self)))
    }

    /// Whether the surface's ends in a parameter are one curve, taken as periodic there. Checked
    /// on the surface, not the flags in the file: those are often wrong.
    fn find_closed(&self) -> (bool, bool) {
        let d = self.dom();
        let tol = forge_math::tol::LINEAR;
        let same = |a: Point2, b: Point2| self.point_at(a).distance(self.point_at(b)) <= tol;
        let at = |k: usize| k as f64 / 8.0;
        let cu = (0..=8).all(|k| {
            let v = d.1.lo + d.1.width() * at(k);
            same(Point2::new(d.0.lo, v), Point2::new(d.0.hi, v))
        });
        let cv = (0..=8).all(|k| {
            let u = d.0.lo + d.0.width() * at(k);
            same(Point2::new(u, d.1.lo), Point2::new(u, d.1.hi))
        });
        (cu, cv)
    }

    fn dom(&self) -> (Interval, Interval) {
        (
            Interval::new(self.u[self.p], self.u[self.nu]),
            Interval::new(self.v[self.q], self.v[self.nv]),
        )
    }

    fn wrap(&self, uv: Point2) -> Point2 {
        let (du, dv) = self.dom();
        let w = |x: f64, d: Interval, c: bool| {
            if c && !(d.lo..=d.hi).contains(&x) && x.is_finite() {
                let k = ((x - d.lo) / d.width()).floor();
                x - k * d.width()
            } else {
                x.clamp(d.lo, d.hi)
            }
        };
        Point2::new(w(uv.x, du, self.closed.0), w(uv.y, dv, self.closed.1))
    }

    fn point_at(&self, uv: Point2) -> Point3 {
        self.eval_all(uv).p
    }

    fn eval_all(&self, uv: Point2) -> SurfDeriv {
        let uv = self.wrap(uv);
        let (su, sv) = (
            span_of(&self.u, self.p, self.nu, uv.x),
            span_of(&self.v, self.q, self.nv, uv.y),
        );
        let (mut bu, mut bv) = ([[0.0; MAX_DEGREE + 1]; 3], [[0.0; MAX_DEGREE + 1]; 3]);
        basis(&self.u, self.p, su, uv.x, &mut bu);
        basis(&self.v, self.q, sv, uv.y, &mut bv);
        // homogeneous value and derivatives, A with weight w
        let mut s = [[[0.0_f64; 4]; 3]; 3];
        for i in 0..=self.p {
            let row = (su - self.p + i) * self.nv;
            for j in 0..=self.q {
                let c = self.net[row + sv - self.q + j];
                for (du, b) in bu.iter().enumerate() {
                    for (dv, a) in bv.iter().enumerate() {
                        if du + dv > 2 {
                            continue;
                        }
                        let k = b[i] * a[j];
                        for x in 0..4 {
                            s[du][dv][x] += k * c[x];
                        }
                    }
                }
            }
        }
        let v3 = |a: [f64; 4]| Vec3::new(a[0], a[1], a[2]);
        let w = s[0][0][3];
        let c = v3(s[0][0]) / w;
        let (wu, wv) = (s[1][0][3], s[0][1][3]);
        let du = (v3(s[1][0]) - c * wu) / w;
        let dv = (v3(s[0][1]) - c * wv) / w;
        let duu = (v3(s[2][0]) - du * (2.0 * wu) - c * s[2][0][3]) / w;
        let duv = (v3(s[1][1]) - du * wv - dv * wu - c * s[1][1][3]) / w;
        let dvv = (v3(s[0][2]) - dv * (2.0 * wv) - c * s[0][2][3]) / w;
        SurfDeriv {
            p: Point3::ORIGIN + c,
            du,
            dv,
            duu,
            duv,
            dvv,
        }
    }

    fn seeds(&self) -> &Seeds {
        self.seeds.get_or_init(|| {
            let (du, dv) = self.dom();
            // about two points a control point's span, within reason
            let grid = |n: usize, p: usize| (2 * (n - p) + 1).clamp(9, 65);
            let (a, b) = (grid(self.nu, self.p), grid(self.nv, self.q));
            let (mut radius, mut lu, mut lv) = (f64::INFINITY, 0.0_f64, 0.0_f64);
            for i in 0..a {
                let u = du.lo + du.width() * i as f64 / (a - 1) as f64;
                for j in 0..b {
                    let v = dv.lo + dv.width() * j as f64 / (b - 1) as f64;
                    let uv = Point2::new(u, v);
                    let d = self.eval_all(uv);
                    let (n, area) = {
                        let c = d.du.cross(d.dv);
                        let l = c.length();
                        (if l > 0.0 { c / l } else { Vec3::ZERO }, l)
                    };
                    lu = lu.max(d.du.length());
                    lv = lv.max(d.dv.length());
                    if area > 0.0 {
                        // normal curvature along each parameter line
                        let (e, g) = (d.du.dot(d.du), d.dv.dot(d.dv));
                        let k = (n.dot(d.duu).abs() / e.max(1e-300)).max(n.dot(d.dvv).abs() / g.max(1e-300));
                        if k > 0.0 {
                            radius = radius.min(1.0 / k);
                        }
                    }
                }
            }
            Seeds {
                radius: radius.max(forge_math::tol::LINEAR),
                scale: (lu.max(forge_math::tol::LINEAR), lv.max(forge_math::tol::LINEAR)),
            }
        })
    }

    /// Newton on the distance from `start`, kept in the domain.
    fn newton(&self, p: Point3, start: Point2) -> (Point2, f64) {
        let mut uv = self.wrap(start);
        let mut d = self.eval_all(uv);
        let mut best = (uv, d.p.distance_sq(p));
        for _ in 0..12 {
            let r = d.p - p;
            let (f, g) = (r.dot(d.du), r.dot(d.dv));
            let (a, b, c) = (
                d.du.dot(d.du) + r.dot(d.duu),
                d.du.dot(d.dv) + r.dot(d.duv),
                d.dv.dot(d.dv) + r.dot(d.dvv),
            );
            let det = a * c - b * b;
            let (mut x, mut y) = if det > 1e-14 * (a * c).abs() && a > 0.0 {
                ((c * f - b * g) / det, (a * g - b * f) / det)
            } else {
                // Gauss-Newton's diagonal where the Hessian is not positive
                let (e, h) = (d.du.dot(d.du).max(1e-300), d.dv.dot(d.dv).max(1e-300));
                (f / e, g / h)
            };
            // never step past a third of the domain
            let (du, dv) = self.dom();
            x = x.clamp(-du.width() / 3.0, du.width() / 3.0);
            y = y.clamp(-dv.width() / 3.0, dv.width() / 3.0);
            let next = self.wrap(Point2::new(uv.x - x, uv.y - y));
            let moved = (next.x - uv.x).abs() / du.width() + (next.y - uv.y).abs() / dv.width();
            uv = next;
            d = self.eval_all(uv);
            let q = d.p.distance_sq(p);
            if q < best.1 {
                best = (uv, q);
            }
            if moved < 1e-13 {
                break;
            }
        }
        best
    }

    fn tree(&self) -> &Tree {
        self.tree.get_or_init(|| {
            // the non-empty knot spans of a direction: (lo, hi, span index)
            let spans = |k: &[f64], d: usize, n: usize| -> Vec<(f64, f64, usize)> {
                (d..n)
                    .filter(|&i| k[i + 1] > k[i])
                    .map(|i| (k[i], k[i + 1], i))
                    .collect()
            };
            let (us, vs) = (spans(&self.u, self.p, self.nu), spans(&self.v, self.q, self.nv));
            let mut nodes = Vec::with_capacity(2 * us.len() * vs.len());
            self.build(&us, &vs, (0, us.len()), (0, vs.len()), &mut nodes);
            Tree { nodes, us, vs }
        })
    }

    /// The node over cells [a, b) x [c, d): its children first, so its box is their hull.
    fn build(
        &self,
        us: &[(f64, f64, usize)],
        vs: &[(f64, f64, usize)],
        (a, b): (usize, usize),
        (c, d): (usize, usize),
        out: &mut Vec<(Box3, u32, u32, (u32, u32))>,
    ) -> u32 {
        let (l, r, leaf, bx);
        if b - a == 1 && d - c == 1 {
            let (si, sj) = (us[a].2, vs[c].2);
            let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
            for i in si - self.p..=si {
                for j in sj - self.q..=sj {
                    let q = self.net[i * self.nv + j];
                    for x in 0..3 {
                        let v = q[x] / q[3];
                        lo[x] = lo[x].min(v);
                        hi[x] = hi[x].max(v);
                    }
                }
            }
            bx = Box3 {
                x: Interval::new(lo[0], hi[0]),
                y: Interval::new(lo[1], hi[1]),
                z: Interval::new(lo[2], hi[2]),
            };
            (l, r, leaf) = (0, 0, (a as u32, c as u32));
        } else {
            let (x, y) = if b - a >= d - c {
                let m = a + (b - a) / 2;
                (
                    self.build(us, vs, (a, m), (c, d), out),
                    self.build(us, vs, (m, b), (c, d), out),
                )
            } else {
                let m = c + (d - c) / 2;
                (
                    self.build(us, vs, (a, b), (c, m), out),
                    self.build(us, vs, (a, b), (m, d), out),
                )
            };
            bx = out[x as usize].0.hull(&out[y as usize].0);
            (l, r, leaf) = (x, y, (0, 0));
        }
        out.push((bx, l, r, leaf));
        out.len() as u32 - 1
    }

    /// The nearest point of the whole surface: cells in order of their boxes' distance, Newton
    /// from each middle, until no box left can beat what was found.
    fn nearest(&self, p: Point3) -> (Point2, f64) {
        let t = self.tree();
        let gap = |b: &Box3| {
            let d = |i: Interval, x: f64| (i.lo - x).max(x - i.hi).max(0.0);
            let (a, c, e) = (d(b.x, p.x), d(b.y, p.y), d(b.z, p.z));
            a * a + c * c + e * e
        };
        let mut best = (Point2::new(t.us[0].0, t.vs[0].0), f64::INFINITY);
        let mut stack: Vec<(f64, u32)> = Vec::with_capacity(32);
        stack.push((0.0, t.nodes.len() as u32 - 1));
        // Newton's cost bounds the search on a net all at one place
        let mut tries = 0;
        while let Some((g, n)) = stack.pop() {
            if g >= best.1 || tries >= 64 {
                continue;
            }
            if best.1 <= 1e-20 {
                break;
            }
            let (b, l, r, leaf) = &t.nodes[n as usize];
            let _ = b;
            if *l == 0 && *r == 0 {
                let (i, j) = (leaf.0 as usize, leaf.1 as usize);
                let start = Point2::new(0.5 * (t.us[i].0 + t.us[i].1), 0.5 * (t.vs[j].0 + t.vs[j].1));
                let found = self.newton(p, start);
                tries += 1;
                if found.1 < best.1 {
                    best = found;
                }
            } else {
                let (gl, gr) = (gap(&t.nodes[*l as usize].0), gap(&t.nodes[*r as usize].0));
                // the nearer box on top
                if gl <= gr {
                    stack.push((gr, *r));
                    stack.push((gl, *l));
                } else {
                    stack.push((gl, *l));
                    stack.push((gr, *r));
                }
            }
        }
        best
    }
}

impl Procedural for Nurbs {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.eval_all(uv)
    }
    fn domain(&self) -> Domain2 {
        let (du, dv) = self.dom();
        let d = |i: Interval, c: bool| Domain1 {
            lo: i.lo,
            hi: i.hi,
            period: c.then(|| i.width()),
        };
        Domain2 {
            u: d(du, self.closed.0),
            v: d(dv, self.closed.1),
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        self.nearest(p).0
    }
    fn project_near(&self, p: Point3, near: Point2) -> Point2 {
        let (uv, d) = self.newton(p, near);
        // a poor walk from there: the search over the whole surface (a point a file left beside
        // the surface, by up to its own tolerance, is a fair walk)
        if d > 1e-4 {
            let (w, e) = self.nearest(p);
            if e < d {
                return w;
            }
        }
        uv
    }
    /// The hull of the control points live over the box, which holds the surface where the
    /// weights are positive (as `new` makes sure).
    fn bbox(&self, uv: Box2) -> Box3 {
        let span = |k: &[f64], d: usize, n: usize, t: Interval, closed: bool| -> (usize, usize) {
            let (dl, dh) = (k[d], k[n]);
            if closed && (t.lo < dl || t.hi > dh) {
                return (0, n - 1);
            }
            (span_of(k, d, n, t.lo.max(dl)) - d, span_of(k, d, n, t.hi.min(dh)))
        };
        let (a0, a1) = span(&self.u, self.p, self.nu, uv.u, self.closed.0);
        let (b0, b1) = span(&self.v, self.q, self.nv, uv.v, self.closed.1);
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for i in a0..=a1.min(self.nu - 1) {
            for j in b0..=b1.min(self.nv - 1) {
                let c = self.net[i * self.nv + j];
                for x in 0..3 {
                    let v = c[x] / c[3];
                    lo[x] = lo[x].min(v);
                    hi[x] = hi[x].max(v);
                }
            }
        }
        Box3 {
            x: Interval::new(lo[0], hi[0]),
            y: Interval::new(lo[1], hi[1]),
            z: Interval::new(lo[2], hi[2]),
        }
    }
    fn implicit(&self, p: Point3) -> f64 {
        let (uv, d2) = self.nearest(p);
        let q = self.eval_all(uv);
        let n = q.du.cross(q.dv);
        let side = n.dot(p - q.p);
        let d = d2.sqrt();
        if side < 0.0 { -d } else { d }
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        let (uv, _) = self.nearest(p);
        let q = self.eval_all(uv);
        let n = q.du.cross(q.dv);
        n.normalize().map_or(Vec3::ZERO, Vec3::from)
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let net = self
            .net
            .iter()
            .map(|c| {
                let q = f.point_to_world(Point3::new(c[0] / c[3], c[1] / c[3], c[2] / c[3]));
                [q.x * c[3], q.y * c[3], q.z * c[3], c[3]]
            })
            .collect();
        let mut n = self.clone();
        n.net = Arc::new(net);
        n.seeds = Arc::new(OnceLock::new());
        n.tree = Arc::new(OnceLock::new());
        Proc(Arc::new(n))
    }
    fn uv_scale(&self) -> (f64, f64) {
        self.seeds().scale
    }
    fn mesh_radius(&self) -> f64 {
        self.seeds().radius
    }
    fn exact_d1(&self) -> bool {
        true
    }
    fn u_kinks(&self, u: Interval) -> Vec<f64> {
        let mut k: Vec<f64> = self.u[self.p + 1..self.nu]
            .iter()
            .copied()
            .filter(|&x| x > u.lo && x < u.hi)
            .collect();
        k.dedup();
        k
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::SurfaceGeom;

    /// A bicubic patch with knots inside and a weight off one, and a sphere octant's rational net.
    fn patch() -> Nurbs {
        let mut pts = Vec::new();
        let mut w = Vec::new();
        for i in 0..5 {
            for j in 0..4 {
                let (x, y) = (i as f64 * 2.0, j as f64 * 3.0);
                pts.push(Point3::new(x, y, ((i * j) % 3) as f64 + 0.3 * x));
                w.push(if i == 2 && j == 1 { 2.5 } else { 1.0 });
            }
        }
        Nurbs::new(
            (3, 2),
            (
                vec![0., 0., 0., 0., 0.4, 1., 1., 1., 1.],
                vec![0., 0., 0., 0.5, 2., 2., 2.],
            ),
            (5, 4),
            &pts,
            Some(&w),
        )
        .unwrap()
    }

    #[test]
    fn derivatives_match_differences() {
        let n = patch();
        for &(u, v) in &[(0.2, 0.3), (0.7, 1.4), (0.41, 0.51), (0.99, 1.9)] {
            let h = 1e-5;
            let d = n.eval_all(Point2::new(u, v));
            let g = |a: f64, b: f64| n.eval_all(Point2::new(a, b));
            let du = (g(u + h, v).p - g(u - h, v).p) / (2.0 * h);
            let dv = (g(u, v + h).p - g(u, v - h).p) / (2.0 * h);
            let duu = (g(u + h, v).du - g(u - h, v).du) / (2.0 * h);
            let duv = (g(u + h, v).dv - g(u - h, v).dv) / (2.0 * h);
            let dvv = (g(u, v + h).dv - g(u, v - h).dv) / (2.0 * h);
            assert!((d.du - du).length() < 1e-6, "{u} {v}");
            assert!((d.dv - dv).length() < 1e-6, "{u} {v}");
            assert!((d.duu - duu).length() < 1e-4, "{u} {v}");
            assert!((d.duv - duv).length() < 1e-4, "{u} {v}");
            assert!((d.dvv - dvv).length() < 1e-4, "{u} {v}");
        }
    }

    #[test]
    fn project_inverts_eval_and_boxes_hold() {
        let n = patch();
        let s = n.clone().surface();
        for &(u, v) in &[(0.2, 0.3), (0.7, 1.4), (0.0, 0.0), (1.0, 2.0)] {
            let q = s.eval(Point2::new(u, v));
            assert!(s.distance(q) < 1e-9);
            let w = Interval::new(u - 0.1, u + 0.1);
            let b = s.bbox(Box2 {
                u: w,
                v: Interval::new(v - 0.1, v + 0.1),
            });
            assert!(
                q.x >= b.x.lo - 1e-12
                    && q.x <= b.x.hi + 1e-12
                    && q.z >= b.z.lo - 1e-12
                    && q.z <= b.z.hi + 1e-12
            );
        }
    }

    #[test]
    fn a_bad_net_is_refused() {
        let pts = vec![Point3::new(0., 0., 0.); 4];
        let knots = (vec![0., 0., 1., 1.], vec![0., 0., 1., 1.]);
        assert!(Nurbs::new((1, 1), knots.clone(), (2, 2), &pts, None).is_some());
        assert!(Nurbs::new((1, 1), knots.clone(), (2, 2), &pts[..3], None).is_none());
        assert!(Nurbs::new((1, 1), knots.clone(), (2, 2), &pts, Some(&[1., 1., 0., 1.])).is_none());
        assert!(Nurbs::new((1, 1), (vec![0., 1., 0., 1.], knots.1), (2, 2), &pts, None).is_none());
    }
}
