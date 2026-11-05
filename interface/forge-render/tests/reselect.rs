//! A selection outlives a rebuild that renumbers the body's faces: each selected face or edge is
//! found again by its Forge name, or dropped, never taken for the face that now has its number.
//! GPU tests: skipped where there's none.

use forge_render::gui::{Gui, Headless};
use forge_render::render::Entity;

fn ready(g: &Gui) -> bool {
    g.fg_anchors.is_some()
        && g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn open(tag: &str) -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-reselect-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let demo = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/demo"));
    for e in std::fs::read_dir(demo).unwrap().flatten() {
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    let mut h = match Headless::new((1400.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(dir.join("part_tools.fg").to_str().unwrap());
    assert!(h.until(ready), "never ready: {}", h.gui.shell.status);
    for _ in 0..3 {
        h.step();
    }
    Some(h)
}

fn set_count(h: &mut Headless, param: &str, n: f64) {
    h.gui
        .fg_apply(forge_lang::error::Op::SetParam {
            param: forge_lang::error::ParamKey::parse(param).unwrap(),
            value: forge_lang::error::Literal {
                value: n,
                dim: forge_lang::values::Dim::Count,
            },
        })
        .unwrap();
    // the rebuild landed: its names are the body's
    let before = h.gui.fg_anchors.clone().unwrap();
    assert!(h.until(|g| ready(g) && g.fg_anchors.as_ref().is_some_and(|a| !std::sync::Arc::ptr_eq(a, &before))));
    for _ in 0..3 {
        h.step();
    }
}

/// The kernel's name of what's selected, by the body that's there now.
fn names(h: &Headless) -> Vec<forge_topo::Name> {
    let a = h.gui.fg_anchors.as_ref().unwrap();
    h.gui.view.as_ref().unwrap().nav.selected.iter().map(|&(_, e)| match e {
        Entity::Face(k) => a.face_names[k as usize],
        Entity::Edge(k) => a.edge_names[k as usize],
        Entity::Vertex(_) => panic!("a vertex"),
    }).collect()
}

fn faces_named(h: &Headless, text: &str) -> Vec<u32> {
    let a = h.gui.fg_anchors.as_ref().unwrap();
    (0..a.faces.len() as u32).filter(|&k| a.faces[k as usize].as_deref() == Some(text)).collect()
}

#[test]
fn a_selected_face_is_the_same_face_after_an_earlier_feature_renumbers_them() {
    let Some(mut h) = open("renumber") else { return };
    // the slot's floor and the boss's top: faces after the hole pattern
    let slot = faces_named(&h, "extrude3.end");
    let boss = faces_named(&h, "extrude2.end");
    assert_eq!((slot.len(), boss.len()), (1, 1));
    let edge = 0u32;
    h.gui.view.as_mut().unwrap().nav.selected = vec![(0, Entity::Face(slot[0])), (0, Entity::Face(boss[0])), (0, Entity::Edge(edge))];
    let want = names(&h);
    // six holes, not four: every face after them numbered anew
    set_count(&mut h, "pattern1.n", 6.0);
    assert_ne!(faces_named(&h, "extrude3.end"), slot, "the faces weren't renumbered: no test");
    let got = names(&h);
    // each kept is the same face; none is another
    assert!(got.iter().all(|n| want.contains(n)), "{want:?} -> {got:?}");
    let sel = &h.gui.view.as_ref().unwrap().nav.selected;
    assert!(sel.contains(&(0, Entity::Face(faces_named(&h, "extrude3.end")[0]))), "{sel:?}");
    assert!(sel.contains(&(0, Entity::Face(faces_named(&h, "extrude2.end")[0]))), "{sel:?}");
}

#[test]
fn a_selected_face_that_is_gone_after_a_rebuild_is_no_longer_selected() {
    let Some(mut h) = open("gone") else { return };
    // every face of the four holes
    let holes = faces_named(&h, "pattern1.faces");
    assert!(holes.len() >= 4, "{holes:?}");
    h.gui.view.as_mut().unwrap().nav.selected = holes.iter().map(|&k| (0, Entity::Face(k))).collect();
    let want = names(&h);
    // two holes, not four: half of them gone
    set_count(&mut h, "pattern1.n", 2.0);
    let got = names(&h);
    assert!(!got.is_empty() && got.len() < want.len(), "{} -> {}", want.len(), got.len());
    assert!(got.iter().all(|n| want.contains(n)), "a face that wasn't selected is");
    // and what's kept is still a hole's
    let left = faces_named(&h, "pattern1.faces");
    let sel = &h.gui.view.as_ref().unwrap().nav.selected;
    assert!(sel.iter().all(|(_, e)| matches!(e, Entity::Face(k) if left.contains(k))), "{sel:?}");
}
