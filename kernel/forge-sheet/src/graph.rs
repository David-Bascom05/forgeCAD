//! The sheet as a graph in folded design terms: walls with outlines in their own coordinates on the
//! reference skin (sharp, to the mold lines), joints along mold lines, cuts held by the wall they
//! were placed on. Folded and flat solids are both built from this (`build`).
use crate::error::{CutId, FeatureId, JointId, SheetError, TrimCurveKind, TrimRef, WallId};
use crate::region::pieces;
use crate::shop::BendRule;
use forge_math::{Frame, Point2, Vec2, dmath, tol};
use forge_ops::{Profile, ProfileLoop, Segment};

/// Thickness, the default inside radius, and how bends develop.
#[derive(Clone, Debug, PartialEq)]
pub struct Gauge {
    pub thickness: f64,
    pub radius: f64,
    pub rule: BendRule,
}

/// A wall: its outline on the reference skin, counter-clockwise, then its holes clockwise, in its
/// own coordinates. Material lies from the reference skin to `thickness` below it (local -z).
#[derive(Clone, Debug, PartialEq)]
pub struct Wall {
    pub id: WallId,
    pub outline: Vec<ProfileLoop>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JointKind {
    /// Turns wall b by `angle` about the mold line: positive towards wall a's +z (its reference
    /// skin is then the bend's inside), negative away from it. `radius` is the inside radius.
    Bend { angle: f64, radius: f64 },
    /// The walls meet without joining, `gap` apart.
    Rip { gap: f64 },
    /// The walls continue each other across the line.
    Tangent,
}

/// Two walls along one mold line: segment `on_a` of a's outline and `on_b` of b's, both straight,
/// of one length, run opposite ways (each with its wall on its left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joint {
    pub id: JointId,
    pub a: WallId,
    pub b: WallId,
    pub on_a: u64,
    pub on_b: u64,
    pub kind: JointKind,
}

/// What one corner gets, where two bends off one wall meet at a vertex of its outline (an
/// override of the rule's). Circular and square reliefs are centred by `placement`, `size` their
/// radius or half side (None: just clear of both bends' zones); open is a square at the vertex a
/// gap larger than that; a tear is a kerf-wide slit along the corner's diagonal; trim to bend cuts
/// the corner back to both tangent lines across both zones. Closed and overlapping corners
/// (flanges extended to meet) are typed but not built.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerKind {
    Open {
        gap: f64,
    },
    Closed,
    Overlap {
        gap: f64,
    },
    Circular {
        size: Option<f64>,
        placement: crate::relief::Placement,
    },
    Square {
        size: Option<f64>,
        placement: crate::relief::Placement,
    },
    Tear,
    TrimToBend,
}

/// Where two joints' ends meet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    pub joints: [JointId; 2],
    pub kind: CornerKind,
}

/// A loop taken out of the sheet, counter-clockwise in the developed coordinates of `wall`: where it
/// runs past the wall's tangent lines it cuts the bend zones and the walls beyond as they unfold.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub id: CutId,
    pub wall: WallId,
    pub lp: ProfileLoop,
    /// Meant to run across a bend (a slot across it): checked as such, not as a hole near it.
    pub crosses_bend: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    pub feature: FeatureId,
    pub gauge: Gauge,
    /// The first wall's place in the folded part; every other wall follows from the bends.
    pub frame: Frame,
    pub walls: Vec<Wall>,
    pub joints: Vec<Joint>,
    pub corners: Vec<Corner>,
    pub cuts: Vec<Cut>,
    /// Hardware pressed in, each with its mounting hole among `cuts`.
    pub mounts: Vec<crate::hardware::Mount>,
    /// Reliefs asked for at joint ends; any end not listed is `Relief::Auto`.
    pub reliefs: Vec<(JointId, crate::error::End, crate::relief::Relief)>,
    /// The shop's relief rule: what `Auto` cuts where a relief is needed, depth references,
    /// corners.
    pub rule: crate::relief::ReliefRule,
    /// What a corner not listed in `corners` gets, overriding the rule's for this sheet.
    pub corner_default: Option<CornerKind>,
    next_wall: u32,
    next_joint: u32,
    next_cut: u32,
}

impl Sheet {
    /// A sheet of one wall: `profile`'s loops (outline, then holes) on the plane `frame`, which is the
    /// reference skin; the material lies on the frame's -z side.
    pub fn face(
        feature: FeatureId,
        gauge: Gauge,
        frame: Frame,
        profile: &[ProfileLoop],
    ) -> Result<(Sheet, WallId), SheetError> {
        if gauge.thickness.is_nan() || gauge.thickness <= tol::LINEAR {
            return Err(SheetError::Thickness {
                feature,
                thickness: gauge.thickness,
                limit: tol::LINEAR,
            });
        }
        if gauge.radius.is_nan() || gauge.radius <= tol::LINEAR {
            return Err(SheetError::Radius {
                feature,
                radius: gauge.radius,
                limit: tol::LINEAR,
            });
        }
        let id = WallId(0);
        let loops = Profile {
            plane: frame,
            loops: profile.to_vec(),
        }
        .normalized()
        .map_err(|why| SheetError::Profile {
            feature,
            wall: id,
            why,
        })?
        .loops;
        let sheet = Sheet {
            feature,
            gauge,
            frame,
            walls: vec![Wall { id, outline: loops }],
            joints: Vec::new(),
            corners: Vec::new(),
            cuts: Vec::new(),
            mounts: Vec::new(),
            reliefs: Vec::new(),
            rule: crate::relief::ReliefRule::default(),
            corner_default: None,
            next_wall: 1,
            next_joint: 0,
            next_cut: 0,
        };
        Ok((sheet, id))
    }

    pub fn wall(&self, id: WallId) -> Option<&Wall> {
        self.walls.iter().find(|w| w.id == id)
    }

    pub fn joint(&self, id: JointId) -> Option<&Joint> {
        self.joints.iter().find(|j| j.id == id)
    }

    /// A flange on the straight outline segment `segment` of `wall`: a new wall turned by `angle`
    /// (radians, signed as `JointKind::Bend`) about it with the gauge's radius, `height` from the
    /// mold line on the reference skin, as long as the segment.
    pub fn flange(
        &mut self,
        wall: WallId,
        segment: u64,
        angle: f64,
        height: f64,
    ) -> Result<(WallId, JointId), SheetError> {
        self.flange_r(wall, segment, angle, height, self.gauge.radius)
    }

    /// `flange` with its own inside radius.
    pub fn flange_r(
        &mut self,
        wall: WallId,
        segment: u64,
        angle: f64,
        height: f64,
        radius: f64,
    ) -> Result<(WallId, JointId), SheetError> {
        let feature = self.feature;
        if radius.is_nan() || radius <= tol::LINEAR {
            return Err(SheetError::Radius {
                feature,
                radius,
                limit: tol::LINEAR,
            });
        }
        let w = self.wall(wall).ok_or(SheetError::NoSuchWall { feature, wall })?;
        let trim = TrimRef::Outline { wall, segment };
        let (a, b) = straight(w, segment).map_err(|e| match e {
            None => SheetError::NoSuchSegment {
                feature,
                wall,
                segment,
            },
            Some(curve) => SheetError::NotStraight { feature, trim, curve },
        })?;
        if let Some(j) = self
            .joints
            .iter()
            .find(|j| (j.a == wall && j.on_a == segment) || (j.b == wall && j.on_b == segment))
        {
            return Err(SheetError::EdgeTaken {
                feature,
                trim,
                joint: j.id,
            });
        }
        // an edge running out of another bend's mold line (a flange's end edge) bends only from
        // that bend's tangent line on: the stretch inside its band is the bend's
        let (a, b) = self.stop_at_tangents(wall, segment, a, b)?;
        let joint = JointId(self.next_joint);
        if !(angle.abs() > tol::ANGULAR && angle.abs() < dmath::PI - tol::ANGULAR) {
            let limit = if angle.abs() <= tol::ANGULAR {
                tol::ANGULAR
            } else {
                dmath::PI - tol::ANGULAR
            };
            return Err(SheetError::BendAngle {
                feature,
                joint,
                angle,
                limit,
            });
        }
        let id = WallId(self.next_wall);
        let len = a.distance(b);
        let s = crate::build::setback_of(&self.gauge, radius, angle);
        if (height - s).is_nan() || height - s <= tol::LINEAR {
            return Err(SheetError::WallTooShort {
                feature,
                wall: id,
                joint,
                length: height - s,
                limit: tol::LINEAR,
            });
        }
        let p = Point2::new;
        let c = [p(0.0, 0.0), p(height, 0.0), p(height, len), p(0.0, len)];
        let outline = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    (
                        Segment::Line {
                            a: c[i],
                            b: c[(i + 1) % 4],
                        },
                        i as u64,
                    )
                })
                .collect(),
        );
        self.walls.push(Wall {
            id,
            outline: vec![outline],
        });
        self.joints.push(Joint {
            id: joint,
            a: wall,
            b: id,
            on_a: segment,
            on_b: 3,
            kind: JointKind::Bend { angle, radius },
        });
        self.next_wall += 1;
        self.next_joint += 1;
        Ok((id, joint))
    }

    /// Takes `lp` out of the sheet, placed in `wall`'s developed coordinates. Either winding.
    pub fn cut(&mut self, wall: WallId, lp: ProfileLoop) -> Result<CutId, SheetError> {
        let feature = self.feature;
        self.wall(wall).ok_or(SheetError::NoSuchWall { feature, wall })?;
        let id = CutId(self.next_cut);
        // forge-geom's own rule for an arc (Spline::ellipse)
        let limit = dmath::TAU * (1.0 + 1e-12);
        for (s, segment) in &lp.segments {
            if let Segment::Arc { sweep, .. } = *s
                && (!sweep.is_finite() || sweep == 0.0 || sweep.abs() > limit)
            {
                return Err(SheetError::ArcSweep {
                    feature,
                    trim: TrimRef::Cut {
                        cut: id,
                        segment: *segment,
                    },
                    sweep,
                    limit,
                });
            }
        }
        let area: f64 = pieces(&lp).iter().map(|p| p.0.area_term()).sum();
        let lp = if area < 0.0 { reversed(&lp) } else { lp };
        self.cuts.push(Cut {
            id,
            wall,
            lp,
            crosses_bend: false,
        });
        self.next_cut += 1;
        Ok(id)
    }
}

/// Where a flange stands against the edge it is put on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// Its material outside the edge: the inner virtual sharp on it.
    Outside,
    /// Its material inside the edge: the outer virtual sharp on it (the edge moves in).
    Inside,
    /// The bend starting at the edge: the tangent line on it (the edge moves out).
    BendOutside,
}

impl Sheet {
    /// The corner treatment asked for where joints a and b meet (None: the rule's).
    pub fn corner(&self, a: JointId, b: JointId) -> Option<CornerKind> {
        self.corners
            .iter()
            .rev()
            .find(|c| c.joints == [a, b] || c.joints == [b, a])
            .map(|c| c.kind)
            .or(self.corner_default)
    }

    pub fn set_corner(&mut self, a: JointId, b: JointId, kind: CornerKind) {
        self.corners
            .retain(|c| !(c.joints == [a, b] || c.joints == [b, a]));
        self.corners.push(Corner { joints: [a, b], kind });
    }

    /// The relief joint end `end` asks for.
    pub fn relief(&self, joint: JointId, end: crate::error::End) -> crate::relief::Relief {
        self.reliefs
            .iter()
            .rev()
            .find(|r| r.0 == joint && r.1 == end)
            .map_or(crate::relief::Relief::Auto, |r| r.2)
    }

    pub fn set_relief(&mut self, joint: JointId, end: crate::error::End, relief: crate::relief::Relief) {
        self.reliefs.retain(|r| !(r.0 == joint && r.1 == end));
        self.reliefs.push((joint, end, relief));
    }

    /// `flange_r` with the flange stood `position` against the edge: the edge moved along its
    /// normal first (its neighbours, lines, follow it).
    pub fn flange_at(
        &mut self,
        wall: WallId,
        segment: u64,
        angle: f64,
        height: f64,
        radius: f64,
        position: Position,
    ) -> Result<(WallId, JointId), SheetError> {
        let t = self.gauge.thickness;
        let theta = angle.abs();
        let sharp = t * dmath::sin(theta) / (1.0 + dmath::cos(theta));
        let s = crate::build::setback_of(&self.gauge, radius, angle);
        // outward, from where the kernel's mold line (the reference skin's sharp) would be
        let d = match (position, angle > 0.0) {
            (Position::Outside, true) | (Position::Inside, false) => 0.0,
            (Position::Inside, true) => -sharp,
            (Position::Outside, false) => sharp,
            (Position::BendOutside, _) => s,
        };
        // all or nothing: a refused flange leaves the sheet as it was
        if let Some(j) = self
            .joints
            .iter()
            .find(|j| (j.a == wall && j.on_a == segment) || (j.b == wall && j.on_b == segment))
        {
            return Err(SheetError::EdgeTaken {
                feature: self.feature,
                trim: TrimRef::Outline { wall, segment },
                joint: j.id,
            });
        }
        if !(theta > tol::ANGULAR && theta < dmath::PI - tol::ANGULAR) {
            return Err(SheetError::BendAngle {
                feature: self.feature,
                joint: JointId(self.next_joint),
                angle,
                limit: if theta <= tol::ANGULAR {
                    tol::ANGULAR
                } else {
                    dmath::PI - tol::ANGULAR
                },
            });
        }
        let mut next = self.clone();
        if d != 0.0 {
            // an end edge first stops at its wall's own bend; then the rest moves, a jog joining
            // it to the stretch in that bend's band (cut away by the corner's relief)
            let w = next.wall(wall).ok_or(SheetError::NoSuchWall {
                feature: self.feature,
                wall,
            })?;
            if let Ok((a, b)) = straight(w, segment) {
                next.stop_at_tangents(wall, segment, a, b)?;
            }
            next.move_edge(wall, segment, d)?;
        }
        let made = next.flange_r(wall, segment, angle, height, radius)?;
        *self = next;
        Ok(made)
    }

    /// Moves straight outline segment `segment` of `wall` by `d` along its outward normal; the
    /// segments either side, lines, are cut or stretched to meet it. A neighbour carrying a
    /// flange (this wall its first) takes the flange along: its wall is lengthened or shortened
    /// at that end to match. A neighbour that is this wall's own mold line is refused.
    pub fn move_edge(&mut self, wall: WallId, segment: u64, d: f64) -> Result<(), SheetError> {
        self.move_edge_with(wall, segment, d, None)
    }

    fn move_edge_with(
        &mut self,
        wall: WallId,
        segment: u64,
        d: f64,
        allow: Option<JointId>,
    ) -> Result<(), SheetError> {
        let feature = self.feature;
        let trim = |segment| TrimRef::Outline { wall, segment };
        let w = self.wall(wall).ok_or(SheetError::NoSuchWall { feature, wall })?;
        let (li, k) = w
            .outline
            .iter()
            .enumerate()
            .find_map(|(li, l)| l.segments.iter().position(|s| s.1 == segment).map(|k| (li, k)))
            .ok_or(SheetError::NoSuchSegment {
                feature,
                wall,
                segment,
            })?;
        let segs = &w.outline[li].segments;
        let n = segs.len();
        let line = |i: usize| match &segs[i].0 {
            Segment::Line { a, b } => Ok((*a, *b)),
            Segment::Arc { .. } | Segment::Circle { .. } => Err(TrimCurveKind::Arc),
            Segment::Spline(_) => Err(TrimCurveKind::Spline),
        };
        let (a, b) = line(k).map_err(|curve| SheetError::NotStraight {
            feature,
            trim: trim(segment),
            curve,
        })?;
        let (pi, ni) = ((k + n - 1) % n, (k + 1) % n);
        // flanges standing on the neighbours, to carry along: (joint, which end of it moves)
        let mut carried: Vec<(JointId, crate::error::End, u64)> = Vec::new();
        for (i, end) in [(pi, crate::error::End::End), (ni, crate::error::End::Start)] {
            let id = segs[i].1;
            if let Some(j) = self
                .joints
                .iter()
                .find(|j| (j.a == wall && j.on_a == id) || (j.b == wall && j.on_b == id))
            {
                if Some(j.id) == allow {
                    continue;
                }
                // a flange on the neighbour follows, and so do flanges on its own end (chains)
                if j.a == wall {
                    carried.push((j.id, end, id));
                    continue;
                }
                return Err(SheetError::EdgeMoveBlocked {
                    feature,
                    trim: trim(id),
                    joint: j.id,
                });
            }
        }
        let before: Vec<(u64, f64)> = [pi, ni]
            .iter()
            .filter_map(|&i| line(i).ok().map(|(a, b)| (segs[i].1, a.distance(b))))
            .collect();
        let curve_err = |i: usize, curve| SheetError::EdgeMoveCurve {
            feature,
            trim: trim(segs[i].1),
            curve,
        };
        let (p0, _) = line(pi).map_err(|c| curve_err(pi, c))?;
        let (_, n1) = line(ni).map_err(|c| curve_err(ni, c))?;
        let dir = (b - a) * (1.0 / a.distance(b));
        let out = Vec2::new(dir.y, -dir.x) * d;
        let a2 = a + out;
        // where the moved line meets each neighbour's line
        let meet = |p: Point2, q: Point2| {
            let e = q - p;
            let den = e.cross(dir);
            if den.abs() <= 1e-15 {
                return None;
            }
            let u = (a2 - p).cross(dir) / den;
            Some(p + e * u)
        };
        // a neighbour running on in line with the edge keeps its place; a jog joins them
        let (a3, jog_a) = match meet(p0, a) {
            Some(x) => (x, false),
            None => (a2, true),
        };
        let b2 = b + out;
        let (b3, jog_b) = match meet(n1, b) {
            Some(x) => (x, false),
            None => (b2, true),
        };
        let mut outline = w.outline.clone();
        let l = &mut outline[li].segments;
        if !jog_a {
            l[pi].0 = Segment::Line { a: p0, b: a3 };
        }
        l[k].0 = Segment::Line { a: a3, b: b3 };
        if !jog_b {
            l[ni].0 = Segment::Line { a: b3, b: n1 };
        }
        // jogs take ids from the edge's own, so they name the same each build
        if jog_b {
            l.insert(k + 1, (Segment::Line { a: b3, b }, segment ^ (1 << 45)));
        }
        if jog_a {
            l.insert(k, (Segment::Line { a, b: a3 }, segment ^ (1 << 44)));
        }
        let n = l.len();
        let (pi, k, ni) = if jog_a {
            (k, k + 1, (k + 2) % n)
        } else {
            (pi, k, ni)
        };
        let ni = if jog_b { (k + 1) % n } else { ni };
        for i in [pi, k, ni] {
            if let Segment::Line { a, b } = l[i].0
                && a.distance(b) <= tol::LINEAR
            {
                return Err(SheetError::Profile {
                    feature,
                    wall,
                    why: forge_ops::ProfileError::Empty,
                });
            }
        }
        let checked = Profile {
            plane: self.frame,
            loops: outline,
        }
        .normalized()
        .map_err(|why| SheetError::Profile { feature, wall, why })?;
        let w = self.walls.iter_mut().find(|w| w.id == wall).expect("found above");
        w.outline = checked.loops;
        // the flanges on the neighbours follow their edges' new lengths
        for (joint, end, id) in carried {
            let now = self
                .wall(wall)
                .and_then(|w| straight(w, id).ok())
                .map(|(a, b)| a.distance(b))
                .unwrap_or(f64::NAN);
            let was = before.iter().find(|x| x.0 == id).map_or(f64::NAN, |x| x.1);
            let j = *self.joint(joint).expect("listed");
            let flange = self
                .wall(j.b)
                .ok_or(SheetError::NoSuchWall { feature, wall: j.b })?;
            // the flange's side at that end: beside its mold line, which runs from the end to the
            // start (q0 at y = len, q1 at y = 0)
            let side = flange.outline.iter().find_map(|l| {
                let m = l.segments.iter().position(|g| g.1 == j.on_b)?;
                let n = l.segments.len();
                Some(match end {
                    crate::error::End::End => l.segments[(m + n - 1) % n].1,
                    crate::error::End::Start => l.segments[(m + 1) % n].1,
                })
            });
            let Some(side) = side else { continue };
            self.move_edge_with(j.b, side, now - was, Some(joint))?;
        }
        Ok(())
    }

    /// Presses `part` into `wall` at `at` (the wall's coordinates): its mounting hole, cut like
    /// any other, and the part recorded for the checks.
    pub fn mount(
        &mut self,
        wall: WallId,
        at: Point2,
        part: crate::hardware::Hardware,
    ) -> Result<CutId, SheetError> {
        let feature = self.feature;
        if part.hole.is_nan() || part.hole <= tol::LINEAR {
            return Err(SheetError::Radius {
                feature,
                radius: 0.5 * part.hole,
                limit: tol::LINEAR,
            });
        }
        let hole = Segment::Circle {
            center: at,
            radius: 0.5 * part.hole,
            ccw: true,
        };
        let cut = self.cut(wall, ProfileLoop::new(vec![(hole, 0)]))?;
        self.mounts.push(crate::hardware::Mount { cut, wall, at, part });
        Ok(cut)
    }
}

impl Sheet {
    /// Splits straight segment `segment` of `wall` (a to b) where it leaves the band of the bend
    /// the wall was turned by: at that bend's setback from an end it shares with its mold line. The stretch in
    /// the band takes a derived id; the rest keeps `segment`. Returns the rest's ends.
    fn stop_at_tangents(
        &mut self,
        wall: WallId,
        segment: u64,
        a: Point2,
        b: Point2,
    ) -> Result<(Point2, Point2), SheetError> {
        let feature = self.feature;
        let w = self.wall(wall).ok_or(SheetError::NoSuchWall { feature, wall })?;
        let molds: Vec<(Point2, Point2, f64)> = self
            .joints
            .iter()
            .filter_map(|j| {
                // the wall's own mold line (it is the bend's second wall): its band runs along
                // the end of every edge leaving it. Bends off neighbouring edges of the wall meet
                // at a corner instead (corner reliefs).
                if j.b != wall {
                    return None;
                }
                let on = j.on_b;
                let JointKind::Bend { angle, radius } = j.kind else {
                    return None;
                };
                let (p, q) = straight(w, on).ok()?;
                Some((p, q, crate::build::setback_of(&self.gauge, radius, angle)))
            })
            .collect();
        let at = |x: Point2| {
            molds
                .iter()
                .filter(|m| m.0.distance(x) <= tol::LINEAR || m.1.distance(x) <= tol::LINEAR)
                .map(|m| m.2)
                .fold(0.0, f64::max)
        };
        let (sa, sb) = (at(a), at(b));
        if sa == 0.0 && sb == 0.0 {
            return Ok((a, b));
        }
        let len = a.distance(b);
        let d = (b - a) * (1.0 / len);
        if sa + sb >= len - tol::LINEAR {
            return Err(SheetError::NoSuchSegment {
                feature,
                wall,
                segment,
            });
        }
        let (a2, b2) = (a + d * sa, b - d * sb);
        let wall_mut = self.walls.iter_mut().find(|w| w.id == wall).expect("found above");
        for l in &mut wall_mut.outline {
            let Some(k) = l.segments.iter().position(|g| g.1 == segment) else {
                continue;
            };
            let mut pieces = Vec::new();
            if sa > 0.0 {
                pieces.push((Segment::Line { a, b: a2 }, segment ^ (1 << 46)));
            }
            pieces.push((Segment::Line { a: a2, b: b2 }, segment));
            if sb > 0.0 {
                pieces.push((Segment::Line { a: b2, b }, segment ^ (1 << 47)));
            }
            l.segments.splice(k..=k, pieces);
            break;
        }
        Ok((a2, b2))
    }
}

/// The ends of a wall's outline segment, if it is a line (Err(None): no such segment).
pub(crate) fn straight(w: &Wall, segment: u64) -> Result<(Point2, Point2), Option<TrimCurveKind>> {
    let s = w
        .outline
        .iter()
        .flat_map(|l| &l.segments)
        .find(|s| s.1 == segment)
        .ok_or(None)?;
    match &s.0 {
        Segment::Line { a, b } => Ok((*a, *b)),
        Segment::Arc { .. } | Segment::Circle { .. } => Err(Some(TrimCurveKind::Arc)),
        Segment::Spline(_) => Err(Some(TrimCurveKind::Spline)),
    }
}

fn reversed(l: &ProfileLoop) -> ProfileLoop {
    let segs: Vec<(Segment, u64)> = l
        .segments
        .iter()
        .rev()
        .map(|(s, id)| {
            let s = match s {
                Segment::Line { a, b } => Segment::Line { a: *b, b: *a },
                Segment::Arc {
                    center,
                    radius,
                    start,
                    sweep,
                } => Segment::Arc {
                    center: *center,
                    radius: *radius,
                    start: start + sweep,
                    sweep: -sweep,
                },
                Segment::Circle { center, radius, ccw } => Segment::Circle {
                    center: *center,
                    radius: *radius,
                    ccw: !ccw,
                },
                Segment::Spline(_) => {
                    let (p, _) = &pieces(&ProfileLoop::new(vec![(s.clone(), *id)]))[0];
                    p.reversed().to_segment().unwrap_or_else(|| s.clone())
                }
            };
            (s, *id)
        })
        .collect();
    ProfileLoop::new(segs)
}
