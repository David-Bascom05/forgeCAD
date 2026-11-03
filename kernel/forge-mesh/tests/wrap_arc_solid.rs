//! A solid with wrapped circle walls: a disc drawn in (s, h) and raised between two radii round a
//! cylinder, as a hole across a sheet metal bend leaves. Its volume and area against the closed form
//! and quadrature of research/sheet-metal/arc/check_arc.py.

use forge_geom::ruled::{Rail, Ruled};
use forge_geom::wrap::{Flat, Wrap};
use forge_geom::{Curve, Cylinder, Line, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point2, dmath};
use forge_mesh::{Quality, tessellate};
use forge_topo::mass::mass;
use forge_topo::{Body, Name, check};

const VOLUME: f64 = 57.59586531581287;
const AREA: f64 = 95.65140247873508;

/// The disc of radius 3 about (2, 1), unrolled at 10.8, between radii 10 and 12: two half arcs.
fn plug() -> Body {
    let (axis, unroll, levels) = (Frame::WORLD, 10.8, [10.0, 12.0]);
    let flats = [0.0, dmath::PI].map(|start| Flat::Arc {
        center: Point2::new(2.0, 1.0),
        radius: 3.0,
        start,
        sweep: dmath::PI,
    });
    let wrap = |radius: f64, flat: Flat| Wrap {
        axis,
        radius,
        unroll,
        flat,
    };
    let name = Name::feature(1);
    let mut body = Body::new();
    let shell = body.add_shell();
    let verts = [0, 1].map(|j| {
        [0, 1].map(|k| {
            let p = wrap(levels[j], flats[k]).at(0.0).0;
            body.add_vertex(p, name.with_str("vertex").with(j as u64).with(k as u64))
        })
    });
    let edges = [0, 1].map(|j| {
        [0, 1].map(|k| {
            let c = wrap(levels[j], flats[k]).curve();
            body.add_edge(
                c,
                Interval::new(0.0, 1.0),
                verts[j][k],
                verts[j][1 - k],
                name.with_str("section").with(j as u64).with(k as u64),
            )
        })
    });
    let rails = [0, 1].map(|k| {
        let (p0, p1) = (body.vertex(verts[0][k]).point, body.vertex(verts[1][k]).point);
        let line = Curve::Line(Line::through(p0, p1).expect("a rail"));
        body.add_edge(
            line,
            Interval::new(0.0, p0.distance(p1)),
            verts[0][k],
            verts[1][k],
            name.with_str("rail").with(k as u64),
        )
    });
    for k in 0..2 {
        let s = Ruled::new(
            Rail::Wrap(wrap(levels[0], flats[k])),
            Rail::Wrap(wrap(levels[1], flats[k])),
        )
        .expect("a wall")
        .surface();
        // outward: along the outline (counter-clockwise in (s, h)), crossed with the way up
        let (p, d1, _) = wrap(levels[0], flats[k]).at(0.5);
        let up = body.vertex(verts[1][k]).point - body.vertex(verts[0][k]).point;
        let same = s.normal(s.project(p)).is_none_or(|n| n.dot(d1.cross(up)) > 0.0);
        let face = body.add_face(shell, s, same, name.with_str("side").with(k as u64));
        body.add_loop(
            face,
            &[
                (edges[0][k], true),
                (rails[1 - k], true),
                (edges[1][k], false),
                (rails[k], false),
            ],
        );
    }
    for (j, same) in [(0, false), (1, true)] {
        let cap = Surface::Cylinder(Cylinder {
            frame: axis,
            radius: levels[j],
        });
        let face = body.add_face(shell, cap, same, name.with_str("cap").with(j as u64));
        let uses = if same {
            [(edges[j][0], true), (edges[j][1], true)]
        } else {
            [(edges[j][1], false), (edges[j][0], false)]
        };
        body.add_loop(face, &uses);
    }
    body
}

#[test]
fn a_wrapped_disc_is_valid_and_its_mass_exact() {
    let body = plug();
    let problems = check(&body);
    assert!(problems.is_empty(), "{problems:?}");
    let m = mass(&body).expect("mass");
    let (dv, da) = ((m.volume - VOLUME).abs(), (m.area - AREA).abs());
    assert!(dv <= 1e-12 * VOLUME && dv <= m.err[0], "{dv} {:?}", m.err);
    // the area right to rounding, and its bound honest and close (research/mass-props/ruled-area/)
    assert!(
        da <= 1e-14 * AREA && da <= m.err[1] && m.err[1] <= 1e-12 * AREA,
        "{da} {:?}",
        m.err
    );
}

#[test]
fn a_wrapped_disc_meshes_closed() {
    let q = Quality::default();
    let mesh = tessellate(&plug(), &q).expect("a mesh");
    assert!(mesh.open_edges().is_empty());
    // chords cut inside the curved faces by at most the chord tolerance
    let miss = mesh.volume() - VOLUME;
    assert!(miss <= 1e-9 && miss > -q.chord * AREA, "{miss}");
}
