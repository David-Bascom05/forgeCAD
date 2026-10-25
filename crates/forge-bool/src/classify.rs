use forge_geom::intersect::{Contact, LineSurface, line_surface};
use forge_geom::{Curve, CurveGeom, Line, Surface, SurfaceGeom};
use forge_math::{Box2, Box3, Interval, Point2, Point3, Vec3, tol};
use forge_topo::uv::{UvLoop, face_uv_loops, point_in_loops};
use forge_topo::{Body, FaceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    Inside,
    Outside,
    /// Within tolerance of the boundary.
    On,
}

/// A face's loops in parameter space, built the first time a query needs them.
pub struct FaceInfo {
    /// Each boundary use with its edge's box.
    edges: Vec<(forge_topo::CoedgeId, Box3)>,
    /// The largest sag bound among the face's edges: an edge farther than
    /// this can never decide a point, so it is skipped without solving.
    reach: f64,
    /// The loops drawn in parameter space, only when a query needs them
    /// (planar faces are decided exactly without them).
    drawn: std::cell::OnceCell<Drawn>,
}

pub struct Drawn {
    pub loops: Vec<UvLoop>,
    centre: Point2,
    /// The parameter window the loops are drawn in.
    window: Box2,
}

pub struct Index<'a> {
    pub body: &'a Body,
    /// A box guaranteed to hold each face.
    pub boxes: Vec<Box3>,
    infos: Vec<std::cell::OnceCell<FaceInfo>>,
    pub bbox: Box3,
}

/// A box holding face `f` (cached on the face).
pub fn face_box(body: &Body, f: FaceId) -> Box3 {
    body.face_box(f)
}

impl<'a> Index<'a> {
    pub fn new(body: &'a Body) -> Self {
        // indexed by face id; removed faces get an empty box, which nothing overlaps
        let boxes: Vec<Box3> = (0..body.faces.len() as u32)
            .map(FaceId)
            .map(|f| {
                if body.face(f).alive {
                    face_box(body, f)
                } else {
                    Box3::EMPTY
                }
            })
            .collect();
        let bbox = boxes.iter().fold(Box3::EMPTY, |a, b| a.hull(b));
        let infos = (0..boxes.len()).map(|_| std::cell::OnceCell::new()).collect();
        Index {
            body,
            boxes,
            infos,
            bbox,
        }
    }

    fn info(&self, f: FaceId) -> &FaceInfo {
        self.infos[f.index()].get_or_init(|| {
            let mut edges = Vec::new();
            let mut reach = 10.0 * tol::LINEAR;
            for &l in &self.body.face(f).loops {
                for &c in &self.body.lp(l).coedges {
                    let e = self.body.edge(self.body.coedge(c).edge);
                    // with margin: an edge in a coordinate plane has a box a few ulps thin
                    edges.push((c, e.curve.bbox(e.range).grow(10.0 * tol::LINEAR)));
                    reach = reach.max(sag_bound(&e.curve));
                }
            }
            FaceInfo {
                edges,
                reach,
                drawn: std::cell::OnceCell::new(),
            }
        })
    }

    fn drawn(&self, f: FaceId) -> &Drawn {
        self.info(f).drawn.get_or_init(|| {
            let loops = face_uv_loops(self.body, f);
            let (mut u, mut v) = (Interval::EMPTY, Interval::EMPTY);
            for p in loops.iter().flat_map(|l| l.points.iter()) {
                u.include(p.x);
                v.include(p.y);
            }
            let centre = loops.first().map_or(Point2::ORIGIN, |l| {
                let n = l.points.len().max(1) as f64;
                let s = l.points.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.y));
                Point2::new(s.0 / n, s.1 / n)
            });
            Drawn {
                loops,
                centre,
                window: Box2::new(u, v),
            }
        })
    }

    /// Where a point that lies on (or within tolerance of) face `f`'s surface
    /// falls relative to the face.
    pub fn point_in_face(&self, f: FaceId, p: Point3) -> Where {
        let body = self.body;
        let face = body.face(f);
        let info = self.info(f);
        // nearest boundary point, exactly
        let mut best: Option<(f64, forge_topo::CoedgeId, f64)> = None;
        for &(c, ref b) in &info.edges {
            if box_distance(b, p) > info.reach.min(best.map_or(f64::INFINITY, |x| x.0)) {
                continue;
            }
            let e = body.edge(body.coedge(c).edge);
            let t = e.curve.closest(p, e.range);
            let d = e.curve.eval(t).distance(p);
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, c, t));
            }
        }
        if let Some((d, c, t)) = best {
            if d <= tol::LINEAR {
                return Where::On;
            }
            let co = body.coedge(c);
            let e = body.edge(co.edge);
            // a seam (an edge the face uses twice) has two sides that are both this face
            let seam = body.partner(c).is_some_and(|o| body.coedge_face(o) == f);
            let interior = !seam
                && t > e.range.lo + 1e-9 * (1.0 + e.range.width())
                && t < e.range.hi - 1e-9 * (1.0 + e.range.width());
            // the polygon of the loop can be off by its chord sag near curved edges; decide those by side
            if interior && d <= sag_bound(&e.curve) {
                let q = e.curve.eval(t);
                let along = if co.forward {
                    e.curve.deriv(t).d1
                } else {
                    -e.curve.deriv(t).d1
                };
                if let Some(n) = body.face_normal(f, face.surface.project(q)) {
                    let left = n.cross(along);
                    return if left.dot(p - q) > 0.0 {
                        Where::Inside
                    } else {
                        Where::Outside
                    };
                }
            }
        }
        if let Surface::Plane(plane) = &face.surface {
            for angle in [0.1234, 1.9876, 4.0123] {
                if let Some(inside) = planar_parity(body, plane, p, &info.edges, angle) {
                    return if inside { Where::Inside } else { Where::Outside };
                }
            }
        }
        let drawn = self.drawn(f);
        let uv = into_window(body, f, face.surface.project(p), drawn);
        if point_in_loops(uv, &drawn.loops) {
            Where::Inside
        } else {
            Where::Outside
        }
    }

    /// Inside, outside or on the solid.
    pub fn classify(&self, p: Point3) -> Where {
        if !self.bbox.contains(p) {
            return Where::Outside;
        }
        for f in self.body.face_ids() {
            if self.boxes[f.index()].contains(p)
                && self.body.face(f).surface.distance(p) <= tol::LINEAR
                && self.point_in_face(f, p) != Where::Outside
            {
                return Where::On;
            }
        }
        'ray: for d in RAYS {
            let dir = Vec3::new(d[0], d[1], d[2])
                .normalize()
                .expect("fixed ray directions are nonzero");
            let line = Line::new(p, dir);
            let mut crossings = 0usize;
            for f in self.body.face_ids() {
                if !ray_hits_box(p, dir.get(), &self.boxes[f.index()]) {
                    continue;
                }
                match line_surface(&line, &self.body.face(f).surface) {
                    LineSurface::OnSurface => continue 'ray,
                    LineSurface::Hits(hits) => {
                        for h in hits.iter().filter(|h| h.t > tol::LINEAR) {
                            if h.contact == Contact::Touching {
                                continue 'ray;
                            }
                            match self.point_in_face(f, h.point) {
                                Where::Inside => crossings += 1,
                                Where::On => continue 'ray,
                                Where::Outside => {}
                            }
                        }
                    }
                }
            }
            return if crossings % 2 == 1 {
                Where::Inside
            } else {
                Where::Outside
            };
        }
        Where::On
    }
}

/// Shifts parameters by whole periods into the face's drawing window (the
/// nearest-to-centre shift misplaces points near a window edge when the face
/// spans most of a period).
fn into_window(body: &Body, f: FaceId, uv: Point2, info: &Drawn) -> Point2 {
    let d = body.face(f).surface.domain();
    let fit = |x: f64, w: Interval, c: f64, period: Option<f64>| match period {
        Some(p) => [0.0, -p, p, -2.0 * p, 2.0 * p]
            .iter()
            .map(|k| x + k)
            .find(|&y| y >= w.lo - 1e-12 && y <= w.hi + 1e-12)
            .unwrap_or_else(|| x + p * ((c - x) / p).round()),
        None => x,
    };
    Point2::new(
        fit(uv.x, info.window.u, info.centre.x, d.u.period),
        fit(uv.y, info.window.v, info.centre.y, d.v.period),
    )
}

/// Fixed, deliberately irregular ray directions: a ray along one of these
/// rarely grazes a model edge, and the next one is tried if it does.
const RAYS: [[f64; 3]; 8] = [
    [0.5773, 0.6071, 0.5463],
    [-0.3149, 0.8352, 0.4510],
    [0.7231, -0.2017, 0.6606],
    [-0.6418, -0.5528, 0.5316],
    [0.1937, 0.4123, -0.8902],
    [-0.8836, 0.3091, -0.3518],
    [0.4402, -0.8577, -0.2657],
    [-0.0912, -0.2384, 0.9669],
];

/// How far a loop polygon (default sampling, pi/16 per segment) can sit from
/// a curved edge, plus tolerance.
fn sag_bound(c: &Curve) -> f64 {
    let r = match c {
        Curve::Line(_) => 0.0,
        Curve::Circle(k) => k.radius,
        Curve::Ellipse(e) => e.major * e.major / e.minor.max(tol::LINEAR),
        Curve::Parabola(_) | Curve::Hyperbola(_) => 1e3,
        Curve::Intersection(k) => return 2.0 * k.sag + 10.0 * tol::LINEAR,
    };
    0.01 * r + 10.0 * tol::LINEAR
}

/// Exact even-odd test for a point of a planar face: a ray from p in the
/// plane at `angle` to its x axis, counting crossings with each edge in
/// closed form (lines, circles, ellipses). None when the ray meets a vertex
/// or an edge the closed forms do not cover, so another direction is tried.
fn planar_parity(
    body: &Body,
    plane: &forge_geom::Plane,
    p: Point3,
    edges: &[(forge_topo::CoedgeId, Box3)],
    angle: f64,
) -> Option<bool> {
    let (sn, cs) = forge_math::dmath::sin_cos(angle);
    let d = plane.frame.x() * cs + plane.frame.y() * sn;
    let mut inside = false;
    for &(c, ref b) in edges {
        if !ray_hits_box(p, d, b) {
            continue;
        }
        let e = body.edge(body.coedge(c).edge);
        // a seam cannot occur on a plane; each use of an edge here is one crossing set
        let hits: Vec<(f64, f64)> = match &e.curve {
            Curve::Line(l) => {
                // ray p + s d against segment o + t dir, t in range
                let w = l.origin - p;
                let n = plane.frame.z().get();
                let den = d.cross(l.dir.get()).dot(n);
                if den.abs() <= 1e-15 {
                    if l.dir.cross(w).dot(n).abs() <= tol::LINEAR {
                        return None; // the ray runs along this edge
                    }
                    continue;
                }
                let s = l.dir.get().cross(w).dot(n) / -den;
                let t = d.cross(w).dot(n) / -den;
                vec![(s, t)]
            }
            Curve::Circle(k) => conic_ray(&k.frame, k.radius, k.radius, p, d),
            Curve::Ellipse(el) => conic_ray(&el.frame, el.major, el.minor, p, d),
            _ => return None,
        };
        let periodic = e.curve.domain().period;
        for (s, t) in hits {
            if s <= 0.0 {
                continue;
            }
            let t = match periodic {
                Some(per) => e.range.lo + (t - e.range.lo).rem_euclid(per),
                None => t,
            };
            let span = e.range.width().max(1e-300);
            let margin = 1e-9 * (1.0 + span);
            if (t - e.range.lo).abs() <= margin || (t - e.range.hi).abs() <= margin {
                return None; // through a vertex
            }
            if t > e.range.lo && t < e.range.hi {
                inside = !inside;
            }
        }
    }
    Some(inside)
}

/// Where the ray p + s d (s any sign) meets a conic o + a cos t x + b sin t y
/// lying in a plane that contains the ray: (s, t) pairs. A tangent ray gives
/// none (it touches without crossing).
fn conic_ray(f: &forge_math::Frame, a: f64, b: f64, p: Point3, d: Vec3) -> Vec<(f64, f64)> {
    let q = f.point_to_local(p);
    let v = f.vec_to_local(d);
    // ((qx + s vx) / a)^2 + ((qy + s vy) / b)^2 = 1
    let (qx, qy, vx, vy) = (q.x / a, q.y / b, v.x / a, v.y / b);
    let (aa, bb, cc) = (
        vx * vx + vy * vy,
        2.0 * (qx * vx + qy * vy),
        qx * qx + qy * qy - 1.0,
    );
    let disc = bb * bb - 4.0 * aa * cc;
    if disc <= 1e-12 * (bb * bb + (4.0 * aa * cc).abs()) || aa == 0.0 {
        return Vec::new();
    }
    let r = disc.sqrt();
    let qq = -0.5 * (bb + if bb >= 0.0 { r } else { -r });
    [qq / aa, cc / qq]
        .into_iter()
        .map(|s| (s, forge_math::dmath::atan2(qy + s * vy, qx + s * vx)))
        .collect()
}

fn box_distance(b: &Box3, p: Point3) -> f64 {
    let gap = |i: Interval, x: f64| (i.lo - x).max(x - i.hi).max(0.0);
    let (dx, dy, dz) = (gap(b.x, p.x), gap(b.y, p.y), gap(b.z, p.z));
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn ray_hits_box(o: Point3, d: Vec3, b: &Box3) -> bool {
    let (mut t0, mut t1) = (0.0f64, f64::INFINITY);
    for (oi, di, iv) in [(o.x, d.x, b.x), (o.y, d.y, b.y), (o.z, d.z, b.z)] {
        if di.abs() < 1e-300 {
            if oi < iv.lo || oi > iv.hi {
                return false;
            }
            continue;
        }
        let (a, c) = ((iv.lo - oi) / di, (iv.hi - oi) / di);
        t0 = t0.max(a.min(c));
        t1 = t1.min(a.max(c));
        if t0 > t1 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Vec2, dmath};
    use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};

    const Y: Axis2 = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };

    fn agree(body: &Body, inside: impl Fn(Point3) -> Option<bool>, size: f64, seed: u64) {
        let ix = Index::new(body);
        let mut r = Rng::new(seed);
        let mut checked = 0;
        for _ in 0..3000 {
            let p = Point3::new(r.range(-size, size), r.range(-size, size), r.range(-size, size));
            let Some(want) = inside(p) else { continue };
            let got = ix.classify(p);
            assert_eq!(got, if want { Where::Inside } else { Where::Outside }, "{p:?}");
            checked += 1;
        }
        assert!(checked > 2000);
    }

    /// None when the point is too close to call.
    fn margin(d: f64) -> Option<bool> {
        (d.abs() > 1e-6).then_some(d < 0.0)
    }

    #[test]
    fn box_and_plate() {
        let b = extrude(
            &Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)],
            },
            5.0,
            1,
        )
        .unwrap();
        agree(
            &b,
            |p| {
                margin(
                    (-p.x)
                        .max(p.x - 10.0)
                        .max(-p.y)
                        .max(p.y - 20.0)
                        .max(-p.z)
                        .max(p.z - 5.0),
                )
            },
            25.0,
            1,
        );
        let plate = Profile {
            plane: Frame::WORLD,
            loops: vec![
                Profile::rectangle(Point2::ORIGIN, 30.0, 20.0, 0),
                Profile::circle(Point2::new(15.0, 10.0), 4.0, 10),
            ],
        };
        let b = extrude(&plate, 6.0, 1).unwrap();
        agree(
            &b,
            |p| {
                let hole = 4.0 - ((p.x - 15.0).powi(2) + (p.y - 10.0).powi(2)).sqrt();
                margin(
                    (-p.x)
                        .max(p.x - 30.0)
                        .max(-p.y)
                        .max(p.y - 20.0)
                        .max(-p.z)
                        .max(p.z - 6.0)
                        .max(hole),
                )
            },
            35.0,
            2,
        );
    }

    #[test]
    fn revolved_solids() {
        let q = dmath::FRAC_PI_2;
        let half = ProfileLoop::new(vec![
            (
                Segment::Arc {
                    center: Point2::ORIGIN,
                    radius: 5.0,
                    start: -q,
                    sweep: 2.0 * q,
                },
                0,
            ),
            (
                Segment::Line {
                    a: Point2::new(0.0, 5.0),
                    b: Point2::new(0.0, -5.0),
                },
                1,
            ),
        ]);
        let sphere = revolve(
            &Profile {
                plane: Frame::WORLD,
                loops: vec![half],
            },
            Y,
            dmath::TAU,
            1,
        )
        .unwrap();
        agree(&sphere, |p| margin(p.to_vec().length() - 5.0), 8.0, 3);

        let torus = revolve(
            &Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::circle(Point2::new(10.0, 0.0), 3.0, 0)],
            },
            Y,
            dmath::TAU,
            1,
        )
        .unwrap();
        agree(
            &torus,
            |p| {
                margin({
                    let d = (p.x * p.x + p.z * p.z).sqrt() - 10.0;
                    (d * d + p.y * p.y).sqrt() - 3.0
                })
            },
            15.0,
            4,
        );

        let rect = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(5.0, 0.0), 4.0, 10.0, 0)],
        };
        let quarter = revolve(&rect, Y, q, 1).unwrap();
        agree(
            &quarter,
            |p| {
                let rho = (p.x * p.x + p.z * p.z).sqrt();
                let phi = dmath::atan2(-p.z, p.x);
                let ang = (-phi).max(phi - q) * rho;
                margin((5.0 - rho).max(rho - 9.0).max(-p.y).max(p.y - 10.0).max(ang))
            },
            12.0,
            5,
        );
    }

    #[test]
    fn points_on_the_boundary() {
        let b = extrude(
            &Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)],
            },
            5.0,
            1,
        )
        .unwrap();
        let ix = Index::new(&b);
        assert_eq!(ix.classify(Point3::new(5.0, 5.0, 5.0)), Where::On);
        assert_eq!(ix.classify(Point3::new(10.0, 20.0, 5.0)), Where::On);
        assert_eq!(ix.classify(Point3::new(5.0, 5.0, 2.5)), Where::Inside);
    }
}

#[cfg(test)]
mod planar_vs_polygon {
    use super::*;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Vec2};
    use forge_ops::{Axis2, Profile, revolve};

    #[test]
    fn agree() {
        let rect = Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(5.0, 0.0), 4.0, 10.0, 0)],
        };
        let body = revolve(
            &rect,
            Axis2 {
                origin: Point2::ORIGIN,
                dir: Vec2::Y,
            },
            forge_math::dmath::FRAC_PI_2,
            1,
        )
        .unwrap();
        let ix = Index::new(&body);
        let mut r = Rng::new(5);
        for f in body.face_ids() {
            let Surface::Plane(pl) = body.face(f).surface else {
                continue;
            };
            for _ in 0..2000 {
                let p = pl
                    .frame
                    .plane_point(Point2::new(r.range(-12.0, 12.0), r.range(-12.0, 12.0)));
                let info = ix.info(f);
                // the drawn polygon is only trustworthy away from curved edges
                let near = info.edges.iter().any(|&(c, _)| {
                    let e = body.edge(body.coedge(c).edge);
                    e.curve.eval(e.curve.closest(p, e.range)).distance(p) < 0.1
                });
                if near {
                    continue;
                }
                let Some(fast) = planar_parity(&body, &pl, p, &info.edges, 0.1234) else {
                    continue;
                };
                let drawn = ix.drawn(f);
                let uv = into_window(&body, f, body.face(f).surface.project(p), drawn);
                let slow = point_in_loops(uv, &drawn.loops);
                assert_eq!(fast, slow, "face {f:?} p {p:?}");
            }
        }
    }
}
