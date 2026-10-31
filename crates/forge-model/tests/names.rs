//! Names follow their entities through upstream edits, and both regen paths give them alike.
use forge_math::{Frame, Point2, Point3};
use forge_model::names::{Class, Edit, corpus, run, run_inserts, run_shifts, shift_corpus, work_part};
use forge_model::{Combine, Extent, Feature, Part, suite};
use forge_ops::Profile;
use forge_topo::{Body, Name};

/// Parts with pieces told apart only by their order: a thread's turns, and the slivers
/// between two seams lying almost together (docs/naming.md).
const REPEATS: [&str; 2] = ["thread", "fillet:shaft-cross-hole"];

#[test]
fn small_edits_never_move_a_name_to_another_entity() {
    let parts: Vec<(String, Part)> = corpus()
        .into_iter()
        .filter(|c| !REPEATS.contains(&c.0.as_str()))
        .collect();
    let cases = run(&parts, 11, 100, false, 1, 30);
    let (mut kept, mut lost, mut used, mut shifted) = (0, 0, 0, 0);
    let mut bad = Vec::new();
    // inserts, deletes and swaps too: features keep their ids, so nothing renumbers
    for c in &cases {
        shifted += c.edit.shifts(c.before) as usize;
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
    assert!(
        used >= 50 && shifted >= 10,
        "only {used} edits judged, {shifted} shifting"
    );
    assert!(bad.is_empty(), "{bad:#?}");
    // names lost though their entity lives: where flush faces part or meet (docs/naming.md)
    assert!(lost * 25 <= kept, "{lost} lost, {kept} kept");
}

/// A work plane's reference and the extrude ending on it keep their features through inserts,
/// deletes and swaps above them, and the plane moves with a resize.
#[test]
fn work_features_follow_their_ids() {
    let cases = run(&[("work".into(), work_part())], 5, 60, false, 1, 30);
    let shifted = cases.iter().filter(|c| c.edit.shifts(c.before)).count();
    let bad: Vec<_> = cases
        .iter()
        .flat_map(|c| c.outcome.findings.iter().map(move |f| (c, f)))
        .filter(|(_, f)| matches!(f.class, Class::Wrong | Class::Duplicate))
        .map(|(c, f)| format!("{:?}: {f:?}", c.edit))
        .collect();
    assert!(
        cases.len() >= 30 && shifted >= 5,
        "{} judged, {shifted} shifting",
        cases.len()
    );
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Inserts, deletes and swaps above finishing features, face fillets, full rounds, the
/// features of several bodies and a work plane: every feature keeps its id, so an insert
/// (which makes nothing) keeps every name, and nothing ever names another entity.
#[test]
fn shifting_edits_rename_nothing() {
    let cases = run_shifts(&shift_corpus());
    let mut shifted = std::collections::BTreeMap::new();
    let mut bad = Vec::new();
    for c in cases.iter().filter(|c| c.edit.shifts(c.before)) {
        *shifted
            .entry(c.part.split(':').next().unwrap_or("").to_string())
            .or_insert(0) += 1;
        let insert = matches!(c.edit, Edit::Insert { .. });
        for f in &c.outcome.findings {
            if insert || matches!(f.class, Class::Wrong | Class::Duplicate) {
                bad.push(format!("{} {:?}: {f:?}", c.part, c.edit));
            }
        }
    }
    for (group, least) in [("bodies", 20), ("finish", 10), ("variant", 2), ("work", 2)] {
        assert!(shifted.get(group).is_some_and(|&n| n >= least), "{shifted:?}");
    }
    assert!(bad.is_empty(), "{} {bad:#?}", bad.len());
}

/// Holes, pockets, slots and bosses put in above finishing features and several bodies split
/// faces and edges where they reach: every entity they leave where it was keeps its name, and
/// nothing ever names another entity.
#[test]
fn inserts_that_split_faces_rename_nothing_they_miss() {
    let corpus = shift_corpus();
    let (mut used, mut bad) = (0, Vec::new());
    for seed in 1..=3 {
        let (cases, _) = run_inserts(&corpus, seed, 4);
        used += cases.len();
        for c in &cases {
            for f in c.outcome.findings.iter().filter(|f| f.class != Class::Jumped) {
                bad.push(format!("{} @{} {:?}: {f:?}", c.part, c.before, c.edit));
            }
        }
    }
    assert!(used >= 600, "only {used} inserts judged");
    assert!(bad.is_empty(), "{} {bad:#?}", bad.len());
}

fn names(b: &Body) -> Vec<Name> {
    let mut out: Vec<Name> = b.face_ids().map(|f| b.face(f).name).collect();
    out.extend(b.edge_ids().map(|e| b.edge(e).name));
    // (vertices that still end a live edge: the n-ary result keeps tombstones until compacted)
    let mut vs: Vec<_> = b
        .edge_ids()
        .flat_map(|e| [b.edge(e).start, b.edge(e).end])
        .collect();
    vs.sort_unstable_by_key(|v| v.index());
    vs.dedup();
    out.extend(vs.into_iter().map(|v| b.vertex(v).name));
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
    // sequential regen names those from the pieces earlier features left (docs/naming.md)
    for name in ["sweep", "loft", "coil", "text", "upto", "upto-flat"] {
        let part = suite::part(name).expect("a suite part");
        let seq = part.regenerate().expect("sequential regen");
        let nary = part.regenerate_nary(1).expect("n-ary regen");
        assert_eq!(names(&seq), names(&nary), "{name}");
    }
}

#[test]
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
    let (top, other) = (Name::feature(2).with_str("end"), Name::feature(3).with_str("end"));
    let both = part.regenerate().expect("regen");
    assert!(both.face_by_name(top).is_some() && both.face_by_name(other).is_some());
    part.features.remove(1);
    // the second boss keeps its id, 3: its top keeps its name, and the first's is gone
    let one = part.regenerate_ids(Some(&[1, 3])).expect("regen");
    assert!(one.face_by_name(top).is_none());
    assert_eq!(face(&one, other), face(&both, other));
    // put back in, after the other: every name where it was
    part.features.push(block(5.0, 5.0, 5.0, 5.0, Combine::Add));
    let moved = part.regenerate_ids(Some(&[1, 3, 2])).expect("regen");
    assert_eq!(face(&moved, top), face(&both, top));
    assert_eq!(face(&moved, other), face(&both, other));
}

/// Where a named face lies: its box.
fn face(b: &Body, n: Name) -> Option<String> {
    b.face_by_name(n).map(|f| format!("{:?}", b.face_box(f)))
}

/// On a one-body part every regen entry point names alike: seq whole, as solids or a feature at a
/// time; n-ary whole, as solids or with the shapes given; and seq and n-ary alike where they agree
/// (as above).
#[test]
fn every_regen_entry_point_names_alike() {
    for (name, part) in suite::parts() {
        let one = |s: Vec<forge_model::Solid>| {
            assert_eq!(s.len(), 1, "{name}");
            names(&s[0].body.compact())
        };
        let seq = names(&part.regenerate().expect("sequential regen"));
        assert_eq!(seq, one(part.regenerate_solids().expect("seq solids")), "{name}");
        let h = forge_model::History::new(&part.features, None);
        let mut solids = Vec::new();
        for i in 0..part.features.len() {
            solids = forge_model::step_solids(h, i, solids).expect("a step");
        }
        // a step leaves finishing to its caller
        for s in &mut solids {
            s.body = forge_model::finish(std::mem::take(&mut s.body));
        }
        assert_eq!(seq, one(solids), "{name}");
        let nary = names(&part.regenerate_nary(1).expect("n-ary regen"));
        assert_eq!(
            nary,
            one(part.regenerate_nary_solids(1).expect("n-ary solids")),
            "{name}"
        );
        assert_eq!(
            nary,
            names(&part.regenerate_nary_with(1, true).expect("n-ary")),
            "{name}"
        );
        if ["sweep", "loft"].contains(&name) {
            assert_eq!(seq, nary, "{name}");
        }
    }
}
