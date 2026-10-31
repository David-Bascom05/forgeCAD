use crate::profile::{Profile, ProfileError, Segment};
use forge_geom::{Circle, Curve, CurveGeom, Cylinder, Line, Plane, Surface};
use forge_math::{Frame, Interval, Point2, Point3, UnitVec3, dmath, tol};
use forge_topo::{Body, EdgeId, Name, VertexId};

#[derive(Clone, Debug, PartialEq)]
pub enum ExtrudeError {
    Profile(ProfileError),
    /// The distance is shorter than the tolerance allows.
    TooShort,
}

impl From<ProfileError> for ExtrudeError {
    fn from(e: ProfileError) -> Self {
        ExtrudeError::Profile(e)
    }
}

/// Sweeps a profile along its plane's normal by `distance` (negative goes the
/// other way) into a solid. `feature` is the history id the names derive from:
/// the caps are "start" and "end", each segment's wall is "side" + its id,
/// edges and vertices are named by the faces that meet there.
pub fn extrude(profile: &Profile, distance: f64, feature: u64) -> Result<Body, ExtrudeError> {
    if distance.abs() <= tol::MIN_LENGTH {
        return Err(ExtrudeError::TooShort);
    }
    // Going backwards is going forwards from the flipped plane with the profile mirrored to match.
    let profile = if distance < 0.0 {
        Profile {
            plane: profile.plane.flipped(),
            loops: profile
                .loops
                .iter()
                .map(|l| crate::profile::ProfileLoop {
                    segments: l.segments.iter().map(|&(s, id)| (s.mirrored(), id)).collect(),
                })
                .collect(),
        }
    } else {
        profile.clone()
    };
    let profile = profile.normalized()?;
    Ok(Builder::new(&profile, distance.abs(), Name::feature(feature)).build())
}

struct Builder<'a> {
    profile: &'a Profile,
    d: f64,
    body: Body,
    start_name: Name,
    end_name: Name,
    root: Name,
}

/// The pieces made for one profile segment.
struct Wall {
    bottom: EdgeId,
    top: EdgeId,
    /// True when the profile direction runs with the edge curve's parameter.
    forward: bool,
}

impl<'a> Builder<'a> {
    fn new(profile: &'a Profile, d: f64, root: Name) -> Self {
        Builder {
            profile,
            d,
            body: Body::new(),
            start_name: root.with_str("start"),
            end_name: root.with_str("end"),
            root,
        }
    }

    fn frame(&self) -> &Frame {
        &self.profile.plane
    }
    fn up(&self) -> UnitVec3 {
        self.frame().z()
    }
    fn at(&self, p: Point2, level: f64) -> Point3 {
        self.frame().plane_point(p) + self.up() * level
    }
    fn side_name(&self, id: u64) -> Name {
        self.root.with_str("side").with(id)
    }

    fn build(mut self) -> Body {
        let shell = self.body.add_shell();
        let bottom_plane = Plane::new(*self.frame());
        let top_plane = Plane::new(self.frame().with_origin(self.at(Point2::ORIGIN, self.d)));
        let bottom = self
            .body
            .add_face(shell, Surface::Plane(bottom_plane), false, self.start_name);
        let top = self
            .body
            .add_face(shell, Surface::Plane(top_plane), true, self.end_name);

        for li in 0..self.profile.loops.len() {
            let segs = self.profile.loops[li].segments.clone();
            let walls = if let [(Segment::Circle { center, radius, ccw }, id)] = segs[..] {
                vec![self.circle_wall(shell, center, radius, ccw, id)]
            } else {
                self.chain_walls(shell, &segs)
            };
            let top_uses: Vec<(EdgeId, bool)> = walls.iter().map(|w| (w.top, w.forward)).collect();
            let bottom_uses: Vec<(EdgeId, bool)> =
                walls.iter().rev().map(|w| (w.bottom, !w.forward)).collect();
            self.body.add_loop(top, &top_uses);
            self.body.add_loop(bottom, &bottom_uses);
        }
        self.body
    }

    fn cap_name(&self, top: bool) -> Name {
        if top { self.end_name } else { self.start_name }
    }

    /// Walls for a loop of lines and arcs: one vertical edge at each joint.
    fn chain_walls(&mut self, shell: forge_topo::ShellId, segs: &[(Segment, u64)]) -> Vec<Wall> {
        let n = segs.len();
        let names: Vec<Name> = segs.iter().map(|&(_, id)| self.side_name(id)).collect();
        // vertex i is where segment i starts (between wall i-1 and wall i)
        let mut verts: Vec<[VertexId; 2]> = Vec::with_capacity(n);
        for (i, &(s, _)) in segs.iter().enumerate() {
            let prev = names[(i + n - 1) % n];
            let mut pair = [VertexId(0); 2];
            for (k, level) in [0.0, self.d].into_iter().enumerate() {
                let name = Name::NONE.with_set(&mut [self.cap_name(k == 1), prev, names[i]]);
                pair[k] = self.body.add_vertex(self.at(s.start(), level), name);
            }
            verts.push(pair);
        }
        let mut uprights: Vec<EdgeId> = Vec::with_capacity(n);
        for (i, &(s, _)) in segs.iter().enumerate() {
            let prev = names[(i + n - 1) % n];
            let line = Line::new(self.at(s.start(), 0.0), self.up());
            let name = Name::NONE.with_set(&mut [prev, names[i]]);
            uprights.push(self.body.add_edge(
                Curve::Line(line),
                Interval::new(0.0, self.d),
                verts[i][0],
                verts[i][1],
                name,
            ));
        }
        let mut walls = Vec::with_capacity(n);
        for (i, &(s, _)) in segs.iter().enumerate() {
            let j = (i + 1) % n;
            let (surface, same_sense, curve0, range, forward) = self.segment_geometry(s);
            let mut edges = [EdgeId(0); 2];
            for (k, level) in [0.0, self.d].into_iter().enumerate() {
                let (a, b) = (verts[i][k], verts[j][k]);
                let (start, end) = if forward { (a, b) } else { (b, a) };
                let curve = curve0.transformed(&Frame::WORLD.with_origin(Point3::ORIGIN + self.up() * level));
                let name = Name::NONE.with_set(&mut [self.cap_name(k == 1), names[i]]);
                edges[k] = self.body.add_edge(curve, range, start, end, name);
            }
            let face = self.body.add_face(shell, surface, same_sense, names[i]);
            self.body.add_loop(
                face,
                &[
                    (edges[0], forward),
                    (uprights[j], true),
                    (edges[1], !forward),
                    (uprights[i], false),
                ],
            );
            walls.push(Wall {
                bottom: edges[0],
                top: edges[1],
                forward,
            });
        }
        walls
    }

    /// The wall surface of a segment, its curve at the bottom, the curve's
    /// parameter range, and whether the profile runs with that parameter.
    fn segment_geometry(&self, s: Segment) -> (Surface, bool, Curve, Interval, bool) {
        let f = *self.frame();
        match s {
            Segment::Line { a, b } => {
                let (pa, pb) = (self.at(a, 0.0), self.at(b, 0.0));
                let line = Line::through(pa, pb).expect("profile rejects degenerate lines");
                let outward = line
                    .dir
                    .cross(self.up())
                    .normalize()
                    .expect("line lies in the plane");
                let wall = Frame::from_zx(pa, outward, line.dir.get()).expect("perpendicular axes");
                (
                    Surface::Plane(Plane::new(wall)),
                    true,
                    Curve::Line(line),
                    Interval::new(0.0, pa.distance(pb)),
                    true,
                )
            }
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => {
                let axis = f.with_origin(self.at(center, 0.0));
                let circle = Circle { frame: axis, radius };
                let range = Interval::new(start.min(start + sweep), start.max(start + sweep));
                let ccw = sweep > 0.0;
                (
                    Surface::Cylinder(Cylinder { frame: axis, radius }),
                    ccw,
                    Curve::Circle(circle),
                    range,
                    ccw,
                )
            }
            Segment::Circle { .. } => unreachable!("circles take circle_wall"),
        }
    }

    /// A whole circle: one vertex at angle 0 on each cap, a closed edge on
    /// each cap, and a seam edge the wall uses twice.
    fn circle_wall(
        &mut self,
        shell: forge_topo::ShellId,
        center: Point2,
        radius: f64,
        ccw: bool,
        id: u64,
    ) -> Wall {
        let name = self.side_name(id);
        let axis = self.frame().with_origin(self.at(center, 0.0));
        let start = Point2::new(center.x + radius, center.y);
        let vb = self
            .body
            .add_vertex(self.at(start, 0.0), name.with_str("seam-start"));
        let vt = self
            .body
            .add_vertex(self.at(start, self.d), name.with_str("seam-end"));
        let full = Interval::new(0.0, dmath::TAU);
        let bottom_c = Curve::Circle(Circle { frame: axis, radius });
        let top_c = Curve::Circle(Circle {
            frame: axis.with_origin(self.at(center, self.d)),
            radius,
        });
        let bottom = self.body.add_edge(
            bottom_c,
            full,
            vb,
            vb,
            Name::NONE.with_set(&mut [self.start_name, name]),
        );
        let top = self.body.add_edge(
            top_c,
            full,
            vt,
            vt,
            Name::NONE.with_set(&mut [self.end_name, name]),
        );
        let seam = self.body.add_edge(
            Curve::Line(Line::new(self.at(start, 0.0), self.up())),
            Interval::new(0.0, self.d),
            vb,
            vt,
            name.with_str("seam"),
        );
        let face = self.body.add_face(
            shell,
            Surface::Cylinder(Cylinder { frame: axis, radius }),
            ccw,
            name,
        );
        self.body
            .add_loop(face, &[(bottom, ccw), (seam, true), (top, !ccw), (seam, false)]);
        Wall {
            bottom,
            top,
            forward: ccw,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ProfileLoop;
    use forge_math::Vec3;
    use forge_math::rng::Rng;
    use forge_topo::{Problem, check, genus};

    fn profile(loops: Vec<ProfileLoop>) -> Profile {
        Profile {
            plane: Frame::WORLD,
            loops,
        }
    }

    fn valid(b: &Body) {
        let problems = check(b);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn box_is_valid() {
        let b = extrude(
            &profile(vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)]),
            5.0,
            1,
        )
        .unwrap();
        valid(&b);
        assert_eq!((b.vertices.len(), b.edges.len(), b.faces.len()), (8, 12, 6));
        assert_eq!(genus(&b), 0);
    }

    #[test]
    fn plate_with_hole_has_genus_one() {
        let p = profile(vec![
            Profile::rectangle(Point2::ORIGIN, 30.0, 20.0, 0),
            Profile::circle(Point2::new(15.0, 10.0), 4.0, 10),
        ]);
        let b = extrude(&p, 6.0, 1).unwrap();
        valid(&b);
        assert_eq!(genus(&b), 1);
        assert_eq!(b.faces.len(), 7);
    }

    #[test]
    fn rounded_and_slotted() {
        let outer = Profile::rounded_rectangle(Point2::new(-20.0, -10.0), 40.0, 20.0, 5.0, 0);
        let slot = Profile::rounded_rectangle(Point2::new(-10.0, -2.0), 20.0, 4.0, 2.0 - 1e-3, 100);
        let p = profile(vec![
            outer,
            slot,
            Profile::circle(Point2::new(15.0, 0.0), 1.5, 200),
        ]);
        let b = extrude(&p, 3.0, 7).unwrap();
        valid(&b);
        assert_eq!(genus(&b), 2);
    }

    #[test]
    fn negative_distance_and_tilted_planes() {
        let mut r = Rng::new(500);
        for _ in 0..200 {
            let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
                .normalize()
                .unwrap();
            let o = Point3::new(
                r.range(-100.0, 100.0),
                r.range(-100.0, 100.0),
                r.range(-100.0, 100.0),
            );
            let plane = Frame::from_z(o, z);
            let w = r.range(5.0, 50.0);
            let h = r.range(5.0, 50.0);
            let rad = r.range(0.5, w.min(h) * 0.45);
            let mut loops = vec![Profile::rounded_rectangle(Point2::ORIGIN, w, h, rad, 0)];
            if r.bool() {
                loops.push(Profile::circle(Point2::new(w / 2.0, h / 2.0), w.min(h) * 0.2, 50));
            }
            let d = r.range(0.5, 30.0) * if r.bool() { -1.0 } else { 1.0 };
            let b = extrude(&Profile { plane, loops }, d, 3).unwrap();
            valid(&b);
        }
    }

    #[test]
    fn names_are_stable_and_meaningful() {
        let p = profile(vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)]);
        let a = extrude(&p, 5.0, 4).unwrap();
        let b = extrude(&p, 8.0, 4).unwrap();
        let names = |x: &Body| x.faces.iter().map(|f| f.name).collect::<Vec<_>>();
        assert_eq!(
            names(&a),
            names(&b),
            "same history, same names, whatever the dimensions"
        );
        let top = a.face_by_name(Name::feature(4).with_str("end")).unwrap();
        assert!(a.face(top).same_sense);
        let c = extrude(&p, 5.0, 5).unwrap();
        assert_ne!(names(&a), names(&c));
    }

    #[test]
    fn checker_catches_mistakes() {
        let good = extrude(
            &profile(vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)]),
            5.0,
            1,
        )
        .unwrap();
        let mut flipped = good.clone();
        flipped.faces[0].same_sense = !flipped.faces[0].same_sense;
        assert!(
            check(&flipped)
                .iter()
                .any(|p| matches!(p, Problem::FaceWrongSide { .. }))
        );

        let mut broken = good.clone();
        let l = broken.faces[2].loops[0];
        let (c0, c1) = (broken.lp(l).coedge(0).index(), broken.lp(l).coedge(1).index());
        let e0 = (broken.coedges[c0].edge, broken.coedges[c0].forward);
        (broken.coedges[c0].edge, broken.coedges[c0].forward) =
            (broken.coedges[c1].edge, broken.coedges[c1].forward);
        (broken.coedges[c1].edge, broken.coedges[c1].forward) = e0;
        assert!(
            check(&broken)
                .iter()
                .any(|p| matches!(p, Problem::LoopBroken { .. }))
        );

        let mut moved = good.clone();
        moved.vertices[0].point.x += 0.01;
        assert!(
            check(&moved)
                .iter()
                .any(|p| matches!(p, Problem::VertexOffEdge { .. }))
        );

        let mut renamed = good;
        renamed.faces[1].name = renamed.faces[0].name;
        assert!(
            check(&renamed)
                .iter()
                .any(|p| matches!(p, Problem::DuplicateName { .. }))
        );
    }

    #[test]
    fn rejects_bad_input() {
        let p = profile(vec![Profile::rectangle(Point2::ORIGIN, 10.0, 20.0, 0)]);
        assert_eq!(extrude(&p, 0.0, 1), Err(ExtrudeError::TooShort));
        let open = profile(vec![ProfileLoop::new(vec![(
            Segment::Line {
                a: Point2::ORIGIN,
                b: Point2::new(1.0, 0.0),
            },
            0,
        )])]);
        assert!(matches!(
            extrude(&open, 1.0, 1),
            Err(ExtrudeError::Profile(ProfileError::Open { .. }))
        ));
    }
}
