//! Lofts: ruled walls between two or more planar profiles, matched loop by loop and segment by
//! segment (each loop taken counter-clockwise about the loft's direction, from its first
//! segment). Each wall is the straight lines between matched points of two exact curves (the
//! same fraction along each): a plane where two lines' four ends are coplanar, a cylinder or
//! cone between coaxial arcs over the same angles, and otherwise a ruled surface defined by its
//! two curves.

use crate::profile::{Profile, ProfileError, Segment};
use crate::skin::{Skin, prepare, section_curve};
use forge_geom::ruled::{Rail, Ruled};
use forge_geom::skinned::{Skinned, Spline};
use forge_geom::{Circle, Cone, Curve, CurveGeom, Cylinder, Line, Plane, Surface};
use forge_math::{Frame, Interval, Point3, Vec3, dmath, tol};
use forge_topo::{Body, Name};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum LoftError {
    /// A loft needs two profiles at least.
    TooFew,
    Profile {
        index: usize,
        error: ProfileError,
    },
    /// Profile `index` has a different number of loops than the first (`loop_index` None), or
    /// its loop `loop_index` a different number of segments (a whole circle counts as two).
    Mismatch {
        index: usize,
        loop_index: Option<usize>,
        want: usize,
        got: usize,
    },
    /// Profiles `index` and `index + 1` meet, or lie on one plane: a ruling between them has
    /// no length.
    Degenerate {
        index: usize,
    },
    /// A wall between profiles `index` and `index + 1` folds over: its rulings cross.
    Twisted {
        index: usize,
    },
    /// The walls run into each other.
    SelfIntersecting,
}

/// How a loft runs between its profiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoftKind {
    /// Straight lines from each profile to the next.
    Ruled,
    /// A curvature-continuous blend through all the profiles (a cubic spline at each matched
    /// point, knots spaced by the distances between the profiles' centres). Free at an end, or
    /// leaving square to the end profile at `weight` times unit speed.
    Smooth { start: Option<f64>, end: Option<f64> },
}

/// Lofts through `profiles` in order. `feature` is the history id the names derive from (see
/// `skin`; walls are named by the first profile's segment ids).
pub fn loft(profiles: &[Profile], kind: LoftKind, feature: u64) -> Result<Body, LoftError> {
    let body = plan(profiles, kind)?.build(Name::feature(feature));
    if crate::selfx::crossing(&body, &vec![true; body.faces.len()]).is_some() {
        return Err(LoftError::SelfIntersecting);
    }
    Ok(body)
}

/// A box holding the loft through `profiles`, without building it.
pub fn loft_bounds(profiles: &[Profile], kind: LoftKind) -> Result<forge_math::Box3, LoftError> {
    let skin = plan(profiles, kind)?;
    let mut b = forge_math::Box3::EMPTY;
    for c in skin.curves.iter().flatten() {
        b = b.hull(&c.curve.bbox(c.range));
    }
    // a ruled wall lies in the hull of its two sections; a smooth one can bulge past them
    for w in skin.walls.iter().flatten() {
        if let Surface::Procedural(pr) = w {
            let d = pr.domain();
            b = b.hull(&pr.bbox(forge_math::Box2::new(d.u.interval(), d.v.interval())));
        }
    }
    Ok(b.grow(tol::LINEAR))
}

fn plan(profiles: &[Profile], kind: LoftKind) -> Result<Skin, LoftError> {
    let n = profiles.len();
    if n < 2 {
        return Err(LoftError::TooFew);
    }
    let centre = |p: &Profile| {
        let pts: Vec<Point3> = p.loops[0]
            .segments
            .iter()
            .flat_map(|(s, _)| s.samples(8))
            .map(|q| p.plane.plane_point(q))
            .collect();
        let sum = pts.iter().fold(Vec3::ZERO, |a, q| a + q.to_vec());
        Point3::ORIGIN + sum / pts.len().max(1) as f64
    };
    let mut preps = Vec::with_capacity(n);
    let mut centres = Vec::with_capacity(n);
    for (i, p) in profiles.iter().enumerate() {
        if p.loops.is_empty() {
            return Err(LoftError::Profile {
                index: i,
                error: ProfileError::Empty,
            });
        }
        // each profile faces the next; the last faces on from the one before
        let (a, b) = if i + 1 < n { (i, i + 1) } else { (i - 1, i) };
        let ahead = centre(&profiles[b]) - centre(&profiles[a]);
        if ahead.length() <= tol::LINEAR {
            return Err(LoftError::Degenerate { index: a });
        }
        centres.push(centre(p));
        preps.push(prepare(p, ahead).map_err(|error| LoftError::Profile { index: i, error })?);
    }
    let sizes = preps[0].loop_sizes();
    for (i, p) in preps.iter().enumerate().skip(1) {
        let s = p.loop_sizes();
        if s.len() != sizes.len() {
            return Err(LoftError::Mismatch {
                index: i,
                loop_index: None,
                want: sizes.len(),
                got: s.len(),
            });
        }
        if let Some(l) = (0..s.len()).find(|&l| s[l] != sizes[l]) {
            return Err(LoftError::Mismatch {
                index: i,
                loop_index: Some(l),
                want: sizes[l],
                got: s[l],
            });
        }
    }
    let segs: Vec<Vec<Segment>> = preps.iter().map(|p| p.segments().copied().collect()).collect();
    let planes: Vec<Frame> = preps.iter().map(|p| p.profile.plane).collect();
    for k in 0..n - 1 {
        for (sa, sb) in segs[k].iter().zip(&segs[k + 1]) {
            let (pa, pb) = (
                planes[k].plane_point(sa.start()),
                planes[k + 1].plane_point(sb.start()),
            );
            if pa.distance(pb) <= tol::MIN_LENGTH {
                return Err(LoftError::Degenerate { index: k });
            }
        }
    }
    let (rails, walls) = match kind {
        // through two profiles with free ends a spline is a straight line: ruled
        LoftKind::Smooth { start, end } if n > 2 || start.is_some() || end.is_some() => {
            smooth(&segs, &planes, &centres, &sizes, start, end)?
        }
        _ => ruled(&segs, &planes)?,
    };
    let curves = planes
        .iter()
        .zip(&segs)
        .map(|(f, ss)| ss.iter().map(|s| section_curve(s, f)).collect())
        .collect();
    Ok(Skin {
        keys: preps[0].keys.clone(),
        loops: sizes,
        planes,
        curves,
        rails,
        walls,
    })
}

type Walls = (Vec<Vec<(Curve, Interval)>>, Vec<Vec<Surface>>);

/// Straight rails and ruled walls between each pair of neighbouring profiles.
fn ruled(segs: &[Vec<Segment>], planes: &[Frame]) -> Result<Walls, LoftError> {
    let mut rails = Vec::with_capacity(planes.len() - 1);
    let mut walls = Vec::with_capacity(planes.len() - 1);
    for k in 0..planes.len() - 1 {
        let (fa, fb) = (&planes[k], &planes[k + 1]);
        let mut rk = Vec::new();
        for (sa, sb) in segs[k].iter().zip(&segs[k + 1]) {
            let (pa, pb) = (fa.plane_point(sa.start()), fb.plane_point(sb.start()));
            let line = Line::through(pa, pb).ok_or(LoftError::Degenerate { index: k })?;
            rk.push((Curve::Line(line), Interval::new(0.0, pa.distance(pb))));
        }
        rails.push(rk);
        walls.push(
            segs[k]
                .iter()
                .zip(&segs[k + 1])
                .map(|(sa, sb)| wall(rail(sa, fa), rail(sb, fb)).ok_or(LoftError::Twisted { index: k }))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    Ok((rails, walls))
}

/// Spline rails and skinned walls through all the profiles.
fn smooth(
    segs: &[Vec<Segment>],
    planes: &[Frame],
    centres: &[Point3],
    sizes: &[usize],
    start: Option<f64>,
    end: Option<f64>,
) -> Result<Walls, LoftError> {
    let n = planes.len();
    let mut knots = vec![0.0];
    for w in centres.windows(2) {
        knots.push(knots[knots.len() - 1] + w[0].distance(w[1]));
    }
    let spline = Arc::new(
        Spline::new(knots, start.is_some(), end.is_some()).ok_or(LoftError::Degenerate { index: 0 })?,
    );
    let slopes: Vec<Vec3> = [(start, 0), (end, n - 1)]
        .iter()
        .filter_map(|&(w, k)| w.map(|w| planes[k].z() * w))
        .collect();
    let nseg = segs[0].len();
    // the segment before each in its loop
    let mut prev = vec![0; nseg];
    let mut first = 0;
    for &len in sizes {
        for i in 0..len {
            prev[first + i] = first + (i + len - 1) % len;
        }
        first += len;
    }
    // across profile k at vertex i: between the normals of the two segments meeting there
    let across = |k: usize, i: usize| {
        let f = &planes[k];
        let right = |s: &Segment, at_end: bool| {
            let c = section_curve(s, f);
            let t = if c.forward == at_end {
                c.range.hi
            } else {
                c.range.lo
            };
            let d = c.curve.deriv(t).d1 * if c.forward { 1.0 } else { -1.0 };
            d.cross(f.z().get()).normalize().map_or(Vec3::ZERO, |u| u.get())
        };
        (right(&segs[k][prev[i]], true) + right(&segs[k][i], false))
            .normalize()
            .map_or(f.x().get(), |u| u.get())
    };
    let size = (0..n)
        .flat_map(|k| {
            segs[k]
                .iter()
                .map(move |s| planes[k].plane_point(s.start()).distance(centres[k]))
        })
        .fold(tol::LINEAR, f64::max);
    let dirs: Vec<Vec<Vec3>> = (0..nseg)
        .map(|i| (0..n).map(|m| across(m, i)).collect())
        .collect();
    let mut rails = Vec::with_capacity(n - 1);
    let mut walls = Vec::with_capacity(n - 1);
    for k in 0..n - 1 {
        let span = Interval::new(spline.knots()[k], spline.knots()[k + 1]);
        let wk = (0..nseg)
            .map(|j| {
                let rs: Vec<Rail> = segs.iter().zip(planes).map(|(sg, f)| rail(&sg[j], f)).collect();
                Skinned::new(rs, slopes.clone(), spline.clone(), k).ok_or(LoftError::Twisted { index: k })
            })
            .collect::<Result<Vec<_>, _>>()?;
        rails.push(
            (0..nseg)
                .map(|i| (wk[i].rail(0.0, &dirs[i], size), span))
                .collect(),
        );
        walls.push(wk.into_iter().map(Skinned::surface).collect());
    }
    Ok((rails, walls))
}

/// A segment drawn in plane `f`, by the fraction along it.
fn rail(s: &Segment, f: &Frame) -> Rail {
    match *s {
        Segment::Line { a, b } => Rail::Line {
            a: f.plane_point(a),
            b: f.plane_point(b),
        },
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => Rail::Arc {
            circle: Circle {
                frame: f.with_origin(f.plane_point(center)),
                radius,
            },
            start,
            sweep,
        },
        Segment::Circle { .. } => unreachable!("prepare splits circles"),
    }
}

/// The ruled wall between two matched curves; None where it folds over.
fn wall(ra: Rail, rb: Rail) -> Option<Surface> {
    match (ra, rb) {
        (Rail::Line { a: a0, b: a1 }, Rail::Line { a: b0, b: b1 }) => {
            // two in-plane directions that a planar quad never has parallel
            let along = (a1 - a0) + (b1 - b0);
            let across = (b0 - a0) + (b1 - a1);
            if let Some(n) = along.cross(across).normalize()
                && [a1, b0, b1]
                    .iter()
                    .all(|&p| n.dot(p - a0).abs() <= 1e-3 * tol::LINEAR)
            {
                // the normal (a' + s (b' - a')) x (b(t) - a(t)) is bilinear in t and s: its corners
                // decide whether it turns over (a bow tie, or a reflex corner the rulings fold at)
                let (da, db) = (a1 - a0, b1 - b0);
                let side = [(da, b0 - a0), (da, b1 - a1), (db, b0 - a0), (db, b1 - a1)]
                    .map(|(d, r)| n.dot(d.cross(r)));
                if !(side.iter().all(|&x| x > 0.0) || side.iter().all(|&x| x < 0.0)) {
                    return None;
                }
                return Some(Surface::Plane(Plane::new(Frame::from_zx(a0, n, along)?)));
            }
        }
        (
            Rail::Arc {
                circle: ca,
                start: sa,
                sweep: wa,
            },
            Rail::Arc {
                circle: cb,
                start: sb,
                sweep: wb,
            },
        ) => {
            let z = ca.frame.z();
            let off = cb.frame.origin - ca.frame.origin;
            let h = z.dot(off);
            let coaxial = z.cross(cb.frame.z()).length() <= tol::ANGULAR
                && z.dot(cb.frame.z()) > 0.0
                && (off - z * h).length() <= 1e-3 * tol::LINEAR
                && h > tol::LINEAR;
            let dir = |c: &Circle, a: f64| (c.eval(a) - c.frame.origin) / c.radius;
            let same =
                (dir(&ca, sa) - dir(&cb, sb)).length() <= tol::ANGULAR && (wa - wb).abs() <= tol::ANGULAR;
            if coaxial && same {
                let (r0, r1) = (ca.radius, cb.radius);
                if (r1 - r0).abs() <= 1e-3 * tol::LINEAR {
                    return Some(Surface::Cylinder(Cylinder {
                        frame: ca.frame,
                        radius: r0,
                    }));
                }
                // grows along z from the narrow end
                let cone = if r1 > r0 {
                    Cone::new(ca.frame, r0, dmath::atan((r1 - r0) / h))
                } else {
                    Cone::new(cb.frame.flipped(), r1, dmath::atan((r0 - r1) / h))
                };
                if let Some(c) = cone {
                    return Some(Surface::Cone(c));
                }
            }
        }
        _ => {}
    }
    Ruled::new(ra, rb).map(Ruled::surface)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ProfileLoop;
    use forge_math::Point2;
    use forge_topo::check;

    fn at(z: f64) -> Frame {
        Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z))
    }

    fn valid(b: &Body) {
        let problems = check(b);
        assert!(problems.is_empty(), "{problems:?}");
    }

    fn kinds(b: &Body) -> (usize, usize) {
        let ruled = b
            .faces
            .iter()
            .filter(|f| matches!(f.surface, Surface::Procedural(_)))
            .count();
        (b.faces.len(), ruled)
    }

    #[test]
    fn closed_forms_where_they_fit() {
        // a frustum of squares: planes; of circles: a cone
        let sq = |z: f64, s: f64| Profile {
            plane: at(z),
            loops: vec![Profile::rectangle(Point2::new(-s, -s), 2.0 * s, 2.0 * s, 0)],
        };
        let b = loft(&[sq(0.0, 4.0), sq(5.0, 2.0), sq(9.0, 3.0)], LoftKind::Ruled, 1).unwrap();
        valid(&b);
        assert_eq!(kinds(&b), (10, 0));
        let ci = |z: f64, r: f64| Profile {
            plane: at(z),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        };
        let b = loft(&[ci(0.0, 4.0), ci(6.0, 2.0)], LoftKind::Ruled, 1).unwrap();
        valid(&b);
        assert!(b.faces.iter().any(|f| matches!(f.surface, Surface::Cone(_))));
        assert_eq!(kinds(&b).1, 0);
    }

    #[test]
    fn twisted_and_offset_sections_are_ruled() {
        let sq = Profile {
            plane: at(0.0),
            loops: vec![Profile::rectangle(Point2::new(-3.0, -3.0), 6.0, 6.0, 0)],
        };
        let turned = Profile {
            plane: at(8.0).rotated_about_z(0.4),
            loops: vec![Profile::rectangle(Point2::new(-2.0, -2.0), 4.0, 4.0, 0)],
        };
        let b = loft(&[sq.clone(), turned], LoftKind::Ruled, 1).unwrap();
        valid(&b);
        assert_eq!(kinds(&b), (6, 4));
        // a circle to a shifted circle: an oblique cone, ruled
        let ci = |o: Point3, r: f64| Profile {
            plane: Frame::WORLD.with_origin(o),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        };
        let b = loft(
            &[ci(Point3::ORIGIN, 4.0), ci(Point3::new(2.0, 0.0, 6.0), 2.0)],
            LoftKind::Ruled,
            1,
        )
        .unwrap();
        valid(&b);
        assert_eq!(kinds(&b), (4, 2));
    }

    #[test]
    fn smooth_lofts_are_valid() {
        let sq = |z: f64, s: f64, turn: f64| Profile {
            plane: at(z).rotated_about_z(turn),
            loops: vec![Profile::rectangle(Point2::new(-s, -s), 2.0 * s, 2.0 * s, 0)],
        };
        let ci = |z: f64, r: f64| Profile {
            plane: at(z),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        };
        let free = LoftKind::Smooth {
            start: None,
            end: None,
        };
        let square_ends = LoftKind::Smooth {
            start: Some(1.0),
            end: Some(0.5),
        };
        for kind in [free, square_ends] {
            for profiles in [
                vec![sq(0.0, 4.0, 0.0), sq(5.0, 2.0, 0.2), sq(9.0, 3.0, 0.3)],
                vec![ci(0.0, 4.0), ci(5.0, 6.0), ci(10.0, 3.0), ci(14.0, 4.0)],
            ] {
                let b = loft(&profiles, kind, 1).unwrap();
                valid(&b);
                let walls = b.faces.len() - 2;
                assert_eq!(kinds(&b).1, walls);
                let bx = loft_bounds(&profiles, kind).unwrap();
                for v in &b.vertices {
                    assert!(bx.contains(v.point));
                }
            }
        }
    }

    #[test]
    fn mismatches_and_folds_are_refused() {
        let sq = Profile {
            plane: at(0.0),
            loops: vec![Profile::rectangle(Point2::new(-3.0, -3.0), 6.0, 6.0, 0)],
        };
        let tri = Profile {
            plane: at(5.0),
            loops: vec![ProfileLoop::new(vec![
                (
                    Segment::Line {
                        a: Point2::new(0.0, 0.0),
                        b: Point2::new(2.0, 0.0),
                    },
                    0,
                ),
                (
                    Segment::Line {
                        a: Point2::new(2.0, 0.0),
                        b: Point2::new(0.0, 2.0),
                    },
                    1,
                ),
                (
                    Segment::Line {
                        a: Point2::new(0.0, 2.0),
                        b: Point2::new(0.0, 0.0),
                    },
                    2,
                ),
            ])],
        };
        assert_eq!(
            loft(&[sq.clone(), tri], LoftKind::Ruled, 1).unwrap_err(),
            LoftError::Mismatch {
                index: 1,
                loop_index: Some(0),
                want: 4,
                got: 3
            }
        );
        assert_eq!(
            loft(std::slice::from_ref(&sq), LoftKind::Ruled, 1).unwrap_err(),
            LoftError::TooFew
        );
        // the far square turned half way round: every wall's rulings meet in the middle
        let half = Profile {
            plane: at(5.0).rotated_about_z(dmath::PI),
            loops: sq.loops.clone(),
        };
        assert_eq!(
            loft(&[sq, half], LoftKind::Ruled, 1).unwrap_err(),
            LoftError::Twisted { index: 0 }
        );
    }
}
