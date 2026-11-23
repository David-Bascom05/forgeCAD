//! A body scaled uniformly (Step::Scale): the same faces, edges, vertices and names, a valid solid,
//! volume times k³, area times k², the centroid's image the centroid, for every suite part (planes,
//! cylinders, cones, spheres, tori, intersection curves, splines, blends, sweeps, coils, threads,
//! text) and for scales mixed with rigid steps.

use forge_math::{Point3, Vec3, dmath};
use forge_model::{BodyError, BodyFeature, Feature, Part, RegenError, Step, suite};
use forge_topo::{Body, check};

fn names(b: &Body) -> (Vec<u64>, Vec<u64>, Vec<u64>) {
    let mut f: Vec<u64> = b.faces.iter().filter(|f| f.alive).map(|f| f.name.0).collect();
    let mut e: Vec<u64> = b.edges.iter().filter(|e| e.alive).map(|e| e.name.0).collect();
    let mut v: Vec<u64> = b.vertices.iter().map(|v| v.name.0).collect();
    f.sort_unstable();
    e.sort_unstable();
    v.sort_unstable();
    (f, e, v)
}

fn with(part: &Part, steps: Vec<Step>) -> Part {
    let mut p = part.clone();
    p.features
        .push(Feature::Body(Box::new(BodyFeature::Move { steps, copy: false })));
    p
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(1e-300)
}

#[test]
fn every_suite_part_scales_exactly() {
    let c = Point3::new(3.0, -7.0, 2.5);
    let parts = suite::parts();
    let rows = suite::each(&parts, |(name, part)| {
        let Ok(b) = part.regenerate() else {
            return None;
        };
        let Ok(m0) = forge_topo::mass::mass(&b) else {
            return None;
        };
        let mut out = vec![];
        for k in [2.5, 0.375, 1.7] {
            let s = with(part, vec![Step::Scale { about: c, k }])
                .regenerate()
                .unwrap_or_else(|e| panic!("{name} k={k}: {e:?}"));
            assert!(check(&s).is_empty(), "{name} k={k}: {:?}", check(&s));
            assert_eq!(names(&s), names(&b.compact()), "{name} k={k}: names");
            assert_eq!(
                (s.faces.len(), s.edges.len(), s.vertices.len(), s.loops.len()),
                {
                    let b = b.compact();
                    (b.faces.len(), b.edges.len(), b.vertices.len(), b.loops.len())
                },
                "{name}: topology"
            );
            let m1 = forge_topo::mass::mass(&s).expect("mass");
            // the mass rules are exact per kind, so the ratios are k³ and k² to rounding
            assert!(
                close(m1.volume, m0.volume * k * k * k, 1e-9),
                "{name} k={k}: volume {} {}",
                m1.volume,
                m0.volume
            );
            assert!(
                close(m1.area, m0.area * k * k, 1e-9),
                "{name} k={k}: area {} {}",
                m1.area,
                m0.area
            );
            let want = c + (m0.centroid - c) * k;
            assert!(
                m1.centroid.distance(want) <= 1e-9 * (1.0 + want.to_vec().length()),
                "{name} k={k}: centroid"
            );
            out.push(k);
        }
        Some(out.len())
    });
    let n = rows.iter().flatten().count();
    assert!(n >= 10, "only {n} suite parts scaled");
}

/// Scales between rigid steps: the vertices are the composite map's images, in order.
#[test]
fn mixed_steps_compose_in_order() {
    let part = suite::mixed();
    let b = part.regenerate().expect("regen").compact();
    let (c, k) = (Point3::new(1.0, 2.0, 3.0), 1.25);
    let steps = vec![
        Step::Translate(Vec3::new(5.0, 0.0, -2.0)),
        Step::Scale { about: c, k },
        Step::Rotate {
            origin: Point3::ORIGIN,
            axis: Vec3::Z,
            angle: dmath::PI / 3.0,
        },
        Step::Scale {
            about: Point3::ORIGIN,
            k: 0.5,
        },
    ];
    let s = with(&part, steps.clone()).regenerate().expect("moved");
    assert!(check(&s).is_empty());
    let m = forge_model::body_motion(&steps);
    for v in &b.vertices {
        let w = s.vertices.iter().find(|w| w.name == v.name).expect("same names");
        let p = v.point;
        let want = m[3]
            + (m[0] - Point3::ORIGIN) * p.x
            + (m[1] - Point3::ORIGIN) * p.y
            + (m[2] - Point3::ORIGIN) * p.z;
        assert!(want.distance(w.point) < 1e-9, "{want:?} {:?}", w.point);
    }
    let (m0, m1) = (
        forge_topo::mass::mass(&b).expect("mass"),
        forge_topo::mass::mass(&s).expect("mass"),
    );
    let kk = k * 0.5;
    assert!(close(m1.volume, m0.volume * kk * kk * kk, 1e-9));
}

/// A copy scaled: a second body, the first unchanged.
#[test]
fn a_scaled_copy_is_a_new_body() {
    let part = suite::mixed();
    let b = part.regenerate().expect("regen");
    let v = forge_topo::mass::mass(&b).expect("mass").volume;
    let mut p = part.clone();
    p.features.push(Feature::Body(Box::new(BodyFeature::Move {
        steps: vec![
            Step::Scale {
                about: Point3::ORIGIN,
                k: 2.0,
            },
            Step::Translate(Vec3::new(500.0, 0.0, 0.0)),
        ],
        copy: true,
    })));
    let solids = p.regenerate_solids().expect("solids");
    assert_eq!(solids.len(), 2);
    let w: Vec<f64> = solids
        .iter()
        .map(|s| forge_topo::mass::mass(&s.body).expect("mass").volume)
        .collect();
    assert!(close(w[0], v, 1e-12) && close(w[1], 8.0 * v, 1e-9), "{w:?}");
}

#[test]
fn bad_factors_are_refused() {
    let part = suite::mixed();
    for k in [0.0, -2.0, f64::NAN, f64::INFINITY] {
        let e = with(
            &part,
            vec![Step::Scale {
                about: Point3::ORIGIN,
                k,
            }],
        )
        .regenerate();
        assert!(
            matches!(
                e,
                Err(RegenError::Body {
                    error: BodyError::Scale(_),
                    ..
                })
            ),
            "{k}: {e:?}"
        );
    }
}
