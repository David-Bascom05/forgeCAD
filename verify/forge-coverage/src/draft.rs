//! G5 as a real drafted part (research/kernel-audit/coverage.md §2): a lid with the block's
//! footprint, its bottom drafted about its middle line and its ends about the block's top.
use forge_geom::SurfaceGeom;
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3};
use forge_model::{Combine, Extent, Feature, Part};
use forge_ops::profile::Profile;
use forge_topo::Body;

pub fn up_block(o: [f64; 3], w: f64, h: f64, d: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(Point3::new(o[0], o[1], o[2])),
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)],
        },
        extent: Extent::Distance(d),
        op,
    }
}

pub fn frame(o: [f64; 3], z: [f64; 3], x: [f64; 3]) -> Frame {
    Frame::from_zx(
        Point3::new(o[0], o[1], o[2]),
        UnitVec3::new_unchecked(Vec3::new(z[0], z[1], z[2])),
        Vec3::new(x[0], x[1], x[2]),
    )
    .expect("a frame")
}

/// The face whose outward normal is `n`.
pub fn face_named(body: &Body, n: [f64; 3]) -> forge_topo::Name {
    let want = Vec3::new(n[0], n[1], n[2]);
    body.face_ids()
        .find(|&f| {
            body.face_normal(f, body.face(f).surface.project(Point3::new(5.0, 5.0, 12.5)))
                .is_some_and(|m| m.get().dot(want) > 0.999)
        })
        .map(|f| body.face(f).name)
        .expect("a face with that normal")
}

/// The drafted lid's features: the lid, the bottom's draft, the ends' draft.
pub fn lid(angle: f64) -> Vec<Feature> {
    let base = vec![up_block([0.0, 0.0, 10.0], 10.0, 10.0, 5.0, Combine::New)];
    let b = Part {
        features: base.clone(),
    }
    .regenerate()
    .expect("the lid");
    let (bottom, front, back) = (
        face_named(&b, [0.0, 0.0, -1.0]),
        face_named(&b, [0.0, -1.0, 0.0]),
        face_named(&b, [0.0, 1.0, 0.0]),
    );
    let mut f = base;
    f.push(Feature::Draft {
        faces: vec![bottom],
        neutral: frame([0.0, 5.0, 10.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        angle,
    });
    f.push(Feature::Draft {
        faces: vec![front, back],
        neutral: frame([0.0, 0.0, 10.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        angle: -angle,
    });
    f
}

/// The drafted lid combined with the block (0..10)^3 by `op`: the part forge-model regenerates.
pub fn part(angle: f64, op: Combine) -> Part {
    let mut f = lid(angle);
    f.push(up_block([0.0, 0.0, 0.0], 10.0, 10.0, 10.0, op));
    Part { features: f }
}

/// A convex body as the polyhedron of its faces (each face one loop of straight edges).
pub fn poly(body: &Body) -> crate::tilt::Poly {
    let mut faces: Vec<Vec<crate::tilt::P>> = body
        .face_ids()
        .map(|f| {
            let l = body.face(f).loops[0];
            body.lp(l)
                .coedges()
                .map(|c| {
                    let p = body.vertex(body.coedge_start(c)).point;
                    [p.x, p.y, p.z]
                })
                .collect()
        })
        .collect();
    let mut pl = crate::tilt::Poly { faces: faces.clone() };
    if pl.volume() < 0.0 {
        for f in &mut faces {
            f.reverse();
        }
        pl = crate::tilt::Poly { faces };
    }
    pl
}

/// The exact volume of `part(angle, op)`, by clipping the drafted lid's polyhedron with the block.
pub fn expected(angle: f64, op: Combine) -> f64 {
    let lid = Part { features: lid(angle) }.regenerate().expect("the lid");
    let l = poly(&lid);
    let a = crate::tilt::prism(
        &frame([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        10.0,
    )
    .1;
    let keep = l.meet(&a).volume();
    match op {
        Combine::Add => a.volume() + l.volume() - keep,
        Combine::Cut => l.volume() - keep,
        _ => keep,
    }
}
