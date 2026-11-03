//! The sheet as a graph in folded design terms: walls with outlines in their own coordinates on the
//! reference skin (sharp, to the mold lines), joints along mold lines, cuts held by the wall they
//! were placed on. Folded and flat solids are both built from this (`build`).
use crate::error::{CutId, FeatureId, JointId, SheetError, TrimCurveKind, TrimRef, WallId};
use crate::region::pieces;
use crate::shop::BendRule;
use forge_math::{Frame, Point2, dmath, tol};
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CornerKind {
    Open { gap: f64 },
    Closed,
    Overlap { gap: f64 },
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
        self.cuts.push(Cut { id, wall, lp });
        self.next_cut += 1;
        Ok(id)
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
