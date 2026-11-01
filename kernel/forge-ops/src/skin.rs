//! The solid a profile makes passing through a run of sections: a cap at each end, and a wall
//! between each pair of neighbouring sections for each profile segment. Sweeps, coils and lofts
//! work out the geometry; this puts the faces, edges and vertices together and names them.
//!
//! Names: the caps are "start" and "end"; the wall of segment `key` between sections k and
//! k + 1 is "side" key k; section s's copy of a segment is "section" s key, of the vertex where
//! a segment starts "vertex" s key; that vertex's path from section k to k + 1 is "rail" k key.

use crate::profile::{Profile, ProfileError, ProfileLoop, Segment};
use forge_geom::{Circle, Curve, CurveGeom, Line, Plane, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point2, Vec3, dmath};
use forge_topo::{Body, EdgeId, Name, VertexId};

/// One profile segment's curve in one section, its parameter range, and whether the profile runs
/// with the curve's parameter.
pub(crate) struct SecCurve {
    pub curve: Curve,
    pub range: Interval,
    pub forward: bool,
}

/// A profile ready to skin: z along the motion, normalized, whole circles in two halves.
pub struct Prepared {
    pub profile: Profile,
    /// Each segment's name key, through the loops in order.
    pub keys: Vec<Name>,
}

impl Prepared {
    pub fn segments(&self) -> impl Iterator<Item = &Segment> {
        self.profile
            .loops
            .iter()
            .flat_map(|l| l.segments.iter().map(|(s, _)| s))
    }
    pub fn loop_sizes(&self) -> Vec<usize> {
        self.profile.loops.iter().map(|l| l.segments.len()).collect()
    }
}

/// The profile turned so its plane's z faces `forward` (flipped and mirrored to match), checked,
/// with every whole circle split into two half arcs so each segment has two ends.
/// A profile with a free-form curve in it: the first such segment's id (for what takes only lines and arcs).
pub(crate) fn curved(p: &Profile) -> Result<(), ProfileError> {
    let c = p
        .loops
        .iter()
        .flat_map(|l| &l.segments)
        .find(|(s, _)| matches!(s, Segment::Spline(_)));
    c.map_or(Ok(()), |(_, id)| Err(ProfileError::Curve { id: *id }))
}

pub(crate) fn prepare(profile: &Profile, forward: Vec3) -> Result<Prepared, ProfileError> {
    let turned = if profile.plane.z().dot(forward) < 0.0 {
        mirrored(profile)
    } else {
        profile.clone()
    };
    let p = turned.normalized()?;
    let mut keys = Vec::new();
    let loops = p
        .loops
        .iter()
        .map(|l| {
            let mut segs = Vec::new();
            for (s, id) in &l.segments {
                let id = *id;
                if let Segment::Circle { center, radius, ccw } = *s {
                    let h = if ccw { dmath::PI } else { -dmath::PI };
                    for (half, start) in [(0, 0.0), (1, h)] {
                        segs.push((
                            Segment::Arc {
                                center,
                                radius,
                                start,
                                sweep: h,
                            },
                            id,
                        ));
                        keys.push(Name::NONE.with(id).with(half));
                    }
                } else {
                    segs.push((s.clone(), id));
                    keys.push(Name::NONE.with(id).with(0));
                }
            }
            ProfileLoop::new(segs)
        })
        .collect();
    Ok(Prepared {
        profile: Profile {
            plane: p.plane,
            loops,
        },
        keys,
    })
}

/// The same region seen from the plane's other side.
pub(crate) fn mirrored(profile: &Profile) -> Profile {
    Profile {
        plane: profile.plane.flipped(),
        loops: profile
            .loops
            .iter()
            .map(|l| ProfileLoop::new(l.segments.iter().map(|(s, id)| (s.mirrored(), *id)).collect()))
            .collect(),
    }
}

/// A segment drawn in the plane `f`.
pub(crate) fn section_curve(s: &Segment, f: &Frame) -> SecCurve {
    match *s {
        Segment::Spline(ref c) => {
            let world = c.c.mapped(|q| f.point_to_world(q));
            let d = c.c.domain();
            SecCurve {
                curve: forge_geom::spline::SplineCurve::curve(std::sync::Arc::new(world), d),
                range: d,
                forward: true,
            }
        }
        Segment::Line { a, b } => {
            let (pa, pb) = (f.plane_point(a), f.plane_point(b));
            SecCurve {
                curve: Curve::Line(Line::through(pa, pb).expect("profile rejects degenerate lines")),
                range: Interval::new(0.0, pa.distance(pb)),
                forward: true,
            }
        }
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => SecCurve {
            curve: Curve::Circle(Circle {
                frame: f.with_origin(f.plane_point(center)),
                radius,
            }),
            range: Interval::new(start.min(start + sweep), start.max(start + sweep)),
            forward: sweep > 0.0,
        },
        Segment::Circle { .. } => unreachable!("prepare splits circles"),
    }
}

/// What a sweep, coil or loft works out: sections 0..=n, and between each neighbouring pair a
/// rail per profile vertex and a wall per profile segment.
pub(crate) struct Skin {
    pub keys: Vec<Name>,
    /// Segments per profile loop; vertex i is where segment i starts.
    pub loops: Vec<usize>,
    /// Each section's plane (z along the motion) and its segment curves.
    pub planes: Vec<Frame>,
    pub curves: Vec<Vec<SecCurve>>,
    /// rails[k][i]: vertex i's path from section k to k + 1, its parameter increasing that way.
    pub rails: Vec<Vec<(Curve, Interval)>>,
    pub walls: Vec<Vec<Surface>>,
}

impl Skin {
    pub fn build(self, root: Name) -> Body {
        let mut body = Body::new();
        let shell = body.add_shell();
        let n = self.planes.len();
        let nseg = self.keys.len();
        // segment i ends where the next in its loop starts
        let mut next = vec![0; nseg];
        let mut first = 0;
        for &len in &self.loops {
            for i in 0..len {
                next[first + i] = first + (i + 1) % len;
            }
            first += len;
        }
        let start_point = |c: &SecCurve| {
            let t = if c.forward { c.range.lo } else { c.range.hi };
            c.curve.eval(t)
        };
        let verts: Vec<Vec<VertexId>> = (0..n)
            .map(|s| {
                (0..nseg)
                    .map(|i| {
                        let name = root.with_str("vertex").with(s as u64).with_name(self.keys[i]);
                        body.add_vertex(start_point(&self.curves[s][i]), name)
                    })
                    .collect()
            })
            .collect();
        let edges: Vec<Vec<EdgeId>> = (0..n)
            .map(|s| {
                (0..nseg)
                    .map(|i| {
                        let c = &self.curves[s][i];
                        let (a, b) = (verts[s][i], verts[s][next[i]]);
                        let (start, end) = if c.forward { (a, b) } else { (b, a) };
                        let name = root.with_str("section").with(s as u64).with_name(self.keys[i]);
                        body.add_edge(c.curve.clone(), c.range, start, end, name)
                    })
                    .collect()
            })
            .collect();
        for k in 0..n - 1 {
            let rails: Vec<EdgeId> = (0..nseg)
                .map(|i| {
                    let (curve, range) = self.rails[k][i].clone();
                    let name = root.with_str("rail").with(k as u64).with_name(self.keys[i]);
                    body.add_edge(curve, range, verts[k][i], verts[k + 1][i], name)
                })
                .collect();
            for (j, surface) in self.walls[k].iter().enumerate() {
                // the wall's outward normal is the profile's direction crossed with the motion
                let c = &self.curves[k][j];
                let mid = c.curve.deriv(c.range.mid());
                let along = if c.forward { mid.d1 } else { -mid.d1 };
                let (rc, rr) = &self.rails[k][j];
                let outward = along.cross(rc.deriv(rr.lo).d1);
                let same = surface
                    .normal(surface.project(mid.p))
                    .is_none_or(|nat| nat.dot(outward) > 0.0);
                let name = root.with_str("side").with_name(self.keys[j]).with(k as u64);
                let face = body.add_face(shell, surface.clone(), same, name);
                let fw = c.forward;
                body.add_loop(
                    face,
                    &[
                        (edges[k][j], fw),
                        (rails[next[j]], true),
                        (edges[k + 1][j], !self.curves[k + 1][j].forward),
                        (rails[j], false),
                    ],
                );
            }
        }
        for (s, same, name) in [(0, false, "start"), (n - 1, true, "end")] {
            let face = body.add_face(
                shell,
                Surface::Plane(Plane::new(self.planes[s])),
                same,
                root.with_str(name),
            );
            let mut first = 0;
            for &len in &self.loops {
                let ring = first..first + len;
                let uses: Vec<(EdgeId, bool)> = if same {
                    ring.map(|i| (edges[s][i], self.curves[s][i].forward)).collect()
                } else {
                    ring.rev()
                        .map(|i| (edges[s][i], !self.curves[s][i].forward))
                        .collect()
                };
                body.add_loop(face, &uses);
                first += len;
            }
        }
        body
    }
}

/// (r, h) of a profile point drawn in `plane`, about the axis frame `rf` (z the axis, x towards
/// the profile, in the plane).
pub(crate) fn radius_height(rf: &Frame, plane: &Frame, p: Point2) -> Point2 {
    let l = rf.point_to_local(plane.plane_point(p));
    Point2::new(l.x, l.z)
}

/// A segment drawn in `plane`, in (r, h) about the axis frame `rf`, whose x lies in the plane.
pub(crate) fn rh_section(rf: &Frame, plane: &Frame, s: &Segment) -> forge_geom::screw::Section {
    use forge_geom::screw::Section;
    match *s {
        Segment::Spline(_) => unreachable!("callers refuse curves"),
        Segment::Line { a, b } => Section::Line {
            a: radius_height(rf, plane, a),
            b: radius_height(rf, plane, b),
        },
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => {
            let dir = |t: f64| {
                let (sn, cs) = dmath::sin_cos(t);
                let v = rf.vec_to_local(plane.x() * cs + plane.y() * sn);
                Point2::new(v.x, v.z)
            };
            let (d0, d1) = (dir(0.0), dir(dmath::FRAC_PI_2));
            // the plane's axes seen in (r, h): a turn or a mirror image
            let det = d0.x * d1.y - d0.y * d1.x;
            let s0 = dir(start);
            Section::Arc {
                c: radius_height(rf, plane, center),
                r: radius,
                start: dmath::atan2(s0.y, s0.x),
                sweep: sweep * det.signum(),
            }
        }
        Segment::Circle { .. } => unreachable!("prepare splits circles"),
    }
}
