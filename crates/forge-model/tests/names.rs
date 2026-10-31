//! Names follow their entities through upstream edits, and both regen paths give them alike.
use forge_math::{Frame, Point2, Point3};
use forge_model::names::{Class, corpus, run};
use forge_model::{Combine, Extent, Feature, Part, suite};
use forge_ops::Profile;
use forge_topo::{Body, Name};

/// Parts where a closed intersection curve's pieces still take their names from where the
/// surfaces' seams cut it: a tiny move swaps them (see the shaft-post repro below).
const SEAMED: [&str; 6] = [
    "fillet:shaft-post",
    "fillet:shaft-post-periodic",
    "fillet:shaft-cross-hole",
    "fillet:boss-foot-periodic",
    "fillet:boss-rim-periodic",
    "fillet:pit-five",
];

#[test]
fn small_edits_never_move_a_name_to_another_entity() {
    let parts: Vec<(String, Part)> = corpus()
        .into_iter()
        .filter(|c| !SEAMED.contains(&c.0.as_str()))
        .collect();
    let cases = run(&parts, 11, 80, false, 1, 30);
    let (mut kept, mut lost, mut used) = (0, 0, 0);
    let mut bad = Vec::new();
    // edits that renumber features rename everything below them (ids are positions)
    for c in cases.iter().filter(|c| !c.edit.shifts(c.before)) {
        used += 1;
        kept += c.outcome.kept;
        for f in &c.outcome.findings {
            match f.class {
                Class::Wrong | Class::Duplicate => bad.push(format!("{} {:?}: {f:?}", c.part, c.edit)),
                Class::Jumped => {}
                _ => lost += 1,
            }
        }
    }
    assert!(used >= 50, "only {used} edits judged");
    assert!(bad.is_empty(), "{bad:#?}");
    // names lost though their entity lives: where flush faces part or meet (see names.rs)
    assert!(lost * 25 <= kept, "{lost} lost, {kept} kept");
}

fn names(b: &Body) -> Vec<Name> {
    let mut out: Vec<Name> = b.face_ids().map(|f| b.face(f).name).collect();
    out.extend(b.edge_ids().map(|e| b.edge(e).name));
    out.extend(b.vertex_ids().map(|v| b.vertex(v).name));
    out.sort();
    out
}

#[test]
fn a_face_cut_into_one_piece_keeps_its_name_on_both_paths() {
    let ex = |o: Point3, l, d, op| Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(o),
            loops: vec![l],
        },
        extent: Extent::Distance(d),
        op,
    };
    let top = Name::feature(1).with_str("end");
    for (z, d) in [(-1.0, 22.0), (12.0, 9.0)] {
        let part = Part {
            features: vec![
                ex(
                    Point3::ORIGIN,
                    Profile::rectangle(Point2::ORIGIN, 20.0, 20.0, 0),
                    20.0,
                    Combine::New,
                ),
                ex(
                    Point3::new(10.0, 10.0, z),
                    Profile::circle(Point2::ORIGIN, 3.0, 0),
                    d,
                    Combine::Cut,
                ),
                Feature::Shell {
                    thickness: 1.0,
                    outward: false,
                    open: vec![top],
                },
            ],
        };
        let seq = part.regenerate().expect("sequential regen");
        let nary = part.regenerate_nary(1).expect("n-ary regen");
        assert_eq!(names(&seq), names(&nary));
    }
}

#[test]
fn both_regen_paths_name_alike() {
    // the other suite parts still differ where a face is split into several kept pieces: the
    // sequential regen names those from the pieces earlier features left (see names.rs)
    for name in ["sweep", "loft"] {
        let part = suite::part(name).expect("a suite part");
        let seq = part.regenerate().expect("sequential regen");
        let nary = part.regenerate_nary(1).expect("n-ary regen");
        assert_eq!(names(&seq), names(&nary), "{name}");
    }
}

#[test]
#[ignore = "feature ids are their positions: taking one out hands its names to the next (a proposal)"]
fn taking_a_feature_out_never_hands_its_names_to_the_next() {
    let block = |x: f64, w: f64, z: f64, h: f64, op| Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(Point3::new(x, 0.0, z)),
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, 10.0, 0)],
        },
        extent: Extent::Distance(h),
        op,
    };
    let mut part = Part {
        features: vec![
            block(0.0, 40.0, 0.0, 5.0, Combine::New),
            block(5.0, 5.0, 5.0, 5.0, Combine::Add),
            block(25.0, 5.0, 5.0, 8.0, Combine::Add),
        ],
    };
    let top = Name::feature(2).with_str("end");
    assert!(part.regenerate().expect("regen").face_by_name(top).is_some());
    part.features.remove(1);
    // the second boss's top now carries the first's name
    assert!(part.regenerate().expect("regen").face_by_name(top).is_none());
}
