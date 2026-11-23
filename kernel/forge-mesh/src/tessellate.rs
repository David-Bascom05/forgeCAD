use crate::cdt::{CdtError, Refine, Triangulation, triangulate_as_far, triangulate_within, zip};
use crate::mesh::TriMesh;
use forge_geom::CurveGeom;
use forge_geom::{Curve, Surface, SurfaceGeom};
use forge_math::{Point2, dmath};
use forge_topo::uv::{align_loops, draw_loop_near};
use forge_topo::{Body, CoedgeId, FaceId};

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

/// Angle step that keeps chords on a circle of radius `r` within quality, exactly: the step θ
/// whose sag r (1 − cos θ/2) is the chord, as 4 asin √(chord / 2r) (1 − cos θ/2 = 2 sin² θ/4;
/// 2 acos(1 − chord / r) loses the digits of a small chord / r). No floor: a step too small for
/// its edge is the point cap's typed error.
fn arc_step(r: f64, q: &Quality) -> f64 {
    let x = q.chord / r;
    let by_chord = if x >= 1.0 {
        dmath::PI
    } else {
        4.0 * dmath::asin((0.5 * x).sqrt())
    };
    by_chord.min(q.max_angle)
}

/// Chords along an edge. On a face curved both ways a chord along the edge and one across
/// it make a triangle that sags by both their sags: there each gets half of the tolerance
/// (`fine`; research/mesh-bounds/), by each curve's own exact bound at half the chord.
fn edge_segments(curve: &Curve, range: forge_math::Interval, q: &Quality, fine: bool) -> usize {
    let width = range.width();
    // half the tolerance, exactly (each bound below is closed form in it), the angle as it was
    let half = Quality {
        chord: 0.5 * q.chord,
        ..*q
    };
    let q = if fine { &half } else { q };
    let n = match curve {
        Curve::Line(_) => 1.0,
        Curve::Circle(c) => width / arc_step(c.radius, q),
        Curve::Ellipse(e) => width / arc_step(e.major, q),
        Curve::Parabola(c) => conic_segments(width, 1.0 / (2.0 * c.focal), 2.0 * c.focal, q),
        Curve::Hyperbola(c) => conic_segments(width, c.a, c.b / c.a, q),
        // the chain already follows the curvature; split each link by the chord its sag allows
        Curve::Intersection(c) => width * (c.sag / q.chord).sqrt().max(1.0),
        Curve::Procedural(c) => c.segments(range, q.chord) as f64,
    };
    // a curved edge gets two chords at least: a face bounded by a line and a short arc
    // still has three corners to triangulate
    let least = if matches!(curve, Curve::Line(_)) { 1 } else { 2 };
    (n.ceil() as usize).max(least)
}

/// Chords of equal parameter steps along a conic whose second derivative is at most `bend` long
/// (mm per unit parameter squared) and whose tangent turns at most one radian per `turn` of
/// parameter: a chord of step h strays from its curve by at most h^2 bend / 8 (the linear
/// interpolant's error, with its kernel, (t - t0)(t1 - t) / 2, at most h^2 / 8), and turns through
/// at most h / turn.
/// - parabola P(t) = (t^2 / 4f) x + t y: P'' = x / 2f, its tangent turns 1 / 2f per unit at most
///   (at the vertex);
/// - hyperbola P(t) = a sqrt(1 + t^2) x + b t y: |P''| = a (1 + t^2)^(-3/2) <= a, and its tangent
///   turns |P' x P''| / |P'|^2 <= a b / b^2 = a / b per unit (at t = 0).
fn conic_segments(width: f64, bend: f64, turn: f64, q: &Quality) -> f64 {
    let by_chord = width * (bend.abs() / (8.0 * q.chord)).sqrt();
    let by_angle = width / (turn.abs() * q.max_angle);
    by_chord.max(by_angle)
}

/// Whether the triangle `x`, its corners on the sphere of centre `o` and radius `r`, sags past
/// `chord` at the centre's foot on its plane, the deepest point of the triangle when it lies in
/// it (cold: only a triangle whose centroid sags near the chord asks).
#[cold]
#[inline(never)]
fn foot_sags(o: forge_math::Point3, r: f64, x: [forge_math::Point3; 3], chord: f64) -> bool {
    let n = (x[1] - x[0]).cross(x[2] - x[0]);
    let nn = n.dot(n);
    if nn <= 0.0 {
        return false;
    }
    let foot = o + n * ((x[0] - o).dot(n) / nn);
    // in the triangle: on the inner side of each edge (as the normal turns)
    let side = |a: forge_math::Point3, b: forge_math::Point3| (b - a).cross(foot - a).dot(n) >= 0.0;
    side(x[0], x[1]) && side(x[1], x[2]) && side(x[2], x[0]) && (foot.distance(o) - r).abs() > chord
}

/// Whether the chord x–y, its ends on the cone `c`, sags past `chord` anywhere: the signed
/// distance (ρ − R₀)·cos α − z·sin α is convex along the chord (ρ the norm of an affine map), so
/// its deepest point is where its slope is zero, ρ′ = tan α·Δz (a root of a quadratic in the
/// chord's parameter, (q + sp)² = k²(c₀ + 2sq + s²p)), or where the chord passes nearest the axis
/// (ρ's kink); each candidate inside the chord is judged (cold: only a chord whose middle sags
/// past half the chord asks).
#[cold]
#[inline(never)]
fn cone_chord_sags(c: &forge_geom::Cone, x: forge_math::Point3, y: forge_math::Point3, chord: f64) -> bool {
    let (a, b) = (c.frame.point_to_local(x), c.frame.point_to_local(y));
    let (dx, dy, dz) = (b.x - a.x, b.y - a.y, b.z - a.z);
    let p = dx * dx + dy * dy;
    let q = a.x * dx + a.y * dy;
    let c0 = a.x * a.x + a.y * a.y;
    let k = c.tan_a() * dz;
    let sags = |s: f64| s > 0.0 && s < 1.0 && Surface::Cone(*c).implicit(x.lerp(y, s)).abs() > chord;
    if p <= 0.0 {
        return false;
    }
    // nearest the axis
    if sags(-q / p) {
        return true;
    }
    // the slope's zeros: p(p − k²)s² + 2q(p − k²)s + (q² − k²c₀) = 0
    let a2 = p * (p - k * k);
    if a2 == 0.0 {
        return false;
    }
    let disc = (q / p) * (q / p) - (q * q - k * k * c0) / a2;
    if disc < 0.0 {
        return false;
    }
    let (m, h) = (-q / p, disc.sqrt());
    sags(m - h) || sags(m + h)
}

/// Whether the triangle `x`, its corners on the cone `c`, sags past `chord` where the cone's
/// axis crosses it (the distance's kink, ρ = 0).
fn cone_axis_sags(c: &forge_geom::Cone, x: [forge_math::Point3; 3], chord: f64) -> bool {
    let l = x.map(|p| c.frame.point_to_local(p));
    // the axis meets the triangle only if the triangle's shadow on the xy plane holds the origin
    let cross = |a: forge_math::Point3, b: forge_math::Point3| a.x * b.y - a.y * b.x;
    let (s0, s1, s2) = (cross(l[0], l[1]), cross(l[1], l[2]), cross(l[2], l[0]));
    if !((s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0)) {
        return false;
    }
    cone_axis_point_sags(c, l, chord)
}

#[cold]
#[inline(never)]
fn cone_axis_point_sags(c: &forge_geom::Cone, l: [forge_math::Point3; 3], chord: f64) -> bool {
    // the axis's point in the triangle's plane: weights from the shadow, the height from them
    let cross = |a: forge_math::Point3, b: forge_math::Point3| a.x * b.y - a.y * b.x;
    let w = [cross(l[1], l[2]), cross(l[2], l[0]), cross(l[0], l[1])];
    let total = w[0] + w[1] + w[2];
    if total == 0.0 {
        return false;
    }
    let z = (w[0] * l[0].z + w[1] * l[1].z + w[2] * l[2].z) / total;
    Surface::Cone(*c)
        .implicit(c.frame.point_to_world(forge_math::Point3::new(0.0, 0.0, z)))
        .abs()
        > chord
}

/// On a torus (major R, minor r) a point's F = h² − r², h its distance from the core circle:
/// in the torus's frame |p|² + R² − r² − 2R·ρ (ρ its distance from the axis). |h − r| = |F|/(h + r).
fn torus_f(t: &forge_geom::Torus, p: forge_math::Point3) -> f64 {
    let l = t.frame.point_to_local(p);
    let rho = (l.x * l.x + l.y * l.y).sqrt();
    l.x * l.x + l.y * l.y + l.z * l.z + t.major * t.major - t.minor * t.minor - 2.0 * t.major * rho
}

/// Whether the chord x–y, its ends on the torus `t`, is shown within `chord` of it. Along the
/// chord a + s·d (in the torus's frame) F is zero at its ends, and its third derivative is
/// −2R·ρ''' with ρ''' = −3(αγ − β²)(γs + β)/ρ⁵ (ρ² = α + 2βs + γs², over the shadows on the
/// plane square to the axis): at most 6R·(αγ − β²)·max(|β|, |β + γ|)/ρ⁵, ρ its least on the
/// chord. So F strays from the parabola through its ends and middle by at most that over 6·(√3/36);
/// and a point where |h − r| reached the chord would have |F| ≥ chord·(2r − chord). A chord that
/// reaches the axis (ρ's kink) is not shown within it, and is split. (A tube no thicker than the
/// chord is left to the middle's test.)
fn torus_chord_holds(
    t: &forge_geom::Torus,
    x: forge_math::Point3,
    y: forge_math::Point3,
    chord: f64,
) -> bool {
    if t.minor <= chord {
        return true;
    }
    let (a, b) = (t.frame.point_to_local(x), t.frame.point_to_local(y));
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (al, be, ga) = (a.x * a.x + a.y * a.y, a.x * dx + a.y * dy, dx * dx + dy * dy);
    // the least ρ² on the chord
    let s = if ga > 0.0 { (-be / ga).clamp(0.0, 1.0) } else { 0.0 };
    let rho2 = al + 2.0 * be * s + ga * s * s;
    if rho2 <= 0.0 {
        return false;
    }
    let rho = rho2.sqrt();
    // (αγ − β² = γ·ρ² at the nearest point when it lies inside the chord: exactly, without the
    // cancellation, so a chord passing near the axis is not shown within the chord by rounding)
    let cross2 = if s > 0.0 && s < 1.0 {
        ga * rho2
    } else {
        (al * ga - be * be).max(0.0)
    };
    let third = 6.0 * t.major * cross2 * be.abs().max((be + ga).abs()) / (rho2 * rho2 * rho);
    let stray = third * 3f64.sqrt() / 216.0;
    torus_f(t, x.lerp(y, 0.5)).abs() + stray < chord * (2.0 * t.minor - chord)
}

/// Whether the triangle `x`, its corners on the torus `t`, lies within `chord` of it inside (its
/// edges are judged as chords). F = h² − r² has gradient 2(p − c(p)), c(p) the core circle's
/// point nearest p, so in the triangle's plane (normal n) its slope is zero only where p − c ∥ n:
/// c at the angle of n's shadow on the plane square to the axis or the opposite one, p = c + t·n
/// on the plane; besides, F has a kink on the axis. Each such point in the triangle is judged.
/// (A plane square to the axis has F constant round the circle ρ = R in it, whose points in the
/// triangle reach its edges unless the triangle holds the whole circle: a triangle that large is
/// left to its other tests.)
fn torus_triangle_holds(t: &forge_geom::Torus, x: [forge_math::Point3; 3], chord: f64) -> bool {
    let l = x.map(|p| t.frame.point_to_local(p));
    let n = (l[1] - l[0]).cross(l[2] - l[0]);
    let nn = n.length();
    if nn <= 0.0 {
        return true;
    }
    let n = n * (1.0 / nn);
    let inside = |p: forge_math::Point3| {
        let side = |a: forge_math::Point3, b: forge_math::Point3| (b - a).cross(p - a).dot(n) >= 0.0;
        side(l[0], l[1]) && side(l[1], l[2]) && side(l[2], l[0])
    };
    let off = |p: forge_math::Point3| {
        let rho = (p.x * p.x + p.y * p.y).sqrt();
        let d = rho - t.major;
        ((d * d + p.z * p.z).sqrt() - t.minor).abs() > chord
    };
    let nxy = (n.x * n.x + n.y * n.y).sqrt();
    if nxy > 0.0 {
        for sign in [1.0, -1.0] {
            let (ex, ey) = (sign * n.x / nxy, sign * n.y / nxy);
            let c = forge_math::Point3::new(t.major * ex, t.major * ey, 0.0);
            let s = n.dot(l[0] - c);
            let p = c + n * s;
            // (its shadow on the plane square to the axis along c's direction, past the axis)
            if t.major + s * sign * nxy > 0.0 && inside(p) && off(p) {
                return false;
            }
        }
    }
    // the axis's point in the plane, when the triangle's shadow holds the axis
    let cross = |a: forge_math::Point3, b: forge_math::Point3| a.x * b.y - a.y * b.x;
    let w = [cross(l[1], l[2]), cross(l[2], l[0]), cross(l[0], l[1])];
    let total = w[0] + w[1] + w[2];
    if total != 0.0 && w.iter().all(|&v| v * total >= 0.0) {
        let z = (w[0] * l[0].z + w[1] * l[1].z + w[2] * l[2].z) / total;
        if off(forge_math::Point3::new(0.0, 0.0, z)) {
            return false;
        }
    }
    true
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
        Surface::Cone(c) => (c.radius.max(forge_math::tol::LINEAR), 1.0 / c.cos_a()),
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
        // into ones as long, and leave triangles across two columns: 3.5 chords deep;
        // research/mesh-bounds/)
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
    let cross = d.du.cross(d.dv);
    let Some(n) = cross.normalize() else {
        return (0.0, 0.0);
    };
    let n = n.get();
    let (e, g) = (d.du.dot(d.du), d.dv.dot(d.dv));
    // on a pole in all but rounding (a file's revolved face): that normal is noise, over a square
    // of noise, and would ask for a million steps both ways
    if e <= 0.0 || g <= 0.0 || cross.length() <= 1e-8 * e.max(g) {
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
            Surface::Procedural(pr) => pr.mesh_radius(),
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

/// The most an edge is cut into, and the most cells an analytic face's grid has.
const MAX_EDGE_CUTS: usize = 1 << 22;

/// The most points a procedural face's first grid has.
const MAX_GRID: usize = 1 << 15;

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
    tessellate_with(body, q, false)
}

/// `tessellate`, but a body from a file (`Body::from_file`), whose edges may meet only within their
/// tolerance and so cross where its faces are laid flat, has those faces drawn as they are: the mesh
/// parts from the next face by that little there instead of failing. On a body made here a crossing
/// is a bug, and it stays an error.
pub fn tessellate_tolerant(body: &Body, q: &Quality) -> Result<TriMesh, TessError> {
    if !body.from_file {
        return tessellate(body, q);
    }
    if body.dead > 0 {
        return tessellate_tolerant(&body.compact(), q);
    }
    tessellate_with(body, q, true)
}

impl TessError {
    /// A boundary that crosses or repeats itself or has two points in one place.
    pub fn tangled(&self) -> bool {
        let TessError::Face { error, .. } = self;
        matches!(
            error,
            CdtError::ConstraintsIntersect { .. }
                | CdtError::ConstraintThroughPoint { .. }
                | CdtError::DuplicatePoint { .. }
                | CdtError::BadLoop { .. }
        )
    }
}

/// The edges' samples, shared by the faces on each side, and for each mesh vertex that is a sample
/// inside an edge, that edge and the sample's parameter.
struct Samples {
    of: Vec<Vec<(f64, u32)>>,
    owner: Vec<(u32, f64)>,
}

impl Samples {
    /// The chord of an edge between mesh vertices `a` and `b`, as that edge and the index of its
    /// first sample, if the two are neighbours along one edge.
    fn chord(&self, body: &Body, a: u32, b: u32) -> Option<(usize, usize)> {
        let owned = |x: u32| self.owner.get(x as usize).is_some_and(|o| o.0 != u32::MAX);
        if !owned(a) && !owned(b) {
            // between two of the body's vertices (or vertices taken for them): a line, or an
            // edge whose samples joined others'
            return (0..self.of.len()).find_map(|e| {
                let s = &self.of[e];
                s.windows(2)
                    .position(|w| (w[0].1, w[1].1) == (a, b) || (w[0].1, w[1].1) == (b, a))
                    .map(|k| (e, k))
                    .filter(|_| !body.edge(forge_topo::EdgeId(e as u32)).coedges.is_empty())
            });
        }
        for (x, y) in [(a, b), (b, a)] {
            if !owned(x) {
                continue;
            }
            let (e, t) = self.owner[x as usize];
            let s = &self.of[e as usize];
            let k = s.partition_point(|p| p.0 < t);
            if s.get(k).is_none_or(|p| p.1 != x) {
                continue;
            }
            if s.get(k + 1).is_some_and(|p| p.1 == y) {
                return Some((e as usize, k));
            }
            if k > 0 && s[k - 1].1 == y {
                return Some((e as usize, k - 1));
            }
        }
        None
    }

    /// The samples changed by `cut`, and the edges it changed.
    fn apply(&mut self, body: &Body, cut: Cut, mesh: &mut TriMesh, changed: &mut Vec<usize>) {
        match cut {
            Cut::At { edge, t, v } => {
                let s = &mut self.of[edge];
                let k = s.partition_point(|p| p.0 < t);
                let id = v.unwrap_or_else(|| {
                    let id = mesh.add_vertex(body.edge(forge_topo::EdgeId(edge as u32)).curve.eval(t));
                    self.owner.resize(id as usize + 1, (u32::MAX, 0.0));
                    self.owner[id as usize] = (edge as u32, t);
                    id
                });
                s.insert(k, (t, id));
                changed.push(edge);
            }
            Cut::Cross { e, t, f, u } => {
                let id = mesh.add_vertex(body.edge(forge_topo::EdgeId(e as u32)).curve.eval(t));
                self.owner.resize(id as usize + 1, (u32::MAX, 0.0));
                self.owner[id as usize] = (e as u32, t);
                for (g, w) in [(e, t), (f, u)] {
                    let s = &mut self.of[g];
                    let k = s.partition_point(|p| p.0 < w);
                    s.insert(k, (w, id));
                    changed.push(g);
                }
            }
            Cut::Same { from, to } => {
                for (e, s) in self.of.iter_mut().enumerate() {
                    let mut hit = false;
                    for x in s.iter_mut().filter(|x| x.1 == from) {
                        x.1 = to;
                        hit = true;
                    }
                    if hit {
                        // (a vertex met twice in a row is met once)
                        s.dedup_by_key(|x| x.1);
                        changed.push(e);
                    }
                }
                if let Some(o) = self.owner.get_mut(from as usize) {
                    o.0 = u32::MAX;
                }
            }
        }
    }
}

fn tessellate_with(body: &Body, q: &Quality, mend: bool) -> Result<TriMesh, TessError> {
    let mut mesh = TriMesh::default();
    let vertex_index: Vec<u32> = body.vertices.iter().map(|v| mesh.add_vertex(v.point)).collect();

    // per edge: its parameter samples and their mesh indices (ends are the body's vertices)
    let mut samples = Samples {
        of: Vec::with_capacity(body.edges.len()),
        owner: vec![(u32::MAX, 0.0); vertex_index.len()],
    };
    let (mut trig, mut pts) = (Vec::new(), Vec::new());
    for e in body.edge_ids() {
        let edge = body.edge(e);
        let fine = edge
            .coedges
            .iter()
            .any(|&c| doubly_curved(&body.face(body.coedge_face(c)).surface));
        let n = edge_segments(&edge.curve, edge.range, q, fine);
        // a file's arc of a huge radius, or a bad number: no room to be had, and no mesh
        if n > MAX_EDGE_CUTS {
            return Err(TessError::Face {
                face: body.coedge_face(edge.coedges[0]),
                error: CdtError::Internal("an edge would take too many points"),
            });
        }
        let mut s = Vec::with_capacity(n + 1);
        s.push((edge.range.lo, vertex_index[edge.start.index()]));
        edge.curve.sample_interior(edge.range, n, &mut trig, &mut pts);
        for (i, &p) in pts.iter().enumerate() {
            let t = edge.range.lo + edge.range.width() * ((i + 1) as f64 / n as f64);
            let id = mesh.add_vertex(p);
            samples.owner.push((e.0, t));
            s.push((t, id));
        }
        s.push((edge.range.hi, vertex_index[edge.end.index()]));
        samples.of.push(s);
    }

    // each face's boundary drawn flat, its crossing chords split first (on their edges, so the
    // faces on both sides take the new samples), until no face's chords cross
    let nf = body.faces.len();
    // (boxed: a prep holds a procedural face's 9 x 9 nodes, and moves)
    let mut preps: Vec<Option<Box<Prep>>> = (0..nf).map(|_| None).collect();
    let mut splits = 0usize;
    // a file's face is drawn again for its cuts this many times, then drawn as it is (`mend`):
    // each drawing projects all its boundary again, and a damaged face's crossings within its
    // tolerance need not end where a body made here's do (the base gave a file's faces 3 rounds)
    const FILE_ROUNDS: u32 = 8;
    let mut drawn = if mend { vec![0u32; nf] } else { Vec::new() };
    // (the first face not drawn: faces before it stay drawn until a cut moves it back)
    let mut i = 0;
    while i < nf {
        if preps[i].is_some() {
            i += 1;
            continue;
        }
        let f = FaceId(i as u32);
        let prep = prepare_face(body, f, &samples.of, q, &mesh)
            .map_err(|error| TessError::Face { face: f, error })?;
        let tired = mend && {
            drawn[i] += 1;
            drawn[i] > FILE_ROUNDS
        };
        let cuts = if tired {
            Vec::new()
        } else {
            uncross(body, &prep, &samples, &mesh, mend).map_err(|error| TessError::Face { face: f, error })?
        };
        if cuts.is_empty() {
            preps[i] = Some(prep);
            continue;
        }
        splits += cuts.len();
        if splits > MAX_EDGE_CUTS {
            return Err(TessError::Face {
                face: f,
                error: CdtError::Internal("crossing chords would take too many points"),
            });
        }
        let mut changed = Vec::new();
        for cut in cuts {
            samples.apply(body, cut, &mut mesh, &mut changed);
        }
        for e in changed {
            for &c in &body.edge(forge_topo::EdgeId(e as u32)).coedges {
                let g = body.coedge_face(c).index();
                preps[g] = None;
                i = i.min(g);
            }
        }
    }
    let edge_samples = samples.of;

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
    for (i, prep) in preps.into_iter().enumerate() {
        let f = FaceId(i as u32);
        let prep = prep.expect("every face drawn");
        mesh_face(body, f, prep, &segments, q, mend, &mut mesh)
            .map_err(|error| TessError::Face { face: f, error })?;
    }
    mesh.edges = edge_samples
        .iter()
        .map(|s| s.iter().map(|&(_, v)| v).collect())
        .collect();
    Ok(mesh)
}

/// A change to the edges' samples, so a face's boundary chords stop crossing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cut {
    /// Edge `edge` takes a sample at `t`: mesh vertex `v` (a point of another edge's on its curve),
    /// or a new one on it.
    At { edge: usize, t: f64, v: Option<u32> },
    /// Mesh vertex `from` is `to` (two boundary points one place): every edge takes `to` for it.
    Same { from: u32, to: u32 },
    /// Two edges of a file's body that cross within its tolerance: each takes a sample where
    /// they cross (at `t` and `u`), one new mesh vertex for both, so the face is pinched there
    /// and the faces across both edges take it too.
    Cross { e: usize, t: f64, f: usize, u: f64 },
}

/// The cuts that make a face's boundary chords, as drawn flat for its triangulation, meet only
/// at shared ends: none crossing, touching or running along another, and no two points in one
/// place.
///
/// Every pair of chords is decided once, exactly (orient2d on the very numbers the triangulation
/// is given), and a pair in conflict is resolved by one rule, in this order:
/// - a point of one chord on the other's curve, within LINEAR (the model's: two points that close
///   are one), is on it: at one of that chord's ends it is that end's vertex (`Same`: a hole
///   tangent inside a rim, each with a sample at the touch), else that curve's sample there, the
///   same mesh vertex (a vertex of one on the other, as a hole's on its rim);
/// - else the longer chord (as drawn) of the two that can be cut is halved in its parameter: a
///   chord of a curve, longer than its edge's tolerance (LINEAR, or a file's edge's own, wider: a
///   chord shorter is its curve, to it; so a file's edges that cross within their tolerance stop
///   being cut at once, and are drawn as they are).
///
/// Chords that follow their curves closer each round stop crossing once they are closer than the
/// curves are apart; curves that themselves cross leave chords that cannot be cut, and that is the
/// face's own error (`ConstraintsIntersect`), or with `mend` (a file's body, whose edges meet only
/// within its tolerance) the triangulation's to draw as it is.
#[inline(never)]
fn uncross(
    body: &Body,
    prep: &Prep,
    samples: &Samples,
    mesh: &TriMesh,
    mend: bool,
) -> Result<Vec<Cut>, CdtError> {
    let pts = &prep.scaled;
    let global = |i: u32| prep.global[i as usize];
    let orient = forge_math::predicates::orient2d;
    let sign = |x: f64| (x > 0.0) as i8 - (x < 0.0) as i8;
    // closed segments a-b and c-d meet
    let meet = |a: Point2, b: Point2, c: Point2, d: Point2| {
        let (o1, o2) = (sign(orient(a, b, c)), sign(orient(a, b, d)));
        let (o3, o4) = (sign(orient(c, d, a)), sign(orient(c, d, b)));
        if o1 * o2 > 0 || o3 * o4 > 0 {
            return false;
        }
        if o1 == 0 && o2 == 0 {
            // along one line: whether their spans overlap
            let s = |p: Point2| if a.x != b.x { p.x } else { p.y };
            return s(c).max(s(d)) >= s(a).min(s(b)) && s(c).min(s(d)) <= s(a).max(s(b));
        }
        true
    };
    let near = |a: u32, b: u32| {
        mesh.positions[a as usize].distance(mesh.positions[b as usize]) <= forge_math::tol::LINEAR
    };
    let mut cuts: Vec<Cut> = Vec::new();
    let add = |c: Cut, cuts: &mut Vec<Cut>| {
        if !cuts.contains(&c) {
            cuts.push(c);
        }
    };
    // two points in one place, as drawn: one point, if they are one in space
    let same = |x: u32, y: u32, cuts: &mut Vec<Cut>| {
        let (gx, gy) = (global(x), global(y));
        if gx != gy && near(gx, gy) {
            add(
                Cut::Same {
                    from: gx.max(gy),
                    to: gx.min(gy),
                },
                cuts,
            );
        }
    };

    // every pair of chords whose boxes overlap, each once (`forge_topo::chords::near_pairs`:
    // monotone chains, so two chords of one chain that share no end are never compared)
    type Seg = (u32, u32);
    let pair = |s: Seg, t: Seg, cuts: &mut Vec<Cut>| -> Result<(), CdtError> {
        let ((a, b), (c, d)) = (s, t);
        if a == c || a == d || b == c || b == d {
            return Ok(());
        }
        let (pa, pb, pc, pd) = (pts[a as usize], pts[b as usize], pts[c as usize], pts[d as usize]);
        if pa.y.max(pb.y) < pc.y.min(pd.y)
            || pc.y.max(pd.y) < pa.y.min(pb.y)
            || pa.x.max(pb.x) < pc.x.min(pd.x)
            || pc.x.max(pd.x) < pa.x.min(pb.x)
        {
            return Ok(());
        }
        let mut at_one = false;
        for (x, px) in [(a, pa), (b, pb)] {
            for (y, py) in [(c, pc), (d, pd)] {
                if px == py {
                    at_one = true;
                    same(x, y, cuts);
                }
            }
        }
        if at_one || !meet(pa, pb, pc, pd) {
            return Ok(());
        }
        match resolve(
            body,
            samples,
            mesh,
            [global(a), global(b)],
            [global(c), global(d)],
            [pa.distance(pb), pc.distance(pd)],
        ) {
            Some(c) => add(c, cuts),
            // the curves themselves cross, as drawn
            // a file's: cut where they cross, on both, one vertex
            None if mend => {
                if let Some(c) = crossing(
                    body,
                    samples,
                    mesh,
                    [global(a), global(b)],
                    [global(c), global(d)],
                ) {
                    add(c, cuts);
                }
            }
            None => return Err(CdtError::ConstraintsIntersect { a, b }),
        }
        Ok(())
    };
    let mut zeros: Vec<Seg> = Vec::new();
    forge_topo::chords::near_pairs(
        &pts[..],
        &prep.loops,
        |a, b| zeros.push((a, b)),
        |s, t| pair(s, t, &mut cuts),
    )?;
    for (a, b) in zeros {
        same(a, b, &mut cuts);
    }
    Ok(cuts)
}

/// The cut for two chords in conflict, between mesh vertices `s` and `t`, drawn `drawn` long.
fn resolve(
    body: &Body,
    samples: &Samples,
    mesh: &TriMesh,
    s: [u32; 2],
    t: [u32; 2],
    drawn: [f64; 2],
) -> Option<Cut> {
    let chords = [s, t].map(|x| samples.chord(body, x[0], x[1]));
    let curve = |e: usize| &body.edge(forge_topo::EdgeId(e as u32)).curve;
    // a point of the other chord on this one's curve
    for (x, other) in [(0, t), (1, s)] {
        let Some((e, k)) = chords[x] else { continue };
        let (sa, sb) = (samples.of[e][k], samples.of[e][k + 1]);
        let c = curve(e);
        for v in other {
            let p = mesh.positions[v as usize];
            let at = c.closest(p, forge_math::Interval::new(sa.0, sb.0));
            if c.eval(at).distance(p) > forge_math::tol::LINEAR {
                continue;
            }
            let end = if at <= sa.0 {
                Some(sa.1)
            } else if at >= sb.0 {
                Some(sb.1)
            } else {
                None
            };
            return Some(match end {
                Some(w) if w == v => continue,
                Some(w) if mesh.positions[w as usize].distance(p) <= forge_math::tol::LINEAR => Cut::Same {
                    from: v.max(w),
                    to: v.min(w),
                },
                Some(_) => continue,
                None => Cut::At {
                    edge: e,
                    t: at,
                    v: Some(v),
                },
            });
        }
    }
    // the longer that can be cut, halved
    let mut best: Option<(f64, usize, f64)> = None;
    for x in 0..2 {
        let Some((e, k)) = chords[x] else { continue };
        let (sa, sb) = (samples.of[e][k], samples.of[e][k + 1]);
        let m = 0.5 * (sa.0 + sb.0);
        let long = mesh.positions[sa.1 as usize].distance(mesh.positions[sb.1 as usize]);
        let tol = body
            .edge(forge_topo::EdgeId(e as u32))
            .tol
            .max(forge_math::tol::LINEAR);
        if matches!(curve(e), Curve::Line(_)) || !(m > sa.0 && m < sb.0) || long <= tol {
            continue;
        }
        if best.is_none_or(|b| drawn[x] > b.0) {
            best = Some((drawn[x], e, m));
        }
    }
    best.map(|(_, edge, t)| Cut::At { edge, t, v: None })
}

/// Where two chords of curves that cannot be cut further come closest, as a parameter of each
/// curve, if inside both: where a file's two edges cross within its tolerance.
fn crossing(body: &Body, samples: &Samples, mesh: &TriMesh, s: [u32; 2], t: [u32; 2]) -> Option<Cut> {
    let (e, k) = samples.chord(body, s[0], s[1])?;
    let (f, l) = samples.chord(body, t[0], t[1])?;
    if e == f {
        return None;
    }
    let (a0, a1) = (samples.of[e][k], samples.of[e][k + 1]);
    let (b0, b1) = (samples.of[f][l], samples.of[f][l + 1]);
    let p = |v: u32| mesh.positions[v as usize];
    // the closest points of the two segments (both of positive length), by their parameters
    let (d1, d2, r) = (p(a1.1) - p(a0.1), p(b1.1) - p(b0.1), p(a0.1) - p(b0.1));
    let (aa, ee, bb) = (d1.dot(d1), d2.dot(d2), d1.dot(d2));
    let (c, ff) = (d1.dot(r), d2.dot(r));
    let den = aa * ee - bb * bb;
    if aa <= 0.0 || ee <= 0.0 || den <= 0.0 {
        return None;
    }
    let x = ((bb * ff - c * ee) / den).clamp(0.0, 1.0);
    let y = ((bb * x + ff) / ee).clamp(0.0, 1.0);
    let (t, u) = (a0.0 + (a1.0 - a0.0) * x, b0.0 + (b1.0 - b0.0) * y);
    (t > a0.0 && t < a1.0 && u > b0.0 && u < b1.0).then_some(Cut::Cross { e, t, f, u })
}

/// A face's boundary drawn flat for its triangulation, and the layout it is drawn in.
struct Prep {
    su: f64,
    sv: f64,
    uv: Vec<Point2>,
    global: Vec<u32>,
    loops: Vec<Vec<u32>>,
    twice: Vec<u32>,
    /// `uv` laid out: the triangulation's very points (before a cone's apex run is one point).
    scaled: Vec<Point2>,
    lo: Point2,
    hi: Point2,
    cells: Option<Cells>,
    nodes: Option<(Nodes, Counts)>,
    layout: Layout,
}

fn prepare_face(
    body: &Body,
    f: FaceId,
    edge_samples: &[Vec<(f64, u32)>],
    q: &Quality,
    mesh: &TriMesh,
) -> Result<Box<Prep>, CdtError> {
    let face = body.face(f);
    let surface = &face.surface;
    let (su, sv) = match surface {
        // a cone framed at its apex has no radius there: the face's own middle gives its scale
        // (a fixed floor would mesh a part and the part scaled differently)
        Surface::Cone(c) => {
            let mid = c.radius_at(surface.project(body.face_box(f).center()).y);
            (c.radius.max(mid).max(forge_math::tol::LINEAR), 1.0 / c.cos_a())
        }
        _ => uv_scale(surface),
    };

    let mut uv: Vec<Point2> = Vec::new();
    let mut global: Vec<u32> = Vec::new();
    let mut rings: Vec<Vec<Point2>> = Vec::new();
    let mut ring_globals: Vec<Vec<u32>> = Vec::new();
    for &l in &face.loops {
        // edges walked there and back (a tool touched this face along a line) bound nothing; on a
        // plane any such run, elsewhere only a whole loop (it may be the seam up to a cone's apex)
        let there_and_back = |c: CoedgeId| body.partner(c).is_some_and(|p| body.coedge(p).lp == l);
        if body.lp(l).coedges().all(there_and_back) {
            continue;
        }
        let mut walk: Vec<CoedgeId> = Vec::new();
        for c in body.lp(l).coedges() {
            if matches!(surface, Surface::Plane(_))
                && walk.last().is_some_and(|&t| body.partner(t) == Some(c))
            {
                walk.pop();
            } else {
                walk.push(c);
            }
        }
        while matches!(surface, Surface::Plane(_))
            && walk.len() >= 2
            && body.partner(walk[0]) == walk.last().copied()
        {
            walk.pop();
            walk.remove(0);
        }
        let len: usize = walk
            .iter()
            .map(|&c| edge_samples[body.coedge(c).edge.index()].len() - 1)
            .sum();
        let mut samples = Vec::with_capacity(len);
        let mut ids = Vec::with_capacity(len);
        for c in walk {
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
        Surface::Cone(c) if c.radius_at(lo.y) <= forge_math::tol::LINEAR => Layout::Unrolled {
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
            // curvature sampled near a pole can ask for millions of points on a small face: the
            // grid stays affordable, and the refinement adds what is really needed where it is
            if size(&c) > MAX_GRID {
                let f = (MAX_GRID as f64 / size(&c) as f64).sqrt();
                let less = |x: usize| (x as f64 * f).ceil() as usize;
                c.iter_mut().flatten().for_each(|x| *x = (less(x.0), less(x.1)));
                if let Some(b) = cells.as_mut() {
                    b.fit = None;
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
    // where each such point went (id and parameters), looked up rather than searched for: a
    // seam's samples are all met twice, and a linear search for each was quadratic in them
    let mut placed: std::collections::HashMap<(u32, u64, u64), u32> = std::collections::HashMap::new();
    let total: usize = rings.iter().map(|r| r.len()).sum();
    uv.reserve(total);
    global.reserve(total);
    for (pts, ids) in rings.into_iter().zip(ring_globals) {
        // a loop through one vertex twice (a face pinched where a curve touches its boundary):
        // one point, met twice
        let mut lp = Vec::with_capacity(pts.len());
        for (p, id) in pts.into_iter().zip(ids) {
            let key = (id, (p.x + 0.0).to_bits(), (p.y + 0.0).to_bits());
            let again = twice.binary_search(&id).is_ok();
            match again.then(|| placed.get(&key).copied()).flatten() {
                Some(k) => lp.push(k),
                None => {
                    if again {
                        placed.insert(key, uv.len() as u32);
                    }
                    lp.push(uv.len() as u32);
                    uv.push(p);
                    global.push(id);
                }
            }
        }
        loops.push(lp);
    }
    // a chord met there and back in one loop (two edges' chords made one where they run within
    // the tolerance) bounds nothing: only a point met twice can begin one
    if !twice.is_empty() {
        for l in &mut loops {
            drop_spikes(l);
        }
        loops.retain(|l| l.len() >= 3);
    }
    let scaled = uv.iter().map(|&p| layout.put(p)).collect();
    Ok(Box::new(Prep {
        su,
        sv,
        uv,
        global,
        loops,
        twice,
        scaled,
        lo,
        hi,
        cells,
        nodes,
        layout,
    }))
}

fn mesh_face(
    body: &Body,
    f: FaceId,
    prep: Box<Prep>,
    segments: &Segments,
    q: &Quality,
    mend: bool,
    mesh: &mut TriMesh,
) -> Result<(), CdtError> {
    let face = body.face(f);
    let surface = &face.surface;
    let Prep {
        su,
        sv,
        mut uv,
        mut global,
        mut loops,
        twice,
        scaled,
        lo,
        hi,
        cells,
        nodes,
        layout,
    } = *prep;

    if loops.len() == 1
        && !matches!(surface, Surface::Procedural(_))
        && let Some(tris) = strips(surface, &uv, &global, q, mesh)
    {
        mesh.triangles.reserve(tris.len());
        mesh.face_of.reserve(tris.len());
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
            mesh.triangles.reserve(tris.len());
            mesh.face_of.reserve(tris.len());
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

    let mut scaled: Vec<Point2> = match layout {
        // (its apex run made one point above)
        Layout::Unrolled { .. } => uv.iter().map(|&p| layout.put(p)).collect(),
        Layout::Scaled(..) => scaled,
    };
    let mut interior = Vec::new();
    // a cone that tapers little takes the grid of its widest radius: developable, so any chord of
    // a cell sags no more than its way round
    let grid_cone = match surface {
        Surface::Cone(c) => {
            let (a, b) = (c.radius_at(lo.y), c.radius_at(hi.y));
            let (r0, r1) = (a.min(b), a.max(b));
            (r0 >= 0.5 * r1 && r1 > forge_math::tol::LINEAR).then(|| {
                let du = arc_step(r1, q);
                (du, 2.0 * r1 * du)
            })
        }
        _ => None,
    };
    // an analytic face's grid: cells clear of the boundary are triangulated straight off the
    // grid, the band between them and the boundary by the constrained Delaunay
    let mut grid: Option<Grid> = None;
    if let Some((n, c)) = &nodes {
        n.grid(c, &mut interior);
        interior.iter_mut().for_each(|p| *p = layout.put(*p));
    } else if let Some((du, dv)) = grid_cone.or(interior_steps(surface, q)) {
        match surface {
            Surface::Cone(c) if grid_cone.is_none() => {
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
                // (a file's bad number can ask for any grid at all)
                if nu.saturating_mul(nv) > MAX_EDGE_CUTS {
                    return Err(CdtError::Internal("a face would take too many points"));
                }
                let mut g = Grid::new(lo, hi, nu, nv, &uv, &loops);
                // a cell's chords sag within the tolerance only where its steps come from the
                // surface's exact radii (not a cone's, whose radius grows along it)
                let exact = matches!(
                    surface,
                    Surface::Sphere(_) | Surface::Torus(_) | Surface::Cylinder(_) | Surface::Cone(_)
                );
                let stairs = if g.any && exact { g.staircase() } else { None };
                g.settle(stairs.is_some());
                interior.reserve((nu - 1) * (nv - 1));
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
    // sides' (by up to a third; research/mesh-bounds/), so triangles there are looked at too
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
    let cell = match (surface, grid_cone.or(interior_steps(surface, q))) {
        (Surface::Sphere(_) | Surface::Torus(_) | Surface::Cylinder(_), Some((du, dv))) => (du * su, dv * sv),
        (Surface::Cone(_), Some((du, dv))) if grid_cone.is_some() => (du * su, dv * sv),
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
                    || (!bounded2(a, b, grid) && {
                        let (x, y) = (pos(ia, a), pos(ib, b));
                        let m = x.lerp(y, 0.5);
                        off(m, a.lerp(b, 0.5))
                            || match surface {
                                // on a cone a chord's deepest point is not its middle: convex
                                // along it and zero at its ends, its sag is at most twice the
                                // middle's, so only a middle sagging past half the chord asks
                                Surface::Cone(c) => {
                                    surface.implicit(m).abs() > 0.5 * q.chord
                                        && cone_chord_sags(c, x, y, q.chord)
                                }
                                // on a torus, a bound: what its middle sags and how far the chord
                                // can stray from the parabola through its ends and middle
                                Surface::Torus(t) => !torus_chord_holds(t, x, y, q.chord),
                                _ => false,
                            }
                    })))
    };
    // (a long triangle on a cone, between rulings at its two ends, can sag in its middle past
    // its sides too)
    let both_ways = doubly_curved(surface) || matches!(surface, Surface::Procedural(_) | Surface::Cone(_));
    // on a sphere the triangle's deepest point is the centre's foot on its plane when that lies
    // in it (else an edge's middle, tested with the edges): the centroid is tested first, and
    // the foot only where the centroid's sag is within what the foot can add to it (with the
    // edges within the chord, at most 8rc/(9 d) for d the foot's distance from the centre, d at
    // least √(r² − 8rc/3): research/tessellation-proof)
    let foot_margin = match surface {
        Surface::Sphere(sp) if 8.0 * q.chord < 3.0 * sp.radius => {
            let r = sp.radius;
            8.0 * r * q.chord / (9.0 * (r * r - 8.0 * r * q.chord / 3.0).sqrt())
        }
        _ => f64::INFINITY,
    };
    let triangle = |v: [u32; 3], p: [Point2; 3]| {
        let m = Point2::new((p[0].x + p[1].x + p[2].x) / 3.0, (p[0].y + p[1].y + p[2].y) / 3.0);
        let grid = v.iter().all(|&k| k >= nb);
        !(grid && (0..3).all(|k| within(p[k], p[(k + 1) % 3]))) && !bounded3(p, grid) && {
            let x = [0, 1, 2].map(|k| pos(v[k], p[k]));
            let g = x[0].lerp(x[1], 0.5).lerp(x[2], 1.0 / 3.0);
            off(g, m)
                || match surface {
                    Surface::Sphere(sp) => {
                        let sag = surface.implicit(g).abs();
                        sag + foot_margin > q.chord && foot_sags(sp.frame.origin, sp.radius, x, q.chord)
                    }
                    // on a cone the distance is convex over the triangle's plane with no zero of
                    // its slope inside but along a line to its edges: its deepest point is on an
                    // edge (judged with the edges) or where the axis crosses it
                    Surface::Cone(c) => cone_axis_sags(c, x, q.chord),
                    Surface::Torus(t) => !torus_triangle_holds(t, x, q.chord),
                    _ => false,
                }
        }
    };
    // what the chord asks of an ideal mesh, asked only of a refinement past the budget its
    // boundary gives it: triangles of side √(8 r c) where the larger curvature is 1/r (sagging
    // c each), so ∫κ dA / (2√3 c) of them and half as many points, the integral on a grid of
    // the layout's box (a cell counted where its middle is in the face)
    let need = || {
        let (mut lo, mut hi) = (scaled[0], scaled[0]);
        for p in &scaled {
            (lo.x, lo.y, hi.x, hi.y) = (lo.x.min(p.x), lo.y.min(p.y), hi.x.max(p.x), hi.y.max(p.y));
        }
        let polys: Vec<Vec<Point2>> = loops
            .iter()
            .map(|l| l.iter().map(|&i| scaled[i as usize]).collect())
            .collect();
        const G: usize = 64;
        let (hx, hy) = ((hi.x - lo.x) / G as f64, (hi.y - lo.y) / G as f64);
        let mut curving = 0.0;
        for i in 0..G {
            for j in 0..G {
                let p = Point2::new(lo.x + (i as f64 + 0.5) * hx, lo.y + (j as f64 + 0.5) * hy);
                let mut inside = false;
                for poly in &polys {
                    inside ^= forge_math::predicates::inside_polygon(p, poly);
                }
                if !inside {
                    continue;
                }
                let uv = layout.uv(p);
                // the cell's area in (u, v): the layout's own stretch there
                let (du, dv) = (
                    layout.uv(Point2::new(p.x + hx, p.y)) - uv,
                    layout.uv(Point2::new(p.x, p.y + hy)) - uv,
                );
                let cell = (du.x * dv.y - du.y * dv.x).abs();
                let d = surface.deriv(uv);
                let n = d.du.cross(d.dv);
                let a = n.length();
                if a.is_nan() || a <= 0.0 {
                    continue;
                }
                let n = n * (1.0 / a);
                let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
                let (l, m, nn) = (n.dot(d.duu), n.dot(d.duv), n.dot(d.dvv));
                let det = e * g - f * f;
                if det.is_nan() || det <= 0.0 {
                    continue;
                }
                let k = (l * nn - m * m) / det;
                let h = (e * nn - 2.0 * f * m + g * l) / (2.0 * det);
                let kappa = h.abs() + (h * h - k).max(0.0).sqrt();
                if kappa.is_finite() {
                    curving += kappa * a * cell;
                }
            }
        }
        let triangles = curving / (2.0 * 3f64.sqrt() * q.chord);
        Some(crate::cdt::Need {
            chord: q.chord,
            points: (0.5 * triangles).min(u64::MAX as f64) as u64 + scaled.len() as u64,
        })
    };
    let strays = Tests(edge, triangle, both_ways, need);
    let zipped = match &loops[..] {
        [lp] if interior.is_empty() && twice.is_empty() => zip(&scaled, lp, &strays),
        _ => None,
    };
    // (a file's face whose refinement does not settle is drawn as far as it got, and said to be)
    let mut settled = true;
    let mut far = |tangled: bool| -> Result<Triangulation, CdtError> {
        let (t, s) = triangulate_as_far(&scaled, &loops, &interior, Some(&strays), tangled)?;
        settled &= s;
        Ok(t)
    };
    let tri = match zipped.map_or_else(
        || {
            if mend {
                far(false)
            } else {
                triangulate_within(&scaled, &loops, &interior, Some(&strays), false)
            }
        },
        |triangles| {
            Ok(Triangulation {
                points: Vec::new(),
                triangles,
            })
        },
    ) {
        Ok(t) => t,
        // a file's edges that meet only within their tolerance can cross in the plane of the
        // triangulation: drawn as they are, the mesh parts from the next face by that little
        Err(_) if mend => far(true)?,
        Err(e) => {
            if let Some(path) = std::env::var_os("FORGE_CDT_DUMP") {
                let pts: Vec<String> = scaled.iter().map(|p| format!("{:e},{:e}", p.x, p.y)).collect();
                let ls: Vec<String> = loops
                    .iter()
                    .map(|l| l.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","))
                    .collect();
                let ins: Vec<String> = interior.iter().map(|p| format!("{:e},{:e}", p.x, p.y)).collect();
                let text = format!("{}\n{}\n{}\n", pts.join(" "), ls.join(" "), ins.join(" "));
                let _ = std::fs::write(path, text);
            }
            return Err(e);
        }
    };
    if !settled {
        mesh.unsettled.push(f.0);
    }
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

    mesh.triangles.reserve(tri.triangles.len());
    mesh.face_of.reserve(tri.triangles.len());
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

/// A loop without its runs there and back (`x y x` is `x`), round its end too.
fn drop_spikes(l: &mut Vec<u32>) {
    let mut out: Vec<u32> = Vec::with_capacity(l.len());
    for &i in l.iter() {
        if out.len() >= 2 && out[out.len() - 2] == i {
            out.pop();
        } else {
            out.push(i);
        }
    }
    loop {
        let n = out.len();
        if n >= 3 && out[n - 1] == out[1] {
            // x | y x ...: the first is a spike's tip
            out.pop();
            out.remove(0);
        } else if n >= 3 && out[n - 2] == out[0] {
            // ... x y | x: the last is
            out.truncate(n - 2);
        } else if n >= 2 && out[n - 1] == out[0] {
            out.pop();
        } else {
            break;
        }
    }
    *l = out;
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
    /// The point the triangulation sees. A coordinate below `2^-200` in magnitude is noise for
    /// zero (far below LINEAR): it becomes exactly 0, so the exact predicates' products of such
    /// noise cannot underflow (forge_math::predicates' precondition, `product_exact`).
    fn put(self, uv: Point2) -> Point2 {
        let p = self.place(uv);
        Point2::new(snap_zero(p.x), snap_zero(p.y))
    }

    fn place(self, uv: Point2) -> Point2 {
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

/// `x`, or exactly 0 when `|x| < 2^-200`.
#[inline]
fn snap_zero(x: f64) -> f64 {
    const NOISE: f64 = f64::from_bits((1023 - 200) << 52);
    if x.abs() < NOISE { 0.0 } else { x }
}

/// Whether `x` lies further than `c` from the surface about `uv`, where it is expected near.
/// The surface's point at `uv` (`y`) bounds the distance from above. Where that is too far but the gap
/// along the normal there is not (the point lies along from the nearest, as by a pole or a seam),
/// Gauss-Newton looks for a nearer one, each step kept only if it comes closer.
fn strays_from(s: &Surface, x: forge_math::Point3, y: forge_math::Point3, uv: Point2, c: f64) -> bool {
    if y.distance(x) <= c {
        return false;
    }
    let (mut uv, mut d) = (uv, s.deriv1(uv));
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
        let dn = s.deriv1(next);
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

/// A refinement's tests: edges, triangles, whether triangles are looked at, and what its chord
/// asks for.
struct Tests<E, T, N>(E, T, bool, N);

impl<
    E: Fn(u32, u32, Point2, Point2) -> bool,
    T: Fn([u32; 3], [Point2; 3]) -> bool,
    N: Fn() -> Option<crate::cdt::Need>,
> Refine for Tests<E, T, N>
{
    fn edge(&self, a: u32, b: u32, pa: Point2, pb: Point2) -> bool {
        (self.0)(a, b, pa, pb)
    }
    fn triangle(&self, v: [u32; 3], p: [Point2; 3]) -> bool {
        (self.1)(v, p)
    }
    fn triangles(&self) -> bool {
        self.2
    }
    #[cold]
    fn need(&self) -> Option<crate::cdt::Need> {
        (self.3)()
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
    let ring = bottom.len() + right.len() + top.len() + left.len();
    let mut pts: Vec<(Point2, forge_math::Point3, u32)> = Vec::with_capacity(ring + mu * nv);
    let mut outer: Vec<(f64, usize)> = Vec::with_capacity(ring);
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
            Surface::Procedural(pr) => (pr.uv_scale().0, pr.mesh_radius()),
        };
        if r <= forge_math::tol::LINEAR {
            return 1;
        }
        let step = if doubly_curved(surface) {
            arc_step(r, q) / core::f64::consts::SQRT_2
        } else {
            arc_step(r, q)
        };
        // two at least: one chord across a sliver of a curved face can be the plane's beside it
        ((u1 - u0) * len / (r * step)).ceil().max(2.0) as usize
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
    // (each at its deepest point, as the refinement judges it: on a cone, its edges' deepest
    // points and where the axis crosses it; on a sphere, its edges' middles and the centre's foot;
    // on a torus, its edges by the bound and its inside where F's slope is zero; a cylinder's rows
    // are an arc's chords apart, its triangles' deepest points their edges' middles, each within
    // the chord)
    let strays = match surface {
        Surface::Cone(c) => {
            let off = |x: forge_math::Point3| surface.implicit(x).abs() > q.chord;
            tris.iter().any(|t| {
                let p = t.map(|i| mesh.positions[i as usize]);
                off(p[0].lerp(p[1], 0.5).lerp(p[2], 1.0 / 3.0))
                    || (0..3).any(|k| {
                        let m = p[k].lerp(p[(k + 1) % 3], 0.5);
                        let sag = surface.implicit(m).abs();
                        sag > q.chord
                            || (sag > 0.5 * q.chord && cone_chord_sags(c, p[k], p[(k + 1) % 3], q.chord))
                    })
                    || cone_axis_sags(c, p, q.chord)
            })
        }
        Surface::Sphere(sp) => {
            let margin = if 8.0 * q.chord < 3.0 * sp.radius {
                let r = sp.radius;
                8.0 * r * q.chord / (9.0 * (r * r - 8.0 * r * q.chord / 3.0).sqrt())
            } else {
                f64::INFINITY
            };
            let sag = |x: forge_math::Point3| surface.implicit(x).abs();
            tris.iter().any(|t| {
                let p = t.map(|i| mesh.positions[i as usize]);
                (0..3).any(|k| sag(p[k].lerp(p[(k + 1) % 3], 0.5)) > q.chord) || {
                    let g = sag(p[0].lerp(p[1], 0.5).lerp(p[2], 1.0 / 3.0));
                    g > q.chord || (g + margin > q.chord && foot_sags(sp.frame.origin, sp.radius, p, q.chord))
                }
            })
        }
        Surface::Torus(tor) => tris.iter().any(|t| {
            let p = t.map(|i| mesh.positions[i as usize]);
            let sag = |x: forge_math::Point3| surface.implicit(x).abs();
            (0..3).any(|k| {
                sag(p[k].lerp(p[(k + 1) % 3], 0.5)) > q.chord
                    || !torus_chord_holds(tor, p[k], p[(k + 1) % 3], q.chord)
            }) || sag(p[0].lerp(p[1], 0.5).lerp(p[2], 1.0 / 3.0)) > q.chord
                || !torus_triangle_holds(tor, p, q.chord)
        }),
        _ => false,
    };
    if strays {
        mesh.positions.truncate(added);
        return None;
    }
    Some(tris)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Point3, Vec3};
    use forge_ops::{Profile, extrude};

    /// A parabola's and a hyperbola's chords stray from them by no more than the chord and turn by
    /// no more than the angle, at every tolerance, near the vertex and far out (where a fixed count
    /// of chords strayed by any amount).
    #[test]
    fn conic_chords_hold_the_tolerance() {
        use forge_geom::curve::{Hyperbola, Parabola};
        use forge_math::Interval;
        let curves = [
            Curve::Parabola(Parabola {
                frame: Frame::WORLD,
                focal: 0.5,
            }),
            Curve::Parabola(Parabola {
                frame: Frame::WORLD,
                focal: 40.0,
            }),
            Curve::Hyperbola(Hyperbola {
                frame: Frame::WORLD,
                a: 3.0,
                b: 0.5,
            }),
            Curve::Hyperbola(Hyperbola {
                frame: Frame::WORLD,
                a: 0.2,
                b: 7.0,
            }),
        ];
        for c in &curves {
            for range in [
                Interval::new(-1.0, 1.0),
                Interval::new(-30.0, 80.0),
                Interval::new(2.0, 2.5),
            ] {
                for (chord, max_angle) in [(0.01, 0.26), (1e-4, 0.26), (0.5, 1.0)] {
                    let q = Quality { chord, max_angle };
                    let n = edge_segments(c, range, &q, false);
                    let mut worst: f64 = 0.0;
                    for i in 0..n {
                        let at = |k: usize| range.lo + range.width() * k as f64 / n as f64;
                        let (a, b) = (c.eval(at(i)), c.eval(at(i + 1)));
                        let (u, w) = (c.deriv(at(i)).d1, c.deriv(at(i + 1)).d1);
                        let turn = dmath::atan2(u.cross(w).length(), u.dot(w));
                        assert!(turn <= max_angle * (1.0 + 1e-9), "{c:?} {range:?} turns {turn}");
                        for j in 1..16 {
                            let t = at(i) + (at(i + 1) - at(i)) * j as f64 / 16.0;
                            let p = c.eval(t);
                            let w = b - a;
                            let s = ((p - a).dot(w) / w.dot(w)).clamp(0.0, 1.0);
                            worst = worst.max((a + w * s).distance(p));
                        }
                    }
                    assert!(
                        worst <= chord * (1.0 + 1e-9),
                        "{c:?} {range:?} at {chord}: {worst} in {n}"
                    );
                }
            }
        }
    }

    /// A circle's chords sag by the tolerance at most, exactly: at any chord over radius (where
    /// the old floor of 1e-4 rad let a huge arc's chords sag past it) and at half the tolerance
    /// on a face curved both ways (where n·√2 gave up to 1.0007 of the half).
    /// On a sphere the triangle's deepest point is the centre's foot on its plane: a triangle
    /// whose edges' middles and centroid sag within the chord, its foot past it, asks to be
    /// split; one whose foot lies outside it does not (its deepest points are its edges' middles).
    #[test]
    fn a_spheres_triangle_is_judged_at_its_deepest_point() {
        use forge_math::{Point3, Vec3};
        let (o, r) = (Point3::new(0.0, 0.0, 0.0), 10.0);
        let at = |theta: f64, phi: f64| {
            Point3::new(
                r * forge_math::dmath::sin(theta) * forge_math::dmath::cos(phi),
                r * forge_math::dmath::sin(theta) * forge_math::dmath::sin(phi),
                r * forge_math::dmath::cos(theta),
            )
        };
        // an acute triangle round the pole, its foot inside, sagging about 0.0225
        let tri = [at(0.0671, 0.0), at(0.0671, 2.1), at(0.0671, 4.2)];
        let sag = |p: Point3| r - p.distance(o);
        let mid = |a: Point3, b: Point3| a.lerp(b, 0.5);
        let g = tri[0].lerp(tri[1], 0.5).lerp(tri[2], 1.0 / 3.0);
        let deepest = r - (tri[0] - o).dot(Vec3::new(0.0, 0.0, 1.0));
        let tested = sag(g).max(sag(mid(tri[0], tri[1]))).max(sag(mid(tri[1], tri[2])));
        // a chord between what the tests see and the foot's sag
        let chord = 0.5 * (tested + deepest);
        assert!(tested < chord && chord < deepest, "{tested} {chord} {deepest}");
        assert!(foot_sags(o, r, tri, chord));
        assert!(!foot_sags(o, r, tri, deepest * 1.0001));
        // an obtuse triangle beside the pole: the foot outside, no say
        let obtuse = [at(0.05, 0.0), at(0.05, 0.3), at(0.05, 0.6)];
        assert!(!foot_sags(o, r, obtuse, 1e-9));
    }

    /// On a cone a chord's deepest point is not its middle: a chord whose middle sags within the
    /// chord, a point of it past, asks to be split; past its deepest point, it does not.
    #[test]
    fn a_cones_chord_is_judged_at_its_deepest_point() {
        use forge_math::Point3;
        let cone = forge_geom::Cone::new(Frame::WORLD, 10.0, 0.5).expect("a cone");
        let s = Surface::Cone(cone);
        // two points of it, far apart in height and a little apart in angle
        let at = |z: f64, phi: f64| {
            let r = cone.radius_at(z);
            Point3::new(
                r * forge_math::dmath::cos(phi),
                r * forge_math::dmath::sin(phi),
                z,
            )
        };
        let (x, y) = (at(0.0, 0.0), at(30.0, 0.35));
        assert!(s.implicit(x).abs() < 1e-12 && s.implicit(y).abs() < 1e-12);
        let mid = s.implicit(x.lerp(y, 0.5)).abs();
        let deepest = (0..=20000)
            .map(|i| s.implicit(x.lerp(y, i as f64 / 20000.0)).abs())
            .fold(0.0, f64::max);
        assert!(deepest > 1.05 * mid, "{mid} {deepest}");
        let between = 0.5 * (mid + deepest);
        assert!(cone_chord_sags(&cone, x, y, between));
        assert!(!cone_chord_sags(&cone, x, y, deepest * 1.0001));
    }

    /// On a torus, what the rule accepts lies within the chord: random chords and triangles with
    /// their corners on it, each accepted one sampled densely (and the rule accepts most of what
    /// the middle's and centroid's tests do: it is a bound, not a wall).
    #[test]
    fn a_toruss_accepted_chords_and_triangles_hold_the_chord() {
        use forge_math::Point3;
        let t = forge_geom::Torus::new(Frame::WORLD, 20.0, 5.0).expect("a torus");
        let s = Surface::Torus(t);
        let at = |u: f64, v: f64| {
            let (su, cu) = forge_math::dmath::sin_cos(u);
            let (sv, cv) = forge_math::dmath::sin_cos(v);
            Point3::new((20.0 + 5.0 * cv) * cu, (20.0 + 5.0 * cv) * su, 5.0 * sv)
        };
        let mut r = Rng::new(7);
        let chord = 0.05;
        let (mut chords, mut tris) = ((0, 0), (0, 0));
        for _ in 0..4000 {
            let (u, v) = (
                r.range(0.0, forge_math::dmath::TAU),
                r.range(0.0, forge_math::dmath::TAU),
            );
            let x: [Point3; 3] = core::array::from_fn(|_| at(u + r.range(-0.1, 0.1), v + r.range(-0.2, 0.2)));
            let deepest = |n: usize, f: &dyn Fn(f64, f64) -> Point3| {
                let mut d = 0.0f64;
                for i in 0..=n {
                    for j in 0..=(n - i) {
                        d = d.max(s.implicit(f(i as f64 / n as f64, j as f64 / n as f64)).abs());
                    }
                }
                d
            };
            let mid = s.implicit(x[0].lerp(x[1], 0.5)).abs();
            if mid <= chord {
                chords.1 += 1;
                if torus_chord_holds(&t, x[0], x[1], chord) {
                    chords.0 += 1;
                    let d = deepest(400, &|a, _| x[0].lerp(x[1], a));
                    assert!(d <= chord * (1.0 + 1e-9), "a chord accepted at {d}");
                }
            }
            let mids = (0..3).all(|k| s.implicit(x[k].lerp(x[(k + 1) % 3], 0.5)).abs() <= chord)
                && s.implicit(x[0].lerp(x[1], 0.5).lerp(x[2], 1.0 / 3.0)).abs() <= chord;
            if mids && (0..3).all(|k| torus_chord_holds(&t, x[k], x[(k + 1) % 3], chord)) {
                tris.1 += 1;
                if torus_triangle_holds(&t, x, chord) {
                    tris.0 += 1;
                    let d = deepest(60, &|a, b| x[0] + (x[1] - x[0]) * a + (x[2] - x[0]) * b);
                    assert!(d <= chord * (1.0 + 1e-6), "a triangle accepted at {d}");
                }
            }
        }
        assert!(chords.0 * 10 >= chords.1 * 9, "{chords:?}");
        assert!(tris.1 > 100, "{tris:?}");
    }

    /// The torus's chord bound certifies no chord that strays: one through the axis (its middle
    /// on the torus, R = 2r, the diameter from the outer equator to the inner one opposite), and
    /// random chords on tori near their axis and with their shadows on a line through it.
    #[test]
    fn the_toruss_chord_bound_certifies_no_stray() {
        use forge_math::Point3;
        let deepest = |s: &Surface, x: Point3, y: Point3| {
            (0..=2000)
                .map(|i| s.implicit(x.lerp(y, i as f64 / 2000.0)).abs())
                .fold(0.0, f64::max)
        };
        let t = forge_geom::Torus::new(Frame::WORLD, 10.0, 5.0).expect("a torus");
        let s = Surface::Torus(t);
        let (x, y) = (Point3::new(15.0, 0.0, 0.0), Point3::new(-5.0, 0.0, 0.0));
        assert!(s.implicit(x.lerp(y, 0.5)).abs() < 1e-12);
        assert!(
            !torus_chord_holds(&t, x, y, 0.05),
            "a chord through the axis certified"
        );
        let mut r = Rng::new(3);
        for (major, minor) in [(10.0, 5.0), (5.02, 5.0), (5.2, 5.0), (30.0, 5.0)] {
            let Some(t) = forge_geom::Torus::new(Frame::WORLD, major, minor) else {
                continue;
            };
            let s = Surface::Torus(t);
            let at = |u: f64, v: f64| {
                let (su, cu) = forge_math::dmath::sin_cos(u);
                let (sv, cv) = forge_math::dmath::sin_cos(v);
                Point3::new((major + minor * cv) * cu, (major + minor * cv) * su, minor * sv)
            };
            for k in 0..20000 {
                let (u, v) = (
                    r.range(0.0, forge_math::dmath::TAU),
                    r.range(0.0, forge_math::dmath::TAU),
                );
                // half of them with both ends at one u: shadows on a line through the axis
                let du = if k % 2 == 0 { 0.0 } else { r.range(-0.3, 0.3) };
                let (x, y) = (at(u, v), at(u + du, v + r.range(-1.0, 1.0)));
                for chord in [0.05, 0.5] {
                    if s.implicit(x.lerp(y, 0.5)).abs() <= chord && torus_chord_holds(&t, x, y, chord) {
                        let d = deepest(&s, x, y);
                        assert!(
                            d <= chord * (1.0 + 1e-6),
                            "R {major} r {minor}: certified at {d} > {chord}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn arc_chords_hold_the_tolerance_exactly() {
        use forge_geom::curve::Circle;
        use forge_math::Interval;
        for (r, chord) in [(1.0, 0.01), (1e6, 1e-6), (1e9, 1e-6), (0.5, 0.25), (40.0, 1e-3)] {
            for max_angle in [dmath::PI / 12.0, 1.0] {
                let q = Quality { chord, max_angle };
                let step = arc_step(r, &q);
                // the sag of one step, computed stably
                let sag = 2.0 * r * dmath::sin(0.25 * step).powi(2);
                assert!(sag <= chord * (1.0 + 1e-12), "{r} {chord}: {sag}");
                assert!(step <= max_angle);
                for fine in [false, true] {
                    let c = Curve::Circle(Circle {
                        frame: Frame::WORLD,
                        radius: r,
                    });
                    let width = 1e-3f64.min(dmath::PI);
                    let n = edge_segments(&c, Interval::new(0.0, width), &q, fine);
                    let sag = 2.0 * r * dmath::sin(0.25 * width / n as f64).powi(2);
                    let allowed = if fine { 0.5 * chord } else { chord };
                    assert!(sag <= allowed * (1.0 + 1e-12), "{r} {chord} {fine}: {sag}");
                }
            }
        }
    }

    /// On a pole in all but rounding, the normal is noise: it must not ask for a million steps.
    #[test]
    fn a_noisy_pole_asks_for_no_steps() {
        let d = forge_geom::surface::SurfDeriv {
            p: Point3::ORIGIN,
            du: Vec3::new(1e-16, 0.0, 0.0),
            dv: Vec3::new(0.0, 1.0, 0.0),
            duu: Vec3::new(0.0, 0.0, 1e-16),
            duv: Vec3::new(0.0, 0.0, 1e-16),
            dvv: Vec3::new(0.0, 0.0, 1.0),
        };
        assert_eq!(line_curvatures(&d), (0.0, 0.0));
        // a tight bend away from any pole still counts
        let d = forge_geom::surface::SurfDeriv {
            du: Vec3::new(1e-3, 0.0, 0.0),
            duu: Vec3::new(0.0, 0.0, 1e-3),
            ..d
        };
        assert!(line_curvatures(&d).0 > 100.0);
    }

    /// Top corners swapped: a boundary that crosses itself. An error, unless the body is from a file;
    /// then drawn as it is, what it parts being the bow tie's two triangles.
    #[test]
    fn a_crossing_boundary_fails_or_is_drawn_as_it_is() {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::ORIGIN, 4.0, 2.0, 0)],
        };
        let mut b = extrude(&p, 1.0, 1).unwrap();
        let top: Vec<usize> = (0..b.vertices.len())
            .filter(|&v| b.vertices[v].point.z > 0.5)
            .collect();
        let mut at: Vec<Point3> = top.iter().map(|&v| b.vertices[v].point).collect();
        at.sort_by(|a, c| (a.x, a.y).partial_cmp(&(c.x, c.y)).unwrap());
        // the two on x = high change places
        let (p0, p1) = (at[2], at[3]);
        for &v in &top {
            let q = b.vertices[v].point;
            if q == p0 {
                b.vertices[v].point = p1;
            } else if q == p1 {
                b.vertices[v].point = p0;
            }
        }
        let q = Quality::default();
        assert!(tessellate(&b, &q).is_err());
        // on a body made here a crossing stays an error
        assert!(tessellate_tolerant(&b, &q).is_err());
        b.from_file = true;
        assert!(tessellate(&b, &q).is_err());
        let m = tessellate_tolerant(&b, &q).unwrap();
        let top_area: f64 = m
            .triangles
            .iter()
            .filter(|t| t.iter().all(|&i| m.positions[i as usize].z > 0.5))
            .map(|&t| 0.5 * m.triangle_normal(t).length())
            .sum();
        assert!(top_area > 0.0 && top_area < 8.0, "{top_area}");
    }

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
    fn each_edge_is_a_chain_of_mesh_segments() {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)],
        };
        let b = extrude(&p, 5.0, 1).unwrap();
        let m = tessellate(&b, &Quality::default()).unwrap();
        assert_eq!(m.edges.len(), b.edges.len());
        let sides: Vec<(u32, u32)> = m
            .triangles
            .iter()
            .flat_map(|&[a, b, c]| [(a, b), (b, c), (c, a)])
            .collect();
        for (e, chain) in m.edges.iter().enumerate() {
            let edge = &b.edges[e];
            let ends = (chain[0], *chain.last().unwrap());
            assert_eq!(ends, (edge.start.0, edge.end.0));
            for w in chain.windows(2) {
                assert!(sides.contains(&(w[0], w[1])) && sides.contains(&(w[1], w[0])));
            }
        }
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
