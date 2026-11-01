//! Lofts: ruled walls between two or more planar profiles, matched loop by loop and segment by
//! segment (each loop taken counter-clockwise about the loft's direction, from its first
//! segment). Each wall is the straight lines between matched points of two exact curves (the
//! same fraction along each): a plane where two lines' four ends are coplanar, a cylinder or
//! cone between coaxial arcs over the same angles, and otherwise a ruled surface defined by its
//! two curves.

use crate::profile::{Profile, ProfileError, Segment};
use crate::skin::{Skin, prepare, section_curve};
use forge_geom::ruled::{Rail, Ruled};
use forge_geom::skinned::{Along, GuidePath, Shift, Skinned, Spline};
use forge_geom::{Circle, Cone, Curve, CurveGeom, Cylinder, Line, Plane, Surface};
use forge_math::{Frame, Interval, Point2, Point3, Vec3, dmath, tol};
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
    /// A loft ending in a point takes no rails or centre line.
    ApexWithGuides,
    /// A loft ending in a point has one loop per profile: holes have nothing to close on.
    ApexWithHoles,
    /// A dome needs a smooth loft.
    DomeNeedsSmooth,
    /// A dome's direction must be square to the profile next to it.
    DomeNotSquare,
    /// A wall between profiles `index` and `index + 1` folds over: its rulings cross.
    Twisted {
        index: usize,
    },
    /// The walls run into each other.
    SelfIntersecting,
    /// Guide rails and centre lines shape only a smooth loft.
    GuidesNeedSmooth,
    /// Guide `guide` is not a tangent-continuous chain of lines and arcs.
    GuideBroken {
        guide: usize,
        error: crate::SweepError,
    },
    /// Guide `guide` misses profile `profile` (a rail its vertex, a centre line its plane), or
    /// meets them out of order.
    GuideMisses {
        guide: usize,
        profile: usize,
    },
    /// A rail names a vertex the profiles don't have.
    NoVertex {
        guide: usize,
    },
}

/// What shapes a smooth loft between its profiles, besides them.
#[derive(Clone, Debug, PartialEq)]
pub enum LoftGuides {
    None,
    /// Rails the loft's edges follow: each through one vertex of every profile (vertex i is
    /// where segment i starts, counting through the first profile's loops in order).
    Rails(Vec<(usize, Vec<crate::PathSeg>)>),
    /// A path the loft follows between the profiles, crossing each profile's plane in turn.
    CentreLine(Vec<crate::PathSeg>),
}

/// A loft's end collapsing to a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Apex {
    pub point: Point3,
    /// A rounded nose on a smooth loft: walls arrive square to this direction (the section's
    /// plane must be square to it too), the widest at `dome.1` times unit speed; None: sharp.
    pub dome: Option<(Vec3, f64)>,
}

/// What a loft does at its ends and how it lines up its sections.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoftOptions {
    /// The start and the end closing on a point.
    pub apex: [Option<Apex>; 2],
    /// Start each section where it comes nearest the start of the one before: what Parasolid does
    /// (research/spline-loops/README.md), so a circle's seam follows, and walls take the twist that
    /// gives. Off: each section starts as written.
    pub align: bool,
    /// Where a profile's outer loop starts, in its plane (the nearest point of a circle, or the nearest
    /// vertex), per profile.
    pub seams: Vec<Option<Point2>>,
}

/// How a loft runs between its profiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoftKind {
    /// Straight lines from each profile to the next.
    Ruled,
    /// A curvature-continuous blend through all the profiles (a cubic spline at each matched
    /// point, knots spaced as a fit to Parasolid's: see research/spline-loops). Free at an end, or
    /// leaving square to the end profile at `weight` times unit speed.
    Smooth { start: Option<f64>, end: Option<f64> },
}

/// Lofts through `profiles` in order. `feature` is the history id the names derive from (see
/// `skin`; walls are named by the first profile's segment ids).
pub fn loft(
    profiles: &[Profile],
    kind: LoftKind,
    guides: &LoftGuides,
    feature: u64,
) -> Result<Body, LoftError> {
    Ok(loft_and_bounds(profiles, kind, guides, &LoftOptions::default(), feature)?.0)
}

/// `loft` and `loft_bounds` from one plan.
pub fn loft_and_bounds(
    profiles: &[Profile],
    kind: LoftKind,
    guides: &LoftGuides,
    options: &LoftOptions,
    feature: u64,
) -> Result<(Body, forge_math::Box3), LoftError> {
    let skin = plan(profiles, kind, guides, options)?;
    let b = skin_bounds(&skin);
    let body = skin.build(Name::feature(feature));
    if crate::selfx::crossing(&body, &vec![true; body.faces.len()]).is_some() {
        return Err(LoftError::SelfIntersecting);
    }
    Ok((body, b))
}

/// A box holding the loft through `profiles`, without building it.
pub fn loft_bounds(
    profiles: &[Profile],
    kind: LoftKind,
    guides: &LoftGuides,
    options: &LoftOptions,
) -> Result<forge_math::Box3, LoftError> {
    Ok(skin_bounds(&plan(profiles, kind, guides, options)?))
}

fn skin_bounds(skin: &Skin) -> forge_math::Box3 {
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
    b.grow(tol::LINEAR)
}

fn plan(
    profiles: &[Profile],
    kind: LoftKind,
    guides: &LoftGuides,
    options: &LoftOptions,
) -> Result<Skin, LoftError> {
    let apex = &options.apex;
    let points = usize::from(apex[0].is_some()) + usize::from(apex[1].is_some());
    if profiles.len() + points < 2 || profiles.is_empty() {
        return Err(LoftError::TooFew);
    }
    if points > 0 && *guides != LoftGuides::None {
        return Err(LoftError::ApexWithGuides);
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
    let empty = profiles.iter().position(|p| p.loops.is_empty());
    if let Some(index) = empty {
        return Err(LoftError::Profile {
            index,
            error: ProfileError::Empty,
        });
    }
    // sections in order: the apexes, where there are any, at the ends
    let first = usize::from(apex[0].is_some());
    let mut centres: Vec<Point3> = Vec::with_capacity(profiles.len() + points);
    centres.extend(apex[0].map(|a| a.point));
    centres.extend(profiles.iter().map(centre));
    centres.extend(apex[1].map(|a| a.point));
    let n = centres.len();
    let mut preps = Vec::with_capacity(profiles.len());
    for (i, p) in profiles.iter().enumerate() {
        let k = first + i;
        // each section faces the next; the last faces on from the one before
        let (a, b) = if k + 1 < n { (k, k + 1) } else { (k - 1, k) };
        let ahead = centres[b] - centres[a];
        if ahead.length() <= tol::LINEAR {
            return Err(LoftError::Degenerate { index: a });
        }
        preps.push(prepare(p, ahead).map_err(|error| LoftError::Profile { index: i, error })?);
    }
    if points > 0 && preps.iter().any(|p| p.profile.loops.len() != 1) {
        return Err(LoftError::ApexWithHoles);
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
    let mut segs: Vec<Vec<Segment>> = preps.iter().map(|p| p.segments().cloned().collect()).collect();
    let mut planes: Vec<Frame> = preps.iter().map(|p| p.profile.plane).collect();
    if options.align || options.seams.iter().any(Option::is_some) {
        let world = |k: usize, q: Point2| profiles[k].plane.plane_point(q);
        for k in 0..segs.len() {
            // a seam asked for, else the last section's own where the sections line up
            let mut first = 0;
            for &len in &sizes {
                let asked = options.seams.get(k).copied().flatten().filter(|_| first == 0);
                if let Some(q) = asked {
                    start_at(&mut segs[k][first..first + len], &planes[k], world(k, q));
                } else if options.align && k > 0 {
                    let (before, after) = segs.split_at_mut(k);
                    let last = &before[k - 1][first..first + len];
                    let this = &mut after[0][first..first + len];
                    let start = (k > 1).then(|| (&before[0][first..first + len], &planes[0]));
                    if !line_up(this, &planes[k], last, &planes[k - 1], start) {
                        let t = planes[k - 1].plane_point(last[0].start());
                        start_at(this, &planes[k], t);
                    }
                }
                first += len;
            }
        }
    }
    // an apex as a section of no size: each segment shrunk onto the point, in the neighbour's plane
    let at_point = |near: &[Segment]| -> Vec<Segment> {
        near.iter()
            .map(|s| match *s {
                Segment::Arc { start, sweep, .. } => Segment::Arc {
                    center: Point2::ORIGIN,
                    radius: 0.0,
                    start,
                    sweep,
                },
                _ => Segment::Line {
                    a: Point2::ORIGIN,
                    b: Point2::ORIGIN,
                },
            })
            .collect()
    };
    if let Some(a) = apex[1] {
        let far = at_point(&segs[segs.len() - 1]);
        segs.push(far);
        planes.push(planes[planes.len() - 1].with_origin(a.point));
    }
    if let Some(a) = apex[0] {
        let near = at_point(&segs[0]);
        segs.insert(0, near);
        planes.insert(0, planes[0].with_origin(a.point));
    }
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
    let guided = *guides != LoftGuides::None;
    let domed = apex.iter().flatten().any(|a| a.dome.is_some());
    let is_apex = [apex[0].is_some(), apex[1].is_some()];
    if domed && !matches!(kind, LoftKind::Smooth { .. }) {
        return Err(LoftError::DomeNeedsSmooth);
    }
    let (rails, walls) = match kind {
        // through two profiles with free ends and nothing guiding, a spline is a straight line
        LoftKind::Smooth { start, end } if n > 2 || start.is_some() || end.is_some() || guided || domed => {
            smooth(&segs, &planes, &centres, &sizes, start, end, guides, apex)?
        }
        LoftKind::Smooth { .. } => ruled(&segs, &planes)?,
        LoftKind::Ruled if guided => return Err(LoftError::GuidesNeedSmooth),
        LoftKind::Ruled => ruled(&segs, &planes)?,
    };
    let curves = planes
        .iter()
        .zip(&segs)
        .enumerate()
        .map(|(k, (f, ss))| {
            if (k == 0 && is_apex[0]) || (k == n - 1 && is_apex[1]) {
                Vec::new()
            } else {
                ss.iter().map(|s| section_curve(s, f)).collect()
            }
        })
        .collect();
    Ok(Skin {
        keys: preps[0].keys.clone(),
        loops: sizes,
        planes,
        curves,
        rails,
        walls,
        apex: is_apex,
    })
}

/// A loop rotated so its vertices, each about the loop's middle, lie nearest the last section's in the
/// least squares sense; false for a circle's two halves, whose start the last section's point settles.
fn line_up(
    this: &mut [Segment],
    plane: &Frame,
    last: &[Segment],
    before: &Frame,
    start: Option<(&[Segment], &Frame)>,
) -> bool {
    let round = |l: &[Segment]| matches!(l, [Segment::Arc { center: a, .. }, Segment::Arc { center: b, .. }] if a.distance(*b) <= tol::LINEAR);
    if round(this) {
        return false;
    }
    let around = |l: &[Segment], f: &Frame| -> Vec<Vec3> {
        let p: Vec<Point3> = l.iter().map(|s| f.plane_point(s.start())).collect();
        let c = p.iter().fold(Vec3::ZERO, |a, q| a + q.to_vec()) / p.len() as f64;
        p.iter().map(|q| q.to_vec() - c).collect()
    };
    let (a, b) = (around(last, before), around(this, plane));
    let n = b.len();
    let cost = |r: usize| (0..n).map(|i| (a[i] - b[(i + r) % n]).length_sq()).sum::<f64>();
    let least = (0..n).map(cost).fold(f64::INFINITY, f64::min);
    // two rotations alike (a hexagon turned 30 degrees): Parasolid's rulings turn forward about the way
    // the loft goes, and come back to the first section's start (three parts of CADFS, research/spline-loops)
    let centre = |l: &[Segment], f: &Frame| {
        l.iter()
            .fold(Vec3::ZERO, |a, s| a + f.plane_point(s.start()).to_vec())
            / l.len() as f64
    };
    let go = centre(this, plane) - centre(last, before);
    let mut ties: Vec<usize> = (0..n)
        .filter(|&r| cost(r) <= least * (1.0 + 1e-9) + 1e-12)
        .collect();
    if let (true, Some((first, f))) = (ties.len() > 1, start) {
        let home = around(first, f)[0];
        let gap = |r: usize| (home - b[r]).length();
        let near = ties.iter().map(|&r| gap(r)).fold(f64::INFINITY, f64::min);
        ties.retain(|&r| gap(r) <= near + 1e-9);
    }
    let best = ties
        .into_iter()
        .max_by(|&x, &y| {
            let turn = |r: usize| a[0].cross(b[r]).dot(go);
            turn(x).total_cmp(&turn(y))
        })
        .unwrap_or(0);
    this.rotate_left(best);
    true
}

/// A loop of a section starting where it comes nearest `target`: the two halves of a circle turned
/// round to it, or the segments rotated to the nearest vertex.
fn start_at(l: &mut [Segment], plane: &Frame, target: Point3) {
    let q = plane.point_to_local(target);
    let q = Point2::new(q.x, q.y);
    if let [
        Segment::Arc {
            center: c0,
            radius: r0,
            sweep: w0,
            ..
        },
        Segment::Arc {
            center: c1,
            radius: r1,
            sweep: w1,
            ..
        },
    ] = *l
        && c0.distance(c1) <= tol::LINEAR
        && (r0 - r1).abs() <= tol::LINEAR
        && (w0.abs() + w1.abs() - dmath::TAU).abs() <= tol::ANGULAR
        && w0 * w1 > 0.0
    {
        let d = q - c0;
        if d.length_sq() > 0.0 {
            let a = dmath::atan2(d.y, d.x);
            l[0] = Segment::Arc {
                center: c0,
                radius: r0,
                start: a,
                sweep: w0,
            };
            l[1] = Segment::Arc {
                center: c1,
                radius: r1,
                start: a + w0,
                sweep: w1,
            };
        }
        return;
    }
    let near = (0..l.len())
        .min_by(|&i, &j| l[i].start().distance(q).total_cmp(&l[j].start().distance(q)))
        .unwrap_or(0);
    l.rotate_left(near);
}

type Walls = (Vec<Vec<(Curve, Interval)>>, Vec<Vec<Surface>>);

/// A guide's path, where it crosses each profile, and the points there it pulls from.
type Pull = (Arc<GuidePath>, Vec<f64>, Vec<Point3>);

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
#[allow(clippy::too_many_arguments)]
fn smooth(
    segs: &[Vec<Segment>],
    planes: &[Frame],
    centres: &[Point3],
    sizes: &[usize],
    start: Option<f64>,
    end: Option<f64>,
    guides: &LoftGuides,
    apex: &[Option<Apex>; 2],
) -> Result<Walls, LoftError> {
    let n = planes.len();
    let mut knots = vec![0.0];
    for k in 0..n - 1 {
        // the tenth percentile of the chords between matched points: a fit to the spacing Parasolid's surfaces show,
        // not its rule, which is not known (research/spline-loops has the residual)
        let mut chords: Vec<f64> = segs[k]
            .iter()
            .zip(&segs[k + 1])
            .flat_map(|(a, b)| {
                let (pa, pb) = (a.samples(16), b.samples(16));
                let (fa, fb) = (planes[k], planes[k + 1]);
                pa.into_iter()
                    .zip(pb)
                    .map(move |(p, q)| fa.plane_point(p).distance(fb.plane_point(q)))
                    .collect::<Vec<f64>>()
            })
            .collect();
        chords.sort_by(f64::total_cmp);
        let near = chords.get(chords.len() / 10).copied().unwrap_or(0.0);
        let d = if near > tol::LINEAR {
            near
        } else {
            centres[k].distance(centres[k + 1])
        };
        knots.push(knots[k] + d);
    }
    let dome = |k: usize| apex[k].and_then(|a| a.dome);
    let spline = Arc::new(
        Spline::new(
            knots.clone(),
            start.is_some() || dome(0).is_some(),
            end.is_some() || dome(1).is_some(),
        )
        .ok_or(LoftError::Degenerate { index: 0 })?,
    );
    // a fixed slope, or at a dome a zero one the wall's ramp stands in for
    let slopes: Vec<Vec3> = [(start, 0, 0), (end, n - 1, 1)]
        .iter()
        .filter_map(|&(w, k, e)| {
            if dome(e).is_some() {
                Some(Vec3::ZERO)
            } else {
                w.map(|w| planes[k].z() * w)
            }
        })
        .collect();
    // the slope along a dome's section: towards or from the point under the apex
    let ramp = |e: usize, j: usize| -> Result<Option<Rail>, LoftError> {
        let Some((dir, w)) = dome(e) else {
            return Ok(None);
        };
        let at = if e == 0 { 1 } else { n - 2 };
        let f = &planes[at];
        let d = dir.normalize().ok_or(LoftError::DomeNotSquare)?;
        if d.cross(f.z().get()).length() > tol::ANGULAR {
            return Err(LoftError::DomeNotSquare);
        }
        let tip = centres[if e == 0 { 0 } else { n - 1 }];
        let foot = tip - f.z() * f.z().dot(tip - f.origin);
        let widest = segs[at]
            .iter()
            .flat_map(|s| s.samples(8))
            .map(|q| f.plane_point(q).distance(foot))
            .fold(tol::LINEAR, f64::max);
        // the wall leaves the apex outward; it comes back in at the far end
        let k = w / widest * if e == 0 { 1.0 } else { -1.0 };
        let v = |p: Point3| Point3::ORIGIN + (p - foot) * k;
        Ok(Some(match rail(&segs[at][j], f) {
            Rail::Curve(c) => Rail::Curve(Arc::new(c.mapped(v))),
            Rail::Line { a, b } => Rail::Line { a: v(a), b: v(b) },
            Rail::Arc { circle, start, sweep } => {
                // a negative scale is the half turn it stands for
                Rail::Arc {
                    circle: Circle {
                        frame: circle.frame.with_origin(v(circle.frame.origin)),
                        radius: circle.radius * k.abs(),
                    },
                    start: if k >= 0.0 { start } else { start + dmath::PI },
                    sweep,
                }
            }
            other => other,
        }))
    };
    let nseg = segs[0].len();
    // each guide: its path, where it crosses the profiles, and the points there it pulls from
    let vertex = |k: usize, i: usize| planes[k].plane_point(segs[k][i].start());
    let pulls = |g: usize, path: &[crate::PathSeg], at: Option<usize>| -> Result<Pull, LoftError> {
        crate::sweep::check_path(path).map_err(|error| LoftError::GuideBroken { guide: g, error })?;
        if let Some(index) = path.iter().position(|s| matches!(s, crate::PathSeg::Spline(_))) {
            return Err(LoftError::GuideBroken {
                guide: g,
                error: crate::SweepError::CurveGuide { index },
            });
        }
        let pieces: Vec<Rail> = path
            .iter()
            .map(|s| match *s {
                crate::PathSeg::Line { a, b } => Rail::Line { a, b },
                crate::PathSeg::Arc { circle, start, sweep } => Rail::Arc { circle, start, sweep },
                crate::PathSeg::Spline(_) => unreachable!("refused above"),
            })
            .collect();
        let gp = Arc::new(GuidePath::new(&pieces));
        let (mut ps, mut vals) = (Vec::with_capacity(n), Vec::with_capacity(n));
        let mut last = f64::NEG_INFINITY;
        for (k, f) in planes.iter().enumerate() {
            let miss = LoftError::GuideMisses { guide: g, profile: k };
            let p = gp
                .crossings(f.origin, f.z().get())
                .into_iter()
                .filter(|&p| p > last + tol::LINEAR)
                .find(|&p| at.is_none_or(|i| gp.at(p).0.distance(vertex(k, i)) <= tol::LINEAR))
                .ok_or(miss)?;
            last = p;
            ps.push(p);
            vals.push(at.map_or(gp.at(p).0, |i| vertex(k, i)));
        }
        Ok((gp, ps, vals))
    };
    // per vertex, the pull a rail there makes; and the centre line's
    let mut at_vertex: Vec<Option<Pull>> = vec![None; nseg];
    let mut whole = None;
    match guides {
        LoftGuides::None => {}
        LoftGuides::Rails(rails) => {
            for (g, (i, path)) in rails.iter().enumerate() {
                if *i >= nseg {
                    return Err(LoftError::NoVertex { guide: g });
                }
                at_vertex[*i] = Some(pulls(g, path, Some(*i))?);
            }
        }
        LoftGuides::CentreLine(path) => whole = Some(pulls(0, path, None)?),
    }
    let shift = |(gp, ps, vals): &Pull, along: Along| {
        Shift::new(gp.clone(), ps.clone(), vals.clone(), &knots, along)
            .ok_or(LoftError::Degenerate { index: 0 })
    };
    // the segment before each in its loop
    let (mut prev, mut next) = (vec![0; nseg], vec![0; nseg]);
    let mut first = 0;
    for &len in sizes {
        for i in 0..len {
            prev[first + i] = first + (i + len - 1) % len;
            next[first + i] = first + (i + 1) % len;
        }
        first += len;
    }
    let mut shifts: Vec<Vec<Shift>> = Vec::with_capacity(nseg);
    for j in 0..nseg {
        let mut sj = Vec::new();
        if let Some(pull) = &at_vertex[j] {
            sj.push(shift(pull, Along::Start)?);
        }
        if let Some(pull) = &at_vertex[next[j]] {
            sj.push(shift(pull, Along::End)?);
        }
        if let Some(pull) = &whole {
            sj.push(shift(pull, Along::Whole)?);
        }
        shifts.push(sj);
    }
    // across profile k at vertex i: between the normals of the two segments meeting there
    let across = |k: usize, i: usize| {
        // an apex has no curves to read: as its neighbour's
        let k = match (k, apex[0].is_some(), apex[1].is_some()) {
            (0, true, _) => 1,
            (m, _, true) if m == n - 1 => n - 2,
            (m, ..) => m,
        };
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
                let mut ramps = Vec::with_capacity(slopes.len());
                if start.is_some() || dome(0).is_some() {
                    ramps.push(ramp(0, j)?);
                }
                if end.is_some() || dome(1).is_some() {
                    ramps.push(ramp(1, j)?);
                }
                Skinned::ramped(rs, slopes.clone(), ramps, spline.clone(), k, shifts[j].clone())
                    .ok_or(LoftError::Twisted { index: k })
            })
            .collect::<Result<Vec<_>, _>>()?;
        rails.push((0..nseg).map(|i| (wk[i].rail(&dirs[i], size), span)).collect());
        walls.push(wk.into_iter().map(Skinned::surface).collect());
    }
    Ok((rails, walls))
}

/// A segment drawn in plane `f`, by the fraction along it.
fn rail(s: &Segment, f: &Frame) -> Rail {
    match *s {
        Segment::Spline(ref c) => Rail::Curve(Arc::new(c.c.mapped(|q| f.point_to_world(q)))),
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
    match (&ra, &rb) {
        (&Rail::Line { a: a0, b: a1 }, &Rail::Line { a: b0, b: b1 }) => {
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
                // a side of no length (an apex) says nothing
                let said = |x: &&f64| **x != 0.0;
                let any = da.length_sq() == 0.0 || db.length_sq() == 0.0;
                let ok = if any {
                    side.iter().filter(said).all(|&x| x > 0.0) || side.iter().filter(said).all(|&x| x < 0.0)
                } else {
                    side.iter().all(|&x| x > 0.0) || side.iter().all(|&x| x < 0.0)
                };
                if !ok {
                    return None;
                }
                return Some(Surface::Plane(Plane::new(Frame::from_zx(a0, n, along)?)));
            }
        }
        (
            &Rail::Arc {
                circle: ca,
                start: sa,
                sweep: wa,
            },
            &Rail::Arc {
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
            // (an apex is a circle of no size: its direction is anything's)
            let same = (ca.radius == 0.0
                || cb.radius == 0.0
                || (dir(&ca, sa) - dir(&cb, sb)).length() <= tol::ANGULAR)
                && (wa - wb).abs() <= tol::ANGULAR;
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
    use forge_geom::CurveGeom;
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
        let b = loft(
            &[sq(0.0, 4.0), sq(5.0, 2.0), sq(9.0, 3.0)],
            LoftKind::Ruled,
            &LoftGuides::None,
            1,
        )
        .unwrap();
        valid(&b);
        assert_eq!(kinds(&b), (10, 0));
        let ci = |z: f64, r: f64| Profile {
            plane: at(z),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        };
        let b = loft(
            &[ci(0.0, 4.0), ci(6.0, 2.0)],
            LoftKind::Ruled,
            &LoftGuides::None,
            1,
        )
        .unwrap();
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
        let b = loft(&[sq.clone(), turned], LoftKind::Ruled, &LoftGuides::None, 1).unwrap();
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
            &LoftGuides::None,
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
        let heavy = LoftKind::Smooth {
            start: Some(4.0),
            end: Some(3.0),
        };
        for kind in [free, square_ends, heavy] {
            for profiles in [
                vec![sq(0.0, 4.0, 0.0), sq(5.0, 2.0, 0.2), sq(9.0, 3.0, 0.3)],
                vec![ci(0.0, 4.0), ci(5.0, 6.0), ci(10.0, 3.0), ci(14.0, 4.0)],
            ] {
                let b = loft(&profiles, kind, &LoftGuides::None, 1).unwrap();
                valid(&b);
                let walls = b.faces.len() - 2;
                assert_eq!(kinds(&b).1, walls);
                let bx = loft_bounds(&profiles, kind, &LoftGuides::None, &LoftOptions::default()).unwrap();
                for v in &b.vertices {
                    assert!(bx.contains(v.point));
                }
            }
        }
    }

    #[test]
    fn guides_shape_smooth_lofts() {
        use crate::PathSeg;
        use forge_math::UnitVec3;
        let sq = |z: f64, h: f64| Profile {
            plane: at(z),
            loops: vec![Profile::rectangle(Point2::new(-h, -h), 2.0 * h, 2.0 * h, 0)],
        };
        let free = LoftKind::Smooth {
            start: None,
            end: None,
        };
        // a centre line straight up, then bending off towards +x
        let bend = Circle {
            frame: Frame::from_zx(
                Point3::new(20.0, 0.0, 4.0),
                UnitVec3::Y,
                Vec3::new(-1.0, 0.0, 0.0),
            )
            .unwrap(),
            radius: 20.0,
        };
        let centre = vec![
            PathSeg::Line {
                a: Point3::new(0.0, 0.0, -2.0),
                b: Point3::new(0.0, 0.0, 4.0),
            },
            PathSeg::Arc {
                circle: bend,
                start: 0.0,
                sweep: 0.6,
            },
        ];
        let profiles = [sq(0.0, 4.0), sq(5.0, 2.0), sq(9.0, 3.0)];
        let plain = loft(&profiles, free, &LoftGuides::None, 1).unwrap();
        let guided = loft(&profiles, free, &LoftGuides::CentreLine(centre), 1).unwrap();
        valid(&guided);
        assert_eq!(kinds(&guided), kinds(&plain));
        // a rail through the first corner of two squares, bowed out sideways by an arc
        let (a, b) = (Point3::new(-4.0, -4.0, 0.0), Point3::new(-3.0, -3.0, 9.0));
        let c = b - a;
        let u = c.normalize().unwrap();
        let side = (Vec3::new(-1.0, 1.0, 0.0) - u.get() * u.dot(Vec3::new(-1.0, 1.0, 0.0)))
            .normalize()
            .unwrap();
        let o = a + c * 0.5 - side * 15.0;
        let r = o.distance(a);
        let frame = Frame::from_zx(o, side.cross(u.get()).normalize().unwrap(), a - o).unwrap();
        let sweep = forge_math::dmath::atan2(frame.vec_to_local(b - o).y, frame.vec_to_local(b - o).x);
        let rail = PathSeg::Arc {
            circle: Circle { frame, radius: r },
            start: 0.0,
            sweep,
        };
        let two = [sq(0.0, 4.0), sq(9.0, 3.0)];
        let railed = loft(&two, free, &LoftGuides::Rails(vec![(0, vec![rail.clone()])]), 1).unwrap();
        valid(&railed);
        // the edge at that corner runs along the rail
        let on_rail =
            |p: Point3| (p.distance(o) - r).abs() < 1e-9 && frame.vec_to_local(p - o).z.abs() < 1e-9;
        let corner = railed
            .edges
            .iter()
            .find(|e| matches!(e.curve, Curve::Procedural(_)) && on_rail(e.curve.eval(e.range.mid())));
        assert!(corner.is_some(), "no edge along the rail");
        assert_eq!(
            loft(
                &two,
                LoftKind::Ruled,
                &LoftGuides::Rails(vec![(0, vec![rail.clone()])]),
                1
            )
            .unwrap_err(),
            LoftError::GuidesNeedSmooth
        );
        assert_eq!(
            loft(&two, free, &LoftGuides::Rails(vec![(1, vec![rail.clone()])]), 1).unwrap_err(),
            LoftError::GuideMisses { guide: 0, profile: 0 }
        );
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
            loft(&[sq.clone(), tri], LoftKind::Ruled, &LoftGuides::None, 1).unwrap_err(),
            LoftError::Mismatch {
                index: 1,
                loop_index: Some(0),
                want: 4,
                got: 3
            }
        );
        assert_eq!(
            loft(std::slice::from_ref(&sq), LoftKind::Ruled, &LoftGuides::None, 1).unwrap_err(),
            LoftError::TooFew
        );
        // the far square turned half way round: every wall's rulings meet in the middle
        let half = Profile {
            plane: at(5.0).rotated_about_z(dmath::PI),
            loops: sq.loops.clone(),
        };
        assert_eq!(
            loft(&[sq, half], LoftKind::Ruled, &LoftGuides::None, 1).unwrap_err(),
            LoftError::Twisted { index: 0 }
        );
    }
}
