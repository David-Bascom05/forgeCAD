//! The Render Studio's Model tab, its appearance part (research/render/studio-model/): an
//! appearance brush chosen from a searchable palette and applied by clicking parts in the view; the
//! parts as a tree (grouped by folder, collapsed, filterable, virtualised) with each one's look as
//! a chip, multi-select to apply one look to many; the overrides alone. The GUI applies a click in
//! the view (`gui_studio.rs`); the rest is here, its logic in plain functions the tests drive.

use super::Shell;
use super::studio_env::StudioUi;
use crate::studio::appearance::{Appearance, Appearances};
use egui::{Color32, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};

/// A row of the parts' tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// A folder's parts: its name, its parts, open.
    Group(String, Vec<usize>, bool),
    /// A part, by its index in the view's parts.
    Part(usize),
}

/// Whether a part's name (or its group) answers the filter `f`, already lower case.
fn matches(st: &StudioUi, k: usize, f: &str) -> bool {
    f.is_empty()
        || st.names.get(k).is_some_and(|n| n.to_lowercase().contains(f))
        || st.groups.get(k).is_some_and(|g| g.to_lowercase().contains(f))
}

/// The tree's rows as they show: the groups (open while filtering), their parts if open; with no
/// groups, the parts alone. One pass over the parts.
pub fn rows(st: &StudioUi) -> Vec<Row> {
    let f = st.filter.trim().to_lowercase();
    let n = st.names.len();
    let group = |k: usize| st.groups.get(k).map_or("", String::as_str);
    let mut by: std::collections::BTreeMap<&str, Vec<usize>> = Default::default();
    for k in (0..n).filter(|&k| matches(st, k, &f)) {
        by.entry(group(k)).or_default().push(k);
    }
    if by.keys().all(|g| g.is_empty()) {
        return by.into_values().flatten().map(Row::Part).collect();
    }
    let mut out = Vec::new();
    for (g, members) in by {
        let open = !f.is_empty() || st.open.contains(g);
        out.push(Row::Group(g.to_string(), members.clone(), open));
        if open {
            out.extend(members.into_iter().map(Row::Part));
        }
    }
    out
}

/// A click on part `k`: alone, Ctrl toggles it in, Shift takes the run from the last one clicked
/// over the parts shown.
pub fn select(st: &mut StudioUi, k: usize, ctrl: bool, shift: bool, shown: &[usize]) {
    if shift && let Some(a) = st.anchor {
        let (i, j) = (
            shown.iter().position(|&x| x == a),
            shown.iter().position(|&x| x == k),
        );
        if let (Some(i), Some(j)) = (i, j) {
            let (i, j) = (i.min(j), i.max(j));
            for &x in &shown[i..=j] {
                if !st.selected.contains(&x) {
                    st.selected.push(x);
                }
            }
            return;
        }
    }
    if ctrl {
        if let Some(i) = st.selected.iter().position(|&x| x == k) {
            st.selected.remove(i);
        } else {
            st.selected.push(k);
        }
    } else {
        st.selected = vec![k];
    }
    st.anchor = Some(k);
}

/// Gives `parts` the look (None: each its material's own).
pub fn apply(st: &mut StudioUi, parts: &[usize], look: Option<&str>) {
    for &k in parts {
        if st.looks.len() <= k {
            st.looks.resize(k + 1, None);
        }
        st.looks[k] = look.map(str::to_string);
    }
}

/// The parts a row's look goes to: the selection when the row is in it, else the row's own.
fn targets(st: &StudioUi, row: &Row) -> Vec<usize> {
    let own = match row {
        Row::Group(_, m, _) => m.clone(),
        Row::Part(k) => vec![*k],
    };
    if own.iter().all(|k| st.selected.contains(k)) && !st.selected.is_empty() {
        st.selected.clone()
    } else {
        own
    }
}

/// The parts with a look of their own, in order.
pub fn overrides(st: &StudioUi) -> Vec<usize> {
    (0..st.names.len())
        .filter(|&k| st.looks.get(k).is_some_and(Option::is_some))
        .collect()
}

/// An appearance's swatch colour: its base (albedo, or a metal's F0) in sRGB.
pub fn swatch(a: &Appearance) -> Color32 {
    let enc = |c: f32| {
        let c = c.clamp(0.0, 1.0);
        let s = if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    };
    // a metal shows its surroundings, not white: its F0 scaled to what a studio's light gives
    // back on average, so steel, aluminium and chrome tell apart and keep their hue
    let k = if a.bsdf.metal > 0.5 { 0.62 } else { 1.0 };
    let b = a.bsdf.base;
    Color32::from_rgb(enc(b.x * k), enc(b.y * k), enc(b.z * k))
}

/// The appearances the palette shows for the search: by family, in the library's order.
pub fn palette<'a>(lib: &'a Appearances, find: &str) -> Vec<(String, Vec<&'a Appearance>)> {
    let f = find.trim().to_lowercase();
    let mut out: Vec<(String, Vec<&Appearance>)> = Vec::new();
    for a in lib.all() {
        if !f.is_empty() && !a.name.to_lowercase().contains(&f) && !a.family.to_lowercase().contains(&f) {
            continue;
        }
        match out.iter_mut().find(|(fam, _)| *fam == a.family) {
            Some((_, v)) => v.push(a),
            None => out.push((a.family.clone(), vec![a])),
        }
    }
    out
}

const CHIP: f32 = 16.0;
const ROW_H: f32 = 24.0;
/// The families open in the palette without "more": what parts are mostly made of.
const FIRST: [&str; 2] = ["metal", "paint"];

/// A look's chip: the appearance's colour, lit a little; none (the material's own) a hollow ring.
fn chip(ui: &Ui, at: egui::Pos2, r: f32, look: Option<&Appearance>, ink2: Color32) {
    let p = ui.painter();
    match look {
        Some(a) => {
            let c = swatch(a);
            if a.bsdf.metal > 0.5 {
                // a metal reads by its contrast: a dark rim, its colour where the light is
                p.circle_filled(at, r, c.gamma_multiply(0.45).to_opaque());
                p.circle_filled(at + vec2(-r * 0.12, -r * 0.12), r * 0.78, c);
            } else {
                p.circle_filled(at, r, c);
            }
            p.circle_filled(
                at + vec2(-r * 0.3, -r * 0.35),
                r * 0.35,
                Color32::from_white_alpha(46),
            );
            p.circle_stroke(at, r, Stroke::new(1.0, Color32::from_black_alpha(110)));
        }
        None => {
            p.circle_stroke(at, r - 0.5, Stroke::new(1.2, ink2));
        }
    }
}

/// A clickable, draggable swatch; true when clicked. Dragged, it carries the appearance's name.
fn swatch_button(ui: &mut Ui, a: &Appearance, on: bool, accent: Color32) -> bool {
    let id = ui.scope_id().with(("swatch", &a.name));
    let r = ui
        .dnd_drag_source(id, a.name.clone(), |ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
            chip(ui, rect.center(), 10.0, Some(a), Color32::GRAY);
            if on {
                ui.painter()
                    .circle_stroke(rect.center(), 12.5, Stroke::new(2.0, accent));
            }
        })
        .response;
    let r = ui.interact(r.rect, id.with("click"), Sense::click());
    let r = r.on_hover_text(format!("{} ({})\n{}", a.name, a.family, a.source));
    r.clicked()
}

/// Swatches in rows as many as fit the width; the one clicked, if any.
fn swatches(ui: &mut Ui, list: &[&Appearance], on: Option<&str>, accent: Color32) -> Option<String> {
    let per = (((ui.available_width() + 6.0) / 28.0).floor() as usize).max(1);
    let mut got = None;
    for chunk in list.chunks(per) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for a in chunk {
                if swatch_button(ui, a, on == Some(a.name.as_str()), accent) {
                    got = Some(a.name.clone());
                }
            }
        });
    }
    got
}

/// The palette: a search, then the families' swatches; a click (or `pick`) chooses one.
fn palette_ui(ui: &mut Ui, s: &Shell, st: &mut StudioUi, lib: &Appearances, pick: &mut Option<String>) {
    ui.add(
        egui::TextEdit::singleline(&mut st.find)
            .hint_text("Find an appearance")
            .desired_width(ui.available_width()),
    );
    let fams = palette(lib, &st.find);
    let searching = !st.find.trim().is_empty();
    let mut hidden = 0;
    for (fam, list) in &fams {
        if !searching && !st.more && !FIRST.contains(&fam.as_str()) {
            hidden += list.len();
            continue;
        }
        ui.label(RichText::new(fam).small().color(s.theme.ink2));
        if let Some(n) = swatches(ui, list, st.brush.as_deref(), s.theme.focus) {
            *pick = Some(n);
        }
    }
    if fams.is_empty() {
        ui.label(RichText::new("No appearance matches").small().color(s.theme.ink2));
    }
    if hidden > 0
        && ui
            .link(RichText::new(format!("Show {hidden} more")).small())
            .clicked()
    {
        st.more = true;
    } else if st.more && !searching && ui.link(RichText::new("Show fewer").small()).clicked() {
        st.more = false;
    }
}

/// The appearance part of the Model tab. Returns nothing: it changes `st`, whose looks the GUI
/// reads.
pub(super) fn ui(ui: &mut Ui, s: &Shell, st: &mut StudioUi, lib: &Appearances) {
    let t = s.theme;
    // the brush
    super::studio_env::head(ui, s, "Appearance");
    let brush = st.brush.as_deref().and_then(|n| lib.find(n));
    let frame = egui::Frame::new()
        .fill(t.raised)
        .stroke(Stroke::new(1.0, t.line))
        .corner_radius(6.0)
        .inner_margin(vec2(8.0, 6.0));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(CHIP + 8.0, CHIP + 4.0), Sense::hover());
            chip(ui, r.center(), CHIP / 2.0 + 1.0, brush, t.ink2);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(brush.map_or("No appearance chosen", |a| a.name.as_str())).color(t.ink),
                );
                ui.label(
                    RichText::new(if st.painting {
                        "click parts in the view to apply · Esc stops"
                    } else if brush.is_some() {
                        "Paint, then click parts in the view"
                    } else {
                        "choose one below"
                    })
                    .small()
                    .color(t.ink2),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let r = ui.add_enabled(
                    brush.is_some(),
                    egui::Button::new(if st.painting { "Stop" } else { "Paint" }).selected(st.painting),
                );
                if r.clicked() {
                    st.painting = !st.painting;
                }
            });
        });
    });
    let mut pick = None;
    palette_ui(ui, s, st, lib, &mut pick);
    if let Some(p) = pick {
        st.brush = Some(p);
        st.painting = true;
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        st.painting = false;
    }

    // the parts
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Parts").font(super::theme::semi(13.0)).color(t.ink));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let occ: usize = st.counts.iter().sum();
            let n = st.names.len();
            let words = if occ > n {
                format!("{n} · {occ} in the assembly")
            } else {
                format!("{n}")
            };
            ui.label(RichText::new(words).small().color(t.ink2));
        });
    });
    if st.names.len() > 1 {
        ui.add(
            egui::TextEdit::singleline(&mut st.filter)
                .hint_text("Filter parts")
                .desired_width(ui.available_width()),
        );
    }
    let rows = rows(st);
    let shown: Vec<usize> = rows
        .iter()
        .filter_map(|r| match r {
            Row::Part(k) => Some(*k),
            _ => None,
        })
        .collect();
    let mut act: Vec<(Row, Option<Option<String>>)> = Vec::new();
    let mut clicks: Vec<(usize, egui::Modifiers)> = Vec::new();
    let mut toggles: Vec<String> = Vec::new();
    let height = (rows.len() as f32 * ROW_H).min(ROW_H * 12.0);
    egui::ScrollArea::vertical()
        .id_salt("studio-parts")
        .max_height(height.max(ROW_H))
        .auto_shrink([false, true])
        .show_rows(ui, ROW_H, rows.len(), |ui, range| {
            for row in &rows[range] {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
                let p = ui.painter();
                let (indent, name, count, looks, sel) = match row {
                    Row::Group(g, m, _) => (
                        0.0,
                        g.clone(),
                        m.len(),
                        m.iter()
                            .map(|&k| st.looks.get(k).cloned().flatten())
                            .collect::<Vec<_>>(),
                        !m.is_empty() && m.iter().all(|k| st.selected.contains(k)),
                    ),
                    Row::Part(k) => (
                        if st.groups.get(*k).is_some_and(|g| !g.is_empty()) {
                            18.0
                        } else {
                            0.0
                        },
                        st.names[*k].clone(),
                        st.counts.get(*k).copied().unwrap_or(1),
                        vec![st.looks.get(*k).cloned().flatten()],
                        st.selected.contains(k),
                    ),
                };
                if sel {
                    p.rect_filled(r, 3.0, t.on);
                } else if resp.hovered() {
                    p.rect_filled(r, 3.0, t.hover);
                }
                let mut x = r.left() + 4.0 + indent;
                if let Row::Group(g, _, open) = row {
                    let a = Rect::from_center_size(pos2(x + 6.0, r.center().y), vec2(16.0, ROW_H));
                    let ar = ui.interact(a, ui.scope_id().with(("open", g)), Sense::click());
                    super::fg::disclosure(
                        ui.painter(),
                        a.center(),
                        *open,
                        if ar.hovered() { t.ink } else { t.ink2 },
                    );
                    if ar.clicked() {
                        toggles.push(g.clone());
                    }
                    x += 16.0;
                }
                // the chip: the look shared, else a hollow ring with a dot (mixed)
                let c = pos2(x + CHIP / 2.0, r.center().y);
                let same = looks.windows(2).all(|w| w[0] == w[1]);
                let look = if same {
                    looks.first().cloned().flatten()
                } else {
                    None
                };
                chip(
                    ui,
                    c,
                    CHIP / 2.0,
                    look.as_deref().and_then(|n| lib.find(n)),
                    t.ink2,
                );
                if !same {
                    ui.painter().circle_filled(c, 2.5, t.ink2);
                }
                let cr = ui.interact(
                    Rect::from_center_size(c, vec2(CHIP + 4.0, CHIP + 4.0)),
                    ui.scope_id().with(("chip", &name, indent as u32)),
                    Sense::click(),
                );
                let own = match row {
                    Row::Part(k) => st.own.get(*k).cloned().flatten(),
                    _ => None,
                };
                let cr = cr.on_hover_text(match (&look, &own) {
                    (Some(l), _) => format!("{l} (click to change)"),
                    (None, Some(m)) if same => format!("{m}'s own look (click to change)"),
                    (None, _) if same => "its material's own look (click to change)".into(),
                    _ => "mixed looks (click to set one)".into(),
                });
                egui::Popup::menu(&cr).show(|ui| {
                    ui.set_max_width(240.0);
                    if ui.button("Material's own").clicked() {
                        act.push((row.clone(), Some(None)));
                    }
                    for (fam, list) in palette(lib, "") {
                        ui.label(RichText::new(fam).small().color(t.ink2));
                        if let Some(n) = swatches(ui, &list, look.as_deref(), t.focus) {
                            act.push((row.clone(), Some(Some(n))));
                        }
                    }
                });
                let g = ui
                    .painter()
                    .layout_no_wrap(name.clone(), super::theme::sans(13.0), t.ink);
                let tx = c.x + CHIP / 2.0 + 8.0;
                let w = (r.right() - 40.0 - tx).max(10.0);
                ui.painter()
                    .with_clip_rect(Rect::from_min_size(pos2(tx, r.top()), vec2(w, ROW_H)))
                    .galley(pos2(tx, r.center().y - g.size().y / 2.0), g, t.ink);
                let words = if matches!(row, Row::Group(..)) {
                    format!("{count}")
                } else if count > 1 {
                    format!("×{count}")
                } else {
                    String::new()
                };
                ui.painter().text(
                    pos2(r.right() - 6.0, r.center().y),
                    egui::Align2::RIGHT_CENTER,
                    words,
                    super::theme::sans(11.0),
                    t.ink2,
                );
                // a swatch dropped on the row
                if let Some(name) = resp.dnd_release_payload::<String>() {
                    act.push((row.clone(), Some(Some((*name).clone()))));
                }
                if resp.clicked() {
                    match row {
                        Row::Part(k) => clicks.push((*k, ui.input(|i| i.modifiers))),
                        Row::Group(_, m, _) => {
                            let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                            if !ctrl {
                                st.selected.clear();
                            }
                            for k in m {
                                if !st.selected.contains(k) {
                                    st.selected.push(*k);
                                }
                            }
                        }
                    }
                }
            }
        });
    for g in toggles {
        if !st.open.remove(&g) {
            st.open.insert(g);
        }
    }
    for (k, m) in clicks {
        select(st, k, m.command || m.ctrl, m.shift, &shown);
    }
    for (row, look) in act {
        if let Some(look) = look {
            let to = targets(st, &row);
            apply(st, &to, look.as_deref());
        }
    }
    if !st.selected.is_empty() {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} selected", st.selected.len()))
                    .small()
                    .color(t.ink2),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    st.selected.clear();
                }
                if ui
                    .button("Reset")
                    .on_hover_text("their materials' own looks")
                    .clicked()
                {
                    let to = st.selected.clone();
                    apply(st, &to, None);
                }
                if let Some(b) = st.brush.clone()
                    && ui.button(format!("Apply {b}")).clicked()
                {
                    let to = st.selected.clone();
                    apply(st, &to, Some(&b));
                }
            });
        });
    }

    // the overrides alone
    let over = overrides(st);
    if !over.is_empty() {
        egui::CollapsingHeader::new(RichText::new(format!("Overrides  {}", over.len())).color(t.ink))
            .id_salt("studio-overrides")
            .default_open(true)
            .show(ui, |ui| {
                let mut reset = None;
                let h = (over.len() as f32 * ROW_H).min(ROW_H * 8.0);
                egui::ScrollArea::vertical()
                    .id_salt("studio-overrides-list")
                    .max_height(h)
                    .auto_shrink([false, true])
                    .show_rows(ui, ROW_H, over.len(), |ui, range| {
                        for k in &over[range] {
                            ui.horizontal(|ui| {
                                ui.set_height(ROW_H - ui.spacing().item_spacing.y);
                                let look = st.looks[*k].clone().unwrap_or_default();
                                let (r, _) = ui.allocate_exact_size(vec2(CHIP, CHIP), Sense::hover());
                                chip(ui, r.center(), CHIP / 2.0 - 1.0, lib.find(&look), t.ink2);
                                ui.add(egui::Label::new(format!("{}: {look}", st.names[*k])).truncate());
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui
                                        .small_button("×")
                                        .on_hover_text("back to its material's own")
                                        .clicked()
                                    {
                                        reset = Some(*k);
                                    }
                                });
                            });
                        }
                    });
                if let Some(k) = reset {
                    apply(st, &[k], None);
                }
                if ui.small_button("Reset all").clicked() {
                    apply(st, &over, None);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn studio(names: &[&str], groups: &[&str]) -> StudioUi {
        StudioUi {
            names: names.iter().map(|s| s.to_string()).collect(),
            groups: groups.iter().map(|s| s.to_string()).collect(),
            counts: vec![1; names.len()],
            looks: vec![None; names.len()],
            ..Default::default()
        }
    }

    #[test]
    fn the_tree_is_grouped_collapsed_and_filtered() {
        let mut st = studio(
            &["post", "pan", "m6_bolt", "m5_bolt"],
            &["frame", "frame", "hw", "hw"],
        );
        // collapsed: the groups alone
        assert_eq!(
            rows(&st),
            vec![
                Row::Group("frame".into(), vec![0, 1], false),
                Row::Group("hw".into(), vec![2, 3], false)
            ]
        );
        st.open.insert("hw".into());
        assert_eq!(rows(&st).len(), 4);
        // a filter opens what matches, and only that
        st.filter = "BOLT".into();
        assert_eq!(
            rows(&st),
            vec![
                Row::Group("hw".into(), vec![2, 3], true),
                Row::Part(2),
                Row::Part(3)
            ]
        );
        // no groups: the parts alone
        let st = studio(&["a", "b"], &["", ""]);
        assert_eq!(rows(&st), vec![Row::Part(0), Row::Part(1)]);
    }

    #[test]
    fn ctrl_and_shift_select_many_and_one_look_goes_to_them_all() {
        let mut st = studio(&["a", "b", "c", "d", "e"], &["", "", "", "", ""]);
        let shown = [0, 1, 2, 3, 4];
        select(&mut st, 1, false, false, &shown);
        select(&mut st, 3, false, true, &shown);
        assert_eq!(st.selected, vec![1, 2, 3]);
        select(&mut st, 2, true, false, &shown);
        assert_eq!(st.selected, vec![1, 3]);
        select(&mut st, 4, true, false, &shown);
        // a look on a selected row goes to the selection; on another row, to that row alone
        let to = targets(&st, &Row::Part(3));
        apply(&mut st, &to, Some("Brass"));
        assert_eq!(overrides(&st), vec![1, 3, 4]);
        let to = targets(&st, &Row::Part(0));
        assert_eq!(to, vec![0]);
        apply(&mut st, &[3], None);
        assert_eq!(overrides(&st), vec![1, 4]);
        // a click alone selects one
        select(&mut st, 0, false, false, &shown);
        assert_eq!(st.selected, vec![0]);
    }

    /// The tab's time per frame with `n` parts in 10 groups, all open: mean over frames, ms.
    fn frame_ms(n: usize) -> f64 {
        let ctx = egui::Context::default();
        let shell = Shell::default();
        let lib = Appearances::builtin();
        let mut st = studio(&[], &[]);
        st.names = (0..n).map(|k| format!("part_{k}")).collect();
        st.groups = (0..n).map(|k| format!("group_{}", k % 10)).collect();
        st.counts = vec![3; n];
        st.looks = (0..n)
            .map(|k| (k % 7 == 0).then(|| "Brass".to_string()))
            .collect();
        st.open = (0..10).map(|g| format!("group_{g}")).collect();
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(330.0, 860.0))),
            ..Default::default()
        };
        let mut run = || {
            let mut out = ctx.run_ui(input(), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| super::ui(ui, &shell, &mut st, &lib));
            });
            out.textures_delta.clear();
        };
        for _ in 0..3 {
            run();
        }
        let t0 = std::time::Instant::now();
        for _ in 0..20 {
            run();
        }
        t0.elapsed().as_secs_f64() * 1000.0 / 20.0
    }

    /// The parts are drawn only as far as they show: ten times the parts costs far less than ten
    /// times the frame (the overrides list, every override drawn, is the only part that grows).
    #[test]
    fn many_parts_keep_the_frame_flat() {
        let (a, b) = (frame_ms(250), frame_ms(2500));
        eprintln!("frame: 250 parts {a:.2} ms, 2500 parts {b:.2} ms");
        assert!(b < a * 4.0, "250 parts {a:.2} ms, 2500 parts {b:.2} ms");
    }

    #[test]
    fn the_palette_searches_names_and_families() {
        let lib = Appearances::builtin();
        let all: usize = palette(&lib, "").iter().map(|f| f.1.len()).sum();
        assert_eq!(all, lib.all().count());
        let brass = palette(&lib, "brass");
        assert_eq!(brass.len(), 1);
        assert!(brass[0].1.iter().all(|a| a.name.to_lowercase().contains("brass")));
        let metals = palette(&lib, "metal");
        assert!(metals.iter().any(|f| f.0 == "metal"));
        assert!(palette(&lib, "zzzz").is_empty());
    }
}
