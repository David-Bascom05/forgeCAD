//! Every regen entry point either applies a feature or returns an error: a reference that names
//! nothing, or a source that makes nothing, is never a feature quietly left out.
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_model::{Combine, Edit, Extent, Feature, Layout, Part, RegenError, Source};
use forge_ops::{ChamferSize, Profile, Radius};
use forge_topo::{Body, Name};

fn block(x: f64, op: Combine) -> Feature {
    Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(Point3::new(x, 0.0, 0.0)),
            loops: vec![Profile::rectangle(Point2::ORIGIN, 10.0, 10.0, 0)],
        },
        extent: Extent::Distance(10.0),
        op,
    }
}

/// Every entry point on `f`, by name.
fn every_regen(f: &[Feature]) -> Vec<(&'static str, Result<Vec<Body>, RegenError>)> {
    let part = Part { features: f.to_vec() };
    let ids: Vec<u64> = (1..=f.len() as u64).collect();
    let bodies =
        |r: Result<Vec<forge_model::Solid>, RegenError>| r.map(|s| s.into_iter().map(|s| s.body).collect());
    let mut out = vec![
        ("regenerate_solids", bodies(part.regenerate_solids())),
        (
            "regenerate_solids_ids",
            bodies(part.regenerate_solids_ids(Some(&ids))),
        ),
        ("regenerate_nary_solids", bodies(part.regenerate_nary_solids(1))),
        (
            "regenerate_nary_solids_ids",
            bodies(part.regenerate_nary_solids_ids(1, Some(&ids))),
        ),
    ];
    // these hand back one body
    if !forge_model::bodies::multi(f) {
        let one = |r: Result<Body, RegenError>| r.map(|b| vec![b]);
        out.extend([
            ("regenerate", one(part.regenerate())),
            ("regenerate_ids", one(part.regenerate_ids(Some(&ids)))),
            ("regenerate_auto", one(part.regenerate_auto(1))),
            ("regenerate_nary", one(part.regenerate_nary(1))),
            ("regenerate_nary_with", one(part.regenerate_nary_with(1, false))),
        ]);
    }
    let h = forge_model::History::new(f, None);
    let mut solids = Ok(vec![]);
    for i in 0..f.len() {
        solids = solids.and_then(|s| forge_model::step_solids(h, i, s));
    }
    out.push(("step_solids", bodies(solids)));
    out
}

#[test]
fn a_reference_to_nothing_is_an_error() {
    let gone = Name::feature(999).with_str("gone");
    let neutral = Frame::WORLD;
    let modifiers = [
        Feature::Fillet {
            only: false,
            edges: vec![(gone, Radius::Constant(1.0))],
            starts: vec![],
            setbacks: vec![],
        },
        Feature::FaceFillet {
            first: vec![gone],
            second: vec![gone],
            radius: 1.0,
        },
        Feature::FullRound {
            side: gone,
            middle: gone,
            other: gone,
        },
        Feature::Chamfer {
            corner: Default::default(),
            edges: vec![gone],
            size: ChamferSize::Equal(1.0),
        },
        Feature::Shell {
            thickness: 1.0,
            outward: false,
            open: vec![gone],
        },
        Feature::Draft {
            faces: vec![gone],
            neutral,
            angle: 0.1,
        },
        Feature::Edit(Edit::Offset {
            faces: vec![gone],
            distance: 1.0,
        }),
        Feature::Edit(Edit::Delete { faces: vec![gone] }),
        Feature::Edit(Edit::Move {
            faces: vec![gone],
            motion: forge_model::Motion::Translate(Vec3::new(0.0, 0.0, 1.0)),
        }),
    ];
    for m in &modifiers {
        for mut part in [
            vec![block(0.0, Combine::New)],
            // two bodies: the path that cuts a feature down to each body's names
            vec![block(0.0, Combine::New), block(50.0, Combine::New)],
        ] {
            part.push(m.clone());
            for (entry, r) in every_regen(&part) {
                assert!(r.is_err(), "{entry} left out {m:?}");
            }
        }
    }
}

/// One real edge and one that is gone: the real one's fillet must not stand in for the feature.
#[test]
fn one_gone_reference_among_good_ones_is_an_error() {
    let first = Part {
        features: vec![block(0.0, Combine::New)],
    }
    .regenerate()
    .expect("a block");
    let edge = first.edge(first.edge_ids().next().expect("an edge")).name;
    let gone = Name::feature(999).with_str("gone");
    let fillet = Feature::Fillet {
        only: false,
        edges: vec![(edge, Radius::Constant(1.0)), (gone, Radius::Constant(1.0))],
        starts: vec![],
        setbacks: vec![],
    };
    for two in [false, true] {
        let mut f = vec![block(0.0, Combine::New)];
        if two {
            f.push(block(50.0, Combine::New));
        }
        f.push(fillet.clone());
        for (entry, r) in every_regen(&f) {
            assert!(r.is_err(), "{entry}: two={two} kept only the edge it found");
        }
    }
}

/// A pattern of a sketch-like feature has nothing to copy.
#[test]
fn a_pattern_of_nothing_is_an_error() {
    let f = vec![
        block(0.0, Combine::New),
        Feature::Plane(Frame::WORLD),
        Feature::Pattern {
            of: Source::Features(vec![1]),
            layout: Layout::Mirror {
                plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 30.0)),
            },
        },
    ];
    for (entry, r) in every_regen(&f) {
        assert!(r.is_err(), "{entry} left out the pattern");
    }
}

/// A feature that works on what it names, given nothing: refused, not a step that does nothing.
#[test]
fn a_feature_naming_nothing_is_an_error() {
    let cases = [
        Feature::Fillet {
            only: false,
            edges: vec![],
            starts: vec![],
            setbacks: vec![],
        },
        Feature::Chamfer {
            corner: Default::default(),
            edges: vec![],
            size: ChamferSize::Equal(1.0),
        },
        Feature::Draft {
            faces: vec![],
            neutral: Frame::WORLD,
            angle: 0.1,
        },
        Feature::Edit(Edit::Offset {
            faces: vec![],
            distance: 1.0,
        }),
        Feature::Edit(Edit::Delete { faces: vec![] }),
    ];
    for m in &cases {
        for mut part in [
            vec![block(0.0, Combine::New)],
            vec![block(0.0, Combine::New), block(50.0, Combine::New)],
        ] {
            part.push(m.clone());
            for (entry, r) in every_regen(&part) {
                assert!(
                    matches!(r, Err(RegenError::NoReferences { .. })),
                    "{entry}: {m:?}: {:?}",
                    r.map(|b| b.len())
                );
            }
        }
    }
}

/// A body with an open edge is what `open_at` names, with where the edge is.
#[test]
fn an_open_edge_is_named() {
    let mut b = Part {
        features: vec![block(0.0, Combine::New)],
    }
    .regenerate()
    .expect("a block");
    assert_eq!(forge_topo::check::open_at(&b), None);
    let e = b.edge_ids().next().expect("an edge");
    let first = b.edge(e).coedges[0];
    b.edges[e.index()].coedges.retain(|c| *c == first);
    let what = forge_topo::check::open_at(&b).expect("open");
    assert!(what.contains("is on 1 face(s), not 2"), "{what}");
}

/// No entry point hands back a solid that is not closed: the torus standing on a block's bottom,
/// kept, loses a face on both paths. It may build one day; it may not come out open.
#[test]
fn a_result_that_is_not_closed_is_refused() {
    let (_, part) = forge_model::adversarial::degenerate_parts()
        .into_iter()
        .find(|p| p.0 == "plane_torus/standing_tangent_top_inside/Keep")
        .expect("the part");
    for (entry, r) in every_regen(&part.features) {
        match r {
            Err(RegenError::NotClosed { feature, what }) => {
                assert_eq!(feature, 1, "{entry}");
                assert!(what.contains("face(s), not 2"), "{entry}: {what}");
            }
            Err(e) => panic!("{entry}: {e:?}"),
            Ok(bodies) => {
                for b in &bodies {
                    assert!(forge_topo::check::open_at(b).is_none(), "{entry}");
                }
            }
        }
    }
}
