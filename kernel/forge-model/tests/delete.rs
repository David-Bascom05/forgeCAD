//! BodyFeature::Delete: the bodies named removed, the others as they were; a body not there, a
//! delete of every body, and a later feature naming a deleted body are refused with typed errors.
use forge_math::{Frame, Point2, Point3};
use forge_model::{BodyError, BodyFeature, Combine, Extent, Feature, Part, Pick, RegenError};
use forge_ops::Profile;

fn block(x: f64, h: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(Point3::new(x, 0.0, 0.0)),
            loops: vec![Profile::rectangle(Point2::ORIGIN, 10.0, 10.0, 0)],
        },
        extent: Extent::Distance(h),
        op,
    }
}

fn del(bodies: Vec<usize>) -> Feature {
    Feature::Body(Box::new(BodyFeature::Delete { bodies }))
}

fn part(mut extra: Vec<Feature>) -> Part {
    let mut features = vec![block(0.0, 10.0, Combine::New), block(40.0, 20.0, Combine::New)];
    features.append(&mut extra);
    Part { features }
}

#[test]
fn deletes_the_named_bodies_only() {
    let before = part(vec![]).regenerate_solids().expect("two");
    let after = part(vec![del(vec![0])]).regenerate_solids().expect("one");
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].id, 1);
    assert_eq!(after[0].body, before[1].body, "the body left is as it was");
    let v = forge_topo::mass::mass(&after[0].body).expect("mass").volume;
    assert!((v - 2000.0).abs() < 1e-9, "{v}");
}

#[test]
fn refusals_are_typed() {
    let e = part(vec![del(vec![0, 1])]).regenerate_solids();
    assert!(
        matches!(
            e,
            Err(RegenError::Body {
                error: BodyError::DeletesAll,
                ..
            })
        ),
        "{e:?}"
    );
    let e = part(vec![del(vec![7])]).regenerate_solids();
    assert!(
        matches!(
            e,
            Err(RegenError::Body {
                error: BodyError::NoBody { id: 7 },
                ..
            })
        ),
        "{e:?}"
    );
    // a later feature on the deleted body
    let e = part(vec![
        del(vec![0]),
        Feature::Body(Box::new(BodyFeature::In {
            bodies: Pick::Some(vec![0]),
            feature: block(0.0, 5.0, Combine::Cut),
        })),
    ])
    .regenerate_solids();
    assert!(
        matches!(
            e,
            Err(RegenError::Body {
                error: BodyError::NoBody { id: 0 },
                ..
            })
        ),
        "{e:?}"
    );
    // twice
    let e = part(vec![del(vec![0]), del(vec![0])]).regenerate_solids();
    assert!(
        matches!(
            e,
            Err(RegenError::Body {
                error: BodyError::NoBody { id: 0 },
                ..
            })
        ),
        "{e:?}"
    );
}
