//! Fillet cases, from easy to brutal. Each must regenerate valid, mesh
//! closed, match its exact volume where there is a closed form, and be
//! tangent continuous wherever a blend meets its neighbours.

use crate::{Combine, Feature, Part};
use forge_geom::CurveGeom;
use forge_math::{Frame, Point2, Point3, dmath};
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
    out
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

    #[test]
    fn corpus() {
        let q = Quality {
            chord: 1e-4,
            max_angle: 0.05,
        };
        for (name, part, volume) in cases() {
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
            let g1 = smooth_mismatch(&body);
            assert!(g1 < 1e-9, "{name}: blend not tangent ({g1})");
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
