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
}

/// The regions the sketch's non-construction lines, arcs and circles enclose:
/// one profile per outer loop, with the loops directly inside it as holes (an
/// island inside a hole is its own profile). Segment ids are entity ids.
pub fn profiles(sketch: &Sketch) -> Result<Vec<Profile>, ExtractError> {
    let mut open: Vec<(Segment, EntityId)> = Vec::new();
    let mut loops: Vec<ProfileLoop> = Vec::new();
    for id in sketch.entity_ids() {
        if sketch.is_construction(id) {
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

    let polys: Vec<Vec<Point2>> = loops
        .iter()
        .map(|l| l.segments.iter().flat_map(|(s, _)| s.samples(16)).collect())
        .collect();
    let n = loops.len();
    let inside = |i: usize, j: usize| i != j && contains(&polys[j], polys[i][polys[i].len() / 2]);
    let depth: Vec<usize> = (0..n).map(|i| (0..n).filter(|&j| inside(i, j)).count()).collect();
    let parent: Vec<Option<usize>> = (0..n)
        .map(|i| (0..n).find(|&j| inside(i, j) && depth[j] + 1 == depth[i]))
        .collect();

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

fn contains(poly: &[Point2], p: Point2) -> bool {
    let mut c = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
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

/// No two segments may meet anywhere except at an end they share.
fn check_crossings(loops: &[ProfileLoop]) -> Result<(), ExtractError> {
    let all: Vec<(Segment, u64)> = loops.iter().flat_map(|l| l.segments.iter().cloned()).collect();
    let shapes: Vec<Shape> = all.iter().map(|(s, _)| shape(s)).collect();
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            let (si, sj) = (&all[i].0, &all[j].0);
            let shared: Vec<Point2> = [si.start(), si.end()]
                .into_iter()
                .filter(|p| {
                    [sj.start(), sj.end()]
                        .iter()
                        .any(|q| q.distance(*p) <= tol::LINEAR)
                })
                .collect();
            // a tangent joint solved to ~1e-9 mm still splits into two roots up to sqrt(2 r delta) apart
            let near = 1e-6 + (2.0 * radius(&shapes[i]).max(radius(&shapes[j])) * 1e-8).sqrt();
            let bad = match hits(&shapes[i], &shapes[j]) {
                Hit::Overlap => true,
                Hit::Points(ps) => ps.iter().any(|p| !shared.iter().any(|q| q.distance(*p) <= near)),
            };
            if bad {
                return Err(ExtractError::SelfIntersection {
                    a: EntityId(all[i].1),
                    b: EntityId(all[j].1),
                });
            }
        }
    }
    Ok(())
}
