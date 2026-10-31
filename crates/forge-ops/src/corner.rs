//! Vertex blends where blends meet at a vertex: where each blend stops (its setback), the
//! trim curve that ends each face there, and the surface that closes the remaining hole.

use forge_geom::corner::{Side, SurfaceRail, corner_surface, foot_velocity, trim_curve};
use forge_geom::{Curve, CurveGeom, Surface};
use forge_math::{Interval, Point3, Vec3, dmath, tol};
use std::sync::Arc;

use crate::fillet::{
    CornerBlend, CornerEdge, CornerInput, CornerPatch, FaceTrim, HoleSide, PatchFace, PatchUse, SideKind,
};

/// Where each edge stops: a spine parameter per blend, a curve parameter per sharp edge.
#[derive(Clone, Debug)]
struct Plan {
    blend: Vec<f64>,
    edge: Vec<f64>,
    /// Per face, where its two edges' curves on it meet (the vertex when both are sharp).
    corner: Vec<Point3>,
}

/// The setback of every blend: past the point where its springs meet the neighbours' by one
/// radius, or exactly at those points when a single ball touches every face (the sphere).
pub fn setbacks(c: &CornerInput) -> Option<Vec<f64>> {
    plan(c).ok().map(|p| p.blend)
}

/// The trim curve on each face, for the given blend setbacks. Faces whose two neighbours stop
/// at one point get none.
pub fn face_curves(c: &CornerInput, setbacks: &[f64]) -> Result<Vec<FaceTrim>, String> {
    let plan = plan(c)?;
    let n = c.edges.len();
    let mut out = Vec::new();
    for i in 0..n {
        let surface = &c.face_surfaces[i].0;
        let (left, right) = (&c.edges[(i + n - 1) % n], &c.edges[i]);
        // the end point, the spring's direction away from the vertex, and whether it is a cut edge
        let end_of = |e: &CornerEdge, j: usize| -> (Point3, Vec3, bool) {
            match e {
                CornerEdge::Blend(b) => {
                    let s = c.blends[*b].spine.deriv(setbacks[*b]);
                    let (p, v) = foot_velocity(surface, s.p, s.d1);
                    (p, v * c.blends[*b].out, false)
                }
                CornerEdge::Sharp { curve, .. } => (curve.eval(plan.edge[j]), Vec3::ZERO, true),
            }
        };
        let (p0, away0, sharp0) = end_of(left, (i + n - 1) % n);
        let (p1, away1, sharp1) = end_of(right, i);
        if p0.distance(p1) <= tol::LINEAR {
            continue;
        }
        // leaving a spring toward the vertex and arriving on the next one going away from it;
        // from or to a cut edge, through a control point halfway along the spring from where
        // the spring meets that edge (or inside the face between two cut edges)
        let q0 = plan.corner[i];
        let (t0, t1) = match (sharp0, sharp1) {
            (false, false) => (-away0, away1),
            (true, false) => (q0.lerp(p1, 0.5) - p0, away1),
            (false, true) => (-away0, p1 - q0.lerp(p0, 0.5)),
            (true, true) => {
                let q = Point3::new(
                    (q0.x + p0.x + p1.x) / 3.0,
                    (q0.y + p0.y + p1.y) / 3.0,
                    (q0.z + p0.z + p1.z) / 3.0,
                );
                (q - p0, p1 - q)
            }
        };
        let (curve, range) = trim_curve(surface, p0, t0, p1, t1)
            .ok_or_else(|| format!("face {i}: no trim curve fits between the springs"))?;
        out.push(FaceTrim {
            face: i,
            curve,
            range,
            start: p0,
            end: p1,
        });
    }
    Ok(out)
}

/// A ball-centre position on a blend, and where its contact with a face is.
fn foot(c: &CornerInput, b: usize, face: usize, s: f64) -> (Point3, Vec3) {
    let d = c.blends[b].spine.deriv(s);
    foot_velocity(&c.face_surfaces[face].0, d.p, d.d1)
}

/// Spine parameters (a, b) where the two curves a(s) and b(t) meet, by Gauss-Newton.
fn meet(
    a: impl Fn(f64) -> (Point3, Vec3),
    b: impl Fn(f64) -> (Point3, Vec3),
    mut s: f64,
    mut t: f64,
) -> Option<(f64, f64)> {
    for _ in 0..60 {
        let ((pa, va), (pb, vb)) = (a(s), b(t));
        let r = pa - pb;
        if r.length() <= 1e-3 * tol::LINEAR {
            return Some((s, t));
        }
        // r + va ds - vb dt = 0
        let (e, f, g) = (va.dot(va), -va.dot(vb), vb.dot(vb));
        let (ds, dt) = forge_math::linalg::solve2(e, f, f, g, -r.dot(va), r.dot(vb))?;
        s += ds;
        t += dt;
    }
    let r = a(s).0.distance(b(t).0);
    (r <= tol::LINEAR).then_some((s, t))
}

/// Edge `e` as a curve on face `face`: a blend's contact with it, or a sharp edge itself.
fn edge_curve<'a>(
    c: &'a CornerInput,
    e: &'a CornerEdge,
    face: usize,
) -> Box<dyn Fn(f64) -> (Point3, Vec3) + 'a> {
    match e {
        CornerEdge::Blend(b) => {
            let b = *b;
            Box::new(move |s| foot(c, b, face, s))
        }
        CornerEdge::Sharp { curve, .. } => Box::new(move |t| {
            let d = curve.deriv(t);
            (d.p, d.d1)
        }),
    }
}

fn plan(c: &CornerInput) -> Result<Plan, String> {
    let n = c.edges.len();
    if n < 3 || c.faces.len() != n || c.face_surfaces.len() != n {
        return Err("a corner needs three or more faces and edges, cyclic".into());
    }
    if c.blends.is_empty() {
        return Err("no blend at the corner".into());
    }
    // the parameter of each edge nearest the vertex, as a seed
    let near = |e: &CornerEdge| match e {
        CornerEdge::Blend(b) => {
            let b = &c.blends[*b];
            let span = 1e3 * b.radius.max(1.0);
            b.spine
                .closest(c.point, Interval::new(b.setback - span, b.setback + span))
        }
        CornerEdge::Sharp { curve, .. } => curve.closest(c.point, Interval::new(-1e6, 1e6)),
    };
    let seeds: Vec<f64> = c.edges.iter().map(near).collect();
    // q[j] = (param where edge j meets its neighbour on face j, on face j + 1)
    let mut q = seeds.iter().map(|&s| (s, s)).collect::<Vec<_>>();
    let mut corner = vec![c.point; n];
    for (i, at) in corner.iter_mut().enumerate() {
        let (l, r) = ((i + n - 1) % n, i);
        if matches!(
            (&c.edges[l], &c.edges[r]),
            (CornerEdge::Sharp { .. }, CornerEdge::Sharp { .. })
        ) {
            continue;
        }
        let (a, b) = meet(
            edge_curve(c, &c.edges[l], i),
            edge_curve(c, &c.edges[r], i),
            seeds[l],
            seeds[r],
        )
        .ok_or_else(|| format!("face {i}: the springs do not meet"))?;
        q[l].1 = a;
        q[r].0 = b;
        *at = edge_curve(c, &c.edges[l], i)(a).0;
    }
    let out_of = |e: &CornerEdge| match e {
        CornerEdge::Blend(b) => c.blends[*b].out,
        CornerEdge::Sharp { out, .. } => *out,
    };
    let point_of = |e: &CornerEdge, s: f64| match e {
        CornerEdge::Blend(b) => c.blends[*b].spine.deriv(s),
        CornerEdge::Sharp { curve, .. } => curve.deriv(s),
    };
    // one ball fits when every blend meets both neighbours at one ball position
    let one_ball = c.edges.iter().zip(&q).all(|(e, &(a, b))| {
        matches!(e, CornerEdge::Blend(_)) && point_of(e, a).p.distance(point_of(e, b).p) <= tol::LINEAR
    });
    let rmax = c.blends.iter().map(|b| b.radius).fold(0.0, f64::max);
    let mut blend = c.blends.iter().map(|b| b.setback).collect::<Vec<_>>();
    let mut edge = vec![0.0; n];
    for (j, e) in c.edges.iter().enumerate() {
        let o = out_of(e);
        let far = o * (o * q[j].0).max(o * q[j].1);
        let d = point_of(e, far);
        let reach = match e {
            CornerEdge::Blend(b) => {
                // as far again as the section is long, at most its radius there (the ball's
                // reach to the face, so a varying radius is read where the blend stops): a
                // shallow edge's short section would otherwise end a long narrow slot
                let (fa, fb) = (foot(c, *b, j, far).0 - d.p, foot(c, *b, (j + 1) % n, far).0 - d.p);
                let theta = dmath::atan2(fa.cross(fb).length(), fa.dot(fb));
                fa.length() * theta.min(1.0)
            }
            CornerEdge::Sharp { .. } => rmax,
        };
        let step = if one_ball { 0.0 } else { reach / d.d1.length() };
        let s = far + o * step;
        match e {
            CornerEdge::Blend(b) => blend[*b] = s,
            CornerEdge::Sharp { .. } => edge[j] = s,
        }
    }
    Ok(Plan { blend, edge, corner })
}

/// The surface closing the hole: the exact sphere where one ball fits, else the n-sided patch.
pub fn patch(_c: &CornerInput, hole: &[HoleSide]) -> Option<Result<CornerPatch, String>> {
    let sides: Vec<Side> = hole.iter().map(hole_side).collect();
    Some(match corner_surface(sides) {
        Ok((surface, same_sense)) => Ok(CornerPatch {
            inner: Vec::new(),
            faces: vec![PatchFace {
                surface,
                same_sense,
                uses: (0..hole.len()).map(PatchUse::Side).collect(),
            }],
        }),
        Err(e) => Err(format!("corner patch: {e:?}")),
    })
}

fn hole_side(h: &HoleSide) -> Side {
    let rail = Arc::new(SurfaceRail::new(
        h.curve.clone(),
        h.neighbour.clone(),
        h.neighbour_same_sense,
    ));
    let (t0, t1) = if h.forward {
        (h.range.lo, h.range.hi)
    } else {
        (h.range.hi, h.range.lo)
    };
    Side::new(rail, t0, t1)
}

/// Corners of planar faces with cylinder fillets, and the holes the fillet engine would hand
/// over for them: for tests and measurements before the fillet engine lands.
#[doc(hidden)]
pub mod synthetic {
    use super::*;
    use forge_geom::{Circle, Cylinder, Line, Plane};
    use forge_math::Frame;
    use forge_topo::{FaceId, VertexId};

    /// A corner of planar faces at `v`. Edge i runs from `v` along `dirs[i]`; face i lies between
    /// edges i - 1 and i with outward normal `normals[i]`; `fillets[i]` is (radius, convex).
    pub fn corner(
        v: Point3,
        dirs: &[Vec3],
        normals: &[Vec3],
        fillets: &[Option<(f64, bool)>],
    ) -> CornerInput {
        let n = dirs.len();
        let mut blends = Vec::new();
        let mut edges = Vec::new();
        for i in 0..n {
            let e = dirs[i].normalize().unwrap();
            match fillets[i] {
                Some((r, convex)) => {
                    let (na, nb) = (normals[i], normals[(i + 1) % n]);
                    let sigma = if convex { -1.0 } else { 1.0 };
                    let o = v + (na + nb) * (sigma * r / (1.0 + na.dot(nb)));
                    edges.push(CornerEdge::Blend(blends.len()));
                    blends.push(CornerBlend {
                        surface: Surface::Cylinder(Cylinder {
                            frame: Frame::from_z(o, e),
                            radius: r,
                        }),
                        same_sense: convex,
                        radius: r,
                        convex,
                        spine: Curve::Line(Line::new(o, e)),
                        out: 1.0,
                        setback: 0.0,
                    });
                }
                None => edges.push(CornerEdge::Sharp {
                    curve: Curve::Line(Line::new(v, e)),
                    out: 1.0,
                }),
            }
        }
        let face_surfaces = normals
            .iter()
            .map(|&nm| {
                (
                    Surface::Plane(Plane::new(Frame::from_z(v, nm.normalize().unwrap()))),
                    true,
                )
            })
            .collect();
        CornerInput {
            vertex: VertexId(0),
            point: v,
            blends,
            faces: (0..n).map(|i| FaceId(i as u32)).collect(),
            edges,
            face_surfaces,
        }
    }

    /// The hole the fillet engine would hand over: trims and sections in order.
    pub fn hole(c: &CornerInput, setbacks: &[f64], trims: &[FaceTrim]) -> Vec<HoleSide> {
        let n = c.edges.len();
        let mut out = Vec::new();
        for i in 0..n {
            let (face, same) = c.face_surfaces[i].clone();
            if let Some(t) = trims.iter().find(|t| t.face == i) {
                out.push(HoleSide {
                    curve: t.curve.clone(),
                    range: t.range,
                    forward: true,
                    start: t.start,
                    end: t.end,
                    neighbour: face,
                    neighbour_same_sense: same,
                    kind: SideKind::FaceCurve { face: i },
                });
            }
            if let CornerEdge::Blend(b) = c.edges[i] {
                let bl = &c.blends[b];
                let ctr = bl.spine.eval(setbacks[b]);
                let ka = foot(c, b, i, setbacks[b]).0;
                let kb = foot(c, b, (i + 1) % n, setbacks[b]).0;
                let (x, y) = (ka - ctr, kb - ctr);
                let z = x.cross(y).normalize().unwrap();
                let frame = Frame::from_zx(ctr, z, x).unwrap();
                let theta = dmath::atan2(x.cross(y).length(), x.dot(y));
                out.push(HoleSide {
                    curve: Curve::Circle(Circle {
                        frame,
                        radius: bl.radius,
                    }),
                    range: Interval::new(0.0, theta),
                    forward: true,
                    start: ka,
                    end: kb,
                    neighbour: bl.surface.clone(),
                    neighbour_same_sense: bl.same_sense,
                    kind: SideKind::Section { blend: b },
                });
            }
        }
        out
    }

    /// The hole of a corner with the setbacks and trims of this module.
    pub fn planned_hole(c: &CornerInput) -> Vec<HoleSide> {
        let s = setbacks(c).expect("setbacks");
        let trims = face_curves(c, &s).expect("trims");
        hole(c, &s, &trims)
    }

    /// The rail sides of a hole.
    pub fn sides(hole: &[HoleSide]) -> Vec<Side> {
        hole.iter().map(hole_side).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::synthetic::*;
    use super::*;
    use forge_geom::corner::{Audit, HolePatch, audit};
    use forge_math::rng::Rng;

    struct Built {
        patch: CornerPatch,
        sides: Vec<HoleSide>,
    }

    fn build(c: &CornerInput) -> Built {
        let s = setbacks(c).expect("setbacks");
        let trims = face_curves(c, &s).expect("trims");
        let sides = hole(c, &s, &trims);
        let patch = patch(c, &sides).unwrap().expect("patch");
        Built { patch, sides }
    }

    fn check_patch(sides: &[HoleSide], what: &str) {
        check_patch_seeded(sides, what, 7);
    }

    fn check_patch_seeded(sides: &[HoleSide], what: &str, seed: u64) -> Audit {
        let p = HolePatch::new(sides.iter().map(hole_side).collect()).unwrap();
        let a = audit(&p, seed);
        let size = sides.iter().map(|s| s.start.distance(s.end)).fold(0.0, f64::max);
        assert!(a.boundary <= 1e-9, "{what} {a:?}");
        assert!(a.normal <= 1e-9, "{what} {a:?}");
        assert!(a.derivs <= 1e-4, "{what} {a:?}");
        assert!(a.project <= 1e-9 * size.max(1.0), "{what} {a:?}");
        assert_eq!(a.bbox_misses, 0, "{what}");
        assert!(a.fold > 0.0, "{what} {a:?}");
        a
    }

    fn box_corner(r: [Option<(f64, bool)>; 3]) -> CornerInput {
        corner(
            Point3::new(1.0, 2.0, 3.0),
            &[-Vec3::X, -Vec3::Y, -Vec3::Z],
            &[Vec3::Y, Vec3::Z, Vec3::X],
            &r,
        )
    }

    /// Outward normals of the faces of a convex corner with these edges.
    fn convex_normals(dirs: &[Vec3]) -> Vec<Vec3> {
        let n = dirs.len();
        (0..n)
            .map(|i| {
                let m = dirs[(i + n - 1) % n].cross(dirs[i]).normalize().unwrap().get();
                if m.dot(dirs[(i + 1) % n]) > 0.0 { -m } else { m }
            })
            .collect()
    }

    #[test]
    fn equal_fillets_meet_on_the_ball() {
        let c = box_corner([Some((2.0, true)); 3]);
        let b = build(&c);
        assert_eq!(b.sides.len(), 3);
        assert!(matches!(b.patch.faces[0].surface, Surface::Sphere(_)) && b.patch.faces[0].same_sense);
        // a skewed convex corner: still one ball
        let dirs = [
            Vec3::new(-1.0, 0.2, 0.1),
            Vec3::new(0.3, -1.0, 0.0),
            Vec3::new(0.1, 0.1, -1.0),
        ];
        let normals = convex_normals(&dirs);
        let c = corner(Point3::ORIGIN, &dirs, &normals, &[Some((1.5, true)); 3]);
        let b = build(&c);
        assert!(matches!(b.patch.faces[0].surface, Surface::Sphere(_)));
    }

    #[test]
    fn unequal_fillets_get_a_six_sided_patch() {
        let c = box_corner([Some((1.0, true)), Some((2.0, true)), Some((3.0, true))]);
        let b = build(&c);
        assert_eq!(b.sides.len(), 6);
        assert!(matches!(b.patch.faces[0].surface, Surface::Procedural(_)));
        check_patch(&b.sides, "box 1 2 3");
    }

    #[test]
    fn random_convex_corners() {
        let mut rng = Rng::new(77);
        for case in 0..40 {
            let dirs: Vec<Vec3> = [-Vec3::X, -Vec3::Y, -Vec3::Z]
                .iter()
                .map(|&d| d + Vec3::new(rng.range(-0.3, 0.3), rng.range(-0.3, 0.3), rng.range(-0.3, 0.3)))
                .collect();
            let normals = convex_normals(&dirs);
            let r: Vec<Option<(f64, bool)>> = (0..3).map(|_| Some((rng.range(0.5, 4.0), true))).collect();
            let c = corner(Point3::new(5.0, -3.0, 2.0), &dirs, &normals, &r);
            let b = build(&c);
            check_patch_seeded(&b.sides, &format!("case {case}"), 1000 + case);
        }
    }

    #[test]
    fn mixed_convexity() {
        // a step: material is z < 0 where x < 0 or y < 0; the edge down -z is concave
        let dirs = [Vec3::X, -Vec3::Z, Vec3::Y];
        let normals = [Vec3::Z, Vec3::Y, Vec3::X];
        for (a, b, k) in [(1.0, 1.0, 1.0), (1.0, 2.0, 0.5), (3.0, 1.0, 2.0)] {
            let c = corner(
                Point3::ORIGIN,
                &dirs,
                &normals,
                &[Some((a, true)), Some((k, false)), Some((b, true))],
            );
            let built = build(&c);
            check_patch(&built.sides, &format!("step {a} {k} {b}"));
        }
    }

    #[test]
    fn fillets_meeting_sharp_edges() {
        for r in [
            [Some((1.0, true)), None, None],
            [Some((1.0, true)), Some((2.0, true)), None],
        ] {
            let c = box_corner(r);
            let b = build(&c);
            check_patch(&b.sides, &format!("{r:?}"));
        }
    }

    /// A vertex with edges at the given azimuths and heights, material below its faces;
    /// whether each edge is convex follows from the geometry.
    fn cone(az: &[f64], h: &[f64], radii: &[f64]) -> CornerInput {
        let n = az.len();
        let dirs: Vec<Vec3> = (0..n)
            .map(|i| {
                let (s, c) = dmath::sin_cos(az[i]);
                Vec3::new(c, s, h[i])
            })
            .collect();
        let normals: Vec<Vec3> = (0..n)
            .map(|i| {
                let m = dirs[(i + n - 1) % n].cross(dirs[i]).normalize().unwrap().get();
                if m.z < 0.0 { -m } else { m }
            })
            .collect();
        let fillets: Vec<Option<(f64, bool)>> = (0..n)
            .map(|i| {
                let (d, e) = (dirs[i].normalize().unwrap().get(), dirs[(i + n - 1) % n]);
                // into face i from edge i, against face i + 1's normal for a convex edge
                let w = e - d * d.dot(e);
                Some((radii[i], normals[(i + 1) % n].dot(w) < 0.0))
            })
            .collect();
        corner(Point3::new(0.5, -1.0, 2.0), &dirs, &normals, &fillets)
    }

    /// The same vertex with material above: every edge flips convexity.
    fn flipped(az: &[f64], h: &[f64], radii: &[f64]) -> CornerInput {
        let rev = |v: &[f64]| v.iter().rev().copied().collect::<Vec<_>>();
        let az: Vec<f64> = rev(az).iter().map(|&a| -a).collect();
        let h: Vec<f64> = rev(h).iter().map(|&x| -x).collect();
        // turned half a turn about x, the edges reversed to stay counter-clockwise
        cone(&az, &h, &rev(radii))
    }

    fn kinds(c: &CornerInput) -> Vec<bool> {
        c.blends.iter().map(|b| b.convex).collect()
    }

    #[test]
    fn four_and_five_sided_corners() {
        use forge_math::dmath::TAU;
        let mut rng = Rng::new(78);
        let mut seen = std::collections::BTreeSet::new();
        let mut worst = Audit {
            fold: 1.0,
            ..Audit::default()
        };
        let (mut checked, mut thin) = (0, 0);
        for case in 0..120u64 {
            let n = 3 + (case % 3) as usize;
            let az: Vec<f64> = (0..n)
                .map(|i| (i as f64 + rng.range(-0.15, 0.15)) * TAU / n as f64)
                .collect();
            // pyramids, and from four edges saddles of alternating heights
            let saddle = n >= 4 && case % 2 == 1;
            let h: Vec<f64> = (0..n)
                .map(|i| {
                    if saddle && i % 2 == 1 {
                        rng.range(0.2, 0.8)
                    } else {
                        -rng.range(0.4, 1.5)
                    }
                })
                .collect();
            let radii: Vec<f64> = (0..n).map(|_| rng.range(0.5, 2.0)).collect();
            for c in [cone(&az, &h, &radii), flipped(&az, &h, &radii)] {
                let k = kinds(&c);
                let what = format!("case {case} n {n} convex {k:?}");
                let b = build(&c);
                assert_eq!(b.sides.len(), 2 * n, "{what}");
                // a side under 1/20 of the longest (a nearly flat edge's section) may fold
                let len: Vec<f64> = b.sides.iter().map(|s| s.start.distance(s.end)).collect();
                let (lo, hi) = len
                    .iter()
                    .fold((f64::INFINITY, 0.0f64), |(a, b), &l| (a.min(l), b.max(l)));
                if lo < 0.05 * hi {
                    thin += 1;
                    continue;
                }
                let a = check_patch_seeded(&b.sides, &what, 2000 + case);
                worst.boundary = worst.boundary.max(a.boundary);
                worst.normal = worst.normal.max(a.normal);
                worst.fold = worst.fold.min(a.fold);
                checked += 1;
                seen.insert((n, k.iter().filter(|&&x| x).count()));
            }
        }
        eprintln!("checked {checked} thin {thin} worst {worst:?}");
        assert!(thin * 10 < checked, "{thin} thin of {checked}");
        // convex, concave and mixed corners of every size
        for n in 3..=5 {
            assert!(seen.contains(&(n, n)) && seen.contains(&(n, 0)), "{seen:?}");
        }
        assert!(seen.iter().any(|&(n, c)| n == 4 && c > 0 && c < 4), "{seen:?}");
        assert!(seen.iter().any(|&(n, c)| n == 5 && c > 0 && c < 5), "{seen:?}");
    }

    #[test]
    fn equal_fillets_on_a_regular_pyramid_meet_on_the_ball() {
        use forge_math::dmath::TAU;
        for n in 3..=5 {
            let az: Vec<f64> = (0..n).map(|i| i as f64 * TAU / n as f64).collect();
            for c in [
                cone(&az, &vec![-1.0; n], &vec![1.5; n]),
                flipped(&az, &vec![-1.0; n], &vec![1.5; n]),
            ] {
                let b = build(&c);
                assert_eq!(b.sides.len(), n, "n {n}");
                let f = &b.patch.faces[0];
                assert!(matches!(f.surface, Surface::Sphere(_)), "n {n}");
                assert_eq!(f.same_sense, c.blends[0].convex);
            }
        }
    }
}
