//! Fillet cases, from easy to brutal. Each must regenerate valid, mesh
//! closed, match its exact volume where there is a closed form, and be
//! tangent continuous wherever a blend meets its neighbours.

use crate::{Combine, Extent, Feature, Part};
use forge_geom::CurveGeom;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3, dmath};
use forge_ops::{Profile, ProfileLoop, Radius, Segment};
use forge_topo::{Body, Name};

/// A w x h x d box with a corner at `o`.
pub fn block_at(o: Point3, w: f64, h: f64, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(o),
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)],
        },
        extent: Extent::Distance(d),
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
        extent: Extent::Distance(h),
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
        extent: Extent::Distance(len),
        op,
    }
}

/// A 20 x 20 x 20 post cut to an apex at (10, 10, 16) by one plane per (azimuth, slope): the
/// plane's outward normal leans from z toward that azimuth. Also the middle of each edge
/// down from the apex, between plane i and plane i + 1.
pub fn apex_post(planes: &[(f64, f64)]) -> (Part, Vec<Point3>) {
    let apex = Point3::new(10.0, 10.0, 16.0);
    let normals: Vec<Vec3> = planes
        .iter()
        .map(|&(az, slope)| {
            let (s, c) = dmath::sin_cos(az);
            Vec3::new(c * slope, s * slope, 1.0)
        })
        .collect();
    let mut part = Part {
        features: vec![block_at(Point3::ORIGIN, 20.0, 20.0, 20.0, Combine::New)],
    };
    for n in &normals {
        part.features.push(Feature::Extrude {
            profile: Profile {
                plane: Frame::from_z(apex, n.normalize().expect("normal")),
                loops: vec![Profile::rectangle(Point2::new(-60.0, -60.0), 120.0, 120.0, 0)],
            },
            extent: Extent::Distance(60.0),
            op: Combine::Cut,
        });
    }
    let k = normals.len();
    let mids = (0..k)
        .map(|i| {
            let mut d = normals[i].cross(normals[(i + 1) % k]);
            if d.z > 0.0 {
                d = -d;
            }
            // to where the edge leaves the post's sides
            let t = [d.x, d.y]
                .iter()
                .zip([apex.x, apex.y])
                .map(|(&v, a)| if v > 0.0 { (20.0 - a) / v } else { -a / v })
                .fold(f64::INFINITY, f64::min);
            apex + d * (0.5 * t)
        })
        .collect();
    (part, mids)
}

/// A pit: a 20 x 20 column whose floor is the highest of planes through (10, 10, 4), one per
/// (azimuth, slope), each rising toward its azimuth, so they meet in a concave point at the
/// bottom. Also the middle of each edge up from it, between plane i and plane i + 1.
pub fn pit(planes: &[(f64, f64)]) -> (Part, Vec<Point3>) {
    let low = Point3::new(10.0, 10.0, 4.0);
    let normals: Vec<Vec3> = planes
        .iter()
        .map(|&(az, slope)| {
            let (s, c) = dmath::sin_cos(az);
            Vec3::new(-c * slope, -s * slope, 1.0)
        })
        .collect();
    // the union of the slabs under each plane, cut to the column
    let mut features: Vec<Feature> = normals
        .iter()
        .enumerate()
        .map(|(i, n)| Feature::Extrude {
            profile: Profile {
                plane: Frame::from_z(low, (-*n).normalize().expect("normal")),
                loops: vec![Profile::rectangle(Point2::new(-60.0, -60.0), 120.0, 120.0, 0)],
            },
            extent: Extent::Distance(40.0),
            op: if i == 0 { Combine::New } else { Combine::Add },
        })
        .collect();
    features.push(block_at(
        Point3::new(0.0, 0.0, -10.0),
        20.0,
        20.0,
        26.0,
        Combine::Keep,
    ));
    let k = normals.len();
    let mids = (0..k)
        .map(|i| {
            let mut d = normals[i].cross(normals[(i + 1) % k]);
            if d.z < 0.0 {
                d = -d;
            }
            // to where the edge leaves the column's sides or top
            let t = [(d.x, low.x, 20.0), (d.y, low.y, 20.0), (d.z, low.z, 16.0)]
                .iter()
                .map(|&(v, a, hi)| {
                    if v > 0.0 {
                        (hi - a) / v
                    } else if v < 0.0 {
                        -a / v
                    } else {
                        f64::INFINITY
                    }
                })
                .fold(f64::INFINITY, f64::min);
            low + d * (0.5 * t)
        })
        .collect();
    (Part { features }, mids)
}

/// A 20 x 10 block with a gable roof: its ridge runs along x at y = 5, z = 8, the roof falling
/// 1 in 1 to either side.
pub fn roofed() -> Part {
    let ridge = Point3::new(10.0, 5.0, 8.0);
    let mut part = Part {
        features: vec![block_at(Point3::ORIGIN, 20.0, 10.0, 12.0, Combine::New)],
    };
    for n in [Vec3::new(0.0, -1.0, 1.0), Vec3::new(0.0, 1.0, 1.0)] {
        part.features.push(Feature::Extrude {
            profile: Profile {
                plane: Frame::from_z(ridge, n.normalize().expect("normal")),
                loops: vec![Profile::rectangle(Point2::new(-60.0, -60.0), 120.0, 120.0, 0)],
            },
            extent: Extent::Distance(40.0),
            op: Combine::Cut,
        });
    }
    part
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

/// The name of the face of `body` nearest `p`.
pub fn face_near(body: &Body, p: Point3) -> Name {
    use forge_geom::SurfaceGeom;
    let d = |f: forge_topo::FaceId| {
        let s = &body.face(f).surface;
        let on = s.eval(s.project(p)).distance(p);
        on + body.face_box(f).grow(0.0).center().distance(p) * 1e-6
    };
    body.face_ids()
        .min_by(|&a, &b| d(a).total_cmp(&d(b)))
        .map(|f| body.face(f).name)
        .expect("a body has faces")
}

/// `part` followed by a face fillet between the faces nearest `first` and those nearest `second`.
pub fn with_face_fillet(part: &Part, first: &[Point3], second: &[Point3], radius: f64) -> Part {
    let body = part.regenerate().expect("base part regenerates");
    let names = |ps: &[Point3]| ps.iter().map(|&p| face_near(&body, p)).collect();
    let mut out = part.clone();
    out.features.push(Feature::FaceFillet {
        first: names(first),
        second: names(second),
        radius,
    });
    out
}

/// `part` followed by a full round of the face nearest `middle` between those nearest `side`, `other`.
pub fn with_full_round(part: &Part, side: Point3, middle: Point3, other: Point3) -> Part {
    let body = part.regenerate().expect("base part regenerates");
    let mut out = part.clone();
    out.features.push(Feature::FullRound {
        side: face_near(&body, side),
        middle: face_near(&body, middle),
        other: face_near(&body, other),
    });
    out
}

/// `part` followed by a fillet of the edges nearest each point, each with its radius.
pub fn with_fillet(part: &Part, near: &[(Point3, f64)]) -> Part {
    let near: Vec<(Point3, Radius)> = near.iter().map(|&(p, r)| (p, r.into())).collect();
    with_var_fillet(part, &near)
}

/// `with_fillet` with a radius law per edge (along each edge as the part's body runs it).
pub fn with_var_fillet(part: &Part, near: &[(Point3, Radius)]) -> Part {
    let body = part.regenerate().expect("base part regenerates");
    let edges = near
        .iter()
        .map(|(p, r)| (edge_near(&body, *p), r.clone()))
        .collect();
    let mut out = part.clone();
    out.features.push(Feature::Fillet {
        only: false,
        edges,
        starts: Vec::new(),
        setbacks: Vec::new(),
    });
    out
}

/// A fillet case's picks: each edge's middle and its radius.
pub type Pick = (Point3, Radius);

/// `part` followed by the fillet of `picks`, the edges picked from `body` (the part as
/// some regeneration made it, which may name things differently). Laws start where the
/// geometry says, so they need no turning round.
pub fn with_fillet_from(part: &Part, body: &Body, picks: &[Pick]) -> Part {
    let edges = picks
        .iter()
        .map(|(mid, r)| (edge_near(body, *mid), r.clone()))
        .collect();
    let mut out = part.clone();
    out.features.push(Feature::Fillet {
        only: false,
        edges,
        starts: Vec::new(),
        setbacks: Vec::new(),
    });
    out
}

/// The picks a fillet case holds (from its names).
pub fn picks(part: &Part) -> Vec<Pick> {
    let mut base = part.clone();
    let Some(Feature::Fillet { edges, .. }) = base.features.pop() else {
        return Vec::new();
    };
    let body = base.regenerate().expect("base part regenerates");
    edges
        .iter()
        .map(|(n, r)| {
            let e = body.edge_by_name(*n).expect("named edge");
            let x = body.edge(e);
            (x.curve.eval(x.range.mid()), r.clone())
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
            extent: Extent::Distance(d),
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
                extent: Extent::Distance(14.0),
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
        extent: Extent::Distance(16.0),
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
    // corner patches: three blends of three radii at a box corner (a six-sided hole)
    out.push((
        "box-corner-unequal",
        with_fillet(
            &boxed,
            &[
                (Point3::new(10.0, 0.0, 8.0), 1.0),
                (Point3::new(0.0, 5.0, 8.0), 2.0),
                (Point3::new(0.0, 0.0, 4.0), 3.0),
            ],
        ),
        None,
    ));
    // two blends and a sharp edge at a box corner: the edge is cut back and the faces trimmed
    out.push((
        "two-blends-sharp-edge",
        with_fillet(
            &boxed,
            &[
                (Point3::new(10.0, 0.0, 8.0), 2.0),
                (Point3::new(20.0, 5.0, 8.0), 2.0),
            ],
        ),
        None,
    ));
    // the pocket with a larger floor radius: four concave six-sided holes
    let floor =
        [(10.0, 2.0), (10.0, 8.0), (5.0, 5.0), (15.0, 5.0)].map(|(x, y)| (Point3::new(x, y, 4.0), 1.5));
    out.push((
        "pocket-unequal-corners",
        with_fillet(&pocket, &[walls, floor].concat()),
        None,
    ));
    // the step's concave edge and the two convex edges at its front end: a mixed corner
    out.push((
        "step-mixed-corner",
        with_fillet(
            &step,
            &[
                (Point3::new(10.0, 5.0, 8.0), 1.0),
                (Point3::new(15.0, 0.0, 8.0), 1.5),
                (Point3::new(10.0, 0.0, 11.0), 2.0),
            ],
        ),
        None,
    ));
    // pyramid apexes: four and five convex blends of different radii, and a regular
    // four-sided one whose equal blends meet on a ball
    let q = pi / 2.0;
    let around = |n: usize, slopes: &[f64]| -> Vec<(f64, f64)> {
        (0..n)
            .map(|i| ((i as f64 + 0.5) * 4.0 * q / n as f64, slopes[i % slopes.len()]))
            .collect()
    };
    for (name, planes, radii) in [
        ("apex-four-ball", around(4, &[1.0]), vec![1.5; 4]),
        (
            "apex-four-unequal",
            around(4, &[1.0, 0.8, 1.2, 0.9]),
            vec![1.0, 1.5, 0.8, 1.2],
        ),
        (
            "apex-five-unequal",
            around(5, &[0.9, 1.0, 1.1, 1.2, 1.3]),
            vec![1.0, 1.4, 0.8, 1.2, 1.1],
        ),
    ] {
        let (part, mids) = apex_post(&planes);
        let picks: Vec<(Point3, f64)> = mids.into_iter().zip(radii).collect();
        out.push((name, with_fillet(&part, &picks), None));
    }
    // pits: four and five concave blends meeting at the bottom
    for (name, planes, radii) in [
        (
            "pit-four",
            around(4, &[1.0, 0.8, 1.2, 0.9]),
            vec![1.0, 1.5, 0.8, 1.2],
        ),
        (
            "pit-five",
            (0..5)
                .map(|i| (i as f64 * 0.8 * q + 0.25, [0.5, 0.6, 0.7, 0.55, 0.65][i]))
                .collect(),
            vec![0.8, 1.0, 0.6, 0.9, 0.7],
        ),
    ] {
        let (part, mids) = pit(&planes);
        let picks: Vec<(Point3, f64)> = mids.into_iter().zip(radii).collect();
        out.push((name, with_fillet(&part, &picks), None));
    }
    // a boss and a slot whose corner sits on a roof's ridge, their sides running along it to
    // the block's end: four edges there, two convex and two concave for the boss, three
    // convex and one concave for the slot
    let mut boss = roofed();
    boss.features.push(block_at(
        Point3::new(8.0, 5.0, 2.0),
        17.0,
        4.0,
        12.0,
        Combine::Add,
    ));
    let mut slot = roofed();
    slot.features.push(block_at(
        Point3::new(8.0, 5.0, 2.0),
        17.0,
        4.0,
        20.0,
        Combine::Cut,
    ));
    let at = |pts: &[(f64, f64, f64)], r: &[f64]| -> Vec<(Point3, f64)> {
        pts.iter()
            .zip(r)
            .map(|(&(x, y, z), &r)| (Point3::new(x, y, z), r))
            .collect()
    };
    out.push((
        "roof-boss-on-ridge",
        with_fillet(
            &boss,
            &at(
                &[
                    (4.0, 5.0, 8.0),
                    (8.0, 5.0, 11.0),
                    (14.0, 5.0, 8.0),
                    (8.0, 7.0, 6.0),
                ],
                &[1.0, 0.8, 0.6, 0.7],
            ),
        ),
        None,
    ));
    out.push((
        "roof-slot-on-ridge",
        with_fillet(
            &slot,
            &at(
                &[
                    (4.0, 5.0, 8.0),
                    (8.0, 5.0, 5.0),
                    (14.0, 5.0, 8.0),
                    (8.0, 7.0, 6.0),
                ],
                &[1.0, 0.8, 0.6, 0.7],
            ),
        ),
        None,
    ));
    // a square boss turned 45 degrees, one corner on the ridge: five edges there, the ridge
    // either side and the boss's edge convex, its two feet concave
    let mut diamond = roofed();
    let d = 4.0 / f64::sqrt(2.0);
    let corners = [
        (10.0, 5.0),
        (10.0 + d, 5.0 + d),
        (10.0, 5.0 + 2.0 * d),
        (10.0 - d, 5.0 + d),
    ]
    .map(|(x, y)| Point2::new(x, y));
    diamond.features.push(Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 2.0)),
            loops: vec![forge_ops::ProfileLoop::new(
                (0..4)
                    .map(|i| {
                        let seg = forge_ops::Segment::Line {
                            a: corners[i],
                            b: corners[(i + 1) % 4],
                        };
                        (seg, i as u64)
                    })
                    .collect(),
            )],
        },
        extent: Extent::Distance(12.0),
        op: Combine::Add,
    });
    let h = 2.0 / f64::sqrt(2.0);
    out.push((
        "roof-diamond-on-ridge",
        with_fillet(
            &diamond,
            &at(
                &[
                    (5.0, 5.0, 8.0),
                    (15.0, 5.0, 8.0),
                    (10.0, 5.0, 11.0),
                    (10.0 + h, 5.0 + h, 8.0 - h),
                    (10.0 - h, 5.0 + h, 8.0 - h),
                ],
                &[1.0, 0.9, 0.6, 0.7, 0.8],
            ),
        ),
        None,
    ));
    // a bar of L section: the edge's side face is narrow at the front, where the ball rolls
    // off it over the ledge onto the inside wall (the corner the spandrel cuts there is
    // the 2 x 1 rectangle less the radius-2 disc's slice between heights 1 and 2 over its centre)
    let r = 2.0;
    let ledge = 2.0 - (pi - 0.75f64.sqrt() - pi / 3.0);
    out.push((
        "l-bar-roll-over",
        with_fillet(&l_bar(), &[(Point3::new(0.0, 6.0, 8.0), r)]),
        Some(20.0 * 66.0 - spandrel(r) * 8.0 - ledge * 2.0),
    ));
    // a pocket in the front face just under the top edge: the ball rolls over it partway
    // along, cutting its walls and roof (the roof's front edge lies in the round and goes);
    // over the pocket only the corner left of the round outside the pocket is cut away
    let r = 2.0;
    let mut pocketed = boxed.clone();
    pocketed
        .features
        .push(block_at(Point3::new(8.0, -1.0, 3.0), 4.0, 2.0, 4.0, Combine::Cut));
    let y0 = 2.0 - 3f64.sqrt();
    let lens = y0 - (pi - 0.75f64.sqrt() - 2.0 * pi / 3.0);
    out.push((
        "box-edge-over-pocket",
        with_fillet(&pocketed, &[(Point3::new(10.0, 0.0, 8.0), r)]),
        Some(1600.0 - 16.0 - 20.0 * spandrel(r) + 4.0 * lens),
    ));
    out.extend(variable_cases());
    out.extend(variant_cases());
    out
}

/// Face fillets and full rounds.
pub fn variant_cases() -> Vec<(&'static str, Part, Option<f64>)> {
    let pi = dmath::PI;
    let spandrel = |r: f64| (1.0 - pi / 4.0) * r * r;
    let boxed = Part {
        features: vec![block_at(Point3::ORIGIN, 20.0, 10.0, 8.0, Combine::New)],
    };
    let (top, front) = (Point3::new(10.0, 5.0, 8.0), Point3::new(10.0, 0.0, 4.0));
    let mut chamfered = boxed.clone();
    chamfered.features.push(Feature::Chamfer {
        corner: Default::default(),
        edges: vec![edge_near(
            &boxed.regenerate().expect("box"),
            Point3::new(10.0, 0.0, 8.0),
        )],
        size: forge_ops::ChamferSize::Equal(1.0),
    });
    let rib = Part {
        features: vec![block_at(Point3::ORIGIN, 20.0, 4.0, 8.0, Combine::New)],
    };
    let pts = [(0.0, 0.0), (4.0, 0.0), (3.5, 8.0), (0.5, 8.0)];
    let drafted = Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::from_zx(Point3::ORIGIN, UnitVec3::X, Vec3::Y).expect("frame"),
                loops: vec![ProfileLoop::new(
                    (0..4)
                        .map(|i| {
                            let (a, b) = (pts[i], pts[(i + 1) % 4]);
                            (
                                Segment::Line {
                                    a: Point2::new(a.0, a.1),
                                    b: Point2::new(b.0, b.1),
                                },
                                i as u64,
                            )
                        })
                        .collect(),
                )],
            },
            extent: Extent::Distance(20.0),
            op: Combine::New,
        }],
    };
    let full_round_drafted = with_full_round(
        &drafted,
        Point3::new(10.0, 0.25, 4.0),
        Point3::new(10.0, 2.0, 8.0),
        Point3::new(10.0, 3.75, 4.0),
    );
    let ring = Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::WORLD,
                loops: vec![
                    Profile::circle(Point2::ORIGIN, 10.0, 0),
                    Profile::circle(Point2::ORIGIN, 6.0, 1),
                ],
            },
            extent: Extent::Distance(8.0),
            op: Combine::New,
        }],
    };
    let full_round_narrowing = with_full_round(
        &narrowing_rib(),
        Point3::new(10.0, 0.5, 4.0),
        Point3::new(10.0, 2.0, 8.0),
        Point3::new(10.0, 3.5, 4.0),
    );
    // the bar rounded at radius 3 (centre (y, z) = (3, 5)); split, less the slope's wedge
    // over the round: depth z - 7.6 by width 7 + sqrt(9 - (z - 5)^2) for z from 7.6 to 8
    let bar = 20.0 * (80.0 - spandrel(3.0));
    let wedge = {
        let f = |u: f64| {
            let w = 9.0 - u * u;
            let root = if w > 0.0 { w.sqrt() } else { 0.0 };
            -w * root / 3.0 - 2.6 * 0.5 * (u * root + 9.0 * dmath::asin((u / 3.0).min(1.0)))
        };
        7.0 * 0.08 + f(3.0) - f(2.6)
    };
    let (top_pt, front_pt) = (Point3::new(10.0, 5.0, 8.0), Point3::new(10.0, 0.0, 3.0));
    vec![
        // a bevel of two facets: the ball rolls over both and they go
        (
            "face-fillet-over-two-bevels",
            with_face_fillet(&two_bevels(false), &[top_pt], &[front_pt], 3.0),
            Some(bar),
        ),
        // the facets end on two faces: the cut runs over both, through the edge between
        (
            "face-fillet-bevels-split-end",
            with_face_fillet(&two_bevels(true), &[top_pt], &[front_pt], 3.0),
            Some(bar - wedge),
        ),
        // a rib drafted across: the ball touches both sides and the top, its radius the same along
        ("full-round-drafted-rib", full_round_drafted, None),
        // a ring's top between its walls: a torus all round, no ends
        (
            "full-round-ring",
            with_full_round(
                &ring,
                Point3::new(10.0, 0.0, 4.0),
                Point3::new(8.0, 0.0, 8.0),
                Point3::new(6.0, 0.0, 4.0),
            ),
            Some(512.0 * pi - 16.0 * pi * (8.0 - 2.0 * pi)),
        ),
        // a rib narrowing along: the ball shrinks as it rolls, one face over the top
        (
            "full-round-narrowing-rib",
            full_round_narrowing,
            Some(narrowing_rib_round_volume()),
        ),
        // the two faces meet: their edge rounds
        (
            "face-fillet-adjacent",
            with_face_fillet(&boxed, &[top], &[front], 2.0),
            Some(1600.0 - spandrel(2.0) * 20.0),
        ),
        // a chamfer joins them: the ball rolls across it and it goes
        (
            "face-fillet-over-chamfer",
            with_face_fillet(&chamfered, &[top], &[Point3::new(10.0, 0.0, 3.0)], 3.0),
            Some(1600.0 - spandrel(3.0) * 20.0),
        ),
        // a rib's top taken away by the ball touching both sides and the top
        (
            "full-round-rib",
            with_full_round(
                &rib,
                Point3::new(10.0, 0.0, 4.0),
                Point3::new(10.0, 2.0, 8.0),
                Point3::new(10.0, 4.0, 4.0),
            ),
            Some(640.0 - 20.0 * (8.0 - 2.0 * pi)),
        ),
    ]
}

/// A rib narrowing along its length: 20 long in x, 8 high, 4 wide at x = 0 and 2 at x = 20,
/// its sides vertical planes.
pub fn narrowing_rib() -> Part {
    let pts = [(0.0, 0.0), (20.0, 1.0), (20.0, 3.0), (0.0, 4.0)];
    Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::WORLD,
                loops: vec![ProfileLoop::new(
                    (0..4)
                        .map(|i| {
                            let (a, b) = (pts[i], pts[(i + 1) % 4]);
                            (
                                Segment::Line {
                                    a: Point2::new(a.0, a.1),
                                    b: Point2::new(b.0, b.1),
                                },
                                i as u64,
                            )
                        })
                        .collect(),
                )],
            },
            extent: Extent::Distance(8.0),
            op: Combine::New,
        }],
    }
}

/// The narrowing rib's volume under its full round. The balls touching both sides and the
/// top have centres on a line and radii linear along it, so the round is a cone (apex
/// (40, 2, 8)); each x slice keeps the rectangle under the slice ellipse's centre and the
/// ellipse's upper half, a quadratic in x, so 3-point Gauss-Legendre is exact.
pub fn narrowing_rib_round_volume() -> f64 {
    let k = f64::sqrt(1.0 + 1.0 / 400.0);
    // axis from the apex toward the ball at x = 0 (centre (0, 2, 8 - 2/k), radius 2/k)
    let (ax, az) = (-40.0, -2.0 / k);
    let len = f64::sqrt(ax * ax + az * az);
    let (dx, dz) = (ax / len, az / len);
    let sin = (2.0 / k) / len;
    let cos2 = 1.0 - sin * sin;
    let slice = |x: f64| {
        let w = x - 40.0;
        let (a, b, c, d) = (cos2, cos2 - dz * dz, -w * dx * dz, w * w * (dx * dx - cos2));
        let e = d + c * c / b;
        let zc = 8.0 - c / b;
        2.0 * (e / a).sqrt() * zc + dmath::PI * e / (2.0 * (a * b).sqrt())
    };
    let g = f64::sqrt(0.6);
    10.0 * (5.0 * slice(10.0 - 10.0 * g) + 8.0 * slice(10.0) + 5.0 * slice(10.0 + 10.0 * g)) / 9.0
}

/// A 20 long bar along x, 10 deep in y and 8 high, its top front edge bevelled in two facets:
/// from (y, z) = (2, 8) to (0.8, 7.6) to (0, 6). With `split`, its far end above z = 7.6 is cut
/// back at 45 degrees, so the upper facet ends on that slope and the lower on the end face.
pub fn two_bevels(split: bool) -> Part {
    let pts = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 8.0),
        (2.0, 8.0),
        (0.8, 7.6),
        (0.0, 6.0),
    ];
    let outline = ProfileLoop::new(
        (0..6)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % 6]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    i as u64,
                )
            })
            .collect(),
    );
    let mut part = Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::from_zx(Point3::ORIGIN, UnitVec3::X, Vec3::Y).expect("frame"),
                loops: vec![outline],
            },
            extent: Extent::Distance(20.0),
            op: Combine::New,
        }],
    };
    if split {
        let wedge = [(20.0, 7.6), (21.0, 7.6), (21.0, 9.0), (18.6, 9.0)];
        part.features.push(Feature::Extrude {
            profile: Profile {
                // x along x, y up z, extruded toward -y through the bar
                plane: Frame::from_zx(Point3::new(0.0, 11.0, 0.0), -UnitVec3::Y, Vec3::X).expect("frame"),
                loops: vec![ProfileLoop::new(
                    (0..4)
                        .map(|i| {
                            let (a, b) = (wedge[i], wedge[(i + 1) % 4]);
                            (
                                Segment::Line {
                                    a: Point2::new(a.0, a.1),
                                    b: Point2::new(b.0, b.1),
                                },
                                i as u64,
                            )
                        })
                        .collect(),
                )],
            },
            extent: Extent::Distance(12.0),
            op: Combine::Cut,
        });
    }
    part
}

/// A 20 long bar along x of L section in yz: 10 wide, 8 high, with a 2 x 7 notch under its
/// front edge, so its end face is only 1 high at the front.
pub fn l_bar() -> Part {
    let pts = [
        (0.0, 7.0),
        (2.0, 7.0),
        (2.0, 0.0),
        (10.0, 0.0),
        (10.0, 8.0),
        (0.0, 8.0),
    ];
    let outline = ProfileLoop::new(
        (0..6)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % 6]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    i as u64,
                )
            })
            .collect(),
    );
    Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::from_zx(Point3::ORIGIN, UnitVec3::X, Vec3::Y).expect("frame"),
                loops: vec![outline],
            },
            extent: Extent::Distance(20.0),
            op: Combine::New,
        }],
    }
}

/// A block whose top outline has one rounded corner: a straight edge, an arc and a straight
/// edge tangent in a row along the top, the other corners sharp.
pub fn one_round_corner() -> Part {
    let q = dmath::FRAC_PI_2;
    let outline = ProfileLoop::new(vec![
        (
            Segment::Line {
                a: Point2::new(0.0, 0.0),
                b: Point2::new(17.0, 0.0),
            },
            0,
        ),
        (
            Segment::Arc {
                center: Point2::new(17.0, 3.0),
                radius: 3.0,
                start: -q,
                sweep: q,
            },
            1,
        ),
        (
            Segment::Line {
                a: Point2::new(20.0, 3.0),
                b: Point2::new(20.0, 10.0),
            },
            2,
        ),
        (
            Segment::Line {
                a: Point2::new(20.0, 10.0),
                b: Point2::new(0.0, 10.0),
            },
            3,
        ),
        (
            Segment::Line {
                a: Point2::new(0.0, 10.0),
                b: Point2::new(0.0, 0.0),
            },
            4,
        ),
    ]);
    Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::WORLD,
                loops: vec![outline],
            },
            extent: Extent::Distance(8.0),
            op: Combine::New,
        }],
    }
}

/// Variable radii: laws along one edge, chains whose picks differ, closed edges with a
/// periodic law, and a variable blend at a corner. No closed forms: the mesh, validity,
/// G1 and n-ary checks hold them.
pub fn variable_cases() -> Vec<(&'static str, Part, Option<f64>)> {
    let o = Point3::ORIGIN;
    let pts = |p: &[(f64, f64)]| Radius::Points(p.to_vec());
    let boxed = Part {
        features: vec![block_at(o, 20.0, 10.0, 8.0, Combine::New)],
    };
    let mut step = boxed.clone();
    step.features.push(block_at(
        Point3::new(0.0, 0.0, 8.0),
        10.0,
        10.0,
        6.0,
        Combine::Add,
    ));
    let mut boss = boxed.clone();
    boss.features
        .push(post_at(Point3::new(10.0, 5.0, 8.0), 3.0, 5.0, Combine::Add));
    let rounded = Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::rounded_rectangle(Point2::ORIGIN, 20.0, 12.0, 3.0, 0)],
            },
            extent: Extent::Distance(6.0),
            op: Combine::New,
        }],
    };
    let mut stand = Part {
        features: vec![block_at(
            Point3::new(-5.0, -5.0, -4.0),
            30.0,
            22.0,
            4.0,
            Combine::New,
        )],
    };
    let mut top = rounded.features[0].clone();
    if let Feature::Extrude { op, .. } = &mut top {
        *op = Combine::Add;
    }
    stand.features.push(top);
    let corner = one_round_corner();
    let arc_mid = Point3::new(17.0 + 3.0 * f64::sqrt(0.5), 3.0 - 3.0 * f64::sqrt(0.5), 8.0);
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
    let q = dmath::FRAC_PI_2;
    let (apex, mids) = apex_post(&[(0.5 * q, 1.0), (1.5 * q, 0.8), (2.5 * q, 1.2), (3.5 * q, 0.9)]);
    let front = Point3::new(10.0, 0.0, 8.0);
    vec![
        (
            "box-linear-edge",
            with_var_fillet(&boxed, &[(front, Radius::Linear(1.0, 3.0))]),
            None,
        ),
        (
            "box-points-edge",
            with_var_fillet(&boxed, &[(front, pts(&[(0.0, 1.0), (0.5, 2.5), (1.0, 1.5)]))]),
            None,
        ),
        (
            "step-linear-concave",
            with_var_fillet(&step, &[(Point3::new(10.0, 5.0, 8.0), Radius::Linear(1.0, 2.5))]),
            None,
        ),
        (
            "boss-rim-periodic",
            with_var_fillet(
                &boss,
                &[(Point3::new(7.0, 5.0, 13.0), pts(&[(0.0, 0.6), (0.5, 1.4)]))],
            ),
            None,
        ),
        (
            "boss-foot-periodic",
            with_var_fillet(
                &boss,
                &[(
                    Point3::new(7.0, 5.0, 8.0),
                    pts(&[(0.0, 0.8), (0.25, 1.5), (0.6, 1.0)]),
                )],
            ),
            None,
        ),
        (
            "shaft-post-periodic",
            with_var_fillet(
                &posted,
                &[(
                    Point3::new(0.0, 2.0, f64::sqrt(32.0)),
                    pts(&[(0.0, 0.8), (0.5, 1.3)]),
                )],
            ),
            None,
        ),
        (
            "rounded-top-linear-chain",
            with_var_fillet(
                &rounded,
                &[(Point3::new(10.0, 0.0, 6.0), Radius::Linear(0.8, 1.6))],
            ),
            None,
        ),
        (
            "rounded-top-two-radii",
            with_var_fillet(
                &rounded,
                &[
                    (Point3::new(10.0, 0.0, 6.0), 1.0.into()),
                    (Point3::new(20.0, 6.0, 6.0), 1.3.into()),
                ],
            ),
            None,
        ),
        (
            "rounded-foot-linear-chain",
            with_var_fillet(&stand, &[(Point3::new(10.0, 0.0, 0.0), Radius::Linear(0.8, 1.4))]),
            None,
        ),
        (
            "corner-chain-two-radii",
            with_var_fillet(
                &corner,
                &[
                    (Point3::new(8.0, 0.0, 8.0), 1.0.into()),
                    (Point3::new(20.0, 6.0, 8.0), 1.3.into()),
                ],
            ),
            None,
        ),
        (
            "corner-chain-adjacent-radii",
            with_var_fillet(
                &corner,
                &[(Point3::new(8.0, 0.0, 8.0), 1.0.into()), (arc_mid, 1.5.into())],
            ),
            None,
        ),
        (
            "corner-chain-mixed",
            with_var_fillet(
                &corner,
                &[
                    (Point3::new(8.0, 0.0, 8.0), Radius::Linear(1.4, 0.6)),
                    (Point3::new(20.0, 6.0, 8.0), 1.0.into()),
                ],
            ),
            None,
        ),
        (
            "l-bar-roll-over-linear",
            with_var_fillet(
                &l_bar(),
                &[(Point3::new(0.0, 6.0, 8.0), Radius::Linear(1.6, 2.4))],
            ),
            None,
        ),
        (
            "box-linear-over-pocket",
            {
                let mut pocketed = boxed.clone();
                pocketed
                    .features
                    .push(block_at(Point3::new(8.0, -1.0, 3.0), 4.0, 2.0, 4.0, Combine::Cut));
                with_var_fillet(&pocketed, &[(front, Radius::Linear(1.6, 2.4))])
            },
            None,
        ),
        (
            "box-corner-variable",
            with_var_fillet(
                &boxed,
                &[
                    (front, Radius::Linear(1.0, 2.0)),
                    (Point3::new(0.0, 5.0, 8.0), 1.0.into()),
                    (Point3::new(0.0, 0.0, 4.0), 1.0.into()),
                ],
            ),
            None,
        ),
        (
            "apex-variable",
            with_var_fillet(
                &apex,
                &[
                    (mids[0], Radius::Linear(1.0, 1.6)),
                    (mids[1], 1.5.into()),
                    (mids[2], 0.8.into()),
                    (mids[3], 1.2.into()),
                ],
            ),
            None,
        ),
    ]
}

/// Fillets that cannot be built, and the error each must give (by its name).
pub fn refusals() -> Vec<(&'static str, Part, &'static str)> {
    let rounded = Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::WORLD,
                loops: vec![Profile::rounded_rectangle(Point2::ORIGIN, 20.0, 12.0, 3.0, 0)],
            },
            extent: Extent::Distance(6.0),
            op: Combine::New,
        }],
    };
    // a ring whose hole is off centre: its top narrows round it, and a ring's round holds one radius
    let eccentric = Part {
        features: vec![Feature::Extrude {
            profile: Profile {
                plane: Frame::WORLD,
                loops: vec![
                    Profile::circle(Point2::ORIGIN, 10.0, 0),
                    Profile::circle(Point2::new(1.0, 0.0), 5.0, 1),
                ],
            },
            extent: Extent::Distance(8.0),
            op: Combine::New,
        }],
    };
    let eccentric_round = with_full_round(
        &eccentric,
        Point3::new(-10.0, 0.0, 4.0),
        Point3::new(-7.0, 0.0, 8.0),
        Point3::new(-4.0, 0.0, 4.0),
    );
    vec![
        ("full-round-eccentric-ring", eccentric_round, "BadRadius"),
        // the seam between a flat side and a rounded corner is already smooth
        (
            "smooth-seam",
            with_fillet(&rounded, &[(Point3::new(17.0, 0.0, 3.0), 1.0)]),
            "Smooth",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::SurfaceGeom;
    use forge_mesh::{Quality, tessellate};
    use forge_topo::mass::mass;

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

    /// How far points of `a`'s procedural faces lie from `b`'s, or None when it has none.
    fn procedural_gap(a: &Body, b: &Body) -> Option<f64> {
        use forge_geom::Surface;
        let procs = |b: &Body| -> Vec<Surface> {
            b.face_ids()
                .map(|f| b.face(f).surface.clone())
                .filter(|s| matches!(s, Surface::Procedural(_)))
                .collect()
        };
        let (pa, pb) = (procs(a), procs(b));
        if pa.is_empty() {
            return None;
        }
        let mut worst = 0.0f64;
        for sa in &pa {
            let (u, v) = (sa.domain().u.interval(), sa.domain().v.interval());
            for i in 0..=6 {
                for j in 0..=6 {
                    let p = sa.eval(Point2::new(
                        u.lo + u.width() * i as f64 / 6.0,
                        v.lo + v.width() * j as f64 / 6.0,
                    ));
                    worst = worst.max(pb.iter().map(|s| s.distance(p)).fold(f64::INFINITY, f64::min));
                }
            }
        }
        Some(worst)
    }

    /// One to three random edges of random parts, random radii (constant, linear or
    /// through three points): every result is a valid solid with a closed mesh, or an
    /// error, never a panic.
    #[test]
    fn random_fillets() {
        use forge_math::rng::Rng;
        use forge_ops::{FilletEdge, fillet};
        // each seed its own draws, so each on its own thread
        let seeds: Vec<u64> = (1..=150).collect();
        let built = crate::suite::each(&seeds, |&seed| {
            let Ok(body) = crate::suite::fuzz(seed, 4).regenerate() else {
                return false;
            };
            let mut r = Rng::new(seed * 7919);
            let edges: Vec<_> = body.edge_ids().collect();
            let mut pick: Vec<FilletEdge> = Vec::new();
            for _ in 0..1 + r.below(3) {
                let edge = edges[r.below(edges.len())];
                let radius = match r.below(3) {
                    0 => Radius::Constant(r.range(0.05, 1.0)),
                    1 => Radius::Linear(r.range(0.05, 1.0), r.range(0.05, 1.0)),
                    _ => Radius::Points(vec![
                        (0.0, r.range(0.05, 1.0)),
                        (r.range(0.2, 0.8), r.range(0.05, 1.0)),
                        (1.0, r.range(0.05, 1.0)),
                    ]),
                };
                if pick.iter().all(|p| p.edge != edge) {
                    pick.push(FilletEdge { edge, radius });
                }
            }
            let Ok(b) = fillet(&body, &pick, 999) else {
                return false;
            };
            let problems = forge_topo::check(&b);
            assert!(problems.is_empty(), "seed {seed}: {problems:?}");
            let mesh = tessellate(&b, &Quality::default()).unwrap_or_else(|e| panic!("seed {seed}: {e:?}"));
            assert!(mesh.open_edges().is_empty(), "seed {seed}: open mesh");
            true
        });
        let ok = built.iter().filter(|&&b| b).count();
        // seam picks are refused as smooth and adjacent picks as corners; the rest build
        assert!(ok >= 70, "only {ok} of 150 built");
    }

    /// A radius running from 1 to 3 along an edge takes off more than 1 all along and less
    /// than 3 all along.
    #[test]
    fn linear_radius_lies_between_its_ends() {
        let boxed = Part {
            features: vec![block_at(Point3::ORIGIN, 20.0, 10.0, 8.0, Combine::New)],
        };
        let front = Point3::new(10.0, 0.0, 8.0);
        let vol = |r: Radius| {
            let body = with_var_fillet(&boxed, &[(front, r)])
                .regenerate()
                .expect("builds");
            mass(&body).expect("mass properties").volume
        };
        let (v1, v13, v3) = (vol(1.0.into()), vol(Radius::Linear(1.0, 3.0)), vol(3.0.into()));
        assert!(v3 < v13 && v13 < v1, "{v3} {v13} {v1}");
        // the cut is about (1 - pi/4) times the integral of r^2
        let want = 1600.0 - (1.0 - dmath::PI / 4.0) * 20.0 * (1.0 + 3.0 + 9.0) / 3.0;
        assert!((v13 - want).abs() < 0.01 * (1600.0 - want), "{v13} {want}");
    }

    /// A law started at the edge's other vertex is the law turned round; corners set back by
    /// given distances build, and a setback where no corner is refuses.
    #[test]
    fn law_starts_and_setbacks() {
        let boxed = Part {
            features: vec![block_at(Point3::ORIGIN, 20.0, 10.0, 8.0, Combine::New)],
        };
        let body = boxed.regenerate().expect("box");
        let pick = |p: Point3| edge_near(&body, p);
        let front = pick(Point3::new(10.0, 0.0, 8.0));
        let e = body.edge_by_name(front).expect("edge");
        let x = body.edge(e);
        // the front top edge runs along x: its law starts at its lower x end
        let first = if body.vertex(x.start).point.x < body.vertex(x.end).point.x {
            x.start
        } else {
            x.end
        };
        let other = if x.start == first { x.end } else { x.start };
        let fillet = |edges: Vec<(Name, Radius)>, starts, setbacks| {
            let mut p = boxed.clone();
            p.features.push(Feature::Fillet {
                only: false,
                edges,
                starts,
                setbacks,
            });
            p.regenerate()
        };
        let vol = |b: &Body| mass(b).expect("mass properties").volume;
        let a = fillet(
            vec![(front, Radius::Linear(1.0, 3.0))],
            vec![(front, body.vertex(other).name)],
            vec![],
        )
        .expect("from the other vertex");
        let b = fillet(vec![(front, Radius::Linear(3.0, 1.0))], vec![], vec![]).expect("turned round");
        assert!((vol(&a) - vol(&b)).abs() < 1e-9 * vol(&a));
        // the three edges at a corner, set back further than their ball would stop
        let corner = body
            .vertex_ids()
            .find(|&v| body.vertex(v).point.distance(Point3::new(0.0, 0.0, 8.0)) < 1e-9)
            .expect("corner");
        let cn = body.vertex(corner).name;
        let three = [(10.0, 0.0, 8.0), (0.0, 5.0, 8.0), (0.0, 0.0, 4.0)]
            .map(|(x, y, z)| (pick(Point3::new(x, y, z)), Radius::Constant(2.0)))
            .to_vec();
        let ball = fillet(three.clone(), vec![], vec![]).expect("ball corner");
        for setbacks in [
            vec![(cn, None, 3.5)],
            vec![(cn, None, 3.0), (cn, Some(three[0].0), 4.5)],
        ] {
            let b = fillet(three.clone(), vec![], setbacks).expect("set back");
            assert!(forge_topo::check(&b).is_empty());
            let m = tessellate(&b, &Quality::default()).expect("meshes");
            assert!(m.open_edges().is_empty());
            assert!(
                (vol(&b) - vol(&ball)).abs() > 1e-6,
                "the setback changes the corner"
            );
        }
        let far = body
            .vertex_ids()
            .find(|&v| body.vertex(v).point.distance(Point3::new(20.0, 0.0, 8.0)) < 1e-9)
            .expect("far end");
        let err =
            fillet(three, vec![], vec![(body.vertex(far).name, None, 3.0)]).expect_err("no corner there");
        assert!(format!("{err:?}").contains("Corner"), "{err:?}");
    }

    #[test]
    fn setbacks_measure_along_curved_edges() {
        // a quarter disc of radius 10, 6 thick: at (10, 0, 6) the top's arc meets a line and the side
        let slice = Part {
            features: vec![Feature::Extrude {
                profile: Profile {
                    plane: Frame::WORLD,
                    loops: vec![ProfileLoop::new(vec![
                        (
                            Segment::Line {
                                a: Point2::ORIGIN,
                                b: Point2::new(10.0, 0.0),
                            },
                            0,
                        ),
                        (
                            Segment::Arc {
                                center: Point2::ORIGIN,
                                radius: 10.0,
                                start: 0.0,
                                sweep: dmath::FRAC_PI_2,
                            },
                            1,
                        ),
                        (
                            Segment::Line {
                                a: Point2::new(0.0, 10.0),
                                b: Point2::ORIGIN,
                            },
                            2,
                        ),
                    ])],
                },
                extent: Extent::Distance(6.0),
                op: Combine::New,
            }],
        };
        let body = slice.regenerate().expect("slice");
        let arc = edge_near(&body, Point3::new(7.07, 7.07, 6.0));
        let edges = [
            arc,
            edge_near(&body, Point3::new(5.0, 0.0, 6.0)),
            edge_near(&body, Point3::new(10.0, 0.0, 3.0)),
        ]
        .map(|e| (e, Radius::Constant(1.5)))
        .to_vec();
        let corner = body
            .vertex_ids()
            .find(|&v| body.vertex(v).point.distance(Point3::new(10.0, 0.0, 6.0)) < 1e-9)
            .expect("corner");
        let mut p = slice.clone();
        p.features.push(Feature::Fillet {
            only: false,
            edges,
            starts: vec![],
            setbacks: vec![
                (body.vertex(corner).name, None, 3.0),
                (body.vertex(corner).name, Some(arc), 6.0),
            ],
        });
        let b = p.regenerate().expect("set back");
        assert!(forge_topo::check(&b).is_empty());
        // the arc's blend ends in the plane 6 along the arc (0.6 rad), not 6 straight (0.609 rad)
        let angles: Vec<f64> = b
            .vertex_ids()
            .map(|v| b.vertex(v).point)
            .filter(|p| p.z > 4.0 && (p.x * p.x + p.y * p.y) > 64.0 && p.y > 1.0 && p.y < 7.0)
            .map(|p| dmath::atan2(p.y, p.x))
            .collect();
        assert!(angles.iter().any(|a| (a - 0.6).abs() < 1e-7), "{angles:?}");
        assert!(
            angles.iter().all(|a| (a - 2.0 * dmath::asin(0.3)).abs() > 1e-3),
            "{angles:?}"
        );
    }

    #[test]
    fn variable_blend_boxes_hug_their_faces() {
        // run-out cuts askew across the blends' ends: each face's box holds its mesh, and
        // reaches no more than a tenth past it
        let all = cases();
        for name in [
            "box-linear-edge",
            "box-points-edge",
            "full-round-narrowing-rib",
            "l-bar-roll-over-linear",
        ] {
            let part = &all.iter().find(|c| c.0 == name).expect("a case").1;
            let b = part.regenerate().expect("builds");
            let m = tessellate(
                &b,
                &Quality {
                    chord: 1e-3,
                    max_angle: 0.3,
                },
            )
            .expect("meshes");
            for f in b.face_ids() {
                if !format!("{:?}", b.face(f).surface).contains("VarBlend") {
                    continue;
                }
                let mut mb = forge_math::Box3::EMPTY;
                for (t, _) in m
                    .triangles
                    .iter()
                    .zip(&m.face_of)
                    .filter(|x| *x.1 as usize == f.index())
                {
                    t.iter().for_each(|&i| mb.include(m.positions[i as usize]));
                }
                let fb = b.face_box(f);
                for (a, c) in [(fb.x, mb.x), (fb.y, mb.y), (fb.z, mb.z)] {
                    assert!(a.lo <= c.lo && c.hi <= a.hi, "{name}: box {fb:?} misses {mb:?}");
                    assert!(
                        c.lo - a.lo < 0.1 && a.hi - c.hi < 0.1,
                        "{name}: box {fb:?} loose on {mb:?}"
                    );
                }
            }
        }
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
            chord: 1e-2,
            max_angle: 0.3,
        };
        for (name, part, volume) in cases() {
            let body = part.regenerate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let problems = forge_topo::check(&body);
            assert!(problems.is_empty(), "{name}: {problems:?}");
            let mesh = tessellate(&body, &q).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(mesh.open_edges().is_empty(), "{name}: open mesh");
            let exact = mass(&body).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(
                exact.err[0] <= 1e-9 * exact.volume,
                "{name}: error bound {}",
                exact.err[0]
            );
            if let Some(v) = volume {
                assert!(
                    (exact.volume - v).abs() <= exact.err[0] + 1e-9 * v,
                    "{name}: volume {} want {v}",
                    exact.volume
                );
            }
            // traced spines hold a pipe's normals to a few 1e-9
            let g1 = smooth_mismatch(&body);
            assert!(g1 < 1e-8, "{name}: blend not tangent ({g1})");
            // n-ary and sequential booleans may name split edges differently: pick again
            let mut base = part.clone();
            base.features.pop();
            let nbase = base.regenerate_nary(2).expect("base regenerates n-ary");
            // (face fillets and full rounds name faces, which both builds name alike)
            let npart = match part.features.last() {
                Some(Feature::Fillet { .. }) => with_fillet_from(&base, &nbase, &picks(&part)),
                _ => part.clone(),
            };
            let nary = npart
                .regenerate_nary(2)
                .unwrap_or_else(|e| panic!("{name} nary: {e:?}"));
            assert!(forge_topo::check(&nary).is_empty(), "{name}: nary result invalid");
            // the same surfaces (traced ones to a few 1e-9)
            let gap = procedural_gap(&body, &nary);
            assert!(
                gap.is_none_or(|g| g < 1e-8),
                "{name}: nary surfaces differ by {gap:?}"
            );
            let m2 = mass(&nary).unwrap_or_else(|e| panic!("{name} nary: {e}"));
            // traced surfaces apart by g move the boundary by about g over the area
            let bound = exact.err[0] + m2.err[0] + gap.unwrap_or(0.0) * exact.area + 1e-12 * exact.volume;
            assert!(
                (m2.volume - exact.volume).abs() <= bound,
                "{name}: nary volume {} vs {} (bound {bound})",
                m2.volume,
                exact.volume
            );
        }
    }
}
