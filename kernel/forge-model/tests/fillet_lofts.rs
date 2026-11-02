//! Rounds along lofts and in narrow wedges.

use forge_geom::CurveGeom;
use forge_math::Point3;
use forge_mesh::{Quality, tessellate};
use forge_ops::{FilletEdge, Radius, fillet};

/// Rounds along a smooth loft through three sections: the edge's pieces meet where the sections
/// are, on two spans of one wall either side, which are one function of their parameters. Three
/// equal squares make a prism, whose rounds have a closed form; three different ones are checked
/// against their own mesh.
#[test]
fn rounds_run_along_a_loft_past_its_sections() {
    use forge_math::{Frame, Point2};
    use forge_ops::{LoftGuides, LoftKind, Profile, loft};
    let (a, r) = (10.0, 2.0);
    let square = |z: f64, s: f64| Profile {
        plane: Frame::from_z(Point3::new(0.0, 0.0, z), forge_math::UnitVec3::Z),
        loops: vec![Profile::rectangle(Point2::new(-s, -s), 2.0 * s, 2.0 * s, 0)],
    };
    let kind = LoftKind::Smooth {
        start: None,
        end: None,
    };
    let rounded = |body: &forge_topo::Body| {
        // the long edges: those running between the sections, not round them
        let long: Vec<FilletEdge> = body
            .edge_ids()
            .filter(|&e| {
                let x = body.edge(e);
                let (p, q) = (x.curve.eval(x.range.lo), x.curve.eval(x.range.hi));
                (p.z - q.z).abs() > 1.0
            })
            .map(|edge| FilletEdge {
                edge,
                radius: Radius::Constant(r),
            })
            .collect();
        assert_eq!(long.len(), 8);
        let b = fillet(body, &long, 999).expect("rounds");
        assert!(forge_topo::check(&b).is_empty());
        let mesh = tessellate(&b, &Quality::default()).expect("meshes");
        assert!(mesh.open_edges().is_empty());
        (forge_topo::mass::mass(&b).expect("mass").volume, mesh.volume())
    };
    let prism = loft(
        &[square(0.0, a), square(30.0, a), square(70.0, a)],
        kind,
        &LoftGuides::None,
        1,
    )
    .expect("loft");
    let (v, _) = rounded(&prism);
    let want = 70.0 * (4.0 * a * a - 4.0 * r * r * (1.0 - std::f64::consts::FRAC_PI_4));
    assert!((v - want).abs() < 1e-6 * want, "volume {v}, want {want}");
    let bulged = loft(
        &[square(0.0, a), square(30.0, 1.3 * a), square(70.0, a)],
        kind,
        &LoftGuides::None,
        1,
    )
    .expect("loft");
    let (v, m) = rounded(&bulged);
    assert!((v - m).abs() < 1e-3 * v, "volume {v}, mesh {m}");
}

/// A round in the narrow wedge between a steep cone and the wall of a pocket round it. The ball's
/// centre stands several radii from the edge. Coaxial, the volume has a closed form (Pappus); with
/// the pocket off the axis, the faces curve under the ball and the mesh is the reference.
#[test]
fn rounds_in_a_narrow_wedge() {
    use forge_bool::{Op, boolean};
    use forge_math::{Frame, Point2};
    use forge_ops::{LoftGuides, LoftKind, Profile, extrude, loft};
    let (rw, r, h) = (30.0, 2.0, 100.0);
    let (r0, r1) = (40.0, 10.0);
    let at_z = |z: f64| Frame::from_z(Point3::new(0.0, 0.0, z), forge_math::UnitVec3::Z);
    let build = |off: f64| {
        let block = extrude(
            &Profile {
                plane: at_z(0.0),
                loops: vec![
                    Profile::rectangle(Point2::new(-60.0, -60.0), 120.0, 120.0, 0),
                    Profile::circle(Point2::new(off, 0.0), rw, 10),
                ],
            },
            h,
            1,
        )
        .expect("block");
        let base = extrude(
            &Profile {
                plane: at_z(-5.0),
                loops: vec![Profile::rectangle(Point2::new(-60.0, -60.0), 120.0, 120.0, 0)],
            },
            5.0,
            2,
        )
        .expect("base");
        let cone = loft(
            &[
                Profile {
                    plane: at_z(0.0),
                    loops: vec![Profile::circle(Point2::ORIGIN, r0, 0)],
                },
                Profile {
                    plane: at_z(h),
                    loops: vec![Profile::circle(Point2::ORIGIN, r1, 0)],
                },
            ],
            LoftKind::Ruled,
            &LoftGuides::None,
            3,
        )
        .expect("cone");
        let cup = boolean(&block, &base, Op::Union, 4).expect("cup");
        boolean(&cup, &cone, Op::Union, 5).expect("union")
    };
    let pick = |b: &forge_topo::Body| -> Vec<FilletEdge> {
        b.edge_ids()
            .filter(|&e| {
                let x = b.edge(e);
                let p = x.curve.eval(x.range.mid());
                p.z > 5.0 && p.z < h - 1.0 && forge_ops::fillet_kind(b, e) == Some(false)
            })
            .map(|edge| FilletEdge {
                edge,
                radius: Radius::Constant(r),
            })
            .collect()
    };
    let body = build(0.0);
    let rounded = fillet(&body, &pick(&body), 999).expect("rounds");
    assert!(forge_topo::check(&rounded).is_empty());
    let added = forge_topo::mass::mass(&rounded).expect("mass").volume
        - forge_topo::mass::mass(&body).expect("mass").volume;
    // in the section (rho, z), from the edge: the wall straight up, the cone tilted in by a
    let a = forge_math::dmath::atan2(r0 - r1, h);
    let along = r / (forge_math::dmath::sin(a / 2.0) / forge_math::dmath::cos(a / 2.0));
    let (s, c) = (forge_math::dmath::sin(a), forge_math::dmath::cos(a));
    let wall = (rw, along);
    let foot = (rw - along * s, along * c);
    let centre = (rw - r, along);
    let tri = |q: (f64, f64)| 0.5 * along * r * (rw + q.0 + centre.0) / 3.0;
    let kite = tri(wall) + tri(foot);
    let theta = std::f64::consts::PI - a;
    let to_edge = (rw - centre.0, -centre.1);
    let len = (to_edge.0 * to_edge.0 + to_edge.1 * to_edge.1).sqrt();
    let cd = 4.0 * r * forge_math::dmath::sin(theta / 2.0) / (3.0 * theta);
    let sector = 0.5 * r * r * theta * (centre.0 + cd * to_edge.0 / len);
    let want = std::f64::consts::TAU * (kite - sector);
    assert!((added - want).abs() < 1e-6 * want, "added {added}, want {want}");
    let body = build(1.5);
    let rounded = fillet(&body, &pick(&body), 999).expect("rounds off the axis");
    assert!(forge_topo::check(&rounded).is_empty());
    let mesh = tessellate(&rounded, &Quality::default()).expect("meshes");
    assert!(mesh.open_edges().is_empty());
    let v = forge_topo::mass::mass(&rounded).expect("mass").volume;
    assert!((v - mesh.volume()).abs() < 1e-3 * v);
}
