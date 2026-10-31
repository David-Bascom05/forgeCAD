//! Ends on a surface: a sweep cut off where it first meets a face's surface (any plane, a
//! sphere, a cylinder or a cone, offset or not), and the next face along the way, found by rays
//! against the earlier tools' solids.
//!
//! Each ray of the sweep must meet the surface once before it leaves the near side of a
//! separating plane: a plane through a sphere's centre square to the travel, through a
//! cylinder's axis, through a cone's apex and the two lines it shows in outline. So the tool is
//! the sweep kept on the near side of that plane, less the solid inside the surface (from
//! outside), or the sweep kept inside it (from inside): one n-ary boolean of plain solids,
//! whose analytic shapes combine the same way. Where some of the profile misses the surface,
//! or the surface cuts the sketch, the tool shows it (a face on the separating plane or the
//! sweep's far end, or a start cap no longer whole), and the feature is refused.

use crate::{Combine, ExtentError, Feature, RegenError, Tool, Turn, tool};
use forge_bool::{Member, Op, Seg2, Shape};
use forge_geom::{Cone, Cylinder, Plane, Sphere, Surface};
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3, dmath, poly, tol};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment};
use forge_topo::{Body, Name};

/// How a sweep is cut off at a surface: solids to keep or take away, in order, and planes no
/// face of the result may lie on.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Plan {
    aux: Vec<(Op, Feature)>,
    walls: Vec<(Point3, UnitVec3)>,
}

/// A box on the near side of the plane through `at` square to `z` (z pointing away from the
/// near side), covering `pts`.
fn near_box(at: Point3, z: UnitVec3, pts: &[Point3]) -> Feature {
    let b = forge_math::Box3::from_points(pts.iter().copied());
    let c = b.center();
    let s = b.diagonal() + 1.0;
    let foot = c - z * z.dot(c - at);
    let depth = z.dot(c - at).abs() + 2.0 * s;
    Feature::Extrude {
        profile: Profile {
            plane: Frame::from_z(foot, -z),
            loops: vec![Profile::rectangle(Point2::new(-s, -s), 2.0 * s, 2.0 * s, 0)],
        },
        extent: crate::Extent::Distance(depth),
        op: Combine::Keep,
    }
}

/// The range of `(p - at) . z` over `pts`.
fn span(pts: &[Point3], at: Point3, z: Vec3) -> (f64, f64) {
    pts.iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            let t = z.dot(*p - at);
            (lo.min(t), hi.max(t))
        })
}

/// The cut-off of a sweep along `d` from `foot` (points on the sketch plane holding the
/// profile, `refp` one inside it) at surface `s`, moved `offset` further on: the sweep's
/// length and the plan.
pub(crate) fn plan(
    d: UnitVec3,
    foot: &[Point3],
    refp: Point3,
    s: &Surface,
    offset: f64,
) -> Result<(f64, Plan), ExtentError> {
    let dv = d.get();
    let swept = |t: f64| -> Vec<Point3> { foot.iter().flat_map(|&p| [p, p + dv * t]).collect() };
    let positive = |r: f64| {
        if r.is_finite() && r > tol::LINEAR {
            Ok(r)
        } else {
            Err(ExtentError::BadSize)
        }
    };
    match s {
        Surface::Plane(p) => {
            let n = p.frame.z();
            let dn = n.dot(dv);
            if dn.abs() <= tol::ANGULAR {
                return Err(ExtentError::NeverHits);
            }
            let z = if dn > 0.0 { n } else { -n };
            let at = p.frame.origin + z * offset;
            // every foot point short of the plane
            let (lo, hi) = span(foot, at, z.get());
            if hi >= -tol::LINEAR {
                return Err(ExtentError::NotAhead);
            }
            let t = -lo / z.dot(dv) + 1.0;
            let plan = Plan {
                aux: vec![(Op::Intersect, near_box(at, z, &swept(t)))],
                // the end itself lies on the plane: every ray meets it
                walls: vec![],
            };
            Ok((t, plan))
        }
        Surface::Sphere(sp) => {
            let c = sp.frame.origin;
            let inside = refp.distance(c) < sp.radius;
            let r = positive(if inside {
                sp.radius + offset
            } else {
                sp.radius - offset
            })?;
            let ball = Feature::Ball {
                frame: Frame::WORLD.with_origin(c),
                radius: r,
                op: Combine::Cut,
            };
            // how far ahead the centre lies, at most
            let ahead = -span(foot, c, dv).0;
            if inside {
                let t = ahead + r + 1.0;
                return Ok((
                    t,
                    Plan {
                        aux: vec![(Op::Intersect, ball)],
                        walls: vec![],
                    },
                ));
            }
            if ahead <= tol::LINEAR {
                return Err(ExtentError::NotAhead);
            }
            let t = ahead + 1.0;
            Ok((
                t,
                Plan {
                    aux: vec![(Op::Intersect, near_box(c, d, &swept(t))), (Op::Subtract, ball)],
                    walls: vec![(c, d)],
                },
            ))
        }
        Surface::Cylinder(cy) => {
            let (a, at) = (cy.frame.z(), cy.frame.origin);
            let side = dv - a * a.dot(dv);
            let Some(across) = side.normalize().filter(|_| side.length() > tol::ANGULAR) else {
                return Err(ExtentError::NeverHits);
            };
            let off = refp - at;
            let inside = (off - a * a.dot(off)).length() < cy.radius;
            let r = positive(if inside {
                cy.radius + offset
            } else {
                cy.radius - offset
            })?;
            let t = if inside {
                2.0 * r / side.length() + 1.0
            } else {
                let (lo, hi) = span(foot, at, across.get());
                if hi >= -tol::LINEAR {
                    return Err(ExtentError::NotAhead);
                }
                -lo / across.dot(dv) + 1.0
            };
            let (lo, hi) = span(&swept(t), at, a.get());
            let rod = Feature::Extrude {
                profile: Profile {
                    plane: Frame::from_z(at + a * (lo - 1.0), a),
                    loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
                },
                extent: crate::Extent::Distance(hi - lo + 2.0),
                op: Combine::Cut,
            };
            if inside {
                return Ok((
                    t,
                    Plan {
                        aux: vec![(Op::Intersect, rod)],
                        walls: vec![],
                    },
                ));
            }
            Ok((
                t,
                Plan {
                    aux: vec![
                        (Op::Intersect, near_box(at, across, &swept(t))),
                        (Op::Subtract, rod),
                    ],
                    walls: vec![(at, across)],
                },
            ))
        }
        Surface::Cone(cn) => {
            let a = cn.frame.z();
            let (sa, ca) = (cn.sin_a(), cn.cos_a());
            let off = refp - cn.apex();
            let inside = a.dot(off) > off.length() * ca;
            // the surface moved square to itself: the apex along the axis
            let shift = if inside { -offset } else { offset } / sa;
            let apex = cn.apex() + a * shift;
            let ad = a.dot(dv);
            let (t, walls) = if inside {
                if ad >= ca - tol::ANGULAR {
                    return Err(ExtentError::NeverHits);
                }
                let far = foot.iter().fold(0.0f64, |m, p| m.max(p.distance(apex)));
                (far * (1.0 + ca) / (ca - ad) + 1.0, vec![])
            } else {
                // the two lines the cone shows in outline seen along d, and the plane through them
                let e1 = cn.frame.x();
                let e2 = cn.frame.y();
                let (d1, d2) = (e1.dot(dv), e2.dot(dv));
                let rho = (d1 * d1 + d2 * d2).sqrt();
                let k = sa / ca * ad / rho;
                if !(rho > tol::ANGULAR && k.abs() < 1.0 - 1e-9) {
                    return Err(ExtentError::Unsupported("a cone met from inside its opening"));
                }
                let phi = dmath::atan2(d2, d1);
                let w = dmath::acos(k);
                let line = |u: f64| {
                    let (s, c) = dmath::sin_cos(u);
                    a.get() * ca + (e1.get() * c + e2.get() * s) * sa
                };
                let n = line(phi + w)
                    .cross(line(phi - w))
                    .normalize()
                    .ok_or(ExtentError::NeverHits)?;
                let n = if n.dot(dv) > 0.0 { n } else { -n };
                if n.dot(dv) <= tol::ANGULAR {
                    return Err(ExtentError::NeverHits);
                }
                let (lo, hi) = span(foot, apex, n.get());
                if hi >= -tol::LINEAR {
                    return Err(ExtentError::NotAhead);
                }
                (-lo / n.dot(dv) + 1.0, vec![(apex, n)])
            };
            // the cone between the heights the sweep spans (clear of the apex where it can be)
            let (lo, h) = span(&swept(t), apex, a.get());
            let (lo, h) = ((lo - 1.0).max(0.0), h.max(0.0) + 1.0);
            let tan = sa / ca;
            let mut pts = vec![
                Point2::new(lo * tan, lo),
                Point2::new(h * tan, h),
                Point2::new(0.0, h),
                Point2::new(0.0, lo),
            ];
            if lo == 0.0 {
                pts.pop();
            }
            let n = pts.len();
            let e = cn.frame.x();
            let tip = Feature::Revolve {
                profile: Profile {
                    plane: Frame::from_zx(apex, e.cross(a).normalize().expect("unit"), e.get())
                        .expect("frame"),
                    loops: vec![ProfileLoop::new(
                        (0..n)
                            .map(|k| {
                                (
                                    Segment::Line {
                                        a: pts[k],
                                        b: pts[(k + 1) % n],
                                    },
                                    k as u64,
                                )
                            })
                            .collect(),
                    )],
                },
                axis: Axis2 {
                    origin: Point2::ORIGIN,
                    dir: Vec2::Y,
                },
                extent: Turn::Angle(dmath::TAU),
                op: Combine::Cut,
            };
            let aux = match walls.first() {
                Some(&(p, n)) => vec![(Op::Intersect, near_box(p, n, &swept(t))), (Op::Subtract, tip)],
                None => vec![(Op::Intersect, tip)],
            };
            Ok((t, Plan { aux, walls }))
        }
        Surface::Torus(_) => Err(ExtentError::Unsupported("ending on a torus")),
        Surface::Procedural(_) => Err(ExtentError::Unsupported("ending on a blend")),
    }
}

/// Whether a face lies on the plane through `at` square to `n`.
fn on_plane(s: &Surface, at: Point3, n: UnitVec3) -> bool {
    let Surface::Plane(p) = s else { return false };
    p.frame.z().dot(n.get()).abs() >= 1.0 - tol::ANGULAR && n.dot(p.frame.origin - at).abs() <= tol::LINEAR
}

/// Feature `i`'s tool: the sweep `base` (its sketch faces `start`, its far end on the plane
/// through `far` square to the travel) cut off as `plan` says.
pub(crate) fn build(
    i: usize,
    base: Tool,
    plan: &Plan,
    start: &[Name],
    far: (Point3, UnitVec3),
    build: bool,
) -> Result<Tool, RegenError> {
    let bad = |error| RegenError::Extent { feature: i, error };
    let aux: Vec<Tool> = plan
        .aux
        .iter()
        .enumerate()
        .map(|(k, (_, f))| tool(i + 2_000_000 + k * 1_000_000, f, build))
        .collect::<Result<_, _>>()?;
    let shape = Shape::Csg {
        first: Box::new(base.2.clone()),
        rest: plan
            .aux
            .iter()
            .zip(&aux)
            .map(|((op, _), t)| (*op, t.2.clone()))
            .collect(),
    };
    if !build {
        return Ok((Body::new(), base.1, shape));
    }
    let mut bodies = vec![&base.0];
    bodies.extend(aux.iter().map(|t| &t.0));
    let ops: Vec<Op> = plan.aux.iter().map(|a| a.0).collect();
    let mut shapes = vec![base.2.clone()];
    shapes.extend(aux.iter().map(|t| t.2.clone()));
    let body = forge_bool::nary_with(
        &bodies,
        &ops,
        Some(&shapes),
        None,
        1,
        crate::cut_root(i + 2_000_000),
    )
    .map_err(|error| RegenError::Boolean { feature: i, error })?
    .compact();
    for f in body.face_ids() {
        let s = &body.face(f).surface;
        if on_plane(s, far.0, far.1) || plan.walls.iter().any(|w| on_plane(s, w.0, w.1)) {
            return Err(bad(ExtentError::NotCovered));
        }
    }
    for &n in start {
        let whole = base.0.face_by_name(n).map(|f| base.0.face(f).loops.len());
        let now = body.face_by_name(n).map(|f| body.face(f).loops.len());
        if whole != now {
            return Err(bad(ExtentError::NotAhead));
        }
    }
    Ok((body, base.1, shape))
}

/// Where a plain solid's boundary may cross the line `o + t d`: each t with the surface there
/// (None for a torus).
fn hits(sh: &Shape, o: Point3, d: Vec3, out: &mut Vec<(f64, Option<Surface>)>) {
    match sh {
        Shape::Csg { first, rest } => {
            hits(first, o, d, out);
            for (_, p) in rest {
                hits(p, o, d, out);
            }
        }
        Shape::Ball { center, radius } => {
            let w = o - *center;
            for r in poly::quadratic(d.dot(d), 2.0 * w.dot(d), w.dot(w) - radius * radius) {
                out.push((r.x, Some(sphere(*center, *radius))));
            }
        }
        Shape::Prism { frame, segs, lo, hi } => {
            let (l, v) = (frame.point_to_local(o), frame.vec_to_local(d));
            let at = |t: f64| Point3::new(l.x + v.x * t, l.y + v.y * t, l.z + v.z * t);
            if v.z.abs() > 1e-12 {
                let flat = Shape::Prism {
                    frame: *frame,
                    segs: segs.clone(),
                    lo: -1.0,
                    hi: 1.0,
                };
                for z in [*lo, *hi] {
                    let t = (z - l.z) / v.z;
                    let p = at(t);
                    if flat.member(frame.point_to_world(Point3::new(p.x, p.y, 0.0)), 0.0) != Member::Out {
                        let plane = frame.with_origin(frame.point_to_world(Point3::new(0.0, 0.0, z)));
                        out.push((t, Some(Surface::Plane(Plane { frame: plane }))));
                    }
                }
            }
            let in_z = |t: f64| {
                let z = l.z + v.z * t;
                z >= *lo && z <= *hi
            };
            for s in segs {
                match *s {
                    Seg2::Line { a, b } => {
                        let e = b - a;
                        let den = v.x * e.y - v.y * e.x;
                        if den.abs() <= 1e-12 * e.length() {
                            continue;
                        }
                        let w = Vec2::new(a.x - l.x, a.y - l.y);
                        let t = (w.x * e.y - w.y * e.x) / den;
                        let u = (w.x * v.y - w.y * v.x) / den;
                        if (0.0..=1.0).contains(&u) && in_z(t) {
                            let n = Vec2::new(e.y, -e.x).normalize().expect("a segment");
                            let z = frame
                                .vec_to_world(Vec3::new(n.x, n.y, 0.0))
                                .normalize()
                                .expect("unit");
                            let plane =
                                Frame::from_zx(frame.plane_point(a), z, frame.z().get()).expect("frame");
                            out.push((t, Some(Surface::Plane(Plane { frame: plane }))));
                        }
                    }
                    Seg2::Arc { c, r, start, sweep } => {
                        let (wx, wy) = (l.x - c.x, l.y - c.y);
                        let qa = v.x * v.x + v.y * v.y;
                        for root in
                            poly::quadratic(qa, 2.0 * (wx * v.x + wy * v.y), wx * wx + wy * wy - r * r)
                        {
                            let p = at(root.x);
                            if in_z(root.x) && on_sweep(dmath::atan2(p.y - c.y, p.x - c.x), start, sweep) {
                                let axis = frame.with_origin(frame.plane_point(c));
                                out.push((
                                    root.x,
                                    Some(Surface::Cylinder(Cylinder {
                                        frame: axis,
                                        radius: r,
                                    })),
                                ));
                            }
                        }
                    }
                }
            }
        }
        Shape::Revolved {
            frame,
            origin,
            dir,
            perp,
            segs,
            angle,
        } => {
            let (l, v) = (frame.point_to_local(o), frame.vec_to_local(d));
            let hr = |q: Point2| ((q - *origin).dot(*dir), (q - *origin).dot(*perp));
            let axis_at = |h: f64| frame.with_origin(frame.point_to_world(Point3::new(0.0, 0.0, h)));
            let s2 = [
                v.x * v.x + v.y * v.y,
                2.0 * (l.x * v.x + l.y * v.y),
                l.x * l.x + l.y * l.y,
            ];
            let radius = |t: f64| ((l.x + v.x * t).powi(2) + (l.y + v.y * t).powi(2)).sqrt();
            for s in segs {
                match *s {
                    Seg2::Line { a, b } => {
                        let ((ha, ra), (hb, rb)) = (hr(a), hr(b));
                        let (h0, h1) = (ha.min(hb), ha.max(hb));
                        if (hb - ha).abs() <= 1e-12 {
                            if v.z.abs() > 1e-12 {
                                let t = (ha - l.z) / v.z;
                                let r = radius(t);
                                if r >= ra.min(rb) && r <= ra.max(rb) {
                                    out.push((t, Some(Surface::Plane(Plane { frame: axis_at(ha) }))));
                                }
                            }
                            continue;
                        }
                        let k = (rb - ra) / (hb - ha);
                        let (r0, r1) = (ra + k * (l.z - ha), k * v.z);
                        for root in poly::quadratic(s2[0] - r1 * r1, s2[1] - 2.0 * r0 * r1, s2[2] - r0 * r0) {
                            let h = l.z + v.z * root.x;
                            if r0 + r1 * root.x < 0.0 || h < h0 || h > h1 {
                                continue;
                            }
                            let surface = if k.abs() <= 1e-12 {
                                Surface::Cylinder(Cylinder {
                                    frame: axis_at(ha),
                                    radius: ra,
                                })
                            } else {
                                let f = axis_at(ha);
                                let f = if k > 0.0 { f } else { f.flipped() };
                                match Cone::new(f, ra, dmath::atan(k.abs())) {
                                    Some(c) => Surface::Cone(c),
                                    None => continue,
                                }
                            };
                            out.push((root.x, Some(surface)));
                        }
                    }
                    Seg2::Arc { c, r, start, sweep } => {
                        let (hc, rc) = hr(c);
                        let w = [(l.z - hc).powi(2), 2.0 * (l.z - hc) * v.z, v.z * v.z];
                        let f = [s2[2] + w[0] + rc * rc - r * r, s2[1] + w[1], s2[0] + w[2]];
                        let roots: Vec<f64> = if rc.abs() <= 1e-12 {
                            poly::quadratic(f[2], f[1], f[0])
                                .into_iter()
                                .map(|x| x.x)
                                .collect()
                        } else {
                            let c4 = [
                                f[0] * f[0] - 4.0 * rc * rc * s2[2],
                                2.0 * f[0] * f[1] - 4.0 * rc * rc * s2[1],
                                f[1] * f[1] + 2.0 * f[0] * f[2] - 4.0 * rc * rc * s2[0],
                                2.0 * f[1] * f[2],
                                f[2] * f[2],
                            ];
                            poly::all_real_roots(&c4).into_iter().map(|x| x.x).collect()
                        };
                        for t in roots {
                            let q = *origin + *dir * (l.z + v.z * t) + *perp * radius(t);
                            if !on_sweep(dmath::atan2(q.y - c.y, q.x - c.x), start, sweep) {
                                continue;
                            }
                            let surface = (rc.abs() <= 1e-12)
                                .then(|| sphere(frame.point_to_world(Point3::new(0.0, 0.0, hc)), r));
                            out.push((t, surface));
                        }
                    }
                }
            }
            if let Some(angle) = angle {
                for th in [0.0, *angle] {
                    let (s, c) = dmath::sin_cos(th);
                    let n = Vec3::new(-s, c, 0.0);
                    let den = n.dot(v);
                    if den.abs() <= 1e-12 {
                        continue;
                    }
                    let t = -n.dot(l.to_vec()) / den;
                    if (l.x + v.x * t) * c + (l.y + v.y * t) * s >= 0.0 {
                        let z = frame.vec_to_world(n).normalize().expect("unit");
                        let plane = Frame::from_zx(frame.origin, z, frame.z().get()).expect("frame");
                        out.push((t, Some(Surface::Plane(Plane { frame: plane }))));
                    }
                }
            }
        }
    }
}

fn sphere(c: Point3, r: f64) -> Surface {
    Surface::Sphere(Sphere {
        frame: Frame::WORLD.with_origin(c),
        radius: r,
    })
}

fn on_sweep(a: f64, start: f64, sweep: f64) -> bool {
    if sweep.abs() >= dmath::TAU - 1e-12 {
        return true;
    }
    let (lo, w) = if sweep >= 0.0 {
        (start, sweep)
    } else {
        (start + sweep, -sweep)
    };
    (a - lo).rem_euclid(dmath::TAU) <= w
}

/// Whether the part the tools make holds `p` (None: on a boundary).
fn holds(tools: &[(Shape, Op)], p: Point3) -> Option<bool> {
    let mut v = false;
    for (k, (sh, op)) in tools.iter().enumerate() {
        let m = match sh.member(p, 0.0) {
            Member::Near => return None,
            m => m == Member::In,
        };
        v = match (k, op) {
            (0, _) => m,
            (_, Op::Union) => v || m,
            (_, Op::Subtract) => v && !m,
            (_, Op::Intersect) => v && m,
        };
    }
    Some(v)
}

/// The surface of the first face the ray from `o` along `d` crosses, in the part the tools
/// (first, then each combined by its op) make.
pub(crate) fn next_face(tools: &[(Shape, Op)], o: Point3, d: Vec3) -> Result<Surface, ExtentError> {
    let mut cand = Vec::new();
    for (sh, _) in tools {
        hits(sh, o, d, &mut cand);
    }
    let eps = tol::LINEAR;
    cand.retain(|c| c.0 > eps);
    cand.sort_by(|a, b| a.0.total_cmp(&b.0));
    let at = |t: f64| o + d * t;
    let first = cand.first().ok_or(ExtentError::NoNextFace)?.0;
    let was = holds(tools, at(first / 2.0)).ok_or(ExtentError::Ambiguous)?;
    let mut k = 0;
    while k < cand.len() {
        let t = cand[k].0;
        let mut j = k;
        while j < cand.len() && cand[j].0 - t <= eps {
            j += 1;
        }
        let next = cand.get(j).map_or(t + 1.0, |c| c.0);
        if holds(tools, at((t + next) / 2.0)).ok_or(ExtentError::Ambiguous)? != was {
            let s = cand[k]
                .1
                .clone()
                .ok_or(ExtentError::Unsupported("ending on a torus"))?;
            if cand[k..j].iter().any(|c| c.1.as_ref() != Some(&s)) {
                return Err(ExtentError::Ambiguous);
            }
            return Ok(s);
        }
        k = j;
    }
    Err(ExtentError::NoNextFace)
}

/// Points just inside a profile (one per outer segment, a hair in from its middle) to cast
/// rays from, in the sketch's coordinates.
pub(crate) fn samples(profile: &Profile) -> Vec<Point2> {
    let Some(outer) = profile.loops.first() else {
        return vec![];
    };
    let ccw = outer.segments.iter().map(|(s, _)| s.area_term()).sum::<f64>() > 0.0;
    let hair = 1e-3;
    outer
        .segments
        .iter()
        .map(|(s, _)| {
            // the chord either side of the middle runs along it there
            let q = s.samples(4);
            let m = q[2];
            let along = (q[3] - q[1]).normalize().unwrap_or(Vec2::X);
            let left = Vec2::new(-along.y, along.x);
            m + if ccw { left } else { left * -1.0 } * hair
        })
        .collect()
}
