use crate::sketch::{Entity, EntityId, Sketch};
use forge_math::{Point2, dmath, tol};
use forge_ops::{Profile, ProfileError, ProfileLoop, Segment};

#[derive(Clone, Debug, PartialEq)]
pub enum ExtractError {
    /// No closed geometry to make a region from.
    Empty,
    /// The chain through this entity does not close.
    OpenChain {
        entity: EntityId,
    },
    /// More than two segments meet at an end of this entity.
    Branch {
        entity: EntityId,
    },
    Degenerate {
        entity: EntityId,
    },
    /// These two entities cross or overlap away from a shared end.
    SelfIntersection {
        a: EntityId,
        b: EntityId,
    },
    Profile(ProfileError),
    /// Whether the loop through `inner` lies inside the loop through `outer` could not be
    /// decided with certainty from any point of it (each on, or too near, the other loop).
    Undecided {
        inner: EntityId,
        outer: EntityId,
    },
}

/// The regions the sketch's non-construction lines, arcs and circles enclose:
/// one profile per outer loop, with the loops directly inside it as holes (an
/// island inside a hole is its own profile). Segment ids are entity ids.
pub fn profiles(sketch: &Sketch) -> Result<Vec<Profile>, ExtractError> {
    profiles_among(sketch, &|_| true)
}

/// `profiles` of only the entities `keep` takes (a text's own glyphs, whatever else the sketch
/// draws round them).
pub fn profiles_among(
    sketch: &Sketch,
    keep: &dyn Fn(EntityId) -> bool,
) -> Result<Vec<Profile>, ExtractError> {
    let mut open: Vec<(Segment, EntityId)> = Vec::new();
    let mut loops: Vec<ProfileLoop> = Vec::new();
    for id in sketch.entity_ids() {
        if sketch.is_construction(id) || !keep(id) {
            continue;
        }
        match sketch.entity(id) {
            Some(Entity::Line { a, b }) => {
                let (pa, pb) = (sketch.point(a), sketch.point(b));
                if pa.distance(pb) <= tol::MIN_LENGTH {
                    return Err(ExtractError::Degenerate { entity: id });
                }
                open.push((Segment::Line { a: pa, b: pb }, id));
            }
            Some(Entity::Arc {
                center,
                start,
                end,
                ccw,
            }) => {
                let c = sketch.point(center);
                let (s, e) = (sketch.point(start), sketch.point(end));
                let radius = s.distance(c);
                let a0 = dmath::atan2(s.y - c.y, s.x - c.x);
                let a1 = dmath::atan2(e.y - c.y, e.x - c.x);
                let sweep = if ccw {
                    dmath::wrap_tau(a1 - a0)
                } else {
                    -dmath::wrap_tau(a0 - a1)
                };
                if radius <= tol::MIN_LENGTH || (sweep * radius).abs() <= tol::MIN_LENGTH {
                    return Err(ExtractError::Degenerate { entity: id });
                }
                open.push((
                    Segment::Arc {
                        center: c,
                        radius,
                        start: a0,
                        sweep,
                    },
                    id,
                ));
            }
            Some(Entity::Circle { center, .. }) => {
                let radius = sketch.radius(id);
                if radius <= tol::MIN_LENGTH {
                    return Err(ExtractError::Degenerate { entity: id });
                }
                let seg = Segment::Circle {
                    center: sketch.point(center),
                    radius,
                    ccw: true,
                };
                loops.push(ProfileLoop::new(vec![(seg, id.0)]));
            }
            Some(Entity::Spline { .. } | Entity::Ellipse { .. }) => {
                let Some(c) = sketch.curve(id) else {
                    return Err(ExtractError::Degenerate { entity: id });
                };
                if c.closed() {
                    loops.push(ProfileLoop::new(vec![(Segment::Spline(c), id.0)]));
                } else {
                    open.push((Segment::Spline(c), id));
                }
            }
            _ => {}
        }
    }
    loops.extend(chain_loops(&open)?);
    if loops.is_empty() {
        return Err(ExtractError::Empty);
    }
    check_crossings(&loops)?;

    // which loop lies inside which, decided exactly (winding.rs); each pair asked as needed
    let n = loops.len();
    let inside = |i: usize, j: usize| -> Result<bool, ExtractError> {
        if i == j {
            return Ok(false);
        }
        crate::winding::inside(&loops[i], &loops[j], crate::winding::bbox(&loops[j])).ok_or(
            ExtractError::Undecided {
                inner: EntityId(loops[i].segments[0].1),
                outer: EntityId(loops[j].segments[0].1),
            },
        )
    };
    let mut depth = vec![0usize; n];
    for (i, d) in depth.iter_mut().enumerate() {
        for j in 0..n {
            *d += inside(i, j)? as usize;
        }
    }
    let mut parent: Vec<Option<usize>> = vec![None; n];
    for (i, p) in parent.iter_mut().enumerate() {
        for j in 0..n {
            if depth[j] + 1 == depth[i] && inside(i, j)? {
                *p = Some(j);
                break;
            }
        }
    }

    let mut out = Vec::new();
    for o in (0..n).filter(|&i| depth[i].is_multiple_of(2)) {
        let mut ls = vec![loops[o].clone()];
        ls.extend((0..n).filter(|&i| parent[i] == Some(o)).map(|i| loops[i].clone()));
        let p = Profile {
            plane: sketch.plane,
            loops: ls,
        };
        out.push(p.normalized_touching().map_err(ExtractError::Profile)?);
    }
    Ok(out)
}

/// Joins open segments end to end into closed loops.
fn chain_loops(open: &[(Segment, EntityId)]) -> Result<Vec<ProfileLoop>, ExtractError> {
    let mut nodes: Vec<Point2> = Vec::new();
    let mut node_of = |p: Point2| -> usize {
        match nodes.iter().position(|q| q.distance(p) <= tol::LINEAR) {
            Some(i) => i,
            None => {
                nodes.push(p);
                nodes.len() - 1
            }
        }
    };
    let ends: Vec<(usize, usize)> = open
        .iter()
        .map(|(s, _)| (node_of(s.start()), node_of(s.end())))
        .collect();
    let mut incident: Vec<Vec<usize>> = Vec::new();
    for (k, &(a, b)) in ends.iter().enumerate() {
        for v in [a, b] {
            if incident.len() <= v {
                incident.resize(v + 1, Vec::new());
            }
            incident[v].push(k);
        }
    }
    for (k, &(a, b)) in ends.iter().enumerate() {
        for v in [a, b] {
            match incident[v].len() {
                2 => {}
                1 => return Err(ExtractError::OpenChain { entity: open[k].1 }),
                _ => return Err(ExtractError::Branch { entity: open[k].1 }),
            }
        }
    }
    let mut used = vec![false; open.len()];
    let mut loops = Vec::new();
    for first in 0..open.len() {
        if used[first] {
            continue;
        }
        let mut segs = Vec::new();
        let (mut k, mut forward) = (first, true);
        loop {
            used[k] = true;
            let (s, id) = &open[k];
            segs.push((if forward { s.clone() } else { s.reversed() }, id.0));
            let at = if forward { ends[k].1 } else { ends[k].0 };
            let next = incident[at].iter().copied().find(|&o| o != k).unwrap_or(k);
            if next == first || used[next] {
                break;
            }
            forward = ends[next].0 == at;
            k = next;
        }
        loops.push(ProfileLoop::new(segs));
    }
    Ok(loops)
}

/// A segment as a line piece or a circular piece (full circles sweep 2pi), or a free curve.
enum Shape {
    Curve(std::sync::Arc<forge_geom::spline::PlaneSpline>),
    Line(Point2, Point2),
    Arc {
        c: Point2,
        r: f64,
        start: f64,
        sweep: f64,
    },
}

fn shape(s: &Segment) -> Shape {
    match *s {
        Segment::Spline(ref c) => Shape::Curve(c.clone()),
        Segment::Line { a, b } => Shape::Line(a, b),
        Segment::Arc {
            center,
            radius,
            start,
            sweep,
        } => {
            // store counter-clockwise
            let (start, sweep) = if sweep < 0.0 {
                (start + sweep, -sweep)
            } else {
                (start, sweep)
            };
            Shape::Arc {
                c: center,
                r: radius,
                start,
                sweep,
            }
        }
        Segment::Circle { center, radius, .. } => Shape::Arc {
            c: center,
            r: radius,
            start: 0.0,
            sweep: dmath::TAU,
        },
    }
}

const EPS: f64 = 1e-9;

fn radius(s: &Shape) -> f64 {
    match s {
        Shape::Line(..) | Shape::Curve(_) => 0.0,
        Shape::Arc { r, .. } => *r,
    }
}

fn on_arc(c: Point2, start: f64, sweep: f64, p: Point2) -> bool {
    let a = dmath::wrap_tau(dmath::atan2(p.y - c.y, p.x - c.x) - start);
    a <= sweep + EPS || a >= dmath::TAU - EPS
}

enum Hit {
    Points(Vec<Point2>),
    Overlap,
}

fn hits(a: &Shape, b: &Shape) -> Hit {
    match (a, b) {
        (Shape::Curve(c), o) | (o, Shape::Curve(c)) => curve_hits(c, o),
        (Shape::Line(p, q), Shape::Line(r, s)) => {
            let (d1, d2) = (*q - *p, *s - *r);
            let den = d1.cross(d2);
            let scale = d1.length() * d2.length();
            if den.abs() <= scale * 1e-12 {
                let off = d1.cross(*r - *p) / d1.length();
                if off.abs() > tol::LINEAR {
                    return Hit::Points(vec![]);
                }
                // collinear: overlap if the projections share more than a point
                let l2 = d1.length_sq();
                let (t0, t1) = (d1.dot(*r - *p) / l2, d1.dot(*s - *p) / l2);
                let (lo, hi) = (t0.min(t1).max(0.0), t0.max(t1).min(1.0));
                return if hi - lo > EPS {
                    Hit::Overlap
                } else if hi - lo >= -EPS {
                    Hit::Points(vec![p.lerp(*q, lo)])
                } else {
                    Hit::Points(vec![])
                };
            }
            let t = (*r - *p).cross(d2) / den;
            let u = (*r - *p).cross(d1) / den;
            if (-EPS..=1.0 + EPS).contains(&t) && (-EPS..=1.0 + EPS).contains(&u) {
                Hit::Points(vec![p.lerp(*q, t)])
            } else {
                Hit::Points(vec![])
            }
        }
        (Shape::Line(p, q), Shape::Arc { c, r, start, sweep })
        | (Shape::Arc { c, r, start, sweep }, Shape::Line(p, q)) => {
            let d = *q - *p;
            let f = *p - *c;
            let roots = forge_math::poly::quadratic(d.length_sq(), 2.0 * f.dot(d), f.length_sq() - r * r);
            Hit::Points(
                roots
                    .into_iter()
                    .filter(|t| (-EPS..=1.0 + EPS).contains(&t.x))
                    .map(|t| p.lerp(*q, t.x))
                    .filter(|&x| on_arc(*c, *start, *sweep, x))
                    .collect(),
            )
        }
        (
            Shape::Arc {
                c: c1,
                r: r1,
                start: s1,
                sweep: w1,
            },
            Shape::Arc {
                c: c2,
                r: r2,
                start: s2,
                sweep: w2,
            },
        ) => {
            let dv = *c2 - *c1;
            let d = dv.length();
            if d <= tol::LINEAR && (r1 - r2).abs() <= tol::LINEAR {
                // same circle: overlap unless the arcs only share ends
                let mid2 = forge_ops::profile::arc_point(*c2, *r2, s2 + 0.5 * w2);
                let mid1 = forge_ops::profile::arc_point(*c1, *r1, s1 + 0.5 * w1);
                let inner = |c: Point2, s: f64, w: f64, p: Point2| {
                    let a = dmath::wrap_tau(dmath::atan2(p.y - c.y, p.x - c.x) - s);
                    a > EPS && a < w - EPS
                };
                let ends2 = [
                    forge_ops::profile::arc_point(*c2, *r2, *s2),
                    forge_ops::profile::arc_point(*c2, *r2, s2 + w2),
                ];
                let ends1 = [
                    forge_ops::profile::arc_point(*c1, *r1, *s1),
                    forge_ops::profile::arc_point(*c1, *r1, s1 + w1),
                ];
                let overlap = inner(*c1, *s1, *w1, mid2)
                    || inner(*c2, *s2, *w2, mid1)
                    || ends2.iter().any(|&p| inner(*c1, *s1, *w1, p))
                    || ends1.iter().any(|&p| inner(*c2, *s2, *w2, p));
                return if overlap {
                    Hit::Overlap
                } else {
                    Hit::Points(vec![])
                };
            }
            if d > r1 + r2 + tol::LINEAR || d < (r1 - r2).abs() - tol::LINEAR || d == 0.0 {
                return Hit::Points(vec![]);
            }
            let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
            let h = (r1 * r1 - a * a).max(0.0).sqrt();
            let base = *c1 + dv * (a / d);
            let perp = dv.perp() * (h / d);
            let mut pts = vec![base + perp];
            if h > tol::LINEAR {
                pts.push(base - perp);
            }
            Hit::Points(
                pts.into_iter()
                    .filter(|&p| on_arc(*c1, *s1, *w1, p) && on_arc(*c2, *s2, *w2, p))
                    .collect(),
            )
        }
    }
}

/// Where a free curve meets another shape: Bernstein roots against a line or circle, hull
/// subdivision against another curve.
fn curve_hits(c: &forge_geom::spline::PlaneSpline, o: &Shape) -> Hit {
    use forge_math::{Point3, Vec3};
    let s = &c.c;
    let on = |t: f64| {
        let q = s.point(t);
        Point2::new(q.x, q.y)
    };
    let d = s.domain();
    match o {
        Shape::Line(p, q) => {
            let v = *q - *p;
            let Some(u) = v.normalize() else {
                return Hit::Points(vec![]);
            };
            let n = Vec3::new(-u.y, u.x, 0.0);
            let k = -(n.x * p.x + n.y * p.y);
            Hit::Points(
                s.plane_roots(n, k, d, tol::LINEAR)
                    .into_iter()
                    .map(|(t, _)| on(t))
                    .filter(|x| {
                        let f = (*x - *p).dot(v) / v.length_sq();
                        (-EPS..=1.0 + EPS).contains(&f)
                    })
                    .collect(),
            )
        }
        Shape::Arc {
            c: ctr,
            r,
            start,
            sweep,
        } => {
            let a = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
            let o3 = Point3::new(ctr.x, ctr.y, 0.0);
            Hit::Points(
                s.quadric_roots(o3, &a, Vec3::ZERO, -r * r, d, 2.0 * r * tol::LINEAR)
                    .into_iter()
                    .map(|(t, _)| on(t))
                    .filter(|&x| on_arc(*ctr, *start, *sweep, x))
                    .collect(),
            )
        }
        Shape::Curve(e) => {
            let f = &e.c;
            if forge_geom::spline::meet::same_curve(s, f) {
                return Hit::Overlap;
            }
            Hit::Points(
                forge_geom::spline::meet::cross_sections(s, d, f, f.domain())
                    .into_iter()
                    .map(|(t, _)| on(t))
                    .collect(),
            )
        }
    }
}

/// Where loops meet away from the ends they share, which of them may: what cannot exist is the
/// solid, not the sketch. A loop meeting itself is refused as a profile refuses it (a knife edge,
/// sides meeting an arc tangentially in a cusp, is not); two loops are refused where they overlap
/// or cross, a touch (tangent, or at a corner) left to the result to judge (an extrude pinched
/// there is refused, a cut whose touch lies in empty space builds).
fn check_crossings(loops: &[ProfileLoop]) -> Result<(), ExtractError> {
    let all: Vec<(Segment, u64, usize)> = loops
        .iter()
        .enumerate()
        .flat_map(|(k, l)| l.segments.iter().map(move |(s, id)| (s.clone(), *id, k)))
        .collect();
    let shapes: Vec<Shape> = all.iter().map(|(s, ..)| shape(s)).collect();
    let mut meets_itself: Vec<Option<(u64, u64)>> = vec![None; loops.len()];
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            let (si, sj) = (&all[i].0, &all[j].0);
            let ends = [si.start(), si.end(), sj.start(), sj.end()];
            let shared: Vec<Point2> = ends[..2]
                .iter()
                .copied()
                .filter(|p| ends[2..].iter().any(|q| q.distance(*p) <= tol::LINEAR))
                .collect();
            // a tangent joint solved to ~1e-9 mm still splits into two roots up to sqrt(2 r delta) apart
            let near = 1e-6 + (2.0 * radius(&shapes[i]).max(radius(&shapes[j])) * 1e-8).sqrt();
            let same = all[i].2 == all[j].2;
            let hit = hits(&shapes[i], &shapes[j]);
            let bad = match &hit {
                Hit::Overlap => true,
                Hit::Points(ps) => ps.iter().any(|p| {
                    !shared.iter().any(|q| q.distance(*p) <= near)
                        && (same || crosses(&shapes[i], &shapes[j], *p, &ends, near))
                }),
            };
            if !bad {
                continue;
            }
            // two curves of one loop that cross at an angle away from their ends and part by more
            // than tolerance before they meet again at their shared end make the loop cross itself,
            // however small the lobe between (a sampled outline's chords can pass a lobe narrower
            // than their sag). One that only meets itself (a touch), or whose lobe is within
            // tolerance (a knife edge's sides, a hair off tangent), is asked of the loop's outline
            let crossing = || match &hit {
                Hit::Points(ps) => ps.iter().any(|p| {
                    !shared.iter().any(|q| q.distance(*p) <= near)
                        && crosses(&shapes[i], &shapes[j], *p, &ends, near)
                        && shared.iter().any(|&q| {
                            apart(&shapes[i], &shapes[j], *p, q).max(apart(&shapes[j], &shapes[i], *p, q))
                                > tol::LINEAR
                        })
                }),
                Hit::Overlap => false,
            };
            if same && !crossing() {
                meets_itself[all[i].2].get_or_insert((all[i].1, all[j].1));
            } else {
                return Err(ExtractError::SelfIntersection {
                    a: EntityId(all[i].1),
                    b: EntityId(all[j].1),
                });
            }
        }
    }
    for (l, m) in loops.iter().zip(meets_itself) {
        if let Some((a, b)) = m
            && l.crosses_itself()
        {
            return Err(ExtractError::SelfIntersection {
                a: EntityId(a),
                b: EntityId(b),
            });
        }
    }
    Ok(())
}

/// Whether two shapes of different loops cross at `p`, where they meet: through each other at an
/// angle, away from either one's ends. Meeting tangentially, or at a corner, is a touch.
fn crosses(a: &Shape, b: &Shape, p: Point2, ends: &[Point2; 4], near: f64) -> bool {
    if ends.iter().any(|e| e.distance(p) <= near) {
        return false;
    }
    match (direction(a, p), direction(b, p)) {
        (Some(u), Some(v)) => u.cross(v).abs() > 1e-3,
        _ => false,
    }
}

/// How far shape `a`, between two of its points `p` and `q`, gets from shape `b`: from the point of
/// `a` halfway between them.
fn apart(a: &Shape, b: &Shape, p: Point2, q: Point2) -> f64 {
    let m = match a {
        Shape::Line(..) => p.lerp(q, 0.5),
        Shape::Arc { c, r, start, .. } => {
            let at = |x: Point2| dmath::wrap_tau(dmath::atan2(x.y - c.y, x.x - c.x) - start);
            let w = *start + 0.5 * (at(p) + at(q));
            let (sn, cs) = dmath::sin_cos(w);
            Point2::new(c.x + r * cs, c.y + r * sn)
        }
        Shape::Curve(e) => {
            let c = &e.c;
            let t = |x: Point2| c.closest(forge_math::Point3::new(x.x, x.y, 0.0), c.domain());
            let m = c.deriv(0.5 * (t(p) + t(q))).p;
            Point2::new(m.x, m.y)
        }
    };
    match b {
        Shape::Line(u, v) => {
            let d = *v - *u;
            let k = ((m - *u).dot(d) / d.dot(d)).clamp(0.0, 1.0);
            m.distance(*u + d * k)
        }
        Shape::Arc { c, r, start, sweep } => {
            if on_arc(*c, *start, *sweep, m) {
                (m.distance(*c) - r).abs()
            } else {
                let end = |w: f64| {
                    let (sn, cs) = dmath::sin_cos(w);
                    Point2::new(c.x + r * cs, c.y + r * sn)
                };
                m.distance(end(*start)).min(m.distance(end(start + sweep)))
            }
        }
        Shape::Curve(e) => {
            let c = &e.c;
            let x = forge_math::Point3::new(m.x, m.y, 0.0);
            c.deriv(c.closest(x, c.domain())).p.distance(x)
        }
    }
}

/// A shape's unit direction at a point on it.
fn direction(s: &Shape, p: Point2) -> Option<forge_math::Vec2> {
    match s {
        Shape::Line(a, b) => (*b - *a).normalize(),
        Shape::Arc { c, .. } => (p - *c).perp().normalize(),
        Shape::Curve(e) => {
            let c = &e.c;
            let d = c.deriv(c.closest(forge_math::Point3::new(p.x, p.y, 0.0), c.domain()));
            forge_math::Vec2::new(d.d1.x, d.d1.y).normalize()
        }
    }
}
