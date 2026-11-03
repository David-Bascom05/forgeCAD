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
