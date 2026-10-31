//! Lofts: ruled walls between two or more planar profiles, matched loop by loop and segment by
//! segment (each loop taken counter-clockwise about the loft's direction, from its first
//! segment). Each wall is the straight lines between matched points of two exact curves (the
//! same fraction along each): a plane where two lines' four ends are coplanar, a cylinder or
//! cone between coaxial arcs over the same angles, and otherwise a ruled surface defined by its
//! two curves.

use crate::profile::{Profile, ProfileError, Segment};
use crate::skin::{Skin, prepare, section_curve};
use forge_geom::ruled::{Rail, Ruled};
use forge_geom::{Circle, Cone, Curve, CurveGeom, Cylinder, Line, Plane, Surface};
use forge_math::{Frame, Interval, Point3, Vec3, dmath, tol};
use forge_topo::{Body, Name};

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

/// Lofts through `profiles` in order with ruled walls. `feature` is the history id the names
/// derive from (see `skin`; walls are named by the first profile's segment ids).
pub fn loft(profiles: &[Profile], feature: u64) -> Result<Body, LoftError> {
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
    let mut rails = Vec::with_capacity(n - 1);
    let mut walls = Vec::with_capacity(n - 1);
    for k in 0..n - 1 {
        let (fa, fb) = (&planes[k], &planes[k + 1]);
        let mut rk = Vec::new();
        for (sa, sb) in segs[k].iter().zip(&segs[k + 1]) {
            let (pa, pb) = (fa.plane_point(sa.start()), fb.plane_point(sb.start()));
            let line = Line::through(pa, pb).ok_or(LoftError::Degenerate { index: k })?;
            if pa.distance(pb) <= tol::MIN_LENGTH {
                return Err(LoftError::Degenerate { index: k });
            }
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
    let curves = planes
        .iter()
        .zip(&segs)
        .map(|(f, ss)| ss.iter().map(|s| section_curve(s, f)).collect())
        .collect();
    let first = &preps[0];
    let body = Skin {
        keys: first.keys.clone(),
        loops: sizes,
        planes,
        curves,
        rails,
        walls,
    }
    .build(Name::feature(feature));
    if crate::selfx::crossing(&body, &vec![true; body.faces.len()]).is_some() {
        return Err(LoftError::SelfIntersecting);
    }
    Ok(body)
}

/// A box holding the loft through `profiles`: a ruled wall lies in the hull of its two sections.
pub fn loft_bounds(profiles: &[Profile]) -> Result<forge_math::Box3, LoftError> {
    if profiles.len() < 2 {
        return Err(LoftError::TooFew);
    }
    let mut b = forge_math::Box3::EMPTY;
    for p in profiles {
        for l in &p.loops {
            for (s, _) in &l.segments {
                let curve = match *s {
                    Segment::Circle { center, radius, .. } => section_curve(
                        &Segment::Arc {
                            center,
                            radius,
                            start: 0.0,
                            sweep: dmath::TAU - 1e-9,
                        },
                        &p.plane,
                    ),
                    _ => section_curve(s, &p.plane),
                };
                b = b.hull(&curve.curve.bbox(curve.range));
            }
        }
    }
    Ok(b.grow(tol::LINEAR))
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
        let b = loft(&[sq(0.0, 4.0), sq(5.0, 2.0), sq(9.0, 3.0)], 1).unwrap();
        valid(&b);
        assert_eq!(kinds(&b), (10, 0));
        let ci = |z: f64, r: f64| Profile {
            plane: at(z),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        };
        let b = loft(&[ci(0.0, 4.0), ci(6.0, 2.0)], 1).unwrap();
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
        let b = loft(&[sq.clone(), turned], 1).unwrap();
        valid(&b);
        assert_eq!(kinds(&b), (6, 4));
        // a circle to a shifted circle: an oblique cone, ruled
        let ci = |o: Point3, r: f64| Profile {
            plane: Frame::WORLD.with_origin(o),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        };
        let b = loft(&[ci(Point3::ORIGIN, 4.0), ci(Point3::new(2.0, 0.0, 6.0), 2.0)], 1).unwrap();
        valid(&b);
        assert_eq!(kinds(&b), (4, 2));
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
            loft(&[sq.clone(), tri], 1).unwrap_err(),
            LoftError::Mismatch {
                index: 1,
                loop_index: Some(0),
                want: 4,
                got: 3
            }
        );
        assert_eq!(loft(std::slice::from_ref(&sq), 1).unwrap_err(), LoftError::TooFew);
        // the far square turned half way round: every wall's rulings meet in the middle
        let half = Profile {
            plane: at(5.0).rotated_about_z(dmath::PI),
            loops: sq.loops.clone(),
        };
        assert_eq!(loft(&[sq, half], 1).unwrap_err(), LoftError::Twisted { index: 0 });
    }
}
