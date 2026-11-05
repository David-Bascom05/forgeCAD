//! Mass properties from the exact B-rep: volume, area, centroid and inertia (unit density).
//!
//! Each volume integral is a flux through the boundary (divergence theorem), and each face's
//! flux an integral over its trimmed parameter domain D of F(u, v) = Phi(S) . (Su x Sv). Green's
//! theorem turns that into an integral round D's loops: with G(a, b) the integral of F along the
//! inner parameter a from a fixed a0, the face integral is the loop integral of G db.
//!
//! G is exact for the analytic surfaces: F is a polynomial (Gauss-Legendre) or a trigonometric
//! polynomial of known degree along a, fitted once per face; so it is along v on a surface
//! straight in v (ruled, a screw's straight section), where the area's integrand is the root of a
//! quadratic the rule's points give, integrated to a bound. A swept surface (pipe, variable blend)
//! works out its section once per node and integrates the column along v by Clenshaw-Curtis.
//! Other procedural surfaces fit F by Chebyshev tiles, laid out by lines across the face first,
//! where it is smooth enough, else integrate by adaptive Gauss-Kronrod. A corner patch's face
//! that is its whole domain polygon is integrated over the polygon directly instead.
//!
//! The loop integrals run along the edges. A curve with an exact derivative on a surface with
//! exact ones (analytic, or procedural ones that say so), or an intersection curve walked by the
//! distance along its chord (its own parameter is only once differentiable at its chain points),
//! gives uv' exactly: Gauss-Kronrod 7-15 there, the two rules' difference the error; Gauss on 3
//! points on a line where G is a polynomial, the trapezoid round a whole circle. A curve made on
//! its surface (a blend's spring) gives its uv directly, others are found by Newton from the
//! last point's. Elsewhere uv' comes from the interpolant of the uv samples on nested Fejer
//! rules (4, 12, 36 points), the last Chebyshev coefficients the error. Pieces are halved until
//! the error is below the tolerance, within a budget per face; what is left over is in the
//! reported error, as is any gap between an edge and its face's surface.

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

/// The signed volume one shell bounds: negative for a void.
pub fn shell_volume(body: &Body, shell: crate::body::ShellId) -> f64 {
    let size = body
        .shell(shell)
        .faces
        .iter()
        .fold(forge_math::Box3::EMPTY, |b, &f| b.hull(&body.face_box(f)))
        .diagonal();
    // about the shell's own box middle: only the sign and rough size are wanted
    let c = body
        .shell(shell)
        .faces
        .iter()
        .fold(forge_math::Box3::EMPTY, |b, &f| b.hull(&body.face_box(f)))
        .center();
    body.shell(shell)
        .faces
        .iter()
        .filter_map(|&f| face_integrals(body, f, c, size, 1e-4).ok())
        .map(|(v, _)| v[0])
        .sum()
}

/// One face's area, exactly as `mass` integrates it, without the rest of the body.
pub fn face_area(body: &Body, f: FaceId) -> Result<f64, MassError> {
    let n = body.vertices.len().max(1) as f64;
    let c = body
        .vertices
        .iter()
        .fold(Point3::ORIGIN, |a, v| a + (v.point - Point3::ORIGIN) * (1.0 / n));
    let size = body.face_box(f).diagonal().max(1e-9);
    face_integrals(body, f, c, size, REL)
        .map(|(v, _)| v[10])
        .map_err(|why| MassError { face: f, why })
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
    let mut loosest = (FaceId(0), 0.0f64);
    for f in body.face_ids() {
        let (v, er) = face_integrals(body, f, c, size, rel).map_err(|why| MassError { face: f, why })?;
        for k in 0..NF {
            t[k] += v[k];
            e[k] += er[k];
        }
        if er[0] > loosest.1 {
            loosest = (f, er[0]);
        }
        // an integral past what geometry allows is no measurement: a loop's parameters went wrong
        if !v[10].is_finite() || v[10] < -size * size * 1e-6 {
            return Err(MassError {
                face: f,
                why: "the face's area came out negative",
            });
        }
        faces.push((f, v[10]));
    }
    let vol = t[0];
    // shells may overlap (`extrude new`), so their volumes add up past one box round them all: each
    // shell's own box, summed. Empty boxes (a shell with every face removed) count nothing.
    let room: f64 = body
        .shells
        .iter()
        .map(|s| {
            let bx = s
                .faces
                .iter()
                .filter(|&&f| body.face(f).alive)
                .fold(forge_math::Box3::EMPTY, |b, &f| b.hull(&body.face_box(f)));
            if bx.is_empty() {
                0.0
            } else {
                bx.x.width() * bx.y.width() * bx.z.width()
            }
        })
        .sum();
    if vol.abs() > 1.01 * room {
        return Err(MassError {
            face: FaceId(0),
            why: "the volume came out larger than the body's box",
        });
    }
    // the rules did not settle: what they give is off by that much, and a number here reads as measured.
    // (Real parts' bound stays under 1e-4 of the volume; p09444's blend said 10%, a wrong volume by .6%.)
    // (A file's bound holds the gaps it leaves between edges and faces, and an import's mesh checks it.)
    if e[0] > 1e-3 * vol.abs() && !body.from_file {
        return Err(MassError {
            face: loosest.0,
            why: "the face's integral did not settle",
        });
    }
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
    /// A rule with an embedded one (derivatives from the curve's own), rather than Fejer
    /// (derivatives from the interpolant).
    kr: Option<&'static Kronrod>,
}

/// A rule on [-1, 1] (increasing x): weights, and an embedded rule's weights (zero off its
/// points) whose difference bounds the error.
struct Kronrod {
    x: Vec<f64>,
    w: Vec<f64>,
    g: Vec<f64>,
}

/// Gauss-Legendre on 3 points, exact for the polynomials of degree 5 a line on a plane or along
/// a cylinder's axis gives (its embedded rule is itself).
fn gauss3() -> &'static Kronrod {
    static R: OnceLock<Kronrod> = OnceLock::new();
    R.get_or_init(|| Kronrod {
        x: GL3.iter().map(|g| g.0).collect(),
        w: GL3.iter().map(|g| g.1).collect(),
        g: GL3.iter().map(|g| g.1).collect(),
    })
}

/// The trapezoid rule on 12 even steps round a whole period, starting at x = -1 (exact for
/// trigonometric polynomials of degree 11; its every other point, of degree 5).
fn periodic() -> &'static Kronrod {
    static R: OnceLock<Kronrod> = OnceLock::new();
    R.get_or_init(|| Kronrod {
        x: (0..12).map(|k| -1.0 + k as f64 / 6.0).collect(),
        w: vec![1.0 / 6.0; 12],
        g: (0..12)
            .map(|k| if k % 2 == 0 { 1.0 / 3.0 } else { 0.0 })
            .collect(),
    })
}

/// Gauss-Kronrod rules on 7 and 15 points.
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
/// curve's exact derivative. None where the surface's parameters are singular. The surface's
/// parameters are found from `seed` on, walking the piece backwards if `rev`.
fn kronrod_piece<'a>(
    s: &Surface,
    way: &Rc<Way<'a>>,
    (lo, hi): (f64, f64),
    rule: &'static Kronrod,
    seed: Option<Point2>,
    rev: bool,
) -> Option<Piece<'a>> {
    let per = periods(s);
    let (mid, half) = (0.5 * (lo + hi), 0.5 * (hi - lo));
    let (xs, n) = (&rule.x, rule.x.len());
    let mut nodes: Vec<Node> = Vec::with_capacity(n);
    // a curve lying on the surface by construction gives its parameters there
    let on = match **way {
        Way::Curve(Curve::Procedural(pc)) if pc.on_surface(mid).is_some_and(|o| o.0 == *s) => Some(pc),
        _ => None,
    };
    for k in 0..n {
        let x = xs[if rev { n - 1 - k } else { k }];
        if let Some((_, uv, d)) = on.and_then(|pc| pc.on_surface(mid + half * x)) {
            // no gap between edge and surface, so the edge point is not needed
            nodes.push(Node {
                uv: [uv.x, uv.y],
                d: [d.x * half, d.y * half],
                ..Node::default()
            });
            continue;
        }
        let (cp, cd1, known) = way.deriv_on(mid + half * x);
        let prev = nodes.last().map(|q| Point2::new(q.uv[0], q.uv[1])).or(seed);
        let q = match (known, s) {
            (Some((on, uv)), _) if on == s => uv,
            (_, Surface::Procedural(_)) => foot(s, cp, prev).0,
            _ => s.project(cp),
        };
        let mut uv = [q.x, q.y];
        if let Some(prev) = prev {
            for a in 0..2 {
                uv[a] = near(uv[a], [prev.x, prev.y][a], per[a]);
            }
        }
        let sd = d1(s, Point2::new(uv[0], uv[1]));
        let (e, f, g) = (sd.du.dot(sd.du), sd.du.dot(sd.dv), sd.dv.dot(sd.dv));
        let (du, dv) = forge_math::linalg::solve2(e, f, f, g, sd.du.dot(cd1), sd.dv.dot(cd1))?;
        nodes.push(Node {
            uv,
            d: [du * half, dv * half],
            p: cp,
            speed: cd1.length() * half,
            gap: sd.p.distance(cp).max(way.off()),
        });
    }
    if rev {
        nodes.reverse();
    }
    Some(Piece {
        n: nodes,
        sign: 1.0,
        way: way.clone(),
        lo,
        hi,
        kr: Some(rule),
    })
}

/// Whether the way's own derivative is exact and the surface's too, so uv' follows from them.
fn exact(s: &Surface, way: &Way) -> bool {
    let analytic = |s: &Surface| !matches!(s, Surface::Procedural(_));
    let surface = match s {
        Surface::Procedural(p) => p.exact_d1(),
        _ => true,
    };
    surface
        && match way {
            Way::Curve(Curve::Procedural(c)) => c.exact_d1(),
            Way::Curve(_) | Way::Bridge { .. } => true,
            // the tangent is exact where both gradients are, or one gradient and the other's
            // own normal
            Way::Chord { c, on, .. } => match on {
                Some((k, _)) => match [&c.a, &c.b][*k].smooth() {
                    Surface::Procedural(p) => p.exact_d1(),
                    _ => true,
                },
                None => analytic(&c.a) && analytic(&c.b),
            },
        }
}

/// Where the normals of an intersection edge's two surfaces are nearer parallel than this sine, its
/// tangent is not walked: at 1e-2 the ten rims measured came to 1.5e-8 of their area, the error cubic
/// in the bridge's length (1e-3 lands among the rounding, 3e-2 is 27 times worse).
const SIN_TOUCH: f64 = 1e-2;

/// A way to walk an edge with a smooth parameter.
enum Way<'a> {
    Curve(&'a Curve),
    /// The last hair of a curve running into a point where its surfaces touch: there the normals'
    /// cross product is rounding and no plane cuts the curve cleanly, so a straight step from where
    /// they still say. `off` is how far the curve strays from the line, carried as a gap into the error.
    Bridge {
        a: Point3,
        d: Vec3,
        off: f64,
    },
    /// An intersection curve's own parameter only joins smoothly at its chain points; this one
    /// is the distance along a fixed direction, the point solved exactly onto both surfaces.
    Chord {
        c: &'a IntCurve,
        anchor: Point3,
        dir: Vec3,
        knots: Vec<(f64, f64)>,
        /// Where one of the surfaces is procedural (0: a, 1: b), the point is solved in its
        /// parameters (its own implicit value is a search), from the last point's.
        on: Option<(usize, std::cell::Cell<Option<Point2>>)>,
        /// Where each surface's nearest point last was: without a start, a file's spline surface
        /// searches itself whole for every one.
        near: [std::cell::Cell<Option<Point2>>; 2],
    },
}

impl Way<'_> {
    /// How far the edge itself may be from this way's point.
    fn off(&self) -> f64 {
        match self {
            Way::Bridge { off, .. } => *off,
            _ => 0.0,
        }
    }

    fn at(&self, s: f64) -> Point3 {
        match self {
            Way::Curve(c) => c.eval(s),
            Way::Bridge { a, d, .. } => *a + *d * s,
            Way::Chord { .. } => self.deriv_on(s).0,
        }
    }

    /// The point, its derivative, and its parameters on a surface where the way knows them.
    fn deriv_on(&self, s: f64) -> (Point3, Vec3, Option<(&Surface, Point2)>) {
        match self {
            Way::Curve(c) => {
                let (p, d) = c.deriv1(s);
                (p, d, None)
            }
            Way::Bridge { a, d, .. } => (*a + *d * s, *d, None),
            Way::Chord {
                c, anchor, dir, on, ..
            } => {
                if let Some((k, last)) = on {
                    let (ps, other) = if *k == 0 { (&c.a, &c.b) } else { (&c.b, &c.a) };
                    let ps = ps.smooth();
                    // the plane at s can cut the curve twice (a V on a hole's wall): from a seed far
                    // along (a piece made finer starts over) Newton can land on the other arm, which
                    // is as exact as this one. The chain says which arm is this one.
                    let (near, reach) = self.chain_at(s);
                    let on_arm = |p: Point3| {
                        let off = p - near;
                        (off - *dir * dir.dot(off)).length() <= reach
                    };
                    let from_last = last
                        .get()
                        .and_then(|seed| chord_on(ps, other, *anchor, *dir, s, seed));
                    let got = from_last.filter(|x| on_arm(x.0)).or_else(|| {
                        chord_on(ps, other, *anchor, *dir, s, ps.project(self.chord_point(s)))
                            .filter(|x| on_arm(x.0))
                    });
                    match got {
                        Some((p, t, uv)) => {
                            last.set(Some(uv));
                            return (p, t, Some((ps, uv)));
                        }
                        None => last.set(None),
                    }
                }
                let p = self.chord_point(s);
                // along the curve, at unit speed along dir
                let t = c.a.gradient(p).cross(c.b.gradient(p));
                (p, t / t.dot(*dir), None)
            }
        }
    }

    /// The chain's point at chord distance s, and how far off it the curve can be there: a part of
    /// the chain step it falls in.
    fn chain_at(&self, s: f64) -> (Point3, f64) {
        let Way::Chord { c, knots, .. } = self else {
            unreachable!("only a chord way has a chain")
        };
        let i = knots.partition_point(|k| k.1 <= s).clamp(1, knots.len() - 1);
        let ((t0, s0), (t1, s1)) = (knots[i - 1], knots[i]);
        (c.eval(t0 + (t1 - t0) * ((s - s0) / (s1 - s0))), 0.25 * (s1 - s0))
    }

    /// A chord way's point at s, solved onto both surfaces' implicit values.
    fn chord_point(&self, s: f64) -> Point3 {
        match self {
            Way::Curve(c) => c.eval(s),
            Way::Bridge { a, d, .. } => *a + *d * s,
            Way::Chord {
                c, anchor, dir, near, ..
            } => {
                let (start, reach) = self.chain_at(s);
                let mut p = start;
                let tight = 4.0 * f64::EPSILON * (1.0 + (p - Point3::ORIGIN).abs_max());
                let value = |x: &Surface, at: &std::cell::Cell<Option<Point2>>, p: Point3| {
                    if let Surface::Procedural(pr) = x
                        && let Some((f, g, uv)) = pr.value_near(p, at.get())
                    {
                        at.set(Some(uv));
                        return (f, g);
                    }
                    x.implicit_gradient(p)
                };
                for _ in 0..8 {
                    let ((fa, ga), (fb, gb)) = (value(&c.a, &near[0], p), value(&c.b, &near[1], p));
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
                    let step = (bc * -fa + ca * -fb + ab * -fs) / det;
                    // nearly tangent, a step can land on the far side of a tube: the chain knows better
                    if (p + step - start).length() > reach {
                        break;
                    }
                    p += step;
                }
                p
            }
        }
    }
}

/// The point of procedural `ps` meeting analytic `other` at distance s along `dir` from
/// `anchor`, by Newton in ps's parameters from `seed`: the point, the curve's derivative at unit
/// speed along dir, and its parameters. None where Newton strays off ps's domain or stalls.
fn chord_on(
    ps: &Surface,
    other: &Surface,
    anchor: Point3,
    dir: Vec3,
    s: f64,
    seed: Point2,
) -> Option<(Point3, Vec3, Point2)> {
    let dom = ps.domain();
    let mut uv = seed;
    for _ in 0..12 {
        let d = ps.deriv(uv);
        let (fa, ga) = other.implicit_gradient(d.p);
        let fs = dir.dot(d.p - anchor) - s;
        let tight = 4.0 * f64::EPSILON * (1.0 + (d.p - Point3::ORIGIN).abs_max());
        if fa.abs().max(fs.abs()) <= tight {
            let t = ga.cross(d.du.cross(d.dv));
            return Some((d.p, t / t.dot(dir), uv));
        }
        let (x, y) =
            forge_math::linalg::solve2(ga.dot(d.du), ga.dot(d.dv), dir.dot(d.du), dir.dot(d.dv), -fa, -fs)?;
        uv = Point2::new(uv.x + x, uv.y + y);
        let inside = [(uv.x, dom.u), (uv.y, dom.v)].iter().all(|&(x, d)| {
            let slack = 0.1 * (d.hi - d.lo);
            d.period.is_some() || !slack.is_finite() || (d.lo - slack..=d.hi + slack).contains(&x)
        });
        if !inside {
            return None;
        }
    }
    None
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

/// The edge points of a way at points xs (increasing) of [lo, hi], with the surface's uv of
/// each, found from `seed` on (walking backwards if `rev`).
fn points(
    s: &Surface,
    way: &Way,
    (lo, hi): (f64, f64),
    xs: &[f64],
    seed: Option<Point2>,
    rev: bool,
) -> Vec<Node> {
    let (mid, half) = (0.5 * (lo + hi), 0.5 * (hi - lo));
    let mut prev = seed;
    let node = |&x: &f64| {
        let p = way.at(mid + half * x);
        let (q, gap) = foot(s, p, prev);
        prev = Some(q);
        Node {
            uv: [q.x, q.y],
            p,
            gap: gap.max(way.off()),
            ..Node::default()
        }
    };
    if rev {
        let mut out: Vec<Node> = xs.iter().rev().map(node).collect();
        out.reverse();
        out
    } else {
        xs.iter().map(node).collect()
    }
}

fn uv(q: &Node) -> Point2 {
    Point2::new(q.uv[0], q.uv[1])
}

/// The point and first derivatives (a procedural surface may skip the second).
fn d1(s: &Surface, uv: Point2) -> forge_geom::surface::SurfDeriv {
    match s {
        Surface::Procedural(p) => {
            let mut d = Vec::with_capacity(1);
            p.column_d1(uv.x, &[uv.y], &mut d);
            d[0]
        }
        _ => s.deriv(uv),
    }
}

/// Where `p` (on or by the surface) lies on it, and how far off it is. A procedural surface's
/// own search is costly and stops short of full precision: Newton steps from a nearby point's
/// parameters (or from the search's answer) go straight there.
fn foot(s: &Surface, p: Point3, near: Option<Point2>) -> (Point2, f64) {
    let direct = match s {
        Surface::Procedural(pr) => pr.direct(),
        _ => true,
    };
    if direct {
        let q = s.project(p);
        return (q, s.eval(q).distance(p));
    }
    let newton = |mut uv: Point2| -> Option<(Point2, f64)> {
        for _ in 0..12 {
            let d = d1(s, uv);
            let r = d.p - p;
            let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
            let (x, y) = forge_math::linalg::solve2(e, f, f, g, -r.dot(d.du), -r.dot(d.dv))?;
            uv = Point2::new(uv.x + x, uv.y + y);
            if (d.du * x + d.dv * y).length() <= 1e-15 * (1.0 + (p - Point3::ORIGIN).abs_max()) {
                // within the surface's own parameters: past them a surface may meet itself again
                // (a screw's straight profile run on along its axis)
                let dom = s.domain();
                // a surface that stops at its ends (a file's spline, held at its last point)
                // leaves a last step past them that means nothing: a loop's parameters there
                // would integrate what is not there, 6% of a face off a revolved spline's end
                let clamp = |x: f64, d: forge_geom::Domain1| {
                    if d.period.is_some() {
                        x
                    } else {
                        x.clamp(d.lo, d.hi)
                    }
                };
                let held = Point2::new(clamp(uv.x, dom.u), clamp(uv.y, dom.v));
                if held != uv && s.eval(held) == s.eval(uv) {
                    uv = held;
                }
                let inside = [(uv.x, dom.u), (uv.y, dom.v)].iter().all(|&(x, d)| {
                    let slack = 0.1 * (d.hi - d.lo);
                    d.period.is_some() || !slack.is_finite() || (d.lo - slack..=d.hi + slack).contains(&x)
                });
                let gap = s.eval(uv).distance(p);
                return (inside && gap <= 10.0 * forge_math::tol::LINEAR).then_some((uv, gap));
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
        kr: None,
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
    // each new stretch's surface parameters found from the old node at its start
    let (first, middle) = (Some(uv(&old[0])), Some(uv(&old[old.len() / 2])));
    let mut out = if let Some(kr) = pc.kr {
        let k15 = |a: f64, b: f64, seed| {
            kronrod_piece(s, way, (a, b), kronrod(15), seed, false)
                .unwrap_or_else(|| finish(points(s, way, (a, b), &rule(12).x, seed, false), per, way, a, b))
        };
        if std::ptr::eq(kr, periodic()) {
            // a whole period not drawn by 12 points: quarters
            let q = 0.25 * (hi - lo);
            (0..4)
                .map(|i| k15(lo + q * i as f64, lo + q * (i + 1) as f64, Some(uv(&old[3 * i]))))
                .collect()
        } else if old.len() < 15 {
            vec![k15(lo, hi, first)]
        } else {
            let mid = 0.5 * (lo + hi);
            vec![k15(lo, mid, first), k15(mid, hi, middle)]
        }
    } else if old.len() == 12 {
        let x36 = &rule(36).x;
        let xs: Vec<f64> = (0..36).filter(|j| j % 3 != 1).map(|j| x36[j]).collect();
        let mut rest = points(s, way, (lo, hi), &xs, first, false).into_iter();
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
        [(lo, mid, first), (mid, hi, middle)]
            .map(|(a, b, seed)| finish(points(s, way, (a, b), &rule(12).x, seed, false), per, way, a, b))
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

/// How far the coarser rule's interpolant misses the piece's other points, in u and in v.
fn trace_miss(pc: &Piece) -> [f64; 2] {
    let m = pc.n.len();
    let (r, rc) = (rule(m), rule(m / 3));
    std::array::from_fn(|a| {
        (0..m)
            .filter(|j| j % 3 != 1)
            .map(|j| {
                let x = r.x[j];
                let (mut num, mut den) = (0.0, 0.0);
                for k in 0..m / 3 {
                    let c = rc.lam[k] / (x - rc.x[k]);
                    num += c * pc.n[3 * k + 1].uv[a];
                    den += c;
                }
                (num / den - pc.n[j].uv[a]).abs()
            })
            .fold(0.0, f64::max)
    })
}

/// A piece whose trace is known only to `miss` in the surface's parameters: its boundary may be off
/// by that much, carried into the error as a gap between edge and surface.
fn carry_miss(s: &Surface, pc: &mut Piece, miss: [f64; 2]) {
    let m = pc.n.len();
    let d = d1(s, uv(&pc.n[m / 2]));
    let off = 2.0 * (miss[0] * d.du.length() + miss[1] * d.dv.length());
    let r = rule(m);
    let p0 = pc.n[0].p;
    let speeds: Vec<f64> = (0..m)
        .map(|i| {
            let row = &r.d[i * m..(i + 1) * m];
            row.iter()
                .zip(&pc.n)
                .fold(Vec3::ZERO, |acc, (w, q)| acc + (q.p - p0) * *w)
                .length()
        })
        .collect();
    for (q, v) in pc.n.iter_mut().zip(speeds) {
        q.gap = q.gap.max(off);
        q.speed = v;
    }
}

/// Past this miss in parameters, a miss that halving does not shrink is still the trace's shape (a
/// kink, say) and worth following; under it, on the parts measured, it is the surface's own noise
/// (research/mass-noise-floor).
const NOISE_MISS: f64 = 1e-9;

/// [lo, hi] on 12 points if 4 already draw it, else on 36 if 12 do, else in halves until they do,
/// or until halving stops helping: an offset surface's nearest points hold to a part in 1e11 or so,
/// and no finer piece draws that. Pieces go out in the order the loop walks them (backwards if
/// `rev`), each one's surface parameters found from where the last one ended (`seed`). `parent` is
/// the enclosing piece's miss.
#[allow(clippy::too_many_arguments)]
fn refine<'a>(
    s: &Surface,
    way: &Rc<Way<'a>>,
    (lo, hi): (f64, f64),
    whole: bool,
    rel: f64,
    depth: u32,
    parent: [f64; 2],
    seed: &mut Option<Point2>,
    rev: bool,
    out: &mut Vec<Piece<'a>>,
) {
    // `face_integrals` says so, once back
    if forge_geom::budget::over() {
        return;
    }
    let mut push = |pc: Piece<'a>, seed: &mut Option<Point2>| {
        *seed = Some(uv(if rev { &pc.n[0] } else { &pc.n[pc.n.len() - 1] }));
        out.push(pc);
    };
    // a line's uv runs linearly on a plane or along a cylinder's axis, where G is a polynomial
    // of degree 4 along it; a whole circle or ellipse goes round a period
    let kr = match **way {
        Way::Curve(Curve::Line(l)) => match s {
            Surface::Plane(_) => gauss3(),
            Surface::Cylinder(c) if l.dir.get().cross(c.frame.z().get()).length() <= 1e-14 => gauss3(),
            _ => kronrod(7),
        },
        _ if whole => periodic(),
        _ => kronrod(15),
    };
    if exact(s, way)
        && let Some(pc) = kronrod_piece(s, way, (lo, hi), kr, *seed, rev)
    {
        push(pc, seed);
        return;
    }
    let pc = finish(
        points(s, way, (lo, hi), &rule(12).x, *seed, rev),
        periods(s),
        way,
        lo,
        hi,
    );
    if resolved(&pc, rel) {
        push(pc, seed);
        return;
    }
    let mut pc = finer(s, &pc).pop().expect("one piece");
    if resolved(&pc, rel) {
        push(pc, seed);
        return;
    }
    let miss = trace_miss(&pc);
    let floor = (0..2).all(|a| miss[a] >= 0.5 * parent[a] && miss[a] <= NOISE_MISS);
    if depth == 0 || floor {
        carry_miss(s, &mut pc, miss);
        push(pc, seed);
        return;
    }
    let mid = 0.5 * (lo + hi);
    let halves = if rev {
        [(mid, hi), (lo, mid)]
    } else {
        [(lo, mid), (mid, hi)]
    };
    for h in halves {
        refine(s, way, h, false, rel, depth - 1, miss, seed, rev, out);
    }
}

/// A coedge's pieces in the order its loop walks them.
fn coedge_pieces<'a>(
    body: &'a Body,
    s: &Surface,
    c: CoedgeId,
    rel: f64,
    seed: &mut Option<Point2>,
) -> Result<Vec<Piece<'a>>, &'static str> {
    let co = body.coedge(c);
    let e = body.edge(co.edge);
    // a spring round a traced spine is walked round the spine walked by chord distance
    let (curve, range) = e.curve.smooth(e.range).unwrap_or((&e.curve, e.range));
    let (lo, hi) = (range.lo, range.hi);
    let rev = !co.forward;
    // stretches of a smooth way, in the edge's order
    let mut ways: Vec<(Rc<Way<'a>>, f64, f64, bool)> = Vec::new();
    match curve {
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
            // where the surfaces touch at an end of the edge (two rims' crown), stop where they
            // still cross: past that the curve's tangent is noise
            let sin = |t: f64| {
                let p = ic.eval(t);
                let (na, nb) = (ic.a.gradient(p), ic.b.gradient(p));
                na.cross(nb).length() / (na.length() * nb.length()).max(1e-300)
            };
            let inward = |end: f64, into: f64| -> f64 {
                if sin(end) >= SIN_TOUCH {
                    return end;
                }
                let (mut near, mut far) = (end, end);
                while (far - end) * into < (hi - lo) && sin(far) < SIN_TOUCH {
                    near = far;
                    far = (far + into).clamp(lo, hi);
                }
                for _ in 0..40 {
                    let m = 0.5 * (near + far);
                    if sin(m) < SIN_TOUCH {
                        near = m;
                    } else {
                        far = m;
                    }
                }
                far
            };
            let (from, to) = (inward(lo, 1.0), inward(hi, -1.0));
            let (from, to) = if from < to { (from, to) } else { (lo, hi) };
            let bridge = |x: f64, y: f64, ways: &mut Vec<(Rc<Way<'a>>, f64, f64, bool)>| {
                let (a, b) = (ic.eval(x), ic.eval(y));
                if let Some(d) = (b - a).normalize() {
                    let off = [0.25, 0.5, 0.75]
                        .iter()
                        .map(|&f| ic.eval(x + (y - x) * f).distance(a + (b - a) * f))
                        .fold(0.0, f64::max);
                    let way = Way::Bridge { a, d: d.get(), off };
                    ways.push((Rc::new(way), 0.0, a.distance(b), false));
                }
            };
            // a link at a time: a chain that resolves the curve's last turn is nearer than one chord
            let links = |x: f64, y: f64, ways: &mut Vec<(Rc<Way<'a>>, f64, f64, bool)>| {
                let mut a = x;
                let mut k = x.floor() + 1.0;
                while k < y - 1e-9 {
                    if k - a > 1e-9 {
                        bridge(a, k, ways);
                        a = k;
                    }
                    k += 1.0;
                }
                bridge(a, y, ways);
            };
            if from > lo {
                links(lo, from, &mut ways);
            }
            let end = hi;
            let (lo, hi) = (from, to);
            let mut t0 = lo;
            while t0 < hi {
                // pieces over which the curve turns less than 30 degrees: the distance along
                // their chord then runs one way
                let tan = |t: f64| {
                    ic.guide_tangent(t)
                        .normalize()
                        .map_or(Vec3::new(0.0, 0.0, 0.0), |d| d.get())
                };
                let tan0 = tan(t0);
                // the next chain point, or the end when that is a hair away
                let next = |t: f64| {
                    let k = (t + 1e-9).floor() + 1.0;
                    if hi - k <= 1e-9 { hi } else { k }
                };
                let mut t1 = next(t0);
                while t1 < hi && tan(t1).dot(tan0) >= 0.866 {
                    t1 = next(t1);
                }
                // a chain step can hold the whole turn (the tip of a curve): stop where it passes 30
                let mut from = (t1 - 1.0).max(t0);
                for _ in 0..12 {
                    if tan(t1).dot(tan0) >= 0.866 || t1 - from <= 1e-9 {
                        break;
                    }
                    let mid = 0.5 * (from + t1);
                    if tan(mid).dot(tan0) >= 0.866 {
                        from = mid;
                    } else {
                        t1 = mid;
                    }
                }
                if t1 - t0 < 1e-6 {
                    t1 = next(t0);
                }
                // (a stop inside a chain step lies on the curve only as near as the chain does: both
                // stretches there meet at the same exact point)
                let exact = |t: f64| {
                    let p = ic.eval(t);
                    if t == lo || t == hi || t.fract() == 0.0 {
                        return p;
                    }
                    let mut p = p;
                    for _ in 0..4 {
                        // the least step onto both
                        let ((fa, na), (fb, nb)) = (ic.a.implicit_gradient(p), ic.b.implicit_gradient(p));
                        let (aa, ab, bb) = (na.dot(na), na.dot(nb), nb.dot(nb));
                        let det = aa * bb - ab * ab;
                        if det <= 1e-12 * aa * bb {
                            break;
                        }
                        p += (na * (ab * fb - bb * fa) + nb * (ab * fa - aa * fb)) / det;
                    }
                    p
                };
                let anchor = exact(t0);
                let end = exact(t1);
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
                let on = match (&ic.a, &ic.b) {
                    (Surface::Procedural(_), Surface::Procedural(_)) => None,
                    (Surface::Procedural(_), _) => Some((0, Default::default())),
                    (_, Surface::Procedural(_)) => Some((1, Default::default())),
                    _ => None,
                };
                let way = Rc::new(Way::Chord {
                    c: ic,
                    anchor,
                    dir,
                    knots,
                    on,
                    near: Default::default(),
                });
                ways.push((way, 0.0, s1, false));
                t0 = t1;
            }
            if to < end {
                links(to, end, &mut ways);
            }
        }
        curve => {
            // a whole circle or ellipse: one piece round its period
            // (not on a procedural surface: its parameters round the curve need not be near trigonometric)
            let whole = matches!(curve, Curve::Circle(_) | Curve::Ellipse(_))
                && range.width() >= dmath::TAU * (1.0 - 1e-12)
                && !matches!(s, Surface::Procedural(_));
            let k = match curve {
                _ if whole => 1.0,
                Curve::Circle(_) | Curve::Ellipse(_) => (range.width() / dmath::FRAC_PI_2).ceil().max(1.0),
                // pieces bending about as much as a quarter of a circle: a chord of length l
                // sags 0.19 l there, and a curve's chord count goes as the root of 1 / sag
                // (research/mass-props)
                Curve::Procedural(pc) => {
                    let l = pc.speed_bound(range) * range.width();
                    let n = pc.segments(range, 1e-4 * l) as f64;
                    (n * n * 1e-4 / 0.19).ceil().clamp(1.0, 256.0)
                }
                _ => 1.0,
            };
            let way = Rc::new(Way::Curve(curve));
            // even steps, and every point where the curve's parameter is not smooth
            let mut cuts: Vec<f64> = (1..k as usize).map(|i| lo + (hi - lo) * (i as f64 / k)).collect();
            cuts.extend(curve.kinks(range));
            cuts.sort_by(f64::total_cmp);
            let mut a = lo;
            for b in cuts.into_iter().chain([hi]) {
                if b > a {
                    ways.push((way.clone(), a, b, whole));
                    a = b;
                }
            }
        }
    }
    if rev {
        ways.reverse();
    }
    let mut out = Vec::new();
    for (way, a, b, whole) in ways {
        refine(
            s,
            &way,
            (a, b),
            whole,
            rel,
            8,
            [f64::INFINITY; 2],
            seed,
            rev,
            &mut out,
        );
    }
    if forge_geom::budget::over() {
        return Err(TOO_LONG);
    }
    if rev {
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

fn trace_loop<'a>(
    body: &'a Body,
    s: &Surface,
    lp: LoopId,
    rel: f64,
    sigma: f64,
) -> Result<LoopTrace<'a>, &'static str> {
    let per = periods(s);
    let cos: Vec<CoedgeId> = body.lp(lp).coedges().collect();
    let sing: Vec<bool> = cos
        .iter()
        .map(|&c| {
            let p = body.vertex(body.coedge_start(c)).point;
            let uv = s.project(p);
            s.normal(uv).is_none() || matches!(s, Surface::Torus(_)) && s.singular(uv)
        })
        .collect();
    let rot = sing.iter().position(|&b| b).unwrap_or(0);
    let mut runs: Vec<Vec<Piece<'a>>> = Vec::new();
    let mut jumps = Vec::new();
    let mut prev: Option<[f64; 2]> = None;
    let mut rise = 0.0;
    let mut seed: Option<Point2> = None;
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
        for (j, mut pc) in coedge_pieces(body, s, cos[k], rel, &mut seed)?
            .into_iter()
            .enumerate()
        {
            if let Some(r) = prev {
                if fresh && j == 0 && sing[k] {
                    jumps.extend(jump(r, pc.n[0].uv));
                }
                let across = fresh && j == 0 && sing[k] && per[0].is_some() && rise != 0.0;
                for a in 0..2 {
                    let mut at = near(pc.n[0].uv[a], r[a], per[a]);
                    // through a pole the angle jumps, and which way is the face's side: walking
                    // up v with the face on the left, the next stretch lies at a lesser angle
                    if across && a == 0 && (at - r[0]).abs() > 1e-9 {
                        let p = per[0].expect("just checked");
                        let want_less = rise * sigma > 0.0;
                        if want_less && at > r[0] {
                            at -= p;
                        } else if !want_less && at < r[0] {
                            at += p;
                        }
                    }
                    let d = at - pc.n[0].uv[a];
                    if d != 0.0 {
                        pc.n.iter_mut().for_each(|q| q.uv[a] += d);
                    }
                }
            }
            rise = pc.n[pc.n.len() - 1].uv[1] - pc.n[0].uv[1];
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

/// Clenshaw-Curtis weights on the n + 1 Lobatto points of [-1, 1], and the interpolant's last
/// two Chebyshev coefficients as weights on the values.
struct Curtis {
    w: Vec<f64>,
    tail: [Vec<f64>; 2],
}

fn curtis_rule(n: usize) -> &'static Curtis {
    static R: OnceLock<[Curtis; 2]> = OnceLock::new();
    let make = |n: usize| {
        let m = coefficient_matrix(n);
        // the integral of T_j over [-1, 1]
        let it = |j: usize| {
            if j.is_multiple_of(2) {
                2.0 / (1.0 - (j * j) as f64)
            } else {
                0.0
            }
        };
        Curtis {
            w: (0..=n)
                .map(|k| (0..=n).map(|j| m[j * (n + 1) + k] * it(j)).sum())
                .collect(),
            tail: [n - 1, n].map(|j| (0..=n).map(|k| m[j * (n + 1) + k]).collect()),
        }
    };
    &R.get_or_init(|| [make(16), make(32)])[usize::from(n == 32)]
}

/// Lobatto points over (lo, hi) for degree n: -cos(k pi / n) mapped, k = 0..=n.
fn lobatto(n: usize, (lo, hi): (f64, f64)) -> Vec<f64> {
    (0..=n)
        .map(|k| 0.5 * (lo + hi) - 0.5 * (hi - lo) * dmath::cos(k as f64 * dmath::PI / n as f64))
        .collect()
}

/// Whether Chebyshev coefficients' last two (`tail`) are negligible against all of them
/// (`scale`) or over a uv area `area` against the face's floor, or stuck at the surface's own
/// rounding (no smaller than `was`, with half the points).
fn settled(cx: &Ctx, area: f64, tail: &F, scale: &F, was: &F) -> bool {
    (0..NF).all(|m| {
        tail[m] <= 1e-2 * cx.rel * scale[m] + 1e-2 * cx.floor[m] / area
            || (tail[m] <= cx.rel * scale[m] && tail[m] >= 0.3 * was[m])
    })
}

impl Cheb {
    /// Interpolants over tiles of the box. Lines across it lay the tiles out first, at a few
    /// evaluations each: along b, pieces 32 degrees draw; along a, the degree each piece needs.
    /// None once `left` evaluations are spent.
    fn tiles(cx: &Ctx, a: (f64, f64), b: (f64, f64), left: &mut usize, out: &mut Vec<Cheb>) -> Option<()> {
        let mid = |r: (f64, f64)| 0.5 * (r.0 + r.1);
        let lines = |r: (f64, f64)| [r.0, mid(r), r.1];
        let area = (a.1 - a.0) * (b.1 - b.0);
        let mut pieces = Vec::new();
        let mut todo = vec![(b, 6u32)];
        while let Some((bi, depth)) = todo.pop() {
            let nb = lines(a).iter().try_fold(8, |n, &x| {
                Some(n.max(Cheb::line(cx, bi, area, 32, left, |ys| {
                    ys.iter().map(|&y| cx.f(x, y)).collect()
                })?))
            });
            match nb {
                Some(nb) => pieces.push((bi, nb)),
                None if depth > 0 && *left > 0 => {
                    todo.push(((mid(bi), bi.1), depth - 1));
                    todo.push(((bi.0, mid(bi)), depth - 1));
                }
                None => return None,
            }
        }
        for (bi, nb) in pieces {
            let na = lines(bi)
                .iter()
                .try_fold(16, |n, &y| {
                    Some(n.max(Cheb::line(cx, a, area, 64, left, |xs| cx.column(xs, y))?))
                })
                .unwrap_or(64);
            Cheb::grow(cx, a, bi, (na, nb), 4, left, out)?;
        }
        Some(())
    }

    /// The degree (a power of two from 8, at most `max`) a Chebyshev interpolant of g over r
    /// needs; None past `max` (or g not finite, or `left` spent).
    fn line(
        cx: &Ctx,
        r: (f64, f64),
        area: f64,
        max: usize,
        left: &mut usize,
        g: impl Fn(&[f64]) -> Vec<F>,
    ) -> Option<usize> {
        let mut vals: Vec<F> = Vec::new();
        let mut was = [f64::INFINITY; NF];
        let mut n = 8;
        loop {
            let xs = lobatto(n, r);
            *left = left.checked_sub(n + 1 - vals.len())?;
            // the new points: all of them at first, then the odd ones
            let new: Vec<f64> = xs
                .iter()
                .enumerate()
                .filter(|(k, _)| vals.is_empty() || k % 2 == 1)
                .map(|(_, &x)| x)
                .collect();
            let mut fresh = g(&new).into_iter();
            let v: Vec<F> = (0..=n)
                .map(|k| {
                    if !vals.is_empty() && k % 2 == 0 {
                        vals[k / 2]
                    } else {
                        fresh.next().expect("a value")
                    }
                })
                .collect();
            if v.iter().flatten().any(|x| !x.is_finite()) {
                return None;
            }
            vals = v;
            let c = coefficients(&vals, n, 0);
            let (mut tail, mut scale) = ([0.0; NF], [0.0; NF]);
            for (i, ci) in c.iter().enumerate() {
                for m in 0..NF {
                    scale[m] += ci[m].abs();
                    if i + 2 > n {
                        tail[m] += ci[m].abs();
                    }
                }
            }
            if settled(cx, area, &tail, &scale, &was) {
                return Some(n);
            }
            was = tail;
            n *= 2;
            if n > max {
                return None;
            }
        }
    }

    /// The tile's interpolant from degrees (na, nb) up; a tile 64 degrees along a, or 32 along
    /// b, cannot draw is halved that way.
    fn grow(
        cx: &Ctx,
        a: (f64, f64),
        b: (f64, f64),
        start: (usize, usize),
        depth: u32,
        left: &mut usize,
        out: &mut Vec<Cheb>,
    ) -> Option<()> {
        match Cheb::fit(cx, a, b, start, left) {
            Ok(c) => out.push(c),
            Err((sa, sb)) if depth > 0 && (sa || sb) => {
                let half = |r: (f64, f64), s: bool| {
                    if s {
                        vec![(r.0, 0.5 * (r.0 + r.1)), (0.5 * (r.0 + r.1), r.1)]
                    } else {
                        vec![r]
                    }
                };
                let next = (
                    if sa { start.0 / 2 } else { start.0 }.max(16),
                    if sb { start.1 / 2 } else { start.1 }.max(8),
                );
                for &x in &half(a, sa) {
                    for &y in &half(b, sb) {
                        Cheb::grow(cx, x, y, next, depth - 1, left, out)?;
                    }
                }
            }
            Err(_) => return None,
        }
        Some(())
    }

    /// Interpolates F at Chebyshev-Lobatto points from degrees `start`, doubling the degree along
    /// a or b while its last coefficients there are not negligible. Err: which ways it needs
    /// more than 64 along a or 32 along b (neither when F is not finite or `left` runs out).
    fn fit(
        cx: &Ctx,
        a: (f64, f64),
        b: (f64, f64),
        start: (usize, usize),
        left: &mut usize,
    ) -> Result<Cheb, (bool, bool)> {
        let (mut na, mut nb) = start;
        let mut last = ([f64::INFINITY; NF], [f64::INFINITY; NF]);
        let mut vals: Vec<F> = Vec::new();
        let (mut xa, mut xb) = (Vec::new(), Vec::new());
        loop {
            // samples on the new grid, reusing those on the old one (every other point)
            let (ya, yb) = (lobatto(na, a), lobatto(nb, b));
            let (oa, ob) = (xa.len().saturating_sub(1), xb.len().saturating_sub(1));
            *left = left.checked_sub((na + 1) * (nb + 1)).ok_or((false, false))?;
            let old = |n: usize, o: usize, k: usize| {
                if o == n {
                    Some(k)
                } else if o > 0 && 2 * o == n && k.is_multiple_of(2) {
                    Some(k / 2)
                } else {
                    None
                }
            };
            // column by column along a (a swept surface's section once per column)
            let mut v = vec![[0.0; NF]; (na + 1) * (nb + 1)];
            for (j, &w) in yb.iter().enumerate() {
                let q = old(nb, ob, j);
                let new: Vec<usize> = (0..=na)
                    .filter(|&i| q.is_none() || old(na, oa, i).is_none())
                    .collect();
                let fresh = cx.column(&new.iter().map(|&i| ya[i]).collect::<Vec<_>>(), w);
                for (i, f) in new.into_iter().zip(fresh) {
                    v[i * (nb + 1) + j] = f;
                }
                if let Some(q) = q {
                    for i in 0..=na {
                        if let Some(p) = old(na, oa, i) {
                            v[i * (nb + 1) + j] = vals[p * (ob + 1) + q];
                        }
                    }
                }
            }
            if v.iter().flatten().any(|x| !x.is_finite()) {
                return Err((false, false));
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
            let area = (a.1 - a.0) * (b.1 - b.0);
            let (oka, okb) = (
                settled(cx, area, &ta, &scale, &last.0),
                settled(cx, area, &tb, &scale, &last.1),
            );
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
/// The matrix taking values at the n + 1 Lobatto points to Chebyshev coefficients: row j,
/// column k.
fn coefficient_matrix(n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![1.0];
    }
    let mut m = vec![0.0; (n + 1) * (n + 1)];
    for k in 0..=n {
        let t = chebyshev(-dmath::cos(k as f64 * dmath::PI / n as f64), n);
        for (j, tj) in t.iter().enumerate() {
            let half = |i: usize| if i == 0 || i == n { 0.5 } else { 1.0 };
            m[j * (n + 1) + k] = 2.0 / n as f64 * tj * half(k) * half(j);
        }
    }
    m
}

fn coefficients(vals: &[F], na: usize, nb: usize) -> Vec<F> {
    let (ma, mb) = (coefficient_matrix(na), coefficient_matrix(nb));
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

/// The integral over [-1, 1] of the root of the quadratic through `q` at Gauss's 3 nodes, and a
/// bound on its error; None where it has no singularity off the line (|n| constant or linear along
/// it), so the 3 nodes' rule is exact.
fn root_line(q: [f64; 3]) -> Option<(f64, f64)> {
    let s = GL3[2].0;
    let (c0, c1, c2) = (
        q[1],
        (q[2] - q[0]) / (2.0 * s),
        (q[2] + q[0] - 2.0 * q[1]) / (2.0 * s * s),
    );
    if !(c2 > 1e-14 * (c0 + c1.abs()) && c2.is_finite()) {
        return None;
    }
    // c2 ((z - re)^2 + im^2)
    let (re, im) = (
        -c1 / (2.0 * c2),
        (4.0 * c0 * c2 - c1 * c1).max(0.0).sqrt() / (2.0 * c2),
    );
    if im == 0.0 && re.abs() >= 1.0 {
        return None;
    }
    Some(root_piece(re, im, c2, (-1.0, 1.0), 24))
}

/// The integral of sqrt(c2 ((z - re)^2 + im^2)) over [l, r] and a bound on its error: Gauss on 7
/// points, halved until the bound is below rounding. The integrand is analytic inside the Bernstein
/// ellipse through re +- i im, where it is at most M, so the error is at most
/// 64 M / (15 (rho^2 - 1) rho^12) of the half width (Trefethen, SIAM Review 50, 2008, Theorem 4.5),
/// and never more than the integral and the sum can be apart. research/mass-props/ruled-area/.
fn root_piece(re: f64, im: f64, c2: f64, (l, r): (f64, f64), depth: u32) -> (f64, f64) {
    let (m, h) = (0.5 * (l + r), 0.5 * (r - l));
    let f = |z: f64| (c2 * ((z - re) * (z - re) + im * im)).sqrt();
    let mut v = WG[3] * f(m);
    for k in 0..3 {
        let x = h * XGK[2 * k + 1];
        v += WG[k] * (f(m - x) + f(m + x));
    }
    v *= h;
    // the singularity seen from [-1, 1], and the ellipse through it
    let (x, y) = ((re - m) / h, im / h);
    let a = 0.5 * (((x - 1.0) * (x - 1.0) + y * y).sqrt() + ((x + 1.0) * (x + 1.0) + y * y).sqrt());
    let rho = a + (a * a - 1.0).max(0.0).sqrt();
    let (r2, k) = (rho * rho, c2.sqrt() * h);
    let most = 4.0 * h * k * (x.abs() + 1.0 + y);
    let bound = if r2 - 1.0 <= 1e-12 {
        most
    } else {
        let r6 = r2 * r2 * r2;
        (h * 64.0 * k * (a + (x * x + y * y).sqrt()) / (15.0 * (r2 - 1.0) * r6 * r6)).min(most)
    };
    if bound <= 1e-15 * v || depth == 0 {
        return (v, bound);
    }
    let (v0, e0) = root_piece(re, im, c2, (l, m), depth - 1);
    let (v1, e1) = root_piece(re, im, c2, (m, r), depth - 1);
    (v0 + v1, e0 + e1)
}

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
    /// Whether a point has asked the fit for a value past its box.
    left_fit: std::cell::Cell<bool>,
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
        self.of(&self.s.deriv(uv))
    }

    fn of(&self, d: &forge_geom::surface::SurfDeriv) -> F {
        let n = d.du.cross(d.dv);
        let mut f = integrand(d.p - self.c, n);
        if let Surface::Torus(t) = self.s
            && t.is_spindle()
        {
            // r rho, signed: it changes sign through the axis, where |du x dv| has a kink the fit
            // round the tube cannot follow
            f[10] = -n.dot(d.dvv) / t.minor;
        }
        if let Surface::Sphere(sp) = self.s {
            // r^2 cos v: |du x dv| as the trigonometric polynomial it is between the poles
            f[10] = n.dot(d.p - sp.frame.origin) / sp.radius;
        }
        f
    }

    /// F at each of `aa` at b: a swept surface works out its section at u once for a column
    /// along v.
    fn column(&self, aa: &[f64], b: f64) -> Vec<F> {
        match self.s {
            Surface::Procedural(p) if self.ax == 1 && p.swept() => {
                let mut d = Vec::with_capacity(aa.len());
                p.column_d1(b, aa, &mut d);
                d.iter().map(|d| self.of(d)).collect()
            }
            _ => aa.iter().map(|&a| self.f(a, b)).collect(),
        }
    }

    /// G at (a, b), the integral of F over a whole period of a (for a periodic a), and G's error.
    fn g(&self, a: f64, b: f64) -> (F, F, F) {
        let mut g = [0.0; NF];
        let x = a - self.a0;
        match self.inner {
            Inner::Poly(rule) => {
                let mut q = [0.0; 3];
                for (k, &(t, w)) in rule.iter().enumerate() {
                    let f = self.f(self.a0 + 0.5 * x * (1.0 + t), b);
                    axpy(&mut g, 0.5 * x * w, &f);
                    q[k.min(2)] = f[10] * f[10];
                }
                let mut e = [0.0; NF];
                // a rule's degree holds for the moments; along a straight line of a twisted wall the
                // area's |n| is the root of a quadratic, no polynomial, which its 3 values give
                if matches!(self.s, Surface::Procedural(_))
                    && rule.len() == 3
                    && let Some((v, err)) = root_line(q)
                {
                    g[10] = 0.5 * x * v;
                    e[10] = 0.5 * x.abs() * err + 1e-15 * g[10].abs();
                }
                (g, [0.0; NF], e)
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
                // a fit holds just past its box, and blows up not far beyond
                let off =
                    |x: f64, (lo, hi): (f64, f64)| x < lo - 1e-3 * (hi - lo) || x > hi + 1e-3 * (hi - lo);
                if off(a, all.0) || off(b, all.1) {
                    self.left_fit.set(true);
                }
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

    /// Clenshaw-Curtis on 17, then 33, points of [lo, hi] (one column each), the interpolant's
    /// last Chebyshev coefficients bounding the error; None where 33 do not settle it.
    fn curtis(&self, lo: f64, hi: f64, b: f64) -> Option<(F, F)> {
        self.panels.set(self.panels.get().saturating_sub(1));
        let h = 0.5 * (hi - lo);
        let mut vals: Vec<F> = Vec::new();
        for n in [16, 32] {
            let xs = lobatto(n, (lo, hi));
            let new: Vec<f64> = xs
                .iter()
                .enumerate()
                .filter(|(k, _)| vals.is_empty() || k % 2 == 1)
                .map(|(_, &x)| x)
                .collect();
            let mut fresh = self.column(&new, b).into_iter();
            vals = (0..=n)
                .map(|k| {
                    if !vals.is_empty() && k % 2 == 0 {
                        vals[k / 2]
                    } else {
                        fresh.next().expect("a value")
                    }
                })
                .collect();
            let r = curtis_rule(n);
            let (mut sum, mut abs, mut tail) = ([0.0; NF], [0.0; NF], [[0.0; NF]; 2]);
            for (k, f) in vals.iter().enumerate() {
                axpy(&mut sum, h * r.w[k], f);
                for m in 0..NF {
                    abs[m] += (h * r.w[k] * f[m]).abs();
                }
                for (t, row) in tail.iter_mut().zip(&r.tail) {
                    axpy(t, row[k], f);
                }
            }
            if sum.iter().any(|x| !x.is_finite()) {
                return None;
            }
            let e: F =
                std::array::from_fn(|m| 2.0 * h * (tail[0][m].abs() + tail[1][m].abs()) + 1e-15 * abs[m]);
            if (0..NF).all(|m| e[m] <= 0.1 * self.rel * abs[m] + self.floor[m]) {
                return Some((sum, e));
            }
        }
        None
    }

    /// Gauss-Kronrod 7-15 on [lo, hi]: the Kronrod sum, the error estimate, and the sum of |F|.
    fn gk(&self, lo: f64, hi: f64, b: f64) -> (F, F, F) {
        self.panels.set(self.panels.get().saturating_sub(1));
        let (c, h) = (0.5 * (lo + hi), 0.5 * (hi - lo));
        let (mut k, mut g, mut abs) = ([0.0; NF], [0.0; NF], [0.0; NF]);
        let at = |i: usize| if i < 8 { (i, -1.0) } else { (14 - i, 1.0) };
        let xs: Vec<f64> = (0..15)
            .map(|i| {
                let (j, sgn) = at(i);
                c + sgn * h * XGK[j]
            })
            .collect();
        for (i, fx) in self.column(&xs, b).into_iter().enumerate() {
            let (j, _) = at(i);
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
        // a swept surface's column costs one section, however many points
        if matches!(self.s, Surface::Procedural(p) if self.ax == 1 && p.swept())
            && let Some(r) = self.curtis(lo, hi, b)
        {
            return r;
        }
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
    /// Summed |w d G| and |w d|: their ratio is G's size along the run.
    g: F,
    dabs: f64,
}

/// Adds a piece's share to `u`, or None (adding nothing) when its integrand is not resolved and
/// it may still be made finer.
fn integrate(cx: &Ctx, pc: &Piece, u: &mut Acc, last: bool) -> Option<()> {
    let (ax, c) = (cx.ax, cx.c);
    let r = rule(pc.n.len());
    let kr = pc.kr;
    let big = pc.n.iter().fold(0.0f64, |m, q| m.max(q.uv[1 - ax].abs()));
    let flat = pc.n.iter().all(|q| q.d[1 - ax].abs() <= 1e-12 * (1.0 + big));
    let (mut sum, mut phis, mut abs) = ([0.0; NF], [0.0; NF], [0.0; NF]);
    let (mut tail, mut e, mut gauss) = ([[0.0; NF]; 2], [0.0; NF], [0.0; NF]);
    let (mut area, mut db, mut dabs) = (0.0, 0.0, 0.0);
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
        dabs += (w * d).abs();
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
        u.g[m] += abs[m];
    }
    u.dabs += dabs;
    u.area += area;
    u.db += db;
    Some(())
}

const TOO_LONG: &str = "it would take too long to integrate";

/// A face's integrals (flux of each moment's field, then area) and their error bounds.
/// `size` sets the scale below which a piece's share is negligible.
fn face_integrals(body: &Body, f: FaceId, c: Point3, size: f64, rel: f64) -> Result<(F, F), &'static str> {
    let face = body.face(f);
    // a pipe round a traced spine is integrated round the spine walked by chord distance
    let s = face.surface.smooth();
    let sigma = if face.same_sense { 1.0 } else { -1.0 };
    if let Some(r) = whole_patch(body, f, c, size, rel) {
        return Ok(r);
    }
    let loops: Vec<LoopTrace> = face
        .loops
        .iter()
        .map(|&l| trace_loop(body, s, l, rel, sigma))
        .collect::<Result<_, _>>()?;
    if forge_geom::budget::over() {
        return Err(TOO_LONG);
    }
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
        // along v where the surface is a polynomial there (a closed rule), else across any u
        // where it is not smooth, else the shorter way across
        let (su, sv) = p.uv_scale();
        let w = |a: usize| {
            let (lo, hi) = span(&loops, a);
            hi - lo
        };
        let (ulo, uhi) = span(&loops, 0);
        let kinked = ulo < uhi && !p.u_kinks(forge_math::Interval::new(ulo, uhi)).is_empty();
        usize::from(p.v_degree().is_some() || kinked || w(1) * sv < w(0) * su)
    } else if matches!(s, Surface::Cylinder(_))
        || matches!(s, Surface::Cone(_))
            && !face.loops.iter().any(|&l| {
                body.lp(l)
                    .coedges()
                    .any(|c| s.singular(s.project(body.vertex(body.coedge_start(c)).point)))
            })
    {
        // along the axis by Gauss-Legendre (2 or 3 points per node where u moves), or round it
        // by a fit (9 points at each of 4 or 5 v) and a Fourier sum where v moves; a cone's face
        // reaching its apex (one seamed face since whole-turn cones) always along the axis
        let gl = if matches!(s, Surface::Cylinder(_)) { 2 } else { 3 };
        let moving = |a: usize| -> usize {
            let pieces = loops.iter().flat_map(|l| l.runs.iter().flatten());
            pieces
                .filter(|pc| pc.n.iter().any(|q| q.d[a].abs() > 1e-12))
                .map(|pc| pc.n.len())
                .sum()
        };
        usize::from(gl * moving(0) < 9 * (gl + 2) + 2 * moving(1))
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
    } else if let (Surface::Torus(t), true) = (s, wd != 0.0 && ax == 1) {
        if !t.is_spindle() {
            return Err("a loop winds round a face that holds no singular point");
        }
        // up to the pole the face holds: the region runs from the loops to it
        let v = t.pole_v();
        let at = |x: f64| {
            let (lo, hi) = span(&loops, 1);
            near(x, 0.5 * (lo + hi), per[1])
        };
        if wd > 0.0 { at(v) } else { at(-v) }
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
        left_fit: std::cell::Cell::new(false),
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
        (Surface::Procedural(p), 1) if p.v_degree() == Some(1) => {
            // F is of degree 5d - 1 = 4 in v
            cx.inner = Inner::Poly(&GL3);
            None
        }
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
    } else if let Surface::Procedural(p) = s
        && !(ax == 1 && p.swept())
        && matches!(cx.inner, Inner::Adapt)
        && !loops.is_empty()
    {
        // (a swept surface's sections are dear and its columns along v cheap: Gauss-Kronrod
        // there needs a section per node, fewer than a fit's)
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
        let u = if ax == 0 { all.0 } else { all.1 };
        let mut cuts = p.u_kinks(forge_math::Interval::new(u.0, u.1));
        cuts.insert(0, u.0);
        cuts.push(u.1);
        let v = if ax == 0 { all.1 } else { all.0 };
        let mut vcuts = p.v_kinks(forge_math::Interval::new(v.0, v.1));
        vcuts.insert(0, v.0);
        vcuts.push(v.1);
        let mut tiles = Vec::new();
        let mut left = 20_000 + 400 * cuts.len() * vcuts.len();
        let fit = ahi > alo
            && bhi > blo
            && cuts.windows(2).all(|w| {
                vcuts.windows(2).all(|x| {
                    let (a, b) = if ax == 0 {
                        ((w[0], w[1]), (x[0], x[1]))
                    } else {
                        ((x[0], x[1]), (w[0], w[1]))
                    };
                    Cheb::tiles(&cx, a, b, &mut left, &mut tiles).is_some()
                })
            });
        if fit {
            cx.inner = Inner::Cheb(tiles, all);
        }
    }
    let mut units: Vec<Acc> = Vec::new();
    // a loop that left the fit's box (near a pole, where the angle wanders) is measured again by
    // columns alone: a fit past its box is no answer, and mixing the two does not cancel
    loop {
        units.clear();
        for l in &loops {
            for run in &l.runs {
                let mut u = Acc::default();
                for pc in run {
                    let mut work = vec![(8u32, None)];
                    while let Some((depth, piece)) = work.pop() {
                        if forge_geom::budget::over() {
                            return Err(TOO_LONG);
                        }
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
        if !cx.left_fit.get() || !matches!(cx.inner, Inner::Cheb(..)) {
            break;
        }
        cx.inner = Inner::Adapt;
        cx.panels
            .set(20_000 + 20 * loops.iter().flat_map(|l| &l.runs).map(Vec::len).sum::<usize>());
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
    // a closed loop's b comes back to where it began, a period or so along: what its derivatives
    // summed to beyond that they got wrong, whatever the rules thought of them, times G
    let mut at = 0;
    for l in &loops {
        if let ([_], Some(u)) = (&l.runs[..], units.get(at))
            && u.dabs > 0.0
        {
            let left = match per[1 - ax] {
                Some(p) => u.db - p * (u.db / p).round(),
                None => u.db,
            };
            for (e, g) in err.iter_mut().zip(&u.g) {
                *e += left.abs() * g / u.dabs;
            }
        }
        at += l.runs.len();
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

/// A corner patch's face that is its whole domain polygon (its vertices on the polygon's
/// corners, its edges along the sides): the integrals over the polygon directly, by the patch's
/// own rule at 12, 16 and 32 points, the last two's difference the error.
fn whole_patch(body: &Body, f: FaceId, c: Point3, size: f64, rel: f64) -> Option<(F, F)> {
    let face = body.face(f);
    let Surface::Procedural(p) = &face.surface else {
        return None;
    };
    let hp = p.as_any()?.downcast_ref::<forge_geom::corner::HolePatch>()?;
    let [lp] = face.loops[..] else {
        return None;
    };
    let corners = hp.domain_corners();
    let n = corners.len();
    let side = corners[0].distance(corners[1]);
    let off = |x: Point2| {
        (0..n)
            .map(|i| {
                let (a, b) = (corners[i], corners[(i + 1) % n]);
                let t = ((x - a).dot(b - a) / (b - a).length_sq()).clamp(0.0, 1.0);
                x.distance(a.lerp(b, t))
            })
            .fold(f64::INFINITY, f64::min)
    };
    let mut hit = vec![false; n];
    for co in body.lp(lp).coedges() {
        let e = body.edge(body.coedge(co).edge);
        let start = p.project(body.vertex(body.coedge_start(co)).point);
        let mid = p.project(e.curve.eval(e.range.mid()));
        if off(start) > 1e-6 * side || off(mid) > 1e-6 * side {
            return None;
        }
        for (k, h) in hit.iter_mut().enumerate() {
            *h |= start.distance(corners[k]) <= 1e-6 * side;
        }
    }
    if !hit.iter().all(|&h| h) {
        return None;
    }
    let sum = |m: usize| {
        let (mut v, mut abs) = ([0.0; NF], [0.0; NF]);
        let mut d = Vec::with_capacity(1);
        for (x, w) in hp.domain_rule(m) {
            d.clear();
            p.column_d1(x.x, &[x.y], &mut d);
            let fx = integrand(d[0].p - c, d[0].du.cross(d[0].dv));
            axpy(&mut v, w, &fx);
            for k in 0..NF {
                abs[k] += (w * fx[k]).abs();
            }
        }
        (v, abs)
    };
    let floor: F = std::array::from_fn(|m| 1e-6 * rel * size.powi([3, 4, 4, 4, 5, 5, 5, 5, 5, 5, 2][m]));
    // the rule converges geometrically: 12 points draw it to about 2e-11, 16 to rounding
    // (measured: research/mass-props)
    let (mut was, _) = sum(12);
    for m in [16, 32] {
        let (v, abs) = sum(m);
        let e: F = std::array::from_fn(|k| (v[k] - was[k]).abs() + 1e-15 * abs[k]);
        if m == 32 || (0..NF).all(|k| e[k] <= 0.1 * rel * abs[k] + floor[k]) {
            let sigma = if face.same_sense { 1.0 } else { -1.0 };
            let out = std::array::from_fn(|k| if k < 10 { sigma * v[k] } else { v[k] });
            return Some((out, e));
        }
        was = v;
    }
    None
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
pub fn eigen(m: [[f64; 3]; 3]) -> ([f64; 3], [Vec3; 3]) {
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

    /// A cylinder of a file: past its ends it stays at them.
    #[derive(Debug)]
    struct Held;
    impl forge_geom::surface::Procedural for Held {
        fn deriv(&self, uv: Point2) -> forge_geom::surface::SurfDeriv {
            let (s, c) = dmath::sin_cos(uv.x);
            forge_geom::surface::SurfDeriv {
                p: Point3::new(c, s, uv.y.clamp(0.0, 1.0)),
                du: Vec3::new(-s, c, 0.0),
                dv: Vec3::new(0.0, 0.0, 1.0),
                duu: Vec3::new(-c, -s, 0.0),
                duv: Vec3::ZERO,
                dvv: Vec3::ZERO,
            }
        }
        fn domain(&self) -> forge_geom::Domain2 {
            let d = |hi| forge_geom::Domain1 {
                lo: 0.0,
                hi,
                period: None,
            };
            forge_geom::Domain2 { u: d(6.0), v: d(1.0) }
        }
        fn project(&self, _: Point3) -> Point2 {
            Point2::new(0.5, 0.5)
        }
        fn bbox(&self, _: forge_math::Box2) -> forge_math::Box3 {
            forge_math::Box3::EMPTY
        }
        fn implicit(&self, _: Point3) -> f64 {
            0.0
        }
        fn gradient(&self, _: Point3) -> Vec3 {
            Vec3::X
        }
        fn transformed(&self, _: &forge_math::Frame) -> forge_geom::surface::Proc {
            forge_geom::surface::Proc(std::sync::Arc::new(Held))
        }
        fn mesh_radius(&self) -> f64 {
            1.0
        }
    }

    /// A point just past where a surface stops has its foot at the end, not where Newton's last
    /// step went: a loop out there integrated what is not there.
    #[test]
    fn a_foot_stays_on_a_surface_that_stops() {
        let s = Surface::Procedural(forge_geom::surface::Proc(std::sync::Arc::new(Held)));
        let (uv, gap) = foot(&s, Point3::new(1.0, 0.0, 1.0), Some(Point2::new(0.0, 0.7)));
        assert!(uv.y <= 1.0, "{uv:?}");
        assert!(gap <= 1e-9, "{gap}");
        let (uv, _) = foot(&s, Point3::new(1.0, 0.0, 1.0 + 1e-6), Some(Point2::new(0.0, 0.7)));
        assert!(uv.y <= 1.0, "{uv:?}");
    }

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
