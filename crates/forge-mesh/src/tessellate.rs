use crate::cdt::{CdtError, triangulate};
use crate::mesh::TriMesh;
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Point2, dmath};
use forge_topo::uv::unwrap_near;
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

fn edge_segments(curve: &Curve, width: f64, q: &Quality) -> usize {
    let n = match curve {
        Curve::Line(_) => 1.0,
        Curve::Circle(c) => width / arc_step(c.radius, q),
        Curve::Ellipse(e) => width / arc_step(e.major, q),
        Curve::Parabola(_) | Curve::Hyperbola(_) => 64.0,
    };
    (n.ceil() as usize).max(1)
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
    }
}

/// Grid steps for points inside a face, where the surface curves in both directions.
fn interior_steps(s: &Surface, q: &Quality) -> Option<(f64, f64)> {
    match s {
        Surface::Plane(_) | Surface::Cylinder(_) | Surface::Cone(_) => None,
        Surface::Sphere(sp) => Some((arc_step(sp.radius, q), arc_step(sp.radius, q))),
        Surface::Torus(t) => Some((arc_step(t.major + t.minor, q), arc_step(t.minor, q))),
    }
}

/// A watertight mesh of a body: each edge is cut once and both faces share
/// those cut points, so neighbouring faces meet exactly.
pub fn tessellate(body: &Body, q: &Quality) -> Result<TriMesh, TessError> {
    let mut mesh = TriMesh::default();
    let vertex_index: Vec<u32> = body.vertices.iter().map(|v| mesh.add_vertex(v.point)).collect();

    // per edge: its parameter samples and their mesh indices (ends are the body's vertices)
    let mut edge_samples: Vec<Vec<(f64, u32)>> = Vec::with_capacity(body.edges.len());
    for e in body.edge_ids() {
        let edge = body.edge(e);
        let n = edge_segments(&edge.curve, edge.range.width(), q);
        let mut s = Vec::with_capacity(n + 1);
        s.push((edge.range.lo, vertex_index[edge.start.index()]));
        for i in 1..n {
            let t = edge.range.lo + edge.range.width() * (i as f64 / n as f64);
            s.push((t, mesh.add_vertex(edge.curve.eval(t))));
        }
        s.push((edge.range.hi, vertex_index[edge.end.index()]));
        edge_samples.push(s);
    }

    for f in body.face_ids() {
        tessellate_face(body, f, &edge_samples, q, &mut mesh)
            .map_err(|error| TessError::Face { face: f, error })?;
    }
    Ok(mesh)
}

fn tessellate_face(
    body: &Body,
    f: FaceId,
    edge_samples: &[Vec<(f64, u32)>],
    q: &Quality,
    mesh: &mut TriMesh,
) -> Result<(), CdtError> {
    let face = body.face(f);
    let surface = &face.surface;
    let (su, sv) = uv_scale(surface);

    let mut uv: Vec<Point2> = Vec::new();
    let mut global: Vec<u32> = Vec::new();
    let mut loops: Vec<Vec<u32>> = Vec::new();
    let mut window: Option<Point2> = None;
    for &l in &face.loops {
        let mut ring = Vec::new();
        let mut prev: Option<Point2> = None;
        for &c in &body.lp(l).coedges {
            let co = body.coedge(c);
            let samples = &edge_samples[co.edge.index()];
            let ordered: Vec<(f64, u32)> = if co.forward {
                samples.clone()
            } else {
                samples.iter().rev().copied().collect()
            };
            let edge = body.edge(co.edge);
            for &(t, gi) in &ordered[..ordered.len() - 1] {
                let p = surface.project(edge.curve.eval(t));
                let p = match prev.or(window) {
                    Some(r) => unwrap_near(body, f, p, r),
                    None => p,
                };
                prev = Some(p);
                ring.push(uv.len() as u32);
                uv.push(p);
                global.push(gi);
            }
        }
        if window.is_none() {
            window = ring.first().map(|&i| uv[i as usize]);
        }
        loops.push(ring);
    }

    let scaled: Vec<Point2> = uv.iter().map(|p| Point2::new(p.x * su, p.y * sv)).collect();
    let mut interior = Vec::new();
    if let Some((du, dv)) = interior_steps(surface, q) {
        let (mut lo, mut hi) = (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for p in &uv {
            lo = Point2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        let nu = ((hi.x - lo.x) / du).ceil().max(1.0) as usize;
        let nv = ((hi.y - lo.y) / dv).ceil().max(1.0) as usize;
        for i in 1..nu {
            for j in 1..nv {
                let u = lo.x + (hi.x - lo.x) * i as f64 / nu as f64;
                let v = lo.y + (hi.y - lo.y) * j as f64 / nv as f64;
                interior.push(Point2::new(u * su, v * sv));
            }
        }
    }

    let tri = triangulate(&scaled, &loops, &interior)?;
    let base = global.len();
    let mut index = global;
    for p in &tri.points[base..] {
        let uvp = Point2::new(p.x / su, p.y / sv);
        index.push(mesh.add_vertex(surface.eval(uvp)));
    }
    for t in &tri.triangles {
        let [a, b, c] = t.map(|k| index[k as usize]);
        // counter-clockwise in (u, v) faces along du x dv, the surface's natural normal
        let t = if face.same_sense { [a, b, c] } else { [a, c, b] };
        mesh.triangles.push(t);
        mesh.face_of.push(f.0);
    }
    Ok(())
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
