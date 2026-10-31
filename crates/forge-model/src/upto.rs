//! Ends on a surface: a sweep cut off where it first meets a face's surface (any plane, a
//! sphere, a cylinder, a cone or a torus, offset or not), and the next face along the way.
//!
//! Each ray of the sweep must meet the surface once before it leaves the near side of a
//! separating plane: through a sphere's centre square to the travel, through a cylinder's axis,
//! through a cone's apex and the two lines it shows in outline, through a torus's middle or its
//! axis (a torus is met along its axis or square to it; square to it from outside, its hole is
//! filled first, and from its hole the cut is at the tube's middle circle). Where the plane is
//! square to the travel the sweep simply ends on it; otherwise a box keeps the near side. So the
//! tool is the sweep less the solid inside the surface (from outside), or kept inside it (from
//! inside): one n-ary boolean of plain solids, whose analytic shapes combine the same way. Where
//! some of the profile misses the surface, or the surface cuts the sketch, the tool shows it (a
//! face of a separating solid or on the sweep's far end, or a start cap no longer whole), and the
//! feature is refused.
//!
//! The next face is picked by rays against the earlier tools' exact solids, then proved for the
//! whole sweep by two booleans with the earlier tools near it (`verify_next`).

use crate::{Combine, ExtentError, Feature, History, RegenError, Tool, Turn, tool};
use forge_bool::{Member, Op, Seg2, Shape};
use forge_geom::{Cone, CurveGeom, Cylinder, Plane, Sphere, Surface, SurfaceGeom, Torus};
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3, dmath, poly, tol};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment};
use forge_topo::{Body, Name};

/// How a sweep is cut off at a surface: solids to keep or take away, in order, each marked
/// when none of its faces may show in the result (a separating bound: a face of it there means
/// a ray missed the surface).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Plan {
    aux: Vec<(Op, Feature, bool)>,
    /// A torus every face of the result on which must face the travel (its end), proved after
    /// the build: a slanted torus's cut-off is placed from rays, so this is its exact check.
    facing: Option<Torus>,
}

fn keep(f: Feature) -> (Op, Feature, bool) {
    (Op::Intersect, f, false)
}

fn less(f: Feature) -> (Op, Feature, bool) {
    (Op::Subtract, f, false)
}

fn bound(f: Feature) -> (Op, Feature, bool) {
    (Op::Intersect, f, true)
}

/// A solid of revolution about the line through `c` along `a` of a loop in the half plane
/// (radius, height), its seams square to the side `refp` is on.
fn spun(c: Point3, a: UnitVec3, refp: Point3, l: ProfileLoop) -> Feature {
    let f = seam_away(c, a.get(), refp);
    Feature::Revolve {
        profile: Profile {
            plane: Frame::from_zx(c, f.x(), f.y().get()).expect("frame"),
            loops: vec![l],
        },
        axis: Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        extent: Turn::Angle(dmath::TAU),
        op: Combine::Cut,
    }
}

/// A rod of radius `r` along the line through `c` along `a`, covering heights `lo..hi`.
fn drum(c: Point3, a: UnitVec3, r: f64, (lo, hi): (f64, f64), refp: Point3) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: seam_away(c + a * (lo - 1.0), a.get(), refp),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        extent: crate::Extent::Distance(hi - lo + 2.0),
        op: Combine::Cut,
    }
}

fn polygon(pts: &[Point2]) -> ProfileLoop {
    let n = pts.len();
    ProfileLoop::new(
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
    )
}

/// A box on the near side of the plane through `at` square to `z` (z pointing away from the
/// near side), covering `pts`.
pub(crate) fn near_box(at: Point3, z: UnitVec3, pts: &[Point3]) -> Feature {
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

/// A frame at `o` whose xy plane holds the line along `along` through `o` and lies square to
/// the way `away` is from it (any frame where `away` is on the line): a solid made from it puts
/// its seams (circles start at x; revolves split at the xy plane) away from that side.
fn seam_away(o: Point3, along: Vec3, away: Point3) -> Frame {
    let w = away - o;
    let w = w - along * (along.dot(w) / along.dot(along));
    let z = along.normalize().expect("a direction");
    match w.normalize().filter(|_| w.length() > tol::LINEAR) {
        // z along the line, x straight away from the far side
        Some(n) => Frame::from_zx(o, z, -n.get()).expect("frame"),
        None => Frame::from_z(o, z),
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
            // the end itself lies on the plane: every ray meets it
            let plan = Plan {
                facing: None,
                aux: vec![keep(near_box(at, z, &swept(t)))],
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
                // poles on the travel line through the centre, off the sweep's end plane; split
                // along a plane holding it, square to the side the sweep is on
                frame: {
                    let f = seam_away(c, d.get(), refp);
                    Frame::from_zx(c, f.x(), dv.cross(f.x().get())).expect("frame")
                },
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
                        facing: None,
                        aux: vec![keep(ball)],
                    },
                ));
            }
            // the sweep's own end on the plane through the centre (the sketch is square to the
            // travel) keeps it short of where any ray leaves the ball
            let t = dv.dot(c - refp);
            if t <= tol::LINEAR {
                return Err(ExtentError::NotAhead);
            }
            Ok((
                t,
                Plan {
                    facing: None,
                    aux: vec![less(ball)],
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
            let rod = drum(at, a, r, span(&swept(t), at, a.get()), refp);
            if inside {
                return Ok((
                    t,
                    Plan {
                        facing: None,
                        aux: vec![keep(rod)],
                    },
                ));
            }
            if across.dot(dv) >= 1.0 - tol::ANGULAR {
                // travel square to the axis: the sweep's own end on the plane through it
                return Ok((
                    across.dot(at - refp),
                    Plan {
                        facing: None,
                        aux: vec![less(rod)],
                    },
                ));
            }
            Ok((
                t,
                Plan {
                    facing: None,
                    aux: vec![bound(near_box(at, across, &swept(t))), less(rod)],
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
                (far * (1.0 + ca) / (ca - ad) + 1.0, None)
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
                (-lo / n.dot(dv) + 1.0, Some((apex, n)))
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
            let tip = spun(apex, a, refp, polygon(&pts));
            let aux = match walls {
                Some((p, n)) => vec![bound(near_box(p, n, &swept(t))), less(tip)],
                None => vec![keep(tip)],
            };
            Ok((t, Plan { facing: None, aux }))
        }
        Surface::Torus(tr) => torus(tr, d, foot, refp, offset),
        Surface::Procedural(_) => Err(ExtentError::Unsupported("ending on a blend")),
    }
}

/// The cut-off at a ring torus, met along its axis or square to it. From outside, along the
/// axis: the near half, cut at the middle plane; square to it, the outer envelope (the torus
/// with its hole filled), cut at the plane through the axis. From inside the tube: kept inside
/// it (cut at the plane through the axis when heading for the hole). From the hole, square to
/// the axis: kept inside the tube's middle circle, less the torus.
fn torus(
    tr: &Torus,
    d: UnitVec3,
    foot: &[Point3],
    refp: Point3,
    offset: f64,
) -> Result<(f64, Plan), ExtentError> {
    let dv = d.get();
    let (a, c) = (tr.frame.z(), tr.frame.origin);
    let off = refp - c;
    let h = a.dot(off);
    let rho = (off - a * h).length();
    let inside = ((rho - tr.major).powi(2) + h * h).sqrt() < tr.minor;
    let r = if inside {
        tr.minor + offset
    } else {
        tr.minor - offset
    };
    if !(r > tol::LINEAR && r < tr.major) {
        return Err(ExtentError::BadSize);
    }
    let big = 2.0 * (tr.major + r) + 1.0;
    let ring = spun(
        c,
        a,
        refp,
        ProfileLoop::new(vec![(
            Segment::Circle {
                center: Point2::new(tr.major, 0.0),
                radius: r,
                ccw: true,
            },
            0,
        )]),
    );
    let ahead = dv.dot(c - refp);
    let ad = a.dot(dv);
    if ad.abs() >= 1.0 - tol::ANGULAR {
        if inside {
            return Ok((
                ahead + r + 1.0,
                Plan {
                    facing: None,
                    aux: vec![keep(ring)],
                },
            ));
        }
        if ahead <= tol::LINEAR {
            return Err(ExtentError::NotAhead);
        }
        return Ok((
            ahead,
            Plan {
                facing: None,
                aux: vec![less(ring)],
            },
        ));
    }
    if ad.abs() > tol::ANGULAR {
        // at a slant: the sweep ends on a plane square to the travel between where rays across
        // the profile first meet the torus and where they first leave it (or, from inside, leave
        // it and meet it again); `faces_ahead` then proves every ray of the tool ends at its
        // first meeting
        let k = Torus {
            frame: tr.frame,
            major: tr.major,
            minor: r,
        };
        let (mut last, mut next) = (f64::NEG_INFINITY, f64::INFINITY);
        let grid = (0..5).flat_map(|i| (0..5).map(move |j| (i as f64 / 4.0, j as f64 / 4.0)));
        let rays = grid
            .map(|(u, v)| foot[0] + (foot[1] - foot[0]) * u + (foot[2] - foot[0]) * v)
            .chain([refp]);
        for p in rays {
            let ts = ray_torus(&k, p, dv);
            if let Some(&t0) = ts.first() {
                last = last.max(t0);
                if let Some(&t1) = ts.get(1) {
                    next = next.min(t1);
                }
            }
        }
        if !(last > tol::LINEAR && last < next) {
            return Err(ExtentError::Unsupported(
                "a torus met at a slant with no plane between its sides",
            ));
        }
        let t = if next.is_finite() {
            (last + next) / 2.0
        } else {
            last + 1.0
        };
        let aux = vec![if inside { keep(ring) } else { less(ring) }];
        return Ok((t, Plan { facing: Some(k), aux }));
    }
    if inside {
        let t = if ahead > tol::LINEAR { ahead } else { big };
        return Ok((
            t,
            Plan {
                facing: None,
                aux: vec![keep(ring)],
            },
        ));
    }
    if rho < tr.major && h.abs() < r {
        // in the hole: rays meet the tube's inside before its middle circle
        let core = drum(c, a, tr.major, (-r, r), refp);
        return Ok((
            big,
            Plan {
                facing: None,
                aux: vec![bound(core), less(ring)],
            },
        ));
    }
    if ahead <= tol::LINEAR {
        return Err(ExtentError::NotAhead);
    }
    let arc = Segment::Arc {
        center: Point2::new(tr.major, 0.0),
        radius: r,
        start: -dmath::FRAC_PI_2,
        sweep: dmath::PI,
    };
    let filled = ProfileLoop::new(vec![
        (
            Segment::Line {
                a: Point2::new(0.0, -r),
                b: Point2::new(tr.major, -r),
            },
            0,
        ),
        (arc, 1),
        (
            Segment::Line {
                a: Point2::new(tr.major, r),
                b: Point2::new(0.0, r),
            },
            2,
        ),
        (
            Segment::Line {
                a: Point2::new(0.0, r),
                b: Point2::new(0.0, -r),
            },
            3,
        ),
    ]);
    Ok((
        ahead,
        Plan {
            facing: None,
            aux: vec![less(spun(c, a, refp, filled))],
        },
    ))
}

/// Where the ray from `o` along `d` crosses a torus, ascending, ahead of `o`.
fn ray_torus(tr: &Torus, o: Point3, d: Vec3) -> Vec<f64> {
    let (l, v) = (tr.frame.point_to_local(o), tr.frame.vec_to_local(d));
    let s2 = [
        l.x * l.x + l.y * l.y,
        2.0 * (l.x * v.x + l.y * v.y),
        v.x * v.x + v.y * v.y,
    ];
    let (rc, r) = (tr.major, tr.minor);
    let f = [
        s2[0] + l.z * l.z + rc * rc - r * r,
        s2[1] + 2.0 * l.z * v.z,
        s2[2] + v.z * v.z,
    ];
    let c4 = [
        f[0] * f[0] - 4.0 * rc * rc * s2[0],
        2.0 * f[0] * f[1] - 4.0 * rc * rc * s2[1],
        f[1] * f[1] + 2.0 * f[0] * f[2] - 4.0 * rc * rc * s2[2],
        2.0 * f[1] * f[2],
        f[2] * f[2],
    ];
    let mut ts: Vec<f64> = poly::all_real_roots(&c4)
        .into_iter()
        .filter(|x| !x.touching && x.x > tol::LINEAR)
        .map(|x| x.x)
        .collect();
    ts.sort_by(f64::total_cmp);
    ts
}

/// Whether every face of `b` on torus `tr` faces the travel `d` (its outward normal has a
/// positive part along d) all over: proved on each face's edges with interval bounds of the
/// torus's normal along d. The outline where it turns is a loop round the axis; when the axis
/// misses the box `sweep` holding the faces, no such loop fits inside one, so it can only enter
/// a face across its edges.
fn faces_ahead(b: &Body, tr: &Torus, d: Vec3, sweep: &forge_math::Box3) -> bool {
    let f = tr.frame;
    let v = f.vec_to_local(d);
    // the normal's part along d, up to a positive factor, over a box: in the torus's frame
    let along = |bx: &forge_math::Box3| -> forge_math::Interval {
        let local =
            |e: Vec3| (bx.x + -f.origin.x) * e.x + (bx.y + -f.origin.y) * e.y + (bx.z + -f.origin.z) * e.z;
        let (x, y, z) = (local(f.x().get()), local(f.y().get()), local(f.z().get()));
        let rho = (x.sqr() + y.sqr()).sqrt();
        ((rho + -tr.major) * (x * v.x + y * v.y)).div(&rho) + z * v.z
    };
    // whether the axis line passes through a box (slab by slab)
    let axis_meets = |bx: &forge_math::Box3| {
        let bx = bx.grow(tol::LINEAR);
        let (mut t0, mut t1) = (f64::NEG_INFINITY, f64::INFINITY);
        for (k, iv) in [bx.x, bx.y, bx.z].into_iter().enumerate() {
            let (o, z) = (f.origin[k], f.z().get()[k]);
            if z.abs() < 1e-15 {
                if !iv.contains(o) {
                    return false;
                }
            } else {
                let (a, b) = ((iv.lo - o) / z, (iv.hi - o) / z);
                t0 = t0.max(a.min(b));
                t1 = t1.min(a.max(b));
            }
        }
        t0 <= t1
    };
    if axis_meets(sweep) {
        return false;
    }
    let target = Surface::Torus(*tr);
    for fid in b.face_ids() {
        let face = b.face(fid);
        if !same_surface(&face.surface, &target) {
            continue;
        }
        // the sign the normal's part along d must keep: from the outward normal at a corner
        let lp = face.loops[0];
        let c0 = b.lp(lp).coedges[0];
        let p = b.vertex(b.coedge_start(c0)).point;
        let Some(out) = b.face_normal(fid, face.surface.project(p)) else {
            return false;
        };
        // the end faces the travel there, and the normal's part along d keeps its sign there
        if out.dot(d) <= 0.0 {
            return false;
        }
        let natural = along(&forge_math::Box3::from_point(p)).mid().signum();
        for &l in &face.loops {
            for &c in &b.lp(l).coedges {
                let e = b.edge(b.coedge(c).edge);
                let mut todo = vec![(e.range, 0)];
                while let Some((r, depth)) = todo.pop() {
                    let g = along(&e.curve.bbox(r));
                    let clear = if natural > 0.0 { g.lo > 0.0 } else { g.hi < 0.0 };
                    if clear {
                        continue;
                    }
                    if depth > 24 {
                        return false;
                    }
                    let (a, b2) = r.split();
                    todo.push((a, depth + 1));
                    todo.push((b2, depth + 1));
                }
            }
        }
    }
    true
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
    id: u64,
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
        .map(|(k, (_, f, _))| tool(i, id + 2_000_000 + k as u64 * 1_000_000, f, build))
        .collect::<Result<_, _>>()?;
    let shape = Shape::Csg {
        first: Box::new(base.2.clone()),
        rest: plan
            .aux
            .iter()
            .zip(&aux)
            .map(|((op, ..), t)| (*op, t.2.clone()))
            .collect(),
    };
    if !build {
        return Ok((Body::new(), base.1, shape));
    }
    let facing = plan.facing;
    let mut bodies = vec![&base.0];
    bodies.extend(aux.iter().map(|t| &t.0));
    let ops: Vec<Op> = plan.aux.iter().map(|a| a.0).collect();
    let bounds: Vec<&Surface> = plan
        .aux
        .iter()
        .zip(&aux)
        .filter(|(a, _)| a.2)
        .flat_map(|(_, t)| t.0.face_ids().map(|f| &t.0.face(f).surface))
        .collect();
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
    .map_err(|error| RegenError::Boolean { feature: i, error })?;
    for f in body.face_ids() {
        let s = &body.face(f).surface;
        if on_plane(s, far.0, far.1) || bounds.contains(&s) {
            return Err(bad(ExtentError::NotCovered));
        }
    }
    // each start face comes through whole (a face trimmed to one piece keeps its name)
    let counts = |b: &Body, f: forge_topo::FaceId| {
        let loops = &b.face(f).loops;
        (
            loops.len(),
            loops.iter().map(|&l| b.lp(l).coedges.len()).sum::<usize>(),
        )
    };
    let close = |a: forge_math::Interval, b: forge_math::Interval| {
        (a.lo - b.lo).abs() <= tol::LINEAR && (a.hi - b.hi).abs() <= tol::LINEAR
    };
    for &n in start {
        let same = match (base.0.face_by_name(n), body.face_by_name(n)) {
            (Some(w), Some(h)) if counts(&base.0, w) == counts(&body, h) => {
                let (w, h) = (base.0.face_box(w), body.face_box(h));
                close(w.x, h.x) && close(w.y, h.y) && close(w.z, h.z)
            }
            (None, None) => true,
            _ => false,
        };
        if !same {
            return Err(bad(ExtentError::NotAhead));
        }
    }
    if let Some(tr) = facing
        && !faces_ahead(&body, &tr, far.1.get(), &shape_box(&base.2))
    {
        return Err(bad(ExtentError::Unsupported(
            "a torus met at a slant where some rays graze it or meet it again first",
        )));
    }
    Ok((body, base.1, shape))
}

/// The surface a prism's wall from segment `s` lies on.
fn prism_wall(frame: &Frame, s: &Seg2) -> Surface {
    match *s {
        Seg2::Line { a, b } => {
            let e = b - a;
            let n = Vec2::new(e.y, -e.x).normalize().expect("a segment");
            let z = frame
                .vec_to_world(Vec3::new(n.x, n.y, 0.0))
                .normalize()
                .expect("unit");
            Surface::Plane(Plane {
                frame: Frame::from_zx(frame.plane_point(a), z, frame.z().get()).expect("frame"),
            })
        }
        Seg2::Arc { c, r, .. } => Surface::Cylinder(Cylinder {
            frame: frame.with_origin(frame.plane_point(c)),
            radius: r,
        }),
    }
}

/// The surface a revolved wall from segment `s` lies on (None for one on the axis, or a torus
/// thinner at its middle than its tube).
fn revolved_wall(frame: &Frame, origin: Point2, dir: Vec2, perp: Vec2, s: &Seg2) -> Option<Surface> {
    let hr = |q: Point2| ((q - origin).dot(dir), (q - origin).dot(perp));
    let axis_at = |h: f64| frame.with_origin(frame.point_to_world(Point3::new(0.0, 0.0, h)));
    Some(match *s {
        Seg2::Line { a, b } => {
            let ((ha, ra), (hb, rb)) = (hr(a), hr(b));
            if (hb - ha).abs() <= 1e-12 {
                Surface::Plane(Plane { frame: axis_at(ha) })
            } else if (rb - ra).abs() <= 1e-12 {
                if ra.abs() <= 1e-12 {
                    return None;
                }
                Surface::Cylinder(Cylinder {
                    frame: axis_at(ha),
                    radius: ra,
                })
            } else {
                let k = (rb - ra) / (hb - ha);
                let f = if k > 0.0 {
                    axis_at(ha)
                } else {
                    axis_at(ha).flipped()
                };
                Surface::Cone(Cone::new(f, ra, dmath::atan(k.abs()))?)
            }
        }
        Seg2::Arc { c, r, .. } => {
            let (hc, rc) = hr(c);
            if rc.abs() <= 1e-12 {
                sphere(frame.point_to_world(Point3::new(0.0, 0.0, hc)), r)
            } else if rc > r {
                Surface::Torus(Torus {
                    frame: axis_at(hc),
                    major: rc,
                    minor: r,
                })
            } else {
                return None;
            }
        }
    })
}

/// The plane through a revolve's axis at angle `th`.
fn sector(frame: &Frame, th: f64) -> Surface {
    let (s, c) = dmath::sin_cos(th);
    let z = frame
        .vec_to_world(Vec3::new(-s, c, 0.0))
        .normalize()
        .expect("unit");
    Surface::Plane(Plane {
        frame: Frame::from_zx(frame.origin, z, frame.z().get()).expect("frame"),
    })
}

/// The surface of the face `name` of feature `k`'s tool (`f`, an extrude, revolve or ball with
/// a plain extent), from its definition alone; None where it takes building the tool.
pub(crate) fn named_surface(f: &Feature, id: u64, name: Name) -> Option<Surface> {
    let root = Name::feature(id);
    let ids: Vec<u64> = match f {
        Feature::Extrude { profile, .. } | Feature::Revolve { profile, .. } => profile
            .loops
            .iter()
            .flat_map(|l| l.segments.iter().map(|s| s.1))
            .collect(),
        Feature::Ball { .. } => vec![0],
        _ => return None,
    };
    let at = |id: u64| ids.iter().position(|&i| i == id);
    match crate::shape(f) {
        Shape::Prism { frame, segs, hi, .. } => {
            if name == root.with_str("start") {
                return Some(Surface::Plane(Plane { frame }));
            }
            if name == root.with_str("end") {
                let top = frame.with_origin(frame.point_to_world(Point3::new(0.0, 0.0, hi)));
                return Some(Surface::Plane(Plane { frame: top }));
            }
            let id = ids.iter().find(|&&i| name == root.with_str("side").with(i))?;
            Some(prism_wall(&frame, &segs[at(*id)?]))
        }
        Shape::Revolved {
            frame,
            origin,
            dir,
            perp,
            segs,
            angle,
        } => {
            if let Some(a) = angle {
                if name == root.with_str("start") {
                    return Some(sector(&frame, 0.0));
                }
                if name == root.with_str("end") {
                    return Some(sector(&frame, a));
                }
            }
            let id = ids
                .iter()
                .find(|&&i| (0..2).any(|h| name == root.with_str("side").with(i).with(h)))?;
            revolved_wall(&frame, origin, dir, perp, &segs[at(*id)?])
        }
        Shape::Ball { center, radius } => (0..2)
            .any(|h| name == root.with_str("side").with(0).with(h))
            .then(|| sphere(center, radius)),
        Shape::Csg { .. } | Shape::Coil { .. } | Shape::Brep(_) => None,
    }
}

/// Where a plain solid's boundary may cross the line `o + t d`: each t with the surface there.
fn hits(sh: &Shape, o: Point3, d: Vec3, out: &mut Vec<(f64, Option<Surface>)>) {
    match sh {
        // no closed form to cross: a mark, and next_face refuses rays through these
        Shape::Brep(b) => {
            if ray_meets(b, o, d) {
                out.push((f64::NAN, None));
            }
        }
        Shape::Coil { .. } => out.push((f64::NAN, None)),
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
                            out.push((t, Some(prism_wall(frame, s))));
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
                                out.push((root.x, Some(prism_wall(frame, s))));
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
            let s2 = [
                v.x * v.x + v.y * v.y,
                2.0 * (l.x * v.x + l.y * v.y),
                l.x * l.x + l.y * l.y,
            ];
            let radius = |t: f64| ((l.x + v.x * t).powi(2) + (l.y + v.y * t).powi(2)).sqrt();
            let wall = |s: &Seg2| revolved_wall(frame, *origin, *dir, *perp, s);
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
                                    out.push((t, wall(s)));
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
                            out.push((root.x, wall(s)));
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
                            if on_sweep(dmath::atan2(q.y - c.y, q.x - c.x), start, sweep) {
                                out.push((t, wall(s)));
                            }
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
                        out.push((t, Some(sector(frame, th))));
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

/// Whether the ray from `o` along `d` meets box `b`.
fn ray_meets(b: &forge_math::Box3, o: Point3, d: Vec3) -> bool {
    let (mut lo, mut hi) = (0.0f64, f64::INFINITY);
    for (oi, di, iv) in [(o.x, d.x, b.x), (o.y, d.y, b.y), (o.z, d.z, b.z)] {
        if di == 0.0 {
            if oi < iv.lo || oi > iv.hi {
                return false;
            }
            continue;
        }
        let (t0, t1) = ((iv.lo - oi) / di, (iv.hi - oi) / di);
        lo = lo.max(t0.min(t1));
        hi = hi.min(t0.max(t1));
    }
    lo <= hi
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
pub(crate) fn next_face(tools: &[(Shape, Op)], o: Point3, d: Vec3) -> Result<(Surface, bool), ExtentError> {
    let mut cand = Vec::new();
    for (sh, _) in tools {
        hits(sh, o, d, &mut cand);
    }
    // a part with no closed-form faces on the ray leaves a mark (NaN) instead of hits
    if cand.iter().any(|c| c.0.is_nan()) {
        return Err(ExtentError::Unsupported("the next face past a coil or loft"));
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
            return Ok((s, was));
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

/// The names of a body's faces.
pub(crate) fn names(b: &Body) -> Vec<Name> {
    b.face_ids().map(|f| b.face(f).name).collect()
}

/// Whether `n` is one of `from` or a piece a boolean split from one.
fn derived(from: &[Name], n: Name) -> bool {
    from.iter().any(|&b| n == b || n.is_piece_of(b))
}

/// Whether a sweep (box `sweep`) from outside convex solid `sh`, ending on its surface `s`
/// (the end within `end`), meets none of it and ends wholly on its skin: a ball's sphere, or a
/// convex prism's cap with the sweep beyond it and the end over the cap.
fn skin_under(sh: &Shape, s: &Surface, sweep: &forge_math::Box3, end: &forge_math::Box3) -> bool {
    let eps = 2.0 * tol::LINEAR;
    match (sh, s) {
        (Shape::Ball { center, radius }, Surface::Sphere(sp)) => {
            sp.frame.origin.distance(*center) <= tol::LINEAR && (sp.radius - radius).abs() <= tol::LINEAR
        }
        (Shape::Prism { frame, lo, hi, .. }, Surface::Plane(p)) => {
            if p.frame.z().dot(frame.z().get()).abs() < 1.0 - tol::ANGULAR {
                return false;
            }
            let h = frame.point_to_local(p.frame.origin).z;
            let (cap, up) = if (h - hi).abs() <= tol::LINEAR {
                (*hi, true)
            } else if (h - lo).abs() <= tol::LINEAR {
                (*lo, false)
            } else {
                return false;
            };
            let z = |q: Point3| frame.point_to_local(q).z;
            let beyond = crate::pattern::corners(sweep)
                .iter()
                .all(|&q| if up { z(q) >= cap - eps } else { z(q) <= cap + eps });
            // the end's box squashed onto the cap must lie over it
            let over = crate::pattern::corners(end).iter().all(|&q| {
                let l = frame.point_to_local(q);
                let flat = frame.point_to_world(Point3::new(l.x, l.y, (lo + hi) / 2.0));
                convex_holds(sh, &forge_math::Box3::from_point(flat))
            });
            beyond && over
        }
        _ => false,
    }
}

/// Whether a convex analytic solid (a ball, or a prism of a convex polygon) holds the box,
/// within tolerance.
fn convex_holds(sh: &Shape, b: &forge_math::Box3) -> bool {
    let corners = crate::pattern::corners(b);
    let eps = tol::LINEAR;
    match sh {
        Shape::Ball { center, radius } => corners.iter().all(|p| p.distance(*center) <= radius + eps),
        Shape::Prism { frame, segs, lo, hi } => {
            let pts: Option<Vec<(Point2, Point2)>> = segs
                .iter()
                .map(|s| match *s {
                    Seg2::Line { a, b } => Some((a, b)),
                    Seg2::Arc { .. } => None,
                })
                .collect();
            let Some(pts) = pts else { return false };
            let n = pts.len();
            // one closed loop turning one way
            let closed = (0..n).all(|k| pts[k].1.distance(pts[(k + 1) % n].0) <= eps);
            let turn = |k: usize| (pts[k].1 - pts[k].0).cross(pts[(k + 1) % n].1 - pts[(k + 1) % n].0);
            let sign = (0..n).map(turn).sum::<f64>().signum();
            if !closed || n < 3 || (0..n).any(|k| turn(k) * sign < 0.0) {
                return false;
            }
            corners.iter().all(|p| {
                let l = frame.point_to_local(*p);
                let q = Point2::new(l.x, l.y);
                l.z >= lo - eps
                    && l.z <= hi + eps
                    && pts.iter().all(|(a, b)| {
                        let e = *b - *a;
                        e.cross(q - *a) * sign >= -eps * e.length()
                    })
            })
        }
        _ => false,
    }
}

/// A box holding a tool's analytic solid.
fn shape_box(sh: &Shape) -> forge_math::Box3 {
    let corners = |s: &Seg2| match *s {
        Seg2::Line { a, b } => vec![a, b],
        Seg2::Arc { c, r, .. } => vec![c + Vec2::new(-r, -r), c + Vec2::new(r, r)],
    };
    match sh {
        Shape::Brep(b) => *b,
        Shape::Coil {
            frame,
            a,
            b,
            angle,
            r,
            h,
            ..
        } => {
            let out = r.1 + (a * angle).max(0.0);
            let rise = b * angle;
            let (lo, hi) = (h.0 + rise.min(0.0), h.1 + rise.max(0.0));
            forge_math::Box3::from_points([
                frame.point_to_world(Point3::new(0.0, 0.0, lo)),
                frame.point_to_world(Point3::new(0.0, 0.0, hi)),
            ])
            .grow(out)
        }
        Shape::Csg { first, .. } => shape_box(first),
        Shape::Ball { center, radius } => forge_math::Box3::from_point(*center).grow(*radius),
        Shape::Prism { frame, segs, lo, hi } => forge_math::Box3::from_points(
            segs.iter()
                .flat_map(corners)
                .flat_map(|q| [*lo, *hi].map(|z| frame.point_to_world(Point3::new(q.x, q.y, z)))),
        ),
        Shape::Revolved {
            frame,
            origin,
            dir,
            perp,
            segs,
            ..
        } => {
            let (mut h0, mut h1, mut r) = (f64::INFINITY, f64::NEG_INFINITY, 0.0f64);
            for q in segs.iter().flat_map(corners) {
                let h = (q - *origin).dot(*dir);
                (h0, h1) = (h0.min(h), h1.max(h));
                r = r.max((q - *origin).dot(*perp).abs());
            }
            let at = |h: f64| frame.point_to_world(Point3::new(0.0, 0.0, h));
            // the discs at either end: each reaches r across the axis, less along it
            let z = frame.z().get();
            let reach = |c: f64| r * (1.0 - c * c).max(0.0).sqrt();
            let b = forge_math::Box3::from_points([at(h0), at(h1)]);
            forge_math::Box3 {
                x: b.x.grow(reach(z.x)),
                y: b.y.grow(reach(z.y)),
                z: b.z.grow(reach(z.z)),
            }
        }
    }
}

fn body_box(b: &Body) -> forge_math::Box3 {
    b.face_ids()
        .fold(forge_math::Box3::EMPTY, |acc, f| acc.hull(&b.face_box(f)))
}

/// Whether two surfaces are the same set of points.
fn same_surface(a: &Surface, b: &Surface) -> bool {
    let par = |u: UnitVec3, v: UnitVec3| u.dot(v.get()).abs() >= 1.0 - tol::ANGULAR;
    let on_axis = |o: Point3, z: UnitVec3, p: Point3| {
        let w = p - o;
        (w - z * z.dot(w)).length() <= tol::LINEAR
    };
    let near = |x: f64, y: f64| (x - y).abs() <= tol::LINEAR;
    match (a, b) {
        (Surface::Plane(p), Surface::Plane(q)) => {
            par(p.frame.z(), q.frame.z()) && near(p.frame.z().dot(q.frame.origin - p.frame.origin), 0.0)
        }
        (Surface::Cylinder(p), Surface::Cylinder(q)) => {
            par(p.frame.z(), q.frame.z())
                && on_axis(p.frame.origin, p.frame.z(), q.frame.origin)
                && near(p.radius, q.radius)
        }
        (Surface::Sphere(p), Surface::Sphere(q)) => {
            p.frame.origin.distance(q.frame.origin) <= tol::LINEAR && near(p.radius, q.radius)
        }
        (Surface::Cone(p), Surface::Cone(q)) => {
            p.apex().distance(q.apex()) <= tol::LINEAR
                && p.frame.z().dot(q.frame.z().get()) >= 1.0 - tol::ANGULAR
                && (p.sin_a() - q.sin_a()).abs() <= tol::ANGULAR
        }
        (Surface::Torus(p), Surface::Torus(q)) => {
            p.frame.origin.distance(q.frame.origin) <= tol::LINEAR
                && par(p.frame.z(), q.frame.z())
                && near(p.major, q.major)
                && near(p.minor, q.minor)
        }
        _ => a == b,
    }
}

/// Proves, for the whole sweep `t` of feature `i` (up to the next face: `material` says whether
/// it starts in the part), what the rays only picked: that no skin of the part lies inside it
/// (the part near it, from the earlier tools, meets none of it, or holds all of it), and that
/// the part's skin lies under all of its end (the end, `end` or what a cut-off made, vanishes
/// into the part, or out of it for a cut). Both are exact booleans, not samples.
pub(crate) fn verify_next(
    features: History<'_>,
    i: usize,
    (t, shape): (&Body, &Shape),
    base: &[Name],
    end: &[Name],
    next: &crate::extent::Next,
) -> Result<(), RegenError> {
    let material = next.material;
    let bad = |error| RegenError::Extent { feature: i, error };
    let ends: Vec<Name> = names(t)
        .into_iter()
        .filter(|&n| derived(end, n) || !derived(base, n))
        .collect();
    // the part near the sweep, from the solids the pick looked at: a tool clear of it holds none
    // of it, so the history folds the same without it (an intersection with one empties it)
    let tb = body_box(t).grow(tol::LINEAR);
    let mut close: Vec<(usize, usize, Op)> = Vec::new();
    let mut at = (usize::MAX, 0);
    for (n, (k, sh, op)) in next.tools.iter().enumerate() {
        // its place among its feature's tools
        at = if at.0 == *k { (*k, at.1 + 1) } else { (*k, 0) };
        let op = if n == 0 { Op::Union } else { *op };
        if !shape_box(sh).overlaps(&tb) {
            if op == Op::Intersect {
                close.clear();
            }
        } else if !close.is_empty() || op == Op::Union {
            close.push((n, at.1, op));
        }
    }
    if close.is_empty() {
        return Err(bad(ExtentError::NoNextFace));
    }
    // one convex solid holding the whole sweep is the part there: nothing inside it, and the end
    // (on its surface, inside it) lies on its skin
    if let [(n, ..)] = close[..]
        && material
        && convex_holds(&next.tools[n].1, &shape_box(shape))
    {
        return Ok(());
    }
    // from outside, solids whose boxes only touch the sweep's share none of it; if one convex
    // solid is left, the sweep was cut off at its own surface and its end lies on its skin
    let sb = shape_box(shape);
    let touches = |sh: &Shape| {
        let b = shape_box(sh);
        [(b.x, sb.x), (b.y, sb.y), (b.z, sb.z)]
            .iter()
            .any(|(u, v)| u.hi.min(v.hi) - u.lo.max(v.lo) <= tol::LINEAR)
    };
    let inside: Vec<&(usize, usize, Op)> = close.iter().filter(|c| !touches(&next.tools[c.0].1)).collect();
    if !material
        && let [&(n, _, Op::Union)] = inside[..]
        && close.iter().all(|c| c.2 != Op::Intersect)
    {
        let end_box = t
            .face_ids()
            .filter(|&f| ends.contains(&t.face(f).name))
            .fold(forge_math::Box3::EMPTY, |b, f| b.hull(&t.face_box(f)));
        if skin_under(&next.tools[n].1, &next.surface, &sb, &end_box) {
            return Ok(());
        }
    }
    // only the tools near the sweep are built
    let mut built: Vec<(Body, Op)> = Vec::new();
    let mut k_built: Option<(usize, Vec<crate::Tool>)> = None;
    for &(n, j, op) in &close {
        let k = next.tools[n].0;
        if k_built.as_ref().is_none_or(|b| b.0 != k) {
            k_built = Some((k, crate::tools(features, k, true)?));
        }
        let tools = &k_built.as_ref().expect("built").1;
        built.push((tools[j].0.clone(), op));
    }
    let near: Vec<(&Body, Op)> = built.iter().map(|(b, op)| (b, *op)).collect();
    let with = |last: Op| -> Result<Body, RegenError> {
        let mut bodies: Vec<&Body> = near.iter().map(|x| x.0).collect();
        bodies.push(t);
        let mut ops: Vec<Op> = near.iter().skip(1).map(|x| x.1).collect();
        ops.push(last);
        // the pick's analytic solids place most points without the B-rep
        let mut shapes: Vec<Shape> = close.iter().map(|c| next.tools[c.0].1.clone()).collect();
        shapes.push(shape.clone());
        forge_bool::nary_with(
            &bodies,
            &ops,
            Some(&shapes),
            None,
            1,
            crate::cut_root(i + 4_000_000),
        )
        .map_err(|error| RegenError::Boolean { feature: i, error })
    };
    let inner = with(Op::Intersect)?;
    let clear = if material {
        let own: Vec<&Surface> = t.face_ids().map(|f| &t.face(f).surface).collect();
        inner.face_ids().next().is_some()
            && inner
                .face_ids()
                .all(|f| own.iter().any(|s| same_surface(s, &inner.face(f).surface)))
    } else {
        inner.face_ids().next().is_none()
    };
    if !clear {
        return Err(bad(ExtentError::Ambiguous));
    }
    let after = with(if material { Op::Subtract } else { Op::Union })?;
    if after.face_ids().any(|f| derived(&ends, after.face(f).name)) {
        return Err(bad(ExtentError::NotCovered));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Extent, Part, Target};

    /// The facing proof passes a slanted torus end that faces the travel, and fails it seen
    /// the other way.
    #[test]
    fn facing_proof() {
        let plane = Frame::from_zx(
            Point3::new(0.0, 0.0, 20.0),
            Vec3::Y.normalize().expect("y"),
            Vec3::X,
        )
        .expect("frame");
        let torus = Feature::Revolve {
            profile: Profile {
                plane,
                loops: vec![Profile::circle(Point2::new(10.0, 0.0), 3.0, 0)],
            },
            axis: Axis2 {
                origin: Point2::ORIGIN,
                dir: Vec2::Y,
            },
            extent: Turn::Angle(dmath::TAU),
            op: Combine::New,
        };
        let tilt = Vec3::new(0.3, 0.0, 1.0).normalize().expect("tilt");
        let rod = Feature::Extrude {
            profile: Profile {
                plane: Frame::from_zx(Point3::new(4.5, 0.0, 0.0), tilt, Vec3::Y).expect("frame"),
                loops: vec![Profile::circle(Point2::ORIGIN, 1.0, 0)],
            },
            extent: Extent::To {
                target: Target::Face {
                    feature: 0,
                    name: Name::feature(1).with_str("side").with(0).with(0),
                },
                offset: 0.0,
            },
            op: Combine::New,
        };
        // the rod is a body of its own beside the torus
        let b = Part {
            features: vec![torus, rod],
        }
        .regenerate_solids()
        .expect("slanted end")
        .pop()
        .expect("the rod")
        .body;
        let tr = b
            .face_ids()
            .find_map(|f| match &b.face(f).surface {
                Surface::Torus(t) => Some(*t),
                _ => None,
            })
            .expect("an end on the torus");
        // the rod's sweep: its edges' box, grown past where the end bulges between them
        let sweep = b
            .edge_ids()
            .fold(forge_math::Box3::EMPTY, |x, e| x.hull(&b.edge_box(e)))
            .grow(0.5);
        assert!(faces_ahead(&b, &tr, tilt.get(), &sweep));
        assert!(!faces_ahead(&b, &tr, -tilt.get(), &sweep));
    }
}
