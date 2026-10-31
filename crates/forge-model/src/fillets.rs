//! Fillet cases, from easy to brutal. Each must regenerate valid, mesh
//! closed, match its exact volume where there is a closed form, and be
//! tangent continuous wherever a blend meets its neighbours.

use crate::{Combine, Feature, Part};
use forge_geom::CurveGeom;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3, dmath};
use forge_ops::Profile;
use forge_topo::{Body, Name};

/// A w x h x d box with a corner at `o`.
pub fn block_at(o: Point3, w: f64, h: f64, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(o),
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)],
        },
        distance: d,
        op,
    }
}

/// An upright cylinder of radius r and height h standing on `o`.
pub fn post_at(o: Point3, r: f64, h: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(o),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        distance: h,
        op,
    }
}

/// A rod of radius r and length `len` along `axis` from `o`.
pub fn rod_at(o: Point3, axis: UnitVec3, r: f64, len: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::from_z(o, axis),
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        distance: len,
        op,
    }
}

/// The name of the edge of `body` whose middle is nearest `p`.
pub fn edge_near(body: &Body, p: Point3) -> Name {
    let mid = |e| {
        let e = body.edge(e);
        e.curve.eval(e.range.mid()).distance(p)
    };
    body.edge_ids()
        .min_by(|&a, &b| mid(a).total_cmp(&mid(b)))
        .map(|e| body.edge(e).name)
        .expect("a body has edges")
}

/// `part` followed by a fillet of the edges nearest each point, each with its radius.
pub fn with_fillet(part: &Part, near: &[(Point3, f64)]) -> Part {
    with_fillet_from(part, &part.regenerate().expect("base part regenerates"), near)
}

/// `with_fillet`, the edges picked from `body` (the part as some regeneration made it).
pub fn with_fillet_from(part: &Part, body: &Body, near: &[(Point3, f64)]) -> Part {
    let edges = near.iter().map(|&(p, r)| (edge_near(body, p), r)).collect();
    let mut out = part.clone();
    out.features.push(Feature::Fillet { edges });
    out
}

/// The points and radii a fillet case picked its edges with (from the names it holds).
pub fn picks(part: &Part) -> Vec<(Point3, f64)> {
    let mut base = part.clone();
    let Some(Feature::Fillet { edges }) = base.features.pop() else {
        return Vec::new();
    };
    let body = base.regenerate().expect("base part regenerates");
    edges
        .iter()
        .map(|&(n, r)| {
            let e = body.edge(body.edge_by_name(n).expect("named edge"));
            (e.curve.eval(e.range.mid()), r)
        })
        .collect()
}

/// Every case: a name, the part, and its exact volume where there is a closed form.
pub fn cases() -> Vec<(&'static str, Part, Option<f64>)> {
    let pi = dmath::PI;
    // the corner a fillet of radius r removes or adds per unit length
    let spandrel = |r: f64| (1.0 - pi / 4.0) * r * r;
    // its centroid's distance from the sharp corner, along either face
    let centroid = |r: f64| r * (10.0 - 3.0 * pi) / (12.0 - 3.0 * pi);
    let o = Point3::ORIGIN;
    let boxed = Part {
        features: vec![block_at(o, 20.0, 10.0, 8.0, Combine::New)],
    };
    let vbox = 1600.0;
    let mut out = Vec::new();

    out.push((
        "box-one-edge",
        with_fillet(&boxed, &[(Point3::new(10.0, 0.0, 8.0), 2.0)]),
        Some(vbox - spandrel(2.0) * 20.0),
    ));
    out.push((
        "box-vertical-edge",
        with_fillet(&boxed, &[(Point3::new(20.0, 10.0, 4.0), 3.0)]),
        Some(vbox - spandrel(3.0) * 8.0),
    ));
    out.push((
        "box-two-opposite-radii",
        with_fillet(
            &boxed,
            &[
                (Point3::new(10.0, 0.0, 8.0), 1.5),
                (Point3::new(10.0, 10.0, 0.0), 2.5),
            ],
        ),
        Some(vbox - (spandrel(1.5) + spandrel(2.5)) * 20.0),
    ));
    // three edges at a corner, one radius: a sphere octant closes it
    let r = 2.0;
    out.push((
        "box-ball-corner",
        with_fillet(
            &boxed,
            &[
                (Point3::new(10.0, 0.0, 8.0), r),
                (Point3::new(0.0, 5.0, 8.0), r),
                (Point3::new(0.0, 0.0, 4.0), r),
            ],
        ),
        Some(vbox - spandrel(r) * ((20.0 - r) + (10.0 - r) + (8.0 - r)) - (r * r * r - pi * r * r * r / 6.0)),
    ));
    // all twelve edges of a box, one radius: eight sphere corners
    let r = 1.5;
    let all: Vec<(Point3, f64)> = [
        (10.0, 0.0, 0.0),
        (10.0, 10.0, 0.0),
        (10.0, 0.0, 8.0),
        (10.0, 10.0, 8.0),
        (0.0, 5.0, 0.0),
        (20.0, 5.0, 0.0),
        (0.0, 5.0, 8.0),
        (20.0, 5.0, 8.0),
        (0.0, 0.0, 4.0),
        (20.0, 0.0, 4.0),
        (0.0, 10.0, 4.0),
        (20.0, 10.0, 4.0),
    ]
    .iter()
    .map(|&(x, y, z)| (Point3::new(x, y, z), r))
    .collect();
    let edges = 4.0 * ((20.0 - 2.0 * r) + (10.0 - 2.0 * r) + (8.0 - 2.0 * r));
    out.push((
        "box-all-edges",
        with_fillet(&boxed, &all),
        Some(vbox - spandrel(r) * edges - 8.0 * (r * r * r - pi * r * r * r / 6.0)),
    ));
    // a concave edge: a block standing on a plate, flush with its sides
    let mut step = boxed.clone();
    step.features.push(block_at(
        Point3::new(0.0, 0.0, 8.0),
        10.0,
        10.0,
        6.0,
        Combine::Add,
    ));
    out.push((
        "step-concave",
        with_fillet(&step, &[(Point3::new(10.0, 5.0, 8.0), 2.0)]),
        Some(vbox + 600.0 + spandrel(2.0) * 10.0),
    ));
    // one foot edge of a block on a plate: the block's ends, carried on, cap the blend
    let mut footed = boxed.clone();
    footed
        .features
        .push(block_at(Point3::new(5.0, 3.0, 8.0), 8.0, 4.0, 5.0, Combine::Add));
    out.push((
        "block-one-foot-edge",
        with_fillet(&footed, &[(Point3::new(9.0, 3.0, 8.0), 1.0)]),
        Some(vbox + 160.0 + spandrel(1.0) * 8.0),
    ));
    // a boss on a plate: a concave circle, rounded by a torus
    let mut boss = boxed.clone();
    boss.features
        .push(post_at(Point3::new(10.0, 5.0, 8.0), 3.0, 5.0, Combine::Add));
    let vboss = vbox + pi * 9.0 * 5.0;
    let r = 1.0;
    out.push((
        "boss-concave-torus",
        with_fillet(&boss, &[(Point3::new(7.0, 5.0, 8.0), r)]),
        Some(vboss + 2.0 * pi * (3.0 + centroid(r)) * spandrel(r)),
    ));
    out.push((
        "boss-convex-rim",
        with_fillet(&boss, &[(Point3::new(7.0, 5.0, 13.0), r)]),
        Some(vboss - 2.0 * pi * (3.0 - centroid(r)) * spandrel(r)),
    ));
    out.push((
        "boss-both-circles",
        with_fillet(
            &boss,
            &[
                (Point3::new(7.0, 5.0, 13.0), 0.8),
                (Point3::new(7.0, 5.0, 8.0), 1.2),
            ],
        ),
        Some(
            vboss - 2.0 * pi * (3.0 - centroid(0.8)) * spandrel(0.8)
                + 2.0 * pi * (3.0 + centroid(1.2)) * spandrel(1.2),
        ),
    ));
    // a hole through a plate: its rims are convex circles on a concave cylinder
    let mut holed = boxed.clone();
    holed
        .features
        .push(post_at(Point3::new(10.0, 5.0, -1.0), 3.0, 10.0, Combine::Cut));
    let vhole = vbox - pi * 9.0 * 8.0;
    out.push((
        "hole-rim",
        with_fillet(&holed, &[(Point3::new(7.0, 5.0, 8.0), 1.0)]),
        Some(vhole - 2.0 * pi * (3.0 + centroid(1.0)) * spandrel(1.0)),
    ));
    // a rounded block: one pick takes the whole tangent chain round its top
    let (w, h, d, rho) = (20.0, 12.0, 6.0, 3.0);
    let rounded = Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::rounded_rectangle(Point2::ORIGIN, w, h, rho, 0)],
            },
            distance: d,
            op: Combine::New,
        }],
    };
    let vround = (w * h - (4.0 - pi) * rho * rho) * d;
    let straight = 2.0 * (w - 2.0 * rho) + 2.0 * (h - 2.0 * rho);
    let r = 1.0;
    out.push((
        "rounded-top-chain",
        with_fillet(&rounded, &[(Point3::new(10.0, 0.0, d), r)]),
        Some(vround - spandrel(r) * straight - 2.0 * pi * (rho - centroid(r)) * spandrel(r)),
    ));
    // the same block standing on a plate: a concave chain round its foot
    let mut stand = Part {
        features: vec![block_at(
            Point3::new(-5.0, -5.0, -4.0),
            30.0,
            22.0,
            4.0,
            Combine::New,
        )],
    };
    stand.features.push(rounded.features[0].clone());
    if let Feature::Extrude { op, .. } = &mut stand.features[1] {
        *op = Combine::Add;
    }
    out.push((
        "rounded-foot-chain",
        with_fillet(&stand, &[(Point3::new(10.0, 0.0, 0.0), r)]),
        Some(
            30.0 * 22.0 * 4.0
                + vround
                + spandrel(r) * straight
                + 2.0 * pi * (rho + centroid(r)) * spandrel(r),
        ),
    ));
    // a rod off the axis of a ball: the edge is a traced quartic, the blend a procedural pipe
    let ball_rod = Part {
        features: vec![
            Feature::Ball {
                frame: Frame::WORLD,
                radius: 8.0,
                op: Combine::New,
            },
            Feature::Extrude {
                profile: Profile {
                    plane: Frame::from_z(Point3::new(0.0, 2.0, 0.0), forge_math::UnitVec3::X),
                    loops: vec![Profile::circle(Point2::ORIGIN, 3.0, 0)],
                },
                distance: 14.0,
                op: Combine::Add,
            },
        ],
    };
    let base = ball_rod.regenerate().expect("ball and rod");
    let seam = base
        .edge_ids()
        .find(|&e| matches!(base.edge(e).curve, forge_geom::Curve::Intersection(_)))
        .map(|e| {
            let x = base.edge(e);
            x.curve.eval(x.range.mid())
        })
        .expect("a traced edge");
    out.push(("ball-rod-pipe", with_fillet(&ball_rod, &[(seam, 1.0)]), None));
    // a pocket: four concave walls and a floor, eight edges, four concave ball corners
    let r = 1.0;
    let mut pocket = boxed.clone();
    pocket
        .features
        .push(block_at(Point3::new(5.0, 2.0, 4.0), 10.0, 6.0, 5.0, Combine::Cut));
    let walls = [(5.0, 2.0), (15.0, 2.0), (5.0, 8.0), (15.0, 8.0)].map(|(x, y)| (Point3::new(x, y, 6.0), r));
    let floor = [(10.0, 2.0), (10.0, 8.0), (5.0, 5.0), (15.0, 5.0)].map(|(x, y)| (Point3::new(x, y, 4.0), r));
    out.push((
        "pocket-ball-corners",
        with_fillet(&pocket, &[walls, floor].concat()),
        Some(
            vbox - 240.0
                + spandrel(r) * (4.0 * (4.0 - r) + 2.0 * (10.0 - 2.0 * r) + 2.0 * (6.0 - 2.0 * r))
                + 4.0 * (r * r * r - pi * r * r * r / 6.0),
        ),
    ));
    // a slot through a block: two concave floor edges running out at both ends
    let mut slot = boxed.clone();
    slot.features.push(block_at(
        Point3::new(-1.0, 3.0, 4.0),
        22.0,
        4.0,
        5.0,
        Combine::Cut,
    ));
    out.push((
        "slot-floor",
        with_fillet(
            &slot,
            &[
                (Point3::new(10.0, 3.0, 4.0), 1.0),
                (Point3::new(10.0, 7.0, 4.0), 1.0),
            ],
        ),
        Some(vbox - 320.0 + 2.0 * spandrel(1.0) * 20.0),
    ));
    // a D shaft: a flat on a cylinder, its two plane-cylinder lines rounded
    let (big, x0, h, r) = (5.0, 3.0, 10.0, 1.0);
    let mut dshaft = Part {
        features: vec![post_at(o, big, h, Combine::New)],
    };
    dshaft.features.push(block_at(
        Point3::new(x0, -6.0, -1.0),
        5.0,
        12.0,
        12.0,
        Combine::Cut,
    ));
    let yc = f64::sqrt((big - r) * (big - r) - (x0 - r) * (x0 - r));
    let yp = f64::sqrt(big * big - x0 * x0);
    // per line: between the line, the circle and the blend (see the tangent points T1 on the
    // flat, T2 on the circle, P the old edge, C the ball's centre)
    let a_p = dmath::atan2(yp, x0);
    let a_c = dmath::atan2(yc, x0 - r);
    let turn = pi / 2.0 - a_c;
    let corner = 0.5 * (x0 * yp - yc * x0) + 0.5 * big * big * (a_c - a_p)
        - 0.5 * r * yc
        - 0.5 * r * r * (pi / 2.0 - turn);
    let disc = pi * big * big - (big * big * dmath::acos(x0 / big) - x0 * yp);
    out.push((
        "d-shaft-flat",
        with_fillet(
            &dshaft,
            &[(Point3::new(x0, yp, 5.0), r), (Point3::new(x0, -yp, 5.0), r)],
        ),
        Some(h * (disc - 2.0 * corner)),
    ));
    // a shaft drilled across: the hole's rims are convex cylinder-cylinder edges, traced
    let mut drilled = Part {
        features: vec![rod_at(
            Point3::new(-15.0, 0.0, 0.0),
            UnitVec3::X,
            6.0,
            30.0,
            Combine::New,
        )],
    };
    // off centre, the hole's seam turned to meet the shaft's: four edges at that rim vertex
    let seam_dir = Vec3::new(f64::sqrt(3.75), -0.5, 0.0);
    drilled.features.push(Feature::Extrude {
        profile: Profile {
            plane: Frame::from_zx(Point3::new(0.0, 0.5, -8.0), UnitVec3::Z, seam_dir).expect("frame"),
            loops: vec![Profile::circle(Point2::ORIGIN, 2.0, 0)],
        },
        distance: 16.0,
        op: Combine::Cut,
    });
    out.push((
        "shaft-cross-hole",
        with_fillet(
            &drilled,
            &[
                (Point3::new(0.0, 2.5, f64::sqrt(29.75)), 0.5),
                (Point3::new(0.0, 2.5, -f64::sqrt(29.75)), 0.5),
            ],
        ),
        None,
    ));
    // a post standing on a shaft: a concave cylinder-cylinder edge
    let mut posted = Part {
        features: vec![rod_at(
            Point3::new(-15.0, 0.0, 0.0),
            UnitVec3::X,
            6.0,
            30.0,
            Combine::New,
        )],
    };
    posted
        .features
        .push(post_at(Point3::new(0.0, 0.0, 0.0), 2.0, 10.0, Combine::Add));
    out.push((
        "shaft-post",
        with_fillet(&posted, &[(Point3::new(0.0, 2.0, f64::sqrt(32.0)), 1.0)]),
        None,
    ));
    // a hole drilled at a slant through a plate: elliptical plane-cylinder rims
    let slant = UnitVec3::new_unchecked(Vec3::new(0.5, 0.0, f64::sqrt(0.75)));
    let mut slanted = boxed.clone();
    slanted.features.push(rod_at(
        Point3::new(10.0, 5.0, 4.0) - slant.get() * 10.0,
        slant,
        2.0,
        20.0,
        Combine::Cut,
    ));
    out.push((
        "slant-hole-rims",
        with_fillet(
            &slanted,
            &[
                (Point3::new(10.0 + 4.0 / f64::sqrt(3.0), 7.0, 8.0), 0.5),
                (Point3::new(10.0 - 4.0 / f64::sqrt(3.0), 7.0, 0.0), 0.5),
            ],
        ),
        None,
    ));
    out
}

/// Fillets that cannot be built, and the error each must give (by its name).
pub fn refusals() -> Vec<(&'static str, Part, &'static str)> {
    let boxed = Part {
        features: vec![block_at(Point3::ORIGIN, 20.0, 10.0, 8.0, Combine::New)],
    };
    let rib = Part {
        features: vec![block_at(Point3::ORIGIN, 20.0, 4.0, 8.0, Combine::New)],
    };
    let rounded = Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::rounded_rectangle(Point2::ORIGIN, 20.0, 12.0, 3.0, 0)],
            },
            distance: 6.0,
            op: Combine::New,
        }],
    };
    vec![
        // the ball runs off the bottom of the front face
        (
            "radius-past-face",
            with_fillet(&boxed, &[(Point3::new(10.0, 0.0, 8.0), 9.0)]),
            "Overflow",
        ),
        // two blends meet exactly along the rib's top: the top face shrinks to a line
        (
            "rib-full-round",
            with_fillet(
                &rib,
                &[
                    (Point3::new(10.0, 0.0, 8.0), 2.0),
                    (Point3::new(10.0, 4.0, 8.0), 2.0),
                ],
            ),
            "Tangency",
        ),
        // the seam between a flat side and a rounded corner is already smooth
        (
            "smooth-seam",
            with_fillet(&rounded, &[(Point3::new(17.0, 0.0, 3.0), 1.0)]),
            "Smooth",
        ),
        // two blends and a sharp edge at a corner need a corner patch
        (
            "two-blends-sharp-edge",
            with_fillet(
                &boxed,
                &[
                    (Point3::new(10.0, 0.0, 8.0), 2.0),
                    (Point3::new(20.0, 5.0, 8.0), 2.0),
                ],
            ),
            "Corner",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::SurfaceGeom;
    use forge_mesh::{Quality, tessellate};

    /// The worst normal mismatch over edges that are meant to be smooth (those
    /// not visibly sharp): zero up to rounding when every blend is G1.
    fn smooth_mismatch(body: &Body) -> f64 {
        let mut worst = 0.0f64;
        for e in body.edge_ids() {
            let edge = body.edge(e);
            let faces: Vec<_> = edge.coedges.iter().map(|&c| body.coedge_face(c)).collect();
            if faces.len() != 2 || faces[0] == faces[1] {
                continue;
            }
            let mut most = 0.0f64;
            for i in 0..=8 {
                let t = edge.range.lo + edge.range.width() * i as f64 / 8.0;
                let p = edge.curve.eval(t);
                let n: Vec<_> = faces
                    .iter()
                    .filter_map(|&f| body.face_normal(f, body.face(f).surface.project(p)))
                    .collect();
                if n.len() < 2 {
                    continue;
                }
                most = most.max(n[0].cross(n[1]).length());
            }
            if most < 1e-2 {
                worst = worst.max(most);
            }
        }
        worst
    }

    /// One to three random edges of random parts, random radii: every result is a
    /// valid solid with a closed mesh, or an error, never a panic.
    #[test]
    fn random_fillets() {
        use forge_math::rng::Rng;
        use forge_ops::{FilletEdge, fillet};
        let mut ok = 0;
        for seed in 1..=150u64 {
            let Ok(body) = crate::suite::fuzz(seed, 4).regenerate() else {
                continue;
            };
            let mut r = Rng::new(seed * 7919);
            let edges: Vec<_> = body.edge_ids().collect();
            let mut pick: Vec<FilletEdge> = Vec::new();
            for _ in 0..1 + r.below(3) {
                let (edge, radius) = (edges[r.below(edges.len())], r.range(0.05, 1.0));
                if pick.iter().all(|p| p.edge != edge) {
                    pick.push(FilletEdge { edge, radius });
                }
            }
            let Ok(b) = fillet(&body, &pick, 999) else {
                continue;
            };
            let problems = forge_topo::check(&b);
            assert!(problems.is_empty(), "seed {seed}: {problems:?}");
            let mesh = tessellate(&b, &Quality::default()).unwrap_or_else(|e| panic!("seed {seed}: {e:?}"));
            assert!(mesh.open_edges().is_empty(), "seed {seed}: open mesh");
            ok += 1;
        }
        // seam picks are refused as smooth and adjacent picks as corners; the rest build
        assert!(ok >= 70, "only {ok} of 150 built");
    }

    #[test]
    fn refusals_are_clean() {
        for (name, part, want) in refusals() {
            match part.regenerate() {
                Ok(b) => panic!(
                    "{name}: built ({} problems), want {want}",
                    forge_topo::check(&b).len()
                ),
                Err(e) => {
                    let got = format!("{e:?}");
                    assert!(got.contains(want), "{name}: {got}, want {want}");
                }
            }
        }
    }

    #[test]
    fn corpus() {
        let q = Quality {
            chord: 1e-4,
            max_angle: 1.0,
        };
        for (name, part, volume) in cases() {
            // fine meshes only where a volume is checked
            let q = if volume.is_some() {
                q
            } else {
                Quality {
                    chord: 1e-2,
                    max_angle: 0.3,
                }
            };
            let body = part.regenerate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let problems = forge_topo::check(&body);
            assert!(problems.is_empty(), "{name}: {problems:?}");
            let mesh = tessellate(&body, &q).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(mesh.open_edges().is_empty(), "{name}: open mesh");
            if let Some(v) = volume {
                // inscribed chords lose volume in proportion to the chord: extrapolate to zero
                let coarse = tessellate(
                    &body,
                    &Quality {
                        chord: 4.0 * q.chord,
                        ..q
                    },
                )
                .expect("meshes")
                .volume();
                let fine = mesh.volume();
                let got = fine + (fine - coarse) / 3.0;
                assert!(
                    (got - v).abs() < 1e-6 * v,
                    "{name}: volume {got} want {v} (mesh {fine})"
                );
            }
            // traced spines hold a pipe's normals to a few 1e-9
            let g1 = smooth_mismatch(&body);
            assert!(g1 < 1e-8, "{name}: blend not tangent ({g1})");
            // n-ary and sequential booleans may name split edges differently: pick again
            let mut base = part.clone();
            base.features.pop();
            let nbase = base.regenerate_nary(2).expect("base regenerates n-ary");
            let npart = with_fillet_from(&base, &nbase, &picks(&part));
            let nary = npart
                .regenerate_nary(2)
                .unwrap_or_else(|e| panic!("{name} nary: {e:?}"));
            assert!(forge_topo::check(&nary).is_empty(), "{name}: nary result invalid");
            let m2 = tessellate(&nary, &q).expect("nary meshes");
            assert!(
                (m2.volume() - mesh.volume()).abs() < 1e-9 * mesh.volume(),
                "{name}: nary volume"
            );
        }
    }
}
