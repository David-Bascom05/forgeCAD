//! A part's topology must not follow its feature ids: the same features under other ids (here
//! every line moved down by params put in above) build the same faces, edges and vertices. The
//! frame's axle slot once kept the bore's seam where the least name put it: 77 edges or 76 by id.
use forge_agent::text;

fn counts(src: &str) -> (usize, usize, usize) {
    let part = text::parse(src).unwrap_or_else(|(l, e)| panic!("line {l}: {e}"));
    let solids = part.regenerate_solids().expect("builds");
    let b = &solids[0].body;
    (b.face_ids().count(), b.edge_ids().count(), b.vertex_ids().count())
}

#[test]
fn the_slot_cut_keeps_one_seam_whatever_the_ids() {
    let src = include_str!("parts/frame-slot-ids.fc");
    let all: Vec<_> = (0..4)
        .map(|k| {
            let above: String = (0..k).map(|i| format!("shift{i} = 1\n")).collect();
            counts(&format!("{above}{src}"))
        })
        .collect();
    assert!(all.iter().all(|c| *c == all[0]), "{all:?}");
    assert_eq!(all[0], (30, 76, 48));
}

/// Two names of one edge (pieces a seam split under other ids, one edge here) in one fillet with
/// one radius: the edge is rounded once, not refused as listed twice.
#[test]
fn a_fillet_naming_one_edge_twice_rounds_it_once() {
    use forge_model::Feature;
    use forge_ops::Radius;
    let mut part = text::parse("b: extrude 10 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 5\n").expect("parses");
    let body = part.regenerate_solids().expect("builds")[0].body.clone();
    let rim = body
        .edge_ids()
        .find(|&e| matches!(body.edge(e).curve, forge_geom::Curve::Circle(_)))
        .expect("a rim");
    let n = body.edge(rim).name;
    let r = Radius::Constant(1.0);
    part.features.push(Feature::Fillet {
        edges: vec![(n, r.clone()), (n, r)],
        starts: vec![],
        setbacks: vec![],
        only: false,
    });
    let once = part.regenerate_solids().expect("rounds").remove(0).body;
    part.features.pop();
    part.features.push(Feature::Fillet {
        edges: vec![(n, Radius::Constant(1.0))],
        starts: vec![],
        setbacks: vec![],
        only: false,
    });
    let single = part.regenerate_solids().expect("rounds").remove(0).body;
    assert_eq!(once.face_ids().count(), single.face_ids().count());
}

/// A cylinder notched below and whole above: its pieces join into one face, and the seam it keeps
/// is a ruling, under any ids; never an arc left across the face (a slit a fillet along the notch
/// would run into).
#[test]
fn a_rejoined_cylinder_keeps_a_ruling_as_its_seam() {
    let src = "extrude 3 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 6\n\
               extrude add 5 z 0 0 1 x 1 0 0 at 0 0 3 circle 0 0 6\n\
               extrude cut 3 z 0 0 1 x 1 0 0 at 0 0 0 rect -2 -7 4 3\n";
    let part = text::parse(src).expect("parses");
    let mut seen = vec![];
    for k in 0..6u64 {
        let ids: Vec<u64> = (0..part.features.len() as u64)
            .map(|i| 1 + i * (k + 1) + 7 * k)
            .collect();
        let b = part
            .regenerate_solids_ids(Some(&ids))
            .expect("builds")
            .remove(0)
            .body;
        for e in b.edge_ids() {
            let x = b.edge(e);
            let faces: Vec<_> = x.coedges.iter().map(|&c| b.coedge_face(c)).collect();
            if faces.len() == 2 && faces[0] == faces[1] {
                assert!(
                    matches!(x.curve, forge_geom::Curve::Line(_)),
                    "ids {ids:?}: a seam that is no ruling"
                );
            }
        }
        seen.push((b.face_ids().count(), b.edge_ids().count()));
    }
    assert!(seen.iter().all(|s| *s == seen[0]), "{seen:?}");
}
