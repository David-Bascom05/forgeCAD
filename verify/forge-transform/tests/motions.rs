//! The kernel's rigid motions, mirrors and patterns against what lean/ForgeTransform proves.
//!
//! - `frames_compose_and_invert`: `Frame::compose` and `Frame::inverse` act as Rigid.lean
//!   proves: compose's motion is the composite, inverse undoes, both stay orthonormal and
//!   right-handed (to rounding).
//! - `turns_add_and_close`: the pattern's turns (pattern.rs:268) add their angles
//!   (`turn_turn`), fix the axis (`turn_axis`) and close up after n steps of 2π/n
//!   (`turn_full`).
//! - `mirrored_bodies_are_valid`: each benchmark part mirrored across a plane clear of it. The
//!   result passes `check`, and each copy's shell bounds the same signed volume as its original:
//!   positive, so the reflection kept the orientation, as Mirror.lean's `left_kept` and
//!   `mirror_*` say. Volume and area double, and the centroid is the mean of the original's and
//!   its image.
//! - `moved_bodies_keep_mass`: each part moved rigidly keeps volume, area and principal moments,
//!   and its centroid is the moved one.

use forge_math::rng::Rng;
use forge_math::{Frame, Point3, Vec3, dmath};
use forge_model::bodies::{BodyFeature, Step};
use forge_model::{Feature, Layout, Part, Source, suite};
use forge_topo::check::check;
use forge_topo::mass::{mass, shell_volume};

fn frame(r: &mut Rng) -> Frame {
    let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
        .normalize()
        .unwrap();
    let o = Point3::new(
        r.range(-100.0, 100.0),
        r.range(-100.0, 100.0),
        r.range(-100.0, 100.0),
    );
    Frame::from_zx(
        o,
        z,
        Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0)),
    )
    .unwrap_or_else(|| Frame::from_z(o, z))
}

fn near(a: Point3, b: Point3, tol: f64) -> bool {
    a.distance(b) <= tol
}

#[test]
fn frames_compose_and_invert() {
    let mut r = Rng::new(3);
    for _ in 0..10_000 {
        let (f, g) = (frame(&mut r), frame(&mut r));
        let p = Point3::new(r.range(-50.0, 50.0), r.range(-50.0, 50.0), r.range(-50.0, 50.0));
        let fg = f.compose(&g);
        assert!(near(
            fg.point_to_world(p),
            f.point_to_world(g.point_to_world(p)),
            1e-12
        ));
        let inv = f.inverse();
        assert!(near(inv.point_to_world(f.point_to_world(p)), p, 1e-12));
        assert!(near(f.point_to_world(inv.point_to_world(p)), p, 1e-12));
        assert!(
            near(inv.point_to_world(p), f.point_to_local(p), 1e-12),
            "inverse_world"
        );
        for h in [fg, inv] {
            let (x, y, z) = (h.x().get(), h.y().get(), h.z().get());
            assert!((x.length() - 1.0).abs() < 1e-14 && (y.length() - 1.0).abs() < 1e-14);
            assert!(x.dot(y).abs() < 1e-14 && y.dot(z).abs() < 1e-14 && z.dot(x).abs() < 1e-14);
            assert!((x.cross(y) - z).length() < 1e-14, "right-handed");
        }
    }
}

#[test]
fn turns_add_and_close() {
    let mut r = Rng::new(4);
    let turn = |at: &Frame, a: f64| at.rotated_about_z(a).compose(&at.inverse());
    for _ in 0..2000 {
        let at = frame(&mut r);
        let (a, b) = (r.range(-7.0, 7.0), r.range(-7.0, 7.0));
        let p = Point3::new(r.range(-50.0, 50.0), r.range(-50.0, 50.0), r.range(-50.0, 50.0));
        let (ta, tb, tab) = (turn(&at, a), turn(&at, b), turn(&at, a + b));
        assert!(near(
            ta.point_to_world(tb.point_to_world(p)),
            tab.point_to_world(p),
            1e-11
        ));
        let on = at.origin + at.z() * r.range(-50.0, 50.0);
        assert!(near(ta.point_to_world(on), on, 1e-11), "turn_axis");
        let n = 1 + (r.next_u64() % 12) as usize;
        let step = dmath::TAU / n as f64;
        let full = turn(&at, step * n as f64);
        assert!(near(full.point_to_world(p), p, 1e-11), "turn_full");
    }
}

fn parts() -> Vec<(&'static str, Part)> {
    suite::parts()
        .into_iter()
        .filter(|(n, _)| {
            matches!(
                *n,
                "plate" | "shaft" | "housing" | "coplanar" | "patterns" | "mixed" | "tour"
            )
        })
        .collect()
}

#[test]
fn mirrored_bodies_are_valid() {
    let mut r = Rng::new(9);
    let mut done = 0;
    for (name, part) in parts() {
        let Ok(b) = part.regenerate() else { continue };
        let m0 = mass(&b).unwrap();
        let bx = b.faces_box();
        let mid = bx.center();
        let reach = bx.diagonal();
        let n = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
            .normalize()
            .unwrap();
        // a mirror plane clear of the body: the copy does not touch it
        let plane = Frame::from_z(mid + n.get() * reach, n);
        let mut mirrored = part.clone();
        mirrored.features.push(Feature::Pattern {
            of: Source::Body,
            layout: Layout::Mirror { plane },
        });
        let both = mirrored.regenerate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(check(&both), vec![], "{name}");
        let m2 = mass(&both).unwrap();
        let tol = 1e-9 * m0.volume.abs();
        assert!((m2.volume - 2.0 * m0.volume).abs() <= tol, "{name}: volume");
        assert!((m2.area - 2.0 * m0.area).abs() <= 1e-9 * m0.area, "{name}: area");
        // the centroid of the two: halfway between the original's and its image
        let c = m0.centroid;
        let l = plane.point_to_local(c);
        let image = plane.point_to_world(Point3::new(l.x, l.y, -l.z));
        let mean = Point3::ORIGIN + ((c - Point3::ORIGIN) + (image - Point3::ORIGIN)) * 0.5;
        assert!(near(m2.centroid, mean, 1e-8 * reach), "{name}: centroid");
        // each shell's signed volume appears twice: the copy is oriented as the original
        let mut a: Vec<f64> = (0..b.shells.len())
            .map(|s| shell_volume(&b, forge_topo::body::ShellId(s as u32)))
            .collect();
        let mut both_v: Vec<f64> = (0..both.shells.len())
            .map(|s| shell_volume(&both, forge_topo::body::ShellId(s as u32)))
            .collect();
        a.extend(a.clone());
        a.sort_by(f64::total_cmp);
        both_v.sort_by(f64::total_cmp);
        assert_eq!(a.len(), both_v.len(), "{name}: shells");
        for (x, y) in a.iter().zip(&both_v) {
            assert!(
                (x - y).abs() <= 1e-9 * m0.volume.abs(),
                "{name}: shell {x} vs {y}"
            );
        }
        done += 1;
    }
    assert!(done >= 5, "{done} parts");
}

#[test]
fn moved_bodies_keep_mass() {
    let mut r = Rng::new(10);
    for (name, part) in parts() {
        let Ok(b) = part.regenerate() else { continue };
        let m0 = mass(&b).unwrap();
        let steps = vec![
            Step::Rotate {
                origin: Point3::new(r.range(-50.0, 50.0), r.range(-50.0, 50.0), r.range(-50.0, 50.0)),
                axis: Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0)),
                angle: r.range(-3.0, 3.0),
            },
            Step::Translate(Vec3::new(
                r.range(-200.0, 200.0),
                r.range(-200.0, 200.0),
                r.range(-200.0, 200.0),
            )),
        ];
        let cols = forge_model::body_motion(&steps);
        let apply = |p: Point3| {
            let (x, y, z) = (
                cols[0] - Point3::ORIGIN,
                cols[1] - Point3::ORIGIN,
                cols[2] - Point3::ORIGIN,
            );
            cols[3] + x * p.x + y * p.y + z * p.z
        };
        let mut moved = part.clone();
        moved
            .features
            .push(Feature::Body(Box::new(BodyFeature::Move { steps, copy: false })));
        let b2 = moved.regenerate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(check(&b2), vec![], "{name}");
        let m2 = mass(&b2).unwrap();
        let reach = b.faces_box().diagonal();
        assert!(
            (m2.volume - m0.volume).abs() <= 1e-9 * m0.volume.abs(),
            "{name}: volume"
        );
        assert!((m2.area - m0.area).abs() <= 1e-9 * m0.area, "{name}: area");
        assert!(
            near(m2.centroid, apply(m0.centroid), 1e-8 * reach),
            "{name}: centroid"
        );
        let big = m0.principal[2].abs();
        for k in 0..3 {
            assert!(
                (m2.principal[k] - m0.principal[k]).abs() <= 1e-8 * big,
                "{name}: principal {k}"
            );
        }
    }
}
