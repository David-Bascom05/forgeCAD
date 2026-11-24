//! The Render Studio's Model tab (shell/studio_model.rs, research/render/studio-model/), headless:
//! an assembly's parts listed once each with their own materials' looks, the brush applied by a
//! click in the view, many parts given one look at once, the filter. `STUDIO_MODEL_SHOTS=DIR`
//! keeps pictures of the tab.

use forge_render::gui::{Gui, Headless};
use forge_render::render::Entity;
use forge_render::shell::studio_model::{Row, apply, rows, select};

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

fn studio(file: &str) -> Option<Headless> {
    let mut h = match Headless::new((1400.0, 860.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui
        .open(&format!("{}/../../examples/{file}", env!("CARGO_MANIFEST_DIR")));
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    h.gui.env_asked = Some("render".into());
    for _ in 0..30 {
        h.step();
        if !h.gui.shell.studio.names.is_empty() {
            break;
        }
    }
    assert!(!h.gui.shell.studio.names.is_empty(), "the Studio found no parts");
    for _ in 0..5 {
        h.step();
    }
    assert_eq!(
        h.gui.shell.env.map(|e| e.key()),
        Some("render"),
        "{}",
        h.gui.shell.status
    );
    Some(h)
}

fn shot(h: &Headless, name: &str) {
    if let Ok(dir) = std::env::var("STUDIO_MODEL_SHOTS") {
        std::fs::write(format!("{dir}/{name}.png"), h.png().unwrap()).unwrap();
    }
}

#[test]
fn a_click_in_the_view_paints_the_part_with_the_brush() {
    let Some(mut h) = studio("robot/arm2.fg") else {
        return;
    };
    let st = &h.gui.shell.studio;
    assert_eq!(
        st.names,
        ["base", "finger", "link"],
        "each part once, by its file"
    );
    assert!(st.looks.iter().all(Option::is_none));
    // the brush, painting: a click in the view selects; the Studio takes it as the brush's
    h.gui.shell.studio.brush = Some("Brass".into());
    h.gui.shell.studio.painting = true;
    let v = h.gui.view.as_mut().unwrap();
    let part = v.scene.placements[0].part as usize;
    let version = h.gui.shell.studio.version;
    v.nav.selected.push((0, Entity::Face(0)));
    h.step();
    let st = &h.gui.shell.studio;
    assert_eq!(st.looks[part].as_deref(), Some("Brass"));
    assert!(st.looks.iter().filter(|l| l.is_some()).count() == 1);
    assert!(st.version > version, "the picture is made again");
    assert!(
        h.gui.view.as_ref().unwrap().nav.selected.is_empty(),
        "the click is spent"
    );
    assert!(st.said.ends_with(": Brass"), "{}", st.said);
    h.step();
    shot(&h, "painted");
    // not painting: a click selects as ever and paints nothing
    h.gui.shell.studio.painting = false;
    h.gui
        .view
        .as_mut()
        .unwrap()
        .nav
        .selected
        .push((1, Entity::Face(0)));
    h.step();
    assert_eq!(h.gui.shell.studio.looks.iter().filter(|l| l.is_some()).count(), 1);
}

#[test]
fn many_parts_take_one_look_and_the_filter_finds_them() {
    let Some(mut h) = studio("robot/arm2.fg") else {
        return;
    };
    let st = &mut h.gui.shell.studio;
    let shown: Vec<usize> = rows(st)
        .iter()
        .filter_map(|r| match r {
            Row::Part(k) => Some(*k),
            _ => None,
        })
        .collect();
    assert_eq!(shown.len(), 3);
    select(st, shown[0], false, false, &shown);
    select(st, shown[2], false, true, &shown);
    let to = st.selected.clone();
    assert_eq!(to.len(), 3);
    apply(st, &to, Some("Paint, machine gloss"));
    assert!(
        st.looks
            .iter()
            .all(|l| l.as_deref() == Some("Paint, machine gloss"))
    );
    st.filter = "fing".into();
    assert_eq!(rows(st), vec![Row::Part(1)]);
    st.filter.clear();
    // the picture is made again with them, however they were changed: the samples start over
    for _ in 0..40 {
        h.step();
        if h.gui.shell.studio.samples >= 4 {
            break;
        }
    }
    let before = h.gui.shell.studio.samples;
    assert!(before >= 4, "{before}");
    apply(&mut h.gui.shell.studio, &[0], Some("Brass"));
    h.step();
    let after = h.gui.shell.studio.samples;
    assert!(
        after <= 2,
        "the picture didn't start again: {after} samples after {before}"
    );
    shot(&h, "many");
}

/// Each part's own material gives its default look, not the assembly's: the rack's parts are
/// steel, nylon, rubber, ... (each `props(material:)`), and the Studio reads each.
#[test]
fn an_assemblys_parts_keep_their_own_materials() {
    let Some(h) = studio("rack18u/asm/rack.fg") else {
        return;
    };
    let st = &h.gui.shell.studio;
    assert!(st.names.len() > 50, "{}", st.names.len());
    let mut own: Vec<&str> = st.own.iter().flatten().map(String::as_str).collect();
    own.sort();
    own.dedup();
    assert!(own.len() >= 2, "one material for all: {own:?}");
    let groups: std::collections::BTreeSet<&str> = st.groups.iter().map(String::as_str).collect();
    assert!(groups.contains("frame") && groups.contains("hw"), "{groups:?}");
    assert!(st.counts.iter().sum::<usize>() > st.names.len());
    shot(&h, "rack");
}
