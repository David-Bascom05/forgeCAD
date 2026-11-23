//! A several-region extrude (`Feature::Regions`) builds each region with the extrude's own code:
//! one region through it is the plain extrude, bit for bit (vertex points, edge ranges, faces);
//! regions combine with the part as each would alone.
use forge_math::hash::BitHasher;
use forge_math::{Frame, Point2};
use forge_model::regions::Regions;
use forge_model::{Combine, Extent, Feature, Part};
use forge_ops::{Profile, ProfileLoop, Segment};

fn square(x: f64, y: f64, w: f64, id: u64) -> Profile {
    let c = [
        Point2::new(x, y),
        Point2::new(x + w, y),
        Point2::new(x + w, y + w),
        Point2::new(x, y + w),
    ];
    Profile {
        plane: Frame::WORLD,
        loops: vec![ProfileLoop::new(
            (0..4)
                .map(|k| {
                    (
                        Segment::Line {
                            a: c[k],
                            b: c[(k + 1) % 4],
                        },
                        id + k as u64,
                    )
                })
                .collect(),
        )],
    }
}

fn bits(features: Vec<Feature>) -> String {
    let b = Part { features }.regenerate().expect("builds");
    let mut h = BitHasher::new();
    for v in &b.vertices {
        h.p3(v.point);
    }
    for e in &b.edges {
        h.f64(e.range.lo);
        h.f64(e.range.hi);
    }
    h.u64(b.faces.len() as u64);
    format!("{:016x}", h.finish())
}

#[test]
fn one_region_is_the_plain_extrude() {
    let base = Feature::Extrude {
        profile: square(0.0, 0.0, 40.0, 1),
        extent: Extent::Distance(5.0),
        op: Combine::New,
    };
    for (extent, op) in [
        (Extent::Distance(3.0), Combine::Add),
        (Extent::Distance(-2.0), Combine::Cut),
        (Extent::Symmetric(4.0), Combine::Add),
        (
            Extent::TwoSided {
                forward: 2.0,
                back: 1.0,
            },
            Combine::Cut,
        ),
    ] {
        let p = square(10.0, 10.0, 8.0, 100);
        let plain = Feature::Extrude {
            profile: p.clone(),
            extent,
            op,
        };
        let one = Feature::Regions(Box::new(Regions {
            profiles: vec![p],
            extent,
            op,
        }));
        assert_eq!(
            bits(vec![base.clone(), plain]),
            bits(vec![base.clone(), one]),
            "{extent:?} {op:?}"
        );
    }
    // a part's first, new body
    let p = square(0.0, 0.0, 8.0, 100);
    let plain = Feature::Extrude {
        profile: p.clone(),
        extent: Extent::Distance(3.0),
        op: Combine::New,
    };
    let one = Feature::Regions(Box::new(Regions {
        profiles: vec![p],
        extent: Extent::Distance(3.0),
        op: Combine::New,
    }));
    assert_eq!(bits(vec![plain]), bits(vec![one]));
}

#[test]
fn reaching_extents_are_refused() {
    let base = Feature::Extrude {
        profile: square(0.0, 0.0, 40.0, 1),
        extent: Extent::Distance(5.0),
        op: Combine::New,
    };
    let r = Feature::Regions(Box::new(Regions {
        profiles: vec![square(10.0, 10.0, 8.0, 100), square(25.0, 10.0, 8.0, 200)],
        extent: Extent::ToNext { reverse: false },
        op: Combine::Add,
    }));
    let e = Part {
        features: vec![base, r],
    }
    .regenerate()
    .unwrap_err();
    assert!(format!("{e:?}").contains("Regions"), "{e:?}");
}
