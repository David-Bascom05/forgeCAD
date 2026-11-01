//! A whole history moved, mirrored or scaled: every frame, point, direction and length of every
//! feature mapped, so the part it builds should be the old one mapped.

use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3};
use forge_model::{End, Extent, Feature, Head, Layout, Part, Turn};
use forge_ops::{Axis2, ChamferSize, Profile, ProfileLoop, Radius, Segment};

#[derive(Clone, Copy, Debug)]
pub enum Map {
    /// Local coordinates of the frame to world.
    Rigid(Frame),
    /// Reflection in the plane n.p = d.
    Mirror(UnitVec3, f64),
    /// Scaling about the world origin.
    Scale(f64),
}

impl Map {
    pub fn point(&self, p: Point3) -> Point3 {
        match *self {
            Map::Rigid(f) => f.point_to_world(p),
            Map::Mirror(n, d) => p - n * (2.0 * (n.dot(p.to_vec()) - d)),
            Map::Scale(k) => Point3::new(p.x * k, p.y * k, p.z * k),
        }
    }

    pub fn vec(&self, v: Vec3) -> Vec3 {
        match *self {
            Map::Rigid(f) => f.vec_to_world(v),
            Map::Mirror(n, _) => v - n * (2.0 * n.dot(v)),
            Map::Scale(_) => v,
        }
    }

    fn len(&self, d: f64) -> f64 {
        match *self {
            Map::Scale(k) => d * k,
            _ => d,
        }
    }

    fn mirrors(&self) -> bool {
        matches!(self, Map::Mirror(..))
    }

    /// The image of a frame, right-handed: a mirror's turns sketch y back (the profile is
    /// reflected across its x to match), so x and z are the images of x and z.
    pub fn frame(&self, f: &Frame) -> Frame {
        match *self {
            Map::Rigid(m) => m.compose(f),
            _ => {
                let (x, y, z) = (
                    self.vec(f.x().get()),
                    self.vec(f.y().get()),
                    self.vec(f.z().get()),
                );
                let y = if self.mirrors() { -y } else { y };
                let o = self.point(f.origin);
                Frame::from_axes(o, x, y, z)
                    .or_else(|| {
                        let zu = z.normalize()?;
                        Frame::from_zx(o, zu, x)
                    })
                    .expect("a mapped frame stays orthonormal")
            }
        }
    }

    fn p2(&self, p: Point2) -> Point2 {
        let y = if self.mirrors() { -p.y } else { p.y };
        Point2::new(self.len(p.x), self.len(y))
    }

    fn v2(&self, v: Vec2) -> Vec2 {
        if self.mirrors() { Vec2::new(v.x, -v.y) } else { v }
    }

    fn angle(&self, a: f64) -> f64 {
        if self.mirrors() { -a } else { a }
    }

    fn profile(&self, p: &Profile) -> Profile {
        let loops = p
            .loops
            .iter()
            .map(|l| {
                let segs = l
                    .segments
                    .iter()
                    .map(|&(s, id)| {
                        let s = match s {
                            Segment::Line { a, b } => Segment::Line {
                                a: self.p2(a),
                                b: self.p2(b),
                            },
                            Segment::Arc {
                                center,
                                radius,
                                start,
                                sweep,
                            } => Segment::Arc {
                                center: self.p2(center),
                                radius: self.len(radius),
                                start: self.angle(start),
                                sweep: self.angle(sweep),
                            },
                            Segment::Circle { center, radius, ccw } => Segment::Circle {
                                center: self.p2(center),
                                radius: self.len(radius),
                                ccw: ccw != self.mirrors(),
                            },
                        };
                        (s, id)
                    })
                    .collect();
                // a reflected loop winds the other way: profiles orient their loops themselves
                ProfileLoop::new(segs)
            })
            .collect();
        Profile {
            plane: self.frame(&p.plane),
            loops,
        }
    }

    fn radius(&self, r: &Radius) -> Radius {
        match r {
            Radius::Constant(x) => Radius::Constant(self.len(*x)),
            Radius::Linear(a, b) => Radius::Linear(self.len(*a), self.len(*b)),
            Radius::Points(p) => Radius::Points(p.iter().map(|&(f, x)| (f, self.len(x))).collect()),
        }
    }

    fn feature(&self, f: &Feature) -> Option<Feature> {
        Some(match f {
            Feature::Extrude { profile, extent, op } => Feature::Extrude {
                profile: self.profile(profile),
                extent: match *extent {
                    Extent::Distance(d) => Extent::Distance(self.len(d)),
                    Extent::Along { forward, back, dir } => Extent::Along {
                        forward: self.len(forward),
                        back: self.len(back),
                        dir: self.vec(dir),
                    },
                    Extent::Tapered { distance, angle } => Extent::Tapered {
                        distance: self.len(distance),
                        angle,
                    },
                    Extent::Symmetric(d) => Extent::Symmetric(self.len(d)),
                    Extent::TwoSided { forward, back } => Extent::TwoSided {
                        forward: self.len(forward),
                        back: self.len(back),
                    },
                    Extent::Through(t) => Extent::Through(t),
                    Extent::ToNext { reverse } => Extent::ToNext { reverse },
                    Extent::To { .. } => return None,
                },
                op: *op,
            },
            Feature::Revolve {
                profile,
                axis,
                extent,
                op,
            } => Feature::Revolve {
                profile: self.profile(profile),
                axis: Axis2 {
                    origin: self.p2(axis.origin),
                    dir: self.v2(axis.dir),
                },
                // a mirrored part turn would run the other way round its axis
                extent: match *extent {
                    Turn::Angle(a) if !self.mirrors() || a >= forge_math::dmath::TAU => Turn::Angle(a),
                    Turn::Symmetric(a) => Turn::Symmetric(a),
                    _ => return None,
                },
                op: *op,
            },
            Feature::Ball { frame, radius, op } => Feature::Ball {
                frame: self.frame(frame),
                radius: self.len(*radius),
                op: *op,
            },
            Feature::Fillet {
                edges,
                starts,
                setbacks,
            } => Feature::Fillet {
                edges: edges.iter().map(|(n, r)| (*n, self.radius(r))).collect(),
                starts: starts.clone(),
                setbacks: setbacks.iter().map(|&(v, e, d)| (v, e, self.len(d))).collect(),
            },
            Feature::FaceFillet {
                first,
                second,
                radius,
            } => Feature::FaceFillet {
                first: first.clone(),
                second: second.clone(),
                radius: self.len(*radius),
            },
            Feature::Chamfer { edges, size } => Feature::Chamfer {
                edges: edges.clone(),
                size: match *size {
                    ChamferSize::Equal(d) => ChamferSize::Equal(self.len(d)),
                    ChamferSize::Distances { face, d1, d2 } => ChamferSize::Distances {
                        face,
                        d1: self.len(d1),
                        d2: self.len(d2),
                    },
                    ChamferSize::DistanceAngle { face, d, angle } => ChamferSize::DistanceAngle {
                        face,
                        d: self.len(d),
                        angle,
                    },
                },
            },
            Feature::Shell {
                thickness,
                outward,
                open,
            } => Feature::Shell {
                thickness: self.len(*thickness),
                outward: *outward,
                open: open.clone(),
            },
            Feature::Hole(h) if h.thread.is_none() || !matches!(self, Map::Scale(_)) => {
                let mut h = h.clone();
                h.at = self.frame(&h.at);
                h.diameter = self.len(h.diameter);
                h.head = match h.head {
                    Head::Plain => Head::Plain,
                    Head::Counterbore { diameter, depth } => Head::Counterbore {
                        diameter: self.len(diameter),
                        depth: self.len(depth),
                    },
                    Head::Countersink { diameter, angle } => Head::Countersink {
                        diameter: self.len(diameter),
                        angle,
                    },
                };
                h.end = match h.end {
                    End::Depth(d) => End::Depth(self.len(d)),
                    End::To { .. } => return None,
                    e => e,
                };
                if matches!(self, Map::Scale(_)) {
                    h.standard = None;
                }
                Feature::Hole(h)
            }
            Feature::Pattern { of, layout } => Feature::Pattern {
                of: of.clone(),
                layout: match layout {
                    Layout::Linear { first, second } => {
                        let row = |r: &forge_model::Row| forge_model::Row {
                            dir: self.vec(r.dir),
                            count: r.count,
                            spacing: self.len(r.spacing),
                        };
                        Layout::Linear {
                            first: row(first),
                            second: second.as_ref().map(row),
                        }
                    }
                    Layout::Circular {
                        origin,
                        axis,
                        count,
                        angle,
                        incremental,
                    } => {
                        // a mirror turns the other way round its axis: the axis reversed turns it back
                        let axis = self.vec(*axis);
                        Layout::Circular {
                            origin: self.point(*origin),
                            axis: if self.mirrors() { -axis } else { axis },
                            count: *count,
                            angle: *angle,
                            incremental: *incremental,
                        }
                    }
                    Layout::Mirror { plane } => Layout::Mirror {
                        plane: self.frame(plane),
                    },
                    _ => return None,
                },
            },
            Feature::Plane(f) => Feature::Plane(self.frame(f)),
            // they make nothing: the features using them hold what they solved to
            Feature::Param(_) | Feature::Sketch(_) => f.clone(),
            _ => return None,
        })
    }

    /// The history mapped, or None when it holds a feature this does not map.
    pub fn part(&self, p: &Part) -> Option<Part> {
        Some(Part {
            features: p
                .features
                .iter()
                .map(|f| self.feature(f))
                .collect::<Option<_>>()?,
        })
    }
}

/// A rotation whose matrix entries are ratios of Pythagorean triples (exact-ish: one rounding
/// each), about each world axis in turn, then a whole-millimetre move.
pub fn rigid(r: &mut forge_math::rng::Rng) -> Map {
    const TRIPLES: [(f64, f64, f64); 5] = [
        (3.0, 4.0, 5.0),
        (5.0, 12.0, 13.0),
        (8.0, 15.0, 17.0),
        (7.0, 24.0, 25.0),
        (20.0, 21.0, 29.0),
    ];
    let mut m = Frame::WORLD;
    for axis in 0..3 {
        let (a, b, c) = TRIPLES[r.below(TRIPLES.len())];
        let (cs, sn) = if r.bool() { (a / c, b / c) } else { (b / c, -a / c) };
        let (x, y, z) = match axis {
            0 => (Vec3::X, Vec3::new(0.0, cs, sn), Vec3::new(0.0, -sn, cs)),
            1 => (Vec3::new(cs, 0.0, -sn), Vec3::Y, Vec3::new(sn, 0.0, cs)),
            _ => (Vec3::new(cs, sn, 0.0), Vec3::new(-sn, cs, 0.0), Vec3::Z),
        };
        let turn = Frame::from_axes(Point3::ORIGIN, x, y, z).expect("a rotation");
        m = turn.compose(&m);
    }
    let t = Point3::new(
        r.below(201) as f64 - 100.0,
        r.below(201) as f64 - 100.0,
        r.below(201) as f64 - 100.0,
    );
    Map::Rigid(m.with_origin(t))
}

/// A quarter turn about a world axis: every coordinate maps exactly (a move would round).
pub fn quarter(r: &mut forge_math::rng::Rng) -> Map {
    let (x, y, z) = match r.below(3) {
        0 => (Vec3::X, Vec3::Z, -Vec3::Y),
        1 => (-Vec3::Z, Vec3::Y, Vec3::X),
        _ => (Vec3::Y, -Vec3::X, Vec3::Z),
    };
    Map::Rigid(Frame::from_axes(Point3::ORIGIN, x, y, z).expect("a rotation"))
}

/// A reflection in a world coordinate plane through the origin (exact) or, `tilted`, in a
/// plane with a random normal.
pub fn mirror(r: &mut forge_math::rng::Rng, tilted: bool) -> Map {
    if tilted {
        let n = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
        let n = n.normalize().unwrap_or(UnitVec3::new_unchecked(Vec3::X));
        Map::Mirror(n, r.range(-50.0, 50.0))
    } else {
        let n = [Vec3::X, Vec3::Y, Vec3::Z][r.below(3)];
        Map::Mirror(UnitVec3::new_unchecked(n), 0.0)
    }
}
