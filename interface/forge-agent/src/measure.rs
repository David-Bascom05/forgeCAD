//! Distances, angles, mass properties and wall thickness.

use crate::select::{self, Ent};
use forge_geom::Curve;
use forge_math::{Point3, Vec3, dmath};
use forge_mesh::TriMesh;
use forge_topo::Body;

pub struct Mass {
    pub volume: f64,
    pub area: f64,
    pub centroid: Point3,
    /// Second moments about the centroid (unit density): Ixx, Iyy, Izz.
    pub inertia: [f64; 3],
}

pub fn mass(m: &TriMesh) -> Mass {
    let (mut vol, mut c) = (0.0, Vec3::ZERO);
    // integrals of x^2, y^2, z^2 over the solid, from tetrahedra on the origin
    let mut sq = [0.0f64; 3];
    for t in &m.triangles {
        let [a, b, d] = t.map(|i| m.positions[i as usize] - Point3::ORIGIN);
        let v = a.dot(b.cross(d)) / 6.0;
        vol += v;
        c += (a + b + d) * (v / 4.0);
        for (k, s) in sq.iter_mut().enumerate() {
            let (x, y, z) = (a[k], b[k], d[k]);
            *s += v / 10.0 * (x * x + y * y + z * z + x * y + y * z + z * x);
        }
    }
    let centroid = Point3::ORIGIN + c * (1.0 / vol);
    let g = centroid - Point3::ORIGIN;
    let about = |k: usize| sq[k] - vol * g[k] * g[k];
    let (xx, yy, zz) = (about(0), about(1), about(2));
    Mass {
        volume: vol,
        area: m.area(),
        centroid,
        inertia: [yy + zz, xx + zz, xx + yy],
    }
}

/// The thinnest wall: from each triangle, straight in against its normal to the next surface
/// the ray leaves the solid through. The thickness and where it is.
pub fn wall(body: &Body, m: &TriMesh) -> Option<(f64, Point3)> {
    // faces meeting at an edge make a wedge there, not a wall
    let mut touching = std::collections::HashSet::new();
    for e in body.edge_ids() {
        let fs = select::faces_of_edge(body, e);
        for &a in &fs {
            for &b in &fs {
                touching.insert((a.0, b.0));
            }
        }
    }
    let tri = |t: &[u32; 3]| t.map(|i| m.positions[i as usize]);
    let normals: Vec<Option<Vec3>> = m
        .triangles
        .iter()
        .map(|t| m.triangle_normal(*t).normalize().map(|n| n.get()))
        .collect();
    let mut best: Option<(f64, Point3)> = None;
    for (i, t) in m.triangles.iter().enumerate() {
        let Some(n) = normals[i] else { continue };
        let [a, b, c] = tri(t);
        let o = a + ((b - a) + (c - a)) * (1.0 / 3.0);
        let d = -n;
        for (j, u) in m.triangles.iter().enumerate() {
            // only the far side: a face the ray leaves through looks along it
            if j == i
                || normals[j].is_none_or(|nj| nj.dot(d) <= 1e-9)
                || touching.contains(&(m.face_of[i], m.face_of[j]))
            {
                continue;
            }
            if let Some(s) = hit(o, d, tri(u))
                && s > 1e-7
                && best.is_none_or(|b| s < b.0)
            {
                best = Some((s, o + d * (s / 2.0)));
            }
        }
    }
    best
}

/// Where the ray o + s d meets the triangle, if it does.
fn hit(o: Point3, d: Vec3, [a, b, c]: [Point3; 3]) -> Option<f64> {
    let (e1, e2) = (b - a, c - a);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-14 {
        return None;
    }
    let tv = o - a;
    let u = tv.dot(p) / det;
    let q = tv.cross(e1);
    let v = d.dot(q) / det;
    if u < -1e-9 || v < -1e-9 || u + v > 1.0 + 1e-9 {
        return None;
    }
    Some(e2.dot(q) / det)
}

/// Points over an entity: a face's mesh vertices, an edge's samples, a vertex.
fn points(body: &Body, mesh: &TriMesh, e: Ent) -> Vec<Point3> {
    match e {
        Ent::Face(f) => {
            let mut p: Vec<Point3> = mesh
                .triangles
                .iter()
                .zip(&mesh.face_of)
                .filter(|x| *x.1 == f.0)
                .flat_map(|x| x.0.map(|i| mesh.positions[i as usize]))
                .collect();
            p.extend(select::samples(body, e));
            p
        }
        _ => select::samples(body, e),
    }
}

/// A direction that stands for the entity: a plane's normal or a line's direction.
fn direction(body: &Body, e: Ent) -> Option<(Vec3, bool)> {
    match e {
        Ent::Face(f) => select::plane_normal(body, f).map(|n| (n, true)),
        Ent::Edge(x) => match &body.edge(x).curve {
            Curve::Line(l) => Some((l.dir.get(), false)),
            _ => None,
        },
        Ent::Vert(_) => None,
    }
}

/// What can be said about one entity, as (name, value) pairs.
pub fn one(body: &Body, mesh: &TriMesh, e: Ent) -> Vec<(&'static str, f64)> {
    let mut out = Vec::new();
    match e {
        Ent::Face(f) => {
            let area: f64 = mesh
                .triangles
                .iter()
                .zip(&mesh.face_of)
                .filter(|x| *x.1 == f.0)
                .map(|x| {
                    let [a, b, c] = x.0.map(|i| mesh.positions[i as usize]);
                    (b - a).cross(c - a).length() / 2.0
                })
                .sum();
            out.push(("area", area));
        }
        Ent::Edge(x) => out.push(("length", select::length(body, x))),
        Ent::Vert(_) => {}
    }
    if let Some(r) = select::radius(body, e) {
        out.push(("radius", r));
    }
    // a round thing's centre is on its axis, not in the middle of a half face
    let c = select::axis(body, e).map_or_else(|| select::centre(body, e), |a| a.0);
    out.extend([("x", c.x), ("y", c.y), ("z", c.z)]);
    out
}

/// The exact gap from a planar face to a round thing clear of it and standing square to it: a cylinder lying
/// along it, a circle parallel to it, a sphere.
fn plane_round(body: &Body, a: Ent, b: Ent) -> Option<f64> {
    let Ent::Face(f) = a else { return None };
    let n = select::plane_normal(body, f)?;
    let (p, dir) = select::axis(body, b)?;
    let off = (p - select::centre(body, a)).dot(n).abs();
    let r = select::radius(body, b).unwrap_or(0.0);
    // a plane cutting through the round thing: the faces' closest points are elsewhere
    if off < r - 1e-9 {
        return None;
    }
    match (b, dir) {
        (Ent::Edge(_), Some(u)) if u.cross(n).length() < 1e-9 => Some(off),
        (Ent::Face(g), Some(u))
            if u.dot(n).abs() < 1e-9 && matches!(body.face(g).surface, forge_geom::Surface::Cylinder(_)) =>
        {
            Some(off - r)
        }
        (Ent::Face(_), None) => Some(off - r),
        _ => None,
    }
}

/// Distance between two entities (exact for parallel planes, else the closest of dense
/// samples), and the angle between them when both have a direction.
pub fn between(body: &Body, mesh: &TriMesh, a: Ent, b: Ent) -> Vec<(&'static str, f64)> {
    let mut out = Vec::new();
    let (da, db) = (direction(body, a), direction(body, b));
    let parallel_planes = match (da, db) {
        (Some((na, true)), Some((nb, true))) => na.cross(nb).length() < 1e-9,
        _ => false,
    };
    if parallel_planes {
        let n = da.expect("planar").0;
        out.push((
            "distance",
            (select::centre(body, b) - select::centre(body, a)).dot(n).abs(),
        ));
    } else if let Some(d) = plane_round(body, a, b).or_else(|| plane_round(body, b, a)) {
        out.push(("distance", d));
    } else {
        let (pa, pb) = (points(body, mesh, a), points(body, mesh, b));
        let mut d = f64::INFINITY;
        for p in &pa {
            for q in &pb {
                d = d.min(p.distance(*q));
            }
        }
        out.push(("distance", d));
    }
    // two holes or bosses: how far apart their axes are, or their centres
    match (select::axis(body, a), select::axis(body, b)) {
        (Some((p, Some(u))), Some((q, Some(w)))) if u.cross(w).length() < 1e-9 => {
            let d = q - p;
            out.push(("axes", (d - u * d.dot(u)).length()));
        }
        (Some((p, None)), Some((q, None))) => out.push(("centres", p.distance(q))),
        _ => {}
    }
    if let (Some((u, pu)), Some((w, pw))) = (da, db) {
        let c = u.dot(w).clamp(-1.0, 1.0);
        let deg = |x: f64| x * 180.0 / dmath::PI;
        let ang = if pu == pw {
            // two planes: the angle between them; two lines: between their directions
            let a = deg(dmath::acos(c));
            a.min(180.0 - a)
        } else {
            90.0 - deg(dmath::acos(c.abs()))
        };
        out.push(("angle", ang));
    }
    out
}

/// One picked entity of a placed body: the body, its mesh, where the body's space lies in the
/// world (an assembly occurrence's pose), and the entity.
#[derive(Clone, Copy)]
pub struct Pick<'a> {
    pub body: &'a Body,
    pub mesh: &'a TriMesh,
    pub at: forge_math::Frame,
    pub ent: Ent,
}

/// What a measurement reads, as Inventor's Measure panel lists it. Lengths mm, angles degrees,
/// areas mm^2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quantity {
    Distance,
    Dx,
    Dy,
    Dz,
    /// Between the centres of two circles, spheres or arcs; or two points.
    CentreDistance,
    /// Between two parallel axes.
    AxisDistance,
    Angle,
    Length,
    /// The closed loop of edges an edge is in, round its face.
    LoopLength,
    Radius,
    Diameter,
    Area,
    Perimeter,
    X,
    Y,
    Z,
    /// The farthest two points of two entities lie apart, where Inventor gives it: points,
    /// lines, arcs and circles in one plane.
    MaxDistance,
    /// A body's: its volume, its mass (from its material's density), its box's three sides.
    Volume,
    Mass,
    SizeX,
    SizeY,
    SizeZ,
}

impl Quantity {
    pub fn label(self) -> &'static str {
        match self {
            Quantity::Distance => "Minimum distance",
            Quantity::Dx => "Delta X",
            Quantity::Dy => "Delta Y",
            Quantity::Dz => "Delta Z",
            Quantity::CentreDistance => "Center to center",
            Quantity::AxisDistance => "Axis to axis",
            Quantity::Angle => "Angle",
            Quantity::Length => "Length",
            Quantity::LoopLength => "Loop length",
            Quantity::Radius => "Radius",
            Quantity::Diameter => "Diameter",
            Quantity::Area => "Area",
            Quantity::Perimeter => "Perimeter",
            Quantity::X => "X",
            Quantity::Y => "Y",
            Quantity::Z => "Z",
            Quantity::MaxDistance => "Maximum distance",
            Quantity::Volume => "Volume",
            Quantity::Mass => "Mass",
            Quantity::SizeX => "Box X",
            Quantity::SizeY => "Box Y",
            Quantity::SizeZ => "Box Z",
        }
    }

    /// What it measures: a length (1), an area (2), a volume (3), an angle (0), a mass (4).
    pub fn power(self) -> u8 {
        match self {
            Quantity::Angle => 0,
            Quantity::Area => 2,
            Quantity::Volume => 3,
            Quantity::Mass => 4,
            _ => 1,
        }
    }
}

/// What the view draws for a measurement, in world coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    /// A dimension line between two points, labelled with this quantity.
    Line(Point3, Point3, Quantity),
    Point(Point3),
    /// A circle's centre, its axis and radius.
    Circle(Point3, Vec3, f64),
    /// An angle: its vertex, the two ways its arms run, and the angle (degrees) the arc spans.
    Angle(Point3, Vec3, Vec3, f64),
    /// A face tinted, by its triangles, with its area and perimeter labelled at this point.
    Face(Vec<[Point3; 3]>, Point3),
    /// A curve or a loop drawn over, labelled with this quantity.
    Curve(Vec<Point3>, Quantity),
    /// A body's box, corner to corner.
    Box(Point3, Point3),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reading {
    pub values: Vec<(Quantity, f64)>,
    pub shown: Vec<Shown>,
}

impl Reading {
    pub fn get(&self, q: Quantity) -> Option<f64> {
        self.values.iter().find(|v| v.0 == q).map(|v| v.1)
    }
}

/// Exact geometry an entity stands for, in the world.
#[derive(Clone, Copy, Debug)]
enum Geom {
    Point(Point3),
    Segment(Point3, Point3),
    Circle(Point3, Vec3, f64),
    Plane(Point3, Vec3),
    Cylinder(Point3, Vec3),
    Sphere(Point3),
    /// An infinite line: a work axis, an origin axis.
    Axis(Point3, Vec3),
    Other,
}

fn geom(p: &Pick) -> Geom {
    let (b, f) = (p.body, &p.at);
    let w = |q: Point3| f.point_to_world(q);
    let v = |d: Vec3| f.vec_to_world(d);
    match p.ent {
        Ent::Vert(x) => Geom::Point(w(b.vertex(x).point)),
        Ent::Edge(x) => {
            let e = b.edge(x);
            match &e.curve {
                Curve::Line(_) => Geom::Segment(w(b.vertex(e.start).point), w(b.vertex(e.end).point)),
                Curve::Circle(c) => Geom::Circle(w(c.frame.origin), v(c.frame.z().get()), c.radius),
                _ => Geom::Other,
            }
        }
        Ent::Face(x) => match &b.face(x).surface {
            forge_geom::Surface::Plane(_) => {
                let n = select::plane_normal(b, x).expect("a plane");
                Geom::Plane(w(select::centre(b, p.ent)), v(n))
            }
            forge_geom::Surface::Cylinder(c) => Geom::Cylinder(w(c.frame.origin), v(c.frame.z().get())),
            forge_geom::Surface::Sphere(s) => Geom::Sphere(w(s.frame.origin)),
            _ => Geom::Other,
        },
    }
}

/// Points over the entity in the world: what the nearest pair starts from.
fn world_points(p: &Pick) -> Vec<Point3> {
    let mut pts = points(p.body, p.mesh, p.ent);
    if let Ent::Edge(x) = p.ent {
        pts.extend(p.body.edge_points(x, 128));
    }
    // a big face's every mesh vertex is more than the nearest pair needs to start
    let step = pts.len() / 3000 + 1;
    pts.into_iter()
        .step_by(step)
        .map(|q| p.at.point_to_world(q))
        .collect()
}

/// The entity's point nearest `q` (a world point), if the entity's exact geometry gives one.
fn nearest_on(p: &Pick, q: Point3) -> Option<Point3> {
    use forge_geom::{CurveGeom, SurfaceGeom};
    let local = p.at.point_to_local(q);
    let b = p.body;
    let found = match p.ent {
        Ent::Vert(x) => b.vertex(x).point,
        Ent::Edge(x) => {
            let e = b.edge(x);
            e.curve.eval(e.curve.closest(local, e.range))
        }
        Ent::Face(f) => {
            let s = &b.face(f).surface;
            let on = s.eval(s.project(local));
            // inside the face's trim: on (or within a hair of) its mesh
            let tol = 0.05f64.max(on.distance(local) * 1e-6);
            if face_gap(p.mesh, f.0, on) > tol {
                return None;
            }
            on
        }
    };
    Some(p.at.point_to_world(found))
}

/// How far a point is from a face's triangles.
fn face_gap(m: &TriMesh, face: u32, q: Point3) -> f64 {
    m.triangles
        .iter()
        .zip(&m.face_of)
        .filter(|x| *x.1 == face)
        .map(|x| tri_distance(q, x.0.map(|i| m.positions[i as usize])))
        .fold(f64::INFINITY, f64::min)
}

fn tri_distance(p: Point3, [a, b, c]: [Point3; 3]) -> f64 {
    let n = (b - a).cross(c - a);
    let nn = n.dot(n);
    if nn > 0.0 {
        // inside the triangle seen along its normal: the height
        let inside = [(a, b), (b, c), (c, a)]
            .iter()
            .all(|(u, v)| (*v - *u).cross(p - *u).dot(n) >= 0.0);
        if inside {
            return (p - a).dot(n).abs() / nn.sqrt();
        }
    }
    [(a, b), (b, c), (c, a)]
        .iter()
        .map(|(u, v)| segment_distance(p, *u, *v).0)
        .fold(f64::INFINITY, f64::min)
}

/// A point's distance to a segment, and the segment's nearest point.
fn segment_distance(p: Point3, a: Point3, b: Point3) -> (f64, Point3) {
    let d = b - a;
    let t = if d.dot(d) > 0.0 {
        ((p - a).dot(d) / d.dot(d)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let q = a + d * t;
    (p.distance(q), q)
}

/// The nearest points of two segments.
fn segments(a0: Point3, a1: Point3, b0: Point3, b1: Point3) -> (Point3, Point3) {
    let (u, v, w) = (a1 - a0, b1 - b0, a0 - b0);
    let (a, b, c, d, e) = (u.dot(u), u.dot(v), v.dot(v), u.dot(w), v.dot(w));
    let den = a * c - b * b;
    let mut s = if den > 1e-12 * a * c {
        ((b * e - c * d) / den).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut t = if c > 0.0 { (b * s + e) / c } else { 0.0 };
    if t < 0.0 || t > 1.0 {
        t = t.clamp(0.0, 1.0);
        s = if a > 0.0 {
            ((b * t - d) / a).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }
    (a0 + u * s, b0 + v * t)
}

/// The nearest pair of points of two picks: exact for points, segments and parallel planes, else
/// the nearest of dense samples walked down by projecting each onto the other's exact geometry.
fn nearest(a: &Pick, b: &Pick) -> (Point3, Point3) {
    match (geom(a), geom(b)) {
        (Geom::Point(p), Geom::Point(q)) => return (p, q),
        (Geom::Segment(a0, a1), Geom::Segment(b0, b1)) => return segments(a0, a1, b0, b1),
        (Geom::Point(p), Geom::Segment(b0, b1)) => return (p, segment_distance(p, b0, b1).1),
        (Geom::Segment(a0, a1), Geom::Point(q)) => return (segment_distance(q, a0, a1).1, q),
        _ => {}
    }
    let (pa, pb) = (world_points(a), world_points(b));
    let mut best = (f64::INFINITY, Point3::ORIGIN, Point3::ORIGIN);
    for p in &pa {
        for q in &pb {
            let d = p.distance(*q);
            if d < best.0 {
                best = (d, *p, *q);
            }
        }
    }
    let (mut d, mut p, mut q) = best;
    for _ in 0..40 {
        let Some(p2) = nearest_on(a, q) else { break };
        let Some(q2) = nearest_on(b, p2) else { break };
        let d2 = p2.distance(q2);
        if d2 >= d - 1e-12 {
            break;
        }
        (d, p, q) = (d2, p2, q2);
    }
    (p, q)
}

fn geom_direction(g: Geom) -> Option<(Vec3, bool)> {
    match g {
        Geom::Segment(a, b) => (b - a).normalize().map(|u| (u.get(), false)),
        Geom::Axis(_, u) => Some((u, false)),
        Geom::Plane(_, n) => Some((n, true)),
        Geom::Cylinder(_, z) => Some((z, false)),
        Geom::Circle(_, z, _) => Some((z, true)),
        _ => None,
    }
}

fn centre_of(g: Geom) -> Option<Point3> {
    match g {
        Geom::Point(p) | Geom::Circle(p, _, _) | Geom::Sphere(p) => Some(p),
        _ => None,
    }
}

/// The length of the loop an edge is in: the longest through it round a flat face it bounds
/// (a curved face's loops run along its seam), else round either face.
fn loop_length(body: &Body, e: forge_topo::EdgeId) -> Option<f64> {
    let faces = select::faces_of_edge(body, e);
    let flat: Vec<_> = faces
        .iter()
        .copied()
        .filter(|&f| select::plane_normal(body, f).is_some())
        .collect();
    let from = if flat.is_empty() { faces } else { flat };
    from.into_iter()
        .flat_map(|f| body.face(f).loops.iter().copied().collect::<Vec<_>>())
        .filter_map(|l| {
            let cs: Vec<_> = body.lp(l).coedges().map(|c| body.coedge(c).edge).collect();
            cs.contains(&e)
                .then(|| cs.iter().map(|&x| select::length(body, x)).sum::<f64>())
        })
        .max_by(f64::total_cmp)
}

/// What one pick reads.
pub fn read_one(p: &Pick) -> Reading {
    let mut r = Reading::default();
    let (b, at) = (p.body, &p.at);
    match p.ent {
        Ent::Vert(x) => {
            let q = at.point_to_world(b.vertex(x).point);
            r.values
                .extend([(Quantity::X, q.x), (Quantity::Y, q.y), (Quantity::Z, q.z)]);
            r.shown.push(Shown::Point(q));
        }
        Ent::Edge(x) => {
            r.values.push((Quantity::Length, select::length(b, x)));
            if let Some(l) = loop_length(b, x) {
                r.values.push((Quantity::LoopLength, l));
            }
            match geom(p) {
                Geom::Circle(c, z, rad) => {
                    r.values
                        .extend([(Quantity::Radius, rad), (Quantity::Diameter, 2.0 * rad)]);
                    r.values
                        .extend([(Quantity::X, c.x), (Quantity::Y, c.y), (Quantity::Z, c.z)]);
                    r.shown.push(Shown::Circle(c, z, rad));
                }
                Geom::Segment(s, e) => r.shown.push(Shown::Line(s, e, Quantity::Length)),
                _ => {}
            }
        }
        Ent::Face(f) => {
            let area = forge_topo::mass::face_area(b, f)
                .ok()
                .unwrap_or_else(|| {
                    one(b, p.mesh, p.ent)
                        .into_iter()
                        .find(|x| x.0 == "area")
                        .map_or(0.0, |x| x.1)
                });
            r.values.push((Quantity::Area, area));
            let per: f64 = select::edges_of_face(b, f)
                .iter()
                .map(|&x| select::length(b, x))
                .sum();
            r.values.push((Quantity::Perimeter, per));
            match geom(p) {
                Geom::Cylinder(..) | Geom::Sphere(..) => {
                    let rad = select::radius(b, p.ent).unwrap_or(0.0);
                    r.values
                        .extend([(Quantity::Radius, rad), (Quantity::Diameter, 2.0 * rad)]);
                    if let Some((c, z)) = select::axis(b, p.ent) {
                        let c = at.point_to_world(c);
                        let z = z.map_or(Vec3::Z, |z| at.vec_to_world(z));
                        r.shown.push(Shown::Circle(c, z, rad));
                    }
                }
                Geom::Plane(c, _) => r.shown.push(Shown::Point(c)),
                _ => {}
            }
        }
    }
    r
}

/// What two picks read: the minimum distance and its deltas, centre to centre and axis to axis
/// for round things, and the angle between them where both have a direction.
pub fn read_two(a: &Pick, b: &Pick) -> Reading {
    let mut r = Reading::default();
    let (ga, gb) = (geom(a), geom(b));
    let (p, q) = match (ga, gb) {
        // parallel planes: the gap between them, from the first's middle
        (Geom::Plane(c, n), Geom::Plane(c2, n2)) if n.cross(n2).length() < 1e-9 => {
            let d = (c2 - c).dot(n);
            (c, c + n * d)
        }
        _ => nearest(a, b),
    };
    let d = q - p;
    r.values.extend([
        (Quantity::Distance, d.length()),
        (Quantity::Dx, d.x.abs()),
        (Quantity::Dy, d.y.abs()),
        (Quantity::Dz, d.z.abs()),
    ]);
    r.shown.push(Shown::Line(p, q, Quantity::Distance));
    if let (Some(c1), Some(c2)) = (centre_of(ga), centre_of(gb))
        && !matches!((ga, gb), (Geom::Point(_), Geom::Point(_)))
    {
        r.values.push((Quantity::CentreDistance, c1.distance(c2)));
        r.shown.push(Shown::Line(c1, c2, Quantity::CentreDistance));
    }
    let axis = |g: Geom| match g {
        Geom::Cylinder(o, z) | Geom::Circle(o, z, _) => Some((o, z)),
        _ => None,
    };
    if let (Some((o1, z1)), Some((o2, z2))) = (axis(ga), axis(gb))
        && z1.cross(z2).length() < 1e-9
    {
        let w = o2 - o1;
        let off = w - z1 * w.dot(z1);
        if !(matches!(ga, Geom::Circle(..)) && matches!(gb, Geom::Circle(..))) || off.length() > 1e-9 {
            r.values.push((Quantity::AxisDistance, off.length()));
        }
    }
    // two edges from one corner: the angle between them there, up to 180
    let corner = match (ga, gb) {
        (Geom::Segment(a0, a1), Geom::Segment(b0, b1)) => [
            (a0, a1, b0, b1),
            (a0, a1, b1, b0),
            (a1, a0, b0, b1),
            (a1, a0, b1, b0),
        ]
        .into_iter()
        .find(|x| x.0.distance(x.2) < 1e-9)
        .and_then(|x| Some(((x.1 - x.0).normalize()?.get(), (x.3 - x.2).normalize()?.get()))),
        _ => None,
    };
    let deg = |x: f64| x * 180.0 / dmath::PI;
    if let Some((u, w)) = corner {
        r.values
            .push((Quantity::Angle, deg(dmath::acos(u.dot(w).clamp(-1.0, 1.0)))));
    } else if let (Some((u, pu)), Some((w, pw))) = (geom_direction(ga), geom_direction(gb))
        // two round things on parallel axes: their gap says it, an angle of 0 doesn't
        && !(axis(ga).is_some() && axis(gb).is_some() && u.cross(w).length() < 1e-9)
    {
        let c = u.dot(w).clamp(-1.0, 1.0);
        let ang = if pu == pw {
            let a = deg(dmath::acos(c));
            a.min(180.0 - a)
        } else {
            90.0 - deg(dmath::acos(c.abs()))
        };
        r.values.push((Quantity::Angle, ang));
    }
    r
}

/// A whole placed body (an assembly's occurrence, a part's body): its mesh, where it stands, and
/// its material's density in kg/mm³ where that's known.
#[derive(Clone, Copy)]
pub struct Whole<'a> {
    pub body: &'a Body,
    pub mesh: &'a TriMesh,
    pub at: forge_math::Frame,
    pub density: Option<f64>,
}

/// Geometry that is no body's, in the world: a work plane, axis or point, an origin item, or
/// sketch geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum Free {
    Point(Point3),
    /// An infinite line through a point.
    Axis(Point3, Vec3),
    Plane(Point3, Vec3),
    Segment(Point3, Point3),
    Circle(Point3, Vec3, f64),
    /// An arc: its centre, axis, radius, and the points it runs through, end to end.
    Arc(Point3, Vec3, f64, Vec<Point3>),
    /// A spline or an ellipse, as the points it runs through.
    Curve(Vec<Point3>),
}

/// What Measure takes: an entity of a body, a whole body, or free geometry.
#[derive(Clone)]
pub enum Target<'a> {
    Ent(Pick<'a>),
    Whole(Whole<'a>),
    Free(Free),
}

fn free_geom(f: &Free) -> Geom {
    match f {
        Free::Point(p) => Geom::Point(*p),
        Free::Axis(p, u) => Geom::Axis(*p, unit(*u)),
        Free::Plane(p, n) => Geom::Plane(*p, unit(*n)),
        Free::Segment(a, b) => Geom::Segment(*a, *b),
        Free::Circle(c, z, r) | Free::Arc(c, z, r, _) => Geom::Circle(*c, unit(*z), *r),
        Free::Curve(_) => Geom::Other,
    }
}

fn unit(v: Vec3) -> Vec3 {
    v.normalize().map_or(Vec3::Z, |u| u.get())
}

fn target_geom(t: &Target) -> Geom {
    match t {
        Target::Ent(p) => geom(p),
        Target::Whole(_) => Geom::Other,
        Target::Free(f) => free_geom(f),
    }
}

fn polyline_length(pts: &[Point3]) -> f64 {
    pts.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The nearest point of a polyline to `q`.
fn polyline_nearest(pts: &[Point3], q: Point3) -> Option<Point3> {
    pts.windows(2)
        .map(|w| segment_distance(q, w[0], w[1]))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|x| x.1)
        .or(pts.first().copied())
}

/// Points over a target in the world, for the nearest pair to start from.
fn target_points(t: &Target) -> Vec<Point3> {
    let ring = |c: Point3, z: Vec3, r: f64| {
        let (u, v) = across(z);
        (0..96)
            .map(|k| {
                let a = k as f64 / 96.0 * dmath::TAU;
                c + (u * dmath::cos(a) + v * dmath::sin(a)) * r
            })
            .collect::<Vec<_>>()
    };
    match t {
        Target::Ent(p) => world_points(p),
        Target::Whole(w) => {
            let step = w.mesh.positions.len() / 3000 + 1;
            w.mesh
                .positions
                .iter()
                .step_by(step)
                .map(|q| w.at.point_to_world(*q))
                .collect()
        }
        Target::Free(f) => match f {
            Free::Point(p) | Free::Axis(p, _) | Free::Plane(p, _) => vec![*p],
            Free::Segment(a, b) => (0..=32).map(|k| *a + (*b - *a) * (k as f64 / 32.0)).collect(),
            Free::Circle(c, z, r) => ring(*c, unit(*z), *r),
            Free::Arc(_, _, _, pts) | Free::Curve(pts) => pts.clone(),
        },
    }
}

/// Two directions square to `z` and to each other.
fn across(z: Vec3) -> (Vec3, Vec3) {
    let x = if z.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = unit(z.cross(x));
    (u, z.cross(u))
}

/// The closest point of a triangle to `p`.
fn triangle_nearest(p: Point3, [a, b, c]: [Point3; 3]) -> Point3 {
    let n = (b - a).cross(c - a);
    let nn = n.dot(n);
    if nn > 0.0 {
        let inside = [(a, b), (b, c), (c, a)]
            .iter()
            .all(|(u, v)| (*v - *u).cross(p - *u).dot(n) >= 0.0);
        if inside {
            return p - n * ((p - a).dot(n) / nn);
        }
    }
    [(a, b), (b, c), (c, a)]
        .iter()
        .map(|(u, v)| segment_distance(p, *u, *v))
        .min_by(|x, y| x.0.total_cmp(&y.0))
        .map_or(a, |x| x.1)
}

/// The target's point nearest `q`, from its exact geometry where it has one.
fn target_nearest_on(t: &Target, q: Point3) -> Option<Point3> {
    match t {
        Target::Ent(p) => nearest_on(p, q),
        Target::Whole(w) => {
            // the nearest triangle says which face; that face's surface says where exactly
            let local = w.at.point_to_local(q);
            let m = w.mesh;
            let (face, near) = m
                .triangles
                .iter()
                .zip(&m.face_of)
                .map(|(tri, f)| {
                    let on = triangle_nearest(local, tri.map(|i| m.positions[i as usize]));
                    (*f, on)
                })
                .min_by(|a, b| a.1.distance(local).total_cmp(&b.1.distance(local)))?;
            let pick = Pick {
                body: w.body,
                mesh: w.mesh,
                at: w.at,
                ent: Ent::Face(forge_topo::FaceId(face)),
            };
            nearest_on(&pick, q).or(Some(w.at.point_to_world(near)))
        }
        Target::Free(f) => Some(match f {
            Free::Point(p) => *p,
            Free::Axis(p, u) => {
                let u = unit(*u);
                *p + u * (q - *p).dot(u)
            }
            Free::Plane(p, n) => {
                let n = unit(*n);
                q - n * (q - *p).dot(n)
            }
            Free::Segment(a, b) => segment_distance(q, *a, *b).1,
            Free::Circle(c, z, r) => {
                let z = unit(*z);
                let flat = (q - *c) - z * (q - *c).dot(z);
                match flat.normalize() {
                    Some(d) => *c + d.get() * *r,
                    None => *c + across(z).0 * *r,
                }
            }
            Free::Arc(_, _, _, pts) | Free::Curve(pts) => polyline_nearest(pts, q)?,
        }),
    }
}

/// The nearest pair of points of two targets: closed forms where the geometry has them, else
/// the nearest of samples walked down by projecting each onto the other.
fn target_nearest(a: &Target, b: &Target) -> (Point3, Point3) {
    if let (Target::Ent(x), Target::Ent(y)) = (a, b) {
        return nearest(x, y);
    }
    let (ga, gb) = (target_geom(a), target_geom(b));
    match (ga, gb) {
        (Geom::Point(p), Geom::Point(q)) => return (p, q),
        (Geom::Segment(a0, a1), Geom::Segment(b0, b1)) => return segments(a0, a1, b0, b1),
        (Geom::Point(p), _) => {
            if let Some(q) = target_nearest_on(b, p) {
                return (p, q);
            }
        }
        (_, Geom::Point(q)) => {
            if let Some(p) = target_nearest_on(a, q) {
                return (p, q);
            }
        }
        _ => {}
    }
    let (pa, pb) = (target_points(a), target_points(b));
    let mut best = (f64::INFINITY, Point3::ORIGIN, Point3::ORIGIN);
    // an axis or plane has one sample: its nearest point to each of the other's is exact
    let one_a = matches!(ga, Geom::Axis(..) | Geom::Plane(..));
    let one_b = matches!(gb, Geom::Axis(..) | Geom::Plane(..));
    if one_a || one_b {
        let (from, onto, flip) = if one_a { (&pb, a, true) } else { (&pa, b, false) };
        for p in from {
            if let Some(q) = target_nearest_on(onto, *p) {
                let d = p.distance(q);
                if d < best.0 {
                    best = if flip { (d, q, *p) } else { (d, *p, q) };
                }
            }
        }
    } else {
        for p in &pa {
            for q in &pb {
                let d = p.distance(*q);
                if d < best.0 {
                    best = (d, *p, *q);
                }
            }
        }
    }
    let (mut d, mut p, mut q) = best;
    for _ in 0..60 {
        let Some(p2) = target_nearest_on(a, q) else { break };
        let Some(q2) = target_nearest_on(b, p2) else { break };
        let d2 = p2.distance(q2);
        if d2 >= d - 1e-13 {
            break;
        }
        (d, p, q) = (d2, p2, q2);
    }
    (p, q)
}

/// The farthest two points of two targets, where Inventor gives it: points, lines, arcs and
/// circles (two circles in one plane).
fn max_distance(ga: Geom, gb: Geom) -> Option<f64> {
    let ends = |g: Geom| match g {
        Geom::Point(p) => Some(vec![p]),
        Geom::Segment(a, b) => Some(vec![a, b]),
        _ => None,
    };
    // a circle's farthest point from a point: across the centre, off its plane as the point is
    let from_circle = |c: Point3, z: Vec3, r: f64, p: Point3| {
        let w = p - c;
        let h = w.dot(z);
        let rho = (w - z * h).length();
        (h * h + (rho + r) * (rho + r)).sqrt()
    };
    match (ga, gb) {
        (Geom::Circle(c1, z1, r1), Geom::Circle(c2, z2, r2)) => {
            let coplanar = z1.cross(z2).length() < 1e-9 && (c2 - c1).dot(z1).abs() < 1e-6;
            coplanar.then(|| c1.distance(c2) + r1 + r2)
        }
        (Geom::Circle(c, z, r), g) | (g, Geom::Circle(c, z, r)) => {
            let e = ends(g)?;
            e.into_iter().map(|p| from_circle(c, z, r, p)).reduce(f64::max)
        }
        (a, b) => {
            let (ea, eb) = (ends(a)?, ends(b)?);
            if ea.len() == 1 && eb.len() == 1 {
                return None;
            }
            ea.iter()
                .flat_map(|p| eb.iter().map(move |q| p.distance(*q)))
                .reduce(f64::max)
        }
    }
}

/// The arc an angle's drawn as: its vertex and two arms, the arms turned so the arc between them
/// is the angle read.
fn angle_shown(ga: Geom, gb: Geom, p: Point3, q: Point3, deg: f64) -> Option<Shown> {
    let dir = |g: Geom| match g {
        Geom::Segment(a, b) => (b - a).normalize().map(|u| u.get()),
        Geom::Axis(_, u) | Geom::Cylinder(_, u) => Some(u),
        _ => None,
    };
    let (vertex, mut u, mut w) = match (ga, gb) {
        (Geom::Segment(a0, a1), Geom::Segment(b0, b1))
            if [a0, a1].iter().any(|x| x.distance(b0) < 1e-9 || x.distance(b1) < 1e-9) =>
        {
            let (v, ua, wb) = [(a0, a1, b0, b1), (a0, a1, b1, b0), (a1, a0, b0, b1), (a1, a0, b1, b0)]
                .into_iter()
                .find(|x| x.0.distance(x.2) < 1e-9)
                .map(|x| (x.0, x.1 - x.0, x.3 - x.2))?;
            (v, ua.normalize()?.get(), wb.normalize()?.get())
        }
        (Geom::Plane(c1, n1), Geom::Plane(c2, n2)) => {
            let d = n1.cross(n2).normalize()?.get();
            // the line both planes hold, nearest the middle of the two faces
            let m = c1 + (c2 - c1) * 0.5;
            let (h1, h2) = (n1.dot(c1 - Point3::ORIGIN), n2.dot(c2 - Point3::ORIGIN));
            let k = n1.dot(n2);
            let den = 1.0 - k * k;
            let a = (h1 - h2 * k) / den;
            let b = (h2 - h1 * k) / den;
            let on = Point3::ORIGIN + n1 * a + n2 * b;
            let v = on + d * (m - on).dot(d);
            let arm = |n: Vec3, c: Point3| {
                let a = unit(n.cross(d));
                if (c - v).dot(a) < 0.0 { a * -1.0 } else { a }
            };
            (v, arm(n1, c1), arm(n2, c2))
        }
        (Geom::Plane(_, n), g) | (g, Geom::Plane(_, n)) => {
            let u = dir(g)?;
            // square to the plane: any way in it will do
            let flat = (u - n * u.dot(n)).normalize().map_or(across(n).0, |f| f.get());
            let v = if matches!(ga, Geom::Plane(..)) { q } else { p };
            (v, u, flat)
        }
        _ => (p + (q - p) * 0.5, dir(ga)?, dir(gb)?),
    };
    let between = |u: Vec3, w: Vec3| dmath::acos(u.dot(w).clamp(-1.0, 1.0)) * 180.0 / dmath::PI;
    if (between(u, w) - deg).abs() > 1e-6 {
        w = w * -1.0;
    }
    if (between(u, w) - deg).abs() > 1e-6 {
        u = u * -1.0;
    }
    Some(Shown::Angle(vertex, u, w, deg))
}

/// A face's triangles in the world.
fn face_tris(p: &Pick, f: forge_topo::FaceId) -> Vec<[Point3; 3]> {
    let m = p.mesh;
    m.triangles
        .iter()
        .zip(&m.face_of)
        .filter(|x| *x.1 == f.0)
        .map(|x| x.0.map(|i| p.at.point_to_world(m.positions[i as usize])))
        .collect()
}

/// What one target reads, with what the view draws for it.
pub fn read(t: &Target) -> Reading {
    match t {
        Target::Ent(p) => {
            let mut r = read_one(p);
            match p.ent {
                Ent::Face(f) => {
                    let c = p.at.point_to_world(select::centre(p.body, p.ent));
                    r.shown.insert(0, Shown::Face(face_tris(p, f), c));
                }
                Ent::Edge(x) if !matches!(geom(p), Geom::Segment(..)) => {
                    let pts = p.body.edge_points(x, 64).into_iter().map(|q| p.at.point_to_world(q)).collect();
                    r.shown.push(Shown::Curve(pts, Quantity::Length));
                }
                _ => {}
            }
            r
        }
        Target::Whole(w) => {
            let mut r = Reading::default();
            // the body's own integrals, exact; the mesh's where the body's can't be had
            let (volume, area) = match forge_topo::mass::mass(w.body) {
                Ok(m) => (m.volume, m.area),
                Err(_) => {
                    let m = mass(w.mesh);
                    (m.volume, m.area)
                }
            };
            r.values.push((Quantity::Volume, volume));
            if let Some(rho) = w.density {
                r.values.push((Quantity::Mass, volume * rho));
            }
            r.values.push((Quantity::Area, area));
            let (mut lo, mut hi) = (Point3::new(f64::MAX, f64::MAX, f64::MAX), Point3::new(f64::MIN, f64::MIN, f64::MIN));
            let mut hull = |q: Point3| {
                lo = Point3::new(lo.x.min(q.x), lo.y.min(q.y), lo.z.min(q.z));
                hi = Point3::new(hi.x.max(q.x), hi.y.max(q.y), hi.z.max(q.z));
            };
            // square to the world: the body's exact box, turned; else the mesh's in the world
            let square = [w.at.x().get(), w.at.y().get(), w.at.z().get()]
                .iter()
                .all(|a| [a.x, a.y, a.z].iter().filter(|c| c.abs() > 1e-12).count() == 1);
            let exact = body_box(w.body, w.mesh);
            if square && exact.x.lo <= exact.x.hi {
                for k in 0..8 {
                    let pick = |i: forge_math::Interval, bit: usize| if k & bit == 0 { i.lo } else { i.hi };
                    hull(w.at.point_to_world(Point3::new(pick(exact.x, 1), pick(exact.y, 2), pick(exact.z, 4))));
                }
            } else {
                for q in &w.mesh.positions {
                    hull(w.at.point_to_world(*q));
                }
            }
            if lo.x <= hi.x {
                r.values.extend([
                    (Quantity::SizeX, hi.x - lo.x),
                    (Quantity::SizeY, hi.y - lo.y),
                    (Quantity::SizeZ, hi.z - lo.z),
                ]);
                r.shown.push(Shown::Box(lo, hi));
            }
            r
        }
        Target::Free(f) => {
            let mut r = Reading::default();
            let at = |r: &mut Reading, q: Point3| {
                r.values.extend([(Quantity::X, q.x), (Quantity::Y, q.y), (Quantity::Z, q.z)]);
            };
            match f {
                Free::Point(q) => {
                    at(&mut r, *q);
                    r.shown.push(Shown::Point(*q));
                }
                Free::Axis(q, _) | Free::Plane(q, _) => {
                    at(&mut r, *q);
                    r.shown.push(Shown::Point(*q));
                }
                Free::Segment(a, b) => {
                    r.values.push((Quantity::Length, a.distance(*b)));
                    r.shown.push(Shown::Line(*a, *b, Quantity::Length));
                }
                Free::Circle(c, z, rad) => {
                    r.values.extend([
                        (Quantity::Length, dmath::TAU * rad),
                        (Quantity::Radius, *rad),
                        (Quantity::Diameter, 2.0 * rad),
                    ]);
                    at(&mut r, *c);
                    r.shown.push(Shown::Circle(*c, unit(*z), *rad));
                }
                Free::Arc(c, _, rad, pts) => {
                    r.values.extend([
                        (Quantity::Length, polyline_length(pts)),
                        (Quantity::Radius, *rad),
                        (Quantity::Diameter, 2.0 * rad),
                    ]);
                    at(&mut r, *c);
                    r.shown.push(Shown::Curve(pts.clone(), Quantity::Length));
                }
                Free::Curve(pts) => {
                    r.values.push((Quantity::Length, polyline_length(pts)));
                    r.shown.push(Shown::Curve(pts.clone(), Quantity::Length));
                }
            }
            r
        }
    }
}

/// What two targets read: the minimum distance and its deltas, the maximum where Inventor gives
/// it, centre to centre and axis to axis for round things, and the angle where both have a way,
/// drawn as an arc.
pub fn read_pair(a: &Target, b: &Target) -> Reading {
    if let (Target::Ent(x), Target::Ent(y)) = (a, b) {
        let mut r = read_two(x, y);
        let (ga, gb) = (geom(x), geom(y));
        finish_pair(&mut r, ga, gb);
        return r;
    }
    let mut r = Reading::default();
    let (ga, gb) = (target_geom(a), target_geom(b));
    let (p, q) = match (ga, gb) {
        (Geom::Plane(c, n), Geom::Plane(c2, n2)) if n.cross(n2).length() < 1e-9 => {
            let d = (c2 - c).dot(n);
            (c, c + n * d)
        }
        _ => target_nearest(a, b),
    };
    let d = q - p;
    r.values.extend([
        (Quantity::Distance, d.length()),
        (Quantity::Dx, d.x.abs()),
        (Quantity::Dy, d.y.abs()),
        (Quantity::Dz, d.z.abs()),
    ]);
    r.shown.push(Shown::Line(p, q, Quantity::Distance));
    if let (Some(c1), Some(c2)) = (centre_of(ga), centre_of(gb))
        && !matches!((ga, gb), (Geom::Point(_), Geom::Point(_)))
    {
        r.values.push((Quantity::CentreDistance, c1.distance(c2)));
        r.shown.push(Shown::Line(c1, c2, Quantity::CentreDistance));
    }
    let axis = |g: Geom| match g {
        Geom::Cylinder(o, z) | Geom::Circle(o, z, _) | Geom::Axis(o, z) => Some((o, z)),
        _ => None,
    };
    if let (Some((o1, z1)), Some((o2, z2))) = (axis(ga), axis(gb))
        && z1.cross(z2).length() < 1e-9
    {
        let w = o2 - o1;
        let off = w - z1 * w.dot(z1);
        r.values.push((Quantity::AxisDistance, off.length()));
    }
    if let (Some((u, pu)), Some((w, pw))) = (geom_direction(ga), geom_direction(gb))
        && !(axis(ga).is_some() && axis(gb).is_some() && u.cross(w).length() < 1e-9)
    {
        let deg = |x: f64| x * 180.0 / dmath::PI;
        let c = u.dot(w).clamp(-1.0, 1.0);
        let ang = if pu == pw {
            let a = deg(dmath::acos(c));
            a.min(180.0 - a)
        } else {
            90.0 - deg(dmath::acos(c.abs()))
        };
        r.values.push((Quantity::Angle, ang));
    }
    finish_pair(&mut r, ga, gb);
    r
}

/// The maximum distance and the angle's arc, for any pair.
fn finish_pair(r: &mut Reading, ga: Geom, gb: Geom) {
    if let Some(m) = max_distance(ga, gb) {
        r.values.push((Quantity::MaxDistance, m));
    }
    let line = r.shown.iter().find_map(|s| match s {
        Shown::Line(p, q, Quantity::Distance) => Some((*p, *q)),
        _ => None,
    });
    if let (Some(deg), Some((p, q))) = (r.get(Quantity::Angle), line)
        && deg > 1e-9
        && let Some(s) = angle_shown(ga, gb, p, q, deg)
    {
        r.shown.push(s);
    }
}

/// A face's outward normal at its point nearest `q` (in the body's space).
pub fn normal_near(body: &Body, f: forge_topo::FaceId, q: Point3) -> Option<Vec3> {
    use forge_geom::SurfaceGeom;
    let face = body.face(f);
    let n = face.surface.normal(face.surface.project(q))?.get();
    Some(if face.same_sense { n } else { -n })
}

/// A body's box in its own space, exact where its edges are lines and circles (their ends and a
/// circle's farthest points along each axis), and from dense samples along other curves; a
/// sphere's or torus's bulge past its edges from its mesh, whose points are on it.
fn body_box(b: &Body, mesh: &TriMesh) -> forge_math::Box3 {
    use forge_geom::CurveGeom;
    let mut out = forge_math::Box3::EMPTY;
    let mut put = |q: Point3| out = out.hull(&forge_math::Box3::from_point(q));
    for e in b.edge_ids() {
        let x = b.edge(e);
        put(b.vertex(x.start).point);
        put(b.vertex(x.end).point);
        match &x.curve {
            Curve::Line(_) => {}
            Curve::Circle(c) => {
                let (u, v) = (c.frame.x().get(), c.frame.y().get());
                for (ux, vx) in [(u.x, v.x), (u.y, v.y), (u.z, v.z)] {
                    let t0 = dmath::atan2(vx, ux);
                    for t in [t0, t0 + dmath::PI] {
                        // the angle, wound into the edge's range
                        let lo = x.range.lo;
                        let t = lo + (t - lo).rem_euclid(dmath::TAU);
                        if t <= x.range.hi + 1e-12 {
                            put(x.curve.eval(t));
                        }
                    }
                }
            }
            _ => {
                for k in 0..=512 {
                    let t = x.range.lo + (x.range.hi - x.range.lo) * k as f64 / 512.0;
                    put(x.curve.eval(t));
                }
            }
        }
    }
    let round: Vec<u32> = b
        .face_ids()
        .filter(|&f| matches!(b.face(f).surface, forge_geom::Surface::Sphere(_) | forge_geom::Surface::Torus(_)))
        .map(|f| f.0)
        .collect();
    for (tri, f) in mesh.triangles.iter().zip(&mesh.face_of) {
        if round.contains(f) {
            for i in tri {
                put(mesh.positions[*i as usize]);
            }
        }
    }
    out
}
