//! Mass properties from the exact B-rep: volume, area, centroid and inertia (unit density).
//!
//! Each volume integral is a flux through the boundary (divergence theorem), and each face's
//! flux an integral over its trimmed parameter domain D of F(u, v) = Phi(S) . (Su x Sv). Green's
//! theorem turns that into an integral round D's loops: with G(a, b) the integral of F along the
//! inner parameter a from a fixed a0, the face integral is the loop integral of G db.
//!
//! G is exact for the analytic surfaces: F is a polynomial (Gauss-Legendre) or a trigonometric
//! polynomial of known degree along a, fitted once per face. On a procedural surface F is fitted
//! by Chebyshev tiles where it is smooth enough, else integrated by adaptive Gauss-Kronrod.
//!
//! The loop integrals run along the edges. An analytic curve on an analytic surface, or an
//! intersection curve walked by the distance along its chord (its own parameter is only once
//! differentiable at its chain points), gives uv' exactly: Gauss-Kronrod 7-15 there, the two
//! rules' difference the error. Elsewhere uv' comes from the interpolant of the uv samples on
//! nested Fejer rules (4, 12, 36 points), the last Chebyshev coefficients the error. Pieces are
//! halved until the error is below the tolerance, within a budget per face; what is left over
//! is in the reported error, as is any gap between an edge and its face's surface.

use crate::body::{Body, CoedgeId, FaceId, LoopId};
use forge_geom::intcurve::IntCurve;
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Point2, Point3, Vec3, dmath};
use std::rc::Rc;
use std::sync::OnceLock;

/// The integrands: volume, first moments (x, y, z), second moments (xx, yy, zz, xy, yz, zx),
/// area.
const NF: usize = 11;
type F = [f64; NF];

/// Default relative accuracy.
pub const REL: f64 = 1e-9;

#[derive(Clone, Debug, PartialEq)]
pub struct MassProps {
    pub volume: f64,
    pub area: f64,
    pub centroid: Point3,
    /// The inertia tensor about the centroid, unit density.
    pub inertia: [[f64; 3]; 3],
    /// Principal moments, smallest first, and their axes.
    pub principal: [f64; 3],
    pub axes: [Vec3; 3],
    /// Error bounds: volume, area, centroid (distance), inertia (largest entry).
    pub err: [f64; 4],
    /// Each face's area.
    pub faces: Vec<(FaceId, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MassError {
    pub face: FaceId,
    pub why: &'static str,
}

impl std::fmt::Display for MassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "face {}: {}", self.face.0, self.why)
    }
}

pub fn mass(body: &Body) -> Result<MassProps, MassError> {
    mass_rel(body, REL)
}

pub fn mass_rel(body: &Body, rel: f64) -> Result<MassProps, MassError> {
    // moments about a point inside the part keep the cancellation small
    let n = body.vertices.len().max(1) as f64;
    let c = body
        .vertices
        .iter()
        .fold(Point3::ORIGIN, |a, v| a + (v.point - Point3::ORIGIN) * (1.0 / n));
    let size = body
        .face_ids()
        .fold(forge_math::Box3::EMPTY, |b, f| b.hull(&body.face_box(f)))
        .diagonal();
    let (mut t, mut e) = ([0.0; NF], [0.0; NF]);
    let mut faces = Vec::new();
    for f in body.face_ids() {
        let (v, er) = face_integrals(body, f, c, size, rel).map_err(|why| MassError { face: f, why })?;
        for k in 0..NF {
            t[k] += v[k];
            e[k] += er[k];
        }
        faces.push((f, v[10]));
    }
    let vol = t[0];
    let g = Vec3::new(t[1], t[2], t[3]) / vol;
    // second moments about the centroid
    let s = |i: usize, j: usize| {
        let m = match (i.min(j), i.max(j)) {
            (0, 0) => t[4],
            (1, 1) => t[5],
            (2, 2) => t[6],
            (0, 1) => t[7],
            (1, 2) => t[8],
            _ => t[9],
        };
        m - vol * g[i] * g[j]
    };
    let tr = s(0, 0) + s(1, 1) + s(2, 2);
    let inertia: [[f64; 3]; 3] =
        std::array::from_fn(|i| std::array::from_fn(|j| if i == j { tr - s(i, i) } else { -s(i, j) }));
    let (principal, axes) = eigen(inertia);
    let gl = g.length();
    let em = e[1].abs().max(e[2]).max(e[3]);
    let es = e[4..10].iter().fold(0.0f64, |a, &x| a.max(x));
    Ok(MassProps {
        volume: vol,
        area: t[10],
        centroid: c + g,
        inertia,
        principal,
        axes,
        err: [
            e[0],
            e[10],
            (em + gl * e[0]) / vol.abs(),
            2.0 * (es + 2.0 * gl * em + gl * gl * e[0]),
        ],
        faces,
    })
}

fn integrand(x: Vec3, n: Vec3) -> F {
    let (a, b, c) = (x.x, x.y, x.z);
    [
        x.dot(n) / 3.0,
        a * a * n.x / 2.0,
        b * b * n.y / 2.0,
        c * c * n.z / 2.0,
        a * a * a * n.x / 3.0,
        b * b * b * n.y / 3.0,
        c * c * c * n.z / 3.0,
        a * a * b * n.x / 2.0,
        b * b * c * n.y / 2.0,
        c * c * a * n.z / 2.0,
        n.length(),
    ]
}

fn axpy(acc: &mut F, s: f64, x: &F) {
    for k in 0..NF {
        acc[k] += s * x[k];
    }
}

/// Fejer's first rule on n Chebyshev points (increasing, on [-1, 1]) and its differentiation
/// matrix.
struct Fejer {
    x: Vec<f64>,
    w: Vec<f64>,
    lam: Vec<f64>,
    d: Vec<f64>,
    /// The last two Chebyshev coefficients of the interpolant, as weights on the values.
    tail: [Vec<f64>; 2],
}

fn fejer(n: usize) -> Fejer {
    let th = |k: usize| (2 * k + 1) as f64 * dmath::PI / (2 * n) as f64;
    let x: Vec<f64> = (0..n).map(|k| -dmath::cos(th(k))).collect();
    let w = (0..n)
        .map(|k| {
            let s: f64 = (1..=n / 2)
                .map(|j| dmath::cos(2.0 * j as f64 * th(k)) / (4.0 * (j * j) as f64 - 1.0))
                .sum();
            2.0 / n as f64 * (1.0 - 2.0 * s)
        })
        .collect();
    let lam: Vec<f64> = (0..n)
        .map(|k| if k % 2 == 0 { 1.0 } else { -1.0 } * dmath::sin(th(k)))
        .collect();
    let mut d = vec![0.0; n * n];
    for i in 0..n {
        let mut s = 0.0;
        for j in 0..n {
            if i != j {
                let v = lam[j] / lam[i] / (x[i] - x[j]);
                d[i * n + j] = v;
                s += v;
            }
        }
        d[i * n + i] = -s;
    }
    let tail = [n - 2, n - 1].map(|k| {
        (0..n)
            .map(|j| 2.0 / n as f64 * dmath::cos((k * (2 * j + 1)) as f64 * dmath::PI / (2 * n) as f64))
            .collect()
    });
    Fejer { x, w, lam, d, tail }
}

/// The 4-, 12- and 36-point rules: each one's points are the next's with index 1 mod 3.
fn rule(n: usize) -> &'static Fejer {
    static R: OnceLock<[Fejer; 3]> = OnceLock::new();
    let r = R.get_or_init(|| [fejer(4), fejer(12), fejer(36)]);
    &r[match n {
        4 => 0,
        12 => 1,
        _ => 2,
    }]
}

#[derive(Clone, Copy, Default)]
struct Node {
    uv: [f64; 2],
    /// d uv / dx on the piece's rule; x in [-1, 1].
    d: [f64; 2],
    p: Point3,
    speed: f64,
    /// Distance from the edge point to the surface point drawn for it.
    gap: f64,
}

/// One stretch [lo, hi] of an edge, sampled on the 12 or 36 points, in the order its loop
/// walks it.
struct Piece<'a> {
    n: Vec<Node>,
    /// -1 where the loop walks the edge backwards (the derivatives are along the edge).
    sign: f64,
    way: Rc<Way<'a>>,
    lo: f64,
    hi: f64,
    /// Gauss-Kronrod (derivatives from the curve's own) rather than Fejer (derivatives from
    /// the interpolant).
    kronrod: bool,
}

/// Gauss-Kronrod rules on 7 and 15 points (increasing x): weights, and the embedded Gauss
/// rule's weights (zero off its points).
struct Kronrod {
    x: Vec<f64>,
    w: Vec<f64>,
    g: Vec<f64>,
}

fn kronrod(n: usize) -> &'static Kronrod {
    static R: OnceLock<[Kronrod; 2]> = OnceLock::new();
    let r = R.get_or_init(|| {
        let make = |x: &[f64], w: &[f64], g: &[f64]| {
            let h = x.len() - 1;
            let idx: Vec<usize> = (0..=h).chain((0..h).rev()).collect();
            Kronrod {
                x: idx
                    .iter()
                    .enumerate()
                    .map(|(i, &k)| if i <= h { -x[k] } else { x[k] })
                    .collect(),
                w: idx.iter().map(|&k| w[k]).collect(),
                g: idx.iter().map(|&k| g[k]).collect(),
            }
        };
        let g15: Vec<f64> = (0..8)
            .map(|j| if j % 2 == 1 || j == 7 { WG[j / 2] } else { 0.0 })
            .collect();
        [
            make(
                &[
                    0.960_491_268_708_020_3,
                    0.774_596_669_241_483_4,
                    0.434_243_749_346_802_6,
                    0.0,
                ],
                &[
                    0.104_656_226_026_467_27,
                    0.268_488_089_868_333_44,
                    0.401_397_414_775_962_2,
                    0.450_916_538_658_474_1,
                ],
                &[0.0, 5.0 / 9.0, 0.0, 8.0 / 9.0],
            ),
            make(&XGK, &WGK, &g15),
        ]
    });
    &r[usize::from(n == 15)]
}

/// A piece of an analytic curve on an analytic surface, on Gauss-Kronrod points: uv' from the
/// curve's exact derivative. None where the surface's parameters are singular.
fn kronrod_piece<'a>(s: &Surface, way: &Rc<Way<'a>>, lo: f64, hi: f64, n: usize) -> Option<Piece<'a>> {
    let per = periods(s);
    let (mid, half) = (0.5 * (lo + hi), 0.5 * (hi - lo));
    let mut nodes: Vec<Node> = Vec::with_capacity(n);
    for &x in &kronrod(n).x {
        let (cp, cd1) = way.deriv(mid + half * x);
        let q = s.project(cp);
        let mut uv = [q.x, q.y];
        if let Some(prev) = nodes.last() {
            for a in 0..2 {
                uv[a] = near(uv[a], prev.uv[a], per[a]);
            }
        }
        let sd = s.deriv(Point2::new(uv[0], uv[1]));
        let (e, f, g) = (sd.du.dot(sd.du), sd.du.dot(sd.dv), sd.dv.dot(sd.dv));
        let (du, dv) = forge_math::linalg::solve2(e, f, f, g, sd.du.dot(cd1), sd.dv.dot(cd1))?;
        nodes.push(Node {
            uv,
            d: [du * half, dv * half],
            p: cp,
            speed: cd1.length() * half,
            gap: sd.p.distance(cp),
        });
    }
    Some(Piece {
        n: nodes,
        sign: 1.0,
        way: way.clone(),
        lo,
        hi,
        kronrod: true,
    })
}

/// Whether the way's own derivative is exact and the surface's too, so uv' follows from them.
fn exact(s: &Surface, way: &Way) -> bool {
    let analytic = |s: &Surface| !matches!(s, Surface::Procedural(_));
    analytic(s)
        && match way {
            Way::Curve(c) => !matches!(c, Curve::Procedural(_)),
            // the tangent is exact where both gradients are
            Way::Chord { c, .. } => analytic(&c.a) && analytic(&c.b),
        }
}

/// A way to walk an edge with a smooth parameter.
enum Way<'a> {
    Curve(&'a Curve),
    /// An intersection curve's own parameter only joins smoothly at its chain points; this one
    /// is the distance along a fixed direction, the point solved exactly onto both surfaces.
    Chord {
        c: &'a IntCurve,
        anchor: Point3,
        dir: Vec3,
        knots: Vec<(f64, f64)>,
    },
}

impl Way<'_> {
    fn at(&self, s: f64) -> Point3 {
        match self {
            Way::Curve(c) => c.eval(s),
            Way::Chord { .. } => self.deriv(s).0,
        }
    }

    /// The point and its derivative.
    fn deriv(&self, s: f64) -> (Point3, Vec3) {
        match self {
            Way::Curve(c) => {
                let d = c.deriv(s);
                (d.p, d.d1)
            }
            Way::Chord {
                c,
                anchor,
                dir,
                knots,
            } => {
                let i = knots.partition_point(|k| k.1 <= s).clamp(1, knots.len() - 1);
                let ((t0, s0), (t1, s1)) = (knots[i - 1], knots[i]);
                let mut p = c.eval(t0 + (t1 - t0) * ((s - s0) / (s1 - s0)));
                let tight = 4.0 * f64::EPSILON * (1.0 + (p - Point3::ORIGIN).abs_max());
                for _ in 0..8 {
                    let ((fa, ga), (fb, gb)) = (c.a.implicit_gradient(p), c.b.implicit_gradient(p));
                    let fs = dir.dot(p - *anchor) - s;
                    if fa.abs().max(fb.abs()).max(fs.abs()) <= tight {
                        break;
                    }
                    // Cramer on rows ga, gb, dir: the step solves ga.d = -fa, gb.d = -fb, dir.d = -fs
                    let (bc, ca, ab) = (gb.cross(*dir), dir.cross(ga), ga.cross(gb));
                    let det = ga.dot(bc);
                    if det.abs() <= 1e-14 {
                        break;
                    }
                    p += (bc * -fa + ca * -fb + ab * -fs) / det;
                }
                // along the curve, at unit speed along dir
                let t = c.a.gradient(p).cross(c.b.gradient(p));
                (p, t / t.dot(*dir))
            }
        }
    }
}

fn near(x: f64, r: f64, period: Option<f64>) -> f64 {
    match period {
        Some(p) => x + p * ((r - x) / p).round(),
        None => x,
    }
}

fn periods(s: &Surface) -> [Option<f64>; 2] {
    let d = s.domain();
    [d.u.period, d.v.period]
}

/// The edge points of a way at the rule's points on [lo, hi], with the surface's uv of each.
fn points(s: &Surface, way: &Way, lo: f64, hi: f64, xs: impl Iterator<Item = f64>) -> Vec<Node> {
    let (mid, half) = (0.5 * (lo + hi), 0.5 * (hi - lo));
    let mut prev: Option<Point2> = None;
    xs.map(|x| {
        let p = way.at(mid + half * x);
        let (q, gap) = foot(s, p, prev);
        prev = Some(q);
        Node {
            uv: [q.x, q.y],
            p,
            gap,
            ..Node::default()
        }
    })
    .collect()
}

/// Where `p` (on or by the surface) lies on it, and how far off it is. A procedural surface's
/// own search is costly and stops short of full precision: Newton steps from a nearby point's
/// parameters (or from the search's answer) go straight there.
fn foot(s: &Surface, p: Point3, near: Option<Point2>) -> (Point2, f64) {
    let Surface::Procedural(_) = s else {
        let q = s.project(p);
        return (q, s.eval(q).distance(p));
    };
    let newton = |mut uv: Point2| -> Option<(Point2, f64)> {
        for _ in 0..12 {
            let d = s.deriv(uv);
            let r = d.p - p;
            let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
            let (x, y) = forge_math::linalg::solve2(e, f, f, g, -r.dot(d.du), -r.dot(d.dv))?;
            uv = Point2::new(uv.x + x, uv.y + y);
            if (d.du * x + d.dv * y).length() <= 1e-15 * (1.0 + (p - Point3::ORIGIN).abs_max()) {
                let gap = s.eval(uv).distance(p);
                return (gap <= 10.0 * forge_math::tol::LINEAR).then_some((uv, gap));
            }
        }
        None
    };
    near.and_then(newton).unwrap_or_else(|| {
        let q = s.project(p);
        newton(q).unwrap_or_else(|| (q, s.eval(q).distance(p)))
    })
}

/// A piece from its nodes in increasing parameter: unwrapped continuously, with derivatives.
fn finish<'a>(mut n: Vec<Node>, per: [Option<f64>; 2], way: &Rc<Way<'a>>, lo: f64, hi: f64) -> Piece<'a> {
    let m = n.len();
    for k in 1..m {
        let (prev, uv) = (n[k - 1].uv, n[k].uv);
        n[k].uv = std::array::from_fn(|a| near(uv[a], prev[a], per[a]));
    }
    let r = rule(m);
    let (u0, p0) = (n[0].uv, n[0].p);
    // the speed only weighs the gap between edge and surface
    let off = n
        .iter()
        .any(|q| q.gap > 1e-13 * (1.0 + (q.p - Point3::ORIGIN).abs_max()));
    for i in 0..m {
        let row = &r.d[i * m..(i + 1) * m];
        let mut d = [0.0; 2];
        for (w, q) in row.iter().zip(&n) {
            d[0] += w * (q.uv[0] - u0[0]);
            d[1] += w * (q.uv[1] - u0[1]);
        }
        n[i].d = d;
        if off {
            let dp = row
                .iter()
                .zip(&n)
                .fold(Vec3::ZERO, |acc, (w, q)| acc + (q.p - p0) * *w);
            n[i].speed = dp.length();
        }
    }
    Piece {
        n,
        sign: 1.0,
        way: way.clone(),
        lo,
        hi,
        kronrod: false,
    }
}

/// The piece on 36 points from 12, or from 36 its halves on 12, walked as it was and unwrapped
/// to follow on from where it started.
fn finer<'a>(s: &Surface, pc: &Piece<'a>) -> Vec<Piece<'a>> {
    let per = periods(s);
    let (way, lo, hi) = (&pc.way, pc.lo, pc.hi);
    let mut old = pc.n.clone();
    if pc.sign < 0.0 {
        old.reverse();
    }
    let mut out = if pc.kronrod {
        let k15 = |a: f64, b: f64| {
            kronrod_piece(s, way, a, b, 15)
                .unwrap_or_else(|| finish(points(s, way, a, b, rule(12).x.iter().copied()), per, way, a, b))
        };
        if old.len() == 7 {
            vec![k15(lo, hi)]
        } else {
            let mid = 0.5 * (lo + hi);
            vec![k15(lo, mid), k15(mid, hi)]
        }
    } else if old.len() == 12 {
        let x36 = &rule(36).x;
        let mut rest = points(s, way, lo, hi, (0..36).filter(|j| j % 3 != 1).map(|j| x36[j])).into_iter();
        let n36 = (0..36)
            .map(|j| {
                if j % 3 == 1 {
                    old[j / 3]
                } else {
                    rest.next().expect("24 more")
                }
            })
            .collect();
        vec![finish(n36, per, way, lo, hi)]
    } else {
        let mid = 0.5 * (lo + hi);
        [(lo, mid), (mid, hi)]
            .map(|(a, b)| finish(points(s, way, a, b, rule(12).x.iter().copied()), per, way, a, b))
            .into()
    };
    if pc.sign < 0.0 {
        out.reverse();
        for p in &mut out {
            p.n.reverse();
            p.sign = -1.0;
        }
    }
    let mut prev = pc.n[0].uv;
    for p in &mut out {
        for a in 0..2 {
            let d = near(p.n[0].uv[a], prev[a], per[a]) - p.n[0].uv[a];
            p.n.iter_mut().for_each(|q| q.uv[a] += d);
        }
        prev = p.n[p.n.len() - 1].uv;
    }
    out
}

/// Whether the coarser rule's interpolant of the trace matches it at the piece's other points.
fn resolved(pc: &Piece, rel: f64) -> bool {
    let m = pc.n.len();
    let (r, rc) = (rule(m), rule(m / 3));
    let coords =
        pc.n.iter()
            .fold(0.0f64, |a, q| a.max((q.p - Point3::ORIGIN).abs_max()));
    (0..2).all(|a| {
        let (lo, hi, big) =
            pc.n.iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY, 0.0f64), |b, q| {
                    (b.0.min(q.uv[a]), b.1.max(q.uv[a]), b.2.max(q.uv[a].abs()))
                });
        // rounding: the uv of a point carries the error of its coordinates
        let tol = 1e-3 * rel * (hi - lo) + 64.0 * f64::EPSILON * (big + coords);
        (0..m).filter(|j| j % 3 != 1).all(|j| {
            let x = r.x[j];
            let (mut num, mut den) = (0.0, 0.0);
            for k in 0..m / 3 {
                let c = rc.lam[k] / (x - rc.x[k]);
                num += c * pc.n[3 * k + 1].uv[a];
                den += c;
            }
            (num / den - pc.n[j].uv[a]).abs() <= tol
        })
    })
}

/// [lo, hi] on 12 points if 4 already draw it, else on 36 if 12 do, else in halves.
fn refine<'a>(
    s: &Surface,
    way: &Rc<Way<'a>>,
    lo: f64,
    hi: f64,
    rel: f64,
    depth: u32,
    out: &mut Vec<Piece<'a>>,
) {
    if exact(s, way)
        && let Some(pc) = kronrod_piece(
            s,
            way,
            lo,
            hi,
            if matches!(**way, Way::Curve(Curve::Line(_))) {
                7
            } else {
                15
            },
        )
    {
        out.push(pc);
        return;
    }
    let pc = finish(
        points(s, way, lo, hi, rule(12).x.iter().copied()),
        periods(s),
        way,
        lo,
        hi,
    );
    if resolved(&pc, rel) {
        out.push(pc);
        return;
    }
    let pc = finer(s, &pc).pop().expect("one piece");
    if depth == 0 || resolved(&pc, rel) {
        out.push(pc);
        return;
    }
    let mid = 0.5 * (lo + hi);
    refine(s, way, lo, mid, rel, depth - 1, out);
    refine(s, way, mid, hi, rel, depth - 1, out);
}

/// A coedge's pieces in the order its loop walks them.
fn coedge_pieces<'a>(
    body: &'a Body,
    s: &Surface,
    c: CoedgeId,
    rel: f64,
) -> Result<Vec<Piece<'a>>, &'static str> {
    let co = body.coedge(c);
    let e = body.edge(co.edge);
    let (lo, hi) = (e.range.lo, e.range.hi);
    let mut out = Vec::new();
    match &e.curve {
        Curve::Intersection(ic) => {
            let n = ic.pts.len() as i64;
            let idx = |t: f64| {
                let k = t.floor() as i64;
                (if ic.closed {
                    k.rem_euclid(n)
                } else {
                    k.clamp(0, n - 1)
                }) as usize
            };
            let mut t0 = lo;
            while t0 < hi {
                // pieces over which the curve turns less than 30 degrees: the distance along
                // their chord then runs one way
                let tan0 = ic.tans[idx(t0)];
                // the next chain point, or the end when that is a hair away
                let next = |t: f64| {
                    let k = (t + 1e-9).floor() + 1.0;
                    if hi - k <= 1e-9 { hi } else { k }
                };
                let mut t1 = next(t0);
                while t1 < hi && ic.tans[idx(t1)].dot(tan0) >= 0.866 {
                    t1 = next(t1);
                }
                let anchor = ic.eval(t0);
                let end = ic.eval(t1);
                let Some(dir) = (end - anchor).normalize() else {
                    // a stretch shorter than rounding adds nothing
                    t0 = t1;
                    continue;
                };
                let dir = dir.get();
                let mut knots = vec![(t0, 0.0)];
                let mut k = (t0 + 1e-9).floor() + 1.0;
                while k < t1 - 1e-9 {
                    knots.push((k, dir.dot(ic.pts[idx(k)] - anchor)));
                    k += 1.0;
                }
                knots.push((t1, dir.dot(end - anchor)));
                if knots.windows(2).any(|w| w[1].1 <= w[0].1) {
                    return Err("an intersection edge doubles back");
                }
                let s1 = knots[knots.len() - 1].1;
                let way = Rc::new(Way::Chord {
                    c: ic,
                    anchor,
                    dir,
                    knots,
                });
                refine(s, &way, 0.0, s1, rel, 8, &mut out);
                t0 = t1;
            }
        }
        curve => {
            let k = match curve {
                Curve::Circle(_) | Curve::Ellipse(_) => (e.range.width() / dmath::FRAC_PI_2).ceil().max(1.0),
                _ => 1.0,
            };
            let way = Rc::new(Way::Curve(curve));
            // even steps, and every point where the curve's parameter is not smooth
            let mut cuts: Vec<f64> = (1..k as usize).map(|i| lo + (hi - lo) * (i as f64 / k)).collect();
            cuts.extend(curve.kinks(e.range));
            cuts.sort_by(f64::total_cmp);
            let mut a = lo;
            for b in cuts.into_iter().chain([hi]) {
                if b > a {
                    refine(s, &way, a, b, rel, 8, &mut out);
                    a = b;
                }
            }
        }
    }
    if !co.forward {
        out.reverse();
        for p in &mut out {
            p.n.reverse();
            p.sign = -1.0;
        }
    }
    Ok(out)
}

/// A loop as runs between singular points (sphere poles, cone apexes), where the surface's
/// parameter jumps: the jump axis of each, and how often the loop winds round each periodic
/// parameter.
struct LoopTrace<'a> {
    runs: Vec<Vec<Piece<'a>>>,
    jumps: Vec<usize>,
    wind: [i64; 2],
}

fn trace_loop<'a>(body: &'a Body, s: &Surface, lp: LoopId, rel: f64) -> Result<LoopTrace<'a>, &'static str> {
    let per = periods(s);
    let cos: Vec<CoedgeId> = body.lp(lp).coedges().collect();
    let sing: Vec<bool> = cos
        .iter()
        .map(|&c| {
            let p = body.vertex(body.coedge_start(c)).point;
            s.normal(s.project(p)).is_none()
        })
        .collect();
    let rot = sing.iter().position(|&b| b).unwrap_or(0);
    let mut runs: Vec<Vec<Piece<'a>>> = Vec::new();
    let mut jumps = Vec::new();
    let mut prev: Option<[f64; 2]> = None;
    let jump = |a: [f64; 2], b: [f64; 2]| {
        let d = [
            (near(b[0], a[0], per[0]) - a[0]).abs(),
            (near(b[1], a[1], per[1]) - a[1]).abs(),
        ];
        (d[0].max(d[1]) > 1e-9).then_some(usize::from(d[1] > d[0]))
    };
    for i in 0..cos.len() {
        let k = (rot + i) % cos.len();
        let fresh = runs.is_empty() || sing[k];
        if fresh {
            runs.push(Vec::new());
        }
        for (j, mut pc) in coedge_pieces(body, s, cos[k], rel)?.into_iter().enumerate() {
            if let Some(r) = prev {
                if fresh && j == 0 && sing[k] {
                    jumps.extend(jump(r, pc.n[0].uv));
                }
                for a in 0..2 {
                    let d = near(pc.n[0].uv[a], r[a], per[a]) - pc.n[0].uv[a];
                    if d != 0.0 {
                        pc.n.iter_mut().for_each(|q| q.uv[a] += d);
                    }
                }
            }
            prev = Some(pc.n[pc.n.len() - 1].uv);
            runs.last_mut().expect("a run").push(pc);
        }
    }
    let first = runs[0][0].n[0].uv;
    let last = prev.expect("a loop has an edge");
    let mut wind = [0; 2];
    if sing[rot] {
        jumps.extend(jump(last, first));
    } else {
        for a in 0..2 {
            if let Some(p) = per[a] {
                wind[a] = ((near(first[a], last[a], per[a]) - first[a]) / p).round() as i64;
            }
        }
    }
    Ok(LoopTrace { runs, jumps, wind })
}

/// How G, the integral along the inner parameter, is found.
enum Inner {
    /// Gauss-Legendre, exact for F polynomial along a.
    Poly(&'static [(f64, f64)]),
    /// F is a trigonometric polynomial of degree k along a, its Fourier coefficients
    /// polynomials or trigonometric polynomials along b: fitted once per face.
    Fourier { k: usize, along: Along },
    /// A Chebyshev interpolant of F over the face's parameter box, its last coefficients
    /// bounding its error.
    Cheb(Vec<Cheb>, ((f64, f64), (f64, f64))),
    /// Adaptive Gauss-Kronrod.
    Adapt,
}

struct Cheb {
    a: (f64, f64),
    b: (f64, f64),
    /// Degrees along a and b; coefficients c[i * (nb + 1) + j] of T_i(a) T_j(b).
    na: usize,
    nb: usize,
    c: Vec<F>,
    tail: F,
}

/// T_0..=T_n at x.
fn chebyshev(x: f64, n: usize) -> Vec<f64> {
    let mut t = vec![1.0; n + 1];
    if n > 0 {
        t[1] = x;
    }
    for k in 2..=n {
        t[k] = 2.0 * x * t[k - 1] - t[k - 2];
    }
    t
}

/// The integrals of T_0..=T_n from -1 to x.
fn chebyshev_integrals(x: f64, n: usize) -> Vec<f64> {
    let t = chebyshev(x, n + 1);
    (0..=n)
        .map(|i| match i {
            0 => x + 1.0,
            1 => 0.5 * (x * x - 1.0),
            _ => {
                let (p, m) = ((i + 1) as f64, (i - 1) as f64);
                let sign = if i % 2 == 0 { -1.0 } else { 1.0 };
                0.5 * (t[i + 1] / p - t[i - 1] / m) - 0.5 * sign * (1.0 / p - 1.0 / m)
            }
        })
        .collect()
}

impl Cheb {
    /// Interpolants over tiles of the box: a tile that 64 degrees along a, or 32 along b, cannot
    /// draw is halved that way; None once `left` evaluations are spent.
    fn tiles(
        cx: &Ctx,
        a: (f64, f64),
        b: (f64, f64),
        depth: u32,
        left: &mut usize,
        out: &mut Vec<Cheb>,
    ) -> Option<()> {
        match Cheb::fit(cx, a, b, left) {
            Ok(c) => out.push(c),
            Err((sa, sb)) if depth > 0 && (sa || sb) => {
                let ha = if sa {
                    vec![(a.0, 0.5 * (a.0 + a.1)), (0.5 * (a.0 + a.1), a.1)]
                } else {
                    vec![a]
                };
                let hb = if sb {
                    vec![(b.0, 0.5 * (b.0 + b.1)), (0.5 * (b.0 + b.1), b.1)]
                } else {
                    vec![b]
                };
                for &x in &ha {
                    for &y in &hb {
                        Cheb::tiles(cx, x, y, depth - 1, left, out)?;
                    }
                }
            }
            Err(_) => return None,
        }
        Some(())
    }

    /// Interpolates F at Chebyshev-Lobatto points, doubling the degree along a or b while its
    /// last coefficients there are not negligible. Err: which ways it needs more than 64 along a
    /// or 32 along b (neither when F is not finite or `left` runs out).
    fn fit(cx: &Ctx, a: (f64, f64), b: (f64, f64), left: &mut usize) -> Result<Cheb, (bool, bool)> {
        let lobatto = |n: usize, (lo, hi): (f64, f64)| -> Vec<f64> {
            (0..=n)
                .map(|k| 0.5 * (lo + hi) - 0.5 * (hi - lo) * dmath::cos(k as f64 * dmath::PI / n as f64))
                .collect()
        };
        let (mut na, mut nb) = (16, 8);
        let mut last = ([f64::INFINITY; NF], [f64::INFINITY; NF]);
        let mut vals: Vec<F> = Vec::new();
        let (mut xa, mut xb) = (Vec::new(), Vec::new());
        loop {
            // samples on the new grid, reusing those on the old one (every other point)
            let (ya, yb) = (lobatto(na, a), lobatto(nb, b));
            let (oa, ob) = (xa.len().saturating_sub(1), xb.len().saturating_sub(1));
            *left = left.checked_sub((na + 1) * (nb + 1)).ok_or((false, false))?;
            let mut v = Vec::with_capacity((na + 1) * (nb + 1));
            for (i, &u) in ya.iter().enumerate() {
                for (j, &w) in yb.iter().enumerate() {
                    let old = |n: usize, o: usize, k: usize| {
                        if o == n {
                            Some(k)
                        } else if o > 0 && 2 * o == n && k.is_multiple_of(2) {
                            Some(k / 2)
                        } else {
                            None
                        }
                    };
                    let f = match (old(na, oa, i), old(nb, ob, j)) {
                        (Some(p), Some(q)) => vals[p * (ob + 1) + q],
                        _ => cx.f(u, w),
                    };
                    if f.iter().any(|x| !x.is_finite()) {
                        return Err((false, false));
                    }
                    v.push(f);
                }
            }
            (vals, xa, xb) = (v, ya, yb);
            let c = coefficients(&vals, na, nb);
            let mut scale = [0.0; NF];
            let (mut ta, mut tb) = ([0.0; NF], [0.0; NF]);
            for i in 0..=na {
                for j in 0..=nb {
                    for m in 0..NF {
                        let x = c[i * (nb + 1) + j][m].abs();
                        scale[m] += x;
                        if i + 2 > na {
                            ta[m] += x;
                        }
                        if j + 2 > nb {
                            tb[m] += x;
                        }
                    }
                }
            }
            // converged, or stuck at the surface's own rounding (no smaller for twice the points)
            let ok = |t: &F, was: &F| {
                (0..NF).all(|m| {
                    t[m] <= 1e-2 * cx.rel * scale[m] + 1e-300
                        || (t[m] <= cx.rel * scale[m] && t[m] >= 0.3 * was[m])
                })
            };
            let (oka, okb) = (ok(&ta, &last.0), ok(&tb, &last.1));
            last = (ta, tb);
            if oka && okb {
                let tail = std::array::from_fn(|m| ta[m] + tb[m]);
                return Ok(Cheb {
                    a,
                    b,
                    na,
                    nb,
                    c,
                    tail,
                });
            }
            if !oka {
                na *= 2;
            }
            if !okb {
                nb *= 2;
            }
            if na > 64 || nb > 32 {
                return Err((!oka, !okb));
            }
        }
    }

    /// G at (a, b) and its error bound, over tiles covering the box `all`.
    fn sum(tiles: &[Cheb], all: ((f64, f64), (f64, f64)), a: f64, b: f64) -> (F, F) {
        let (mut g, mut e) = ([0.0; NF], [0.0; NF]);
        for t in tiles {
            // a tile on the box's side also takes what lies beyond it
            let inb = (t.b.0 <= b || t.b.0 == all.1.0) && (b < t.b.1 || t.b.1 == all.1.1);
            if !inb || (a <= t.a.0 && t.a.0 != all.0.0) {
                continue;
            }
            let x = if t.a.1 == all.0.1 { a } else { a.min(t.a.1) };
            let (gt, et) = t.g(x, b);
            for m in 0..NF {
                g[m] += gt[m];
                e[m] += et[m];
            }
        }
        (g, e)
    }

    /// G within this tile, from its a side to `a`, at b, and its error bound.
    fn g(&self, a: f64, b: f64) -> (F, F) {
        let (ha, hb) = (0.5 * (self.a.1 - self.a.0), 0.5 * (self.b.1 - self.b.0));
        let it = chebyshev_integrals((a - self.a.0) / ha - 1.0, self.na);
        let tb = chebyshev((b - self.b.0) / hb - 1.0, self.nb);
        let mut g = [0.0; NF];
        for (ci, iti) in self.c.chunks(self.nb + 1).zip(&it) {
            let mut row = [0.0; NF];
            for (c, t) in ci.iter().zip(&tb) {
                axpy(&mut row, *t, c);
            }
            axpy(&mut g, ha * iti, &row);
        }
        (g, std::array::from_fn(|m| 4.0 * ha * self.tail[m]))
    }
}

/// Chebyshev coefficients of values at Lobatto points (x_k = -cos(k pi / n)).
fn coefficients(vals: &[F], na: usize, nb: usize) -> Vec<F> {
    let matrix = |n: usize| -> Vec<f64> {
        let mut m = vec![0.0; (n + 1) * (n + 1)];
        for k in 0..=n {
            let t = chebyshev(-dmath::cos(k as f64 * dmath::PI / n as f64), n);
            for (j, tj) in t.iter().enumerate() {
                let half = |i: usize| if i == 0 || i == n { 0.5 } else { 1.0 };
                m[j * (n + 1) + k] = 2.0 / n as f64 * tj * half(k) * half(j);
            }
        }
        m
    };
    let (ma, mb) = (matrix(na), matrix(nb));
    // along b for each a sample, then along a
    let mut rows = vec![[0.0; NF]; (na + 1) * (nb + 1)];
    for i in 0..=na {
        for j in 0..=nb {
            let mut v = [0.0; NF];
            for k in 0..=nb {
                axpy(&mut v, mb[j * (nb + 1) + k], &vals[i * (nb + 1) + k]);
            }
            rows[i * (nb + 1) + j] = v;
        }
    }
    let mut c = vec![[0.0; NF]; (na + 1) * (nb + 1)];
    for i in 0..=na {
        for j in 0..=nb {
            let mut v = [0.0; NF];
            for k in 0..=na {
                axpy(&mut v, ma[i * (na + 1) + k], &rows[k * (nb + 1) + j]);
            }
            c[i * (nb + 1) + j] = v;
        }
    }
    c
}

enum Along {
    /// The coefficients at Chebyshev points `bs` (barycentric weights `lam`).
    Poly {
        bs: Vec<f64>,
        lam: Vec<f64>,
        coef: Vec<Vec<F>>,
    },
    /// The coefficients' own Fourier coefficients, from b = 0.
    Trig { m: usize, coef: Vec<Vec<F>> },
}

/// cos and sin of 2 pi j / n, for n up to 11.
fn roots(n: usize) -> &'static [(f64, f64)] {
    static R: OnceLock<Vec<Vec<(f64, f64)>>> = OnceLock::new();
    &R.get_or_init(|| {
        (0..12)
            .map(|n| {
                (0..n)
                    .map(|j| {
                        let (s, c) = dmath::sin_cos(dmath::TAU * (j as f64 / n as f64));
                        (c, s)
                    })
                    .collect()
            })
            .collect()
    })[n]
}

/// Fourier coefficients (c0, c1, s1, c2, s2, ..) of 2k + 1 samples at equal steps from 0.
fn dft(samples: &[F], k: usize) -> Vec<F> {
    let n = samples.len();
    let r = roots(n);
    let mut out = vec![[0.0; NF]; 2 * k + 1];
    for (j, sm) in samples.iter().enumerate() {
        axpy(&mut out[0], 1.0 / n as f64, sm);
        for m in 1..=k {
            let (co, si) = r[(j * m) % n];
            axpy(&mut out[2 * m - 1], 2.0 / n as f64 * co, sm);
            axpy(&mut out[2 * m], 2.0 / n as f64 * si, sm);
        }
    }
    out
}

/// cos and sin of m x for m = 1..=k (k at most 5).
fn harmonics(x: f64, k: usize) -> [(f64, f64); 5] {
    let (sx, cx) = dmath::sin_cos(x);
    let (mut s, mut c) = (0.0, 1.0);
    let mut out = [(0.0, 0.0); 5];
    for o in out.iter_mut().take(k) {
        (s, c) = (s * cx + c * sx, c * cx - s * sx);
        *o = (c, s);
    }
    out
}

/// Chebyshev points over [lo, hi] and their barycentric weights.
fn cheb(n: usize, lo: f64, hi: f64) -> (Vec<f64>, Vec<f64>) {
    (0..n)
        .map(|k| {
            let (s, c) = dmath::sin_cos((2 * k + 1) as f64 * dmath::PI / (2 * n) as f64);
            (
                0.5 * (lo + hi) - 0.5 * (hi - lo) * c,
                if k % 2 == 0 { s } else { -s },
            )
        })
        .unzip()
}

const GL2: [(f64, f64); 2] = [(-0.577_350_269_189_625_8, 1.0), (0.577_350_269_189_625_8, 1.0)];
const GL3: [(f64, f64); 3] = [
    (-0.774_596_669_241_483_4, 5.0 / 9.0),
    (0.0, 8.0 / 9.0),
    (0.774_596_669_241_483_4, 5.0 / 9.0),
];

const XGK: [f64; 8] = [
    0.991_455_371_120_812_6,
    0.949_107_912_342_758_5,
    0.864_864_423_359_769_1,
    0.741_531_185_599_394_4,
    0.586_087_235_467_691_1,
    0.405_845_151_377_397_2,
    0.207_784_955_007_898_5,
    0.0,
];
const WGK: [f64; 8] = [
    0.022_935_322_010_529_22,
    0.063_092_092_629_978_55,
    0.104_790_010_322_250_2,
    0.140_653_259_715_525_9,
    0.169_004_726_639_267_9,
    0.190_350_578_064_785_4,
    0.204_432_940_075_298_9,
    0.209_482_141_084_727_8,
];
const WG: [f64; 4] = [
    0.129_484_966_168_869_7,
    0.279_705_391_489_276_7,
    0.381_830_050_505_118_9,
    0.417_959_183_673_469_4,
];

struct Ctx<'a> {
    s: &'a Surface,
    /// The inner parameter: 0 for u, 1 for v.
    ax: usize,
    a0: f64,
    c: Point3,
    inner: Inner,
    rel: f64,
    /// Below these a share of each integral is negligible: rel times a far smaller part's.
    floor: F,
    /// Gauss-Kronrod panels left to spend on the face; past that each is taken as it is, with
    /// its error.
    panels: std::cell::Cell<usize>,
}

impl Ctx<'_> {
    fn f(&self, a: f64, b: f64) -> F {
        let uv = if self.ax == 0 {
            Point2::new(a, b)
        } else {
            Point2::new(b, a)
        };
        let d = self.s.deriv(uv);
        let n = d.du.cross(d.dv);
        let mut f = integrand(d.p - self.c, n);
        if let Surface::Sphere(sp) = self.s {
            // r^2 cos v: |du x dv| as the trigonometric polynomial it is between the poles
            f[10] = n.dot(d.p - sp.frame.origin) / sp.radius;
        }
        f
    }

    /// G at (a, b), the integral of F over a whole period of a (for a periodic a), and G's error.
    fn g(&self, a: f64, b: f64) -> (F, F, F) {
        let mut g = [0.0; NF];
        let x = a - self.a0;
        match self.inner {
            Inner::Poly(rule) => {
                for &(t, w) in rule {
                    axpy(&mut g, 0.5 * x * w, &self.f(self.a0 + 0.5 * x * (1.0 + t), b));
                }
                (g, [0.0; NF], [0.0; NF])
            }
            Inner::Fourier { k, ref along } => {
                // the Fourier coefficients along a at this b
                let mut coef = [[0.0; NF]; 11];
                match along {
                    Along::Poly { bs, lam, coef: at } => {
                        let mut w = [0.0; 5];
                        match bs.iter().position(|&bi| bi == b) {
                            Some(i) => w[i] = 1.0,
                            None => {
                                let mut sum = 0.0;
                                for i in 0..bs.len() {
                                    w[i] = lam[i] / (b - bs[i]);
                                    sum += w[i];
                                }
                                w.iter_mut().for_each(|x| *x /= sum);
                            }
                        }
                        for (q, cq) in coef.iter_mut().enumerate().take(2 * k + 1) {
                            for i in 0..bs.len() {
                                axpy(cq, w[i], &at[i][q]);
                            }
                        }
                    }
                    Along::Trig { m, coef: at } => {
                        let h = harmonics(b, *m);
                        for (cq, aq) in coef.iter_mut().zip(at) {
                            *cq = aq[0];
                            for (r, &(co, si)) in h.iter().take(*m).enumerate() {
                                axpy(cq, co, &aq[2 * r + 1]);
                                axpy(cq, si, &aq[2 * r + 2]);
                            }
                        }
                    }
                }
                axpy(&mut g, x, &coef[0]);
                for (m, &(co, si)) in harmonics(x, k).iter().take(k).enumerate() {
                    let d = (m + 1) as f64;
                    axpy(&mut g, si / d, &coef[2 * m + 1]);
                    axpy(&mut g, (1.0 - co) / d, &coef[2 * m + 2]);
                }
                let mut phi = [0.0; NF];
                axpy(&mut phi, dmath::TAU, &coef[0]);
                (g, phi, [0.0; NF])
            }
            Inner::Cheb(ref ch, all) => {
                let (g, e) = Cheb::sum(ch, all, a, b);
                (g, [0.0; NF], e)
            }
            Inner::Adapt => {
                if x.abs() <= 1e-15 * (1.0 + self.a0.abs()) {
                    return ([0.0; NF], [0.0; NF], [0.0; NF]);
                }
                let (v, e) = self.adapt(self.a0, a, b, 12);
                (v, [0.0; NF], e)
            }
        }
    }

    /// Gauss-Kronrod 7-15 on [lo, hi]: the Kronrod sum, the error estimate, and the sum of |F|.
    fn gk(&self, lo: f64, hi: f64, b: f64) -> (F, F, F) {
        self.panels.set(self.panels.get().saturating_sub(1));
        let (c, h) = (0.5 * (lo + hi), 0.5 * (hi - lo));
        let (mut k, mut g, mut abs) = ([0.0; NF], [0.0; NF], [0.0; NF]);
        for i in 0..15 {
            let (j, sgn) = if i < 8 { (i, -1.0) } else { (14 - i, 1.0) };
            let fx = self.f(c + sgn * h * XGK[j], b);
            axpy(&mut k, h * WGK[j], &fx);
            if j % 2 == 1 || j == 7 {
                axpy(&mut g, h * WG[j / 2], &fx);
            }
            for m in 0..NF {
                abs[m] += (h * WGK[j] * fx[m]).abs();
            }
        }
        let e = std::array::from_fn(|m| (k[m] - g[m]).abs());
        (k, e, abs)
    }

    /// Bisects while the halves' error falls; where it stops falling it is the surface's own
    /// rounding (a derivative found by differences), and that is the error reported.
    fn adapt(&self, lo: f64, hi: f64, b: f64, depth: u32) -> (F, F) {
        let (k, e, abs) = self.gk(lo, hi, b);
        let (k, e, _) = self.split(lo, hi, b, depth, (k, e, abs));
        (k, e)
    }

    fn split(&self, lo: f64, hi: f64, b: f64, depth: u32, (k, e, abs): (F, F, F)) -> (F, F, bool) {
        if depth == 0
            || self.panels.get() == 0
            || (0..NF).all(|m| e[m] <= 0.1 * self.rel * abs[m] + self.floor[m])
        {
            return (k, e, true);
        }
        let mid = 0.5 * (lo + hi);
        let (l, r) = (self.gk(lo, mid, b), self.gk(mid, hi, b));
        if (0..NF).all(|m| l.1[m] + r.1[m] >= 0.5 * e[m]) {
            return (
                std::array::from_fn(|m| l.0[m] + r.0[m]),
                std::array::from_fn(|m| l.1[m] + r.1[m]),
                false,
            );
        }
        let (k1, e1, _) = self.split(lo, mid, b, depth - 1, l);
        let (k2, e2, _) = self.split(mid, hi, b, depth - 1, r);
        (
            std::array::from_fn(|m| k1[m] + k2[m]),
            std::array::from_fn(|m| e1[m] + e2[m]),
            true,
        )
    }
}

/// What one run of a loop adds: the loop integral, its error, the integral of a period's worth
/// of F (what shifting the run a period along a adds), and its share of the uv area and of b.
#[derive(Default)]
struct Acc {
    s: F,
    e: F,
    phi: F,
    area: f64,
    db: f64,
}

/// Adds a piece's share to `u`, or None (adding nothing) when its integrand is not resolved and
/// it may still be made finer.
fn integrate(cx: &Ctx, pc: &Piece, u: &mut Acc, last: bool) -> Option<()> {
    let (ax, c) = (cx.ax, cx.c);
    let r = rule(pc.n.len());
    let kr = pc.kronrod.then(|| kronrod(pc.n.len()));
    let big = pc.n.iter().fold(0.0f64, |m, q| m.max(q.uv[1 - ax].abs()));
    let flat = pc.n.iter().all(|q| q.d[1 - ax].abs() <= 1e-12 * (1.0 + big));
    let (mut sum, mut phis, mut abs) = ([0.0; NF], [0.0; NF], [0.0; NF]);
    let (mut tail, mut e, mut gauss) = ([[0.0; NF]; 2], [0.0; NF], [0.0; NF]);
    let (mut area, mut db) = (0.0, 0.0);
    for (k, q) in pc.n.iter().enumerate() {
        let (a, d) = (q.uv[ax], q.d[1 - ax]);
        let wk = kr.map_or_else(|| r.w[k], |kr| kr.w[k]);
        let w = wk * pc.sign;
        area += w * a * d;
        db += w * d;
        // the gap between edge and surface moves the boundary by that much
        let x = (q.p - c).length();
        let len = wk * q.speed * q.gap;
        let m = [x / 3.0, x * x / 2.0, x * x * x / 2.0, 1.0];
        for (i, ei) in e.iter_mut().enumerate() {
            *ei += len * m[[0, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3][i]];
        }
        if flat {
            continue;
        }
        let (g, phi, ge) = cx.g(a, q.uv[1 - ax]);
        axpy(&mut sum, w * d, &g);
        axpy(&mut phis, w * d, &phi);
        for m in 0..NF {
            abs[m] += (w * d * g[m]).abs();
            e[m] += (w * d * ge[m]).abs();
        }
        match kr {
            Some(kr) => axpy(&mut gauss, kr.g[k] * pc.sign * d, &g),
            None => {
                for (t, row) in tail.iter_mut().zip(&r.tail) {
                    axpy(t, row[k] * d, &g);
                }
            }
        }
    }
    // the embedded Gauss rule's difference, or the interpolant's last coefficients, bound what
    // the rule leaves out
    let est: F = std::array::from_fn(|m| {
        let x = if kr.is_some() {
            (sum[m] - gauss[m]).abs()
        } else {
            2.0 * (tail[0][m].abs() + tail[1][m].abs())
        };
        x + 1e-15 * abs[m]
    });
    if !last && (0..NF).any(|m| est[m] > 0.1 * cx.rel * abs[m] + cx.floor[m]) {
        return None;
    }
    for m in 0..NF {
        u.s[m] += sum[m];
        u.phi[m] += phis[m];
        u.e[m] += e[m] + est[m];
    }
    u.area += area;
    u.db += db;
    Some(())
}

/// A face's integrals (flux of each moment's field, then area) and their error bounds.
/// `size` sets the scale below which a piece's share is negligible.
fn face_integrals(body: &Body, f: FaceId, c: Point3, size: f64, rel: f64) -> Result<(F, F), &'static str> {
    let face = body.face(f);
    let s = &face.surface;
    let sigma = if face.same_sense { 1.0 } else { -1.0 };
    let loops: Vec<LoopTrace> = face
        .loops
        .iter()
        .map(|&l| trace_loop(body, s, l, rel))
        .collect::<Result<_, _>>()?;
    let per = periods(s);
    let wind = |a: usize| loops.iter().map(|l| l.wind[a]).sum::<i64>();
    let winds = |a: usize| loops.iter().any(|l| l.wind[a] != 0);
    let jump_ax = loops.iter().flat_map(|l| l.jumps.iter().copied()).next();
    if loops.iter().any(|l| l.jumps.iter().any(|&j| Some(j) != jump_ax)) {
        return Err("singular points jump both ways");
    }
    // the inner parameter: one the loops do not wind round, along any parameter jump
    let ax = if winds(0) && winds(1) {
        return Err("a loop winds round both parameters");
    } else if winds(0) {
        1
    } else if winds(1) {
        0
    } else if let Some(j) = jump_ax {
        j
    } else if let Surface::Procedural(p) = s {
        // across any u where the surface is not smooth, else the shorter way across
        let (su, sv) = p.uv_scale();
        let w = |a: usize| {
            let (lo, hi) = span(&loops, a);
            hi - lo
        };
        let (ulo, uhi) = span(&loops, 0);
        let kinked = ulo < uhi && !p.u_kinks(forge_math::Interval::new(ulo, uhi)).is_empty();
        usize::from(kinked || w(1) * sv < w(0) * su)
    } else {
        0
    };
    if jump_ax.is_some_and(|j| j != ax) {
        return Err("a loop winds round a parameter its singular points jump along");
    }
    let sgn = if ax == 0 { 1.0 } else { -1.0 };
    let dom = s.domain();
    let (alo, ahi) = if ax == 0 {
        (dom.u.lo, dom.u.hi)
    } else {
        (dom.v.lo, dom.v.hi)
    };
    // a0: where G is zero. A face holding a singular line (pole, apex) inside starts there, so
    // that line, a boundary of the parameter domain, adds nothing.
    let wd = -sgn * sigma * wind(1 - ax) as f64;
    let a0 = if wd > 0.0 && ahi.is_finite() && per[ax].is_none() {
        ahi
    } else if wd < 0.0 && alo.is_finite() && per[ax].is_none() {
        alo
    } else if wd != 0.0 {
        return Err("a loop winds round a face that holds no singular point");
    } else if loops.is_empty() {
        alo
    } else {
        span(&loops, ax).0
    };
    let mut cx = Ctx {
        s,
        ax,
        a0,
        c,
        inner: Inner::Adapt,
        panels: std::cell::Cell::new(
            20_000 + 20 * loops.iter().flat_map(|l| &l.runs).map(Vec::len).sum::<usize>(),
        ),
        rel,
        floor: std::array::from_fn(|m| {
            let k = [3, 4, 4, 4, 5, 5, 5, 5, 5, 5, 2][m];
            1e-6 * rel * size.powi(k)
        }),
    };
    // F's degree along a, and along b: a polynomial (Some) or trigonometric (None)
    let fourier = match (s, ax) {
        (Surface::Plane(_), _) | (Surface::Cylinder(_), 1) => {
            cx.inner = Inner::Poly(&GL2);
            None
        }
        (Surface::Cone(_), 1) => {
            cx.inner = Inner::Poly(&GL3);
            None
        }
        (Surface::Cylinder(_), _) => Some((4, Some(3))),
        (Surface::Cone(_), _) => Some((4, Some(4))),
        (Surface::Sphere(_) | Surface::Torus(_), 0) => Some((4, None)),
        (Surface::Sphere(_) | Surface::Torus(_), _) => Some((5, None)),
        (Surface::Procedural(_), _) => None,
    };
    if let Some((k, deg)) = fourier {
        let na = 2 * k + 1;
        let row = |b: f64| {
            let f: Vec<F> = (0..na)
                .map(|j| cx.f(a0 + dmath::TAU * (j as f64 / na as f64), b))
                .collect();
            dft(&f, k)
        };
        let along = match deg {
            Some(d) => {
                let (lo, hi) = if loops.is_empty() {
                    (-1.0, 1.0)
                } else {
                    span(&loops, 1 - ax)
                };
                let (lo, hi) = if hi - lo > 1e-9 {
                    (lo, hi)
                } else {
                    (lo - 1.0, hi + 1.0)
                };
                let (bs, lam) = cheb(d + 1, lo, hi);
                let coef = bs.iter().map(|&b| row(b)).collect();
                Along::Poly { bs, lam, coef }
            }
            None => {
                let m = 9 - k;
                let nb = 2 * m + 1;
                let rows: Vec<Vec<F>> = (0..nb)
                    .map(|i| row(dmath::TAU * (i as f64 / nb as f64)))
                    .collect();
                let coef = (0..na)
                    .map(|q| dft(&rows.iter().map(|r| r[q]).collect::<Vec<F>>(), m))
                    .collect();
                Along::Trig { m, coef }
            }
        };
        cx.inner = Inner::Fourier { k, along };
    } else if matches!(s, Surface::Procedural(_)) && !loops.is_empty() {
        let (alo, ahi) = span(&loops, ax);
        let (blo, bhi) = span(&loops, 1 - ax);
        // inside the surface's domain: past its ends a procedural surface may take another branch
        let dom = [dom.u, dom.v];
        let inside = |x: f64, k: usize| {
            if dom[k].period.is_some() {
                x
            } else {
                x.clamp(dom[k].lo, dom[k].hi)
            }
        };
        let (a0c, bl, bh) = (inside(a0, ax), inside(blo, 1 - ax), inside(bhi, 1 - ax));
        let all = ((a0c, inside(ahi, ax).max(a0c + 1e-12)), (bl, bh.max(bl + 1e-12)));
        // tiles start split where the surface is not smooth along u
        let Surface::Procedural(p) = s else { unreachable!() };
        let u = if ax == 0 { all.0 } else { all.1 };
        let mut cuts = p.u_kinks(forge_math::Interval::new(u.0, u.1));
        cuts.insert(0, u.0);
        cuts.push(u.1);
        let mut tiles = Vec::new();
        let mut left = 20_000 + 400 * cuts.len();
        let fit = ahi > alo
            && bhi > blo
            && cuts.windows(2).all(|w| {
                let (a, b) = if ax == 0 {
                    ((w[0], w[1]), all.1)
                } else {
                    (all.0, (w[0], w[1]))
                };
                Cheb::tiles(&cx, a, b, 6, &mut left, &mut tiles).is_some()
            });
        if fit {
            cx.inner = Inner::Cheb(tiles, all);
        }
    }
    let mut units: Vec<Acc> = Vec::new();
    for l in &loops {
        for run in &l.runs {
            let mut u = Acc::default();
            for pc in run {
                let mut work = vec![(8u32, None)];
                while let Some((depth, piece)) = work.pop() {
                    let pc = piece.as_ref().unwrap_or(pc);
                    // made finer while the face's budget lasts
                    match integrate(&cx, pc, &mut u, depth == 0 || cx.panels.get() == 0) {
                        Some(()) => {}
                        None => {
                            cx.panels.set(cx.panels.get().saturating_sub(pc.n.len() / 4));
                            let mut f = finer(s, pc);
                            while let Some(p) = f.pop() {
                                work.push((depth - 1, Some(p)));
                            }
                        }
                    }
                }
            }
            units.push(u);
        }
    }
    // shift runs a period along a so the region drawn is the face: positive area, the least
    let mut total = [0.0; NF];
    let mut err = [0.0; NF];
    let mut area: f64 = units.iter().map(|u| u.area).sum();
    for u in &units {
        for m in 0..NF {
            total[m] += u.s[m];
            err[m] += u.e[m];
        }
    }
    let want = sigma * sgn;
    if let Some(p) = per[ax] {
        let free: Vec<&Acc> = units.iter().filter(|u| u.db.abs() > 1e-9).skip(1).collect();
        if free.len() > 8 {
            return Err("too many runs between singular points");
        }
        let mut best: Option<(f64, usize)> = None;
        for combo in 0..3usize.pow(free.len() as u32) {
            let (mut cb, mut a) = (combo, area);
            for u in &free {
                a += ((cb % 3) as f64 - 1.0) * p * u.db;
                cb /= 3;
            }
            if a * want > 0.0 && best.is_none_or(|b| a.abs() < b.0) {
                best = Some((a.abs(), combo));
            }
        }
        if let Some((_, mut cb)) = best {
            for u in &free {
                let k = (cb % 3) as f64 - 1.0;
                cb /= 3;
                axpy(&mut total, k, &u.phi);
                area += k * p * u.db;
            }
        }
    }
    let mut out = [0.0; NF];
    for m in 0..NF {
        out[m] = if m < 10 { sgn * total[m] } else { want * total[m] };
    }
    // a face that is everything but its loops (a sphere with a hole), or no loops at all
    if loops.is_empty() || area * want < 0.0 {
        let (blo, bhi) = if ax == 0 {
            (dom.v.lo, dom.v.hi)
        } else {
            (dom.u.lo, dom.u.hi)
        };
        if per[ax].is_none()
            || !(blo.is_finite() && bhi.is_finite())
            || !loops.iter().all(|l| l.runs.len() == 1)
        {
            return Err("the face's loops draw no region");
        }
        let (mid, half) = (0.5 * (blo + bhi), 0.5 * (bhi - blo));
        let r = rule(36);
        for k in 0..36 {
            let (_, phi, _) = cx.g(a0, mid + half * r.x[k]);
            for m in 0..NF {
                let v = half * r.w[k] * phi[m];
                out[m] += if m < 10 { sigma * v } else { v };
                err[m] += 1e-15 * v.abs();
            }
        }
    }
    Ok((out, err))
}

/// The range of parameter `a` over the loops' samples.
fn span(loops: &[LoopTrace], a: usize) -> (f64, f64) {
    loops
        .iter()
        .flat_map(|l| l.runs.iter().flatten().flat_map(|p| p.n.iter()))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |m, q| {
            (m.0.min(q.uv[a]), m.1.max(q.uv[a]))
        })
}

/// Eigenvalues (ascending) and unit eigenvectors of a symmetric 3x3 matrix, by Jacobi
/// rotations.
fn eigen(m: [[f64; 3]; 3]) -> ([f64; 3], [Vec3; 3]) {
    let mut a = m;
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..50 {
        let off = a[0][1].abs() + a[0][2].abs() + a[1][2].abs();
        let diag = a[0][0].abs() + a[1][1].abs() + a[2][2].abs();
        if off <= 1e-300 || off <= f64::EPSILON * 1e-3 * diag {
            break;
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if a[p][q] == 0.0 {
                continue;
            }
            let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
            let t = if theta == 0.0 { 1.0 } else { t };
            let cs = 1.0 / (t * t + 1.0).sqrt();
            let sn = t * cs;
            for row in &mut a {
                let (akp, akq) = (row[p], row[q]);
                row[p] = cs * akp - sn * akq;
                row[q] = sn * akp + cs * akq;
            }
            let (rp, rq) = (a[p], a[q]);
            a[p] = std::array::from_fn(|k| cs * rp[k] - sn * rq[k]);
            a[q] = std::array::from_fn(|k| sn * rp[k] + cs * rq[k]);
            for row in &mut v {
                let (vp, vq) = (row[p], row[q]);
                row[p] = cs * vp - sn * vq;
                row[q] = sn * vp + cs * vq;
            }
        }
    }
    let mut order = [0, 1, 2];
    order.sort_by(|&i, &j| a[i][i].total_cmp(&a[j][j]));
    let axis = |i: usize| {
        let x = Vec3::new(v[0][i], v[1][i], v[2][i]);
        // a fixed sign: the largest component positive
        let k = (0..3).fold(0, |b, k| if x[k].abs() > x[b].abs() { k } else { b });
        if x[k] < 0.0 { -x } else { x }
    };
    (order.map(|i| a[i][i]), order.map(axis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fejer_rules_integrate_and_differentiate() {
        let (r12, r36) = (rule(12), rule(36));
        for r in [rule(4), r12, r36] {
            let n = r.x.len();
            // exact for polynomials below degree n
            let i: f64 = (0..n).map(|k| r.w[k] * r.x[k].powi(2)).sum();
            assert!((i - 2.0 / 3.0).abs() < 1e-14, "{i}");
            for k in 0..n {
                let d: f64 = (0..n).map(|j| r.d[k * n + j] * r.x[j].powi(3)).sum();
                assert!((d - 3.0 * r.x[k].powi(2)).abs() < 1e-11, "{d}");
            }
        }
        for k in 0..12 {
            assert!((r12.x[k] - r36.x[3 * k + 1]).abs() < 1e-15);
        }
    }

    #[test]
    fn jacobi_finds_principal_axes() {
        let (l, v) = eigen([[2.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 5.0]]);
        assert!((l[0] - 1.0).abs() < 1e-14 && (l[1] - 3.0).abs() < 1e-14 && (l[2] - 5.0).abs() < 1e-14);
        let s = 0.5f64.sqrt();
        assert!(
            (v[0] - Vec3::new(s, -s, 0.0)).length() < 1e-14
                || (v[0] - Vec3::new(-s, s, 0.0)).length() < 1e-14
        );
        assert!((v[2] - Vec3::Z).length() < 1e-14);
    }
}
