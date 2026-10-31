use crate::cdt::{CdtError, Refine, triangulate_refined};
use crate::mesh::TriMesh;
use forge_geom::{Curve, Surface, SurfaceGeom};
use forge_math::{Point2, dmath};
use forge_topo::uv::{align_loops, draw_loop_near};
use forge_topo::{Body, FaceId};

/// How closely the triangles must follow the exact shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quality {
    /// Largest gap between a chord and the true curve or surface (mm).
    pub chord: f64,
    /// Largest angle a single chord may turn through (radians).
    pub max_angle: f64,
}

impl Default for Quality {
    fn default() -> Self {
        Quality {
            chord: 0.01,
            max_angle: dmath::PI / 12.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TessError {
    Face { face: FaceId, error: CdtError },
}

/// Angle step that keeps chords on a circle of radius `r` within quality.
fn arc_step(r: f64, q: &Quality) -> f64 {
    let by_chord = if q.chord >= r {
        dmath::PI
    } else {
        2.0 * dmath::acos(1.0 - q.chord / r)
    };
    by_chord.min(q.max_angle).max(1e-4)
}

/// Chords along an edge. On a face curved both ways a chord along the edge and one across
/// it make a triangle that sags by both their sags: there each gets half of the tolerance,
/// a step shorter by sqrt 2 (`fine`).
fn edge_segments(curve: &Curve, range: forge_math::Interval, q: &Quality, fine: bool) -> usize {
    let width = range.width();
    let n = match curve {
        Curve::Line(_) => 1.0,
        Curve::Circle(c) => width / arc_step(c.radius, q),
        Curve::Ellipse(e) => width / arc_step(e.major, q),
        Curve::Parabola(_) | Curve::Hyperbola(_) => 64.0,
        // the chain already follows the curvature; split each link by the chord its sag allows
        Curve::Intersection(c) => width * (c.sag / q.chord).sqrt().max(1.0),
        Curve::Procedural(c) => c.segments(range, q.chord) as f64,
    };
    // a curved edge gets two chords at least: a face bounded by a line and a short arc
    // still has three corners to triangulate
    let least = if matches!(curve, Curve::Line(_)) { 1 } else { 2 };
    let n = if fine && least == 2 {
        n * core::f64::consts::SQRT_2
    } else {
        n
    };
    (n.ceil() as usize).max(least)
}

/// Whether the surface curves both ways, so a triangle on it sags by its sag along each side.
fn doubly_curved(s: &Surface) -> bool {
    matches!(s, Surface::Sphere(_) | Surface::Torus(_))
}

/// The surface's parameter scale, so distances in the scaled plane roughly
/// match distances on the surface and the triangulation is well shaped.
fn uv_scale(s: &Surface) -> (f64, f64) {
    match s {
        Surface::Plane(_) => (1.0, 1.0),
        Surface::Cylinder(c) => (c.radius, 1.0),
        Surface::Cone(c) => (c.radius.max(1.0), 1.0 / c.cos_a()),
        Surface::Sphere(sp) => (sp.radius, sp.radius),
        Surface::Torus(t) => (t.major + t.minor, t.minor),
        Surface::Procedural(pr) => pr.uv_scale(),
    }
}

/// Grid steps for points inside a face, where the surface curves in both directions.
fn interior_steps(s: &Surface, q: &Quality) -> Option<(f64, f64)> {
    match s {
        Surface::Plane(_) => None,
        // straight along v: columns across u, rows only as far apart as keeps triangles shapely
        // (rows four columns apart let refinement near a boundary run on, splitting edges
        // into ones as long, and leave triangles across two columns: 3.5 chords deep)
        Surface::Cylinder(c) => Some((arc_step(c.radius, q), 2.0 * c.radius * arc_step(c.radius, q))),
        Surface::Cone(c) => {
            let r = c.radius.max(1.0);
            Some((arc_step(r, q), 2.0 * r * arc_step(r, q)))
        }
        // curved both ways: half the tolerance each way (see edge_segments), and a little
        // under, so a cell's diagonal does not sag by just about the whole tolerance and get
        // split by the refinement
        Surface::Sphere(sp) => {
            let a = 0.9 * arc_step(sp.radius, q) / core::f64::consts::SQRT_2;
            Some((a, a))
        }
        Surface::Torus(t) => Some((
            0.9 * arc_step(t.major + t.minor, q) / core::f64::consts::SQRT_2,
            0.9 * arc_step(t.minor, q) / core::f64::consts::SQRT_2,
        )),
        // laid out by `Nodes` instead
        Surface::Procedural(_) => None,
    }
}

/// Bounds on the normal curvature of chords running mostly along u and mostly along v at a
/// regular point (zero at a singular one): each line's own, plus the twist between them.
fn line_curvatures(d: &forge_geom::surface::SurfDeriv) -> (f64, f64) {
    let Some(n) = d.du.cross(d.dv).normalize() else {
        return (0.0, 0.0);
    };
    let n = n.get();
    let (e, g) = (d.du.dot(d.du), d.dv.dot(d.dv));
    if e <= 0.0 || g <= 0.0 {
        return (0.0, 0.0);
    }
    let twist = (d.duv.dot(n) / (e * g).sqrt()).abs();
    ((d.duu.dot(n) / e).abs() + twist, (d.dvv.dot(n) / g).abs() + twist)
}

/// A procedural face's surface sampled at the nodes of an 8 x 8 split of its parameter box: per
/// node the lengths per unit of u and of v, and the tightest radii along each (a blend bends
/// across, hardly along; a corner patch curves tightly in a corner and gently elsewhere).
struct Nodes {
    lo: Point2,
    hi: Point2,
    at: [[(f64, f64, f64, f64); G + 1]; G + 1],
}

const G: usize = 8;

impl Nodes {
    fn new(pr: &forge_geom::surface::Proc, lo: Point2, hi: Point2) -> Nodes {
        let mut at = [[(0.0, 0.0, f64::INFINITY, f64::INFINITY); G + 1]; G + 1];
        let vs: [f64; G + 1] = std::array::from_fn(|j| Nodes::node(lo, hi, 0, j).y);
        let mut col = Vec::with_capacity(G + 1);
        for (i, row) in at.iter_mut().enumerate() {
            col.clear();
            pr.column_derivs(Nodes::node(lo, hi, i, 0).x, &vs, &mut col);
            for (n, d) in row.iter_mut().zip(&col) {
                let (ku, kv) = line_curvatures(d);
                *n = (d.du.length(), d.dv.length(), 1.0 / ku, 1.0 / kv);
            }
        }
        Nodes { lo, hi, at }
    }

    /// Steps from bounds on the second derivatives' sizes over the whole box: a little under half
    /// the tolerance along u, the rest of it on a cell's diagonal.
    fn whole_counts(&self, [uu, uv, vv]: [f64; 3], q: &Quality) -> Counts {
        let c = 0.9 * 8.0 * q.chord;
        let du = if uu > 0.0 {
            (0.5 * c / uu).sqrt()
        } else {
            f64::INFINITY
        };
        let du = du.min(self.hi.x - self.lo.x);
        // vv dv^2 + 2 uv du dv = c - uu du^2
        let room = c - uu * du * du;
        let dv = if vv > 0.0 {
            (-uv * du + ((uv * du) * (uv * du) + vv * room).sqrt()) / vv
        } else if uv > 0.0 {
            room / (2.0 * uv * du)
        } else {
            f64::INFINITY
        };
        let k = |w: f64, d: f64| (w / G as f64 / d).ceil().clamp(1.0, 1e6) as usize;
        [[(k(self.hi.x - self.lo.x, du), k(self.hi.y - self.lo.y, dv)); G]; G]
    }

    fn node(lo: Point2, hi: Point2, i: usize, j: usize) -> Point2 {
        Point2::new(
            lo.x + (hi.x - lo.x) * i as f64 / G as f64,
            lo.y + (hi.y - lo.y) * j as f64 / G as f64,
        )
    }

    /// How many grid steps each cell takes across u and across v: as many as the lengths and
    /// tightest curvatures at the nodes round it need, never more than the face's tightest radius
    /// anywhere does.
    fn counts(&self, s: &Surface, q: &Quality) -> Counts {
        let want = self.want(s, q, false);
        // in whole steps: a cell needing half a step or more as many rounded up, and the cells
        // needing less along a line of them sharing steps, as many as their sum rounded up (a
        // thin face needing three rows across gets three, not one per cell): one may get none
        let share = |w: f64, sum: &mut f64| {
            if w >= 0.5 {
                w.ceil() as usize
            } else {
                let k = (*sum + w).ceil() - sum.ceil();
                *sum += w;
                k as usize
            }
        };
        let mut n = [[(0, 0); G]; G];
        for i in 0..G {
            let (mut su, mut sv) = (0.0f64, 0.0f64);
            for j in 0..G {
                n[j][i].0 = share(want[j][i].0, &mut su);
                n[i][j].1 = share(want[i][j].1, &mut sv);
            }
        }
        n
    }

    /// The steps each cell needs across u and across v, the densest any node of it or its
    /// neighbours needs. Each way's chords sag within the tolerance, or with `shared`, both
    /// ways' together do (a triangle across a cell sags by its sides' sags), each turning through
    /// no more than the largest angle.
    fn want(&self, s: &Surface, q: &Quality, shared: bool) -> [[(f64, f64); G]; G] {
        let r0 = match s {
            Surface::Procedural(pr) => pr.min_radius(),
            _ => f64::INFINITY,
        };
        let step = |r: f64| {
            let r = r.max(r0);
            if r.is_finite() {
                arc_step(r, q) * r
            } else {
                f64::INFINITY
            }
        };
        let (cw, ch) = (
            (self.hi.x - self.lo.x) / G as f64,
            (self.hi.y - self.lo.y) / G as f64,
        );
        // the sag per unit step squared along a line of length l per unit and radius r, and the
        // longest step turning through the largest angle
        let sag = |l: f64, r: f64| if r.is_finite() { l * l / (8.0 * r) } else { 0.0 };
        let turn = |l: f64, r: f64| {
            if l > 0.0 && r.is_finite() {
                q.max_angle * r / l
            } else {
                f64::INFINITY
            }
        };
        let longest = |a: f64, room: f64| {
            if a > 0.0 {
                (room.max(0.0) / a).sqrt()
            } else {
                f64::INFINITY
            }
        };
        let c = 0.9 * q.chord;
        let split = |(lu, lv, ru, rv): (f64, f64, f64, f64)| {
            let (ru, rv) = (ru.max(r0), rv.max(r0));
            let (au, av, tu, tv) = (sag(lu, ru), sag(lv, rv), turn(lu, ru), turn(lv, rv));
            // half the tolerance each way, or where one way turns too far first, the rest the other
            let sags = |a: f64, d: f64| if d.is_finite() { a * d * d } else { 0.0 };
            let (du, dv) = if tu <= longest(au, 0.5 * c) {
                (tu, tv.min(longest(av, c - sags(au, tu))))
            } else if tv <= longest(av, 0.5 * c) {
                (tu.min(longest(au, c - sags(av, tv))), tv)
            } else {
                (longest(au, 0.5 * c), longest(av, 0.5 * c))
            };
            (1.0 / du, 1.0 / dv)
        };
        // per node, the steps per unit u and v it needs
        let per = self.at.map(|row| {
            row.map(|n| {
                if shared {
                    split(n)
                } else {
                    (n.0 / step(n.2), n.1 / step(n.3))
                }
            })
        });
        let mut want = [[(0.0, 0.0); G]; G];
        for (ci, row) in want.iter_mut().enumerate() {
            for (cj, c) in row.iter_mut().enumerate() {
                let (mut du, mut dv) = (0.0f64, 0.0f64);
                for row in &per[ci.saturating_sub(1)..(ci + 3).min(G + 1)] {
                    for &n in &row[cj.saturating_sub(1)..(cj + 3).min(G + 1)] {
                        du = du.max(n.0);
                        dv = dv.max(n.1);
                    }
                }
                *c = ((cw * du).min(1e6), (ch * dv).min(1e6));
            }
        }
        want
    }

    /// The layout scale that makes the grid's mean cell square, so the Delaunay joins neighbours.
    fn scale(&self, n: &Counts) -> (f64, f64) {
        let (mut u, mut v) = (0.0, 0.0);
        for row in n {
            for &(a, b) in row {
                u += a as f64;
                v += b as f64;
            }
        }
        let side = |k: f64, w: f64| {
            if w > 0.0 {
                k.max(G as f64) / G as f64 / w
            } else {
                1.0
            }
        };
        (side(u, self.hi.x - self.lo.x), side(v, self.hi.y - self.lo.y))
    }

    /// The grid's points inside the box.
    fn grid(&self, n: &Counts, out: &mut Vec<Point2>) {
        let (lo, hi) = (self.lo, self.hi);
        for (ci, row) in n.iter().enumerate() {
            for (cj, &(nu, nv)) in row.iter().enumerate() {
                let (a, b) = (Nodes::node(lo, hi, ci, cj), Nodes::node(lo, hi, ci + 1, cj + 1));
                // each cell owns its low sides; the box's own low sides are boundary
                for i in usize::from(ci == 0)..nu {
                    for j in usize::from(cj == 0)..nv {
                        out.push(Point2::new(
                            a.x + (b.x - a.x) * i as f64 / nu as f64,
                            a.y + (b.y - a.y) * j as f64 / nv as f64,
                        ));
                    }
                }
            }
        }
    }
}

/// Grid steps per cell of `Nodes`, across u and across v.
type Counts = [[(usize, usize); G]; G];

/// A procedural face's bounds on its bending over each cell of the G x G split of its parameter
/// box (see `Bend`), over the cells round each (a point's nearest on the surface lies within a
/// cell of it), and over the box grown by half each way.
struct Cells {
    lo: Point2,
    hi: Point2,
    b: Vec<Bend>,
    round: Vec<Bend>,
    far: (Bend, Point2, Point2),
    /// Bounds on the second derivatives' sizes over the whole box: a chord across d sags at most
    /// Q_m(d) / 8 from the surface, so most chords need nothing finer.
    whole: [f64; 3],
    /// Cells per unit u and v (none along a way the box has no width).
    inv: (f64, f64),
    /// The grid's steps per cell, where they are the bounds' own (`counts`): a chord or triangle
    /// of grid points within one cell and a step each way lies within the chord.
    fit: Option<Counts>,
}

/// Bounds over a cell: on the normal's part of the second derivatives (`k`) and on their sizes
/// (`m`), on the normal's turn per unit u and v, and on the rows of [S_u S_v]'s pseudo-inverse
/// (|S_v| and |S_u| over |S_u x S_v|: a step in the tangent plane of length l moves u and v by at
/// most l times these). Without bounds, all infinite.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bend {
    k: [f64; 3],
    m: [f64; 3],
    turn: (f64, f64),
    inv: (f64, f64),
}

impl Bend {
    const NONE: Bend = Bend {
        k: [f64::INFINITY; 3],
        m: [f64::INFINITY; 3],
        turn: (f64::INFINITY, f64::INFINITY),
        inv: (f64::INFINITY, f64::INFINITY),
    };

    fn of(x: Option<forge_geom::ShapeBounds>) -> Bend {
        x.map_or(Bend::NONE, |x| Bend {
            k: x.k,
            m: x.m,
            turn: x.turn(),
            inv: (x.d[1] / x.area, x.d[0] / x.area),
        })
    }

    fn max(self, o: Bend) -> Bend {
        let mx = |a: [f64; 3], b: [f64; 3]| [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])];
        Bend {
            k: mx(self.k, o.k),
            m: mx(self.m, o.m),
            turn: (self.turn.0.max(o.turn.0), self.turn.1.max(o.turn.1)),
            inv: (self.inv.0.max(o.inv.0), self.inv.1.max(o.inv.1)),
        }
    }
}

impl Cells {
    /// The bounds over the whole box (None without any); the cells' come with `split`.
    fn new(pr: &forge_geom::surface::Proc, lo: Point2, hi: Point2) -> Option<Cells> {
        // a boundary sample past the surface's own parameters by a rounding error is on it still
        let d = pr.domain();
        let (lo, hi) = (
            Point2::new(lo.x.max(d.u.lo).min(hi.x), lo.y.max(d.v.lo).min(hi.y)),
            Point2::new(hi.x.min(d.u.hi).max(lo.x), hi.y.min(d.v.hi).max(lo.y)),
        );
        let whole = over(pr, lo, hi)?;
        let inv = |w: f64| if w > 0.0 { G as f64 / w } else { 0.0 };
        Some(Cells {
            lo,
            hi,
            b: Vec::new(),
            round: Vec::new(),
            far: (Bend::NONE, lo, hi),
            whole: whole.m,
            inv: (inv(hi.x - lo.x), inv(hi.y - lo.y)),
            fit: None,
        })
    }

    /// The bounds over each cell, over the cells round each, and over the grown box.
    fn split(&mut self, pr: &forge_geom::surface::Proc) {
        let (lo, hi) = (self.lo, self.hi);
        let mut b = Vec::with_capacity(G * G);
        for i in 0..G {
            for j in 0..G {
                b.push(Bend::of(over(
                    pr,
                    Nodes::node(lo, hi, i, j),
                    Nodes::node(lo, hi, i + 1, j + 1),
                )));
            }
        }
        self.round = (0..G * G)
            .map(|c| {
                let (i, j) = (c / G, c % G);
                let mut r = b[c];
                for a in i.saturating_sub(1)..(i + 2).min(G) {
                    for d in j.saturating_sub(1)..(j + 2).min(G) {
                        r = r.max(b[a * G + d]);
                    }
                }
                r
            })
            .collect();
        self.b = b;
        let (w, h) = (0.5 * (hi.x - lo.x), 0.5 * (hi.y - lo.y));
        let (flo, fhi) = (Point2::new(lo.x - w, lo.y - h), Point2::new(hi.x + w, hi.y + h));
        self.far = (Bend::of(over(pr, flo, fhi)), flo, fhi);
    }

    /// The cell holding a point.
    fn cell(&self, p: Point2) -> (usize, usize) {
        (
            self.at(p.x, self.lo.x, self.inv.0),
            self.at(p.y, self.lo.y, self.inv.1),
        )
    }

    /// The cell index along one way (truncation is the floor: never below zero).
    fn at(&self, v: f64, lo: f64, inv: f64) -> usize {
        (((v - lo) * inv).max(0.0) as usize).min(G - 1)
    }

    fn size(&self) -> (f64, f64) {
        (
            (self.hi.x - self.lo.x) / G as f64,
            (self.hi.y - self.lo.y) / G as f64,
        )
    }

    /// Whether the mean of points `p` (in parameters, `grid`: all the grid's or inner ones) lies
    /// within the chord of the surface, its corners on it, by the bounds (`gap_by`).
    fn within<const N: usize>(&self, p: [Point2; N], grid: bool, chord: f64) -> bool {
        let (mut lo, mut hi) = (p[0], p[0]);
        for q in &p[1..] {
            (lo, hi) = (
                Point2::new(lo.x.min(q.x), lo.y.min(q.y)),
                Point2::new(hi.x.max(q.x), hi.y.max(q.y)),
            );
        }
        let n = N as f64;
        let x = Point2::new(
            p.iter().map(|q| q.x).sum::<f64>() / n,
            p.iter().map(|q| q.y).sum::<f64>() / n,
        );
        if p.iter()
            .map(|q| quad(self.whole, (q.x - x.x, q.y - x.y)))
            .sum::<f64>()
            / (2.0 * n)
            <= 0.98 * chord
        {
            return true;
        }
        if self.b.is_empty() {
            return false;
        }
        let ((i0, j0), (i1, j1)) = (self.cell(lo), self.cell(hi));
        if i0 == i1 && j0 == j1 {
            if grid && let Some(n) = &self.fit {
                let (cw, ch) = self.size();
                let (nu, nv) = n[i0][j0];
                if hi.x - lo.x <= cw / nu as f64 * (1.0 + 1e-9)
                    && hi.y - lo.y <= ch / nv as f64 * (1.0 + 1e-9)
                {
                    return true;
                }
            }
            let c = i0 * G + j0;
            return self.gap_by(&self.b[c], &self.round[c], p) <= 0.98 * chord;
        }
        let (mut b, mut r) = (self.b[i0 * G + j0], self.round[i0 * G + j0]);
        for i in i0..=i1 {
            for j in j0..=j1 {
                (b, r) = (b.max(self.b[i * G + j]), r.max(self.round[i * G + j]));
            }
        }
        self.gap_by(&b, &r, p) <= 0.98 * chord
    }

    /// The most the mean x of points `p` lies off the surface, its corners on it, by bounds `b`
    /// holding over the points' box and `r` over the cells round it.
    ///
    /// By Taylor with the integral remainder about x, the interpolation less S(x) is the mean of
    /// the remainders R_i = int (1 - t) S''(x + t h_i)(h_i, h_i), h_i = p_i - x. Along the normal
    /// n at x each is at most half of Q_k(h_i) + turn Q_m(h_i), the normal turning by at most
    /// `turn` between x and the segments' points; the rest, at most the mean of Q_m(h_i) / 2,
    /// lies in the tangent plane (`shift`).
    fn gap_by<const N: usize>(&self, b: &Bend, r: &Bend, p: [Point2; N]) -> f64 {
        let n = N as f64;
        let x = Point2::new(
            p.iter().map(|q| q.x).sum::<f64>() / n,
            p.iter().map(|q| q.y).sum::<f64>() / n,
        );
        let (mut hu, mut hv) = (0.0f64, 0.0f64);
        for pi in &p {
            (hu, hv) = (hu.max((pi.x - x.x).abs()), hv.max((pi.y - x.y).abs()));
        }
        let turn = (b.turn.0 * hu + b.turn.1 * hv).min(2.0);
        let (mut along, mut e) = (0.0, 0.0);
        for pi in &p {
            let h = (pi.x - x.x, pi.y - x.y);
            let qm = quad(b.m, h);
            (along, e) = (along + quad(b.k, h) + turn * qm, e + qm);
        }
        self.shift(b, r, Some(x), along / (2.0 * n), e / (2.0 * n))
    }

    /// The most any chord or triangle no wider than `du` by `dv` and within bounds `b` lies off
    /// the surface, wherever it is in the box. A chord's offsets are half of it; a triangle's
    /// from its middle sum at most 2/3 of the box's squares (three points in a width w spread at
    /// most 2 w^2 / 3 about their mean) and its products, and reach at most 2/3 of it.
    fn gap_box(&self, b: &Bend, r: &Bend, du: f64, dv: f64) -> f64 {
        let (k, m) = (quad(b.k, (du, dv)), quad(b.m, (du, dv)));
        let turn = b.turn.0 * du + b.turn.1 * dv;
        let chord = (k + (0.5 * turn).min(2.0) * m) / 8.0;
        let triangle = (k + (2.0 / 3.0 * turn).min(2.0) * m) / 9.0;
        self.shift(b, r, None, chord.max(triangle), m / 8.0)
    }

    /// The most a point lies off the surface that lies `along` off it at x along the normal n
    /// there and `e` off it within the tangent plane, by bounds `b` at x and `r` over the cells
    /// round it: the surface's point at x + t, J t that tangent part, its u and v at most e
    /// times the rows of J's pseudo-inverse, is off S(x) + J t by at most S''(t, t) / 2. The step
    /// stays within the cells round x, or else within the grown box and its bounds; with x
    /// unknown (None), within half the box and by both.
    fn shift(&self, b: &Bend, r: &Bend, x: Option<Point2>, along: f64, e: f64) -> f64 {
        let (du, dv) = (e * b.inv.0, e * b.inv.1);
        let (cw, ch) = self.size();
        let inside = |x: Point2, lo: Point2, hi: Point2| {
            x.x - du >= lo.x && x.x + du <= hi.x && x.y - dv >= lo.y && x.y + dv <= hi.y
        };
        let m = match x {
            None if du <= 0.5 * (self.hi.x - self.lo.x) && dv <= 0.5 * (self.hi.y - self.lo.y) => {
                r.max(self.far.0).m
            }
            Some(x) if du <= cw && dv <= ch && inside(x, self.lo, self.hi) => r.m,
            Some(x) if inside(x, self.far.1, self.far.2) => self.far.0.m,
            _ => return f64::INFINITY,
        };
        along + 0.5 * quad(m, (du, dv))
    }

    /// Grid steps per cell from its bounds, so that any chord or triangle in the cell no wider
    /// than a step each way lies within the chord by `gap_box`, with room to spare: from one step
    /// each way, each round adds steps the way that brings the gap down further.
    fn counts(&self, q: &Quality) -> Option<Counts> {
        let (cw, ch) = self.size();
        let mut n = [[(1, 1); G]; G];
        let mut last: Option<(Bend, (usize, usize))> = None;
        for (i, row) in n.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                let b = &self.b[i * G + j];
                // cells alike (as along a screw) step alike
                if let Some((x, s)) = last
                    && x == *b
                {
                    *cell = s;
                    continue;
                }
                // no fewer steps than the normal bending alone needs
                let least =
                    |w: f64, k: f64| (w * (k / (7.2 * q.chord)).sqrt()).ceil().clamp(1.0, 1e6) as usize;
                let (mut a, mut c) = (least(cw, b.k[0]), least(ch, b.k[2]));
                let r = &self.round[i * G + j];
                let at = |a: usize, c: usize| self.gap_box(b, r, cw / a as f64, ch / c as f64);
                let mut now = at(a, c);
                while now > 0.9 * q.chord {
                    if a * c > 1 << 20 {
                        return None;
                    }
                    let (x, y) = (at(a + 1 + a / 8, c), at(a, c + 1 + c / 8));
                    if x <= y {
                        (a, now) = (a + 1 + a / 8, x);
                    } else {
                        (c, now) = (c + 1 + c / 8, y);
                    }
                }
                *cell = (a, c);
                last = Some((*b, (a, c)));
            }
        }
        Some(n)
    }
}

/// The surface's bounds over the parameter box from `a` to `c`.
fn over(pr: &forge_geom::surface::Proc, a: Point2, c: Point2) -> Option<forge_geom::ShapeBounds> {
    pr.shape_bounds(forge_math::Box2::new(
        forge_math::Interval::new(a.x, c.x),
        forge_math::Interval::new(a.y, c.y),
    ))
}

/// b_uu h_u^2 + 2 b_uv |h_u h_v| + b_vv h_v^2.
fn quad(b: [f64; 3], h: (f64, f64)) -> f64 {
    b[0] * h.0 * h.0 + 2.0 * b[1] * (h.0 * h.1).abs() + b[2] * h.1 * h.1
}

/// Mesh vertices joined along edges: `nbr[start[v]..start[v + 1]]` are v's neighbours, and
/// `edges[on[v]..on[v + 1]]` the edges v lies on.
struct Segments {
    start: Vec<u32>,
    nbr: Vec<u32>,
    on: Vec<u32>,
    edges: Vec<u32>,
}

/// A watertight mesh of a body: each edge is cut once and both faces share
/// those cut points, so neighbouring faces meet exactly.
pub fn tessellate(body: &Body, q: &Quality) -> Result<TriMesh, TessError> {
    if body.dead > 0 {
        return tessellate(&body.compact(), q);
    }
    // a face whose boundary chords cross (two of its edges running close, as a curve all but
    // touching a line, or nearly tangent where they meet) gets its edges cut finer, so their
    // chords follow them closer, until they no longer do
    let mut finer = vec![1usize; body.edges.len()];
    let mut rounds = 0;
    loop {
        match tessellate_with(body, q, &finer) {
            Err(TessError::Face {
                face,
                error: CdtError::ConstraintsIntersect { .. },
            }) if rounds < 6 => {
                for &l in &body.face(face).loops {
                    for &c in &body.lp(l).coedges {
                        finer[body.coedge(c).edge.index()] *= 2;
                    }
                }
                rounds += 1;
            }
            r => return r,
        }
    }
}

fn tessellate_with(body: &Body, q: &Quality, finer: &[usize]) -> Result<TriMesh, TessError> {
    let mut mesh = TriMesh::default();
    let vertex_index: Vec<u32> = body.vertices.iter().map(|v| mesh.add_vertex(v.point)).collect();

    // per edge: its parameter samples and their mesh indices (ends are the body's vertices)
    let mut edge_samples: Vec<Vec<(f64, u32)>> = Vec::with_capacity(body.edges.len());
    let (mut trig, mut pts) = (Vec::new(), Vec::new());
    for e in body.edge_ids() {
        let edge = body.edge(e);
        let fine = edge
            .coedges
            .iter()
            .any(|&c| doubly_curved(&body.face(body.coedge_face(c)).surface));
        let n = edge_segments(&edge.curve, edge.range, q, fine) * finer[e.index()];
        let mut s = Vec::with_capacity(n + 1);
        s.push((edge.range.lo, vertex_index[edge.start.index()]));
        edge.curve.sample_interior(edge.range, n, &mut trig, &mut pts);
        for (i, &p) in pts.iter().enumerate() {
            let t = edge.range.lo + edge.range.width() * ((i + 1) as f64 / n as f64);
            s.push((t, mesh.add_vertex(p)));
        }
        s.push((edge.range.hi, vertex_index[edge.end.index()]));
        edge_samples.push(s);
    }

    // every mesh segment an edge's own samples make, as each sample's neighbours along its edges
    // (start offsets, then neighbours): a face's inner chord must never repeat one (four
    // triangles on one segment), as when a coarse chord across a cylinder lands exactly on an
    // edge between two other faces
    let nv = mesh.positions.len();
    let mut start = vec![0u32; nv + 1];
    for s in &edge_samples {
        for w in s.windows(2) {
            start[w[0].1 as usize + 1] += 1;
            start[w[1].1 as usize + 1] += 1;
        }
    }
    for i in 0..nv {
        start[i + 1] += start[i];
    }
    let mut fill = start.clone();
    let mut nbr = vec![0u32; start[nv] as usize];
    for s in &edge_samples {
        for w in s.windows(2) {
            let (a, b) = (w[0].1 as usize, w[1].1 as usize);
            nbr[fill[a] as usize] = b as u32;
            fill[a] += 1;
            nbr[fill[b] as usize] = a as u32;
            fill[b] += 1;
        }
    }
    let mut on = vec![0u32; nv + 1];
    for s in &edge_samples {
        for &(_, v) in s {
            on[v as usize + 1] += 1;
        }
    }
    for i in 0..nv {
        on[i + 1] += on[i];
    }
    let mut fill = on.clone();
    let mut edges = vec![0u32; on[nv] as usize];
    for (e, s) in edge_samples.iter().enumerate() {
        for &(_, v) in s {
            edges[fill[v as usize] as usize] = e as u32;
            fill[v as usize] += 1;
        }
    }
    let segments = Segments {
        start,
        nbr,
        on,
        edges,
    };
    for f in body.face_ids() {
        // a cone is unrolled only to reach its apex cleanly; unrolled, two coarse chords of
        // curves meeting at a sharp corner can cross (in its parameters both run straight)
        match tessellate_face(body, f, &edge_samples, &segments, q, true, &mut mesh) {
            Err(CdtError::ConstraintsIntersect { .. })
                if matches!(body.face(f).surface, Surface::Cone(_)) =>
            {
                tessellate_face(body, f, &edge_samples, &segments, q, false, &mut mesh)
            }
            r => r,
        }
        .map_err(|error| TessError::Face { face: f, error })?;
    }
    Ok(mesh)
}

fn tessellate_face(
    body: &Body,
    f: FaceId,
    edge_samples: &[Vec<(f64, u32)>],
    segments: &Segments,
    q: &Quality,
    unroll: bool,
    mesh: &mut TriMesh,
) -> Result<(), CdtError> {
    let face = body.face(f);
    let surface = &face.surface;
    let (su, sv) = uv_scale(surface);

    let mut uv: Vec<Point2> = Vec::new();
    let mut global: Vec<u32> = Vec::new();
    let mut rings: Vec<Vec<Point2>> = Vec::new();
    let mut ring_globals: Vec<Vec<u32>> = Vec::new();
    for &l in &face.loops {
        let mut samples = Vec::new();
        let mut ids = Vec::new();
        for &c in &body.lp(l).coedges {
            let co = body.coedge(c);
            let s = &edge_samples[co.edge.index()];
            // the edge's samples are already mesh vertices: their points, not evaluated again
            let mut take = |&(_, gi): &(f64, u32)| {
                samples.push(mesh.positions[gi as usize]);
                ids.push(gi);
            };
            if co.forward {
                s[..s.len() - 1].iter().for_each(&mut take);
            } else {
                s[1..].iter().rev().for_each(&mut take);
            }
        }
        let (pts, sample_of) = draw_loop_near(body, f, &samples);
        ring_globals.push(sample_of.iter().map(|&k| ids[k]).collect());
        rings.push(pts);
    }
    align_loops(body, f, rings.iter_mut());
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in rings.iter().flatten() {
        lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    // a procedural surface's bending bounded by its construction, where it can
    let mut cells = match surface {
        Surface::Procedural(pr) => Cells::new(pr, lo, hi),
        _ => None,
    };
    let mut nodes = None;
    let layout = match surface {
        // unrolled where the face reaches the apex: there its rulings meet, and a triangle with a
        // corner at the apex must not lie along one
        Surface::Cone(c) if unroll && c.radius_at(lo.y) <= forge_math::tol::LINEAR => Layout::Unrolled {
            u_mid: 0.5 * (lo.x + hi.x),
            apex_v: c.apex_v(),
            sin_a: c.sin_a(),
            cos_a: c.cos_a(),
        },
        Surface::Procedural(pr) => {
            // the grid from curvatures sampled at the nodes, or where the bounds' grids (whose
            // chords and triangles need no look) have no more points, theirs: from the whole box's
            // bounds, or on a face of many points the cells'
            let n = Nodes::new(pr, lo, hi);
            let mut c = n.counts(surface, q);
            let size = |c: &Counts| c.iter().flatten().map(|&(a, b)| a * b).sum::<usize>();
            if let (Some(b), Surface::Procedural(pr)) = (cells.as_mut(), surface) {
                let w = n.whole_counts(b.whole, q);
                if size(&w) <= size(&c) {
                    c = w;
                }
                if size(&c) > 2 * G * G {
                    b.split(pr);
                    if let Some(t) = b.counts(q)
                        && size(&t) <= size(&c)
                    {
                        (c, b.fit) = (t, Some(t));
                    }
                }
            }
            let (a, b) = n.scale(&c);
            nodes = Some((n, c));
            Layout::Scaled(a, b)
        }
        _ => Layout::Scaled(su, sv),
    };
    let mut loops: Vec<Vec<u32>> = Vec::new();
    let mut seen: Vec<u32> = ring_globals.iter().flatten().copied().collect();
    seen.sort_unstable();
    let mut twice: Vec<u32> = seen.windows(2).filter(|w| w[0] == w[1]).map(|w| w[0]).collect();
    twice.dedup();
    for (pts, ids) in rings.into_iter().zip(ring_globals) {
        // a loop through one vertex twice (a face pinched where a curve touches its boundary):
        // one point, met twice
        let mut lp = Vec::with_capacity(pts.len());
        for (p, id) in pts.into_iter().zip(ids) {
            match (twice.binary_search(&id).is_ok())
                .then(|| (0..uv.len()).find(|&k| global[k] == id && uv[k] == p))
                .flatten()
            {
                Some(k) => lp.push(k as u32),
                None => {
                    lp.push(uv.len() as u32);
                    uv.push(p);
                    global.push(id);
                }
            }
        }
        loops.push(lp);
    }

    if loops.len() == 1
        && !matches!(surface, Surface::Procedural(_))
        && let Some(tris) = strips(surface, &uv, &global, q, mesh)
    {
        for [a, b, c] in tris {
            if a == b || b == c || c == a {
                continue;
            }
            mesh.triangles
                .push(if face.same_sense { [a, b, c] } else { [a, c, b] });
            mesh.face_of.push(f.0);
        }
        return Ok(());
    }

    // a swept face square in its parameters (a blend cut square at its ends): a grid of columns
    // joined to its boundary, where every chord and triangle holds
    if let (Some((n, _)), Surface::Procedural(pr), [_]) = (&nodes, surface, &loops[..])
        && pr.swept()
    {
        let off = |x, y, p| strays_from(surface, x, y, p, q.chord);
        if let Some(tris) = proc_strips(pr, &uv, &global, &n.want(surface, q, true), q.chord, &off, mesh) {
            for [a, b, c] in tris {
                mesh.triangles
                    .push(if face.same_sense { [a, b, c] } else { [a, c, b] });
                mesh.face_of.push(f.0);
            }
            return Ok(());
        }
    }

    if let Layout::Unrolled { .. } = layout {
        // an apex drawn as a run along its parameter line is one point unrolled
        let (old_uv, old_global) = (std::mem::take(&mut uv), std::mem::take(&mut global));
        for l in &mut loops {
            let start = uv.len() as u32;
            for (k, &i) in l.iter().enumerate() {
                let j = l[(k + l.len() - 1) % l.len()];
                let (i, j) = (i as usize, j as usize);
                if i != j && old_global[i] == old_global[j] && layout.put(old_uv[i]) == layout.put(old_uv[j])
                {
                    continue;
                }
                uv.push(old_uv[i]);
                global.push(old_global[i]);
            }
            *l = (start..uv.len() as u32).collect();
        }
    }

    let mut scaled: Vec<Point2> = uv.iter().map(|&p| layout.put(p)).collect();
    let mut interior = Vec::new();
    // an analytic face's grid: cells clear of the boundary are triangulated straight off the
    // grid, the band between them and the boundary by the constrained Delaunay
    let mut grid: Option<Grid> = None;
    if let Some((n, c)) = &nodes {
        n.grid(c, &mut interior);
        interior.iter_mut().for_each(|p| *p = layout.put(*p));
    } else if let Some((du, dv)) = interior_steps(surface, q) {
        match surface {
            Surface::Cone(c) => {
                // rows across the rulings, each with points as close as its own radius needs but
                // no closer than half the rows' spacing (rows run together towards the apex)
                let r = c.radius_at(0.5 * (lo.y + hi.y)).max(forge_math::tol::LINEAR);
                let nv = ((hi.y - lo.y) / (2.0 * r * arc_step(r, q))).ceil().max(1.0) as usize;
                let gap = (hi.y - lo.y) / nv as f64 / c.cos_a();
                for j in 1..nv {
                    let v = lo.y + (hi.y - lo.y) * j as f64 / nv as f64;
                    let r = c.radius_at(v);
                    let span = hi.x - lo.x;
                    let nu = (span / arc_step(r, q))
                        .min(span * r / (0.5 * gap))
                        .ceil()
                        .max(1.0) as usize;
                    for i in 1..nu {
                        interior.push(layout.put(Point2::new(lo.x + span * i as f64 / nu as f64, v)));
                    }
                }
            }
            _ => {
                let nu = ((hi.x - lo.x) / du).ceil().max(1.0) as usize;
                let nv = ((hi.y - lo.y) / dv).ceil().max(1.0) as usize;
                let mut g = Grid::new(lo, hi, nu, nv, &uv, &loops);
                // a cell's chords sag within the tolerance only where its steps come from the
                // surface's exact radii (not a cone's, whose radius grows along it)
                let exact = matches!(
                    surface,
                    Surface::Sphere(_) | Surface::Torus(_) | Surface::Cylinder(_)
                );
                let stairs = if g.any && exact { g.staircase() } else { None };
                g.settle(stairs.is_some());
                for i in 1..nu {
                    for j in 1..nv {
                        if !g.covered(i, j) {
                            let p = g.node(i, j);
                            interior.push(layout.put(p));
                        }
                    }
                }
                if let Some(stairs) = stairs {
                    for cycle in stairs {
                        loops.push((scaled.len() as u32..(scaled.len() + cycle.len()) as u32).collect());
                        for &(i, j) in &cycle {
                            let p = g.node(i, j);
                            scaled.push(layout.put(p));
                        }
                        g.cycles.borrow_mut().push(cycle);
                    }
                    grid = Some(g);
                }
            }
        }
    }

    // chords that stray from the surface by more than the tolerance get a surface point at
    // their middle, inserted into the same triangulation round after round (a thin band between
    // two boundary curves has no grid points at all; a chord across a radian of a cylinder needs
    // several halvings); on a surface curved both ways a triangle's middle sags further than its
    // sides' (by up to a third), so triangles there are looked at too
    let at = |p: Point2| surface.eval(layout.uv(p));
    // each point's position is worked out once, however many edges and rounds test it (the
    // boundary's and the grid's are known already)
    let mut node_index: Vec<u32> = Vec::new();
    let mut known: Vec<Option<forge_math::Point3>> =
        global.iter().map(|&g| Some(mesh.positions[g as usize])).collect();
    if let Some(g) = &grid {
        node_index = g.place(surface, mesh);
        for cycle in g.cycles.borrow().iter() {
            known.extend(
                cycle
                    .iter()
                    .map(|&(i, j)| Some(mesh.positions[node_index[g.id(i, j)] as usize])),
            );
        }
    }
    let cache = std::cell::RefCell::new(known);
    let pos = |i: u32, p: Point2| {
        let mut c = cache.borrow_mut();
        if c.len() <= i as usize {
            c.resize(i as usize + 1, None);
        }
        *c[i as usize].get_or_insert_with(|| at(p))
    };
    let nb = global.len() as u32;
    let edges_at = |i: u32| match global[i as usize] as usize {
        x if x + 1 < segments.on.len() => {
            &segments.edges[segments.on[x] as usize..segments.on[x + 1] as usize]
        }
        _ => &[],
    };
    let on_edge = |ia: u32, ib: u32| {
        ia < nb && ib < nb && {
            let (x, y) = (global[ia as usize] as usize, global[ib as usize] as usize);
            x < segments.start.len() - 1
                && segments.nbr[segments.start[x] as usize..segments.start[x + 1] as usize]
                    .contains(&(y as u32))
        }
    };
    // a chord between two points on the boundary of one other face (of one edge, the face across
    // it, as nearly tangent as a run-out meeting a wall) can be that face's chord too: the two
    // faces would share its two triangles
    let shared = |ia: u32, ib: u32| {
        ia < nb && ib < nb && {
            let others = |e: u32| {
                body.edge(forge_topo::EdgeId(e))
                    .coedges
                    .iter()
                    .map(|&c| body.coedge_face(c))
                    .filter(|&g| g != f)
            };
            let eb = edges_at(ib);
            edges_at(ia)
                .iter()
                .any(|&x| eb.contains(&x) || eb.iter().any(|&y| others(x).any(|g| others(y).any(|h| h == g))))
        }
    };
    // a chord of a plane never strays from it
    let flat = matches!(surface, Surface::Plane(_));
    // on a sphere, torus or cylinder the grid's steps come from its exact radii (on a sphere or
    // torus each way a little under half the tolerance): a chord between two inner points within
    // one grid step each way sags less than the tolerance, and so does a triangle of three (its
    // middle sags at most as far as the circle round it, no larger than the step's), and neither
    // needs a look (a procedural face's curvatures at its nodes are only samples: everything there
    // is looked at)
    let cell = match (surface, interior_steps(surface, q)) {
        (Surface::Sphere(_) | Surface::Torus(_) | Surface::Cylinder(_), Some((du, dv))) => (du * su, dv * sv),
        _ => (0.0, 0.0),
    };
    let within = |a: Point2, b: Point2| {
        (a.x - b.x).abs() <= cell.0 * (1.0 + 1e-9) && (a.y - b.y).abs() <= cell.1 * (1.0 + 1e-9)
    };
    // whether a point off the face (laid out at `p`) lies further than the tolerance from its
    // surface (exactly, on an analytic one)
    let off = |x: forge_math::Point3, p: Point2| match surface {
        Surface::Procedural(_) => {
            let uv = layout.uv(p);
            strays_from(surface, x, surface.eval(uv), uv, q.chord)
        }
        _ => surface.implicit(x).abs() > q.chord,
    };
    // within the chord by the bounds on a procedural surface's bending (`grid`: all inner points)
    let bounded2 = |a: Point2, b: Point2, grid: bool| {
        cells
            .as_ref()
            .is_some_and(|c| c.within([layout.uv(a), layout.uv(b)], grid, q.chord))
    };
    let bounded3 = |p: [Point2; 3], grid: bool| {
        cells
            .as_ref()
            .is_some_and(|c| c.within(p.map(|x| layout.uv(x)), grid, q.chord))
    };
    let edge = |ia: u32, ib: u32, a: Point2, b: Point2| {
        let grid = ia >= nb && ib >= nb;
        if grid && within(a, b) {
            return false;
        }
        on_edge(ia, ib)
            || (!flat
                && (shared(ia, ib)
                    || (!bounded2(a, b, grid) && off(pos(ia, a).lerp(pos(ib, b), 0.5), a.lerp(b, 0.5)))))
    };
    // (a long triangle on a cone, between rulings at its two ends, can sag in its middle past
    // its sides too)
    let both_ways = doubly_curved(surface) || matches!(surface, Surface::Procedural(_) | Surface::Cone(_));
    let triangle = |v: [u32; 3], p: [Point2; 3]| {
        let m = Point2::new((p[0].x + p[1].x + p[2].x) / 3.0, (p[0].y + p[1].y + p[2].y) / 3.0);
        let grid = v.iter().all(|&k| k >= nb);
        !(grid && (0..3).all(|k| within(p[k], p[(k + 1) % 3]))) && !bounded3(p, grid) && {
            let x = [0, 1, 2].map(|k| pos(v[k], p[k]));
            off(x[0].lerp(x[1], 0.5).lerp(x[2], 1.0 / 3.0), m)
        }
    };
    let strays = Tests(edge, triangle, both_ways);
    let tri = match triangulate_refined(&scaled, &loops, &interior, Some(&strays)) {
        Ok(t) => t,
        Err(e) => {
            if let Some(path) = std::env::var_os("FORGE_CDT_DUMP") {
                let pts: Vec<String> = scaled.iter().map(|p| format!("{:e},{:e}", p.x, p.y)).collect();
                let ls: Vec<String> = loops.iter().map(|l| format!("{}+{}", l[0], l.len())).collect();
                let ins: Vec<String> = interior.iter().map(|p| format!("{:e},{:e}", p.x, p.y)).collect();
                let text = format!("{}\n{}\n{}\n", pts.join(" "), ls.join(" "), ins.join(" "));
                let _ = std::fs::write(path, text);
            }
            return Err(e);
        }
    };
    let mut index = global;
    let known = cache.into_inner();
    if let Some(g) = &grid {
        for cycle in g.cycles.borrow().iter() {
            index.extend(cycle.iter().map(|&(i, j)| node_index[g.id(i, j)]));
        }
    }
    for (i, p) in tri.points.iter().enumerate().skip(index.len()) {
        let pos = known.get(i).copied().flatten().unwrap_or_else(|| at(*p));
        index.push(mesh.add_vertex(pos));
    }
    if let Some(g) = &grid {
        for (i, j) in g.inner_cells() {
            let [a, b, c, d] =
                [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)].map(|(x, y)| node_index[g.id(x, y)]);
            for t in [[a, b, c], [a, c, d]] {
                let t = if face.same_sense { t } else { [t[0], t[2], t[1]] };
                mesh.triangles.push(t);
                mesh.face_of.push(f.0);
            }
        }
    }

    for t in &tri.triangles {
        let [a, b, c] = t.map(|k| index[k as usize]);
        // two corners on one pole: the triangle collapses to the pole and is dropped
        if a == b || b == c || c == a {
            continue;
        }
        // counter-clockwise in (u, v) faces along du x dv, the surface's natural normal
        let t = if face.same_sense { [a, b, c] } else { [a, c, b] };
        mesh.triangles.push(t);
        mesh.face_of.push(f.0);
    }
    Ok(())
}

/// Where a face is laid out flat for the triangulation.
#[derive(Clone, Copy)]
enum Layout {
    /// Its parameters, scaled so lengths roughly match the surface's.
    Scaled(f64, f64),
    /// A cone unrolled (exactly: it is developable), its rulings straight lines through the apex,
    /// so no triangle with a corner there runs along one.
    Unrolled {
        u_mid: f64,
        apex_v: f64,
        sin_a: f64,
        cos_a: f64,
    },
}

impl Layout {
    fn put(self, uv: Point2) -> Point2 {
        match self {
            Layout::Scaled(su, sv) => Point2::new(uv.x * su, uv.y * sv),
            Layout::Unrolled {
                u_mid,
                apex_v,
                sin_a,
                cos_a,
            } => {
                let rho = (uv.y - apex_v) / cos_a;
                let (s, c) = dmath::sin_cos((uv.x - u_mid) * sin_a);
                // mirrored, so counter-clockwise stays counter-clockwise in (u, v)
                Point2::new(rho * c, -rho * s)
            }
        }
    }

    fn uv(self, p: Point2) -> Point2 {
        match self {
            Layout::Scaled(su, sv) => Point2::new(p.x / su, p.y / sv),
            Layout::Unrolled {
                u_mid,
                apex_v,
                sin_a,
                cos_a,
            } => Point2::new(
                u_mid + dmath::atan2(-p.y, p.x) / sin_a,
                apex_v + (p.x * p.x + p.y * p.y).sqrt() * cos_a,
            ),
        }
    }
}

/// Whether `x` lies further than `c` from the surface about `uv`, where it is expected near.
/// The surface's point at `uv` (`y`) bounds the distance from above. Where that is too far but the gap
/// along the normal there is not (the point lies along from the nearest, as by a pole or a seam),
/// Gauss-Newton looks for a nearer one, each step kept only if it comes closer.
fn strays_from(s: &Surface, x: forge_math::Point3, y: forge_math::Point3, uv: Point2, c: f64) -> bool {
    if y.distance(x) <= c {
        return false;
    }
    let (mut uv, mut d) = (uv, s.deriv(uv));
    if let Some(n) = d.du.cross(d.dv).normalize()
        && (x - d.p).dot(n.get()).abs() > c
    {
        return true;
    }
    let mut best = d.p.distance(x);
    for _ in 0..6 {
        let r = x - d.p;
        let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
        let det = e * g - f * f;
        if det <= 0.0 {
            break;
        }
        let (a, b) = (d.du.dot(r), d.dv.dot(r));
        let next = Point2::new(uv.x + (g * a - f * b) / det, uv.y + (e * b - f * a) / det);
        let dn = s.deriv(next);
        let dist = dn.p.distance(x);
        if dist.is_nan() || dist >= best {
            break;
        }
        (uv, d, best) = (next, dn, dist);
        if best <= c {
            return false;
        }
    }
    true
}

/// A refinement's tests: edges, triangles, and whether triangles are looked at.
struct Tests<E, T>(E, T, bool);

impl<E: Fn(u32, u32, Point2, Point2) -> bool, T: Fn([u32; 3], [Point2; 3]) -> bool> Refine for Tests<E, T> {
    fn edge(&self, a: u32, b: u32, pa: Point2, pb: Point2) -> bool {
        (self.0)(a, b, pa, pb)
    }
    fn triangle(&self, v: [u32; 3], p: [Point2; 3]) -> bool {
        (self.1)(v, p)
    }
    fn triangles(&self) -> bool {
        self.2
    }
}

/// A face's interior grid in (u, v): `nu` x `nv` cells over the face's parameter box, and the
/// cells clear of its boundary (`inner`): no boundary segment's box meets the cell or any cell
/// round it, and the cell's middle is inside the face.
struct Grid {
    lo: Point2,
    hi: Point2,
    nu: usize,
    nv: usize,
    inner: Vec<bool>,
    any: bool,
    /// Per node: a corner of an inner cell.
    cover: Vec<bool>,
    /// The staircase cycles round the inner cells, once traced.
    cycles: std::cell::RefCell<Vec<Vec<(usize, usize)>>>,
}

impl Grid {
    fn new(lo: Point2, hi: Point2, nu: usize, nv: usize, uv: &[Point2], loops: &[Vec<u32>]) -> Grid {
        let (cw, ch) = ((hi.x - lo.x) / nu as f64, (hi.y - lo.y) / nv as f64);
        let mut touched = vec![false; nu * nv];
        // the cells a segment's box comes within a quarter cell of: no grid node of an untouched
        // cell lies on or right by the boundary
        let cell_of = |x: f64, w: f64, l: f64, n: usize| (((x - l) / w).floor().max(0.0) as usize).min(n - 1);
        let reach = 0.25;
        let segs = || {
            loops.iter().flat_map(|l| {
                (0..l.len()).map(move |k| (uv[l[k] as usize], uv[l[(k + 1) % l.len()] as usize]))
            })
        };
        for (a, b) in segs() {
            let (i0, i1) = (
                cell_of(a.x.min(b.x) - reach * cw, cw, lo.x, nu),
                cell_of(a.x.max(b.x) + reach * cw, cw, lo.x, nu),
            );
            let (j0, j1) = (
                cell_of(a.y.min(b.y) - reach * ch, ch, lo.y, nv),
                cell_of(a.y.max(b.y) + reach * ch, ch, lo.y, nv),
            );
            for i in i0..=i1 {
                for j in j0..=j1 {
                    touched[i * nv + j] = true;
                }
            }
        }
        // inside by the parity of crossings along each row's middle: each segment adds its
        // crossings to the rows it spans
        let mut inside = vec![false; nu * nv];
        let mut rows: Vec<Vec<f64>> = vec![Vec::new(); nv];
        let row_at = |y: f64| (y - lo.y) / ch - 0.5;
        for (a, b) in segs() {
            let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
            let j0 = row_at(y0).floor().max(0.0) as usize;
            let j1 = (row_at(y1).ceil().max(0.0) as usize).min(nv - 1);
            for (j, xs) in rows.iter_mut().enumerate().take(j1 + 1).skip(j0) {
                let v = lo.y + ch * (j as f64 + 0.5);
                if (a.y > v) != (b.y > v) {
                    xs.push(a.x + (v - a.y) / (b.y - a.y) * (b.x - a.x));
                }
            }
        }
        for (j, xs) in rows.iter_mut().enumerate() {
            xs.sort_by(f64::total_cmp);
            let mut k = 0;
            for i in 0..nu {
                let u = lo.x + cw * (i as f64 + 0.5);
                while k < xs.len() && xs[k] < u {
                    k += 1;
                }
                inside[i * nv + j] = k % 2 == 1;
            }
        }
        let clear = |i: usize, j: usize| !touched[i * nv + j];
        let mut inner: Vec<bool> = (0..nu * nv).map(|c| inside[c] && clear(c / nv, c % nv)).collect();
        // no two inner cells meeting at a corner only (their staircases would touch there)
        loop {
            let mut changed = false;
            for i in 0..nu.saturating_sub(1) {
                for j in 0..nv.saturating_sub(1) {
                    let at = |x: usize, y: usize| inner[x * nv + y];
                    let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1));
                    if (a && d && !b && !c) || (b && c && !a && !d) {
                        inner[(i + 1) * nv + j + 1] = false;
                        inner[(i + 1) * nv + j] = false;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let any = inner.contains(&true);
        Grid {
            lo,
            hi,
            nu,
            nv,
            inner,
            any,
            cover: Vec::new(),
            cycles: Default::default(),
        }
    }

    fn node(&self, i: usize, j: usize) -> Point2 {
        Point2::new(
            self.lo.x + (self.hi.x - self.lo.x) * i as f64 / self.nu as f64,
            self.lo.y + (self.hi.y - self.lo.y) * j as f64 / self.nv as f64,
        )
    }

    fn id(&self, i: usize, j: usize) -> usize {
        i * (self.nv + 1) + j
    }

    fn is_inner(&self, i: usize, j: usize) -> bool {
        i < self.nu && j < self.nv && self.inner[i * self.nv + j]
    }

    /// Whether a node is a corner of an inner cell.
    fn covered(&self, i: usize, j: usize) -> bool {
        self.any && self.cover[self.id(i, j)]
    }

    /// Settles which nodes the inner cells use (none unless `any`).
    fn settle(&mut self, any: bool) {
        self.any = any;
        self.cover = vec![false; (self.nu + 1) * (self.nv + 1)];
        for c in 0..self.nu * self.nv {
            if self.inner[c] {
                let (i, j) = (c / self.nv, c % self.nv);
                for (x, y) in [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)] {
                    self.cover[x * (self.nv + 1) + y] = true;
                }
            }
        }
    }

    fn inner_cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        (0..self.nu * self.nv)
            .filter(|&c| self.inner[c])
            .map(|c| (c / self.nv, c % self.nv))
    }

    /// The boundary of the inner cells as cycles of nodes (each node on one at most once); None
    /// if a node would start two sides (cannot happen once corner-only contacts are gone).
    fn staircase(&self) -> Option<Vec<Vec<(usize, usize)>>> {
        // each inner cell's sides against a cell that is not: from node to node, cell on the left
        let mut next = vec![usize::MAX; (self.nu + 1) * (self.nv + 1)];
        let mut twice = false;
        let mut side = |from: usize, to: usize| {
            twice |= next[from] != usize::MAX;
            next[from] = to;
        };
        for (i, j) in self.inner_cells() {
            let (i1, j1) = (i + 1, j + 1);
            if j == 0 || !self.is_inner(i, j - 1) {
                side(self.id(i, j), self.id(i1, j));
            }
            if !self.is_inner(i1, j) {
                side(self.id(i1, j), self.id(i1, j1));
            }
            if !self.is_inner(i, j1) {
                side(self.id(i1, j1), self.id(i, j1));
            }
            if i == 0 || !self.is_inner(i - 1, j) {
                side(self.id(i, j1), self.id(i, j));
            }
        }
        if twice {
            return None;
        }
        let mut out = Vec::new();
        for start in 0..next.len() {
            if next[start] == usize::MAX {
                continue;
            }
            let mut cycle = Vec::new();
            let mut n = start;
            while next[n] != usize::MAX {
                cycle.push((n / (self.nv + 1), n % (self.nv + 1)));
                let m = next[n];
                next[n] = usize::MAX;
                n = m;
            }
            if n != start {
                return None;
            }
            out.push(cycle);
        }
        Some(out)
    }

    /// Mesh vertices for every node an inner cell or the staircase uses, row by row.
    fn place(&self, surface: &Surface, mesh: &mut TriMesh) -> Vec<u32> {
        let mut index = vec![u32::MAX; (self.nu + 1) * (self.nv + 1)];
        let (mut trig, mut pts) = (Vec::new(), Vec::new());
        for j in 1..self.nv {
            if !(1..self.nu).any(|i| self.cover[self.id(i, j)]) {
                continue;
            }
            let v = self.node(0, j).y;
            surface.eval_row(v, self.lo.x, self.hi.x, self.nu, &mut trig, &mut pts);
            for i in 1..self.nu {
                if self.covered(i, j) {
                    index[self.id(i, j)] = mesh.add_vertex(pts[i - 1]);
                }
            }
        }
        index
    }
}

/// A face's boundary as a rectangle in its parameters: its bottom (v0), right, top and left
/// sides, each running from u0 to u1 or from bottom to top and each with both its corners, and
/// u0 and u1. None when it is not one.
fn rectangle(uv: &[Point2]) -> Option<([Vec<usize>; 4], f64, f64)> {
    let n = uv.len();
    let (mut u0, mut u1, mut v0, mut v1) =
        (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for p in uv {
        u0 = u0.min(p.x);
        u1 = u1.max(p.x);
        v0 = v0.min(p.y);
        v1 = v1.max(p.y);
    }
    let eps = 1e-9 * (u1 - u0).max(v1 - v0).max(1.0);
    let near = |a: f64, b: f64| (a - b).abs() <= eps;
    let start = (0..n).find(|&i| near(uv[i].x, u0) && near(uv[i].y, v0))?;
    // walk counter-clockwise from the (u0, v0) corner
    let ccw = forge_topo::uv::signed_area(uv) > 0.0;
    let at = |k: usize| if ccw { (start + k) % n } else { (start + n - k) % n };
    let mut sides: [Vec<usize>; 4] = Default::default();
    let mut side = 0;
    sides[0].push(at(0));
    for k in 1..=n {
        let i = at(k);
        let p = uv[i];
        let on = |s: usize| match s {
            0 => near(p.y, v0),
            1 => near(p.x, u1),
            2 => near(p.y, v1),
            _ => near(p.x, u0),
        };
        if !on(side) {
            return None;
        }
        sides[side].push(i);
        let corner = match side {
            0 => near(p.x, u1),
            1 => near(p.y, v1),
            2 => near(p.x, u0),
            _ => k == n,
        };
        if corner && side < 3 {
            side += 1;
            sides[side].push(i);
        }
    }
    if side != 3 {
        return None;
    }
    let [bottom, right, mut top, mut left] = sides;
    top.reverse();
    left.reverse();
    Some(([bottom, right, top, left], u0, u1))
}

/// Points spread over [lo, hi] as `want` (steps per G-th of it) asks: as many steps as it wants
/// in all, the cumulative want inverted; the inner ones.
fn spread(lo: f64, hi: f64, want: &[f64; G]) -> Vec<f64> {
    let total: f64 = want.iter().sum();
    let m = (total.ceil() as usize).max(1);
    let w = (hi - lo) / G as f64;
    let (mut c, mut before) = (0, 0.0);
    (1..m)
        .map(|i| {
            let t = total * i as f64 / m as f64;
            while c + 1 < G && before + want[c] < t {
                before += want[c];
                c += 1;
            }
            let f = if want[c] > 0.0 {
                ((t - before) / want[c]).clamp(0.0, 1.0)
            } else {
                0.5
            };
            lo + w * (c as f64 + f)
        })
        .collect()
}

/// A swept face that is a rectangle in its parameters (`rectangle`: a blend cut square at its
/// ends): a grid inside spaced as its cells want (`want`), each column worked out in one go,
/// triangulated straight, and joined to the boundary by a band stitched round it. Every inner
/// chord and every triangle is looked at as the refinement would (`off`: whether a point strays
/// from the surface, given the surface's point at parameters by it); where any strays, nothing
/// is added (None). How it was chosen and measured: research/mesh-cost.
fn proc_strips(
    pr: &forge_geom::surface::Proc,
    uv: &[Point2],
    global: &[u32],
    want: &[[(f64, f64); G]; G],
    chord: f64,
    off: &dyn Fn(forge_math::Point3, forge_math::Point3, Point2) -> bool,
    mesh: &mut TriMesh,
) -> Option<Vec<[u32; 3]>> {
    let ([bottom, right, top, left], u0, u1) = rectangle(uv)?;
    let (v0, v1) = (uv[bottom[0]].y, uv[top[0]].y);
    let (w, h) = (u1 - u0, v1 - v0);
    // each column of cells as dense along u as its densest cell, each row along v likewise
    let us = spread(
        u0,
        u1,
        &std::array::from_fn(|i| want[i].iter().fold(0.0f64, |m, c| m.max(c.0))),
    );
    let vs = spread(
        v0,
        v1,
        &std::array::from_fn(|j| want.iter().fold(0.0f64, |m, c| m.max(c[j].1))),
    );
    let (mu, nv) = (us.len(), vs.len());
    // a grid with a ring round it (two columns and two rows at least), and not past all reason
    if mu < 2 || nv < 2 || mu * nv > 1 << 20 {
        return None;
    }
    // per point: parameters, position, the boundary's index (u32::MAX inside); the boundary's
    // first, each with its place round the rectangle (one per side, counter-clockwise from (u0, v0))
    let mut pts: Vec<(Point2, forge_math::Point3, u32)> = Vec::new();
    let mut outer: Vec<(f64, usize)> = Vec::new();
    let place = |p: Point2, side: usize| match side {
        0 => (p.x - u0) / w,
        1 => 1.0 + (p.y - v0) / h,
        2 => 2.0 + (u1 - p.x) / w,
        _ => 3.0 + (v1 - p.y) / h,
    };
    let round = (bottom[..bottom.len() - 1].iter().map(|&i| (i, 0)))
        .chain(right[..right.len() - 1].iter().map(|&i| (i, 1)))
        .chain(top[1..].iter().rev().map(|&i| (i, 2)))
        .chain(left[1..].iter().rev().map(|&i| (i, 3)));
    for (i, side) in round {
        outer.push((place(uv[i], side), pts.len()));
        pts.push((uv[i], mesh.positions[global[i] as usize], i as u32));
    }
    let inner = pts.len();
    // each column of the grid with its chords' middles across, then between each two columns the
    // middles of the chords along and corner to corner, and the two triangles' middles: all of
    // one u at a time; x strays from the surface not where within the chord of its point y at p,
    // else as the refinement looks
    let off = |x: forge_math::Point3, y: forge_math::Point3, p: Point2| y.distance(x) > chord && off(x, y, p);
    let lerp = |a: f64, b: f64| a + (b - a) * 0.5;
    let third = |a: f64, b: f64, c: f64| (a + b + c) / 3.0;
    let mids: Vec<f64> = vs.windows(2).map(|w| lerp(w[0], w[1])).collect();
    let both: Vec<f64> = vs.iter().chain(&mids).copied().collect();
    // the triangles' middles' v, low then high in each cell
    let thirds: Vec<f64> = (vs.windows(2).map(|w| third(w[0], w[0], w[1])))
        .chain(vs.windows(2).map(|w| third(w[0], w[1], w[1])))
        .collect();
    let at = |u: f64, v: &[f64], out: &mut Vec<forge_math::Point3>| {
        out.clear();
        pr.column(u, v, out);
    };
    let (mut col, mut next, mut half, mut lo, mut hi) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut tris = Vec::with_capacity(2 * mu * nv + outer.len() + 2 * (mu + nv));
    let id = |i: usize, j: usize| inner + i * nv + j;
    let mid3 = |a: forge_math::Point3, b: forge_math::Point3, c: forge_math::Point3| {
        a.lerp(b, 0.5).lerp(c, 1.0 / 3.0)
    };
    at(us[0], &both, &mut next);
    for i in 0..mu {
        std::mem::swap(&mut col, &mut next);
        let u = us[i];
        pts.extend(
            vs.iter()
                .zip(&col)
                .map(|(&v, &x)| (Point2::new(u, v), x, u32::MAX)),
        );
        for j in 0..nv - 1 {
            if off(col[j].lerp(col[j + 1], 0.5), col[nv + j], Point2::new(u, mids[j])) {
                return None;
            }
        }
        if i + 1 == mu {
            break;
        }
        let w = us[i + 1];
        at(w, &both, &mut next);
        at(lerp(u, w), &both, &mut half);
        // the triangles' middles' u: towards w, towards u
        let (tw, tu) = (third(u, w, w), third(u, w, u));
        at(tw, &thirds, &mut lo);
        at(tu, &thirds, &mut hi);
        for j in 0..nv {
            let (a, b) = (col[j], next[j]);
            if off(a.lerp(b, 0.5), half[j], Point2::new(lerp(u, w), vs[j])) {
                return None;
            }
            if j + 1 == nv {
                continue;
            }
            let (c, d) = (next[j + 1], col[j + 1]);
            // corner to corner, the diagonal nearer the surface at the cell's middle (a twisted
            // cell sags far more along one than the other), and the triangles either side of it
            let (m, y) = (Point2::new(lerp(u, w), mids[j]), half[nv + j]);
            let (k, l) = (j, nv - 1 + j);
            let ids = [id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1)];
            if a.lerp(c, 0.5).distance(y) <= b.lerp(d, 0.5).distance(y) {
                if off(a.lerp(c, 0.5), y, m)
                    || off(mid3(a, b, c), lo[k], Point2::new(tw, thirds[k]))
                    || off(mid3(a, c, d), hi[l], Point2::new(tu, thirds[l]))
                {
                    return None;
                }
                tris.extend([[ids[0], ids[1], ids[2]], [ids[0], ids[2], ids[3]]]);
            } else {
                if off(b.lerp(d, 0.5), y, m)
                    || off(mid3(a, b, d), hi[k], Point2::new(tu, thirds[k]))
                    || off(mid3(b, c, d), lo[l], Point2::new(tw, thirds[l]))
                {
                    return None;
                }
                tris.extend([[ids[0], ids[1], ids[3]], [ids[1], ids[2], ids[3]]]);
            }
        }
    }
    // the grid's outermost ring, counter-clockwise from its (u0, v0) corner, its corners placed
    // at the boundary's
    let mut ring: Vec<(f64, usize)> = Vec::with_capacity(2 * (mu + nv));
    let mut walk = |i: usize, j: usize, side: usize| {
        let corner = [(0, 0), (mu - 1, 0), (mu - 1, nv - 1), (0, nv - 1)]
            .iter()
            .position(|&c| c == (i, j));
        ring.push((
            corner.map_or_else(|| place(pts[id(i, j)].0, side), |k| k as f64),
            id(i, j),
        ));
    };
    for i in 0..mu {
        walk(i, 0, 0);
    }
    for j in 1..nv {
        walk(mu - 1, j, 1);
    }
    for i in (0..mu - 1).rev() {
        walk(i, nv - 1, 2);
    }
    for j in (1..nv - 1).rev() {
        walk(0, j, 3);
    }
    let strays = |a: usize, b: usize| {
        let (m, x) = (pts[a].0.lerp(pts[b].0, 0.5), pts[a].1.lerp(pts[b].1, 0.5));
        off(x, pr.point(m), m)
    };
    let sags = |t: [usize; 3]| {
        let (p, x) = (t.map(|k| pts[k].0), t.map(|k| pts[k].1));
        let m = Point2::new((p[0].x + p[1].x + p[2].x) / 3.0, (p[0].y + p[1].y + p[2].y) / 3.0);
        // turned the wrong way round in the parameters, it would fold over its neighbours
        (p[1] - p[0]).cross(p[2] - p[0]) <= 0.0 || off(mid3(x[0], x[1], x[2]), pr.point(m), m)
    };
    // the band: the boundary and the ring walked together by their places round the rectangle
    if strays(outer[0].1, ring[0].1) {
        return None;
    }
    let (no, nr) = (outer.len(), ring.len());
    let next = |s: &[(f64, usize)], k: usize| if k + 1 < s.len() { s[k + 1].0 } else { s[0].0 + 4.0 };
    let (mut a, mut b) = (0, 0);
    while a < no || b < nr {
        let t = if b >= nr || (a < no && next(&outer, a) < next(&ring, b)) {
            a += 1;
            [outer[a - 1].1, outer[a % no].1, ring[b % nr].1]
        } else {
            b += 1;
            [outer[a % no].1, ring[b % nr].1, ring[b - 1].1]
        };
        if sags(t) || ((a < no || b < nr) && strays(outer[a % no].1, ring[b % nr].1)) {
            return None;
        }
        tris.push(t);
    }
    let index: Vec<u32> = pts
        .iter()
        .map(|&(_, x, i)| {
            if i == u32::MAX {
                mesh.add_vertex(x)
            } else {
                global[i as usize]
            }
        })
        .collect();
    Some(tris.into_iter().map(|t| t.map(|k| index[k])).collect())
}

/// Faces that are rectangles in parameter space (every revolve face, every
/// extrude wall) need no Delaunay: rows of points across u at each v sample
/// of the side edges, stitched strip by strip. Counter-clockwise in (u, v).
/// None when the face is not such a rectangle (`rectangle`), or its two sides
/// are sampled at different v (stitching rows then would leave T-junctions).
fn strips(
    surface: &Surface,
    uv: &[Point2],
    global: &[u32],
    q: &Quality,
    mesh: &mut TriMesh,
) -> Option<Vec<[u32; 3]>> {
    let ([bottom, right, top, left], u0, u1) = rectangle(uv)?;
    let eps = 1e-9 * (u1 - u0).max(uv[top[0]].y - uv[bottom[0]].y).max(1.0);
    if left.len() != right.len()
        || left
            .iter()
            .zip(&right)
            .any(|(&a, &b)| (uv[a].y - uv[b].y).abs() > eps)
    {
        return None;
    }
    let row_count = |v: f64| -> usize {
        // length per unit u along the row, and the radius that sets the arc step
        let (len, r) = match surface {
            Surface::Plane(_) => return 1,
            Surface::Cylinder(c) => (c.radius, c.radius),
            Surface::Cone(c) => (c.radius_at(v), c.radius_at(v)),
            Surface::Sphere(sp) => (sp.radius * dmath::cos(v), sp.radius * dmath::cos(v)),
            Surface::Torus(t) => (
                t.major + t.minor * dmath::cos(v),
                t.major + t.minor * dmath::cos(v),
            ),
            Surface::Procedural(pr) => (pr.uv_scale().0, pr.min_radius()),
        };
        if r <= forge_math::tol::LINEAR {
            return 1;
        }
        let step = if doubly_curved(surface) {
            arc_step(r, q) / core::f64::consts::SQRT_2
        } else {
            arc_step(r, q)
        };
        ((u1 - u0) * len / (r * step)).ceil().max(1.0) as usize
    };
    // each row: (u, mesh index), increasing u
    let added = mesh.positions.len();
    let mut rows: Vec<Vec<(f64, u32)>> = Vec::with_capacity(left.len());
    let (mut trig, mut pts) = (Vec::new(), Vec::new());
    rows.push(bottom.iter().map(|&i| (uv[i].x, global[i])).collect());
    for j in 1..left.len() - 1 {
        let v = uv[left[j]].y;
        let m = row_count(v);
        let mut row = vec![(u0, global[left[j]])];
        surface.eval_row(v, u0, u1, m, &mut trig, &mut pts);
        for (k, &p) in pts.iter().enumerate() {
            row.push((u0 + (u1 - u0) * ((k + 1) as f64 / m as f64), mesh.add_vertex(p)));
        }
        row.push((u1, global[right[j]]));
        rows.push(row);
    }
    rows.push(top.iter().map(|&i| (uv[i].x, global[i])).collect());
    let mut tris = Vec::new();
    for w in rows.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let (mut i, mut k) = (0, 0);
        while i + 1 < a.len() || k + 1 < b.len() {
            if k + 1 >= b.len() || (i + 1 < a.len() && a[i + 1].0 <= b[k + 1].0) {
                tris.push([a[i].1, a[i + 1].1, b[k].1]);
                i += 1;
            } else {
                tris.push([a[i].1, b[k + 1].1, b[k].1]);
                k += 1;
            }
        }
    }
    // a cone's radius changes between rows, so a triangle reaching across can sag past the
    // chord in its middle or along a side: each is checked (cheaply: the distance is exact),
    // and where one strays the Delaunay meshes the face instead
    if let Surface::Cone(_) = surface {
        let off = |x: forge_math::Point3| surface.implicit(x).abs() > q.chord;
        let strays = tris.iter().any(|t| {
            let p = t.map(|i| mesh.positions[i as usize]);
            off(p[0].lerp(p[1], 0.5).lerp(p[2], 1.0 / 3.0))
                || (0..3).any(|k| off(p[k].lerp(p[(k + 1) % 3], 0.5)))
        });
        if strays {
            mesh.positions.truncate(added);
            return None;
        }
    }
    Some(tris)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Point3, Vec3};
    use forge_ops::{Profile, extrude};

    fn closed(m: &TriMesh) {
        let open = m.open_edges();
        assert!(
            open.is_empty(),
            "{} open edges, e.g. {:?}",
            open.len(),
            &open[..open.len().min(5)]
        );
        assert_eq!(m.degenerate_triangles(1e-12), 0);
    }

    #[test]
    fn box_mesh_is_exact() {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)],
        };
        let b = extrude(&p, 5.0, 1).unwrap();
        let m = tessellate(&b, &Quality::default()).unwrap();
        closed(&m);
        assert_eq!(m.triangles.len(), 12);
        assert!((m.volume() - 1000.0).abs() < 1e-9, "{}", m.volume());
    }

    #[test]
    fn curved_parts_are_closed_and_close() {
        let q = Quality::default();
        let mut r = Rng::new(600);
        for _ in 0..60 {
            let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
                .normalize()
                .unwrap();
            let plane = Frame::from_z(Point3::new(r.range(-50.0, 50.0), r.range(-50.0, 50.0), 0.0), z);
            let (w, h) = (r.range(10.0, 60.0), r.range(10.0, 60.0));
            let rad = r.range(0.5, w.min(h) * 0.45);
            let hole = w.min(h) * 0.2;
            let p = Profile {
                plane,
                loops: vec![
                    Profile::rounded_rectangle(Point2::ORIGIN, w, h, rad, 0),
                    Profile::circle(Point2::new(w / 2.0, h / 2.0), hole, 50),
                ],
            };
            let d = r.range(1.0, 20.0) * if r.bool() { -1.0 } else { 1.0 };
            let b = extrude(&p, d, 1).unwrap();
            let m = tessellate(&b, &q).unwrap();
            closed(&m);
            let exact = p.normalized().unwrap().area() * d.abs();
            // inscribed chords lose at most chord x perimeter of area per unit depth
            let perimeter = 2.0 * (w + h) + dmath::TAU * hole;
            let err = (m.volume() - exact).abs();
            assert!(err <= q.chord * perimeter * d.abs(), "{} vs {exact}", m.volume());
        }
    }

    #[test]
    fn chords_stay_within_tolerance() {
        let q = Quality {
            chord: 0.005,
            max_angle: 0.5,
        };
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![
                Profile::circle(Point2::ORIGIN, 25.0, 0),
                Profile::circle(Point2::ORIGIN, 10.0, 1),
            ],
        };
        let b = extrude(&p, 4.0, 1).unwrap();
        let m = tessellate(&b, &q).unwrap();
        closed(&m);
        // every triangle centroid lies within the chord tolerance of its face
        for (t, &f) in m.triangles.iter().zip(&m.face_of) {
            let c = t
                .iter()
                .fold(Vec3::ZERO, |a, &i| a + m.positions[i as usize].to_vec())
                / 3.0;
            let d = b.face(FaceId(f)).surface.distance(Point3::new(c.x, c.y, c.z));
            assert!(d <= q.chord, "{d}");
        }
    }
}

#[cfg(test)]
mod revolve_tests {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Vec2};
    use forge_ops::{Axis2, Profile, ProfileLoop, Segment, revolve};

    const Y: Axis2 = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };

    fn mesh_volume(p: &Profile, angle: f64) -> f64 {
        let b = revolve(p, Y, angle, 1).unwrap();
        let m = tessellate(
            &b,
            &Quality {
                chord: 0.002,
                max_angle: 0.1,
            },
        )
        .unwrap();
        let open = m.open_edges();
        assert!(open.is_empty(), "{} open edges", open.len());
        m.volume()
    }

    fn rel(a: f64, b: f64) -> f64 {
        (a - b).abs() / b.abs()
    }

    #[test]
    fn exact_volumes() {
        let q = dmath::FRAC_PI_2;
        let r = 5.0;
        let half = ProfileLoop::new(vec![
            (
                Segment::Arc {
                    center: Point2::ORIGIN,
                    radius: r,
                    start: -q,
                    sweep: 2.0 * q,
                },
                0,
            ),
            (
                Segment::Line {
                    a: Point2::new(0.0, r),
                    b: Point2::new(0.0, -r),
                },
                1,
            ),
        ]);
        let sphere = Profile {
            plane: Frame::WORLD,
            loops: vec![half],
        };
        let v = mesh_volume(&sphere, dmath::TAU);
        assert!(rel(v, 4.0 / 3.0 * dmath::PI * r * r * r) < 2e-3, "sphere {v}");

        let torus = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::circle(Point2::new(10.0, 0.0), 3.0, 0)],
        };
        let v = mesh_volume(&torus, dmath::TAU);
        assert!(
            rel(v, 2.0 * dmath::PI * dmath::PI * 10.0 * 9.0) < 2e-3,
            "torus {v}"
        );

        let tri = ProfileLoop::new(vec![
            (
                Segment::Line {
                    a: Point2::ORIGIN,
                    b: Point2::new(4.0, 0.0),
                },
                0,
            ),
            (
                Segment::Line {
                    a: Point2::new(4.0, 0.0),
                    b: Point2::new(0.0, 7.0),
                },
                1,
            ),
            (
                Segment::Line {
                    a: Point2::new(0.0, 7.0),
                    b: Point2::ORIGIN,
                },
                2,
            ),
        ]);
        let cone = Profile {
            plane: Frame::WORLD,
            loops: vec![tri],
        };
        let v = mesh_volume(&cone, dmath::TAU);
        assert!(rel(v, dmath::PI * 16.0 * 7.0 / 3.0) < 2e-3, "cone {v}");
    }

    #[test]
    fn pappus_on_random_profiles() {
        let mut rng = Rng::new(800);
        for _ in 0..25 {
            let x0 = rng.range(0.5, 15.0);
            let (w, h) = (rng.range(3.0, 15.0), rng.range(3.0, 15.0));
            let rad = rng.range(0.3, w.min(h) * 0.45);
            let mut loops = vec![Profile::rounded_rectangle(Point2::new(x0, 0.0), w, h, rad, 0)];
            if rng.bool() {
                loops.push(Profile::circle(
                    Point2::new(x0 + w / 2.0, h / 2.0),
                    w.min(h) * 0.2,
                    40,
                ));
            }
            let p = Profile {
                plane: Frame::WORLD,
                loops,
            };
            let angle = if rng.bool() {
                dmath::TAU
            } else {
                rng.range(0.3, 6.0)
            };
            // symmetric profile: the centroid is the rectangle's centre
            let want = angle * (x0 + w / 2.0) * p.normalized().unwrap().area();
            let v = mesh_volume(&p, angle);
            assert!(rel(v, want) < 3e-3, "{v} vs {want}");
        }
    }
}
