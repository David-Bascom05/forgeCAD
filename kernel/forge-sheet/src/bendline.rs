//! Sketched bends and jogs: a bend line drawn on a flat wall splits it in two, joined by a bend
//! along the line. The side right of the line (looking from its start to its end) turns; if the
//! wall hangs off a bend on that side, the line is read the other way so the turning side is
//! always the one away from the wall's own parent (the shape is the same either way).
//!
//! Where the bend lands against the line: `Centre` keeps the flat as drawn (the bend zone centred
//! on the line, so the folded part moves with K); a `Position` keeps the folded part (the line a
//! mold line, as a flange's edge: the flat moves with K). The bend zone is a rectangle, so the
//! outline must cross the line on two straight edges square to it across the band the two walls
//! give up; anything else is refused, typed. A jog is two such bends turning back, `offset`
//! between the reference skins of the walls either side.
//!
//! Cuts, hits, forms and hardware lying wholly beyond the line go with the turning wall.
use crate::error::{JointId, SheetError, WallId};
use crate::graph::{Joint, JointKind, Position, Sheet, Wall};
use forge_math::{Point2, Vec2, dmath, tol};
use forge_ops::{Profile, ProfileLoop, Segment};

/// Where a sketched bend lands against its line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BendAt {
    /// The zone centred on the line: the flat as drawn.
    Centre,
    /// The line as a flange's edge (`Sheet::flange_at`): the folded part as drawn.
    Position(Position),
}

/// Why a bend line is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BendLineError {
    /// The line's ends coincide.
    Line { length: f64, limit: f64 },
    /// The wall's outline crosses the line (at `at` from its start, along the normal) other than
    /// twice.
    Crossings { count: usize, at: f64 },
    /// A crossed outline segment not a line square to the bend line, or ending inside the band
    /// the two walls give up (`angle` from square; NaN for a curve).
    NotSquare { segment: u64, angle: f64 },
    /// A hole of the wall's own outline lies across the band.
    HoleAcross { segment: u64 },
    /// A bend already stands on a crossed edge.
    EdgeTaken { segment: u64, joint: JointId },
    /// A jog's middle wall leaves `length` between its bends' setbacks.
    JogTooShort { length: f64, limit: f64 },
}

/// A sketched bend's mold-line segment id, the same on both walls.
pub fn mold_segment(joint: JointId) -> u64 {
    (1 << 48) | joint.0 as u64
}

fn err(feature: crate::error::FeatureId, why: BendLineError) -> SheetError {
    SheetError::BendLine { feature, why }
}

/// The loop's segment ids with a point beyond u = c, and whether all of it is.
fn side_of(l: &ProfileLoop, u: &dyn Fn(Point2) -> f64, c: f64) -> (bool, bool) {
    let us: Vec<f64> = l
        .segments
        .iter()
        .flat_map(|(s, _)| s.samples(64))
        .map(|p| u(p) - c)
        .collect();
    (us.iter().any(|x| *x > 0.0), us.iter().all(|x| *x > 0.0))
}

impl Sheet {
    /// A bend along the line p0 -> p1 drawn on `wall`, turning the side right of it by `angle`
    /// (signed as `JointKind::Bend`) round inside radius `radius`. Returns the turned wall and
    /// the joint. All or nothing.
    #[allow(clippy::too_many_arguments)]
    pub fn sketched_bend(
        &mut self,
        wall: WallId,
        p0: Point2,
        p1: Point2,
        angle: f64,
        radius: f64,
        at: BendAt,
    ) -> Result<(WallId, JointId), SheetError> {
        let feature = self.feature;
        let t = self.gauge.thickness;
        let joint = JointId(self.next_joint);
        let w = self
            .wall(wall)
            .ok_or(SheetError::NoSuchWall { feature, wall })?
            .clone();
        if radius.is_nan() || radius <= tol::LINEAR {
            return Err(SheetError::Radius {
                feature,
                radius,
                limit: tol::LINEAR,
            });
        }
        let theta = angle.abs();
        if !(theta > tol::ANGULAR && theta < dmath::PI - tol::ANGULAR) {
            return Err(SheetError::BendAngle {
                feature,
                joint,
                angle,
                limit: if theta <= tol::ANGULAR {
                    tol::ANGULAR
                } else {
                    dmath::PI - tol::ANGULAR
                },
            });
        }
        let len = p0.distance(p1);
        if len.is_nan() || len <= tol::LINEAR {
            return Err(err(
                feature,
                BendLineError::Line {
                    length: len,
                    limit: tol::LINEAR,
                },
            ));
        }
        // the side the wall hangs from stays: read the line the other way if its parent is right
        let (mut p0, mut p1) = (p0, p1);
        let right = |p0: Point2, p1: Point2| {
            let d = (p1 - p0) * (1.0 / p0.distance(p1));
            Vec2::new(d.y, -d.x)
        };
        if let Some(parent) = self.joints.iter().find(|j| j.b == wall)
            && let Ok((a, b)) = crate::graph::straight(&w, parent.on_b)
            && (a.lerp(b, 0.5) - p0).dot(right(p0, p1)) > 0.0
        {
            std::mem::swap(&mut p0, &mut p1);
        }
        let n = right(p0, p1);
        let u = |p: Point2| (p - p0).dot(n);
        let ba = self
            .gauge
            .rule
            .allowance(t, radius, theta)
            .map_err(|why| SheetError::BendRule { feature, joint, why })?;
        let s = crate::build::setback_of(&self.gauge, radius, angle);
        // a's mold line, b's (as drawn), and how far b's outline moves along n
        let (la, lb, shift) = match at {
            BendAt::Centre => (s - 0.5 * ba, 0.5 * ba - s, 0.0),
            BendAt::Position(p) => {
                let sharp = t * dmath::sin(theta) / (1.0 + dmath::cos(theta));
                let d = match (p, angle > 0.0) {
                    (Position::Outside, true) | (Position::Inside, false) => 0.0,
                    (Position::Inside, true) => -sharp,
                    (Position::Outside, false) => sharp,
                    (Position::BendOutside, _) => s,
                };
                (d, d, ba - 2.0 * s)
            }
        };
        let (lo, hi) = (la.min(lb), la.max(lb));
        let outer = &w.outline[0];
        // the outline's crossings of both lines: two, on the same two square straight edges
        let mut crossed: Vec<usize> = Vec::new();
        for c in [la, lb] {
            let mut here = Vec::new();
            for (k, (seg, id)) in outer.segments.iter().enumerate() {
                let hit = match *seg {
                    Segment::Line { a, b } => {
                        (u(a) - c) * (u(b) - c) < 0.0 || (u(a) - c).abs() <= tol::LINEAR
                    }
                    _ => {
                        let pts = seg.samples(64);
                        pts.windows(2).any(|q| (u(q[0]) - c) * (u(q[1]) - c) <= 0.0)
                    }
                };
                if hit {
                    here.push(k);
                    let square = match *seg {
                        Segment::Line { a, b } => {
                            let e = (b - a) * (1.0 / a.distance(b));
                            let cross = e.cross(n).abs();
                            // its ends clear of the band
                            let (ua, ub) = (u(a), u(b));
                            let clear = ua.min(ub) < lo - tol::LINEAR && ua.max(ub) > hi + tol::LINEAR;
                            (cross <= 1e-12 && clear, dmath::asin(cross.min(1.0)))
                        }
                        _ => (false, f64::NAN),
                    };
                    if !square.0 {
                        return Err(err(
                            feature,
                            BendLineError::NotSquare {
                                segment: *id,
                                angle: square.1,
                            },
                        ));
                    }
                }
            }
            if here.len() != 2 {
                return Err(err(
                    feature,
                    BendLineError::Crossings {
                        count: here.len(),
                        at: c,
                    },
                ));
            }
            if crossed.is_empty() {
                crossed = here;
            }
        }
        // each wall reaches past the bend's setback from its mold line
        let us: Vec<f64> = outer
            .segments
            .iter()
            .flat_map(|(q, _)| q.samples(64))
            .map(u)
            .collect();
        let (umin, umax) = us.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |m, x| {
            (m.0.min(*x), m.1.max(*x))
        });
        for (id, length) in [(wall, la - umin - s), (WallId(self.next_wall), umax - lb - s)] {
            if length.is_nan() || length <= tol::LINEAR {
                return Err(SheetError::WallTooShort {
                    feature,
                    wall: id,
                    joint,
                    length,
                    limit: tol::LINEAR,
                });
            }
        }
        for &k in &crossed {
            let id = outer.segments[k].1;
            if let Some(j) = self
                .joints
                .iter()
                .find(|j| (j.a == wall && j.on_a == id) || (j.b == wall && j.on_b == id))
            {
                return Err(err(
                    feature,
                    BendLineError::EdgeTaken {
                        segment: id,
                        joint: j.id,
                    },
                ));
            }
        }
        // holes: wholly one side of the band
        let mut holes_a = Vec::new();
        let mut holes_b = Vec::new();
        for h in &w.outline[1..] {
            let (any_lo, _) = side_of(h, &u, lo - tol::LINEAR);
            let (_, all_hi) = side_of(h, &u, hi + tol::LINEAR);
            if all_hi {
                holes_b.push(h.clone());
            } else if !any_lo {
                holes_a.push(h.clone());
            } else {
                return Err(err(
                    feature,
                    BendLineError::HoleAcross {
                        segment: h.segments[0].1,
                    },
                ));
            }
        }
        let mold = mold_segment(joint);
        // each side's loop: from the edge leaving the other side, round, to the edge entering it,
        // closed by the mold line
        let piece = |c: f64, keep_low: bool| -> ProfileLoop {
            let nseg = outer.segments.len();
            // the crossed edge where the loop enters this side
            let enters = |k: usize| {
                let Segment::Line { b, .. } = outer.segments[k].0 else {
                    unreachable!("crossed edges are lines")
                };
                (u(b) < c) == keep_low
            };
            let (k_in, k_out) = if enters(crossed[0]) {
                (crossed[0], crossed[1])
            } else {
                (crossed[1], crossed[0])
            };
            let at_c = |k: usize| {
                let Segment::Line { a, b } = outer.segments[k].0 else {
                    unreachable!("crossed edges are lines")
                };
                let f = (c - u(a)) / (u(b) - u(a));
                a.lerp(b, f)
            };
            let (x_in, x_out) = (at_c(k_in), at_c(k_out));
            let mut segs = Vec::new();
            let Segment::Line { b: end_in, .. } = outer.segments[k_in].0 else {
                unreachable!()
            };
            segs.push((Segment::Line { a: x_in, b: end_in }, outer.segments[k_in].1));
            let mut k = (k_in + 1) % nseg;
            while k != k_out {
                segs.push(outer.segments[k].clone());
                k = (k + 1) % nseg;
            }
            let Segment::Line { a: start_out, .. } = outer.segments[k_out].0 else {
                unreachable!()
            };
            segs.push((
                Segment::Line {
                    a: start_out,
                    b: x_out,
                },
                outer.segments[k_out].1,
            ));
            segs.push((Segment::Line { a: x_out, b: x_in }, mold));
            ProfileLoop::new(segs)
        };
        let moved = |l: &ProfileLoop| {
            let by = n * shift;
            ProfileLoop::new(
                l.segments
                    .iter()
                    .map(|(s, id)| {
                        let s = match *s {
                            Segment::Line { a, b } => Segment::Line { a: a + by, b: b + by },
                            Segment::Arc {
                                center,
                                radius,
                                start,
                                sweep,
                            } => Segment::Arc {
                                center: center + by,
                                radius,
                                start,
                                sweep,
                            },
                            Segment::Circle { center, radius, ccw } => Segment::Circle {
                                center: center + by,
                                radius,
                                ccw,
                            },
                            Segment::Spline(ref ps) => {
                                let c =
                                    ps.c.mapped(|q| forge_math::Point3::new(q.x + by.x, q.y + by.y, 0.0));
                                let (degree, knots, pts, weights) = c.control_form();
                                let pts = pts.iter().map(|p| Point2::new(p.x, p.y)).collect();
                                match forge_geom::spline::PlaneSpline::control(
                                    degree,
                                    pts,
                                    Some(knots),
                                    weights,
                                ) {
                                    Some(c) => Segment::Spline(std::sync::Arc::new(c)),
                                    None => s.clone(),
                                }
                            }
                        };
                        (s, *id)
                    })
                    .collect(),
            )
        };
        let mut loops_a = vec![piece(la, true)];
        loops_a.extend(holes_a);
        let mut loops_b = vec![moved(&piece(lb, false))];
        loops_b.extend(holes_b.iter().map(&moved));
        let checked = |loops: Vec<ProfileLoop>, id: WallId| {
            Profile {
                plane: self.frame,
                loops,
            }
            .normalized()
            .map(|p| p.loops)
            .map_err(|why| SheetError::Profile {
                feature,
                wall: id,
                why,
            })
        };
        let b_id = WallId(self.next_wall);
        let (loops_a, loops_b) = (checked(loops_a, wall)?, checked(loops_b, b_id)?);
        let ids_b: Vec<u64> = loops_b[0].segments.iter().map(|s| s.1).collect();
        let mut work = self.clone();
        work.walls
            .iter_mut()
            .find(|x| x.id == wall)
            .expect("found above")
            .outline = loops_a;
        work.walls.push(Wall {
            id: b_id,
            outline: loops_b,
        });
        // bends already off the far side's edges now leave the new wall
        for j in &mut work.joints {
            if j.a == wall
                && ids_b.contains(&j.on_a)
                && !crossed.iter().any(|&k| outer.segments[k].1 == j.on_a)
            {
                j.a = b_id;
            }
        }
        work.joints.push(Joint {
            id: joint,
            a: wall,
            b: b_id,
            on_a: mold,
            on_b: mold,
            kind: JointKind::Bend { angle, radius },
        });
        // what lies wholly beyond the line goes with the turning wall
        let beyond = |p: Point2| u(p) > hi + tol::LINEAR;
        let by = n * shift;
        let mut moved_cuts = Vec::new();
        for c in &mut work.cuts {
            if c.wall == wall && c.lp.segments.iter().flat_map(|(s, _)| s.samples(16)).all(beyond) {
                c.wall = b_id;
                c.lp = moved(&c.lp);
                moved_cuts.push(c.id);
            }
        }
        for p in &mut work.punches {
            if p.wall == wall && p.cuts.iter().all(|c| moved_cuts.contains(c)) && !p.cuts.is_empty() {
                p.wall = b_id;
                p.at += by;
            }
        }
        for m in &mut work.mounts {
            if m.wall == wall && moved_cuts.contains(&m.cut) {
                m.wall = b_id;
                m.at += by;
            }
        }
        for f in &mut work.forms {
            let r = f.tool.shape.rim(t).unwrap_or(0.0);
            if f.wall == wall && u(f.at) - r > hi + tol::LINEAR {
                f.wall = b_id;
                f.at += by;
            }
        }
        work.next_wall += 1;
        work.next_joint += 1;
        *self = work;
        Ok((b_id, joint))
    }

    /// A jog on `wall` along the line p0 -> p1: a sketched bend there by `angle`, and one back by
    /// -`angle` further on, so the wall beyond stands `offset` (between reference skins) off
    /// this one. Returns the middle wall, its joint, the far wall and its joint.
    #[allow(clippy::too_many_arguments)]
    pub fn jog(
        &mut self,
        wall: WallId,
        p0: Point2,
        p1: Point2,
        offset: f64,
        angle: f64,
        radius: f64,
        at: BendAt,
    ) -> Result<(WallId, JointId, WallId, JointId), SheetError> {
        let feature = self.feature;
        let mut work = self.clone();
        let (mid, j1) = work.sketched_bend(wall, p0, p1, angle, radius, at)?;
        let s1 = crate::build::setback_of(&work.gauge, radius, angle);
        let s2 = crate::build::setback_of(&work.gauge, radius, -angle);
        let m = offset / dmath::sin(angle.abs());
        if (m - s1 - s2).is_nan() || m - s1 - s2 <= tol::LINEAR {
            return Err(err(
                feature,
                BendLineError::JogTooShort {
                    length: m - s1 - s2,
                    limit: tol::LINEAR,
                },
            ));
        }
        // the middle wall's mold line, in its coordinates, and the second line m on from it
        let w = work.wall(mid).expect("just made");
        let (a, b) = crate::graph::straight(w, mold_segment(j1)).expect("its mold line");
        // the middle wall's mold line runs with it on its left: its right is back towards `wall`
        let d = (b - a) * (1.0 / a.distance(b));
        let inward = Vec2::new(-d.y, d.x);
        let (q0, q1) = (b + inward * m, a + inward * m);
        // the line as the mold line: Outside for a bend up, Inside for one down
        let on = if angle < 0.0 {
            Position::Outside
        } else {
            Position::Inside
        };
        let (far, j2) = work.sketched_bend(mid, q0, q1, -angle, radius, BendAt::Position(on))?;
        *self = work;
        Ok((mid, j1, far, j2))
    }
}
