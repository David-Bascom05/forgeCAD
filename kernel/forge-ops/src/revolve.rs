use crate::profile::{Profile, ProfileError, Segment};
use forge_geom::{Circle, Cone, Curve, Cylinder, Line, Plane, Sphere, Surface, SurfaceGeom, Torus};
use forge_math::{Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use forge_topo::{Body, EdgeId, FaceId, Name, ShellId, VertexId};

/// A line in the sketch plane to revolve about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axis2 {
    pub origin: Point2,
    pub dir: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RevolveError {
    Profile(ProfileError),
    /// The angle must be in (0, 2pi].
    BadAngle,
    BadAxis,
    /// Part of the profile lies on both sides of the axis.
    CrossesAxis,
    /// A shape the kernel cannot make yet (e.g. a self-intersecting torus).
    Unsupported(&'static str),
}

impl From<ProfileError> for RevolveError {
    fn from(e: ProfileError) -> Self {
        RevolveError::Profile(e)
    }
}

/// Spins a profile about an axis in its plane by `angle` (2pi for a whole
/// turn). Lines become planes, cylinders or cones and arcs become spheres or
/// tori. A whole turn of a profile that makes only planes and cylinders gives one face per
/// segment with the profile's copy at angle 0 as its seam (as an extrude's round walls; the names
/// the second half-turn had stay as aliases); a profile with a cone, sphere or torus in it gives
/// two half-turn faces per segment.
pub fn revolve(profile: &Profile, axis: Axis2, angle: f64, feature: u64) -> Result<Body, RevolveError> {
    if !(angle > tol::ANGULAR && angle <= dmath::TAU + tol::ANGULAR) {
        return Err(RevolveError::BadAngle);
    }
    let full = angle >= dmath::TAU - tol::ANGULAR;
    let profile = profile.normalized()?;
    let dir2 = axis.dir.normalize().ok_or(RevolveError::BadAxis)?;
    let perp2 = dir2.perp();
    let mut side = 0.0f64;
    for l in &profile.loops {
        for (s, _) in &l.segments {
            for p in s.samples(32) {
                let r = (p - axis.origin).dot(perp2);
                side = if r.abs() > side.abs() { r } else { side };
            }
        }
    }
    if side.abs() <= tol::LINEAR {
        return Err(RevolveError::CrossesAxis);
    }
    let side = side.signum();
    let plane = profile.plane;
    let vec3 = |v: Vec2| plane.x() * v.x + plane.y() * v.y;
    let z = vec3(dir2).normalize().ok_or(RevolveError::BadAxis)?;
    let x0 = (vec3(perp2) * side).normalize().ok_or(RevolveError::BadAxis)?;
    let rf = Frame::from_zx(plane.plane_point(axis.origin), z, x0.get()).ok_or(RevolveError::BadAxis)?;
    let b = Revolver {
        profile: &profile,
        axis_origin: axis.origin,
        dir2,
        perp2: perp2 * side,
        rf,
        angles: vec![0.0, if full { dmath::PI } else { angle }],
        full,
        root: Name::feature(feature),
        body: Body::new(),
    };
    let mut b = b;
    let flat_or_round = |s: &Segment| {
        matches!(
            b.surface_of(s),
            Ok(None | Some(Surface::Plane(_) | Surface::Cylinder(_) | Surface::Cone(_)))
        )
    };
    if full
        && profile
            .loops
            .iter()
            .all(|l| l.segments.iter().all(|(s, _)| flat_or_round(s)))
    {
        b.angles = vec![0.0];
    }
    for l in &profile.loops {
        for (s, _) in &l.segments {
            for p in s.samples(32) {
                if (p - axis.origin).dot(perp2) * side < -tol::LINEAR {
                    return Err(RevolveError::CrossesAxis);
                }
            }
        }
    }
    b.build()
}

struct Revolver<'a> {
    profile: &'a Profile,
    axis_origin: Point2,
    dir2: Vec2,
    perp2: Vec2,
    /// z along the axis, x towards the profile.
    rf: Frame,
    /// Angles of the profile copies; spans run between consecutive copies
    /// (and from the last back to the first on a whole turn).
    angles: Vec<f64>,
    full: bool,
    root: Name,
    body: Body,
}

/// Where a profile segment goes: its edges in each copy and how it walks them.
struct SegInfo {
    seg: Segment,
    id: u64,
    start: usize,
    end: usize,
    /// Profile direction runs with the edge curve's parameter.
    forward: bool,
}

impl Revolver<'_> {
    /// Height along the axis and distance from it.
    fn hr(&self, p: Point2) -> (f64, f64) {
        let d = p - self.axis_origin;
        (d.dot(self.dir2), d.dot(self.perp2).max(0.0))
    }
    fn on_axis(&self, p: Point2) -> bool {
        self.hr(p).1 <= tol::LINEAR
    }
    fn point3(&self, p: Point2) -> Point3 {
        self.profile.plane.plane_point(p)
    }
    fn vec3(&self, v: Vec2) -> Vec3 {
        let f = &self.profile.plane;
        f.x() * v.x + f.y() * v.y
    }
    fn rotate(&self, p: Point3, a: f64) -> Point3 {
        let l = self.rf.point_to_local(p);
        let (s, c) = dmath::sin_cos(a);
        self.rf
            .point_to_world(Point3::new(l.x * c - l.y * s, l.x * s + l.y * c, l.z))
    }
    fn rotate_vec(&self, v: Vec3, a: f64) -> Vec3 {
        let l = self.rf.vec_to_local(v);
        let (s, c) = dmath::sin_cos(a);
        self.rf
            .vec_to_world(Vec3::new(l.x * c - l.y * s, l.x * s + l.y * c, l.z))
    }
    fn rotate_frame(&self, f: &Frame, a: f64) -> Frame {
        let z = self
            .rotate_vec(f.z().get(), a)
            .normalize()
            .expect("rotation keeps length");
        let x = self.rotate_vec(f.x().get(), a);
        Frame::from_zx(self.rotate(f.origin, a), z, x).expect("rotation keeps axes perpendicular")
    }
    fn axis_point(&self, h: f64) -> Point3 {
        self.rf.origin + self.rf.z() * h
    }
    fn span_end(&self, k: usize) -> f64 {
        if k + 1 < self.angles.len() {
            self.angles[k + 1]
        } else {
            self.angles[0] + dmath::TAU
        }
    }
    fn next_copy(&self, k: usize) -> usize {
        (k + 1) % self.angles.len()
    }
    fn spans(&self) -> usize {
        if self.full {
            self.angles.len()
        } else {
            self.angles.len() - 1
        }
    }

    fn surface_of(&self, s: &Segment) -> Result<Option<Surface>, RevolveError> {
        let rf = self.rf;
        match *s {
            Segment::Line { a, b } => {
                let ((h0, r0), (h1, r1)) = (self.hr(a), self.hr(b));
                if r0 <= tol::LINEAR && r1 <= tol::LINEAR {
                    return Ok(None);
                }
                if (h1 - h0).abs() <= tol::LINEAR {
                    return Ok(Some(Surface::Plane(Plane::new(
                        rf.with_origin(self.axis_point(h0)),
                    ))));
                }
                if (r1 - r0).abs() <= tol::LINEAR {
                    return Ok(Some(Surface::Cylinder(Cylinder {
                        frame: rf,
                        radius: 0.5 * (r0 + r1),
                    })));
                }
                let k = (r1 - r0) / (h1 - h0);
                let (frame, radius) = if k > 0.0 {
                    if r0 <= r1 {
                        (rf.with_origin(self.axis_point(h0)), r0)
                    } else {
                        (rf.with_origin(self.axis_point(h1)), r1)
                    }
                } else {
                    let (h, r) = if r0 <= r1 { (h0, r0) } else { (h1, r1) };
                    (rf.with_origin(self.axis_point(h)).flipped(), r)
                };
                let cone = Cone::new(frame, radius.max(0.0), dmath::atan(k.abs()))
                    .ok_or(RevolveError::Unsupported("cone too close to a flat disc"))?;
                Ok(Some(Surface::Cone(cone)))
            }
            Segment::Arc { center, radius, .. } | Segment::Circle { center, radius, .. } => {
                let (hc, rc) = self.hr(center);
                let frame = rf.with_origin(self.axis_point(hc));
                if rc <= tol::LINEAR {
                    return Ok(Some(Surface::Sphere(Sphere { frame, radius })));
                }
                Torus::new(frame, rc, radius)
                    .map(|t| Some(Surface::Torus(t)))
                    .ok_or(RevolveError::Unsupported("arc whose circle reaches the axis"))
            }
        }
    }

    /// The segment's curve in copy k, its range, start/end profile vertex order.
    fn copy_curve(&self, s: &Segment, a: f64) -> (Curve, Interval) {
        match *s {
            Segment::Line { a: p, b: q } => {
                let (p3, q3) = (self.rotate(self.point3(p), a), self.rotate(self.point3(q), a));
                (
                    Curve::Line(Line::through(p3, q3).expect("profile rejects degenerate lines")),
                    Interval::new(0.0, p3.distance(q3)),
                )
            }
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let frame = self.rotate_frame(&self.profile.plane.with_origin(self.point3(center)), a);
                (
                    Curve::Circle(Circle { frame, radius }),
                    Interval::new(start.min(start + sweep), start.max(start + sweep)),
                )
            }
            Segment::Circle { center, radius, .. } => {
                let frame = self.rotate_frame(&self.profile.plane.with_origin(self.point3(center)), a);
                (
                    Curve::Circle(Circle { frame, radius }),
                    Interval::new(0.0, dmath::TAU),
                )
            }
        }
    }

    fn build(mut self) -> Result<Body, RevolveError> {
        let shell = self.body.add_shell();
        let copies = self.angles.len();

        // profile vertices and segments, loop by loop
        let mut pverts: Vec<Point2> = Vec::new();
        let mut pvert_name: Vec<u64> = Vec::new();
        let mut segs: Vec<SegInfo> = Vec::new();
        let mut loop_ranges: Vec<std::ops::Range<usize>> = Vec::new();
        for l in &self.profile.loops {
            let first_v = pverts.len();
            let first_s = segs.len();
            let n = l.segments.len();
            for (i, &(seg, id)) in l.segments.iter().enumerate() {
                pverts.push(seg.start());
                pvert_name.push(id);
                let forward = match seg {
                    Segment::Line { .. } => true,
                    Segment::Arc { sweep, .. } => sweep > 0.0,
                    Segment::Circle { ccw, .. } => ccw,
                };
                segs.push(SegInfo {
                    seg,
                    id,
                    start: first_v + i,
                    end: first_v + (i + 1) % n,
                    forward,
                });
            }
            loop_ranges.push(first_s..segs.len());
        }

        // vertices per copy; a vertex on the axis is one vertex for every copy
        let mut vid: Vec<Vec<VertexId>> = vec![Vec::new(); copies];
        // on a whole turn only a cone's seam ends on the axis (at its apex)
        let mut seam_end = vec![false; pverts.len()];
        if copies == 1 {
            for s in &segs {
                if let Ok(Some(Surface::Cone(_))) = self.surface_of(&s.seg) {
                    seam_end[s.start] = true;
                    seam_end[s.end] = true;
                }
            }
        }
        for (i, &p) in pverts.iter().enumerate() {
            if self.on_axis(p) && copies == 1 && !seam_end[i] {
                // a whole-turn disc has no seam to end there: nothing uses the vertex
                vid[0].push(VertexId(u32::MAX));
            } else if self.on_axis(p) {
                let v = self
                    .body
                    .add_vertex(self.point3(p), self.root.with_str("pole").with(pvert_name[i]));
                for c in vid.iter_mut() {
                    c.push(v);
                }
            } else {
                for (k, c) in vid.iter_mut().enumerate() {
                    let name = self.root.with_str("vertex").with(k as u64).with(pvert_name[i]);
                    c.push(
                        self.body
                            .add_vertex(self.rotate(self.point3(p), self.angles[k]), name),
                    );
                }
            }
        }

        // copy edges; a segment on the axis keeps a single edge, used only by the caps
        let mut eid: Vec<Vec<Option<EdgeId>>> = vec![Vec::new(); copies];
        let mut surfaces = Vec::with_capacity(segs.len());
        for s in &segs {
            let surface = self.surface_of(&s.seg)?;
            let mut shared: Option<EdgeId> = None;
            // a whole-turn disc or ring has no seam: just its circles
            let seamless = copies == 1 && matches!(surface, Some(Surface::Plane(_)));
            for k in 0..copies {
                if (surface.is_none() && self.full) || seamless {
                    eid[k].push(None);
                    continue;
                }
                if let Some(e) = shared {
                    eid[k].push(Some(e));
                    continue;
                }
                let (curve, range) = self.copy_curve(&s.seg, self.angles[k]);
                let (v0, v1) = (vid[k][s.start], vid[k][s.end]);
                let (start, end) = if s.forward { (v0, v1) } else { (v1, v0) };
                let name = self.root.with_str("copy").with(k as u64).with(s.id);
                let e = self.body.add_edge(curve, range, start, end, name);
                if surface.is_none() {
                    shared = Some(e);
                }
                eid[k].push(Some(e));
            }
            surfaces.push(surface);
        }

        // sweep edges: each vertex off the axis traces an arc per span
        let mut sid: Vec<Vec<Option<EdgeId>>> = vec![Vec::new(); self.spans()];
        for (i, &p) in pverts.iter().enumerate() {
            for (k, row) in sid.iter_mut().enumerate() {
                if self.on_axis(p) {
                    row.push(None);
                    continue;
                }
                let (h, r) = self.hr(p);
                let centre = self.axis_point(h);
                let frame = Frame::from_zx(centre, self.rf.z(), self.point3(p) - centre)
                    .expect("vertex off the axis");
                let range = Interval::new(self.angles[k], self.span_end(k));
                let name = self.root.with_str("sweep").with(k as u64).with(pvert_name[i]);
                let (v0, v1) = (vid[k][i], vid[self.next_copy(k)][i]);
                row.push(Some(self.body.add_edge(
                    Curve::Circle(Circle { frame, radius: r }),
                    range,
                    v0,
                    v1,
                    name,
                )));
            }
        }

        for (si, s) in segs.iter().enumerate() {
            let Some(surface) = surfaces[si].clone() else {
                continue;
            };
            for k in 0..self.spans() {
                let k1 = self.next_copy(k);
                let (same_sense, flip) = self.orientation(&s.seg, &surface, self.angles[k], self.span_end(k));
                let name = self.root.with_str("side").with(s.id).with(k as u64);
                let face = self.body.add_face(shell, surface.clone(), same_sense, name);
                let Some(first) = eid[k][si] else {
                    // a whole-turn disc or ring: each circle a loop of its own, the outer first
                    let mut rims = [
                        (sid[k][s.end], true, pverts[s.end]),
                        (sid[k][s.start], false, pverts[s.start]),
                    ];
                    rims.sort_by(|a, b| self.hr(b.2).1.total_cmp(&self.hr(a.2).1));
                    for (e, fwd, _) in rims {
                        if let Some(e) = e {
                            self.body.add_loop(face, &[(e, fwd != flip)]);
                        }
                    }
                    continue;
                };
                let mut uses: Vec<(EdgeId, bool)> = vec![(first, s.forward)];
                if let Some(e) = sid[k][s.end] {
                    uses.push((e, true));
                }
                uses.push((eid[k1][si].expect("segment has a face"), !s.forward));
                if let Some(e) = sid[k][s.start] {
                    uses.push((e, false));
                }
                if flip {
                    uses = uses.into_iter().rev().map(|(e, f)| (e, !f)).collect();
                }
                self.body.add_loop(face, &uses);
            }
        }

        if !self.full {
            self.add_cap(shell, 0, &segs, &eid, &loop_ranges);
            self.add_cap(shell, 1, &segs, &eid, &loop_ranges);
        }
        if copies == 1 {
            // what the second half-turn was named finds the whole
            let root = self.root;
            for s in &segs {
                let side = root.with_str("side").with(s.id);
                let copy = |k: u64| root.with_str("copy").with(k).with(s.id);
                let sweep = |k: u64| root.with_str("sweep").with(k).with(s.id);
                self.body.joins.aliases.extend([
                    (side.with(1), side.with(0)),
                    (copy(1), copy(0)),
                    (sweep(1), sweep(0)),
                ]);
            }
        }
        Ok(self.body)
    }

    /// The face sense, and whether the loop as built runs the wrong way round.
    fn orientation(&self, seg: &Segment, surface: &Surface, a0: f64, a1: f64) -> (bool, bool) {
        let samples = seg.samples(2);
        let m = match *seg {
            Segment::Line { a, b } => a.lerp(b, 0.5),
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => crate::profile::arc_point(center, radius, start + 0.5 * sweep),
            Segment::Circle { .. } => samples[1],
        };
        let t2 = match *seg {
            Segment::Line { a, b } => b - a,
            Segment::Arc { center, sweep, .. } => (m - center).perp() * sweep.signum(),
            Segment::Circle { center, ccw, .. } => (m - center).perp() * if ccw { 1.0 } else { -1.0 },
        };
        // material is left of every profile loop, so outward is to the right
        let out2 = Vec2::new(t2.y, -t2.x);
        let mid = 0.5 * (a0 + a1);
        let p = self.rotate(self.point3(m), mid);
        let out = self.rotate_vec(self.vec3(out2), mid);
        let same_sense = surface
            .normal(surface.project(p))
            .is_none_or(|n| n.dot(out) > 0.0);
        // at copy a0, walking the profile direction, the face lies towards increasing angle
        let p0 = self.rotate(self.point3(m), a0);
        let into = self.rf.z().cross(p0 - self.rf.origin);
        let n0 = self.rotate_vec(self.vec3(out2), a0);
        let t0 = self.rotate_vec(self.vec3(t2), a0);
        let flip = n0.cross(t0).dot(into) < 0.0;
        (same_sense, flip)
    }

    fn add_cap(
        &mut self,
        shell: ShellId,
        copy: usize,
        segs: &[SegInfo],
        eid: &[Vec<Option<EdgeId>>],
        loop_ranges: &[std::ops::Range<usize>],
    ) -> FaceId {
        let a = self.angles[copy];
        let frame = self.rotate_frame(&self.profile.plane, a);
        // outward at the start cap is against the turn, at the end cap with it
        let turn = self.rf.z().cross(self.rf.x());
        let with_turn = frame.z().dot(self.rotate_vec(turn, a)) > 0.0;
        let same_sense = if copy == 0 { !with_turn } else { with_turn };
        let name = self.root.with_str(if copy == 0 { "start" } else { "end" });
        let face = self
            .body
            .add_face(shell, Surface::Plane(Plane::new(frame)), same_sense, name);
        for r in loop_ranges {
            let mut uses: Vec<(EdgeId, bool)> = r
                .clone()
                .map(|si| (eid[copy][si].expect("caps use every segment"), segs[si].forward))
                .collect();
            if !same_sense {
                uses = uses.into_iter().rev().map(|(e, f)| (e, !f)).collect();
            }
            self.body.add_loop(face, &uses);
        }
        face
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ProfileLoop;
    use forge_math::rng::Rng;
    use forge_topo::{check, genus};

    const Y_AXIS: Axis2 = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };

    fn valid(b: &Body) {
        let problems = check(b);
        assert!(problems.is_empty(), "{problems:?}");
    }

    fn profile(loops: Vec<ProfileLoop>) -> Profile {
        Profile {
            plane: Frame::WORLD,
            loops,
        }
    }

    fn half_disc(r: f64) -> ProfileLoop {
        let q = dmath::FRAC_PI_2;
        ProfileLoop::new(vec![
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
        ])
    }

    #[test]
    fn sphere() {
        let b = revolve(&profile(vec![half_disc(5.0)]), Y_AXIS, dmath::TAU, 1).unwrap();
        valid(&b);
        assert_eq!(genus(&b), 0);
        assert_eq!((b.vertices.len(), b.edges.len(), b.faces.len()), (2, 2, 2));
    }

    #[test]
    fn torus() {
        let b = revolve(
            &profile(vec![crate::Profile::circle(Point2::new(10.0, 0.0), 3.0, 0)]),
            Y_AXIS,
            dmath::TAU,
            1,
        )
        .unwrap();
        valid(&b);
        assert_eq!(genus(&b), 1);
    }

    #[test]
    fn cylinder_and_cone_from_the_axis() {
        let rect = crate::Profile::rectangle(Point2::ORIGIN, 4.0, 10.0, 0);
        let b = revolve(&profile(vec![rect]), Y_AXIS, dmath::TAU, 1).unwrap();
        valid(&b);
        assert_eq!(genus(&b), 0);
        // whole faces with a seam: two discs and a wall; the second half's name finds the wall
        assert_eq!(b.faces.len(), 3);
        let wall = Name::feature(1).with_str("side").with(1);
        assert!(b.face_by_name(wall.with(0)).is_some());
        assert_eq!(b.face_by_name(wall.with(1)), b.face_by_name(wall.with(0)));
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
        let b = revolve(&profile(vec![tri]), Y_AXIS, dmath::TAU, 1).unwrap();
        valid(&b);
    }

    #[test]
    fn partial_turns() {
        let rect = crate::Profile::rectangle(Point2::new(5.0, 0.0), 4.0, 10.0, 0);
        for angle in [0.3, dmath::FRAC_PI_2, 3.0, 4.5, 6.0] {
            let b = revolve(&profile(vec![rect.clone()]), Y_AXIS, angle, 1).unwrap();
            valid(&b);
            assert_eq!(genus(&b), 0);
        }
        let b = revolve(&profile(vec![half_disc(3.0)]), Y_AXIS, 2.0, 1).unwrap();
        valid(&b);
    }

    #[test]
    fn random_rounded_profiles() {
        let mut r = Rng::new(700);
        for _ in 0..80 {
            let x0 = r.range(0.5, 20.0);
            let (w, h) = (r.range(3.0, 20.0), r.range(3.0, 20.0));
            let rad = r.range(0.3, w.min(h) * 0.45);
            let mut loops = vec![crate::Profile::rounded_rectangle(
                Point2::new(x0, r.range(-10.0, 10.0)),
                w,
                h,
                rad,
                0,
            )];
            if r.bool() {
                let c = loops[0].segments[0].0.start();
                loops.push(crate::Profile::circle(
                    Point2::new(x0 + w / 2.0, c.y + h / 2.0),
                    w.min(h) * 0.2,
                    40,
                ));
            }
            let angle = if r.bool() { dmath::TAU } else { r.range(0.2, 6.2) };
            let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
                .normalize()
                .unwrap();
            let plane = Frame::from_z(Point3::new(r.range(-50.0, 50.0), 0.0, r.range(-50.0, 50.0)), z);
            let b = revolve(&Profile { plane, loops }, Y_AXIS, angle, 2).unwrap();
            valid(&b);
        }
    }

    #[test]
    fn rejects_crossing_the_axis() {
        let rect = crate::Profile::rectangle(Point2::new(-2.0, 0.0), 4.0, 10.0, 0);
        assert_eq!(
            revolve(&profile(vec![rect.clone()]), Y_AXIS, dmath::TAU, 1),
            Err(RevolveError::CrossesAxis)
        );
        assert_eq!(
            revolve(&profile(vec![rect]), Y_AXIS, 0.0, 1),
            Err(RevolveError::BadAngle)
        );
    }
}
