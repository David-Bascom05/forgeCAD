//! Contour flanges: an open profile of lines and arcs, the reference skin's trace in a section
//! plane, made into walls (its lines) and bends (its arcs, and its sharp corners at the sheet's
//! radius), either as a sheet's first walls (extruded `width` along the section's normal) or run
//! off a straight wall edge (as long as the edge). Every wall and bend is an ordinary flange of the
//! graph, so folded, flat, names and checks are the graph's own: the profile's lines are walls
//! between their virtual sharps, an arc of radius r on the reference skin is a bend of inside radius
//! r when it turns towards the reference side and r - t when it turns away. Splines, whole circles,
//! two arcs in a row (a tangent joint) and arcs at the profile's ends are refused by name.
use crate::error::{FeatureId, JointId, SheetError, WallId};
use crate::graph::{Gauge, Sheet};
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath, tol};
use forge_ops::{ProfileLoop, Segment};

/// Which side of the profile, walking along it, the material lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hand {
    Left,
    Right,
}

/// Why a contour is refused; `piece` indexes the profile as given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContourError {
    /// No pieces, or no line.
    Empty,
    /// A spline: a general bent strip, not built yet.
    Spline {
        piece: usize,
    },
    /// A whole circle: a contour is open.
    Closed {
        piece: usize,
    },
    /// Piece `piece` does not start where the one before ends.
    Gap {
        piece: usize,
        gap: f64,
        limit: f64,
    },
    /// An arc first or last: a bend with no wall beyond it.
    ArcAtEnd {
        piece: usize,
    },
    /// Two arcs in a row (a tangent joint between bends, with no wall): not built yet.
    ArcsMeet {
        piece: usize,
    },
    /// An arc not tangent to the line before or after it, by `angle`.
    NotTangent {
        piece: usize,
        angle: f64,
        limit: f64,
    },
    /// A turn of half a turn or more (`limit` is the largest a bend takes).
    Turn {
        piece: usize,
        angle: f64,
        limit: f64,
    },
    /// An arc whose bend's inside radius (r on the reference skin, less t when the bend turns away
    /// from it) is below the least the shop bends, or not positive.
    RadiusBelowMin {
        piece: usize,
        radius: f64,
        limit: f64,
    },
    /// Line `piece` is too short for the setbacks of the bends at its ends.
    WallTooShort {
        piece: usize,
        length: f64,
        limit: f64,
    },
    /// Run off an edge, the profile's start is not on the edge (the section's origin).
    StartOffEdge {
        gap: f64,
        limit: f64,
    },
    /// Run off an edge, the first line carries on in the wall's plane (no bend there).
    Coplanar {
        piece: usize,
    },
    Width {
        width: f64,
        limit: f64,
    },
}

/// What a contour made: the wall of each of the profile's lines and the bend of each arc or
/// sharp corner (the piece index of the arc, or of the line after the corner).
#[derive(Clone, Debug, PartialEq)]
pub struct ContourMade {
    pub walls: Vec<(usize, WallId)>,
    pub joints: Vec<(usize, JointId)>,
}

/// One planned wall: its length between virtual sharps; one planned bend: signed turn in the
/// section plane (counter-clockwise positive) and the reference skin's radius (None: sharp).
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// (piece, mold-line length, its start point on the profile line, its direction)
    pub walls: Vec<(usize, f64, Point2, Vec2)>,
    /// (piece, turn, reference-skin radius if an arc)
    pub bends: Vec<(usize, f64, Option<f64>)>,
}

const MEET: f64 = tol::LINEAR;
/// Arcs are tangent to their lines to this (radians).
const TANGENT: f64 = 1e-9;

fn arc_end(center: Point2, radius: f64, a: f64) -> Point2 {
    let (s, c) = dmath::sin_cos(a);
    Point2::new(center.x + radius * c, center.y + radius * s)
}

/// The direction an arc runs at angle `a` (sweep's sign).
fn arc_dir(a: f64, sweep: f64) -> Vec2 {
    let (s, c) = dmath::sin_cos(a);
    if sweep > 0.0 {
        Vec2::new(-s, c)
    } else {
        Vec2::new(s, -c)
    }
}

/// The signed angle from u to v (unit), counter-clockwise positive, in (-pi, pi].
fn turn(u: Vec2, v: Vec2) -> f64 {
    dmath::atan2(u.cross(v), u.dot(v))
}

#[derive(Clone, Copy)]
enum P {
    Line {
        a: Point2,
        b: Point2,
        d: Vec2,
        len: f64,
    },
    Arc {
        r: f64,
        d0: Vec2,
        d1: Vec2,
        a: Point2,
        b: Point2,
        sweep: f64,
    },
}

impl P {
    fn ends(&self) -> (Point2, Point2) {
        match *self {
            P::Line { a, b, .. } | P::Arc { a, b, .. } => (a, b),
        }
    }
}

/// The walls and bends of a profile in its section's coordinates, before any sheet is touched.
/// `start` is the direction a contour run off an edge leaves it in (+x), None for a base.
pub fn plan(profile: &[Segment], start: Option<Vec2>) -> Result<Plan, ContourError> {
    let mut ps: Vec<P> = Vec::with_capacity(profile.len());
    for (piece, s) in profile.iter().enumerate() {
        ps.push(match *s {
            Segment::Line { a, b } => {
                let len = a.distance(b);
                if len.is_nan() || len <= MEET {
                    return Err(ContourError::WallTooShort {
                        piece,
                        length: len,
                        limit: MEET,
                    });
                }
                P::Line {
                    a,
                    b,
                    d: (b - a) * (1.0 / len),
                    len,
                }
            }
            Segment::Arc {
                center,
                radius,
                start,
                sweep,
            } => P::Arc {
                r: radius,
                d0: arc_dir(start, sweep),
                d1: arc_dir(start + sweep, sweep),
                a: arc_end(center, radius, start),
                b: arc_end(center, radius, start + sweep),
                sweep,
            },
            Segment::Circle { .. } => return Err(ContourError::Closed { piece }),
            Segment::Spline(_) => return Err(ContourError::Spline { piece }),
        });
    }
    if ps.is_empty() {
        return Err(ContourError::Empty);
    }
    for k in 1..ps.len() {
        let gap = ps[k - 1].ends().1.distance(ps[k].ends().0);
        if gap.is_nan() || gap > MEET {
            return Err(ContourError::Gap {
                piece: k,
                gap,
                limit: MEET,
            });
        }
    }
    for k in [0, ps.len() - 1] {
        if matches!(ps[k], P::Arc { .. }) {
            return Err(ContourError::ArcAtEnd { piece: k });
        }
    }
    // lines in a row with no turn are one wall; a bend sits at each other corner and arc
    let mut walls: Vec<(usize, f64, Point2, Vec2)> = Vec::new();
    let mut bends: Vec<(usize, f64, Option<f64>)> = Vec::new();
    // reference-skin setbacks either side of each wall, for the length check
    let mut k = 0;
    while k < ps.len() {
        let P::Line { a, d, len, .. } = ps[k] else {
            unreachable!("lines stand between arcs")
        };
        let mut b = a + d * len;
        let mut j = k + 1;
        while let Some(&P::Line { d: d2, len: l2, .. }) = ps.get(j) {
            if turn(d, d2).abs() > TANGENT {
                break;
            }
            b += d * l2;
            j += 1;
        }
        walls.push((k, a.distance(b), a, d));
        match ps.get(j) {
            None => break,
            Some(&P::Line { d: d2, .. }) => {
                bends.push((j, turn(d, d2), None));
                k = j;
            }
            Some(&P::Arc { r, d0, d1, sweep, .. }) => {
                let P::Line { d: d2, .. } = ps[j + 1] else {
                    return Err(ContourError::ArcsMeet { piece: j + 1 });
                };
                for (x, y) in [(d, d0), (d1, d2)] {
                    let angle = turn(x, y);
                    if angle.is_nan() || angle.abs() > TANGENT {
                        return Err(ContourError::NotTangent {
                            piece: j,
                            angle,
                            limit: TANGENT,
                        });
                    }
                }
                let th = turn(d, d2);
                if (th - sweep).abs() > 1e-9 {
                    // tangent at both ends but turning the other way round: a turn past half
                    return Err(ContourError::Turn {
                        piece: j,
                        angle: sweep,
                        limit: dmath::PI - tol::ANGULAR,
                    });
                }
                bends.push((j, th, Some(r)));
                k = j + 1;
            }
        }
    }
    if let Some(s) = start {
        // the corner where the profile leaves the edge
        let (_, _, _, d0) = walls[0];
        let th = turn(s, d0);
        if th.abs() <= TANGENT {
            return Err(ContourError::Coplanar { piece: 0 });
        }
        bends.insert(0, (0, th, None));
    }
    for &(piece, angle, _) in &bends {
        if angle.is_nan() || angle.abs() >= dmath::PI - tol::ANGULAR {
            return Err(ContourError::Turn {
                piece,
                angle,
                limit: dmath::PI - tol::ANGULAR,
            });
        }
    }
    // a wall runs from virtual sharp to virtual sharp: an arc's lines carry on past its ends by
    // r tan(turn / 2)
    let mut out = Vec::with_capacity(walls.len());
    let off = if start.is_some() { 1 } else { 0 };
    for (w, &(piece, len, a, d)) in walls.iter().enumerate() {
        let before = if w == 0 { None } else { bends.get(w - 1 + off) };
        let after = bends.get(w + off);
        let ext = |b: Option<&(usize, f64, Option<f64>)>| {
            b.and_then(|&(_, th, r)| r.map(|r| crate::shop::setback(r, th.abs())))
                .unwrap_or(0.0)
        };
        let (e0, e1) = (ext(before), ext(after));
        out.push((piece, len + e0 + e1, a - d * e0, d));
    }
    Ok(Plan { walls: out, bends })
}

/// A planned bend's angle as the graph signs it (towards the reference side positive) and its
/// inside radius, for material on `hand` of the profile.
fn signed(g: &Gauge, hand: Hand, turn: f64, r: Option<f64>, default: f64) -> (f64, f64) {
    // the reference side is the side away from the material: a left turn heads there when the
    // material lies on the right
    let angle = if hand == Hand::Right { turn } else { -turn };
    let radius = match r {
        None => default,
        Some(r) if angle > 0.0 => r,
        Some(r) => r - g.thickness,
    };
    (angle, radius)
}

/// Checks the planned bends' radii and walls' lengths against the setbacks.
fn checked(
    g: &Gauge,
    plan: &Plan,
    hand: Hand,
    default: f64,
    min_radius: f64,
) -> Result<Vec<(f64, f64)>, ContourError> {
    let limit = min_radius.max(tol::LINEAR);
    let mut out = Vec::with_capacity(plan.bends.len());
    for &(piece, th, r) in &plan.bends {
        let (angle, radius) = signed(g, hand, th, r, default);
        if radius.is_nan() || radius < limit {
            return Err(ContourError::RadiusBelowMin { piece, radius, limit });
        }
        out.push((angle, radius));
    }
    Ok(out)
}

fn wall_lengths(g: &Gauge, plan: &Plan, bends: &[(f64, f64)], edge: bool) -> Result<(), ContourError> {
    let off = usize::from(edge);
    for (w, &(piece, len, _, _)) in plan.walls.iter().enumerate() {
        let s = |i: Option<usize>| {
            i.and_then(|i| bends.get(i))
                .map_or(0.0, |&(a, r)| crate::build::setback_of(g, r, a))
        };
        let before = if w == 0 && !edge { None } else { Some(w + off - 1) };
        let rest = len - s(before) - s(Some(w + off).filter(|&i| i < bends.len()));
        if rest.is_nan() || rest <= tol::LINEAR {
            return Err(ContourError::WallTooShort {
                piece,
                length: rest,
                limit: tol::LINEAR,
            });
        }
    }
    Ok(())
}

fn err(feature: FeatureId, why: ContourError) -> SheetError {
    SheetError::Contour { feature, why }
}

/// The chain of flanges: wall i + 1 turned off wall i's far edge (segment 1).
fn chain(
    sheet: &mut Sheet,
    mut wall: WallId,
    mut segment: u64,
    plan: &Plan,
    bends: &[(f64, f64)],
    first_wall: usize,
    made: &mut ContourMade,
) -> Result<(), SheetError> {
    for (i, &(angle, radius)) in bends.iter().enumerate() {
        let (piece, len, _, _) = plan.walls[first_wall + i];
        let (w, j) = sheet.flange_r(wall, segment, angle, len, radius)?;
        made.joints.push((plan.bends[i].0, j));
        made.walls.push((piece, w));
        wall = w;
        segment = 1;
    }
    Ok(())
}

impl Sheet {
    /// A sheet whose first walls are `profile` (open; lines and tangent arcs, in `section`'s xy)
    /// run `width` along the section's +z, the material on `hand` of the profile. Sharp corners
    /// bend at the gauge's radius; `min_radius` is the least inside radius allowed (the shop's).
    pub fn contour(
        feature: FeatureId,
        gauge: Gauge,
        section: Frame,
        profile: &[Segment],
        width: f64,
        hand: Hand,
        min_radius: f64,
    ) -> Result<(Sheet, ContourMade), SheetError> {
        if width.is_nan() || width <= tol::LINEAR {
            return Err(err(
                feature,
                ContourError::Width {
                    width,
                    limit: tol::LINEAR,
                },
            ));
        }
        let plan = plan(profile, None).map_err(|w| err(feature, w))?;
        let default = gauge.radius;
        let bends = checked(&gauge, &plan, hand, default, min_radius).map_err(|w| err(feature, w))?;
        wall_lengths(&gauge, &plan, &bends, false).map_err(|w| err(feature, w))?;
        let (piece, len, a, d) = plan.walls[0];
        let n = section.z().get();
        let x = section.vec_to_world(Vec3::new(d.x, d.y, 0.0));
        let at = section.point_to_world(Point3::new(a.x, a.y, 0.0));
        // the wall's +z is away from the material: y = +n puts it on the right of the profile
        let (origin, y) = match hand {
            Hand::Left => (at, n),
            Hand::Right => (at + n * width, -n),
        };
        let frame = Frame::from_zx(
            at,
            x.cross(y).normalize().ok_or(err(feature, ContourError::Empty))?,
            x,
        )
        .map(|f| f.with_origin(origin))
        .ok_or(err(feature, ContourError::Empty))?;
        let p = Point2::new;
        let c = [p(0.0, 0.0), p(len, 0.0), p(len, width), p(0.0, width)];
        let base = ProfileLoop::new(
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
        let (mut sheet, w0) = Sheet::face(feature, gauge, frame, &[base])?;
        let mut made = ContourMade {
            walls: vec![(piece, w0)],
            joints: Vec::new(),
        };
        chain(&mut sheet, w0, 1, &plan, &bends, 1, &mut made)?;
        Ok((sheet, made))
    }

    /// Runs `profile` off straight outline segment `segment` of `wall`, as long as the segment.
    /// The profile is in the edge's section: origin at the segment's start on the mold line, x
    /// out of the wall across the edge, y the wall's reference side; it starts at the origin and
    /// its material lies on its right (continuing the wall's).
    pub fn contour_on(
        &mut self,
        wall: WallId,
        segment: u64,
        profile: &[Segment],
        min_radius: f64,
    ) -> Result<ContourMade, SheetError> {
        let feature = self.feature;
        let first = match profile.first() {
            Some(Segment::Line { a, .. }) => *a,
            _ => {
                return Err(err(
                    feature,
                    match profile.first() {
                        None => ContourError::Empty,
                        Some(Segment::Spline(_)) => ContourError::Spline { piece: 0 },
                        Some(Segment::Circle { .. }) => ContourError::Closed { piece: 0 },
                        Some(_) => ContourError::ArcAtEnd { piece: 0 },
                    },
                ));
            }
        };
        let gap = first.distance(Point2::ORIGIN);
        if gap.is_nan() || gap > MEET {
            return Err(err(feature, ContourError::StartOffEdge { gap, limit: MEET }));
        }
        let plan = plan(profile, Some(Vec2::new(1.0, 0.0))).map_err(|w| err(feature, w))?;
        let default = self.gauge.radius;
        let bends =
            checked(&self.gauge, &plan, Hand::Right, default, min_radius).map_err(|w| err(feature, w))?;
        wall_lengths(&self.gauge, &plan, &bends, true).map_err(|w| err(feature, w))?;
        let mut made = ContourMade {
            walls: Vec::new(),
            joints: Vec::new(),
        };
        // all or nothing: a refusal part way leaves the sheet as it was
        let mut work = self.clone();
        chain(&mut work, wall, segment, &plan, &bends, 0, &mut made)?;
        *self = work;
        Ok(made)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(a: (f64, f64), b: (f64, f64)) -> Segment {
        Segment::Line {
            a: Point2::new(a.0, a.1),
            b: Point2::new(b.0, b.1),
        }
    }

    #[test]
    fn a_hat_plans_to_its_virtual_sharps() {
        // up 20, a quarter arc r 3 to the right, along 30, sharp down 20
        let r = 3.0;
        let profile = vec![
            line((0.0, 0.0), (0.0, 20.0 - r)),
            Segment::Arc {
                center: Point2::new(r, 20.0 - r),
                radius: r,
                start: dmath::PI,
                sweep: -dmath::PI / 2.0,
            },
            line((r, 20.0), (30.0, 20.0)),
            line((30.0, 20.0), (30.0, 0.0)),
        ];
        let p = plan(&profile, None).unwrap();
        assert_eq!(p.walls.len(), 3);
        assert!((p.walls[0].1 - 20.0).abs() < 1e-14);
        assert!((p.walls[1].1 - 30.0).abs() < 1e-14);
        assert!((p.walls[2].1 - 20.0).abs() < 1e-14);
        assert_eq!(p.bends.len(), 2);
        assert!((p.bends[0].1 + dmath::PI / 2.0).abs() < 1e-15);
        assert_eq!(p.bends[0].2, Some(r));
        assert_eq!(p.bends[1].2, None);
    }

    #[test]
    fn refusals_name_the_piece() {
        let arc = Segment::Arc {
            center: Point2::new(0.0, 5.0),
            radius: 5.0,
            start: -dmath::PI / 2.0,
            sweep: dmath::PI / 2.0,
        };
        assert_eq!(
            plan(std::slice::from_ref(&arc), None),
            Err(ContourError::ArcAtEnd { piece: 0 })
        );
        let not_tangent = vec![
            line((-10.0, 1.0), (0.0, 0.0)),
            arc.clone(),
            line((5.0, 5.0), (5.0, 10.0)),
        ];
        assert!(matches!(
            plan(&not_tangent, None),
            Err(ContourError::NotTangent { piece: 1, .. })
        ));
        let gap = vec![line((0.0, 0.0), (1.0, 0.0)), line((1.1, 0.0), (1.1, 5.0))];
        assert!(matches!(
            plan(&gap, None),
            Err(ContourError::Gap { piece: 1, .. })
        ));
        let two = vec![
            line((-10.0, 0.0), (0.0, 0.0)),
            arc.clone(),
            Segment::Arc {
                center: Point2::new(10.0, 5.0),
                radius: 5.0,
                start: dmath::PI,
                sweep: -dmath::PI / 2.0,
            },
            line((10.0, 10.0), (20.0, 10.0)),
        ];
        assert_eq!(plan(&two, None), Err(ContourError::ArcsMeet { piece: 2 }));
    }
}
